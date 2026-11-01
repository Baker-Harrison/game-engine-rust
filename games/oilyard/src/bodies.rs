use crate::rig::BotRig;
use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;

pub const STAND_HEIGHT: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;
pub const BODY_RADIUS: f32 = 0.3;
pub const HEAD_CENTER: f32 = 1.62;
pub const HEAD_RADIUS: f32 = 0.17;
pub const CHEST_HEIGHT: f32 = 1.2;
pub const SHOULDER_HEIGHT: f32 = 1.42;
pub const HIP_HEIGHT: f32 = 0.9;
pub const LEG_OFFSET: f32 = 0.1;
pub const ARM_OFFSET: f32 = 0.25;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub struct BotModel {
    pub torso: Mesh,
    pub vest: Mesh,
    pub head: Mesh,
    pub helmet: Mesh,
    pub arm: Mesh,
    pub leg: Mesh,
    pub rifle: Mesh,
    pub rig: BotRig,
}

impl BotModel {
    pub fn build() -> BotModel {
        let cell = 0.055;
        let torso = Sdf::round_box(v(0.4, 0.5, 0.22), 0.05).translate(v(0.0, 1.17, 0.0));
        let hips = Sdf::round_box(v(0.36, 0.2, 0.22), 0.05).translate(v(0.0, 0.93, 0.0));
        let vest = Sdf::round_box(v(0.43, 0.34, 0.27), 0.05).translate(v(0.0, 1.24, 0.0));
        let head = Sdf::sphere(0.105).translate(v(0.0, HEAD_CENTER, 0.0));
        let neck = Sdf::limb(v(0.0, 1.42, 0.0), v(0.0, 1.54, 0.0), [0.05, 0.05], [0.05, 0.05], 0.02);
        let helmet = Sdf::round_box(v(0.27, 0.13, 0.3), 0.05).translate(v(0.0, 1.69, 0.0));
        let arm = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, -0.58, 0.0), [0.055, 0.055], [0.043, 0.043], 0.02);
        let leg = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, -0.88, 0.0), [0.085, 0.09], [0.058, 0.065], 0.03);
        let boot = Sdf::round_box(v(0.1, 0.1, 0.24), 0.03).translate(v(0.0, -0.84, -0.05));
        let mut rifle = Mesh::new();
        rifle.add_box(v(0.0, 0.0, -0.1), v(0.05, 0.09, 0.5), v(0.12, 0.12, 0.13));
        rifle.add_box(v(0.0, 0.015, -0.5), v(0.026, 0.026, 0.4), v(0.07, 0.07, 0.08));
        rifle.add_box(v(0.0, -0.1, -0.05), v(0.035, 0.14, 0.06), v(0.1, 0.1, 0.1));
        rifle.add_box(v(0.0, 0.01, 0.2), v(0.045, 0.08, 0.2), v(0.1, 0.1, 0.11));
        BotModel {
            torso: mesh_sdf(&torso.union(hips), cell, v(1.0, 1.0, 1.0)),
            vest: mesh_sdf(&vest, cell, v(1.0, 1.0, 1.0)),
            head: mesh_sdf(&head.union(neck), cell, v(0.78, 0.6, 0.48)),
            helmet: mesh_sdf(&helmet, cell, v(0.3, 0.32, 0.26)),
            arm: mesh_sdf(&arm, cell, v(1.0, 1.0, 1.0)),
            leg: mesh_sdf(&leg.union(boot), cell, v(1.0, 1.0, 1.0)),
            rifle,
            rig: BotRig::build(),
        }
    }

    #[cfg(test)]
    pub fn triangle_count(&self) -> usize {
        [&self.torso, &self.vest, &self.head, &self.helmet, &self.arm, &self.leg, &self.rifle]
            .iter()
            .map(|mesh| mesh.triangle_count())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_parts_stay_within_a_human_sized_box() {
        let model = BotModel::build();
        let parts = [("torso", &model.torso), ("vest", &model.vest), ("head", &model.head), ("helmet", &model.helmet), ("arm", &model.arm), ("leg", &model.leg), ("rifle", &model.rifle)];
        for (name, mesh) in parts {
            let (low, high) = mesh.bounds();
            println!("{} {} tris {:?} {:?}", name, mesh.triangle_count(), low, high);
            assert!(mesh.triangle_count() > 0, "{} is empty", name);
            assert!((high - low).max_element() < 2.0, "{} is oversized", name);
        }
        println!("total {}", model.triangle_count());
    }
}
