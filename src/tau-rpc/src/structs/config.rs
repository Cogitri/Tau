use crate::ViewId;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigChanged {
    pub view_id: ViewId,
    pub changes: ConfigChanges,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ConfigChanges {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_face: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_ending: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_search_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tab_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translate_tabs_to_spaces: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_wrap: Option<bool>,
}
