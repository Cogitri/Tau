use crate::errors::Error;
use gettextrs::gettext;
use gio::{Settings, SettingsExt, SettingsSchemaSource};
use log::{debug, error, trace, warn};
use serde_derive::*;
use std::fs::OpenOptions;
use std::io::prelude::*;
use tempfile::tempdir;

/// Wrapper struct around `XiConfig`, it's annoying to pass around path otherwise
#[derive(Debug)]
pub struct Config {
    pub path: String,
    pub config: XiConfig,
}

/// For stuff that goes into preferences.xiconfig
#[derive(Debug, Deserialize, Serialize)]
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
    pub surrounding_pairs: Vec<Vec<String>>,
    pub save_with_newline: bool,
}

impl Default for XiConfig {
    fn default() -> Self {
        #[cfg(windows)]
        const LINE_ENDING: &str = "\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &str = "\n";

        let surrounding_pairs = vec![
            vec!["\"".to_string(), "\"".to_string()],
            vec!["'".to_string(), "'".to_string()],
            vec!["{".to_string(), "}".to_string()],
            vec!["[".to_string(), "]".to_string()],
        ];

        // Default valuess as dictated by https://github.com/xi-editor/xi-editor/blob/master/rust/core-lib/assets/client_example.toml
        Self {
            tab_size: 4,
            translate_tabs_to_spaces: false,
            use_tab_stops: true,
            plugin_search_path: vec![String::new()],
            font_face: get_default_monospace_font_schema(),
            font_size: 12,
            auto_indent: true,
            scroll_past_end: false,
            wrap_width: 0,
            word_wrap: false,
            autodetect_whitespace: true,
            line_ending: LINE_ENDING.to_string(),
            surrounding_pairs,
            save_with_newline: true,
        }
    }
}

impl Config {
    pub fn new() -> (String, Self) {
        if let Some(user_config_dir) = dirs::config_dir() {
            let config_dir = user_config_dir.join("gxi");
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| {
                    error!(
                        "{}: {}",
                        gettext("Failed to create the config dir"),
                        e.to_string()
                    )
                })
                .unwrap();

            let mut xi_config = Self {
                config: XiConfig::default(),
                path: config_dir
                    .join("preferences.xiconfig")
                    .to_str()
                    .map(std::string::ToString::to_string)
                    .unwrap(),
            };

            xi_config = if let Ok(xi_config) = xi_config.open() {
                /*
                We have to immediately save the config file here to "upgrade" it (as in add missing
                entries which have been added by us during a version upgrade). This works because
                the above call to Config::new() sets defaults.
                */
                xi_config
                    .save()
                    .unwrap_or_else(|e| error!("{}", e.to_string()));

                Self {
                    path: xi_config.path.to_string(),
                    config: XiConfig {
                        tab_size: xi_config.config.tab_size,
                        translate_tabs_to_spaces: xi_config.config.translate_tabs_to_spaces,
                        use_tab_stops: xi_config.config.use_tab_stops,
                        plugin_search_path: xi_config.config.plugin_search_path.clone(),
                        font_face: xi_config.config.font_face.to_string(),
                        font_size: xi_config.config.font_size,
                        auto_indent: xi_config.config.auto_indent,
                        scroll_past_end: xi_config.config.scroll_past_end,
                        wrap_width: xi_config.config.wrap_width,
                        word_wrap: xi_config.config.word_wrap,
                        autodetect_whitespace: xi_config.config.autodetect_whitespace,
                        line_ending: xi_config.config.line_ending.to_string(),
                        surrounding_pairs: xi_config.config.surrounding_pairs.clone(),
                        save_with_newline: xi_config.config.save_with_newline,
                    },
                }
            } else {
                error!(
                    "{}",
                    gettext("Couldn't read config, falling back to the default XI-Editor config")
                );
                xi_config
                    .save()
                    .unwrap_or_else(|e| error!("{}", e.to_string()));
                xi_config
            };

            let config_dir = config_dir.into_os_string().into_string().unwrap();
            debug!(
                "{}: '{}'",
                gettext("Discovered config dir in home dir"),
                &config_dir
            );

            (config_dir, xi_config)
        } else {
            error!(
                "{}",
                gettext("Couldn't determine home dir! Settings will be temporary")
            );

            let config_dir = tempfile::Builder::new()
                .prefix("gxi-config")
                .tempdir()
                .map_err(|e| {
                    error!(
                        "{} {}",
                        gettext("Failed to create temporary config dir"),
                        e.to_string()
                    )
                })
                .unwrap()
                .into_path();

            let xi_config = Self {
                config: XiConfig::default(),
                path: config_dir
                    .join("preferences.xiconfig")
                    .to_str()
                    .map(std::string::ToString::to_string)
                    .unwrap(),
            };
            xi_config
                .save()
                .unwrap_or_else(|e| error!("{}", e.to_string()));

            let config_dir = config_dir.into_os_string().into_string().unwrap();

            debug!(
                "{}: '{}'",
                gettext("Discovered config dir in temporary dir"),
                &config_dir
            );

            (config_dir, xi_config)
        }
    }

    pub fn open(&mut self) -> Result<&mut Self, Error> {
        trace!("{}", gettext("Opening config file"));
        let mut config_file = OpenOptions::new().read(true).open(&self.path)?;
        let mut config_string = String::new();

        trace!("{}", gettext("Reading config file"));
        config_file.read_to_string(&mut config_string)?;

        let config_toml: XiConfig = toml::from_str(&config_string)?;
        debug!("{}: {:?}", gettext("Xi-Config"), config_toml);

        self.config = config_toml;

        Ok(self)
    }

    /// Atomically write the config. First writes the config to a tmp_file (non-atomic) and then
    /// copies that (atomically). This ensures that the config files stay valid
    pub fn save(&self) -> Result<(), Error> {
        trace!("{} '{}'", gettext("Saving config to"), &self.path);
        let tmp_dir = tempdir()?;
        let tmp_file_path = tmp_dir.path().join(".gxi-atomic");
        let mut tmp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&tmp_file_path)?;

        tmp_file.write_all(toml::to_string(&self.config).unwrap().as_bytes())?;
        std::fs::copy(&tmp_file_path, &self.path)?;
        OpenOptions::new().read(true).open(&self.path)?.sync_all()?;

        Ok(())
    }
}

pub trait GSchemaExt<RHS = Self> {
    fn get(schema_name: &str, field_name: &str) -> Option<RHS>;

    fn set(schema_name: &str, field_name: &str, val: RHS);
}

pub struct GSchema {}

impl GSchemaExt<String> for GSchema {
    fn get(schema_name: &str, field_name: &str) -> Option<String> {
        SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .and_then(|_| Settings::new(schema_name).get_string(field_name))
            .map(|s| s.to_string())
    }

    fn set(schema_name: &str, field_name: &str, val: String) {
        if SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .is_some()
        {
            Settings::new(schema_name).set_string(field_name, &val);
        };
    }
}

impl GSchemaExt<bool> for GSchema {
    fn get(schema_name: &str, field_name: &str) -> Option<bool> {
        SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .map(|_| Settings::new(schema_name).get_boolean(field_name))
    }

    fn set(schema_name: &str, field_name: &str, val: bool) {
        if SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .is_some()
        {
            Settings::new(schema_name).set_boolean(field_name, val);
        };
    }
}

impl GSchemaExt<u32> for GSchema {
    fn get(schema_name: &str, field_name: &str) -> Option<u32> {
        SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .map(|_| Settings::new(schema_name).get_uint(field_name))
    }

    fn set(schema_name: &str, field_name: &str, val: u32) {
        if SettingsSchemaSource::get_default()
            .and_then(|settings_source| settings_source.lookup(schema_name, true))
            .is_some()
        {
            Settings::new(schema_name).set_uint(field_name, val);
        };
    }
}

pub fn get_theme_schema() -> String {
    GSchema::get(app_id!(), "theme-name").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to default theme.");
        "InspiredGitHub".to_string()
    })
}

pub fn set_theme_schema(theme_name: String) {
    GSchema::set(app_id!(), "theme-name", theme_name);
}

pub fn get_default_monospace_font_schema() -> String {
    GSchema::get("org.gnome.desktop.interface", "monospace-font-name").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to default monospace font.");
        "Monospace".to_string()
    })
}

pub fn get_default_interface_font_schema() -> String {
    GSchema::get("org.gnome.desktop.interface", "font-name").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to default interface font.");
        "Cantarell 11".to_string()
    })
}

pub fn get_draw_trailing_spaces_schema() -> bool {
    GSchema::get(app_id!(), "draw-trailing-spaces").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to not drawing tabs!");
        false
    })
}

pub fn set_draw_trailing_spaces_schema(val: bool) {
    GSchema::set(app_id!(), "draw-trailing-spaces", val);
}

pub fn get_draw_right_margin() -> bool {
    GSchema::get(app_id!(), "draw-right-margin").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to not drawing a right hand margin!");
        false
    })
}

pub fn set_draw_right_margin(val: bool) {
    GSchema::set(app_id!(), "draw-right-margin", val);
}

pub fn get_column_right_margin() -> u32 {
    GSchema::get(app_id!(), "column-right-margin").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to drawing right hand marging at column 80");
        80
    })
}

pub fn set_column_right_margin(val: u32) {
    GSchema::set(app_id!(), "column-right-margin", val);
}

pub fn get_highlight_line() -> bool {
    GSchema::get(app_id!(), "highlight-line").unwrap_or_else(|| {
        warn!("Couldn't find GSchema! Defaulting to not highlighting the current line");
        false
    })
}

pub fn set_highlight_line(val: bool) {
    GSchema::set(app_id!(), "highlight-line", val);
}
