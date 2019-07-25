pub use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SyntaxParams {
    pub domain: Domain,
    pub changes: Changes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Domain {
    pub syntax: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Changes {
    #[serde(default)]
    pub translate_tabs_to_spaces: bool,
    #[serde(default = "default_tab_size")]
    pub tab_size: u32,
}

fn default_tab_size() -> u32 {
    4
}
