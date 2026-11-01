
use super::{dl, NativeHandle, Window};
use crate::input::{Button, Input, Key};
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

macro_rules! xlib_table {
    ($($f:ident($($a:ty),*) -> $r:ty;)*) => {
        #[allow(non_snake_case)]
        struct Xlib { $($f: unsafe extern "C" fn($($a),*) -> $r),* }
        impl Xlib {
            unsafe fn load() -> Result<Xlib, String> {
                let h = dl::open(&["libX11.so.6", "libX11.so"]);
                if h.is_null() {
                    return Err("libX11 not found (is an X11 server/desktop installed?)".into());
                }
                Ok(Xlib { $($f: {
                    let p = dl::sym(h, stringify!($f));
                    if p.is_null() { return Err(format!("libX11 is missing {}", stringify!($f))); }
                    std::mem::transmute::<*mut c_void, unsafe extern "C" fn($($a),*) -> $r>(p)
                }),* })
            }
        }
    };
}

xlib_table! {
    XOpenDisplay(*const c_char) -> Display;
    XDefaultScreen(Display) -> c_int;
    XRootWindow(Display, c_int) -> c_ulong;
    XBlackPixel(Display, c_int) -> c_ulong;
    XCreateSimpleWindow(Display, c_ulong, c_int, c_int, c_uint, c_uint, c_uint, c_ulong, c_ulong) -> c_ulong;
    XStoreName(Display, c_ulong, *const c_char) -> c_int;
    XSelectInput(Display, c_ulong, c_long) -> c_int;
    XMapWindow(Display, c_ulong) -> c_int;
    XDefaultGC(Display, c_int) -> *mut c_void;
    XDefaultVisual(Display, c_int) -> *mut c_void;
    XDefaultDepth(Display, c_int) -> c_int;
    XCreateImage(Display, *mut c_void, c_uint, c_int, c_int, *mut c_char, c_uint, c_uint, c_int, c_int) -> *mut c_void;
    XPutImage(Display, c_ulong, *mut c_void, *mut c_void, c_int, c_int, c_int, c_int, c_uint, c_uint) -> c_int;
    XPending(Display) -> c_int;
    XNextEvent(Display, *mut XEvent) -> c_int;
    XLookupKeysym(*mut XEvent, c_int) -> c_ulong;
    XInternAtom(Display, *const c_char, c_int) -> c_ulong;
    XSetWMProtocols(Display, c_ulong, *mut c_ulong, c_int) -> c_int;
    XGrabPointer(Display, c_ulong, c_int, c_uint, c_int, c_int, c_ulong, c_ulong, c_ulong) -> c_int;
    XUngrabPointer(Display, c_ulong) -> c_int;
    XWarpPointer(Display, c_ulong, c_ulong, c_int, c_int, c_uint, c_uint, c_int, c_int) -> c_int;
    XCreateBitmapFromData(Display, c_ulong, *const c_char, c_uint, c_uint) -> c_ulong;
    XCreatePixmapCursor(Display, c_ulong, c_ulong, *mut XColor, *mut XColor, c_uint, c_uint) -> c_ulong;
    XDefineCursor(Display, c_ulong, c_ulong) -> c_int;
    XUndefineCursor(Display, c_ulong) -> c_int;
    XFlush(Display) -> c_int;
    XkbSetDetectableAutoRepeat(Display, c_int, *mut c_int) -> c_int;
}

const KEY_PRESS_MASK: c_long = 1;
const KEY_RELEASE_MASK: c_long = 2;
const BUTTON_PRESS_MASK: c_long = 4;
const BUTTON_RELEASE_MASK: c_long = 8;
const POINTER_MOTION_MASK: c_long = 1 << 6;
const FOCUS_CHANGE_MASK: c_long = 1 << 21;

const KEY_PRESS: i32 = 2;
const KEY_RELEASE: i32 = 3;
const BUTTON_PRESS: i32 = 4;
const BUTTON_RELEASE: i32 = 5;
const MOTION_NOTIFY: i32 = 6;
const FOCUS_OUT: i32 = 10;
const CLIENT_MESSAGE: i32 = 33;

unsafe fn field<T: Copy>(ev: &XEvent, off: usize) -> T {
    std::ptr::read_unaligned((ev as *const XEvent as *const u8).add(off) as *const T)
}

fn key_from_keysym(keysym: u32) -> Option<Key> {
    match keysym {
        0x20..=0x7e => Key::from_char(char::from_u32(keysym)?),
        0xffe1 | 0xffe2 => Some(Key::Shift),
        0xffe3 | 0xffe4 => Some(Key::Control),
        0xffe9 | 0xffea => Some(Key::Alt),
        0xff09 => Some(Key::Tab),
        0xff1b => Some(Key::Escape),
        0xff0d => Some(Key::Enter),
        0xff08 => Some(Key::Backspace),
        0xff51 => Some(Key::Left),
        0xff52 => Some(Key::Up),
        0xff53 => Some(Key::Right),
        0xff54 => Some(Key::Down),
        0xffbe => Some(Key::F1),
        0xffbf => Some(Key::F2),
        0xffc0 => Some(Key::F3),
        0xffc1 => Some(Key::F4),
        _ => None,
    }
}

pub struct X11Window {
    x: Xlib,
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
            let x = Xlib::load()?;
            let d = (x.XOpenDisplay)(std::ptr::null());
            if d.is_null() {
                return Err("cannot open X display (is DISPLAY set?)".into());
            }
            let s = (x.XDefaultScreen)(d);
            let black = (x.XBlackPixel)(d, s);
            let win = (x.XCreateSimpleWindow)(d, (x.XRootWindow)(d, s), 0, 0, w as c_uint, h as c_uint, 0, black, black);
            let t = CString::new(title).unwrap();
            (x.XStoreName)(d, win, t.as_ptr());
            (x.XSelectInput)(d, win, KEY_PRESS_MASK | KEY_RELEASE_MASK | BUTTON_PRESS_MASK | POINTER_MOTION_MASK | FOCUS_CHANGE_MASK | BUTTON_RELEASE_MASK);
            let name = CString::new("WM_DELETE_WINDOW").unwrap();
            let mut wm_delete = (x.XInternAtom)(d, name.as_ptr(), 0);
            (x.XSetWMProtocols)(d, win, &mut wm_delete, 1);
            let mut ok = 0;
            (x.XkbSetDetectableAutoRepeat)(d, 1, &mut ok);
            (x.XMapWindow)(d, win);

            let mut buf = vec![0u32; w * h];
            let image = (x.XCreateImage)(d, (x.XDefaultVisual)(d, s), (x.XDefaultDepth)(d, s) as c_uint, 2, 0,
                buf.as_mut_ptr() as *mut c_char, w as c_uint, h as c_uint, 32, 0);
            if image.is_null() {
                return Err("XCreateImage failed".into());
            }
            let zero: [c_char; 8] = [0; 8];
            let bm = (x.XCreateBitmapFromData)(d, win, zero.as_ptr(), 1, 1);
            let mut col = XColor { pixel: 0, red: 0, green: 0, blue: 0, flags: 0, pad: 0 };
            let mut col2 = XColor { pixel: 0, red: 0, green: 0, blue: 0, flags: 0, pad: 0 };
            let blank_cursor = (x.XCreatePixmapCursor)(d, bm, bm, &mut col, &mut col2, 0, 0);
            (x.XFlush)(d);
            let gc = (x.XDefaultGC)(d, s);
            Ok(X11Window { x, d, win, gc, image, buf, w, h, wm_delete, blank_cursor, captured: false })
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
            while (self.x.XPending)(self.d) > 0 {
                let mut ev = XEvent { data: [0; 24] };
                (self.x.XNextEvent)(self.d, &mut ev);
                match field::<i32>(&ev, 0) {
                    KEY_PRESS | KEY_RELEASE => {
                        let down = field::<i32>(&ev, 0) == KEY_PRESS;
                        if let Some(key) = key_from_keysym((self.x.XLookupKeysym)(&mut ev, 0) as u32) {
                            input.set_key(key, down);
                        }
                    }
                    BUTTON_PRESS | BUTTON_RELEASE => {
                        let pressed = field::<i32>(&ev, 0) == BUTTON_PRESS;
                        match field::<u32>(&ev, 84) {
                            1 => input.set_button(Button::Left, pressed),
                            2 => input.set_button(Button::Middle, pressed),
                            3 => input.set_button(Button::Right, pressed),
                            4 if pressed => input.wheel += 1.0,
                            5 if pressed => input.wheel -= 1.0,
                            _ => {}
                        }
                    }
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
                (self.x.XWarpPointer)(self.d, 0, self.win, 0, 0, 0, 0, cx, cy);
                (self.x.XFlush)(self.d);
            }
        }
        open
    }

    fn present(&mut self, fb: &Framebuffer) {
        let n = self.buf.len().min(fb.pixels.len());
        self.buf[..n].copy_from_slice(&fb.pixels[..n]);
        unsafe {
            (self.x.XPutImage)(self.d, self.win, self.gc, self.image, 0, 0, 0, 0, self.w as c_uint, self.h as c_uint);
            (self.x.XFlush)(self.d);
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
                let r = (self.x.XGrabPointer)(self.d, self.win, 1, (BUTTON_PRESS_MASK | BUTTON_RELEASE_MASK | POINTER_MOTION_MASK) as c_uint, 1, 1, self.win, self.blank_cursor, 0);
                if r != 0 {
                    return;
                }
                (self.x.XDefineCursor)(self.d, self.win, self.blank_cursor);
                let (cx, cy) = self.center();
                (self.x.XWarpPointer)(self.d, 0, self.win, 0, 0, 0, 0, cx, cy);
            } else {
                (self.x.XUngrabPointer)(self.d, 0);
                (self.x.XUndefineCursor)(self.d, self.win);
            }
            self.captured = on;
            (self.x.XFlush)(self.d);
        }
    }
}
