use crate::about_win::AboutWin;
use crate::edit_view::EditView;
use crate::errors::{ErrorDialog, ErrorMsg};
use crate::pref_storage::Config;
use crate::prefs_win::PrefsWin;
use crate::rpc::Core;
use crate::shared_queue::{CoreMsg, SharedQueue};
use crate::theme::{u32_from_color, LineStyle};
use gettextrs::gettext;
use gio::{ActionMapExt, SimpleAction};
use glib::MainContext;
use gtk::*;
use log::{debug, error, info, trace, warn};
use serde_derive::*;
use serde_json::{self, json, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::thread;
use syntect::highlighting::ThemeSettings;

/// Returned by an `ask_save_dialog` when we ask the user if he wants to either:
/// - `Save`(save unsaved changes and close view)
/// - `CloseWithoutSave` (discard pending changes and close view)
/// - `Cancel` (cancel the action and return to editing)
#[derive(Debug, PartialEq)]
enum SaveAction {
    Save = 100,
    CloseWithoutSave = 101,
    Cancel = 102,
}

impl SaveAction {
    fn from_i32(value: i32) -> Option<Self> {
        match value {
            100 => Some(SaveAction::Save),
            101 => Some(SaveAction::CloseWithoutSave),
            102 => Some(SaveAction::Cancel),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
pub struct MeasureWidth {
    pub id: u64,
    pub strings: Vec<String>,
}

#[derive(Default)]
pub struct MainState {
    pub themes: Vec<String>,
    pub theme_name: String,
    pub theme: ThemeSettings,
    pub styles: HashMap<usize, LineStyle>,
    pub fonts: Vec<String>,
    pub avail_languages: Vec<String>,
    pub selected_language: String,
    pub config: Config,
}

pub struct MainWin {
    core: Rc<RefCell<Core>>,
    shared_queue: SharedQueue,
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
    pub fn new(
        application: &Application,
        shared_queue: SharedQueue,
        core: Rc<RefCell<Core>>,
        config: Config,
    ) -> Rc<RefCell<Self>> {
        let glade_src = include_str!("ui/gxi.glade");
        let builder = Builder::new_from_string(glade_src);

        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();
        let notebook: Notebook = builder.get_object("notebook").unwrap();
        let syntax_combo_box: ComboBoxText = builder.get_object("syntax_combo_box").unwrap();

        let theme_name = crate::pref_storage::get_theme_schema();
        debug!("{}: {}", gettext("Theme name"), &theme_name);

        let main_state = Rc::new(RefCell::new(MainState {
            config,
            theme_name,
            ..Default::default()
        }));

        let main_win = Rc::new(RefCell::new(Self {
            core: core.clone(),
            shared_queue: shared_queue.clone(),
            window: window.clone(),
            notebook: notebook.clone(),
            builder: builder.clone(),
            views: Default::default(),
            w_to_ev: Default::default(),
            view_id_to_w: Default::default(),
            state: main_state.clone(),
        }));

        let (msg_tx, msg_rx) = MainContext::channel::<CoreMsg>(glib::PRIORITY_HIGH);
        let main_context = MainContext::default();
        main_context.acquire();

        thread::spawn(move || loop {
            if let Ok(msg) = shared_queue.queue_rx.lock().pop() {
                trace!("{}: {:?}", gettext("Found message in queue"), msg);
                msg_tx.send(msg).unwrap();
            }
        });

        msg_rx.attach(
            Some(&main_context),
            clone!(main_win => move |msg| {
                trace!("{}", gettext("Found a message from xi"));
                Self::handle_msg(main_win.clone(), msg);
                glib::source::Continue(true)
            }),
        );

        window.set_application(Some(application));

        //This is called when the window is closed with the 'X' or via the application menu, etc.
        window.connect_delete_event(clone!(main_win, window => move |_, _| {
            // Only destroy the window when the user has saved the changes or closes without saving
            if Self::close_all(main_win.clone()) == SaveAction::Cancel {
                debug!("{}", gettext("User chose to cancel exiting"));
                Inhibit(true)
            } else {
                debug!("{}", gettext("User chose to close the application"));
                window.destroy();
                Inhibit(false)
            }
        }));

        {
            let main_win = main_win.clone();

            syntax_combo_box.append_text(&gettext("Plain Text"));
            syntax_combo_box.set_active(Some(0));

            syntax_combo_box.connect_changed(move |cb| {
                if let Some(lang) = cb.get_active_text() {
                    let lang = lang.to_string();
                    // xi-editor doesn't know about the translations
                    let lang = if lang == gettext("Plain Text") {
                        "Plain Text".to_string()
                    } else {
                        lang
                    };
                    if let Some(ev) = main_win.borrow().get_current_edit_view() {
                        let core = &main_win.borrow().core;
                        trace!("{} {}", gettext("Setting language to"), &lang);
                        Self::set_language(&core, &ev.borrow().view_id, &lang);
                    }
                }
            });
        }
        {
            let open_action = SimpleAction::new("open", None);
            open_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'open' {}", gettext("Handling"), gettext("action"));
                Self::handle_open_button(&main_win);
            }));
            application.add_action(&open_action);
        }
        {
            let new_action = SimpleAction::new("new", None);
            new_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'new' {}", gettext("Handling"), gettext("action"));
                main_win.borrow_mut().req_new_view(None);
            }));
            application.add_action(&new_action);
        }
        {
            let prefs_action = SimpleAction::new("prefs", None);
            prefs_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'prefs' {}", gettext("Handling"), gettext("action"));
                Self::prefs(main_win.clone())
            }));
            application.add_action(&prefs_action);
        }
        {
            let about_action = SimpleAction::new("about", None);
            about_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'about' {}", gettext("Handling"), gettext("action"));
                Self::about(main_win.clone())
            }));
            application.add_action(&about_action);
        }
        {
            let find_action = SimpleAction::new("find", None);
            find_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'find' {}", gettext("Handling"), gettext("action"));
                Self::find(&main_win);
            }));
            application.add_action(&find_action);
        }
        {
            let replace_action = SimpleAction::new("replace", None);
            replace_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'replace' {}", gettext("Handling"), gettext("action"));
                Self::replace(&main_win);
            }));
            application.add_action(&replace_action);
        }
        {
            let save_action = SimpleAction::new("save", None);
            save_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'save' {}", gettext("Handling"), gettext("action"));
                Self::handle_save_button(&main_win.clone());
            }));
            application.add_action(&save_action);
        }
        {
            let save_as_action = SimpleAction::new("save_as", None);
            save_as_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'save_as' {}", gettext("Handling"), gettext("action"));
                Self::current_save_as(&main_win.clone());
            }));
            application.add_action(&save_as_action);
        }
        {
            let close_action = SimpleAction::new("close", None);
            close_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'close' {}", gettext("Handling"), gettext("action"));
                Self::close(&main_win.clone());
            }));
            application.add_action(&close_action);
        }
        {
            let close_all_action = SimpleAction::new("close_all", None);
            close_all_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'close_all' {}", gettext("Handling"), gettext("action"));
                Self::close_all(main_win.clone());
            }));
            application.add_action(&close_all_action);
        }
        {
            // This is called when we run app.quit, e.g. via Ctrl+Q
            let quit_action = SimpleAction::new("quit", None);
            quit_action.connect_activate(clone!(main_win => move |_,_| {
                trace!("{} 'quit' {}", gettext("Handling"), gettext("action"));
                // Same as in connect_destroy, only quit if the user saves or wants to close without saving
                if Self::close_all(main_win.clone()) == SaveAction::Cancel {
                    debug!("{}", gettext("User chose to not quit application"));
                } else {
                    debug!("{}", gettext("User chose to quit application"));
                    main_win.borrow().window.destroy();
                }
            }));
            application.add_action(&quit_action);
        }
        {
            let auto_indent_action = SimpleAction::new_stateful(
                "auto_indent",
                None,
                &main_state.borrow().config.config.auto_indent.to_variant(),
            );

            auto_indent_action.connect_change_state(clone!(main_state => move |action, value| {
                if let Some(value) = value.as_ref() {
                    action.set_state(value);
                    let value: bool = value.get().unwrap();
                    debug!("{}: {}", gettext("Auto indent"), value);
                    main_state.borrow_mut().config.config.auto_indent = value;
                    main_state.borrow().config.save()
                        .map_err(|e| error!("{}", e.to_string()))
                        .unwrap();
                }
            }));
            application.add_action(&auto_indent_action);
        }
        {
            let space_indent_action = SimpleAction::new_stateful(
                "insert_spaces",
                None,
                &main_state
                    .borrow()
                    .config
                    .config
                    .translate_tabs_to_spaces
                    .to_variant(),
            );;
            space_indent_action.connect_change_state(move |action, value| {
                if let Some(value) = value.as_ref() {
                    action.set_state(value);
                    let value: bool = value.get().unwrap();
                    debug!("{}: {}", gettext("Space indent"), value);
                    main_state
                        .borrow_mut()
                        .config
                        .config
                        .translate_tabs_to_spaces = value;
                    main_state
                        .borrow()
                        .config
                        .save()
                        .map_err(|e| error!("{}", e.to_string()))
                        .unwrap();
                }
            });
            application.add_action(&space_indent_action);
        }

        /* Put keyboard shortcuts here*/
        if let Some(app) = window.get_application() {
            app.set_accels_for_action("app.find", &["<Primary>f"]);
            app.set_accels_for_action("app.save", &["<Primary>s"]);
            app.set_accels_for_action("app.new", &["<Primary>n"]);
            app.set_accels_for_action("app.open", &["<Primary>o"]);
            app.set_accels_for_action("app.quit", &["<Primary>q"]);
            app.set_accels_for_action("app.replace", &["<Primary>r"]);
            app.set_accels_for_action("app.close", &["<Primary>w"]);
        }

        debug!("{}", gettext("Showing main window"));
        window.show_all();

        main_win
    }
    /*
    pub fn activate(_application: &Application, _shared_queue: Arc<Mutex<SharedQueue>>) {
        // TODO
        unimplemented!();
    }
    pub fn open(_application: &Application, _shared_queue: Arc<Mutex<SharedQueue>>) {
        // TODO
        unimplemented!();
    }
    */
}

impl MainWin {
    pub fn handle_msg(main_win: Rc<RefCell<Self>>, msg: CoreMsg) {
        trace!("{}: {:?}", gettext("Handling CoreMsg"), msg);
        match msg {
            CoreMsg::NewViewReply { file_name, value } => {
                Self::new_view_response(&main_win, file_name, &value)
            }
            CoreMsg::Notification { method, params, id } => {
                match method.as_ref() {
                    "alert" => main_win.borrow_mut().alert(&params),
                    "available_themes" => main_win.borrow_mut().available_themes(&params),
                    "available_plugins" => main_win.borrow_mut().available_plugins(&params),
                    "config_changed" => main_win.borrow_mut().config_changed(&params),
                    "def_style" => main_win.borrow_mut().def_style(&params),
                    "find_status" => main_win.borrow_mut().find_status(&params),
                    "replace_status" => main_win.borrow_mut().replace_status(&params),
                    "update" => main_win.borrow_mut().update(&params),
                    "scroll_to" => main_win.borrow_mut().scroll_to(&params),
                    "theme_changed" => main_win.borrow_mut().theme_changed(&params),
                    "measure_width" => main_win.borrow().measure_width(id, params),
                    "available_languages" => main_win.borrow_mut().available_languages(&params),
                    "language_changed" => main_win.borrow_mut().language_changed(&params),
                    "plugin_started" => main_win.borrow_mut().plugin_started(&params),
                    "plugin_stopped" => main_win.borrow_mut().plugin_stopped(&params),
                    _ => {
                        error!(
                            "{}: {}",
                            gettext("!!! UNHANDLED NOTIFICATION, PLEASE OPEN A BUGREPORT!"),
                            method
                        );
                    }
                };
            }
        };
    }

    pub fn alert(&self, params: &Value) {
        if let Some(msg) = params["msg"].as_str() {
            ErrorDialog::new(ErrorMsg {
                msg: msg.to_string(),
                fatal: false,
            });
        }
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

        if !state.themes.contains(&state.theme_name) {
            error!(
                "{} {} {}",
                gettext("Theme"),
                &state.theme_name,
                gettext("isn't available, setting to default"),
            );

            if let Some(theme_name) = state.themes.first().map(Clone::clone) {
                state.theme_name = theme_name.clone();
            } else {
                return;
            }
        }

        self.core
            .borrow()
            .send_notification("set_theme", &json!({ "theme_name": state.theme_name }));
    }

    pub fn theme_changed(&mut self, params: &Value) {
        let theme_settings = params["theme"].clone();
        let theme: ThemeSettings = match serde_json::from_value(theme_settings) {
            Err(e) => {
                error!("{}: {}", gettext("Failed to convert theme settings"), e);
                return;
            }
            Ok(ts) => ts,
        };

        // FIXME: Use annotations instead of constructing the selection style here
        let selection_style = LineStyle {
            fg_color: theme
                .selection_foreground
                .and_then(|s| Some(u32_from_color(s))),
            bg_color: theme.selection.and_then(|s| Some(u32_from_color(s))),
            weight: None,
            italic: None,
            underline: None,
        };

        let mut state = self.state.borrow_mut();
        state.theme = theme;
        state.styles.insert(0, selection_style);
    }

    pub fn available_plugins(&mut self, params: &Value) {
        let mut has_syntect = false;

        if let Some(available_plugins) = params["plugins"].as_array() {
            for x in available_plugins {
                if x["name"] == "xi-syntect-plugin" {
                    has_syntect = true;
                }
            }
        }

        if !has_syntect {
            ErrorDialog::new(ErrorMsg {
                msg: format!("{}: {:?}", gettext("Couldn't find syntect plugin, functionality will be limited! Only found the following plugins"), params["plugins"].as_array()),
                fatal: false,
            });
        }
    }

    pub fn config_changed(&mut self, params: &Value) {
        if let Some(ev) = params["view_id"].as_str().and_then(|id| self.views.get(id)) {
            ev.borrow_mut().config_changed(&params["changes"])
        }
    }

    pub fn find_status(&mut self, params: &Value) {
        if let Some(ev) = params["view_id"].as_str().and_then(|id| self.views.get(id)) {
            ev.borrow_mut().find_status(&params["queries"])
        }
    }

    pub fn replace_status(&mut self, params: &Value) {
        if let Some(ev) = params["view_id"].as_str().and_then(|id| self.views.get(id)) {
            ev.borrow_mut().replace_status(&params["status"])
        }
    }

    pub fn def_style(&mut self, params: &Value) {
        let style: LineStyle = serde_json::from_value(params.clone()).unwrap();

        if let Some(id) = params["id"].as_u64() {
            let mut state = self.state.borrow_mut();
            state.styles.insert(id as usize, style);
        }
    }

    pub fn update(&mut self, params: &Value) {
        trace!("{} 'update': {:?}", gettext("Handling"), params);

        if let Some(ev) = params["view_id"].as_str().and_then(|id| self.views.get(id)) {
            ev.borrow_mut().update(params)
        }
    }

    pub fn scroll_to(&mut self, params: &Value) {
        trace!("{} 'scroll_to' {:?}", gettext("Handling"), params);

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

        if let Some(ev) = params["view_id"].as_str().and_then(|id| self.views.get(id)) {
            let idx = self.notebook.page_num(&ev.borrow().root_widget);
            self.notebook.set_current_page(idx);
            ev.borrow_mut().scroll_to(line, col);
        }
    }

    fn plugin_started(&self, _params: &Value) {}

    fn plugin_stopped(&self, params: &Value) {
        if let Some(plugin) = params["plugin"].as_str() {
            let err_code = params["code"].as_u64();

            let err_msg = match err_code {
                Some(0) => gettext("has stopped due to an user-initiated exit"),
                Some(_) => format!(
                    "{} {}",
                    gettext("has crashed with error code"),
                    err_code.unwrap()
                ),
                None => gettext("has crashed"),
            };

            ErrorDialog::new(ErrorMsg {
                msg: format!(
                    "{} {} {} {}",
                    gettext("Plugin"),
                    plugin,
                    err_msg,
                    gettext("functionality will be limited")
                ),
                fatal: false,
            });
        }
    }

    pub fn measure_width(&self, id: Option<u64>, params: Value) {
        trace!(
            "{} 'measure_width' id: {:?} {:?}",
            gettext("Handling"),
            id,
            params
        );
        if let Some(ev) = self.get_current_edit_view() {
            let request: Vec<MeasureWidth> = serde_json::from_value(params).unwrap();

            let mut widths = Vec::new();

            for mes_width in &request {
                for string in &mes_width.strings {
                    widths.push(ev.borrow().line_width(string))
                }
            }
            //let widths: Vec<f64> = request.iter().map(|x| x.strings.iter().map(|v| edit_view.borrow().line_width(&v)).collect::<Vec<f64>>()).collect();

            if let Some(id) = id {
                self.core
                    .borrow()
                    .send_result(id, &serde_json::to_value(vec![widths]).unwrap());
            }
        }
    }

    pub fn available_languages(&mut self, params: &Value) {
        debug!("{} 'available_languages' {:?}", gettext("Handling"), params);
        let mut main_state = self.state.borrow_mut();
        main_state.avail_languages.clear();
        if let Some(languages) = params["languages"].as_array() {
            for lang in languages {
                if let Some(lang) = lang.as_str() {
                    main_state.avail_languages.push(lang.to_string());
                }
            }
        }
    }

    pub fn language_changed(&mut self, params: &Value) {
        debug!("{} 'language_changed' {:?}", gettext("Handling"), params);
        let lang_val = params["language_id"].clone();
        if let Some(lang) = lang_val.as_str() {
            let mut state = self.state.borrow_mut();
            state.selected_language = lang.to_string();
        }
    }

    pub fn set_language(core: &Rc<RefCell<Core>>, view_id: &str, lang: &str) {
        debug!("{} '{:?}'", gettext("Changing language to"), lang);
        core.borrow().set_language(&view_id, &lang);
    }

    /// Display the FileChooserNative for opening, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    pub fn handle_open_button(main_win: &Rc<RefCell<Self>>) {
        let fcn = FileChooserNative::new(
            Some(gettext("Open a file to edit").as_str()),
            Some(&main_win.borrow().window),
            FileChooserAction::Open,
            Some(gettext("Open").as_str()),
            Some(gettext("Cancel").as_str()),
        );
        fcn.set_transient_for(Some(&main_win.borrow().window.clone()));
        fcn.set_select_multiple(true);

        fcn.connect_response(clone!(main_win => move |fcd, res| {
            debug!(
                "{}: {:#?}",
                gettext("FileChooserNative open response"),
                res
            );

            if res == ResponseType::Accept {
                let win = main_win.borrow();
                for file in fcd.get_filenames() {
                    let file_str = &file.to_string_lossy().into_owned();
                    match &std::fs::File::open(file_str) {
                        Ok(_) => win.req_new_view(Some(&file_str)),
                        Err(e) => {
                            let err_msg = format!("{} '{}': {}", &gettext("Couldn't open file"), &file_str, &e.to_string());
                            ErrorDialog::new(ErrorMsg{msg: err_msg, fatal: false});
                        }
                    }
                }
            }
        }));

        fcn.run();
    }

    pub fn handle_save_button(main_win: &Rc<RefCell<Self>>) {
        if let Some(edit_view) = main_win.borrow().get_current_edit_view() {
            if edit_view.borrow().file_name.is_some() {
                let ev = edit_view.borrow_mut();
                let core = main_win.borrow().core.clone();
                core.borrow()
                    .save(&ev.view_id, ev.file_name.as_ref().unwrap());
            } else {
                Self::save_as(main_win, &edit_view);
            }
        }
    }

    fn current_save_as(main_win: &Rc<RefCell<Self>>) {
        if let Some(edit_view) = main_win.borrow().get_current_edit_view() {
            Self::save_as(main_win, &edit_view);
        }
    }

    /// Display the FileChooserNative, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn save_as(main_win: &Rc<RefCell<Self>>, edit_view: &Rc<RefCell<EditView>>) {
        let fcn = FileChooserNative::new(
            Some(gettext("Save file").as_str()),
            Some(&main_win.borrow().window),
            FileChooserAction::Save,
            Some(gettext("Save").as_str()),
            Some(gettext("Cancel").as_str()),
        );
        fcn.set_transient_for(Some(&main_win.borrow().window.clone()));
        fcn.set_current_name("");

        fcn.connect_response(clone!(edit_view, main_win => move |fcd, res| {
            debug!(
                "{}: {:#?}",
                gettext("FileChooserNative save response"),
                res
            );

            if res == ResponseType::Accept {
                let win = main_win.borrow();
                for file in fcd.get_filenames() {
                    let file_str = &file.to_string_lossy().into_owned();
                    if let Some(file) = fcd.get_filename() {
                        match &std::fs::OpenOptions::new().write(true).create(true).open(&file) {
                            Ok(_) => {
                                debug!("{} {:?}", gettext("Saving file"), &file);
                                let view_id = edit_view.borrow().view_id.clone();
                                let file = file.to_string_lossy();
                                win.core.borrow().save(&view_id, &file);
                                edit_view.borrow_mut().set_file(&file);
                            }
                        Err(e) => {
                            let err_msg = format!("{} '{}': {}", &gettext("Couldn't save file"), &file_str, &e.to_string());
                            ErrorDialog::new(ErrorMsg {msg: err_msg, fatal: false});
                        }
                    }
                }
            }
                }
        }));

        fcn.run();
    }

    fn prefs(main_win: Rc<RefCell<Self>>) {
        // let (main_state, core) = {
        //     let main_win = main_win.borrow();
        //     (main_win.state.clone(), main_win.core.clone())
        // };
        let main_win = main_win.borrow();
        let main_state = main_win.state.clone();
        let core = main_win.core.clone();
        PrefsWin::new(
            &main_win.window,
            &main_state,
            &core,
            main_win.get_current_edit_view(),
        );
    }

    fn about(main_win: Rc<RefCell<Self>>) {
        AboutWin::new(&main_win.borrow().window);
    }

    fn find(main_win: &Rc<RefCell<Self>>) {
        if let Some(edit_view) = main_win.borrow().get_current_edit_view() {
            edit_view.borrow().start_search();
        }
    }

    fn replace(main_win: &Rc<RefCell<Self>>) {
        if let Some(edit_view) = main_win.borrow().get_current_edit_view() {
            edit_view.borrow().start_replace();
        }
    }

    fn get_current_edit_view(&self) -> Option<Rc<RefCell<EditView>>> {
        if let Some(idx) = self.notebook.get_current_page() {
            if let Some(w) = self.notebook.get_nth_page(Some(idx)) {
                if let Some(edit_view) = self.w_to_ev.get(&w) {
                    return Some(edit_view.clone());
                }
            }
        }
        info!("{}", gettext("Couldn't get current EditView. This may only mean that you don't have an editing tab open right now."));
        None
    }

    fn req_new_view(&self, file_name: Option<&str>) {
        trace!("{}", gettext("Requesting new view"));
        let mut params = json!({});
        if let Some(file_name) = file_name {
            params["file_path"] = json!(file_name);
        }

        let shared_queue = self.shared_queue.clone();
        let file_name2 = file_name.map(std::string::ToString::to_string);
        self.core
            .borrow_mut()
            .send_request("new_view", &params, move |value| {
                let value = value.clone();
                shared_queue.add_core_msg(CoreMsg::NewViewReply {
                    file_name: file_name2,
                    value,
                })
            });
    }

    fn new_view_response(main_win: &Rc<RefCell<Self>>, file_name: Option<String>, value: &Value) {
        trace!("{}", gettext("Creating new EditView"));
        let mut old_ev = None;
        {
            let mut win = main_win.borrow_mut();

            // Add all available langs to the syntax_combo_box for the user to select it. We're doing
            // it here because we can be sure that xi-editor has sent available_languages by now.
            let syntax_combo_box: ComboBoxText =
                win.builder.get_object("syntax_combo_box").unwrap();

            win.state
                .borrow()
                .avail_languages
                .iter()
                .filter(|l| l != &&"Plain Text".to_string())
                .for_each(|lang| syntax_combo_box.append_text(lang));

            if let Some(view_id) = value.as_str() {
                let position = if let Some(curr_ev) = win.get_current_edit_view() {
                    if curr_ev.borrow().is_empty() {
                        old_ev = Some(curr_ev.clone());
                        if let Some(w) = win.view_id_to_w.get(&curr_ev.borrow().view_id) {
                            win.notebook.page_num(w)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let hamburger_button = win.builder.get_object("hamburger_button").unwrap();
                let edit_view = EditView::new(
                    &win.state,
                    &win.core,
                    &hamburger_button,
                    file_name,
                    view_id,
                    &win.window,
                );
                {
                    let ev = edit_view.borrow();
                    let page_num = win.notebook.insert_page(
                        &ev.root_widget,
                        Some(&ev.top_bar.tab_widget),
                        position,
                    );
                    if let Some(w) = win.notebook.get_nth_page(Some(page_num)) {
                        win.w_to_ev.insert(w.clone(), edit_view.clone());
                        win.view_id_to_w.insert(view_id.to_string(), w);
                    }

                    ev.top_bar.close_button.connect_clicked(
                        clone!(main_win, edit_view => move |_| {
                            Self::close_view(&main_win, &edit_view);
                        }),
                    );
                }

                win.views.insert(view_id.to_string(), edit_view);
            }
        }
        if let Some(empty_ev) = old_ev {
            Self::close_view(&main_win, &empty_ev);
        }
    }

    fn close_all(main_win: Rc<RefCell<Self>>) -> SaveAction {
        trace!("{}", gettext("Closing all EditViews"));
        // Get all views that we currently have opened
        let views = {
            let main_win = main_win.borrow();
            main_win.views.clone()
        };
        // Close each one of them
        let actions: Vec<SaveAction> = views
            .iter()
            .map(|(_, ev)| {
                let save_action = Self::close_view(&main_win.clone(), &ev);
                if save_action != SaveAction::Cancel {
                    main_win.borrow_mut().views.remove(&ev.borrow().view_id);
                }
                save_action
            })
            .collect();

        // If the user _doesn't_ want us to close one of the Views (because its not pristine he chose
        // 'cancel' we want to return SaveAction::Cancel, so that connect_destroy and quit do
        // not close the entire application and as such the EditView.
        let mut cancel = false;

        actions.iter().for_each(|action| {
            if let SaveAction::Cancel = action {
                cancel = true
            }
        });

        if cancel {
            SaveAction::Cancel
        } else {
            SaveAction::CloseWithoutSave
        }
    }

    fn close(main_win: &Rc<RefCell<Self>>) -> SaveAction {
        trace!("{}", gettext("Closing current Editview"));
        if let Some(edit_view) = main_win.borrow().get_current_edit_view() {
            Self::close_view(&main_win, &edit_view)
        } else {
            SaveAction::Cancel
        }
    }

    fn close_view(main_win: &Rc<RefCell<Self>>, edit_view: &Rc<RefCell<EditView>>) -> SaveAction {
        trace!(
            "{} {}",
            gettext("Closing Editview"),
            edit_view.borrow().view_id
        );
        let pristine = edit_view.borrow().pristine;
        let save_action = if pristine {
            // If it's pristine we don't ask the user if he really wants to quit because everything
            // is saved already and as such always close without saving
            SaveAction::CloseWithoutSave
        } else {
            // Change the tab to the EditView we want to ask the user about saving to give him a
            // change to review that action
            if let Some(w) = main_win
                .borrow()
                .view_id_to_w
                .get(&edit_view.borrow().view_id)
                .map(Clone::clone)
            {
                if let Some(page_num) = main_win.borrow().notebook.page_num(&w) {
                    main_win
                        .borrow()
                        .notebook
                        .set_property_page(page_num as i32);
                }
            }

            let ask_save_dialog = MessageDialog::new(
                Some(&main_win.borrow().window),
                DialogFlags::all(),
                MessageType::Question,
                ButtonsType::None,
                gettext("Save unsaved changes").as_str(),
            );
            ask_save_dialog.add_button(
                &gettext("Close Without Saving"),
                ResponseType::Other(SaveAction::CloseWithoutSave as u16),
            );
            ask_save_dialog.add_button(
                &gettext("Cancel"),
                ResponseType::Other(SaveAction::Cancel as u16),
            );
            ask_save_dialog.add_button(
                &gettext("Save"),
                ResponseType::Other(SaveAction::Save as u16),
            );
            ask_save_dialog.set_default_response(ResponseType::Other(SaveAction::Cancel as u16));
            let ret = ask_save_dialog.run();
            ask_save_dialog.destroy();
            match SaveAction::from_i32(ret.into()) {
                Some(SaveAction::Save) => {
                    Self::handle_save_button(main_win);
                    SaveAction::Save
                }
                Some(SaveAction::CloseWithoutSave) => SaveAction::CloseWithoutSave,
                None => {
                    warn!(
                        "{}",
                        &gettext("Save dialog has been destroyed before the user clicked a button")
                    );
                    SaveAction::Cancel
                }
                _ => SaveAction::Cancel,
            }
        };
        debug!("SaveAction: {:?}", save_action);

        if save_action != SaveAction::Cancel {
            let view_id = edit_view.borrow().view_id.clone();
            let mut main_win = main_win.borrow_mut();
            if let Some(w) = main_win.view_id_to_w.get(&view_id).map(Clone::clone) {
                if let Some(page_num) = main_win.notebook.page_num(&w) {
                    main_win.notebook.remove_page(Some(page_num));
                }
                main_win.w_to_ev.remove(&w.clone());
            }
            main_win.view_id_to_w.remove(&view_id);
            main_win.views.remove(&view_id);
            main_win.core.borrow().close_view(&view_id);
        }
        save_action
    }
}
