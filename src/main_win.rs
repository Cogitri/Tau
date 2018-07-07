use gio::{
    ActionExt,
    ActionMapExt,
    ApplicationFlags,
    SimpleAction,
    SimpleActionExt,
};
use glib::variant::{FromVariant, Variant};
use gtk::*;
use CoreMsg;
use SharedQueue;
use edit_view::EditView;
use prefs_win::PrefsWin;
use proto::{self, ThemeSettings};
use rpc::{Core, Handler};
use serde_json::{self, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
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
    builder: Builder,
    views: BTreeMap<String, Rc<RefCell<EditView>>>,
    w_to_ev: HashMap<Widget, Rc<RefCell<EditView>>>,
    view_id_to_w: HashMap<String, Widget>,
    state: Rc<RefCell<MainState>>,
}

const GLADE_SRC: &str = include_str!("ui/gxi.glade");

impl MainWin {

    pub fn new(application: &Application, shared_queue: Arc<Mutex<SharedQueue>>, core: Rc<RefCell<Core>>) -> Rc<RefCell<MainWin>> {
        let glade_src = include_str!("ui/gxi.glade");
        let builder = Builder::new_from_string(glade_src);

        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();
        let notebook: Notebook = builder.get_object("notebook").unwrap();

        let main_win = Rc::new(RefCell::new(MainWin{
            core: core.clone(),
            shared_queue: shared_queue.clone(),
            window: window.clone(),
            notebook: notebook.clone(),
            builder: builder.clone(),
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
            let prefs_action = SimpleAction::new("prefs", None);
            prefs_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::prefs(main_win.clone());
            }));
            application.add_action(&prefs_action);
        }
        {
            let find_action = SimpleAction::new("find", None);
            find_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::find(main_win.clone());
            }));
            application.add_action(&find_action);
        }
        {
            let save_action = SimpleAction::new("save", None);
            save_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::handle_save_button(&main_win.clone());
            }));
            application.add_action(&save_action);
        }
        {
            let save_as_action = SimpleAction::new("save_as", None);
            save_as_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::current_save_as(&main_win.clone());
            }));
            application.add_action(&save_as_action);
        }
        {
            let close_action = SimpleAction::new("close", None);
            close_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::close(&main_win.clone());
            }));
            application.add_action(&close_action);
        }
        {
            let close_all_action = SimpleAction::new("close_all", None);
            close_all_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::close_all(&main_win.clone());
            }));
            application.add_action(&close_all_action);
        }
        {
            let quit_action = SimpleAction::new("quit", None);
            quit_action.connect_activate(clone!(main_win => move |_,_| {
                let main_win = main_win.borrow();
                main_win.window.destroy();
            }));
            application.add_action(&quit_action);
        }
        {
            let auto_indent_action = SimpleAction::new_stateful("auto_indent", None, &false.to_variant());;
            auto_indent_action.connect_change_state(clone!(main_win => move |action, value| {
                let mut main_win = main_win.borrow_mut();
                main_win.set_auto_indent(action, value);
            }));
            application.add_action(&auto_indent_action);
        }

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
                    "find_status" => main_win.borrow_mut().find_status(&params),
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
        if let Some(theme_name) = state.themes.first().map(Clone::clone) {
            state.theme_name = theme_name.clone();
            self.core.borrow().send_notification("set_theme", &json!({"theme_name": theme_name}));
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
        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() { return; }
            view_id.unwrap().to_string()
        };

        if let Some(ev) = self.views.get(&view_id) {
            ev.borrow_mut().config_changed(&params["changes"]);
        }
    }

    pub fn find_status(&mut self, params: &Value) {
        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() { return; }
            view_id.unwrap().to_string()
        };

        if let Some(ev) = self.views.get(&view_id) {
            ev.borrow_mut().find_status(&params["queries"]);
        }
    }

    pub fn set_auto_indent(&mut self, action: &SimpleAction, value: &Option<Variant>) {
        if value.is_none() { return; }
        if let Some(value) = value.as_ref() {
            action.set_state(value);
            let value: bool = value.get().unwrap();
            debug!("auto indent {}", value);
            self.core.borrow().modify_user_config(&json!("general"), &json!({"auto_indent": value}));
        }
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
                EditView::scroll_to(edit_view, line, col);
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

    pub fn handle_save_button(main_win: &Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view().clone();
        if edit_view.borrow().file_name.is_some() {
            let ev = edit_view.borrow_mut();
            let core = main_win.borrow().core.clone();
            core.borrow().save(&ev.view_id, ev.file_name.as_ref().unwrap());
        } else {
            MainWin::save_as(main_win, &edit_view);
        }
    }

    fn current_save_as(main_win: &Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view().clone();
        MainWin::save_as(main_win, &edit_view);
    }

    /// Display the FileChooserDialog, send the result to the Xi core.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn save_as(main_win: &Rc<RefCell<MainWin>>, edit_view: &Rc<RefCell<EditView>>) {
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

    fn prefs(main_win: Rc<RefCell<MainWin>>) {
        // let (main_state, core) = {
        //     let main_win = main_win.borrow();
        //     (main_win.state.clone(), main_win.core.clone())
        // };
        let main_win = main_win.borrow();
        let main_state = main_win.state.clone();
        let core = main_win.core.clone();
        let prefs_win = PrefsWin::new(&main_win.window, &main_state, &core);
        // prefs_win.run();
    }

    fn find(main_win: Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view().clone();
        edit_view.borrow().start_search();
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

                ev.close_button.connect_clicked(clone!(main_win, edit_view => move |_| {
                    MainWin::close_view(&main_win, &edit_view);
                }));
            }

            win.views.insert(view_id.to_string(), edit_view);
        }
    }

    fn close_all(main_win: &Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view();
        MainWin::close_view(&main_win, &edit_view);
    }

    fn close(main_win: &Rc<RefCell<MainWin>>) {
        let edit_view = main_win.borrow().get_current_edit_view();
        MainWin::close_view(&main_win, &edit_view);
    }

    fn close_view(main_win: &Rc<RefCell<MainWin>>, edit_view: &Rc<RefCell<EditView>>) {
        let pristine = edit_view.borrow().pristine;
        if !pristine {
            let builder = Builder::new_from_string(&GLADE_SRC);
            let ask_save_dialog: Dialog = builder.get_object("ask_save_dialog").unwrap();
            let ret = ask_save_dialog.run();
            ask_save_dialog.destroy();
            debug!("ask_save_dialog = {}", ret);
            match ret {
                1 => MainWin::save_as(main_win, edit_view),
                2 => return,
                _ => {},
            };
        }
        let view_id = edit_view.borrow().view_id.clone();
        let mut main_win = main_win.borrow_mut();
        if let Some(w) = main_win.view_id_to_w.get(&view_id).map(Clone::clone) {
            if let Some(page_num) = main_win.notebook.page_num(&w) {
                main_win.notebook.remove_page(Some(page_num));
            }
            main_win.w_to_ev.remove(&w.clone());
        }
        main_win.view_id_to_w.remove(&view_id);
        main_win.core.borrow().close_view(&view_id);
    }
}

