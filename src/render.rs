//! Software rasterizer: near-plane clipping, perspective-correct depth,
//! z-buffer, flat shading with a directional sun, and distance fog.
//! Pixels are 0x00RRGGBB.

use crate::camera::Camera;
use crate::math::{clamp, Vec3};
use crate::scene::Scene;
use std::io::Write;

#[derive(Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    depth: Vec<f32>, // stores 1/z; larger is nearer, 0 is infinitely far
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer {
        Framebuffer { width, height, pixels: vec![0; width * height], depth: vec![0.0; width * height] }
    }

    pub fn clear(&mut self, color: Vec3) {
        let c = pack(color);
        self.pixels.iter_mut().for_each(|p| *p = c);
        self.depth.iter_mut().for_each(|d| *d = 0.0);
    }

    /// Writes a binary PPM (P6) that agents can open or convert.
    pub fn write_ppm(&self, path: &str) -> std::io::Result<()> {
        let mut out = Vec::with_capacity(self.pixels.len() * 3 + 32);
        write!(out, "P6\n{} {}\n255\n", self.width, self.height)?;
        for p in &self.pixels {
            out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(path, out)
    }
}

fn pack(c: Vec3) -> u32 {
    let q = |v: f32| (clamp(v, 0.0, 1.0) * 255.0 + 0.5) as u32;
    (q(c.x) << 16) | (q(c.y) << 8) | q(c.z)
}

pub const SUN: Vec3 = Vec3::new(0.45, 0.80, 0.40);
pub const AMBIENT: f32 = 0.45;

#[derive(Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    inv_z: f32,
    normal_over_z: Vec3,
    color_over_z: Vec3,
}

#[derive(Clone, Copy)]
struct ViewVertex {
    position: Vec3,
    normal: Vec3,
    color: Vec3,
}

impl ViewVertex {
    fn lerp(self, other: ViewVertex, t: f32) -> ViewVertex {
        ViewVertex {
            position: self.position.lerp(other.position, t),
            normal: self.normal.lerp(other.normal, t),
            color: self.color.lerp(other.color, t),
        }
    }
}

pub fn render(scene: &Scene, cam: &Camera, fb: &mut Framebuffer) {
    fb.clear(scene.sky);
    let right = cam.right();
    let forward = cam.forward();
    let up = cam.up();
    let tan_half = (cam.fov_y * 0.5).tan();
    let aspect = fb.width as f32 / fb.height as f32;
    let (width, height) = (fb.width as f32, fb.height as f32);

    let to_view = |p: Vec3| {
        let d = p - cam.pos;
        Vec3::new(d.dot(right), d.dot(up), d.dot(forward))
    };
    let project = |v: ViewVertex| {
        let inv_z = 1.0 / v.position.z;
        ScreenVertex {
            x: (v.position.x * inv_z / (tan_half * aspect) * 0.5 + 0.5) * width,
            y: (0.5 - v.position.y * inv_z / tan_half * 0.5) * height,
            inv_z,
            normal_over_z: v.normal * inv_z,
            color_over_z: v.color * inv_z,
        }
    };

    for tri in &scene.tris {
        let [a, b, c] = tri.v;
        let facing_away = (b - a).cross(c - a).dot(cam.pos - a) < 0.0;
        let normal_sign = if facing_away { -1.0 } else { 1.0 };

        let corners: [ViewVertex; 3] = std::array::from_fn(|i| ViewVertex {
            position: to_view(tri.v[i]),
            normal: Vec3::new(
                tri.n[i].dot(right),
                tri.n[i].dot(up),
                tri.n[i].dot(forward),
            ) * normal_sign,
            color: tri.c[i],
        });

        let mut polygon = [corners[0]; 4];
        let mut count = 0;
        for i in 0..3 {
            let (current, next) = (corners[i], corners[(i + 1) % 3]);
            let (current_in, next_in) = (current.position.z >= cam.near, next.position.z >= cam.near);
            if current_in {
                polygon[count] = current;
                count += 1;
            }
            if current_in != next_in {
                let t = (cam.near - current.position.z) / (next.position.z - current.position.z);
                polygon[count] = current.lerp(next, t);
                count += 1;
            }
        }
        if count < 3 {
            continue;
        }
        let first = project(polygon[0]);
        for i in 1..count - 1 {
            raster(fb, first, project(polygon[i]), project(polygon[i + 1]), view_sun(right, up, forward), scene.sky, scene.fog);
        }
    }
}

fn view_sun(right: Vec3, up: Vec3, forward: Vec3) -> Vec3 {
    let sun = SUN.normalized();
    Vec3::new(sun.dot(right), sun.dot(up), sun.dot(forward))
}

fn raster(fb: &mut Framebuffer, a: ScreenVertex, b: ScreenVertex, c: ScreenVertex, sun: Vec3, sky: Vec3, fog: f32) {
    let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area.abs() < 1e-8 {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(fb.width as f32 - 1.0) as i32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(fb.height as f32 - 1.0) as i32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    let inv_area = 1.0 / area;
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) * inv_area;
            let w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) * inv_area;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let inv_z = w0 * a.inv_z + w1 * b.inv_z + w2 * c.inv_z;
            let idx = y as usize * fb.width + x as usize;
            if inv_z <= fb.depth[idx] {
                continue;
            }
            fb.depth[idx] = inv_z;
            let normal = (a.normal_over_z * w0 + b.normal_over_z * w1 + c.normal_over_z * w2) / inv_z;
            let base = (a.color_over_z * w0 + b.color_over_z * w1 + c.color_over_z * w2) / inv_z;
            let shade = AMBIENT + (1.0 - AMBIENT) * normal.normalized().dot(sun).max(0.0);
            let t = clamp(1.0 / inv_z / fog, 0.0, 1.0);
            let t = t * t;
            fb.pixels[idx] = pack((base * shade).lerp(sky, t));
        }
    }
}
