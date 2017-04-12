use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Child, ChildStdin, Command};
use std::rc::Rc;

use cairo::enums::FontSlant;

use gdk::{Cursor, DisplayManager, EventType, EventMask};
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
pub struct Document {
    line_cache: LineCache,
    drawing_area: DrawingArea,
}

impl Document {
    pub fn new(da: DrawingArea) -> Document {
        Document {
            line_cache: LineCache::new(),
            drawing_area: da,
        }
    }
}

#[derive(Debug)]
pub struct Ui<'a> {
    rpc_index: usize,
    core_stdin: ChildStdin,
    pending: HashMap<usize, Request<'a>>,
    window: Window,
    new_button: Button,
    notebook: Notebook,
    tab_to_idx: HashMap<String, u32>,
    da_to_tab: HashMap<DrawingArea, String>,
    tab_to_doc: HashMap<String, Document>
}

impl Ui<'static> {
    pub fn new(core_stdin: ChildStdin) -> Rc<RefCell<Ui<'static>>> {
        let builder = Builder::new_from_file("gxi.ui");
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
            tab_to_idx: HashMap::new(),
            da_to_tab: HashMap::new(),
            tab_to_doc: HashMap::new(),
        }));

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        new_button.connect_clicked(clone!(ui => move |_| {
            ui.borrow_mut().request_new_tab();
        }));

        ui
    }

    /// Called when xi-core gives us a new line
    pub fn handle_line(&mut self, line: &str) -> Result<(), GxiError> {
        let json: Value = serde_json::from_str(line)?;
        debug!("json: {:?}", json);
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
                    if let (Some(tab), Some(line), Some(col)) =
                        (dict_get_string(dict, "tab"),
                        dict_get_u64(dict, "line"),
                        dict_get_u64(dict, "col")) {
                            self.handle_scroll_to(tab, line, col)
                    } else {Err(GxiError::MalformedMethodParams(method.to_string(), params.clone()))}
                })
            }
            //"update" => self.handle_update(),
            // "update" => {
            //     params
            //     .as_object()
            //     .ok_or_else(|| GxiError::MalformedMethodParams(method.to_string(), params.clone()))
            //     .and_then(|dict| {
            //         if let (Some(tab), Some(update)) =
            //         (dict_get_string(dict, "tab"), dict_get_dict(dict, "update")) {
            //             debug!("tab{}, update={:?}", tab, update);
            //             if let (Some(ops) = dict_get_)
            //             self.handle_update(tab, ops)
            //         } else {Err(GxiError::MalformedMethodParams(method.to_string(), params.clone()))}
            //     })
            // }
            "update" => {
                let p: UpdateParams = serde_json::from_value(params.clone())?;
                self.handle_update(&p.tab, &p.update.ops)
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
                        if let Some(tab_name) = result.as_str() {
                            self.response_new_tab(tab_name)
                        } else {Err(GxiError::Custom("Unexpected result type on new tab".to_string()))}
                    })

                },
                TabCommand::DeleteTab{ tab_name } => self.response_delete_tab(tab_name),
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

    pub fn handle_update(&mut self, tab: &str, ops: &Vec<UpdateOp>) -> Result<(), GxiError> {
        debug!("update: {:?}", ops);
        let mut new_invalid_before = 0;
        let new_lines: Vec<Option<Line>> = Vec::new();
        let mut new_invalid_after = 0;

        for op in ops {
            let mut doc = self.tab_to_doc.get_mut(tab).unwrap(); //FIXME error handling
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
            for update_line in op.lines.iter().flat_map(|l| l.iter()) {
                let mut cursor: Vec<usize> = Vec::new();
                if let Some(ref ul_cursor) = update_line.cursor {
                    cursor.append(&mut ul_cursor.clone());
                }
                let line = Line{
                    cursor: cursor,
                    text: update_line.text.clone(),
                };
                doc.line_cache.insert(idx as u64, line);
                doc.drawing_area.queue_draw();
                idx += 1;
            }
        }
        Ok(())
    }

    pub fn handle_scroll_to(&self, tab: &str, line: u64, col: u64) -> Result<(), GxiError> {
        debug!("scroll_to {} {} {}", tab, line, col);
        if let Some(idx) = self.tab_to_idx.get(tab) {
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

    fn notify(&mut self, method: &str, params: Value) -> usize {
        self.rpc_index += 1;
        let message = json!({
            "method": method,
            "params": params,
        });
        self.send(&message);
        self.rpc_index
    }

    /// Serialize JSON object and send it to the server
    fn send(&mut self, message: &Value) {
        debug!(">>> {:?}", message);
        let mut str_msg = serde_json::ser::to_string(&message).unwrap();
        str_msg.push('\n');
        self.core_stdin.write_all(str_msg.as_bytes()).unwrap();
    }

    pub fn request_new_tab(&mut self) {
        let req = Request::TabCommand{tab_command: TabCommand::NewTab};
        let id = self.request("new_tab", json!([]));
        self.pending.insert(id, req);
    }

    pub fn request_delete_tab(&mut self, tab_name: &str) -> Result<(), GxiError> {
        Ok(())
    }

    pub fn response_delete_tab(&mut self, tab_name: &str) -> Result<(), GxiError> {
        Ok(())
    }

    // fn get_ui() -> &mut Ui {
    //
    // }
    pub fn response_new_tab(&mut self, tab_name: &str) -> Result<(), GxiError> {
        let adj = Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 10.0);
        let scrollbar = Scrollbar::new(Orientation::Vertical, Some(&adj));
        let hbox = Box::new(Orientation::Horizontal, 0);
        let drawing_area = DrawingArea::new();
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
        drawing_area.connect_key_press_event(|w,ek| {
            //{"method":"edit","params": {"method": "insert", "params":{"chars":"A"}, "tab":"0"}}
            debug!("key press {:?}", ek);
            debug!("key press {:?}", ek.get_keyval());
            debug!("key press {:?}", w==w);
            debug!("key press {:?}", *w==w.clone());
            GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
                let mut ui = ui.borrow_mut();
                let tab_name = ui.da_to_tab.get(&w.clone()).unwrap().clone();
                let mut s = String::new();
                if let Some(ch) = ::gdk::keyval_to_unicode(ek.get_keyval()) {
                    let mut ch = ch;
                    if ch == '\r' {ch = '\n';}
                    //s.push(ch);
                    let id = ui.notify("edit", json!({"method": "insert",
                        "params": {"chars":ch},
                        "tab": tab_name,
                        }));
                }
            });
            Inhibit(false)
        });
        drawing_area.connect_key_release_event(|w,ek| {
            debug!("key release {:?}", ek);
            Inhibit(false)
        });
        drawing_area.connect_draw(
            |w,cr| {
                GLOBAL.with(|global| if let Some(ref mut ui) = *global.borrow_mut() {
                    let mut ui = ui.borrow_mut();
                    debug!("We're drawing");
                    let tab = ui.da_to_tab.get(w).unwrap();
                    let doc = ui.tab_to_doc.get(tab).unwrap();

                    cr.select_font_face("Mono", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
                    cr.set_font_size(12.0);
                    let font_extents = cr.font_extents();

                    // Draw background
                    cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
                    cr.rectangle(0.0, 0.0, w.get_allocated_width() as f64, w.get_allocated_height()  as f64);
                    cr.fill();

                    for i in 0..10u64 {
                        cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
                        if let Some(line) = doc.line_cache.get(i) {
                            cr.move_to(0.0, font_extents.height*((i+1) as f64));
                            cr.show_text(&line.text);

                            for c in &line.cursor {
                                cr.set_source_rgba(0.5, 0.5, 1.0, 1.0);
                                cr.rectangle(font_extents.max_x_advance* (*c as f64), font_extents.height*((i+1) as f64) - font_extents.ascent, 2.0, font_extents.ascent + font_extents.descent);
                                cr.fill();
                            }
                        }
                    }
                });
                Inhibit(false)
            }
        );

        drawing_area.connect_size_allocate(|_,alloc| {
            debug!("Size changed to w={} h={}", alloc.width, alloc.height);
        });

        // Set the text cursor
        drawing_area.connect_realize(|w|{
            DisplayManager::get().get_default_display()
                .map(|disp| {
                    let cur = Cursor::new_for_display(&disp, GdkCursorType::Xterm);
                    w.get_window().map(|win| win.set_cursor(&cur));
                });
        });
        drawing_area.connect_scroll_event(|w,e|{
            debug!("scroll event {:?} {:?}", w, e);
            Inhibit(false)
        });

        self.da_to_tab.insert(drawing_area.clone(), tab_name.to_owned());
        self.tab_to_doc.insert(tab_name.to_owned(), Document::new(drawing_area.clone()));
        hbox.pack_start(&drawing_area, true, true, 0);
        hbox.pack_start(&scrollbar, false, false, 0);
        let label = Label::new("Untitled");
        let tab_label: Option<&Label> = Some(&label);
        let idx = self.notebook.insert_page(&hbox, tab_label, Some(0xffffffffu32));
        self.tab_to_idx.insert(tab_name.to_string(), idx);
        self.notebook.show_all();
        Ok(())
    }
}
