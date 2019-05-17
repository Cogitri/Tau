use crate::errors::ErrorDialog;
use gettextrs::gettext;
use gio::prelude::*;
use gtk::{ContainerExt, GtkWindowExt, TextBufferExt, TextViewExt, WidgetExt};
use gxi_peer::ErrorMsg;
use human_panic::{handle_dump, Metadata};
use std::panic::{self, PanicInfo};

pub struct PanicHandler {}

impl PanicHandler {
    pub fn new() {
        let meta = Metadata {
            version: env!("CARGO_PKG_VERSION").into(),
            name: env!("CARGO_PKG_NAME").into(),
            authors: env!("CARGO_PKG_AUTHORS").replace(":", ", ").into(),
            homepage: env!("CARGO_PKG_HOMEPAGE").into(),
        };

        panic::set_hook(Box::new(move |info: &PanicInfo| {
            let file_path = handle_dump(&meta, info).unwrap();

            let application = gtk::Application::new(
                Some("com.github.Cogitri.gxi.error-reporter"),
                Default::default(),
            )
            .unwrap();

            application.connect_activate(move |app| {
                let text_view = gtk::TextView::new();
                text_view.set_editable(false);
                text_view
                    .get_buffer()
                    .unwrap()
                    .set_text(&std::fs::read_to_string(file_path.clone()).unwrap());

                let window = gtk::ApplicationWindow::new(app);
                window.set_title(&gettext("gxi crash reporter"));
                let scroll_win =
                gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
                scroll_win.add(&text_view);
                window.add(&scroll_win);
                window.set_border_width(5);
                //TODO: Set a dconf value to make this the same size as the original gxi window
                window.set_default_size(800, 400);
                window.show_all();

                let crash1 = gettext("It seems like gxi has crashed, sorry!");
                let crash2 = gettext("Please send the contents of the file below to our GitHub issue tracker so we can fix it! Thank you =)");
                let err_msg = ErrorMsg {
                    fatal: false,
                    msg: format!("{}\n{}\n\n{}: {:#?}\n{}: {}", crash1, crash2, gettext("File"), file_path, gettext("URL"), "https://github.com/Cogitri/gxi/issues"),
                };

                ErrorDialog::new(err_msg.clone());
            });

            application.run(&Vec::new());
        }));
    }
}
