use crate::syntax_config::*;
use editview::MainState;
use gettextrs::gettext;
use gio::{SettingsBindFlags, SettingsExt};
use gschema_config_storage::{GSchema, GSchemaExt};
use gtk::*;
use log::{debug, error, trace};
use pango::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use xrl::Client;

pub struct PrefsWin {
    pub core: Client,
    pub window: Window,
}

impl PrefsWin {
    pub fn new(
        parent: &ApplicationWindow,
        main_state: &Rc<RefCell<MainState>>,
        core: &Client,
        gschema: &GSchema,
        current_syntax: Option<&str>,
    ) -> Self {
        let builder = Builder::new_from_resource("/org/gnome/Tau/prefs_win.glade");

        let window: Window = builder.get_object("prefs_win").unwrap();
        let font_chooser_widget: FontChooserWidget =
            builder.get_object("font_chooser_widget").unwrap();
        let theme_combo_box: ComboBoxText = builder.get_object("theme_combo_box").unwrap();
        let tab_stops_checkbutton: ToggleButton =
            builder.get_object("tab_stops_checkbutton").unwrap();
        let word_wrap_checkbutton: ToggleButton =
            builder.get_object("word_wrap_checkbutton").unwrap();
        let margin_checkbutton: ToggleButton = builder.get_object("margin_checkbutton").unwrap();
        let margin_spinbutton: SpinButton = builder.get_object("margin_spinbutton").unwrap();
        let highlight_line_checkbutton: ToggleButton =
            builder.get_object("highlight_line_checkbutton").unwrap();
        let tab_size_spinbutton: SpinButton = builder.get_object("tab_size_spinbutton").unwrap();
        let auto_indention_checkbutton: ToggleButton =
            builder.get_object("auto_indention_checkbutton").unwrap();
        let insert_spaces_checkbutton: ToggleButton =
            builder.get_object("insert_spaces_checkbutton").unwrap();

        let draw_trailing_tabs_radio: RadioButton =
            builder.get_object("tabs_trailing_radio_button").unwrap();
        let draw_leading_tabs_radio: RadioButton =
            builder.get_object("tabs_leading_radio_button").unwrap();
        let draw_all_tabs_radio: RadioButton = builder.get_object("tabs_all_radio_button").unwrap();

        let draw_trailing_spaces_radio: RadioButton =
            builder.get_object("spaces_trailing_radio_button").unwrap();
        let draw_leading_spaces_radio: RadioButton =
            builder.get_object("spaces_leading_radio_button").unwrap();
        let draw_all_spaces_radio: RadioButton =
            builder.get_object("spaces_all_radio_button").unwrap();

        let syntax_config_combo_box: ComboBoxText =
            builder.get_object("syntax_config_combo_box").unwrap();
        let syntax_config_insert_spaces_checkbutton: CheckButton = builder
            .get_object("syntax_config_insert_spaces_checkbutton")
            .unwrap();
        let syntax_config_insert_spaces_switch: Switch = builder
            .get_object("syntax_config_insert_spaces_switch")
            .unwrap();
        let syntax_config_tab_size_switch: Switch =
            builder.get_object("syntax_config_tab_size_switch").unwrap();
        let syntax_config_tab_size_spinbutton: SpinButton = builder
            .get_object("syntax_config_tab_size_spinbutton")
            .unwrap();
        let syntax_config_apply_button: Button =
            builder.get_object("syntax_config_apply_button").unwrap();
        let syntax_config_tab_size_label: Label =
            builder.get_object("syntax_config_tab_size_label").unwrap();

        let font_desc: &String = &gschema.get_key("font");
        font_chooser_widget.set_font_desc(&FontDescription::from_string(font_desc));

        {
            let main_state = main_state.borrow();
            for (i, theme_name) in main_state.themes.iter().enumerate() {
                theme_combo_box.append_text(theme_name);
                if &main_state.theme_name == theme_name {
                    trace!("{}: {}", gettext("Setting active theme"), i);
                    theme_combo_box.set_active(Some(i as u32));
                }
            }

            // We can't select any syntaxes if there are none
            if main_state.avail_languages.is_empty() {
                syntax_config_tab_size_spinbutton.set_sensitive(false);
                syntax_config_insert_spaces_checkbutton.set_sensitive(false);
                syntax_config_insert_spaces_switch.set_sensitive(false);
                syntax_config_tab_size_switch.set_sensitive(false);
                syntax_config_apply_button.set_sensitive(false);
                syntax_config_tab_size_label.set_sensitive(false);
            } else {
                for (i, lang) in main_state.avail_languages.iter().enumerate() {
                    syntax_config_combo_box.append_text(lang);
                    if current_syntax == Some(lang) {
                        trace!(
                            "{}: {}",
                            gettext("Setting active syntax in config combo box"),
                            i
                        );
                        syntax_config_combo_box.set_active(Some(i as u32))
                    }
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

        margin_checkbutton.connect_toggled(enclose!((margin_spinbutton) move |toggle_btn| {
            let value = toggle_btn.get_active();
            margin_spinbutton.set_sensitive(value);
        }));

        margin_spinbutton.set_sensitive(gschema.get_key("draw-right-margin"));

        gschema.settings.bind(
            "font",
            &font_chooser_widget,
            "font",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "word-wrap",
            &word_wrap_checkbutton,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "use-tab-stops",
            &tab_stops_checkbutton,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-trailing-spaces",
            &draw_trailing_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-leading-spaces",
            &draw_leading_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-all-spaces",
            &draw_all_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-right-margin",
            &margin_checkbutton,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "highlight-line",
            &highlight_line_checkbutton,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "column-right-margin",
            &margin_spinbutton,
            "value",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "tab-size",
            &tab_size_spinbutton,
            "value",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-trailing-tabs",
            &draw_trailing_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-leading-tabs",
            &draw_leading_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "draw-all-tabs",
            &draw_all_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "auto-indent",
            &auto_indention_checkbutton,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.settings.bind(
            "translate-tabs-to-spaces",
            &insert_spaces_checkbutton,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        let syntax_changes = gschema.settings.get_strv("syntax-config");
        let syntax_config: HashMap<String, SyntaxParams> = syntax_changes
            .iter()
            .map(|s| s.as_str())
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| error!("{} {}", gettext("Failed to deserialize syntax config"), e))
                    .unwrap()
            })
            .map(|sc: SyntaxParams| (sc.domain.syntax.clone(), sc))
            .collect();
        let syntax_config = Rc::new(RefCell::new(syntax_config));

        syntax_config_combo_box.connect_changed(enclose!((syntax_config_insert_spaces_checkbutton, syntax_config_tab_size_spinbutton, syntax_config) move |cb| {
            if let Some(lang) = cb.get_active_text() {
                if let Some(config) = syntax_config.borrow().get(lang.as_str()) {
                    // This is an Option, so set a default here
                    let insert_spaces = if let Some(setting) = config.changes.translate_tabs_to_spaces {
                        setting
                    } else {
                        false
                    };
                    syntax_config_insert_spaces_checkbutton.set_active(insert_spaces);

                    let tab_size = if let Some(setting) = config.changes.tab_size {
                        f64::from(setting)
                    } else {
                        4.0
                    };
                    syntax_config_tab_size_spinbutton.set_value(tab_size);
                }
            }
        }));

        syntax_config_apply_button.connect_clicked(
            enclose!((
                syntax_config_combo_box,
                syntax_config_insert_spaces_checkbutton,
                syntax_config_insert_spaces_switch,
                syntax_config_tab_size_switch,
                syntax_config_tab_size_spinbutton,
                syntax_config,
                gschema,
                ) move |_| {
                    if let Some(lang) = syntax_config_combo_box.get_active_text() {
                        let tab_size = if syntax_config_tab_size_switch.get_active() {
                            Some(syntax_config_tab_size_spinbutton.get_value_as_int() as u32)
                        } else {
                            None
                        };
                        let insert_spaces = if syntax_config_insert_spaces_switch.get_active() {
                            Some(syntax_config_insert_spaces_checkbutton.get_active())
                        } else {
                            None
                        };

                        let mut syntax_config = syntax_config.borrow_mut();
                        if tab_size.is_none() && insert_spaces.is_none() {
                            syntax_config.remove(lang.as_str());
                        } else if let Some(config) = syntax_config.get_mut(lang.as_str()) {
                            config.changes.translate_tabs_to_spaces = insert_spaces;
                            config.changes.tab_size = tab_size;
                        } else {
                            let params = SyntaxParams {
                                domain: Domain {
                                    syntax: lang.to_string(),
                                },
                                changes: Changes {
                                    tab_size,
                                    translate_tabs_to_spaces: insert_spaces
                                },
                            };
                            syntax_config.insert(lang.to_string(), params);
                        }

                        let json_setting: Vec<String> = syntax_config.iter().map(|(_, sc)| serde_json::to_string(sc).unwrap()).collect();
                        let json_setting: Vec<_> = json_setting.iter().map(AsRef::as_ref).collect();
                        gschema.settings.set_strv("syntax-config", &json_setting);
                    }
                }
            )
        );

        syntax_config_insert_spaces_switch.connect_property_active_notify(enclose!(
            (syntax_config_insert_spaces_checkbutton) move | sw | {
                syntax_config_insert_spaces_checkbutton.set_sensitive(sw.get_active());
            }
        ));

        syntax_config_tab_size_switch.connect_property_active_notify(enclose!(
            (
                syntax_config_tab_size_label,
                syntax_config_tab_size_spinbutton
            ) move | sw | {
                    let active = sw.get_active();
                    syntax_config_tab_size_label.set_sensitive(active);
                    syntax_config_tab_size_spinbutton.set_sensitive(active);
                }
        ));

        let prefs_win = Self {
            core: core.clone(),
            window: window.clone(),
        };

        window.set_transient_for(Some(parent));
        window.show_all();

        prefs_win
    }
}
