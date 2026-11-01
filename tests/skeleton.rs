use engine::math::Vec3;
use engine::skeleton::{AnimPlayer, Clip, Quat, Skeleton};

fn arm() -> Skeleton {
    let mut skeleton = Skeleton::new();
    skeleton.add("shoulder", None, Vec3::new(0.0, 0.0, 0.0));
    skeleton.add("elbow", Some("shoulder"), Vec3::new(0.0, 0.0, 0.3));
    skeleton.add("wrist", Some("elbow"), Vec3::new(0.0, 0.0, 0.28));
    skeleton
}

#[test]
fn forward_kinematics_follows_rotations() {
    let skeleton = arm();
    let mut pose = skeleton.rest_pose();
    let world = skeleton.world(&pose);
    assert!((world[2].position - Vec3::new(0.0, 0.0, 0.58)).length() < 1e-5);
    pose.rotations[0] = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2);
    let world = skeleton.world(&pose);
    assert!((world[2].position - Vec3::new(0.58, 0.0, 0.0)).length() < 1e-4, "{:?}", world[2].position);
}

#[test]
fn two_bone_ik_reaches_targets_and_keeps_bone_lengths() {
    let skeleton = arm();
    for target in [Vec3::new(0.2, 0.1, 0.35), Vec3::new(-0.15, 0.25, 0.3), Vec3::new(0.0, -0.3, 0.4), Vec3::new(0.3, 0.0, 0.2)] {
        let mut pose = skeleton.rest_pose();
        let error = skeleton.solve_two_bone(&mut pose, "shoulder", "elbow", "wrist", target, Vec3::new(0.0, 1.0, 0.2)).unwrap();
        assert!(error < 0.002, "target {:?} missed by {}", target, error);
        let world = skeleton.world(&pose);
        assert!(((world[1].position - world[0].position).length() - 0.3).abs() < 1e-4);
        assert!(((world[2].position - world[1].position).length() - 0.28).abs() < 1e-4);
    }
}

#[test]
fn two_bone_ik_stretches_toward_unreachable_targets_without_breaking() {
    let skeleton = arm();
    let mut pose = skeleton.rest_pose();
    let error = skeleton.solve_two_bone(&mut pose, "shoulder", "elbow", "wrist", Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 1.0, 0.0)).unwrap();
    assert!((error - 1.42).abs() < 0.01, "error {}", error);
}

#[test]
fn pole_vector_chooses_the_elbow_side() {
    let skeleton = arm();
    let mut pose = skeleton.rest_pose();
    skeleton.solve_two_bone(&mut pose, "shoulder", "elbow", "wrist", Vec3::new(0.0, 0.0, 0.4), Vec3::new(0.0, 1.0, 0.1)).unwrap();
    assert!(skeleton.world(&pose)[1].position.y > 0.05);
    skeleton.solve_two_bone(&mut pose, "shoulder", "elbow", "wrist", Vec3::new(0.0, 0.0, 0.4), Vec3::new(0.0, -1.0, 0.1)).unwrap();
    assert!(skeleton.world(&pose)[1].position.y < -0.05);
}

#[test]
fn clips_loop_blend_and_report_progress() {
    let skeleton = arm();
    let up = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), -1.0);
    let swing = Clip::new("swing", 1.0, true).rotate(&skeleton, "shoulder", &[(0.0, Quat::IDENTITY), (0.5, up), (1.0, Quat::IDENTITY)]);
    let reload = Clip::new("reload", 0.5, false).rotate(&skeleton, "elbow", &[(0.0, Quat::IDENTITY), (0.5, up)]);
    let idle = swing.sample(&skeleton, 0.0);
    let peak = swing.sample(&skeleton, 0.5);
    let wrapped = swing.sample(&skeleton, 1.5);
    assert!(idle.rotations[0].w > 0.999);
    assert!(peak.rotations[0].w < 0.9);
    assert!((wrapped.rotations[0].w - peak.rotations[0].w).abs() < 1e-4);

    let mut player = AnimPlayer::new(vec![swing, reload]);
    player.update(0.5, 1.0);
    player.play("reload", 0.2).unwrap();
    assert_eq!(player.current(), "reload");
    let start = player.pose(&skeleton);
    assert!((start.rotations[0].w - peak.rotations[0].w).abs() < 1e-3);
    player.update(0.1, 1.0);
    let middle = player.pose(&skeleton);
    assert!(middle.rotations[0].w > peak.rotations[0].w);
    assert!(!player.finished());
    player.update(0.5, 1.0);
    assert!(player.finished());
    assert!((player.progress() - 1.0).abs() < 1e-5);
}
