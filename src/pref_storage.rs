use crate::errors::Error;
use log::{debug, trace};
use serde::{Serialize, de::DeserializeOwned};
use serde_derive::*;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::prelude::*;
use toml::Value;

/// Generic wrapper struct around GtkXiConfig and XiConfig
#[derive(Clone, Debug)]
pub struct Config<T> {
    pub path: String,
    pub config: T,
}

/// For stuff that _doesn't_ go into preferences.xiconfig and has to be set by us
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct GtkXiConfig {
    pub theme: Value,
}

impl Default for GtkXiConfig {
    fn default() -> GtkXiConfig {
        GtkXiConfig {
            theme: Value::String("InspiredGitHub".to_string()),
        }
    }
}

/// For stuff that goes into preferences.xiconfig
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
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
    pub line_ending: Value,
}

impl Default for XiConfig {
    fn default() -> XiConfig {
        #[cfg(windows)]
        const LINE_ENDING: &str = "\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &str = "\n";

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
            line_ending: Value::String(LINE_ENDING.to_string()),
        }
    }
}

impl <T> Config<T> {
    pub fn new(path: String) -> Config<T>
    where
        T: Default,
    {
        Config {
            config: T::default(),
            path,
        }
    }

    pub fn open(&mut self) -> Result<&mut Config<T>, Error>
        where
            T: Clone + Debug + DeserializeOwned,
    {
        trace!("Opening config file!");
        let mut config_file = OpenOptions::new().read(true).open(&self.path)?;
        let mut config_string = String::new();

        trace!("Reading config file!");
        config_file.read_to_string(&mut config_string)?;

        let config_toml: T = toml::from_str(&config_string)?;
        debug!("XI-Config: {:?}", config_toml);

        self.config = config_toml.clone();

        Ok(self)
    }

    pub fn save(&self) -> Result<(), Error>
        where T:
            Serialize,
    {
        let mut config_file = OpenOptions::new().write(true).create(true).open(&self.path)?;

        config_file.write_all(toml::to_string(&self.config)?.as_bytes())?;

        Ok(())
    }
}