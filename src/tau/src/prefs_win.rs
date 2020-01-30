use crate::main_win::StartedPlugins;
use crate::syntax_config::{Changes, Domain, SyntaxParams};
use editview::MainState;
use gettextrs::gettext;
use gio::prelude::*;
use gio::{Settings, SettingsBindFlags};
use glib::GString;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Builder, Button, ComboBoxText, FontChooserWidget, Image, Label, RadioButton,
    SpinButton, Switch, ToggleButton,
};
use log::{debug, error, trace};
use pango::FontDescription;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use xrl::Client;

#[cfg(not(feature = "libhandy"))]
use gtk::Window;
#[cfg(feature = "libhandy")]
use libhandy::PreferencesWindow;

const TAB_SIZE_DEFAULT: f64 = 4.0;
const INSERT_SPACES_DEFAULT: bool = false;

#[cfg(not(feature = "libhandy"))]
pub struct PrefsWin {
    pub core: Client,
    pub window: Window,
}

#[cfg(feature = "libhandy")]
pub struct PrefsWin {
    pub core: Client,
    pub window: PreferencesWindow,
}

impl PrefsWin {
    pub fn new(
        parent: &ApplicationWindow,
        main_state: &Rc<RefCell<MainState>>,
        core: &Client,
        gschema: &Settings,
        current_syntax: Option<&str>,
        started_plugins: &StartedPlugins,
    ) -> Self {
        #[cfg(feature = "libhandy")]
        let builder = Builder::new_from_resource("/org/gnome/Tau/prefs_win_handy.glade");
        #[cfg(not(feature = "libhandy"))]
        let builder = Builder::new_from_resource("/org/gnome/Tau/prefs_win.glade");

        #[cfg(feature = "libhandy")]
        let window: PreferencesWindow = builder.get_object("prefs_win").unwrap();
        #[cfg(not(feature = "libhandy"))]
        let window: Window = builder.get_object("prefs_win").unwrap();

        let font_chooser_widget: FontChooserWidget =
            builder.get_object("font_chooser_widget").unwrap();
        let theme_combo_box: ComboBoxText = builder.get_object("theme_combo_box").unwrap();
        let tab_stops_switch: Switch = builder.get_object("tab_stops_switch").unwrap();
        let word_wrap_switch: Switch = builder.get_object("word_wrap_switch").unwrap();
        let margin_switch: Switch = builder.get_object("margin_switch").unwrap();
        let margin_spinbutton: SpinButton = builder.get_object("margin_spinbutton").unwrap();
        let highlight_line_switch: Switch = builder.get_object("highlight_line_switch").unwrap();
        let tab_size_spinbutton: SpinButton = builder.get_object("tab_size_spinbutton").unwrap();
        let auto_indention_switch: Switch = builder.get_object("auto_indention_switch").unwrap();
        let insert_spaces_switch: Switch = builder.get_object("insert_spaces_switch").unwrap();
        let save_when_out_of_focus_switch: Switch =
            builder.get_object("save_when_out_of_focus_switch").unwrap();
        let show_lintcount_switch: Switch = builder.get_object("show_linecount_switch").unwrap();
        let full_title_switch: Switch = builder.get_object("full_title_switch").unwrap();
        let restore_session_switch: Switch = builder.get_object("restore_session_switch").unwrap();

        let draw_trailing_tabs_radio: RadioButton =
            builder.get_object("tabs_trailing_radio_button").unwrap();
        let draw_leading_tabs_radio: RadioButton =
            builder.get_object("tabs_leading_radio_button").unwrap();
        let draw_selection_tabs_radio: RadioButton =
            builder.get_object("tabs_selection_radio_button").unwrap();
        let draw_all_tabs_radio: RadioButton = builder.get_object("tabs_all_radio_button").unwrap();

        let draw_trailing_spaces_radio: RadioButton =
            builder.get_object("spaces_trailing_radio_button").unwrap();
        let draw_leading_spaces_radio: RadioButton =
            builder.get_object("spaces_leading_radio_button").unwrap();
        let draw_selection_spaces_radio: RadioButton =
            builder.get_object("spaces_selection_radio_button").unwrap();
        let draw_all_spaces_radio: RadioButton =
            builder.get_object("spaces_all_radio_button").unwrap();

        let syntect_warn_automatic_indention_image: Image = builder
            .get_object("syntect_warn_automatic_indention_image")
            .unwrap();
        let syntect_warn_insert_spaces_image: Image = builder
            .get_object("syntect_warn_insert_spaces_image")
            .unwrap();

        let syntax_config_combo_box: ComboBoxText =
            builder.get_object("syntax_config_combo_box").unwrap();
        let syntax_config_insert_spaces_checkbutton: ToggleButton = builder
            .get_object("syntax_config_insert_spaces_checkbutton")
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

        let syntax_changes = gschema.get_strv("syntax-config");
        let syntax_config: HashMap<String, SyntaxParams> = syntax_changes
            .iter()
            .map(GString::as_str)
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| error!("Failed to deserialize syntax config {}", e))
                    .unwrap()
            })
            .map(|sc: SyntaxParams| (sc.domain.syntax.clone(), sc))
            .collect();
        let syntax_config = Rc::new(RefCell::new(syntax_config));

        if !started_plugins.syntect {
            let gettext_msg = gettext("Couldn’t find the xi-syntect-plugin. As such these settings won’t work in the current session.");
            syntect_warn_insert_spaces_image.set_visible(true);
            syntect_warn_insert_spaces_image.set_tooltip_text(Some(&gettext_msg));

            syntect_warn_automatic_indention_image.set_visible(true);
            syntect_warn_automatic_indention_image.set_tooltip_text(Some(&gettext_msg));
        }

        let font_desc: &String = &gschema.get("font");
        font_chooser_widget.set_font_desc(&FontDescription::from_string(font_desc));
        font_chooser_widget.connect_property_font_desc_notify(
            enclose!((font_chooser_widget) move |_| {
                    match font_chooser_widget.get_font_size() / pango::SCALE {
                        size if size < 6 => {
                            if let Some(mut desc) = font_chooser_widget.get_font_desc() {
                                desc.set_size(6 * pango::SCALE);
                                font_chooser_widget.set_font_desc(&desc);
                            }
                        },
                        size if size > 72 => {
                            if let Some(mut desc) = font_chooser_widget.get_font_desc() {
                                desc.set_size(72 * pango::SCALE);
                                font_chooser_widget.set_font_desc(&desc);
                            }
                        },
                        _ => (),
                    }
                }
            ),
        );

        {
            let main_state = main_state.borrow();
            for (i, theme_name) in main_state.themes.iter().enumerate() {
                theme_combo_box.append_text(theme_name);
                if &main_state.theme_name == theme_name {
                    trace!("Setting active theme number {}; '{}'", i, theme_name);
                    theme_combo_box.set_active(Some(i as u32));
                }
            }

            // We can't select any syntaxes if there are none
            if main_state.avail_languages.is_empty() {
                syntax_config_tab_size_spinbutton.set_sensitive(false);
                syntax_config_insert_spaces_checkbutton.set_sensitive(false);
                syntax_config_insert_spaces_checkbutton.set_sensitive(false);
                syntax_config_tab_size_switch.set_sensitive(false);
                syntax_config_apply_button.set_sensitive(false);
                syntax_config_tab_size_label.set_sensitive(false);
            } else {
                for (i, lang) in main_state.avail_languages.iter().enumerate() {
                    syntax_config_combo_box.append_text(lang);
                    if current_syntax == Some(lang) {
                        trace!(
                            "Setting active syntax in config combo box number {}; '{}'",
                            i,
                            lang,
                        );
                        syntax_config_combo_box.set_active(Some(i as u32));
                        syntax_config_set_buttons(
                            lang,
                            &syntax_config.borrow(),
                            &syntax_config_insert_spaces_checkbutton,
                            &syntax_config_tab_size_spinbutton,
                        );
                    }
                }
            }
        }

        theme_combo_box.connect_changed(enclose!((core, main_state, gschema) move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                let theme_name = theme_name.to_string();
                debug!("Theme changed to '{}'", &theme_name);
                let _ =  core.set_theme(&theme_name);

                gschema.set("theme-name", &theme_name).unwrap();

                let mut main_state = main_state.borrow_mut();
                main_state.theme_name = theme_name;
            }
        }));

        margin_switch.connect_activate(enclose!((margin_spinbutton) move |toggle_btn| {
            let value = toggle_btn.get_active();
            margin_spinbutton.set_sensitive(value);
        }));

        margin_spinbutton.set_sensitive(gschema.get("draw-right-margin"));

        gschema.bind(
            "font",
            &font_chooser_widget,
            "font",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "word-wrap",
            &word_wrap_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "use-tab-stops",
            &tab_stops_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-trailing-spaces",
            &draw_trailing_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-leading-spaces",
            &draw_leading_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-selection-spaces",
            &draw_selection_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-all-spaces",
            &draw_all_spaces_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-right-margin",
            &margin_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "highlight-line",
            &highlight_line_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "column-right-margin",
            &margin_spinbutton,
            "value",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "tab-size",
            &tab_size_spinbutton,
            "value",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-trailing-tabs",
            &draw_trailing_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-leading-tabs",
            &draw_leading_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-selection-tabs",
            &draw_selection_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "draw-all-tabs",
            &draw_all_tabs_radio,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "auto-indent",
            &auto_indention_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "translate-tabs-to-spaces",
            &insert_spaces_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "save-when-out-of-focus",
            &save_when_out_of_focus_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "show-linecount",
            &show_lintcount_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "restore-session",
            &restore_session_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        gschema.bind(
            "full-title",
            &full_title_switch,
            "active",
            SettingsBindFlags::DEFAULT,
        );

        syntax_config_combo_box.connect_changed(enclose!((
            syntax_config_insert_spaces_checkbutton,
            syntax_config_tab_size_spinbutton,
            syntax_config,
            ) move |cb| {
                if let Some(lang) = cb.get_active_text() {
                    syntax_config_set_buttons(
                        lang.as_str(),
                        &syntax_config.borrow(),
                        &syntax_config_insert_spaces_checkbutton,
                        &syntax_config_tab_size_spinbutton,
                    );
                }
            }
        ));

        syntax_config_apply_button.connect_clicked(
            enclose!((
                syntax_config_combo_box,
                syntax_config_insert_spaces_checkbutton,
                syntax_config_insert_spaces_checkbutton,
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
                        let insert_spaces = if syntax_config_insert_spaces_checkbutton.get_active() {
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
                        gschema.set_strv("syntax-config", &json_setting).unwrap();
                    }
                }
            )
        );

        syntax_config_insert_spaces_checkbutton.connect_property_active_notify(enclose!(
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

        window.set_transient_for(Some(parent));
        window.show_all();

        Self {
            core: core.clone(),
            window,
        }
    }
}

fn syntax_config_set_buttons(
    lang: &str,
    syntax_config: &HashMap<String, SyntaxParams>,
    insert_spaces_switch: &ToggleButton,
    tab_size_spinbutton: &SpinButton,
) {
    if let Some(config) = syntax_config.get(lang) {
        // This is an Option, so set a default here
        let insert_spaces = if let Some(setting) = config.changes.translate_tabs_to_spaces {
            setting
        } else {
            INSERT_SPACES_DEFAULT
        };
        insert_spaces_switch.set_active(insert_spaces);

        let tab_size = if let Some(setting) = config.changes.tab_size {
            f64::from(setting)
        } else {
            TAB_SIZE_DEFAULT
        };
        tab_size_spinbutton.set_value(tab_size);
    }
}
