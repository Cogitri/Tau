//! Welcome to the tau docs!
//! Since tau isn't a library these docs are meant to help contributors understand tau's code.
//!
//! tau's structure can be simplified like this:
//!
//!```
//! ---------  spawns w/    -----------                 ----------
//! |       |    tokio      |         |changes MainState|        |
//! |  xrl  |<--------------| MainWin |<----------------|EditView|
//! |Client |  XiRequest    |MainState|  forwards msgs  |        |
//! |       |-------------->|         |---------------->|        |
//! ---------XiNotification ----------- related to edit ----------
//! ^   ^                                                  |
//! |   |---------                                         |
//! |            |                                         |
//! xi-editor    |                                         |
//! sends        |                                         |
//! msgs         |------------------------------------------
//!              sends editing events to RPC, which forwards
//!                      them to xi to process
//!```
//!
//! Now onto more detailed explanation:
//!
//! - `MainWin`:  This is main window (as the name suggests). It holds all buttons you can see when
//!               opening tau, such as the open button, new tab button, the syntax selection, the save
//!               button and window controls. It also has a `Notebook` inside of it, which holds `EditView`s.
//!               The `Notebook` shows a tab for every open `EditView`, allowing the user to open multiple
//!               documents at once.
//!               The `MainWin` also manages the `XiNotifications` and `XiRequests` it receives from
//!               `xrl`, the RPC lib which deals with communicating with Xi in an async way. It receives
//!               messages (`XiNotifications`, meaning Xi tells us something, e.g. and `Update` when
//!               the text has changed due to user input) and requests (`XiRequest`, meaning Xi wants
//!               some information from us, e.g. how wide a string of text is for word wrapping) via
//!               a `crossbeam_channel` pair. Some of these messages are dealt with in the `MainWin`
//!               already, like `alert`, which opens an error dialog, e.g. when an unreadable files
//!               is attempted to be opened. Other messages are forwarded to the respective `EditView`
//!               they're meant for, e.g. `update`, which updates the text/styling of the document.
//!               Please see [the xi-frontend docs](https://xi-editor.io/docs/frontend-protocol.html)
//!               for more info.
//!               The `MainWin` also holds a `SharedState`, which includes settings like the fontsize,
//!               fontname etc. which is shared among all `EditView`s.
//!
//! - `EditView`: This is where all the actual editing takes place. Since this is a `GtkLayout`
//!               we have to handle everything ourselves: Scrolling to the right lines, setting editing
//!               shortcuts (e.g. copy&paste), drawing each line and sending changes to xi-editor.
//!               It also processes the `XiNotifications` and `XiRequests` it receives from the `MainWin`,
//!               updating the visible text or its styling and much more.
//!
//! - `Client`:   This is a Struct of `xrl`. It interfaces with `xi-editor`. Please see its docs for more
//!               info on `xrl`.
//!
//! tau also contains some more minor modules, please see their documentation for more info:
//!
//! - [AboutWin](about_win/struct.AboutWin.html)
//! - [ErrWin](errors/struct.ErrorDialog.html)
//! - [Frontend](frontend/struct.TauFrontend.html)
//! - [PrefsWin](prefs_win/struct.PrefsWin.html)
//!
//! I can very much recommend you to look at [the following tutorial](https://mmstick.github.io/gtkrs-tutorials/) if you don't
//! know gtk-rs yet!

#![recursion_limit = "128"]
#![deny(clippy::all)]
// Below we log xi's log messages depending on what log level is selected which clippy doesn't like
// because we use the same println! for all of these.
#![allow(clippy::if_same_then_else)]

#[macro_use]
extern crate enclose;

mod about_win;
mod errors;
mod frontend;
mod functions;
mod globals;
mod main_win;
mod prefs_win;
mod session;
mod shortcuts_win;
mod syntax_config;
mod view_history;

use crate::errors::XiClientError;
use crate::frontend::{TauFrontendBuilder, XiEvent, XiRequest};
use crate::main_win::MainWin;
use crate::session::SessionHandler;
use crossbeam_channel::unbounded;
use futures::stream::Stream;
use futures::{future, future::Future};
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::{Char, MainContext};
use gschema_config_storage::{GSchema, GSchemaExt};
use gtk::Application;
use log::{debug, error, info, max_level as log_level, warn, LevelFilter};
use parking_lot::Mutex;
use std::cell::RefCell;
use std::env::args;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;
use xrl::spawn as spawn_xi;

fn main() {
    //PanicHandler::new();

    env_logger::Builder::from_default_env().init();

    let application = Application::new(
        Some(crate::globals::APP_ID.unwrap_or("org.gnome.TauDevel")),
        ApplicationFlags::HANDLES_OPEN,
    )
    .unwrap_or_else(|_| panic!("Failed to create the GTK+ application"));

    application.add_main_option(
        "new-instance",
        Char::new('n').unwrap(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        &gettext("Start a new instance of the application"),
        None,
    );

    // The channel through which all events from Xi are sent from `crate::frontend::TauFrontend` to
    // the MainWin
    let (event_tx, event_rx) = MainContext::sync_channel::<XiEvent>(glib::PRIORITY_HIGH, 5);
    // Set this to none here so we can move it into the closures without actually starting Xi every time.
    // This significantly improves startup time when Tau is already opened and you open a new file via
    // the CLI.
    let core_opt = Arc::new(Mutex::new(None));

    //FIXME: This is a hack to satisfy the borrowchecker. `connect_startup` is a FnMut even though
    // it's only called once, so it's fine to move new_view_rx and event_rx into connect_startup
    let event_rx_opt = Rc::new(RefCell::new(Some(event_rx)));

    let runtime_opt = Rc::new(RefCell::new(None));

    application.connect_startup(
        enclose!((core_opt, application, event_rx_opt, event_tx, runtime_opt) move |_| {
            debug!("Starting Tau");

            // The channel to send the result of a request back to Xi
            let (request_tx, request_rx) = unbounded::<XiRequest>();

            let xi_config_dir = std::env::var("XI_CONFIG_DIR").ok();

            let mut runtime = Runtime::new().unwrap();

            let core_res = runtime.block_on(future::lazy(enclose!((request_tx, core_opt, event_tx) move || {
                let res = spawn_xi(
                    crate::globals::XI_PATH.unwrap_or("xi-core"),
                    TauFrontendBuilder {
                        request_rx,
                        event_tx,
                        request_tx: request_tx.clone(),
                    },
                );

                if let Ok((client, core_stderr)) = res {
                    let _ = client.client_started(
                        xi_config_dir.as_ref().map(String::as_str),
                        crate::globals::PLUGIN_DIR,
                    );

                    core_opt.lock().replace(Some(client.clone()));

                    Ok((client,core_stderr))
                } else {
                    Err(res.err().unwrap())
                }
            })));

            let (core, core_stderr) = core_res.unwrap_or_else(|e| {
                error!("{}", e);
                panic!();
            });

            runtime.spawn(future::lazy(move || {
                tokio::spawn(
                    core_stderr
                        .for_each(|msg| {
                            if msg.contains("[ERROR]") {
                                println!("{}", msg)
                            } else if msg.contains("[WARN]") {
                                if log_level() >= LevelFilter::Warn {
                                    println!("{}", msg)
                                } else if log_level() >= LevelFilter::Info && msg.contains("deprecated") {
                                    println!("{}", msg)
                                }
                            } else if msg.contains("[INFO]") && log_level() >= LevelFilter::Info {
                                println!("{}", msg)
                            } else if msg.contains("[DEBUG]") && log_level() >= LevelFilter::Debug {
                                println!("{}", msg)
                            } else if msg.contains("[TRACE]") && log_level() >= LevelFilter::Trace {
                                println!("{}", msg)
                            }
                            Ok(())
                        })
                        .map_err(|_| ()),
                );

                future::ok(())
            }));

            glib::set_application_name(crate::globals::NAME.unwrap_or("Tau (Development)"));

            // No need to gettext this, gettext doesn't work yet
            match TextDomain::new("tau")
                .push(crate::globals::LOCALEDIR.unwrap_or("po"))
                .init()
            {
                Ok(locale) => info!("Translation found, setting locale to {:?}", locale),
                Err(TextDomainError::TranslationNotFound(lang)) => {
                    // We don't have an 'en' catalog since the messages are English by default
                    if lang != "en" {
                        warn!("Translation not found for lang {}", lang)
                    }
                }
                Err(TextDomainError::InvalidLocale(locale)) => warn!("Invalid locale {}", locale),
            }

            runtime_opt.replace(Some(runtime));

            crate::functions::setup_config(&core);

            MainWin::new(
                &application,
                core,
                event_rx_opt.borrow_mut().take().unwrap(),
                event_tx.clone(),
                request_tx,
                runtime_opt.clone(),
            );
        }),
    );

    application.connect_activate(enclose!((core_opt, event_tx => new_view_tx, runtime_opt) move |_| {
        debug!("Activating new view");

        // It's fine to unwrap here - we already made sure this is Some in connect_startup.
        let core = core_opt.lock().clone().unwrap().unwrap();

        let schema = GSchema::new("org.gnome.Tau");
        if schema
            .get_key("restore-session") {
                let paths = schema.get_session();
                for file in paths {
                    if Path::new(&file).exists() {
                        runtime_opt.borrow_mut().as_mut().unwrap().spawn(
                            future::lazy(enclose!((core, new_view_tx) move || {
                                core
                                .new_view(Some(file.clone()))
                                .then(|res|
                                    future::lazy(move || {
                                        match res {
                                            Ok(view_id) => new_view_tx.send(XiEvent::NewView(Ok((view_id, Some(file))))).unwrap(),
                                            Err(_) => {
                                                GSchema::new("org.gnome.Tau").session_remove(&file);
                                                error!("Failed to restore file `{}`", file);
                                            },
                                        }
                                        Ok(())
                                    })
                                )
                            }))
                        );
                    } else {
                        GSchema::new("org.gnome.Tau").session_remove(&file);
                        error!("Failed to restore file `{}`", file);
                    }
                }
        } else {
            runtime_opt.borrow_mut().as_mut().unwrap().spawn(
                future::lazy(enclose!((core, new_view_tx) move || {
                    core
                    .new_view(None)
                    .then(|res|
                        future::lazy(move || {
                            match res {
                                Ok(view_id) => new_view_tx.send(XiEvent::NewView(Ok((view_id, None)))).unwrap(),
                                Err(e) => {
                                    if let xrl::ClientError::ErrorReturned(value) = e {
                                        let err: XiClientError = serde_json::from_value(value).unwrap();
                                        new_view_tx.send(XiEvent::NewView(Err(format!("{}: '{}'", gettext("Failed open new view due to error"), err.message)))).unwrap()
                                    }
                                },
                            }
                            Ok(())
                        })
                        )
                }))
            );
        };
    }));

    application.connect_open(
        enclose!((core_opt, event_tx => new_view_tx, runtime_opt) move |_,files,_| {
            debug!("Opening new file");

            // See above for why it's fine to unwrap here.
            let core = core_opt.lock().clone().unwrap().unwrap();

            let schema = GSchema::new("org.gnome.Tau");
            if schema
                .get_key("restore-session") {
                    let paths = schema.get_session();
                    for file in paths {
                        if Path::new(&file).exists() {
                            runtime_opt.borrow_mut().as_mut().unwrap().spawn(
                                future::lazy(enclose!((core, new_view_tx) move || {
                                    core
                                    .new_view(Some(file.clone()))
                                    .then(|res|
                                        future::lazy(move || {
                                            match res {
                                                Ok(view_id) => new_view_tx.send(XiEvent::NewView(Ok((view_id, Some(file))))).unwrap(),
                                                Err(_) => {
                                                    GSchema::new("org.gnome.Tau").session_remove(&file);
                                                    error!("Failed to restore file `{}`", file);
                                                },
                                            }
                                            Ok(())
                                        })
                                    )
                                }))
                            );
                        } else {
                            GSchema::new("org.gnome.Tau").session_remove(&file);
                            error!("Failed to restore file `{}`", file);
                        }
                    }
            }

            let paths: Vec<String> = files.iter()
                .filter_map(gio::File::get_path)
                .map(std::path::PathBuf::into_os_string)
                .filter_map(|s| s.into_string().ok())
                .collect();

            for file in paths {
                runtime_opt.borrow_mut().as_mut().unwrap().spawn(
                    future::lazy(enclose!((core, new_view_tx) move || {
                        core
                        .new_view(Some(file.clone()))
                        .then(|res|
                            future::lazy(move || {
                                match res {
                                    Ok(view_id) => new_view_tx.send(XiEvent::NewView(Ok((view_id, Some(file))))).unwrap(),
                                    Err(e) => {
                                        if let xrl::ClientError::ErrorReturned(value) = e {
                                            let err: XiClientError = serde_json::from_value(value).unwrap();
                                            new_view_tx.send(XiEvent::NewView(Err(format!("{}: '{}'", gettext("Failed to open new view due to error"), err.message)))).unwrap()
                                        }
                                    },
                                }
                                Ok(())
                            })
                        )
                    }))
                );
            }
        }));

    application.connect_shutdown(enclose!((runtime_opt)move |_| {
        debug!("Shutting down…");
        if let Some(runtime) = runtime_opt.borrow_mut().take() {
            runtime.shutdown_now().wait().unwrap();
        }
    }));

    let args = &args().collect::<Vec<_>>();
    //FIXME: Use handle-local-options once https://github.com/gtk-rs/gtk/issues/580 is a thing
    let mut new_instance = false;
    for arg in args {
        match arg.as_str() {
            "-n" | "--new-instance" => new_instance = true,
            _ => (),
        }
    }

    if new_instance {
        application.set_flags(ApplicationFlags::HANDLES_OPEN | ApplicationFlags::NON_UNIQUE);
    }

    application.run(args);
}
