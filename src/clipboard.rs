use glib::CallbackGuard;
use glib::translate::*;
use glib_sys::gpointer;
use gtk::Clipboard;
use gtk_sys as ffi;
use libc::{self, c_char};
use std::mem::transmute;



pub trait ClipboardRequest {
    fn request_text<F: Fn(&Clipboard, String) + 'static>(&self, callback: F);
}

impl ClipboardRequest for Clipboard {
    fn request_text<F: Fn(&Clipboard, String) + 'static>(&self, callback: F)
    {
        unsafe {
            let trampoline = trampoline_request_text as *mut libc::c_void;
            ffi::gtk_clipboard_request_text(self.to_glib_none().0, Some(transmute(trampoline)), into_raw_request_text(callback));
        }
    }
}

fn into_raw_request_text<F: Fn(&Clipboard, String) + 'static>(func: F) -> gpointer {
    let func: Box<Box<Fn(&Clipboard, String) + 'static>> =
        Box::new(Box::new(func));
    Box::into_raw(func) as gpointer
}

unsafe extern "C" fn trampoline_request_text(clipboard: *mut ffi::GtkClipboard, text: *const c_char, data: gpointer) {
    let _guard = CallbackGuard::new();
    let f: &&(Fn(&Clipboard, String) + 'static) = transmute(data);
    f(&from_glib_borrow(clipboard), from_glib_none(text));
}