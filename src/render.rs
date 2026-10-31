//! Software rasterizer: near-plane clipping, perspective-correct depth,
//! z-buffer, flat shading with a directional sun, and distance fog.
//! Pixels are 0x00RRGGBB.

use crate::camera::Camera;
use crate::math::{clamp, Vec3};
use crate::scene::Scene;
use std::io::Write;

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

const SUN: Vec3 = Vec3::new(0.45, 0.80, 0.40);
const AMBIENT: f32 = 0.45;

#[derive(Clone, Copy)]
struct Sv {
    x: f32,
    y: f32,
    inv_z: f32,
}

pub fn render(scene: &Scene, cam: &Camera, fb: &mut Framebuffer) {
    fb.clear(scene.sky);
    let sun = SUN.normalized();
    let right = cam.right();
    let fwd = cam.forward();
    let up = cam.up();
    let tan_half = (cam.fov_y * 0.5).tan();
    let aspect = fb.width as f32 / fb.height as f32;
    let (w, h) = (fb.width as f32, fb.height as f32);

    let to_view = |p: Vec3| {
        let d = p - cam.pos;
        Vec3::new(d.dot(right), d.dot(up), d.dot(fwd))
    };
    let project = |v: Vec3| {
        let inv_z = 1.0 / v.z;
        Sv {
            x: (v.x * inv_z / (tan_half * aspect) * 0.5 + 0.5) * w,
            y: (0.5 - v.y * inv_z / tan_half * 0.5) * h,
            inv_z,
        }
    };

    for tri in &scene.tris {
        let [a, b, c] = tri.v;
        let mut n = (b - a).cross(c - a).normalized();
        if n.dot(cam.pos - a) < 0.0 {
            n = -n; // double-sided: light the side that faces the viewer
        }
        let shade = AMBIENT + (1.0 - AMBIENT) * n.dot(sun).max(0.0);
        let color = tri.color * shade;

        // Clip against the near plane in view space (Sutherland-Hodgman).
        let vs = [to_view(a), to_view(b), to_view(c)];
        let mut poly = [Vec3::ZERO; 4];
        let mut count = 0;
        for i in 0..3 {
            let (cur, next) = (vs[i], vs[(i + 1) % 3]);
            let (in_cur, in_next) = (cur.z >= cam.near, next.z >= cam.near);
            if in_cur {
                poly[count] = cur;
                count += 1;
            }
            if in_cur != in_next {
                let t = (cam.near - cur.z) / (next.z - cur.z);
                poly[count] = cur.lerp(next, t);
                count += 1;
            }
        }
        if count < 3 {
            continue;
        }
        let p0 = project(poly[0]);
        for i in 1..count - 1 {
            raster(fb, p0, project(poly[i]), project(poly[i + 1]), color, scene.sky, scene.fog);
        }
    }
}

fn raster(fb: &mut Framebuffer, a: Sv, b: Sv, c: Sv, color: Vec3, sky: Vec3, fog: f32) {
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
            let t = clamp(1.0 / inv_z / fog, 0.0, 1.0);
            let t = t * t;
            fb.pixels[idx] = pack(color.lerp(sky, t));
        }
    }
}
