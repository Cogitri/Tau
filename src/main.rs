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
mod theme;
mod xi_thread;

use crate::main_win::MainWin;
use crate::pref_storage::{Config, XiConfig};
use crate::rpc::Core;
use crossbeam_deque::{Injector, Worker};
use gettextrs::{gettext, TextDomain, TextDomainError};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::prelude::*;
use gtk::Application;
use log::{debug, error, info, trace, warn};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::env::args;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum CoreMsg {
    Notification {
        method: String,
        params: Value,
    },
    NewViewReply {
        file_name: Option<String>,
        value: Value,
    },
    ShutDown {},
}

#[derive(Clone)]
pub struct SharedQueue {
    queue_rx: Arc<Mutex<Injector<CoreMsg>>>,
    queue_tx: Arc<Mutex<Injector<CoreMsg>>>,
}

impl SharedQueue {
    /// A message from xi-editor that we have to process (e.g. that we should scroll)
    pub fn add_core_msg(&self, msg: CoreMsg) {
        trace!("{}", gettext("Pushing to rx queue"));
        self.queue_rx.lock().unwrap().push(msg);
    }
    /// A message that we want to send to xi-editor in order for it to process it (e.g. a key stroke)
    pub fn send_msg(&self, msg: CoreMsg) {
        trace!("{}", gettext("Pushing to tx queue"));
        self.queue_tx.lock().unwrap().push(msg);
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let shared_queue = SharedQueue {
        queue_rx: Arc::new(Mutex::new(Injector::<CoreMsg>::new())),
        queue_tx: Arc::new(Mutex::new(Injector::<CoreMsg>::new())),
    };

    let (xi_peer, rx) = xi_thread::start_xi_thread();
    let core = Core::new(xi_peer, rx, shared_queue.clone());

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

    //TODO: This part really needs better error handling...
    let (xi_config_dir, xi_config) = if let Some(user_config_dir) = dirs::config_dir() {
        let config_dir = user_config_dir.join("gxi");
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| {
                error!(
                    "{}: {}",
                    gettext("Failed to create the config dir"),
                    e.to_string()
                )
            })
            .unwrap();

        let mut xi_config = Config::<XiConfig>::new(
            config_dir
                .join("preferences.xiconfig")
                .to_str()
                .map(|s| s.to_string())
                .unwrap(),
        );

        xi_config = match xi_config.open() {
            Ok(_) => {
                let xi_config = xi_config.open().unwrap();
                /*
                We have to immediately save the config file here to "upgrade" it (as in add missing
                entries which have been added by us during a version upgrade
                */
                xi_config
                    .save()
                    .unwrap_or_else(|e| error!("{}", e.to_string()));

                xi_config.clone()
            }
            Err(_) => {
                error!(
                    "{}",
                    gettext("Couldn't read config, falling back to the default XI-Editor config")
                );
                xi_config
                    .save()
                    .unwrap_or_else(|e| error!("{}", e.to_string()));
                xi_config
            }
        };

        (
            config_dir.to_str().map(|s| s.to_string()).unwrap(),
            xi_config,
        )
    } else {
        error!(
            "{}",
            gettext("Couldn't determine home dir! Settings will be temporary")
        );

        let config_dir = tempfile::Builder::new()
            .prefix("gxi-config")
            .tempdir()
            .map_err(|e| {
                error!(
                    "{} {}",
                    gettext("Failed to create temporary config dir"),
                    e.to_string()
                )
            })
            .unwrap()
            .into_path();

        let xi_config = Config::<XiConfig>::new(
            config_dir
                .join("preferences.xiconfig")
                .to_str()
                .map(|s| s.to_string())
                .unwrap(),
        );
        xi_config
            .save()
            .unwrap_or_else(|e| error!("{}", e.to_string()));

        (
            config_dir.to_str().map(|s| s.to_string()).unwrap(),
            xi_config,
        )
    };

    application.connect_startup(clone!(shared_queue, core => move |application| {
        debug!("{}", gettext("Starting gxi"));

        core.client_started(&xi_config_dir, include_str!(concat!(env!("OUT_DIR"), "/plugin-dir.in")));

        let main_win = MainWin::new(
            application,
            &shared_queue,
            &Rc::new(RefCell::new(core.clone())),
            Arc::new(Mutex::new(xi_config.clone())),
        );

        let local = Worker::new_fifo();
        let mut cont_gtk = true;

        gtk::idle_add(clone!(shared_queue, main_win => move || {
            while let Some(msg) = local.pop().or_else(|| {
                std::iter::repeat_with(|| {
                    shared_queue.queue_rx.lock().unwrap().steal_batch_and_pop(&local)
                })
                .find(|s| !s.is_retry())
                .and_then(|s| s.success())
            }) {
                match msg {
                    CoreMsg::ShutDown { } => {
                        debug!("Shutdown receive");
                        cont_gtk = false;
                        },
                    _ => {
                        trace!("{}", gettext("Found a message for xi"));
                        MainWin::handle_msg(main_win.clone(), msg);
                    },
                }
            }
            gtk::Continue(cont_gtk)
        }));
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

    application.connect_shutdown(clone!(shared_queue => move |_| {
        debug!("{}", gettext("Shutting down..."));
        shared_queue.add_core_msg(
            CoreMsg::ShutDown {}
        )
    }));

    application.run(&args().collect::<Vec<_>>());
}
