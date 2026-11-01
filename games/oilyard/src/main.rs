mod level;
mod props;
use engine::camera::Camera;
use engine::math::Vec3;
use engine::render::{render, Framebuffer};
use engine::scene::Scene;

fn main() {
    let t = std::time::Instant::now();
    let level = level::build_level(std::env::args().any(|a| a == "bake"));
    println!("built in {:?}: {} tris, {} solids, {} spawns", t.elapsed(), level.mesh.triangle_count(), level.solid_count, level.spawns.len());
    let mut scene = Scene::from_mesh(&level.mesh, Vec3::new(0.62, 0.78, 0.95));
    scene.fog = 260.0;
    let views = [(Vec3::new(-40.0, 14.0, 30.0), 0.9, -0.38), (Vec3::new(0.0, 1.7, 14.0), 0.0, 0.0), (Vec3::new(0.0, 40.0, 1.0), 0.0, -1.5)];
    for (i, (pos, yaw, pitch)) in views.iter().enumerate() {
        let mut cam = Camera::new(*pos);
        cam.yaw = *yaw;
        cam.pitch = *pitch;
        scene.camera = cam;
        let mut fb = Framebuffer::new(1280, 720);
        render(&scene, &cam, &mut fb);
        engine::png::write(&fb, &format!("/tmp/level{}.png", i)).unwrap();
    }
}
