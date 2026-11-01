use super::bvh::Bvh;
use super::mesh::Mesh;
use crate::math::Vec3;
use crate::parallel::for_each_chunk;
use crate::random::hash_to_unit;
use crate::render::{AMBIENT, SUN};

#[derive(Clone, Copy)]
pub struct Light {
    pub direction: Vec3,
    pub intensity: f32,
    pub casts_shadow: bool,
}

#[derive(Clone)]
pub struct Lighting {
    pub sky: f32,
    pub ground: f32,
    pub lights: Vec<Light>,
}

impl Lighting {
    pub fn sun() -> Lighting {
        Lighting {
            sky: AMBIENT,
            ground: AMBIENT,
            lights: vec![Light { direction: SUN.normalized(), intensity: 1.0 - AMBIENT, casts_shadow: true }],
        }
    }

    pub fn studio() -> Lighting {
        Lighting {
            sky: 0.55,
            ground: 0.26,
            lights: vec![
                Light { direction: Vec3::new(0.55, 0.75, 0.45).normalized(), intensity: 0.55, casts_shadow: true },
                Light { direction: Vec3::new(-0.8, 0.2, 0.5).normalized(), intensity: 0.25, casts_shadow: false },
                Light { direction: Vec3::new(-0.3, 0.4, -0.85).normalized(), intensity: 0.4, casts_shadow: false },
            ],
        }
    }

    fn hemisphere(&self, normal: Vec3) -> f32 {
        self.ground + (self.sky - self.ground) * (normal.y * 0.5 + 0.5)
    }
}

#[derive(Clone)]
pub struct BakeSettings {
    pub ao_samples: usize,
    pub ao_distance: f32,
    pub ao_strength: f32,
    pub sun_shadows: bool,
    pub lighting: Lighting,
}

impl Default for BakeSettings {
    fn default() -> BakeSettings {
        BakeSettings { ao_samples: 24, ao_distance: 1.2, ao_strength: 0.85, sun_shadows: true, lighting: Lighting::sun() }
    }
}

impl BakeSettings {
    pub fn studio() -> BakeSettings {
        BakeSettings { ao_samples: 64, ao_distance: 0.25, ao_strength: 0.8, sun_shadows: true, lighting: Lighting::studio() }
    }
}

const SURFACE_OFFSET: f32 = 0.004;
const LIGHT_DISTANCE: f32 = 200.0;
const MINIMUM_VISIBILITY: f32 = 0.12;
const MAXIMUM_BOOST: f32 = 3.0;

pub fn bake(mesh: &mut Mesh, occluders: &Bvh, settings: &BakeSettings) {
    let shader_sun = SUN.normalized();
    let mut factors = vec![1.0f32; mesh.positions.len()];
    let positions = &mesh.positions;
    let normals = &mesh.normals;
    for_each_chunk(&mut factors, 1, |start, chunk| {
        for (offset, factor) in chunk.iter_mut().enumerate() {
            let index = start + offset;
            let normal = normals[index];
            let origin = positions[index] + normal * SURFACE_OFFSET;
            let ao = (1.0 - settings.ao_strength * occlusion(occluders, origin, normal, index as u32, settings)).max(MINIMUM_VISIBILITY);
            let mut irradiance = settings.lighting.hemisphere(normal);
            for light in &settings.lighting.lights {
                let facing = normal.dot(light.direction);
                if facing <= 0.0 {
                    continue;
                }
                let blocked = settings.sun_shadows && light.casts_shadow && occluders.any_hit(origin, light.direction, LIGHT_DISTANCE);
                if !blocked {
                    irradiance += light.intensity * facing;
                }
            }
            let shader_shade = AMBIENT + (1.0 - AMBIENT) * normal.dot(shader_sun).max(0.0);
            *factor = (ao * irradiance / shader_shade).min(MAXIMUM_BOOST).max(MINIMUM_VISIBILITY);
        }
    });
    for (color, factor) in mesh.colors.iter_mut().zip(&factors) {
        *color = *color * *factor;
    }
}

fn occlusion(occluders: &Bvh, origin: Vec3, normal: Vec3, seed: u32, settings: &BakeSettings) -> f32 {
    let helper = if normal.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let tangent = normal.cross(helper).normalized();
    let bitangent = normal.cross(tangent);
    let twist = hash_to_unit(seed) * std::f32::consts::TAU;
    let mut blocked = 0;
    for sample in 0..settings.ao_samples {
        let radial = ((sample as f32 + 0.5) / settings.ao_samples as f32).sqrt();
        let angle = sample as f32 * 2.399_963 + twist;
        let (sin, cos) = angle.sin_cos();
        let up = (1.0 - radial * radial).max(0.0).sqrt();
        let direction = tangent * (cos * radial) + bitangent * (sin * radial) + normal * up;
        if occluders.any_hit(origin, direction, settings.ao_distance) {
            blocked += 1;
        }
    }
    blocked as f32 / settings.ao_samples as f32
}
