use crate::edit_view::EditView;
use gdk::{Cursor, CursorType, DisplayManager, WindowExt};
use gettextrs::gettext;
use gio::Resource;
use glib::Bytes;
use gtk::prelude::*;
use gtk::{
    Adjustment, Box, Builder, Button, CheckButton, EventBox, GestureDrag, Grid, Inhibit, Label,
    Layout, ListStore, Menu, MenuButton, Popover, PositionType, Revealer, ScrolledWindow,
    SearchBar, SearchEntry, SpinButton, Statusbar, ToggleButton, TreeView, Widget,
};
use log::{debug, trace};
use serde_json::json;
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
    list_model: ListStore,
    edit_settings_popover: Popover,
    pub auto_indention_button: ToggleButton,
    pub insert_spaces_button: ToggleButton,
    pub tab_size_button: SpinButton,
    pub tab_width_label: Label,
}

#[derive(Clone)]
pub struct Gestures {
    pub drag: GestureDrag,
    drag_data: Rc<RefCell<DragData>>,
}

struct DragData {
    start_x: f64,
    start_y: f64,
}
/// The `ViewItem` contains the various GTK parts related to the `edit_area` of the `EditView`
#[derive(Clone)]
pub struct ViewItem {
    pub root_box: Grid,
    pub ev_scrolled_window: ScrolledWindow,
    pub edit_area: Layout,
    pub linecount: Layout,
    pub hadj: Adjustment,
    pub vadj: Adjustment,
    pub statusbar: EvBar,
    pub context_menu: Menu,
    pub gestures: Gestures,
}

impl ViewItem {
    /// Sets up the drawing areas and scrollbars.
    pub fn new(tab_size: u32) -> Self {
        let gbytes = Bytes::from_static(RESOURCE);
        let resource = Resource::new_from_data(&gbytes).unwrap();
        gio::resources_register(&resource);

        let builder = Builder::new_from_resource("/org/gnome/Tau/editview/ev.glade");

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
            list_model: builder.get_object("syntax_liststore").unwrap(),
            edit_settings_popover: builder.get_object("edit_settings_popover").unwrap(),
            auto_indention_button: builder
                .get_object("edit_settings_automatic_indention_checkbutton")
                .unwrap(),
            insert_spaces_button: builder
                .get_object("edit_settings_insert_spaces_checkbutton")
                .unwrap(),
            tab_size_button: builder
                .get_object("edit_settings_tab_size_spinbutton")
                .unwrap(),
            tab_width_label: builder.get_object("tab_width_label").unwrap(),
        };
        statusbar
            .tab_width_label
            .set_text(&format!("{}: {}", gettext("Tab Size"), tab_size));
        statusbar.tab_size_button.set_value(f64::from(tab_size));

        let context_menu_builder =
            Builder::new_from_resource("/org/gnome/Tau/editview/context_menu.glade");
        let gmenu: gio::Menu = context_menu_builder.get_object("context_menu").unwrap();
        let context_menu = gtk::Menu::new_from_model(&gmenu);
        //FIXME: This should take IsA<Widget> so we don't have to upcast to a widget
        context_menu.set_property_attach_widget(Some(&edit_area.clone().upcast::<Widget>()));

        let ev_scrolled_window: ScrolledWindow = builder.get_object("ev_scrolled_window").unwrap();
        let drag = GestureDrag::new(&ev_scrolled_window);
        let hbox: Grid = builder.get_object("ev_root_widget").unwrap();
        hbox.show_all();

        Self {
            edit_area,
            linecount,
            hadj,
            vadj,
            statusbar,
            ev_scrolled_window,
            context_menu,
            root_box: hbox,
            gestures: Gestures {
                drag,
                drag_data: Rc::new(RefCell::new(DragData {
                    start_x: 0.0,
                    start_y: 0.0,
                })),
            },
        }
    }

    /// Sets up event listeners for the ViewItem
    pub fn connect_events(&self, edit_view: &Rc<EditView>) {
        trace!(
            "{} '{}'",
            edit_view.view_id,
            gettext("Connecting events of EditView")
        );

        self.ev_scrolled_window
            .connect_button_press_event(enclose!((edit_view) move |_,eb| {
                edit_view.handle_button_press(eb)
            }));

        self.edit_area
            .connect_draw(enclose!((edit_view) move |_,ctx| {
                edit_view.handle_da_draw(ctx)
            }));

        self.ev_scrolled_window
            .connect_key_press_event(enclose!((edit_view) move |_, ek| {
                edit_view.handle_key_press_event(ek)
            }));

        self.gestures.drag.connect_drag_begin(
            enclose!((self.gestures.drag_data => drag_data) move |_, start_x, start_y| {
                let new_data = DragData {
                    start_x,
                    start_y,
                };
                drag_data.replace(new_data);
            }),
        );

        self.gestures.drag.connect_drag_update(
            enclose!((edit_view, self.gestures.drag_data => drag_data) move |_, offset_x, offset_y| {
                let drag_data = drag_data.borrow();
                edit_view.handle_drag(drag_data.start_x + offset_x, drag_data.start_y + offset_y);
            }),
        );

        self.gestures.drag.connect_drag_end(
            enclose!((edit_view, self.gestures.drag_data => drag_data) move |_, offset_x, offset_y| {
                let drag_data = drag_data.borrow();
                edit_view.handle_drag(drag_data.start_x + offset_x, drag_data.start_y + offset_y);
                edit_view.do_copy_primary();
            }),
        );

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
            edit_view.da_size_allocate(alloc.width, alloc.height);
            edit_view.do_resize(alloc.width, alloc.height);
        }));

        self.linecount
            .connect_draw(enclose!((edit_view) move |_,ctx| {
                edit_view.handle_linecount_draw(ctx)
            }));

        self.statusbar
            .syntax_treeview
            .get_selection()
            .connect_changed(enclose!((edit_view) move |ts| {
                if let Some(syntax_tup) = ts.get_selected() {
                    let selected_syntax =  syntax_tup.0.get_value(&syntax_tup.1, 0);
                    if let Some(lang) = selected_syntax.get::<&str>() {
                        // DONT set the lang if we already selected it, otherwise we way loop here!
                        if lang == edit_view.view_item.statusbar.syntax_label.get_text().unwrap().as_str() {
                            return;
                        }
                        edit_view.view_item.statusbar.syntax_label.set_text(lang);
                        // We localized it ourselves, so we have to turn it into English again when sending it to Xi
                        if lang == gettext("Plain Text") {
                             edit_view.set_language("Plain Text");
                        } else {
                             edit_view.set_language(lang);
                        }
                    }
                }
            }));

        self.statusbar
            .tab_size_button
            .connect_value_changed(enclose!((edit_view) move |sb| {
                // We only allow vals that fit in a u32 via es_tab_size_spinbutton_adj
                let val = sb.get_value() as u32;
                edit_view.view_item.statusbar.tab_width_label.set_text(
                    &format!("{}: {}",
                    gettext("Tab Size"),
                    &val,
                ));

                if *edit_view.default_tab_size.borrow() == val {
                    edit_view.tab_size.replace(None);
                } else {
                    edit_view.tab_size.replace(Some(val));
                }

                edit_view.core.notify(
                    "modify_user_config",
                    json!({
                        "domain": { "user_override": edit_view.view_id },
                        "changes": { "tab_size": val },
                    }),
                );
            }));

        self.statusbar
            .insert_spaces_button
            .connect_toggled(enclose!(
                (edit_view) move | tb | {
                    let val = tb.get_active();
                    edit_view.core.notify(
                        "modify_user_config",
                        json!({
                            "domain": { "user_override": edit_view.view_id },
                            "changes": { "translate_tabs_to_spaces": val },
                        }),
                    );
                }
            ));

        self.statusbar
            .auto_indention_button
            .connect_toggled(enclose!(
                (edit_view) move | tb | {
                    let val = tb.get_active();
                    edit_view.core.notify(
                        "modify_user_config",
                        json!({
                            "domain": { "user_override": edit_view.view_id },
                            "changes": { "autodetect_whitespace": val },
                        }),
                    );
                }
            ));

        self.ev_scrolled_window
            .get_vadjustment()
            .unwrap()
            .connect_value_changed(enclose!((edit_view) move |_| {
                edit_view.update_visible_scroll_region();
            }));

        // Make scrolling possible even when scrolling on the linecount
        self.linecount
            .connect_scroll_event(enclose!((edit_view) move |_,es| {
                    edit_view.view_item.ev_scrolled_window.emit("scroll-event", &[&es.to_value()]).unwrap();
                    Inhibit(false)
            }));
    }

    /// Gets the pango Context from the main drawing area.
    pub fn get_pango_ctx(&self) -> pango::Context {
        self.edit_area
            .get_pango_context()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango context")))
    }

    pub fn set_avail_langs<T>(&self, langs: &[T])
    where
        T: AsRef<str> + PartialEq<&'static str> + glib::value::SetValue,
    {
        for lang in langs {
            // Localize 'Plain Text'
            if lang == &"Plain Text" {
                let translated_plaintext = gettext("Plain Text");
                self.statusbar.syntax_label.set_text(&translated_plaintext);
                // Set Plain Text as selected item.
                let iter = self.statusbar.list_model.insert_with_values(
                    None,
                    &[0],
                    &[&translated_plaintext],
                );
                let path = self.statusbar.list_model.get_path(&iter).unwrap();
                self.statusbar
                    .syntax_treeview
                    .get_selection()
                    .select_path(&path);
            } else {
                self.statusbar
                    .list_model
                    .insert_with_values(None, &[0], &[lang]);
            }
        }
    }
}

/// Contains the top part of the `EditView`, tab widget and top bar.
pub struct TopBar {
    pub tab_widget: gtk::Box,
    pub label: Label,
    pub close_button: Button,
    pub event_box: EventBox,
}

impl TopBar {
    /// Make the widgets for the tab
    pub fn new() -> Self {
        let builder = Builder::new_from_resource("/org/gnome/Tau/editview/close_tab.glade");
        let tab_widget: Box = builder.get_object("tab_widget").unwrap();
        let label = builder.get_object("tab_label").unwrap();
        let close_button = builder.get_object("close_button").unwrap();
        let event_box = builder.get_object("tab_event_box").unwrap();
        tab_widget.show_all();

        Self {
            tab_widget,
            label,
            close_button,
            event_box,
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
        let builder = Builder::new_from_resource("/org/gnome/Tau/editview/find_replace.glade");
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
    pub fn connect_events(&self, ev: &Rc<EditView>) {
        trace!(
            "{} '{}'",
            gettext("Connecting FindReplace events for EditView"),
            ev.view_id
        );

        self.popover.connect_event(enclose!((ev) move |_, event| {
            ev.find_replace.search_bar.handle_event(event);

            Inhibit(false)
        }));

        self.popover.connect_closed(enclose!((ev) move |_| {
            ev.stop_search();
            ev.stop_replace();
        }));

        self.show_replace_button
            .connect_toggled(enclose!((ev) move |toggle_btn| {
                if toggle_btn.get_active() {
                    ev.show_replace();
                } else {
                    ev.hide_replace();
                }
            }));

        self.show_options_button
            .connect_toggled(enclose!((ev) move |toggle_btn| {
                if toggle_btn.get_active() {
                    ev.show_findreplace_opts();
                } else {
                    ev.hide_findreplace_opts();
                }
            }));

        self.search_bar.connect_entry(&self.search_entry);

        self.search_bar
            .connect_property_search_mode_enabled_notify(enclose!((ev) move |sb| {
                if ! sb.get_search_mode() {
                    ev.stop_search();
                }
            }));

        self.search_entry
            .connect_search_changed(enclose!((ev) move |w| {
                if let Some(text) = w.get_text() {
                    ev.search_changed(Some(text.to_string()));
                } else {
                    ev.search_changed(None);
                }
            }));

        self.replace_entry
            .connect_next_match(enclose!((ev) move |_| {
                ev.find_next();
            }));

        self.replace_entry
            .connect_previous_match(enclose!((ev) move |_| {
                ev.find_prev();
            }));

        self.replace_entry
            .connect_stop_search(enclose!((ev) move |_| {
                ev.stop_replace();
            }));

        let restart_search = move |edit_view: &Rc<EditView>| {
            let text_opt = { edit_view.find_replace.search_entry.get_text() };
            if let Some(text) = text_opt {
                edit_view.search_changed(Some(text.to_string()));
            } else {
                edit_view.search_changed(None);
            }
        };

        self.use_regex_button
            .connect_toggled(enclose!((ev) move |_| restart_search(&ev)));

        self.whole_word_button
            .connect_toggled(enclose!((ev) move |_| restart_search(&ev)));

        self.case_sensitive_button
            .connect_toggled(enclose!((ev) move |_| restart_search(&ev)));

        self.search_entry.connect_activate(enclose!((ev) move |_| {
            ev.find_next();
        }));

        self.search_entry
            .connect_stop_search(enclose!((ev) move |_| {
                ev.stop_search();
            }));

        self.replace_button.connect_clicked(enclose!((ev) move |_| {
            ev.replace();
        }));

        self.replace_all_button
            .connect_clicked(enclose!((ev) move |_| {
                ev.replace_all();
            }));

        self.go_down_button.connect_clicked(enclose!((ev) move |_| {
            ev.find_next();
        }));

        self.go_up_button.connect_clicked(enclose!((ev) move |_| {
            ev.find_prev();
        }));
    }
}
