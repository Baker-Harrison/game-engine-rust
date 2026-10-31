//! Renders the same frame with the CPU reference renderer and each GPU backend
//! available on this OS, and checks they agree. Skips a backend when this
//! machine has no usable GPU for it.
use engine::camera::Camera;
use engine::gfx;
use engine::math::Vec3;
use engine::scene::Scene;

fn channel_diff(a: u32, b: u32) -> u32 {
    (0..3)
        .map(|s| ((a >> (8 * s)) & 255).abs_diff((b >> (8 * s)) & 255))
        .max()
        .unwrap()
}

fn check_backend(name: &str) {
    let scene = Scene::parse(include_str!("../scenes/default.scene")).unwrap();
    let views = [
        (Vec3::new(0.0, 1.7, 8.0), 0.0f32, 0.0f32),
        (Vec3::new(-3.0, 1.7, 2.0), 40.0, -5.0),
        (Vec3::new(5.0, 3.0, 4.0), -30.0, -15.0),
    ];
    let (w, h) = (320, 180);
    let mut cpu = gfx::create_headless("cpu", w, h).unwrap();
    let mut gpu = match gfx::create_headless(name, w, h) {
        Ok(g) => g,
        Err(e) if e.contains("no ") && e.contains("device") => {
            eprintln!("skipping {}: {}", name, e);
            return;
        }
        Err(e) => panic!("{} failed to start: {}", name, e),
    };
    cpu.upload_scene(&scene).unwrap();
    gpu.upload_scene(&scene).unwrap();
    for (pos, yaw, pitch) in views {
        let mut cam: Camera = scene.camera;
        cam.pos = pos;
        cam.yaw = yaw.to_radians();
        cam.pitch = pitch.to_radians();
        let a = cpu.render_offscreen(&scene, &cam).unwrap();
        let b = gpu.render_offscreen(&scene, &cam).unwrap();
        let total = a.pixels.len();
        let bad = a.pixels.iter().zip(&b.pixels).filter(|(x, y)| channel_diff(**x, **y) > 12).count();
        // Triangle edges and fog rounding can differ a little between rasterizers.
        assert!(bad * 100 <= total * 2, "{}: {} of {} pixels differ at {:?}", name, bad, total, pos);
    }
}

#[test]
fn gpu_backends_match_cpu_reference() {
    #[cfg(target_os = "macos")]
    check_backend("metal");
    let _ = check_backend; // keeps the helper used on platforms with no GPU backend yet
}
