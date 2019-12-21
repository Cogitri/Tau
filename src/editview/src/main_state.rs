use gschema_config_storage::GSchema;
use std::collections::HashMap;
use syntect::highlighting::ThemeSettings;

/// Options for drawing of invisibles, e.g. tabs, spaces
pub enum ShowInvisibles {
    None,
    All,
    Leading,
    Trailing,
    Selected,
}

/// A Struct containing setting switches for the `EditView`
pub struct Settings {
    pub gschema: GSchema,
    pub draw_spaces: ShowInvisibles,
    pub draw_tabs: ShowInvisibles,
    pub highlight_line: bool,
    pub right_margin: bool,
    pub column_right_margin: u32,
    pub interface_font: String,
    pub edit_font: String,
    pub draw_cursor: bool,
    pub show_linecount: bool,
    pub restore_session: bool,
}

pub struct MainState {
    pub themes: Vec<String>,
    pub theme_name: String,
    pub theme: ThemeSettings,
    pub styles: HashMap<usize, xrl::Style>,
    pub fonts: Vec<String>,
    pub avail_languages: Vec<String>,
    pub selected_language: String,
    pub settings: Settings,
}
