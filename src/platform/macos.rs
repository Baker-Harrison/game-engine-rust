
use super::Window;
use crate::input::{Button, Input, Key};
use crate::render::Framebuffer;
use std::ffi::c_void;

use super::objc::*;
use super::NativeHandle;

#[repr(C)]
#[derive(Clone, Copy)]
struct NSRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    fn CGDataProviderCreateWithData(
        info: *mut c_void,
        data: *const c_void,
        size: usize,
        release: extern "C" fn(*mut c_void, *const c_void, usize),
    ) -> *mut c_void;
    fn CGImageCreate(
        width: usize,
        height: usize,
        bits_per_component: usize,
        bits_per_pixel: usize,
        bytes_per_row: usize,
        space: *mut c_void,
        bitmap_info: u32,
        provider: *mut c_void,
        decode: *const f64,
        should_interpolate: bool,
        intent: i32,
    ) -> *mut c_void;
    fn CGAssociateMouseAndMouseCursorPosition(connected: u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(obj: *const c_void);
}

extern "C" fn release_pixels(info: *mut c_void, _data: *const c_void, _size: usize) {
    unsafe { drop(Box::from_raw(info as *mut Vec<u32>)) };
}

const KEY_DOWN: usize = 10;
const KEY_UP: usize = 11;
const FLAGS_CHANGED: usize = 12;
const MOUSE_MOVES: [usize; 4] = [5, 6, 7, 27];
const SHIFT_FLAG: usize = 1 << 17;
const CONTROL_FLAG: usize = 1 << 18;
const OPTION_FLAG: usize = 1 << 19;

fn key_from_code(code: u16) -> Option<Key> {
    const LETTERS: [(u16, char); 26] = [
        (0, 'a'), (11, 'b'), (8, 'c'), (2, 'd'), (14, 'e'), (3, 'f'), (5, 'g'), (4, 'h'), (34, 'i'),
        (38, 'j'), (40, 'k'), (37, 'l'), (46, 'm'), (45, 'n'), (31, 'o'), (35, 'p'), (12, 'q'),
        (15, 'r'), (1, 's'), (17, 't'), (32, 'u'), (9, 'v'), (13, 'w'), (7, 'x'), (16, 'y'), (6, 'z'),
    ];
    const DIGITS: [(u16, char); 10] = [
        (29, '0'), (18, '1'), (19, '2'), (20, '3'), (21, '4'), (23, '5'), (22, '6'), (26, '7'), (28, '8'), (25, '9'),
    ];
    if let Some((_, character)) = LETTERS.iter().chain(DIGITS.iter()).find(|(c, _)| *c == code) {
        return Key::from_char(*character);
    }
    match code {
        49 => Some(Key::Space),
        48 => Some(Key::Tab),
        53 => Some(Key::Escape),
        36 => Some(Key::Enter),
        51 => Some(Key::Backspace),
        123 => Some(Key::Left),
        124 => Some(Key::Right),
        125 => Some(Key::Down),
        126 => Some(Key::Up),
        122 => Some(Key::F1),
        120 => Some(Key::F2),
        99 => Some(Key::F3),
        118 => Some(Key::F4),
        _ => None,
    }
}

pub struct MacWindow {
    app: Id,
    window: Id,
    view: Id,
    captured: bool,
    cursor_hidden: bool,
    size: (usize, usize),
}

impl MacWindow {
    pub fn new(title: &str, w: usize, h: usize) -> Result<MacWindow, String> {
        unsafe {
            let pool = objc_autoreleasePoolPush();
            let app = send!(Id; class("NSApplication"), "sharedApplication");
            if app.is_null() {
                return Err("could not create NSApplication".into());
            }
            send!(bool; app, "setActivationPolicy:", isize = 0);
            let rect = NSRect { x: 0.0, y: 0.0, w: w as f64, h: h as f64 };
            let alloc = send!(Id; class("NSWindow"), "alloc");
            let window = send!(Id; alloc, "initWithContentRect:styleMask:backing:defer:",
                NSRect = rect, usize = 1 | 2 | 4, usize = 2, bool = false);
            if window.is_null() {
                return Err("could not create NSWindow".into());
            }
            send!((); window, "setTitle:", Id = nsstring(title));
            send!((); window, "setAcceptsMouseMovedEvents:", bool = true);
            send!((); window, "center");
            let view = send!(Id; window, "contentView");
            send!((); view, "setWantsLayer:", bool = true);
            send!((); window, "makeKeyAndOrderFront:", Id = std::ptr::null_mut());
            send!((); app, "activateIgnoringOtherApps:", bool = true);
            send!((); app, "finishLaunching");
            objc_autoreleasePoolPop(pool);
            Ok(MacWindow { app, window, view, captured: false, cursor_hidden: false, size: (w, h) })
        }
    }
}

impl Window for MacWindow {
    fn poll(&mut self, input: &mut Input) -> bool {
        unsafe {
            let pool = objc_autoreleasePoolPush();
            let mode = nsstring("kCFRunLoopDefaultMode");
            loop {
                let ev = send!(Id; self.app, "nextEventMatchingMask:untilDate:inMode:dequeue:",
                    usize = usize::MAX, Id = std::ptr::null_mut(), Id = mode, bool = true);
                if ev.is_null() {
                    break;
                }
                let ty = send!(usize; ev, "type");
                if ty == KEY_DOWN || ty == KEY_UP {
                    if let Some(key) = key_from_code(send!(u16; ev, "keyCode")) {
                        input.set_key(key, ty == KEY_DOWN);
                    }
                    continue;
                }
                if ty == FLAGS_CHANGED {
                    let flags = send!(usize; ev, "modifierFlags");
                    input.set_key(Key::Shift, flags & SHIFT_FLAG != 0);
                    input.set_key(Key::Control, flags & CONTROL_FLAG != 0);
                    input.set_key(Key::Alt, flags & OPTION_FLAG != 0);
                    continue;
                }
                match ty {
                    1 | 2 => input.set_button(Button::Left, ty == 1),
                    3 | 4 => input.set_button(Button::Right, ty == 3),
                    25 | 26 => input.set_button(Button::Middle, ty == 25),
                    22 => input.wheel += send!(f64; ev, "scrollingDeltaY") as f32 * 0.1,
                    _ => {}
                }
                if MOUSE_MOVES.contains(&ty) && self.captured {
                    input.mouse_dx += send!(f64; ev, "deltaX") as f32;
                    input.mouse_dy += send!(f64; ev, "deltaY") as f32;
                }
                send!((); self.app, "sendEvent:", Id = ev);
            }
            let visible = send!(bool; self.window, "isVisible");
            let key = send!(bool; self.window, "isKeyWindow");
            if self.captured && !key {
                self.set_capture(false);
                input.capture_lost = true;
            }
            objc_autoreleasePoolPop(pool);
            visible
        }
    }

    fn present(&mut self, fb: &Framebuffer) {
        unsafe {
            let pool = objc_autoreleasePoolPush();
            let buf = Box::new(fb.pixels.clone());
            let data = buf.as_ptr() as *const c_void;
            let size = buf.len() * 4;
            let info = Box::into_raw(buf) as *mut c_void;
            let provider = CGDataProviderCreateWithData(info, data, size, release_pixels);
            let space = CGColorSpaceCreateDeviceRGB();
            let img = CGImageCreate(
                fb.width, fb.height, 8, 32, fb.width * 4, space, 6 | (2 << 12),
                provider, std::ptr::null(), false, 0,
            );
            let layer = send!(Id; self.view, "layer");
            let tx = class("CATransaction");
            send!((); tx, "begin");
            send!((); tx, "setDisableActions:", bool = true);
            send!((); layer, "setContents:", Id = img);
            send!((); tx, "commit");
            CFRelease(img);
            CFRelease(space);
            CFRelease(provider);
            objc_autoreleasePoolPop(pool);
        }
    }

    fn native_handle(&self) -> NativeHandle {
        NativeHandle::MacView(self.view)
    }

    fn size(&self) -> (usize, usize) {
        self.size
    }

    fn set_capture(&mut self, on: bool) {
        unsafe {
            if on == self.captured {
                return;
            }
            self.captured = on;
            CGAssociateMouseAndMouseCursorPosition(if on { 0 } else { 1 });
            if on && !self.cursor_hidden {
                send!((); class("NSCursor"), "hide");
                self.cursor_hidden = true;
            } else if !on && self.cursor_hidden {
                send!((); class("NSCursor"), "unhide");
                self.cursor_hidden = false;
            }
        }
    }
}

impl Drop for MacWindow {
    fn drop(&mut self) {
        self.set_capture(false);
    }
}
