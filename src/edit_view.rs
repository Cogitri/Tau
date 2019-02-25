use crate::linecache::{Line, LineCache};
use crate::main_win::MainState;
use crate::pref_storage::*;
use crate::rpc::{self, Core};
use crate::theme::set_source_color;
use cairo::Context;
use gdk::enums::key;
use gdk::*;
use gettextrs::gettext;
use gtk::{self, *};
use log::{debug, error, trace};
use pango::{self, ContextExt, LayoutExt, *};
use pangocairo::functions::*;
use serde_json::Value;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::rc::Rc;
use std::u32;

/// The EvReplace struct holds the find+replace elements of the EditView.
pub struct EVReplace {
    replace_expander: Expander,
    replace_revealer: Revealer,
    replace_entry: Entry,
}

/// The EvFont struct holds all information about the font used in the EditView.
pub struct EVFont {
    font_height: f64,
    font_width: f64,
    font_ascent: f64,
    font_descent: f64,
    font_desc: FontDescription,
}

pub struct EVInterfaceFont {
    font_height: f64,
    font_width: f64,
    font_ascent: f64,
    font_descent: f64,
    font_desc: FontDescription,
}

/// The EditView is the part of gxi that does the actual editing. This is where you edit documents.
pub struct EditView {
    core: Rc<RefCell<Core>>,
    main_state: Rc<RefCell<MainState>>,
    pub view_id: String,
    pub file_name: Option<String>,
    pub pristine: bool,
    pub da: DrawingArea,
    linecount_da: DrawingArea,
    pub root_widget: gtk::Box,
    pub tab_widget: gtk::Box,
    search_bar: SearchBar,
    search_entry: SearchEntry,
    find_status_label: Label,
    pub label: Label,
    pub close_button: Button,
    hscrollbar: Scrollbar,
    vscrollbar: Scrollbar,
    line_cache: LineCache,
    replace: EVReplace,
    edit_font: EVFont,
    interface_font: EVInterfaceFont,
}

impl EditView {
    /// Initialises a new EditView. Sets up scrollbars, the actual editing area, the fonts,
    /// the syntax lang and connects all events which might happen during usage (e.g. scrolling)
    pub fn new(
        main_state: &Rc<RefCell<MainState>>,
        core: &Rc<RefCell<Core>>,
        file_name: Option<String>,
        view_id: &str,
    ) -> Rc<RefCell<EditView>> {
        let da = DrawingArea::new();
        let linecount_da = DrawingArea::new();
        let hscrollbar = Scrollbar::new(Orientation::Horizontal, None::<&gtk::Adjustment>);
        let vscrollbar = Scrollbar::new(Orientation::Vertical, None::<&gtk::Adjustment>);

        da.set_events(
            EventMask::BUTTON_PRESS_MASK
                | EventMask::BUTTON_RELEASE_MASK
                | EventMask::BUTTON_MOTION_MASK
                | EventMask::SCROLL_MASK
                | EventMask::SMOOTH_SCROLL_MASK,
        );
        debug!("{}: {:?}", gettext("Events"), da.get_events());
        da.set_can_focus(true);

        let find_rep_src = include_str!("ui/find_replace.glade");
        let find_rep_builder = Builder::new_from_string(find_rep_src);
        let search_bar: SearchBar = find_rep_builder.get_object("search_bar").unwrap();
        let replace_expander: Expander = find_rep_builder.get_object("replace_expander").unwrap();
        let replace_revealer: Revealer = find_rep_builder.get_object("replace_revealer").unwrap();
        let replace_entry: Entry = find_rep_builder.get_object("replace_entry").unwrap();
        let replace_button: Button = find_rep_builder.get_object("replace_button").unwrap();
        let replace_all_button: Button = find_rep_builder.get_object("replace_all_button").unwrap();
        let find_status_label: Label = find_rep_builder.get_object("find_status_label").unwrap();

        let replace = EVReplace {
            replace_expander: replace_expander.clone(),
            replace_revealer: replace_revealer.clone(),
            replace_entry: replace_entry.clone(),
        };

        // let overlay: Overlay = frame_builder.get_object("overlay").unwrap();
        // let search_revealer: Revealer = frame_builder.get_object("revealer").unwrap();
        // let frame: Frame = frame_builder.get_object("frame").unwrap();
        let search_entry: SearchEntry = find_rep_builder.get_object("search_entry").unwrap();
        let go_down_button: Button = find_rep_builder.get_object("go_down_button").unwrap();
        let go_up_button: Button = find_rep_builder.get_object("go_up_button").unwrap();

        let hbox = Box::new(Orientation::Horizontal, 0);
        let vbox = Box::new(Orientation::Vertical, 0);
        hbox.pack_start(&linecount_da, false, false, 0);
        hbox.pack_start(&vbox, true, true, 0);
        hbox.pack_start(&vscrollbar, false, false, 0);
        vbox.pack_start(&search_bar, false, false, 0);
        vbox.pack_start(&da, true, true, 0);
        vbox.pack_start(&hscrollbar, false, false, 0);
        hbox.show_all();

        // Make the widgets for the tab
        let tab_hbox = gtk::Box::new(Orientation::Horizontal, 5);
        let label = Label::new(Some(""));
        tab_hbox.add(&label);
        let close_button = Button::new_from_icon_name("window-close", IconSize::Button);
        tab_hbox.add(&close_button);
        tab_hbox.show_all();

        let pango_ctx = da
            .get_pango_context()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango context")));
        let font_list: Vec<String> = pango_ctx
            .list_families()
            .iter()
            .filter(|f| f.is_monospace())
            .filter_map(|f| f.get_name())
            .map(|f| f.to_string())
            .collect();
        main_state.borrow_mut().fonts = font_list;

        let font_desc = FontDescription::from_string("Inconsolata 16");
        // font_desc.set_size(14 * pango::SCALE);
        pango_ctx.set_font_description(&font_desc);
        let language = pango_ctx
            .get_language()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango language")));
        let fontset = pango_ctx
            .load_fontset(&font_desc, &language)
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font set")));
        let metrics = fontset
            .get_metrics()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font metrics")));

        // cr.select_font_face("Inconsolata", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
        // cr.set_font_size(16.0);
        // let font_extents = cr.font_extents();

        let layout = pango::Layout::new(&pango_ctx);
        layout.set_text("a");
        let (_, log_extents) = layout.get_extents();
        debug!("{}: {:?}", gettext("Pango edit font size"), log_extents);

        let font_height = f64::from(log_extents.height) / f64::from(pango::SCALE);
        let font_width = f64::from(log_extents.width) / f64::from(pango::SCALE);
        let font_ascent = f64::from(metrics.get_ascent()) / f64::from(pango::SCALE);
        let font_descent = f64::from(metrics.get_descent()) / f64::from(pango::SCALE);

        debug!(
            "{}: {} {} {} {}",
            gettext("Edit font metrics"),
            font_width,
            font_height,
            font_ascent,
            font_descent
        );

        // The font used for editing
        let edit_font = EVFont {
            font_height,
            font_width,
            font_ascent,
            font_descent,
            font_desc,
        };

        let interface_font = FontDescription::from_string(&get_default_interface_font_schema());

        pango_ctx.set_font_description(&interface_font);
        let if_language = pango_ctx
            .get_language()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango language")));
        let if_fontset = pango_ctx
            .load_fontset(&interface_font, &if_language)
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font set")));
        let if_metrics = if_fontset
            .get_metrics()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font metrics")));

        let if_layout = pango::Layout::new(&pango_ctx);
        if_layout.set_text("a");
        let (_, if_log_extents) = layout.get_extents();
        debug!(
            "{}: {:?}",
            gettext("Pango interface font size"),
            log_extents
        );

        let interface_font_height = f64::from(if_log_extents.height) / f64::from(pango::SCALE);
        let interface_font_width = f64::from(if_log_extents.width) / f64::from(pango::SCALE);
        let interface_font_ascent = f64::from(if_metrics.get_ascent()) / f64::from(pango::SCALE);
        let interface_font_descent = f64::from(if_metrics.get_descent()) / f64::from(pango::SCALE);

        debug!(
            "{}: {} {} {} {}",
            gettext("Interface font metrics"),
            interface_font_width,
            interface_font_height,
            interface_font_ascent,
            interface_font_descent
        );

        // The font used for the interface (the linecount)
        let interface_font = EVInterfaceFont {
            font_height: interface_font_height,
            font_width: interface_font_width,
            font_ascent: interface_font_ascent,
            font_descent: interface_font_descent,
            font_desc: interface_font,
        };

        let edit_view = Rc::new(RefCell::new(EditView {
            core: core.clone(),
            main_state: main_state.clone(),
            file_name,
            pristine: true,
            view_id: view_id.to_string(),
            linecount_da: linecount_da.clone(),
            da: da.clone(),
            root_widget: hbox.clone(),
            tab_widget: tab_hbox.clone(),
            label: label.clone(),
            close_button: close_button.clone(),
            hscrollbar: hscrollbar.clone(),
            vscrollbar: vscrollbar.clone(),
            line_cache: LineCache::new(),
            search_bar: search_bar.clone(),
            search_entry: search_entry.clone(),
            find_status_label: find_status_label.clone(),
            edit_font,
            interface_font,
            replace,
        }));

        edit_view.borrow_mut().update_title();

        da.connect_button_press_event(clone!(edit_view => move |_,eb| {
            edit_view.borrow().handle_button_press(eb)
        }));

        da.connect_draw(clone!(edit_view, linecount_da => move |_,ctx| {
            //FIXME: Hack to make sure the linecount is in sync with the text. This should be done more effeciently!
            linecount_da.queue_draw();
            edit_view.borrow_mut().handle_da_draw(&ctx)
        }));

        linecount_da.connect_draw(clone!(edit_view => move |_,ctx| {
            edit_view.borrow_mut().handle_linecount_draw(&ctx)
        }));

        da.connect_key_press_event(clone!(edit_view => move |_, ek| {
            edit_view.borrow_mut().handle_key_press_event(ek)
        }));

        da.connect_motion_notify_event(clone!(edit_view => move |_,em| {
            edit_view.borrow_mut().handle_drag(em)
        }));

        search_entry.connect_search_changed(clone!(edit_view => move |w| {
            if let Some(text) = w.get_text() {
                edit_view.borrow_mut().search_changed(Some(text.to_string()));
            } else {
                edit_view.borrow_mut().search_changed(None);
            }
        }));

        search_entry.connect_activate(clone!(edit_view => move |_| {
            edit_view.borrow_mut().find_next();
        }));

        search_entry.connect_stop_search(clone!(edit_view => move |_| {
            edit_view.borrow().stop_search();
        }));

        replace_expander.connect_property_expanded_notify(clone!(replace_revealer => move|w| {
            if w.get_expanded() {
                replace_revealer.set_reveal_child(true);
            } else {
                replace_revealer.set_reveal_child(false);
            }
        }));

        replace_button.connect_clicked(clone!(edit_view => move |_| {
            edit_view.borrow().replace();
        }));

        replace_all_button.connect_clicked(clone!(edit_view => move |_| {
            edit_view.borrow().replace_all();
        }));

        go_down_button.connect_clicked(clone!(edit_view => move |_| {
            edit_view.borrow_mut().find_next();
        }));

        go_up_button.connect_clicked(clone!(edit_view => move |_| {
            edit_view.borrow_mut().find_prev();
        }));

        da.connect_realize(|w| {
            // Set the text cursor
            if let Some(disp) = DisplayManager::get().get_default_display() {
                let cur = Cursor::new_for_display(&disp, CursorType::Xterm);
                if let Some(win) = w.get_window() {
                    win.set_cursor(&cur)
                }
            }
            w.grab_focus();
        });

        da.connect_scroll_event(clone!(edit_view => move |_,es| {
            edit_view.borrow_mut().handle_scroll(es)
        }));

        da.connect_size_allocate(clone!(edit_view => move |_,alloc| {
            debug!("{}: {}={} {}={}", gettext("Size changed to"), gettext("width"), alloc.width, gettext("height"), alloc.height);
            edit_view.borrow_mut().da_size_allocate(alloc.width, alloc.height);
            edit_view.borrow().do_resize(&edit_view.borrow().view_id,alloc.width, alloc.height);
        }));

        vscrollbar.connect_change_value(clone!(edit_view => move |_,_,value| {
            edit_view.borrow_mut().vscrollbar_change_value(value)
        }));

        crate::MainWin::set_language(&core, view_id, "Plain Text");

        edit_view
    }
}

/// Convert gtk modifiers to xi ones
fn convert_gtk_modifier(mt: ModifierType) -> u32 {
    let mut ret = 0;
    if mt.contains(ModifierType::SHIFT_MASK) {
        ret |= rpc::XI_SHIFT_KEY_MASK;
    }
    if mt.contains(ModifierType::CONTROL_MASK) {
        ret |= rpc::XI_CONTROL_KEY_MASK;
    }
    if mt.contains(ModifierType::MOD1_MASK) {
        ret |= rpc::XI_ALT_KEY_MASK;
    }
    ret
}

impl EditView {
    /// Set the name of the file the EditView is currently editing and calls [update_title](struct.EditView.html#method.update_title)
    pub fn set_file(&mut self, file_name: &str) {
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

        trace!("{} {}", gettext("Setting title to"), full_title);
        self.label.set_text(&full_title);
    }

    /// If xi-editor sends us a [config_changed](https://xi-editor.io/docs/frontend-protocol.html#config_changed)
    /// msg we process it here, e.g. setting the font face/size xi-editor tells us. Most configs don't
    /// need special handling by us though.
    pub fn config_changed(&mut self, changes: &Value) {
        if let Some(map) = changes.as_object() {
            for (name, value) in map {
                match name.as_ref() {
                    "font_size" => {
                        if let Some(font_size) = value.as_u64() {
                            self.edit_font
                                .font_desc
                                .set_size(font_size as i32 * pango::SCALE);
                        }
                    }
                    "font_face" => {
                        if let Some(font_face) = value.as_str() {
                            debug!("{}: {}", gettext("Setting font to"), font_face);
                            self.edit_font.font_desc.set_family(font_face);
                        }
                    }
                    // These are handled in main_win via XiConfig
                    "auto_indent" => (),
                    "autodetect_whitespace" => (),
                    "plugin_search_path" => (),
                    "scroll_past_end" => (),
                    "tab_size" => (),
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
    /// is pristine (has unsaved changes) and queue a new draw of the EditView.
    pub fn update(&mut self, params: &Value) {
        let update = &params["update"];
        self.line_cache.apply_update(update);

        // let (text_width, text_height) = self.get_text_size();
        // debug!("{}{}", text_width, text_height);
        // let (lwidth, lheight) = self.layout.get_size();
        // debug!("{}{}", lwidth, lheight);
        // if (lwidth as f64) < text_width || (lheight as f64) < text_height {
        //     error!("hi");
        //     self.layout.set_size(text_width as u32 * 2, text_height as u32 * 2);
        // }

        // update scrollbars to the new text width and height
        let (_, text_height) = self.get_text_size();
        let vadj = self.vscrollbar.get_adjustment();
        vadj.set_lower(0f64);
        vadj.set_upper(text_height as f64);
        if vadj.get_value() + vadj.get_page_size() > vadj.get_upper() {
            vadj.set_value(vadj.get_upper() - vadj.get_page_size())
        }

        // let hadj = self.hscrollbar.get_adjustment();
        // hadj.set_lower(0f64);
        // hadj.set_upper(text_width as f64);
        // if hadj.get_value() + hadj.get_page_size() > hadj.get_upper() {
        //     hadj.set_value(hadj.get_upper() - hadj.get_page_size())
        // }

        if let Some(pristine) = update["pristine"].as_bool() {
            if self.pristine != pristine {
                self.pristine = pristine;
                self.update_title();
            }
        }

        // self.change_scrollbar_visibility();

        self.da.queue_draw();
    }

    fn change_scrollbar_visibility(&self) {
        let vadj = self.vscrollbar.get_adjustment();
        let hadj = self.hscrollbar.get_adjustment();

        if vadj.get_value() <= vadj.get_lower()
            && vadj.get_value() + vadj.get_page_size() >= vadj.get_upper()
        {
            self.vscrollbar.hide();
        } else {
            self.vscrollbar.show();
        }

        if hadj.get_value() <= hadj.get_lower()
            && hadj.get_value() + hadj.get_page_size() >= hadj.get_upper()
        {
            self.hscrollbar.hide();
        } else {
            debug!(
                "SHOWING HSCROLLBAR: {} {}-{} {}",
                hadj.get_value(),
                hadj.get_lower(),
                hadj.get_upper(),
                hadj.get_page_size()
            );
            self.hscrollbar.show();
        }
    }

    /// Maps x|y pixel coordinates to the line num and col. This can be used e.g. for
    /// determining the firt and last time, but setting the y coordinate to 0 and the
    /// last pixel.
    pub fn da_px_to_cell(&self, main_state: &MainState, x: f64, y: f64) -> (u64, u64) {
        // let first_line = (vadj.get_value() / font_extents.height) as usize;
        let x = x + self.hscrollbar.get_adjustment().get_value();
        let y = y + self.vscrollbar.get_adjustment().get_value();

        let mut y = y - self.edit_font.font_descent;
        if y < 0.0 {
            y = 0.0;
        }
        let line_num = (y / self.edit_font.font_height) as u64;
        let index = if let Some(line) = self.line_cache.get_line(line_num) {
            let pango_ctx = self
                .da
                .get_pango_context()
                .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango context")));

            let padding: usize = format!("{}", self.line_cache.height().saturating_sub(1)).len();
            let linecount_layout =
                self.create_layout_for_linecount(&pango_ctx, &main_state, line_num, padding);
            let linecount_offset = f64::from(linecount_layout.get_extents().1.width / pango::SCALE);

            let layout = self.create_layout_for_line(&pango_ctx, &main_state, line);
            let (_, index, trailing) =
                layout.xy_to_index((x - linecount_offset) as i32 * pango::SCALE, 0);
            index + trailing
        } else {
            0
        };
        (index as u64, (y / self.edit_font.font_height) as u64)
    }

    /// Allocate the space our DrawingArea needs.
    fn da_size_allocate(&mut self, da_width: i32, da_height: i32) {
        debug!("{}", gettext("Allocating DrawingArea size"));
        let vadj = self.vscrollbar.get_adjustment();
        vadj.set_page_size(f64::from(da_height));
        let hadj = self.hscrollbar.get_adjustment();
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
        let main_state = self.main_state.borrow();
        let da_height = self.da.get_allocated_height();
        let (_, first_line) = self.da_px_to_cell(&main_state, 0.0, 0.0);
        let (_, last_line) = self.da_px_to_cell(&main_state, 0.0, f64::from(da_height));
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
    fn get_text_size(&self) -> (f64, f64) {
        let da_width = f64::from(self.da.get_allocated_width());
        let da_height = f64::from(self.da.get_allocated_height());
        let num_lines = self.line_cache.height();

        let all_text_height =
            num_lines as f64 * self.edit_font.font_height + self.edit_font.font_descent;
        let height = if da_height > all_text_height {
            da_height
        } else {
            all_text_height
        };

        let all_text_width = self.line_cache.width() as f64 * self.edit_font.font_width;
        let width = if da_width > all_text_width {
            da_width
        } else {
            all_text_width
        };
        (width, height)
    }

    /// Handles the drawing of the EditView. This is called when we get a update from xi-editor or if
    /// gtk requests us to draw the EditView. This draws the background, all lines and the cursor.
    pub fn handle_da_draw(&mut self, cr: &Context) -> Inhibit {
        // let foreground = self.main_state.borrow().theme.foreground;
        let theme = &self.main_state.borrow().theme;

        let da_width = self.da.get_allocated_width();
        let da_height = self.da.get_allocated_height();

        //debug!("Drawing");
        // cr.select_font_face("Mono", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
        // let mut font_options = cr.get_font_options();
        // debug!("font options: {:?} {:?} {:?}", font_options, font_options.get_antialias(), font_options.get_hint_style());
        // font_options.set_hint_style(HintStyle::Full);

        // let (text_width, text_height) = self.get_text_size();
        let num_lines = self.line_cache.height();

        let vadj = self.vscrollbar.get_adjustment();
        let hadj = self.hscrollbar.get_adjustment();
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

        let pango_ctx = self.da.get_pango_context().unwrap();
        pango_ctx.set_font_description(&self.edit_font.font_desc);

        // Draw editing background
        set_source_color(cr, theme.background);
        cr.rectangle(0.0, 0.0, f64::from(da_width), f64::from(da_height));
        cr.fill();

        set_source_color(cr, theme.foreground);

        // Highlight cursor lines
        // for i in first_line..last_line {
        //     cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
        //     if let Some(line) = self.line_cache.get_line(i) {

        //         if !line.cursor().is_empty() {
        //             cr.set_source_rgba(0.23, 0.23, 0.23, 1.0);
        //             cr.rectangle(0f64,
        //                 font_extents.height*((i+1) as f64) - font_extents.ascent - vadj.get_value(),
        //                 da_width as f64,
        //                 font_extents.ascent + font_extents.descent);
        //             cr.fill();
        //         }
        //     }
        // }

        const CURSOR_WIDTH: f64 = 2.0;

        let mut max_width = 0;

        let main_state = self.main_state.borrow();

        for i in first_line..last_line {
            // Keep track of the starting x position
            if let Some(line) = self.line_cache.get_line(i) {
                cr.move_to(
                    -hadj.get_value(),
                    self.edit_font.font_height * (i as f64) - vadj.get_value(),
                );

                let pango_ctx = self
                    .da
                    .get_pango_context()
                    .unwrap_or_else(|| panic!("{}", &gettext("Failed to get Pango context")));

                let layout = self.create_layout_for_line(&pango_ctx, &main_state, line);
                max_width = max(max_width, layout.get_extents().1.width);
                // debug!("width={}", layout.get_extents().1.width);
                update_layout(cr, &layout);
                show_layout(cr, &layout);

                // Well this is stupid, but (for some reason) Pango gets the width of "·" wrong!
                // It only thinks that the width of that char is 5, when it actually is 10 (like all
                // other chars. So we have to replace it with some other char here to trick Pango into
                // drawing the cursor at the correct position later on
                layout.set_text(&layout.get_text().unwrap().replace("·", " "));

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

        hadj.set_upper(f64::from(max_width / pango::SCALE));

        Inhibit(false)
    }

    /// This draws the linecount. We have this as our own widget to make sure we don't mess up text
    /// selection etc.
    pub fn handle_linecount_draw(&mut self, cr: &Context) -> Inhibit {
        let theme = &self.main_state.borrow().theme;
        let linecount_height = self.linecount_da.get_allocated_height();

        let num_lines = self.line_cache.height();

        let vadj = self.vscrollbar.get_adjustment();

        let first_line = (vadj.get_value() / self.edit_font.font_height) as u64;
        let last_line = ((vadj.get_value() + f64::from(linecount_height))
            / self.edit_font.font_height) as u64
            + 1;
        let last_line = min(last_line, num_lines);

        let pango_ctx = self.linecount_da.get_pango_context().unwrap();

        // Make the linecount at least 4 chars big
        let linecount_width = if format!("{} ", last_line).len() > 4 {
            let width = self.interface_font.font_width * format!("{} ", last_line).len() as f64;
            // Make sure the linecount_width is even to properly center the line number
            if width % 2.0 == 0.0 {
                width
            } else {
                width + 1.0
            }
        } else {
            self.interface_font.font_width * 4.0
        };

        const SEP_LINE_WIDTH: f64 = 1.0;

        // Draw linecount background
        set_source_color(cr, theme.gutter);
        cr.rectangle(0.0, 0.0, linecount_width, f64::from(linecount_height));
        cr.fill();

        set_source_color(cr, theme.foreground);
        //FIXME: Xi sends us the 'ln' (logical linenumber) param for this, but that isn't updated on every draw!
        let mut current_line = first_line;

        for i in first_line..last_line {
            // Keep track of the starting x position
            if let Some(line) = self.line_cache.get_line(i) {
                cr.move_to(
                    0.0,
                    self.edit_font.font_height * (i as f64) - vadj.get_value(),
                );
                if line.line_num().is_some() {
                    current_line += 1
                }

                let linecount_layout = self.create_layout_for_linecount(
                    &pango_ctx,
                    &self.main_state.borrow(),
                    current_line,
                    linecount_width as usize,
                );
                update_layout(cr, &linecount_layout);
                show_layout(cr, &linecount_layout);
            }
        }

        // Set the appropriate size for the linecount DrawingArea, otherwise it's only 1 px wide.
        self.linecount_da
            .set_size_request(linecount_width as i32, linecount_height);
        Inhibit(false)
    }

    /// Creates a pango layout for a particular linecount (the count on the left) in the linecache
    fn create_layout_for_linecount(
        &self,
        pango_ctx: &pango::Context,
        _main_state: &MainState,
        n: u64,
        padding: usize,
    ) -> pango::Layout {
        let line_view = format!(
            "{:^offset$}",
            n,
            offset = padding / self.interface_font.font_width as usize + 1
        );
        let layout = pango::Layout::new(pango_ctx);
        layout.set_font_description(&self.interface_font.font_desc);
        layout.set_text(line_view.as_str());
        layout
    }

    /// Checks how wide a line is
    pub fn line_width(&self, line_string: &str) -> f64 {
        let line = Line::from_json(
            &serde_json::json!({
                "text": line_string,
            }),
            None,
        );
        let main_state = self.main_state.borrow();
        let pango_ctx = self.da.get_pango_context().unwrap();
        let linecount_layout = self.create_layout_for_line(&pango_ctx, &main_state, &line);

        f64::from(linecount_layout.get_extents().1.width / pango::SCALE)
    }

    /// Creates a pango layout for a particular line in the linecache
    fn create_layout_for_line(
        &self,
        pango_ctx: &pango::Context,
        main_state: &MainState,
        line: &Line,
    ) -> pango::Layout {
        let line_view = if line.text().ends_with('\n') {
            &line.text()[0..line.text().len() - 1]
        } else {
            &line.text()
        };

        // Replace spaces with '·'. Do this here since we only want
        // to draw this, we don't want to save the file like that.
        let line_view = if get_draw_trailing_spaces_schema() && line_view.ends_with(' ') {
            let last_char = line_view.trim_end().len();
            let (line_view_without_space, spaces) = line_view.split_at(last_char);
            let space_range = std::ops::Range {
                start: 0,
                end: spaces.len(),
            };
            let highlighted_spaces: String = space_range.into_iter().map(|_| "·").collect();

            format!("{}{}", line_view_without_space, highlighted_spaces)
        } else {
            line_view.to_string()
        };

        // let layout = create_layout(cr).unwrap();
        let layout = pango::Layout::new(pango_ctx);
        layout.set_font_description(&self.edit_font.font_desc);
        layout.set_text(&line_view);

        let mut ix = 0;
        let attr_list = pango::AttrList::new();
        for style in &line.styles {
            let start_index = (ix + style.start) as u32;
            let end_index = (ix + style.start + style.len as i64) as u32;

            let foreground = main_state.styles.get(style.id).and_then(|s| s.fg_color);
            if let Some(foreground) = foreground {
                let mut attr = Attribute::new_foreground(
                    foreground.r_u16(),
                    foreground.g_u16(),
                    foreground.b_u16(),
                )
                .unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            let background = main_state.styles.get(style.id).and_then(|s| s.bg_color);
            if let Some(background) = background {
                let mut attr = Attribute::new_background(
                    background.r_u16(),
                    background.g_u16(),
                    background.b_u16(),
                )
                .unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            let weight = main_state.styles.get(style.id).and_then(|s| s.weight);
            if let Some(weight) = weight {
                let mut attr =
                    Attribute::new_weight(pango::Weight::__Unknown(weight as i32)).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            let italic = main_state.styles.get(style.id).and_then(|s| s.italic);
            if let Some(italic) = italic {
                let mut attr = if italic {
                    Attribute::new_style(pango::Style::Italic).unwrap()
                } else {
                    Attribute::new_style(pango::Style::Normal).unwrap()
                };
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.insert(attr);
            }

            let underline = main_state.styles.get(style.id).and_then(|s| s.underline);
            if let Some(underline) = underline {
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

        layout.set_attributes(&attr_list);
        layout
    }

    /// Scrolls vertically to the line specified and horizontally to the column specified.
    pub fn scroll_to(&mut self, line: u64, col: u64) {
        {
            let cur_top =
                self.edit_font.font_height * ((line + 1) as f64) - self.edit_font.font_ascent;
            let cur_bottom = cur_top + self.edit_font.font_ascent + self.edit_font.font_descent;
            let vadj = self.vscrollbar.get_adjustment();
            if cur_top < vadj.get_value() {
                vadj.set_value(cur_top);
            } else if cur_bottom > vadj.get_value() + vadj.get_page_size()
                && vadj.get_page_size() != 0.0
            {
                vadj.set_value(cur_bottom - vadj.get_page_size());
            }
        }

        {
            let cur_left = self.edit_font.font_width * (col as f64) - self.edit_font.font_ascent;
            let cur_right = cur_left + self.edit_font.font_width * 2.0;
            let hadj = self.hscrollbar.get_adjustment();
            if cur_left < hadj.get_value() {
                hadj.set_value(cur_left);
            } else if cur_right > hadj.get_value() + hadj.get_page_size()
                && hadj.get_page_size() != 0.0
            {
                let new_value = hadj.get_page_size() - cur_right;
                if new_value + hadj.get_page_size() > hadj.get_upper() {
                    hadj.set_upper(new_value + hadj.get_page_size());
                }
                hadj.set_value(new_value);
            }
        }
    }

    /// Handles button presses such as Shift, Ctrl etc. and primary pasting (i.e. via Ctrl+V, not
    /// via middle mouse click).
    pub fn handle_button_press(&self, eb: &EventButton) -> Inhibit {
        self.da.grab_focus();

        let (x, y) = eb.get_position();
        let (col, line) = {
            let main_state = self.main_state.borrow();
            self.da_px_to_cell(&main_state, x, y)
        };

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
        let (col, line) = {
            let main_state = self.main_state.borrow();
            self.da_px_to_cell(&main_state, x, y)
        };
        self.core.borrow().drag(
            &self.view_id,
            line,
            col,
            convert_gtk_modifier(em.get_state()),
        );
        Inhibit(false)
    }

    /// Handles scroll events, i.e. the user dragging the scrollbar, scrolling via a mouse wheel
    /// or via a touchpad/drawing tablet (which use SmoothScrolling, which may scroll vertically
    /// and horizontally at the same time).
    pub fn handle_scroll(&mut self, es: &EventScroll) -> Inhibit {
        self.da.grab_focus();
        // TODO: Make this user configurable!
        let amt = self.edit_font.font_height;

        let vadj = self.vscrollbar.get_adjustment();
        let hadj = self.hscrollbar.get_adjustment();
        match es.get_direction() {
            ScrollDirection::Smooth => {
                let (scroll_change_hori, scroll_change_vert) = match es.get_scroll_deltas() {
                    Some(v) => v,
                    None => {
                        error!("{}", gettext("Smooth scrolling failed"));
                        (0.0, 0.0)
                    }
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
    fn handle_key_press_event(&mut self, ek: &EventKey) -> Inhibit {
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
                            self.core.borrow().insert(view_id, &c.to_string());
                        }
                        _ => {
                            debug!("unhandled key: {:?}", ch);
                        }
                    }
                }
            }
        };
        Inhibit(true)
    }

    /// Copies text to the clipboard
    fn do_cut(&self, view_id: &str) {
        if let Some(text) = self.core.borrow_mut().cut(view_id) {
            Clipboard::get(&SELECTION_CLIPBOARD).set_text(&text);
        }
    }

    /// Copies text to the clipboard
    fn do_copy(&self, view_id: &str) {
        if let Some(text) = self.core.borrow_mut().copy(view_id) {
            Clipboard::get(&SELECTION_CLIPBOARD).set_text(&text);
        }
    }

    /// Pastes text from the clipboard into the EditView
    fn do_paste(&self, view_id: &str) {
        // if let Some(text) = Clipboard::get(&SELECTION_CLIPBOARD).wait_for_text() {
        //     self.core.borrow().insert(view_id, &text);
        // }
        let view_id2 = view_id.to_string().clone();
        let core = self.core.clone();
        Clipboard::get(&SELECTION_CLIPBOARD).request_text(move |_, text| {
            core.borrow().insert(&view_id2, &text);
        });
    }

    fn do_paste_primary(&self, view_id: &str, line: u64, col: u64) {
        // if let Some(text) = Clipboard::get(&SELECTION_PRIMARY).wait_for_text() {
        //     self.core.borrow().insert(view_id, &text);
        // }
        let view_id2 = view_id.to_string().clone();
        let core = self.core.clone();
        Clipboard::get(&SELECTION_PRIMARY).request_text(move |_, text| {
            core.borrow().gesture_point_select(&view_id2, line, col);
            core.borrow().insert(&view_id2, &text);
        });
    }

    /// Resize the EditView
    fn do_resize(&self, view_id: &str, width: i32, height: i32) {
        self.core.borrow().resize(view_id, width, height);
    }

    /// Opens the find dialog (Ctrl+F)
    pub fn start_search(&self) {
        self.search_bar.set_search_mode(true);
        self.replace.replace_expander.set_expanded(false);
        self.replace.replace_revealer.set_reveal_child(false);
        self.search_entry.grab_focus();
        let needle = self.search_entry.get_text().unwrap();
        self.core
            .borrow()
            .find(&self.view_id, &needle, false, Some(false));
    }

    /// Opens the replace dialog (Ctrl+R)
    pub fn start_replace(&self) {
        self.search_bar.set_search_mode(true);
        self.replace.replace_expander.set_expanded(true);
        self.replace.replace_revealer.set_reveal_child(true);
        self.search_entry.grab_focus();
    }

    /// Closes the find/replace dialog
    pub fn stop_search(&self) {
        self.search_bar.set_search_mode(false);
        self.da.grab_focus();
    }

    /// Displays how many matches have been found in the find/replace dialog.
    pub fn find_status(&self, queries: &Value) {
        if let Some(queries) = queries.as_array() {
            for query in queries {
                if let Some(query_obj) = query.as_object() {
                    if let Some(matches) = query_obj["matches"].as_u64() {
                        self.find_status_label
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
            self.replace.replace_entry.set_text(chars);
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
        self.core
            .borrow()
            .find(&self.view_id, &needle, false, Some(false));
    }

    /// Replace _one_ match with the replacement string
    pub fn replace(&self) {
        if let Some(replace_chars) = self.replace.replace_entry.get_text() {
            self.core
                .borrow()
                .replace(&self.view_id, replace_chars.as_str(), false);
            self.core.borrow().replace_next(&self.view_id);
        }
    }

    /// Replace _all_ matches with the replacement string
    pub fn replace_all(&self) {
        if let Some(replace_chars) = self.replace.replace_entry.get_text() {
            self.core
                .borrow()
                .replace(&self.view_id, replace_chars.as_str(), false);
            self.core.borrow().replace_all(&self.view_id);
        }
    }
}
