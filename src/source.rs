use std::mem;
use std::ptr;

use glib::Source;
use glib::translate::from_glib_none;
use glib_sys::{GSource, GSourceFunc, GSourceFuncs, gboolean, g_source_new};
use libc;

pub trait SourceFuncs {
    fn check(&self) -> bool {
        false
    }

    fn dispatch(&self) -> bool;
    fn prepare(&self) -> (bool, Option<u32>);
}

struct SourceData<T> {
    _source: GSource,
    funcs: Box<GSourceFuncs>,
    data: T,
}

pub fn new_source<T: SourceFuncs>(data: T) -> Source {
    unsafe {
        let mut funcs: GSourceFuncs = mem::zeroed();
        funcs.prepare = Some(prepare::<T>);
        funcs.check = Some(check::<T>);
        funcs.dispatch = Some(dispatch::<T>);
        funcs.finalize = Some(finalize::<T>);
        let mut funcs = Box::new(funcs);
        let source = g_source_new(&mut *funcs, mem::size_of::<SourceData<T>>() as u32);
        ptr::write(&mut (*(source as *mut SourceData<T>)).data, data);
        ptr::write(&mut (*(source as *mut SourceData<T>)).funcs, funcs);
        from_glib_none(source)
    }
}

unsafe extern "C" fn check<T: SourceFuncs>(source: *mut GSource) -> gboolean {
    let object = source as *mut SourceData<T>;
    bool_to_int((*object).data.check())
}

unsafe extern "C" fn dispatch<T: SourceFuncs>(source: *mut GSource, _callback: GSourceFunc, _user_data: *mut libc::c_void)
    -> gboolean
{
    let object = source as *mut SourceData<T>;
    bool_to_int((*object).data.dispatch())
}

unsafe extern "C" fn finalize<T: SourceFuncs>(source: *mut GSource) {
    // TODO: needs a bomb to abort on panic
    let source = source as *mut SourceData<T>;
    ptr::read(&(*source).funcs);
    ptr::read(&(*source).data);
}

extern "C" fn prepare<T: SourceFuncs>(source: *mut GSource, timeout: *mut libc::c_int) -> gboolean {
    let object = source as *mut SourceData<T>;
    let (result, source_timeout) = unsafe { (*object).data.prepare() };
    if let Some(source_timeout) = source_timeout {
        unsafe { *timeout = source_timeout as i32; }
    }
    bool_to_int(result)
}

fn bool_to_int(boolean: bool) -> gboolean {
    if boolean {
        1
    }
    else {
        0
    }
}