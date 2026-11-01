use super::BLOCK_FRAMES;
use crate::mixer::{SharedMixer, SAMPLE_RATE};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

#[repr(C)]
struct WaveFormat {
    format_tag: u16,
    channels: u16,
    samples_per_second: u32,
    bytes_per_second: u32,
    block_align: u16,
    bits_per_sample: u16,
    extra_size: u16,
}

#[repr(C)]
struct WaveHeader {
    data: *mut u8,
    buffer_length: u32,
    bytes_recorded: u32,
    user: usize,
    flags: u32,
    loops: u32,
    next: *mut WaveHeader,
    reserved: usize,
}

#[link(name = "winmm")]
extern "system" {
    fn waveOutOpen(device: *mut *mut c_void, id: u32, format: *const WaveFormat, callback: usize, instance: usize, flags: u32) -> u32;
    fn waveOutPrepareHeader(device: *mut c_void, header: *mut WaveHeader, size: u32) -> u32;
    fn waveOutWrite(device: *mut c_void, header: *mut WaveHeader, size: u32) -> u32;
    fn waveOutUnprepareHeader(device: *mut c_void, header: *mut WaveHeader, size: u32) -> u32;
    fn waveOutReset(device: *mut c_void) -> u32;
    fn waveOutClose(device: *mut c_void) -> u32;
}

const WAVE_MAPPER: u32 = 0xFFFF_FFFF;
const HEADER_DONE: u32 = 1;
const BUFFER_COUNT: usize = 4;

struct Device(*mut c_void);

unsafe impl Send for Device {}

struct Slot {
    header: Box<WaveHeader>,
    pcm: Vec<i16>,
}

unsafe impl Send for Slot {}

pub fn start(mixer: SharedMixer, stop: Arc<AtomicBool>) -> Result<Option<JoinHandle<()>>, String> {
    unsafe {
        let format = WaveFormat {
            format_tag: 1,
            channels: 2,
            samples_per_second: SAMPLE_RATE,
            bytes_per_second: SAMPLE_RATE * 4,
            block_align: 4,
            bits_per_sample: 16,
            extra_size: 0,
        };
        let mut device: *mut c_void = std::ptr::null_mut();
        let status = waveOutOpen(&mut device, WAVE_MAPPER, &format, 0, 0, 0);
        if status != 0 {
            return Err(format!("waveOutOpen failed ({})", status));
        }
        let device = Device(device);
        let header_size = std::mem::size_of::<WaveHeader>() as u32;
        let mut slots: Vec<Slot> = (0..BUFFER_COUNT)
            .map(|_| {
                let mut pcm = vec![0i16; BLOCK_FRAMES * 2];
                let header = Box::new(WaveHeader {
                    data: pcm.as_mut_ptr() as *mut u8,
                    buffer_length: (BLOCK_FRAMES * 4) as u32,
                    bytes_recorded: 0,
                    user: 0,
                    flags: 0,
                    loops: 0,
                    next: std::ptr::null_mut(),
                    reserved: 0,
                });
                Slot { header, pcm }
            })
            .collect();
        for slot in &mut slots {
            waveOutPrepareHeader(device.0, &mut *slot.header, header_size);
            slot.header.flags |= HEADER_DONE;
        }
        let thread = std::thread::spawn(move || {
            let device = device;
            let mut slots = slots;
            let mut block = vec![0.0f32; BLOCK_FRAMES * 2];
            while !stop.load(Ordering::SeqCst) {
                let mut wrote = false;
                for slot in &mut slots {
                    if slot.header.flags & HEADER_DONE == 0 {
                        continue;
                    }
                    if let Ok(mut guard) = mixer.lock() {
                        guard.mix(&mut block);
                    }
                    for (target, source) in slot.pcm.iter_mut().zip(block.iter()) {
                        *target = (source.clamp(-1.0, 1.0) * 32767.0) as i16;
                    }
                    unsafe {
                        slot.header.flags &= !HEADER_DONE;
                        waveOutWrite(device.0, &mut *slot.header, header_size);
                    }
                    wrote = true;
                }
                if !wrote {
                    std::thread::sleep(std::time::Duration::from_millis(4));
                }
            }
            unsafe {
                waveOutReset(device.0);
                for slot in &mut slots {
                    waveOutUnprepareHeader(device.0, &mut *slot.header, header_size);
                }
                waveOutClose(device.0);
            }
        });
        Ok(Some(thread))
    }
}
