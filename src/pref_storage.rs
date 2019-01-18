use crate::errors::Error;
use log::{debug, trace};
use serde::{de::DeserializeOwned, Serialize};
use serde_derive::*;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::prelude::*;

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
    pub theme: String,
}

impl Default for GtkXiConfig {
    fn default() -> GtkXiConfig {
        GtkXiConfig {
            theme: "InspiredGitHub".to_string(),
        }
    }
}

/// For stuff that goes into preferences.xiconfig
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct XiConfig {
    pub tab_size: u32,
    pub translate_tabs_to_spaces: bool,
    pub use_tab_stops: bool,
    pub plugin_search_path: Vec<String>,
    pub font_face: String,
    pub font_size: u32,
    pub auto_indent: bool,
    pub scroll_past_end: bool,
    pub wrap_width: u32,
    pub word_wrap: bool,
    pub autodetect_whitespace: bool,
    pub line_ending: String,
}

impl Default for XiConfig {
    fn default() -> XiConfig {
        #[cfg(windows)]
        const LINE_ENDING: &str = "\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &str = "\n";

        // Default valuess as dictated by https://github.com/xi-editor/xi-editor/blob/master/rust/core-lib/assets/client_example.toml
        XiConfig {
            tab_size: 4,
            translate_tabs_to_spaces: false,
            use_tab_stops: true,
            plugin_search_path: vec![String::new()],
            font_face: "Inconsolata".to_string(),
            font_size: 12,
            auto_indent: true,
            scroll_past_end: false,
            wrap_width: 0,
            word_wrap: false,
            autodetect_whitespace: true,
            line_ending: LINE_ENDING.to_string(),
        }
    }
}

impl<T> Config<T> {
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

        config_file.sync_all()?;

        Ok(self)
    }

    pub fn save(&self) -> Result<(), Error>
    where
        T: Serialize,
    {
        let mut config_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.path)?;

        config_file.write_all(toml::to_string(&self.config)?.as_bytes())?;

        config_file.sync_all()?;

        Ok(())
    }
}
