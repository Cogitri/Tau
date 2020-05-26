// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifySelection {
    None,
    Set,
    Add,
    AddRemoveCurrent,
}

#[test]
fn serialize_ok() {
    use serde_json;

    assert_eq!(
        "\"none\"",
        serde_json::to_string(&ModifySelection::None).unwrap()
    );
    assert_eq!(
        "\"set\"",
        serde_json::to_string(&ModifySelection::Set).unwrap()
    );
    assert_eq!(
        "\"add\"",
        serde_json::to_string(&ModifySelection::Add).unwrap()
    );
    assert_eq!(
        "\"add_remove_current\"",
        serde_json::to_string(&ModifySelection::AddRemoveCurrent).unwrap()
    );
}
