use gettextrs::gettext;
use gtk::*;
use log::trace;
use std::cell::RefCell;
use std::rc::Rc;

/// The about window, which displays some simple info about gxi
pub struct AboutWin {
    about_dialog: AboutDialog,
}

impl AboutWin {
    pub fn new(parent: &ApplicationWindow) -> Rc<RefCell<Self>> {
        let about_dialog = gtk::AboutDialog::new();
        about_dialog
            .set_comments(gettext("GTK frontend for the xi text editor, written in Rust").as_str());
        about_dialog.set_copyright("\u{a9} 2017 Brian Vincent, 2019 Rasmus Thomsen.");
        about_dialog.set_license_type(gtk::License::MitX11);
        about_dialog.set_modal(true);
        about_dialog.set_version(crate::globals::VERSION.unwrap_or("0.0.0"));
        about_dialog.set_program_name(crate::globals::APP_NAME.unwrap_or("gxi"));
        about_dialog.set_website("https://gxi.cogitri.dev");
        about_dialog.set_website_label(gettext("gxi's Github Repo").as_str());
        about_dialog.set_translator_credits(gettext("translator-credits").as_str());
        about_dialog.set_logo_icon_name("com.github.Cogitri.gxi");

        about_dialog.set_authors(&["Brian Vincent", "Rasmus Thomsen"]);

        about_dialog.set_transient_for(parent);
        trace!("{}", gettext("Showing about window"));
        about_dialog.show_all();

        Rc::new(RefCell::new(Self {
            about_dialog: about_dialog.clone(),
        }))
    }
}
