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

mod about_win;
mod errors;
mod functions;
mod globals;
mod main_win;
mod main_win_builder;
mod prefs_win;
mod session;
mod shortcuts_win;
mod syntax_config;
mod view_history;

use crate::main_win::MainWinExt;
use crate::main_win_builder::MainWinBuilder;
use crate::session::SessionHandler;
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::prelude::*;
use gio::ApplicationFlags;
use glib::source::{Continue, Priority};
use glib::{clone, Char, MainContext};
use gtk::Application;
use log::{debug, error, info, warn};
use serde_json::{from_value, Value};
use std::cell::RefCell;
use std::env::args;
use std::path::Path;
use std::rc::Rc;

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

    let args = &args().collect::<Vec<_>>();
    //FIXME: Use handle-local-options once https://github.com/gtk-rs/gtk/issues/580 is a thing
    let mut new_instance = false;
    for arg in args {
        match arg.as_str() {
            "-n" | "--new-instance" => new_instance = true,
            _ => (),
        }
    }

    let main_win_builder = Rc::new(RefCell::new(MainWinBuilder::new(application.clone())));

    application.connect_startup(clone!(@weak main_win_builder => @default-panic, move |_| {
        debug!("Starting Tau");

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

        main_win_builder.borrow_mut().build();
    }));

    application.connect_activate(clone!(@weak main_win_builder => @default-panic, move |_| {
        debug!("Activating new view");

        let schema = gio::Settings::new("org.gnome.Tau");
        let paths = schema.get_session();
        if schema
            .get("restore-session") && !new_instance && !paths.is_empty() {
                for file in paths {
                    if Path::new(&file).exists() {
                        let (tx, rx) = MainContext::channel::<Result<Value, Value>>(Priority::default());
                        let main_win = main_win_builder.borrow().main_win.clone();
                        rx.attach(None, clone!(@strong schema, @strong main_win, @strong file => move |res| {
                            match res {
                                Ok(val) => main_win.as_ref().unwrap().new_view(Ok((serde_json::from_value(val).unwrap(), Some(file.clone())))),
                                Err(e) => {
                                    error!("Failed to restore file `{}`", &e);
                                    schema.session_remove(&from_value::<String>(e).unwrap());
                                }
                            }
                            Continue(false)
                        }));

                        main_win_builder.borrow().spawn_view(Some(file.clone()), move |res| {
                            tx.send(res).unwrap()
                        });
                    } else {
                        schema.session_remove(&file);
                        error!("Failed to restore file `{}`", file);
                    }
                }
        } else {
            let (tx, rx) = MainContext::channel::<Result<Value, Value>>(Priority::default());
            let main_win = main_win_builder.borrow().main_win.clone();
            rx.attach(None, clone!(@strong schema, @strong main_win => move |res| {
                match res {
                    Ok(val) => main_win.as_ref().unwrap().new_view(Ok((serde_json::from_value(val).unwrap(), None))),
                    Err(e) => {
                        error!("Failed to open new view due to error `{}`", &e);
                        schema.session_remove(&from_value::<String>(e.clone()).unwrap());
                        main_win.as_ref().unwrap().new_view(Err(serde_json::from_value(e).unwrap()));
                    }
                }
                Continue(false)
            }));

            main_win_builder.borrow().spawn_view(None, move |res| {
                tx.send(res).unwrap()
            });
        };
    }));

    application.connect_open(
        clone!(@weak main_win_builder => @default-panic, move |_,files,_| {
            debug!("Opening new files");

            let mut paths: Vec<String> = files.iter()
                .filter_map(gio::File::get_path)
                .map(std::path::PathBuf::into_os_string)
                .filter_map(|s| s.into_string().ok())
                .collect();

            debug!("Files: {:#?}", paths);


            let schema = gio::Settings::new("org.gnome.Tau");
            let mut session_paths = Vec::new();
            if schema
                .get("restore-session") && !new_instance {
                    session_paths.append(&mut schema.get_session());
                    paths.extend(session_paths.iter().cloned())
                }

            let session_paths_rc = Rc::new(session_paths);
            for file in paths {
                let (tx, rx) = MainContext::channel::<Result<Value, Value>>(Priority::default());
                let main_win = main_win_builder.borrow().main_win.clone();
                rx.attach(None, clone!(@strong schema, @strong main_win, @strong file, @strong session_paths_rc => move |res| {
                    match res {
                        Ok(val) => main_win.as_ref().unwrap().new_view(Ok((serde_json::from_value(val).unwrap(), Some(file.clone())))),
                        Err(e) => {
                            if session_paths_rc.contains(&file) {
                                error!("Failed to restore file `{}`", &e);
                                schema.session_remove(&from_value::<String>(e).unwrap());
                            } else {
                                main_win.as_ref().unwrap().new_view(Err(serde_json::from_value(e).unwrap()));
                            }
                        }
                    }
                    Continue(false)
                }));

                main_win_builder.borrow().spawn_view(Some(file.clone()), move |res| {
                    tx.send(res).unwrap()
                });
            }
        }),
    );

    application.connect_shutdown(move |_| {
        debug!("Shutting down!");
    });

    if new_instance {
        application.set_flags(ApplicationFlags::HANDLES_OPEN | ApplicationFlags::NON_UNIQUE);
    }

    application.run(args);
}
