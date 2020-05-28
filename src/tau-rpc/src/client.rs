// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

use crate::message::{Notification, Request, Response};
use crate::*;
use glib::clone;
use glib::source::Priority;
use glib::MainContext;
use glib::Receiver;
use log::*;
use pipe::{pipe, PipeReader, PipeWriter};
use serde_json::{self, from_value, json, to_vec, Value};
use std::cell::Cell;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use xi_core_lib::XiCore;
use xi_rpc::RpcLoop;

type XiSender = Mutex<PipeWriter>;
type XiReceiver = PipeReader;

pub trait Callback: Send {
    fn call(self: Box<Self>, result: Result<Value, Value>);
}

impl<F: FnOnce(Result<Value, Value>) + Send> Callback for F {
    fn call(self: Box<Self>, result: Result<Value, Value>) {
        (*self)(result)
    }
}

pub struct Client {
    sender: XiSender,
    pending_requests: Arc<Mutex<HashMap<u64, Box<dyn Callback>>>>,
    current_request_id: Cell<u64>,
}

impl Client {
    pub fn new() -> (Rc<Client>, Receiver<RpcOperations>) {
        let (mut receiver, sender) = Client::start_xi_thread();
        let client = Rc::new(Client {
            sender,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            current_request_id: Cell::new(0),
        });

        let (frontend_sender, frontend_receiver) =
            MainContext::channel::<RpcOperations>(Priority::default());

        thread::spawn(
            clone!(@weak client.pending_requests as pending_requests => @default-panic, move || {
                let mut buf = String::new();
                while receiver.read_line(&mut buf).is_ok() {
                    let msg = Message::decode(&buf).unwrap();
                    trace!("Received message from xi: {:?}", msg);
                    match msg {
                        Message::Request(res) => {
                            let Request { method, params, id } = res;
                                let operation = match method.as_str() {
                                    "measure_width" => {
                                        RpcOperations::MeasureWidth((id, from_value::<MeasureWidth>(params).unwrap()))
                                    }
                                    _ => {
                                        unreachable!("Unknown method {}", method);
                                    }
                                };
                                frontend_sender.send(operation).unwrap();
                        }
                        Message::Response(res) => {
                            let Response { id, result } = res;
                            if let Some(cb) = pending_requests.lock().unwrap().remove(&id) {
                                cb.call(result);
                            }
                        }
                        Message::Notification(res) => {
                            let Notification { method, params } = res;
                            let operation = match method.as_str() {
                                "update" => {
                                    RpcOperations::Update(from_value::<Update>(params).unwrap())
                                }
                                "scroll_to" => {
                                    RpcOperations::ScrollTo(from_value::<ScrollTo>(params).unwrap())
                                }
                                "def_style" => {
                                    RpcOperations::DefStyle(from_value::<Style>(params).unwrap())
                                }
                                "available_plugins" => {
                                    RpcOperations::AvailablePlugins(from_value::<AvailablePlugins>(params).unwrap())
                                }
                                "plugin_started" => {
                                    RpcOperations::PluginStarted(from_value::<PluginStarted>(params).unwrap())
                                }
                                "plugin_stopped" => {
                                    RpcOperations::PluginStopped(from_value::<PluginStopped>(params).unwrap())
                                }
                                "update_cmds" => {
                                    RpcOperations::UpdateCmds(from_value::<UpdateCmds>(params).unwrap())
                                }
                                "config_changed" => {
                                    RpcOperations::ConfigChanged(from_value::<ConfigChanged>(params).unwrap())
                                }
                                "theme_changed" => {
                                    RpcOperations::ThemeChanged(from_value::<ThemeChanged>(params).unwrap())
                                }
                                "alert" => {
                                    RpcOperations::Alert(from_value::<Alert>(params).unwrap())
                                }
                                "available_themes" => {
                                    RpcOperations::AvailableThemes(from_value::<AvailableThemes>(params).unwrap())
                                }
                                "find_status" => {
                                    RpcOperations::FindStatus(from_value::<FindStatus>(params).unwrap())
                                }
                                "replace_status" => {
                                    RpcOperations::ReplaceStatus(from_value::<ReplaceStatus>(params).unwrap())
                                }
                                "available_languages" => {
                                    RpcOperations::AvailableLanguages(from_value::<AvailableLanguages>(params).unwrap())
                                }
                                "language_changed" => {
                                    RpcOperations::LanguageChanged(from_value::<LanguageChanged>(params).unwrap())
                                }
                                _ => unreachable!("Unknown method {}", method),
                            };
                            frontend_sender.send(operation).unwrap();
                        }
                    }
                    buf.clear();
                }
            }),
        );

        (client, frontend_receiver)
    }

    fn start_xi_thread() -> (XiReceiver, XiSender) {
        let (to_core_rx, to_core_tx) = pipe();
        let (from_core_rx, from_core_tx) = pipe();
        let mut state = XiCore::new();
        let mut rpc_looper = RpcLoop::new(from_core_tx);
        thread::spawn(move || rpc_looper.mainloop(|| to_core_rx, &mut state));
        (from_core_rx, Mutex::new(to_core_tx))
    }

    fn send_notification(&self, method: &str, params: &Value) {
        let cmd = json!({
            "method": method,
            "params": params,
        });
        let mut sender = self.sender.lock().unwrap();
        debug!("Xi-CORE <-- {}", cmd);
        sender.write_all(&to_vec(&cmd).unwrap()).unwrap();
        sender.write_all(b"\n").unwrap();
        sender.flush().unwrap();
    }

    fn send_result(&self, id: u64, result: &Value) {
        let mut sender = self.sender.lock().unwrap();
        let cmd = json!({
            "id": id,
            "result": result,
        });
        debug!("Xi-CORE <-- result: {}", cmd);
        sender.write_all(&to_vec(&cmd).unwrap()).unwrap();
        sender.write_all(b"\n").unwrap();
        sender.flush().unwrap();
    }

    pub fn width_measured(&self, id: u64, widths: &[Vec<f32>]) {
        self.send_result(id, &serde_json::to_value(widths).unwrap());
    }

    /// Calls the callback with the result (from a different thread).
    fn send_request<F>(&self, method: &str, params: &Value, callback: F)
    where
        F: FnOnce(Result<Value, Value>) + Send + 'static,
    {
        let mut sender = self.sender.lock().unwrap();
        let cmd = json!({
            "method": method,
            "params": params,
            "id": self.current_request_id,
        });
        let id = { self.current_request_id.get() };
        debug!(
            "Xi-CORE <-- {{\"id\"={}, \"method\": {}, \"params\":{}}}",
            id, method, params
        );
        sender.write_all(&to_vec(&cmd).unwrap()).unwrap();
        sender.write_all(b"\n").unwrap();
        sender.flush().unwrap();
        self.pending_requests
            .lock()
            .unwrap()
            .insert(id, Box::new(callback));
        self.current_request_id.set(id + 1);
    }

    pub fn modify_user_config_domain_user_override(&self, view_id: ViewId, changes: &Value) {
        self.send_notification(
            "modify_user_config",
            &json!({
                "domain": { "user_override": view_id },
                "changes": changes,
            }),
        )
    }

    pub fn modify_user_config_domain(&self, domain: &str, changes: &Value) {
        self.send_notification(
            "modify_user_config",
            &json!({
                "domain": domain,
                "changes": changes,
            }),
        )
    }

    pub fn modify_user_config(&self, params: Value) {
        self.send_notification("modify_user_config", &params)
    }

    pub fn save(&self, view_id: ViewId, file_path: &str) {
        self.send_notification(
            "save",
            &json!({
                "view_id": view_id,
                "file_path": file_path,
            }),
        )
    }

    pub fn new_view<F>(&self, file_path: Option<&String>, callback: F)
    where
        F: FnOnce(Result<Value, Value>) + Send + 'static,
    {
        self.send_request(
            "new_view",
            &json!({
                "file_path": file_path,
            }),
            callback,
        );
    }

    pub fn close_view(&self, view_id: ViewId) {
        self.send_notification(
            "close_view",
            &json!({
                "view_id": view_id,
            }),
        )
    }

    fn send_edit_cmd(&self, view_id: ViewId, method: &str, params: &Value) {
        let edit_params = json!({
            "method": method,
            "params": params,
            "view_id": view_id,
        });
        self.send_notification("edit", &edit_params);
    }

    pub fn client_started(&self, config_dir: Option<&String>, client_extras_dir: Option<&String>) {
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

    /// Inserts the `chars` string at the current cursor location.
    pub fn insert(&self, view_id: ViewId, chars: &str) {
        self.send_edit_cmd(
            view_id,
            "insert",
            &json!({
                "chars": chars.to_string(),
            }),
        );
    }

    pub fn goto_line(&self, view_id: ViewId, line: u64) {
        self.send_edit_cmd(
            view_id,
            "insert",
            &json!({
                "line": line,
            }),
        );
    }

    pub fn resize(&self, view_id: ViewId, width: i32, height: i32) {
        self.send_edit_cmd(
            view_id,
            "resize",
            &json!({
                "width": width,
                "height": height,
            }),
        )
    }

    pub fn delete_forward(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "delete_forward", &json!({}))
    }
    pub fn delete_backward(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "delete_backward", &json!({}))
    }
    pub fn delete_word_backward(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "delete_word_backward", &json!({}))
    }
    pub fn insert_newline(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "insert_newline", &json!({}))
    }
    pub fn insert_tab(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "insert_tab", &json!({}))
    }
    pub fn outdent(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "outdent", &json!({}))
    }
    pub fn up(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_up", &json!({}))
    }
    pub fn down(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_down", &json!({}))
    }
    pub fn left(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_left", &json!({}))
    }
    pub fn right(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_right", &json!({}))
    }
    pub fn up_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_up_and_modify_selection", &json!({}))
    }
    pub fn down_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_down_and_modify_selection", &json!({}))
    }
    pub fn left_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_left_and_modify_selection", &json!({}))
    }
    pub fn right_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_right_and_modify_selection", &json!({}))
    }
    pub fn word_left(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_word_left", &json!({}))
    }
    pub fn word_right(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_word_right", &json!({}))
    }
    pub fn word_left_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_word_left_and_modify_selection", &json!({}))
    }
    pub fn word_right_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_word_right_and_modify_selection", &json!({}))
    }
    pub fn left_end(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_to_left_end_of_line", &json!({}))
    }
    pub fn right_end(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_to_right_end_of_line", &json!({}))
    }
    pub fn left_end_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(
            view_id,
            "move_to_left_end_of_line_and_modify_selection",
            &json!({}),
        )
    }
    pub fn right_end_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(
            view_id,
            "move_to_right_end_of_line_and_modify_selection",
            &json!({}),
        )
    }

    pub fn document_begin(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_to_beginning_of_document", &json!({}))
    }

    pub fn document_end(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_to_end_of_document", &json!({}))
    }

    pub fn document_begin_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(
            view_id,
            "move_to_beginning_of_document_and_modify_selection",
            &json!({}),
        )
    }

    pub fn document_end_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(
            view_id,
            "move_to_end_of_document_and_modify_selection",
            &json!({}),
        )
    }

    pub fn line_start(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_to_left_end_of_line", &json!({}))
    }

    pub fn line_start_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(
            view_id,
            "move_to_left_end_of_line_and_modify_selection",
            &json!({}),
        )
    }

    pub fn line_end(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "move_to_right_end_of_line", &json!({}))
    }

    pub fn line_end_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(
            view_id,
            "move_to_right_end_of_line_and_modify_selection",
            &json!({}),
        )
    }

    pub fn page_up(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "scroll_page_up", &json!({}))
    }

    pub fn page_down(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "scroll_page_down", &json!({}))
    }

    pub fn page_up_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "page_up_and_modify_selection", &json!({}))
    }

    pub fn page_down_sel(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "page_down_and_modify_selection", &json!({}))
    }

    pub fn select_all(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "select_all", &json!({}))
    }

    /// moves the cursor to a point (click)
    pub fn gesture_point_select(&self, view_id: ViewId, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": {
                    "select": {
                        "granularity": "point",
                        "multi": false,
                    },
                },
            }),
        )
    }
    /// adds or removes a selection at a point (new cursor)
    pub fn gesture_toggle_sel(&self, view_id: ViewId, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": {
                    "select_extend": {
                        "granularity": "point",
                    },
                },
            }),
        )
    }
    /// modifies the selection to include a point (shift+click)
    pub fn gesture_range_select(&self, view_id: ViewId, line: u64, col: u64) {
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
    pub fn gesture_line_select(&self, view_id: ViewId, line: u64, col: u64) {
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
    pub fn gesture_word_select(&self, view_id: ViewId, line: u64, col: u64) {
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
    pub fn gesture_multi_line_select(&self, view_id: ViewId, line: u64, col: u64) {
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
    pub fn gesture_multi_word_select(&self, view_id: ViewId, line: u64, col: u64) {
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
    pub fn scroll(&self, view_id: ViewId, first: u64, last: u64) {
        self.send_edit_cmd(view_id, "scroll", &json!([first, last]))
    }

    pub fn drag(&self, view_id: ViewId, line: u64, col: u64) {
        self.send_edit_cmd(
            view_id,
            "gesture",
            &json!({
                "line": line,
                "col": col,
                "ty": "drag",
            }),
        )
    }

    pub fn undo(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "undo", &json!({}))
    }
    pub fn redo(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "redo", &json!({}))
    }

    pub fn cut<F>(&self, view_id: ViewId, callback: F)
    where
        F: FnOnce(Result<Value, Value>) + Send + 'static,
    {
        self.send_request(
            "edit",
            &json!({
                "view_id": view_id,
                "method": "cut",
                "params:": &json!({}),
            }),
            callback,
        );
    }

    pub fn copy<F>(&self, view_id: ViewId, callback: F)
    where
        F: FnOnce(Result<Value, Value>) + Send + 'static,
    {
        self.send_request(
            "edit",
            &json!({
                "view_id": view_id,
                "method": "copy",
                "params:": &json!({}),
            }),
            callback,
        );
    }

    pub fn paste(&self, view_id: ViewId, chars: &str) {
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
    pub fn find(
        &self,
        view_id: ViewId,
        chars: &str,
        case_sensitive: bool,
        regex: bool,
        whole_words: bool,
    ) {
        self.send_edit_cmd(
            view_id,
            "find",
            &json!({
                "chars": chars,
                "case_sensitive": case_sensitive,
                "regex": regex,
                "whole_words": whole_words,
            }),
        )
    }

    pub fn find_next(
        &self,
        view_id: ViewId,
        wrap_around: Option<bool>,
        allow_same: Option<bool>,
        modify_selection: Option<ModifySelection>,
    ) {
        self.send_edit_cmd(
            view_id,
            "find_next",
            &json!({
                "wrap_around": wrap_around,
                "allow_same": allow_same,
                "modify_selection": modify_selection,
            }),
        )
    }
    pub fn find_previous(
        &self,
        view_id: ViewId,
        wrap_around: Option<bool>,
        allow_same: Option<bool>,
        modify_selection: Option<ModifySelection>,
    ) {
        self.send_edit_cmd(
            view_id,
            "find_previous",
            &json!({
                "wrap_around": wrap_around,
                "modify_selection": modify_selection,
                "allow_same": allow_same,
            }),
        )
    }

    /// Searches the document for `chars`, if present, falling back on
    /// the last selection region if `chars` is `None`.
    ///
    /// If `chars` is `None` and there is an active selection, returns
    /// the string value used for the search, else returns `Null`.
    pub fn find_other(
        &self,
        view_id: ViewId,
        wrap_around: bool,
        allow_same: bool,
        modify_selection: Option<ModifySelection>,
    ) {
        self.send_edit_cmd(
            view_id,
            "find_other",
            &json!(
                (json!({
                "wrap_around": wrap_around,
                "allow_same": allow_same,
                "modify_selection": modify_selection}))
            ),
        )
    }

    pub fn find_all(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "find_all", &json!({}))
    }

    pub fn highlight_find(&self, view_id: ViewId, visible: bool) {
        self.send_edit_cmd(
            view_id,
            "highlight_find",
            &json!({
                "visible": visible,
            }),
        )
    }

    pub fn replace(&self, view_id: ViewId, chars: &str, preserve_case: bool) {
        self.send_edit_cmd(
            view_id,
            "replace",
            &json!({
                "chars": chars,
                "preserve_case": preserve_case,
            }),
        )
    }

    pub fn replace_next(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "replace_next", &json!({}))
    }

    pub fn replace_all(&self, view_id: ViewId) {
        self.send_edit_cmd(view_id, "replace_all", &json!({}))
    }

    pub fn set_language(&self, view_id: ViewId, lang_name: &str) {
        self.send_notification(
            "set_language",
            &json!({ "view_id": view_id, "language_id": lang_name }),
        );
    }
}
