use engine::camera::Camera;
use engine::gfx::{build_vertices, FrameUniforms, Vertex};
use engine::math::Vec3;
use engine::render::{render, Framebuffer};
use engine::scene::Scene;

#[test]
fn gpu_struct_layouts_match_shaders() {
    // The shaders in every GPU backend assume these exact sizes.
    assert_eq!(std::mem::size_of::<Vertex>(), 48);
    assert_eq!(std::mem::size_of::<FrameUniforms>(), 128);
}

#[test]
fn view_proj_matches_cpu_projection() {
    let mut cam = Camera::new(Vec3::new(1.0, 2.0, 3.0));
    cam.yaw = 0.7;
    cam.pitch = -0.2;
    let aspect = 16.0 / 9.0;
    let m = cam.view_proj(aspect, 500.0, false);
    let p = Vec3::new(-2.0, 1.0, -6.0);
    let c = m.mul_point(p);
    // The CPU rasterizer's math, written out.
    let d = p - cam.pos;
    let (x, y, z) = (d.dot(cam.right()), d.dot(cam.up()), d.dot(cam.forward()));
    let t = (cam.fov_y * 0.5).tan();
    assert!(z > 0.0);
    assert!((c[3] - z).abs() < 1e-4);
    assert!((c[0] / c[3] - x / (z * t * aspect)).abs() < 1e-4);
    assert!((c[1] / c[3] - y / (z * t)).abs() < 1e-4);
    let depth = c[2] / c[3];
    assert!((0.0..=1.0).contains(&depth));
    // flip_y only negates Y.
    let f = cam.view_proj(aspect, 500.0, true).mul_point(p);
    assert!((f[1] + c[1]).abs() < 1e-5 && (f[0] - c[0]).abs() < 1e-5);
}

#[test]
fn scene_parses_and_builds_vertices() {
    let s = Scene::parse(include_str!("../scenes/default.scene")).unwrap();
    assert!(!s.boxes.is_empty());
    assert_eq!(build_vertices(&s).len(), s.tris.len() * 3);
    assert!(Scene::parse("bogus 1 2 3").is_err());
}

#[test]
fn headless_render_draws_sky_and_ground() {
    let s = Scene::parse(include_str!("../scenes/default.scene")).unwrap();
    let mut fb = Framebuffer::new(160, 90);
    render(&s, &s.camera, &mut fb);
    let sky = fb.pixels[5]; // top row
    let ground = fb.pixels[85 * 160 + 80]; // bottom center
    assert_ne!(sky, ground);
    assert_eq!(sky, 0x0087BDED_u32 & 0x00FFFFFF);
}
