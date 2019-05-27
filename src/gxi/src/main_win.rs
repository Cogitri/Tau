use crate::about_win::AboutWin;
use crate::errors::{ErrorDialog, ErrorMsg};
use crate::prefs_win::PrefsWin;
use editview::{theme::u32_from_color, EditView, MainState, Settings};
use gettextrs::gettext;
use gio::{ActionMapExt, ApplicationExt, SettingsExt, SimpleAction};
use glib::{MainContext, Receiver, Sender};
use gtk::*;
use gxi_config_storage::{GSchema, GSchemaExt};
use log::{debug, error, info, trace, warn};
use serde_derive::*;
use serde_json::{self, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use xrl::{Client, Style, ViewId, XiEvent, XiEvent::*};

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

struct WinProp {
    height: i32,
    width: i32,
    is_maximized: bool,
    gschema: GSchema,
}

impl WinProp {
    pub fn new(application: &Application) -> Self {
        let gschema = GSchema::new(application.get_application_id().unwrap().as_str());
        Self {
            height: gschema.get_key("window-height"),
            width: gschema.get_key("window-width"),
            is_maximized: gschema.get_key("window-maximized"),
            gschema,
        }
    }
    pub fn save(&self) {
        self.gschema.set_key("window-height", self.height).unwrap();
        self.gschema.set_key("window-width", self.width).unwrap();
        self.gschema
            .set_key("window-maximized", self.is_maximized)
            .unwrap();
    }
}

pub struct MainWin {
    core: Client,
    window: ApplicationWindow,
    notebook: Notebook,
    builder: Builder,
    views: RefCell<BTreeMap<ViewId, Rc<RefCell<EditView>>>>,
    w_to_ev: RefCell<HashMap<Widget, Rc<RefCell<EditView>>>>,
    view_id_to_w: RefCell<HashMap<ViewId, Widget>>,
    state: Rc<RefCell<MainState>>,
    properties: RefCell<WinProp>,
    new_view_tx: Sender<ViewId>,
}

const GLADE_SRC: &str = include_str!("ui/gxi.glade");

impl MainWin {
    pub fn new(
        application: Application,
        core: Client,
        new_view_rx: Receiver<ViewId>,
        new_view_tx: Sender<ViewId>,
        event_rx: Receiver<XiEvent>,
    ) -> Rc<Self> {
        let glade_src = GLADE_SRC;
        let builder = Builder::new_from_string(glade_src);

        let properties = RefCell::new(WinProp::new(&application));
        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();

        if properties.borrow().is_maximized {
            window.maximize();
        } else {
            window.set_default_size(properties.borrow().width, properties.borrow().height);
        }

        let notebook: Notebook = builder.get_object("notebook").unwrap();

        let theme_name = properties.borrow().gschema.get_key("theme-name");
        debug!("{}: {}", gettext("Theme name"), &theme_name);

        let settings = new_settings();

        let main_state = Rc::new(RefCell::new(MainState {
            settings,
            theme_name,
            themes: Default::default(),
            theme: Default::default(),
            styles: Default::default(),
            fonts: Default::default(),
            avail_languages: Default::default(),
            selected_language: Default::default(),
        }));

        let main_win = Rc::new(Self {
            core: core.clone(),
            window: window.clone(),
            notebook: notebook.clone(),
            builder: builder.clone(),
            views: Default::default(),
            w_to_ev: Default::default(),
            view_id_to_w: Default::default(),
            state: main_state.clone(),
            new_view_tx,
            properties,
        });

        connect_settings_change(&main_win, &core);

        window.set_application(Some(&application));

        //This is called when the window is closed with the 'X' or via the application menu, etc.
        window.connect_delete_event(enclose!((main_win, window) move |_, _| {
            // Only destroy the window when the user has saved the changes or closes without saving
            if Self::close_all(main_win.clone()) == SaveAction::Cancel {
                debug!("{}", gettext("User chose to cancel exiting"));
                Inhibit(true)
            } else {
                debug!("{}", gettext("User chose to close the application"));
                main_win.properties.borrow().save();
                window.destroy();
                Inhibit(false)
            }
        }));

        window.connect_size_allocate(enclose!((main_win, window) move |_, _| {
            let win_size = window.get_size();
            let maximized = window.is_maximized();

            let mut properties = main_win.properties.borrow_mut();
            properties.is_maximized = maximized;
            if ! maximized {
                properties.width = win_size.0;
                properties.height = win_size.1;
            }
        }));

        {
            let open_action = SimpleAction::new("open", None);
            open_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'open' {}", gettext("Handling"), gettext("action"));
                Self::handle_open_button(&main_win);
            }));
            application.add_action(&open_action);
        }
        {
            let new_action = SimpleAction::new("new", None);
            new_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'new' {}", gettext("Handling"), gettext("action"));
                main_win.req_new_view(None);
            }));
            application.add_action(&new_action);
        }
        {
            let prefs_action = SimpleAction::new("prefs", None);
            prefs_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'prefs' {}", gettext("Handling"), gettext("action"));
                Self::prefs(main_win.clone())
            }));
            application.add_action(&prefs_action);
        }
        {
            let about_action = SimpleAction::new("about", None);
            about_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'about' {}", gettext("Handling"), gettext("action"));
                Self::about(main_win.clone())
            }));
            application.add_action(&about_action);
        }
        {
            let find_action = SimpleAction::new("find", None);
            find_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'find' {}", gettext("Handling"), gettext("action"));
                Self::find(&main_win);
            }));
            application.add_action(&find_action);
        }
        {
            let replace_action = SimpleAction::new("replace", None);
            replace_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'replace' {}", gettext("Handling"), gettext("action"));
                Self::replace(&main_win);
            }));
            application.add_action(&replace_action);
        }
        {
            let save_action = SimpleAction::new("save", None);
            save_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'save' {}", gettext("Handling"), gettext("action"));
                Self::handle_save_button(&main_win.clone());
            }));
            application.add_action(&save_action);
        }
        {
            let save_as_action = SimpleAction::new("save_as", None);
            save_as_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'save_as' {}", gettext("Handling"), gettext("action"));
                Self::current_save_as(&main_win.clone());
            }));
            application.add_action(&save_as_action);
        }
        {
            let close_action = SimpleAction::new("close", None);
            close_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'close' {}", gettext("Handling"), gettext("action"));
                Self::close(&main_win.clone());
            }));
            application.add_action(&close_action);
        }
        {
            let close_all_action = SimpleAction::new("close_all", None);
            close_all_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'close_all' {}", gettext("Handling"), gettext("action"));
                Self::close_all(main_win.clone());
            }));
            application.add_action(&close_all_action);
        }
        {
            // This is called when we run app.quit, e.g. via Ctrl+Q
            let quit_action = SimpleAction::new("quit", None);
            quit_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'quit' {}", gettext("Handling"), gettext("action"));
                // Same as in connect_destroy, only quit if the user saves or wants to close without saving
                if Self::close_all(main_win.clone()) == SaveAction::Cancel {
                    debug!("{}", gettext("User chose to not quit application"));
                } else {
                    debug!("{}", gettext("User chose to quit application"));
                    main_win.window.destroy();
                }
            }));
            application.add_action(&quit_action);
        }
        {
            let auto_indent_action = SimpleAction::new_stateful(
                "auto_indent",
                None,
                &main_state.borrow().settings.gschema.get_key("auto-indent"),
            );

            auto_indent_action.connect_change_state(enclose!((main_state) move |action, value| {
                if let Some(value) = value.as_ref() {
                    action.set_state(value);
                    main_state.borrow().settings.gschema.set_key("auto-indent", value.get::<bool>().unwrap()).unwrap();
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
                    .settings
                    .gschema
                    .get_key("translate-tabs-to-spaces"),
            );

            space_indent_action.connect_change_state(enclose!((main_state) move |action, value| {
                if let Some(value) = value.as_ref() {
                    action.set_state(value);
                    main_state.borrow().settings.gschema.set_key("translate-tabs-to-spaces", value.get::<bool>().unwrap()).unwrap();
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
            app.set_accels_for_action("app.replace", &["<Primary>r"]);
            app.set_accels_for_action("app.close", &["<Primary>w"]);
        }

        let main_context = MainContext::default();

        new_view_rx.attach(
            Some(&main_context),
            enclose!((main_win) move |view_id| {
                // TODO: This isn't correct
                MainWin::new_view_response(&main_win, None, &view_id);
                Continue(true)
            }),
        );

        event_rx.attach(
            Some(&main_context),
            enclose!((main_win) move |ev| {
                    MainWin::handle_event(&main_win, ev);
                    Continue(true)
            }),
        );

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
    fn handle_event(main_win: &Rc<Self>, ev: XiEvent) {
        trace!("{}: {:?}", gettext("Handling XiEvent"), ev);
        match ev {
            Alert(alert) => main_win.alert(alert),
            AvailableThemes(themes) => main_win.available_themes(themes),
            AvailablePlugins(plugins) => main_win.available_plugins(plugins),
            ConfigChanged(config) => main_win.config_changed(config),
            DefStyle(style) => main_win.def_style(style),
            FindStatus(status) => main_win.find_status(status),
            ReplaceStatus(status) => main_win.replace_status(status),
            Update(update) => main_win.update(update),
            ScrollTo(scroll) => main_win.scroll_to(scroll),
            ThemeChanged(theme) => main_win.theme_changed(theme),
            //MeasureWidth(measure_width) => main_win.measure_width(&measure_width),
            AvailableLanguages(langs) => main_win.available_languages(langs),
            LanguageChanged(lang) => main_win.language_changed(lang),
            PluginStarted(plugin) => main_win.plugin_started(plugin),
            PluginStoped(plugin) => main_win.plugin_stopped(plugin),
            _ => {}
        }
    }

    pub fn alert(&self, params: xrl::Alert) {
        ErrorDialog::new(ErrorMsg {
            msg: params.msg,
            fatal: false,
        });
    }

    pub fn available_themes(&self, params: xrl::AvailableThemes) {
        let mut state = self.state.borrow_mut();
        state.themes.clear();
        for theme in params.themes {
            state.themes.push(theme.to_string());
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

        self.core.set_theme(&state.theme_name);
    }

    pub fn theme_changed(&self, params: xrl::ThemeChanged) {
        // FIXME: Use annotations instead of constructing the selection style here
        let selection_style = Style {
            id: 0,
            fg_color: params
                .theme
                .selection_foreground
                .and_then(|s| Some(u32_from_color(s))),
            bg_color: params.theme.selection.and_then(|s| Some(u32_from_color(s))),
            weight: None,
            italic: None,
            underline: None,
        };

        let mut state = self.state.borrow_mut();
        state.theme = params.theme.clone();
        state.styles.insert(0, selection_style);
    }

    pub fn available_plugins(&self, params: xrl::AvailablePlugins) {
        let mut has_syntect = false;

        for x in &params.plugins {
            if &x.name == "xi-syntect-plugin" {
                has_syntect = true;
            }
        }

        if !has_syntect {
            ErrorDialog::new(ErrorMsg {
                msg: format!("{}: {:?}", gettext("Couldn't find syntect plugin, functionality will be limited! Only found the following plugins"), params.plugins),
                fatal: false,
            });
        }
    }

    pub fn config_changed(&self, params: xrl::ConfigChanged) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.borrow_mut().config_changed(&params.changes)
        }
    }

    pub fn find_status(&self, params: xrl::FindStatus) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.borrow().find_status(&params.queries)
        }
    }

    pub fn replace_status(&self, params: xrl::ReplaceStatus) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.borrow().replace_status(&params.status)
        }
    }

    pub fn def_style(&self, params: xrl::Style) {
        let mut state = self.state.borrow_mut();
        state.styles.insert(params.id as usize, params);
    }

    pub fn update(&self, params: xrl::Update) {
        trace!("{} 'update': {:?}", gettext("Handling"), params);
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.borrow_mut().update(&params)
        }
    }

    pub fn scroll_to(&self, params: xrl::ScrollTo) {
        trace!("{} 'scroll_to' {:?}", gettext("Handling"), params);

        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            let idx = self.notebook.page_num(&ev.borrow().root_widget);
            self.notebook.set_current_page(idx);
            ev.borrow().scroll_to(params.line, params.column);
        }
    }

    fn plugin_started(&self, _params: xrl::PluginStarted) {}

    fn plugin_stopped(&self, params: xrl::PluginStoped) {
        ErrorDialog::new(ErrorMsg {
            msg: format!(
                "{} {} {} {}",
                gettext("Plugin"),
                params.plugin,
                gettext("has crashed"),
                gettext("functionality will be limited")
            ),
            fatal: false,
        });
    }

    /*
    pub fn measure_width(&self, params: xrl::MeasureWidth) {
        trace!("{} 'measure_width' {:?}", gettext("Handling"), params);
        if let Some(ev) = self.get_current_edit_view() {
            let mut widths = Vec::new();

            for mes_width in params.0 {
                for string in &mes_width.strings {
                    widths.push(ev.borrow().line_width(string))
                }
            }
            //let widths: Vec<f64> = request.iter().map(|x| x.strings.iter().map(|v| edit_view.borrow().line_width(&v)).collect::<Vec<f64>>()).collect();

             self.core.send_result(id, &vec![widths]);
        }
    }*/

    pub fn available_languages(&self, params: xrl::AvailableLanguages) {
        debug!("{} 'available_languages' {:?}", gettext("Handling"), params);
        let mut main_state = self.state.borrow_mut();
        main_state.avail_languages.clear();
        for lang in params.languages {
            main_state.avail_languages.push(lang.to_string());
        }
    }

    pub fn language_changed(&self, params: xrl::LanguageChanged) {
        debug!("{} 'language_changed' {:?}", gettext("Handling"), params);
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.borrow().language_changed(&params.language_id)
        }
    }

    /// Display the FileChooserNative for opening, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    pub fn handle_open_button(main_win: &Rc<Self>) {
        let fcn = FileChooserNative::new(
            Some(gettext("Open a file to edit").as_str()),
            Some(&main_win.window),
            FileChooserAction::Open,
            Some(gettext("Open").as_str()),
            Some(gettext("Cancel").as_str()),
        );
        fcn.set_transient_for(Some(&main_win.window.clone()));
        fcn.set_select_multiple(true);

        fcn.connect_response(enclose!((main_win) move |fcd, res| {
            debug!(
                "{}: {:#?}",
                gettext("FileChooserNative open response"),
                res
            );

            if res == ResponseType::Accept {
                for file in fcd.get_filenames() {
                    let file_str = &file.to_string_lossy().into_owned();
                    match &std::fs::File::open(file_str) {
                        Ok(_) => main_win.req_new_view(Some(&file_str)),
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

    pub fn handle_save_button(main_win: &Rc<Self>) {
        if let Some(edit_view) = main_win.get_current_edit_view() {
            if edit_view.borrow().file_name.is_some() {
                let ev = edit_view.borrow();
                let core = main_win.core.clone();
                core.save(&ev.view_id, ev.file_name.as_ref().unwrap());
            } else {
                Self::save_as(main_win, &edit_view);
            }
        }
    }

    fn current_save_as(main_win: &Rc<Self>) {
        if let Some(edit_view) = main_win.get_current_edit_view() {
            Self::save_as(main_win, &edit_view);
        }
    }

    /// Display the FileChooserNative, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn save_as(main_win: &Rc<Self>, edit_view: &Rc<RefCell<EditView>>) {
        let fcn = FileChooserNative::new(
            Some(gettext("Save file").as_str()),
            Some(&main_win.window),
            FileChooserAction::Save,
            Some(gettext("Save").as_str()),
            Some(gettext("Cancel").as_str()),
        );
        fcn.set_transient_for(Some(&main_win.window.clone()));
        fcn.set_current_name("");

        fcn.connect_response(enclose!((edit_view, main_win) move |fcd, res| {
            debug!(
                "{}: {:#?}",
                gettext("FileChooserNative save response"),
                res
            );

            if res == ResponseType::Accept {
                for file in fcd.get_filenames() {
                    let file_str = &file.to_string_lossy().into_owned();
                    if let Some(file) = fcd.get_filename() {
                        match &std::fs::OpenOptions::new().write(true).create(true).open(&file) {
                            Ok(_) => {
                                debug!("{} {:?}", gettext("Saving file"), &file);
                                let view_id = edit_view.borrow().view_id.clone();
                                let file = file.to_string_lossy();
                                main_win.core.save(&view_id, &file);
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

    fn prefs(main_win: Rc<Self>) {
        let gschema = { &main_win.properties.borrow().gschema };
        PrefsWin::new(&main_win.window, &main_win.state, &main_win.core, &gschema);
    }

    fn about(main_win: Rc<Self>) {
        AboutWin::new(&main_win.window);
    }

    fn find(main_win: &Rc<Self>) {
        if let Some(edit_view) = main_win.get_current_edit_view() {
            edit_view.borrow().start_search();
        }
    }

    fn replace(main_win: &Rc<Self>) {
        if let Some(edit_view) = main_win.get_current_edit_view() {
            edit_view.borrow().start_replace();
        }
    }

    fn get_current_edit_view(&self) -> Option<Rc<RefCell<EditView>>> {
        if let Some(idx) = self.notebook.get_current_page() {
            if let Some(w) = self.notebook.get_nth_page(Some(idx)) {
                if let Some(edit_view) = self.w_to_ev.borrow().get(&w) {
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

        let view_id = tokio::executor::current_thread::block_on_all(
            self.core.new_view(file_name.map(|s| s.to_string())),
        )
        .unwrap();
        //TODO!
        self.new_view_tx.send(view_id).unwrap();
    }

    fn new_view_response(main_win: &Rc<Self>, file_name: Option<String>, view_id: &ViewId) {
        trace!("{}", gettext("Creating new EditView"));
        let mut old_ev = None;

        let position = if let Some(curr_ev) = main_win.get_current_edit_view() {
            if curr_ev.borrow().is_empty() {
                old_ev = Some(curr_ev.clone());
                if let Some(w) = main_win
                    .view_id_to_w
                    .borrow()
                    .get(&curr_ev.borrow().view_id)
                {
                    main_win.notebook.page_num(w)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let hamburger_button = main_win.builder.get_object("hamburger_button").unwrap();
        let edit_view = EditView::new(
            &main_win.state,
            &main_win.core,
            &hamburger_button,
            file_name,
            view_id.clone(),
            &main_win.window,
        );
        {
            let ev = edit_view.borrow();
            let page_num = main_win.notebook.insert_page(
                &ev.root_widget,
                Some(&ev.top_bar.tab_widget),
                position,
            );
            if let Some(w) = main_win.notebook.get_nth_page(Some(page_num)) {
                main_win
                    .w_to_ev
                    .borrow_mut()
                    .insert(w.clone(), edit_view.clone());
                main_win
                    .view_id_to_w
                    .borrow_mut()
                    .insert(view_id.clone(), w);
            }

            ev.top_bar
                .close_button
                .connect_clicked(enclose!((main_win, edit_view) move |_| {
                    Self::close_view(&main_win, &edit_view);
                }));
        }

        main_win
            .views
            .borrow_mut()
            .insert(view_id.clone(), edit_view);
        if let Some(empty_ev) = old_ev {
            Self::close_view(&main_win, &empty_ev);
        }
    }

    fn close_all(main_win: Rc<Self>) -> SaveAction {
        trace!("{}", gettext("Closing all EditViews"));
        // Get all views that we currently have opened
        let views = { main_win.views.borrow().clone() };
        // Close each one of them
        let actions: Vec<SaveAction> = views
            .iter()
            .map(|(_, ev)| {
                let save_action = Self::close_view(&main_win.clone(), &ev);
                if save_action != SaveAction::Cancel {
                    main_win.views.borrow_mut().remove(&ev.borrow().view_id);
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

    fn close(main_win: &Rc<Self>) -> SaveAction {
        trace!("{}", gettext("Closing current Editview"));
        if let Some(edit_view) = main_win.get_current_edit_view() {
            Self::close_view(&main_win, &edit_view)
        } else {
            SaveAction::Cancel
        }
    }

    fn close_view(main_win: &Rc<Self>, edit_view: &Rc<RefCell<EditView>>) -> SaveAction {
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
                .view_id_to_w
                .borrow()
                .get(&edit_view.borrow().view_id)
                .map(Clone::clone)
            {
                if let Some(page_num) = main_win.notebook.page_num(&w) {
                    main_win.notebook.set_property_page(page_num as i32);
                }
            }

            let ask_save_dialog = MessageDialog::new(
                Some(&main_win.window),
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
            if let Some(w) = main_win
                .view_id_to_w
                .borrow()
                .get(&view_id)
                .map(Clone::clone)
            {
                if let Some(page_num) = main_win.notebook.page_num(&w) {
                    main_win.notebook.remove_page(Some(page_num));
                }
                main_win.w_to_ev.borrow_mut().remove(&w.clone());
            }
            main_win.view_id_to_w.borrow_mut().remove(&view_id);
            main_win.views.borrow_mut().remove(&view_id);
            main_win.core.close_view(&view_id);
        }
        save_action
    }
}

pub fn new_settings() -> Settings {
    let gschema = GSchema::new("com.github.Cogitri.gxi");
    let interface_font = {
        use gtk::SettingsExt;
        let gtk_settings = gtk::Settings::get_default().unwrap();
        gtk_settings
            .get_property_gtk_font_name()
            .unwrap()
            .to_string()
    };

    Settings {
        trailing_spaces: gschema.get_key("draw-trailing-spaces"),
        highlight_line: gschema.get_key("highlight-line"),
        right_margin: gschema.get_key("draw-right-margin"),
        column_right_margin: gschema.get_key("column-right-margin"),
        edit_font: gschema.get_key("font"),
        tab_size: gschema.get_key("tab-size"),
        interface_font,
        gschema,
    }
}

pub fn connect_settings_change(main_win: &Rc<MainWin>, core: &Client) {
    let gschema = main_win.state.borrow().settings.gschema.clone();
    gschema
        .settings
        .connect_changed(enclose!((gschema, main_win, core) move |_, key| {
            trace!("Key '{}' has changed!", key);
            match key {
                "draw-trailing-spaces" => {
                    let val = gschema.get_key("draw-trailing-spaces");
                    main_win.state.borrow_mut().settings.trailing_spaces = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }
                "highlight-line" => {
                    let val = gschema.get_key("highlight-line");
                    main_win.state.borrow_mut().settings.highlight_line = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }
                "draw-right-margin" => {
                    let val = gschema.get_key("draw-right-margin");
                    main_win.state.borrow_mut().settings.right_margin = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }
                "column-right-margin" => {
                    let val = gschema.get_key("column-right-margin");
                    main_win.state.borrow_mut().settings.column_right_margin = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }
                "translate-tabs-to-spaces" => {
                    let val: bool = gschema.get_key("translate-tabs-to-spaces");
                    core.modify_user_config(
                        "general",
                        json!({ "translate_tabs_to_spaces": val })
                    );
                }
                "auto-indent" => {
                    let val: bool = gschema.get_key("auto-indent");
                    core.modify_user_config(
                        "general",
                        json!({ "autodetect_whitespace": val })
                    );
                }
                "tab-size" => {
                    let val: u32 = gschema.get_key("tab-size");
                    core.modify_user_config(
                        "general",
                        json!({ "tab_size": val })
                    );
                    main_win.state.borrow_mut().settings.tab_size = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }
                "font" => {
                    let val: String = gschema.get_key("font");
                    let font_vec = val.split_whitespace().collect::<Vec<_>>();
                    if let Some((size, splitted_name)) = font_vec.split_last() {
                        let font_name = splitted_name.join(" ");
                        let font_size = size.parse::<f32>().unwrap();
                        core.modify_user_config(
                            "general",
                            json!({ "font_face": font_name, "font_size": font_size })
                        );
                        main_win.state.borrow_mut().settings.edit_font = val;
                        if let Some(ev) = main_win.get_current_edit_view() {
                            ev.borrow().view_item.edit_area.queue_draw();
                        }
                    }
                }
                "use-tab-stops" => {
                    let val: bool = gschema.get_key("use-tab-stops");
                    core.modify_user_config(
                        "general",
                        json!({ "use_tab_stops": val })
                    );
                }
                "word-wrap" => {
                    let val: bool = gschema.get_key("word-wrap");
                    core.modify_user_config(
                        "general",
                        json!({ "word_wrap": val })
                    );
                }
                "theme-name" => {
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                },
                // We load these during startup
                "window-height" | "window-width" | "window-maximized" => {}
                _key => {
                    warn!("{}: {}", gettext("Unknown key change event"), _key)
                }
            }
        }));
}
