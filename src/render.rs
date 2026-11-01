
use crate::camera::Camera;
use crate::math::{clamp, Vec3};
use crate::scene::{AlphaMode, OverlayItem, OverlayRect, OverlayTri, Scene, SurfaceMaps};
use std::io::Write;

#[derive(Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    depth: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Framebuffer {
        Framebuffer { width, height, pixels: vec![0; width * height], depth: vec![0.0; width * height] }
    }

    pub fn downsampled(&self, factor: usize) -> Framebuffer {
        let mut result = Framebuffer::new(self.width / factor, self.height / factor);
        let samples = (factor * factor) as u32;
        for y in 0..result.height {
            for x in 0..result.width {
                let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
                for sy in 0..factor {
                    for sx in 0..factor {
                        let pixel = self.pixels[(y * factor + sy) * self.width + x * factor + sx];
                        r += (pixel >> 16) & 255;
                        g += (pixel >> 8) & 255;
                        b += pixel & 255;
                    }
                }
                result.pixels[y * result.width + x] = ((r / samples) << 16) | ((g / samples) << 8) | (b / samples);
            }
        }
        result
    }

    pub fn clear(&mut self, color: Vec3) {
        let c = pack(color);
        self.pixels.iter_mut().for_each(|p| *p = c);
        self.depth.iter_mut().for_each(|d| *d = 0.0);
    }

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
    uv_over_z: [f32; 2],
}

#[derive(Clone, Copy)]
struct ViewVertex {
    position: Vec3,
    normal: Vec3,
    color: Vec3,
    uv: [f32; 2],
}

struct TexturedSurface<'a> {
    maps: &'a SurfaceMaps,
    tangent: Vec3,
    bitangent: Vec3,
}

pub const CPU_SHADOW_SIZE: usize = 1024;

struct ShadowMap {
    size: usize,
    depth: Vec<f32>,
    matrix: crate::math::Mat4,
    texel: f32,
}

struct ShadowFrame<'a> {
    map: &'a ShadowMap,
    eye: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
}

impl ShadowMap {
    fn build(scene: &Scene) -> Option<ShadowMap> {
        let uniforms = crate::gfx::shadow_uniforms(scene, false, CPU_SHADOW_SIZE);
        if uniforms.params[0] < 0.5 {
            return None;
        }
        let matrix = crate::math::Mat4(uniforms.light_view_proj);
        let size = CPU_SHADOW_SIZE;
        let mut depth = vec![f32::MAX; size * size];
        let to_map = |p: Vec3| {
            let q = matrix.mul_point(p);
            ((q[0] * 0.5 + 0.5) * size as f32, (q[1] * 0.5 + 0.5) * size as f32, q[2])
        };
        for tri in scene.tris.iter().chain(scene.dynamic.iter()) {
            if tri.unlit || scene.is_blended(tri) {
                continue;
            }
            let (a, b, c) = (to_map(tri.v[0]), to_map(tri.v[1]), to_map(tri.v[2]));
            let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            if area.abs() < 1e-9 {
                continue;
            }
            let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
            let max_x = a.0.max(b.0).max(c.0).ceil().min(size as f32 - 1.0) as i32;
            let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
            let max_y = a.1.max(b.1).max(c.1).ceil().min(size as f32 - 1.0) as i32;
            let inverse = 1.0 / area;
            for y in min_y..=max_y {
                let py = y as f32 + 0.5;
                for x in min_x..=max_x {
                    let px = x as f32 + 0.5;
                    let w0 = ((b.0 - px) * (c.1 - py) - (b.1 - py) * (c.0 - px)) * inverse;
                    let w1 = ((c.0 - px) * (a.1 - py) - (c.1 - py) * (a.0 - px)) * inverse;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                        continue;
                    }
                    let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
                    let slot = &mut depth[y as usize * size + x as usize];
                    if z < *slot {
                        *slot = z;
                    }
                }
            }
        }
        Some(ShadowMap { size, depth, matrix, texel: uniforms.params[1] })
    }

    fn lit(&self, point: Vec3) -> f32 {
        let q = self.matrix.mul_point(point);
        let (u, v, z) = ((q[0] * 0.5 + 0.5) * self.size as f32, (q[1] * 0.5 + 0.5) * self.size as f32, q[2]);
        let (cx, cy) = (u.floor() as i32, v.floor() as i32);
        let mut lit = 0.0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= self.size as i32 || y >= self.size as i32 {
                    lit += 1.0;
                    continue;
                }
                if z - 0.0006 <= self.depth[y as usize * self.size + x as usize] {
                    lit += 1.0;
                }
            }
        }
        lit / 9.0
    }
}

struct PixelLayout {
    tan_half: f32,
    aspect: f32,
    width: f32,
    height: f32,
}

impl ViewVertex {
    fn lerp(self, other: ViewVertex, t: f32) -> ViewVertex {
        ViewVertex {
            position: self.position.lerp(other.position, t),
            normal: self.normal.lerp(other.normal, t),
            color: self.color.lerp(other.color, t),
            uv: [self.uv[0] + (other.uv[0] - self.uv[0]) * t, self.uv[1] + (other.uv[1] - self.uv[1]) * t],
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
            uv_over_z: [v.uv[0] * inv_z, v.uv[1] * inv_z],
        }
    };
    let layout = PixelLayout { tan_half, aspect, width, height };
    let shadow_map = ShadowMap::build(scene);
    let shadow_frame = shadow_map.as_ref().map(|map| ShadowFrame { map, eye: cam.pos, right, up, forward });

    for tri in scene.tris.iter().chain(scene.dynamic.iter()).filter(|t| !scene.is_blended(t)).chain(scene.tris.iter().chain(scene.dynamic.iter()).filter(|t| scene.is_blended(t))) {
        let a = tri.v[0];
        let facing_away = (tri.n[0] + tri.n[1] + tri.n[2]).dot(cam.pos - a) < 0.0;
        let normal_sign = if facing_away { -1.0 } else { 1.0 };

        let corners: [ViewVertex; 3] = std::array::from_fn(|i| ViewVertex {
            position: to_view(tri.v[i]),
            normal: Vec3::new(
                tri.n[i].dot(right),
                tri.n[i].dot(up),
                tri.n[i].dot(forward),
            ) * normal_sign,
            color: tri.c[i],
            uv: tri.t[i],
        });

        let textured = if tri.material > 0 && !tri.unlit {
            scene.materials.get(tri.material as usize - 1).and_then(|maps| {
                let edge1 = corners[1].position - corners[0].position;
                let edge2 = corners[2].position - corners[0].position;
                let (du1, dv1) = (tri.t[1][0] - tri.t[0][0], tri.t[1][1] - tri.t[0][1]);
                let (du2, dv2) = (tri.t[2][0] - tri.t[0][0], tri.t[2][1] - tri.t[0][1]);
                let determinant = du1 * dv2 - du2 * dv1;
                if determinant.abs() < 1e-12 {
                    return None;
                }
                let scale = 1.0 / determinant;
                let tangent = (edge1 * dv2 - edge2 * dv1) * scale;
                let bitangent = (edge2 * du1 - edge1 * du2) * scale;
                Some(TexturedSurface { maps, tangent: tangent.normalized(), bitangent: bitangent.normalized() })
            })
        } else {
            None
        };

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
            raster(fb, first, project(polygon[i]), project(polygon[i + 1]), view_sun(right, up, forward), scene.sky, scene.fog, tri.unlit, textured.as_ref(), &layout, shadow_frame.as_ref());
        }
    }
    draw_overlay(fb, &scene.overlay);
}

fn draw_overlay(fb: &mut Framebuffer, items: &[OverlayItem]) {
    for item in items {
        match item {
            OverlayItem::Rect(rect) => draw_overlay_rect(fb, rect),
            OverlayItem::Tri(tri) => draw_overlay_tri(fb, tri),
        }
    }
}

fn draw_overlay_rect(fb: &mut Framebuffer, rect: &OverlayRect) {
    let (width, height) = (fb.width as f32, fb.height as f32);
    let x0 = (rect.min.0 * width).round().max(0.0) as usize;
    let x1 = ((rect.max.0 * width).round().max(0.0) as usize).min(fb.width);
    let y0 = (rect.min.1 * height).round().max(0.0) as usize;
    let y1 = ((rect.max.1 * height).round().max(0.0) as usize).min(fb.height);
    let top = pack(rect.color);
    for y in y0..y1 {
        for x in x0..x1 {
            let index = y * fb.width + x;
            fb.pixels[index] = if rect.alpha >= 0.999 { top } else { blend(fb.pixels[index], rect.color, rect.alpha) };
        }
    }
}

fn draw_overlay_tri(fb: &mut Framebuffer, tri: &OverlayTri) {
    let (width, height) = (fb.width as f32, fb.height as f32);
    let p = [
        (tri.points[0].0 * width, tri.points[0].1 * height),
        (tri.points[1].0 * width, tri.points[1].1 * height),
        (tri.points[2].0 * width, tri.points[2].1 * height),
    ];
    let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0);
    if area.abs() < 1e-9 {
        return;
    }
    let min_x = p.iter().fold(f32::MAX, |m, q| m.min(q.0)).floor().max(0.0) as i32;
    let max_x = p.iter().fold(f32::MIN, |m, q| m.max(q.0)).ceil().min(width - 1.0) as i32;
    let min_y = p.iter().fold(f32::MAX, |m, q| m.min(q.1)).floor().max(0.0) as i32;
    let max_y = p.iter().fold(f32::MIN, |m, q| m.max(q.1)).ceil().min(height - 1.0) as i32;
    let inverse = 1.0 / area;
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let w0 = ((p[1].0 - px) * (p[2].1 - py) - (p[1].1 - py) * (p[2].0 - px)) * inverse;
            let w1 = ((p[2].0 - px) * (p[0].1 - py) - (p[2].1 - py) * (p[0].0 - px)) * inverse;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let alpha = w0 * tri.alpha[0] + w1 * tri.alpha[1] + w2 * tri.alpha[2];
            if alpha <= 0.002 {
                continue;
            }
            let index = y as usize * fb.width + x as usize;
            fb.pixels[index] = blend(fb.pixels[index], tri.color, alpha.min(1.0));
        }
    }
}

fn blend(under: u32, over: Vec3, alpha: f32) -> u32 {
    let channel = |shift: u32| ((under >> shift) & 255) as f32 / 255.0;
    let below = Vec3::new(channel(16), channel(8), channel(0));
    pack(below.lerp(over, alpha))
}

fn view_sun(right: Vec3, up: Vec3, forward: Vec3) -> Vec3 {
    let sun = SUN.normalized();
    Vec3::new(sun.dot(right), sun.dot(up), sun.dot(forward))
}

fn raster(fb: &mut Framebuffer, a: ScreenVertex, b: ScreenVertex, c: ScreenVertex, sun: Vec3, sky: Vec3, fog: f32, unlit: bool, textured: Option<&TexturedSurface>, layout: &PixelLayout, shadow_frame: Option<&ShadowFrame>) {
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
            let mut normal = (a.normal_over_z * w0 + b.normal_over_z * w1 + c.normal_over_z * w2) / inv_z;
            let mut base = (a.color_over_z * w0 + b.color_over_z * w1 + c.color_over_z * w2) / inv_z;
            let mut roughness = 0.85;
            let mut metallic = 0.0;
            let mut occlusion = 1.0;
            let mut opacity = 1.0;
            let mut blended = false;
            if let Some(surface) = textured {
                let uv_at = |sx: f32, sy: f32| {
                    let v0 = ((b.x - sx) * (c.y - sy) - (b.y - sy) * (c.x - sx)) * inv_area;
                    let v1 = ((c.x - sx) * (a.y - sy) - (c.y - sy) * (a.x - sx)) * inv_area;
                    let v2 = 1.0 - v0 - v1;
                    let z = v0 * a.inv_z + v1 * b.inv_z + v2 * c.inv_z;
                    [
                        (v0 * a.uv_over_z[0] + v1 * b.uv_over_z[0] + v2 * c.uv_over_z[0]) / z,
                        (v0 * a.uv_over_z[1] + v1 * b.uv_over_z[1] + v2 * c.uv_over_z[1]) / z,
                    ]
                };
                let here = uv_at(px, py);
                let right_uv = uv_at(px + 1.0, py);
                let down_uv = uv_at(px, py + 1.0);
                let (tw, th) = surface.maps.albedo.base_size();
                let span = |other: [f32; 2]| ((other[0] - here[0]) * tw as f32).hypot((other[1] - here[1]) * th as f32);
                let lod = span(right_uv).max(span(down_uv)).max(1e-6).log2();
                let albedo = surface.maps.albedo.sample(here[0], here[1], lod);
                let packed = surface.maps.packed.sample(here[0], here[1], lod);
                let texel_normal = surface.maps.normal.sample(here[0], here[1], lod);
                let (nx, ny, nz) = (texel_normal[0] * 2.0 - 1.0, texel_normal[1] * 2.0 - 1.0, texel_normal[2] * 2.0 - 1.0);
                let geometric = normal.normalized();
                let perturbed = surface.tangent * nx + surface.bitangent * ny + geometric * nz;
                normal = if perturbed.length() > 1e-6 { perturbed.normalized() } else { geometric };
                base = Vec3::new(albedo[0], albedo[1], albedo[2]);
                match surface.maps.alpha_mode {
                    AlphaMode::Cutout if albedo[3] < 0.5 => continue,
                    AlphaMode::Blend => {
                        opacity = albedo[3];
                        blended = true;
                    }
                    _ => {}
                }
                roughness = packed[0].max(0.04);
                metallic = packed[1];
                occlusion = packed[2];
            }
            if !blended {
                fb.depth[idx] = inv_z;
            }
            let shadow = match (shadow_frame, unlit) {
                (Some(frame), false) => {
                    let z = 1.0 / inv_z;
                    let vx = ((px / layout.width) - 0.5) * 2.0 * layout.tan_half * layout.aspect * z;
                    let vy = (0.5 - py / layout.height) * 2.0 * layout.tan_half * z;
                    let world = frame.eye + frame.right * vx + frame.up * vy + frame.forward * z;
                    let n = normal.normalized();
                    let world_normal = frame.right * n.x + frame.up * n.y + frame.forward * n.z;
                    frame.map.lit(world + world_normal * (frame.map.texel * 1.5))
                }
                _ => 1.0,
            };
            let shade = if unlit {
                1.0
            } else {
                AMBIENT * occlusion + (1.0 - AMBIENT) * normal.normalized().dot(sun).max(0.0) * shadow
            };
            let mut lit = base * shade;
            if textured.is_some() {
                let ray = Vec3::new(
                    ((px / layout.width) - 0.5) * 2.0 * layout.tan_half * layout.aspect,
                    (0.5 - py / layout.height) * 2.0 * layout.tan_half,
                    1.0,
                );
                let view = ray.normalized() * -1.0;
                let half = (sun + view).normalized();
                let sharpness = 2.0 / (roughness * roughness * roughness * roughness).max(0.0004) - 2.0;
                let highlight = normal.normalized().dot(half).max(0.0).powf(sharpness.min(2000.0));
                let facing = normal.normalized().dot(sun).max(0.0);
                let strength = highlight * facing * (1.0 - roughness) * 0.6 * shadow;
                let tint = Vec3::new(1.0, 1.0, 1.0).lerp(base, metallic);
                lit = base * (1.0 - metallic * 0.8) * shade + tint * strength;
            }
            let t = clamp(1.0 / inv_z / fog, 0.0, 1.0);
            let t = t * t;
            let color = lit.lerp(sky, t);
            fb.pixels[idx] = if blended { blend(fb.pixels[idx], color, opacity) } else { pack(color) };
        }
    }
}
