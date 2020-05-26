// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

use crate::main_win::MainWin;
use gio::prelude::{SettingsExt, SettingsExtManual};
use glib::Receiver;
use gtk::Application;
use log::{debug, error};
use serde_json::json;
use serde_json::Value;
use std::cmp::max;
use std::rc::Rc;
use tau_rpc::{Client, RpcOperations};

pub(crate) struct MainWinBuilder {
    application: Application,
    core: Rc<Client>,
    from_core: Option<Receiver<RpcOperations>>,
    pub(crate) main_win: Option<Rc<MainWin>>,
}

impl MainWinBuilder {
    pub fn new(application: Application) -> MainWinBuilder {
        let (client, recv) = Client::new();
        client.client_started(
            std::env::var("XI_CONFIG_DIR").ok().as_ref(),
            crate::globals::PLUGIN_DIR.map(|s| s.to_string()).as_ref(),
        );

        MainWinBuilder {
            application,
            core: client,
            from_core: Some(recv),
            main_win: None,
        }
    }

    fn init_config(&self) {
        #[cfg(windows)]
        const LINE_ENDING: &str = "\r\n";
        #[cfg(not(windows))]
        const LINE_ENDING: &str = "\n";

        debug!("Initialising user config");

        let gschema = gio::Settings::new("org.gnome.Tau");

        let tab_size = gschema.get::<u32>("tab-size");
        let autodetect_whitespace = gschema.get::<bool>("auto-indent");
        let translate_tabs_to_spaces = gschema.get::<bool>("translate-tabs-to-spaces");
        let use_tab_stops = gschema.get::<bool>("use-tab-stops");
        let word_wrap = gschema.get::<bool>("word-wrap");

        let font = gschema.get::<String>("font");
        let font_vec = font.split_whitespace().collect::<Vec<_>>();
        let (font_size, font_name) = if let Some((size, splitted_name)) = font_vec.split_last() {
            (size.parse::<f32>().unwrap_or(14.0), splitted_name.join(" "))
        } else {
            error!("Failed to get font configuration. Resetting...");
            gschema.reset("font");
            (14.0, "Monospace".to_string())
        };

        self.core.as_ref().modify_user_config_domain(
            "general",
            &json!({
                "tab_size": max(1, tab_size),
                "autodetect_whitespace": autodetect_whitespace,
                "translate_tabs_to_spaces": translate_tabs_to_spaces,
                "font_face": font_name,
                "font_size": if font_size.is_nan() {
                    14.0
                } else if font_size < 6.0 {
                    6.0
                } else if font_size > 72.0 {
                    72.0
                } else { font_size },
                "use_tab_stops": use_tab_stops,
                "word_wrap": word_wrap,
                "line_ending": LINE_ENDING,
            }),
        );

        let val = gschema.get_strv("syntax-config");

        for x in val {
            if let Ok(val) = serde_json::from_str(x.as_str()) {
                self.core.as_ref().modify_user_config(val);
            } else {
                error!("Failed to deserialize syntax config. Resetting...");
                gschema.reset("syntax-config");
            }
        }
    }

    pub fn spawn_view<F>(&self, file_path: Option<String>, cb: F)
    where
        F: FnOnce(Result<Value, Value>) + Send + 'static,
    {
        debug!("Spawning view with filepath {:?}", file_path);

        self.core.new_view(file_path.as_ref(), cb)
    }

    pub fn build(&mut self) {
        debug!("Building MainWin");

        self.init_config();

        let win = MainWin::new(
            &self.application,
            self.core.clone(),
            self.from_core.take().unwrap(),
        );

        self.main_win = Some(win);
    }
}
