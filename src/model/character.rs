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
