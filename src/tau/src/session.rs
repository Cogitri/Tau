// Copyright (C) 2019 Tom Steu <steudtner.tom@gmail.com>
// SPDX-License-Identifier: MIT

use gio::SettingsExt;

pub trait SessionHandler {
    /// Add path to session
    fn session_add(&self, path: String);
    /// Remove path from session
    fn session_remove(&self, path: &str);
    /// List all paths in current session
    fn get_session(&self) -> Vec<String>;
}

impl SessionHandler for gio::Settings {
    fn session_add(&self, path: String) {
        let old_session = self.get_strv("session");
        if old_session.iter().find(|x| x.as_str() == path) == None {
            let mut new_session: Vec<_> = old_session.iter().map(|x| x.as_str()).collect();
            new_session.push(&path);
            self.set_strv("session", new_session.as_slice()).unwrap();
        }
    }

    fn session_remove(&self, path: &str) {
        let old_session = self.get_strv("session");
        let new_session: Vec<_> = old_session
            .iter()
            .filter_map(|x| if *x != path { Some(x.as_str()) } else { None })
            .collect();
        self.set_strv("session", new_session.as_slice()).unwrap();
    }

    fn get_session(&self) -> Vec<String> {
        self.get_strv("session")
            .iter()
            .map(|x| x.to_string())
            .collect()
    }
}
