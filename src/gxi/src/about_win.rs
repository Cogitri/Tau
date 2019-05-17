use gettextrs::gettext;
use gtk::*;
use gxi_utils::globals;
use log::trace;
use std::cell::RefCell;
use std::rc::Rc;

/// The about window, which displays some simple info about gxi
pub struct AboutWin {
    pub about_dialog: AboutDialog,
}

impl AboutWin {
    pub fn new(parent: &ApplicationWindow) -> Rc<RefCell<Self>> {
        let about_dialog = gtk::AboutDialog::new();
        about_dialog.set_comments(Some(
            gettext("GTK frontend for the xi text editor, written in Rust").as_str(),
        ));
        about_dialog.set_copyright(Some("\u{a9} 2017 Brian Vincent, 2019 Rasmus Thomsen."));
        about_dialog.set_license_type(gtk::License::MitX11);
        about_dialog.set_modal(true);
        about_dialog.set_version(globals::VERSION);
        about_dialog.set_program_name(globals::APP_NAME.unwrap_or("com.github.Cogitri.gxi"));
        about_dialog.set_website(Some("https://gxi.cogitri.dev"));
        about_dialog.set_translator_credits(Some(gettext("translator-credits").as_str()));
        about_dialog.set_logo_icon_name(Some("com.github.Cogitri.gxi"));

        about_dialog.set_authors(&["Brian Vincent", "Rasmus Thomsen"]);

        about_dialog.set_transient_for(Some(parent));
        trace!("{}", gettext("Showing about window"));
        about_dialog.show_all();

        Rc::new(RefCell::new(Self {
            about_dialog: about_dialog.clone(),
        }))
    }
}
