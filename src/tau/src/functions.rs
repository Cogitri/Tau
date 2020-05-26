// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

use editview::main_state::ShowInvisibles;
use editview::Settings;
use gettextrs::gettext;
use gio::prelude::*;

pub fn get_font_properties(font: &str) -> Option<(String, f32)> {
    let font_vec = font.split_whitespace().collect::<Vec<_>>();
    font_vec.split_last().map(|(size, name)| {
        let font_name = name.join(" ");
        let font_size = size.parse::<f32>().unwrap();
        (font_name, font_size)
    })
}

/// Generate a new `Settings` object, which we pass to the `EditView` to set its behaviour.
pub fn new_settings() -> editview::Settings {
    let gschema = gio::Settings::new("org.gnome.Tau");
    let interface_font = {
        use gtk::SettingsExt;
        let gtk_settings = gtk::Settings::get_default().unwrap();
        gtk_settings
            .get_property_gtk_font_name()
            .unwrap()
            .to_string()
    };

    Settings {
        draw_spaces: {
            if gschema.get("draw-trailing-spaces") {
                ShowInvisibles::Trailing
            } else if gschema.get("draw-leading-spaces") {
                ShowInvisibles::Leading
            } else if gschema.get("draw-all-spaces") {
                ShowInvisibles::All
            } else if gschema.get("draw-selection-spaces") {
                ShowInvisibles::Selected
            } else {
                ShowInvisibles::None
            }
        },
        draw_tabs: {
            if gschema.get("draw-trailing-tabs") {
                ShowInvisibles::Trailing
            } else if gschema.get("draw-leading-tabs") {
                ShowInvisibles::Leading
            } else if gschema.get("draw-all-tabs") {
                ShowInvisibles::All
            } else if gschema.get("draw-selection-tabs") {
                ShowInvisibles::Selected
            } else {
                ShowInvisibles::None
            }
        },
        highlight_line: gschema.get("highlight-line"),
        right_margin: gschema.get("draw-right-margin"),
        column_right_margin: gschema.get("column-right-margin"),
        edit_font: gschema.get("font"),
        draw_cursor: gschema.get("draw-cursor"),
        show_linecount: gschema.get("show-linecount"),
        full_title: gschema.get("full-title"),
        interface_font,
        gschema,
    }
}

/// Run in terminal once it has finished initializing
pub fn vte_callback() {
    println!("{}", gettext("Welcome to Tau's terminal."));
}
