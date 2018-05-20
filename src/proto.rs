
#[derive(Clone, PartialEq, Eq, Default, Hash, Debug, Serialize, Deserialize)]
/// A mergeable style. All values except priority are optional.
///
/// Note: A `None` value represents the absense of preference; in the case of
/// boolean options, `Some(false)` means that this style will override a lower
/// priority value in the same field.
pub struct Style {
    pub id: usize,
    /// 32-bit RGBA value
    pub fg_color: Option<u32>,
    /// 32-bit RGBA value, default 0
    pub bg_color: Option<u32>,
    /// 100..900, default 400
    pub weight: Option<u32>,
    /// default false
    pub italic: Option<bool>, 
    /// default false
    pub underline: Option<bool>,
}