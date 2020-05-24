use crate::ViewId;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub running: bool,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailablePlugins {
    pub view_id: ViewId,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginStarted {
    pub view_id: ViewId,
    pub plugin: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PluginStopped {
    pub view_id: ViewId,
    pub plugin: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateCmds {
    pub cmds: Vec<String>,
    pub plugin: String,
    pub view_id: ViewId,
}
