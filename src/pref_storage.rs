use crate::errors::Error;
use serde_derive::*;
use std::fs::OpenOptions;
use std::io::prelude::*;
use toml::Value;

// For stuff that goes into preferences.xiconfig
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct XiConfig {
    pub tab_size: Value,
    pub translate_tabs_to_spaces: Value,
    pub use_tab_stops: Value,
    pub plugin_search_path: Value,
    pub font_face: Value,
    pub font_size: Value,
    pub auto_indent: Value,
    pub scroll_past_end: Value,
    pub wrap_width: Value,
    pub word_wrap: Value,
    pub autodetect_whitespace: Value,
}

impl XiConfig {
    pub fn new() -> XiConfig {
        // Default valuess as dictated by https://github.com/xi-editor/xi-editor/blob/master/rust/core-lib/assets/client_example.toml
        XiConfig {
            tab_size: Value::Integer(4),
            translate_tabs_to_spaces: Value::Boolean(false),
            use_tab_stops: Value::Boolean(true),
            plugin_search_path: Value::String("".to_string()),
            font_face: Value::String("Inconsolata".to_string()),
            font_size: Value::Integer(12),
            auto_indent: Value::Boolean(true),
            scroll_past_end: Value::Boolean(false),
            wrap_width: Value::Integer(0),
            word_wrap: Value::Boolean(false),
            autodetect_whitespace: Value::Boolean(true),
        }
    }

    pub fn open(&mut self, path: &str) -> Result<XiConfig, Error> {
        let mut config_file = OpenOptions::new().read(true).open(path)?;
        let mut config_string = String::new();

        config_file.read_to_string(&mut config_string)?;

        let config_toml: XiConfig = toml::from_str(&config_string)?;

        Ok(config_toml)
    }

    pub fn save(&self, path: &str) -> Result<(), Error> {
        let mut config_file = OpenOptions::new().write(true).create(true).open(path)?;

        config_file.write_all(toml::to_string(self)?.as_bytes())?;

        Ok(())
    }
}

// For stuff that _doesn't_ go into preferences.xiconfig and has to be set by us
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GtkXiConfig {
    pub theme: Value,
}

impl GtkXiConfig {
    pub fn new() -> GtkXiConfig {
        GtkXiConfig {
            theme: Value::String("InspiredGitHub".to_string()),
        }
    }

    pub fn open(&mut self, path: &str) -> Result<GtkXiConfig, Error> {
        let mut config_file = OpenOptions::new().read(true).open(path)?;
        let mut config_string = String::new();

        config_file.read_to_string(&mut config_string)?;

        let config_toml: GtkXiConfig = toml::from_str(&config_string)?;

        Ok(config_toml)
    }

    pub fn save(&self, path: &str) -> Result<(), Error> {
        let mut config_file = OpenOptions::new().write(true).create(true).open(path)?;

        config_file.write_all(toml::to_string(self)?.as_bytes())?;

        Ok(())
    }
}
