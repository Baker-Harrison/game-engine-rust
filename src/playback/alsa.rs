use super::BLOCK_FRAMES;
use crate::mixer::{SharedMixer, SAMPLE_RATE};
use crate::platform::dl;
use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

type Open = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
type SetParams = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
type WriteInterleaved = unsafe extern "C" fn(*mut c_void, *const c_void, c_ulong) -> c_long;
type Prepare = unsafe extern "C" fn(*mut c_void) -> c_int;
type Close = unsafe extern "C" fn(*mut c_void) -> c_int;

const FORMAT_FLOAT_LE: c_int = 14;
const ACCESS_INTERLEAVED: c_int = 3;
const BROKEN_PIPE: c_long = -32;

struct Pcm {
    handle: *mut c_void,
    write: WriteInterleaved,
    prepare: Prepare,
    close: Close,
}

unsafe impl Send for Pcm {}

fn open_pcm() -> Result<Pcm, String> {
    unsafe {
        let library = dl::open(&["libasound.so.2", "libasound.so"]);
        if library.is_null() {
            return Err("libasound (ALSA) not found".into());
        }
        let load = |name: &str| -> Result<*mut c_void, String> {
            let symbol = dl::sym(library, name);
            if symbol.is_null() {
                Err(format!("libasound is missing {}", name))
            } else {
                Ok(symbol)
            }
        };
        let open: Open = std::mem::transmute(load("snd_pcm_open")?);
        let set_params: SetParams = std::mem::transmute(load("snd_pcm_set_params")?);
        let write: WriteInterleaved = std::mem::transmute(load("snd_pcm_writei")?);
        let prepare: Prepare = std::mem::transmute(load("snd_pcm_prepare")?);
        let close: Close = std::mem::transmute(load("snd_pcm_close")?);
        let device = CString::new(std::env::var("ENGINE_AUDIO_DEVICE").unwrap_or_else(|_| "default".into())).map_err(|e| e.to_string())?;
        let mut handle: *mut c_void = std::ptr::null_mut();
        let status = open(&mut handle, device.as_ptr(), 0, 0);
        if status < 0 || handle.is_null() {
            return Err(format!("snd_pcm_open failed ({})", status));
        }
        let status = set_params(handle, FORMAT_FLOAT_LE, ACCESS_INTERLEAVED, 2, SAMPLE_RATE, 1, 60_000);
        if status < 0 {
            close(handle);
            return Err(format!("snd_pcm_set_params failed ({})", status));
        }
        Ok(Pcm { handle, write, prepare, close })
    }
}

pub fn start(mixer: SharedMixer, stop: Arc<AtomicBool>) -> Result<Option<JoinHandle<()>>, String> {
    let pcm = open_pcm()?;
    let thread = std::thread::spawn(move || {
        let pcm = pcm;
        let mut block = vec![0.0f32; BLOCK_FRAMES * 2];
        while !stop.load(Ordering::SeqCst) {
            if let Ok(mut guard) = mixer.lock() {
                guard.mix(&mut block);
            }
            unsafe {
                let written = (pcm.write)(pcm.handle, block.as_ptr() as *const c_void, BLOCK_FRAMES as c_ulong);
                if written == BROKEN_PIPE {
                    (pcm.prepare)(pcm.handle);
                } else if written < 0 {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        unsafe {
            (pcm.close)(pcm.handle);
        }
    });
    Ok(Some(thread))
}
