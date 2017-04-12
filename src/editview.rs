#[macro_use]
use glib::object::IsA;
use glib::wrapper::Wrapper;
use gtk;
use gtk::prelude::*;
use gtk::*;

// pub struct Blah {}
// pub fn new() -> Blah {
//
// }
// glib_wrapper! {
//     pub struct EditView(Object<::editview::Blah>): Bin, Container, Widget, Scrollable;
//
//     match fn {
//         get_type => || new(),
//     }
// }

use gtk_sys as gtk_ffi;
use gio;
use gio_sys as gio_ffi;


// glib_wrapper! {
//     pub struct EditView(Object<gtk_ffi::GtkDrawingArea>): [
//         gtk::Scrollable => gtk_ffi::GtkScrollable,
//     ];
//
//     match fn {
//         get_type => || gtk_ffi::gtk_application_get_type(),
//     }
// }
//
// impl EditView {
//     pub fn new() -> EditView {
//         EditView{}
//     }
// }

// struct EditView {
//
// }
// impl StaticType for EditView {
//
// }
// impl IsA<Object> for EditView {
//
// }
//
// impl IsA<Widget> for EditView {
//
// }
