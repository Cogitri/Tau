use crate::main_win::MainState;
use crate::pref_storage::*;
use crate::rpc::Core;
use gettextrs::gettext;
use gtk::*;
use log::{debug, error, trace};
use pango::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PrefsWin {
    core: Rc<RefCell<Core>>,
    window: Window,
}

impl PrefsWin {
    pub fn new(
        parent: &ApplicationWindow,
        main_state: &Rc<RefCell<MainState>>,
        core: &Rc<RefCell<Core>>,
    ) -> Rc<RefCell<Self>> {
        const SRC: &'static str = include_str!("ui/prefs_win.glade");
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

        let xi_config = &main_state.borrow().config;

        {
            let mut font_desc = FontDescription::new();
            let font_face = &xi_config.borrow().config.font_face;
            font_desc.set_size(xi_config.borrow().config.font_size as i32 * pango::SCALE);
            font_desc.set_family(font_face);

            trace!("{}: {}", gettext("Setting font description"), font_face);

            font_chooser_widget.set_font_desc(&font_desc);
        }

        {
            font_chooser_widget.connect_property_font_desc_notify(
                clone!(xi_config => move |font_widget| {
                    if let Some(font_desc) = font_widget.get_font_desc() {
                        let mut font_conf = xi_config.borrow_mut();

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
                    theme_combo_box.set_active(i as u32);
                }
            }
        }

        theme_combo_box.connect_changed(clone!(core, main_state => move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                debug!("{} {:?}", gettext("Theme changed to"), &theme_name);
                let core = core.borrow();
                core.set_theme(&theme_name);

                crate::pref_storage::set_theme_schema(&theme_name);

                let mut main_state = main_state.borrow_mut();
                main_state.theme_name = theme_name.to_string();
            }
        }));

        {
            {
                scroll_past_end_checkbutton.set_active(xi_config.borrow().config.scroll_past_end);
            }

            scroll_past_end_checkbutton.connect_toggled(clone!(xi_config => move |toggle_btn| {
                let value = toggle_btn.get_active();;
                debug!("{}: {}", gettext("Scrolling past end"), value);
                xi_config.borrow_mut().config.scroll_past_end = value;
                xi_config.borrow().save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
            }));
        }

        {
            {
                word_wrap_checkbutton.set_active(xi_config.borrow().config.word_wrap);
            }

            word_wrap_checkbutton.connect_toggled(clone!(xi_config => move |toggle_btn| {
                let value = toggle_btn.get_active();
                debug!("{}: {}", gettext("Word wrapping"), value);
                xi_config.borrow_mut().config.word_wrap = value;
                xi_config.borrow_mut().save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
            }));
        }

        {
            {
                tab_stops_checkbutton.set_active(xi_config.borrow().config.use_tab_stops);
            }

            tab_stops_checkbutton.connect_toggled(clone!(xi_config => move |toggle_btn| {
                let value = toggle_btn.get_active();
                debug!("{}: {}", gettext("Tab stops"), value);
                xi_config.borrow_mut().config.use_tab_stops = value;
                xi_config.borrow().save()
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
            }));
        }

        {
            draw_trailing_spaces_checkbutton.set_active(get_draw_trailing_spaces_schema());

            draw_trailing_spaces_checkbutton.connect_toggled(move |toggle_btn| {
                let value = toggle_btn.get_active();
                set_draw_trailing_spaces_schema(value);
            });
        }

        {
            margin_checkbutton.set_active(get_draw_right_margin());

            margin_checkbutton.connect_toggled(clone!(margin_spinbutton => move |toggle_btn| {
                let value = toggle_btn.get_active();
                set_draw_right_margin(value);
                margin_spinbutton.set_sensitive(value);
            }));
        }

        {
            margin_spinbutton.set_sensitive(get_draw_right_margin());
            margin_spinbutton.set_value(get_column_right_margin() as f64);

            margin_spinbutton.connect_value_changed(move |spin_btn| {
                set_column_right_margin(spin_btn.get_value() as u32)
            });
        }

        let prefs_win = Rc::new(RefCell::new(Self {
            core: core.clone(),
            window: window.clone(),
        }));

        window.set_transient_for(parent);
        window.show_all();

        prefs_win
    }
}
