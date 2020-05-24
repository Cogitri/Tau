use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailableThemes {
    pub themes: Vec<String>,
}

pub type ThemeSettings = ::syntect::highlighting::ThemeSettings;

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeChanged {
    pub name: String,
    pub theme: ThemeSettings,
}
