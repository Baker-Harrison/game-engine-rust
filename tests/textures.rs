use engine::camera::Camera;
use engine::math::Vec3;
use engine::render::{render, Framebuffer};
use engine::scene::{Scene, Tri};
use engine::texture::Texture;

fn checker(size: usize, a: [u8; 4], b: [u8; 4]) -> Texture {
    let mut texture = Texture::new(size, size);
    for y in 0..size {
        for x in 0..size {
            texture.set(x, y, if (x + y) % 2 == 0 { a } else { b });
        }
    }
    texture
}

fn flat_normal(size: usize) -> Texture {
    let mut texture = Texture::new(size, size);
    for y in 0..size {
        for x in 0..size {
            texture.set(x, y, [128, 128, 255, 255]);
        }
    }
    texture
}

fn packed(size: usize, roughness: u8) -> Texture {
    let mut texture = Texture::new(size, size);
    for y in 0..size {
        for x in 0..size {
            texture.set(x, y, [roughness, 0, 255, 255]);
        }
    }
    texture
}

fn floor_scene(albedo: &Texture, tiles: f32) -> Scene {
    let mut scene = Scene::empty(Vec3::new(0.5, 0.6, 0.8), 1.0e6);
    let material = scene.add_material(albedo, &flat_normal(8), &packed(8, 220));
    let corners = [Vec3::new(-50.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 200.0), Vec3::new(-50.0, 0.0, 200.0)];
    let uvs = [[0.0, 0.0], [tiles, 0.0], [tiles, tiles * 2.0], [0.0, tiles * 2.0]];
    for indices in [[0usize, 1, 2], [0, 2, 3]] {
        let mut tri = Tri::flat(indices.map(|i| corners[i]), Vec3::new(1.0, 1.0, 1.0));
        tri.n = [Vec3::new(0.0, 1.0, 0.0); 3];
        tri.t = indices.map(|i| uvs[i]);
        tri.material = material;
        scene.tris.push(tri);
    }
    scene
}

fn luminance(pixel: u32) -> f32 {
    (((pixel >> 16) & 255) + ((pixel >> 8) & 255) + (pixel & 255)) as f32 / 3.0
}

fn variance(fb: &Framebuffer, x0: usize, x1: usize, y0: usize, y1: usize) -> f32 {
    let values: Vec<f32> = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).map(|(x, y)| luminance(fb.pixels[y * fb.width + x])).collect();
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / values.len() as f32
}

#[test]
fn nearby_texture_shows_detail_and_distant_texture_is_filtered() {
    let texture = checker(64, [255, 255, 255, 255], [20, 20, 20, 255]);
    let scene = floor_scene(&texture, 50.0);
    let camera = Camera::look_at(Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 0.0, 6.0), 70.0);
    let mut fb = Framebuffer::new(320, 200);
    render(&scene, &camera, &mut fb);
    let near = variance(&fb, 100, 220, 170, 198);
    let far = variance(&fb, 100, 220, 101, 106);
    assert!(near > 100.0, "near variance {}", near);
    assert!(far < near * 0.05, "far variance {} vs near {}", far, near);
}

#[test]
fn albedo_texture_replaces_vertex_colour() {
    let texture = checker(4, [255, 0, 0, 255], [255, 0, 0, 255]);
    let scene = floor_scene(&texture, 1.0);
    let camera = Camera::look_at(Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 0.0, 6.0), 70.0);
    let mut fb = Framebuffer::new(160, 100);
    render(&scene, &camera, &mut fb);
    let pixel = fb.pixels[95 * fb.width + 80];
    let (r, g, b) = ((pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255);
    assert!(r > 80 && g < 40 && b < 40, "pixel {} {} {}", r, g, b);
}

#[test]
fn gpu_textured_render_matches_cpu_reference() {
    let name = if cfg!(target_os = "macos") { "metal" } else { "vulkan" };
    let mut texture = checker(64, [230, 200, 150, 255], [60, 50, 40, 255]);
    for y in 0..64 {
        for x in 0..64 {
            if (x / 8 + y / 8) % 2 == 0 {
                texture.set(x, y, [200, 60, 60, 255]);
            }
        }
    }
    let scene = floor_scene(&texture, 12.0);
    let camera = Camera::look_at(Vec3::new(0.0, 1.5, 0.0), Vec3::new(0.0, 0.0, 6.0), 70.0);
    let (w, h) = (320, 200);
    let mut cpu = engine::gfx::create_headless("cpu", w, h).unwrap();
    let mut gpu = match engine::gfx::create_headless(name, w, h) {
        Ok(gpu) => gpu,
        Err(e) if e.contains("no Vulkan") || e.contains("no Metal") => return,
        Err(e) => panic!("{} failed to start: {}", name, e),
    };
    cpu.upload_scene(&scene).unwrap();
    gpu.upload_scene(&scene).unwrap();
    let a = cpu.render_offscreen(&scene, &camera).unwrap();
    let b = gpu.render_offscreen(&scene, &camera).unwrap();
    a.write_ppm("/tmp/cpu.ppm").unwrap();
    b.write_ppm("/tmp/gpu.ppm").unwrap();
    let mut total = 0u64;
    for (x, y) in a.pixels.iter().zip(&b.pixels) {
        for shift in [0, 8, 16] {
            total += (((x >> shift) & 255) as i64 - ((y >> shift) & 255) as i64).unsigned_abs();
        }
    }
    let mean = total as f64 / (a.pixels.len() * 3) as f64;
    assert!(mean < 8.0, "mean channel difference {}", mean);
}

fn wall_scene(mode: engine::scene::AlphaMode, alpha_left: u8, alpha_right: u8) -> Scene {
    let mut scene = floor_scene(&checker(4, [20, 160, 20, 255], [20, 160, 20, 255]), 8.0);
    let mut albedo = Texture::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            albedo.set(x, y, [230, 230, 230, if x < 4 { alpha_left } else { alpha_right }]);
        }
    }
    let material = scene.add_material_alpha(&albedo, &flat_normal(8), &packed(8, 200), mode);
    let corners = [Vec3::new(-2.0, 0.0, 4.0), Vec3::new(2.0, 0.0, 4.0), Vec3::new(2.0, 3.0, 4.0), Vec3::new(-2.0, 3.0, 4.0)];
    let uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    for indices in [[0usize, 1, 2], [0, 2, 3]] {
        let mut tri = Tri::flat(indices.map(|i| corners[i]), Vec3::new(1.0, 1.0, 1.0));
        tri.n = [Vec3::new(0.0, 0.0, -1.0); 3];
        tri.t = indices.map(|i| uvs[i]);
        tri.material = material;
        scene.tris.push(tri);
    }
    scene
}

fn channels(pixel: u32) -> (u32, u32, u32) {
    ((pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255)
}

fn wall_camera() -> Camera {
    Camera::look_at(Vec3::new(0.0, 1.5, -2.0), Vec3::new(0.0, 1.2, 4.0), 70.0)
}

#[test]
fn cutout_discards_transparent_texels_and_blend_mixes_with_background() {
    let camera = wall_camera();
    let (w, h) = (320, 200);
    let mut cut = Framebuffer::new(w, h);
    render(&wall_scene(engine::scene::AlphaMode::Cutout, 255, 0), &camera, &mut cut);
    let (left, right) = (channels(cut.pixels[90 * w + 190]), channels(cut.pixels[90 * w + 130]));
    assert!(left.0 > 100 && left.2 > 100, "solid half should be pale grey {:?}", left);
    assert!(right.2 > left.2 + 30 || right.0 + 30 < left.0, "cut half should show the green floor/sky behind {:?}", right);

    let mut glass = Framebuffer::new(w, h);
    render(&wall_scene(engine::scene::AlphaMode::Blend, 255, 60), &camera, &mut glass);
    let (opaque, clear) = (channels(glass.pixels[90 * w + 190]), channels(glass.pixels[90 * w + 130]));
    let behind = channels(cut.pixels[90 * w + 130]);
    assert!(opaque.0 > 100, "{:?}", opaque);
    assert!(clear.0 < behind.0 && clear.0 > opaque.0, "glass should sit between background {:?} and pane {:?}: {:?}", behind, opaque, clear);
}

#[test]
fn gpu_alpha_matches_cpu_reference() {
    let name = if cfg!(target_os = "macos") { "metal" } else { "vulkan" };
    let (w, h) = (320, 200);
    for mode in [engine::scene::AlphaMode::Cutout, engine::scene::AlphaMode::Blend] {
        let scene = wall_scene(mode, 255, 70);
        let mut cpu = engine::gfx::create_headless("cpu", w, h).unwrap();
        let mut gpu = match engine::gfx::create_headless(name, w, h) {
            Ok(gpu) => gpu,
            Err(e) if e.contains("no Vulkan") || e.contains("no Metal") => return,
            Err(e) => panic!("{} failed to start: {}", name, e),
        };
        cpu.upload_scene(&scene).unwrap();
        gpu.upload_scene(&scene).unwrap();
        let a = cpu.render_offscreen(&scene, &wall_camera()).unwrap();
        let b = gpu.render_offscreen(&scene, &wall_camera()).unwrap();
        let mut total = 0u64;
        for (x, y) in a.pixels.iter().zip(&b.pixels) {
            let (p, q) = (channels(*x), channels(*y));
            total += (p.0 as i64 - q.0 as i64).unsigned_abs() + (p.1 as i64 - q.1 as i64).unsigned_abs() + (p.2 as i64 - q.2 as i64).unsigned_abs();
        }
        let mean = total as f64 / (a.pixels.len() * 3) as f64;
        assert!(mean < 8.0, "{:?}: mean channel difference {}", mode, mean);
    }
}
