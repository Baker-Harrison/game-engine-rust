use engine::mixer::{Sound, SAMPLE_RATE};
use std::f32::consts::TAU;

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

struct LowPass {
    state: f32,
}

impl LowPass {
    fn step(&mut self, input: f32, cutoff_hz: f32) -> f32 {
        let alpha = 1.0 - (-TAU * cutoff_hz / SAMPLE_RATE as f32).exp();
        self.state += alpha * (input - self.state);
        self.state
    }
}

fn samples(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32) as usize
}

fn envelope(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack {
        t / attack
    } else {
        (-(t - attack) / decay).exp()
    }
}

fn normalize(mut data: Vec<f32>, peak: f32) -> Sound {
    let max = data.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    data.iter_mut().for_each(|s| *s *= peak / max);
    Sound::from_samples(data)
}

fn gunshot(seed: u32, seconds: f32, body_hz: f32, crack: f32, decay: f32, thump: f32) -> Sound {
    let mut noise = Noise(seed);
    let mut low = LowPass { state: 0.0 };
    let mut high_state = 0.0f32;
    let mut data = Vec::with_capacity(samples(seconds));
    for i in 0..samples(seconds) {
        let t = i as f32 / SAMPLE_RATE as f32;
        let n = noise.next();
        let cutoff = 400.0 + 7000.0 * (-t * 22.0).exp();
        let body = low.step(n, cutoff) * envelope(t, 0.001, decay);
        high_state += 0.35 * (n - high_state);
        let snap = (n - high_state) * envelope(t, 0.0005, 0.012) * crack;
        let frequency = body_hz * (0.45 + 0.55 * (-t * 30.0).exp());
        let boom = (TAU * frequency * t).sin() * envelope(t, 0.002, 0.07) * thump;
        data.push(body + snap + boom);
    }
    normalize(data, 0.9)
}

fn click(frequency: f32, seconds: f32, seed: u32) -> Vec<f32> {
    let mut noise = Noise(seed);
    (0..samples(seconds))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            (noise.next() * 0.6 + (TAU * frequency * t).sin()) * envelope(t, 0.0003, 0.006)
        })
        .collect()
}

fn mix_at(target: &mut Vec<f32>, source: &[f32], start_seconds: f32, gain: f32) {
    let start = samples(start_seconds);
    if target.len() < start + source.len() {
        target.resize(start + source.len(), 0.0);
    }
    for (i, s) in source.iter().enumerate() {
        target[start + i] += s * gain;
    }
}

fn tone(frequency: f32, seconds: f32, decay: f32, gain: f32) -> Vec<f32> {
    (0..samples(seconds))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            ((TAU * frequency * t).sin() + 0.3 * (TAU * frequency * 2.0 * t).sin()) * envelope(t, 0.002, decay) * gain
        })
        .collect()
}

pub struct Sounds {
    pub rifle: Sound,
    pub shotgun: Sound,
    pub pistol: Sound,
    pub distant_shot: Sound,
    pub reload: Sound,
    pub dry_fire: Sound,
    pub hit: Sound,
    pub headshot: Sound,
    pub kill: Sound,
    pub hurt: Sound,
    pub step: Sound,
    pub pickup: Sound,
    pub explosion: Sound,
    pub bounce: Sound,
    pub streak: Sound,
    pub respawn: Sound,
    pub match_end: Sound,
    pub wind: Sound,
}

impl Sounds {
    pub fn build() -> Sounds {
        let mut reload = Vec::new();
        mix_at(&mut reload, &click(900.0, 0.05, 11), 0.0, 0.6);
        mix_at(&mut reload, &click(500.0, 0.07, 12), 0.35, 0.9);
        mix_at(&mut reload, &click(1300.0, 0.04, 13), 0.95, 0.7);
        mix_at(&mut reload, &click(700.0, 0.06, 14), 1.1, 0.8);
        let mut kill = Vec::new();
        mix_at(&mut kill, &tone(880.0, 0.25, 0.08, 0.6), 0.0, 1.0);
        mix_at(&mut kill, &tone(1320.0, 0.35, 0.12, 0.6), 0.08, 1.0);
        let mut streak = Vec::new();
        for (i, f) in [523.0, 659.0, 784.0, 1047.0].iter().enumerate() {
            mix_at(&mut streak, &tone(*f, 0.3, 0.1, 0.5), i as f32 * 0.07, 1.0);
        }
        let mut pickup = Vec::new();
        mix_at(&mut pickup, &tone(660.0, 0.15, 0.05, 0.6), 0.0, 1.0);
        mix_at(&mut pickup, &tone(990.0, 0.2, 0.06, 0.6), 0.07, 1.0);
        let mut respawn = Vec::new();
        mix_at(&mut respawn, &tone(330.0, 0.3, 0.12, 0.5), 0.0, 1.0);
        mix_at(&mut respawn, &tone(495.0, 0.4, 0.15, 0.5), 0.1, 1.0);
        let mut match_end = Vec::new();
        for (i, f) in [392.0, 330.0, 262.0].iter().enumerate() {
            mix_at(&mut match_end, &tone(*f, 0.6, 0.25, 0.6), i as f32 * 0.25, 1.0);
        }
        Sounds {
            rifle: gunshot(1, 0.35, 140.0, 0.9, 0.09, 0.55),
            shotgun: gunshot(2, 0.7, 90.0, 0.8, 0.2, 1.0),
            pistol: gunshot(3, 0.3, 200.0, 1.0, 0.07, 0.4),
            distant_shot: gunshot(4, 0.4, 110.0, 0.2, 0.14, 0.5),
            reload: normalize(reload, 0.5),
            dry_fire: normalize(click(1400.0, 0.04, 9), 0.4),
            hit: normalize(tone(1900.0, 0.05, 0.015, 1.0), 0.45),
            headshot: normalize({ let mut d = tone(2600.0, 0.07, 0.02, 1.0); mix_at(&mut d, &tone(3300.0, 0.08, 0.025, 1.0), 0.04, 1.0); d }, 0.55),
            kill: normalize(kill, 0.55),
            hurt: normalize(gunshot(5, 0.18, 70.0, 0.1, 0.05, 1.0).samples().to_vec(), 0.7),
            step: normalize({ let mut n = Noise(21); let mut l = LowPass { state: 0.0 }; (0..samples(0.09)).map(|i| l.step(n.next(), 700.0) * envelope(i as f32 / SAMPLE_RATE as f32, 0.002, 0.025)).collect() }, 0.35),
            pickup: normalize(pickup, 0.5),
            explosion: explosion(),
            bounce: normalize(click(300.0, 0.08, 31), 0.5),
            streak: normalize(streak, 0.5),
            respawn: normalize(respawn, 0.4),
            match_end: normalize(match_end, 0.5),
            wind: wind(),
        }
    }
}

fn explosion() -> Sound {
    let mut noise = Noise(77);
    let mut low = LowPass { state: 0.0 };
    let data: Vec<f32> = (0..samples(1.6))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let cutoff = 150.0 + 3500.0 * (-t * 5.0).exp();
            let rumble = low.step(noise.next(), cutoff) * envelope(t, 0.003, 0.4);
            let sub = (TAU * 48.0 * t * (1.0 - 0.3 * t)).sin() * envelope(t, 0.004, 0.45);
            rumble * 1.4 + sub
        })
        .collect();
    normalize(data, 0.95)
}

fn wind() -> Sound {
    let length = samples(6.0);
    let mut noise = Noise(5);
    let mut low = LowPass { state: 0.0 };
    let mut data: Vec<f32> = (0..length + samples(1.0))
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            let gust = 0.55 + 0.45 * (TAU * t / 6.0).sin() * (TAU * t / 2.3).cos();
            low.step(noise.next(), 380.0 + 260.0 * gust) * gust
        })
        .collect();
    let overlap = samples(1.0);
    for i in 0..overlap {
        let fade = i as f32 / overlap as f32;
        data[i] = data[i] * fade + data[length + i] * (1.0 - fade);
    }
    data.truncate(length);
    normalize(data, 0.35)
}
