use cairo::Context;
use log::trace;
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
/// A Rectangle representing a tab's/space's position on the screen
pub struct Rectangle {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
}

impl Rectangle {
    /// Draw a circle representing a space at the position of the `Rectangle`
    pub fn draw_space(&self, cr: &Context) {
        trace!("Drawing space at: {:?}", self);

        let x = self.x;
        let y = self.y + self.height * 0.5;

        let width = self.width;

        cr.save();
        cr.move_to(x + width * 0.5, y);
        cr.arc(x + width * 0.5, y, 1.0, 0.0, 2.0 * PI);
        cr.stroke();
        cr.restore();
    }

    /// Draw an arrow representing a tab at the position of the `Rectangle`
    pub fn draw_tab(&self, cr: &Context) {
        trace!("Drawing tab at: {:?}", self);

        let x = self.x;
        let y = self.y + self.height * 0.5;

        let width = self.width;
        let height = self.height;

        cr.save();
        cr.move_to(x + width * 1.0 / 8.0, y);
        cr.rel_line_to(width * 6.0 / 8.0, 0.0);
        cr.rel_line_to(-height * 1.0 / 4.0, -height * 1.0 / 4.0);
        cr.rel_move_to(height * 1.0 / 4.0, height * 1.0 / 4.0);
        cr.rel_line_to(-height * 1.0 / 4.0, height * 1.0 / 4.0);
        cr.stroke();
        cr.restore();
    }

    /// Locate the positions of spaces/tabs in form of a `Vec<Rectangle>` in a `pango::Layout`.
    pub fn from_layout_index(index: Vec<i32>, layout: &pango::Layout) -> Vec<Self> {
        let mut vec = Vec::new();

        for index in index.iter() {
            let pos = layout.index_to_pos(*index);
            let rect = Self {
                x: (pos.x / pango::SCALE).into(),
                y: (pos.y / pango::SCALE).into(),
                width: (pos.width / pango::SCALE).into(),
                height: (pos.height / pango::SCALE).into(),
            };
            vec.push(rect);
        }

        vec
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Spaces {
    pub index: Vec<i32>,
}

impl Spaces {
    /// Get all spaces in a string
    pub fn all(text: &str) -> Self {
        let mut space_index = Vec::new();
        for (i, char) in text.bytes().enumerate() {
            if char == b" "[0] {
                space_index.push(i as i32)
            }
        }

        Self { index: space_index }
    }

    /// Get leading spaces in a string
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::Spaces;
    ///
    /// assert_eq!(Spaces::leading("  example"), Spaces { index: vec![0,1] })
    /// ```
    pub fn leading(text: &str) -> Self {
        let mut space_index = Vec::new();
        let last_char = text.replace("\t", "a").trim_start().len();
        let (_, spaces) = text.split_at(last_char);
        for (i, _) in spaces.chars().enumerate() {
            space_index.push(i as i32)
        }

        Self { index: space_index }
    }

    /// Get trailing spaces in a string
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::Spaces;
    ///
    /// assert_eq!(Spaces::trailing("example  "), Spaces { index: vec![7,8] })
    /// ```
    pub fn trailing(text: &str) -> Self {
        let mut space_index = Vec::new();
        let last_char = text.replace("\t", "a").trim_end().len();
        let (text_without_spaces, spaces) = text.split_at(last_char);
        let char_count = text_without_spaces.bytes().count();
        for (i, _) in spaces.chars().enumerate() {
            space_index.push((i + char_count) as i32)
        }

        Self { index: space_index }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Tabs {
    pub index: Vec<i32>,
}

impl Tabs {
    /// Get all tabs in your string
    pub fn all(text: &str) -> Self {
        let mut tab_index = Vec::new();
        for (i, char) in text.bytes().enumerate() {
            if char == b"\t"[0] {
                tab_index.push(i as i32)
            }
        }

        Self { index: tab_index }
    }

    /// Get leading tabs in a string
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::Tabs;
    ///
    /// assert_eq!(Tabs::leading("\t\texample"), Tabs { index: vec![0,1] })
    /// ```
    pub fn leading(text: &str) -> Self {
        let mut tab_index = Vec::new();

        let last_char = text.replace(" ", "a").trim_start().len();
        let (_, tabs) = text.split_at(last_char);
        for (i, _) in tabs.bytes().enumerate() {
            tab_index.push((i) as i32)
        }

        Self { index: tab_index }
    }

    /// Get leading tabs in a string
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::Tabs;
    ///
    /// assert_eq!(Tabs::trailing("example\t\t"), Tabs { index: vec![7,8] })
    /// ```
    pub fn trailing(text: &str) -> Self {
        let mut tab_index = Vec::new();

        let last_char = text.replace(" ", "a").trim_end().len();
        let (text_without_tabs, tabs) = text.split_at(last_char);
        let char_count = text_without_tabs.bytes().count();
        for (i, _) in tabs.bytes().enumerate() {
            tab_index.push((i + char_count) as i32)
        }

        Self { index: tab_index }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ::fontconfig::fontconfig;
    use pango::{FontDescription, FontMapExt};
    use std::ffi::CString;

    // Misc strings to test. Try to use special chars here
    const EXM1: &str = "Traga tinta em trinta taças";
    const EXM2: &str = "Völlig übertrieben";
    const EXM3: &str = "\t\tÜberall ganz\tviele Tabs!\t\t";
    const EXM4: &str = "  Überall\tganz    \tviele Spaces und\tTabs!";

    #[test]
    fn spaces_special_char() {
        assert_eq!(Spaces::trailing(EXM1).index.len(), 0);
        assert_eq!(Spaces::all(EXM1).index.len(), 4);
        assert_eq!(Spaces::leading(EXM4).index.len(), 2);
    }

    #[test]
    fn tabs_special_char() {
        assert_eq!(Tabs::trailing(EXM1).index.len(), 0);
        assert_eq!(Tabs::all(EXM3).index.len(), 5);
        assert_eq!(Tabs::leading(EXM3).index.len(), 2);
    }

    #[test]
    fn mixed_tabs_spaces() {
        assert_eq!(
            (Tabs::all(EXM4).index.len(), Spaces::all(EXM4).index.len()),
            (3, 8),
        );
        assert_eq!(
            (Tabs::all(EXM3).index.len(), Spaces::all(EXM3).index.len()),
            (5, 2),
        );
    }
}
