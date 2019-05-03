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
//Just for now...
#![allow(dead_code)]
#![deny(clippy::all)]

#[macro_use]
mod macros;

mod about_win;
mod edit_view;
mod errors;
mod globals;
mod linecache;
mod main_win;
mod pref_storage;
mod prefs_win;
mod rpc;
mod shared_queue;
mod theme;
mod xi_thread;

use crate::errors::ErrorMsg;
use crate::main_win::MainWin;
use crate::pref_storage::Config;
use crate::rpc::Core;
use crate::shared_queue::{CoreMsg, SharedQueue};
use crate::xi_thread::XiPeer;
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::MainContext;
use gtk::Application;
use human_panic::setup_panic;
use log::{debug, info, trace, warn};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;

fn main() {
    setup_panic!();

    // Only set Warn as loglevel if the user hasn't explicitly set something else
    if std::env::var_os("RUST_LOG").is_none() {
        // Xi likes to return some not-so-necessary Warnings (e.g. if the config
        // hasn't changed), so let's only turn on warnings for gxi.
        env_logger::Builder::new()
            .filter_module("gxi", log::LevelFilter::Warn)
            .default_format_timestamp(false)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .default_format_timestamp(false)
            .init();
    }

    let shared_queue = SharedQueue::new();

    let (err_tx, err_rx) = MainContext::channel::<ErrorMsg>(glib::PRIORITY_DEFAULT_IDLE);

    let (xi_peer, xi_rx) = XiPeer::new();
    let core = Core::new(xi_peer, xi_rx, err_tx, shared_queue.clone());

    trace!("application_id: {}", app_id!());
    let application = Application::new(Some(app_id!()), ApplicationFlags::HANDLES_OPEN)
        .unwrap_or_else(|_| panic!("Failed to create the GTK+ application"));

    let main_context = MainContext::default();
    main_context.acquire();
    // Used to create error msgs from threads other than the main thread
    err_rx.attach(Some(&main_context), |err_msg| {
        crate::errors::ErrorDialog::new(err_msg);
        glib::source::Continue(false)
    });

    application.connect_startup(clone!(shared_queue, core => move |application| {
        debug!("{}", gettext("Starting gxi"));

        let (config_dir, xi_config) = Config::new();

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

        core.client_started(&config_dir, crate::globals::PLUGIN_DIR.unwrap_or("/usr/local/lib/gxi/plugins"));

        MainWin::new(
            application,
            shared_queue.clone(),
            Rc::new(RefCell::new(core.clone())),
            Rc::new(RefCell::new(xi_config)),
           );
    }));

    application.connect_activate(clone!(shared_queue, core => move |_| {
        debug!("{}", gettext("Activating new view"));

        let mut params = json!({});
        params["file_path"] = Value::Null;

        let shared_queue = shared_queue.clone();
        core.send_request("new_view", &params,
            move |value| {
                shared_queue.add_core_msg(CoreMsg::NewViewReply{
                    file_name: None,
                    value: value.clone(),
                })
            }
        );
    }));

    application.connect_open(clone!(shared_queue, core => move |_,files,_| {
        debug!("{}", gettext("Opening new file"));

        for file in files {
            if let Some(path) = file.get_path() {
                let path = path.to_string_lossy().into_owned();

                let mut params = json!({});
                params["file_path"] = json!(path);

                let shared_queue = shared_queue.clone();
                core.send_request("new_view", &params,
                    move |value| {
                        shared_queue.add_core_msg(CoreMsg::NewViewReply{
                            file_name: Some(path),
                        value: value.clone(),
                        })
                    }
                );
            }
        }
    }));

    application.connect_shutdown(move |_| {
        debug!("{}", gettext("Shutting down…"));
    });

    application.run(&args().collect::<Vec<_>>());
}
