//! Platform layer: window, keyboard/mouse, and presenting pixels.
//! Each backend is raw FFI to the OS, with no external crates.

use crate::input::Input;
use crate::render::Framebuffer;

pub trait Window {
    /// Pumps OS events into `input`. Returns false once the window is closed.
    fn poll(&mut self, input: &mut Input) -> bool;
    /// Shows the framebuffer in the window.
    fn present(&mut self, fb: &Framebuffer);
    /// Captures (hides and locks) or releases the mouse.
    fn set_capture(&mut self, on: bool);
}

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
