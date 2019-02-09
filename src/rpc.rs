use crate::shared_queue::{CoreMsg, ErrMsg, SharedQueue};
use crate::xi_thread::XiPeer;
use gettextrs::gettext;
use log::{debug, error};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

pub const XI_SHIFT_KEY_MASK: u32 = 1 << 1;
pub const XI_CONTROL_KEY_MASK: u32 = 1 << 2;
pub const XI_ALT_KEY_MASK: u32 = 1 << 3;

#[derive(Clone)]
pub struct Core {
    pub state: Arc<Mutex<CoreState>>,
}

pub struct CoreState {
    pub xi_peer: XiPeer,
    pub id: u64,
    pub pending: BTreeMap<u64, Box<Callback>>,
}

pub trait Callback: Send {
    fn call(self: Box<Self>, result: &Value);
}

impl<F: FnOnce(&Value) + Send> Callback for F {
    fn call(self: Box<F>, result: &Value) {
        (*self)(result)
    }
}

impl Core {
    /// Sets up a new RPC connection, also starting a thread to receive
    /// responses.
    ///
    /// The handler is invoked for incoming RPC notifications. Note that
    /// it must be `Send` because it is called from a dedicated thread.
    pub fn new(
        xi_peer: XiPeer,
        xi_rx: Receiver<Value>,
        err_tx: glib::Sender<ErrMsg>,
        shared_queue: SharedQueue,
    ) -> Core {
        let state = CoreState {
            xi_peer,
            id: 0,
            pending: BTreeMap::new(),
        };
        let core = Core {
            state: Arc::new(Mutex::new(state)),
        };

        let rx_core = core.clone();
        thread::spawn(move || {
            while let Ok(msg) = xi_rx.recv() {
                debug!("{:?}", msg);
                if let Value::String(ref method) = msg["method"] {
                    shared_queue.add_core_msg(CoreMsg::Notification {
                        method: method.to_string(),
                        params: msg["params"].clone(),
                        id: msg["id"].as_u64(),
                    });
                } else if let Some(id) = msg["id"].as_u64() {
                    debug!(
                        "Xi-CORE --> {{\"method\": \"{}\", \"params\":{}}}",
                        &msg["method"], &msg["params"]
                    );
                    let callback = {
                        let mut state = rx_core.state.lock().unwrap();
                        state.pending.remove(&id)
                    };
                    if let Some(callback) = callback {
                        callback.call(&msg["result"]);
                    } else {
                        error!("{}: {:?}", gettext("unexpected result"), msg);
                    }
                } else {
                    error!("{} {:?} {}", gettext("Got"), msg, gettext("at RPC level"));
                }
            }

            //TODO: Gracefully handle xi-editor crashes
            err_tx
                .send(ErrMsg {
                    msg: "Xi-Editor has crashed!".to_string(),
                    fatal: true,
                })
                .unwrap();
        });

        core
    }

    pub fn send_notification(&self, method: &str, params: &Value) {
        let cmd = json!({
            "method": method,
            "params": params,
        });
        let state = self.state.lock().unwrap();
        debug!("Xi-CORE <-- {}", cmd);
        state.xi_peer.send_json(&cmd);
    }

    pub fn send_result(&self, id: u64, result: &Value) {
        let state = self.state.lock().unwrap();
        let cmd = json!({
            "id": id,
            "result": result,
        });
        debug!("Xi-CORE <-- result: {}", cmd);
        state.xi_peer.send_json(&cmd);
    }

    /// Calls the callback with the result (from a different thread).
    pub fn send_request<F>(&self, method: &str, params: &Value, callback: F)
    where
        F: FnOnce(&Value) + Send + 'static,
    {
        let mut state = self.state.lock().unwrap();
        let id = state.id;
        let cmd = json!({
            "method": method,
            "params": params,
            "id": id,
        });
        debug!(
            "Xi-CORE <-- {{\"id\"={}, \"method\": {}, \"params\":{}}}",
            id, method, params
        );
        state.xi_peer.send_json(&cmd);
        state.pending.insert(id, Box::new(callback));
        state.id += 1;
    }

    pub fn save(&self, view_id: &str, file_path: &str) {
        self.send_notification(
            "save",
            &json!({
                "view_id": view_id,
                "file_path": file_path,
            }),
        )
    }

    pub fn close_view(&self, view_id: &str) {
        self.send_notification(
            "close_view",
            &json!({
                "view_id": view_id,
            }),
        )
    }

    fn send_edit_cmd(&self, view_id: &str, method: &str, params: &Value) {
        let edit_params = json!({
            "method": method,
            "params": params,
            "view_id": view_id,
        });
        self.send_notification("edit", &edit_params);
    }

    pub fn client_started(&self, config_dir: &str, client_extras_dir: &str) {
        self.send_notification(
            "client_started",
            &json!({
                "config_dir": config_dir,
                "client_extras_dir": client_extras_dir,
            }),
        );
    }

    pub fn set_theme(&self, theme_name: &str) {
        self.send_notification("set_theme", &json!({ "theme_name": theme_name }));
    }

    pub fn request_lines(&self, view_id: &str, first_line: u64, last_line: u64) {
        self.send_edit_cmd(view_id, "request_lines", &json!([first_line, last_line]));
    }

    /// Inserts the `chars` string at the current cursor location.
    pub fn insert(&self, view_id: &str, chars: &str) {
        self.send_edit_cmd(
            view_id,
            "insert",
            &json!({
                "chars": chars.to_string(),
            }),
        );
    }

    pub fn resize(&self, view_id: &str, width: i32, height: i32) {
        self.send_edit_cmd(
            view_id,
            "resize",
            &json!({
                "width": width,
                "height": height,
            }),
        )
    }

    pub fn delete_forward(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "delete_forward", &json!({}))
    }
    pub fn delete_backward(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "delete_backward", &json!({}))
    }
    pub fn insert_newline(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "insert_newline", &json!({}))
    }
    pub fn insert_tab(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "insert_tab", &json!({}))
    }
    pub fn move_up(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_up", &json!({}))
    }
    pub fn move_down(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_down", &json!({}))
    }
    pub fn move_left(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_left", &json!({}))
    }
    pub fn move_right(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_right", &json!({}))
    }
    pub fn move_up_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_up_and_modify_selection", &json!({}))
    }
    pub fn move_down_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_down_and_modify_selection", &json!({}))
    }
    pub fn move_left_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_left_and_modify_selection", &json!({}))
    }
    pub fn move_right_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_right_and_modify_selection", &json!({}))
    }
    pub fn move_word_left(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_word_left", &json!({}))
    }
    pub fn move_word_right(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_word_right", &json!({}))
    }
    pub fn move_word_left_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_word_left_and_modify_selection", &json!({}))
    }
    pub fn move_word_right_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_word_right_and_modify_selection", &json!({}))
    }
    pub fn move_to_left_end_of_line(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_to_left_end_of_line", &json!({}))
    }
    pub fn move_to_right_end_of_line(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_to_right_end_of_line", &json!({}))
    }
    pub fn move_to_left_end_of_line_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(
            view_id,
            "move_to_left_end_of_line_and_modify_selection",
            &json!({}),
        )
    }
    pub fn move_to_right_end_of_line_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(
            view_id,
            "move_to_right_end_of_line_and_modify_selection",
            &json!({}),
        )
    }
    pub fn move_to_beginning_of_document(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_to_beginning_of_document", &json!({}))
    }
    pub fn move_to_end_of_document(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "move_to_end_of_document", &json!({}))
    }
    pub fn move_to_beginning_of_document_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(
            view_id,
            "move_to_beginning_of_document_and_modify_selection",
            &json!({}),
        )
    }
    pub fn move_to_end_of_document_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(
            view_id,
            "move_to_end_of_document_and_modify_selection",
            &json!({}),
        )
    }
    pub fn page_up(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "scroll_page_up", &json!({}))
    }
    pub fn page_down(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "scroll_page_down", &json!({}))
    }
    pub fn page_up_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "page_up_and_modify_selection", &json!({}))
    }
    pub fn page_down_and_modify_selection(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "page_down_and_modify_selection", &json!({}))
    }
    pub fn select_all(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "select_all", &json!({}))
    }

    /// moves the cursor to a point (click)
    pub fn gesture_point_select(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "point_select",
            }),
        )
    }
    /// adds or removes a selection at a point (new cursor)
    pub fn gesture_toggle_sel(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "toggle_sel",
            }),
        )
    }
    /// modifies the selection to include a point (shift+click)
    pub fn gesture_range_select(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "range_select",
            }),
        )
    }
    /// sets the selection to a given line (triple click)
    pub fn gesture_line_select(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "line_select",
            }),
        )
    }
    /// sets the selection to a given word (double click)
    pub fn gesture_word_select(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "word_select",
            }),
        )
    }
    /// adds a line to the selection
    pub fn gesture_multi_line_select(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "multi_line_select",
            }),
        )
    }
    /// adds a word to the selection
    pub fn gesture_multi_word_select(&self, view_id: &str, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "multi_word_select",
            }),
        )
    }

    /// Notifies the back-end of the visible scroll region, defined as the first and last
    /// (non-inclusive) formatted lines. The visible scroll region is used to compute movement
    /// distance for page up and page down commands, and also controls the size of the fragment
    /// sent in the `update` method.
    pub fn scroll(&self, view_id: &str, first: u64, last: u64) {
        self.send_edit_cmd(view_id, "scroll", &json!([first, last]))
    }

    pub fn drag(&self, view_id: &str, line: u64, col: u64, modifier: u32) {
        self.send_edit_cmd(view_id, "drag", &json!([line, col, modifier]))
    }

    pub fn undo(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "undo", &json!({}))
    }
    pub fn redo(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "redo", &json!({}))
    }

    pub fn cut(&mut self, view_id: &str) -> Option<String> {
        let (sender, receiver) = channel();

        self.send_request(
            "edit",
            &json!({
                "view_id": view_id,
                "method": "cut",
                "params:": &json!({}),
            }),
            move |value| {
                if let Some(selection) = value.as_str() {
                    sender.send(Some(selection.to_string())).unwrap();
                } else {
                    sender.send(None).unwrap();
                }
            },
        );

        receiver.recv().unwrap()
    }

    pub fn copy(&mut self, view_id: &str) -> Option<String> {
        let (sender, receiver) = channel();

        self.send_request(
            "edit",
            &json!({
                "view_id": view_id,
                "method": "copy",
                "params:": &json!({}),
            }),
            move |value| {
                if let Some(selection) = value.as_str() {
                    sender.send(Some(selection.to_string())).unwrap();
                } else {
                    sender.send(None).unwrap();
                }
            },
        );

        receiver.recv().unwrap()
    }

    pub fn paste(&self, view_id: &str, chars: &str) {
        self.send_edit_cmd(
            view_id,
            "paste",
            &json!({
                "chars": chars,
            }),
        )
    }

    /// Searches the document for `chars`, if present, falling back on
    /// the last selection region if `chars` is `None`.
    ///
    /// If `chars` is `None` and there is an active selection, returns
    /// the string value used for the search, else returns `Null`.
    pub fn find(&self, view_id: &str, chars: &str, case_sensitive: bool, regex: Option<bool>) {
        self.send_edit_cmd(
            view_id,
            "find",
            &json!({
                "chars": chars,
                "case_sensitive": case_sensitive,
                "regex": regex,
            }),
        )
    }
    pub fn find_next(&self, view_id: &str, wrap_around: Option<bool>, allow_same: Option<bool>) {
        self.send_edit_cmd(
            view_id,
            "find_next",
            &json!({
                "wrap_around": wrap_around,
                "allow_same": allow_same,
            }),
        )
    }
    pub fn find_previous(&self, view_id: &str, wrap_around: Option<bool>) {
        self.send_edit_cmd(
            view_id,
            "find_previous",
            &json!({
                "wrap_around": wrap_around,
            }),
        )
    }

    pub fn highlight_find(&self, view_id: &str, visible: bool) {
        self.send_edit_cmd(
            view_id,
            "highlight_find",
            &json!({
                "visible": visible,
            }),
        )
    }

    pub fn replace(&self, view_id: &str, chars: &str, preserve_case: bool) {
        self.send_edit_cmd(
            view_id,
            "replace",
            &json!({
                "chars": chars,
                "preserve_case": preserve_case,
            }),
        )
    }

    pub fn replace_next(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "replace_next", &json!({}))
    }

    pub fn replace_all(&self, view_id: &str) {
        self.send_edit_cmd(view_id, "replace_all", &json!({}))
    }

    pub fn set_language(&self, view_id: &str, lang_name: &str) {
        self.send_notification(
            "set_language",
            &json!({ "view_id": view_id, "language_id": lang_name }),
        );
    }
}
