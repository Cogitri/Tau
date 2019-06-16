use failure::Fail;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Fail)]
pub enum Error {
    #[fail(display = "Failed to set readonly key '{}'", _0)]
    ReadOnly(String),
    #[fail(display = "Tried to get non-existent key '{}'!", _0)]
    GetNonExistent(String),
    #[fail(display = "Tried to set non-existent key '{}'!", _0)]
    SetNonExistent(String),
    #[fail(display = "Couldn't retrieve schema source!")]
    NoSchemaSource,
    #[fail(display = "Couldn't get String for key '{}'", _0)]
    NoString(String),
    #[fail(display = "Couldn't get Variant for key '{}'", _0)]
    NoValue(String),
}
