use std::cmp::max;

use cairo::Context;

use gtk::prelude::*;
use gtk::*;

use linecache::*;
use error::*;
use structs::*;

#[derive(Debug)]
pub struct Document {
    pub line_cache: LineCache,
    pub drawing_area: Layout,
}

impl Document {
    pub fn new(da: Layout) -> Document {
        Document {
            line_cache: LineCache::new(),
            drawing_area: da,
        }
    }
}

impl Document {
    pub fn handle_update(&mut self, ops: &[UpdateOp]) -> Result<(), GxiError> {
        self.line_cache.handle_update(ops)?;
        self.drawing_area.queue_draw();
        Ok(())
    }

    pub fn handle_draw(&mut self, cr: &Context) {
        let da_width = self.drawing_area.get_allocated_width();
        let da_height = self.drawing_area.get_allocated_height();
        let vadj = self.drawing_area.get_vadjustment().unwrap();
        //let hadj = self.drawing_area.get_hadjustment().unwrap();
        let num_lines = self.line_cache.len();

        //debug!("Drawing");
        cr.select_font_face("Mono", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
        cr.set_font_size(12.0);
        let font_extents = cr.font_extents();

        // Set vertical adjustment
        vadj.set_lower(0f64);
        let all_text_height = num_lines as f64 * font_extents.height;
        if all_text_height > da_height as f64 {
            vadj.set_upper(all_text_height);
        } else {
            vadj.set_upper(da_height as f64);
        }
        vadj.set_page_size(da_height as f64);
        self.drawing_area.set_vadjustment(Some(&vadj));

        // Draw background
        cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
        cr.rectangle(0.0, 0.0, da_width as f64, da_height  as f64);
        cr.fill();

        // Highlight cursor lines
        for i in 0..self.line_cache.len() {
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
            }
        }

        // Draw styles
        for i in 0..self.line_cache.len() {
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
                        cr.rectangle(font_extents.max_x_advance* (*s as f64),
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
        for i in 0..self.line_cache.len() {
            cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
            if let Some(line) = self.line_cache.get(i) {
                cr.move_to(0.0, font_extents.height*((i+1) as f64) - vadj.get_value());

                // Don't draw the newline
                let line_view = if line.text.ends_with('\n') {
                    &line.text[0..line.text.len()-1]
                } else {
                    &line.text
                };
                cr.show_text(line_view);

                for c in &line.cursor {
                    cr.set_source_rgba(0.5, 0.5, 1.0, 1.0);
                    cr.rectangle(font_extents.max_x_advance* (*c as f64),
                        font_extents.height*((i+1) as f64) - font_extents.ascent - vadj.get_value(),
                        2.0,
                        font_extents.ascent + font_extents.descent);
                    cr.fill();
                }
            }
        }

    }
}
