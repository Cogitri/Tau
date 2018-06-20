use gtk::*;
use main_win::MainState;
use rpc::Core;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PrefsWin {
    core: Rc<RefCell<Core>>,
    window: Window,
    // pub themes: Vec<String>,
    // pub theme_name: String,
    // pub theme: Theme,
    // pub styles: Vec<Style>,
}

impl PrefsWin {

    pub fn new(parent: &ApplicationWindow, main_state: &Rc<RefCell<MainState>>, core: &Rc<RefCell<Core>>) -> Rc<RefCell<PrefsWin>> {
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
                    debug!("setting active {}", i);
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
            window: window.clone(),
        }));

        window.set_transient_for(parent);
        window.show_all();

        prefs_win
    }

}