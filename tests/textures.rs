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
