use cairo;
use syntect::highlighting::Color as SynColor;
use syntect::highlighting::{ThemeSettings, UnderlineOption};

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const WHITE: Color = Color{r: 1.0, g: 1.0, b: 1.0, a: 1.0};
    pub const BLACK: Color = Color{r: 0.0, g: 0.0, b: 0.0, a: 1.0};

    pub fn make_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color{r: r as f64/255.0, g: g as f64/255.0, b: b as f64/255.0, a: a as f64/255.0}
    }
    pub fn make_u32_argb(c: u32) -> Color {
        Color::make_u8(
            (c >> 16) as u8,
            (c >> 8) as u8,
            c as u8,
            (c >> 24) as u8,
        )
    }
}

#[inline]
pub fn set_source_color(cr: &cairo::Context, c: Color) {
    cr.set_source_rgba(c.r, c.g, c.b, c.a);
}

#[derive(Clone, Debug)]
pub struct Theme {
    /// Text color for the view.
    pub foreground: Color,
    /// Backgound color of the view.
    pub background: Color,
    /// Color of the caret.
    pub caret: Color,
    /// Color of the line the caret is in.
    pub line_highlight: Option<Color>,

    /// Background color of regions matching the current search.
    pub find_highlight: Color,
    pub find_highlight_foreground: Option<Color>,

    /// Background color of the gutter.
    pub gutter: Color,
    /// The color of the line numbers in the gutter.
    pub gutter_foreground: Color,

    /// The background color of selections.
    pub selection: Color,
    /// text color of the selection regions.
    pub selection_foreground: Color,
    /// Color of the selection regions border.
    pub selection_border: Option<Color>,
    pub inactive_selection: Option<Color>,
    pub inactive_selection_foreground: Option<Color>,

    /// The color of the shadow used when a text area can be horizontally scrolled.
    pub shadow: Color,
}

impl Default for Theme {
    fn default() -> Theme {
        Theme {
            foreground: Color::make_u8(50, 50, 50, 255),
            background: Color::WHITE,
            caret: Color::make_u8(50, 50, 50, 255),
            line_highlight: Some(Color::make_u8(245, 245, 245, 255)),
            find_highlight: Color::BLACK,
            find_highlight_foreground: Some(Color::make_u8(50, 50, 50, 255)),
            gutter: Color::WHITE,
            gutter_foreground: Color::make_u8(179, 179, 179, 255),
            selection: Color::make_u8(248, 238, 199, 255),
            selection_foreground: Color::BLACK,
            selection_border: Some(Color::WHITE),
            inactive_selection: None,
            inactive_selection_foreground: None,
            shadow: Color::WHITE,
        }        
    }
}
