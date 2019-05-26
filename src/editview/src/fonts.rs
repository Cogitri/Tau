use gettextrs::gettext;
use log::debug;
use pango::{ContextExt, FontDescription, FontsetExt, Language, LayoutExt};

/// The `Font` Struct holds all information about the font used in the `EditView` for the editing area
/// or the interface font (used for the linecount)
pub struct Font {
    pub font_height: f64,
    pub font_width: f64,
    pub font_ascent: f64,
    pub font_descent: f64,
    pub font_desc: FontDescription,
}

impl Font {
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
