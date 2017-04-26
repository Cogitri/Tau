extern crate cairo;
extern crate env_logger;
extern crate gdk;
extern crate gdk_sys;
extern crate gtk;
extern crate gio;
extern crate glib;
extern crate gtk_sys;
extern crate gio_sys;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use std::process::{ChildStdout, ChildStderr, Command, Stdio};
use std::rc::Rc;
use std::thread;

mod document;
mod error;
mod key;
mod linecache;
mod request;
mod structs;
mod ui;
mod util;

use error::GxiError;
use ui::Ui;

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
                move |$(clone!(@param $p),)+| $body
        }
    );
}

// declare a new thread local storage key
thread_local!(
    static GLOBAL: RefCell<Option<Rc<RefCell<Ui>>>> = RefCell::new(None)
);

fn receive_json(line: &str) -> glib::Continue {
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        if let Err(e) = ui.borrow_mut().handle_line(line) {
            error!("Failed to handle xi-core line {}: {}", line, e);
        }
    });
    glib::Continue(false)
}

fn gxi_main() -> Result<(), GxiError> {

    let child = Command::new("xi-core").stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let child = match child {
        Ok(child) => child,
        Err(e) => return Err(GxiError::FailedToExec("xi-core".into(), e)),
    };

    let stdin = child.stdin.unwrap();
    let stdout = child.stdout.unwrap();
    let stderr = child.stderr.unwrap();

    GLOBAL.with(move |global| *global.borrow_mut() = Some(Ui::new(stdin)));
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        ui.borrow_mut().show_all();
        ui.borrow_mut().request_new_view();
    });


    thread::spawn(move || { core_read_thread(stdout); });
    thread::spawn(move || { core_read_stderr_thread(stderr); });

    gtk::main();

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResult {
    id: u64,
    result: String,
}

fn core_read_stderr_thread(stdout: ChildStderr) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(n) => {
                if n == 0 {
                    debug!("xi-core stderr finished");
                    break;
                }
            }
            Err(e) => {
                error!("Failed to read line: {}", e);
                break;
            }
        }
        error!("xi-core: {}", line);

        // Tell the main thread to process our new line
        {
            let line_clone = line.clone();
            glib::idle_add(move || receive_json(&line_clone));
        }
    }
}

fn core_read_thread(stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(n) => {
                if n == 0 {
                    debug!("xi-core finished");
                    break;
                }
            }
            Err(e) => {
                println!("Failed to read line: {}", e);
                break;
            }
        }

        // Tell the main thread to process our new line
        {
            let line_clone = line.clone();
            glib::idle_add(move || receive_json(&line_clone));
        }
    }

}

fn main() {
    env_logger::init().unwrap();

    if gtk::init().is_err() {
        error!("Failed to initialize GTK.");
        return;
    }

    if let Err(e) = gxi_main() {
        error!("{}", e);
    }
}
