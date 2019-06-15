//! Welcome to the gxi docs!
//! Since gxi isn't a library these docs are meant to help contributors understand gxi's code.
//!
//! gxi's structure can be simplified like this:
//!
//!```
//! ----------            -----------                 ----------
//! |        |   spawns   |         |changes MainState|        |
//! |  Core  |<-----------| MainWin |<----------------|EditView|
//! | thread | sends msgs |MainState|  forwards msgs  |        |
//! |        |----------->|         |---------------->|        |
//! ----------            ----------- related to edit ----------
//! ^   ^                                                  |
//! |   |---------                                         |
//! |            |                                         |
//! xi-editor    |                                         |
//! sends        |                                         |
//! msgs         |------------------------------------------
//!              sends editing events to RPC, which forwards
//!                      them to xi to process them
//!```
//!
//! Now onto more detailed explanation:
//!
//! - `MainWin`:  This is main window (as the name suggests). It holds all buttons you can see when
//!               opening gxi, such as the open button, new tab button, the syntax selection, the save
//!               button and window controls. It also has a `Notebook` inside of it, which holds `EditView`s.
//!               The `Notebook` shows a tab for every open `EditView`, allowing the user to open multiple
//!               documents at once.
//!               The `MainWin` also has another important feature: It deals with so called `CoreMsg`s.
//!               It grabs them from a `SharedQueue` which is `crossbeam_deque::Injector` under the hood.
//!               They are messages xi-editor sends us, telling us stuff like config changes by the user
//!               (e.g. the font size has been changed) or that we should measure the view's size for it,
//!               for word wrapping. Please see [the xi-frontend docs](https://xi-editor.io/docs/frontend-protocol.html)
//!               for more info.
//!               `MainWin` processes some messages by itself, e.g. displaying an error message if xi
//!               sends us an `alert`. It sends editing related messages to the appropriate `EditView`.
//!               `MainWin` also holds the `MainState`, which holds stuff the `EditView` might need too,
//!               e.g. the selected font face&font size, which syntax or theme has been selected etc.
//!
//! - `EditView`: This is where all the actual editing takes place. Since this is a GTK `DrawingArea`
//!               we have to handle everything ourselves: Scrolling to the right lines, setting editing
//!               shortcuts (e.g. copy&paste), drawing each line and sending changes to xi-editor.
//!               It also processes the `CoreMsg`s it receives from `MainWin`, e.g. setting the appropriate
//!               font size&font face.
//!
//! - `Core`:     This deals with receiving messages from xi-editor (via a new thread) and adding them
//!               to the `SharedQueue` for `MainWin` to deal with later on. It also contains the functions
//!               to send messages back to xi-editor, e.g. for notifying it about new editing events
//!               such as us inserting a character. Again, Please see [the xi-frontend docs](https://xi-editor.io/docs/frontend-protocol.html)
//!               for more info on how this works and what messages can be exchanged and how the RPC works.
//!
//! gxi also contains some more minor modules, please see their documentation for more info:
//!
//! - [AboutWin](about_win/struct.AboutWin.html)
//! - [Config](pref_storage/struct.Config.html) and [XiConfig](pref_storage/struct.XiConfig.html)
//! - [ErrWin](errors/struct.ErrorDialog.html)
//! - [PrefsWin](prefs_win/struct.PrefsWin.html)
//! - [SharedQueue](shared_queue/struct.SharedQueue.html)
//!
//! I can very much recommend you to look at [the following tutorial](https://mmstick.github.io/gtkrs-tutorials/) if you don't
//! know gtk-rs yet!

#![recursion_limit = "128"]
#![deny(clippy::all)]

#[macro_use]
extern crate enclose;

mod about_win;
mod errors;
mod frontend;
mod globals;
mod main_win;
//mod panic_handler;
mod prefs_win;

use crate::frontend::*;
use crate::main_win::MainWin;
//use crate::panic_handler::PanicHandler;
use crossbeam_channel::unbounded;
use futures::stream::Stream;
use futures::{future, future::Future};
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::{Char, MainContext};
use gtk::Application;
use gxi_config_storage::pref_storage::GSchemaExt;
use gxi_config_storage::GSchema;
use log::{debug, error, info, warn};
use serde_json::json;
use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;
use xrl::{spawn as spawn_xi, Client, ViewId};

fn main() {
    //PanicHandler::new();

    env_logger::Builder::from_default_env().init();

    let application = Application::new(
        Some("com.github.Cogitri.gxi"),
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

    // The channel to signal MainWin to create a new tab with an EditView
    let (new_view_tx, new_view_rx) =
        MainContext::channel::<(ViewId, Option<String>)>(glib::PRIORITY_HIGH);
    // Set this to none here so we can move it into the closures without actually starting Xi every time.
    // This significantly improves startup time when gxi is already opened and you open a new file via
    // the CLI.
    let core = Rc::new(RefCell::new(None));

    //FIXME: This is a hack to satisfy the borrowchecker. `connect_startup` is a FnMut even though
    // it's only called once, so it's fine to move new_view_rx and event_rx into connect_startup
    let new_view_rx_opt = Rc::new(RefCell::new(Some(new_view_rx)));

    application.connect_startup(
        enclose!((core, application, new_view_rx_opt, new_view_tx) move |_| {
            debug!("{}", gettext("Starting gxi"));

        // The channel through which all events from Xi are sent from `crate::frontend::GxiFrontend` to
        // the MainWin
        let (event_tx, event_rx) = MainContext::sync_channel::<XiEvent>(glib::PRIORITY_HIGH, 5);

        // The channel to send the result of a request back to Xi
        let (request_tx, request_rx) = unbounded::<XiRequest>();

        let (client_tx, client_rx) = MainContext::channel::<Client>(glib::PRIORITY_HIGH);

        std::thread::spawn(enclose!((request_tx) move || {
            let xi_config_dir = std::env::var("XI_CONFIG_DIR").ok();
            let (client, core_stderr) = spawn_xi(
                crate::globals::XI_PATH.unwrap_or("xi-core"),
                GxiFrontendBuilder {
                    request_rx,
                    event_tx,
                    request_tx: request_tx.clone(),
                },
            );

            client_tx.send(client.clone()).unwrap();

            tokio::run(future::lazy( move || {
                error!("starting spawn");

                tokio::spawn(
            core_stderr
                .for_each(|msg| {
                    println!("xi-core stderr: {}", msg);
                    Ok(())
                })
                .map_err(|_| ()),
        );

                        client.client_started(
                            xi_config_dir.as_ref().map(String::as_str),
                            crate::globals::PLUGIN_DIR,
                        );

                        future::ok(())
                    }));
            }));

            glib::set_application_name("gxi");

            // No need to gettext this, gettext doesn't work yet
            match TextDomain::new("gxi")
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

            // See above comment for new_view_rx_opt
            let event_rx_opt = Rc::new(RefCell::new(Some(event_rx)));

            let main_context = MainContext::default();

            client_rx.attach(Some(&main_context), enclose!((core, request_tx, application, new_view_tx, new_view_rx_opt, event_rx_opt) move |client| {
                core.replace(Some(client.clone()));

                setup_config(&client);

                MainWin::new(
                    application.clone(),
                    client.clone(),
                    new_view_rx_opt.borrow_mut().take().unwrap(),
                    new_view_tx.clone(),
                    event_rx_opt.borrow_mut().take().unwrap(),
                    request_tx.clone(),
                );

                glib::source::Continue(false)
        }));
    }));

    application.connect_activate(enclose!((core, new_view_tx) move |_| {
        debug!("{}", gettext("Activating new view"));

        let view_id = tokio::executor::current_thread::block_on_all(core.borrow().as_ref().unwrap().new_view(None)).unwrap();

        new_view_tx.send((view_id, None)).unwrap();
    }));

    application.connect_open(enclose!((core) move |_,files,_| {
        debug!("{}", gettext("Opening new file"));

        for file in files {
            if let Some(path) = file.get_path() {
                let file =  path.to_str().map(|s| s.to_string());
                let view_id = tokio::executor::current_thread::block_on_all(core.borrow().as_ref().unwrap().new_view(file.clone())).unwrap();
                new_view_tx.send((view_id, file)).unwrap();
            }
        }
    }));

    application.connect_shutdown(move |_| {
        debug!("{}", gettext("Shutting down…"));
    });

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

/// Send the current config to xi-editor during startup
fn setup_config(core: &Client) {
    let gschema = GSchema::new("com.github.Cogitri.gxi");

    let tab_size: u32 = gschema.get_key("tab-size");
    let autodetect_whitespace: bool = gschema.get_key("auto-indent");
    let translate_tabs_to_spaces: bool = gschema.get_key("translate-tabs-to-spaces");
    let use_tab_stops: bool = gschema.get_key("use-tab-stops");
    let word_wrap: bool = gschema.get_key("word-wrap");

    let font: String = gschema.get_key("font");
    let font_vec = font.split_whitespace().collect::<Vec<_>>();
    let (font_size, font_name) = if let Some((size, splitted_name)) = font_vec.split_last() {
        (size.parse::<f32>().ok(), Some(splitted_name.join(" ")))
    } else {
        (None, None)
    };

    #[cfg(windows)]
    const LINE_ENDING: &str = "\r\n";
    #[cfg(not(windows))]
    const LINE_ENDING: &str = "\n";

    tokio::executor::current_thread::block_on_all(core.modify_user_config(
        "general",
        json!({
            "tab_size": tab_size,
            "autodetect_whitespace": autodetect_whitespace,
            "translate_tabs_to_spaces": translate_tabs_to_spaces,
            "font_face": font_name,
            "font_size": font_size,
            "use_tab_stops": use_tab_stops,
            "word_wrap": word_wrap,
            "line_ending": LINE_ENDING,
        }),
    ))
    .unwrap();
}
