// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use syntect::highlighting::ThemeSettings;

/// Options for drawing of invisibles, e.g. tabs, spaces
pub enum ShowInvisibles {
    None,
    All,
    Leading,
    Trailing,
    Selected,
}

/// A Struct containing setting switches for the `EditView`
pub struct Settings {
    pub gschema: gio::Settings,
    pub draw_spaces: ShowInvisibles,
    pub draw_tabs: ShowInvisibles,
    pub highlight_line: bool,
    pub right_margin: bool,
    pub column_right_margin: u32,
    pub interface_font: String,
    pub edit_font: String,
    pub draw_cursor: bool,
    pub show_linecount: bool,
    pub full_title: bool,
}

pub struct MainState {
    pub themes: Vec<String>,
    pub theme_name: String,
    pub theme: ThemeSettings,
    pub styles: HashMap<usize, tau_rpc::Style>,
    pub fonts: Vec<String>,
    pub avail_languages: Vec<String>,
    pub selected_language: String,
    pub settings: Settings,
}
