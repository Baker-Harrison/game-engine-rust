use crate::gfx;
use crate::input::{Button, Input, Key};
use crate::platform;
use crate::playback::AudioOutput;
use crate::png;
use crate::render::{render, Framebuffer};
use crate::runtime::{Engine, Game, TICK_SECONDS};
use std::time::{Duration, Instant};

const MAX_TICKS_PER_FRAME: usize = 6;

pub struct RunOptions {
    pub width: usize,
    pub height: usize,
    pub renderer: String,
    pub max_frames: Option<usize>,
    pub audio: bool,
    pub seed: u64,
}

impl Default for RunOptions {
    fn default() -> RunOptions {
        RunOptions { width: 960, height: 540, renderer: "auto".into(), max_frames: None, audio: true, seed: 1 }
    }
}

pub fn run(game: &mut dyn Game, options: &RunOptions) -> Result<(), String> {
    let mut engine = Engine::new(options.seed);
    game.setup(&mut engine)?;
    let mut window = platform::open(game.name(), options.width, options.height)?;
    let (pixel_width, pixel_height) = window.size();
    engine.ui.set_target(pixel_width, pixel_height);
    let mut gpu = gfx::create(&options.renderer, pixel_width, pixel_height, window.as_ref())?;
    gpu.upload_scene(&engine.scene)?;
    engine.scene_dirty = false;
    println!("renderer: {} at {}x{} pixels (scale {:.1})", gpu.name(), pixel_width, pixel_height, window.scale_factor());
    let audio = start_audio(&engine, options);

    let mut input = Input::new();
    let mut captured = true;
    window.set_capture(true);
    let mut accumulator = 0.0f32;
    let mut last = Instant::now();
    let mut report_timer = Instant::now();
    let mut frames_since_report = 0u32;
    let mut frame_count = 0usize;
    let mut screenshots = 0u32;

    loop {
        if !window.poll(&mut input) || input.quit || input.key_pressed(Key::Escape) {
            break;
        }
        if input.capture_lost {
            captured = false;
        }
        if input.key_pressed(Key::Tab) {
            captured = !captured;
            window.set_capture(captured);
            input.mouse_dx = 0.0;
            input.mouse_dy = 0.0;
        } else if !captured && input.button_pressed(Button::Left) {
            captured = true;
            window.set_capture(true);
            input.clear_edges();
        }
        if input.key_pressed(Key::F2) {
            screenshots += 1;
            save_screenshot(&mut engine, (pixel_width, pixel_height), screenshots);
        }
        if !captured {
            input.mouse_dx = 0.0;
            input.mouse_dy = 0.0;
        }

        let now = Instant::now();
        accumulator += (now - last).as_secs_f32().min(0.25);
        last = now;
        let mut ticks = 0;
        while accumulator >= TICK_SECONDS && ticks < MAX_TICKS_PER_FRAME {
            game.tick(&mut engine, &input);
            engine.time.tick += 1;
            input.clear_edges();
            accumulator -= TICK_SECONDS;
            ticks += 1;
        }
        if ticks == MAX_TICKS_PER_FRAME {
            accumulator = 0.0;
        }
        if game.finished() {
            break;
        }

        game.draw(&mut engine, accumulator / TICK_SECONDS);
        engine.sync_listener();
        if engine.scene_dirty {
            gpu.upload_scene(&engine.scene)?;
            engine.scene_dirty = false;
        }
        gpu.render(&engine.scene, &engine.scene.camera, window.as_mut())?;

        frames_since_report += 1;
        frame_count += 1;
        if options.max_frames.map_or(false, |m| frame_count >= m) {
            break;
        }
        if report_timer.elapsed() >= Duration::from_secs(2) {
            println!("{:.0} fps", frames_since_report as f32 / report_timer.elapsed().as_secs_f32());
            frames_since_report = 0;
            report_timer = Instant::now();
        }
        let spent = now.elapsed();
        if spent < Duration::from_millis(3) {
            std::thread::sleep(Duration::from_millis(3) - spent);
        }
    }
    window.set_capture(false);
    drop(audio);
    println!("{}", game.report(&engine));
    Ok(())
}

fn start_audio(engine: &Engine, options: &RunOptions) -> Option<AudioOutput> {
    if !options.audio {
        return None;
    }
    match AudioOutput::start(engine.mixer.clone()) {
        Ok(output) => {
            println!("audio: {}", output.backend);
            Some(output)
        }
        Err(error) => {
            eprintln!("warning: audio disabled ({})", error);
            None
        }
    }
}

fn save_screenshot(engine: &mut Engine, pixels: (usize, usize), number: u32) {
    let mut framebuffer = Framebuffer::new(pixels.0, pixels.1);
    render(&engine.scene, &engine.scene.camera, &mut framebuffer);
    let path = format!("screenshot-{}.png", number);
    match png::write(&framebuffer, &path) {
        Ok(()) => println!("wrote {}", path),
        Err(error) => eprintln!("screenshot failed: {}", error),
    }
}
