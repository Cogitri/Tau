use serde_derive::*;

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

/// RGBA colour, these numbers come directly from the theme so
/// for now you might have to do your own colour space conversion if you are outputting
/// a different colour space from the theme. This can be a problem because some Sublime
/// themes use sRGB and some don't. This is specified in an attribute syntect doesn't parse yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    /// Red component
    pub r: u8,
    /// Green component
    pub g: u8,
    /// Blue component
    pub b: u8,
    /// Alpha component
    pub a: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnderlineOption {
    None,
    Underline,
    StippledUnderline,
    SquigglyUnderline,
}

/// Various properties meant to be used to style a text editor.
/// Basically all the styles that aren't directly applied to text like selection colour.
/// Use this to make your editor UI match the highlighted text.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThemeSettings {
    /// The default color for text.
    pub foreground: Option<Color>,
    /// The default backgound color of the view.
    pub background: Option<Color>,
    /// Color of the caret.
    pub caret: Option<Color>,
    /// Color of the line the caret is in.
    /// Only used when the `higlight_line` setting is set to `true`.
    pub line_highlight: Option<Color>,

    /// The color to use for the squiggly underline drawn under misspelled words.
    pub misspelling: Option<Color>,
    /// The color of the border drawn around the viewport area of the minimap.
    /// Only used when the `draw_minimap_border` setting is enabled.
    pub minimap_border: Option<Color>,
    /// A color made available for use by the theme.
    pub accent: Option<Color>,
    /// CSS passed to popups.
    pub popup_css: Option<String>,
    /// CSS passed to phantoms.
    pub phantom_css: Option<String>,

    /// Color of bracketed sections of text when the caret is in a bracketed section.
    /// Only applied when the `match_brackets` setting is set to `true`.
    pub bracket_contents_foreground: Option<Color>,
    /// Controls certain options when the caret is in a bracket section.
    /// Only applied when the `match_brackets` setting is set to `true`.
    pub bracket_contents_options: Option<UnderlineOption>,
    /// Foreground color of the brackets when the caret is next to a bracket.
    /// Only applied when the `match_brackets` setting is set to `true`.
    pub brackets_foreground: Option<Color>,
    /// Background color of the brackets when the caret is next to a bracket.
    /// Only applied when the `match_brackets` setting is set to `true`.
    pub brackets_background: Option<Color>,
    /// Controls certain options when the caret is next to a bracket.
    /// Only applied when the match_brackets setting is set to `true`.
    pub brackets_options: Option<UnderlineOption>,

    /// Color of tags when the caret is next to a tag.
    /// Only used when the `match_tags` setting is set to `true`.
    pub tags_foreground: Option<Color>,
    /// Controls certain options when the caret is next to a tag.
    /// Only applied when the match_tags setting is set to `true`.
    pub tags_options: Option<UnderlineOption>,

    /// The border color for "other" matches.
    pub highlight: Option<Color>,
    /// Background color of regions matching the current search.
    pub find_highlight: Option<Color>,
    /// Text color of regions matching the current search.
    pub find_highlight_foreground: Option<Color>,

    /// Background color of the gutter.
    pub gutter: Option<Color>,
    /// Foreground color of the gutter.
    pub gutter_foreground: Option<Color>,

    /// The background color of selected text.
    pub selection: Option<Color>,
    /// A color that will override the scope-based text color of the selection.
    pub selection_foreground: Option<Color>,

    /// Deprecated!
    ///
    /// This property is not part of the recognized tmTheme format. It may be
    /// removed in a future release.
    pub selection_background: Option<Color>,

    /// Color of the selection regions border.
    pub selection_border: Option<Color>,
    /// The background color of a selection in a view that is not currently focused.
    pub inactive_selection: Option<Color>,
    /// A color that will override the scope-based text color of the selection
    /// in a view that is not currently focused.
    pub inactive_selection_foreground: Option<Color>,

    /// Color of the guides displayed to indicate nesting levels.
    pub guide: Option<Color>,
    /// Color of the guide lined up with the caret.
    /// Only applied if the `indent_guide_options` setting is set to `draw_active`.
    pub active_guide: Option<Color>,
    /// Color of the current guide’s parent guide level.
    /// Only used if the `indent_guide_options` setting is set to `draw_active`.
    pub stack_guide: Option<Color>,

    /// Foreground color for regions added via `sublime.add_regions()`
    /// with the `sublime.DRAW_OUTLINED` flag added.
    ///
    /// Deprecated!
    /// This setting does not exist in any available documentation.
    /// Use is discouraged, and it may be removed in a future release.
    pub highlight_foreground: Option<Color>,

    /// The color of the shadow used when a text area can be horizontally scrolled.
    pub shadow: Option<Color>,
}
