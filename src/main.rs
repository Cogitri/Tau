#![recursion_limit = "128"]
//Just for now...
#![allow(dead_code)]

extern crate cairo;
extern crate clap;
extern crate env_logger;
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate glib_sys;
extern crate gobject_sys;
extern crate gtk;
extern crate gtk_sys;
extern crate libc;
#[macro_use]
extern crate log;
extern crate mio;
extern crate pango;
extern crate pangocairo;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate fontconfig;
extern crate xi_core_lib;
extern crate xi_rpc;

#[macro_use]
mod macros;

mod clipboard;
mod edit_view;
mod linecache;
mod main_win;
mod prefs_win;
mod proto;
mod rpc;
mod source;
mod theme;
mod xi_thread;

use clap::{App, Arg, SubCommand};
use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, FileExt};
use glib::MainContext;
use gtk::Application;
use main_win::MainWin;
use mio::unix::{pipe, PipeReader, PipeWriter};
use mio::TryRead;
use rpc::{Core, Handler};
use serde_json::Value;
use source::{new_source, SourceFuncs};
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
    env_logger::init();
    // let matches = App::new("gxi")
    //     .version("0.2.0")
    //     .author("brainn <brainn@gmail.com>")
    //     .about("Xi frontend")
    //     .arg(Arg::with_name("FILE")
    //         .multiple(true)
    //         .help("file to open")
    //     )
    //     .get_matches();

    // let mut files = vec![];
    // if matches.is_present("FILE") {
    //     files = matches.values_of("FILE").unwrap().collect::<Vec<_>>();
    // }
    // debug!("files {:?}", files);

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

    let mut config_dir = None;
    let mut plugin_dir = None;
    if let Some(home_dir) = dirs::home_dir() {
        let xi_config = home_dir.join(".config").join("xi");
        let xi_plugin = xi_config.join("plugins");
        config_dir = xi_config.to_str().map(|s| s.to_string());
        plugin_dir = xi_plugin.to_str().map(|s| s.to_string());
    }

    application.connect_startup(clone!(shared_queue, core => move |application| {
        debug!("startup");
        core.client_started(config_dir.clone(), plugin_dir.clone());

        let main_win = MainWin::new(application, shared_queue.clone(), Rc::new(RefCell::new(core.clone())));

        let source = new_source(QueueSource {
            win: main_win.clone(),
            queue: shared_queue.clone(),
        });
        unsafe {
            use glib::translate::ToGlibPtr;
            ::glib_sys::g_source_add_unix_fd(source.to_glib_none().0, reader_raw_fd, ::glib_sys::GIOCondition::IN);
        }
        let main_context = MainContext::default().expect("no main context");
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
