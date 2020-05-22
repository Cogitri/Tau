use crate::frontend::{TauFrontendBuilder, XiEvent, XiRequest};
use crate::main_win::MainWin;
use crossbeam_channel::{unbounded, Sender};
use futures::stream::Stream;
use futures::{future, future::Future};
use gio::prelude::{SettingsExt, SettingsExtManual};
use glib::{clone, MainContext, Receiver, SyncSender};
use gtk::Application;
use log::{debug, error, max_level as log_level, LevelFilter};
use serde_json::json;
use std::cell::RefCell;
use std::cmp::max;
use std::rc::Rc;
use tokio::runtime::Runtime;
use xrl::spawn as spawn_xi;
use xrl::{Client, ClientError, ViewId};

pub(crate) struct MainWinBuilder {
    application: Application,
    core: Option<Client>,
    event_rx: Option<Receiver<XiEvent>>,
    pub(crate) event_tx: SyncSender<XiEvent>,
    request_tx: Option<Sender<XiRequest>>,
    runtime: Rc<RefCell<Option<Runtime>>>,
}

impl MainWinBuilder {
    pub fn new(application: Application) -> MainWinBuilder {
        // The channel through which all events from Xi are sent from `crate::frontend::TauFrontend` to
        // the MainWin
        let (event_tx, event_rx) = MainContext::sync_channel::<XiEvent>(glib::PRIORITY_HIGH, 5);

        MainWinBuilder {
            application,
            event_tx,
            core: None,
            event_rx: Some(event_rx),
            request_tx: None,
            // Unwrapping and then putting it in Some might be odd at first sight,
            // but we want to blow up here in case things go wrong
            runtime: Rc::new(RefCell::new(Some(Runtime::new().unwrap()))),
        }
    }

    fn init_core(&mut self) {
        debug!("Initialising xi-core");

        // The channel to send the result of a request back to Xi
        let (request_tx, request_rx) = unbounded::<XiRequest>();

        self.request_tx = Some(request_tx);

        let mut runtime = self.runtime.borrow_mut().take().unwrap();

        let core_res = runtime.block_on(future::lazy(clone!(@strong self.event_tx as event_tx, @strong self.request_tx as request_tx => move || {
            let res = spawn_xi(
                crate::globals::XI_PATH.unwrap_or("xi-core"),
                TauFrontendBuilder {
                    request_rx,
                    event_tx: event_tx.clone(),
                    request_tx: request_tx.as_ref().unwrap().clone(),
                },
            );

            if let Ok((client, core_stderr)) = res {
                let xi_config_dir = std::env::var("XI_CONFIG_DIR").ok();
                let _ = client.client_started(xi_config_dir.as_deref(), crate::globals::PLUGIN_DIR);

                Ok((client, core_stderr))
            } else {
                Err(res.err().unwrap())
            }
        })));

        match core_res {
            Ok((core, core_stderr)) => {
                self.core = Some(core);
                runtime.spawn(
                    core_stderr
                        .for_each(|msg| {
                            if msg.contains("[ERROR]") {
                                println!("{}", msg)
                            } else if msg.contains("[WARN]") {
                                if log_level() >= LevelFilter::Warn {
                                    println!("{}", msg)
                                } else if log_level() >= LevelFilter::Info
                                    && msg.contains("deprecated")
                                {
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
            }
            Err(e) => {
                error!("Couldn't init xi-core due to error: {}", e);
                panic!();
            }
        }

        self.runtime.swap(&RefCell::new(Some(runtime)));
    }

    fn init_config(&self) {
        #[cfg(windows)]
        const LINE_ENDING: &str = "\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &str = "\n";

        debug!("Initialising user config");

        let gschema = gio::Settings::new("org.gnome.Tau");

        let tab_size = gschema.get::<u32>("tab-size");
        let autodetect_whitespace = gschema.get::<bool>("auto-indent");
        let translate_tabs_to_spaces = gschema.get::<bool>("translate-tabs-to-spaces");
        let use_tab_stops = gschema.get::<bool>("use-tab-stops");
        let word_wrap = gschema.get::<bool>("word-wrap");

        let font = gschema.get::<String>("font");
        let font_vec = font.split_whitespace().collect::<Vec<_>>();
        let (font_size, font_name) = if let Some((size, splitted_name)) = font_vec.split_last() {
            (size.parse::<f32>().unwrap_or(14.0), splitted_name.join(" "))
        } else {
            error!("Failed to get font configuration. Resetting...");
            gschema.reset("font");
            (14.0, "Monospace".to_string())
        };

        tokio::executor::current_thread::block_on_all(
            self.core.as_ref().unwrap().modify_user_config(
                "general",
                json!({
                    "tab_size": max(1, tab_size),
                    "autodetect_whitespace": autodetect_whitespace,
                    "translate_tabs_to_spaces": translate_tabs_to_spaces,
                    "font_face": font_name,
                    "font_size": if font_size.is_nan() {
                        14.0
                    } else if font_size < 6.0 {
                        6.0
                    } else if font_size > 72.0 {
                        72.0
                    } else { font_size },
                    "use_tab_stops": use_tab_stops,
                    "word_wrap": word_wrap,
                    "line_ending": LINE_ENDING,
                }),
            ),
        )
        .unwrap();

        let val = gschema.get_strv("syntax-config");

        for x in val {
            if let Ok(val) = serde_json::from_str(x.as_str()) {
                tokio::executor::current_thread::block_on_all(
                    self.core
                        .as_ref()
                        .unwrap()
                        .notify("modify_user_config", val),
                )
                .unwrap();
            } else {
                error!("Failed to deserialize syntax config. Resetting...");
                gschema.reset("syntax-config");
            }
        }
    }

    pub fn spawn_view(&self, file_path: Option<String>) -> Result<ViewId, ClientError> {
        debug!("Spawning view with filepath {:?}", file_path);

        tokio::executor::current_thread::block_on_all(future::lazy(|| {
            self.core
                .as_ref()
                .unwrap()
                .new_view(file_path.clone())
                .then(|res| {
                    future::lazy(move || {
                        if let Ok(view_id) = res {
                            self.event_tx
                                .send(XiEvent::NewView(Ok((view_id, file_path))))
                                .unwrap();
                        }
                        res
                    })
                })
        }))
    }

    pub fn build(&mut self) -> Rc<MainWin> {
        debug!("Building MainWin");

        self.init_core();
        self.init_config();

        MainWin::new(
            &self.application,
            self.core.clone().unwrap(),
            self.event_rx.take().unwrap(),
            self.event_tx.clone(),
            self.request_tx.clone().unwrap(),
            self.runtime.clone(),
        )
    }

    pub fn shutdown(&mut self) {
        debug!("Shutting down the runtime");

        self.runtime
            .borrow_mut()
            .take()
            .unwrap()
            .shutdown_now()
            .wait()
            .unwrap();
    }
}
