use gettextrs::gettext;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::*;
use gxi_peer::ErrorMsg;
use log::error;

/// A simple `ErrorDialog` used for if stuff goes south.
pub struct ErrorDialog {
    pub dialog: MessageDialog,
    pub msg: ErrorMsg,
}

impl ErrorDialog {
    /// Creates a new `ErrorDialog` containing the `err_msg`. Quits the application if `fatal` is true.
    pub fn new(err_msg: ErrorMsg) -> Self {
        error!("{}", err_msg.msg);
        let application = gio::Application::get_default()
            .unwrap_or_else(|| panic!("{}", &gettext("No default application")))
            .downcast::<gtk::Application>()
            .unwrap_or_else(|_| panic!("{}", &gettext("Default application has wrong type")));

        let err_dialog = MessageDialog::new(
            application.get_active_window().as_ref(),
            DialogFlags::MODAL,
            MessageType::Error,
            ButtonsType::Ok,
            &err_msg.msg,
        );

        err_dialog.connect_response(clone!(err_msg => move |err_dialog, _| {
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
