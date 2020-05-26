// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019 Tom Steu <steudtner.tom@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

use crate::about_win::AboutWin;
use crate::errors::{ErrorDialog, ErrorMsg};
use crate::functions;
use crate::prefs_win::PrefsWin;
use crate::session::SessionHandler;
use crate::shortcuts_win::ShortcutsWin;
use crate::syntax_config::SyntaxParams;
use crate::view_history::{ViewHistory, ViewHistoryExt};
use chrono::{DateTime, Utc};
use editview::{
    main_state::ShowInvisibles, theme::u32_from_color, EditView, EditViewExt, MainState,
};
use gdk::{enums::key, ModifierType, WindowState};
use gdk_pixbuf::Pixbuf;
use gettextrs::gettext;
use gio::prelude::*;
use gio::{ActionMapExt, ApplicationExt, Resource, Settings, SettingsExt, SimpleAction};
use glib::{clone, Bytes, GString, MainContext, Receiver, SpawnFlags};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Builder, Button, ButtonsType, DialogFlags, EventBox,
    FileChooserAction, FileChooserNative, HeaderBar, IconSize, MenuButton, MessageDialog,
    MessageType, Notebook, Orientation, Paned, PositionType, ResponseType, Revealer, Widget,
};
use log::{debug, error, info, trace, warn};
use serde_json::{self, json};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::convert::TryFrom;
use std::rc::Rc;
use tau_rpc::*;
use vte::{Terminal, TerminalExt};

pub const RESOURCE: &[u8] = include_bytes!("ui/resources.gresource");

/// Returned by an `ask_save_dialog` when we ask the user if he wants to either:
/// - `Save`(save unsaved changes and close view)
/// - `CloseWithoutSave` (discard pending changes and close view)
/// - `Cancel` (cancel the action and return to editing)
#[derive(Debug, PartialEq)]
pub enum SaveAction {
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
    /// The `gio::Settings` we save the fields of the WinProp to
    gschema: Settings,
}

impl WinProp {
    /// Create a new WinProp. Gets the `gio::Settings` of the name of the `Application`'s id
    ///
    /// # Panics
    ///
    /// This will panic if there's no `gio::Settings` of the name of the `Application`s id.
    pub fn new() -> Self {
        let gschema = Settings::new("org.gnome.Tau");
        Self {
            height: gschema.get("window-height"),
            width: gschema.get("window-width"),
            is_maximized: gschema.get("window-maximized"),
            gschema,
        }
    }

    /// Save the WinProp to the `WinProp.gschema`
    pub fn save(&self) {
        self.gschema.set("window-height", &self.height).unwrap();
        self.gschema.set("window-width", &self.width).unwrap();
        self.gschema
            .set("window-maximized", &self.is_maximized)
            .unwrap();
    }
}

/// Indicates which plugins, which have tight integration with Tau, have been started
#[derive(Debug, Default, PartialEq)]
pub struct StartedPlugins {
    /// Provides auto-indentation and syntax highlighting
    pub syntect: bool,
}

/// The `MainWin` is (as the name suggests) tau's main window. It holds buttons like `Open` and `Save`
/// and holds the `EditViews`, which do the actual editing. Refer to [the module level docs](main/index.html)
/// for more information.
pub struct MainWin {
    /// The handle to communicate with Xi.
    core: Rc<Client>,
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
    /// A `HashMap` containing the different configs for each syntax
    syntax_config: RefCell<HashMap<String, SyntaxParams>>,
    /// Indicates which special plugins (for which we have to do additional work) have been started
    started_plugins: RefCell<StartedPlugins>,
    /// The `GtkHeaderbar` of Tau, to set different titles
    header_bar: HeaderBar,
    /// Top bar when in fullscreen mode
    fullscreen_bar: HeaderBar,
    /// Revealer for fullscreen Topbar
    fullscreen_revealer: Revealer,
    /// Hamburger menu button of fullscreen headerbar
    fullscreen_hamburger_button: MenuButton,
    /// Whether or not the `MainWin` is saving right now
    saving: RefCell<bool>,
    /// Tab history
    view_history: Rc<RefCell<ViewHistory>>,
    /// If the window is in fullscreen mode
    fullscreen: Cell<bool>,
    /// The `GtkPaned` which holds the `vte::Terminal` and `GtkNoteBook`
    paned: Paned,
    /// Notebook holding the Terminals
    term_notebook: Notebook,
}

impl MainWin {
    /// Create a new `MainWin` instance, which facilitates Tau's buttons (like save/open) and
    /// bootstrap Tau
    pub fn new(
        // The `gio::Application` which this `MainWin` belongs to
        application: &Application,
        // The `xi-core` we can send commands to
        core: Rc<Client>,
        // The `Receiver` on which we receive messages from `xi-core`
        event_rx: Receiver<RpcOperations>,
    ) -> Rc<Self> {
        let gbytes = Bytes::from_static(RESOURCE);
        let resource = Resource::new_from_data(&gbytes).unwrap();
        gio::resources_register(&resource);

        // Add custom CSS, mainly to make the statusbar smaller
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/gnome/Tau/app.css");
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Failed to get default CssProvider!"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let builder = Builder::new_from_resource("/org/gnome/Tau/tau.glade");

        let properties = RefCell::new(WinProp::new());
        let window: ApplicationWindow = builder.get_object("appwindow").unwrap();

        if let Some(true) = application
            .get_application_id()
            .map(|id| id.ends_with("Devel"))
        {
            window.get_style_context().add_class("devel");
        }
        let header_bar = builder.get_object("header_bar").unwrap();
        let fullscreen_bar = builder.get_object("fullscreen_bar").unwrap();
        let fullscreen_revealer = builder.get_object("fullscreen_revealer").unwrap();
        let fullscreen_hamburger_button =
            builder.get_object("fullscreen_hamburger_button").unwrap();
        let paned: Paned = builder.get_object("paned").unwrap();

        let icon = Pixbuf::new_from_resource("/org/gnome/Tau/org.gnome.Tau.svg");
        window.set_icon(icon.ok().as_ref());

        if properties.borrow().is_maximized {
            window.maximize();
        } else {
            window.set_default_size(properties.borrow().width, properties.borrow().height);
        }

        let notebook: Notebook = builder.get_object("notebook").unwrap();

        let theme_name = properties.borrow().gschema.get("theme-name");
        debug!("Theme name: '{}'", &theme_name);

        let settings = functions::new_settings();

        let gschema = settings.gschema.clone();

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

        let syntax_changes = gschema.get_strv("syntax-config");
        let syntax_config: HashMap<String, SyntaxParams> = syntax_changes
            .iter()
            .map(GString::as_str)
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| error!("Failed to deserialize syntax config due to error: {}", e))
                    .unwrap()
            })
            .map(|sc: SyntaxParams| (sc.domain.syntax.clone(), sc))
            .collect();

        let view_history = ViewHistory::new(&notebook);

        let main_win = Rc::new(Self {
            core,
            window,
            notebook,
            builder,
            properties,
            header_bar,
            fullscreen_bar,
            fullscreen_revealer,
            fullscreen_hamburger_button,
            view_history,
            views: Default::default(),
            w_to_ev: Default::default(),
            view_id_to_w: Default::default(),
            state: main_state,
            syntax_config: RefCell::new(syntax_config),
            started_plugins: RefCell::new(Default::default()),
            saving: RefCell::new(false),
            fullscreen: Cell::new(false),
            paned,
            term_notebook: Notebook::new(),
        });

        main_win.term_notebook.set_tab_pos(PositionType::Bottom);

        main_win.connect_settings_change();

        main_win.window.set_application(Some(&application.clone()));

        // This is called when the window is closed with the 'X' or via the application menu, etc.
        main_win.window.connect_delete_event(
            clone!(@weak main_win => @default-panic, move |window, _| {
                // Only destroy the window when the user has saved the changes or closes without saving
                if main_win.close_all() == SaveAction::Cancel {
                    debug!("User chose to cancel exiting");
                    Inhibit(true)
                } else {
                    debug!("User chose to close the application");
                    main_win.properties.borrow().save();
                    window.destroy();
                    Inhibit(false)
                }
            }),
        );

        // Save to `WinProp` when the size of the window is changed
        main_win.window.connect_size_allocate(
            clone!(@weak main_win => @default-panic, move |window, _| {
                let win_size = window.get_size();
                let maximized = window.is_maximized();

                let mut properties = main_win.properties.borrow_mut();
                properties.is_maximized = maximized;
                if ! maximized {
                    properties.width = win_size.0;
                    properties.height = win_size.1;
                }
            }),
        );

        main_win.notebook.connect_switch_page(
            clone!(@weak main_win => @default-panic, move |_, w, _| {
                // adjust headerbar title
                main_win.update_titlebar(main_win.w_to_ev.borrow().get(w));

                // stop all searches and close dialogs
                main_win.views.borrow().values().for_each(|view| view.stop_search());
            }),
        );

        main_win.notebook.connect_page_removed(
            clone!(@weak main_win => @default-panic, move |notebook, _, _| {
                // Set a sensible title if no tab is open (and we can't display a
                // document's name)
                if notebook.get_n_pages() == 0 {
                    main_win.set_title(glib::get_application_name().unwrap().as_str());
                }
            }),
        );

        main_win.window.connect_focus_out_event(
            clone!(@weak main_win, @weak gschema => @default-panic, move |_, _| {
                // main_win.saving is true if we're currently saving via a save dialog, so don't try
                // to save again here
                if gschema.get("save-when-out-of-focus") && !*main_win.saving.borrow() {
                    for ev in main_win.views.borrow().values() {
                        let old_name = ev.file_name.borrow().clone();
                        match main_win.autosave_view(old_name, ev.view_id) {
                            Ok(file_name) => ev.set_file(&file_name),
                            Err(e) => {
                                let msg = ErrorMsg::new(e, false);
                                ErrorDialog::new(msg);
                            }
                        }
                    }
                }

                Inhibit(false)
            }),
        );

        main_win
            .window
            .connect_window_state_event(clone!(@weak main_win => @default-panic, move |_, event| {
                let fullscreen_mode = event.get_new_window_state().contains(WindowState::FULLSCREEN);

                if main_win.fullscreen.get() ^ fullscreen_mode {
                    main_win.fullscreen.set(fullscreen_mode);
                }

                Inhibit(false)
            }));

        let fullscreen_eventbox: EventBox =
            main_win.builder.get_object("fullscreen_eventbox").unwrap();

        fullscreen_eventbox.connect_enter_notify_event(
            clone!(@weak main_win => @default-panic, move |_, _| {
                if main_win.fullscreen.get() {
                    main_win.fullscreen_revealer.set_reveal_child(true);
                }
                Inhibit(false)
            }),
        );

        fullscreen_eventbox.connect_leave_notify_event(
            clone!(@weak main_win => @default-panic, move |_, _| {
                if !main_win.fullscreen_hamburger_button.get_active() {
                    main_win.fullscreen_revealer.set_reveal_child(false);
                }
                Inhibit(false)
            }),
        );

        // Below here we connect all actions, meaning that these closures will be run when the respective
        // action is triggered (e.g. by a button press)
        {
            let open_action = SimpleAction::new("open", None);
            open_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'open'");
                main_win.handle_open_button();
            }));
            application.add_action(&open_action);
        }
        {
            let new_action = SimpleAction::new("new", None);
            new_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'new'");
                main_win.req_new_view(None);
            }));
            application.add_action(&new_action);
        }
        {
            let prefs_action = SimpleAction::new("prefs", None);
            prefs_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'prefs'");
                main_win.prefs()
            }));
            application.add_action(&prefs_action);
        }
        {
            let about_action = SimpleAction::new("about", None);
            about_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'about'");
                main_win.about()
            }));
            application.add_action(&about_action);
        }
        {
            let multicursor_select_all_action = SimpleAction::new("multicursor_select_all", None);
            multicursor_select_all_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    trace!("Handling action: 'multicursor_select_all'");
                    main_win.multicursor_select_all();
                }),
            );
            application.add_action(&multicursor_select_all_action);
        }
        {
            let find_action = SimpleAction::new("find", None);
            find_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'find'");
                main_win.find();
            }));
            application.add_action(&find_action);
        }
        {
            let replace_action = SimpleAction::new("replace", None);
            replace_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'replace'");
                main_win.replace()
            }));
            application.add_action(&replace_action);
        }
        {
            let copy_action = SimpleAction::new("copy", None);
            copy_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'copy'");
                if let Some(ev) = main_win.get_current_edit_view() {
                    ev.do_copy()
                }
            }));
            application.add_action(&copy_action);
        }
        {
            let cut_action = SimpleAction::new("cut", None);
            cut_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'cut'");
                if let Some(ev) = main_win.get_current_edit_view() {
                    ev.do_cut()
                }
            }));
            application.add_action(&cut_action);
        }
        {
            let paste_action = SimpleAction::new("paste", None);
            paste_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'paste'");
                if let Some(ev) = main_win.get_current_edit_view() {
                    ev.do_paste()
                }
            }));
            application.add_action(&paste_action);
        }
        {
            let undo_action = SimpleAction::new("undo", None);
            undo_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'undo'");
                if let Some(ev) = main_win.get_current_edit_view() {
                    main_win.core.undo(ev.view_id);
                }
            }));
            application.add_action(&undo_action);
        }
        {
            let redo_action = SimpleAction::new("redo", None);
            redo_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'redo'");
                if let Some(ev) = main_win.get_current_edit_view() {
                    main_win.core.redo(ev.view_id);
                }
            }));
            application.add_action(&redo_action);
        }
        {
            let select_all_action = SimpleAction::new("select_all", None);
            select_all_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    trace!("Handling action: 'select_all'");
                    if let Some(ev) = main_win.get_current_edit_view() {
                        main_win.core.select_all(ev.view_id);
                    }
                }),
            );
            application.add_action(&select_all_action);
        }
        {
            let save_action = SimpleAction::new("save", None);
            save_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'save'");
                main_win.handle_save_button();
            }));
            application.add_action(&save_action);
        }
        {
            let save_as_action = SimpleAction::new("save_as", None);
            save_as_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'save_as'");
                main_win.current_save_as();
            }));
            application.add_action(&save_as_action);
        }
        {
            let save_all_action = SimpleAction::new("save_all", None);
            save_all_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'save_all'");
                main_win.save_all();
            }));
            application.add_action(&save_all_action);
        }
        {
            let close_action = SimpleAction::new("close", None);
            close_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'close'");
                main_win.close();
            }));
            application.add_action(&close_action);
        }
        {
            let shortcuts_action = SimpleAction::new("shortcuts", None);
            shortcuts_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_, _| {
                    trace!("Handling action: 'shortcuts'");
                    main_win.shortcuts();
                }),
            );
            application.add_action(&shortcuts_action);
        }
        {
            let increase_font_size_action = SimpleAction::new("increase_font_size", None);
            increase_font_size_action.connect_activate(clone!(@weak gschema => @default-panic, move |_,_| {
                let font: String = gschema.get("font");
                if let Some((name, mut size)) = functions::get_font_properties(&font) {
                    size += 1.0;
                    if size <= 72.0 {
                        gschema.set("font", &format!("{} {}", name, size)).map_err(|e| error!("Failed to increase font size due to error: '{}'", e)).unwrap();
                    }
                }
            }));
            application.add_action(&increase_font_size_action);
        }
        {
            let decrease_font_size_action = SimpleAction::new("decrease_font_size", None);
            decrease_font_size_action.connect_activate(clone!(@weak gschema => @default-panic, move |_,_| {
                let font: String = gschema.get("font");
                if let Some((name, mut size)) = functions::get_font_properties(&font) {
                    size -= 1.0;
                    if size >= 6.0 {
                        gschema.set("font", &format!("{} {}", name, size)).map_err(|e| error!("Failed to increase font size due to error: '{}'", e)).unwrap();
                    }
                }
            }));
            application.add_action(&decrease_font_size_action);
        }
        {
            // This is called when we run app.quit, e.g. via Ctrl+Q
            let quit_action = SimpleAction::new("quit", None);
            quit_action.connect_activate(clone!(@weak main_win => @default-panic, move |_,_| {
                trace!("Handling action: 'quit'");
                // Same as in connect_destroy, only quit if the user saves or wants to close without saving
                if main_win.close_all() == SaveAction::Cancel {
                    debug!("User chose to not quit application");
                } else {
                    debug!("User chose to quit application");
                    main_win.window.close();
                }
            }));
            application.add_action(&quit_action);
        }
        {
            let cycle_backward_action = SimpleAction::new("cycle_backward", None);
            cycle_backward_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    trace!("Handling action: 'cycle-backward'");
                    main_win.view_history.cycle_backward();
                }),
            );
            application.add_action(&cycle_backward_action);
        }
        {
            let cycle_forward_action = SimpleAction::new("cycle_forward", None);
            cycle_forward_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    trace!("Handling action: 'cycle-forward'");
                    main_win.view_history.cycle_forward();
                }),
            );
            application.add_action(&cycle_forward_action);
        }
        {
            let fullscreen_action = SimpleAction::new("toggle_fullscreen", None);
            fullscreen_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    trace!("Handling action: 'toggle_fullscreen'");
                    main_win.toggle_fullscreen();
                }),
            );
            application.add_action(&fullscreen_action);
        }
        {
            let show_terminal_action = SimpleAction::new("show_terminal", None);
            show_terminal_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    main_win.add_terminal(true);
                }),
            );
            application.add_action(&show_terminal_action);
        }
        {
            let go_to_line_action = SimpleAction::new("go_to_line", None);
            go_to_line_action.connect_activate(
                clone!(@weak main_win => @default-panic, move |_,_| {
                    trace!("Handling action: 'find_next'");
                    main_win.go_to_line();
                }),
            );
            application.add_action(&go_to_line_action);
        }

        // Put keyboard shortcuts here
        application.set_accels_for_action("app.find", &["<Primary>f"]);
        application.set_accels_for_action("app.save", &["<Primary>s"]);
        application.set_accels_for_action("app.save_as", &["<Primary><Shift>s"]);
        application.set_accels_for_action("app.new", &["<Primary>n"]);
        application.set_accels_for_action("app.open", &["<Primary>o"]);
        application.set_accels_for_action("app.quit", &["<Primary>q"]);
        application.set_accels_for_action("app.replace", &["<Primary>r"]);
        application.set_accels_for_action("app.close", &["<Primary>w"]);
        application.set_accels_for_action(
            "app.increase_font_size",
            &["<Primary>plus", "<Primary>KP_Add"],
        );
        application.set_accels_for_action(
            "app.decrease_font_size",
            &["<Primary>minus", "<Primary>KP_Subtract"],
        );
        application.set_accels_for_action("app.cycle_backward", &["<Primary>Tab"]);
        application.set_accels_for_action("app.cycle_forward", &["<Primary><Shift>Tab"]);
        application.set_accels_for_action("app.toggle_fullscreen", &["F11"]);
        application.set_accels_for_action("app.go_to_line", &["<Primary>i"]);
        application.set_accels_for_action("app.multicursor_select_all", &["<Primary>l"]);

        main_win.window.connect_key_press_event(
            clone!(@strong main_win => @default-panic, move |_, ek| {
                let key_val = ek.get_keyval();
                let ctrl = ek.get_state().contains(ModifierType::CONTROL_MASK);

                if let Some(edit_view) = main_win.get_current_edit_view() {
                    match key_val {
                        key::Return | key::KP_Enter if ctrl => {
                            Inhibit(edit_view.find_all())
                        },
                        key::g if ctrl => {
                            Inhibit(edit_view.find_next())
                        },
                        key::G if ctrl => {
                            Inhibit(edit_view.find_prev())
                        },
                        _ => {
                            Inhibit(false)
                        }
                    }
                } else {
                    Inhibit(false)
                }
            }),
        );

        let main_context = MainContext::default();

        event_rx.attach(
            Some(&main_context),
            clone!(@strong main_win => @default-panic, move |ev| {
                    main_win.handle_event(ev);
                    Continue(true)
            }),
        );

        debug!("Showing main window");
        main_win.window.show_all();

        main_win
    }
}

impl MainWin {
    /// Open an `ErrorDialog` when with the `Alert`'s msg
    pub fn alert(&self, params: tau_rpc::Alert) {
        ErrorDialog::new(ErrorMsg {
            msg: params.msg,
            fatal: false,
        });
    }

    /// Register the `AvailableThemes` with our `MainState`
    pub fn available_themes(&self, params: tau_rpc::AvailableThemes) {
        let mut state = self.state.borrow_mut();
        state.themes.clear();
        for theme in params.themes {
            state.themes.push(theme.to_string());
        }

        if !state.themes.contains(&state.theme_name) {
            error!(
                "Theme '{}' isn't available, setting to default",
                &state.theme_name,
            );

            if let Some(theme_name) = state.themes.first() {
                state
                    .settings
                    .gschema
                    .set("theme-name", &theme_name)
                    .unwrap_or_else(|e| {
                        error!("Failed to set theme name in GSettings due to error: {}", e)
                    });
                state.theme_name = theme_name.clone();
            } else {
                return;
            }
        }

        self.core.set_theme(&state.theme_name);
    }

    /// Change the theme in our `MainState`
    pub fn theme_changed(&self, params: tau_rpc::ThemeChanged) {
        // FIXME: Use annotations instead of constructing the selection style here
        let selection_style = Style {
            id: 0,
            fg_color: params.theme.selection_foreground.map(u32_from_color),
            bg_color: params.theme.selection.map(u32_from_color),
            weight: None,
            italic: None,
            underline: None,
        };

        for view in self.views.borrow().values() {
            view.theme_changed(&params);
        }

        let mut state = self.state.borrow_mut();
        state.theme = params.theme;
        state.styles.insert(0, selection_style);
    }

    /// Forward `ConfigChanged` to the respective `EditView`
    pub fn config_changed(&self, params: &tau_rpc::ConfigChanged) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.config_changed(&params.changes)
        }
    }

    /// Forward `FindStatus` to the respective `EditView`
    pub fn find_status(&self, params: &tau_rpc::FindStatus) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.find_status(&params.queries)
        }
    }

    /// Forward `ReplaceStatus` to the respective `EditView`
    pub fn replace_status(&self, params: &tau_rpc::ReplaceStatus) {
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            ev.replace_status(&params.status)
        }
    }

    /// Insert a style into our `MainState`
    pub fn def_style(&self, params: tau_rpc::Style) {
        let mut state = self.state.borrow_mut();
        state.styles.insert(params.id as usize, params);
    }

    /// Forward `Update` to the respective `EditView`
    pub fn update(&self, params: tau_rpc::Update) {
        trace!("Handling msg: 'update': {:?}", params);
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            let view_id = params.view_id;

            ev.update(params);

            if let Some(w) = self.view_id_to_w.borrow().get(&view_id).map(Clone::clone) {
                if let Some(page_num) = self.notebook.page_num(&w) {
                    if Some(page_num) == self.notebook.get_current_page() {
                        self.update_titlebar(self.views.borrow().get(&view_id));
                    }
                }
            }
        } else {
            error!("Received update for unknown EditView {}!", params.view_id);
        }
    }

    /// Forward `ScrollTo` to the respective `EditView`. Also set our `GtkNotebook`'s
    /// current page to that `EditView`
    pub fn scroll_to(&self, params: &tau_rpc::ScrollTo) {
        trace!("Handling msg: 'scroll_to' {:?}", params);

        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            let idx = self.notebook.page_num(&ev.root_widget);
            self.notebook.set_current_page(idx);
            ev.scroll_to(params.line, params.column);
        }
    }

    fn plugin_started(&self, params: &tau_rpc::PluginStarted) {
        if params.plugin == "xi-syntect-plugin" {
            self.started_plugins.borrow_mut().syntect = true;
            if let Some(ev) = self.views.borrow().get(&params.view_id) {
                ev.view_item
                    .statusbar
                    .insert_spaces_button
                    .set_sensitive(true);
                ev.view_item
                    .statusbar
                    .auto_indentation_button
                    .set_sensitive(true);
            }
        }
    }

    /// Open an error dialog if a plugin has crashed
    fn plugin_stopped(&self, params: &tau_rpc::PluginStopped) {
        if params.plugin == "xi-syntect-plugin" {
            self.started_plugins.borrow_mut().syntect = false;
            if let Some(ev) = self.views.borrow().get(&params.view_id) {
                ev.view_item
                    .statusbar
                    .insert_spaces_button
                    .set_sensitive(false);
                ev.view_item
                    .statusbar
                    .auto_indentation_button
                    .set_sensitive(false);
            }
        }
    }

    /// Measure the width of a string for Xi and send it the result. Used for line wrapping.
    pub fn measure_width(&self, params: tau_rpc::MeasureWidth) {
        trace!("Handling msg: 'measure_width' {:?}", params);
        if let Some(ev) = self.get_current_edit_view() {
            let mut widths = Vec::new();

            let mut id = 0;
            for mes_width in params.0 {
                id = mes_width.id;
                for string in &mes_width.strings {
                    widths.push(ev.line_width(string) as f32)
                }
            }

            self.core.width_measured(id, &[widths]);
        }
    }

    /// Set available syntaxes in our `MainState` and set the syntax_seletion_sensitivity
    /// of all `EditView`s, so it's unsensitive when we don't have any syntaxes to choose from.
    pub fn available_languages(&self, params: tau_rpc::AvailableLanguages) {
        debug!("Handling msg: 'available_languages' {:?}", params);
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
    pub fn language_changed(&self, params: &tau_rpc::LanguageChanged) {
        debug!("Handling msg: 'language_changed' {:?}", params);
        let views = self.views.borrow();
        if let Some(ev) = views.get(&params.view_id) {
            // Set the default_tab_size so the EditView
            if let Some(sc) = self.syntax_config.borrow().get(&params.language_id) {
                if let Some(tab_size) = sc.changes.tab_size {
                    debug!(
                        "Setting the following to the syntax attached tab size: '{}'",
                        tab_size,
                    );
                    ev.set_default_tab_size(tab_size);
                } else {
                    debug!("No tab size attached to the syntax");
                }
            }
            ev.language_changed(&params.language_id);
        }
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
            self.core.clone(),
            gschema,
            lang.as_deref(),
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

    fn go_to_line(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.start_go_to_line();
        }
    }

    fn multicursor_select_all(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.multicursor_select_all();
        }
    }

    /// Open the find dialog of the current `EditView`
    fn find(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            edit_view.start_search();
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
        info!("Couldn't get current EditView. This may only mean that you don't have an editing tab open right now.");
        None
    }

    fn autosave_view(&self, file_name: Option<String>, view_id: ViewId) -> Result<String, String> {
        if let Some(name) = file_name {
            self.core.save(view_id, &name);
            Ok(name)
        } else {
            let mut doc_dir = match dirs::data_dir() {
                Some(dir) => dir,
                None => {
                    return Err(gettext("Couldn’t get Documents directory to autosave unnamed file. Please make sure “XDG_DATA_DIR” or similar is set."));
                }
            };

            doc_dir.push("tau");
            doc_dir.push("autosave");

            if let Err(e) = std::fs::create_dir_all(&doc_dir) {
                return Err(format!("{}: {}", gettext("Couldn’t get Documents directory to autosave unnamed file. Please make sure “XDG_DATA_DIR” or similar is set."), e));
            }

            let now: DateTime<Utc> = Utc::now();
            let time_string = format!("tau-autosave-{}", now.format("%Y-%m-%d-%H-%M"));

            doc_dir.push(&time_string);

            // If file exists already, save it as Untitled.n
            let mut n = None::<u8>;
            while doc_dir.is_file() {
                if let Some(ref mut n) = n {
                    *n += 1;
                    doc_dir.set_file_name(&format!("{}.{}", time_string, n));
                } else {
                    n = Some(1);
                    doc_dir.set_file_name(&format!("{}.{}", time_string, n.unwrap()));
                }
            }

            let name = doc_dir.to_string_lossy().into_owned();
            self.core.save(view_id, &name);
            Ok(name)
        }
    }

    /// Set title of the main window
    fn set_title(&self, title: &str) {
        self.header_bar.set_title(Some(title));
        self.fullscreen_bar.set_title(Some(title));
    }

    /// Updates the title bar
    fn update_titlebar(&self, view: Option<&Rc<EditView>>) {
        if let Some(ev) = view {
            // Update window title
            let mut title = String::new();

            if !*ev.pristine.borrow() {
                title.push('*');
            }

            if let Some(ref path_string) = &*ev.file_name.borrow() {
                let path = std::path::Path::new(path_string);
                if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                    title.push_str(file_name);
                    if self.state.borrow().settings.full_title {
                        if let Some(parent_directory) = path.parent().and_then(|p| p.to_str()) {
                            title.push_str(&" — ");
                            title.push_str(parent_directory);
                        }
                    }
                } else {
                    title.push_str(&gettext("Untitled"));
                }
            } else {
                title.push_str(&gettext("Untitled"));
            }

            self.set_title(&title);
        } else {
            self.set_title(glib::get_application_name().unwrap().as_str());
        }
    }

    fn close_terminal(&self, term_num: u32) {
        self.term_notebook.remove_page(Some(term_num));
        if self.term_notebook.get_n_pages() < 2 {
            self.term_notebook.set_show_tabs(false);
        }
    }

    fn remove_terminal_area(&self) {
        self.paned.remove(&self.paned.get_child2().unwrap());
    }
}

/// An Extension trait for `MainWin`. This is implemented for `Rc<MainWin>`, allowing for a nicer
/// API (where we can do stuff like `self.close()` instead of `Self::close(main_win)`).
pub trait MainWinExt {
    fn add_terminal(&self, always_create_new: bool);

    fn close(&self) -> SaveAction;

    fn close_all(&self) -> SaveAction;

    fn close_view(&self, edit_view: &Rc<EditView>) -> SaveAction;

    fn connect_settings_change(&self);

    fn current_save_as(&self);

    fn handle_event(&self, ev: RpcOperations);

    fn handle_open_button(&self);

    fn handle_save_button(&self);

    fn req_new_view(&self, file_name: Option<String>);

    fn new_view(&self, res: Result<(ViewId, Option<String>), String>);

    fn new_view_response(&self, file_name: Option<String>, view_id: ViewId);

    fn save_all(&self);

    fn save_as(&self, edit_view: &Rc<EditView>);

    fn toggle_fullscreen(&self);
}

impl MainWinExt for Rc<MainWin> {
    /// Add a Terminal to the bottom of the `MainWin`. If `always_create_new` is true, this will only
    /// create a new terminal area if it doesn't exist yet. If it's false it adds a new terminal to the
    /// `GtkNotebook` holding the terminals.
    fn add_terminal(&self, always_create_new: bool) {
        let term = Terminal::new();
        let shell: String = self.state.borrow().settings.gschema.get("terminal-path");
        term.spawn_sync(
            vte::PtyFlags::DEFAULT,
            None,
            &[&std::path::Path::new(&shell)],
            &[],
            SpawnFlags::DEFAULT,
            Some(&mut || functions::vte_callback()),
            None::<&gio::Cancellable>,
        )
        .unwrap();

        let top_bar = editview::TopBar::new();

        let term_box = self.paned.get_child2().map(|w| w.downcast::<gtk::Box>());
        if term_box.as_ref().map(|x| x.as_ref().ok()).is_some() && always_create_new {
            return;
        }

        if let Some(Ok(_)) = term_box {
            let num = self.term_notebook.get_n_pages();

            top_bar
                .label
                .set_text(&format!("{} {}", gettext("Terminal"), num + 1));
            let page_num = self
                .term_notebook
                .insert_page(&term, Some(&top_bar.event_box), None);
            self.term_notebook.set_tab_reorderable(&term, true);

            top_bar.close_button.connect_clicked(
                clone!(@weak self as main_win, @strong page_num => @default-panic, move |_| {
                    main_win.close_terminal(page_num);
                }),
            );

            top_bar.event_box.connect_button_press_event(
                clone!(@weak self as main_win => @default-panic, move |_, eb| {
                    // 2 == middle click
                    if eb.get_button() == 2 {
                        main_win.close_terminal(page_num);
                    }
                    Inhibit(false)
                }),
            );
        } else {
            let termbox = gtk::Box::new(Orientation::Horizontal, 0);
            termbox.pack_start(&self.term_notebook, true, true, 0);
            self.paned.add2(&termbox);

            let buttonbox = gtk::Box::new(Orientation::Vertical, 0);
            let close_button =
                Button::new_from_icon_name(Some("window-close-symbolic"), IconSize::SmallToolbar);
            let add_button =
                Button::new_from_icon_name(Some("window-new-symbolic"), IconSize::SmallToolbar);
            buttonbox.pack_start(&add_button, false, false, 0);
            buttonbox.pack_start(&close_button, false, false, 0);
            termbox.pack_start(&buttonbox, false, false, 0);

            top_bar
                .label
                .set_text(&format!("{} {}", gettext("Terminal"), 1));
            let page_num = self
                .term_notebook
                .insert_page(&term, Some(&top_bar.event_box), None);

            close_button.connect_clicked(
                clone!(@weak self as main_win => @default-panic, move |_| {
                    main_win.remove_terminal_area();
                }),
            );

            add_button.connect_clicked(clone!(@weak self as main_win => @default-panic,move |_| {
                main_win.add_terminal(false);
            }));

            top_bar.close_button.connect_clicked(
                clone!(@weak self as main_win, @strong page_num => @default-panic,move |_| {
                    main_win.close_terminal(page_num);
                }),
            );

            top_bar.event_box.connect_button_press_event(
                clone!(@weak self as main_win => @default-panic, move |_, eb| {
                    // 2 == middle click
                    if eb.get_button() == 2 {
                        main_win.close_terminal(page_num);
                    }
                    Inhibit(false)
                }),
            );

            self.paned.show_all();
        }
        term.show();

        if self.term_notebook.get_n_pages() < 2 {
            self.term_notebook.set_show_tabs(false);
        } else {
            self.term_notebook.set_show_tabs(true);
        }
    }

    /// Close the current `EditView`
    ///
    /// # Returns
    ///
    /// - `SaveAction` determining if the `EdtiView` has been closed.
    fn close(&self) -> SaveAction {
        trace!("Closing current Editview");
        if let Some(edit_view) = self.get_current_edit_view() {
            let result = self.close_view(&edit_view);
            if result != SaveAction::Cancel {
                if let Some(path) = edit_view.file_name.borrow().as_ref() {
                    self.state
                        .borrow_mut()
                        .settings
                        .gschema
                        .session_remove(path);
                }
            }
            result
        } else {
            SaveAction::Cancel
        }
    }

    /// Close all `EditView`s, checking if the user wants to close them if there are unsaved changes
    ///
    /// # Returns
    ///
    /// - `SaveAction` determining if all `EditView`s have been closed.
    fn close_all(&self) -> SaveAction {
        trace!("Closing all EditViews");
        // Get all views that we currently have opened
        let views = { self.views.borrow().clone() };
        // Close each one of them
        let actions: Vec<SaveAction> = views
            .iter()
            .map(|(_, ev)| {
                let save_action = self.close_view(ev);
                if save_action != SaveAction::Cancel {
                    self.views.borrow_mut().remove(&ev.view_id);
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

    /// Close a specific `EditView`. Changes the `GtkNotebook` to the supplied `EditView`, so that the
    /// user can see which one is being closed. Presents the user a close dialog giving him the choice
    /// of either saving, aborting or closing without saving, if the `EditView` has unsaved changes.
    ///
    /// # Returns
    ///
    /// `SaveAction` determining which choice the user has made in the save dialog
    fn close_view(&self, edit_view: &Rc<EditView>) -> SaveAction {
        trace!("Closing Editview {}", edit_view.view_id);
        let save_action = if *edit_view.pristine.borrow() {
            // If it's pristine we don't ask the user if he really wants to quit because everything
            // is saved already and as such always close without saving
            SaveAction::CloseWithoutSave
        } else {
            // Change the tab to the EditView we want to ask the user about saving to give him a
            // change to review that action
            if let Some(w) = self
                .view_id_to_w
                .borrow()
                .get(&edit_view.view_id)
                .map(Clone::clone)
            {
                if let Some(page_num) = self.notebook.page_num(&w) {
                    self.notebook.set_property_page(page_num as i32);
                }
            }

            let ask_save_dialog = MessageDialog::new(
                Some(&self.window),
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
            self.saving.replace(true);
            let ret: i32 = ask_save_dialog.run().into();
            ask_save_dialog.destroy();
            self.saving.replace(false);
            match SaveAction::try_from(ret) {
                Ok(SaveAction::Save) => {
                    self.handle_save_button();
                    SaveAction::Save
                }
                Ok(SaveAction::CloseWithoutSave) => SaveAction::CloseWithoutSave,
                Err(_) => {
                    warn!("Save dialog has been destroyed before the user clicked a button");
                    SaveAction::Cancel
                }
                _ => SaveAction::Cancel,
            }
        };
        debug!("SaveAction: {:?}", save_action);

        if save_action != SaveAction::Cancel {
            if let Some(w) = self
                .view_id_to_w
                .borrow()
                .get(&edit_view.view_id)
                .map(Clone::clone)
            {
                if let Some(page_num) = self.notebook.page_num(&w) {
                    self.notebook.remove_page(Some(page_num));
                }
                self.w_to_ev.borrow_mut().remove(&w);
            }
            self.view_id_to_w.borrow_mut().remove(&edit_view.view_id);
            self.views.borrow_mut().remove(&edit_view.view_id);
            self.core.close_view(edit_view.view_id);

            // If we only have 0 or 1 EditViews left (and as such 0/1 tabs, which
            // means the user can't switch tabs anyway) don't display tabs
            if self.notebook.get_n_pages() < 2 {
                self.notebook.set_show_tabs(false);
            }
        }
        save_action
    }

    /// Connect changes in our `GSchema` to actions in Tau. E.g. when the `draw-trailing-spaces` key has
    /// been modified we make sure to set this in the `MainState` (so that the `EditView`s actually notice
    /// the change) and redraw the current one so that the user sees what has changed.
    fn connect_settings_change(&self) {
        let gschema = self.state.borrow_mut().settings.gschema.clone();
        let core = &self.core;
        gschema
            .connect_changed(clone!(@weak self as main_win, @weak gschema, @strong core => @default-panic, move |_, key| {
            trace!("Key '{}' has changed!", key);
            match key {
                "draw-trailing-spaces" | "draw-leading-spaces" | "draw-selection-spaces" | "draw-all-spaces" => {
                    main_win.state.borrow_mut().settings.draw_spaces = {
                        if gschema.get("draw-trailing-spaces") {
                            ShowInvisibles::Trailing
                        } else if gschema.get("draw-leading-spaces") {
                            ShowInvisibles::Leading
                        } else if gschema.get("draw-all-spaces") {
                            ShowInvisibles::All
                        } else if gschema.get("draw-selection-spaces") {
                            ShowInvisibles::Selected
                        } else {
                            ShowInvisibles::None
                        }
                    };

                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-trailing-tabs" | "draw-leading-tabs" | "draw-selection-tabs" | "draw-all-tabs" => {
                    main_win.state.borrow_mut().settings.draw_tabs = {
                        if gschema.get("draw-trailing-tabs") {
                            ShowInvisibles::Trailing
                        } else if gschema.get("draw-leading-tabs") {
                            ShowInvisibles::Leading
                        } else if gschema.get("draw-all-tabs") {
                            ShowInvisibles::All
                        } else if gschema.get("draw-selection-tabs") {
                            ShowInvisibles::Selected
                        } else {
                            ShowInvisibles::None
                        }
                    };

                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "highlight-line" => {
                    let val = gschema.get("highlight-line");
                    main_win.state.borrow_mut().settings.highlight_line = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-right-margin" => {
                    let val = gschema.get("draw-right-margin");
                    main_win.state.borrow_mut().settings.right_margin = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "column-right-margin" => {
                    let val = gschema.get("column-right-margin");
                    if val >= 1 && val <= 1000 {
                        main_win.state.borrow_mut().settings.column_right_margin = val;
                    }
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "draw-cursor" => {
                    let val = gschema.get("draw-cursor");
                    main_win.state.borrow_mut().settings.draw_cursor = val;
                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "translate-tabs-to-spaces" => {
                    let val: bool = gschema.get("translate-tabs-to-spaces");
                    core.modify_user_config_domain(
                        "general",
                        &json!({ "translate_tabs_to_spaces": val })
                    );
                }
                "auto-indent" => {
                    let val: bool = gschema.get("auto-indent");
                    core.modify_user_config_domain(
                        "general",
                        &json!({ "autodetect_whitespace": val })
                    );
                }
                "tab-size" => {
                    let val: u32 = gschema.get("tab-size");
                    if val >= 1 && val <= 100 {
                        core.modify_user_config_domain(
                            "general",
                            &json!({ "tab_size": val })
                        );
                    }
                }
                "font" => {
                    let val: String = gschema.get("font");
                    if let Some((font_name, font_size)) = functions::get_font_properties(&val) {
                        if font_size >= 6.0 && font_size <= 72.0 {
                            core.modify_user_config_domain(
                                "general",
                                &json!({ "font_face": font_name, "font_size": font_size })
                            );
                            main_win.state.borrow_mut().settings.edit_font = val;
                        }
                        if let Some(ev) = main_win.get_current_edit_view() {
                            ev.view_item.edit_area.queue_draw();
                        }
                    } else {
                        error!("Failed to get font configuration. Resetting...");
                        gschema.reset("font");
                    }
                }
                "use-tab-stops" => {
                    let val: bool = gschema.get("use-tab-stops");
                    core.modify_user_config_domain(
                        "general",
                        &json!({ "use_tab_stops": val })
                    );
                }
                "word-wrap" => {
                    let val: bool = gschema.get("word-wrap");
                    core.modify_user_config_domain(
                        "general",
                        &json!({ "word_wrap": val })
                    );
                }
                "syntax-config" => {
                    let val = gschema.get_strv("syntax-config");

                    for x in &val {
                        if let Ok(val) = serde_json::from_str(x.as_str()) {
                            core.as_ref().modify_user_config(val);
                        } else {
                            error!("Failed to deserialize syntax config. Resetting...");
                            gschema.reset("syntax-config");
                        }
                    }

                    let syntax_config: HashMap<String, SyntaxParams> = val
                        .iter()
                        .map(GString::as_str)
                        .map(|s| {
                            serde_json::from_str(s)
                                .map_err(|e| error!("Failed to deserialize syntax config due to error: '{}'", e))
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
                "show-linecount" => {
                    let val = gschema.get("show-linecount");
                    main_win.state.borrow_mut().settings.show_linecount = val;

                    for ev in main_win.w_to_ev.borrow().values() {
                        if val {
                            ev.view_item.linecount.show()
                        } else {
                            ev.view_item.linecount.hide()
                        }
                    }

                    if let Some(ev) = main_win.get_current_edit_view() {
                        ev.view_item.edit_area.queue_draw();
                    }
                }
                "full-title" => {
                    main_win.state.borrow_mut().settings.full_title = gschema.get("full-title");
                    main_win.update_titlebar(main_win.get_current_edit_view().as_ref());
                }
                "show-terminal" => {
                    if gschema.get("show-terminal") {
                        main_win.add_terminal(true);
                    } else {
                        main_win.remove_terminal_area();
                    }
                }
                // Valid keys, but no immediate action to be taken
                "window-height" | "window-width" | "window-maximized" | "save-when-out-of-focus" | "session" | "terminal-path" | "restore-session" => {}
                key => {
                    error!("Unknown GSettings key change event '{}'. Please make sure your GSchema is up-to-date.", key);
                }
            }
        }));
    }

    /// Open a filesaver dialog for the user to choose a name where to save the
    /// file and save to it.
    fn current_save_as(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            self.save_as(&edit_view);
        }
    }

    /// Display the FileChooserNative for opening, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn handle_open_button(&self) {
        let fcn = FileChooserNative::new(
            Some(gettext("Open a file to edit").as_str()),
            Some(&self.window),
            FileChooserAction::Open,
            Some(gettext("Open").as_str()),
            Some(gettext("Cancel").as_str()),
        );
        fcn.set_transient_for(Some(&self.window.clone()));
        fcn.set_select_multiple(true);

        if let Some(edit_view) = self.get_current_edit_view() {
            if let Some(ref file_name) = edit_view.file_name.borrow().clone() {
                if let Some(path) = std::path::Path::new(file_name).parent() {
                    fcn.set_current_folder(path);
                }
            }
        }

        fcn.connect_response(clone!(@weak self as main_win => @default-panic, move |fcd, res| {
            debug!(
                "FileChooserNative open response: '{:#?}'",
                res
            );

            if res == ResponseType::Accept {
                for file in fcd.get_filenames() {
                    let file_str = file.to_string_lossy().into_owned();
                    match std::fs::File::open(&file_str) {
                        Ok(_) => main_win.req_new_view(Some(file_str)),
                        Err(e) => {
                            let err_msg = format!("{} '{}': {}", &gettext("Couldn’t open file"), &file_str, &e.to_string());
                            ErrorDialog::new(ErrorMsg{msg: err_msg, fatal: false});
                        }
                    }
                }
            }
        }));

        self.saving.replace(true);
        fcn.run();
        self.saving.replace(false);
    }

    /// Save the `EditView`'s document if a filename is set, or open a filesaver
    /// dialog for the user to choose a name
    fn handle_save_button(&self) {
        if let Some(edit_view) = self.get_current_edit_view() {
            let name = { edit_view.file_name.borrow().clone() };
            if let Some(ref file_name) = name {
                self.core.save(edit_view.view_id, file_name);
            } else {
                self.save_as(&edit_view);
            }
        }
    }

    /// Request a new view from `xi-core` and send
    fn req_new_view(&self, file_name: Option<String>) {
        trace!("Requesting new view");

        let (tx, rx) = MainContext::channel(glib::source::PRIORITY_HIGH);

        rx.attach(
            None,
            clone!(@weak self as main_win, @strong file_name => @default-panic, move |res| {
                match res {
                    Ok(val) => main_win.new_view(Ok((serde_json::from_value(val).unwrap(), file_name.clone()))),
                    Err(e) => main_win.new_view(Err(serde_json::from_value(e).unwrap())),
                }
                glib::source::Continue(true)
            }),
        );

        self.core
            .new_view(file_name.as_ref(), move |res| tx.send(res).unwrap());
    }

    /// When `xi-core` tells us to create a new view, we have to connect the ways to close the `EditView`,
    /// either via a middle click or by clicking the X of the tab
    fn new_view_response(&self, file_name: Option<String>, view_id: ViewId) {
        trace!("Creating new EditView");

        let hamburger_button = self.builder.get_object("hamburger_button").unwrap();
        let edit_view = EditView::new(
            &self.state,
            self.core.clone(),
            &hamburger_button,
            file_name,
            view_id,
            &self.window,
        );

        if self.notebook.get_n_pages() == 0
            && self.state.borrow().settings.gschema.get("show-terminal")
        {
            self.add_terminal(true);
        }
        {
            let page_num = self.notebook.insert_page(
                &edit_view.root_widget,
                Some(&edit_view.top_bar.event_box),
                None,
            );
            self.notebook
                .set_tab_reorderable(&edit_view.root_widget, true);
            if let Some(w) = self.notebook.get_nth_page(Some(page_num)) {
                self.w_to_ev
                    .borrow_mut()
                    .insert(w.clone(), edit_view.clone());
                self.view_id_to_w.borrow_mut().insert(view_id, w);
            }

            edit_view.top_bar.close_button.connect_clicked(
                clone!(@weak self as main_win, @weak edit_view => @default-panic, move |_| {
                    if main_win.close_view(&edit_view) != SaveAction::Cancel {
                        if let Some(path) = edit_view.file_name.borrow().as_ref() {
                            main_win
                                .state
                                .borrow_mut()
                                .settings
                                .gschema
                                .session_remove(path);
                        }
                    }
                }),
            );

            #[allow(clippy::collapsible_if)]
            edit_view.top_bar.event_box.connect_button_press_event(
                clone!(@weak self as main_win, @weak edit_view => @default-panic, move |_, eb| {
                    // 2 == middle click
                    if eb.get_button() == 2 {
                        if main_win.close_view(&edit_view) != SaveAction::Cancel {
                            if let Some(path) = edit_view.file_name.borrow().as_ref() {
                                main_win
                                    .state
                                    .borrow_mut()
                                    .settings.gschema
                                    .session_remove(path);
                            }
                        }
                    }
                    Inhibit(false)
                }),
            );
        }
        if let Some(path) = edit_view.file_name.borrow().as_ref() {
            self.state
                .borrow_mut()
                .settings
                .gschema
                .session_add(path.clone());
        }
        self.views.borrow_mut().insert(view_id, edit_view);
    }

    fn save_all(&self) {
        for edit_view in self.views.borrow().values() {
            let name = { edit_view.file_name.borrow().clone() };
            if let Some(ref file_name) = name {
                self.core.save(edit_view.view_id, file_name);
            } else {
                self.save_as(&edit_view);
            }
        }
    }

    /// Display the FileChooserNative, send the result to the Xi core.
    /// Don't use FileChooserDialog here, it doesn't work for Flatpaks.
    /// This may call the GTK main loop.  There must not be any RefCell borrows out while this
    /// function runs.
    fn save_as(&self, edit_view: &Rc<EditView>) {
        let fcn = FileChooserNative::new(
            Some(gettext("Save file").as_str()),
            Some(&self.window),
            FileChooserAction::Save,
            Some(gettext("Save").as_str()),
            Some(gettext("Cancel").as_str()),
        );
        fcn.set_transient_for(Some(&self.window.clone()));
        fcn.set_do_overwrite_confirmation(true);
        if let Some(ref file_name) = *edit_view.file_name.borrow() {
            fcn.set_filename(file_name);
        } else {
            fcn.set_current_name("Untitled.txt");
        }

        fcn.connect_response(clone!(@weak self as main_win, @weak edit_view => @default-panic, move |fcd, res| {
            debug!(
                "FileChooserNative save response: '{:#?}'",
                res
            );

            if res == ResponseType::Accept {
                for file in fcd.get_filenames() {
                    let file_str = &file.to_string_lossy().into_owned();
                    if let Some(file) = fcd.get_filename() {
                        match &std::fs::OpenOptions::new().write(true).create(true).open(&file) {
                            Ok(_) => {
                                debug!("Saving file '{:?}'", &file);
                                let file = file.to_string_lossy();
                                main_win.core.save(edit_view.view_id, &file);
                                if let Some(old_file_name) = edit_view.file_name.borrow().as_ref() {
                                    main_win
                                    .state
                                    .borrow_mut()
                                    .settings
                                    .gschema
                                    .session_remove(old_file_name);
                                }

                                edit_view.set_file(&file);
                                main_win.state.borrow_mut().settings.gschema.session_add(file.to_string());
                            }
                        Err(e) => {
                            let err_msg = format!("{} '{}': {}", &gettext("Couldn’t save file"), &file_str, &e.to_string());
                            ErrorDialog::new(ErrorMsg {msg: err_msg, fatal: false});
                        }
                    }
                }
            }
                }
        }));

        if let Some(w) = self
            .view_id_to_w
            .borrow()
            .get(&edit_view.view_id)
            .map(Clone::clone)
        {
            if let Some(page_num) = self.notebook.page_num(&w) {
                self.notebook.set_property_page(page_num as i32);
            }
        }

        self.saving.replace(true);
        fcn.run();
        self.saving.replace(false);
    }

    /// Toggles fullscreen mode
    fn toggle_fullscreen(&self) {
        if self.fullscreen.get() {
            self.window.unfullscreen();
            self.fullscreen_revealer.set_reveal_child(false);
        } else {
            self.window.fullscreen();
        }
    }

    fn handle_event(&self, op: RpcOperations) {
        trace!("Handling msg: {:?}", op);
        match op {
            RpcOperations::Alert(alert) => self.alert(alert),
            RpcOperations::AvailableThemes(themes) => self.available_themes(themes),
            RpcOperations::ConfigChanged(config) => self.config_changed(&config),
            RpcOperations::DefStyle(style) => self.def_style(style),
            RpcOperations::FindStatus(status) => self.find_status(&status),
            RpcOperations::ReplaceStatus(status) => self.replace_status(&status),
            RpcOperations::Update(update) => self.update(update),
            RpcOperations::ScrollTo(scroll) => self.scroll_to(&scroll),
            RpcOperations::ThemeChanged(theme) => self.theme_changed(theme),
            RpcOperations::AvailableLanguages(langs) => self.available_languages(langs),
            RpcOperations::LanguageChanged(lang) => self.language_changed(&lang),
            RpcOperations::PluginStarted(plugin) => self.plugin_started(&plugin),
            RpcOperations::PluginStopped(plugin) => self.plugin_stopped(&plugin),
            RpcOperations::MeasureWidth(measure_width) => self.measure_width(measure_width),
            _ => {}
        }
    }

    fn new_view(&self, res: Result<(ViewId, Option<String>), String>) {
        match res {
            Ok((view_id, path)) => {
                if let Some(ref path_string) = path {
                    if let Some(name) = std::path::Path::new(path_string).file_name() {
                        self.set_title(&name.to_string_lossy());
                    }
                } else {
                    self.set_title(&gettext("Untitled"));
                }

                self.new_view_response(path, view_id);

                if self.notebook.get_n_pages() > 1 {
                    self.notebook.set_show_tabs(true);
                }
            }
            Err(e) => {
                ErrorDialog::new(ErrorMsg {
                    msg: e,
                    fatal: false,
                });
            }
        }
    }
}
