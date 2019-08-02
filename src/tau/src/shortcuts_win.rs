use gtk::prelude::*;
use gtk::{ApplicationWindow, Builder, ShortcutsWindow};
use log::trace;

/// The shortcuts window, which shows the user all keyboard shortcuts Tau offers
#[derive(Clone)]
pub struct ShortcutsWin {
    pub window: ShortcutsWindow,
}

impl ShortcutsWin {
    pub fn new(parent: &ApplicationWindow) -> Self {
        let builder = Builder::new_from_resource("/org/gnome/Tau/shortcuts_win.glade");

        trace!("Opening ShortcutsWin");

        let window: ShortcutsWindow = builder.get_object("shortcuts_win").unwrap();
        window.set_transient_for(Some(parent));
        window.show_all();

        Self { window }
    }
}
