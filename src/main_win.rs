use gio::{
    ActionExt,
    ActionMapExt,
    ApplicationFlags,
    SimpleAction,
    SimpleActionExt,
};
use gtk::*;
use CoreMsg;
use SharedQueue;
use edit_view::EditView;
use proto::{self, ThemeSettings};
use rpc::{Core, Handler};
use serde_json::{self, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::env::home_dir;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use theme::{Color, Style, Theme};
use xi_thread;

pub struct MainState {
    pub themes: Vec<String>,
    pub theme_name: String,
    pub theme: Theme,
    pub styles: Vec<Style>,
}

pub struct MainWin {
    core: Rc<RefCell<Core>>,
    shared_queue: Arc<Mutex<SharedQueue>>,
    window: ApplicationWindow,
    notebook: Notebook,
    views: BTreeMap<String, Rc<RefCell<EditView>>>,
    w_to_ev: HashMap<Widget, Rc<RefCell<EditView>>>,
    view_id_to_w: HashMap<String, Widget>,
    state: Rc<RefCell<MainState>>,
}

impl MainWin {

    pub fn new_application() -> Application {
        Application::new("com.github.bvinc.gxi", ApplicationFlags::FLAGS_NONE)
            .expect("failed to make application")
    }
    pub fn new(application: &Application, shared_queue: Arc<Mutex<SharedQueue>>) -> Rc<RefCell<MainWin>> {
        let (xi_peer, rx) = xi_thread::start_xi_thread();
        let handler = MyHandler::new(shared_queue.clone());
        let core = Core::new(xi_peer, rx, handler.clone());

        let mut config_dir = None;
        let mut plugin_dir = None;
        if let Some(home_dir) = home_dir() {
            let xi_config = home_dir.join(".config").join("xi");
            let xi_plugin = xi_config.join("plugins");
            config_dir = xi_config.to_str().map(|s| s.to_string());
            plugin_dir = xi_plugin.to_str().map(|s| s.to_string());
        }
        core.client_started(config_dir, plugin_dir);
        core.modify_user_config(&json!("general"), &json!({"auto_indent": true}));

        let glade_src = include_str!("gxi.glade");
        let builder = Builder::new_from_string(glade_src);

        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();
        let notebook: Notebook = builder.get_object("notebook").unwrap();
        let new_button: Button = builder.get_object("new_button").unwrap();
        let open_button: Button = builder.get_object("open_button").unwrap();
        let save_button: Button = builder.get_object("save_button").unwrap();

        notebook.remove_page(Some(0));

        let main_win = Rc::new(RefCell::new(MainWin{
            core: Rc::new(RefCell::new(core)),
            shared_queue: shared_queue.clone(),
            window: window.clone(),
            notebook: notebook.clone(),
            views: Default::default(),
            w_to_ev: Default::default(),
            view_id_to_w: Default::default(),
            state: Rc::new(RefCell::new(
                MainState{
                    themes: Default::default(),
                    theme_name: "default".to_string(),
                    theme: Default::default(),
                    styles: Default::default(),
                }
            ))
        }));


        window.set_application(application);

        window.connect_delete_event(clone!(window => move |_, _| {
            window.destroy();
            Inhibit(false)
        }));

        {
            let open_action = SimpleAction::new("open", None);
            open_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::handle_open_button(main_win.clone());
            }));
            application.add_action(&open_action);
        }
        {
            let new_action = SimpleAction::new("new", None);
            new_action.connect_activate(clone!(main_win => move |_,_| {
                main_win.borrow_mut().req_new_view(None);
            }));
            application.add_action(&new_action);
        }
        {
            let save_action = SimpleAction::new("save", None);
            save_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::handle_save_button(main_win.clone());
            }));
            application.add_action(&save_action);
        }
        {
            let save_as_action = SimpleAction::new("save_as", None);
            save_as_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::current_save_as(main_win.clone());
            }));
            application.add_action(&save_as_action);
        }
        {
            let close_action = SimpleAction::new("close", None);
            close_action.connect_activate(clone!(main_win => move |_,_| {
                let mut main_win = main_win.borrow_mut();
                main_win.close();
            }));
            application.add_action(&close_action);
        }
        {
            let quit_action = SimpleAction::new("quit", None);
            quit_action.connect_activate(clone!(main_win => move |_,_| {
                let main_win = main_win.borrow();
                main_win.window.destroy();
            }));
            application.add_action(&quit_action);
        }

        main_win.borrow_mut().req_new_view(None);

        window.show_all();

        main_win
    }
    pub fn activate(_application: &Application, _shared_queue: Arc<Mutex<SharedQueue>>) {
        // TODO
        unimplemented!();
    }
    pub fn open(_application: &Application, _shared_queue: Arc<Mutex<SharedQueue>>) {
        // TODO
        unimplemented!();
    }
}

impl MainWin {
    pub fn handle_msg(main_win: Rc<RefCell<MainWin>>, msg: CoreMsg) {
        match msg {
            CoreMsg::NewViewReply{file_name, value} => {
                MainWin::new_view_response(main_win, file_name, &value)
            },
            CoreMsg::Notification{method, params} => {
                match method.as_ref() {
                    "available_themes" => main_win.borrow_mut().available_themes(&params),
                    "available_plugins" => main_win.borrow_mut().available_plugins(&params),
                    "config_changed" => main_win.borrow_mut().config_changed(&params),
                    "def_style" => main_win.borrow_mut().def_style(&params),
                    "update" => main_win.borrow_mut().update(&params),
                    "scroll_to" => main_win.borrow_mut().scroll_to(&params),
                    "theme_changed" => main_win.borrow_mut().theme_changed(&params),
                    _ => {
                        error!("!!! UNHANDLED NOTIFICATION: {}", method);
                    }
                };
            },
        };
    }
    pub fn available_themes(&mut self, params: &Value) {
        let mut state = self.state.borrow_mut();
        state.themes.clear();
        if let Some(themes) = params["themes"].as_array() {
            for theme in themes {
                if let Some(theme) = theme.as_str() {
                    state.themes.push(theme.to_string());
                }
            }
        }
        if let Some(theme) = state.themes.first() {
            self.core.borrow().send_notification("set_theme", &json!({"theme_name": theme}));
        }
    }

    pub fn theme_changed(&mut self, params: &Value) {
        let theme_settings = params["theme"].clone();
        let theme_settings: ThemeSettings = match serde_json::from_value(theme_settings) {
            Err(e) => {
                error!("failed to convert theme settings: {}", e);
                return;
            }
            Ok(ts) => ts,
        };

        let selection_foreground = theme_settings.selection_foreground.map(Color::from_ts_proto);
        let selection = theme_settings.selection.map(Color::from_ts_proto);

        let theme = Theme::from_proto(&theme_settings);
        {
            let mut state = self.state.borrow_mut();
            state.theme = theme;
        }

        let selection_sytle = Style{
            fg_color: selection_foreground,
            bg_color: selection,
            weight: None,
            italic: None,
            underline: None,
        };

        self.set_style(0, selection_sytle);
    }

    pub fn available_plugins(&mut self, params: &Value) {
        error!("UNHANDLED available_plugins {}", params);
    }

    pub fn config_changed(&mut self, params: &Value) {
        error!("UNHANDLED config_changed {}", params);
    }

    pub fn def_style(&mut self, params: &Value) {
        let style: proto::Style = serde_json::from_value(params.clone()).unwrap();
        let style = Style::from_proto(&style);

        if let Some(id) = params["id"].as_u64() {
            let id = id as usize;
            
            self.set_style(id, style);
        }
    }

    pub fn set_style(&self, id: usize, style: Style) {
        let mut state = self.state.borrow_mut();
        // bump the array size up if needed
        while state.styles.len() < id {
            state.styles.push(Style{
                fg_color: None,
                bg_color: None,
                weight: None,
                italic: None,
                underline: None,
            })
        }
        if state.styles.len() == id {
            state.styles.push(style);
        } else {
            state.styles[id] = style;
        }
    }

    pub fn update(&mut self, params: &Value) {
        trace!("handling update {:?}", params);

        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() { return; }
            view_id.unwrap().to_string()
        };

        if let Some(ev) = self.views.get(&view_id) {
            ev.borrow_mut().update(params)
        }
    }

    pub fn scroll_to(&mut self, params: &Value) {
        trace!("handling scroll_to {:?}", params);
        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() { return; }
            view_id.unwrap().to_string()
        };

        let line = {
            match params["line"].as_u64() {
                None => return,
                Some(line) => line,
            }
        };

        let col = {
            match params["col"].as_u64() {
                None => return,
                Some(col) => col,
            }
        };

        match self.views.get(&view_id) {
            None => debug!("failed to find view {}", view_id),
            Some(edit_view) => {
                let idx = self.notebook.page_num(&edit_view.borrow().root_widget);
                self.notebook.set_current_page(idx);
                edit_view.borrow_mut().scroll_to(line, col);
            }
        }
    }

    /// Display the FileChooserDialog for opening, send the result to the Xi core.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    pub fn handle_open_button(main_win: Rc<RefCell<MainWin>>) {
        let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Open);
        fcd.set_transient_for(Some(&main_win.borrow().window.clone()));
        fcd.add_button("Open", 33);
        fcd.set_default_response(33);
        fcd.set_select_multiple(true);
        let response = fcd.run(); // Can call main loop, can't have any borrows out
        debug!("open response = {}", response);
        if response == 33 {
            let win = main_win.borrow();
            for file in fcd.get_filenames() {
                win.req_new_view(Some(&file.to_string_lossy()));
            }
        }
        fcd.destroy();
    }

    pub fn handle_save_button(main_win: Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view().clone();
        if edit_view.borrow().file_name.is_some() {
            let ev = edit_view.borrow_mut();
            let core = main_win.borrow().core.clone();
            core.borrow().save(&ev.view_id, ev.file_name.as_ref().unwrap());
        } else {
            MainWin::save_as(main_win, edit_view);
        }
    }

    fn current_save_as(main_win: Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view().clone();
        MainWin::save_as(main_win, edit_view);
    }

    /// Display the FileChooserDialog, send the result to the Xi core.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn save_as(main_win: Rc<RefCell<MainWin>>, edit_view: Rc<RefCell<EditView>>) {
        let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Save);
        fcd.set_transient_for(Some(&main_win.borrow().window.clone()));
        fcd.add_button("Save", 33);
        fcd.set_default_response(33);
        let response = fcd.run(); // Can call main loop, can't have any borrows out
        debug!("save response = {}", response);
        if response == 33 {
            let win = main_win.borrow();
            if let Some(file) = fcd.get_filename() {
                debug!("saving {:?}", file);
                let view_id = edit_view.borrow().view_id.clone();
                let file = file.to_string_lossy();
                win.core.borrow().save(&view_id, &file);
                edit_view.borrow_mut().set_file(&file);
            }
        }
        fcd.destroy();
    }

    fn get_current_edit_view(&self) -> Rc<RefCell<EditView>> {
        if let Some(idx) = self.notebook.get_current_page() {
            if let Some(w) = self.notebook.get_nth_page(Some(idx)) {
                if let Some(edit_view) = self.w_to_ev.get(&w) {
                    return edit_view.clone();
                }
            }
        }
        unreachable!("failed to get the current editview");
    }

    fn req_new_view(&self, file_name: Option<&str>) {
        let mut params = json!({});
        if let Some(file_name) = file_name {
            params["file_path"] = json!(file_name);
        }

        let shared_queue2 = self.shared_queue.clone();
        let file_name2 = file_name.map(|s| s.to_string());
        self.core.borrow_mut().send_request("new_view", &params,
            move |value| {
                let value = value.clone();
                let mut shared_queue = shared_queue2.lock().unwrap();
                shared_queue.add_core_msg(CoreMsg::NewViewReply{
                    file_name: file_name2,
                    value,
                })
            }
        );
    }

    fn new_view_response(main_win: Rc<RefCell<MainWin>>, file_name: Option<String>, value: &Value) {
        let mut win = main_win.borrow_mut();
        if let Some(view_id) = value.as_str() {
            let edit_view = EditView::new(win.state.clone(), win.core.clone(), file_name, view_id);
            {
                let ev = edit_view.borrow();
                let page_num = win.notebook.insert_page(&ev.root_widget, Some(&ev.tab_widget), None);
                if let Some(w) = win.notebook.get_nth_page(Some(page_num)) {
                    win.w_to_ev.insert(w.clone(), edit_view.clone());
                    win.view_id_to_w.insert(view_id.to_string(), w);
                }

                let view_id_clone = view_id.to_string();
                ev.close_button.connect_clicked(clone!(main_win => move |_| {
                    main_win.borrow_mut().close_view(&view_id_clone)
                }));
            }

            win.views.insert(view_id.to_string(), edit_view);
        }
    }

    fn close(&mut self) {
        let view_id = {
            let edit_view = self.get_current_edit_view();
            let edit_view = edit_view.borrow();
            edit_view.view_id.clone()
        };
        self.close_view(&view_id);
    }

    fn close_view(&mut self, view_id: &str) {
        if let Some(w) = self.view_id_to_w.get(view_id) {
            if let Some(page_num) = self.notebook.page_num(w) {
                self.notebook.remove_page(Some(page_num));
            }
            self.w_to_ev.remove(&w.clone());
        }
        self.view_id_to_w.remove(view_id);
        self.core.borrow().close_view(view_id);
    }
}

#[derive(Clone)]
struct MyHandler {
    shared_queue: Arc<Mutex<SharedQueue>>,
}

impl MyHandler {
    fn new(shared_queue: Arc<Mutex<SharedQueue>>) -> MyHandler {
        MyHandler {
            shared_queue,
        }
    }
}

impl Handler for MyHandler {
    fn notification(&self, method: &str, params: &Value) {
        debug!("CORE --> {{\"method\": \"{}\", \"params\":{}}}", method, params);
        let method2 = method.to_string();
        let params2 = params.clone();
        self.shared_queue.lock().unwrap().add_core_msg(
            CoreMsg::Notification{
                method: method2,
                params: params2
            }
        );
    }
}
