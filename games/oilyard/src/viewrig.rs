use crate::pose::{push_mesh_bone, Placement};
use crate::weapons::WeaponKind;
use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;
use engine::scene::Tri;
use engine::skeleton::{JointTransform, Quat, Skeleton};

const UPPER_ARM: f32 = 0.30;
const FOREARM: f32 = 0.28;
const SLEEVE: Vec3 = Vec3::new(0.24, 0.27, 0.19);
const GLOVE: Vec3 = Vec3::new(0.1, 0.1, 0.11);

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub struct WeaponSockets {
    pub right_grip: Vec3,
    pub left_grip: Vec3,
    pub magazine: Vec3,
}

pub fn sockets(kind: WeaponKind) -> WeaponSockets {
    match kind {
        WeaponKind::Carbine => WeaponSockets { right_grip: v(0.0, -0.115, 0.05), left_grip: v(-0.005, -0.06, -0.27), magazine: v(0.0, -0.11, -0.07) },
        WeaponKind::Scattergun => WeaponSockets { right_grip: v(0.0, -0.12, 0.0), left_grip: v(0.0, -0.06, -0.5), magazine: v(0.0, -0.07, -0.12) },
        WeaponKind::Sidearm => WeaponSockets { right_grip: v(0.0, -0.1, 0.015), left_grip: v(-0.03, -0.12, 0.03), magazine: v(0.0, -0.15, 0.03) },
    }
}

pub struct ViewArms {
    skeleton: Skeleton,
    upper: Mesh,
    fore: Mesh,
    glove: Mesh,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

pub struct ReloadFrame {
    pub weapon_pitch: f32,
    pub weapon_roll: f32,
    pub weapon_drop: f32,
    pub left_hand: Vec3,
    pub left_in_weapon_space: bool,
}

pub fn reload_frame(kind: WeaponKind, progress: f32) -> ReloadFrame {
    let s = sockets(kind);
    let pouch = v(-0.2, -0.34, -0.18);
    let out = s.magazine + v(-0.02, -0.2, 0.0);
    let keys: [(f32, Vec3, bool); 7] = [
        (0.0, s.left_grip, true),
        (0.14, s.magazine + v(-0.03, 0.0, 0.0), true),
        (0.28, out, true),
        (0.48, pouch, false),
        (0.68, s.magazine + v(-0.02, -0.12, 0.0), true),
        (0.8, s.magazine, true),
        (1.0, s.left_grip, true),
    ];
    let mut index = 0;
    while index + 2 < keys.len() && progress > keys[index + 1].0 {
        index += 1;
    }
    let (t0, p0, w0) = keys[index];
    let (t1, p1, w1) = keys[index + 1];
    let t = smooth(((progress - t0) / (t1 - t0)).clamp(0.0, 1.0));
    let left_hand = lerp(p0, p1, t);
    let left_in_weapon_space = if t < 0.5 { w0 } else { w1 };
    let tilt = (std::f32::consts::PI * progress.clamp(0.0, 1.0)).sin();
    let slap = if (0.8..0.88).contains(&progress) { ((progress - 0.8) / 0.08 * std::f32::consts::PI).sin() * 0.02 } else { 0.0 };
    ReloadFrame { weapon_pitch: -0.3 * tilt, weapon_roll: 0.45 * tilt, weapon_drop: 0.06 * tilt + slap, left_hand, left_in_weapon_space }
}

impl ViewArms {
    pub fn build() -> ViewArms {
        let mut skeleton = Skeleton::new();
        skeleton.add("shoulder", None, Vec3::ZERO);
        skeleton.add("elbow", Some("shoulder"), v(0.0, -UPPER_ARM, 0.0));
        skeleton.add("wrist", Some("elbow"), v(0.0, -FOREARM, 0.0));
        let upper = Sdf::limb(Vec3::ZERO, v(0.0, -UPPER_ARM, 0.0), [0.058, 0.058], [0.05, 0.05], 0.02).union(Sdf::sphere(0.062));
        let fore = Sdf::limb(Vec3::ZERO, v(0.0, -FOREARM, 0.0), [0.05, 0.05], [0.04, 0.04], 0.018).union(Sdf::sphere(0.052));
        let glove = Sdf::round_box(v(0.085, 0.11, 0.07), 0.03).translate(v(0.0, -0.06, 0.0)).union(Sdf::round_box(v(0.03, 0.07, 0.04), 0.015).translate(v(-0.05, -0.04, -0.03)));
        ViewArms { skeleton, upper: mesh_sdf(&upper, 0.012, SLEEVE), fore: mesh_sdf(&fore, 0.012, SLEEVE), glove: mesh_sdf(&glove, 0.01, GLOVE) }
    }

    pub fn arm_chain(&self, shoulder: Vec3, target: Vec3, pole: Vec3) -> [JointTransform; 3] {
        let mut pose = self.skeleton.rest_pose();
        pose.root_offset = shoulder;
        let _ = self.skeleton.solve_two_bone(&mut pose, "shoulder", "elbow", "wrist", target - Vec3::ZERO, pole);
        let world = self.skeleton.world(&pose);
        [world[0], world[1], world[2]]
    }

    pub fn emit(&self, out: &mut Vec<Tri>, kind: WeaponKind, weapon: &Placement, body: &Placement, reload: Option<f32>, show_left: bool) {
        let s = sockets(kind);
        let mut left_target = weapon.point(s.left_grip);
        let right_target = weapon.point(s.right_grip);
        if let Some(progress) = reload {
            let frame = reload_frame(kind, progress);
            left_target = if frame.left_in_weapon_space { weapon.point(frame.left_hand) } else { body.point(frame.left_hand) };
        }
        let plain = Placement::at(Vec3::ZERO, 0.0);
        let hands = [(right_target, body.point(v(0.2, -0.32, 0.3)), body.point(v(0.55, -0.5, 0.25)), true), (left_target, body.point(v(-0.22, -0.34, 0.26)), body.point(v(-0.5, -0.5, 0.2)), show_left)];
        for (target, shoulder, pole, visible) in hands {
            if !visible {
                continue;
            }
            let chain = self.arm_chain(shoulder, target, pole);
            push_mesh_bone(out, &self.upper, &chain[0], Vec3::ZERO, &plain);
            push_mesh_bone(out, &self.fore, &chain[1], Vec3::ZERO, &plain);
            let hand_frame = JointTransform { position: chain[2].position, rotation: weapon_aligned(chain[2].rotation, weapon) };
            push_mesh_bone(out, &self.glove, &hand_frame, Vec3::ZERO, &plain);
        }
    }
}

fn weapon_aligned(forearm: Quat, weapon: &Placement) -> Quat {
    let weapon_forward = weapon.direction(v(0.0, 0.0, -1.0));
    let hand_forward = forearm.rotate(v(0.0, 0.0, -1.0));
    Quat::between(hand_forward, weapon_forward).mul(forearm)
}
