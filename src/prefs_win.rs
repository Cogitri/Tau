use gio::{
    ActionExt,
    ActionMapExt,
    ApplicationFlags,
    SimpleAction,
    SimpleActionExt,
};
use glib::variant::{FromVariant, Variant};
use gtk::*;
use CoreMsg;
use SharedQueue;
use edit_view::EditView;
use main_win::MainState;
use proto::{self, ThemeSettings};
use rpc::{Core, Handler};
use serde_json::{self, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::env::home_dir;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use theme::{Color, Style, Theme};
use xi_thread;

pub struct PrefsWin {
    core: Rc<RefCell<Core>>,
    // pub themes: Vec<String>,
    // pub theme_name: String,
    // pub theme: Theme,
    // pub styles: Vec<Style>,
}

impl PrefsWin {

    pub fn new(main_state: &Rc<RefCell<MainState>>, core: &Rc<RefCell<Core>>) -> Rc<RefCell<PrefsWin>> {
        let glade_src = include_str!("ui/prefs_win.glade");
        let builder = Builder::new_from_string(glade_src);

        let window: Window = builder.get_object("prefs_win").unwrap();
        let font_combo_box: ComboBoxText = builder.get_object("font_combo_box").unwrap();
        let theme_combo_box: ComboBoxText = builder.get_object("theme_combo_box").unwrap();

        {
            let main_state = main_state.borrow();
            for (i, theme_name) in main_state.themes.iter().enumerate() {
                theme_combo_box.append_text(theme_name);
                if &main_state.theme_name == theme_name {
                    theme_combo_box.set_active(i as i32);
                }
            }
        }

        theme_combo_box.connect_changed(clone!(core => move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                debug!("theme changed to {:?}", cb.get_active_text());
                let core = core.borrow();
                core.set_theme(&theme_name);
            }
        }));

        let prefs_win = Rc::new(RefCell::new(PrefsWin{
            core: core.clone(),
        }));

        window.show_all();

        prefs_win
    }
}