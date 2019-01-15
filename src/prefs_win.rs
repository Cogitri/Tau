use crate::main_win::MainState;
use crate::pref_storage::GtkXiConfig;
use crate::rpc::Core;
use gtk::*;
use log::{debug, error, warn};
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
        gxi_config: Arc<Mutex<GtkXiConfig>>,
        gxi_config_file_path: Option<String>,
    ) -> Rc<RefCell<PrefsWin>> {
        let glade_src = include_str!("ui/prefs_win.glade");
        let builder = Builder::new_from_string(glade_src);

        let window: Window = builder.get_object("prefs_win").unwrap();
        //let font_combo_box: ComboBoxText = builder.get_object("font_combo_box").unwrap();
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

        theme_combo_box.connect_changed(clone!(core, main_state => move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                debug!("theme changed to {:?}", cb.get_active_text());
                let core = core.borrow();
                core.set_theme(&theme_name);

                if let Some(gxi_config_file_path) = gxi_config_file_path.as_ref() {
                    let mut config = gxi_config.lock().unwrap();
                    config.theme = Value::String(theme_name.clone());
                    config.save(&gxi_config_file_path).map_err(|e| error!("{}", e.to_string())).unwrap();
                } else {
                    warn!("No config dir set, settings will be temporary!");
                }

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
