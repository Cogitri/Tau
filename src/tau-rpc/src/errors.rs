use serde_json::error::Category;
use serde_json::error::Error as SerdeError;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum DecodeError {
    Truncated,
    Io(io::Error),
    InvalidJson,
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        self.to_string().fmt(f)
    }
}

impl Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::Truncated => "not enough bytes to decode a complete message",
            DecodeError::Io(_) => "failure to read or write bytes on an IO stream",
            DecodeError::InvalidJson => "the byte sequence is not valid JSON",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        if let DecodeError::Io(ref io_err) = *self {
            Some(io_err)
        } else {
            None
        }
    }
}

impl From<SerdeError> for DecodeError {
    fn from(err: SerdeError) -> DecodeError {
        match err.classify() {
            Category::Io => DecodeError::Io(err.into()),
            Category::Eof => DecodeError::Truncated,
            Category::Data | Category::Syntax => DecodeError::InvalidJson,
        }
    }
}
