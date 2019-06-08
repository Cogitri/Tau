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

pub fn draw_space(cr: &Context, rect: &Rectangle) {
    trace!("Drawing space at: {:?}", rect);

    let x = rect.x;
    let y = rect.y + rect.height * (2.0 / 3.0);

    let width = rect.width;

    cr.save();
    cr.move_to(x + width * 0.5, y);
    cr.arc(x + width * 0.5, y, 0.8, 0.0, 2. * PI);
    cr.stroke();
    cr.restore();
}

pub fn draw_tab(cr: &Context, rect: &Rectangle) {
    trace!("Drawing tab at: {:?}", rect);

    let x = rect.x;
    let y = rect.y + rect.height * (2.0 / 3.0);

    let width = rect.width;
    let height = rect.height;

    cr.save();
    cr.move_to(x + width * 1.0 / 8.0, y);
    cr.move_to(x + width * 1.0 / 8.0, y);
    cr.rel_line_to(width * 6.0 / 8.0, 0.0);
    cr.rel_line_to(-height * 1.0 / 4.0, -height * 1.0 / 4.0);
    cr.rel_move_to(height * 1.0 / 4.0, height * 1.0 / 4.0);
    cr.rel_line_to(-height * 1.0 / 4.0, height * 1.0 / 4.0);
    cr.stroke();
    cr.restore();
}
