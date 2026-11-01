use engine::camera::Camera;
use engine::gfx;
use engine::math::Vec3;
use engine::scene::{OverlayItem, OverlayRect, Scene, Tri};

fn channel_diff(a: u32, b: u32) -> u32 {
    (0..3)
        .map(|s| ((a >> (8 * s)) & 255).abs_diff((b >> (8 * s)) & 255))
        .max()
        .unwrap()
}

fn check_backend(name: &str) {
    let mut scene = Scene::parse(include_str!("../scenes/default.scene")).unwrap();
    scene.dynamic.push(Tri::flat([Vec3::new(-1.0, 0.2, 5.0), Vec3::new(1.0, 0.2, 5.0), Vec3::new(0.0, 2.0, 5.0)], Vec3::new(0.9, 0.3, 0.2)));
    scene.dynamic.push(Tri::glowing([Vec3::new(2.0, 0.2, 4.0), Vec3::new(3.5, 0.2, 4.0), Vec3::new(2.75, 1.6, 4.0)], Vec3::new(1.0, 0.9, 0.2)));
    scene.overlay.push(OverlayItem::Rect(OverlayRect { min: (0.1, 0.1), max: (0.4, 0.3), color: Vec3::new(0.1, 0.1, 0.1), alpha: 1.0 }));
    scene.overlay.push(OverlayItem::Rect(OverlayRect { min: (0.2, 0.2), max: (0.6, 0.5), color: Vec3::new(1.0, 0.2, 0.2), alpha: 0.5 }));
    let views = [
        (Vec3::new(0.0, 1.7, 9.0), 0.0f32, 0.0f32),
        (Vec3::new(-3.0, 1.7, 2.0), 40.0, -5.0),
        (Vec3::new(5.0, 3.0, 4.0), -30.0, -15.0),
    ];
    let (w, h) = (320, 180);
    let mut cpu = gfx::create_headless("cpu", w, h).unwrap();
    let mut gpu = match gfx::create_headless(name, w, h) {
        Ok(g) => g,
        Err(e) if e.contains("no Vulkan") || e.contains("no Metal") => {
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
        assert!(bad * 100 <= total * 2, "{}: {} of {} pixels differ at {:?}", name, bad, total, pos);
    }
}

#[test]
fn gpu_backends_match_cpu_reference() {
    #[cfg(target_os = "macos")]
    check_backend("metal");
    #[cfg(not(target_os = "macos"))]
    check_backend("vulkan");
    let _ = check_backend;
}
