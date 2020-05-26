// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

use crate::globals;
use gettextrs::gettext;
use gtk::prelude::*;
use gtk::{AboutDialog, ApplicationWindow};
use log::trace;

/// The about window, which displays some simple info about tau
#[derive(Clone)]
pub struct AboutWin {
    pub about_dialog: AboutDialog,
}

impl AboutWin {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let about_dialog = gtk::AboutDialog::new();
        about_dialog.set_comments(Some(
            gettext("GTK frontend for the xi text editor, written in Rust").as_str(),
        ));
        about_dialog.set_copyright(Some("\u{a9} 2017 Brian Vincent, 2019 Rasmus Thomsen."));
        about_dialog.set_license_type(gtk::License::MitX11);
        about_dialog.set_modal(true);
        about_dialog.set_version(globals::VERSION);
        about_dialog.set_program_name(glib::get_application_name().unwrap().as_str());
        about_dialog.set_website(Some("https://gitlab.gnome.org/World/tau"));
        about_dialog.set_translator_credits(Some(gettext("translator-credits").as_str()));
        about_dialog.set_logo_icon_name(Some(globals::APP_ID.unwrap_or("org.gnome.TauDevel")));

        about_dialog.set_authors(&["Brian Vincent", "Rasmus Thomsen"]);

        about_dialog.set_transient_for(Some(parent));
        trace!("Showing about window");
        about_dialog.show_all();

        Self { about_dialog }
    }
}
