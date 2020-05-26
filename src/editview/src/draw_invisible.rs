// Copyright (C) 2019-2020 Rasmus Thomsen <oss@cogitri.dev>
// SPDX-License-Identifier: MIT

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
        cr.arc(x + width * 0.5, y, self.height / 10., 0.0, 2.0 * PI);
        cr.fill();
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
        cr.rel_line_to(-height * 1.0 / 4.0, -height * 3.0 / 16.0);
        cr.rel_move_to(height * 1.0 / 4.0, height * 3.0 / 16.0);
        cr.rel_line_to(-height * 1.0 / 4.0, height * 3.0 / 16.0);
        cr.stroke();
        cr.restore();
    }

    /// Locate the positions of spaces/tabs in form of a `Vec<Rectangle>` in a `pango::Layout`.
    pub fn from_layout_index<'a>(
        index: impl Iterator<Item = i32> + 'a,
        layout: &'a pango::Layout,
    ) -> impl Iterator<Item = Self> + 'a {
        index
            .map(move |index| layout.index_to_pos(index))
            .map(|pos| Self {
                x: (pos.x / pango::SCALE).into(),
                y: (pos.y / pango::SCALE).into(),
                width: (pos.width / pango::SCALE).into(),
                height: (pos.height / pango::SCALE).into(),
            })
    }
}

pub mod spaces {
    use std::ops::Range;

    /// Get all spaces in a string
    pub fn all(text: &str) -> impl Iterator<Item = i32> + '_ {
        text.bytes()
            .zip(0..)
            .flat_map(|(ch, i)| if ch == b' ' { Some(i) } else { None })
    }

    /// Get all spaces in a string from `start_index` to `start_index + length`
    pub fn all_from(text: &str, start_index: u64, length: u64) -> impl Iterator<Item = i32> + '_ {
        text.bytes()
            .skip(start_index as usize)
            .take(length as usize)
            .enumerate()
            .filter_map(move |(num, ch)| {
                if ch == b' ' {
                    Some((num as u64 + start_index) as i32)
                } else {
                    None
                }
            })
    }

    /// Get leading spaces in a string
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::spaces;
    ///
    /// assert_eq!(spaces::leading("  example"), 0..2)
    /// ```
    pub fn leading(text: &str) -> Range<i32> {
        let last_space = text
            .bytes()
            .position(|ch| ch != b' ')
            .unwrap_or_else(|| text.len());
        0..last_space as i32
    }

    /// Get trailing spaces in a string. Be mindful that this _does not_ remove newline chars ('\n').
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::spaces;
    ///
    /// assert_eq!(spaces::trailing("example  "), 7..9)
    /// ```
    ///
    /// ```
    /// use editview::draw_invisible::spaces;
    ///
    /// assert_eq!(spaces::trailing("example  \n"), 10..10)
    /// ```
    pub fn trailing(text: &str) -> Range<i32> {
        let first_space = text.bytes().rposition(|ch| ch != b' ').map_or(0, |x| x + 1);
        first_space as _..text.len() as _
    }
}

pub mod tabs {
    use std::ops::Range;

    /// Get all tabs in a string
    pub fn all(text: &str) -> impl Iterator<Item = i32> + '_ {
        text.bytes()
            .zip(0..)
            .flat_map(|(ch, i)| if ch == b'\t' { Some(i) } else { None })
    }

    /// Get all tabs in a string from `start_index` to `start_index + length`
    pub fn all_from(text: &str, start_index: u64, length: u64) -> impl Iterator<Item = i32> + '_ {
        text.bytes()
            .skip(start_index as usize)
            .take(length as usize)
            .enumerate()
            .filter_map(move |(num, ch)| {
                if ch == b'\t' {
                    Some((num as u64 + start_index) as i32)
                } else {
                    None
                }
            })
    }

    /// Get leading tabs in a string
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::tabs;
    ///
    /// assert_eq!(tabs::leading("\t\texample"), 0..2)
    /// ```
    pub fn leading(text: &str) -> Range<i32> {
        let last_tab = text
            .bytes()
            .position(|ch| ch != b'\t')
            .unwrap_or_else(|| text.len());
        0..last_tab as i32
    }

    /// Get trailing tabs in a string. Be mindful that this _does not_ remove newline chars ('\n').
    ///
    /// # Example
    ///
    /// ```
    /// use editview::draw_invisible::tabs;
    ///
    /// assert_eq!(tabs::trailing("example\t\t"), 7..9)
    /// ```
    ///
    ///```
    /// use editview::draw_invisible::tabs;
    ///
    /// assert_eq!(tabs::trailing("example\t\t\n"), 10..10)
    ///```
    pub fn trailing(text: &str) -> Range<i32> {
        let first_tab = text
            .bytes()
            .rposition(|ch| ch != b'\t')
            .map_or(0, |x| x + 1);
        first_tab as _..text.len() as _
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Misc strings to test. Try to use special chars here
    const EXM1: &str = "Traga tinta em trinta taças";
    const EXM3: &str = "\t\tÜberall ganz\tviele Tabs!\t\t";
    const EXM4: &str = "  Überall\tganz    \tviele Spaces und\tTabs!";

    #[test]
    fn spaces_special_char() {
        assert_eq!(spaces::trailing(EXM1).count(), 0);
        assert_eq!(spaces::all(EXM1).count(), 4);
        assert_eq!(spaces::leading(EXM4).count(), 2);
    }

    #[test]
    fn tabs_special_char() {
        assert_eq!(tabs::trailing(EXM1).count(), 0);
        assert_eq!(tabs::all(EXM3).count(), 5);
        assert_eq!(tabs::leading(EXM3).count(), 2);
    }

    #[test]
    fn mixed_tabs_spaces() {
        assert_eq!((tabs::all(EXM4).count(), spaces::all(EXM4).count()), (3, 8),);
        assert_eq!((tabs::all(EXM3).count(), spaces::all(EXM3).count()), (5, 2),);
    }

    #[test]
    fn trailing_tabs() {
        assert_eq!(tabs::trailing(EXM3).count(), 2);
    }
}
