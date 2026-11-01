use engine::assets;
use engine::math::Vec3;
use engine::model::bvh::Bvh;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;
use engine::model::validate::{validate, Severity};
use engine::noise::value_noise;
use engine::png;
use engine::render::Framebuffer;
use engine::scene::Scene;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn errors_of(asset: &engine::model::part::Asset) -> Vec<String> {
    validate(asset).into_iter().filter(|f| f.severity == Severity::Error).map(|f| f.format()).collect()
}

#[test]
fn sphere_mesh_is_watertight_with_correct_volume() {
    let mesh = mesh_sdf(&Sdf::sphere(0.1), 0.004, v(1.0, 1.0, 1.0));
    assert!(mesh.topology_report().watertight);
    let expected = 4.0 / 3.0 * std::f32::consts::PI * 0.1f32.powi(3);
    assert!((mesh.volume() - expected).abs() / expected < 0.03, "volume {}", mesh.volume());
}

#[test]
fn rounded_box_volume_matches_formula() {
    let mesh = mesh_sdf(&Sdf::round_box(v(0.2, 0.1, 0.3), 0.02), 0.004, v(1.0, 1.0, 1.0));
    let r = 0.02f32;
    let (a, b, c) = (0.2f32 - 2.0 * r, 0.1f32 - 2.0 * r, 0.3f32 - 2.0 * r);
    let pi = std::f32::consts::PI;
    let expected = a * b * c + 2.0 * r * (a * b + b * c + a * c) + pi * r * r * (a + b + c) + 4.0 / 3.0 * pi * r * r * r;
    assert!((mesh.volume() - expected).abs() / expected < 0.08, "volume {} expected about {}", mesh.volume(), expected);
}

#[test]
fn revolve_and_extrude_produce_closed_solids() {
    let cylinder = Sdf::revolve(&[[0.0, 0.0], [0.02, 0.0], [0.02, 0.2], [0.0, 0.2]]);
    let mesh = mesh_sdf(&cylinder, 0.004, v(1.0, 1.0, 1.0));
    let expected = std::f32::consts::PI * 0.02f32.powi(2) * 0.2;
    assert!(mesh.topology_report().watertight);
    assert!((mesh.volume() - expected).abs() / expected < 0.08);

    let prism = Sdf::extrude(&[[0.0, 0.0], [0.1, 0.0], [0.1, 0.1], [0.0, 0.1]], 0.05, 0.0);
    let mesh = mesh_sdf(&prism, 0.004, v(1.0, 1.0, 1.0));
    assert!((mesh.volume() - 0.1 * 0.1 * 0.05).abs() / (0.1 * 0.1 * 0.05) < 0.1);
}

#[test]
fn array_repeats_the_shape() {
    let bar = Sdf::round_box(v(0.01, 0.02, 0.02), 0.002).linear_array_x(4, 0.03);
    assert!(bar.distance(v(0.0, 0.0, 0.0)) < 0.0);
    assert!(bar.distance(v(0.09, 0.0, 0.0)) < 0.0);
    assert!(bar.distance(v(0.015, 0.0, 0.0)) > 0.0);
    assert!(bar.distance(v(0.12, 0.0, 0.0)) > 0.0);
}

#[test]
fn smooth_union_adds_a_fillet_between_shapes() {
    let a = Sdf::round_box(v(0.1, 0.1, 0.1), 0.0).translate(v(-0.05, 0.0, 0.0));
    let b = Sdf::round_box(v(0.1, 0.1, 0.1), 0.0).translate(v(0.05, 0.1, 0.0));
    let sharp = a.clone().union(b.clone());
    let smooth = a.smooth_union(b, 0.04);
    let corner = v(0.0, 0.05, 0.0);
    assert!(smooth.distance(corner) < sharp.distance(corner));
}

#[test]
fn meshing_is_deterministic() {
    let build = || assets::build("chair", &[("cell".to_string(), 0.01)]).unwrap();
    let (first, second) = (build(), build());
    for (a, b) in first.parts.iter().zip(&second.parts) {
        assert_eq!(a.mesh.indices, b.mesh.indices);
        assert_eq!(a.mesh.positions.len(), b.mesh.positions.len());
        assert!(a.mesh.positions.iter().zip(&b.mesh.positions).all(|(p, q)| p == q));
    }
}

#[test]
fn default_chair_passes_validation() {
    let chair = assets::build("chair", &[("cell".to_string(), 0.008)]).unwrap();
    assert_eq!(errors_of(&chair), Vec::<String>::new());
    let (low, high) = chair.object_bounds();
    assert!((high.y - low.y - 0.90).abs() < 0.05);
}

#[test]
fn raised_leg_is_reported_as_not_touching_the_floor() {
    let chair = assets::build("chair", &[("cell".to_string(), 0.008), ("raise_front_left_leg".to_string(), 0.03)]).unwrap();
    let errors = errors_of(&chair);
    assert!(errors.iter().any(|e| e.contains("leg_front_left") && e.contains("above the floor")), "{:?}", errors);
}

#[test]
fn unknown_parameter_is_rejected_with_the_known_list() {
    let error = assets::build("chair", &[("seat_hight".to_string(), 0.5)]).err().unwrap();
    assert!(error.contains("seat_height"));
}

#[test]
fn house_has_no_validation_errors_and_removing_doors_breaks_access() {
    let overrides = [("furniture_cell".to_string(), 0.01)];
    let mut house = assets::build("house", &overrides).unwrap();
    assert_eq!(errors_of(&house), Vec::<String>::new());
    house.doors.clear();
    assert!(errors_of(&house).iter().any(|e| e.contains("no door path")));
}

#[test]
fn misplaced_furniture_is_reported() {
    let mut house = assets::build("house", &[("furniture_cell".to_string(), 0.01)]).unwrap();
    let blocker = house.footprints[0].clone();
    let mut copy = blocker.clone();
    copy.subject = "ghost".to_string();
    house.footprints.push(copy);
    assert!(errors_of(&house).iter().any(|e| e.contains("overlap")));
}

#[test]
fn bvh_detects_hits_and_misses() {
    let mut mesh = Mesh::new();
    mesh.add_box(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0), v(1.0, 1.0, 1.0));
    let bvh = Bvh::from_meshes(&[&mesh]);
    assert!(bvh.any_hit(v(-3.0, 0.0, 0.0), v(1.0, 0.0, 0.0), 10.0));
    assert!(!bvh.any_hit(v(-3.0, 2.0, 0.0), v(1.0, 0.0, 0.0), 10.0));
    assert!(!bvh.any_hit(v(-3.0, 0.0, 0.0), v(1.0, 0.0, 0.0), 1.0));
}

#[test]
fn png_has_valid_signature_and_chunks() {
    let mut frame = Framebuffer::new(5, 3);
    frame.clear(v(1.0, 0.0, 0.0));
    let bytes = png::encode(&frame);
    assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    assert_eq!(&bytes[12..16], b"IHDR");
    assert_eq!(&bytes[bytes.len() - 8..bytes.len() - 4], b"IEND");
}

#[test]
fn noise_is_deterministic_and_bounded() {
    for i in 0..200 {
        let x = i as f32 * 0.37;
        let first = value_noise(x, x * 0.5, 1.0, 3);
        assert_eq!(first, value_noise(x, x * 0.5, 1.0, 3));
        assert!((-1.0..=1.0).contains(&first));
    }
}

#[test]
fn scene_files_can_place_assets() {
    let scene = Scene::parse("asset stool pos 1 0 2 yaw 30 cell=0.02").unwrap();
    assert!(!scene.tris.is_empty());
    assert!(Scene::parse("asset nothing").is_err());
    assert!(Scene::parse("asset stool cell=abc").is_err());
}

#[test]
fn carbine_prop_is_a_closed_solid_with_expected_length() {
    let carbine = assets::build("carbine", &[("cell".to_string(), 0.002)]).unwrap();
    assert_eq!(errors_of(&carbine), Vec::<String>::new());
    let (low, high) = carbine.object_bounds();
    assert!((high.x - low.x - 0.83).abs() < 0.05);
}

#[test]
fn sweep_follows_a_curve_and_stays_smooth() {
    let path = [v(0.0, 0.0, 0.0), v(0.0, -0.05, 0.0), v(0.02, -0.1, 0.0), v(0.06, -0.14, 0.0)];
    let sizes = [[0.012, 0.02], [0.012, 0.02], [0.011, 0.019], [0.011, 0.019]];
    let sweep = Sdf::sweep(&path, &sizes, 0.003, 6, v(0.0, 0.0, 1.0));
    let mesh = mesh_sdf(&sweep, 0.002, v(1.0, 1.0, 1.0));
    let report = mesh.topology_report();
    assert!(report.watertight);
    assert!(report.max_edge_angle_degrees < 60.0, "scalloped: {}", report.max_edge_angle_degrees);
    assert!(sweep.distance(v(0.02, -0.1, 0.0)) < 0.0);
    assert!(sweep.distance(v(0.06, -0.02, 0.0)) > 0.0);
}

#[test]
fn edge_wear_lightens_convex_edges_only() {
    use engine::model::mesher::mesh_sdf_painted;
    use engine::model::paint::SurfacePaint;
    let paint = SurfacePaint::worn(v(0.1, 0.1, 0.1), v(0.9, 0.9, 0.9));
    let plain = SurfacePaint { grain: 0.0, ..paint };
    let shape = Sdf::round_box(v(0.1, 0.1, 0.1), 0.004);
    let mesh = mesh_sdf_painted(&shape, 0.002, &|sample| plain.color_at(sample));
    let brightness = |index: usize| mesh.colors[index].x;
    let face_center = (0..mesh.positions.len()).filter(|i| mesh.positions[*i].x.abs() < 0.01 && mesh.positions[*i].y.abs() < 0.01 && mesh.positions[*i].z > 0.04).map(brightness).fold(0.0f32, f32::max);
    let edge = (0..mesh.positions.len()).filter(|i| mesh.positions[*i].x > 0.045 && mesh.positions[*i].y > 0.045).map(brightness).fold(0.0f32, f32::max);
    assert!(edge > face_center + 0.2, "edge {} face {}", edge, face_center);
}

#[test]
fn studio_lighting_brightens_the_side_facing_away_from_the_sun() {
    use engine::model::bake::{bake, BakeSettings};
    let mut sun_mesh = Mesh::new();
    sun_mesh.add_box(v(0.0, 0.5, 0.0), v(1.0, 1.0, 1.0), v(0.5, 0.5, 0.5));
    let mut studio_mesh = sun_mesh.clone();
    let occluders = Bvh::from_meshes(&[&sun_mesh.clone()]);
    bake(&mut sun_mesh, &occluders, &BakeSettings::default());
    bake(&mut studio_mesh, &occluders, &BakeSettings::studio());
    let facing_away = |mesh: &Mesh| (0..mesh.positions.len()).filter(|i| mesh.normals[*i].x < -0.9).map(|i| mesh.colors[i].x).fold(0.0f32, f32::max);
    assert!(facing_away(&studio_mesh) > facing_away(&sun_mesh));
}

#[test]
fn downsampling_averages_blocks() {
    let mut frame = Framebuffer::new(4, 2);
    frame.clear(v(1.0, 1.0, 1.0));
    for x in 0..2 {
        for y in 0..2 {
            frame.pixels[y * 4 + x] = 0;
        }
    }
    let small = frame.downsampled(2);
    assert_eq!(small.pixels[0], 0);
    assert_eq!(small.pixels[1], 0xFFFFFF);
}
