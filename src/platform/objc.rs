//! Minimal Objective-C runtime bindings shared by the macOS window and Metal backends.

use std::ffi::{c_char, c_void, CString};

pub type Id = *mut c_void;
pub type Sel = *mut c_void;

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Id;
    pub fn sel_registerName(name: *const c_char) -> Sel;
    // Declared without arguments on purpose: it is transmuted to the exact
    // signature of each call (the variadic ABI differs on Apple arm64).
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[link(name = "AppKit", kind = "framework")]
#[link(name = "Foundation", kind = "framework")]
#[link(name = "QuartzCore", kind = "framework")]
#[link(name = "Metal", kind = "framework")]
extern "C" {}

pub unsafe fn sel(name: &str) -> Sel {
    let c = CString::new(name).unwrap();
    sel_registerName(c.as_ptr())
}

pub unsafe fn class(name: &str) -> Id {
    let c = CString::new(name).unwrap();
    objc_getClass(c.as_ptr())
}

/// `send!(RetType; receiver, "selector:", ArgType = arg, ...)` calls objc_msgSend
/// with exactly that signature.
macro_rules! send {
    ($ret:ty; $recv:expr, $sel:expr $(, $argt:ty = $arg:expr)* $(,)?) => {{
        let f: unsafe extern "C" fn($crate::platform::objc::Id, $crate::platform::objc::Sel $(, $argt)*) -> $ret =
            std::mem::transmute($crate::platform::objc::objc_msgSend as *const ());
        f($recv as $crate::platform::objc::Id, $crate::platform::objc::sel($sel) $(, $arg)*)
    }};
}
pub(crate) use send;

pub unsafe fn nsstring(s: &str) -> Id {
    let c = CString::new(s).unwrap();
    send!(Id; class("NSString"), "stringWithUTF8String:", *const c_char = c.as_ptr())
}

/// Text of an NSError, for messages.
pub unsafe fn error_text(err: Id) -> String {
    if err.is_null() {
        return "unknown error".into();
    }
    let desc = send!(Id; err, "localizedDescription");
    let p = send!(*const c_char; desc, "UTF8String");
    if p.is_null() {
        "unknown error".into()
    } else {
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}
