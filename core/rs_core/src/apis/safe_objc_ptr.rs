use cocoa::appkit::{NSPasteboard, NSPasteboardTypeString};
use cocoa::base::id;
use cocoa::foundation::NSData;
use cocoa::foundation::NSString;
use objc::runtime::Object;
use std::marker::PhantomData;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct SafeObjcPtr {
    ptr: *mut Object,
    _marker: PhantomData<&'static mut Object>,
}

unsafe impl Send for SafeObjcPtr {}
unsafe impl Sync for SafeObjcPtr {}

impl SafeObjcPtr {
    pub(crate) fn new(ptr: *mut Object) -> Self {
        SafeObjcPtr {
            ptr,
            _marker: PhantomData,
        }
    }

    pub(crate) fn get(&self) -> *mut Object {
        self.ptr
    }
}

impl Deref for SafeObjcPtr {
    type Target = *mut Object;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}
