#![recursion_limit = "128"]
//Just for now...
#![allow(dead_code)]

#[macro_use]
mod macros;

mod clipboard;
mod edit_view;
mod errors;
mod linecache;
mod main_win;
mod pref_storage;
mod prefs_win;
mod proto;
mod rpc;
mod source;
mod theme;
mod xi_thread;

use crate::main_win::MainWin;
use crate::pref_storage::{GtkXiConfig, XiConfig};
use crate::rpc::{Core, Handler};
use crate::source::{new_source, SourceFuncs};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::MainContext;
use gtk::Application;
use log::{debug, error, trace};
use mio::unix::{pipe, PipeReader, PipeWriter};
use mio::TryRead;
use serde_json::{json, Value};
use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::env::args;
use std::io::Write;
use std::os::unix::io::AsRawFd;
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
}

pub struct SharedQueue {
    queue: VecDeque<CoreMsg>,
    pipe_writer: PipeWriter,
    pipe_reader: PipeReader,
}

impl SharedQueue {
    pub fn add_core_msg(&mut self, msg: CoreMsg) {
        if self.queue.is_empty() {
            self.pipe_writer
                .write_all(&[0u8])
                .expect("failed to write to signalling pipe");
        }
        trace!("pushing to queue");
        self.queue.push_back(msg);
    }
}

trait IdleCallback: Send {
    fn call(self: Box<Self>, a: &Any);
}

impl<F: FnOnce(&Any) + Send> IdleCallback for F {
    fn call(self: Box<F>, a: &Any) {
        (*self)(a)
    }
}

struct QueueSource {
    win: Rc<RefCell<MainWin>>,
    queue: Arc<Mutex<SharedQueue>>,
}

impl SourceFuncs for QueueSource {
    fn check(&self) -> bool {
        false
    }

    fn prepare(&self) -> (bool, Option<u32>) {
        (false, None)
    }

    fn dispatch(&self) -> bool {
        trace!("dispatch");
        let mut shared_queue = self.queue.lock().unwrap();
        while let Some(msg) = shared_queue.queue.pop_front() {
            trace!("found a msg");
            MainWin::handle_msg(self.win.clone(), msg);
        }
        let mut buf = [0u8; 64];
        shared_queue
            .pipe_reader
            .try_read(&mut buf)
            .expect("failed to read signalling pipe");
        true
    }
}

#[derive(Clone)]
struct MyHandler {
    shared_queue: Arc<Mutex<SharedQueue>>,
}

impl MyHandler {
    fn new(shared_queue: Arc<Mutex<SharedQueue>>) -> MyHandler {
        MyHandler { shared_queue }
    }
}

impl Handler for MyHandler {
    fn notification(&self, method: &str, params: &Value) {
        debug!(
            "CORE --> {{\"method\": \"{}\", \"params\":{}}}",
            method, params
        );
        let method2 = method.to_string();
        let params2 = params.clone();
        self.shared_queue
            .lock()
            .unwrap()
            .add_core_msg(CoreMsg::Notification {
                method: method2,
                params: params2,
            });
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let queue: VecDeque<CoreMsg> = Default::default();
    let (reader, writer) = pipe().unwrap();
    let reader_raw_fd = reader.as_raw_fd();

    let shared_queue = Arc::new(Mutex::new(SharedQueue {
        queue: queue.clone(),
        pipe_writer: writer,
        pipe_reader: reader,
    }));

    let (xi_peer, rx) = xi_thread::start_xi_thread();
    let handler = MyHandler::new(shared_queue.clone());
    let core = Core::new(xi_peer, rx, handler.clone());

    let application = Application::new("com.github.bvinc.gxi", ApplicationFlags::HANDLES_OPEN)
        .expect("failed to create gtk application");

    let mut xi_config_dir = None;
    let mut xi_config = XiConfig::new();
    let mut xi_config_file_path = None;
    let mut gxi_config = GtkXiConfig::new();
    let mut gxi_config_file_path = None;

    if let Some(user_config_dir) = dirs::config_dir() {
        let config_dir = user_config_dir.join("gxi");
        // The path to the main XI config
        let xi_main_config = config_dir.join("preferences.xiconfig");

        xi_config_file_path = xi_main_config.to_str().map(|s| s.to_string());
        xi_config = match xi_config.open(&xi_config_file_path.as_ref().unwrap()) {
            Ok(_) => xi_config
                .open(&xi_config_file_path.as_ref().unwrap())
                .unwrap(),
            Err(_) => {
                error!("Couldn't read config, falling back to default XI-Editor config!");
                xi_config
                    .save(&xi_config_file_path.as_ref().unwrap())
                    .unwrap_or_else(|e| error!("{}", e.to_string()));
                xi_config
            }
        };
        xi_config_dir = config_dir.to_str().map(|s| s.to_string());

        let gxi_main_config = config_dir.join("gxi.toml");
        gxi_config_file_path = gxi_main_config.to_str().map(|s| s.to_string());
        gxi_config = match gxi_config.open(&gxi_config_file_path.as_ref().unwrap()) {
            Ok(_) => gxi_config
                .open(&gxi_config_file_path.as_ref().unwrap())
                .unwrap(),
            Err(_) => {
                error!("Couldn't read config, falling back to default GXI config!");
                gxi_config
                    .save(&gxi_config_file_path.as_ref().unwrap())
                    .unwrap_or_else(|e| error!("{}", e.to_string()));
                gxi_config
            }
        };
    } else {
        error!("Couldn't determine home dir! Settings will be temporary!")
    }

    application.connect_startup(clone!(shared_queue, core => move |application| {
        debug!("startup");

        core.client_started(&xi_config_dir.clone(), include_str!(concat!(env!("OUT_DIR"), "/plugin-dir.in")));

        let main_win = MainWin::new(application, &shared_queue, &Rc::new(RefCell::new(core.clone())), Arc::new(Mutex::new(xi_config.clone())), xi_config_file_path.clone(), Arc::new(Mutex::new(gxi_config.clone())), gxi_config_file_path.clone());

        let source = new_source(QueueSource {
            win: main_win.clone(),
            queue: shared_queue.clone(),
        });
        unsafe {
            use glib::translate::ToGlibPtr;
            ::glib_sys::g_source_add_unix_fd(source.to_glib_none().0, reader_raw_fd, ::glib_sys::G_IO_IN);
        }
        let main_context = MainContext::default();
        source.attach(&main_context);
    }));

    application.connect_activate(clone!(shared_queue, core => move |_| {
        debug!("activate");

        let mut params = json!({});
        params["file_path"] = Value::Null;

        let shared_queue2 = shared_queue.clone();
        core.send_request("new_view", &params,
            move |value| {
                let mut shared_queue = shared_queue2.lock().unwrap();
                shared_queue.add_core_msg(CoreMsg::NewViewReply{
                    file_name: None,
                    value: value.clone(),
                })
            }
        );
    }));

    application.connect_open(clone!(shared_queue, core => move |_,files,_| {
        debug!("open");

        for file in files {
            let path = file.get_path();
            if path.is_none() { continue; }
            let path = path.unwrap();
            let path = path.to_string_lossy().into_owned();

            let mut params = json!({});
            params["file_path"] = json!(path);

            let shared_queue2 = shared_queue.clone();
            core.send_request("new_view", &params,
                move |value| {
                    let mut shared_queue = shared_queue2.lock().unwrap();
                    shared_queue.add_core_msg(CoreMsg::NewViewReply{
                        file_name: Some(path),
                    value: value.clone(),
                    })
                }
            );
        }
    }));
    application.connect_shutdown(move |_| {
        debug!("shutdown");
    });

    application.run(&args().collect::<Vec<_>>());
}
