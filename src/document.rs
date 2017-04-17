use cairo::Context;

use gtk::prelude::*;
use gtk::*;

use linecache::*;
use error::*;
use structs::*;

#[derive(Debug)]
pub struct Document {
    pub line_cache: LineCache,
    pub drawing_area: DrawingArea,
}

impl Document {
    pub fn new(da: DrawingArea) -> Document {
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
        cr.select_font_face("Mono", ::cairo::enums::FontSlant::Normal, ::cairo::enums::FontWeight::Normal);
        cr.set_font_size(12.0);
        let font_extents = cr.font_extents();

        // Draw background
        let da_width = self.drawing_area.get_allocated_width();
        let da_height = self.drawing_area.get_allocated_height();
        cr.set_source_rgba(0.2, 0.2, 0.2, 1.0);
        cr.rectangle(0.0, 0.0, da_width as f64, da_height  as f64);
        cr.fill();

        //debug!("lc {:?}", self.line_cache);
        for i in 0..self.line_cache.len() {
            cr.set_source_rgba(0.8, 0.8, 0.8, 1.0);
            if let Some(line) = self.line_cache.get(i) {
                cr.move_to(0.0, font_extents.height*((i+1) as f64));
                cr.show_text(&line.text);

                for c in &line.cursor {
                    cr.set_source_rgba(0.5, 0.5, 1.0, 1.0);
                    cr.rectangle(font_extents.max_x_advance* (*c as f64), font_extents.height*((i+1) as f64) - font_extents.ascent, 2.0, font_extents.ascent + font_extents.descent);
                    cr.fill();
                }
            }
        }
    }
}
