//! Windows backend: raw FFI to user32/gdi32/kernel32. Pixels are shown with
//! StretchDIBits. Keys and buttons are polled with GetAsyncKeyState, and mouse
//! look re-centers the cursor each frame.

use super::{NativeHandle, Window};
use crate::input::Input;
use crate::render::Framebuffer;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

type Hwnd = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
    private: u32,
}
#[repr(C)]
struct WndClassW {
    style: u32,
    proc_: unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    instance: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu: *const u16,
    class_name: *const u16,
}
#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_ppm: i32,
    y_ppm: i32,
    clr_used: u32,
    clr_important: u32,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(c: *const WndClassW) -> u16;
    fn CreateWindowExW(ex: u32, class: *const u16, title: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: Hwnd, menu: *mut c_void, inst: *mut c_void, param: *mut c_void) -> Hwnd;
    fn DefWindowProcW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
    fn PeekMessageW(msg: *mut Msg, h: Hwnd, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn AdjustWindowRect(r: *mut Rect, style: u32, menu: i32) -> i32;
    fn GetDC(h: Hwnd) -> *mut c_void;
    fn ReleaseDC(h: Hwnd, dc: *mut c_void) -> i32;
    fn GetAsyncKeyState(vk: i32) -> i16;
    fn GetForegroundWindow() -> Hwnd;
    fn GetCursorPos(p: *mut Point) -> i32;
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn ShowCursor(show: i32) -> i32;
    fn ClipCursor(r: *const Rect) -> i32;
    fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
    fn ClientToScreen(h: Hwnd, p: *mut Point) -> i32;
    fn LoadCursorW(inst: *mut c_void, name: usize) -> *mut c_void;
    fn DestroyWindow(h: Hwnd) -> i32;
}
#[link(name = "gdi32")]
extern "system" {
    fn StretchDIBits(dc: *mut c_void, xd: i32, yd: i32, wd: i32, hd: i32, xs: i32, ys: i32, ws: i32, hs: i32, bits: *const c_void, info: *const BitmapInfoHeader, usage: u32, rop: u32) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
}

const WM_CLOSE: u32 = 0x0010;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_MINIMIZEBOX: u32 = 0x0002_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;
const SRCCOPY: u32 = 0x00CC_0020;

static CLOSED: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn wnd_proc(h: Hwnd, m: u32, w: usize, l: isize) -> isize {
    if m == WM_CLOSE {
        CLOSED.store(true, Ordering::SeqCst);
        return 0;
    }
    DefWindowProcW(h, m, w, l)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct WinWindow {
    hwnd: Hwnd,
    w: usize,
    h: usize,
    captured: bool,
    tab_prev: bool,
    click_prev: bool,
}

impl WinWindow {
    pub fn new(title: &str, w: usize, h: usize) -> Result<WinWindow, String> {
        unsafe {
            let inst = GetModuleHandleW(std::ptr::null());
            let class = wide("EngineWindow");
            let wc = WndClassW {
                style: 0,
                proc_: wnd_proc,
                cls_extra: 0,
                wnd_extra: 0,
                instance: inst,
                icon: std::ptr::null_mut(),
                cursor: LoadCursorW(std::ptr::null_mut(), 32512),
                background: std::ptr::null_mut(),
                menu: std::ptr::null(),
                class_name: class.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                return Err("RegisterClassW failed".into());
            }
            let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE;
            let mut r = Rect { left: 0, top: 0, right: w as i32, bottom: h as i32 };
            AdjustWindowRect(&mut r, style, 0);
            let t = wide(title);
            let hwnd = CreateWindowExW(0, class.as_ptr(), t.as_ptr(), style, CW_USEDEFAULT, CW_USEDEFAULT,
                r.right - r.left, r.bottom - r.top, std::ptr::null_mut(), std::ptr::null_mut(), inst, std::ptr::null_mut());
            if hwnd.is_null() {
                return Err("CreateWindowExW failed".into());
            }
            Ok(WinWindow { hwnd, w, h, captured: false, tab_prev: false, click_prev: false })
        }
    }

    fn center_screen(&self) -> (i32, i32) {
        let mut p = Point { x: (self.w / 2) as i32, y: (self.h / 2) as i32 };
        unsafe { ClientToScreen(self.hwnd, &mut p) };
        (p.x, p.y)
    }
}

fn down(vk: i32) -> bool {
    unsafe { GetAsyncKeyState(vk) < 0 }
}

impl Window for WinWindow {
    fn poll(&mut self, input: &mut Input) -> bool {
        unsafe {
            let mut msg: Msg = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, 1) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let focused = GetForegroundWindow() == self.hwnd;
            if focused {
                input.forward = down(b'W' as i32) || down(0x26);
                input.back = down(b'S' as i32) || down(0x28);
                input.left = down(b'A' as i32) || down(0x25);
                input.right = down(b'D' as i32) || down(0x27);
                input.sprint = down(0x10);
                if down(0x1B) {
                    input.quit = true;
                }
                let tab = down(0x09);
                input.toggle_capture |= tab && !self.tab_prev;
                self.tab_prev = tab;
                let click = down(0x01) || down(0x02);
                input.click |= click && !self.click_prev;
                self.click_prev = click;
                if self.captured {
                    let (cx, cy) = self.center_screen();
                    let mut p = Point::default();
                    GetCursorPos(&mut p);
                    input.mouse_dx += (p.x - cx) as f32;
                    input.mouse_dy += (p.y - cy) as f32;
                    SetCursorPos(cx, cy);
                }
            } else {
                input.forward = false;
                input.back = false;
                input.left = false;
                input.right = false;
                input.sprint = false;
                if self.captured {
                    self.set_capture(false);
                    input.capture_lost = true;
                }
            }
        }
        !CLOSED.load(Ordering::SeqCst)
    }

    fn present(&mut self, fb: &Framebuffer) {
        unsafe {
            let hdr = BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: fb.width as i32,
                height: -(fb.height as i32), // top-down
                planes: 1,
                bit_count: 32,
                compression: 0,
                size_image: 0,
                x_ppm: 0,
                y_ppm: 0,
                clr_used: 0,
                clr_important: 0,
            };
            let dc = GetDC(self.hwnd);
            StretchDIBits(dc, 0, 0, self.w as i32, self.h as i32, 0, 0, fb.width as i32, fb.height as i32,
                fb.pixels.as_ptr() as *const c_void, &hdr, 0, SRCCOPY);
            ReleaseDC(self.hwnd, dc);
        }
    }

    fn native_handle(&self) -> NativeHandle {
        NativeHandle::Win32(self.hwnd)
    }

    fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    fn set_capture(&mut self, on: bool) {
        unsafe {
            if on == self.captured {
                return;
            }
            self.captured = on;
            if on {
                let mut r = Rect::default();
                GetClientRect(self.hwnd, &mut r);
                let mut tl = Point { x: r.left, y: r.top };
                let mut br = Point { x: r.right, y: r.bottom };
                ClientToScreen(self.hwnd, &mut tl);
                ClientToScreen(self.hwnd, &mut br);
                let clip = Rect { left: tl.x, top: tl.y, right: br.x, bottom: br.y };
                ClipCursor(&clip);
                ShowCursor(0);
                let (cx, cy) = self.center_screen();
                SetCursorPos(cx, cy);
            } else {
                ClipCursor(std::ptr::null());
                ShowCursor(1);
            }
        }
    }
}

impl Drop for WinWindow {
    fn drop(&mut self) {
        self.set_capture(false);
        unsafe { DestroyWindow(self.hwnd) };
    }
}
