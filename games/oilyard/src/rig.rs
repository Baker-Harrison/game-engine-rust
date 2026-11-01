use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;
use engine::skeleton::{JointTransform, Pose, Quat, Skeleton};

const HIPS_HEIGHT: f32 = 0.92;
const UPPER_LEG: f32 = 0.45;
const LOWER_LEG: f32 = 0.43;
const UPPER_ARM: f32 = 0.29;
const LOWER_ARM: f32 = 0.27;
const ANKLE_CLEARANCE: f32 = 0.09;
const STRIDE: f32 = 0.30;
const STEP_LIFT: f32 = 0.13;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub struct BotRig {
    pub skeleton: Skeleton,
    pub upper_arm: Mesh,
    pub forearm: Mesh,
    pub thigh: Mesh,
    pub shin: Mesh,
}

pub struct BotPose {
    pub transforms: Vec<JointTransform>,
}

pub struct RigInput {
    pub phase: f32,
    pub speed: f32,
    pub aim_pitch: f32,
    pub recoil: f32,
}

impl BotRig {
    pub fn build() -> BotRig {
        let mut skeleton = Skeleton::new();
        skeleton.add("hips", None, v(0.0, HIPS_HEIGHT, 0.0));
        skeleton.add("chest", Some("hips"), v(0.0, 0.30, 0.0));
        skeleton.add("head", Some("chest"), v(0.0, 0.40, 0.0));
        for (side, sign) in [("r", 1.0f32), ("l", -1.0)] {
            skeleton.add(&format!("shoulder_{}", side), Some("chest"), v(0.22 * sign, 0.2, 0.0));
            skeleton.add(&format!("elbow_{}", side), Some(&format!("shoulder_{}", side)), v(0.0, -UPPER_ARM, 0.0));
            skeleton.add(&format!("wrist_{}", side), Some(&format!("elbow_{}", side)), v(0.0, -LOWER_ARM, 0.0));
            skeleton.add(&format!("thigh_{}", side), Some("hips"), v(0.1 * sign, 0.0, 0.0));
            skeleton.add(&format!("knee_{}", side), Some(&format!("thigh_{}", side)), v(0.0, -UPPER_LEG, 0.0));
            skeleton.add(&format!("ankle_{}", side), Some(&format!("knee_{}", side)), v(0.0, -LOWER_LEG, 0.0));
        }
        let cell = 0.05;
        let white = v(1.0, 1.0, 1.0);
        let upper_arm = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, -UPPER_ARM, 0.0), [0.052, 0.052], [0.045, 0.045], 0.02).union(Sdf::sphere(0.058));
        let forearm = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, -LOWER_ARM, 0.0), [0.045, 0.045], [0.036, 0.036], 0.015).union(Sdf::sphere(0.047)).union(Sdf::sphere(0.05).translate(v(0.0, -LOWER_ARM - 0.02, 0.0)));
        let thigh = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, -UPPER_LEG, 0.0), [0.085, 0.09], [0.066, 0.07], 0.03).union(Sdf::sphere(0.09));
        let shin = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, -LOWER_LEG, 0.0), [0.066, 0.07], [0.052, 0.056], 0.025)
            .union(Sdf::sphere(0.068))
            .union(Sdf::round_box(v(0.1, 0.1, 0.26), 0.03).translate(v(0.0, -LOWER_LEG - 0.03, -0.07)));
        BotRig {
            skeleton,
            upper_arm: mesh_sdf(&upper_arm, cell, white),
            forearm: mesh_sdf(&forearm, cell, white),
            thigh: mesh_sdf(&thigh, cell, white),
            shin: mesh_sdf(&shin, cell, white),
        }
    }

    pub fn pose(&self, input: &RigInput) -> BotPose {
        let sk = &self.skeleton;
        let index = |name: &str| sk.index(name).expect("bot joint");
        let motion = input.speed.clamp(0.0, 1.0);
        let mut pose = sk.rest_pose();
        let bob = (input.phase * 2.0).sin().abs() * 0.03 * motion;
        pose.root_offset = v(0.0, -0.07 * motion - bob, 0.0);
        pose.rotations[index("hips")] = Quat::from_axis_angle(v(0.0, 1.0, 0.0), (input.phase).sin() * 0.08 * motion);
        pose.rotations[index("chest")] = Quat::from_axis_angle(v(1.0, 0.0, 0.0), -0.08 * motion).mul(Quat::from_axis_angle(v(0.0, 1.0, 0.0), -(input.phase).sin() * 0.1 * motion));
        pose.rotations[index("head")] = Quat::from_axis_angle(v(1.0, 0.0, 0.0), -input.aim_pitch * 0.6);

        let base = sk.world(&pose);
        let hips = base[index("hips")];
        let chest = base[index("chest")];
        let to_world = |frame: &JointTransform, local: Vec3| frame.position + frame.rotation.rotate(local);

        for (side, sign, offset) in [("r", 1.0f32, 0.0f32), ("l", -1.0, std::f32::consts::PI)] {
            let leg_phase = input.phase + offset;
            let forward = -STRIDE * motion * leg_phase.sin();
            let lift = STEP_LIFT * motion * leg_phase.cos().max(0.0);
            let target = v(0.1 * sign, ANKLE_CLEARANCE + lift, forward);
            let pole = hips.position + v(0.1 * sign, -0.1, -1.0);
            let _ = sk.solve_two_bone(&mut pose, &format!("thigh_{}", side), &format!("knee_{}", side), &format!("ankle_{}", side), target, pole);
        }

        let recoil_back = input.recoil * 0.05;
        let right_grip = to_world(&chest, v(0.17, -0.14, -0.2 + recoil_back));
        let left_grip = to_world(&chest, v(0.02, -0.06, -0.52 + recoil_back * 0.6));
        let right_pole = to_world(&chest, v(0.55, -0.35, 0.15));
        let left_pole = to_world(&chest, v(-0.45, -0.3, -0.2));
        let _ = sk.solve_two_bone(&mut pose, "shoulder_r", "elbow_r", "wrist_r", right_grip, right_pole);
        let _ = sk.solve_two_bone(&mut pose, "shoulder_l", "elbow_l", "wrist_l", left_grip, left_pole);
        BotPose { transforms: sk.world(&pose) }
    }

    pub fn transform(&self, pose: &BotPose, name: &str) -> JointTransform {
        pose.transforms[self.skeleton.index(name).expect("bot joint")]
    }

    pub fn rest_position(&self, name: &str) -> Vec3 {
        let index = self.skeleton.index(name).expect("bot joint");
        self.skeleton.world(&self.skeleton.rest_pose())[index].position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feet_stay_above_ground_and_hands_reach_the_rifle() {
        let rig = BotRig::build();
        for step in 0..32 {
            let phase = step as f32 * std::f32::consts::TAU / 32.0;
            let pose = rig.pose(&RigInput { phase, speed: 1.0, aim_pitch: 0.0, recoil: 0.0 });
            for side in ["r", "l"] {
                let ankle = rig.transform(&pose, &format!("ankle_{}", side)).position;
                assert!(ankle.y > 0.0 && ankle.y < 0.4, "ankle height {} at phase {}", ankle.y, phase);
                let knee = rig.transform(&pose, &format!("knee_{}", side)).position;
                let thigh = rig.transform(&pose, &format!("thigh_{}", side)).position;
                assert!(((knee - thigh).length() - UPPER_LEG).abs() < 1e-3);
            }
            let chest = rig.transform(&pose, "chest");
            let grip = chest.position + chest.rotation.rotate(v(0.17, -0.14, -0.2));
            let wrist = rig.transform(&pose, "wrist_r").position;
            assert!((wrist - grip).length() < 0.02, "right hand missed the grip by {}", (wrist - grip).length());
        }
    }

    #[test]
    fn standing_still_plants_both_feet_on_the_ground() {
        let rig = BotRig::build();
        let pose = rig.pose(&RigInput { phase: 1.3, speed: 0.0, aim_pitch: 0.0, recoil: 0.0 });
        for side in ["r", "l"] {
            let ankle = rig.transform(&pose, &format!("ankle_{}", side)).position;
            assert!((ankle.y - ANKLE_CLEARANCE).abs() < 0.01, "ankle {}", ankle.y);
        }
    }
}

#[cfg(test)]
mod preview {
    use super::*;
    use crate::bodies::BotModel;
    use crate::pose::{push_mesh_bone, Placement};
    use engine::camera::Camera;
    use engine::render::{render, Framebuffer};
    use engine::scene::Scene;

    #[test]
    #[ignore]
    fn render_run_cycle_sheet() {
        let model = BotModel::build();
        let rig = &model.rig;
        let mut scene = Scene::empty(v(0.6, 0.75, 0.9), 200.0);
        let mut tris = Vec::new();
        for step in 0..6 {
            let phase = step as f32 * std::f32::consts::TAU / 6.0;
            let pose = rig.pose(&RigInput { phase, speed: 1.0, aim_pitch: 0.0, recoil: 0.0 });
            let base = Placement::at(v(step as f32 * 1.3 - 3.2, 0.0, 0.0), -1.2);
            let (hips, chest, head) = (rig.transform(&pose, "hips"), rig.transform(&pose, "chest"), rig.transform(&pose, "head"));
            push_mesh_bone(&mut tris, &model.torso, &hips, rig.rest_position("hips"), &base.tint(v(0.34, 0.36, 0.28)));
            push_mesh_bone(&mut tris, &model.vest, &chest, rig.rest_position("chest"), &base);
            push_mesh_bone(&mut tris, &model.head, &head, rig.rest_position("head"), &base);
            push_mesh_bone(&mut tris, &model.helmet, &head, rig.rest_position("head"), &base);
            push_mesh_bone(&mut tris, &model.rifle, &chest, rig.rest_position("chest") - v(0.14, 1.2, -0.3), &base);
            for side in ["r", "l"] {
                let pants = base.tint(v(0.2, 0.22, 0.2));
                push_mesh_bone(&mut tris, &rig.upper_arm, &rig.transform(&pose, &format!("shoulder_{}", side)), Vec3::ZERO, &base.tint(v(0.34, 0.36, 0.28)));
                push_mesh_bone(&mut tris, &rig.forearm, &rig.transform(&pose, &format!("elbow_{}", side)), Vec3::ZERO, &base.tint(v(0.34, 0.36, 0.28)));
                push_mesh_bone(&mut tris, &rig.thigh, &rig.transform(&pose, &format!("thigh_{}", side)), Vec3::ZERO, &pants);
                push_mesh_bone(&mut tris, &rig.shin, &rig.transform(&pose, &format!("knee_{}", side)), Vec3::ZERO, &pants);
            }
        }
        scene.dynamic = tris;
        let camera = Camera::look_at(v(0.0, 1.1, 6.5), v(0.0, 0.9, 0.0), 40.0);
        let mut fb = Framebuffer::new(1200, 360);
        render(&scene, &camera, &mut fb);
        fb.write_ppm("/tmp/rig.ppm").unwrap();
    }
}
