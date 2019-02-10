#![recursion_limit = "128"]
//Just for now...
#![allow(dead_code)]

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
mod proto;
mod rpc;
mod shared_queue;
mod theme;
mod xi_thread;

use crate::main_win::MainWin;
use crate::pref_storage::Config;
use crate::rpc::Core;
use crate::shared_queue::{CoreMsg, ErrMsg, SharedQueue};
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::MainContext;
use gtk::Application;
use log::{debug, info, warn};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let shared_queue = SharedQueue::new();

    let (err_tx, err_rx) = MainContext::channel::<ErrMsg>(glib::PRIORITY_DEFAULT);

    let (xi_peer, xi_rx) = xi_thread::start_xi_thread();
    let core = Core::new(xi_peer, xi_rx, err_tx, shared_queue.clone());

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

    let application = Application::new(
        crate::globals::APP_ID.unwrap_or("com.github.Cogitri.gxi"),
        ApplicationFlags::HANDLES_OPEN,
    )
    .unwrap_or_else(|_| panic!("{}", gettext("Failed to create the GTK+ application")));

    let main_context = MainContext::default();
    main_context.acquire();
    err_rx.attach(&main_context, |err_msg| {
        crate::errors::ErrorDialog::new(err_msg);
        glib::source::Continue(false)
    });

    let xi_config = Arc::new(Mutex::new(Config::new()));

    application.connect_startup(clone!(shared_queue, core, xi_config => move |application| {
        debug!("{}", gettext("Starting gxi"));

        let xi_config_dir = { xi_config.lock().unwrap().path.clone() };
        core.client_started(&xi_config_dir, include_str!(concat!(env!("OUT_DIR"), "/plugin-dir.in")));

        MainWin::new(
            application,
            shared_queue.clone(),
            &Rc::new(RefCell::new(core.clone())),
            xi_config.clone(),
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
            let path = file.get_path();
            if path.is_none() { continue; }
            let path = path.unwrap();
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
    }));

    application.connect_shutdown(move |_| {
        debug!("{}", gettext("Shutting down..."));
    });

    application.run(&args().collect::<Vec<_>>());
}
