// Copyright (C) 2017-2018 Brian Vincent <brainn@gmail.com>
// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: GPL-3.0-or-later

use syntect::highlighting::Color;

/// Pango doesn't use rgb but values ranging fom 0 to 65535.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PangoColor {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

impl From<Color> for PangoColor {
    /// Convert rgb's 0-255 values to Pango's 0-65535
    fn from(c: Color) -> Self {
        Self {
            r: u16::from(c.r) << 8,
            g: u16::from(c.g) << 8,
            b: u16::from(c.b) << 8,
            a: u16::from(c.a) << 8,
        }
    }
}

/// Helper function for `cairo::Context::set_source_rgba` which sets a sane default if the Color is None
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

/// Used for the right hand margin to make the margin a bit darker than the original background
pub fn set_margin_source_color(cr: &cairo::Context, color: Option<Color>) {
    let source_color = if let Some(c) = color {
        // Primitive check to see if the theme is light (if so, subtract more for an actually
        // noticeable different) or dark (then subtract less to not get too dark).
        // FIXME: This should be handled in Xi, see https://github.com/xi-editor/xi-editor/issues/1125
        if (c.r > 200) && (c.g > 200) & (c.b > 200) {
            Some(Color {
                r: c.r - 10,
                g: c.g - 10,
                b: c.b - 10,
                a: c.a,
            })
        } else {
            // Use saturating_sub here to make sure we don't overflow
            Some(Color {
                r: c.r.saturating_sub(5),
                g: c.g.saturating_sub(5),
                b: c.b.saturating_sub(5),
                a: c.a,
            })
        }
    } else {
        None
    };

    set_source_color(cr, source_color);
}
/// Explode an u32 into its individual RGBA values
pub const fn color_from_u32(c: u32) -> Color {
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
