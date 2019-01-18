use crate::main_win::MainState;
use crate::pref_storage::{Config, GtkXiConfig, XiConfig};
use crate::rpc::Core;
use gtk::*;
use log::{debug, error};
use pango::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
        let font_chooser_widget: FontChooserWidget =
            builder.get_object("font_chooser_widget").unwrap();
        let theme_combo_box: ComboBoxText = builder.get_object("theme_combo_box").unwrap();

        {
            let conf = xi_config.lock().unwrap();

            let mut font_desc = FontDescription::new();
            font_desc.set_size(conf.config.font_size as i32);
            font_desc.set_family(&conf.config.font_face);

            debug!("Setting font desc: {}", &conf.config.font_face);

            font_chooser_widget.set_font_desc(&font_desc);
        }

        #[allow(unused_variables)]
        font_chooser_widget.connect_property_font_notify(clone!(core => move |font_widget|{
            let mut conf = xi_config.lock().unwrap();

            if let Some(font_desc) = font_widget.get_font_desc() {
                debug!("Setting font to {}", &font_desc.get_family().unwrap());

                conf.config.font_face = font_desc.get_family().unwrap();
                debug!("Setting font size to {}", font_desc.get_size() / 1000);
                conf.config.font_size = font_desc.get_size() as u32 / 1000;
                conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
            }
        }));

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

                let mut conf = gxi_config.lock().unwrap();
                conf.config.theme = theme_name.clone();
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
