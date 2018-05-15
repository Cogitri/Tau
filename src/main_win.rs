use gio::ApplicationFlags;
use gtk::{
    Application,
    ApplicationWindow,
    Builder,
    Button,
    ButtonExt,
    DialogExt,
    FileChooserAction,
    FileChooserExt,
    FileChooserDialog,
    GtkWindowExt,
    Inhibit,
    Notebook,
    NotebookExtManual,
    Widget,
    WidgetExt,
};
use CoreMsg;
use SharedQueue;
use edit_view::EditView;
use rpc::{Core, Handler};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use syntect::highlighting::{Color, ThemeSettings, UnderlineOption};
use xi_thread;

const DEFAULT_THEME: ThemeSettings = ThemeSettings {
    foreground: Some(Color{r: 50, g: 50, b: 50, a: 255}),
    background: Some(Color::WHITE),
    caret: Some(Color{r: 50, g: 50, b: 50, a: 255}),
    line_highlight: Some(Color::BLACK),
    misspelling: Some(Color::BLACK),
    minimap_border: Some(Color::BLACK),
    accent: Some(Color::BLACK),
    popup_css: None,
    phantom_css: None,
    bracket_contents_foreground: Some(Color::BLACK),
    bracket_contents_options: Some(UnderlineOption::Underline),
    brackets_foreground: Some(Color::BLACK),
    brackets_background: None,
    brackets_options: Some(UnderlineOption::Underline),
    tags_foreground: Some(Color::BLACK),
    tags_options: Some(UnderlineOption::Underline),
    highlight: Some(Color::BLACK),
    find_highlight: Some(Color::BLACK),
    find_highlight_foreground: Some(Color{r: 50, g: 50, b: 50, a: 255}),
    gutter: Some(Color::WHITE),
    gutter_foreground: Some(Color{r: 179, g: 179, b: 179, a: 255}),
    selection: Some(Color::BLACK),
    selection_foreground: Some(Color::BLACK),
    selection_background: None,
    selection_border: Some(Color::WHITE),
    inactive_selection: Some(Color::BLACK),
    inactive_selection_foreground: Some(Color::BLACK),
    guide: Some(Color::BLACK),
    active_guide: Some(Color{r: 179, g: 179, b: 179, a: 255}),
    stack_guide: Some(Color::BLACK),
    highlight_foreground: Some(Color::BLACK),
    shadow: None,
};

pub struct MainWin {
    core: Rc<RefCell<Core>>,
    shared_queue: Arc<Mutex<SharedQueue>>,
    window: ApplicationWindow,
    notebook: Notebook,
    views: BTreeMap<String, Rc<RefCell<EditView>>>,
    w_to_view: HashMap<Widget, Rc<RefCell<EditView>>>,
    themes: Vec<String>,
    theme_settings: ThemeSettings,
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

        core.send_notification("client_started", &json!({}));

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
            w_to_view: Default::default(),
            themes: Default::default(),
            theme_settings: DEFAULT_THEME,
        }));

        window.set_application(application);

        window.connect_delete_event(clone!(window => move |_, _| {
            window.destroy();
            Inhibit(false)
        }));

        new_button.connect_clicked(clone!(main_win => move |_| {
            main_win.borrow_mut().req_new_view(None);
        }));
        open_button.connect_clicked(clone!(main_win => move |_| {
            main_win.borrow_mut().handle_open_button();
        }));
        save_button.connect_clicked(clone!(main_win => move |_| {
            main_win.borrow_mut().handle_save_button();
        }));

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
    pub fn handle_msg(&mut self, msg: CoreMsg) {
        match msg {
            CoreMsg::NewViewReply{file_name, value} => {
                self.new_view_response(file_name, value)
            },
            CoreMsg::Notification{ref method, ref params} => {
                match method.as_ref() {
                    "available_themes" => self.available_themes(params),
                    "available_plugins" => self.available_plugins(params),
                    "config_changed" => self.config_changed(params),
                    "update" => self.update(params),
                    "scroll_to" => self.scroll_to(params),
                    "theme_changed" => self.theme_changed(params),
                    _ => {
                        error!("!!! UNHANDLED NOTIFICATION: {}", method);
                    }
                };
            },
        };
    }
    pub fn available_themes(&mut self, params: &Value) {
        debug!("available_themes {:?}", params);
        self.themes.clear();
        if let Some(themes) = params["themes"].as_array() {
            for theme in themes {
                if let Some(theme) = theme.as_str() {
                    self.themes.push(theme.to_string());
                }
            }
        }
        if let Some(theme) = self.themes.first() {
            self.core.borrow().send_notification("set_theme", &json!({"theme_name": theme}));
        }
    }

    pub fn available_plugins(&mut self, params: &Value) {
        error!("UNHANDLED available_plugins {:?}", params);
    }

    pub fn config_changed(&mut self, params: &Value) {
        error!("UNHANDLED config_changed {:?}", params);
    }

    pub fn update(&mut self, params: &Value) {
        trace!("handling update {:?}", params);

        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() { return; }
            view_id.unwrap().to_string()
        };

        self.views.get(&view_id)
            .map(|ev| ev.borrow_mut().update(params));

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

    pub fn theme_changed(&mut self, params: &Value) {
        error!("UNHANDLED theme_changed {:?}", params);
    }


    pub fn handle_open_button(&self) {
        let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Open);
        fcd.set_transient_for(Some(&self.window));
        fcd.add_button("Open", 33);
        fcd.set_default_response(33);
        fcd.set_select_multiple(true);
        let response = fcd.run();
        debug!("open response = {}", response);
        if response == 33 {
            for file in fcd.get_filenames() {
                self.req_new_view(Some(&file.to_string_lossy()));
            }
        }
        fcd.destroy();
    }

    pub fn handle_save_button(&self) {
        let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Save);
        fcd.set_transient_for(Some(&self.window));
        fcd.add_button("Save", 33);
        fcd.set_default_response(33);
        let response = fcd.run();
        debug!("save response = {}", response);
        if response == 33 {
            for file in fcd.get_filename() {

                if let Some(idx) = self.notebook.get_current_page() {
                    if let Some(w) = self.notebook.get_nth_page(Some(idx)) {
                        if let Some(edit_view) = self.w_to_view.get(&w) {
                            debug!("saving {:?}", file);
                            let view_id = edit_view.borrow().view_id.clone();
                            let file = file.to_string_lossy();
                            self.core.borrow().save(&view_id, &file);
                            edit_view.borrow_mut().update_file(&file);
                        }
                    }
                }
            }
        }
        fcd.destroy();
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

    fn new_view_response(&mut self, file_name: Option<String>, value: Value) {
        if let Some(view_id) = value.as_str() {
            let edit_view = EditView::new(self.core.clone(), file_name, view_id.to_string());
            {
                {
                    let ev = edit_view.borrow();
                    let label = ev.label.clone();
                    let idx = self.notebook.insert_page(&ev.root_widget, Some(&label), None);
                    if let Some(w) = self.notebook.get_nth_page(Some(idx)) {
                        self.w_to_view.insert(w, edit_view.clone());
                    }
                }
            }

            self.views.insert(view_id.to_string(), edit_view);
        }
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
