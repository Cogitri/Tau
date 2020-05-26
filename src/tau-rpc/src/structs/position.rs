// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct Position(pub u64, pub u64);

#[test]
fn deserialize_ok() {
    use serde_json;

    let s = r#"[12, 1]"#;
    let deserialized: Result<Position, _> = serde_json::from_str(s);
    assert_eq!(deserialized.unwrap(), Position(12, 1));
}
