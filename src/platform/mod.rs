
use crate::input::Input;
use crate::render::Framebuffer;

#[derive(Clone, Copy, Debug)]
pub enum NativeHandle {
    None,
    MacView(*mut std::ffi::c_void),
    Win32(*mut std::ffi::c_void),
    X11 { display: *mut std::ffi::c_void, window: u64 },
}

pub(crate) mod dl;

pub trait Window {
    fn poll(&mut self, input: &mut Input) -> bool;
    fn present(&mut self, fb: &Framebuffer);
    fn set_capture(&mut self, on: bool);
    fn native_handle(&self) -> NativeHandle {
        NativeHandle::None
    }
    fn size(&self) -> (usize, usize);
    fn scale_factor(&self) -> f32 {
        1.0
    }
}

#[cfg(target_os = "macos")]
#[macro_use]
pub(crate) mod objc;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub fn open(title: &str, w: usize, h: usize) -> Result<Box<dyn Window>, String> {
    macos::MacWindow::new(title, w, h).map(|w| Box::new(w) as Box<dyn Window>)
}

#[cfg(all(unix, not(target_os = "macos")))]
mod x11;
#[cfg(all(unix, not(target_os = "macos")))]
pub fn open(title: &str, w: usize, h: usize) -> Result<Box<dyn Window>, String> {
    x11::X11Window::new(title, w, h).map(|w| Box::new(w) as Box<dyn Window>)
}

#[cfg(windows)]
mod win32;
#[cfg(windows)]
pub fn open(title: &str, w: usize, h: usize) -> Result<Box<dyn Window>, String> {
    win32::WinWindow::new(title, w, h).map(|w| Box::new(w) as Box<dyn Window>)
}
