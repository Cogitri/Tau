use gxi_config_storage::GSchema;
use std::collections::HashMap;
use syntect::highlighting::ThemeSettings;

/// A Struct containing setting switches for the EditView
pub struct Settings {
    pub gschema: GSchema,
    pub trailing_spaces: bool,
    pub highlight_line: bool,
    pub right_margin: bool,
    pub column_right_margin: u32,
    pub interface_font: String,
    pub edit_font: String,
    pub tab_size: u32,
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
