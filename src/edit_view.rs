use crate::linecache::{Line, LineCache, StyleSpan};
use crate::main_win::MainState;
use crate::pref_storage::*;
use crate::rpc::Core;
use crate::theme::{color_from_u32, set_margin_source_color, set_source_color, PangoColor};
use cairo::Context;
use gdk::enums::key;
use gdk::*;
use gettextrs::gettext;
use glib::{source, MainContext};
use gtk::{self, *};
use log::{debug, error, trace, warn};
use pango::{self, ContextExt, LayoutExt, *};
use pangocairo::functions::*;
use serde_json::Value;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::rc::Rc;
use std::u32;

/// The `Font` Struct holds all information about the font used in the `EditView` for the editing area
/// or the interface font (used for the linecount)
pub struct Font {
    font_height: f64,
    font_width: f64,
    font_ascent: f64,
    font_descent: f64,
    font_desc: FontDescription,
}

impl Font {
    fn new(pango_ctx: &pango::Context, font_desc: FontDescription) -> Self {
        pango_ctx.set_font_description(&font_desc);
        // FIXME: Just use en-US lang here, otherwise FontMetrics may be different (as in font_ascent/
        // font_descent being larger to account for the language's special signs), which breaks cursor positioning.
        let fontset = pango_ctx
            .load_fontset(&font_desc, &Language::from_string("en-US"))
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font set")));
        let metrics = fontset
            .get_metrics()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font metrics")));

        let layout = pango::Layout::new(pango_ctx);
        layout.set_text("a");
        let (_, log_extents) = layout.get_extents();
        debug!("{}: {:?}", gettext("Pango font size"), log_extents);

        let font_height = f64::from(log_extents.height) / f64::from(pango::SCALE);
        let font_width = f64::from(log_extents.width) / f64::from(pango::SCALE);
        let font_ascent = f64::from(metrics.get_ascent()) / f64::from(pango::SCALE);
        let font_descent = f64::from(metrics.get_descent()) / f64::from(pango::SCALE);

        debug!(
            "{}: {} {} {} {}",
            gettext("Font metrics"),
            font_width,
            font_height,
            font_ascent,
            font_descent
        );

        Self {
            font_height,
            font_width,
            font_ascent,
            font_descent,
            font_desc,
        }
    }
}

/// The ViewItem contains the drawing areas and scrollbars of the EditView.
#[derive(Clone)]
pub struct ViewItem {
    pub edit_area: DrawingArea,
    linecount: DrawingArea,
    horiz_bar: Scrollbar,
    verti_bar: Scrollbar,
}

impl ViewItem {
    /// Sets up the drawing areas and scrollbars.
    fn new() -> Self {
        let edit_area = DrawingArea::new();
        let linecount = DrawingArea::new();
        let horiz_bar = Scrollbar::new(Orientation::Horizontal, None::<&gtk::Adjustment>);
        let verti_bar = Scrollbar::new(Orientation::Vertical, None::<&gtk::Adjustment>);

        edit_area.set_events(
            EventMask::BUTTON_PRESS_MASK
                | EventMask::BUTTON_RELEASE_MASK
                | EventMask::BUTTON_MOTION_MASK
                | EventMask::SCROLL_MASK
                | EventMask::SMOOTH_SCROLL_MASK,
        );
        debug!("{}: {:?}", gettext("Events"), edit_area.get_events());
        edit_area.set_can_focus(true);

        Self {
            edit_area,
            linecount,
            horiz_bar,
            verti_bar,
        }
    }

    /// Sets up event listeners for the ViewItem
    fn connect_events(&self, edit_view: &Rc<RefCell<EditView>>) {
        trace!(
            "{} '{}'",
            edit_view.borrow().view_id,
            gettext("Connecting events of EditView")
        );
        self.edit_area
            .connect_button_press_event(clone!(edit_view => move |_,eb| {
                edit_view.borrow().handle_button_press(eb)
            }));

        self.edit_area
            .connect_draw(clone!(edit_view => move |_,ctx| {
                edit_view.borrow_mut().handle_da_draw(&ctx)
            }));

        self.edit_area
            .connect_key_press_event(clone!(edit_view => move |_, ek| {
                edit_view.borrow().handle_key_press_event(ek)
            }));

        self.edit_area
            .connect_motion_notify_event(clone!(edit_view => move |_,em| {
                edit_view.borrow_mut().handle_drag(em)
            }));

        self.edit_area.connect_realize(|w| {
            // Set the text cursor
            if let Some(disp) = DisplayManager::get().get_default_display() {
                let cur = Cursor::new_for_display(&disp, CursorType::Xterm);
                if let Some(win) = w.get_window() {
                    win.set_cursor(Some(&cur))
                }
            }
            w.grab_focus();
        });

        self.edit_area
            .connect_scroll_event(clone!(edit_view => move |_,es| {
                edit_view.borrow_mut().handle_scroll(es)
            }));

        self.edit_area.connect_size_allocate(clone!(edit_view => move |_,alloc| {
            debug!("{}: {}={} {}={}", gettext("Size changed to"), gettext("width"), alloc.width, gettext("height"), alloc.height);
            edit_view.borrow_mut().da_size_allocate(alloc.width, alloc.height);
            edit_view.borrow().do_resize(&edit_view.borrow().view_id,alloc.width, alloc.height);
        }));

        self.linecount
            .connect_draw(clone!(edit_view => move |_,ctx| {
                edit_view.borrow_mut().handle_linecount_draw(&ctx)
            }));

        self.verti_bar
            .connect_change_value(clone!(edit_view => move |_,_,value| {
                edit_view.borrow_mut().vscrollbar_change_value(value)
            }));
    }

    /// Gets the pango Context from the main drawing area.
    fn get_pango_ctx(&self) -> pango::Context {
        self.edit_area
            .get_pango_context()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango context")))
    }
}

/// Returned by `EditView::get_text_size()` and used to adjust the scrollbars.
pub struct TextSize {
    /// The height of the entire document
    height: f64,
    /// The width of the entire document
    width: f64,
    /// If the height of the document is contained within the edit_area (if it's smaller)
    contained_height: bool,
    /// If the width of the document is contained within the edit_area (if it's smaller)
    contained_width: bool,
}

/// The EditView is the part of gxi that does the actual editing. This is where you edit documents.
pub struct EditView {
    core: Rc<RefCell<Core>>,
    main_state: Rc<RefCell<MainState>>,
    pub view_id: String,
    pub file_name: Option<String>,
    pub pristine: bool,
    pub root_widget: gtk::Box,
    pub top_bar: TopBar,
    pub view_item: ViewItem,
    line_cache: LineCache,
    find_replace: FindReplace,
    edit_font: Font,
    interface_font: Font,
    im_context: IMContextSimple,
}

impl EditView {
    /// Initialises a new EditView. Sets up scrollbars, the actual editing area, the fonts,
    /// the syntax lang and connects all events which might happen during usage (e.g. scrolling)
    pub fn new(
        main_state: &Rc<RefCell<MainState>>,
        core: &Rc<RefCell<Core>>,
        // The FindReplace dialog is relative to this
        hamburger_button: &MenuButton,
        file_name: Option<String>,
        view_id: &str,
        parent: &ApplicationWindow,
    ) -> Rc<RefCell<Self>> {
        trace!("{}, '{}'", gettext("Creating new EditView"), view_id);
        let view_item = ViewItem::new();
        let find_replace = FindReplace::new(&hamburger_button);
        let pango_ctx = view_item.get_pango_ctx();
        let im_context = IMContextSimple::new();

        let edit_view = Rc::new(RefCell::new(Self {
            core: core.clone(),
            main_state: main_state.clone(),
            file_name,
            pristine: true,
            view_id: view_id.to_string(),
            root_widget: Self::setup_root_box(&view_item),
            top_bar: TopBar::new(),
            view_item: view_item.clone(),
            line_cache: LineCache::new(),
            edit_font: Self::get_edit_font(&pango_ctx, &main_state.borrow().config),
            interface_font: Self::get_interface_font(&pango_ctx),
            find_replace: find_replace.clone(),
            im_context: im_context.clone(),
        }));

        edit_view.borrow_mut().update_title();
        crate::MainWin::set_language(&core, view_id, "Plain Text");

        view_item.connect_events(&edit_view);
        find_replace.connect_events(&edit_view);
        EditView::connect_im_events(&edit_view, &im_context);

        im_context.set_client_window(parent.get_window().as_ref());

        edit_view
    }

    fn connect_im_events(edit_view: &Rc<RefCell<EditView>>, im_context: &IMContextSimple) {
        im_context.connect_commit(clone!(edit_view => move |_, text| {
            let ev = edit_view.borrow();
            ev.core.borrow().insert(&ev.view_id, text);
        }));
    }

    fn setup_root_box(view_item: &ViewItem) -> Box {
        let root_box = Box::new(Orientation::Vertical, 0);
        let hbox = Box::new(Orientation::Horizontal, 0);
        let vbox = Box::new(Orientation::Vertical, 0);
        root_box.pack_start(&hbox, true, true, 0);
        hbox.pack_start(&view_item.linecount, false, false, 0);
        hbox.pack_start(&vbox, true, true, 0);
        hbox.pack_start(&view_item.verti_bar, false, false, 0);
        vbox.pack_start(&view_item.edit_area, true, true, 0);
        vbox.pack_start(&view_item.horiz_bar, false, false, 0);
        root_box.show_all();

        root_box
    }

    fn get_interface_font(pango_ctx: &pango::Context) -> Font {
        Font::new(
            &pango_ctx,
            FontDescription::from_string(&get_default_interface_font_schema()),
        )
    }

    fn get_edit_font(pango_ctx: &pango::Context, config: &Config) -> Font {
        Font::new(
            pango_ctx,
            FontDescription::from_string(&format!(
                "{} {}",
                &config.config.font_face, &config.config.font_size,
            )),
        )
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
    fn new() -> Self {
        let tab_widget = gtk::Box::new(Orientation::Horizontal, 5);
        let label = Label::new(Some(""));
        tab_widget.add(&label);
        let close_button = Button::new_from_icon_name(Some("window-close"), IconSize::Button);
        tab_widget.add(&close_button);
        tab_widget.show_all();

        Self {
            tab_widget,
            label,
            close_button,
        }
    }
}

/// Contains the Find & Replace elements
#[derive(Clone)]
struct FindReplace {
    search_bar: SearchBar,
    replace_revealer: Revealer,
    replace_entry: SearchEntry,
    replace_button: Button,
    replace_all_button: Button,
    find_status_label: Label,
    option_revealer: Revealer,
    search_entry: SearchEntry,
    go_down_button: Button,
    go_up_button: Button,
    popover: Popover,
    show_replace_button: ToggleButton,
    show_options_button: ToggleButton,
    use_regex_button: CheckButton,
    case_sensitive_button: CheckButton,
    whole_word_button: CheckButton,
}

impl FindReplace {
    /// Loads the glade description of the window, and builds gtk-rs objects.
    fn new(btn: &MenuButton) -> Self {
        const SRC: &str = include_str!("ui/find_replace.glade");

        let builder = Builder::new_from_string(SRC);
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
    fn connect_events(&self, ev: &Rc<RefCell<EditView>>) {
        trace!(
            "{} '{}'",
            gettext("Connecting FindReplace events for EditView"),
            ev.borrow().view_id
        );

        self.popover.connect_event(clone!(ev => move |_, event| {
            ev.borrow().find_replace.search_bar.handle_event(event);

            Inhibit(false)
        }));

        self.popover.connect_closed(clone!(ev => move |_| {
            ev.borrow().stop_search();
            ev.borrow().stop_replace();
        }));

        self.show_replace_button
            .connect_toggled(clone!(ev => move |toggle_btn| {
                if toggle_btn.get_active() {
                    ev.borrow().show_replace();
                } else {
                    ev.borrow().hide_replace();
                }
            }));

        self.show_options_button
            .connect_toggled(clone!(ev => move |toggle_btn| {
                if toggle_btn.get_active() {
                    ev.borrow().show_findreplace_opts();
                } else {
                    ev.borrow().hide_findreplace_opts();
                }
            }));

        self.search_bar.connect_entry(&self.search_entry);

        self.search_bar
            .connect_property_search_mode_enabled_notify(clone!(ev => move |sb| {
                if ! sb.get_search_mode() {
                    ev.borrow().stop_search();
                }
            }));

        self.search_entry
            .connect_search_changed(clone!(ev => move |w| {
                if let Some(text) = w.get_text() {
                    ev.borrow().search_changed(Some(text.to_string()));
                } else {
                    ev.borrow().search_changed(None);
                }
            }));

        self.replace_entry
            .connect_next_match(clone!(ev => move |_| {
                ev.borrow().find_next();
            }));

        self.replace_entry
            .connect_previous_match(clone!(ev => move |_| {
                ev.borrow().find_prev();
            }));

        self.replace_entry
            .connect_stop_search(clone!(ev => move |_| {
                ev.borrow().stop_replace();
            }));

        let restart_search = move |edit_view: Rc<RefCell<EditView>>| {
            let text_opt = { edit_view.borrow().find_replace.search_entry.get_text() };
            if let Some(text) = text_opt {
                edit_view.borrow().search_changed(Some(text.to_string()));
            } else {
                edit_view.borrow_mut().search_changed(None);
            }
        };

        self.use_regex_button
            .connect_toggled(clone!(ev => move |_| restart_search(ev.clone())));

        self.whole_word_button
            .connect_toggled(clone!(ev => move |_| restart_search(ev.clone())));

        self.case_sensitive_button
            .connect_toggled(clone!(ev => move |_| restart_search(ev.clone())));

        self.search_entry.connect_activate(clone!(ev => move |_| {
            ev.borrow_mut().find_next();
        }));

        self.search_entry
            .connect_stop_search(clone!(ev => move |_| {
                ev.borrow().stop_search();
            }));

        self.replace_button.connect_clicked(clone!(ev => move |_| {
            ev.borrow().replace();
        }));

        self.replace_all_button
            .connect_clicked(clone!(ev => move |_| {
                ev.borrow().replace_all();
            }));

        self.go_down_button.connect_clicked(clone!(ev => move |_| {
            ev.borrow_mut().find_next();
        }));

        self.go_up_button.connect_clicked(clone!(ev => move |_| {
            ev.borrow_mut().find_prev();
        }));
    }
}

impl EditView {
    /// Set the name of the file the EditView is currently editing and calls [update_title](struct.EditView.html#method.update_title)
    pub fn set_file(&mut self, file_name: &str) {
        trace!(
            "{} 'FindReplace' {} '{}'",
            gettext("Connecting"),
            gettext("events for EditView"),
            self.view_id
        );
        self.file_name = Some(file_name.to_string());
        self.update_title();
    }

    /// Update the title of the EditView to the currently set file_name
    fn update_title(&self) {
        let title = match self.file_name {
            Some(ref f) => f
                .split(::std::path::MAIN_SEPARATOR)
                .last()
                .unwrap_or(&gettext("Untitled"))
                .to_string(),
            None => gettext("Untitled"),
        };

        let mut full_title = String::new();
        if !self.pristine {
            full_title.push('*');
        }
        full_title.push_str(&title);

        trace!(
            "{} '{}': {}",
            gettext("Setting title for EditView"),
            self.view_id,
            full_title
        );
        self.top_bar.label.set_text(&full_title);
    }

    /// If xi-editor sends us a [config_changed](https://xi-editor.io/docs/frontend-protocol.html#config_changed)
    /// msg we process it here, e.g. setting the font face/size xi-editor tells us. Most configs don't
    /// need special handling by us though.
    pub fn config_changed(&mut self, changes: &Value) {
        trace!(
            "{} 'config_changed' {} '{}': {:?}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            changes
        );
        if let Some(map) = changes.as_object() {
            for (name, value) in map {
                match name.as_ref() {
                    "font_size" => {
                        if let Some(font_size) = value.as_u64() {
                            let pango_ctx = self.view_item.get_pango_ctx();
                            self.edit_font
                                .font_desc
                                .set_size(font_size as i32 * pango::SCALE);
                            // We've set the new fontsize previously, now we have to regenerate the font height/width etc.
                            self.edit_font =
                                Font::new(&pango_ctx, self.edit_font.font_desc.clone());
                            self.view_item.edit_area.queue_draw();
                        }
                    }
                    "font_face" => {
                        if let Some(font_face) = value.as_str() {
                            debug!("{}: {}", gettext("Setting edit font to"), font_face);
                            let pango_ctx = self.view_item.get_pango_ctx();
                            self.edit_font = Font::new(
                                &pango_ctx,
                                FontDescription::from_string(&format!(
                                    "{} {}",
                                    font_face,
                                    self.edit_font.font_desc.get_size() / pango::SCALE
                                )),
                            );
                            self.view_item.edit_area.queue_draw();
                        }
                    }
                    "tab_size" => {
                        if let Some(size) = value.as_u64() {
                            debug!("{}: {}", gettext("Setting tab size to"), size);
                            self.main_state.borrow_mut().config.config.tab_size = size as u32;
                            self.view_item.edit_area.queue_draw();
                        }
                    }
                    // These are handled in main_win via XiConfig
                    "auto_indent" => (),
                    "autodetect_whitespace" => (),
                    "plugin_search_path" => (),
                    "scroll_past_end" => (),
                    "translate_tabs_to_spaces" => (),
                    "use_tab_stops" => (),
                    "word_wrap" => (),
                    "wrap_width" => (),
                    "line_ending" => (),
                    "surrounding_pairs" => (),
                    "save_with_newline" => (),
                    _ => {
                        error!(
                            "{}: {}",
                            gettext("Unhandled config option, open a bug report!"),
                            name
                        );
                    }
                }
            }
        }
    }

    /// If xi-editor sends us a [update](https://xi-editor.io/docs/frontend-protocol.html#config_changed)
    /// msg we process it here, setting the scrollbars upper limit accordingly, checking if the EditView
    /// is pristine (_does not_ has unsaved changes) and queue a new draw of the EditView.
    pub fn update(&mut self, params: &Value) {
        trace!(
            "{} 'update' {} '{}': {:?}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            params
        );
        let update = &params["update"];
        self.line_cache.apply_update(update);

        // update scrollbars to the new text width and height
        let text_size = self.get_text_size();
        let text_height = text_size.height;
        let text_width = text_size.width;
        let vadj = self.view_item.verti_bar.get_adjustment();
        vadj.set_lower(0_f64);
        vadj.set_upper(text_height as f64);
        if vadj.get_value() + vadj.get_page_size() > vadj.get_upper() {
            vadj.set_value(vadj.get_upper() - vadj.get_page_size())
        }

        let hadj = self.view_item.horiz_bar.get_adjustment();
        hadj.set_lower(0f64);
        let text_width = if text_size.contained_width {
            text_width
        } else {
            text_width + self.edit_font.font_width * 4.0
        };
        hadj.set_upper(text_width);
        if hadj.get_value() + hadj.get_page_size() > hadj.get_upper() {
            hadj.set_value(hadj.get_upper() - hadj.get_page_size())
        }

        if let Some(pristine) = update["pristine"].as_bool() {
            if self.pristine != pristine {
                self.pristine = pristine;
                self.update_title();
            }
        }

        // self.change_scrollbar_visibility();

        self.view_item.edit_area.queue_draw();
        self.view_item.linecount.queue_draw();
    }

    fn change_scrollbar_visibility(&self) {
        let vadj = self.view_item.verti_bar.get_adjustment();
        let hadj = self.view_item.horiz_bar.get_adjustment();

        if vadj.get_value() <= vadj.get_lower()
            && vadj.get_value() + vadj.get_page_size() >= vadj.get_upper()
        {
            self.view_item.verti_bar.hide();
        } else {
            self.view_item.verti_bar.show();
        }

        if hadj.get_value() <= hadj.get_lower()
            && hadj.get_value() + hadj.get_page_size() >= hadj.get_upper()
        {
            self.view_item.horiz_bar.hide();
        } else {
            debug!(
                "SHOWING HSCROLLBAR: {} {}-{} {}",
                hadj.get_value(),
                hadj.get_lower(),
                hadj.get_upper(),
                hadj.get_page_size()
            );
            self.view_item.horiz_bar.show();
        }
    }

    /// Maps x|y pixel coordinates to the line num and col. This can be used e.g. for
    /// determining the firt and last time, but setting the y coordinate to 0 and the
    /// last pixel.
    pub fn da_px_to_cell(&self, x: f64, y: f64) -> (u64, u64) {
        trace!(
            "{} 'da_px_to_cell' {} '{}': x: {} y: {}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            x,
            y
        );
        // let first_line = (vadj.get_value() / font_extents.height) as usize;
        let x = x + self.view_item.horiz_bar.get_adjustment().get_value();
        let y = y + self.view_item.verti_bar.get_adjustment().get_value();

        let mut y = y - self.edit_font.font_descent;
        if y < 0.0 {
            y = 0.0;
        }
        let line_num = (y / self.edit_font.font_height) as u64;
        let index = if let Some(line) = self.line_cache.get_line(line_num) {
            let pango_ctx = self.view_item.get_pango_ctx();

            let layout = self.create_layout_for_line(&pango_ctx, line, &self.get_tabs());
            let (_, index, trailing) = layout.xy_to_index(x as i32 * pango::SCALE, 0);
            index + trailing
        } else {
            0
        };
        (index as u64, (y / self.edit_font.font_height) as u64)
    }

    /// Allocate the space our DrawingArea needs.
    fn da_size_allocate(&mut self, da_width: i32, da_height: i32) {
        debug!("{}", gettext("Allocating DrawingArea size"));
        let vadj = self.view_item.verti_bar.get_adjustment();
        vadj.set_page_size(f64::from(da_height));
        let hadj = self.view_item.horiz_bar.get_adjustment();
        hadj.set_page_size(f64::from(da_width));

        self.update_visible_scroll_region();
    }

    /// Upon changing the vertical scrollbar we have to call [update_visible_scroll_region](struct.EditView.html#method.vscrollbar_change_value)
    fn vscrollbar_change_value(&mut self, value: f64) -> Inhibit {
        debug!("{} {}", gettext("Vertical scrollbar changed value"), value);

        self.update_visible_scroll_region();

        Inhibit(false)
    }

    /// This updates the part of the document that's visible to the user, e.g. when scrolling.
    /// This requests the required lines from xi-editor to add them to the line cache and then
    /// adjusts the scrolling to the visible region.
    fn update_visible_scroll_region(&self) {
        trace!(
            "{} 'update_visible_scroll_region' {} '{}'",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id
        );
        let da_height = self.view_item.edit_area.get_allocated_height();
        let (_, first_line) = self.da_px_to_cell(0.0, 0.0);
        let (_, last_line) = self.da_px_to_cell(0.0, f64::from(da_height));
        let last_line = last_line + 1;

        debug!(
            "{} {} {}",
            gettext("Updating visible scroll region"),
            first_line,
            last_line
        );

        self.core
            .borrow()
            .scroll(&self.view_id, first_line, last_line);
    }

    /// Returns the width&height of the entire document
    fn get_text_size(&self) -> TextSize {
        trace!(
            "{} 'get_text_size' {} '{}'",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id
        );

        let mut contained_height = false;
        let mut contained_width = false;

        let da_width = f64::from(self.view_item.edit_area.get_allocated_width());
        let da_height = f64::from(self.view_item.edit_area.get_allocated_height());
        let num_lines = self.line_cache.height();

        let all_text_height =
            num_lines as f64 * self.edit_font.font_height + self.edit_font.font_descent;
        let height = if da_height > all_text_height {
            contained_height = true;
            da_height
        } else {
            all_text_height
        };

        let vadj = self.view_item.verti_bar.get_adjustment();
        let first_line = (vadj.get_value() / self.edit_font.font_height) as u64;
        let last_line = (vadj.get_value() + da_height / self.edit_font.font_height) as u64 + 1;
        let last_line = min(last_line, num_lines);
        // Set this to pango::SCALE, we divide by that later on.
        let mut max_width = pango::SCALE;

        let pango_ctx = self.view_item.get_pango_ctx();
        let tabs = self.get_tabs();

        // Determine the longest line as per Pango. Creating layouts with Pango here is kind of expensive
        // here, but it's hard determining an accurate width otherwise.
        for i in first_line..last_line {
            if let Some(line) = self.line_cache.get_line(i) {
                let layout = self.create_layout_for_line(&pango_ctx, line, &tabs);
                max_width = max(max_width, layout.get_extents().1.width);
            }
        }

        let render_width = f64::from(max_width / pango::SCALE);

        let width = if da_width > render_width {
            contained_width = true;
            da_width
        } else {
            render_width
        };

        TextSize {
            width,
            height,
            contained_height,
            contained_width,
        }
    }

    /// Handles the drawing of the EditView. This is called when we get a update from xi-editor or if
    /// gtk requests us to draw the EditView. This draws the background, all lines and the cursor.
    pub fn handle_da_draw(&mut self, cr: &Context) -> Inhibit {
        const CURSOR_WIDTH: f64 = 2.0;

        // let foreground = self.main_state.borrow().theme.foreground;
        let theme = &self.main_state.borrow().theme;

        let da_width = self.view_item.edit_area.get_allocated_width();
        let da_height = self.view_item.edit_area.get_allocated_height();

        //debug!("Drawing");
        // cr.select_font_face("Mono", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
        // let mut font_options = cr.get_font_options();
        // debug!("font options: {:?} {:?} {:?}", font_options, font_options.get_antialias(), font_options.get_hint_style());
        // font_options.set_hint_style(HintStyle::Full);

        // let (text_width, text_height) = self.get_text_size();
        let num_lines = self.line_cache.height();

        let vadj = self.view_item.verti_bar.get_adjustment();
        let hadj = self.view_item.horiz_bar.get_adjustment();
        trace!(
            "{}  {}: {}/{}; {}: {}/{}",
            gettext("Drawing EditView"),
            gettext("vertical adjustment"),
            vadj.get_value(),
            vadj.get_upper(),
            gettext("horizontal adjustment"),
            hadj.get_value(),
            hadj.get_upper()
        );

        let first_line = (vadj.get_value() / self.edit_font.font_height) as u64;
        let last_line =
            ((vadj.get_value() + f64::from(da_height)) / self.edit_font.font_height) as u64 + 1;
        let last_line = min(last_line, num_lines);

        let pango_ctx = self.view_item.get_pango_ctx();
        pango_ctx.set_font_description(&self.edit_font.font_desc);

        // Draw a line at x chars
        if get_draw_right_margin() {
            let until_margin_width =
                self.edit_font.font_width * f64::from(get_column_right_margin());
            // Draw editing background
            set_source_color(cr, theme.background);
            cr.rectangle(0.0, 0.0, until_margin_width, f64::from(da_height));
            cr.fill();

            set_margin_source_color(cr, theme.background);
            cr.rectangle(
                until_margin_width - self.view_item.horiz_bar.get_value(),
                0.0,
                f64::from(da_width) + self.view_item.horiz_bar.get_value(),
                f64::from(da_height),
            );
            cr.fill();
        } else {
            // Draw editing background
            set_source_color(cr, theme.background);
            cr.rectangle(0.0, 0.0, f64::from(da_width), f64::from(da_height));
            cr.fill();
        }

        set_source_color(cr, theme.foreground);

        let tabs = self.get_tabs();

        for i in first_line..last_line {
            // Keep track of the starting x position
            if let Some(line) = self.line_cache.get_line(i) {
                if get_highlight_line() && !line.cursor().is_empty() {
                    set_source_color(cr, theme.line_highlight);
                    cr.rectangle(
                        0.0,
                        self.edit_font.font_height * i as f64 - vadj.get_value(),
                        f64::from(da_width),
                        self.edit_font.font_height,
                    );
                    cr.fill();
                    set_source_color(cr, theme.foreground);
                }

                cr.move_to(
                    -hadj.get_value(),
                    self.edit_font.font_height * (i as f64) - vadj.get_value(),
                );

                let pango_ctx = self.view_item.get_pango_ctx();
                let layout = self.create_layout_for_line(&pango_ctx, line, &tabs);
                // debug!("width={}", layout.get_extents().1.width);
                update_layout(cr, &layout);
                show_layout(cr, &layout);

                // Well this is stupid, but (for some reason) Pango gets the width of "·" wrong!
                // It only thinks that the width of that char is 5, when it actually is 10 (like all
                // other chars. So we have to replace it with some other char here to trick Pango into
                // drawing the cursor at the correct position later on
                layout.set_text(&layout.get_text().unwrap().replace("\u{b7}", " "));

                let layout_line = layout.get_line(0);
                if layout_line.is_none() {
                    continue;
                }
                let layout_line = layout_line.unwrap();

                // Set cursor color
                set_source_color(cr, theme.caret);

                for c in line.cursor() {
                    let x = layout_line.index_to_x(*c as i32, false) / pango::SCALE;
                    // Draw the cursor
                    cr.rectangle(
                        (f64::from(x)) - hadj.get_value(),
                        (((self.edit_font.font_ascent + self.edit_font.font_descent) as u64) * i)
                            as f64
                            - vadj.get_value(),
                        CURSOR_WIDTH,
                        self.edit_font.font_ascent + self.edit_font.font_descent,
                    );
                    cr.fill();
                }
            }
        }

        Inhibit(false)
    }

    /// This draws the linecount. We have this as our own widget to make sure we don't mess up text
    /// selection etc.
    pub fn handle_linecount_draw(&mut self, cr: &Context) -> Inhibit {
        trace!(
            "{} 'linecount_draw' {} '{}'",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id
        );
        let theme = &self.main_state.borrow().theme;
        let linecount_height = self.view_item.linecount.get_allocated_height();

        let num_lines = self.line_cache.height();

        let vadj = self.view_item.verti_bar.get_adjustment();

        let first_line = (vadj.get_value() / self.edit_font.font_height) as u64;
        let last_line = ((vadj.get_value() + f64::from(linecount_height))
            / self.edit_font.font_height) as u64
            + 1;
        let last_line = min(last_line, num_lines);

        let pango_ctx = self.view_item.get_pango_ctx();

        // Make the linecount at least 6 chars big
        let linecount_width = if format!("  {}  ", last_line).len() > 6 {
            let width = self.interface_font.font_width * format!("  {}  ", last_line).len() as f64;
            // Make sure the linecount_width is even to properly center the line number
            if width % 2.0 == 0.0 {
                width
            } else {
                width + 1.0
            }
        } else {
            self.interface_font.font_width * 6.0
        };

        // Draw linecount background
        set_source_color(cr, theme.background);
        cr.rectangle(0.0, 0.0, linecount_width, f64::from(linecount_height));
        cr.fill();

        //FIXME: Xi sends us the 'ln' (logical linenumber) param for this, but that isn't updated on every draw!
        let mut current_line = first_line;
        let center_diff = (self.edit_font.font_height - self.interface_font.font_height) / 2.0;

        set_source_color(cr, theme.foreground);
        for i in first_line..last_line {
            // Keep track of the starting x position
            if let Some(line) = self.line_cache.get_line(i) {
                cr.move_to(
                    0.0,
                    self.edit_font.font_height * (i as f64) - vadj.get_value() + center_diff,
                );
                if line.line_num().is_some() {
                    current_line += 1
                }

                let linecount_layout = self.create_layout_for_linecount(
                    &pango_ctx,
                    current_line,
                    linecount_width as usize,
                );
                update_layout(cr, &linecount_layout);
                show_layout(cr, &linecount_layout);
            }
        }

        // Set the appropriate size for the linecount DrawingArea, otherwise it's only 1 px wide.
        self.view_item
            .linecount
            .set_size_request(linecount_width as i32, -1);
        Inhibit(false)
    }

    /// Creates a pango layout for a particular linecount (the count on the left) in the linecache
    fn create_layout_for_linecount(
        &self,
        pango_ctx: &pango::Context,
        n: u64,
        padding: usize,
    ) -> pango::Layout {
        let line_view = format!(
            "{:^offset$}",
            n,
            offset = padding / self.interface_font.font_width as usize + 1
        );
        let layout = pango::Layout::new(pango_ctx);
        layout.set_alignment(pango::Alignment::Center);
        layout.set_font_description(Some(&self.interface_font.font_desc));
        layout.set_text(line_view.as_str());
        layout
    }

    fn get_tabs(&self) -> TabArray {
        let mut tabs = TabArray::new(1, false);
        tabs.set_tab(
            0,
            TabAlign::Left,
            self.edit_font.font_width as i32
                * self.main_state.borrow().config.config.tab_size as i32
                * pango::SCALE,
        );

        tabs
    }

    /// Checks how wide a line is
    pub fn line_width(&self, line_string: &str) -> f64 {
        let line = Line::from_json(
            &serde_json::json!({
                "text": line_string,
            }),
            None,
        );
        let pango_ctx = self.view_item.get_pango_ctx();
        let linecount_layout = self.create_layout_for_line(&pango_ctx, &line, &self.get_tabs());

        f64::from(linecount_layout.get_extents().1.width / pango::SCALE)
    }

    /// Creates a pango layout for a particular line in the linecache
    fn create_layout_for_line(
        &self,
        pango_ctx: &pango::Context,
        line: &Line,
        tabs: &TabArray,
    ) -> pango::Layout {
        let line_view = if line.text().ends_with('\n') {
            &line.text()[0..line.text().len() - 1]
        } else {
            &line.text()
        };

        // Replace spaces with '·'. Do this here since we only want
        // to draw this, we don't want to save the file like that.
        let line_view = if get_draw_trailing_spaces_schema() && line_view.ends_with(' ') {
            // Replace tabs here to make sure trim_end doesn't remove them
            let last_char = line_view.replace("\t", "a").trim_end().len();
            let (line_view_without_space, spaces) = line_view.split_at(last_char);
            let space_range = std::ops::Range {
                start: 0,
                end: spaces.len(),
            };
            let highlighted_spaces: String = space_range.map(|_| "\u{b7}").collect();

            format!("{}{}", line_view_without_space, highlighted_spaces)
        } else {
            line_view.to_string()
        };

        // let layout = create_layout(cr).unwrap();
        let layout = pango::Layout::new(pango_ctx);
        layout.set_tabs(Some(tabs));
        layout.set_font_description(Some(&self.edit_font.font_desc));
        layout.set_text(&line_view);

        let mut ix = 0;
        let attr_list = pango::AttrList::new();
        for style in &line.styles {
            let start_index = (ix + style.start) as u32;
            let end_index = (ix + style.start + style.len as i64) as u32;
            let main_state = self.main_state.borrow();
            let line_style = main_state.styles.get(&style.id);

            if let Some(foreground) = line_style.and_then(|s| s.fg_color) {
                let pango_color = PangoColor::from(color_from_u32(foreground));
                let mut attr =
                    Attribute::new_foreground(pango_color.r, pango_color.g, pango_color.b).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            if let Some(background) = line_style.and_then(|s| s.bg_color) {
                let pango_color = PangoColor::from(color_from_u32(background));
                let mut attr =
                    Attribute::new_background(pango_color.r, pango_color.g, pango_color.b).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            if let Some(weight) = line_style.and_then(|s| s.weight) {
                let mut attr =
                    Attribute::new_weight(pango::Weight::__Unknown(weight as i32)).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            if let Some(italic) = line_style.and_then(|s| s.italic) {
                let mut attr = if italic {
                    Attribute::new_style(pango::Style::Italic).unwrap()
                } else {
                    Attribute::new_style(pango::Style::Normal).unwrap()
                };
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            if let Some(underline) = line_style.and_then(|s| s.underline) {
                let mut attr = if underline {
                    Attribute::new_underline(pango::Underline::Single).unwrap()
                } else {
                    Attribute::new_underline(pango::Underline::None).unwrap()
                };
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            ix += style.start + style.len as i64;
        }

        layout.set_attributes(Some(&attr_list));
        layout
    }

    /// Scrolls vertically to the line specified and horizontally to the column specified.
    pub fn scroll_to(&mut self, line: u64, col: u64) {
        trace!(
            "{} 'scroll_to' {} '{}': l: {} c: {}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            line,
            col
        );

        {
            // The new height is the current last line + 1
            let new_height = self.edit_font.font_height * line as f64;
            let padding = self.edit_font.font_height * 4.0;
            // The font height doesn't include these, so we have to add them for the last line
            let vadj = self.view_item.verti_bar.get_adjustment();
            // If the cursor above our current view, this is true. Scroll a bit higher than necessary
            // to make sure that during find the text isn't right at the edge of the view.
            if new_height < vadj.get_value() {
                vadj.set_value(new_height - padding);
            // If it's below out current view, this is true. Scroll a bit lower than neccessary
            // for the same reasons cited above.
            } else if new_height + padding > vadj.get_value() + vadj.get_page_size()
                && vadj.get_page_size() != 0.0
            {
                vadj.set_value(
                    new_height
                        + self.edit_font.font_height
                        + padding
                        // These two aren't included in the font height and we need them to line up with the line
                        + self.edit_font.font_ascent
                        + self.edit_font.font_descent
                        - vadj.get_page_size(),
                );
            }
        }

        {
            // Collect all styles with id 0/1 (selections/find results) to make sure they're in the frame
            if let Some(line) = self.line_cache.get_line(line) {
                let line_selections: Vec<&StyleSpan> = line
                    .styles
                    .iter()
                    .filter(|s| s.id == 0 || s.id == 1)
                    .collect();

                let mut begin_selection = None;
                let mut end_selection = None;

                for x in line_selections {
                    if let Some(cur) = begin_selection {
                        // Make sure to use the lowest value of any selection so it's in the view
                        begin_selection = Some(min(cur, x.start));
                    } else {
                        begin_selection = Some(x.start);
                    }
                    if let Some(cur) = end_selection {
                        // Make sure to use the highest value of any selection so it's in the view
                        end_selection = Some(max(cur, x.start + x.len as i64));
                    } else {
                        end_selection = Some(x.start + x.len as i64);
                    }
                }

                let mut line_text = line.text().to_string();
                // Only measure width up to the right column
                line_text.truncate(col as usize);
                let line_length = self.line_width(&line_text);
                let padding = self.edit_font.font_width * 4.0;
                let hadj = self.view_item.horiz_bar.get_adjustment();

                let min = min(
                    begin_selection.unwrap_or(line_length as i64),
                    line_length as i64,
                ) as f64;
                let max = max(
                    end_selection.unwrap_or(line_length as i64),
                    line_length as i64,
                ) as f64;

                trace!("Horizontal scrolling to min: {}; max: {}", min, max);

                // If the cursor/selection is to the left of our current view, this is true
                if min < hadj.get_value() {
                    hadj.set_value(min - padding);
                // If the cursor/selection is to the right of our current view, this is true
                } else if max > hadj.get_value() + hadj.get_page_size()
                    && hadj.get_page_size() != 0.0
                {
                    hadj.set_value(max - hadj.get_page_size() + padding);
                }
            } else {
                warn!("{}", gettext("Couldn't update hscrollbar value because I couldn't get the line to scroll to!"));
            }
        }
    }

    /// Handles button presses such as Shift, Ctrl etc. and primary pasting (i.e. via Ctrl+V, not
    /// via middle mouse click).
    pub fn handle_button_press(&self, eb: &EventButton) -> Inhibit {
        trace!(
            "{} 'button_press' {} '{}': {:?}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            eb
        );
        self.view_item.edit_area.grab_focus();

        let (x, y) = eb.get_position();
        let (col, line) = self.da_px_to_cell(x, y);

        match eb.get_button() {
            1 => {
                if eb.get_state().contains(ModifierType::SHIFT_MASK) {
                    self.core
                        .borrow()
                        .gesture_range_select(&self.view_id, line, col);
                } else if eb.get_state().contains(ModifierType::CONTROL_MASK) {
                    self.core
                        .borrow()
                        .gesture_toggle_sel(&self.view_id, line, col);
                } else if eb.get_event_type() == EventType::DoubleButtonPress {
                    self.core
                        .borrow()
                        .gesture_word_select(&self.view_id, line, col);
                } else if eb.get_event_type() == EventType::TripleButtonPress {
                    self.core
                        .borrow()
                        .gesture_line_select(&self.view_id, line, col);
                } else {
                    self.core
                        .borrow()
                        .gesture_point_select(&self.view_id, line, col);
                }
            }
            2 => {
                self.do_paste_primary(&self.view_id, line, col);
            }
            _ => {}
        }
        Inhibit(false)
    }

    /// Handle selecting line(s) by dragging the mouse across them while having the left mouse
    /// button clicked.
    pub fn handle_drag(&mut self, em: &EventMotion) -> Inhibit {
        let (x, y) = em.get_position();
        let (col, line) = self.da_px_to_cell(x, y);
        self.core.borrow().drag(&self.view_id, line, col);
        Inhibit(false)
    }

    /// Handles scroll events, i.e. the user dragging the scrollbar, scrolling via a mouse wheel
    /// or via a touchpad/drawing tablet (which use SmoothScrolling, which may scroll vertically
    /// and horizontally at the same time).
    pub fn handle_scroll(&mut self, es: &EventScroll) -> Inhibit {
        trace!(
            "{} 'scroll' {} '{}': {:?}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            es
        );
        self.view_item.edit_area.grab_focus();
        // TODO: Make this user configurable!
        let amt = self.edit_font.font_height;

        let vadj = self.view_item.verti_bar.get_adjustment();
        let hadj = self.view_item.horiz_bar.get_adjustment();
        match es.get_direction() {
            ScrollDirection::Smooth => {
                let (scroll_change_hori, scroll_change_vert) =
                    if let Some(v) = es.get_scroll_deltas() {
                        v
                    } else {
                        error!("{}", gettext("Smooth scrolling failed"));
                        (0.0, 0.0)
                    };

                vadj.set_value(vadj.get_value() + (scroll_change_vert * amt));
                hadj.set_value(hadj.get_value() + (scroll_change_hori * amt));
            }
            ScrollDirection::Up => vadj.set_value(vadj.get_value() - (hadj.get_value() * amt)),
            ScrollDirection::Down => vadj.set_value(vadj.get_value() + (hadj.get_value() * amt)),
            ScrollDirection::Left => hadj.set_value(hadj.get_value() - (hadj.get_value() * amt)),
            ScrollDirection::Right => hadj.set_value(hadj.get_value() + (hadj.get_value() * amt)),
            _ => {}
        }

        self.update_visible_scroll_region();

        Inhibit(false)
    }

    /// Handles all (special) key press events, e.g. copy, pasting, PgUp/Down etc.
    // Allow this to be a long function since splitting up the matching into multiple functions
    // would be a pain
    #[allow(clippy::cyclomatic_complexity)]
    fn handle_key_press_event(&self, ek: &EventKey) -> Inhibit {
        trace!(
            "{} 'key_press_event' {} '{}': {:?}",
            gettext("Handling"),
            gettext("for EditView"),
            self.view_id,
            ek
        );
        debug!(
            "{}: {}={:?}, {}={:?}, {}={:?} {}={:?} {}={:?}",
            gettext("Processing key press"),
            gettext("value"),
            ek.get_keyval(),
            gettext("state"),
            ek.get_state(),
            gettext("length"),
            ek.get_length(),
            gettext("group"),
            ek.get_group(),
            gettext("unicode"),
            ::gdk::keyval_to_unicode(ek.get_keyval())
        );
        let view_id = &self.view_id;
        let ch = ::gdk::keyval_to_unicode(ek.get_keyval());

        let alt = ek.get_state().contains(ModifierType::MOD1_MASK);
        let ctrl = ek.get_state().contains(ModifierType::CONTROL_MASK);
        let meta = ek.get_state().contains(ModifierType::META_MASK);
        let shift = ek.get_state().contains(ModifierType::SHIFT_MASK);
        let norm = !alt && !ctrl && !meta;

        match ek.get_keyval() {
            key::Delete if norm => self.core.borrow().delete_forward(view_id),
            key::BackSpace if norm => self.core.borrow().delete_backward(view_id),
            key::Return | key::KP_Enter => {
                self.core.borrow().insert_newline(&view_id);
            }
            key::Tab if norm && !shift => self.core.borrow().insert_tab(view_id),
            key::Up if norm && !shift => self.core.borrow().move_up(view_id),
            key::Down if norm && !shift => self.core.borrow().move_down(view_id),
            key::Left if norm && !shift => self.core.borrow().move_left(view_id),
            key::Right if norm && !shift => self.core.borrow().move_right(view_id),
            key::Up if norm && shift => {
                self.core.borrow().move_up_and_modify_selection(view_id);
            }
            key::Down if norm && shift => {
                self.core.borrow().move_down_and_modify_selection(view_id);
            }
            key::Left if norm && shift => {
                self.core.borrow().move_left_and_modify_selection(view_id);
            }
            key::Right if norm && shift => {
                self.core.borrow().move_right_and_modify_selection(view_id);
            }
            key::Left if ctrl && !shift => {
                self.core.borrow().move_word_left(view_id);
            }
            key::Right if ctrl && !shift => {
                self.core.borrow().move_word_right(view_id);
            }
            key::Left if ctrl && shift => {
                self.core
                    .borrow()
                    .move_word_left_and_modify_selection(view_id);
            }
            key::Right if ctrl && shift => {
                self.core
                    .borrow()
                    .move_word_right_and_modify_selection(view_id);
            }
            key::Home if norm && !shift => {
                self.core.borrow().move_to_left_end_of_line(view_id);
            }
            key::End if norm && !shift => {
                self.core.borrow().move_to_right_end_of_line(view_id);
            }
            key::Home if norm && shift => {
                self.core
                    .borrow()
                    .move_to_left_end_of_line_and_modify_selection(view_id);
            }
            key::End if norm && shift => {
                self.core
                    .borrow()
                    .move_to_right_end_of_line_and_modify_selection(view_id);
            }
            key::Home if ctrl && !shift => {
                self.core.borrow().move_to_beginning_of_document(view_id);
            }
            key::End if ctrl && !shift => {
                self.core.borrow().move_to_end_of_document(view_id);
            }
            key::Home if ctrl && shift => {
                self.core
                    .borrow()
                    .move_to_beginning_of_document_and_modify_selection(view_id);
            }
            key::End if ctrl && shift => {
                self.core
                    .borrow()
                    .move_to_end_of_document_and_modify_selection(view_id);
            }
            key::Page_Up if norm && !shift => {
                self.core.borrow().page_up(view_id);
            }
            key::Page_Down if norm && !shift => {
                self.core.borrow().page_down(view_id);
            }
            key::Page_Up if norm && shift => {
                self.core.borrow().page_up_and_modify_selection(view_id);
            }
            key::Page_Down if norm && shift => {
                self.core.borrow().page_down_and_modify_selection(view_id);
            }
            _ => {
                if let Some(ch) = ch {
                    match ch {
                        'a' if ctrl => {
                            self.core.borrow().select_all(view_id);
                        }
                        'c' if ctrl => {
                            self.do_copy(view_id);
                        }
                        'v' if ctrl => {
                            self.do_paste(view_id);
                        }
                        't' if ctrl => {
                            // TODO new tab
                        }
                        'x' if ctrl => {
                            self.do_cut(view_id);
                        }
                        'z' if ctrl => {
                            self.core.borrow().undo(view_id);
                        }
                        'Z' if ctrl && shift => {
                            self.core.borrow().redo(view_id);
                        }
                        c if (norm) && c >= '\u{0020}' => {
                            debug!("inserting key");
                            self.im_context.filter_keypress(ek);
                        }
                        _ => {
                            warn!("unhandled key: {:?}", ch);
                        }
                    }
                } else {
                    debug!("Inserting non char key");
                    self.im_context.filter_keypress(ek);
                }
            }
        };
        Inhibit(true)
    }

    /// Copies text to the clipboard
    fn do_cut(&self, view_id: &str) {
        debug!("{}", gettext("Cutting text"));

        let (clipboard_tx, clipboard_rx) =
            MainContext::channel::<Option<String>>(glib::PRIORITY_HIGH);
        let main_context = MainContext::default();

        clipboard_rx.attach(Some(&main_context), move |text_opt| {
            if let Some(text) = text_opt {
                Clipboard::get(&SELECTION_CLIPBOARD).set_text(&text);
            }
            source::Continue(false)
        });

        self.core.borrow_mut().cut(view_id, clipboard_tx);
    }

    /// Copies text to the clipboard
    fn do_copy(&self, view_id: &str) {
        debug!("{}", gettext("Copying text"));
        let (clipboard_tx, clipboard_rx) =
            MainContext::channel::<Option<String>>(glib::PRIORITY_HIGH);
        let main_context = MainContext::default();

        clipboard_rx.attach(Some(&main_context), move |text_opt| {
            if let Some(text) = text_opt {
                Clipboard::get(&SELECTION_CLIPBOARD).set_text(&text);
            }
            source::Continue(false)
        });

        self.core.borrow_mut().copy(view_id, clipboard_tx);
    }

    /// Pastes text from the clipboard into the EditView
    fn do_paste(&self, view_id: &str) {
        // if let Some(text) = Clipboard::get(&SELECTION_CLIPBOARD).wait_for_text() {
        //     self.core.borrow().insert(view_id, &text);
        // }
        debug!("{}", gettext("Pasting text"));
        let view_id2 = view_id.to_string().clone();
        let core = self.core.clone();
        Clipboard::get(&SELECTION_CLIPBOARD).request_text(move |_, text| {
            if let Some(clip_content) = text {
                core.borrow().insert(&view_id2, &clip_content);
            }
        });
    }

    fn do_paste_primary(&self, view_id: &str, line: u64, col: u64) {
        // if let Some(text) = Clipboard::get(&SELECTION_PRIMARY).wait_for_text() {
        //     self.core.borrow().insert(view_id, &text);
        // }
        debug!("{}", gettext("Pasting primary text"));
        let view_id2 = view_id.to_string().clone();
        let core = self.core.clone();
        Clipboard::get(&SELECTION_PRIMARY).request_text(move |_, text| {
            core.borrow().gesture_point_select(&view_id2, line, col);
            if let Some(clip_content) = text {
                core.borrow().insert(&view_id2, &clip_content);
            }
        });
    }

    /// Resize the EditView
    fn do_resize(&self, view_id: &str, width: i32, height: i32) {
        trace!("{} '{}'", gettext("Resizing EditView"), view_id);
        self.core.borrow().resize(view_id, width, height);
    }

    /// Opens the find dialog (Ctrl+F)
    pub fn start_search(&self) {
        if self.find_replace.search_bar.get_search_mode() {
            // If you've enabled the replace dialog, Ctrl+F brings back the find dialog (and as such
            // collapses the replace dialog) instead of stopping the entire search
            if self.find_replace.replace_revealer.get_reveal_child() {
                self.find_replace.show_replace_button.set_active(false);
            } else {
                self.stop_search();
            }
        } else {
            self.find_replace.search_bar.set_search_mode(true);
            #[cfg(feature = "gtk_v3_22")]
            self.find_replace.popover.popup();
            #[cfg(not(feature = "gtk_v3_22"))]
            self.find_replace.popover.show();
            self.find_replace
                .option_revealer
                .set_reveal_child(self.find_replace.show_options_button.get_active());
            self.find_replace
                .replace_revealer
                .set_reveal_child(self.find_replace.show_replace_button.get_active());
            self.find_replace.search_entry.grab_focus();
            if let Some(needle) = self.find_replace.search_entry.get_text() {
                // No need to pass the actual values of case_sensitive etc. to Xi here, we as soon
                // as we start typing something into the search box/flick one of the switches we call
                // EditView::search_changed() anyway, which does that for us.
                self.core
                    .borrow()
                    .find(&self.view_id, &needle, false, false, false);
            }
        }
    }

    /// Opens the replace dialog (Ctrl+R)
    pub fn start_replace(&self) {
        if self.find_replace.search_bar.get_search_mode() {
            // If you've enabled the replace dialog, Ctrl+R will collapse the entire Popover, if the
            // Popover is already open but the replace dialog is hidden (e.g. because Ctrl+F has been
            // pressed before) we'll expand the replace dialog instead of closing the entire Popover
            if self.find_replace.replace_revealer.get_reveal_child() {
                self.stop_replace();
            } else {
                self.find_replace.show_replace_button.set_active(true);
            }
        } else {
            self.find_replace.show_replace_button.set_active(true);
            self.find_replace.search_bar.set_search_mode(true);
            #[cfg(feature = "gtk_v3_22")]
            self.find_replace.popover.popup();
            #[cfg(not(feature = "gtk_v3_22"))]
            self.find_replace.popover.show();
            self.find_replace
                .option_revealer
                .set_reveal_child(self.find_replace.show_options_button.get_active());
            self.show_replace();
            self.find_replace.search_entry.grab_focus();
        }
    }

    pub fn show_replace(&self) {
        self.find_replace.replace_revealer.set_reveal_child(true);
    }

    pub fn hide_replace(&self) {
        self.find_replace.replace_revealer.set_reveal_child(false);
    }

    fn show_findreplace_opts(&self) {
        self.find_replace.option_revealer.set_reveal_child(true);
    }

    fn hide_findreplace_opts(&self) {
        self.find_replace.option_revealer.set_reveal_child(false);
    }

    pub fn stop_replace(&self) {
        self.find_replace.show_replace_button.set_active(false);
        self.hide_replace();
        self.stop_search();
    }

    /// Closes the find/replace dialog
    pub fn stop_search(&self) {
        #[cfg(feature = "gtk_v3_22")]
        self.find_replace.popover.popdown();
        #[cfg(not(feature = "gtk_v3_22"))]
        self.find_replace.popover.hide();
        self.find_replace.show_replace_button.set_active(false);
        self.find_replace.show_options_button.set_active(false);
        self.find_replace.search_bar.set_search_mode(false);
        self.view_item.edit_area.grab_focus();
    }

    /// Displays how many matches have been found in the find/replace dialog.
    pub fn find_status(&self, queries: &Value) {
        if let Some(queries) = queries.as_array() {
            for query in queries {
                if let Some(query_obj) = query.as_object() {
                    if let Some(matches) = query_obj["matches"].as_u64() {
                        self.find_replace
                            .find_status_label
                            .set_text(&format!("{} Results", matches));
                    }
                }
                debug!("query {}", query);
            }
        }
    }

    /// Displays what chars will be replaced in the replace dialog
    //TODO: Handle preserve_case
    pub fn replace_status(&self, status: &Value) {
        if let Some(chars) = status["chars"].as_str() {
            self.find_replace.replace_entry.set_text(chars);
        }
    }

    /// Go to the next match in the find/replace dialog
    pub fn find_next(&self) {
        self.core
            .borrow()
            .find_next(&self.view_id, Some(true), Some(true));
    }

    /// Go the to previous match in the find/replace dialog
    pub fn find_prev(&self) {
        self.core.borrow().find_previous(&self.view_id, Some(true));
    }

    /// Tells xi-editor that we're searching for a different string (or none) now
    pub fn search_changed(&self, s: Option<String>) {
        let needle = s.unwrap_or_default();
        let regex = self.find_replace.use_regex_button.get_active();
        let whole_worlds = self.find_replace.whole_word_button.get_active();
        let case_sensitive = self.find_replace.case_sensitive_button.get_active();
        self.core
            .borrow()
            .find(&self.view_id, &needle, case_sensitive, regex, whole_worlds);
    }

    /// Replace _one_ match with the replacement string
    pub fn replace(&self) {
        if let Some(replace_chars) = self.find_replace.replace_entry.get_text() {
            self.core
                .borrow()
                .replace(&self.view_id, replace_chars.as_str(), false);
            self.core.borrow().replace_next(&self.view_id);
        }
    }

    /// Replace _all_ matches with the replacement string
    pub fn replace_all(&self) {
        if let Some(replace_chars) = self.find_replace.replace_entry.get_text() {
            self.core
                .borrow()
                .replace(&self.view_id, replace_chars.as_str(), false);
            self.core.borrow().replace_all(&self.view_id);
        }
    }
}
