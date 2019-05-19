use crossbeam_queue::SegQueue;
use gettextrs::gettext;
use log::trace;
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::Arc;

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

#[derive(Clone, Default)]
pub struct SharedQueue {
    pub queue_rx: Arc<Mutex<SegQueue<CoreMsg>>>,
    pub queue_tx: Arc<Mutex<SegQueue<CoreMsg>>>,
}

impl SharedQueue {
    pub fn new() -> Self {
        Self {
            queue_rx: Arc::new(Mutex::new(SegQueue::<CoreMsg>::new())),
            queue_tx: Arc::new(Mutex::new(SegQueue::<CoreMsg>::new())),
        }
    }

    /// A message from xi-editor that we have to process (e.g. that we should scroll)
    pub fn add_core_msg(&self, msg: CoreMsg) {
        trace!("{}: {:?}", gettext("Pushing message to rx queue"), msg);
        self.queue_rx.lock().push(msg);
    }
    /// A message that we want to send to xi-editor in order for it to process it (e.g. a key stroke)
    pub fn send_msg(&self, msg: CoreMsg) {
        trace!("{}: {:?}", gettext("Pushing message to tx queue"), msg);
        self.queue_tx.lock().push(msg);
    }
}
