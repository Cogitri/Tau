use crate::edit_view::EditView;
use crate::pref_storage::{Config, GtkXiConfig, XiConfig};
use crate::prefs_win::PrefsWin;
use crate::proto::{self, ThemeSettings};
use crate::rpc::Core;
use crate::theme::{Color, Style, Theme};
use crate::CoreMsg;
use crate::SharedQueue;
use gettextrs::gettext;
use gio::{ActionMapExt, SimpleAction, SimpleActionExt};
use gtk::*;
use log::{debug, error, trace};
use serde_derive::*;
use serde_json::{self, json, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
pub struct MeasureWidth {
    pub id: u64,
    pub strings: Vec<String>,
}

pub struct MainState {
    pub themes: Vec<String>,
    pub theme_name: String,
    pub theme: Theme,
    pub styles: Vec<Style>,
    pub fonts: Vec<String>,
    pub avail_languages: Vec<String>,
    pub selected_language: String,
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
    pub fn new(
        application: &Application,
        shared_queue: &Arc<Mutex<SharedQueue>>,
        core: &Rc<RefCell<Core>>,
        config: Arc<Mutex<Config<XiConfig>>>,
        gxi_config: Arc<Mutex<Config<GtkXiConfig>>>,
    ) -> Rc<RefCell<MainWin>> {
        let glade_src = include_str!("ui/gxi.glade");
        let builder = Builder::new_from_string(glade_src);

        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();
        let notebook: Notebook = builder.get_object("notebook").unwrap();
        let syntax_combo_box: ComboBoxText = builder.get_object("syntax_combo_box").unwrap();

        let theme_name = gxi_config.lock().unwrap().config.theme.to_string();
        debug!("{}: {}", gettext("Theme name"), &theme_name);

        let main_win = Rc::new(RefCell::new(MainWin {
            core: core.clone(),
            shared_queue: shared_queue.clone(),
            window: window.clone(),
            notebook: notebook.clone(),
            builder: builder.clone(),
            views: Default::default(),
            w_to_ev: Default::default(),
            view_id_to_w: Default::default(),
            state: Rc::new(RefCell::new(MainState {
                themes: Default::default(),
                theme_name,
                theme: Default::default(),
                styles: Default::default(),
                fonts: Default::default(),
                avail_languages: Default::default(),
                selected_language: Default::default(),
            })),
        }));

        window.set_application(application);

        window.connect_delete_event(clone!(window => move |_, _| {
            window.destroy();
            Inhibit(false)
        }));

        {
            let main_win = main_win.clone();

            syntax_combo_box.append_text("None");
            syntax_combo_box.set_active(0);

            #[allow(unused_variables)]
            syntax_combo_box.connect_changed(clone!(core => move |cb|{
                if let Some(lang) = cb.get_active_text() {
                    main_win.borrow().set_language(&lang);
                }
            }));
        }
        {
            let open_action = SimpleAction::new("open", None);
            open_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::handle_open_button(&main_win);
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
            let xi_config = config.clone();
            prefs_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::prefs(main_win.clone(), xi_config.clone(), gxi_config.clone())
            }));
            application.add_action(&prefs_action);
        }
        {
            let find_action = SimpleAction::new("find", None);
            find_action.connect_activate(clone!(main_win => move |_,_| {
                MainWin::find(&main_win);
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
            let config = config.clone();

            let auto_indent_action = SimpleAction::new_stateful(
                "auto_indent",
                None,
                &config.lock().unwrap().config.auto_indent.to_variant(),
            );;

            #[allow(unused_variables)]
            auto_indent_action.connect_change_state(clone!(main_win => move |action, value| {
                if let Some(value) = value.as_ref() {
                    action.set_state(value);
                    let value: bool = value.get().unwrap();
                    debug!("{}: {}", gettext("Auto indent"), value);
                    let mut conf = config.lock().unwrap();
                    conf.config.auto_indent = value;
                    conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
                }
            }));
            application.add_action(&auto_indent_action);
        }
        {
            let space_indent_action = SimpleAction::new_stateful(
                "insert_spaces",
                None,
                &config
                    .lock()
                    .unwrap()
                    .config
                    .translate_tabs_to_spaces
                    .to_variant(),
            );;
            #[allow(unused_variables)]
            space_indent_action.connect_change_state(clone!(main_win => move |action, value| {
                if let Some(value) = value.as_ref() {
                    action.set_state(value);
                    let value: bool = value.get().unwrap();
                    debug!("{}: {}", gettext("Space indent"), value);
                    let mut conf = config.lock().unwrap();
                    conf.config.translate_tabs_to_spaces = value;
                    conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
                }
            }));
            application.add_action(&space_indent_action);
        }

        /* Put keyboard shortcuts here*/
        if let Some(app) = window.get_application() {
            app.set_accels_for_action("app.find", &["<Primary>f"]);
            app.set_accels_for_action("app.save", &["<Primary>s"]);
            app.set_accels_for_action("app.new", &["<Primary>n"]);
            app.set_accels_for_action("app.open", &["<Primary>o"]);
            app.set_accels_for_action("app.quit", &["<Primary>q"]);
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
            CoreMsg::NewViewReply { file_name, value } => {
                MainWin::new_view_response(&main_win, file_name, &value)
            }
            CoreMsg::Notification { method, params } => {
                match method.as_ref() {
                    "available_themes" => main_win.borrow_mut().available_themes(&params),
                    "available_plugins" => main_win.borrow_mut().available_plugins(&params),
                    "config_changed" => main_win.borrow_mut().config_changed(&params),
                    "def_style" => main_win.borrow_mut().def_style(&params),
                    "find_status" => main_win.borrow_mut().find_status(&params),
                    "update" => main_win.borrow_mut().update(&params),
                    "scroll_to" => main_win.borrow_mut().scroll_to(&params),
                    "theme_changed" => main_win.borrow_mut().theme_changed(&params),
                    "measure_width" => main_win.borrow().measure_width(params),
                    "available_languages" => main_win.borrow_mut().available_languages(&params),
                    "language_changed" => main_win.borrow_mut().language_changed(&params),
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
                gettext("isn't available, setting to default..."),
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
        let theme_settings: ThemeSettings = match serde_json::from_value(theme_settings) {
            Err(e) => {
                error!("{}: {}", gettext("Failed to convert theme settings"), e);
                return;
            }
            Ok(ts) => ts,
        };

        let selection_foreground = theme_settings
            .selection_foreground
            .map(Color::from_ts_proto);
        let selection = theme_settings.selection.map(Color::from_ts_proto);

        let theme = Theme::from_proto(&theme_settings);
        {
            let mut state = self.state.borrow_mut();
            state.theme = theme;
        }

        let selection_sytle = Style {
            fg_color: selection_foreground,
            bg_color: selection,
            weight: None,
            italic: None,
            underline: None,
        };

        self.set_style(0, selection_sytle);
    }

    pub fn available_plugins(&mut self, params: &Value) {
        if let Some(_) = params.get("plugins") {
            // TODO: There is one (or more!) plugins available, handle them!
        } else {
            error!(
                "{} {}",
                gettext("!!! UNHANDLED available_plugins, PLEASE OPEN A BUGRPEORT"),
                params
            );
        }
    }

    pub fn config_changed(&mut self, params: &Value) {
        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() {
                return;
            }
            view_id.unwrap().to_string()
        };

        if let Some(ev) = self.views.get(&view_id) {
            ev.borrow_mut().config_changed(&params["changes"]);
        }
    }

    pub fn find_status(&mut self, params: &Value) {
        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() {
                return;
            }
            view_id.unwrap().to_string()
        };

        if let Some(ev) = self.views.get(&view_id) {
            ev.borrow_mut().find_status(&params["queries"]);
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
            state.styles.push(Style {
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
        trace!("{} 'update': {:?}", gettext("Handling"), params);

        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() {
                return;
            }
            view_id.unwrap().to_string()
        };

        if let Some(ev) = self.views.get(&view_id) {
            ev.borrow_mut().update(params)
        }
    }

    pub fn scroll_to(&mut self, params: &Value) {
        trace!("{} 'scroll_to' {:?}", gettext("Handling"), params);
        let view_id = {
            let view_id = params["view_id"].as_str();
            if view_id.is_none() {
                return;
            }
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
            None => debug!("{} '{}'", gettext("Failed to find view"), view_id),
            Some(edit_view) => {
                let idx = self.notebook.page_num(&edit_view.borrow().root_widget);
                self.notebook.set_current_page(idx);
                edit_view.borrow_mut().scroll_to(line, col)
            }
        }
    }

    pub fn measure_width(&self, line_string: Value) {
        debug!("{} 'measure_width' {:?}", gettext("Handling"), line_string);
        let request: Vec<MeasureWidth> = serde_json::from_value(line_string).unwrap();
        let edit_view = self.get_current_edit_view();

        let mut widths = Vec::new();

        for mes_width in &request {
            for string in &mes_width.strings {
                widths.push(edit_view.borrow().line_width(string))
            }
        }
        //let widths: Vec<f64> = request.iter().map(|x| x.strings.iter().map(|v| edit_view.borrow().line_width(&v)).collect::<Vec<f64>>()).collect();

        self.core
            .borrow()
            .send_result(&serde_json::to_value(vec![widths]).unwrap());
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

    pub fn set_language(&self, lang: &str) {
        debug!("{} '{:?}'", gettext("Chainging language to"), lang);
        let core = self.core.borrow();
        let edit_view = self.get_current_edit_view().clone();
        core.set_language(&edit_view.borrow().view_id, &lang);
    }

    /// Display the FileChooserDialog for opening, send the result to the Xi core.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    pub fn handle_open_button(main_win: &Rc<RefCell<MainWin>>) {
        let fcd = FileChooserDialog::new::<FileChooserDialog>(None, None, FileChooserAction::Open);
        fcd.set_transient_for(Some(&main_win.borrow().window.clone()));
        fcd.add_button("Open", 33);
        fcd.set_default_response(33);
        fcd.set_select_multiple(true);
        let response = fcd.run(); // Can call main loop, can't have any borrows out
        debug!(
            "{}: {}",
            gettext("FileChooserDialog open response"),
            response
        );
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
            core.borrow()
                .save(&ev.view_id, ev.file_name.as_ref().unwrap());
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
        debug!(
            "{}: {}",
            gettext("FileChooserDialog open response"),
            response
        );
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

    fn prefs(
        main_win: Rc<RefCell<MainWin>>,
        xi_config: Arc<Mutex<Config<XiConfig>>>,
        gxi_config: Arc<Mutex<Config<GtkXiConfig>>>,
    ) {
        // let (main_state, core) = {
        //     let main_win = main_win.borrow();
        //     (main_win.state.clone(), main_win.core.clone())
        // };
        let main_win = main_win.borrow();
        let main_state = main_win.state.clone();
        let core = main_win.core.clone();
        PrefsWin::new(&main_win.window, &main_state, &core, xi_config, gxi_config);

        //prefs_win.run();
    }

    fn find(main_win: &Rc<RefCell<MainWin>>) {
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
        unreachable!(gettext("Failed to get the current editview!"));
    }

    fn req_new_view(&self, file_name: Option<&str>) {
        let mut params = json!({});
        if let Some(file_name) = file_name {
            params["file_path"] = json!(file_name);
        }

        let shared_queue2 = self.shared_queue.clone();
        let file_name2 = file_name.map(|s| s.to_string());
        self.core
            .borrow_mut()
            .send_request("new_view", &params, move |value| {
                let value = value.clone();
                let mut shared_queue = shared_queue2.lock().unwrap();
                shared_queue.add_core_msg(CoreMsg::NewViewReply {
                    file_name: file_name2,
                    value,
                })
            });
    }

    fn new_view_response(
        main_win: &Rc<RefCell<MainWin>>,
        file_name: Option<String>,
        value: &Value,
    ) {
        let mut win = main_win.borrow_mut();

        // Add all available langs to the syntax_combo_box for the user to select it. We're doing
        // it here because we can be sure that xi-editor has sent available_languages by now.
        let syntax_combo_box: ComboBoxText = win.builder.get_object("syntax_combo_box").unwrap();

        win.state
            .borrow()
            .avail_languages
            .iter()
            .for_each(|lang| syntax_combo_box.append_text(lang));

        if let Some(view_id) = value.as_str() {
            let edit_view = EditView::new(&win.state, &win.core, file_name, view_id);
            {
                let ev = edit_view.borrow();
                let page_num =
                    win.notebook
                        .insert_page(&ev.root_widget, Some(&ev.tab_widget), None);
                if let Some(w) = win.notebook.get_nth_page(Some(page_num)) {
                    win.w_to_ev.insert(w.clone(), edit_view.clone());
                    win.view_id_to_w.insert(view_id.to_string(), w);
                }

                ev.close_button
                    .connect_clicked(clone!(main_win, edit_view => move |_| {
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
            debug!("{}: {}", gettext("AskSaveDialog response (1=save)"), ret);
            match ret {
                1 => MainWin::save_as(main_win, edit_view),
                2 => return,
                _ => {}
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
