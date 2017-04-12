use serde_json;

use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum GxiError {
    Custom(String),
    Io(io::Error),
    SerdeJson(serde_json::Error),
    MalformedMethodParams(String, serde_json::Value),
    UnknownMethod(String),
}

impl fmt::Display for GxiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GxiError::Custom(ref msg) => write!(f, "{}", msg),
            GxiError::Io(ref err) => err.fmt(f),
            GxiError::SerdeJson(ref err) => err.fmt(f),
            GxiError::MalformedMethodParams(ref method, ref params) => {
                write!(f, "{}: '{}' params: {}", self.description(), method, params)
            }
            GxiError::UnknownMethod(ref method) => {
                write!(f, "{}: {}", self.description(), method)
            }
        }
    }
}

impl Error for GxiError {
    fn description(&self) -> &str {
        match *self {
            GxiError::Custom(ref msg) => msg,
            GxiError::Io(ref err) => err.description(),
            GxiError::SerdeJson(ref err) => err.description(),
            GxiError::MalformedMethodParams(ref method, ref params) =>
                "Malformed method params",
            GxiError::UnknownMethod(ref method) => "Unknown method",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            GxiError::Custom(ref msg) => None,
            GxiError::Io(ref err) => Some(err),
            GxiError::SerdeJson(ref err) => Some(err),
            GxiError::MalformedMethodParams(ref method, ref params) => None,
            GxiError::UnknownMethod(ref method) => None,
        }
    }
}

impl From<io::Error> for GxiError {
    fn from(err: io::Error) -> GxiError {
        GxiError::Io(err)
    }
}

impl From<serde_json::Error> for GxiError {
    fn from(err: serde_json::Error) -> GxiError {
        GxiError::SerdeJson(err)
    }
}
