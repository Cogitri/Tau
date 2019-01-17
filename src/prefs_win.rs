use crate::main_win::MainState;
use crate::pref_storage::{Config, GtkXiConfig, XiConfig};
use crate::rpc::Core;
use gtk::*;
use log::{debug, error};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use toml::Value;

pub struct PrefsWin {
    core: Rc<RefCell<Core>>,
    window: Window,
}

impl PrefsWin {
    pub fn new(
        parent: &ApplicationWindow,
        main_state: &Rc<RefCell<MainState>>,
        core: &Rc<RefCell<Core>>,
        xi_config: Arc<Mutex<Config<XiConfig>>>,
        gxi_config: Arc<Mutex<Config<GtkXiConfig>>>,
    ) -> Rc<RefCell<PrefsWin>> {
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
                    debug!("settings active {}", i);
                    theme_combo_box.set_active(i as i32);
                }
            }
        }

        {
            let main_state = main_state.borrow();
            let conf = xi_config.lock().unwrap();
            for (i, font_name) in main_state.fonts.iter().enumerate() {
                font_combo_box.append_text(font_name);
                if conf.config.font_face == Value::String(font_name.to_string()) {
                    debug!("Setting active font {}, num {}", &font_name, i);
                    font_combo_box.set_active(i as i32);
                }
            }
        }
        #[allow(unused_variables)]
        font_combo_box.connect_changed(clone!(core => move |cb|{
            if let Some(font_name) = cb.get_active_text() {
                debug!("font changed to {:?}", &font_name);

                let mut conf = xi_config.lock().unwrap();
                conf.config.font_face = Value::String(font_name);
                conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
            }
        }));

        theme_combo_box.connect_changed(clone!(core, main_state => move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                debug!("theme changed to {:?}", cb.get_active_text());
                let core = core.borrow();
                core.set_theme(&theme_name);

                let mut conf = gxi_config.lock().unwrap();
                conf.config.theme = Value::String(theme_name.clone());
                conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();

                let mut main_state = main_state.borrow_mut();
                main_state.theme_name = theme_name;
            }
        }));

        let prefs_win = Rc::new(RefCell::new(PrefsWin {
            core: core.clone(),
            window: window.clone(),
        }));

        window.set_transient_for(parent);
        window.show_all();

        prefs_win
    }
}
