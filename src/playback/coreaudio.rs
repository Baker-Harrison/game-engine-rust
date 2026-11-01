use super::BLOCK_FRAMES;
use crate::mixer::{SharedMixer, SAMPLE_RATE};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

#[repr(C)]
struct StreamDescription {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    bytes_per_frame: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
    reserved: u32,
}

#[repr(C)]
struct QueueBuffer {
    capacity: u32,
    data: *mut c_void,
    byte_size: u32,
    user_data: *mut c_void,
    packet_capacity: u32,
    packet_descriptions: *mut c_void,
    packet_count: u32,
}

type Queue = *mut c_void;
type Callback = extern "C" fn(*mut c_void, Queue, *mut QueueBuffer);

#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {
    fn AudioQueueNewOutput(
        format: *const StreamDescription,
        callback: Callback,
        user_data: *mut c_void,
        run_loop: *mut c_void,
        run_loop_mode: *mut c_void,
        flags: u32,
        queue: *mut Queue,
    ) -> i32;
    fn AudioQueueAllocateBuffer(queue: Queue, bytes: u32, buffer: *mut *mut QueueBuffer) -> i32;
    fn AudioQueueEnqueueBuffer(queue: Queue, buffer: *mut QueueBuffer, descriptions: u32, packets: *const c_void) -> i32;
    fn AudioQueueStart(queue: Queue, start_time: *const c_void) -> i32;
    fn AudioQueueStop(queue: Queue, immediate: bool) -> i32;
    fn AudioQueueDispose(queue: Queue, immediate: bool) -> i32;
}

extern "C" fn fill(user_data: *mut c_void, queue: Queue, buffer: *mut QueueBuffer) {
    unsafe {
        let mixer = &*(user_data as *const SharedMixer);
        let samples = std::slice::from_raw_parts_mut((*buffer).data as *mut f32, BLOCK_FRAMES * 2);
        match mixer.lock() {
            Ok(mut guard) => guard.mix(samples),
            Err(_) => samples.iter_mut().for_each(|s| *s = 0.0),
        }
        (*buffer).byte_size = (BLOCK_FRAMES * 2 * 4) as u32;
        AudioQueueEnqueueBuffer(queue, buffer, 0, std::ptr::null());
    }
}

struct QueueHandle(Queue);

unsafe impl Send for QueueHandle {}

pub fn start(mixer: SharedMixer, stop: Arc<AtomicBool>) -> Result<Option<JoinHandle<()>>, String> {
    unsafe {
        let format = StreamDescription {
            sample_rate: SAMPLE_RATE as f64,
            format_id: 0x6C70636D,
            format_flags: 1 | 8,
            bytes_per_packet: 8,
            frames_per_packet: 1,
            bytes_per_frame: 8,
            channels_per_frame: 2,
            bits_per_channel: 32,
            reserved: 0,
        };
        let user_data = Box::into_raw(Box::new(mixer)) as *mut c_void;
        let mut queue: Queue = std::ptr::null_mut();
        let status = AudioQueueNewOutput(&format, fill, user_data, std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut queue);
        if status != 0 {
            return Err(format!("AudioQueueNewOutput failed ({})", status));
        }
        for _ in 0..3 {
            let mut buffer: *mut QueueBuffer = std::ptr::null_mut();
            let status = AudioQueueAllocateBuffer(queue, (BLOCK_FRAMES * 2 * 4) as u32, &mut buffer);
            if status != 0 {
                return Err(format!("AudioQueueAllocateBuffer failed ({})", status));
            }
            fill(user_data, queue, buffer);
        }
        let status = AudioQueueStart(queue, std::ptr::null());
        if status != 0 {
            return Err(format!("AudioQueueStart failed ({})", status));
        }
        let handle = QueueHandle(queue);
        let thread = std::thread::spawn(move || {
            let handle = handle;
            while !stop.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            AudioQueueStop(handle.0, true);
            AudioQueueDispose(handle.0, true);
        });
        Ok(Some(thread))
    }
}
