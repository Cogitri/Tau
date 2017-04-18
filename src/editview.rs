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

use gio;
use gio_sys as gio_ffi;
use gtk_sys as gtk_ffi;
use gtk::Widget;
use glib::translate::FromGlibPtrNone;
use glib::object::Downcast;

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
//         //assert_initialized_main_thread!();
//         unsafe {
//             Widget::from_glib_none(gtk_ffi::gtk_drawing_area_new()).downcast_unchecked()
//         }
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
