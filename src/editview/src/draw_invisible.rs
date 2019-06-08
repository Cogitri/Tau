use cairo::Context;
use log::trace;
use std::f64::consts::PI;

#[derive(Debug)]
pub struct Rectangle {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
}

impl Rectangle {
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
}
