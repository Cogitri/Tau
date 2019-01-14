use crate::errors::Error;
use serde_derive::*;
use std::fs::OpenOptions;
use std::io::prelude::*;
use toml::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigToml {
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

impl ConfigToml {
    pub fn new() -> ConfigToml {
        // Default valuess as dictated by https://github.com/xi-editor/xi-editor/blob/master/rust/core-lib/assets/client_example.toml
        ConfigToml {
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

    pub fn open(&mut self, path: &str) -> Result<ConfigToml, Error> {
        let mut config_file = OpenOptions::new().read(true).open(path)?;
        let mut config_string = String::new();

        config_file.read_to_string(&mut config_string)?;

        let config_toml: ConfigToml = toml::from_str(&config_string)?;

        Ok(config_toml)
    }

    pub fn save(&self, path: &str) -> Result<(), Error> {
        let mut config_file = OpenOptions::new().write(true).create(true).open(path)?;

        config_file.write_all(toml::to_string(self)?.as_bytes())?;

        Ok(())
    }
}
