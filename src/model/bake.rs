use super::bvh::Bvh;
use super::mesh::Mesh;
use crate::math::Vec3;
use crate::parallel::for_each_chunk;
use crate::random::hash_to_unit;
use crate::render::{AMBIENT, SUN};

pub struct BakeSettings {
    pub ao_samples: usize,
    pub ao_distance: f32,
    pub ao_strength: f32,
    pub sun_shadows: bool,
}

impl Default for BakeSettings {
    fn default() -> BakeSettings {
        BakeSettings { ao_samples: 24, ao_distance: 1.2, ao_strength: 0.85, sun_shadows: true }
    }
}

const SURFACE_OFFSET: f32 = 0.004;
const SUN_DISTANCE: f32 = 200.0;
const MINIMUM_VISIBILITY: f32 = 0.12;

pub fn bake(mesh: &mut Mesh, occluders: &Bvh, settings: &BakeSettings) {
    let sun = SUN.normalized();
    let mut factors = vec![1.0f32; mesh.positions.len()];
    let positions = &mesh.positions;
    let normals = &mesh.normals;
    for_each_chunk(&mut factors, 1, |start, chunk| {
        for (offset, factor) in chunk.iter_mut().enumerate() {
            let index = start + offset;
            let normal = normals[index];
            let origin = positions[index] + normal * SURFACE_OFFSET;
            let mut value = 1.0 - settings.ao_strength * occlusion(occluders, origin, normal, index as u32, settings);
            if settings.sun_shadows {
                let facing = normal.dot(sun);
                if facing > 0.0 && occluders.any_hit(origin, sun, SUN_DISTANCE) {
                    let lit = AMBIENT + (1.0 - AMBIENT) * facing;
                    value *= AMBIENT / lit;
                }
            }
            *factor = value.max(MINIMUM_VISIBILITY);
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
