use failure::Fail;

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
