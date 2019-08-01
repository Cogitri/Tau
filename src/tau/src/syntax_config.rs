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
    pub translate_tabs_to_spaces: Option<bool>,
    pub tab_size: Option<u32>,
}
