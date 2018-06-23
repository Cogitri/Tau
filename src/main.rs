#![recursion_limit="128"]

extern crate cairo;
extern crate clap;
extern crate env_logger;
extern crate gdk;
extern crate gio;
#[macro_use] extern crate glib;
extern crate glib_sys;
extern crate gobject_sys;
extern crate gtk;
extern crate gtk_sys;
extern crate libc;
#[macro_use] extern crate log;
extern crate mio;
extern crate pango;
extern crate pangocairo;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate fontconfig;
extern crate xi_core_lib;
extern crate xi_rpc;

#[macro_use] mod macros;

mod clipboard;
mod edit_view;
mod scrollable_drawing_area;
mod linecache;
mod main_win;
mod prefs_win;
mod proto;
mod rpc;
mod source;
mod theme;
mod xi_thread;

use clap::{Arg, App, SubCommand};
use gio::{
    ApplicationExt,
    ApplicationExtManual,
    ApplicationFlags,
};
use glib::MainContext;
use gtk::{
    Application,
    GtkApplicationExt,
};
use main_win::MainWin;
use mio::unix::{PipeReader, PipeWriter, pipe};
use mio::TryRead;
use serde_json::Value;
use source::{SourceFuncs, new_source};
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
    Notification{method: String, params: Value},
    NewViewReply{file_name: Option<String>, value: Value},
}

pub struct SharedQueue {
    queue: VecDeque<CoreMsg>,
    pipe_writer: PipeWriter,
    pipe_reader: PipeReader,
}

impl SharedQueue {
    pub fn add_core_msg(&mut self, msg: CoreMsg)
    {
        if self.queue.is_empty() {
            self.pipe_writer.write_all(&[0u8])
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
        shared_queue.pipe_reader.try_read(&mut buf)
            .expect("failed to read signalling pipe");
        true
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

    let shared_queue = Arc::new(Mutex::new(SharedQueue{
        queue: queue.clone(),
        pipe_writer: writer,
        pipe_reader: reader,
    }));

    let application = Application::new("com.github.bvinc.gxi", ApplicationFlags::HANDLES_OPEN)
        .expect("failed to create gtk application");

    application.connect_startup(move |_|{
        debug!("startup");
    });

    application.connect_activate(move |application| {
        debug!("activate");
        let main_win = MainWin::new(application, shared_queue.clone());

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
    });
    application.connect_open(move |_,files,s| {
        debug!("open {:?} {}", files, s);
    });
    application.connect_shutdown(move |_| {
        debug!("shutdown");
    });

    application.run(&args().collect::<Vec<_>>());
}