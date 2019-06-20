use gettextrs::gettext;
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
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font set")));
        let metrics = fontset
            .get_metrics()
            .unwrap_or_else(|| panic!("{}", &gettext("Failed to load Pango font metrics")));

        let layout = pango::Layout::new(pango_ctx);
        layout.set_text("a");
        let (_, log_extents) = layout.get_extents();
        debug!("{}: {:?}", gettext("Pango font size"), log_extents);

        let font_height = f64::from(log_extents.height) / f64::from(pango::SCALE);
        let font_width = f64::from(log_extents.width) / f64::from(pango::SCALE);
        let font_ascent = f64::from(metrics.get_ascent()) / f64::from(pango::SCALE);
        let font_descent = f64::from(metrics.get_descent()) / f64::from(pango::SCALE);

        debug!(
            "{}: {} {} {} {}",
            gettext("Font metrics"),
            font_width,
            font_height,
            font_ascent,
            font_descent
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
    use ::fontconfig::fontconfig;
    use pango::prelude::*;
    use pango::FontDescription;
    use std::ffi::CString;

    #[test]
    fn test_font_metrics() {
        unsafe {
            let fonts_dir = CString::new("tests/assets").unwrap();
            let ret = fontconfig::FcConfigAppFontAddDir(
                fontconfig::FcConfigGetCurrent(),
                fonts_dir.as_ptr() as *const u8,
            );
            if ret != 1 {
                panic!("Couldn't set fontconfig dir!");
            }
        }

        let font_map = pangocairo::FontMap::get_default().unwrap();
        let pango_ctx = font_map.create_context().unwrap();

        assert_eq!(
            Font::new(&pango_ctx, FontDescription::from_string("Vera 12")),
            Font {
                font_ascent: 15.0,
                font_descent: 4.0,
                font_height: 19.0,
                font_width: 9.0,
                font_desc: FontDescription::from_string("Vera 12"),
            }
        )
    }

    #[test]
    fn test_bold_font_metrics() {
        unsafe {
            let fonts_dir = CString::new("tests/assets").unwrap();
            let ret = fontconfig::FcConfigAppFontAddDir(
                fontconfig::FcConfigGetCurrent(),
                fonts_dir.as_ptr() as *const u8,
            );
            if ret != 1 {
                panic!("Couldn't set fontconfig dir!");
            }
        }

        let font_map = pangocairo::FontMap::get_default().unwrap();
        let pango_ctx = font_map.create_context().unwrap();

        assert_eq!(
            Font::new(&pango_ctx, FontDescription::from_string("VeraBold 12")),
            Font {
                font_ascent: 15.0,
                font_descent: 4.0,
                font_height: 19.0,
                font_width: 9.0,
                font_desc: FontDescription::from_string("VeraBold 12"),
            }
        )
    }
}
