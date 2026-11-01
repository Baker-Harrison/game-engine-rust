use crate::mixer::SharedMixer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

#[cfg(target_os = "linux")]
mod alsa;
#[cfg(target_os = "macos")]
mod coreaudio;
#[cfg(windows)]
mod winmm;

pub const BLOCK_FRAMES: usize = 512;

pub struct AudioOutput {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub backend: &'static str,
}

impl AudioOutput {
    pub fn start(mixer: SharedMixer) -> Result<AudioOutput, String> {
        let stop = Arc::new(AtomicBool::new(false));
        #[cfg(target_os = "linux")]
        let (thread, backend) = (alsa::start(mixer, stop.clone())?, "alsa");
        #[cfg(windows)]
        let (thread, backend) = (winmm::start(mixer, stop.clone())?, "winmm");
        #[cfg(target_os = "macos")]
        let (thread, backend) = (coreaudio::start(mixer, stop.clone())?, "coreaudio");
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        let (thread, backend): (Option<JoinHandle<()>>, &'static str) = {
            let _ = mixer;
            return Err("no audio backend for this OS".into());
        };
        Ok(AudioOutput { stop, thread, backend })
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
