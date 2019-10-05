use log::debug;
use pango::{FontDescription, FontsetExt, Language};

/// The `Font` Struct holds all information about the font used in the `EditView` for the editing area
/// or the interface font (used for the linecount)
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Font {
    pub font_height: f64,
    pub font_width: f64,
    pub font_ascent: f64,
    pub font_descent: f64,
    pub font_desc: FontDescription,
}

impl Font {
    /// Create a new `Font` Struct. If `FontDescription` doesn't match any of the fonts installed on
    /// the system Pango will choose the closest match, or the default if nothing matches.
    pub fn new(pango_ctx: &pango::Context, font_desc: FontDescription) -> Self {
        pango_ctx.set_font_description(&font_desc);
        // FIXME: Just use en-US lang here, otherwise FontMetrics may be different (as in font_ascent/
        // font_descent being larger to account for the language's special signs), which breaks cursor positioning.
        let fontset = pango_ctx
            .load_fontset(&font_desc, &Language::from_string("en-US"))
            .expect("Failed to load Pango font set");
        let metrics = fontset
            .get_metrics()
            .expect("Failed to load Pango font metrics");

        let layout = pango::Layout::new(pango_ctx);
        layout.set_text("a");
        let (_, log_extents) = layout.get_extents();
        debug!("Pango font size: {:?}", log_extents);

        let font_height = f64::from(log_extents.height) / f64::from(pango::SCALE);
        let font_width = f64::from(log_extents.width) / f64::from(pango::SCALE);
        let font_ascent = f64::from(metrics.get_ascent()) / f64::from(pango::SCALE);
        let font_descent = f64::from(metrics.get_descent()) / f64::from(pango::SCALE);

        debug!(
            "Font Metrics: Width: '{}'; Height: '{}'; Ascent: '{}'; Descent: '{}'",
            font_width, font_height, font_ascent, font_descent
        );

        Self {
            font_height,
            font_width,
            font_ascent,
            font_descent,
            font_desc,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::fonts::Font;
    use pango::prelude::*;
    use pango::FontDescription;

    #[test]
    fn test_font_metrics() {
        let font_map = pangocairo::FontMap::get_default().unwrap();

        let fonts: Vec<String> = font_map
            .list_families()
            .iter()
            .filter_map(|s| s.get_name())
            .map(|s| s.as_str().to_string())
            .collect();

        if !fonts.contains(&"Source Code Pro".to_string()) {
            panic!("Couldn't find font 'Source Code Pro' required for test!");
        }

        let pango_ctx = font_map.create_context().unwrap();

        let mut font = Font::new(
            &pango_ctx,
            FontDescription::from_string("Source Code Pro 12"),
        );

        font.font_ascent = font.font_ascent.ceil();
        font.font_descent = font.font_descent.ceil();

        assert_eq!(
            font,
            Font {
                font_ascent: 16.0,
                font_descent: 5.0,
                font_height: 21.0,
                font_width: 10.0,
                font_desc: FontDescription::from_string("Source Code Pro 12"),
            }
        )
    }

    #[test]
    fn test_bold_font_metrics() {
        let font_map = pangocairo::FontMap::get_default().unwrap();

        let fonts: Vec<String> = font_map
            .list_families()
            .iter()
            .filter_map(|s| s.get_name())
            .map(|s| s.as_str().to_string())
            .collect();

        if !fonts.contains(&"Source Code Pro".to_string()) {
            panic!("Couldn't find font 'Source Code Pro' required for test!");
        }

        let pango_ctx = font_map.create_context().unwrap();

        let mut font = Font::new(
            &pango_ctx,
            FontDescription::from_string("Source Code Pro Bold 12"),
        );

        font.font_ascent = font.font_ascent.ceil();
        font.font_descent = font.font_descent.ceil();

        assert_eq!(
            font,
            Font {
                font_ascent: 16.0,
                font_descent: 5.0,
                font_height: 21.0,
                font_width: 10.0,
                font_desc: FontDescription::from_string("Source Code Pro Bold 12"),
            }
        )
    }
}
