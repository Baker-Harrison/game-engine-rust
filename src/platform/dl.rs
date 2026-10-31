//! Runtime loading of system libraries (dlopen / LoadLibrary), so building needs no dev packages.

#[cfg(unix)]
mod imp {
    use std::ffi::{c_char, c_int, c_void};
    #[link(name = "dl")]
    extern "C" {
        pub fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
        pub fn dlsym(h: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    pub unsafe fn open(names: &[&str]) -> *mut c_void {
        for n in names {
            let c = std::ffi::CString::new(*n).unwrap();
            let h = dlopen(c.as_ptr(), 2); // RTLD_NOW
            if !h.is_null() {
                return h;
            }
        }
        std::ptr::null_mut()
    }
    pub unsafe fn sym(h: *mut c_void, name: &str) -> *mut c_void {
        let c = std::ffi::CString::new(name).unwrap();
        dlsym(h, c.as_ptr())
    }
}
#[cfg(windows)]
mod imp {
    use std::ffi::{c_char, c_void};
    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(h: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    pub unsafe fn open(names: &[&str]) -> *mut c_void {
        for n in names {
            let c = std::ffi::CString::new(*n).unwrap();
            let h = LoadLibraryA(c.as_ptr());
            if !h.is_null() {
                return h;
            }
        }
        std::ptr::null_mut()
    }
    pub unsafe fn sym(h: *mut c_void, name: &str) -> *mut c_void {
        let c = std::ffi::CString::new(name).unwrap();
        GetProcAddress(h, c.as_ptr())
    }
}
pub use imp::*;
