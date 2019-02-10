use crossbeam_deque::Injector;
use gettextrs::gettext;
use log::trace;
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum CoreMsg {
    Notification {
        method: String,
        params: Value,
        id: Option<u64>,
    },
    NewViewReply {
        file_name: Option<String>,
        value: Value,
    },
}

pub struct ErrMsg {
    pub msg: String,
    pub fatal: bool,
}

#[derive(Clone)]
pub struct SharedQueue {
    pub queue_rx: Arc<Mutex<Injector<CoreMsg>>>,
    pub queue_tx: Arc<Mutex<Injector<CoreMsg>>>,
}

impl SharedQueue {
    pub fn new() -> SharedQueue {
        SharedQueue {
            queue_rx: Arc::new(Mutex::new(Injector::<CoreMsg>::new())),
            queue_tx: Arc::new(Mutex::new(Injector::<CoreMsg>::new())),
        }
    }

    /// A message from xi-editor that we have to process (e.g. that we should scroll)
    pub fn add_core_msg(&self, msg: CoreMsg) {
        trace!("{}: {:?}", gettext("Pushing message to rx queue"), msg);
        self.queue_rx.lock().unwrap().push(msg);
    }
    /// A message that we want to send to xi-editor in order for it to process it (e.g. a key stroke)
    pub fn send_msg(&self, msg: CoreMsg) {
        trace!("{}: {:?}", gettext("Pushing message to tx queue"), msg);
        self.queue_tx.lock().unwrap().push(msg);
    }
}
