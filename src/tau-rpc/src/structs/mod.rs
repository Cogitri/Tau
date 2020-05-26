// Based on xrl (https://github.com/xi-frontend/xrl), which is:
// Copyright (c) 2017 Corentin Henry
// SPDX-License-Identifier: MIT

mod alert;
mod config;
mod findreplace;
mod language;
mod line;
mod modifyselection;
mod operation;
mod plugins;
mod position;
mod scroll_to;
mod style;
mod theme;
mod update;
mod view;

pub use self::alert::Alert;
pub use self::config::ConfigChanged;
pub use self::config::ConfigChanges;
pub use self::findreplace::{FindStatus, Query, ReplaceStatus, Status};
pub use self::language::{AvailableLanguages, LanguageChanged};
pub use self::line::{Line, StyleDef};
pub use self::modifyselection::ModifySelection;
pub use self::operation::{Operation, OperationType};
pub use self::plugins::AvailablePlugins;
pub use self::plugins::Plugin;
pub use self::plugins::PluginStarted;
pub use self::plugins::PluginStopped;
pub use self::plugins::UpdateCmds;
pub use self::position::Position;
pub use self::scroll_to::ScrollTo;
pub use self::style::Style;
pub use self::theme::{AvailableThemes, ThemeChanged, ThemeSettings};
pub use self::update::Update;
pub use self::view::{MeasureWidth, ViewId};

/// Represents all possible RPC messages recieved from xi-core.
#[derive(Debug)]
pub enum RpcOperations {
    Update(Update),
    ScrollTo(ScrollTo),
    DefStyle(Style),
    AvailablePlugins(AvailablePlugins),
    UpdateCmds(UpdateCmds),
    PluginStarted(PluginStarted),
    PluginStopped(PluginStopped),
    ConfigChanged(ConfigChanged),
    ThemeChanged(ThemeChanged),
    Alert(Alert),
    AvailableThemes(AvailableThemes),
    FindStatus(FindStatus),
    ReplaceStatus(ReplaceStatus),
    AvailableLanguages(AvailableLanguages),
    LanguageChanged(LanguageChanged),
    MeasureWidth(MeasureWidth),
}
