use super::material::{dominant_plane, smoothstep, Material, Surface, SurfaceSample};
use crate::math::Vec3;
use crate::noise::{fractal_noise, value_noise};
use crate::random::hash_to_unit;

#[derive(Clone, Copy, Debug)]
pub struct Weathering {
    pub moss: f32,
    pub grime: f32,
    pub edge_wear: f32,
    pub moss_color: Vec3,
    pub grime_color: Vec3,
    pub seed: u32,
}

impl Weathering {
    pub fn none() -> Weathering {
        Weathering { moss: 0.0, grime: 0.0, edge_wear: 0.0, moss_color: Vec3::new(0.2, 0.3, 0.1), grime_color: Vec3::new(0.08, 0.07, 0.06), seed: 0 }
    }

    pub fn apply(&self, base: Material, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let up = sample.normal.y.max(0.0);
        let patches = fractal_noise(p.x * 2.2, p.y * 2.2, p.z * 2.2, 4, self.seed) * 0.5 + 0.5;
        let fine = value_noise(p.x * 60.0, p.y * 60.0, p.z * 60.0, self.seed + 1);
        let cavity = (-sample.curvature * 0.004).clamp(0.0, 1.0);
        let moss_amount = smoothstep(1.0 - self.moss, 1.0 - self.moss + 0.18, patches + 0.4 * up + cavity * 0.35 + 0.08 * fine - 0.25);
        let streaks = fractal_noise(p.x * 5.0, p.y * 0.35, p.z * 5.0, 3, self.seed + 2) * 0.5 + 0.5;
        let grime_amount = (self.grime * (streaks * 0.8 + cavity * 0.6 + (1.0 - up) * 0.2)).clamp(0.0, 0.9);
        let edge = (sample.curvature * 0.004).clamp(0.0, 1.0) * self.edge_wear;
        let worn = Material { albedo: base.albedo * (1.0 + 0.35 * edge), roughness: (base.roughness - 0.1 * edge).max(0.2), metallic: base.metallic };
        let grimy = worn.lerp(Material::new(self.grime_color, 0.95, 0.0), grime_amount);
        let moss_surface = Material::new(self.moss_color * (0.75 + 0.5 * fine), 0.96, 0.0);
        grimy.lerp(moss_surface, moss_amount * (1.0 - 0.5 * edge))
    }
}

#[derive(Clone, Copy)]
pub struct StoneBlocks {
    pub stone: Vec3,
    pub mortar: Vec3,
    pub block_size: (f32, f32),
    pub joint_width: f32,
    pub weathering: Weathering,
    pub seed: u32,
}

impl Surface for StoneBlocks {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let (u, v) = dominant_plane(sample.position, sample.normal);
        let row = (v / self.block_size.1).floor();
        let row_shift = hash_to_unit(row as i32 as u32 ^ self.seed) * self.block_size.0;
        let column = ((u + row_shift) / self.block_size.0).floor();
        let local_u = (u + row_shift) - column * self.block_size.0;
        let local_v = v - row * self.block_size.1;
        let edge = local_u.min(self.block_size.0 - local_u).min(local_v).min(self.block_size.1 - local_v);
        let block_id = (row as i32).wrapping_mul(7919).wrapping_add((column as i32).wrapping_mul(104729)) as u32 ^ self.seed;
        let tone = 0.8 + 0.4 * hash_to_unit(block_id);
        let chips = fractal_noise(u * 9.0, v * 9.0, sample.position.y * 9.0, 3, self.seed + 7);
        let pits = value_noise(u * 140.0, v * 140.0, sample.position.y * 140.0, self.seed + 8);
        let rounded = smoothstep(0.0, self.joint_width * 2.0, edge);
        let base = if edge < self.joint_width {
            Material::new(self.mortar * (0.85 + 0.3 * pits), 0.95, 0.0)
        } else {
            let tint = Vec3::new(1.0 + 0.05 * chips, 1.0, 1.0 - 0.06 * chips);
            Material::new(self.stone.mul_elements(tint) * (tone * (0.92 + 0.12 * pits) * (0.85 + 0.15 * rounded)), 0.88, 0.0)
        };
        self.weathering.apply(base, sample)
    }
}

#[derive(Clone, Copy)]
pub struct Cobblestone {
    pub stone: Vec3,
    pub gap: Vec3,
    pub cell: f32,
    pub weathering: Weathering,
    pub seed: u32,
}

impl Surface for Cobblestone {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let (u, v) = dominant_plane(sample.position, sample.normal);
        let (gu, gv) = (u / self.cell, v / self.cell);
        let (cu, cv) = (gu.floor(), gv.floor());
        let mut nearest = (f32::MAX, f32::MAX, 0u32);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (nx, ny) = (cu + dx as f32, cv + dy as f32);
                let id = (nx as i32).wrapping_mul(73856093) as u32 ^ (ny as i32).wrapping_mul(19349663) as u32 ^ self.seed;
                let point = (nx + 0.2 + 0.6 * hash_to_unit(id), ny + 0.2 + 0.6 * hash_to_unit(id.wrapping_add(1)));
                let distance = ((gu - point.0).powi(2) + (gv - point.1).powi(2)).sqrt();
                if distance < nearest.0 {
                    nearest = (distance, nearest.0, id);
                } else if distance < nearest.1 {
                    nearest.1 = distance;
                }
            }
        }
        let ridge = nearest.1 - nearest.0;
        let tone = 0.75 + 0.5 * hash_to_unit(nearest.2);
        let pits = value_noise(u * 120.0, v * 120.0, 0.0, self.seed + 3);
        let base = if ridge < 0.08 {
            Material::new(self.gap * (0.8 + 0.4 * pits), 0.97, 0.0)
        } else {
            Material::new(self.stone * (tone * (0.9 + 0.2 * pits) * (0.8 + 0.2 * smoothstep(0.08, 0.35, ridge))), 0.8, 0.0)
        };
        self.weathering.apply(base, sample)
    }
}

#[derive(Clone, Copy)]
pub struct Slate {
    pub slate: Vec3,
    pub tile_size: (f32, f32),
    pub weathering: Weathering,
    pub seed: u32,
}

impl Surface for Slate {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let (u, v) = dominant_plane(sample.position, sample.normal);
        let row = (v / self.tile_size.1).floor();
        let shift = if row as i64 % 2 == 0 { 0.0 } else { self.tile_size.0 * 0.5 };
        let column = ((u + shift) / self.tile_size.0).floor();
        let local_v = (v / self.tile_size.1) - row;
        let id = (row as i32).wrapping_mul(2654435761u32 as i32).wrapping_add(column as i32) as u32 ^ self.seed;
        let tone = 0.75 + 0.5 * hash_to_unit(id);
        let overlap_shadow = 0.7 + 0.3 * smoothstep(0.0, 0.25, local_v);
        let flake = fractal_noise(u * 25.0, v * 25.0, 0.0, 3, self.seed + 4) * 0.5 + 0.5;
        let base = Material::new(self.slate * (tone * overlap_shadow * (0.85 + 0.3 * flake)), 0.55, 0.0);
        self.weathering.apply(base, sample)
    }
}

#[derive(Clone, Copy)]
pub struct WroughtIron {
    pub iron: Vec3,
    pub rust: Vec3,
    pub rust_amount: f32,
    pub seed: u32,
}

impl Surface for WroughtIron {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let blotch = fractal_noise(p.x * 7.0, p.y * 7.0, p.z * 7.0, 4, self.seed) * 0.5 + 0.5;
        let hammer = value_noise(p.x * 90.0, p.y * 90.0, p.z * 90.0, self.seed + 1);
        let cavity = (-sample.curvature * 0.004).clamp(0.0, 1.0);
        let coverage = smoothstep(1.0 - self.rust_amount, 1.0 - self.rust_amount + 0.2, blotch + cavity * 0.4);
        let metal = Material::new(self.iron * (0.85 + 0.3 * hammer), 0.5, 0.85);
        metal.lerp(Material::new(self.rust * (0.8 + 0.4 * hammer), 0.9, 0.0), coverage)
    }
}

#[derive(Clone, Copy)]
pub struct PaintedPlaster {
    pub paint: Vec3,
    pub plaster: Vec3,
    pub peel: f32,
    pub weathering: Weathering,
    pub seed: u32,
}

impl Surface for PaintedPlaster {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let peel_mask = fractal_noise(p.x * 1.6, p.y * 1.6, p.z * 1.6, 5, self.seed) * 0.5 + 0.5;
        let exposed = smoothstep(1.0 - self.peel, 1.0 - self.peel + 0.06, peel_mask);
        let brush = value_noise(p.x * 14.0, p.y * 3.0, p.z * 14.0, self.seed + 2);
        let grain = value_noise(p.x * 150.0, p.y * 150.0, p.z * 150.0, self.seed + 3);
        let painted = Material::new(self.paint * (0.93 + 0.1 * brush) * (0.97 + 0.05 * grain), 0.7, 0.0);
        let bare = Material::new(self.plaster * (0.9 + 0.2 * grain), 0.95, 0.0);
        self.weathering.apply(painted.lerp(bare, exposed), sample)
    }
}

#[derive(Clone, Copy)]
pub struct Bark {
    pub bark: Vec3,
    pub groove: Vec3,
    pub ridge_frequency: f32,
    pub weathering: Weathering,
    pub seed: u32,
}

impl Surface for Bark {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let angle = p.z.atan2(p.x);
        let warp = fractal_noise(p.x * 3.0, p.y * 1.2, p.z * 3.0, 3, self.seed) * 0.8;
        let ridge = ((angle * self.ridge_frequency + warp * 4.0).sin() * 0.5 + 0.5).powf(1.5);
        let flake = value_noise(p.x * 40.0, p.y * 12.0, p.z * 40.0, self.seed + 1);
        let base = Material::new(self.groove.lerp(self.bark, ridge) * (0.85 + 0.3 * flake), 0.95, 0.0);
        self.weathering.apply(base, sample)
    }
}

#[derive(Clone, Copy)]
pub struct Skin {
    pub base: Vec3,
    pub flush: Vec3,
    pub seed: u32,
}

impl Surface for Skin {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let mottling = fractal_noise(p.x * 14.0, p.y * 14.0, p.z * 14.0, 3, self.seed) * 0.5 + 0.5;
        let pores = value_noise(p.x * 400.0, p.y * 400.0, p.z * 400.0, self.seed + 1);
        let cavity = (-sample.curvature * 0.01).clamp(0.0, 1.0);
        let tone = self.base.lerp(self.flush, (mottling * 0.35 + cavity * 0.5).clamp(0.0, 1.0));
        Material::new(tone * (0.96 + 0.06 * pores), 0.55 + 0.1 * pores, 0.0)
    }
}

#[derive(Clone, Copy)]
pub struct PaintedMetal {
    pub paint: Vec3,
    pub rust: Vec3,
    pub chipping: f32,
    pub streaks: f32,
    pub weathering: Weathering,
    pub seed: u32,
}

impl Surface for PaintedMetal {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let fade = fractal_noise(p.x * 0.9, p.y * 0.9, p.z * 0.9, 3, self.seed) * 0.5 + 0.5;
        let drips = fractal_noise(p.x * 14.0, p.y * 0.5, p.z * 14.0, 3, self.seed + 1) * 0.5 + 0.5;
        let chips = value_noise(p.x * 24.0, p.y * 24.0, p.z * 24.0, self.seed + 2);
        let edge = (sample.curvature * 0.004).clamp(0.0, 1.0);
        let bare = smoothstep(1.0 - self.chipping, 1.0 - self.chipping + 0.08, chips * 0.6 + fade * 0.2 + edge * 0.5 + (1.0 - fade) * 0.1);
        let painted = Material::new(self.paint * (0.8 + 0.3 * fade) * (1.0 - 0.35 * self.streaks * drips), 0.55, 0.15);
        let rusted = Material::new(self.rust * (0.7 + 0.5 * value_noise(p.x * 60.0, p.y * 60.0, p.z * 60.0, self.seed + 3)), 0.88, 0.25);
        self.weathering.apply(painted.lerp(rusted, bare), sample)
    }
}

#[derive(Clone, Copy)]
pub struct Sandbag {
    pub burlap: Vec3,
    pub seed: u32,
}

impl Surface for Sandbag {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let weave = ((p.x * 220.0).sin() * (p.y * 220.0).sin() + (p.z * 220.0).sin() * (p.y * 220.0).sin()) * 0.05;
        let stain = fractal_noise(p.x * 3.0, p.y * 3.0, p.z * 3.0, 3, self.seed) * 0.5 + 0.5;
        Material::new(self.burlap * (0.75 + 0.35 * stain + weave), 0.97, 0.0)
    }
}
