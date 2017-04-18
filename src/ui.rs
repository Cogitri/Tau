use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, ChildStdin, Command};
use std::rc::Rc;

use cairo::Context;
use cairo::enums::FontSlant;

use gdk::{Cursor, DisplayManager, EventKey, EventType, EventMask, SHIFT_MASK};
use gdk_sys::GdkCursorType;
use gtk;
use gtk::prelude::*;
use gtk::*;

use serde_json;
use serde_json::Value;

use xi_core_lib::rpc::Request;
use xi_core_lib::rpc::{EditCommand, TabCommand};

use error::GxiError;
use linecache::*;
use structs::*;
use util::*;
use GLOBAL;
use document::Document;

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

#[derive(Debug)]
pub struct Ui<'a> {
    rpc_index: usize,
    core_stdin: ChildStdin,
    pending: HashMap<usize, Request<'a>>,
    window: Window,
    new_button: Button,
    notebook: Notebook,
    view_to_idx: HashMap<String, u32>,
    da_to_view: HashMap<Layout, String>,
    sb_to_view: HashMap<Scrollbar, String>,
    view_to_doc: HashMap<String, Document>
}

impl Ui<'static> {
    pub fn new(core_stdin: ChildStdin) -> Rc<RefCell<Ui<'static>>> {
        let builder = Builder::new_from_file("resources/gxi.ui");
        let window: Window = builder.get_object("appwindow").unwrap();
        let notebook: Notebook = builder.get_object("notebook").unwrap();
        let new_button: Button = builder.get_object("new_button").unwrap();

        let ui = Rc::new(RefCell::new(Ui {
            rpc_index: 0,
            core_stdin: core_stdin,
            pending: HashMap::new(),
            window: window.clone(),
            new_button: new_button.clone(),
            notebook: notebook.clone(),
            view_to_idx: HashMap::new(),
            da_to_view: HashMap::new(),
            sb_to_view: HashMap::new(),
            view_to_doc: HashMap::new(),
        }));

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        new_button.connect_clicked(clone!(ui => move |_| {
            ui.borrow_mut().request_new_view();
        }));

        ui
    }

    /// Called when xi-core gives us a new line
    pub fn handle_line(&mut self, line: &str) -> Result<(), GxiError> {
        debug!(">>> {}", line);
        let json: Value = serde_json::from_str(line)?;
        //debug!("json: {:?}", json);
        let is_method = json.as_object().map_or(false, |dict|
            dict.contains_key("method"));
        if is_method {
            self.handle_method(json)?;
        } else {
            self.handle_response(json)?;
        }
        //match()
        Ok(())
    }

    pub fn handle_method(&mut self, json: Value) -> Result<(), GxiError> {
        let method = json.as_object().unwrap().get("method").unwrap().as_str().unwrap();
        let params = json.as_object().unwrap().get("params").unwrap();
        match method {
            "scroll_to" => {
                params
                .as_object()
                .ok_or_else(|| GxiError::MalformedMethodParams(method.to_string(), params.clone()))
                .and_then(|dict| {
                    debug!("dict={:?}", dict);
                    if let (Some(view_id), Some(line), Some(col)) =
                        (dict_get_string(dict, "view_id"),
                        dict_get_u64(dict, "line"),
                        dict_get_u64(dict, "col")) {
                            self.handle_scroll_to(view_id, line, col)
                    } else {Err(GxiError::MalformedMethodParams(method.to_string(), params.clone()))}
                })
            }
            "update" => {
                let p: UpdateParams = serde_json::from_value(params.clone())?;
                self.handle_update(&p.view_id, &p.update.ops)
            }
            _ => Err(GxiError::UnknownMethod(method.to_string()))
        }
        //Ok(())
    }

    pub fn handle_response(&mut self, mut response: Value) -> Result<(), GxiError> {
        let mut dict = response.as_object_mut().unwrap();
        let id = dict.get("id").and_then(Value::as_u64);
        if id.is_none() {
            return Err(GxiError::Custom("id missing from response, or is not u64".to_string()));
        }
        let id = id.unwrap() as usize;
        let result = dict.remove("result");
        let error = dict.remove("error");
        //let req = self.pending.remove(id);
        let req = match self.pending.remove(&id) {
            None => {return Err(GxiError::Custom(format!("Unexpected id: {}", id)));}
            Some(req) => req,
        };
        match req {
            Request::TabCommand{ tab_command } => match tab_command {
                TabCommand::NewTab => {
                    //if let Some(tab_name) = dict_get_string()
                    result
                    .ok_or_else(|| GxiError::Custom("No result on new tab".to_string()))
                    //.as_str()
                    .and_then(|result| {
                        if let Some(view_id) = result.as_str() {
                            self.response_new_tab(view_id)
                        } else {Err(GxiError::Custom("Unexpected result type on new view".to_string()))}
                    })

                },
                TabCommand::DeleteTab{ tab_name } => self.response_delete_view(tab_name),
                _ => Err(GxiError::Custom("Unexpected result".to_string()))

                // TabCommand::Edit{tab_name, edit_command} => match edit_command {
                //         EditCommand::RenderLines { first_line, last_line } => {},
                //         EditCommand::Key { chars, flags } => {},
                //         EditCommand::Insert { chars } => {},
                //         EditCommand::DeleteForward => {},
                //         EditCommand::DeleteBackward => {},
                //         EditCommand::DeleteToEndOfParagraph => {},
                //         EditCommand::DeleteToBeginningOfLine => {},
                //         EditCommand::InsertNewline => {},
                //         EditCommand::InsertTab => {},
                //         EditCommand::MoveUp => {},
                //         EditCommand::MoveUpAndModifySelection => {},
                //         EditCommand::MoveDown => {},
                //         EditCommand::MoveDownAndModifySelection => {},
                //         EditCommand::MoveLeft => {},
                //         EditCommand::MoveLeftAndModifySelection => {},
                //         EditCommand::MoveRight => {},
                //         EditCommand::MoveRightAndModifySelection => {},
                //         // EditCommand::MoveWordLeft => {},
                //         // EditCommand::MoveWordLeftAndModifySelection => {},
                //         // EditCommand::MoveWordRight => {},
                //         // EditCommand::MoveWordRightAndModifySelection => {},
                //         EditCommand::MoveToBeginningOfParagraph => {},
                //         EditCommand::MoveToEndOfParagraph => {},
                //         EditCommand::MoveToLeftEndOfLine => {},
                //         EditCommand::MoveToLeftEndOfLineAndModifySelection => {},
                //         EditCommand::MoveToRightEndOfLine => {},
                //         EditCommand::MoveToRightEndOfLineAndModifySelection => {},
                //         EditCommand::MoveToBeginningOfDocument => {},
                //         EditCommand::MoveToBeginningOfDocumentAndModifySelection => {},
                //         EditCommand::MoveToEndOfDocument => {},
                //         EditCommand::MoveToEndOfDocumentAndModifySelection => {},
                //         EditCommand::ScrollPageUp => {},
                //         EditCommand::PageUpAndModifySelection => {},
                //         EditCommand::ScrollPageDown => {},
                //         EditCommand::PageDownAndModifySelection => {},
                //         // EditCommand::SelectAll => {},
                //         EditCommand::Open { file_path } => {},
                //         EditCommand::Save { file_path } => {},
                //         EditCommand::Scroll { first, last } => {},
                //         // EditCommand::RequestLines { first, last } => {},
                //         EditCommand::Yank => {},
                //         EditCommand::Transpose => {},
                //         EditCommand::Click { line, column, flags, click_count } => {},
                //         EditCommand::Drag { line, column, flags } => {},
                //         EditCommand::Undo => {},
                //         EditCommand::Redo => {},
                //         EditCommand::Cut => {},
                //         EditCommand::Copy => {},
                //         EditCommand::DebugRewrap => {},
                //         EditCommand::DebugTestFgSpans => {},
                //         EditCommand::DebugRunPlugin => {},
                    // },
            }
        }
    }

    pub fn handle_update(&mut self, view_id: &str, ops: &Vec<UpdateOp>) -> Result<(), GxiError> {
        debug!("update: {:?}", ops);
        let mut doc = self.view_to_doc.get_mut(view_id).unwrap(); //FIXME error handling

        doc.handle_update(ops);

        let mut new_invalid_before = 0;
        let new_lines: Vec<Option<Line>> = Vec::new();
        let mut new_invalid_after = 0;

        for op in ops {
            // let op_type = op.op;
            let mut idx = 0;
            let mut n = op.n;
            // let mut old_ix = 0;
            // match op_type.as_ref() {
            //     "invalidate" => {
            //         if new_lines.len() == 0 {
            //             new_invalid_before += n;
            //         } else {
            //             new_invalid_after += n;
            //         }
            //     },
            //     "ins" => {
            //         for _ in 0..new_invalid_after {
            //             new_lines.push(None);
            //         }
            //         new_invalid_after = 0;
            //         let json_lines = op.lines.unwrap_or_else(Vec::new);
            //         for json_line in json_lines {
            //             new_lines.push(Some(Line{
            //                 cursor: json_line.cursor.unwrap_or_else(Vec::new),
            //                 text: json_line.text,
            //             }));
            //         }
            //     },
            //     "copy" | "update" => {
            //         let n_remaining = n;
            //         if old_ix < n_invalid_before {
            //
            //         }
            //     },
            //     "skip" => {
            //
            //     },
            //     _ => {
            //
            //     },
            // }



            // for update_line in op.lines.iter().flat_map(|l| l.iter()) {
            //     let mut cursor: Vec<usize> = Vec::new();
            //     if let Some(ref ul_cursor) = update_line.cursor {
            //         cursor.append(&mut ul_cursor.clone());
            //     }
            //     let line = Line{
            //         cursor: cursor,
            //         text: update_line.text.clone(),
            //     };
            //     doc.line_cache.insert(idx as u64, line);
            //     doc.drawing_area.queue_draw();
            //     idx += 1;
            // }
        }
        Ok(())
    }

    pub fn handle_scroll_to(&self, view_id: &str, line: u64, col: u64) -> Result<(), GxiError> {
        debug!("scroll_to {} {} {}", view_id, line, col);
        if let Some(idx) = self.view_to_idx.get(view_id) {
            self.notebook.set_current_page(Some(*idx));
        }
        Ok(())
    }

    pub fn show_all(&self) {
        self.window.show_all();
    }

    /// Build and send a JSON RPC request, returning the associated request ID to pair it with
    /// the response
    fn request(&mut self, method: &str, params: Value) -> usize {
        self.rpc_index += 1;
        let message = json!({
            "id": self.rpc_index,
            "method": method,
            "params": params,
        });
        self.send(&message);
        self.rpc_index
    }

    fn notify(&mut self, method: &str, params: Value) {
        let message = json!({
            "method": method,
            "params": params,
        });
        self.send(&message);
    }

    /// Serialize JSON object and send it to the server
    fn send(&mut self, message: &Value) {
        // debug!(">>> {:?}", message);
        let mut str_msg = serde_json::ser::to_string(&message).unwrap();
        debug!("<<< {}", str_msg);
        str_msg.push('\n');
        self.core_stdin.write_all(str_msg.as_bytes()).unwrap();
    }

    pub fn request_new_view(&mut self) {
        let req = Request::TabCommand{tab_command: TabCommand::NewTab};
        let id = self.request("new_view", json!({}));
        self.pending.insert(id, req);
    }

    pub fn request_delete_view(&mut self, view_id: &str) -> Result<(), GxiError> {
        Ok(())
    }

    pub fn response_delete_view(&mut self, view_id: &str) -> Result<(), GxiError> {
        Ok(())
    }

    pub fn response_new_tab(&mut self, view_id: &str) -> Result<(), GxiError> {
        let adj = Adjustment::new(0.0, 0.0, 3.0, 1.0, 2.0, 1.0);
        let scrolled_window = ScrolledWindow::new(None, None);
        let drawing_area = Layout::new(None, Some(&adj));
        //let ui = self.clone();
        debug!("events={:?}", drawing_area.get_events());
        //drawing_area.set_events(EventMask::all().bits() as i32);
        drawing_area.set_events(::gdk::BUTTON_PRESS_MASK.bits() as i32);
        debug!("events={:?}", drawing_area.get_events());
        drawing_area.set_can_focus(true);
        drawing_area.connect_button_press_event(|w,eb| {
            debug!("button press {:?}", eb);
            w.grab_focus();
            Inhibit(false)
        });
        drawing_area.connect_key_press_event(handle_key_press_event);
        // drawing_area.connect_key_release_event(|w,ek| {
        //     debug!("key release {:?}", ek);
        //     Inhibit(false)
        // });
        drawing_area.connect_draw(handle_draw);

        drawing_area.connect_size_allocate(|_,alloc| {
            debug!("Size changed to w={} h={}", alloc.width, alloc.height);
        });

        drawing_area.connect_realize(|w|{
            // Set the text cursor
            DisplayManager::get().get_default_display()
                .map(|disp| {
                    let cur = Cursor::new_for_display(&disp, GdkCursorType::Xterm);
                    w.get_window().map(|win| win.set_cursor(&cur));
            });
            w.set_size(1000,1000);
            w.grab_focus();
        });
        drawing_area.connect_scroll_event(|w,e|{
            debug!("scroll event {:?} {:?}", w, e);
            Inhibit(false)
        });

        scrolled_window.connect_scroll_child(|w,a,b| {
            debug!("crolled_window.connect_scroll_child {:?} {:?}", a, b);
            true
        });


        self.da_to_view.insert(drawing_area.clone(), view_id.to_owned());
        //self.sb_to_view.insert(scrollbar.clone(), view_id.to_owned());
        self.view_to_doc.insert(view_id.to_owned(), Document::new(drawing_area.clone()));
        scrolled_window.add(&drawing_area);
        let label = Label::new("Untitled");
        let view_label: Option<&Label> = Some(&label);
        let idx = self.notebook.insert_page(&scrolled_window, view_label, Some(0xffffffffu32));
        self.view_to_idx.insert(view_id.to_string(), idx);
        self.notebook.show_all();

        // self.notify("edit", json!({"method": "scroll",
        //     "view_id": view_id,
        //     "params": [0, 30],
        // }));

        //self.notify("scroll", json!([0, 30]));
        Ok(())
    }
}

///////////////////////////////////////////////////////////////////////////////
// Gtk Handler Functions
///////////////////////////////////////////////////////////////////////////////

fn handle_draw(w: &Layout, cr: &Context) -> Inhibit {
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui = ui.borrow_mut();
        let view_id = ui.da_to_view.get(w).unwrap().clone();

        // let missing = ui.view_to_doc.get_mut(&view_id).unwrap().line_cache.get_missing(0, 1);
        // debug!("MISSING={:?}", missing);
        // for run in missing {
        //     ui.notify("edit", json!({"method": "request_lines",
        //         "view_id": view_id,
        //         "params": [run.0, run.1],
        //     }));
        // }

        let doc = ui.view_to_doc.get_mut(&view_id).unwrap();
        doc.handle_draw(cr);
    });
    Inhibit(true)
}

fn handle_key_press_event(w: &Layout, ek: &EventKey) -> Inhibit {
    debug!("key press {:?}", ek);
    debug!("key press keyval={:?}, state={:?}, length={:?} group={:?} uc={:?}",
        ek.get_keyval(), ek.get_state(), ek.get_length(), ek.get_group(),
        ::gdk::keyval_to_unicode(ek.get_keyval())
    );
    GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
        let mut ui = ui.borrow_mut();
        let view_id = ui.da_to_view.get(&w.clone()).unwrap().clone();
        match ek.get_keyval() {
            65361 if ek.get_state().is_empty() => {
                ui.notify("edit", json!({"method": "move_left",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65362 if ek.get_state().is_empty() => {
                ui.notify("edit", json!({"method": "move_up",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65363 if ek.get_state().is_empty() => {
                ui.notify("edit", json!({"method": "move_right",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65364 if ek.get_state().is_empty() => {
                ui.notify("edit", json!({"method": "move_down",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65361 if ek.get_state() == SHIFT_MASK => {
                ui.notify("edit", json!({"method": "move_left_and_modify_selection",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65362 if ek.get_state() == SHIFT_MASK => {
                ui.notify("edit", json!({"method": "move_up_and_modify_selection",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65363 if ek.get_state() == SHIFT_MASK => {
                ui.notify("edit", json!({"method": "move_right_and_modify_selection",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            65364 if ek.get_state() == SHIFT_MASK => {
                ui.notify("edit", json!({"method": "move_down_and_modify_selection",
                    "view_id": view_id,
                    "params": [],
                }));
                return;
            }
            _ => {},
        };
        if let Some(ch) = ::gdk::keyval_to_unicode(ek.get_keyval()) {
            let mut ch = ch;
            if ch == '\r' {ch = '\n';}
            if ch == '\u{0008}' {
                ui.notify("edit", json!({"method": "delete_backward",
                    "view_id": view_id,
                    "params": [],
                }));
            } else {
                ui.notify("edit", json!({"method": "insert",
                    "view_id": view_id,
                    "params": {"chars":ch},
                }));
            }
        }
    });
    Inhibit(true)
}
