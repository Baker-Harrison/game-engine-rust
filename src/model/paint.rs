use super::material::{Material, Surface, SurfaceSample};
use crate::math::Vec3;
use crate::noise::fractal_noise;

const EDGE_RADIUS_SCALE: f32 = 0.0035;
const CAVITY_RADIUS_SCALE: f32 = 0.004;

#[derive(Clone, Copy)]
pub struct SurfacePaint {
    pub base: Vec3,
    pub edge: Vec3,
    pub edge_wear: f32,
    pub cavity_darkening: f32,
    pub grain: f32,
    pub grain_frequency: f32,
    pub seed: u32,
    pub roughness: f32,
    pub metallic: f32,
    pub edge_roughness: f32,
    pub edge_metallic: f32,
}

impl SurfacePaint {
    pub fn plain(base: Vec3) -> SurfacePaint {
        SurfacePaint { base, edge: base, edge_wear: 0.0, cavity_darkening: 0.0, grain: 0.0, grain_frequency: 600.0, seed: 1, roughness: 0.8, metallic: 0.0, edge_roughness: 0.8, edge_metallic: 0.0 }
    }

    pub fn worn(base: Vec3, edge: Vec3) -> SurfacePaint {
        SurfacePaint { base, edge, edge_wear: 1.0, cavity_darkening: 0.5, grain: 0.08, grain_frequency: 500.0, seed: 1, roughness: 0.6, metallic: 0.0, edge_roughness: 0.4, edge_metallic: 0.0 }
    }

    pub fn with_grain(mut self, strength: f32, frequency: f32) -> SurfacePaint {
        self.grain = strength;
        self.grain_frequency = frequency;
        self
    }

    pub fn with_surface(mut self, roughness: f32, metallic: f32) -> SurfacePaint {
        self.roughness = roughness;
        self.metallic = metallic;
        self
    }

    pub fn with_edge_surface(mut self, roughness: f32, metallic: f32) -> SurfacePaint {
        self.edge_roughness = roughness;
        self.edge_metallic = metallic;
        self
    }

    pub fn with_seed(mut self, seed: u32) -> SurfacePaint {
        self.seed = seed;
        self
    }

    pub fn color_at(&self, sample: &SurfaceSample) -> Vec3 {
        self.material_at(sample).albedo
    }

    pub fn material_at(&self, sample: &SurfaceSample) -> Material {
        let p = sample.position;
        let grain = fractal_noise(p.x * self.grain_frequency, p.y * self.grain_frequency, p.z * self.grain_frequency, 2, self.seed);
        let patchy = 0.5 + 0.5 * fractal_noise(p.x * 40.0, p.y * 40.0, p.z * 40.0, 3, self.seed + 17);
        let convex = (sample.curvature * EDGE_RADIUS_SCALE).clamp(0.0, 1.0);
        let wear = (convex * self.edge_wear * (0.55 + 0.9 * patchy)).clamp(0.0, 1.0);
        let concave = (-sample.curvature * CAVITY_RADIUS_SCALE).clamp(0.0, 1.0);
        let shaded = self.base.lerp(self.edge, wear) * (1.0 - concave * self.cavity_darkening);
        Material {
            albedo: shaded * (1.0 + grain * self.grain),
            roughness: self.roughness + (self.edge_roughness - self.roughness) * wear,
            metallic: self.metallic + (self.edge_metallic - self.metallic) * wear,
        }
    }
}

impl Surface for SurfacePaint {
    fn evaluate(&self, sample: &SurfaceSample) -> Material {
        self.material_at(sample)
    }
}
