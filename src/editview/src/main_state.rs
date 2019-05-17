use crate::theme::LineStyle;
use gxi_config_storage::Config;
use std::collections::HashMap;
use syntect::highlighting::ThemeSettings;

#[derive(Default)]
pub struct MainState {
    pub themes: Vec<String>,
    pub theme_name: String,
    pub theme: ThemeSettings,
    pub styles: HashMap<usize, LineStyle>,
    pub fonts: Vec<String>,
    pub avail_languages: Vec<String>,
    pub selected_language: String,
    pub config: Config,
}
