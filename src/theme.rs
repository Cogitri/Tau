use crate::proto;
use cairo;

#[derive(Clone, Copy, Debug)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };

    pub fn from_u8s(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }
    pub fn from_ts_proto(c: proto::Color) -> Color {
        Color::from_u8s(c.r, c.g, c.b, c.a)
    }
    pub fn from_u32_argb(c: u32) -> Color {
        Color::from_u8s((c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8)
    }

    pub fn r_u16(&self) -> u16 {
        u16::from(self.r) << 8
    }
    pub fn g_u16(&self) -> u16 {
        u16::from(self.g) << 8
    }
    pub fn b_u16(&self) -> u16 {
        u16::from(self.b) << 8
    }
}

#[inline]
pub fn set_source_color(cr: &cairo::Context, c: Color) {
    cr.set_source_rgba(
        f64::from(c.r) / 255.0,
        f64::from(c.g) / 255.0,
        f64::from(c.b) / 255.0,
        f64::from(c.a) / 255.0,
    );
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// 32-bit RGBA value
    pub fg_color: Option<Color>,
    /// 32-bit RGBA value, default 0
    pub bg_color: Option<Color>,
    /// 100..900, default 400
    pub weight: Option<u32>,
    /// default false
    pub italic: Option<bool>,
    /// default false
    pub underline: Option<bool>,
}

impl Style {
    pub fn from_proto(style: &proto::Style) -> Style {
        Style {
            fg_color: style.fg_color.map(Color::from_u32_argb),
            bg_color: style.bg_color.map(Color::from_u32_argb),
            weight: style.weight,
            italic: style.italic,
            underline: style.underline,
        }
    }
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
            foreground: Color::from_u8s(50, 50, 50, 255),
            background: Color::WHITE,
            caret: Color::from_u8s(50, 50, 50, 255),
            line_highlight: Some(Color::from_u8s(245, 245, 245, 255)),
            find_highlight: Color::BLACK,
            find_highlight_foreground: Some(Color::from_u8s(50, 50, 50, 255)),
            gutter: Color::WHITE,
            gutter_foreground: Color::from_u8s(179, 179, 179, 255),
            selection: Color::from_u8s(248, 238, 199, 255),
            selection_foreground: Color::BLACK,
            selection_border: Some(Color::WHITE),
            inactive_selection: None,
            inactive_selection_foreground: None,
            shadow: Color::WHITE,
        }
    }
}

impl Theme {
    pub fn from_proto(theme_settings: &proto::ThemeSettings) -> Theme {
        let mut theme: Theme = Default::default();

        if let Some(foreground) = theme_settings.foreground {
            theme.foreground = Color::from_ts_proto(foreground);
        }
        if let Some(background) = theme_settings.background {
            theme.background = Color::from_ts_proto(background);
        }
        if let Some(caret) = theme_settings.caret {
            theme.caret = Color::from_ts_proto(caret);
        }
        theme.line_highlight = theme_settings.line_highlight.map(Color::from_ts_proto);
        if let Some(find_highlight) = theme_settings.find_highlight {
            theme.find_highlight = Color::from_ts_proto(find_highlight);
        }
        theme.find_highlight_foreground = theme_settings
            .find_highlight_foreground
            .map(Color::from_ts_proto);
        if let Some(gutter) = theme_settings.gutter {
            theme.gutter = Color::from_ts_proto(gutter);
        }
        if let Some(gutter_foreground) = theme_settings.gutter_foreground {
            theme.gutter_foreground = Color::from_ts_proto(gutter_foreground);
        }
        if let Some(selection) = theme_settings.selection {
            theme.selection = Color::from_ts_proto(selection);
        }
        if let Some(selection_foreground) = theme_settings.selection_foreground {
            theme.selection_foreground = Color::from_ts_proto(selection_foreground);
        }
        theme.selection_border = theme_settings.selection_border.map(Color::from_ts_proto);
        theme.inactive_selection = theme_settings.inactive_selection.map(Color::from_ts_proto);
        theme.inactive_selection_foreground = theme_settings
            .inactive_selection_foreground
            .map(Color::from_ts_proto);
        if let Some(shadow) = theme_settings.shadow {
            theme.shadow = Color::from_ts_proto(shadow);
        }

        theme
    }
}
