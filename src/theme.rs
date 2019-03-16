use serde_derive::*;
use syntect::highlighting::Color;

/// Pango doesn't use rgb but values ranging fom 0 to 65535.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PangoColor {
    pub r: u16,
    pub g: u16,
    pub b: u16,
}

impl From<Color> for PangoColor {
    /// Convert rgb's 0-255 values to Pango's 0-65535
    fn from(c: Color) -> Self {
        Self {
            r: u16::from(c.r) << 8,
            g: u16::from(c.g) << 8,
            b: u16::from(c.b) << 8,
        }
    }
}

/// A LineStyle represents different styling options for a line
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct LineStyle {
    /// 32-bit RGBA value which sets the font color
    pub fg_color: Option<u32>,
    /// 32-bit RGBA value which sets the background of the Pango layout
    pub bg_color: Option<u32>,
    /// 100..900, default 400
    pub weight: Option<u32>,
    /// default false
    pub italic: Option<bool>,
    /// default false
    pub underline: Option<bool>,
}

/// Helper function for cairo::Context::set_source_rgba which sets a sane default if the Color is None
pub fn set_source_color(cr: &cairo::Context, color: Option<Color>) {
    if let Some(c) = color {
        cr.set_source_rgba(
            f64::from(c.r) / 255.0,
            f64::from(c.g) / 255.0,
            f64::from(c.b) / 255.0,
            f64::from(c.a) / 255.0,
        );
    } else {
        // Hopefully a sane default.
        cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
    }
}

/// Explode an u32 into its individual RGBA values
pub fn color_from_u32(c: u32) -> Color {
    Color {
        r: (c >> 16) as u8,
        g: (c >> 8) as u8,
        b: c as u8,
        a: (c >> 24) as u8,
    }
}

/// Implode an Color with its individual RGBA values into an u32
pub fn u32_from_color(c: Color) -> u32 {
    (u32::from(c.a) << 24) | (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)
}
