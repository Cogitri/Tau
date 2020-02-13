use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::{ButtonsType, DialogFlags, MessageDialog, MessageType};
use log::error;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct XiClientError {
    pub code: i64,
    pub message: String,
}

/// A struct holding the error message to be displayed
#[derive(Clone, Debug)]
pub struct ErrorMsg {
    /// The error message
    pub msg: String,
    /// Whether or not the program should terminate after the error message has been shown.
    pub fatal: bool,
}

impl ErrorMsg {
    pub fn new(msg: String, fatal: bool) -> Self {
        Self { msg, fatal }
    }
}

/// A simple `ErrorDialog` used for if stuff goes south.
#[derive(Clone)]
pub struct ErrorDialog {
    /// The GTK `MessageDialog`, if we have to do something custom
    pub dialog: MessageDialog,
    /// The text of the `MessageDialog`
    pub msg: ErrorMsg,
}

impl ErrorDialog {
    /// Creates a new `ErrorDialog` containing the `err_msg`. Quits the application if `fatal` is true.
    pub fn new(err_msg: ErrorMsg) -> Self {
        error!("{}", err_msg.msg);
        let application = gio::Application::get_default()
            .expect("No default application")
            .downcast::<gtk::Application>()
            .expect("Default application has wrong type");

        let err_dialog = MessageDialog::new(
            application.get_active_window().as_ref(),
            DialogFlags::MODAL,
            MessageType::Error,
            ButtonsType::Ok,
            &err_msg.msg,
        );

        err_dialog.connect_response(clone!(@strong err_msg => move |err_dialog, _| {
            err_dialog.destroy();

            if err_msg.fatal {
                application.quit();
            }
        }));

        err_dialog.show_all();

        Self {
            dialog: err_dialog,
            msg: err_msg,
        }
    }
}
