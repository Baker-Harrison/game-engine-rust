use super::detail::{build_styled_part, DetailSpec};
use super::humanoid::{body, hand, head, BodySpec, HandPose, Landmarks};
use super::part::{Asset, Part, PartKind};
use super::sdf::Sdf;
use super::style::Style;
use crate::math::Vec3;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn skin_part(name: &str, sdf: &Sdf, style: &Style, feature: f32) -> Result<Part, String> {
    build_styled_part(name, PartKind::Object, sdf, style, "skin", DetailSpec::new(feature).with_budget(600_000), Vec::new())
}

pub fn build_human(style: &Style, spec: &BodySpec, pose: &HandPose) -> Result<Asset, String> {
    let landmarks = Landmarks::for_spec(spec);
    let mut asset = Asset::new("human");
    asset.add_stage_floor(0.0, 0.0, 2.5, 0.1, 0.0, Vec3::new(0.62, 0.62, 0.60));
    asset.parts.push(skin_part("body", &body(spec, &landmarks), style, spec.height * 0.03)?);
    let head_height = spec.height * 0.13;
    let head_sdf = head(spec.height).translate(v(0.0, landmarks.chin.y - head_height * 0.02, 0.0));
    asset.parts.push(skin_part("head", &head_sdf, style, head_height * 0.12)?);
    let hand_length = spec.height * 0.108;
    for (side, wrist) in [(1.0f32, landmarks.wrist_right), (-1.0, landmarks.wrist_left)] {
        let shape = hand(hand_length, pose).rotate_z(180.0).rotate_y(if side > 0.0 { 0.0 } else { 0.0 }).translate(wrist);
        let mirrored = if side > 0.0 { shape } else { hand(hand_length, pose).rotate_z(180.0).mirror(super::sdf::Axis::X).translate(wrist) };
        let name = if side > 0.0 { "hand_right" } else { "hand_left" };
        asset.parts.push(skin_part(name, &mirrored, style, hand_length * 0.07)?);
    }
    Ok(asset)
}

pub fn build_hand_poses(style: &Style, hand_length: f32) -> Result<Asset, String> {
    let mut asset = Asset::new("hands");
    let poses = [("open", HandPose::open()), ("fist", HandPose::fist()), ("pistol_grip", HandPose::pistol_grip()), ("trigger_finger", HandPose::trigger_finger()), ("foregrip", HandPose::foregrip())];
    for (index, (name, pose)) in poses.iter().enumerate() {
        let shape = hand(hand_length, pose).rotate_x(-90.0).translate(v(index as f32 * hand_length * 1.4, hand_length * 0.3, 0.0));
        asset.parts.push(skin_part(name, &shape, style, hand_length * 0.07)?);
    }
    Ok(asset)
}

pub fn build_viewmodel(style: &Style, hand_length: f32) -> Result<Asset, String> {
    use super::props::{build_carbine, carbine_sockets};
    let sockets = carbine_sockets();
    let mut asset = build_carbine(0.003);
    let palm_center = v(0.0, -hand_length * 0.26, 0.0);
    let right = sockets.pistol_grip.place(hand(hand_length, &HandPose::trigger_finger()).translate(palm_center));
    let left = sockets.foregrip.place(hand(hand_length, &HandPose::foregrip()).translate(palm_center));
    asset.parts.push(skin_part("hand_firing", &right, style, hand_length * 0.07)?);
    asset.parts.push(skin_part("hand_support", &left, style, hand_length * 0.07)?);
    Ok(asset)
}

pub struct RiggedHuman {
    pub asset: Asset,
    pub joints: std::collections::BTreeMap<String, Vec3>,
}

fn capsule(a: Vec3, b: Vec3, start: f32, end: f32, girth: f32) -> Sdf {
    let limb = Sdf::limb(a, b, [start * girth, start * girth * 0.9], [end * girth, end * girth * 0.9], start * 0.4);
    limb.union(Sdf::sphere(start * girth * 0.95).translate(a)).union(Sdf::sphere(end * girth * 0.95).translate(b))
}

pub fn build_human_rigged(style: &Style, spec: &BodySpec, pose: &HandPose) -> Result<RiggedHuman, String> {
    let landmarks = Landmarks::for_spec(spec);
    let h = spec.height;
    let girth = 0.85 + 0.3 * spec.build;
    let mut asset = Asset::new("human_rigged");
    asset.add_stage_floor(0.0, 0.0, 2.5, 0.1, 0.0, Vec3::new(0.62, 0.62, 0.60));
    let mut joints = std::collections::BTreeMap::new();
    let mut add = |asset: &mut Asset, name: &str, sdf: Sdf, feature: f32| -> Result<(), String> {
        asset.parts.push(skin_part(name, &sdf, style, feature)?);
        Ok(())
    };

    let chest = Sdf::round_box(v(h * 0.255 * girth, h * 0.20, h * 0.13 * girth), h * 0.05).translate(v(0.0, h * 0.72, 0.0));
    let waist = Sdf::round_box(v(h * 0.20 * girth, h * 0.12, h * 0.115 * girth), h * 0.045).translate(v(0.0, h * 0.60, 0.0));
    add(&mut asset, "torso", chest.smooth_union(waist, h * 0.03), h * 0.05)?;
    add(&mut asset, "pelvis", Sdf::round_box(v(h * 0.20 * girth, h * 0.11, h * 0.12 * girth), h * 0.05).translate(v(0.0, h * 0.52, 0.0)), h * 0.05)?;
    add(&mut asset, "neck", capsule(v(0.0, h * 0.80, 0.0), v(0.0, h * 0.88, h * 0.01), h * 0.045, h * 0.04, girth), h * 0.05)?;
    let head_height = h * 0.13;
    add(&mut asset, "head", head(h).translate(v(0.0, landmarks.chin.y - head_height * 0.02, 0.0)), head_height * 0.12)?;

    let sides = [
        ("l", landmarks.shoulder_left, landmarks.elbow_left, landmarks.wrist_left, landmarks.hip_left, landmarks.knee_left, landmarks.ankle_left, true),
        ("r", landmarks.shoulder_right, landmarks.elbow_right, landmarks.wrist_right, landmarks.hip_right, landmarks.knee_right, landmarks.ankle_right, false),
    ];
    let hand_length = h * 0.108;
    for (side, shoulder, elbow, wrist, hip, knee, ankle, mirror) in sides {
        add(&mut asset, &format!("upper_arm_{}", side), capsule(shoulder, elbow, h * 0.032, h * 0.026, girth), h * 0.045)?;
        add(&mut asset, &format!("forearm_{}", side), capsule(elbow, wrist, h * 0.026, h * 0.019, girth), h * 0.04)?;
        add(&mut asset, &format!("thigh_{}", side), capsule(hip, knee, h * 0.056, h * 0.036, girth), h * 0.07)?;
        add(&mut asset, &format!("shin_{}", side), capsule(knee, ankle, h * 0.036, h * 0.024, girth), h * 0.05)?;
        let toe = v(ankle.x, h * 0.02, ankle.z + h * 0.11);
        add(&mut asset, &format!("foot_{}", side), Sdf::limb(v(ankle.x, ankle.y + h * 0.01, ankle.z - h * 0.015), toe, [h * 0.025, h * 0.02], [h * 0.02, h * 0.014], h * 0.008).union(Sdf::sphere(h * 0.024).translate(ankle)), h * 0.04)?;
        let shape = hand(hand_length, pose).rotate_z(180.0);
        let shape = if mirror { shape.mirror(super::sdf::Axis::X) } else { shape };
        add(&mut asset, &format!("hand_{}", side), shape.translate(wrist), hand_length * 0.07)?;
        joints.insert(format!("shoulder_{}", side), shoulder);
        joints.insert(format!("elbow_{}", side), elbow);
        joints.insert(format!("wrist_{}", side), wrist);
        joints.insert(format!("hip_{}", side), hip);
        joints.insert(format!("knee_{}", side), knee);
        joints.insert(format!("ankle_{}", side), ankle);
    }
    joints.insert("neck".to_string(), v(0.0, h * 0.80, 0.0));
    joints.insert("head".to_string(), landmarks.chin);
    joints.insert("spine".to_string(), v(0.0, h * 0.56, 0.0));
    Ok(RiggedHuman { asset, joints })
}
