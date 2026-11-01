use crate::gfx;
use crate::input::{Input, InputScript};
use crate::mixer::{wav_bytes, SAMPLE_RATE};
use crate::png;
use crate::runtime::{Engine, Game, TICK_RATE};
use crate::worldfile::fnv1a;
use std::fmt::Write;

pub struct SimOptions {
    pub ticks: u64,
    pub seed: u64,
    pub script: Option<InputScript>,
    pub capture_every: Option<u64>,
    pub capture_at: Vec<u64>,
    pub output_dir: String,
    pub width: usize,
    pub height: usize,
    pub renderer: String,
    pub wav_path: Option<String>,
}

impl Default for SimOptions {
    fn default() -> SimOptions {
        SimOptions {
            ticks: 600,
            seed: 1,
            script: None,
            capture_every: None,
            capture_at: Vec::new(),
            output_dir: "sim".into(),
            width: 960,
            height: 540,
            renderer: "cpu".into(),
            wav_path: None,
        }
    }
}

pub struct SimResult {
    pub report: String,
    pub files: Vec<String>,
    pub checksum: u64,
}

pub fn simulate(game: &mut dyn Game, options: &SimOptions) -> Result<SimResult, String> {
    let mut engine = Engine::new(options.seed);
    game.setup(&mut engine)?;
    engine.ui.set_target(options.width, options.height);
    let mut input = Input::new();
    let wants_frames = options.capture_every.is_some() || !options.capture_at.is_empty();
    let mut renderer = if wants_frames {
        std::fs::create_dir_all(&options.output_dir).map_err(|e| format!("{}: {}", options.output_dir, e))?;
        let mut r = gfx::create_headless(&options.renderer, options.width, options.height)?;
        r.upload_scene(&engine.scene)?;
        engine.scene_dirty = false;
        Some(r)
    } else {
        None
    };

    let frames_per_tick = SAMPLE_RATE as usize / TICK_RATE as usize;
    let mut block = vec![0.0f32; frames_per_tick * 2];
    let mut recorded: Vec<f32> = Vec::new();
    let (mut peak, mut energy, mut samples_seen, mut max_voices) = (0.0f32, 0.0f64, 0u64, 0usize);
    let mut files = Vec::new();
    let mut tick_hashes = 0xcbf29ce484222325u64;

    for tick in 0..options.ticks {
        input.clear_edges();
        if let Some(script) = &options.script {
            script.apply(tick, &mut input);
        }
        if input.quit {
            break;
        }
        game.tick(&mut engine, &input);
        engine.time.tick += 1;
        if game.finished() {
            break;
        }

        engine.sync_listener();
        if let Ok(mut mixer) = engine.mixer.lock() {
            max_voices = max_voices.max(mixer.voice_count());
            mixer.mix(&mut block);
        }
        for sample in &block {
            peak = peak.max(sample.abs());
            energy += (*sample as f64) * (*sample as f64);
        }
        samples_seen += block.len() as u64;
        if options.wav_path.is_some() {
            recorded.extend_from_slice(&block);
        }

        let wanted = options.capture_at.contains(&engine.time.tick) || options.capture_every.map_or(false, |n| n > 0 && engine.time.tick % n == 0);
        if let (true, Some(r)) = (wanted, renderer.as_mut()) {
            game.draw(&mut engine, 1.0);
            if engine.scene_dirty {
                r.upload_scene(&engine.scene)?;
                engine.scene_dirty = false;
            }
            let frame = r.render_offscreen(&engine.scene, &engine.scene.camera)?;
            let path = format!("{}/frame_{:05}.png", options.output_dir, engine.time.tick);
            png::write(&frame, &path)?;
            files.push(path);
        }
        tick_hashes = fnv1a(&[tick_hashes.to_le_bytes().as_slice(), game.report(&engine).as_bytes()].concat());
    }

    if let Some(path) = &options.wav_path {
        std::fs::write(path, wav_bytes(&recorded, 2)).map_err(|e| format!("{}: {}", path, e))?;
        files.push(path.clone());
    }

    let rms = if samples_seen > 0 { (energy / samples_seen as f64).sqrt() } else { 0.0 };
    let checksum = fnv1a(&[engine.registry.checksum(&engine.world).to_le_bytes().as_slice(), tick_hashes.to_le_bytes().as_slice()].concat());
    let mut report = String::new();
    let _ = writeln!(report, "game: {}", game.name());
    let _ = writeln!(report, "ticks: {} ({:.1} s at {} Hz), seed {}", engine.time.tick, engine.time.seconds(), TICK_RATE, options.seed);
    let _ = writeln!(report, "entities: {}", engine.world.entity_count());
    let _ = writeln!(report, "audio: peak {:.3}, rms {:.4}, max simultaneous voices {}", peak, rms, max_voices);
    let _ = writeln!(report, "checksum: {:016x}", checksum);
    let _ = write!(report, "{}", game.report(&engine));
    if !engine.log.is_empty() {
        let _ = writeln!(report, "\nevents:");
        for line in &engine.log {
            let _ = writeln!(report, "  {}", line);
        }
    }
    Ok(SimResult { report, files, checksum })
}
