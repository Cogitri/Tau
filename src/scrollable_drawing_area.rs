/**
 * This file is a monstrosity.  It was generated from the gnome-class project.
 * Due to various reasons, I could not use the gnome-class macro, or the
 * glib object wrapper macro.  One day, I hope to be able to use gnome-class.
 * 
 * The entire point of this file is to create a custom widget that extends
 * DrawingArea and implements Scrollable.  This was the only way I could find
 * to get nice GTK auto-hiding scrollbars.
 * 
 * This probably needs cleanup.
 */
use std;

use glib::translate::*;
use gobject_sys;
use gtk::{DrawingArea, Scrollable};

use gtk_sys::{GtkDrawingArea, GtkWidget};

use std::cell::Cell;

/********************************************************************************/

pub mod ScrollableDrawingAreaMod {
    #![allow(non_snake_case)]
    extern crate glib;
    extern crate glib_sys as glib_ffi;
    extern crate gobject_sys as gobject_ffi;
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use glib::object::Downcast;
    use glib::IsA;
    use std::mem;
    use std::ptr;

    pub struct ScrollableDrawingArea(
        ::glib::object::ObjectRef,
        ::std::marker::PhantomData<imp::ScrollableDrawingAreaFfi>,
    );
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::std::clone::Clone for ScrollableDrawingArea {
        #[inline]
        fn clone(&self) -> ScrollableDrawingArea {
            match *self {
                ScrollableDrawingArea(ref __self_0_0, ref __self_0_1) => ScrollableDrawingArea(
                    ::std::clone::Clone::clone(&(*__self_0_0)),
                    ::std::clone::Clone::clone(&(*__self_0_1)),
                ),
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::std::fmt::Debug for ScrollableDrawingArea {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            match *self {
                ScrollableDrawingArea(ref __self_0_0, ref __self_0_1) => {
                    let mut debug_trait_builder = f.debug_tuple("ScrollableDrawingArea");
                    let _ = debug_trait_builder.field(&&(*__self_0_0));
                    let _ = debug_trait_builder.field(&&(*__self_0_1));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::std::hash::Hash for ScrollableDrawingArea {
        fn hash<__H: ::std::hash::Hasher>(&self, state: &mut __H) -> () {
            match *self {
                ScrollableDrawingArea(ref __self_0_0, ref __self_0_1) => {
                    ::std::hash::Hash::hash(&(*__self_0_0), state);
                    ::std::hash::Hash::hash(&(*__self_0_1), state)
                }
            }
        }
    }
    #[doc(hidden)]
    impl Into<::glib::object::ObjectRef> for ScrollableDrawingArea {
        fn into(self) -> ::glib::object::ObjectRef {
            self.0
        }
    }
    #[doc(hidden)]
    impl ::glib::wrapper::UnsafeFrom<::glib::object::ObjectRef> for ScrollableDrawingArea {
        unsafe fn from(t: ::glib::object::ObjectRef) -> Self {
            ScrollableDrawingArea(t, ::std::marker::PhantomData)
        }
    }
    #[doc(hidden)]
    impl ::glib::translate::GlibPtrDefault for ScrollableDrawingArea {
        type GlibType = *mut imp::ScrollableDrawingAreaFfi;
    }
    #[doc(hidden)]
    impl ::glib::wrapper::Wrapper for ScrollableDrawingArea {
        type GlibType = imp::ScrollableDrawingAreaFfi;
        type GlibClassType = imp::ScrollableDrawingAreaClass;
    }
    #[doc(hidden)]
    impl<'a> ::glib::translate::ToGlibPtr<'a, *const imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        type Storage = <::glib::object::ObjectRef as ::glib::translate::ToGlibPtr<
            'a,
            *mut ::glib::object::GObject,
        >>::Storage;
        #[inline]
        fn to_glib_none(
            &'a self,
        ) -> ::glib::translate::Stash<'a, *const imp::ScrollableDrawingAreaFfi, Self> {
            let stash = self.0.to_glib_none();
            ::glib::translate::Stash(stash.0 as *const _, stash.1)
        }
        #[inline]
        fn to_glib_full(&self) -> *const imp::ScrollableDrawingAreaFfi {
            self.0.to_glib_full() as *const _
        }
    }
    #[doc(hidden)]
    impl<'a> ::glib::translate::ToGlibPtr<'a, *mut imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        type Storage = <::glib::object::ObjectRef as ::glib::translate::ToGlibPtr<
            'a,
            *mut ::glib::object::GObject,
        >>::Storage;
        #[inline]
        fn to_glib_none(
            &'a self,
        ) -> ::glib::translate::Stash<'a, *mut imp::ScrollableDrawingAreaFfi, Self> {
            let stash = self.0.to_glib_none();
            ::glib::translate::Stash(stash.0 as *mut _, stash.1)
        }
        #[inline]
        fn to_glib_full(&self) -> *mut imp::ScrollableDrawingAreaFfi {
            self.0.to_glib_full() as *mut _
        }
    }
    #[doc(hidden)]
    impl<'a>
        ::glib::translate::ToGlibContainerFromSlice<'a, *mut *mut imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        type Storage = (
            Vec<Stash<'a, *mut imp::ScrollableDrawingAreaFfi, ScrollableDrawingArea>>,
            Option<Vec<*mut imp::ScrollableDrawingAreaFfi>>,
        );
        fn to_glib_none_from_slice(
            t: &'a [ScrollableDrawingArea],
        ) -> (*mut *mut imp::ScrollableDrawingAreaFfi, Self::Storage) {
            let v: Vec<_> = t.iter().map(|s| s.to_glib_none()).collect();
            let mut v_ptr: Vec<_> = v.iter().map(|s| s.0).collect();
            v_ptr.push(ptr::null_mut() as *mut imp::ScrollableDrawingAreaFfi);
            (
                v_ptr.as_ptr() as *mut *mut imp::ScrollableDrawingAreaFfi,
                (v, Some(v_ptr)),
            )
        }
        fn to_glib_container_from_slice(
            t: &'a [ScrollableDrawingArea],
        ) -> (*mut *mut imp::ScrollableDrawingAreaFfi, Self::Storage) {
            let v: Vec<_> = t.iter().map(|s| s.to_glib_none()).collect();
            let v_ptr = unsafe {
                let v_ptr = glib_ffi::g_malloc0(
                    mem::size_of::<*mut imp::ScrollableDrawingAreaFfi>() * (t.len() + 1),
                ) as *mut *mut imp::ScrollableDrawingAreaFfi;
                for (i, s) in v.iter().enumerate() {
                    ptr::write(v_ptr.offset(i as isize), s.0);
                }
                v_ptr
            };
            (v_ptr, (v, None))
        }
        fn to_glib_full_from_slice(
            t: &[ScrollableDrawingArea],
        ) -> *mut *mut imp::ScrollableDrawingAreaFfi {
            unsafe {
                let v_ptr = glib_ffi::g_malloc0(
                    mem::size_of::<*mut imp::ScrollableDrawingAreaFfi>() * (t.len() + 1),
                ) as *mut *mut imp::ScrollableDrawingAreaFfi;
                for (i, s) in t.iter().enumerate() {
                    ptr::write(v_ptr.offset(i as isize), s.to_glib_full());
                }
                v_ptr
            }
        }
    }
    #[doc(hidden)]
    impl<'a>
        ::glib::translate::ToGlibContainerFromSlice<'a, *const *mut imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        type Storage = (
            Vec<Stash<'a, *mut imp::ScrollableDrawingAreaFfi, ScrollableDrawingArea>>,
            Option<Vec<*mut imp::ScrollableDrawingAreaFfi>>,
        );
        fn to_glib_none_from_slice(
            t: &'a [ScrollableDrawingArea],
        ) -> (*const *mut imp::ScrollableDrawingAreaFfi, Self::Storage) {
            let (ptr, stash) = ::glib::translate::ToGlibContainerFromSlice::<
                'a,
                *mut *mut imp::ScrollableDrawingAreaFfi,
            >::to_glib_none_from_slice(t);
            (ptr as *const *mut imp::ScrollableDrawingAreaFfi, stash)
        }
        fn to_glib_container_from_slice(
            _: &'a [ScrollableDrawingArea],
        ) -> (*const *mut imp::ScrollableDrawingAreaFfi, Self::Storage) {
            {
                panic!("not yet implemented")
            }
        }
        fn to_glib_full_from_slice(
            _: &[ScrollableDrawingArea],
        ) -> *const *mut imp::ScrollableDrawingAreaFfi {
            {
                panic!("not yet implemented")
            }
        }
    }
    #[doc(hidden)]
    impl ::glib::translate::FromGlibPtrNone<*mut imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        #[inline]
        unsafe fn from_glib_none(ptr: *mut imp::ScrollableDrawingAreaFfi) -> Self {
            if true {
                if !::glib::types::instance_of::<Self>(ptr as *const _) {
                    {
                        panic!(
                            "assertion failed: ::glib::types::instance_of::<Self>(ptr as *const _)"
                        )
                    }
                };
            };
            ScrollableDrawingArea(
                ::glib::translate::from_glib_none(ptr as *mut _),
                ::std::marker::PhantomData,
            )
        }
    }
    #[doc(hidden)]
    impl ::glib::translate::FromGlibPtrNone<*const imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        #[inline]
        unsafe fn from_glib_none(ptr: *const imp::ScrollableDrawingAreaFfi) -> Self {
            if true {
                if !::glib::types::instance_of::<Self>(ptr as *const _) {
                    {
                        panic!(
                            "assertion failed: ::glib::types::instance_of::<Self>(ptr as *const _)"
                        )
                    }
                };
            };
            ScrollableDrawingArea(
                ::glib::translate::from_glib_none(ptr as *mut _),
                ::std::marker::PhantomData,
            )
        }
    }
    #[doc(hidden)]
    impl ::glib::translate::FromGlibPtrFull<*mut imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        #[inline]
        unsafe fn from_glib_full(ptr: *mut imp::ScrollableDrawingAreaFfi) -> Self {
            if true {
                if !::glib::types::instance_of::<Self>(ptr as *const _) {
                    {
                        panic!(
                            "assertion failed: ::glib::types::instance_of::<Self>(ptr as *const _)"
                        )
                    }
                };
            };
            ScrollableDrawingArea(
                ::glib::translate::from_glib_full(ptr as *mut _),
                ::std::marker::PhantomData,
            )
        }
    }
    #[doc(hidden)]
    impl ::glib::translate::FromGlibPtrBorrow<*mut imp::ScrollableDrawingAreaFfi>
        for ScrollableDrawingArea
    {
        #[inline]
        unsafe fn from_glib_borrow(ptr: *mut imp::ScrollableDrawingAreaFfi) -> Self {
            if true {
                if !::glib::types::instance_of::<Self>(ptr as *const _) {
                    {
                        panic!(
                            "assertion failed: ::glib::types::instance_of::<Self>(ptr as *const _)"
                        )
                    }
                };
            };
            ScrollableDrawingArea(
                ::glib::translate::from_glib_borrow(ptr as *mut _),
                ::std::marker::PhantomData,
            )
        }
    }
    #[doc(hidden)]
    impl
        ::glib::translate::FromGlibContainerAsVec<
            *mut imp::ScrollableDrawingAreaFfi,
            *mut *mut imp::ScrollableDrawingAreaFfi,
        > for ScrollableDrawingArea
    {
        unsafe fn from_glib_none_num_as_vec(
            ptr: *mut *mut imp::ScrollableDrawingAreaFfi,
            num: usize,
        ) -> Vec<Self> {
            if num == 0 || ptr.is_null() {
                return Vec::new();
            }
            let mut res = Vec::with_capacity(num);
            for i in 0..num {
                res.push(::glib::translate::from_glib_none(ptr::read(
                    ptr.offset(i as isize),
                )));
            }
            res
        }
        unsafe fn from_glib_container_num_as_vec(
            ptr: *mut *mut imp::ScrollableDrawingAreaFfi,
            num: usize,
        ) -> Vec<Self> {
            let res =
                ::glib::translate::FromGlibContainerAsVec::from_glib_none_num_as_vec(ptr, num);
            glib_ffi::g_free(ptr as *mut _);
            res
        }
        unsafe fn from_glib_full_num_as_vec(
            ptr: *mut *mut imp::ScrollableDrawingAreaFfi,
            num: usize,
        ) -> Vec<Self> {
            if num == 0 || ptr.is_null() {
                return Vec::new();
            }
            let mut res = Vec::with_capacity(num);
            for i in 0..num {
                res.push(::glib::translate::from_glib_full(ptr::read(
                    ptr.offset(i as isize),
                )));
            }
            glib_ffi::g_free(ptr as *mut _);
            res
        }
    }
    #[doc(hidden)]
    impl
        ::glib::translate::FromGlibPtrArrayContainerAsVec<
            *mut imp::ScrollableDrawingAreaFfi,
            *mut *mut imp::ScrollableDrawingAreaFfi,
        > for ScrollableDrawingArea
    {
        unsafe fn from_glib_none_as_vec(ptr: *mut *mut imp::ScrollableDrawingAreaFfi) -> Vec<Self> {
            ::glib::translate::FromGlibContainerAsVec::from_glib_none_num_as_vec(
                ptr,
                ::glib::translate::c_ptr_array_len(ptr),
            )
        }
        unsafe fn from_glib_container_as_vec(
            ptr: *mut *mut imp::ScrollableDrawingAreaFfi,
        ) -> Vec<Self> {
            ::glib::translate::FromGlibContainerAsVec::from_glib_container_num_as_vec(
                ptr,
                ::glib::translate::c_ptr_array_len(ptr),
            )
        }
        unsafe fn from_glib_full_as_vec(ptr: *mut *mut imp::ScrollableDrawingAreaFfi) -> Vec<Self> {
            ::glib::translate::FromGlibContainerAsVec::from_glib_full_num_as_vec(
                ptr,
                ::glib::translate::c_ptr_array_len(ptr),
            )
        }
    }
    #[doc(hidden)]
    impl
        ::glib::translate::FromGlibContainerAsVec<
            *mut imp::ScrollableDrawingAreaFfi,
            *const *mut imp::ScrollableDrawingAreaFfi,
        > for ScrollableDrawingArea
    {
        unsafe fn from_glib_none_num_as_vec(
            ptr: *const *mut imp::ScrollableDrawingAreaFfi,
            num: usize,
        ) -> Vec<Self> {
            ::glib::translate::FromGlibContainerAsVec::from_glib_none_num_as_vec(
                ptr as *mut *mut _,
                num,
            )
        }
        unsafe fn from_glib_container_num_as_vec(
            _: *const *mut imp::ScrollableDrawingAreaFfi,
            _: usize,
        ) -> Vec<Self> {
            {
                unimplemented!()
            }
        }
        unsafe fn from_glib_full_num_as_vec(
            _: *const *mut imp::ScrollableDrawingAreaFfi,
            _: usize,
        ) -> Vec<Self> {
            {
                unimplemented!()
            }
        }
    }
    #[doc(hidden)]
    impl
        ::glib::translate::FromGlibPtrArrayContainerAsVec<
            *mut imp::ScrollableDrawingAreaFfi,
            *const *mut imp::ScrollableDrawingAreaFfi,
        > for ScrollableDrawingArea
    {
        unsafe fn from_glib_none_as_vec(
            ptr: *const *mut imp::ScrollableDrawingAreaFfi,
        ) -> Vec<Self> {
            ::glib::translate::FromGlibPtrArrayContainerAsVec::from_glib_none_as_vec(
                ptr as *mut *mut _,
            )
        }
        unsafe fn from_glib_container_as_vec(
            _: *const *mut imp::ScrollableDrawingAreaFfi,
        ) -> Vec<Self> {
            {
                unimplemented!()
            }
        }
        unsafe fn from_glib_full_as_vec(_: *const *mut imp::ScrollableDrawingAreaFfi) -> Vec<Self> {
            {
                unimplemented!()
            }
        }
    }
    impl ::glib::types::StaticType for ScrollableDrawingArea {
        fn static_type() -> ::glib::types::Type {
            unsafe { ::glib::translate::from_glib(imp::scrollable_drawing_area_get_type()) }
        }
    }
    impl<T: ::glib::object::IsA<::glib::object::Object>> ::std::cmp::PartialEq<T>
        for ScrollableDrawingArea
    {
        #[inline]
        fn eq(&self, other: &T) -> bool {
            use glib::translate::ToGlibPtr;
            self.0.to_glib_none().0 == other.to_glib_none().0
        }
    }
    #[doc(hidden)]
    impl<'a> ::glib::value::FromValueOptional<'a> for ScrollableDrawingArea {
        unsafe fn from_value_optional(value: &::glib::value::Value) -> Option<Self> {
            Option::<ScrollableDrawingArea>::from_glib_full(gobject_ffi::g_value_dup_object(
                value.to_glib_none().0,
            )
                as *mut imp::ScrollableDrawingAreaFfi)
                .map(|o| {
                ::glib::object::Downcast::downcast_unchecked(o)
            })
        }
    }
    #[doc(hidden)]
    impl ::glib::value::SetValue for ScrollableDrawingArea {
        unsafe fn set_value(value: &mut ::glib::value::Value, this: &Self) {
            gobject_ffi::g_value_set_object(
                value.to_glib_none_mut().0,
                ::glib::translate::ToGlibPtr::<*mut imp::ScrollableDrawingAreaFfi>::to_glib_none(
                    this,
                ).0 as *mut gobject_ffi::GObject,
            )
        }
    }
    #[doc(hidden)]
    impl ::glib::value::SetValueOptional for ScrollableDrawingArea {
        unsafe fn set_value_optional(value: &mut ::glib::value::Value, this: Option<&Self>) {
            gobject_ffi::g_value_set_object(
                value.to_glib_none_mut().0,
                ::glib::translate::ToGlibPtr::<*mut imp::ScrollableDrawingAreaFfi>::to_glib_none(
                    &this,
                ).0 as *mut gobject_ffi::GObject,
            )
        }
    }
    impl ::std::cmp::Eq for ScrollableDrawingArea {}
    #[doc(hidden)]
    impl<'a> ::glib::translate::ToGlibPtr<'a, *mut GtkDrawingArea> for ScrollableDrawingArea {
        type Storage = <::glib::object::ObjectRef as ::glib::translate::ToGlibPtr<
            'a,
            *mut ::glib::object::GObject,
        >>::Storage;
        #[inline]
        fn to_glib_none(
            &'a self,
        ) -> ::glib::translate::Stash<
            'a,
            *mut <DrawingArea as ::glib::wrapper::Wrapper>::GlibType,
            Self,
        > {
            let stash = self.0.to_glib_none();
            unsafe {
                if true {
                    if !::glib::types::instance_of::<DrawingArea>(stash.0 as *const _) {
                        {
                            panic!("assertion failed: ::glib::types::instance_of::<DrawingArea>(stash.0 as *const _)")
                        }
                    };
                };
            }
            ::glib::translate::Stash(stash.0 as *mut _, stash.1)
        }
        #[inline]
        fn to_glib_full(&self) -> *mut <DrawingArea as ::glib::wrapper::Wrapper>::GlibType {
            let ptr = self.0.to_glib_full();
            unsafe {
                if true {
                    if !::glib::types::instance_of::<DrawingArea>(ptr as *const _) {
                        {
                            panic!("assertion failed: ::glib::types::instance_of::<DrawingArea>(ptr as *const _)")
                        }
                    };
                };
            }
            ptr as *mut _
        }
    }
    unsafe impl ::glib::object::IsA<DrawingArea> for ScrollableDrawingArea {}
    #[doc(hidden)]
    impl<'a> ::glib::translate::ToGlibPtr<'a, *mut GtkWidget> for ScrollableDrawingArea {
        type Storage = <::glib::object::ObjectRef as ::glib::translate::ToGlibPtr<
            'a,
            *mut ::glib::object::GObject,
        >>::Storage;
        #[inline]
        fn to_glib_none(
            &'a self,
        ) -> ::glib::translate::Stash<
            'a,
            *mut GtkWidget,
            Self,
        > {
            let stash = self.0.to_glib_none();
            unsafe {
                if true {
                    if !::glib::types::instance_of::<DrawingArea>(stash.0 as *const _) {
                        {
                            panic!("assertion failed: ::glib::types::instance_of::<DrawingArea>(stash.0 as *const _)")
                        }
                    };
                };
            }
            ::glib::translate::Stash(stash.0 as *mut _, stash.1)
        }
        #[inline]
        fn to_glib_full(&self) -> *mut GtkWidget {
            let ptr = self.0.to_glib_full();
            unsafe {
                if true {
                    if !::glib::types::instance_of::<DrawingArea>(ptr as *const _) {
                        {
                            panic!("assertion failed: ::glib::types::instance_of::<DrawingArea>(ptr as *const _)")
                        }
                    };
                };
            }
            ptr as *mut _
        }
    }
    unsafe impl ::glib::object::IsA<::gtk::Widget> for ScrollableDrawingArea {}
    #[doc(hidden)]
    impl<'a> ::glib::translate::ToGlibPtr<'a, *mut ::glib::object::GObject> for ScrollableDrawingArea {
        type Storage = <::glib::object::ObjectRef as ::glib::translate::ToGlibPtr<
            'a,
            *mut ::glib::object::GObject,
        >>::Storage;
        #[inline]
        fn to_glib_none(
            &'a self,
        ) -> ::glib::translate::Stash<'a, *mut ::glib::object::GObject, Self> {
            let stash = self.0.to_glib_none();
            ::glib::translate::Stash(stash.0 as *mut _, stash.1)
        }
        #[inline]
        fn to_glib_full(&self) -> *mut ::glib::object::GObject {
            (&self.0).to_glib_full() as *mut _
        }
    }
    unsafe impl ::glib::object::IsA<::glib::object::Object> for ScrollableDrawingArea {}
    pub mod imp {
        #[allow(unused_imports)]
        use super::super::*;
        use super::glib;
        use super::glib_ffi;
        use super::gobject_ffi;
        #[allow(unused_imports)]
        use glib::translate::*;
        #[allow(unused_imports)]
        use std::ffi::CString;
        use std::mem;
        use std::ptr;
        #[repr(C)]
        pub struct ScrollableDrawingAreaFfi {
            pub parent: <DrawingArea as glib::wrapper::Wrapper>::GlibType,
        }
        #[repr(C)]
        pub struct ScrollableDrawingAreaClass {
            pub parent_class: <DrawingArea as glib::wrapper::Wrapper>::GlibClassType,
            pub value_changed:
                Option<unsafe extern "C" fn(this: *mut ScrollableDrawingAreaFfi) -> (())>,
            pub size_allocate: Option<
                unsafe extern "C" fn(this: *mut ScrollableDrawingAreaFfi, i: u32, j: u32) -> u32,
            >,
        }
        #[repr(u32)]
        enum Properties {
            hadjustment = 1u32,
            hscroll_policy = 2u32,
            vadjustment = 3u32,
            vscroll_policy = 4u32,
        }
        struct ScrollableDrawingAreaClassPrivate {
            parent_class: *const <DrawingArea as glib::wrapper::Wrapper>::GlibClassType,
            properties: *const Vec<*const gobject_ffi::GParamSpec>,
            value_changed_signal_id: u32,
        }
        static mut PRIV: ScrollableDrawingAreaClassPrivate = ScrollableDrawingAreaClassPrivate {
            parent_class: 0 as *const _,
            properties: 0 as *const _,
            value_changed_signal_id: 0,
        };
        struct ScrollableDrawingAreaPriv {
            hadjustment: Cell<u32>,
            hscroll_policy: Cell<u32>,
            vadjustment: Cell<u32>,
            vscroll_policy: Cell<u32>,
        }
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl ::std::default::Default for ScrollableDrawingAreaPriv {
            #[inline]
            fn default() -> ScrollableDrawingAreaPriv {
                ScrollableDrawingAreaPriv {
                    hadjustment: ::std::default::Default::default(),
                    hscroll_policy: ::std::default::Default::default(),
                    vadjustment: ::std::default::Default::default(),
                    vscroll_policy: ::std::default::Default::default(),
                }
            }
        }
        impl super::ScrollableDrawingArea {
            #[allow(dead_code)]
            fn get_priv(&self) -> &ScrollableDrawingAreaPriv {
                unsafe {
                    let _private = gobject_ffi::g_type_instance_get_private(
                        <Self as ToGlibPtr<*mut ScrollableDrawingAreaFfi>>::to_glib_none(self).0
                            as *mut gobject_ffi::GTypeInstance,
                        scrollable_drawing_area_get_type(),
                    )
                        as *const Option<ScrollableDrawingAreaPriv>;
                    (&*_private).as_ref().unwrap()
                }
            }
            #[allow(unused_variables)]
            fn value_changed_impl(&self) -> (()) {
                {
                    panic!("Called default signal handler with no implementation")
                };
            }
            fn size_allocate_impl(&self, i: u32, j: u32) -> u32 {
                1 + i + j
            }
            #[allow(unused)]
            fn emit_value_changed(&self) -> (()) {
                let params: &[glib::Value] = &[(self as &glib::ToValue).to_value()];
                unsafe {
                    let mut ret = glib::Value::uninitialized();
                    gobject_sys::g_signal_emitv(
                        mut_override(params.as_ptr()) as *mut gobject_sys::GValue,
                        PRIV.value_changed_signal_id,
                        0,
                        ret.to_glib_none_mut().0,
                    );
                    ()
                }
            }
            #[allow(dead_code, unused_variables)]
            fn set_property_impl(&self, property_id: u32, value: *mut gobject_ffi::GValue) {
                match property_id {
                    1u32 => {
                        let v: glib::Value = unsafe { FromGlibPtrNone::from_glib_none(value) };
                        if let Some(value) = v.get::<u32>() {
                            let mut private = self.get_priv();
                            private.hadjustment.set(value);
                        }
                    }
                    2u32 => {
                        let v: glib::Value = unsafe { FromGlibPtrNone::from_glib_none(value) };
                        if let Some(value) = v.get::<u32>() {
                            let mut private = self.get_priv();
                            private.hscroll_policy.set(value);
                        }
                    }
                    3u32 => {
                        let v: glib::Value = unsafe { FromGlibPtrNone::from_glib_none(value) };
                        if let Some(value) = v.get::<u32>() {
                            let mut private = self.get_priv();
                            private.vadjustment.set(value);
                        }
                    }
                    4u32 => {
                        let v: glib::Value = unsafe { FromGlibPtrNone::from_glib_none(value) };
                        if let Some(value) = v.get::<u32>() {
                            let mut private = self.get_priv();
                            private.vscroll_policy.set(value);
                        }
                    }
                    _ => {}
                }
            }
            #[allow(dead_code, unused_variables)]
            fn get_property_impl(&self, property_id: u32, value: *mut gobject_ffi::GValue) {
                match property_id {
                    1u32 => {
                        let ret: u32 = (|| {
                            let private = self.get_priv();
                            return private.hadjustment.get();
                        })();
                        unsafe {
                            gobject_ffi::g_value_set_uint(value, ret);
                        }
                    }
                    2u32 => {
                        let ret: u32 = (|| {
                            let private = self.get_priv();
                            return private.hscroll_policy.get();
                        })();
                        unsafe {
                            gobject_ffi::g_value_set_uint(value, ret);
                        }
                    }
                    3u32 => {
                        let ret: u32 = (|| {
                            let private = self.get_priv();
                            return private.vadjustment.get();
                        })();
                        unsafe {
                            gobject_ffi::g_value_set_uint(value, ret);
                        }
                    }
                    4u32 => {
                        let ret: u32 = (|| {
                            let private = self.get_priv();
                            return private.vscroll_policy.get();
                        })();
                        unsafe {
                            gobject_ffi::g_value_set_uint(value, ret);
                        }
                    }
                    _ => {}
                }
            }
        }
        impl ScrollableDrawingAreaFfi {
            #[allow(dead_code)]
            fn get_vtable(&self) -> &ScrollableDrawingAreaClass {
                unsafe {
                    let klass = (*(self as *const _ as *const gobject_ffi::GTypeInstance)).g_class;
                    &*(klass as *const ScrollableDrawingAreaClass)
                }
            }
            unsafe extern "C" fn init(
                obj: *mut gobject_ffi::GTypeInstance,
                _klass: glib_ffi::gpointer,
            ) {
                #[allow(unused_variables)]
                let obj = obj;
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                let _private = gobject_ffi::g_type_instance_get_private(
                    obj,
                    scrollable_drawing_area_get_type(),
                ) as *mut Option<ScrollableDrawingAreaPriv>;
                ptr::write(
                    _private,
                    Some(<ScrollableDrawingAreaPriv as Default>::default()),
                );
            }
            unsafe extern "C" fn finalize(obj: *mut gobject_ffi::GObject) {
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                let _private = gobject_ffi::g_type_instance_get_private(
                    obj as *mut gobject_ffi::GTypeInstance,
                    scrollable_drawing_area_get_type(),
                ) as *mut Option<ScrollableDrawingAreaPriv>;
                let _ = (*_private).take();
                (*(PRIV.parent_class as *mut gobject_ffi::GObjectClass))
                    .finalize
                    .map(|f| f(obj));
            }
            unsafe extern "C" fn set_property(
                obj: *mut gobject_ffi::GObject,
                property_id: u32,
                value: *mut gobject_ffi::GValue,
                _pspec: *mut gobject_ffi::GParamSpec,
            ) {
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                let this: &ScrollableDrawingArea =
                    &ScrollableDrawingArea::from_glib_borrow(obj as *mut ScrollableDrawingAreaFfi);
                this.set_property_impl(property_id, value);
            }
            unsafe extern "C" fn get_property(
                obj: *mut gobject_ffi::GObject,
                property_id: u32,
                value: *mut gobject_ffi::GValue,
                _pspec: *mut gobject_ffi::GParamSpec,
            ) {
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                let this: &ScrollableDrawingArea =
                    &ScrollableDrawingArea::from_glib_borrow(obj as *mut ScrollableDrawingAreaFfi);
                this.get_property_impl(property_id, value);
            }
            unsafe extern "C" fn value_changed_slot_trampoline(
                this: *mut ScrollableDrawingAreaFfi,
            ) -> (()) {
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                let this = this as *mut ScrollableDrawingAreaFfi;
                let instance: &super::ScrollableDrawingArea = &from_glib_borrow(this);
                instance.value_changed_impl()
            }
            unsafe extern "C" fn size_allocate_slot_trampoline(
                this: *mut ScrollableDrawingAreaFfi,
                i: u32,
                j: u32,
            ) -> u32 {
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                let this = this as *mut ScrollableDrawingAreaFfi;
                let instance: &super::ScrollableDrawingArea = &from_glib_borrow(this);
                instance.size_allocate_impl(i, j)
            }
        }
        impl ScrollableDrawingAreaClass {
            unsafe extern "C" fn init(vtable: glib_ffi::gpointer, _klass_data: glib_ffi::gpointer) {
                #[allow(deprecated)]
                let _guard = glib::CallbackGuard::new();
                gobject_ffi::g_type_class_add_private(
                    vtable,
                    mem::size_of::<Option<ScrollableDrawingAreaPriv>>(),
                );
                {
                    let gobject_class = &mut *(vtable as *mut gobject_ffi::GObjectClass);
                    gobject_class.finalize = Some(ScrollableDrawingAreaFfi::finalize);
                    gobject_class.set_property = Some(ScrollableDrawingAreaFfi::set_property);
                    gobject_class.get_property = Some(ScrollableDrawingAreaFfi::get_property);
                    let mut properties = Vec::new();
                    properties.push(ptr::null());
                    properties.push(gobject_ffi::g_param_spec_uint(
                        CString::new("hadjustment").unwrap().as_ptr(),
                        CString::new("hadjustment").unwrap().as_ptr(),
                        CString::new("hadjustment").unwrap().as_ptr(),
                        std::u32::MIN,
                        std::u32::MAX,
                        0,
                        gobject_ffi::G_PARAM_READWRITE,
                    ));
                    properties.push(gobject_ffi::g_param_spec_uint(
                        CString::new("hscroll_policy").unwrap().as_ptr(),
                        CString::new("hscroll_policy").unwrap().as_ptr(),
                        CString::new("hscroll_policy").unwrap().as_ptr(),
                        std::u32::MIN,
                        std::u32::MAX,
                        0,
                        gobject_ffi::G_PARAM_READWRITE,
                    ));
                    properties.push(gobject_ffi::g_param_spec_uint(
                        CString::new("vadjustment").unwrap().as_ptr(),
                        CString::new("vadjustment").unwrap().as_ptr(),
                        CString::new("vadjustment").unwrap().as_ptr(),
                        std::u32::MIN,
                        std::u32::MAX,
                        0,
                        gobject_ffi::G_PARAM_READWRITE,
                    ));
                    properties.push(gobject_ffi::g_param_spec_uint(
                        CString::new("vscroll_policy").unwrap().as_ptr(),
                        CString::new("vscroll_policy").unwrap().as_ptr(),
                        CString::new("vscroll_policy").unwrap().as_ptr(),
                        std::u32::MIN,
                        std::u32::MAX,
                        0,
                        gobject_ffi::G_PARAM_READWRITE,
                    ));
                    if properties.len() > 1 {
                        gobject_ffi::g_object_class_install_properties(
                            gobject_class,
                            properties.len() as u32,
                            properties.as_mut_ptr() as *mut *mut _,
                        );
                    }
                    PRIV.properties = Box::into_raw(Box::new(properties));
                }
                {
                    #[allow(unused_variables)]
                    let vtable = &mut *(vtable as *mut ScrollableDrawingAreaClass);
                    vtable.value_changed =
                        Some(ScrollableDrawingAreaFfi::value_changed_slot_trampoline);
                    vtable.size_allocate =
                        Some(ScrollableDrawingAreaFfi::size_allocate_slot_trampoline);
                }
                {
                    let param_gtypes = [];
                    PRIV.value_changed_signal_id = gobject_sys::g_signal_newv(
                        b"value-changed\x00" as *const u8 as *const i8,
                        scrollable_drawing_area_get_type(),
                        gobject_sys::G_SIGNAL_RUN_LAST,
                        ptr::null_mut(),
                        None,
                        ptr::null_mut(),
                        None,
                        gobject_sys::G_TYPE_NONE,
                        0u32,
                        mut_override(param_gtypes.as_ptr()),
                    );
                }
                PRIV.parent_class = gobject_ffi::g_type_class_peek_parent(vtable)
                    as *const <DrawingArea as glib::wrapper::Wrapper>::GlibClassType;
            }
        }
        pub unsafe extern "C" fn scrollable_drawing_area_new() -> *mut ScrollableDrawingAreaFfi {
            #[allow(deprecated)]
            let _guard = glib::CallbackGuard::new();
            let this =
                gobject_ffi::g_object_newv(scrollable_drawing_area_get_type(), 0, ptr::null_mut());
            this as *mut ScrollableDrawingAreaFfi
        }
        pub unsafe extern "C" fn scrollable_drawing_area_size_allocate(
            this: *mut ScrollableDrawingAreaFfi,
            i: u32,
            j: u32,
        ) -> u32 {
            #[allow(deprecated)]
            let _guard = glib::CallbackGuard::new();
            let vtable = (*this).get_vtable();
            (vtable.size_allocate.as_ref().unwrap())(this, i, j)
        }
        pub unsafe extern "C" fn scrollable_drawing_area_get_type() -> glib_ffi::GType {
            #[allow(deprecated)]
            let _guard = glib::CallbackGuard::new();
            use std::sync::{Once, ONCE_INIT};
            use std::u16;
            static mut TYPE: glib_ffi::GType = gobject_ffi::G_TYPE_INVALID;
            static ONCE: Once = ONCE_INIT;
            ONCE.call_once(|| {
                let class_size = mem::size_of::<ScrollableDrawingAreaClass>();
                if !(class_size <= u16::MAX as usize) {
                    {
                        panic!("assertion failed: class_size <= u16::MAX as usize")
                    }
                };
                let instance_size = mem::size_of::<ScrollableDrawingAreaFfi>();
                if !(instance_size <= u16::MAX as usize) {
                    {
                        panic!("assertion failed: instance_size <= u16::MAX as usize")
                    }
                };
                TYPE = gobject_ffi::g_type_register_static_simple(
                    <DrawingArea as glib::StaticType>::static_type().to_glib(),
                    b"ScrollableDrawingArea\x00" as *const u8 as *const i8,
                    class_size as u32,
                    Some(ScrollableDrawingAreaClass::init),
                    instance_size as u32,
                    Some(ScrollableDrawingAreaFfi::init),
                    gobject_ffi::GTypeFlags::empty(),
                );

                let interface_info = gobject_ffi::GInterfaceInfo{
                    interface_init: None, //TODO
                    interface_finalize: None, //TODO
                    interface_data: ptr::null_mut(), //TODO
                };
                gobject_ffi::g_type_add_interface_static(
                    TYPE,
                    <Scrollable as glib::StaticType>::static_type().to_glib(),
                    &interface_info,
                );
            });
            TYPE
        }
    }
    impl ScrollableDrawingArea {
        pub fn new() -> ScrollableDrawingArea {
            unsafe { from_glib_full(imp::scrollable_drawing_area_new()) }
        }
    }
    pub trait ScrollableDrawingAreaExt {
        fn connect_value_changed<F: Fn(&Self) -> (()) + 'static>(
            &self,
            f: F,
        ) -> glib::SignalHandlerId;
        fn size_allocate(&self, i: u32, j: u32) -> u32;
        fn get_property_hadjustment(&self) -> u32;
        fn get_property_hscroll_policy(&self) -> u32;
        fn get_property_vadjustment(&self) -> u32;
        fn get_property_vscroll_policy(&self) -> u32;
        fn set_property_hadjustment(&self, v: u32);
        fn set_property_hscroll_policy(&self, v: u32);
        fn set_property_vadjustment(&self, v: u32);
        fn set_property_vscroll_policy(&self, v: u32);
    }
    impl<O: IsA<ScrollableDrawingArea> + IsA<glib::object::Object> + glib::object::ObjectExt>
        ScrollableDrawingAreaExt for O
    {
        fn connect_value_changed<F: Fn(&Self) -> (()) + 'static>(
            &self,
            f: F,
        ) -> glib::SignalHandlerId {
            unsafe {
                let f: Box<Box<Fn(&Self) -> (()) + 'static>> = Box::new(Box::new(f));
                glib::signal::connect(
                    self.to_glib_none().0,
                    "value_changed",
                    mem::transmute(value_changed_signal_handler_trampoline::<Self> as usize),
                    Box::into_raw(f) as *mut _,
                )
            }
        }
        fn size_allocate(&self, i: u32, j: u32) -> u32 {
            unsafe { imp::scrollable_drawing_area_size_allocate(self.to_glib_none().0, i, j) }
        }
        fn get_property_hadjustment(&self) -> u32 {
            let mut value = glib::Value::from(&u32::default());
            unsafe {
                gobject_ffi::g_object_get_property(
                    self.to_glib_none().0,
                    "hadjustment".to_glib_none().0,
                    value.to_glib_none_mut().0,
                );
            }
            value.get::<u32>().unwrap()
        }
        fn get_property_hscroll_policy(&self) -> u32 {
            let mut value = glib::Value::from(&u32::default());
            unsafe {
                gobject_ffi::g_object_get_property(
                    self.to_glib_none().0,
                    "hscroll_policy".to_glib_none().0,
                    value.to_glib_none_mut().0,
                );
            }
            value.get::<u32>().unwrap()
        }
        fn get_property_vadjustment(&self) -> u32 {
            let mut value = glib::Value::from(&u32::default());
            unsafe {
                gobject_ffi::g_object_get_property(
                    self.to_glib_none().0,
                    "vadjustment".to_glib_none().0,
                    value.to_glib_none_mut().0,
                );
            }
            value.get::<u32>().unwrap()
        }
        fn get_property_vscroll_policy(&self) -> u32 {
            let mut value = glib::Value::from(&u32::default());
            unsafe {
                gobject_ffi::g_object_get_property(
                    self.to_glib_none().0,
                    "vscroll_policy".to_glib_none().0,
                    value.to_glib_none_mut().0,
                );
            }
            value.get::<u32>().unwrap()
        }
        fn set_property_hadjustment(&self, v: u32) {
            unsafe {
                gobject_ffi::g_object_set_property(
                    self.to_glib_none().0,
                    "hadjustment".to_glib_none().0,
                    glib::Value::from(&v).to_glib_none().0,
                );
            }
        }
        fn set_property_hscroll_policy(&self, v: u32) {
            unsafe {
                gobject_ffi::g_object_set_property(
                    self.to_glib_none().0,
                    "hscroll_policy".to_glib_none().0,
                    glib::Value::from(&v).to_glib_none().0,
                );
            }
        }
        fn set_property_vadjustment(&self, v: u32) {
            unsafe {
                gobject_ffi::g_object_set_property(
                    self.to_glib_none().0,
                    "vadjustment".to_glib_none().0,
                    glib::Value::from(&v).to_glib_none().0,
                );
            }
        }
        fn set_property_vscroll_policy(&self, v: u32) {
            unsafe {
                gobject_ffi::g_object_set_property(
                    self.to_glib_none().0,
                    "vscroll_policy".to_glib_none().0,
                    glib::Value::from(&v).to_glib_none().0,
                );
            }
        }
    }
    unsafe extern "C" fn value_changed_signal_handler_trampoline<P>(
        this: *mut imp::ScrollableDrawingAreaFfi,
        f: glib_ffi::gpointer,
    ) -> (())
    where
        P: IsA<ScrollableDrawingArea>,
    {
        #[allow(deprecated)]
        let _guard = glib::CallbackGuard::new();
        let f: &&(Fn(&P) -> (()) + 'static) = mem::transmute(f);
        f(&ScrollableDrawingArea::from_glib_borrow(this).downcast_unchecked())
    }
}
pub use self::ScrollableDrawingAreaMod::*;
