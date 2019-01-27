use failure::Fail;
use gettextrs::gettext;
use gio::prelude::*;
use gtk::prelude::*;
use gtk::*;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Fail)]
pub enum Error {
    //#[fail(display="Failed! {}", _0)]
    //PrefStorage(String),
    #[fail(display = "Failed to read/write config file! Error: {}", _0)]
    IO(String),
    #[fail(display = "Failed to deserialize config TOML! Error: {}", _0)]
    DeToml(String),
    #[fail(display = "Failed to serialize config TOML! Error: {}", _0)]
    SerToml(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::DeToml(e.to_string())
    }
}
impl From<toml::ser::Error> for Error {
    fn from(e: toml::ser::Error) -> Self {
        Error::SerToml(e.to_string())
    }
}

/// A simple ErrorDialog used for if stuff goes south.
pub struct ErrorDialog {}

impl ErrorDialog {
    /// Creates a new ErrorDialog containing the err_msg. Quits the application if fatal is true.
    pub fn new(err_msg: &str, fatal: bool) {
        let application = gio::Application::get_default()
            .expect(&gettext("No default application"))
            .downcast::<gtk::Application>()
            .expect(&gettext("Default application has wrong type"));

        let err_dialog = MessageDialog::new(
            application.get_active_window().as_ref(),
            DialogFlags::MODAL,
            MessageType::Error,
            ButtonsType::Ok,
            &err_msg,
        );

        err_dialog.connect_response(move |err_dialog, _| {
            err_dialog.destroy();

            if fatal {
                application.quit();
            }
        });

        err_dialog.show_all();
    }
}
