use crate::math::Vec3;
use crate::noise::{fractal_noise, value_noise};

pub struct SurfaceSample {
    pub position: Vec3,
    pub normal: Vec3,
    pub curvature: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub albedo: Vec3,
    pub roughness: f32,
    pub metallic: f32,
}

impl Material {
    pub fn new(albedo: Vec3, roughness: f32, metallic: f32) -> Material {
        Material { albedo, roughness, metallic }
    }

    pub fn with_roughness(mut self, roughness: f32) -> Material {
        self.roughness = roughness;
        self
    }

    pub fn lerp(self, other: Material, t: f32) -> Material {
        Material {
            albedo: self.albedo.lerp(other.albedo, t),
            roughness: self.roughness + (other.roughness - self.roughness) * t,
            metallic: self.metallic + (other.metallic - self.metallic) * t,
        }
    }
}

pub trait Surface: Sync {
    fn evaluate(&self, sample: &SurfaceSample) -> Material;
}

impl<F: Fn(&SurfaceSample) -> Material + Sync> Surface for F {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        self(sample)
    }
}

pub type Paint<'a> = &'a (dyn Fn(&SurfaceSample) -> Vec3 + Sync);

pub(super) fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn dominant_plane(position: Vec3, normal: Vec3) -> (f32, f32) {
    let n = normal.abs();
    if n.y >= n.x && n.y >= n.z {
        (position.x, position.z)
    } else if n.x >= n.z {
        (position.z, position.y)
    } else {
        (position.x, position.y)
    }
}

#[derive(Clone, Copy)]
pub struct Plain {
    pub material: Material,
}

impl Surface for Plain {
    fn evaluate(&self, _: &SurfaceSample) -> Material {
        self.material
    }
}

#[derive(Clone, Copy)]
pub struct WoodGrain {
    pub light: Vec3,
    pub dark: Vec3,
    pub grain_axis: Vec3,
    pub scale: f32,
    pub roughness: f32,
    pub seed: u32,
}

impl Surface for WoodGrain {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let along = self.grain_axis.normalized();
        let across = p - along * (p.dot(along) * 0.92);
        let warp = fractal_noise(across.x * self.scale * 0.6, across.y * self.scale * 0.6, across.z * self.scale * 0.6, 3, self.seed);
        let rings = ((across.x * across.x + across.z * across.z).sqrt() * self.scale * 2.0 + warp * 2.5).sin() * 0.5 + 0.5;
        let streak = fractal_noise(across.x * self.scale * 3.0, across.y * self.scale * 3.0, across.z * self.scale * 3.0, 3, self.seed + 5);
        let tone = (rings * 0.55 + (streak * 0.5 + 0.5) * 0.45).clamp(0.0, 1.0);
        let albedo = self.light.lerp(self.dark, smoothstep(0.2, 0.85, tone));
        Material { albedo, roughness: self.roughness + 0.1 * (streak * 0.5 + 0.5), metallic: 0.0 }
    }
}

#[derive(Clone, Copy)]
pub struct Plaster {
    pub base: Vec3,
    pub variation: f32,
    pub seed: u32,
}

impl Surface for Plaster {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let cloud = fractal_noise(p.x * 2.5, p.y * 2.5, p.z * 2.5, 4, self.seed);
        let grain = value_noise(p.x * 180.0, p.y * 180.0, p.z * 180.0, self.seed + 3);
        let tint = 1.0 + self.variation * cloud + 0.03 * grain;
        Material { albedo: self.base * tint, roughness: 0.93, metallic: 0.0 }
    }
}

#[derive(Clone, Copy)]
pub struct BrickPattern {
    pub brick: Vec3,
    pub mortar: Vec3,
    pub brick_size: (f32, f32),
    pub mortar_width: f32,
    pub seed: u32,
}

impl Surface for BrickPattern {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let (u, v) = dominant_plane(sample.position, sample.normal);
        let row = (v / self.brick_size.1).floor();
        let offset = if row as i64 % 2 == 0 { 0.0 } else { self.brick_size.0 * 0.5 };
        let column = ((u + offset) / self.brick_size.0).floor();
        let local_u = (u + offset) - column * self.brick_size.0;
        let local_v = v - row * self.brick_size.1;
        let edge = local_u.min(self.brick_size.0 - local_u).min(local_v).min(self.brick_size.1 - local_v);
        let is_mortar = edge < self.mortar_width;
        let tone = 0.85 + 0.3 * crate::random::hash_to_unit((row as i32 * 131 + column as i32 * 17) as u32 ^ self.seed);
        let speckle = 0.9 + 0.2 * value_noise(u * 120.0, v * 120.0, 0.0, self.seed);
        if is_mortar {
            Material::new(self.mortar * speckle, 0.95, 0.0)
        } else {
            Material::new(self.brick * tone * speckle, 0.85, 0.0)
        }
    }
}

#[derive(Clone, Copy)]
pub struct Concrete {
    pub base: Vec3,
    pub stain: f32,
    pub seed: u32,
}

impl Surface for Concrete {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let blotches = fractal_noise(p.x * 1.2, p.y * 1.2, p.z * 1.2, 4, self.seed);
        let pores = value_noise(p.x * 220.0, p.y * 220.0, p.z * 220.0, self.seed + 1);
        let streaks = fractal_noise(p.x * 3.0, p.y * 0.4, p.z * 3.0, 2, self.seed + 2);
        let tone = 1.0 + self.stain * (blotches * 0.7 + streaks * 0.5) + 0.04 * pores;
        Material::new(self.base * tone, 0.9, 0.0)
    }
}

#[derive(Clone, Copy)]
pub struct Sand {
    pub light: Vec3,
    pub dark: Vec3,
    pub seed: u32,
}

impl Surface for Sand {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let ripples = ((p.x * 1.7 + p.z * 0.6) * 3.0 + 3.0 * fractal_noise(p.x * 0.5, 0.0, p.z * 0.5, 3, self.seed)).sin() * 0.5 + 0.5;
        let patches = fractal_noise(p.x * 0.15, 0.0, p.z * 0.15, 4, self.seed + 4);
        let grain = value_noise(p.x * 200.0, p.y * 200.0, p.z * 200.0, self.seed + 9);
        let tone = (0.5 + patches * 0.6 + ripples * 0.15).clamp(0.0, 1.0);
        Material::new(self.light.lerp(self.dark, tone) * (1.0 + 0.04 * grain), 0.97, 0.0)
    }
}

#[derive(Clone, Copy)]
pub struct Rusted {
    pub metal: Material,
    pub rust: Vec3,
    pub amount: f32,
    pub seed: u32,
}

impl Surface for Rusted {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let blotch = fractal_noise(p.x * 6.0, p.y * 6.0, p.z * 6.0, 4, self.seed) * 0.5 + 0.5;
        let edge = (sample.curvature * 0.003).clamp(0.0, 0.5);
        let coverage = smoothstep(1.0 - self.amount, 1.0 - self.amount + 0.2, blotch + edge * 0.5);
        let rusty = Material::new(self.rust * (0.8 + 0.4 * value_noise(p.x * 90.0, p.y * 90.0, p.z * 90.0, self.seed + 2)), 0.9, 0.0);
        self.metal.lerp(rusty, coverage)
    }
}

#[derive(Clone, Copy)]
pub struct Tiles {
    pub tile: Vec3,
    pub grout: Vec3,
    pub size: f32,
    pub grout_width: f32,
    pub seed: u32,
}

impl Surface for Tiles {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let (u, v) = dominant_plane(sample.position, sample.normal);
        let (cu, cv) = ((u / self.size).floor(), (v / self.size).floor());
        let (lu, lv) = (u - cu * self.size, v - cv * self.size);
        let edge = lu.min(self.size - lu).min(lv).min(self.size - lv);
        if edge < self.grout_width {
            return Material::new(self.grout, 0.95, 0.0);
        }
        let tone = 0.92 + 0.16 * crate::random::hash_to_unit((cu as i32 * 7919 + cv as i32 * 104729) as u32 ^ self.seed);
        Material::new(self.tile * tone, 0.35, 0.0)
    }
}

#[derive(Clone, Copy)]
pub struct Fabric {
    pub base: Vec3,
    pub weave_frequency: f32,
    pub seed: u32,
}

impl Surface for Fabric {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        let (u, v) = dominant_plane(sample.position, sample.normal);
        let weave = ((u * self.weave_frequency).sin() * (v * self.weave_frequency).sin()) * 0.5 + 0.5;
        let fuzz = fractal_noise(u * 30.0, v * 30.0, 0.0, 2, self.seed);
        Material::new(self.base * (0.92 + 0.1 * weave + 0.05 * fuzz), 0.92, 0.0)
    }
}
