use failure::Fail;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Fail)]
pub enum Error {
    #[fail(display = "Failed to set GSettings key! Error: {}", _0)]
    GSettings(String),
}
