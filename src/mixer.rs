use crate::math::Vec3;
use std::sync::{Arc, Mutex};

pub const SAMPLE_RATE: u32 = 48_000;
pub type SharedMixer = Arc<Mutex<Mixer>>;

#[derive(Clone)]
pub struct Sound {
    samples: Arc<Vec<f32>>,
}

impl Sound {
    pub fn from_samples(samples: Vec<f32>) -> Sound {
        Sound { samples: Arc::new(samples) }
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn duration_seconds(&self) -> f32 {
        self.samples.len() as f32 / SAMPLE_RATE as f32
    }

    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |p, s| p.max(s.abs()))
    }

    pub fn from_wav(bytes: &[u8]) -> Result<Sound, String> {
        let header_ok = bytes.len() > 44 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE";
        if !header_ok {
            return Err("not a RIFF/WAVE file".into());
        }
        let (mut channels, mut bits, mut data): (usize, usize, &[u8]) = (0, 0, &[]);
        let mut cursor = 12;
        while cursor + 8 <= bytes.len() {
            let size = u32::from_le_bytes([bytes[cursor + 4], bytes[cursor + 5], bytes[cursor + 6], bytes[cursor + 7]]) as usize;
            let body = &bytes[cursor + 8..(cursor + 8 + size).min(bytes.len())];
            match &bytes[cursor..cursor + 4] {
                b"fmt " if body.len() >= 16 => {
                    channels = u16::from_le_bytes([body[2], body[3]]) as usize;
                    bits = u16::from_le_bytes([body[14], body[15]]) as usize;
                }
                b"data" => data = body,
                _ => {}
            }
            cursor += 8 + size + (size & 1);
        }
        if bits != 16 || channels == 0 {
            return Err("only 16-bit PCM WAV is supported".into());
        }
        let frames = data.len() / (2 * channels);
        let mut samples = Vec::with_capacity(frames);
        for frame in 0..frames {
            let mut sum = 0.0;
            for channel in 0..channels {
                let at = (frame * channels + channel) * 2;
                sum += i16::from_le_bytes([data[at], data[at + 1]]) as f32 / 32768.0;
            }
            samples.push(sum / channels as f32);
        }
        Ok(Sound::from_samples(samples))
    }

    pub fn to_wav(&self) -> Vec<u8> {
        wav_bytes(&self.samples, 1)
    }
}

pub fn wav_bytes(samples: &[f32], channels: u16) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2 * channels as u32).to_le_bytes());
    out.extend_from_slice(&(2 * channels).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&((sample.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VoiceId(pub u64);

#[derive(Clone, Copy, Debug)]
pub struct PlayParams {
    pub gain: f32,
    pub pitch: f32,
    pub looped: bool,
    pub position: Option<Vec3>,
    pub reference_distance: f32,
    pub max_distance: f32,
}

impl Default for PlayParams {
    fn default() -> PlayParams {
        PlayParams { gain: 1.0, pitch: 1.0, looped: false, position: None, reference_distance: 3.0, max_distance: 90.0 }
    }
}

impl PlayParams {
    pub fn at(position: Vec3, gain: f32) -> PlayParams {
        PlayParams { gain, position: Some(position), ..PlayParams::default() }
    }

    pub fn flat(gain: f32) -> PlayParams {
        PlayParams { gain, ..PlayParams::default() }
    }
}

struct Voice {
    id: VoiceId,
    sound: Sound,
    cursor: f64,
    params: PlayParams,
}

pub struct Mixer {
    voices: Vec<Voice>,
    next_id: u64,
    listener_position: Vec3,
    listener_right: Vec3,
    pub master_gain: f32,
    pub max_voices: usize,
    frames_mixed: u64,
    pub peak: f32,
}

impl Default for Mixer {
    fn default() -> Mixer {
        Mixer {
            voices: Vec::new(),
            next_id: 1,
            listener_position: Vec3::ZERO,
            listener_right: Vec3::new(1.0, 0.0, 0.0),
            master_gain: 0.8,
            max_voices: 48,
            frames_mixed: 0,
            peak: 0.0,
        }
    }
}

impl Mixer {
    pub fn new() -> Mixer {
        Mixer::default()
    }

    pub fn shared() -> SharedMixer {
        Arc::new(Mutex::new(Mixer::new()))
    }

    pub fn set_listener(&mut self, position: Vec3, right: Vec3) {
        self.listener_position = position;
        self.listener_right = right;
    }

    pub fn play(&mut self, sound: &Sound, params: PlayParams) -> VoiceId {
        let id = VoiceId(self.next_id);
        self.next_id += 1;
        if self.voices.len() >= self.max_voices {
            let quietest = self
                .voices
                .iter()
                .enumerate()
                .filter(|(_, v)| !v.params.looped)
                .min_by(|a, b| a.1.params.gain.partial_cmp(&b.1.params.gain).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i);
            match quietest {
                Some(index) => {
                    self.voices.remove(index);
                }
                None => return id,
            }
        }
        self.voices.push(Voice { id, sound: sound.clone(), cursor: 0.0, params });
        id
    }

    pub fn stop(&mut self, id: VoiceId) {
        self.voices.retain(|v| v.id != id);
    }

    pub fn stop_all(&mut self) {
        self.voices.clear();
    }

    pub fn move_voice(&mut self, id: VoiceId, position: Vec3) {
        if let Some(voice) = self.voices.iter_mut().find(|v| v.id == id) {
            voice.params.position = Some(position);
        }
    }

    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn is_playing(&self, id: VoiceId) -> bool {
        self.voices.iter().any(|v| v.id == id)
    }

    pub fn frames_mixed(&self) -> u64 {
        self.frames_mixed
    }

    fn spatial_gains(&self, params: &PlayParams) -> (f32, f32) {
        let Some(position) = params.position else {
            return (params.gain, params.gain);
        };
        let offset = position - self.listener_position;
        let distance = offset.length();
        if distance >= params.max_distance {
            return (0.0, 0.0);
        }
        let attenuation = params.reference_distance / (params.reference_distance + (distance - params.reference_distance).max(0.0));
        let fade = (1.0 - distance / params.max_distance).clamp(0.0, 1.0);
        let level = params.gain * attenuation * fade.sqrt();
        let pan = if distance > 0.05 { offset.normalized().dot(self.listener_right).clamp(-1.0, 1.0) } else { 0.0 };
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        (level * angle.cos() * 1.3, level * angle.sin() * 1.3)
    }

    pub fn mix(&mut self, out: &mut [f32]) {
        out.iter_mut().for_each(|s| *s = 0.0);
        let frames = out.len() / 2;
        let mut finished = Vec::new();
        let gains: Vec<(f32, f32)> = self.voices.iter().map(|v| self.spatial_gains(&v.params)).collect();
        for (index, voice) in self.voices.iter_mut().enumerate() {
            let (left_gain, right_gain) = gains[index];
            let samples = voice.sound.samples();
            if samples.is_empty() {
                finished.push(index);
                continue;
            }
            let step = voice.params.pitch as f64;
            for frame in 0..frames {
                let position = voice.cursor;
                let whole = position as usize;
                if whole >= samples.len() {
                    if voice.params.looped {
                        voice.cursor -= samples.len() as f64;
                    } else {
                        finished.push(index);
                        break;
                    }
                }
                let whole = voice.cursor as usize;
                let fraction = (voice.cursor - whole as f64) as f32;
                let next = samples[(whole + 1) % samples.len()];
                let value = samples[whole] * (1.0 - fraction) + next * fraction;
                out[frame * 2] += value * left_gain;
                out[frame * 2 + 1] += value * right_gain;
                voice.cursor += step;
            }
        }
        for index in finished.into_iter().rev() {
            self.voices.remove(index);
        }
        let mut peak = 0.0f32;
        for sample in out.iter_mut() {
            *sample = soft_clip(*sample * self.master_gain);
            peak = peak.max(sample.abs());
        }
        self.peak = peak;
        self.frames_mixed += frames as u64;
    }
}

fn soft_clip(x: f32) -> f32 {
    if x.abs() <= 0.7 {
        x
    } else {
        let sign = x.signum();
        let over = x.abs() - 0.7;
        sign * (0.7 + 0.3 * (over / 0.3).tanh())
    }
}

pub fn render_to_wav(mixer: &mut Mixer, seconds: f32, path: &str) -> Result<(), String> {
    let frames = (seconds * SAMPLE_RATE as f32) as usize;
    let mut all = Vec::with_capacity(frames * 2);
    let mut block = vec![0.0f32; 512 * 2];
    while all.len() < frames * 2 {
        mixer.mix(&mut block);
        all.extend_from_slice(&block);
    }
    all.truncate(frames * 2);
    std::fs::write(path, wav_bytes(&all, 2)).map_err(|e| format!("{}: {}", path, e))
}
