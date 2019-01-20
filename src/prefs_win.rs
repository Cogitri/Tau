use crate::main_win::MainState;
use crate::pref_storage::{Config, GtkXiConfig, XiConfig};
use crate::rpc::Core;
use gettextrs::{bindtextdomain, gettext, setlocale, textdomain, LocaleCategory};
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
        let tab_stops_checkbutton: ToggleButton =
            builder.get_object("tab_stops_checkbutton").unwrap();
        let scroll_past_end_checkbutton: ToggleButton =
            builder.get_object("scroll_past_end_checkbutton").unwrap();
        let word_wrap_checkbutton: ToggleButton =
            builder.get_object("word_wrap_checkbutton").unwrap();

        // Set up the textdomain for gettext
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain("gxi", crate::globals::LOCALEDIR.unwrap_or("./po"));
        textdomain("gxi");

        {
            let conf = xi_config.lock().unwrap();

            let mut font_desc = FontDescription::new();
            font_desc.set_size(conf.config.font_size as i32 * 1024);
            font_desc.set_family(&conf.config.font_face);

            debug!(
                "{}: {}",
                gettext("Setting font description"),
                &conf.config.font_face
            );

            font_chooser_widget.set_font_desc(&font_desc);
        }

        {
            let conf = xi_config.lock().unwrap().clone();

            #[allow(unused_variables)]
            font_chooser_widget.connect_property_font_notify(clone!(core => move |font_widget|{
                if let Some(font_desc) = font_widget.get_font_desc() {
                    let mut font_conf = conf.clone();

                    let font_family = font_desc.get_family().unwrap();
                    let font_size = font_desc.get_size() / 1024;
                    debug!("{} {}", gettext("Setting font to"), &font_family);
                    debug!("{} {}", gettext("Setting font size to"), &font_size);

                    font_conf.config.font_size = font_size as u32;
                    font_conf.config.font_face = font_family;
                    font_conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
                }
            }));
        }

        {
            let main_state = main_state.borrow();
            for (i, theme_name) in main_state.themes.iter().enumerate() {
                theme_combo_box.append_text(theme_name);
                if &main_state.theme_name == theme_name {
                    debug!("{} {}", gettext("Settings window active"), i);
                    theme_combo_box.set_active(i as i32);
                }
            }
        }

        theme_combo_box.connect_changed(clone!(core, main_state => move |cb|{
            if let Some(theme_name) = cb.get_active_text() {
                debug!("{} {:?}", gettext("Theme changed to"), cb.get_active_text());
                let core = core.borrow();
                core.set_theme(&theme_name);

                let mut conf = gxi_config.lock().unwrap();
                conf.config.theme = theme_name.clone();
                conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();

                let mut main_state = main_state.borrow_mut();
                main_state.theme_name = theme_name;
            }
        }));

        {
            let conf = xi_config.lock().unwrap().clone();

            scroll_past_end_checkbutton.set_active(conf.config.scroll_past_end);

            #[allow(unused_variables)]
            scroll_past_end_checkbutton.connect_toggled(clone!(core => move |toggle_btn| {
                let value = toggle_btn.get_active();
                    let mut conf = conf.clone();
                    debug!("{}: {}", gettext("Scrolling past end"), value);
                    conf.config.scroll_past_end = value;
                    conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
            }));
        }

        {
            let conf = xi_config.lock().unwrap().clone();

            word_wrap_checkbutton.set_active(conf.config.word_wrap);

            #[allow(unused_variables)]
            word_wrap_checkbutton.connect_toggled(clone!(core => move |toggle_btn| {
                let value = toggle_btn.get_active();
                    let mut conf = conf.clone();
                    debug!("{}: {}", gettext("Word wrapping"), value);
                    conf.config.word_wrap = value;
                    conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
            }));
        }

        {
            let conf = xi_config.lock().unwrap().clone();

            tab_stops_checkbutton.set_active(conf.config.use_tab_stops);

            #[allow(unused_variables)]
            tab_stops_checkbutton.connect_toggled(clone!(core => move |toggle_btn| {
                let value = toggle_btn.get_active();
                    let mut conf = conf.clone();
                    debug!("{}: {}", gettext("Tab stops"), value);
                    conf.config.use_tab_stops = value;
                    conf.save().map_err(|e| error!("{}", e.to_string())).unwrap();
            }));
        }

        let prefs_win = Rc::new(RefCell::new(PrefsWin {
            core: core.clone(),
            window: window.clone(),
        }));

        window.set_transient_for(parent);
        window.show_all();

        prefs_win
    }
}
