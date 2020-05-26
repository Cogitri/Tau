// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    pub msg: String,
}
