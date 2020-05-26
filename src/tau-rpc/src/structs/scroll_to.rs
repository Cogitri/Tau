// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

use crate::ViewId;
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct ScrollTo {
    pub line: u64,
    #[serde(rename = "col")]
    pub column: u64,
    pub view_id: ViewId,
}

#[test]
fn deserialize_ok() {
    use serde_json;
    use std::str::FromStr;

    let s = r#"{"col":18,"line":0,"view_id":"view-id-1"}"#;
    let deserialized: Result<ScrollTo, _> = serde_json::from_str(s);
    let scroll_to = ScrollTo {
        line: 0,
        column: 18,
        view_id: FromStr::from_str("view-id-1").unwrap(),
    };
    assert_eq!(deserialized.unwrap(), scroll_to);
}
