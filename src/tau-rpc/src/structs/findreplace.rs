use super::view::ViewId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Query {
    pub id: u64,
    pub chars: Option<String>,
    pub case_sensitive: Option<bool>,
    pub is_regex: Option<bool>,
    pub whole_words: Option<bool>,
    pub matches: u64,
    pub lines: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FindStatus {
    pub view_id: ViewId,
    pub queries: Vec<Query>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub chars: String,
    pub preserve_case: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ReplaceStatus {
    pub view_id: ViewId,
    pub status: Status,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_findstatus() {
        use crate::structs::findreplace::{FindStatus, Query};
        use serde_json;
        use std::str::FromStr;

        let s = r#"{"view_id": "view-id-1", "queries": [{"id": 1, "chars": "a", "case_sensitive": false, "is_regex": false, "whole_words": true, "matches": 6, "lines": [1, 3, 3, 6]}]}"#;
        let deserialized: Result<FindStatus, _> = serde_json::from_str(s);
        let find_status = FindStatus {
            view_id: FromStr::from_str("view-id-1").unwrap(),
            queries: vec![Query {
                id: 1,
                chars: Some("a".to_string()),
                case_sensitive: Some(false),
                is_regex: Some(false),
                whole_words: Some(true),
                matches: 6,
                lines: vec![1, 3, 3, 6],
            }],
        };

        assert_eq!(deserialized.unwrap(), find_status);
    }

    #[test]
    fn test_replacestatus() {
        use crate::structs::findreplace::{ReplaceStatus, Status};
        use serde_json;
        use std::str::FromStr;

        let s = r#"{"view_id": "view-id-1", "status": {"chars": "abc", "preserve_case": false}}"#;
        let deserialized: Result<ReplaceStatus, _> = serde_json::from_str(s);
        let replace_status = ReplaceStatus {
            view_id: FromStr::from_str("view-id-1").unwrap(),
            status: Status {
                chars: "abc".to_string(),
                preserve_case: Some(false),
            },
        };

        assert_eq!(deserialized.unwrap(), replace_status);
    }
}
