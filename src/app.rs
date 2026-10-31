use crate::camera::MAX_PITCH;
use crate::input::Input;
use crate::platform;
use crate::render::{render, Framebuffer};
use crate::scene::Scene;
use crate::math::{clamp, Vec3};
use std::time::{Duration, Instant};

const WALK_SPEED: f32 = 4.5;
const SPRINT_MULT: f32 = 2.0;
const MOUSE_SENS: f32 = 0.0025; // radians per pixel
const PLAYER_RADIUS: f32 = 0.3;

/// Advances the player by one frame of input.
pub fn update(scene: &mut Scene, input: &Input, dt: f32) {
    let cam = &mut scene.camera;
    cam.yaw -= input.mouse_dx * MOUSE_SENS;
    cam.pitch = clamp(cam.pitch - input.mouse_dy * MOUSE_SENS, -MAX_PITCH, MAX_PITCH);

    let mut dir = Vec3::ZERO;
    if input.forward { dir = dir + cam.walk_forward(); }
    if input.back { dir = dir - cam.walk_forward(); }
    if input.right { dir = dir + cam.right(); }
    if input.left { dir = dir - cam.right(); }
    let dir = dir.normalized();
    let speed = WALK_SPEED * if input.sprint { SPRINT_MULT } else { 1.0 };
    let step = dir * (speed * dt);
    let (px, pz) = (cam.pos.x, cam.pos.z);
    // Axes are tested separately so the player slides along walls.
    let nx = px + step.x;
    if !scene.blocked(nx, pz, PLAYER_RADIUS) {
        scene.camera.pos.x = nx;
    }
    let nz = pz + step.z;
    if !scene.blocked(scene.camera.pos.x, nz, PLAYER_RADIUS) {
        scene.camera.pos.z = nz;
    }
}

pub fn run(mut scene: Scene, width: usize, height: usize) -> Result<(), String> {
    let mut window = platform::open("engine", width, height)?;
    let mut fb = Framebuffer::new(width, height);
    let mut input = Input::default();
    window.set_capture(true);
    let mut captured = true;
    let mut last = Instant::now();
    let mut title_timer = Instant::now();
    let mut frames = 0u32;
    println!("Controls: WASD move, mouse look, Shift sprint, Tab release/capture mouse, click to recapture, Esc quit");
    loop {
        input.begin_frame();
        if !window.poll(&mut input) || input.quit {
            break;
        }
        if input.capture_lost {
            captured = false;
        }
        if input.toggle_capture || (input.click && !captured) {
            captured = if input.toggle_capture { !captured } else { true };
            window.set_capture(captured);
            input.mouse_dx = 0.0;
            input.mouse_dy = 0.0;
        }
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;
        update(&mut scene, &input, dt);
        render(&scene, &scene.camera, &mut fb);
        window.present(&fb);
        frames += 1;
        if title_timer.elapsed() >= Duration::from_secs(2) {
            println!("{:.0} fps", frames as f32 / title_timer.elapsed().as_secs_f32());
            frames = 0;
            title_timer = Instant::now();
        }
        let spent = now.elapsed();
        if spent < Duration::from_millis(4) {
            std::thread::sleep(Duration::from_millis(4) - spent);
        }
    }
    window.set_capture(false);
    Ok(())
}
