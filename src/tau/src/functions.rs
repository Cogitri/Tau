use editview::Settings;
use gio::prelude::*;
use gschema_config_storage::{GSchema, GSchemaExt};
use log::error;
use serde_json::json;
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
pub fn new_settings() -> Settings {
    let gschema = GSchema::new("org.gnome.Tau");
    let interface_font = {
        use gtk::SettingsExt;
        let gtk_settings = gtk::Settings::get_default().unwrap();
        gtk_settings
            .get_property_gtk_font_name()
            .unwrap()
            .to_string()
    };

    Settings {
        trailing_spaces: gschema.get_key("draw-trailing-spaces"),
        all_spaces: gschema.get_key("draw-all-spaces"),
        leading_spaces: gschema.get_key("draw-leading-spaces"),
        selection_spaces: gschema.get_key("draw-selection-spaces"),
        highlight_line: gschema.get_key("highlight-line"),
        right_margin: gschema.get_key("draw-right-margin"),
        column_right_margin: gschema.get_key("column-right-margin"),
        edit_font: gschema.get_key("font"),
        trailing_tabs: gschema.get_key("draw-trailing-tabs"),
        all_tabs: gschema.get_key("draw-all-tabs"),
        leading_tabs: gschema.get_key("draw-leading-tabs"),
        selection_tabs: gschema.get_key("draw-selection-tabs"),
        draw_cursor: gschema.get_key("draw-cursor"),
        show_linecount: gschema.get_key("show-linecount"),
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

    let gschema = GSchema::new("org.gnome.Tau");

    let tab_size: u32 = gschema.get_key("tab-size");
    let autodetect_whitespace: bool = gschema.get_key("auto-indent");
    let translate_tabs_to_spaces: bool = gschema.get_key("translate-tabs-to-spaces");
    let use_tab_stops: bool = gschema.get_key("use-tab-stops");
    let word_wrap: bool = gschema.get_key("word-wrap");

    let font: String = gschema.get_key("font");
    let font_vec = font.split_whitespace().collect::<Vec<_>>();
    let (font_size, font_name) = if let Some((size, splitted_name)) = font_vec.split_last() {
        (size.parse::<f32>().unwrap_or(14.0), splitted_name.join(" "))
    } else {
        error!("Failed to get font configuration. Resetting...");
        gschema.settings.reset("font");
        (14.0, "Monospace".to_string())
    };

    tokio::executor::current_thread::block_on_all(core.modify_user_config(
        "general",
        json!({
            "tab_size": tab_size,
            "autodetect_whitespace": autodetect_whitespace,
            "translate_tabs_to_spaces": translate_tabs_to_spaces,
            "font_face": font_name,
            "font_size": font_size,
            "use_tab_stops": use_tab_stops,
            "word_wrap": word_wrap,
            "line_ending": LINE_ENDING,
        }),
    ))
    .unwrap();

    let val = gschema.settings.get_strv("syntax-config");

    for x in val {
        if let Ok(val) = serde_json::from_str(x.as_str()) {
            tokio::executor::current_thread::block_on_all(core.notify("modify_user_config", val))
                .unwrap();
        } else {
            error!("Failed to deserialize syntax config. Resetting...");
            gschema.settings.reset("syntax-config");
        }
    }
}
