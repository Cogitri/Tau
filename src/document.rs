use std::cmp::max;

use cairo::{Context, FontExtents};

use gtk::prelude::*;
use gtk::*;

use linecache::*;
use error::*;
use structs::*;

const CURSOR_WIDTH: f64 = 2.0;

#[derive(Debug)]
pub struct Document {
    pub line_cache: LineCache,
    pub drawing_area: Layout,
    pub first_line: u64,
    pub last_line: u64,
    //font_ext: FontExtents,
    font_height: f64,
    font_width: f64,
    font_ascent: f64,
    font_descent: f64,
}

impl Document {
    pub fn new(da: Layout) -> Document {
        Document {
            line_cache: LineCache::new(),
            drawing_area: da,
            first_line: 0u64,
            last_line: 0u64,
            font_height: 1.0,
            font_width: 1.0,
            font_ascent: 1.0,
            font_descent: 1.0,
            // font_ext: FontExtents {
            //     ascent: 1f64,
            //     descent: 1f64,
            //     height: 1f64,
            //     max_x_advance: 1f64,
            //     max_y_advance: 1f64,
            // }
        }
    }
}

impl Document {
    pub fn handle_update(&mut self, ops: &[UpdateOp]) -> Result<(), GxiError> {
        self.line_cache.handle_update(ops)?;
        self.drawing_area.queue_draw();
        Ok(())
    }

    pub fn handle_draw(&mut self, cr: &Context) -> (u64, u64, Vec<(u64, u64)>) {
        let da_width = self.drawing_area.get_allocated_width();
        let da_height = self.drawing_area.get_allocated_height();
        //let hadj = self.drawing_area.get_hadjustment().unwrap();
        let num_lines = self.line_cache.height();

        //debug!("Drawing");
        cr.select_font_face("Mono", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
        cr.set_font_size(12.0);
        let font_extents = cr.font_extents();

        //self.font_ext = font_extents;
        self.font_height = font_extents.height;
        self.font_width = font_extents.max_x_advance;
        self.font_ascent = font_extents.ascent;
        self.font_descent = font_extents.descent;

        // Set vertical adjustment
        let vadj = self.drawing_area.get_vadjustment().unwrap();
        vadj.set_lower(0f64);
        let all_text_height = num_lines as f64 * font_extents.height;
        if all_text_height > da_height as f64 {
            vadj.set_upper(all_text_height);
        } else {
            vadj.set_upper(da_height as f64);
        }
        vadj.set_page_size(da_height as f64);
        vadj.value_changed();
        self.drawing_area.set_vadjustment(Some(&vadj));

        // Set horizontal adjustment
        let hadj = self.drawing_area.get_hadjustment().unwrap();
        hadj.set_lower(0f64);
        let all_text_width = self.line_cache.width() as f64 * font_extents.max_x_advance;
        if all_text_width > da_width as f64 {
            hadj.set_upper(all_text_width);
        } else {
            hadj.set_upper(da_width as f64);
        }
        hadj.set_page_size(da_width as f64);
        hadj.value_changed();
        self.drawing_area.set_hadjustment(Some(&hadj));

        let first_line = (vadj.get_value() / font_extents.height) as u64;
        let last_line = ((vadj.get_value() + da_height as f64) / font_extents.height) as u64 + 1;

        let missing = self.line_cache.get_missing(first_line, last_line);
        // if !missing.is_empty() {
        //     return missing;
        // }

        // Draw background
        cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
        cr.rectangle(0.0, 0.0, da_width as f64, da_height  as f64);
        cr.fill();

        // Highlight cursor lines
        for i in 0..self.line_cache.height() {
            cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
            if let Some(line) = self.line_cache.get(i) {

                if !line.cursor.is_empty() {
                    cr.set_source_rgba(0.23, 0.23, 0.23, 1.0);
                    cr.rectangle(0f64,
                        font_extents.height*((i+1) as f64) - font_extents.ascent - vadj.get_value(),
                        da_width as f64,
                        font_extents.ascent + font_extents.descent);
                    cr.fill();
                }
            // } else {
            //     cr.set_source_rgba(1.0, 0.0, 0.0, 1.0);
            //     cr.rectangle(0f64,
            //         font_extents.height*((i+1) as f64) - font_extents.ascent - vadj.get_value(),
            //         da_width as f64,
            //         font_extents.ascent + font_extents.descent);
            //     cr.fill();
            }
        }

        // Draw styles
        for i in 0..self.line_cache.height() {
            cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
            if let Some(line) = self.line_cache.get(i) {

                let mut si = 0;
                loop {
                    let s1 = line.styles.get(si);
                    si+=1;
                    let s2 = line.styles.get(si);
                    si+=1;
                    let s3 = line.styles.get(si);
                    si+=1;
                    if let (Some(s), Some(f), Some(_)) = (s1,s2,s3) {
                        cr.set_source_rgba(0.35, 0.35, 0.35, 1.0);
                        cr.rectangle(font_extents.max_x_advance* (*s as f64) - hadj.get_value(),
                            font_extents.height*((i+1) as f64) - font_extents.ascent - vadj.get_value(),
                            font_extents.max_x_advance* (*f as f64),
                            font_extents.ascent + font_extents.descent);
                        cr.fill();
                    } else {
                        break;
                    }
                }
            }
        }

        // Draw text
        for i in 0..self.line_cache.height() {
            cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
            if let Some(line) = self.line_cache.get(i) {
                cr.move_to(0.0 - hadj.get_value(),
                    font_extents.height*((i+1) as f64) - vadj.get_value()
                );

                // Don't draw the newline
                let line_view = if line.text.ends_with('\n') {
                    &line.text[0..line.text.len()-1]
                } else {
                    &line.text
                };
                cr.show_text(line_view);

                for c in &line.cursor {
                    cr.set_source_rgba(0.5, 0.5, 1.0, 1.0);
                    cr.rectangle(font_extents.max_x_advance* (*c as f64) - hadj.get_value(),
                        font_extents.height*((i+1) as f64) - font_extents.ascent - vadj.get_value(),
                        CURSOR_WIDTH,
                        font_extents.ascent + font_extents.descent);
                    cr.fill();
                }
            }
        }

        (first_line, last_line, missing)
    }

    pub fn scroll_to(&mut self, line: u64, col: u64) {
        {
            let da_height = self.drawing_area.get_allocated_height() as f64;
            let cur_top = self.font_height*((line+1) as f64) - self.font_ascent;
            let cur_bottom = cur_top + self.font_ascent + self.font_descent;
            let vadj = self.drawing_area.get_vadjustment().unwrap();
            if cur_top < vadj.get_value() {
                vadj.set_value(cur_top);
                vadj.value_changed();
            } else if cur_bottom > vadj.get_value() + da_height {
                if cur_bottom > vadj.get_upper() {
                    vadj.set_upper(cur_bottom);
                }
                vadj.set_value(cur_bottom - da_height);
                vadj.value_changed();
            }
            debug!("vadj={:?}", vadj);
            self.drawing_area.set_vadjustment(Some(&vadj));
        }

        {
            let da_width = self.drawing_area.get_allocated_width() as f64;
            let cur_left = self.font_width*(col as f64) - self.font_ascent;
            let cur_right = cur_left + self.font_width*2.0;
            let hadj = self.drawing_area.get_hadjustment().unwrap();
            if cur_left < hadj.get_value() {
                hadj.set_value(cur_left);
                hadj.value_changed();
            } else if cur_right > hadj.get_value() + da_width {
                if cur_right > hadj.get_upper() {
                    hadj.set_upper(cur_right);
                }
                hadj.set_value(cur_right - da_width);
                hadj.value_changed();
            }
            debug!("hadj={:?}", hadj);
            self.drawing_area.set_hadjustment(Some(&hadj));
        }
    }
}
