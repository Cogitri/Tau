use editview::main_state::ShowInvisibles;
use editview::Settings;
use gio::prelude::*;
use log::error;
use serde_json::json;
use std::cmp::max;
use xrl::Client;

pub fn get_font_properties(font: &str) -> Option<(String, f32)> {
    let font_vec = font.split_whitespace().collect::<Vec<_>>();
    font_vec.split_last().map(|(size, name)| {
        let font_name = name.join(" ");
        let font_size = size.parse::<f32>().unwrap();
        (font_name, font_size)
    })
}

/// Generate a new `Settings` object, which we pass to the `EditView` to set its behaviour.
pub fn new_settings() -> editview::Settings {
    let gschema = gio::Settings::new("org.gnome.Tau");
    let interface_font = {
        use gtk::SettingsExt;
        let gtk_settings = gtk::Settings::get_default().unwrap();
        gtk_settings
            .get_property_gtk_font_name()
            .unwrap()
            .to_string()
    };

    Settings {
        draw_spaces: {
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
        },
        draw_tabs: {
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
        },
        highlight_line: gschema.get("highlight-line"),
        right_margin: gschema.get("draw-right-margin"),
        column_right_margin: gschema.get("column-right-margin"),
        edit_font: gschema.get("font"),
        draw_cursor: gschema.get("draw-cursor"),
        show_linecount: gschema.get("show-linecount"),
        restore_session: gschema.get("restore-session"),
        full_title: gschema.get("full-title"),
        interface_font,
        gschema,
    }
}

/// Send the current config to xi-editor during startup
pub fn setup_config(core: &Client) {
    #[cfg(windows)]
    const LINE_ENDING: &str = "\r\n";
    #[cfg(not(windows))]
    const LINE_ENDING: &str = "\n";

    let gschema = gio::Settings::new("org.gnome.Tau");

    let tab_size = gschema.get::<u32>("tab-size");
    let autodetect_whitespace = gschema.get::<bool>("auto-indent");
    let translate_tabs_to_spaces = gschema.get::<bool>("translate-tabs-to-spaces");
    let use_tab_stops = gschema.get::<bool>("use-tab-stops");
    let word_wrap = gschema.get::<bool>("word-wrap");

    let font = gschema.get::<String>("font");
    let font_vec = font.split_whitespace().collect::<Vec<_>>();
    let (font_size, font_name) = if let Some((size, splitted_name)) = font_vec.split_last() {
        (size.parse::<f32>().unwrap_or(14.0), splitted_name.join(" "))
    } else {
        error!("Failed to get font configuration. Resetting...");
        gschema.reset("font");
        (14.0, "Monospace".to_string())
    };

    tokio::executor::current_thread::block_on_all(core.modify_user_config(
        "general",
        json!({
            "tab_size": max(1, tab_size),
            "autodetect_whitespace": autodetect_whitespace,
            "translate_tabs_to_spaces": translate_tabs_to_spaces,
            "font_face": font_name,
            "font_size": if font_size.is_nan() {
                14.0
            } else if font_size < 6.0 {
                6.0
            } else if font_size > 72.0 {
                72.0
            } else { font_size },
            "use_tab_stops": use_tab_stops,
            "word_wrap": word_wrap,
            "line_ending": LINE_ENDING,
        }),
    ))
    .unwrap();

    let val = gschema.get_strv("syntax-config");

    for x in val {
        if let Ok(val) = serde_json::from_str(x.as_str()) {
            tokio::executor::current_thread::block_on_all(core.notify("modify_user_config", val))
                .unwrap();
        } else {
            error!("Failed to deserialize syntax config. Resetting...");
            gschema.reset("syntax-config");
        }
    }
}
