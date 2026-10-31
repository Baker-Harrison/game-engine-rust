//! Linux/X11 backend: raw FFI to libX11. Used for development and testing.

use super::{NativeHandle, Window};
use crate::input::Input;
use crate::render::Framebuffer;
use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void, CString};

type Display = *mut c_void;

#[repr(C)]
struct XEvent {
    data: [c_long; 24],
}

#[repr(C)]
struct XColor {
    pixel: c_ulong,
    red: u16,
    green: u16,
    blue: u16,
    flags: c_char,
    pad: c_char,
}

#[link(name = "X11")]
extern "C" {
    fn XOpenDisplay(name: *const c_char) -> Display;
    fn XDefaultScreen(d: Display) -> c_int;
    fn XRootWindow(d: Display, s: c_int) -> c_ulong;
    fn XBlackPixel(d: Display, s: c_int) -> c_ulong;
    fn XCreateSimpleWindow(d: Display, parent: c_ulong, x: c_int, y: c_int, w: c_uint, h: c_uint, bw: c_uint, border: c_ulong, bg: c_ulong) -> c_ulong;
    fn XStoreName(d: Display, w: c_ulong, name: *const c_char) -> c_int;
    fn XSelectInput(d: Display, w: c_ulong, mask: c_long) -> c_int;
    fn XMapWindow(d: Display, w: c_ulong) -> c_int;
    fn XDefaultGC(d: Display, s: c_int) -> *mut c_void;
    fn XDefaultVisual(d: Display, s: c_int) -> *mut c_void;
    fn XDefaultDepth(d: Display, s: c_int) -> c_int;
    fn XCreateImage(d: Display, v: *mut c_void, depth: c_uint, format: c_int, offset: c_int, data: *mut c_char, w: c_uint, h: c_uint, pad: c_int, bpl: c_int) -> *mut c_void;
    fn XPutImage(d: Display, dr: c_ulong, gc: *mut c_void, img: *mut c_void, sx: c_int, sy: c_int, dx: c_int, dy: c_int, w: c_uint, h: c_uint) -> c_int;
    fn XPending(d: Display) -> c_int;
    fn XNextEvent(d: Display, ev: *mut XEvent) -> c_int;
    fn XLookupKeysym(ev: *mut XEvent, index: c_int) -> c_ulong;
    fn XInternAtom(d: Display, name: *const c_char, only_if_exists: c_int) -> c_ulong;
    fn XSetWMProtocols(d: Display, w: c_ulong, protocols: *mut c_ulong, n: c_int) -> c_int;
    fn XGrabPointer(d: Display, w: c_ulong, owner: c_int, mask: c_uint, pmode: c_int, kmode: c_int, confine: c_ulong, cursor: c_ulong, time: c_ulong) -> c_int;
    fn XUngrabPointer(d: Display, time: c_ulong) -> c_int;
    fn XWarpPointer(d: Display, sw: c_ulong, dw: c_ulong, sx: c_int, sy: c_int, sw_: c_uint, sh: c_uint, dx: c_int, dy: c_int) -> c_int;
    fn XCreateBitmapFromData(d: Display, dr: c_ulong, data: *const c_char, w: c_uint, h: c_uint) -> c_ulong;
    fn XCreatePixmapCursor(d: Display, src: c_ulong, mask: c_ulong, fg: *mut XColor, bg: *mut XColor, x: c_uint, y: c_uint) -> c_ulong;
    fn XDefineCursor(d: Display, w: c_ulong, c: c_ulong) -> c_int;
    fn XUndefineCursor(d: Display, w: c_ulong) -> c_int;
    fn XFlush(d: Display) -> c_int;
    fn XkbSetDetectableAutoRepeat(d: Display, detectable: c_int, supported: *mut c_int) -> c_int;
}

const KEY_PRESS_MASK: c_long = 1;
const KEY_RELEASE_MASK: c_long = 2;
const BUTTON_PRESS_MASK: c_long = 4;
const POINTER_MOTION_MASK: c_long = 1 << 6;
const FOCUS_CHANGE_MASK: c_long = 1 << 21;

const KEY_PRESS: i32 = 2;
const KEY_RELEASE: i32 = 3;
const BUTTON_PRESS: i32 = 4;
const MOTION_NOTIFY: i32 = 6;
const FOCUS_OUT: i32 = 10;
const CLIENT_MESSAGE: i32 = 33;

// 64-bit Xlib event layout offsets.
unsafe fn field<T: Copy>(ev: &XEvent, off: usize) -> T {
    std::ptr::read_unaligned((ev as *const XEvent as *const u8).add(off) as *const T)
}

pub struct X11Window {
    d: Display,
    win: c_ulong,
    gc: *mut c_void,
    image: *mut c_void,
    buf: Vec<u32>,
    w: usize,
    h: usize,
    wm_delete: c_ulong,
    blank_cursor: c_ulong,
    captured: bool,
}

impl X11Window {
    pub fn new(title: &str, w: usize, h: usize) -> Result<X11Window, String> {
        unsafe {
            let d = XOpenDisplay(std::ptr::null());
            if d.is_null() {
                return Err("cannot open X display (is DISPLAY set?)".into());
            }
            let s = XDefaultScreen(d);
            let black = XBlackPixel(d, s);
            let win = XCreateSimpleWindow(d, XRootWindow(d, s), 0, 0, w as c_uint, h as c_uint, 0, black, black);
            let t = CString::new(title).unwrap();
            XStoreName(d, win, t.as_ptr());
            XSelectInput(d, win, KEY_PRESS_MASK | KEY_RELEASE_MASK | BUTTON_PRESS_MASK | POINTER_MOTION_MASK | FOCUS_CHANGE_MASK);
            let name = CString::new("WM_DELETE_WINDOW").unwrap();
            let mut wm_delete = XInternAtom(d, name.as_ptr(), 0);
            XSetWMProtocols(d, win, &mut wm_delete, 1);
            let mut ok = 0;
            XkbSetDetectableAutoRepeat(d, 1, &mut ok);
            XMapWindow(d, win);

            let mut buf = vec![0u32; w * h];
            let image = XCreateImage(d, XDefaultVisual(d, s), XDefaultDepth(d, s) as c_uint, 2, 0,
                buf.as_mut_ptr() as *mut c_char, w as c_uint, h as c_uint, 32, 0);
            if image.is_null() {
                return Err("XCreateImage failed".into());
            }
            let zero: [c_char; 8] = [0; 8];
            let bm = XCreateBitmapFromData(d, win, zero.as_ptr(), 1, 1);
            let mut col = XColor { pixel: 0, red: 0, green: 0, blue: 0, flags: 0, pad: 0 };
            let mut col2 = XColor { pixel: 0, red: 0, green: 0, blue: 0, flags: 0, pad: 0 };
            let blank_cursor = XCreatePixmapCursor(d, bm, bm, &mut col, &mut col2, 0, 0);
            XFlush(d);
            Ok(X11Window { d, win, gc: XDefaultGC(d, s), image, buf, w, h, wm_delete, blank_cursor, captured: false })
        }
    }

    fn center(&self) -> (i32, i32) {
        ((self.w / 2) as i32, (self.h / 2) as i32)
    }
}

impl Window for X11Window {
    fn poll(&mut self, input: &mut Input) -> bool {
        let mut open = true;
        let (cx, cy) = self.center();
        let mut moved: Option<(i32, i32)> = None;
        unsafe {
            while XPending(self.d) > 0 {
                let mut ev = XEvent { data: [0; 24] };
                XNextEvent(self.d, &mut ev);
                match field::<i32>(&ev, 0) {
                    KEY_PRESS | KEY_RELEASE => {
                        let down = field::<i32>(&ev, 0) == KEY_PRESS;
                        match XLookupKeysym(&mut ev, 0) {
                            0x77 | 0xff52 => input.forward = down,
                            0x73 | 0xff54 => input.back = down,
                            0x61 | 0xff51 => input.left = down,
                            0x64 | 0xff53 => input.right = down,
                            0xffe1 | 0xffe2 => input.sprint = down,
                            0xff1b if down => input.quit = true,
                            0xff09 if down => input.toggle_capture = true,
                            _ => {}
                        }
                    }
                    BUTTON_PRESS => input.click = true,
                    MOTION_NOTIFY => {
                        let (x, y) = (field::<i32>(&ev, 64), field::<i32>(&ev, 68));
                        if !(x == cx && y == cy) {
                            moved = Some((x, y));
                        }
                    }
                    FOCUS_OUT => {
                        if self.captured {
                            self.set_capture(false);
                            input.capture_lost = true;
                        }
                    }
                    CLIENT_MESSAGE => {
                        if field::<c_long>(&ev, 56) as c_ulong == self.wm_delete {
                            open = false;
                        }
                    }
                    _ => {}
                }
            }
            if self.captured {
                if let Some((x, y)) = moved {
                    input.mouse_dx += (x - cx) as f32;
                    input.mouse_dy += (y - cy) as f32;
                }
                XWarpPointer(self.d, 0, self.win, 0, 0, 0, 0, cx, cy);
                XFlush(self.d);
            }
        }
        open
    }

    fn present(&mut self, fb: &Framebuffer) {
        let n = self.buf.len().min(fb.pixels.len());
        self.buf[..n].copy_from_slice(&fb.pixels[..n]);
        unsafe {
            XPutImage(self.d, self.win, self.gc, self.image, 0, 0, 0, 0, self.w as c_uint, self.h as c_uint);
            XFlush(self.d);
        }
    }

    fn native_handle(&self) -> NativeHandle {
        NativeHandle::X11 { display: self.d, window: self.win as u64 }
    }

    fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    fn set_capture(&mut self, on: bool) {
        unsafe {
            if on {
                let r = XGrabPointer(self.d, self.win, 1, (BUTTON_PRESS_MASK | POINTER_MOTION_MASK) as c_uint, 1, 1, self.win, self.blank_cursor, 0);
                if r != 0 {
                    return;
                }
                XDefineCursor(self.d, self.win, self.blank_cursor);
                let (cx, cy) = self.center();
                XWarpPointer(self.d, 0, self.win, 0, 0, 0, 0, cx, cy);
            } else {
                XUngrabPointer(self.d, 0);
                XUndefineCursor(self.d, self.win);
            }
            self.captured = on;
            XFlush(self.d);
        }
    }
}
