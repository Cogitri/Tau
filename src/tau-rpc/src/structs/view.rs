// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
use std::str::FromStr;

use serde::de::Deserializer;
use serde::de::Error;
use serde::de::Visitor;
use serde::ser::Serializer;
use std::error::Error as StdError;
use std::fmt;
use std::num::ParseIntError;

/// Error Returned when a malformed `ViewId` is received.
#[derive(Debug, PartialEq)]
pub struct IdParseError(String);

impl IdParseError {
    pub fn new<S: Into<String>>(s: S) -> IdParseError {
        IdParseError(s.into())
    }
}

impl From<ParseIntError> for IdParseError {
    fn from(err: ParseIntError) -> IdParseError {
        IdParseError(format!("{}", err))
    }
}

impl fmt::Display for IdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl StdError for IdParseError {
    fn description(&self) -> &str {
        &self.0
    }
}

impl Error for IdParseError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        IdParseError(format!("{}", msg))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, Ord, PartialOrd)]
pub struct ViewId(pub usize);

impl FromStr for ViewId {
    type Err = IdParseError;
    fn from_str(s: &str) -> Result<ViewId, Self::Err> {
        if &s[..8] != "view-id-" {
            Err(IdParseError::new(
                "expected view id to be in the form of `view-id-x`.",
            ))
        } else {
            Ok(ViewId(s[8..].parse()?))
        }
    }
}

impl fmt::Display for ViewId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "view-id-{}", self.0)
    }
}

impl Serialize for ViewId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ViewId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ViewVisitor)
    }
}

struct ViewVisitor;

impl<'de> Visitor<'de> for ViewVisitor {
    type Value = ViewId;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expecting a string in the form of `view-id-x`.")
    }
    fn visit_str<E: Error>(self, s: &str) -> Result<Self::Value, E> {
        match ViewId::from_str(s) {
            Err(err) => Err(E::custom(&err)),
            Ok(v) => Ok(v),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeasureWidth(pub Vec<MeasureWidthInner>);

#[derive(Debug, Serialize, Deserialize)]
pub struct MeasureWidthInner {
    pub id: u64,
    pub strings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::*;
    use std::str::FromStr;
    #[test]
    fn from_string() {
        assert_eq!(Ok(ViewId(1)), FromStr::from_str("view-id-1"));
        assert_eq!(Ok(ViewId(1111)), FromStr::from_str("view-id-1111"));
        assert_eq!(Ok(ViewId(1234)), FromStr::from_str("view-id-1234"));
    }
    #[test]
    fn display() {
        assert_eq!("view-id-1".to_string(), ViewId(1).to_string());
        assert_eq!("view-id-1234".to_string(), ViewId(1234).to_string());
    }
    #[test]
    fn serialize() {
        assert_eq!(json!("view-id-1"), to_value(&ViewId(1)).unwrap());
    }
    #[test]
    fn deserialize() {
        assert_eq!(ViewId(1), from_str("\"view-id-1\"").unwrap());
    }
}
