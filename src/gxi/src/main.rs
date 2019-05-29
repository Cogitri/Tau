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
use futures::future::Future;
use futures::stream::Stream;
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::MainContext;
use gtk::Application;
use gxi_config_storage::pref_storage::GSchemaExt;
use gxi_config_storage::GSchema;
use log::{debug, info, warn};
use serde_json::json;
use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;
use std::thread;
use xrl::{spawn as spawn_xi, Client, ViewId, XiEvent};

fn main() {
    //PanicHandler::new();

    env_logger::Builder::from_default_env().init();

    let application = Application::new(
        Some("com.github.Cogitri.gxi"),
        ApplicationFlags::HANDLES_OPEN,
    )
    .unwrap_or_else(|_| panic!("Failed to create the GTK+ application"));

    // The channel to signal MainWin to create a new tab with an EditView
    let (new_view_tx, new_view_rx) =
        MainContext::channel::<(ViewId, Option<String>)>(glib::PRIORITY_LOW);
    // The channel through which all events from Xi are sent from `crate::frontend::GxiFrontend` to
    // the MainWin
    let (event_tx, event_rx) = MainContext::sync_channel::<XiEvent>(glib::PRIORITY_HIGH, 5);
    let (core, core_stderr) = spawn_xi("xi-core", GxiFrontendBuilder { event_tx });

    let log_core_errors = core_stderr
        .for_each(|msg| {
            eprintln!("xi-core stderr: {}", msg);
            Ok(())
        })
        .map_err(|_| ());
    thread::spawn(move || {
        tokio::run(log_core_errors);
    });

    //FIXME: This is a hack to satisfy the borrowchecker. `connect_startup` is a FnMut even though
    // it's only called once, so it's fine to move new_view_rx and event_rx into connect_startup
    let new_view_rx_opt = Rc::new(RefCell::new(Some(new_view_rx)));
    let event_rx_opt = Rc::new(RefCell::new(Some(event_rx)));

    application.connect_startup(
        enclose!((core, application, new_view_rx_opt, event_rx_opt, new_view_tx) move |_| {
            debug!("{}", gettext("Starting gxi"));

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

            // Start xi-editor
            tokio::run(core.client_started(None, crate::globals::PLUGIN_DIR).map_err(|_|()));

            setup_config(&core);

            MainWin::new(
                application.clone(),
                core.clone(),
                new_view_rx_opt.borrow_mut().take().unwrap(),
                new_view_tx.clone(),
                event_rx_opt.borrow_mut().take().unwrap(),
            );
        }),
    );

    application.connect_activate(enclose!((core, new_view_tx) move |_| {
        debug!("{}", gettext("Activating new view"));

        let view_id = tokio::executor::current_thread::block_on_all(core.new_view(None)).unwrap();

        new_view_tx.send((view_id, None)).unwrap();
    }));

    application.connect_open(enclose!((core) move |_,files,_| {
        debug!("{}", gettext("Opening new file"));

        for file in files {
            if let Some(path) = file.get_path() {
                let view_id = tokio::executor::current_thread::block_on_all(core.new_view(None)).unwrap();
                new_view_tx.send((view_id, path.to_str().map(|s| s.to_string()))).unwrap();
            }
        }
    }));

    application.connect_shutdown(move |_| {
        debug!("{}", gettext("Shutting down…"));
    });

    application.run(&args().collect::<Vec<_>>());
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
