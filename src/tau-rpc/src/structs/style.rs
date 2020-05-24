use serde::Deserialize;

#[derive(Default, Deserialize, Debug, PartialEq, Clone)]
pub struct Style {
    pub id: u64,
    pub fg_color: Option<u32>,
    pub bg_color: Option<u32>,
    pub weight: Option<u32>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
}
