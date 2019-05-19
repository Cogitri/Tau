use editview::EditView;
use editview::MainState;
use gettextrs::gettext;
use gtk::*;
use gxi_config_storage::{GSchema, GSchemaExt};
use gxi_peer::Core;
use log::{debug, error, trace};
use pango::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PrefsWin {
    pub core: Core,
    pub window: Window,
}

impl PrefsWin {
    pub fn new(
        parent: &ApplicationWindow,
        main_state: &Rc<RefCell<MainState>>,
        core: &Core,
        edit_view: Option<Rc<RefCell<EditView>>>,
        gschema: &GSchema,
    ) -> Self {
        const SRC: &str = include_str!("ui/prefs_win.glade");
        let builder = Builder::new_from_string(SRC);

        let window: Window = builder.get_object("prefs_win").unwrap();
        let font_chooser_widget: FontChooserWidget =
            builder.get_object("font_chooser_widget").unwrap();
        let theme_combo_box: ComboBoxText = builder.get_object("theme_combo_box").unwrap();
        let tab_stops_checkbutton: ToggleButton =
            builder.get_object("tab_stops_checkbutton").unwrap();
        let scroll_past_end_checkbutton: ToggleButton =
            builder.get_object("scroll_past_end_checkbutton").unwrap();
        let word_wrap_checkbutton: ToggleButton =
            builder.get_object("word_wrap_checkbutton").unwrap();
        let draw_trailing_spaces_checkbutton: ToggleButton = builder
            .get_object("draw_trailing_spaces_checkbutton")
            .unwrap();
        let margin_checkbutton: ToggleButton = builder.get_object("margin_checkbutton").unwrap();
        let margin_spinbutton: SpinButton = builder.get_object("margin_spinbutton").unwrap();
        let highlight_line_checkbutton: ToggleButton =
            builder.get_object("highlight_line_checkbutton").unwrap();
        let tab_size_spinbutton: SpinButton = builder.get_object("tab_size_spinbutton").unwrap();

        let xi_config = &main_state.borrow().config;

        {
            let mut font_desc = FontDescription::new();
            let font_face = &xi_config.config.font_face;
            font_desc.set_size(xi_config.config.font_size as i32 * pango::SCALE);
            font_desc.set_family(font_face);

            trace!("{}: {}", gettext("Setting font description"), font_face);

            font_chooser_widget.set_font_desc(&font_desc);
        }

        {
            font_chooser_widget.connect_property_font_desc_notify(
                enclose!((main_state) move |font_widget| {
                    if let Some(font_desc) = font_widget.get_font_desc() {
                        let mut font_conf = &mut main_state.borrow_mut().config;

                        let font_family = font_desc.get_family().unwrap();
                        let font_size = font_desc.get_size() / pango::SCALE;
                        debug!("{} {}", gettext("Setting font to"), &font_family);
                        debug!("{} {}", gettext("Setting font size to"), &font_size);

                        font_conf.config.font_size = font_size as u32;
                        font_conf.config.font_face = font_family.to_string();
                        font_conf
                            .save()
                            .map_err(|e| error!("{}", e.to_string()))
                            .unwrap();
                    }
                }),
            );
        }

        {
            let main_state = main_state.borrow();
            for (i, theme_name) in main_state.themes.iter().enumerate() {
                theme_combo_box.append_text(theme_name);
                if &main_state.theme_name == theme_name {
                    trace!("{}: {}", gettext("Setting active theme"), i);
                    theme_combo_box.set_active(Some(i as u32));
                }
            }
        }

        theme_combo_box.connect_changed(enclose!((core, main_state, gschema) move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                let theme_name = theme_name.to_string();
                debug!("{} {}", gettext("Theme changed to"), &theme_name);
                core.set_theme(&theme_name);

                gschema.set_key("theme-name", theme_name.clone()).unwrap();

                let mut main_state = main_state.borrow_mut();
                main_state.theme_name = theme_name;
            }
        }));

        {
            {
                scroll_past_end_checkbutton
                    .set_active(main_state.borrow().config.config.scroll_past_end);
            }

            scroll_past_end_checkbutton.connect_toggled(enclose!((main_state) move |toggle_btn| {
                let value = toggle_btn.get_active();;
                debug!("{}: {}", gettext("Scrolling past end"), value);
                main_state.borrow_mut().config.config.scroll_past_end = value;
                main_state.borrow().config.save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
            }));
        }

        {
            {
                word_wrap_checkbutton.set_active(main_state.borrow().config.config.word_wrap);
            }

            word_wrap_checkbutton.connect_toggled(enclose!((main_state) move |toggle_btn| {
                let value = toggle_btn.get_active();
                debug!("{}: {}", gettext("Word wrapping"), value);
                main_state.borrow_mut().config.config.word_wrap = value;
                main_state.borrow().config.save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
            }));
        }

        {
            {
                tab_stops_checkbutton.set_active(main_state.borrow().config.config.use_tab_stops);
            }

            tab_stops_checkbutton.connect_toggled(enclose!((main_state) move |toggle_btn| {
                let value = toggle_btn.get_active();
                debug!("{}: {}", gettext("Tab stops"), value);
                main_state.borrow_mut().config.config.use_tab_stops = value;
                main_state.borrow().config.save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
            }));
        }

        {
            draw_trailing_spaces_checkbutton.set_active(gschema.get_key("draw-trailing-spaces"));

            draw_trailing_spaces_checkbutton.connect_toggled(
                enclose!((gschema) move |toggle_btn| {
                    let value = toggle_btn.get_active();
                    gschema.set_key("draw-trailing-spaces", value).unwrap();
                }),
            );
        }

        {
            margin_checkbutton.set_active(gschema.get_key("draw-right-margin"));

            margin_checkbutton.connect_toggled(
                enclose!((edit_view, margin_spinbutton, gschema) move |toggle_btn| {
                    let value = toggle_btn.get_active();
                    debug!("{}: {}", gettext("Right hand margin"), value);
                    gschema.set_key("draw-right-margin", value).unwrap();
                    if let Some(ev) = edit_view.clone() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                    margin_spinbutton.set_sensitive(value);
                }),
            );
        }

        {
            margin_spinbutton.set_sensitive(gschema.get_key("draw-right-margin"));
            let margin_value: u32 = gschema.get_key("column-right-margin");
            margin_spinbutton.set_value(f64::from(margin_value));

            margin_spinbutton.connect_value_changed(
                enclose!((edit_view, gschema) move |spin_btn| {
                    let value = spin_btn.get_value() as u32;
                    debug!("{}: {}", gettext("Right hand margin width"), value);
                    gschema.set_key("column-right-margin", value).unwrap();
                    if let Some(ev) = edit_view.clone() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }),
            );
        }

        {
            tab_size_spinbutton.set_value(f64::from(main_state.borrow().config.config.tab_size));

            tab_size_spinbutton.connect_value_changed(
                enclose!((main_state, edit_view) move |spin_btn| {
                    let value = spin_btn.get_value();
                    debug!("{}: {}", gettext("Setting tab size"), value);
                    main_state.borrow_mut().config.config.tab_size = value as u32;
                    main_state.borrow().config.save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
                    if let Some(ev) = edit_view.clone() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }),
            );
        }

        {
            highlight_line_checkbutton.set_active(gschema.get_key("highlight-line"));

            highlight_line_checkbutton.connect_toggled(
                enclose!((edit_view, gschema) move |toggle_btn| {
                    let value = toggle_btn.get_active();
                    gschema.set_key("highlight-line", value).unwrap();
                    if let Some(ev) = edit_view.clone() {
                        ev.borrow().view_item.edit_area.queue_draw();
                    }
                }),
            );
        }

        let prefs_win = Self {
            core: core.clone(),
            window: window.clone(),
        };

        window.set_transient_for(Some(parent));
        window.show_all();

        prefs_win
    }
}
