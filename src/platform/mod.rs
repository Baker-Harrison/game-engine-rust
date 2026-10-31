//! Platform layer: window, keyboard/mouse, and presenting pixels.
//! Each backend is raw FFI to the OS, with no external crates.

use crate::input::Input;
use crate::render::Framebuffer;

/// The OS objects a GPU backend needs to attach a surface to a window.
#[derive(Clone, Copy, Debug)]
pub enum NativeHandle {
    None,
    /// NSView (content view of the NSWindow).
    MacView(*mut std::ffi::c_void),
    /// HWND.
    Win32(*mut std::ffi::c_void),
    /// Xlib Display* and Window.
    X11 { display: *mut std::ffi::c_void, window: u64 },
}

pub(crate) mod dl;

pub trait Window {
    /// Pumps OS events into `input`. Returns false once the window is closed.
    fn poll(&mut self, input: &mut Input) -> bool;
    /// Shows the framebuffer in the window.
    fn present(&mut self, fb: &Framebuffer);
    /// Captures (hides and locks) or releases the mouse.
    fn set_capture(&mut self, on: bool);
    /// OS handles for GPU surfaces.
    fn native_handle(&self) -> NativeHandle {
        NativeHandle::None
    }
    /// Client area size in pixels.
    fn size(&self) -> (usize, usize);
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
