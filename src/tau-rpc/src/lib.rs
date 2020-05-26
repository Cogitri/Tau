// Based, in part, on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// Copyright (c) 2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

mod client;
mod errors;
mod message;
mod structs;

pub use crate::client::{Callback, Client};
pub use crate::message::Message;
pub use crate::structs::{
    Alert, AvailableLanguages, AvailablePlugins, AvailableThemes, ConfigChanged, ConfigChanges,
    FindStatus, LanguageChanged, Line, MeasureWidth, ModifySelection, Operation, OperationType,
    PluginStarted, PluginStopped, Position, Query, ReplaceStatus, RpcOperations, ScrollTo, Status,
    Style, StyleDef, ThemeChanged, ThemeSettings, Update, UpdateCmds, ViewId,
};
