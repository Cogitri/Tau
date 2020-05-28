// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019 Tom Steu <steudtner.tom@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

use crate::draw_invisible;
use crate::fonts::Font;
use crate::main_state::{MainState, ShowInvisibles};
use crate::theme::{color_from_u32, set_margin_source_color, set_source_color, PangoColor};
use crate::view_item::{FindReplace, TopBar, ViewItem};
use cairo::Context;
use crossbeam_channel::{unbounded, Sender};
use gdk::{
    enums::key, EventButton, EventKey, EventType, ModifierType, SELECTION_CLIPBOARD,
    SELECTION_PRIMARY,
};
use gettextrs::gettext;
use gio::prelude::*;
use glib::{clone, source::Continue, MainContext, PRIORITY_HIGH};
use gtk::prelude::*;
use gtk::{ApplicationWindow, Clipboard, CssProvider, Grid, IMContextSimple, MenuButton, TreePath};
use log::{debug, info, trace, warn};
use pango::{Attribute, Direction, FontDescription, TabAlign, TabArray};
use pangocairo::functions as pangocairofuncs;
use parking_lot::Mutex;
use std::cell::{Cell, RefCell};
use std::cmp::{max, min};
use std::rc::Rc;
use std::sync::Arc;
use std::u32;
use tau_linecache::{Line, LineCache};
use tau_rpc::{Client, ConfigChanges, Query, Status, ThemeChanged, Update, ViewId};
use unicode_segmentation::UnicodeSegmentation;

/// Returned by `EditView::get_text_size()` and used to adjust the scrollbars.
pub struct TextSize {
    /// The height of the entire document
    height: f64,
    /// The width of the entire document
    width: f64,
    /// If the height of the document is contained within the edit_area (if it's smaller)
    #[allow(dead_code)]
    contained_height: bool,
    /// If the width of the document is contained within the edit_area (if it's smaller)
    contained_width: bool,
}

/// The `EditView` is the part of Tau that does the actual editing. This is where you edit documents.
pub struct EditView {
    pub(crate) core: Rc<Client>,
    pub(crate) main_state: Rc<RefCell<MainState>>,
    pub view_id: ViewId,
    pub file_name: RefCell<Option<String>>,
    pub pristine: RefCell<bool>,
    pub root_widget: Grid,
    pub top_bar: TopBar,
    pub view_item: ViewItem,
    line_cache: Arc<Mutex<LineCache>>,
    pub(crate) find_replace: FindReplace,
    pub(crate) edit_font: RefCell<Font>,
    interface_font: Font,
    im_context: IMContextSimple,
    update_sender: Sender<Update>,
    pub(crate) default_tab_size: RefCell<u32>,
    pub(crate) tab_size: RefCell<Option<u32>>,
    style_provider: CssProvider,
    in_multicursor_edit: Cell<bool>,
}

impl EditView {
    /// Initialises a new EditView. Sets up scrollbars, the actual editing area, the fonts,
    /// the syntax lang and connects all events which might happen during usage (e.g. scrolling)
    pub fn new(
        main_state: &Rc<RefCell<MainState>>,
        core: Rc<Client>,
        // The FindReplace dialog is relative to this
        hamburger_button: &MenuButton,
        file_name: Option<String>,
        view_id: ViewId,
        parent: &ApplicationWindow,
    ) -> Rc<Self> {
        trace!("Creating new EditView '{}'", view_id);
        let gschema = main_state.borrow().settings.gschema.clone();
        let default_tab_size = gschema.get::<u32>("tab-size");
        let view_item = ViewItem::new(default_tab_size, hamburger_button);
        let find_replace = FindReplace::new(&hamburger_button);
        let pango_ctx = view_item.get_pango_ctx();
        let im_context = IMContextSimple::new();
        let interface_font = Font::new(
            &pango_ctx,
            FontDescription::from_string(&main_state.borrow().settings.interface_font),
        );

        // We should it by default, so need to do anything if this is true
        if !main_state.borrow().settings.show_linecount {
            view_item.linecount.hide();
        }

        let (update_sender, update_recv) = unbounded();

        let edit_view = Rc::new(Self {
            core,
            main_state: main_state.clone(),
            file_name: RefCell::new(file_name),
            pristine: RefCell::new(true),
            view_id,
            root_widget: view_item.root_box.clone(),
            top_bar: TopBar::new(),
            view_item: view_item.clone(),
            line_cache: Arc::new(Mutex::new(LineCache::new())),
            edit_font: RefCell::new(Font::new(
                &pango_ctx,
                FontDescription::from_string(&main_state.borrow().settings.edit_font),
            )),
            interface_font,
            find_replace: find_replace.clone(),
            im_context: im_context.clone(),
            update_sender,
            default_tab_size: RefCell::new(default_tab_size),
            tab_size: RefCell::new(None),
            style_provider: CssProvider::new(),
            in_multicursor_edit: Cell::new(false),
        });

        view_item
            .edit_area
            .get_style_context()
            .add_provider(&edit_view.style_provider, 800);

        edit_view.update_title();

        view_item.connect_events(&edit_view);
        find_replace.connect_events(&edit_view);
        EditView::connect_im_events(&edit_view, &im_context);

        im_context.set_client_window(parent.get_window().as_ref());

        std::thread::spawn(
            clone!(@strong edit_view.line_cache as linecache, @strong edit_view.update_sender as _sender => move || {
                while let Ok(update) =  update_recv.recv() {
                    linecache.lock().update(update);
                }
                info!("Xi-Update sender disconnected");
            }),
        );

        // Don't panic here - this will be invoked on shutdown after the `EditView` has been destroyed already
        // when saving the window state.
        gschema.connect_changed(
            clone!(@weak gschema, @weak edit_view => @default-return (), move |_, key| {
            trace!("Key '{}' has changed!", key);
            if key == "tab-size" {
                    let val = gschema.get("tab-size");
                    edit_view.set_default_tab_size(val);
                }
            }),
        );

        edit_view
            .view_item
            .set_avail_langs(&main_state.borrow().avail_languages);

        edit_view.view_item.ev_scrolled_window.grab_focus();

        edit_view.update_visible_scroll_region();

        edit_view
    }

    fn connect_im_events(edit_view: &Rc<EditView>, im_context: &IMContextSimple) {
        im_context.connect_commit(clone!(@weak edit_view => @default-panic, move |_, text| {
            edit_view.core.insert(edit_view.view_id, text);
        }));
    }
}

impl EditView {
    /// Set the name of the file the EditView is currently editing and calls [update_title](struct.EditView.html#method.update_title)
    pub fn set_file(&self, file_name: &str) {
        trace!(
            "Setting file name of EditView '{}' to '{}'",
            self.view_id,
            file_name
        );
        self.file_name.replace(Some(file_name.to_string()));
        self.update_title();
    }

    /// Update the title of the EditView to the currently set file_name
    fn update_title(&self) {
        let title = match *self.file_name.borrow() {
            Some(ref f) => f
                .split(::std::path::MAIN_SEPARATOR)
                .last()
                .unwrap_or(&gettext("Untitled"))
                .to_string(),
            None => gettext("Untitled"),
        };

        let mut full_title = String::new();
        if !*self.pristine.borrow() {
            full_title.push('*');
        }

        let mut graphemes: Vec<_> = UnicodeSegmentation::graphemes(title.as_str(), true).collect();
        let num_graphemes = graphemes.len();

        if num_graphemes < 24 {
            full_title.push_str(&title);
        } else {
            graphemes.splice(12..num_graphemes - 12, ["…"].iter().copied());
            graphemes.iter().for_each(|gr| {
                full_title.push_str(gr);
            });
        }

        trace!(
            "Setting title for EditView '{}': {}",
            self.view_id,
            full_title
        );
        self.top_bar.label.set_text(&full_title);
        self.top_bar
            .tab_widget
            .set_tooltip_text(self.file_name.borrow().as_ref().map(|x| x.as_ref()));
    }

    /// If xi-editor sends us a [theme_changed](https://xi-editor.io/docs/frontend-protocol.html#theme_changed)
    pub fn theme_changed(&self, changed: &ThemeChanged) {
        trace!(
            "Handling msg: 'theme_changed' for EditView '{}': {:?}",
            self.view_id,
            changed
        );

        if let Some(color) = changed.theme.caret {
            self.style_provider
                .load_from_data(
                    format!(
                        "* {{ caret-color: rgba({}, {}, {}, {});}}",
                        color.r,
                        color.g,
                        color.b,
                        f64::from(color.a) / 255.0
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
    }

    /// If xi-editor sends us a [config_changed](https://xi-editor.io/docs/frontend-protocol.html#config_changed)
    /// msg we process it here, e.g. setting the font face/size xi-editor tells us. Most configs don't
    /// need special handling by us though.
    pub fn config_changed(&self, changes: &ConfigChanges) {
        trace!(
            "Handling msg: 'config_changed' for EditView '{}': {:?}",
            self.view_id,
            changes
        );

        if let Some(font_size) = changes.font_size {
            let pango_ctx = self.view_item.get_pango_ctx();
            let mut font_desc = self.edit_font.borrow().font_desc.clone();
            font_desc.set_size(font_size as i32 * pango::SCALE);
            // We've set the new fontsize previously, now we have to regenerate the font height/width etc.
            self.edit_font.replace(Font::new(&pango_ctx, font_desc));
            self.view_item.edit_area.queue_draw();
        }

        if let Some(font_face) = &changes.font_face {
            debug!("Setting edit font to '{}'", font_face);
            let pango_ctx = self.view_item.get_pango_ctx();
            let font_size = self.edit_font.borrow().font_desc.get_size();
            self.edit_font.replace(Font::new(
                &pango_ctx,
                FontDescription::from_string(&format!(
                    "{} {}",
                    font_face,
                    font_size / pango::SCALE
                )),
            ));
            self.view_item.edit_area.queue_draw();
        }
    }

    /// If xi-editor sends us a [update](https://xi-editor.io/docs/frontend-protocol.html#config_changed)
    /// msg we process it here, setting the scrollbars upper limit accordingly, checking if the EditView
    /// is pristine (_does not_ has unsaved changes) and queue a new draw of the EditView.
    pub fn update(&self, params: Update) {
        trace!(
            "Handling msg: 'update' for EditView '{}': {:?}",
            self.view_id,
            params
        );

        self.pristine.replace(params.pristine);
        self.update_title();

        self.update_sender.send(params).unwrap();

        // update scrollbars to the new text width and height
        let text_size = self.get_text_size();
        let text_height = text_size.height;
        let text_width = if text_size.contained_width {
            text_size.width
        } else {
            text_size.width + self.edit_font.borrow().font_width * 4.0
        };

        self.view_item
            .edit_area
            .set_size(text_width as u32, text_height as u32);

        self.view_item.edit_area.queue_draw();
        self.view_item.linecount.queue_draw();
    }

    /// Maps x|y pixel coordinates to the line num and col. This can be used e.g. for
    /// determining the first and last line, by setting the y coordinate to 0 and the
    /// last pixel.
    pub fn da_px_to_cell(&self, x: f64, y: f64) -> (u64, u64) {
        trace!(
            "Handling msg: 'da_px_to_cell' for EditView '{}': x: {} y: {}",
            self.view_id,
            x,
            y
        );
        let x = x + self.view_item.hadj.get_value();
        let y = y + self.view_item.vadj.get_value();

        let mut y = y - self.edit_font.borrow().font_descent;
        if y < 0.0 {
            y = 0.0;
        }
        let line_num = (y / self.edit_font.borrow().font_height) as u64;
        let index = if let Some(line) = self.line_cache.lock().get_line(line_num) {
            let pango_ctx = self.view_item.get_pango_ctx();

            let layout = self.create_layout_for_line(&pango_ctx, line, &self.get_tabs());
            let (_, index, trailing) = layout.xy_to_index(x as i32 * pango::SCALE, 0);

            let byte_index = (index + trailing) as u64;

            let last_char = UnicodeSegmentation::graphemes(line.text.as_str(), true).last();
            let last_char_byte_width = last_char.map(|s| s.as_bytes().len()).unwrap_or(1) as u64;

            // This is messy, but we have to add the length of the last character to the index,
            // since pango (?) otherwise always assumes that it's only one byte long, when it
            // can actually be multiple bytes long.
            if byte_index + last_char_byte_width - 1 == line.text.bytes().len() as u64 {
                byte_index + (last_char_byte_width - 1) as u64
            } else {
                byte_index
            }
        } else {
            0
        };

        (index, (y / self.edit_font.borrow().font_height) as u64)
    }

    /// Allocate the space our DrawingArea needs.
    pub(crate) fn da_size_allocate(&self, da_width: i32, da_height: i32) {
        debug!(
            "Allocating editing pane size: width: '{}', height: '{}'",
            da_width, da_height,
        );

        self.update_visible_scroll_region();
    }

    /// This updates the part of the document that's visible to the user, e.g. when scrolling.
    /// This requests the required lines from xi-editor to add them to the line cache and then
    /// adjusts the scrolling to the visible region.
    pub(crate) fn update_visible_scroll_region(&self) {
        trace!(
            "Handling msg: 'update_visible_scroll_region' for EditView '{}'",
            self.view_id
        );
        let da_height = self.view_item.edit_area.get_allocated_height();
        let vadj = &self.view_item.vadj;
        let first_line = (vadj.get_value() / self.edit_font.borrow().font_height) as u64;
        let last_line = ((vadj.get_value() + f64::from(da_height))
            / self.edit_font.borrow().font_height) as u64
            + 1;

        debug!(
            "Updating visible scroll region: first: '{}', last: '{}'",
            first_line, last_line
        );

        self.core.scroll(self.view_id, first_line, last_line);
        self.view_item.linecount.queue_draw();
    }

    /// Returns the width&height of the entire document
    fn get_text_size(&self) -> TextSize {
        trace!(
            "Handling msg: 'get_text_size' for EditView '{}'",
            self.view_id
        );

        let mut contained_height = false;
        let mut contained_width = false;

        let da_width = f64::from(self.view_item.edit_area.get_allocated_width());
        let da_height = f64::from(self.view_item.edit_area.get_allocated_height());
        let num_lines = self.line_cache.lock().height();

        let all_text_height = num_lines as f64 * self.edit_font.borrow().font_height
            + self.edit_font.borrow().font_descent;
        let height = if da_height > all_text_height {
            contained_height = true;
            da_height
        } else {
            all_text_height
        };

        let vadj = &self.view_item.vadj;
        let first_line = (vadj.get_value() / self.edit_font.borrow().font_height) as u64;
        let last_line =
            (vadj.get_value() + da_height / self.edit_font.borrow().font_height) as u64 + 1;
        let last_line = min(last_line, num_lines as u64);
        // Set this to pango::SCALE, we divide by that later on.
        let mut max_width = pango::SCALE;

        let pango_ctx = self.view_item.get_pango_ctx();
        let tabs = self.get_tabs();

        // Determine the longest line as per Pango. Creating layouts with Pango here is kind of expensive
        // here, but it's hard determining an accurate width otherwise.
        for i in first_line..last_line {
            if let Some(line) = self.line_cache.lock().get_line(i) {
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
    #[allow(clippy::cognitive_complexity)]
    pub fn handle_da_draw(&self, cr: &Context) -> Inhibit {
        // let foreground = self.main_state.borrow().theme.foreground;
        let theme = &self.main_state.borrow().theme;

        let da_width = self.view_item.edit_area.get_allocated_width();
        let da_height = self.view_item.edit_area.get_allocated_height();

        let num_lines = self.line_cache.lock().height();

        let vadj = &self.view_item.vadj;
        let hadj = &self.view_item.hadj;
        trace!(
            "Drawing EditView. Vertical Adj: {}/{}; Horizontal Adj: {}/{}",
            vadj.get_value(),
            vadj.get_upper(),
            hadj.get_value(),
            hadj.get_upper()
        );

        let first_line = (vadj.get_value() / self.edit_font.borrow().font_height) as u64;
        let last_line = ((vadj.get_value() + f64::from(da_height))
            / self.edit_font.borrow().font_height) as u64
            + 1;
        let last_line = min(last_line, num_lines as u64);

        let pango_ctx = self.view_item.get_pango_ctx();
        pango_ctx.set_font_description(&self.edit_font.borrow().font_desc);

        // Draw in darker colour at column X to symbol the user that the's writing further than the margin
        if self.main_state.borrow().settings.right_margin {
            let until_margin_width = self.edit_font.borrow().font_width
                * f64::from(self.main_state.borrow().settings.column_right_margin);
            // Draw editing background
            set_source_color(cr, theme.background);
            cr.rectangle(0.0, 0.0, until_margin_width, f64::from(da_height));
            cr.fill();

            set_margin_source_color(cr, theme.background);
            // Don't go negative here
            let normal_background_start =
                if until_margin_width - self.view_item.hadj.get_value() < 0.0 {
                    0.0
                } else {
                    until_margin_width - self.view_item.hadj.get_value()
                };

            cr.rectangle(
                normal_background_start,
                0.0,
                f64::from(da_width),
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

        let line_cache = self.line_cache.lock();

        for i in first_line..last_line {
            // Keep track of the starting x position
            if let Some(line) = line_cache.get_line(i) {
                if self.main_state.borrow().settings.highlight_line && !line.cursor.is_empty() {
                    set_source_color(cr, theme.line_highlight);
                    cr.rectangle(
                        0.0,
                        self.edit_font.borrow().font_height * i as f64 - vadj.get_value(),
                        f64::from(da_width),
                        self.edit_font.borrow().font_height,
                    );
                    cr.fill();
                }

                set_source_color(cr, theme.foreground);

                cr.move_to(
                    -hadj.get_value(),
                    self.edit_font.borrow().font_height * (i as f64) - vadj.get_value(),
                );

                let pango_ctx = self.view_item.get_pango_ctx();
                let layout = self.create_layout_for_line(&pango_ctx, line, &tabs);
                // debug!("width={}", layout.get_extents().1.width);
                pangocairofuncs::update_layout(cr, &layout);
                pangocairofuncs::show_layout(cr, &layout);

                // make invisibles more transparent and less distractive
                match theme.foreground {
                    Some(mut color) => {
                        color.a /= 3u8;
                        set_source_color(cr, Some(color));
                    }
                    None => {
                        set_source_color(cr, theme.caret);
                    }
                }

                let draw_tab = |rect: &mut dyn Iterator<Item = draw_invisible::Rectangle>| {
                    rect.filter(|r| r.width != 0.0).for_each(|mut r| {
                        r.y = self.edit_font.borrow().font_height * i as f64 - vadj.get_value();
                        r.x -= self.view_item.hadj.get_value();
                        r.draw_tab(cr);
                    })
                };

                match self.main_state.borrow().settings.draw_tabs {
                    ShowInvisibles::Trailing => {
                        draw_tab(&mut draw_invisible::Rectangle::from_layout_index(
                            draw_invisible::tabs::trailing(
                                line.text.replace("\n", "").replace("\r\n", "").as_str(),
                            ),
                            &layout,
                        ));
                    }

                    ShowInvisibles::Leading => {
                        draw_tab(&mut draw_invisible::Rectangle::from_layout_index(
                            draw_invisible::tabs::leading(line.text.as_str()),
                            &layout,
                        ));
                    }

                    ShowInvisibles::All => {
                        draw_tab(&mut draw_invisible::Rectangle::from_layout_index(
                            draw_invisible::tabs::all(line.text.as_str()),
                            &layout,
                        ));
                    }

                    ShowInvisibles::Selected => {
                        let mut style_index = 0u64;
                        for style in line.styles.iter() {
                            style_index += style.offset as u64;
                            if style.style_id == 0 {
                                draw_tab(&mut draw_invisible::Rectangle::from_layout_index(
                                    draw_invisible::tabs::all_from(
                                        line.text.as_str(),
                                        style_index,
                                        style.length,
                                    ),
                                    &layout,
                                ));
                            }
                            style_index += style.length;
                        }
                    }

                    _ => (),
                }

                let draw_space = |rect: &mut dyn Iterator<Item = draw_invisible::Rectangle>| {
                    rect.filter(|r| r.width != 0.0).for_each(|mut r| {
                        r.y = self.edit_font.borrow().font_height * i as f64 - vadj.get_value();
                        r.x -= self.view_item.hadj.get_value();
                        r.draw_space(cr);
                    })
                };

                match self.main_state.borrow().settings.draw_spaces {
                    // If the next line is a soft broken line we don't want to display trailing spaces since they're actually
                    // not trailing: they're required for the linebreak to occur.
                    ShowInvisibles::Trailing
                        if line_cache
                            .get_line(i + 1)
                            .map(|line| line.line_num.is_some())
                            .unwrap_or(true) =>
                    {
                        draw_space(&mut draw_invisible::Rectangle::from_layout_index(
                            draw_invisible::spaces::trailing(
                                line.text.replace("\n", "").replace("\r\n", "").as_str(),
                            ),
                            &layout,
                        ));
                    }

                    ShowInvisibles::Leading => {
                        draw_space(&mut draw_invisible::Rectangle::from_layout_index(
                            draw_invisible::spaces::leading(line.text.as_str()),
                            &layout,
                        ));
                    }

                    ShowInvisibles::All => {
                        draw_space(&mut draw_invisible::Rectangle::from_layout_index(
                            draw_invisible::spaces::all(line.text.as_str()),
                            &layout,
                        ));
                    }

                    ShowInvisibles::Selected => {
                        let mut style_index = 0u64;
                        for style in line.styles.iter() {
                            style_index += style.offset as u64;
                            if style.style_id == 0 {
                                draw_space(&mut draw_invisible::Rectangle::from_layout_index(
                                    draw_invisible::spaces::all_from(
                                        line.text.as_str(),
                                        style_index,
                                        style.length,
                                    ),
                                    &layout,
                                ));
                            }
                            style_index += style.length;
                        }
                    }

                    _ => (),
                }

                if self.main_state.borrow().settings.draw_cursor {
                    let style_context = self.view_item.edit_area.get_style_context();
                    for c in &line.cursor {
                        gtk::render_insertion_cursor(
                            &style_context,
                            cr,
                            1.0 - self.view_item.hadj.get_value(),
                            self.edit_font.borrow().font_height * i as f64 - vadj.get_value(),
                            &layout,
                            *c as i32,
                            Direction::Neutral,
                        )
                    }
                }
            }
        }

        Inhibit(false)
    }

    /// This draws the linecount. We have this as our own widget to make sure we don't mess up text
    /// selection etc.
    pub fn handle_linecount_draw(&self, cr: &Context) -> Inhibit {
        trace!("Handling: 'linecount_draw' for EditView '{}'", self.view_id);

        let theme = &self.main_state.borrow().theme;
        let linecount_height = self.view_item.linecount.get_allocated_height();

        let num_lines = self.line_cache.lock().height();

        let vadj = &self.view_item.vadj;

        let first_line = (vadj.get_value() / self.edit_font.borrow().font_height) as u64;
        let last_line = ((vadj.get_value() + f64::from(linecount_height))
            / self.edit_font.borrow().font_height) as u64
            + 1;
        let last_line = min(last_line, num_lines as u64);

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

        let mut current_line: Option<u64> = None;
        let center_diff =
            (self.edit_font.borrow().font_height - self.interface_font.font_height) / 2.0;

        set_source_color(cr, theme.foreground);
        for i in first_line..last_line {
            // Keep track of the starting x position
            if let Some(line) = self.line_cache.lock().get_line(i) {
                if let Some(ln) = line.line_num {
                    if let Some(ref mut cl) = current_line {
                        *cl += 1;
                    } else {
                        current_line = Some(ln)
                    }

                    cr.move_to(
                        0.0,
                        self.edit_font.borrow().font_height * (i as f64) - vadj.get_value()
                            + center_diff,
                    );

                    let linecount_layout = self.create_layout_for_linecount(
                        &pango_ctx,
                        current_line.unwrap(),
                        linecount_width as usize,
                    );
                    pangocairofuncs::update_layout(cr, &linecount_layout);
                    pangocairofuncs::show_layout(cr, &linecount_layout);
                }
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
        let tab_size = if let Some(size) = *self.tab_size.borrow() {
            size
        } else {
            *self.default_tab_size.borrow()
        };
        tabs.set_tab(
            0,
            TabAlign::Left,
            self.edit_font.borrow().font_width as i32 * tab_size as i32 * pango::SCALE,
        );

        tabs
    }

    /// Checks how wide a line is
    pub fn line_width(&self, line_string: &str, style_opt: Option<tau_rpc::Style>) -> f64 {
        let pango_ctx = self.view_item.get_pango_ctx();
        let layout = pango::Layout::new(&pango_ctx);
        layout.set_tabs(Some(&self.get_tabs()));
        layout.set_font_description(Some(&self.edit_font.borrow().font_desc));
        layout.set_text(&line_string);

        // No need to handle foreground/background colour here since those do not impact the
        // line width.
        if let Some(style) = style_opt {
            let attr_list = pango::AttrList::new();
            let str_len = line_string.bytes().len();
            if let Some(weight) = style.weight {
                let mut attr =
                    Attribute::new_weight(pango::Weight::__Unknown(weight as i32)).unwrap();
                attr.set_start_index(0);
                attr.set_end_index(str_len as u32);
                attr_list.change(attr);
            }

            if let Some(italic) = style.italic {
                let mut attr = if italic {
                    Attribute::new_style(pango::Style::Italic).unwrap()
                } else {
                    Attribute::new_style(pango::Style::Normal).unwrap()
                };
                attr.set_start_index(0);
                attr.set_end_index(str_len as u32);
                attr_list.change(attr);
            }

            if let Some(underline) = style.underline {
                let mut attr = if underline {
                    Attribute::new_underline(pango::Underline::Single).unwrap()
                } else {
                    Attribute::new_underline(pango::Underline::None).unwrap()
                };
                attr.set_start_index(0);
                attr.set_end_index(str_len as u32);
                attr_list.change(attr);
            }
        }

        f64::from(layout.get_extents().1.width / pango::SCALE)
    }

    /// Creates a pango layout for a particular line in the linecache
    fn create_layout_for_line(
        &self,
        pango_ctx: &pango::Context,
        line: &Line,
        tabs: &TabArray,
    ) -> pango::Layout {
        let layout = pango::Layout::new(pango_ctx);
        layout.set_tabs(Some(tabs));
        layout.set_font_description(Some(&self.edit_font.borrow().font_desc));
        layout.set_text(&line.text);

        // Pango attributes need to be added in reverse order of the style list
        // Find the end index of the last style first
        let mut end_index = 0u32;
        for style in &line.styles {
            end_index = (end_index as i64 + (style.offset + style.length as i64)) as u32;
        }

        let attr_list = pango::AttrList::new();
        for style in line.styles.iter().rev() {
            let start_index = (u64::from(end_index) - style.length) as u32;
            let main_state = self.main_state.borrow();
            let line_style = main_state.styles.get(&(style.style_id as usize));

            if let Some(foreground) = line_style.and_then(|s| s.fg_color) {
                let pango_color = PangoColor::from(color_from_u32(foreground));
                let mut attr =
                    Attribute::new_foreground(pango_color.r, pango_color.g, pango_color.b).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.change(attr);
                let mut alpha_attr = Attribute::new_foreground_alpha(pango_color.a).unwrap();
                alpha_attr.set_start_index(start_index);
                alpha_attr.set_end_index(end_index);
                attr_list.change(alpha_attr);
            }

            if let Some(background) = line_style.and_then(|s| s.bg_color) {
                let pango_color = PangoColor::from(color_from_u32(background));
                let mut attr =
                    Attribute::new_background(pango_color.r, pango_color.g, pango_color.b).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.change(attr);
                let mut alpha_attr = Attribute::new_background_alpha(pango_color.a).unwrap();
                alpha_attr.set_start_index(start_index);
                alpha_attr.set_end_index(end_index);
                attr_list.change(alpha_attr);
            }

            if let Some(weight) = line_style.and_then(|s| s.weight) {
                let mut attr =
                    Attribute::new_weight(pango::Weight::__Unknown(weight as i32)).unwrap();
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.change(attr);
            }

            if let Some(italic) = line_style.and_then(|s| s.italic) {
                let mut attr = if italic {
                    Attribute::new_style(pango::Style::Italic).unwrap()
                } else {
                    Attribute::new_style(pango::Style::Normal).unwrap()
                };
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.change(attr);
            }

            if let Some(underline) = line_style.and_then(|s| s.underline) {
                let mut attr = if underline {
                    Attribute::new_underline(pango::Underline::Single).unwrap()
                } else {
                    Attribute::new_underline(pango::Underline::None).unwrap()
                };
                attr.set_start_index(start_index);
                attr.set_end_index(end_index);
                attr_list.change(attr);
            }

            end_index = (i64::from(start_index) - style.offset) as u32;
        }

        layout.set_attributes(Some(&attr_list));
        layout
    }

    /// Scrolls vertically to the line specified and horizontally to the column specified.
    pub fn scroll_to(&self, line: u64, col: u64) {
        trace!(
            "Handling msg: 'scroll_to' for EditView '{}': Line: '{}'; Column: '{}'",
            self.view_id,
            line,
            col
        );

        // If we do multicursor select we don't have to change the view
        if self.in_multicursor_edit.get() {
            trace!("Aborting scrolling since we're in multicursor select mode");
            self.in_multicursor_edit.set(false);
            return;
        }

        // xi initially sends a 'scroll_to' with line == 0 when the linecache doesn't have lines in
        // it yet, so the below function would keep going forever. Make sure line isn't 0 so it's actually
        // a valid line with a line_number
        if line != 0 {
            let mut n = 0;
            let linecache = self.line_cache.lock();
            // get the first line with a line_num
            while linecache
                .get_line(line - n)
                .and_then(|l| l.line_num)
                .is_none()
            {
                n += 1
            }

            self.view_item.statusbar.line_label.set_text(&format!(
                "{}: {}",
                gettext("Line"),
                linecache
                    .get_line(line - n)
                    .and_then(|l| l.line_num)
                    .unwrap()
            ));
        } else {
            self.view_item
                .statusbar
                .line_label
                .set_text(&format!("{}: {}", gettext("Line"), 1));
        }
        self.view_item
            .statusbar
            .column_label
            .set_text(&format!("{}: {}", gettext("Column"), col));
        {
            // The new height is the current last line + 1
            let new_height = self.edit_font.borrow().font_height * line as f64;
            let padding = self.edit_font.borrow().font_height * 4.0;
            // The font height doesn't include these, so we have to add them for the last line
            let vadj = &self.view_item.vadj;
            // If the cursor above our current view, this is true. Scroll a bit higher than necessary
            // to make sure that during find the text isn't right at the edge of the view.
            if new_height < vadj.get_value() {
                vadj.set_value(new_height - padding);
            // If it's below out current view, this is true. Scroll a bit lower than necessary
            // for the same reasons cited above.
            } else if new_height + padding > vadj.get_value() + vadj.get_page_size()
                && (vadj.get_page_size() as u32 != 0 && vadj.get_page_size() as u32 != 1)
            {
                vadj.set_value(
                    new_height
                        + self.edit_font.borrow().font_height
                        + padding
                        // These two aren't included in the font height and we need them to line up with the line
                        + self.edit_font.borrow().font_ascent
                        + self.edit_font.borrow().font_descent
                        - vadj.get_page_size(),
                );
            }
        }

        {
            // Collect all styles with id 0/1 (selections/find results) to make sure they're in the frame
            if let Some(line) = self.line_cache.lock().get_line(line) {
                let line_selections: Vec<_> = line
                    .styles
                    .iter()
                    .filter(|s| s.style_id == 0 || s.style_id == 1)
                    .collect();

                let mut begin_selection = None;
                let mut end_selection = None;

                for x in line_selections {
                    if let Some(cur) = begin_selection {
                        // Make sure to use the lowest value of any selection so it's in the view
                        begin_selection = Some(min(cur, x.offset));
                    } else {
                        begin_selection = Some(x.offset);
                    }
                    if let Some(cur) = end_selection {
                        // Make sure to use the highest value of any selection so it's in the view
                        end_selection = Some(max(cur, x.offset + x.length as i64));
                    } else {
                        end_selection = Some(x.offset + x.length as i64);
                    }
                }

                let mut line_text = line.text.to_string();
                // Only measure width up to the right column
                line_text.truncate(col as usize);
                let line_length = self.line_width(&line_text, None);

                let min = min(
                    begin_selection.unwrap_or(line_length as i64),
                    line_length as i64,
                ) as f64;
                let max = max(
                    end_selection.unwrap_or(line_length as i64),
                    line_length as i64,
                ) as f64;

                trace!("Horizontal scrolling to min: {}; max: {}", min, max);

                let padding = self.edit_font.borrow().font_width * 4.0;
                let hadj = &self.view_item.hadj;

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
                warn!("Couldn't update hscrollbar value because I couldn't get the line to scroll to!");
            }
        }
    }

    /// Handles button presses such as Shift, Ctrl etc. and primary pasting (i.e. via Ctrl+V, not
    /// via middle mouse click).
    pub fn handle_button_press(&self, eb: &EventButton) -> Inhibit {
        trace!(
            "Handling 'button_press' for EditView '{}': {:?}",
            self.view_id,
            eb
        );
        self.view_item.ev_scrolled_window.grab_focus();

        let (x, y) = eb.get_position();
        let (col, line) = self.da_px_to_cell(x, y);

        match eb.get_button() {
            1 => {
                if eb.get_state().contains(ModifierType::SHIFT_MASK) {
                    self.core.gesture_range_select(self.view_id, line, col);
                    self.do_copy_primary();
                } else if eb.get_state().contains(ModifierType::CONTROL_MASK) {
                    self.core.gesture_toggle_sel(self.view_id, line, col);
                } else if eb.get_event_type() == EventType::DoubleButtonPress {
                    self.core.gesture_word_select(self.view_id, line, col);
                    self.do_copy_primary();
                } else if eb.get_event_type() == EventType::TripleButtonPress {
                    self.core.gesture_line_select(self.view_id, line, col);
                    self.do_copy_primary();
                } else {
                    self.core.gesture_point_select(self.view_id, line, col);
                }
            }
            2 => {
                self.do_paste_primary(line, col);
            }
            3 => {
                self.view_item.context_menu.popup_at_pointer(Some(&eb));
            }
            _ => {}
        }
        Inhibit(false)
    }

    /// Handle selecting line(s) by dragging the mouse across them while having the left mouse
    /// button clicked.
    pub fn handle_drag(&self, x: f64, y: f64) {
        let (col, line) = self.da_px_to_cell(x, y);
        self.core.drag(self.view_id, line, col);
    }

    /// Handles all (special) key press events, e.g. copy, pasting, PgUp/Down etc.
    // Allow this to be a long function since splitting up the matching into multiple functions
    // would be a pain
    #[allow(clippy::cognitive_complexity)]
    pub(crate) fn handle_key_press_event(&self, ek: &EventKey) -> Inhibit {
        trace!(
            "Handling 'key_press_event' for EditView '{}': {:?}",
            self.view_id,
            ek
        );
        debug!(
            "Processing key press: Value={:?}, State={:?}, Length={:?} Group={:?} Unicode={:?}",
            ek.get_keyval(),
            ek.get_state(),
            ek.get_length(),
            ek.get_group(),
            ::gdk::keyval_to_unicode(ek.get_keyval())
        );
        let view_id = self.view_id;

        let alt = ek.get_state().contains(ModifierType::MOD1_MASK);
        let ctrl = ek.get_state().contains(ModifierType::CONTROL_MASK);
        let meta = ek.get_state().contains(ModifierType::META_MASK);
        let shift = ek.get_state().contains(ModifierType::SHIFT_MASK);
        let norm = !alt && !ctrl && !meta;
        match ek.get_keyval() {
            key::Delete if !shift => {
                self.core.delete_forward(view_id);
            }
            key::KP_Delete | key::Delete if norm && shift => {
                self.do_cut();
            }
            key::KP_Insert | key::Insert if !alt && !meta && !shift && ctrl => {
                self.do_copy();
            }
            key::KP_Insert | key::Insert if norm && shift => {
                self.do_paste();
            }
            key::BackSpace if norm => {
                self.core.delete_backward(view_id);
            }
            key::BackSpace if ctrl => {
                self.core.delete_word_backward(view_id);
            }
            key::Return | key::KP_Enter => {
                self.core.insert_newline(view_id);
            }
            key::Tab if norm && !shift => {
                self.core.insert_tab(view_id);
            }
            key::Tab | key::ISO_Left_Tab if norm && shift => {
                self.core.outdent(view_id);
            }
            key::Up | key::KP_Up if norm && !shift => {
                self.core.up(view_id);
            }
            key::Down | key::KP_Down if norm && !shift => {
                self.core.down(view_id);
            }
            key::Left | key::KP_Left if norm && !shift => {
                self.core.left(view_id);
            }
            key::Right | key::KP_Right if norm && !shift => {
                self.core.right(view_id);
            }
            key::Up | key::KP_Up if norm && shift => {
                self.core.up_sel(view_id);
                self.do_copy_primary();
            }
            key::Down | key::KP_Down if norm && shift => {
                self.core.down_sel(view_id);
                self.do_copy_primary();
            }
            key::Left | key::KP_Left if norm && shift => {
                self.core.left_sel(view_id);
                self.do_copy_primary();
            }
            key::Right | key::KP_Right if norm && shift => {
                self.core.right_sel(view_id);
                self.do_copy_primary();
            }
            key::Left | key::KP_Left if ctrl && !shift => {
                self.core.word_left(view_id);
            }
            key::Right | key::KP_Right if ctrl && !shift => {
                self.core.word_right(view_id);
            }
            key::Left | key::KP_Left if ctrl && shift => {
                self.core.word_left_sel(view_id);
                self.do_copy_primary();
            }
            key::Right | key::KP_Right if ctrl && shift => {
                self.core.word_right_sel(view_id);
                self.do_copy_primary();
            }
            key::Home | key::KP_Home if norm && !shift => {
                self.core.line_start(view_id);
            }
            key::End | key::KP_End if norm && !shift => {
                self.core.line_end(view_id);
            }
            key::Home | key::KP_Home if norm && shift => {
                self.core.line_start_sel(view_id);
                self.do_copy_primary();
            }
            key::End | key::KP_End if norm && shift => {
                self.core.line_end_sel(view_id);
                self.do_copy_primary();
            }
            key::Home | key::KP_Home if ctrl && !shift => {
                self.core.document_begin(view_id);
            }
            key::End | key::KP_End if ctrl && !shift => {
                self.core.document_end(view_id);
            }
            key::Home | key::KP_Home if ctrl && shift => {
                self.core.document_begin_sel(view_id);
                self.do_copy_primary();
            }
            key::End | key::KP_End if ctrl && shift => {
                self.core.document_end_sel(view_id);
                self.do_copy_primary();
            }
            key::Page_Up | key::KP_Page_Up if norm && !shift => {
                self.core.page_up(view_id);
            }
            key::Page_Down | key::KP_Page_Down if norm && !shift => {
                self.core.page_down(view_id);
            }
            key::Page_Up | key::KP_Page_Up if norm && shift => {
                self.core.page_up_sel(view_id);
                self.do_copy_primary();
            }
            key::Page_Down | key::KP_Page_Down if norm && shift => {
                self.core.page_down_sel(view_id);
                self.do_copy_primary();
            }
            key::Escape => {
                self.stop_search();
            }
            key::F7 => {
                let mut main_state = self.main_state.borrow_mut();
                let draw_cursor = main_state.settings.draw_cursor;
                main_state.settings.draw_cursor = !draw_cursor;
            }
            key::a | key::backslash | key::slash if ctrl => {
                self.core.select_all(view_id);
                self.do_copy_primary();
            }
            key::c if ctrl => {
                self.do_copy();
            }
            key::v if ctrl => {
                self.do_paste();
            }
            key::x if ctrl => {
                self.do_cut();
            }
            key::z if ctrl => {
                self.core.undo(view_id);
            }
            key::Z if ctrl => {
                self.core.redo(view_id);
            }
            _ => {
                debug!("Inserting non char key");
                self.im_context.filter_keypress(ek);
            }
        };
        Inhibit(true)
    }

    /// Copies text to the clipboard
    pub fn do_cut(&self) {
        debug!("Adding cutting text op to idle queue");

        let (clipboard_tx, clipboard_rx) =
            MainContext::sync_channel::<serde_json::value::Value>(PRIORITY_HIGH, 1);

        clipboard_rx.attach(None, move |val| {
            if let Some(ref text) = val.as_str() {
                Clipboard::get(&SELECTION_CLIPBOARD).set_text(&text);
            }

            Continue(false)
        });

        self.core.cut(self.view_id, move |res| {
            if let Ok(val) = res {
                clipboard_tx.send(val).unwrap();
            }
        });
    }

    /// Copies text to the clipboard
    pub fn do_copy(&self) {
        debug!("Adding copying text op to idle queue");

        let (clipboard_tx, clipboard_rx) =
            MainContext::sync_channel::<serde_json::value::Value>(PRIORITY_HIGH, 1);

        clipboard_rx.attach(None, move |val| {
            if let Some(text) = val.as_str() {
                Clipboard::get(&SELECTION_CLIPBOARD).set_text(text);
                Clipboard::get(&SELECTION_PRIMARY).set_text(text);
            }

            Continue(false)
        });

        self.core.copy(self.view_id, move |res| {
            if let Ok(val) = res {
                clipboard_tx.send(val).unwrap();
            }
        });
    }

    /// Copies text to primary clipboard
    pub fn do_copy_primary(&self) {
        debug!("Adding selection text op to idle queue");

        let (clipboard_tx, clipboard_rx) =
            MainContext::sync_channel::<serde_json::value::Value>(PRIORITY_HIGH, 1);

        clipboard_rx.attach(None, move |val| {
            if let Some(text) = val.as_str() {
                Clipboard::get(&SELECTION_PRIMARY).set_text(text);
            }

            Continue(false)
        });

        self.core.copy(self.view_id, move |res| {
            if let Ok(val) = res {
                clipboard_tx.send(val).unwrap();
            }
        });
    }

    /// Pastes text from the clipboard into the EditView
    pub fn do_paste(&self) {
        debug!("Pasting text");

        Clipboard::get(&SELECTION_CLIPBOARD).request_text(
            clone!(@strong self.core as core, @strong self.view_id as view_id => move |_, text| {
                if let Some(clip_content) = text {
                    core.insert(view_id, clip_content);
                }
            }),
        );

        self.view_item.ev_scrolled_window.grab_focus();
    }

    pub fn do_paste_primary(&self, line: u64, col: u64) {
        debug!("Pasting primary text");

        Clipboard::get(&SELECTION_PRIMARY).request_text(
            clone!(@strong self.core as core, @strong self.view_id as view_id => move |_, text| {
                core.gesture_point_select(view_id, line, col);
                if let Some(clip_content) = text {
                    core.insert(view_id, clip_content);
                }
            }),
        );
    }

    /// Resize the EditView
    pub fn do_resize(&self, width: i32, height: i32) {
        trace!("Resizing EditView '{}'", self.view_id);

        self.core.resize(self.view_id, width, height);
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
            self.find_replace.popover.popup();
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
                self.core.find(self.view_id, &needle, false, false, false);
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
                self.find_replace.replace_entry.grab_focus();
            }
        } else {
            self.find_replace.show_replace_button.set_active(true);
            self.find_replace.search_bar.set_search_mode(true);
            self.find_replace.popover.popup();
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

    pub(crate) fn show_findreplace_opts(&self) {
        self.find_replace.option_revealer.set_reveal_child(true);
    }

    pub(crate) fn hide_findreplace_opts(&self) {
        self.find_replace.option_revealer.set_reveal_child(false);
    }

    pub fn stop_replace(&self) {
        self.find_replace.show_replace_button.set_active(false);
        self.hide_replace();
        self.stop_search();
    }

    /// Closes the find/replace dialog
    pub fn stop_search(&self) {
        self.find_replace.popover.popdown();
        self.find_replace.show_replace_button.set_active(false);
        self.find_replace.show_options_button.set_active(false);
        self.find_replace.search_bar.set_search_mode(false);
        self.view_item.ev_scrolled_window.grab_focus();
    }

    /// Displays how many matches have been found in the find/replace dialog.
    pub fn find_status(&self, queries: &[Query]) {
        for query in queries {
            self.find_replace
                .find_status_label
                .set_text(&format!("{} Results", query.matches));
            debug!("query {:?}", query);
        }
    }

    /// Displays what chars will be replaced in the replace dialog
    //TODO: Handle preserve_case
    pub fn replace_status(&self, status: &Status) {
        self.find_replace.replace_entry.set_text(&status.chars);
    }

    /// Go to the next match in the find/replace dialog
    ///
    /// #Returns
    ///
    /// Returns `true` if search mode is activated, `false` otherwise
    pub fn find_next(&self) -> bool {
        if self.find_replace.search_bar.get_search_mode() {
            self.core.find_next(
                self.view_id,
                Some(true),
                Some(true),
                Some(tau_rpc::ModifySelection::Set),
            );
            self.do_copy_primary();
            true
        } else {
            false
        }
    }

    /// Go the to previous match in the find/replace dialog
    ///
    /// #Returns
    ///
    /// Returns `true` if search mode is activated, `false` otherwise
    pub fn find_prev(&self) -> bool {
        if self.find_replace.search_bar.get_search_mode() {
            self.core.find_previous(
                self.view_id,
                Some(true),
                Some(true),
                Some(tau_rpc::ModifySelection::Set),
            );
            self.do_copy_primary();
            true
        } else {
            false
        }
    }

    /// Select all matches of the search
    ///
    /// #Returns
    ///
    /// Returns `true` if search mode is activated, `false` otherwise
    pub fn find_all(&self) -> bool {
        if self.find_replace.search_bar.get_search_mode() {
            self.core.find_all(self.view_id);
            self.do_copy_primary();
            return true;
        }
        false
    }

    /// Tells xi-editor that we're searching for a different string (or none) now
    pub fn search_changed(&self, s: Option<String>) {
        let needle = s.unwrap_or_default();
        let regex = self.find_replace.use_regex_button.get_active();
        let whole_worlds = self.find_replace.whole_word_button.get_active();
        let case_sensitive = self.find_replace.case_sensitive_button.get_active();
        self.core
            .find(self.view_id, &needle, case_sensitive, regex, whole_worlds);
    }

    /// Replace _one_ match with the replacement string
    pub fn replace(&self) {
        if let Some(replace_chars) = self.find_replace.replace_entry.get_text() {
            self.core
                .replace(self.view_id, replace_chars.as_str(), false);
            self.core.replace_next(self.view_id);
        }
    }

    /// Replace _all_ matches with the replacement string
    pub fn replace_all(&self) {
        if let Some(replace_chars) = self.find_replace.replace_entry.get_text() {
            self.core
                .replace(self.view_id, replace_chars.as_str(), false);
            self.core.replace_all(self.view_id);
        }
    }

    /// Returns true if this EditView is empty (contains no text)
    pub fn is_empty(&self) -> bool {
        self.line_cache.lock().is_empty()
    }

    pub fn set_language(&self, lang: &str) {
        debug!("Changing language to '{:?}'", lang);
        self.core.set_language(self.view_id, lang);
    }

    pub fn language_changed(&self, syntax: &str) {
        debug!("Language has been changed to '{:?}'", syntax);
        // https://github.com/xi-editor/xi-editor/issues/1194
        let lang = if syntax == "" || syntax == "Plain Text" {
            gettext("Plain Text")
        } else {
            syntax.to_string()
        };
        let syntax_treeview = &self.view_item.statusbar.syntax_treeview;
        let lang_pos = self
            .main_state
            .borrow()
            .avail_languages
            .iter()
            .position(|s| s == &lang);
        if let Some(pos) = lang_pos {
            syntax_treeview
                .get_selection()
                .select_path(&TreePath::new_from_string(&format!("{}", pos)));
        } else {
            warn!(
                "Couldn't determine what position the following language is in: '{}'",
                lang
            )
        }
    }

    pub fn set_syntax_selection_sensitivity(&self, state: bool) {
        self.view_item
            .statusbar
            .syntax_menu_button
            .set_sensitive(state);
    }

    pub fn set_default_tab_size(&self, size: u32) {
        self.default_tab_size.replace(size);
        self.view_item
            .statusbar
            .tab_size_button
            .set_value(f64::from(size));
        // We only need to redraw if the `default_tab_size` is actually in use right now.
        if self.tab_size.borrow().is_none() {
            self.view_item.edit_area.queue_draw();
        }
    }

    pub fn start_go_to_line(&self) {
        if self.view_item.go_to_line.search_bar.get_search_mode() {
            self.stop_go_to_line();
        } else {
            self.view_item.go_to_line.search_bar.set_search_mode(true);
            self.view_item.go_to_line.popover.popup();
            self.view_item.go_to_line.search_entry.grab_focus();
        }
    }

    pub fn stop_go_to_line(&self) {
        self.view_item.go_to_line.popover.popdown();
        self.view_item.go_to_line.search_bar.set_search_mode(false);
        self.view_item.ev_scrolled_window.grab_focus();
    }

    pub fn go_to_line(&self, line: u64) {
        self.core.goto_line(self.view_id, line - 1);
    }
}

// An extension trait for `EditViewExt, used when we need a Rc<EditView> for things like callbacks
pub trait EditViewExt {
    fn multicursor_select_all(&self);
}

impl EditViewExt for Rc<EditView> {
    /// Select all occurences of what's currently selected
    fn multicursor_select_all(&self) {
        let (clipboard_tx, clipboard_rx) =
            MainContext::sync_channel::<serde_json::value::Value>(PRIORITY_HIGH, 1);

        clipboard_rx.attach(
            None,
            clone!(@weak self as edit_view => @default-panic, move |val| {
                edit_view.in_multicursor_edit.set(true);
                edit_view.core.find(
                    edit_view.view_id,
                    &val.as_str().unwrap_or_default(),
                    false,
                    false,
                    false,
                );
                edit_view.core.find_all(edit_view.view_id);
                edit_view.do_copy_primary();

                Continue(false)
            }),
        );

        self.core.copy(self.view_id, move |res| {
            if let Ok(val) = res {
                clipboard_tx.send(val).unwrap();
            }
        });
    }
}
