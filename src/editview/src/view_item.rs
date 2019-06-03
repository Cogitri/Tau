use crate::edit_view::EditView;
use crate::main_state::MainState;
use gdk::{Cursor, CursorType, DisplayManager, WindowExt};
use gettextrs::gettext;
use gio::Resource;
use glib::Bytes;
use gtk::*;
use log::{debug, trace};
use std::cell::RefCell;
use std::rc::Rc;

const RESOURCE: &[u8] = include_bytes!("ui/resources.gresource");

#[derive(Clone)]
pub struct EvBar {
    statusbar: Statusbar,
    pub syntax_menu_button: MenuButton,
    pub syntax_label: Label,
    pub syntax_treeview: TreeView,
    syntax_popover: Popover,
    pub line_label: Label,
    pub column_label: Label,
}

/// The ViewItem contains the various GTK parts related to the edit_area of the EditView
#[derive(Clone)]
pub struct ViewItem {
    pub root_box: Grid,
    pub ev_scrolled_window: ScrolledWindow,
    pub edit_area: gtk::Layout,
    pub linecount: gtk::Layout,
    pub hadj: Adjustment,
    pub vadj: Adjustment,
    pub statusbar: EvBar,
}

impl ViewItem {
    /// Sets up the drawing areas and scrollbars.
    pub fn new(main_state: &MainState) -> Self {
        let gbytes = Bytes::from_static(RESOURCE);
        let resource = Resource::new_from_data(&gbytes).unwrap();
        gio::resources_register(&resource);

        let builder = Builder::new_from_resource("/com/github/Cogitri/editview/ev.glade");

        let hadj = builder.get_object("hadj").unwrap();
        let vadj = builder.get_object("vadj").unwrap();
        let edit_area: gtk::Layout = builder.get_object("edit_area").unwrap();
        let linecount: gtk::Layout = builder.get_object("line_count").unwrap();
        let statusbar = EvBar {
            statusbar: builder.get_object("statusbar").unwrap(),
            syntax_treeview: builder.get_object("syntax_treeview").unwrap(),
            syntax_label: builder.get_object("syntax_label").unwrap(),
            syntax_popover: builder.get_object("syntax_popover").unwrap(),
            syntax_menu_button: builder.get_object("syntax_menu_button").unwrap(),
            line_label: builder.get_object("line_label").unwrap(),
            column_label: builder.get_object("column_label").unwrap(),
        };

        // Creation of a model with two rows.
        let list_model: ListStore = builder.get_object("syntax_liststore").unwrap();;

        for lang in main_state.avail_languages.iter() {
            // Localize 'Plain Text'
            if lang == "Plain Text" {
                let translated_plaintext = gettext("Plain Text");
                statusbar.syntax_label.set_text(&translated_plaintext);
                // Set Plain Text as selected item.
                let iter = list_model.insert_with_values(None, &[0], &[&translated_plaintext]);
                let path = list_model.get_path(&iter).unwrap();
                statusbar.syntax_treeview.get_selection().select_path(&path);
            } else {
                list_model.insert_with_values(None, &[0], &[lang]);
            }
        }

        let ev_scrolled_window = builder.get_object("ev_scrolled_window").unwrap();
        let hbox: Grid = builder.get_object("ev_root_widget").unwrap();
        hbox.show_all();

        Self {
            edit_area,
            linecount,
            hadj,
            vadj,
            statusbar,
            ev_scrolled_window,
            root_box: hbox,
        }
    }

    /// Sets up event listeners for the ViewItem
    pub fn connect_events(&self, edit_view: &Rc<RefCell<EditView>>) {
        trace!(
            "{} '{}'",
            edit_view.borrow().view_id,
            gettext("Connecting events of EditView")
        );

        self.ev_scrolled_window
            .connect_button_press_event(enclose!((edit_view) move |_,eb| {
                edit_view.borrow().handle_button_press(eb)
            }));

        self.edit_area
            .connect_draw(enclose!((edit_view) move |_,ctx| {
                edit_view.borrow().handle_da_draw(&ctx)
            }));

        self.ev_scrolled_window
            .connect_key_press_event(enclose!((edit_view) move |_, ek| {
                edit_view.borrow().handle_key_press_event(ek)
            }));

        self.ev_scrolled_window
            .connect_motion_notify_event(enclose!((edit_view) move |_,em| {
               edit_view.borrow().handle_drag(em)
            }));

        self.ev_scrolled_window.connect_realize(|w| {
            // Set the text cursor
            if let Some(disp) = DisplayManager::get().get_default_display() {
                let cur = Cursor::new_for_display(&disp, CursorType::Xterm);
                if let Some(win) = w.get_window() {
                    win.set_cursor(Some(&cur))
                }
            }
            w.grab_focus();
        });

        self.edit_area.connect_size_allocate(enclose!((edit_view) move |_,alloc| {
            debug!("{}: {}={} {}={}", gettext("Size changed to"), gettext("width"), alloc.width, gettext("height"), alloc.height);
            edit_view.borrow().da_size_allocate(alloc.width, alloc.height);
            edit_view.borrow().do_resize(edit_view.borrow().view_id,alloc.width, alloc.height);
        }));

        self.linecount
            .connect_draw(enclose!((edit_view) move |_,ctx| {
                edit_view.borrow().handle_linecount_draw(&ctx)
            }));

        self.statusbar
            .syntax_treeview
            .get_selection()
            .connect_changed(enclose!((edit_view) move |ts| {
                if let Some(syntax_tup) = ts.get_selected() {
                    let selected_syntax =  syntax_tup.0.get_value(&syntax_tup.1, 0);
                    if let Some(lang) = selected_syntax.get::<&str>() {
                        edit_view.borrow().view_item.statusbar.syntax_label.set_text(lang);
                        // We localized it ourselves, so we have to turn it into English again when sending it to Xi
                        if lang == gettext("Plain Text") {
                             edit_view.borrow().set_language("Plain Text");
                        } else {
                             edit_view.borrow().set_language(&lang);
                        }
                    }
                }
            }));

        self.ev_scrolled_window
            .connect_scroll_event(enclose!((edit_view) move |_,_| {
                edit_view.borrow().update_visible_scroll_region();
                Inhibit(false)
            }));

        // Make scrolling possible even when scrolling on the linecount
        self.linecount
            .connect_scroll_event(enclose!((edit_view) move |_,es| {
                    edit_view.borrow().view_item.ev_scrolled_window.emit("scroll-event", &[&es.to_value()]).unwrap();
                    Inhibit(false)
            }));
    }

    /// Gets the pango Context from the main drawing area.
    pub fn get_pango_ctx(&self) -> pango::Context {
        self.edit_area
            .get_pango_context()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango context")))
    }
}

/// Contains the top part of the EditView, tab widget and top bar.
pub struct TopBar {
    pub tab_widget: gtk::Box,
    pub label: Label,
    pub close_button: Button,
}

impl TopBar {
    /// Make the widgets for the tab
    pub fn new() -> Self {
        let builder = Builder::new_from_resource("/com/github/Cogitri/editview/close_tab.glade");
        let tab_widget: Box = builder.get_object("tab_widget").unwrap();
        let label = builder.get_object("tab_label").unwrap();
        let close_button = builder.get_object("close_button").unwrap();
        tab_widget.show_all();

        Self {
            tab_widget,
            label,
            close_button,
        }
    }
}

impl Default for TopBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Contains the Find & Replace elements
#[derive(Clone)]
pub struct FindReplace {
    pub search_bar: SearchBar,
    pub replace_revealer: Revealer,
    pub replace_entry: SearchEntry,
    pub replace_button: Button,
    pub replace_all_button: Button,
    pub find_status_label: Label,
    pub option_revealer: Revealer,
    pub search_entry: SearchEntry,
    pub go_down_button: Button,
    pub go_up_button: Button,
    pub popover: Popover,
    pub show_replace_button: ToggleButton,
    pub show_options_button: ToggleButton,
    pub use_regex_button: CheckButton,
    pub case_sensitive_button: CheckButton,
    pub whole_word_button: CheckButton,
}

impl FindReplace {
    /// Loads the glade description of the window, and builds gtk-rs objects.
    pub fn new(btn: &MenuButton) -> Self {
        let builder = Builder::new_from_resource("/com/github/Cogitri/editview/find_replace.glade");
        let search_bar = builder.get_object("search_bar").unwrap();
        let popover: Popover = builder.get_object("search_popover").unwrap();
        let replace_revealer: Revealer = builder.get_object("replace_revealer").unwrap();
        let option_revealer: Revealer = builder.get_object("option_revealer").unwrap();
        let replace_entry: SearchEntry = builder.get_object("replace_entry").unwrap();
        let replace_button = builder.get_object("replace_button").unwrap();
        let replace_all_button = builder.get_object("replace_all_button").unwrap();
        let find_status_label = builder.get_object("find_status_label").unwrap();
        let search_entry = builder.get_object("search_entry").unwrap();
        let go_down_button = builder.get_object("go_down_button").unwrap();
        let go_up_button = builder.get_object("go_up_button").unwrap();
        let use_regex_button = builder.get_object("use_regex_button").unwrap();
        let case_sensitive_button = builder.get_object("case_sensitive_button").unwrap();
        let whole_word_button = builder.get_object("whole_word_button").unwrap();
        let show_replace_button = builder.get_object("show_replace_button").unwrap();
        let show_options_button = builder.get_object("show_options_button").unwrap();

        popover.set_position(PositionType::Bottom);
        #[cfg(not(feature = "gtk_v3_22"))]
        popover.set_transitions_enabled(true);
        popover.set_relative_to(Some(btn));

        Self {
            replace_revealer,
            replace_entry,
            replace_button,
            replace_all_button,
            search_entry,
            go_down_button,
            go_up_button,
            use_regex_button,
            popover,
            show_replace_button,
            show_options_button,
            case_sensitive_button,
            whole_word_button,
            option_revealer,
            find_status_label,
            search_bar,
        }
    }

    /// Sets up event listeners
    pub fn connect_events(&self, ev: &Rc<RefCell<EditView>>) {
        trace!(
            "{} '{}'",
            gettext("Connecting FindReplace events for EditView"),
            ev.borrow().view_id
        );

        self.popover.connect_event(enclose!((ev) move |_, event| {
            ev.borrow().find_replace.search_bar.handle_event(event);

            Inhibit(false)
        }));

        self.popover.connect_closed(enclose!((ev) move |_| {
            ev.borrow().stop_search();
            ev.borrow().stop_replace();
        }));

        self.show_replace_button
            .connect_toggled(enclose!((ev) move |toggle_btn| {
                if toggle_btn.get_active() {
                    ev.borrow().show_replace();
                } else {
                    ev.borrow().hide_replace();
                }
            }));

        self.show_options_button
            .connect_toggled(enclose!((ev) move |toggle_btn| {
                if toggle_btn.get_active() {
                    ev.borrow().show_findreplace_opts();
                } else {
                    ev.borrow().hide_findreplace_opts();
                }
            }));

        self.search_bar.connect_entry(&self.search_entry);

        self.search_bar
            .connect_property_search_mode_enabled_notify(enclose!((ev) move |sb| {
                if ! sb.get_search_mode() {
                    ev.borrow().stop_search();
                }
            }));

        self.search_entry
            .connect_search_changed(enclose!((ev) move |w| {
                if let Some(text) = w.get_text() {
                    ev.borrow().search_changed(Some(text.to_string()));
                } else {
                    ev.borrow().search_changed(None);
                }
            }));

        self.replace_entry
            .connect_next_match(enclose!((ev) move |_| {
                ev.borrow().find_next();
            }));

        self.replace_entry
            .connect_previous_match(enclose!((ev) move |_| {
                ev.borrow().find_prev();
            }));

        self.replace_entry
            .connect_stop_search(enclose!((ev) move |_| {
                ev.borrow().stop_replace();
            }));

        let restart_search = move |edit_view: Rc<RefCell<EditView>>| {
            let text_opt = { edit_view.borrow().find_replace.search_entry.get_text() };
            if let Some(text) = text_opt {
                edit_view.borrow().search_changed(Some(text.to_string()));
            } else {
                edit_view.borrow().search_changed(None);
            }
        };

        self.use_regex_button
            .connect_toggled(enclose!((ev) move |_| restart_search(ev.clone())));

        self.whole_word_button
            .connect_toggled(enclose!((ev) move |_| restart_search(ev.clone())));

        self.case_sensitive_button
            .connect_toggled(enclose!((ev) move |_| restart_search(ev.clone())));

        self.search_entry.connect_activate(enclose!((ev) move |_| {
            ev.borrow().find_next();
        }));

        self.search_entry
            .connect_stop_search(enclose!((ev) move |_| {
                ev.borrow().stop_search();
            }));

        self.replace_button.connect_clicked(enclose!((ev) move |_| {
            ev.borrow().replace();
        }));

        self.replace_all_button
            .connect_clicked(enclose!((ev) move |_| {
                ev.borrow().replace_all();
            }));

        self.go_down_button.connect_clicked(enclose!((ev) move |_| {
            ev.borrow().find_next();
        }));

        self.go_up_button.connect_clicked(enclose!((ev) move |_| {
            ev.borrow().find_prev();
        }));
    }
}
