use crate::about_win::AboutWin;
use crate::errors::{ErrorDialog, ErrorMsg};
use crate::frontend::{XiEvent, XiRequest};
use crate::prefs_win::PrefsWin;
use crate::shortcuts_win::ShortcutsWin;
use crate::syntax_config::SyntaxParams;
use editview::{theme::u32_from_color, EditView, MainState, Settings};
use gdk_pixbuf::Pixbuf;
use gettextrs::gettext;
use gio::{ActionMapExt, ApplicationExt, Resource, SettingsExt, SimpleAction};
use glib::{Bytes, GString, MainContext, Receiver, Sender};
use gschema_config_storage::{GSchema, GSchemaExt};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Builder, ButtonsType, DialogFlags, FileChooserAction,
    FileChooserNative, MessageDialog, MessageType, Notebook, ResponseType, Widget,
};
use log::{debug, error, info, trace, warn};
use serde_json::{self, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::rc::Rc;
use xrl::{Client, Style, ViewId, XiNotification};

pub const RESOURCE: &[u8] = include_bytes!("ui/resources.gresource");

/// Returned by an `ask_save_dialog` when we ask the user if he wants to either:
/// - `Save`(save unsaved changes and close view)
/// - `CloseWithoutSave` (discard pending changes and close view)
/// - `Cancel` (cancel the action and return to editing)
#[derive(Debug, PartialEq)]
enum SaveAction {
    /// Symbols that we should save&close
    Save = 100,
    //// Symbols that we should close w/o save
    CloseWithoutSave = 101,
    /// Symbols to close without saving
    Cancel = 102,
}

impl TryFrom<i32> for SaveAction {
    type Error = String;

    /// Try to convert from an i32 to `SaveAction`, used to check what the `SaveDialog` has returned.
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            100 => Ok(SaveAction::Save),
            101 => Ok(SaveAction::CloseWithoutSave),
            102 => Ok(SaveAction::Cancel),
            _ => Err(format!(
                "The i32 '{}' doesn't match any of the variants of the enum!",
                value
            )),
        }
    }
}

/// The `WinProp` struct, which holds some information about the current state of the Window. It's
/// saved to `GSettings` during shutdown to restore the window state when it's started again.
struct WinProp {
    /// Height of the MainWin
    height: i32,
    /// Width of the MainWin
    width: i32,
    /// Whether or not the MainWin is maximized
    is_maximized: bool,
    /// The GSchema we save the fields of the WinProp to
    gschema: GSchema,
}

impl WinProp {
    /// Create a new WinProp. Gets the GSchema of the name of the `Application`'s id
    ///
    /// # Panics
    ///
    /// This will panic if there's no GSchema of the name of the `Application`s id.
    pub fn new(application: &Application) -> Self {
        let gschema = GSchema::new(application.get_application_id().unwrap().as_str());
        Self {
            height: gschema.get_key("window-height"),
            width: gschema.get_key("window-width"),
            is_maximized: gschema.get_key("window-maximized"),
            gschema,
        }
    }

    /// Save the WinProp to the `WinProp.gschema`
    pub fn save(&self) {
        self.gschema.set_key("window-height", self.height).unwrap();
        self.gschema.set_key("window-width", self.width).unwrap();
        self.gschema
            .set_key("window-maximized", self.is_maximized)
            .unwrap();
    }
}

/// Indicates which plugins, which have tight integration with Tau, have been started
#[derive(Debug, Default, PartialEq)]
pub struct StartedPlugins {
    /// Provides auto-indention and syntax highlighting
    pub syntect: bool,
}

/// The `MainWin` is (as the name suggests) tau's main window. It holds buttons like `Open` and `Save`
/// and holds the `EditViews`, which do the actual editing. Refer to [the module level docs](main/index.html)
/// for more information.
pub struct MainWin {
    /// The handle to communicate with Xi.
    core: Client,
    /// The GTK Window.
    window: ApplicationWindow,
    /// The Notebook holding all `EditView`s.
    notebook: Notebook,
    /// The `Builder` from which we build the GTK Widgets.
    builder: Builder,
    /// A Map mapping `ViewId`s to `EditView`s.
    views: RefCell<BTreeMap<ViewId, Rc<EditView>>>,
    /// A Map mapping GTK `Widget`s to `EditView`s.
    w_to_ev: RefCell<HashMap<Widget, Rc<EditView>>>,
    /// A map mapping `ViewId`s to GTK `Widget`s.
    view_id_to_w: RefCell<HashMap<ViewId, Widget>>,
    /// The `MainState`, which are common settings among all `EditView`s.
    state: Rc<RefCell<MainState>>,
    /// The `WinProp` Struct, used for saving the window state during shutdown
    properties: RefCell<WinProp>,
    /// A glib `Sender` from whom we receive something when we should create new `EditView`s.
    new_view_tx: Sender<(ViewId, Option<String>)>,
    /// A crossbeam_channel `Sender` from whom we receive something when Xi requests something.
    request_tx: crossbeam_channel::Sender<XiRequest>,
    /// A `HashMap` containing the different configs for each syntax
    syntax_config: RefCell<HashMap<String, SyntaxParams>>,
    /// Indicates which special plugins (for which we have to do additional work) have been started
    started_plugins: RefCell<StartedPlugins>,
}

impl MainWin {
    /// Create a new `MainWin` instance, which facilitates Tau's buttons (like save/open) and
    /// bootstrap Tau
    pub fn new(
        // The `gio::Application` which this `MainWin` belongs to
        application: &Application,
        // The `xi-core` we can send commands to
        core: Client,
        // The `Receiver` we get requests to open new views from
        new_view_rx: Receiver<(ViewId, Option<String>)>,
        // The `Sender` to open new views
        new_view_tx: Sender<(ViewId, Option<String>)>,
        // The `Receiver` on which we receive messages from `xi-core`
        event_rx: Receiver<XiEvent>,
        // The `Receiver` on which we receive requests from `xi-core`
        request_tx: crossbeam_channel::Sender<XiRequest>,
    ) -> Rc<Self> {
        let gbytes = Bytes::from_static(RESOURCE);
        let resource = Resource::new_from_data(&gbytes).unwrap();
        gio::resources_register(&resource);

        // Add custom CSS, mainly to make the statusbar smaller
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/gnome/Tau/app.css");
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default()
                .unwrap_or_else(|| panic!("{}", gettext("Failed to get default CssProvider!"))),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let builder = Builder::new_from_resource("/org/gnome/Tau/tau.glade");

        let properties = RefCell::new(WinProp::new(&application));
        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();

        let icon = Pixbuf::new_from_resource("/org/gnome/Tau/org.gnome.Tau.svg");
        window.set_icon(icon.ok().as_ref());

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

        let syntax_changes = main_state
            .borrow()
            .settings
            .gschema
            .settings
            .get_strv("syntax-config");
        let syntax_config: HashMap<String, SyntaxParams> = syntax_changes
            .iter()
            .map(GString::as_str)
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| error!("{} {}", gettext("Failed to deserialize syntax config"), e))
                    .unwrap()
            })
            .map(|sc: SyntaxParams| (sc.domain.syntax.clone(), sc))
            .collect();

        let main_win = Rc::new(Self {
            core,
            window,
            notebook,
            builder,
            new_view_tx,
            properties,
            request_tx,
            views: Default::default(),
            w_to_ev: Default::default(),
            view_id_to_w: Default::default(),
            state: main_state,
            syntax_config: RefCell::new(syntax_config),
            started_plugins: RefCell::new(Default::default()),
        });

        connect_settings_change(&main_win, &main_win.core);

        main_win.window.set_application(Some(&application.clone()));

        // This is called when the window is closed with the 'X' or via the application menu, etc.
        main_win
            .window
            .connect_delete_event(enclose!((main_win) move |window, _| {
                // Only destroy the window when the user has saved the changes or closes without saving
                if Self::close_all(&main_win) == SaveAction::Cancel {
                    debug!("{}", gettext("User chose to cancel exiting"));
                    Inhibit(true)
                } else {
                    debug!("{}", gettext("User chose to close the application"));
                    main_win.properties.borrow().save();
                    window.destroy();
                    Inhibit(false)
                }
            }));

        // Save to `WinProp` when the size of the window is changed
        main_win
            .window
            .connect_size_allocate(enclose!((main_win) move |window, _| {
                let win_size = window.get_size();
                let maximized = window.is_maximized();

                let mut properties = main_win.properties.borrow_mut();
                properties.is_maximized = maximized;
                if ! maximized {
                    properties.width = win_size.0;
                    properties.height = win_size.1;
                }
            }));

        // Below here we connect all actions, meaning that these closures will be run when the respective
        // action is triggered (e.g. by a button press)
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
                Self::prefs(&main_win)
            }));
            application.add_action(&prefs_action);
        }
        {
            let about_action = SimpleAction::new("about", None);
            about_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'about' {}", gettext("Handling"), gettext("action"));
                Self::about(&main_win)
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
            let copy_action = SimpleAction::new("copy", None);
            copy_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'copy' {}", gettext("Handling"), gettext("action"));
                if let Some(ev) = main_win.get_current_edit_view() {
                    ev.do_copy(ev.view_id)
                }
            }));
            application.add_action(&copy_action);
        }
        {
            let cut_action = SimpleAction::new("cut", None);
            cut_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'cut' {}", gettext("Handling"), gettext("action"));
                if let Some(ev) = main_win.get_current_edit_view() {
                    ev.do_cut(ev.view_id)
                }
            }));
            application.add_action(&cut_action);
        }
        {
            let paste_action = SimpleAction::new("paste", None);
            paste_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'paste' {}", gettext("Handling"), gettext("action"));
                if let Some(ev) = main_win.get_current_edit_view() {
                    ev.do_paste(ev.view_id)
                }
            }));
            application.add_action(&paste_action);
        }
        {
            let undo_action = SimpleAction::new("undo", None);
            undo_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'undo' {}", gettext("Handling"), gettext("action"));
                if let Some(ev) = main_win.get_current_edit_view() {
                    main_win.core.undo(ev.view_id);
                }
            }));
            application.add_action(&undo_action);
        }
        {
            let redo_action = SimpleAction::new("redo", None);
            redo_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'redo' {}", gettext("Handling"), gettext("action"));
                if let Some(ev) = main_win.get_current_edit_view() {
                    main_win.core.redo(ev.view_id);
                }
            }));
            application.add_action(&redo_action);
        }
        {
            let select_all_action = SimpleAction::new("select_all", None);
            select_all_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'select_all' {}", gettext("Handling"), gettext("action"));
                if let Some(ev) = main_win.get_current_edit_view() {
                    main_win.core.select_all(ev.view_id);
                }
            }));
            application.add_action(&select_all_action);
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
                Self::close_all(&main_win);
            }));
            application.add_action(&close_all_action);
        }
        {
            let shortcuts_action = SimpleAction::new("shortcuts", None);
            shortcuts_action.connect_activate(enclose!((main_win) move |_, _| {
                trace!("{} 'shortcuts' {}", gettext("Handling"), gettext("action"));
                main_win.shortcuts();
            }));
            application.add_action(&shortcuts_action);
        }
        {
            let find_prev_action = SimpleAction::new("find_prev", None);
            find_prev_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'find_prev' {}", gettext("Handling"), gettext("action"));
                main_win.find_prev();
            }));
            application.add_action(&find_prev_action);
        }
        {
            let find_next_action = SimpleAction::new("find_next", None);
            find_next_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'find_next' {}", gettext("Handling"), gettext("action"));
                main_win.find_next();
            }));
            application.add_action(&find_next_action);
        }
        {
            // This is called when we run app.quit, e.g. via Ctrl+Q
            let quit_action = SimpleAction::new("quit", None);
            quit_action.connect_activate(enclose!((main_win) move |_,_| {
                trace!("{} 'quit' {}", gettext("Handling"), gettext("action"));
                // Same as in connect_destroy, only quit if the user saves or wants to close without saving
                if Self::close_all(&main_win) == SaveAction::Cancel {
                    debug!("{}", gettext("User chose to not quit application"));
                } else {
                    debug!("{}", gettext("User chose to quit application"));
                    main_win.window.close();
                }
            }));
            application.add_action(&quit_action);
        }

        // Put keyboard shortcuts here
        application.set_accels_for_action("app.find", &["<Primary>f"]);
        application.set_accels_for_action("app.save", &["<Primary>s"]);
        application.set_accels_for_action("app.new", &["<Primary>n"]);
        application.set_accels_for_action("app.open", &["<Primary>o"]);
        application.set_accels_for_action("app.quit", &["<Primary>q"]);
        application.set_accels_for_action("app.replace", &["<Primary>r"]);
        application.set_accels_for_action("app.close", &["<Primary>w"]);
        application.set_accels_for_action("app.find_next", &["<Primary>g"]);
        application.set_accels_for_action("app.find_prev", &["<Primary><Shift>g"]);

        let main_context = MainContext::default();

        // Open new `EditView`s when we receives something here. This is a channel because we can
        // also receive this from `connect_open`/`connect_activate` in main.rs
        new_view_rx.attach(
            Some(&main_context),
            enclose!((main_win) move |(view_id, path)| {
                Self::new_view_response(&main_win, path, view_id);
                Continue(true)
            }),
        );

        event_rx.attach(
            Some(&main_context),
            enclose!((main_win) move |ev| {
                    Self::handle_event(&main_win, ev);
                    Continue(true)
            }),
        );

        debug!("{}", gettext("Showing main window"));
        main_win.window.show_all();

        main_win
    }
}

impl MainWin {
    fn handle_event(&self, ev: XiEvent) {
        trace!("{}: {:?}", gettext("Handling XiEvent"), ev);
        match ev {
            XiEvent::Notification(notification) => match notification {
                XiNotification::Alert(alert) => self.alert(alert),
                XiNotification::AvailableThemes(themes) => self.available_themes(themes),
                XiNotification::ConfigChanged(config) => self.config_changed(&config),
                XiNotification::DefStyle(style) => self.def_style(style),
                XiNotification::FindStatus(status) => self.find_status(&status),
                XiNotification::ReplaceStatus(status) => self.replace_status(&status),
                XiNotification::Update(update) => self.update(update),
                XiNotification::ScrollTo(scroll) => self.scroll_to(&scroll),
                XiNotification::ThemeChanged(theme) => self.theme_changed(theme),
                XiNotification::AvailableLanguages(langs) => self.available_languages(langs),
                XiNotification::LanguageChanged(lang) => self.language_changed(&lang),
                XiNotification::PluginStarted(plugin) => self.plugin_started(&plugin),
                XiNotification::PluginStoped(plugin) => self.plugin_stopped(&plugin),
                _ => {}
            },
            XiEvent::MeasureWidth(measure_width) => self.measure_width(measure_width),
        }
    }

    /// Open an `ErrorDialog` when with the `Alert`'s msg
    pub fn alert(&self, params: xrl::Alert) {
        ErrorDialog::new(ErrorMsg {
            msg: params.msg,
            fatal: false,
        });
    }

    /// Register the `AvailableThemes` with our `MainState`
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

            if let Some(theme_name) = state.themes.first() {
                state
                    .settings
                    .gschema
                    .set_key("theme-name", theme_name.clone())
                    .unwrap_or_else(|e| {
                        error!(
                            "{}: {}",
                            gettext("Failed to set theme name in GSettings due to error"),
                            e
                        )
                    });
                state.theme_name = theme_name.clone();
            } else {
                return;
            }
        }

        self.core.set_theme(&state.theme_name);
    }

    /// Change the theme in our `MainState`
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
        state.theme = params.theme;
        state.styles.insert(0, selection_style);
    }

    /// Forward `ConfigChanged` to the respective `EditView`
    pub fn config_changed(&self, params: &xrl::ConfigChanged) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.config_changed(&params.changes)
        }
    }

    /// Forward `FindStatus` to the respective `EditView`
    pub fn find_status(&self, params: &xrl::FindStatus) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.find_status(&params.queries)
        }
    }

    /// Forward `ReplaceStatus` to the respective `EditView`
    pub fn replace_status(&self, params: &xrl::ReplaceStatus) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.replace_status(&params.status)
        }
    }

    /// Insert a style into our `MainState`
    pub fn def_style(&self, params: xrl::Style) {
        let mut state = self.state.borrow_mut();
        state.styles.insert(params.id as usize, params);
    }

    /// Forward `Update` to the respective `EditView`
    pub fn update(&self, params: xrl::Update) {
        trace!("{} 'update': {:?}", gettext("Handling"), params);
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.update(params)
        }
    }

    /// Forward `ScrollTo` to the respective `EditView`. Also set our `GtkNotebook`'s
    /// current page to that `EditView`
    pub fn scroll_to(&self, params: &xrl::ScrollTo) {
        trace!("{} 'scroll_to' {:?}", gettext("Handling"), params);

        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            let idx = self.notebook.page_num(&ev.root_widget);
            self.notebook.set_current_page(idx);
            ev.scroll_to(params.line, params.column);
        }
    }

    fn plugin_started(&self, params: &xrl::PluginStarted) {
        if params.plugin == "xi-syntect-plugin" {
            self.started_plugins.borrow_mut().syntect = true;
            if let Some(ev) = self.views.borrow().get(&params.view_id) {
                ev.view_item
                    .statusbar
                    .insert_spaces_button
                    .set_sensitive(true);
                ev.view_item
                    .statusbar
                    .auto_indention_button
                    .set_sensitive(true);
            }
        }
    }

    /// Open an error dialog if a plugin has crashed
    fn plugin_stopped(&self, params: &xrl::PluginStoped) {
        if params.plugin == "xi-syntect-plugin" {
            self.started_plugins.borrow_mut().syntect = false;
            if let Some(ev) = self.views.borrow().get(&params.view_id) {
                ev.view_item
                    .statusbar
                    .insert_spaces_button
                    .set_sensitive(false);
                ev.view_item
                    .statusbar
                    .auto_indention_button
                    .set_sensitive(false);
            }
        }
    }

    /// Measure the width of a string for Xi and send it the result. Used for line wrapping.
    pub fn measure_width(&self, params: xrl::MeasureWidth) {
        trace!("{} 'measure_width' {:?}", gettext("Handling"), params);
        if let Some(ev) = self.get_current_edit_view() {
            let mut widths = Vec::new();

            for mes_width in params.0 {
                for string in &mes_width.strings {
                    widths.push(ev.line_width(string) as f32)
                }
            }

            self.request_tx
                .send(XiRequest::MeasureWidth(vec![widths]))
                .unwrap();
        }
    }

    /// Set available syntaxes in our `MainState` and set the syntax_seletion_sensitivity
    /// of all `EditView`s, so it's unsensitive when we don't have any syntaxes to choose from.
    pub fn available_languages(&self, params: xrl::AvailableLanguages) {
        debug!("{} 'available_languages' {:?}", gettext("Handling"), params);
        let mut main_state = self.state.borrow_mut();
        main_state.avail_languages.clear();

        // If there are no syntaxes to choose from, disable the selection
        if params.languages.is_empty() {
            for (_, ev) in self.views.borrow().iter() {
                ev.set_syntax_selection_sensitivity(false);
            }
        } else {
            for (_, ev) in self.views.borrow().iter() {
                ev.set_syntax_selection_sensitivity(true);
            }
        }

        for lang in params.languages {
            main_state.avail_languages.push(lang.to_string());
        }

        let langs: Vec<&str> = main_state
            .avail_languages
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<&str>>();

        for (_, ev) in self.views.borrow().iter() {
            ev.view_item.set_avail_langs(&langs);
        }
    }

    /// Forward `LanguageChanged` to the respective `EditView`
    pub fn language_changed(&self, params: &xrl::LanguageChanged) {
        debug!("{} 'language_changed' {:?}", gettext("Handling"), params);
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            // Set the default_tab_size so the EditView
            if let Some(sc) = self.syntax_config.borrow().get(&params.language_id) {
                if let Some(tab_size) = sc.changes.tab_size {
                    debug!(
                        "{}: '{}'",
                        gettext("Setting the following to the syntax attached tab size"),
                        tab_size
                    );
                    ev.set_default_tab_size(tab_size);
                } else {
                    debug!("{}", gettext("No tab size attached to the syntax"));
                }
            }
            ev.language_changed(&params.language_id);
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
                    let file_str = file.to_string_lossy().into_owned();
                    match std::fs::File::open(&file_str) {
                        Ok(_) => main_win.req_new_view(Some(file_str)),
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

    /// Save the `EditView`'s document if a filename is set, or open a filesaver
    /// dialog for the user to choose a name
    pub fn handle_save_button(main_win: &Rc<Self>) {
        if let Some(edit_view) = main_win.get_current_edit_view() {
            let name = { edit_view.file_name.borrow().clone() };
            if let Some(ref file_name) = name {
                main_win.core.save(edit_view.view_id, file_name);
            } else {
                Self::save_as(main_win, &edit_view);
            }
        }
    }

    /// Open a filesaver dialog for the user to choose a name where to save the
    /// file and save to it.
    fn current_save_as(main_win: &Rc<Self>) {
        if let Some(edit_view) = main_win.get_current_edit_view() {
            Self::save_as(main_win, &edit_view);
        }
    }

    /// Display the FileChooserNative, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn save_as(main_win: &Rc<Self>, edit_view: &Rc<EditView>) {
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
                                let file = file.to_string_lossy();
                                main_win.core.save(edit_view.view_id, &file);
                                edit_view.set_file(&file);
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

    /// Open a `PrefsWin` for the user to configure things like the theme
    fn prefs(&self) {
        let gschema = { &self.properties.borrow().gschema };
        let lang = if let Some(ev) = self.get_current_edit_view() {
            ev.view_item
                .statusbar
                .syntax_label
                .get_text()
                .map(|s| s.to_string())
        } else {
            None
        };
        PrefsWin::new(
            &self.window,
            &self.state,
            &self.core,
            gschema,
            lang.as_ref().map(String::as_str),
            &self.started_plugins.borrow(),
        );
    }

    /// Open the `AboutWin`, which contains some info about Tau
    fn about(&self) {
        AboutWin::new(&self.window);
    }

    /// Open the `ShortcutsWin`, which contains info about shortcuts
    fn shortcuts(&self) {
        ShortcutsWin::new(&self.window);
    }

    /// Open the find dialog of the current `EditView`
    fn find(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.start_search();
        }
    }

    fn find_prev(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.find_prev();
        }
    }

    fn find_next(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.find_next();
        }
    }

    /// Open the replace dialog of the current `EditView
    fn replace(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.start_replace();
        }
    }

    /// Get the currently opened `EditView` in our `GtkNotebook`
    fn get_current_edit_view(&self) -> Option<Rc<EditView>> {
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

    /// Request a new view from `xi-core` and send
    fn req_new_view(&self, file_name: Option<String>) {
        trace!("{}", gettext("Requesting new view"));

        let core = self.core.clone();
        let new_view_tx = self.new_view_tx.clone();
        std::thread::spawn(move || {
            let view_id = tokio::executor::current_thread::block_on_all(
                core.new_view(file_name.as_ref().map(ToString::to_string)),
            )
            .unwrap();

            new_view_tx.send((view_id, file_name)).unwrap();
        });
    }

    /// When `xi-core` tells us to create a new view, we have to do multiple things:
    ///
    /// 1) Check if the current `EditView` is empty (doesn't contain ANY text). If so, replace that `EditView`
    ///    with the new `EditView`. That way we don't stack empty, useless views.
    /// 2) Connect the ways to close the `EditView`, either via a middle click or by clicking the X of the tab
    fn new_view_response(main_win: &Rc<Self>, file_name: Option<String>, view_id: ViewId) {
        trace!("{}", gettext("Creating new EditView"));
        let mut old_ev = None;

        let position = if let Some(curr_ev) = main_win.get_current_edit_view() {
            if curr_ev.is_empty() {
                old_ev = Some(curr_ev.clone());
                if let Some(w) = main_win.view_id_to_w.borrow().get(&curr_ev.view_id) {
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
            view_id,
            &main_win.window,
        );
        {
            let page_num = main_win.notebook.insert_page(
                &edit_view.root_widget,
                Some(&edit_view.top_bar.event_box),
                position,
            );
            if let Some(w) = main_win.notebook.get_nth_page(Some(page_num)) {
                main_win
                    .w_to_ev
                    .borrow_mut()
                    .insert(w.clone(), edit_view.clone());
                main_win.view_id_to_w.borrow_mut().insert(view_id, w);
            }

            edit_view.top_bar.close_button.connect_clicked(
                enclose!((main_win, edit_view) move |_| {
                    Self::close_view(&main_win, &edit_view);
                }),
            );

            edit_view.top_bar.event_box.connect_button_press_event(
                enclose!((main_win, edit_view) move |_, eb| {
                    // 2 == middle click
                    if eb.get_button() == 2 {
                        Self::close_view(&main_win, &edit_view);
                    }
                    Inhibit(false)
                }),
            );
        }

        main_win.views.borrow_mut().insert(view_id, edit_view);
        if let Some(empty_ev) = old_ev {
            Self::close_view(main_win, &empty_ev);
        }
    }

    /// Close all `EditView`s, checking if the user wants to close them if there are unsaved changes
    ///
    /// # Returns
    ///
    /// - `SaveAction` determining if all `EditView`s have been closed.
    fn close_all(main_win: &Rc<Self>) -> SaveAction {
        trace!("{}", gettext("Closing all EditViews"));
        // Get all views that we currently have opened
        let views = { main_win.views.borrow().clone() };
        // Close each one of them
        let actions: Vec<SaveAction> = views
            .iter()
            .map(|(_, ev)| {
                let save_action = Self::close_view(&main_win.clone(), ev);
                if save_action != SaveAction::Cancel {
                    main_win.views.borrow_mut().remove(&ev.view_id);
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

    /// Close the current `EditView`
    ///
    /// # Returns
    ///
    /// - `SaveAction` determining if the `EdtiView` has been closed.
    fn close(main_win: &Rc<Self>) -> SaveAction {
        trace!("{}", gettext("Closing current Editview"));
        if let Some(edit_view) = main_win.get_current_edit_view() {
            Self::close_view(main_win, &edit_view)
        } else {
            SaveAction::Cancel
        }
    }

    /// Close a specific `EditView`. Changes the `GtkNotebook` to the supplied `EditView`, so that the
    /// user can see which one is being closed. Presents the user a close dialog giving him the choice
    /// of either saving, aborting or closing without saving, if the `EditView` has unsaved changes.
    ///
    /// # Returns
    ///
    /// `SaveAction` determining which choice the user has made in the save dialog
    fn close_view(main_win: &Rc<Self>, edit_view: &Rc<EditView>) -> SaveAction {
        trace!("{} {}", gettext("Closing Editview"), edit_view.view_id);
        let save_action = if *edit_view.pristine.borrow() {
            // If it's pristine we don't ask the user if he really wants to quit because everything
            // is saved already and as such always close without saving
            SaveAction::CloseWithoutSave
        } else {
            // Change the tab to the EditView we want to ask the user about saving to give him a
            // change to review that action
            if let Some(w) = main_win
                .view_id_to_w
                .borrow()
                .get(&edit_view.view_id)
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
            let ret: i32 = ask_save_dialog.run().into();
            ask_save_dialog.destroy();
            match SaveAction::try_from(ret) {
                Ok(SaveAction::Save) => {
                    Self::handle_save_button(main_win);
                    SaveAction::Save
                }
                Ok(SaveAction::CloseWithoutSave) => SaveAction::CloseWithoutSave,
                Err(_) => {
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
            if let Some(w) = main_win
                .view_id_to_w
                .borrow()
                .get(&edit_view.view_id)
                .map(Clone::clone)
            {
                if let Some(page_num) = main_win.notebook.page_num(&w) {
                    main_win.notebook.remove_page(Some(page_num));
                }
                main_win.w_to_ev.borrow_mut().remove(&w);
            }
            main_win
                .view_id_to_w
                .borrow_mut()
                .remove(&edit_view.view_id);
            main_win.views.borrow_mut().remove(&edit_view.view_id);
            main_win.core.close_view(edit_view.view_id);
        }
        save_action
    }
}

/// Generate a new `Settings` object, which we pass to the `EditView` to set its behaviour.
pub fn new_settings() -> Settings {
    let gschema = GSchema::new("org.gnome.Tau");
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
        all_spaces: gschema.get_key("draw-all-spaces"),
        leading_spaces: gschema.get_key("draw-leading-spaces"),
        highlight_line: gschema.get_key("highlight-line"),
        right_margin: gschema.get_key("draw-right-margin"),
        column_right_margin: gschema.get_key("column-right-margin"),
        edit_font: gschema.get_key("font"),
        trailing_tabs: gschema.get_key("draw-trailing-tabs"),
        all_tabs: gschema.get_key("draw-all-tabs"),
        leading_tabs: gschema.get_key("draw-leading-tabs"),
        draw_cursor: gschema.get_key("draw-cursor"),
        interface_font,
        gschema,
    }
}

/// Connect changes in our `GSchema` to actions in Tau. E.g. when the `draw-trailing-spaces` key has
/// been modified we make sure to set this in the `MainState` (so that the `EditView`s actually notice
/// the change) and redraw the current one so that the user sees what has changed.
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
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-leading-spaces" => {
                    let val = gschema.get_key("draw-leading-spaces");
                    main_win.state.borrow_mut().settings.leading_spaces = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-all-spaces" => {
                    let val = gschema.get_key("draw-all-spaces");
                    main_win.state.borrow_mut().settings.all_spaces = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-trailing-tabs" => {
                    let val = gschema.get_key("draw-trailing-tabs");
                    main_win.state.borrow_mut().settings.trailing_tabs = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-leading-tabs" => {
                    let val = gschema.get_key("draw-leading-tabs");
                    main_win.state.borrow_mut().settings.leading_tabs = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-all-tabs" => {
                    let val = gschema.get_key("draw-all-tabs");
                    main_win.state.borrow_mut().settings.all_tabs = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "highlight-line" => {
                    let val = gschema.get_key("highlight-line");
                    main_win.state.borrow_mut().settings.highlight_line = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-right-margin" => {
                    let val = gschema.get_key("draw-right-margin");
                    main_win.state.borrow_mut().settings.right_margin = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "column-right-margin" => {
                    let val = gschema.get_key("column-right-margin");
                    main_win.state.borrow_mut().settings.column_right_margin = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-cursor" => {
                    let val = gschema.get_key("draw-cursor");
                    main_win.state.borrow_mut().settings.draw_cursor = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
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
                            ev.view_item.edit_area.queue_draw();
                        }
                    } else {
                        error!("{}. {}", gettext("Failed to get font configuration"), gettext("Resetting."));
                        gschema.settings.reset("font");
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
                "syntax-config" => {
                    let val = gschema.settings.get_strv("syntax-config");

                    for x in &val {
                        if let Ok(val) = serde_json::from_str(x.as_str()) {
                            core.notify(
                                "modify_user_config",
                                val,
                            );
                        } else {
                            error!("{}. {}", gettext("Failed to deserialize syntax config"), gettext("Resetting."));
                            gschema.settings.reset("syntax-config");
                        }
                    }

                    let syntax_config: HashMap<String, SyntaxParams> = val
                        .iter()
                        .map(GString::as_str)
                        .map(|s| {
                            serde_json::from_str(s)
                                .map_err(|e| error!("{} {}", gettext("Failed to deserialize syntax config"), e))
                                .unwrap()
                        })
                        .map(|sc: SyntaxParams| (sc.domain.syntax.clone(), sc))
                        .collect();

                    main_win.syntax_config.replace(syntax_config);
                }
                "theme-name" => {
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                },
                // We load these during startup
                "window-height" | "window-width" | "window-maximized" => {}
                key => {
                    warn!("{}: {}", gettext("Unknown key change event"), key)
                }
            }
        }));
}
