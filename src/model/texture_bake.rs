use super::material::{Material, Surface, SurfaceSample};
use super::style::{AlphaKind, Style};
use crate::math::Vec3;
use crate::texture::Texture;

#[derive(Clone)]
pub struct MaterialMaps {
    pub albedo: Texture,
    pub normal: Texture,
    pub packed: Texture,
}

const HEIGHT_STRENGTH: f32 = 3.0;
const AMBIENT_OCCLUSION_RADIUS: usize = 3;

fn smooth_weight(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn evaluate_at(surface: &dyn Surface, tile_meters: f32, u: f32, v: f32) -> Material {
    let sample = SurfaceSample { position: Vec3::new(u * tile_meters, 0.0, v * tile_meters), normal: Vec3::new(0.0, 0.35, 0.94).normalized(), curvature: 0.0 };
    surface.evaluate(&sample)
}

fn seamless_material(surface: &dyn Surface, tile_meters: f32, u: f32, v: f32) -> Material {
    let (su, sv) = (smooth_weight(u), smooth_weight(v));
    let here = evaluate_at(surface, tile_meters, u, v);
    let wrapped_u = evaluate_at(surface, tile_meters, u - 1.0, v);
    let wrapped_v = evaluate_at(surface, tile_meters, u, v - 1.0);
    let wrapped_both = evaluate_at(surface, tile_meters, u - 1.0, v - 1.0);
    let top = here.lerp(wrapped_u, su);
    let bottom = wrapped_v.lerp(wrapped_both, su);
    top.lerp(bottom, sv)
}

fn luminance(color: Vec3) -> f32 {
    color.x * 0.299 + color.y * 0.587 + color.z * 0.114
}

fn to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

pub fn bake_surface(surface: &dyn Surface, tile_meters: f32, size: usize) -> MaterialMaps {
    let mut materials = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let (u, v) = ((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
            materials.push(seamless_material(surface, tile_meters, u, v));
        }
    }
    let height: Vec<f32> = materials.iter().map(|m| luminance(m.albedo)).collect();
    let at = |x: i64, y: i64| height[(y.rem_euclid(size as i64) as usize) * size + x.rem_euclid(size as i64) as usize];
    let mut maps = MaterialMaps { albedo: Texture::new(size, size), normal: Texture::new(size, size), packed: Texture::new(size, size) };
    let texel_meters = tile_meters / size as f32;
    for y in 0..size {
        for x in 0..size {
            let material = materials[y * size + x];
            let (xi, yi) = (x as i64, y as i64);
            let dx = (at(xi + 1, yi) - at(xi - 1, yi)) * HEIGHT_STRENGTH * 0.5 / texel_meters * 0.01;
            let dy = (at(xi, yi + 1) - at(xi, yi - 1)) * HEIGHT_STRENGTH * 0.5 / texel_meters * 0.01;
            let normal = Vec3::new(-dx, -dy, 1.0).normalized();
            let mut occlusion_sum = 0.0;
            let mut count = 0.0;
            let radius = AMBIENT_OCCLUSION_RADIUS as i64;
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    occlusion_sum += at(xi + ox, yi + oy);
                    count += 1.0;
                }
            }
            let local_mean = occlusion_sum / count;
            let cavity = (1.0 - (local_mean - at(xi, yi)).max(0.0) * 2.5).clamp(0.5, 1.0);
            maps.albedo.set(x, y, [to_byte(material.albedo.x), to_byte(material.albedo.y), to_byte(material.albedo.z), 255]);
            maps.normal.set(x, y, [to_byte(normal.x * 0.5 + 0.5), to_byte(normal.y * 0.5 + 0.5), to_byte(normal.z * 0.5 + 0.5), 255]);
            maps.packed.set(x, y, [to_byte(material.roughness), to_byte(material.metallic), to_byte(cavity), 255]);
        }
    }
    maps
}

impl Style {
    pub fn bake_material_maps(&self, name: &str, size: usize) -> Result<MaterialMaps, String> {
        let surface = self.material(name)?;
        let mut maps = bake_surface(&*surface, self.meters_per_tile, size);
        if let Some(kind) = self.alpha.get(name) {
            apply_alpha(&mut maps, *kind);
        }
        Ok(maps)
    }
}

fn leaf_coverage(u: f32, v: f32) -> f32 {
    let half_width = 0.5 * (std::f32::consts::PI * v.clamp(0.0, 1.0).powf(0.8)).sin().max(0.0).powf(0.7);
    let inside = (u - 0.5).abs() < half_width;
    let midrib = (u - 0.5).abs() < 0.015 && v < 0.98;
    if inside || midrib { 1.0 } else { 0.0 }
}

fn apply_alpha(maps: &mut MaterialMaps, kind: AlphaKind) {
    let size = maps.albedo.width;
    for y in 0..size {
        for x in 0..size {
            let (u, v) = ((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
            let alpha = match kind {
                AlphaKind::LeafCutout => leaf_coverage(u, v),
                AlphaKind::GlassBlend(value) => value,
            };
            let mut texel = maps.albedo.get(x, y);
            texel[3] = to_byte(alpha);
            maps.albedo.set(x, y, texel);
        }
    }
}
