// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailableThemes {
    pub themes: Vec<String>,
}

pub type ThemeSettings = ::syntect::highlighting::ThemeSettings;

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeChanged {
    pub name: String,
    pub theme: ThemeSettings,
}
