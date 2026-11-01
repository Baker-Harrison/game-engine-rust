use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::part::{Asset, Part, PartKind};
use engine::model::sdf::Sdf;
use engine::probe::targets::Targets;
use engine::probe::{run, ProbeScene, Severity};

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn boxed(center: Vec3, size: Vec3) -> Mesh {
    let mut mesh = Mesh::new();
    mesh.add_box(center, size, v(0.6, 0.6, 0.6));
    mesh
}

fn scene_of(meshes: Vec<(&str, Mesh)>) -> ProbeScene {
    let mut asset = Asset::new("test");
    for (name, mesh) in meshes {
        asset.parts.push(Part::from_mesh(name, PartKind::Object, mesh));
    }
    ProbeScene::from_asset(&asset, Vec::new())
}

fn errors(scene: &ProbeScene) -> Vec<String> {
    run(scene, &Targets::default()).findings.iter().filter(|f| f.severity == Severity::Error).map(|f| format!("{} {}", f.subject, f.metric)).collect()
}

#[test]
fn overlapping_boxes_are_reported_with_depth() {
    let scene = scene_of(vec![("a", boxed(v(0.0, 0.5, 0.0), v(1.0, 1.0, 1.0))), ("b", boxed(v(0.6, 0.9, 0.2), v(1.0, 1.0, 1.0)))]);
    let report = run(&scene, &Targets::default());
    assert!(errors(&scene).iter().any(|e| e.contains("a / b") && e.contains("interpenetration")), "{}", report.text());
}

#[test]
fn touching_boxes_are_not_interpenetrating() {
    let scene = scene_of(vec![("a", boxed(v(0.0, 0.5, 0.0), v(1.0, 1.0, 1.0))), ("b", boxed(v(1.0, 0.5, 0.0), v(1.0, 1.0, 1.0)))]);
    assert!(!errors(&scene).iter().any(|e| e.contains("interpenetration")), "{:?}", errors(&scene));
}

#[test]
fn separate_boxes_have_no_placement_findings() {
    let scene = scene_of(vec![("a", boxed(v(0.0, 0.5, 0.0), v(1.0, 1.0, 1.0))), ("b", boxed(v(3.0, 0.5, 0.0), v(1.0, 1.0, 1.0)))]);
    assert!(!errors(&scene).iter().any(|e| e.contains("interpenetration")));
}

#[test]
fn raw_box_fails_the_hard_edge_gate_and_rounded_box_passes() {
    let raw = scene_of(vec![("raw", boxed(v(0.0, 0.5, 0.0), v(1.0, 1.0, 1.0)))]);
    assert!(errors(&raw).iter().any(|e| e.contains("raw") && e.contains("unbeveled_hard_edge_length_m")));
    let rounded = mesh_sdf(&Sdf::round_box(v(0.5, 0.5, 0.5), 0.05), 0.02, v(0.6, 0.6, 0.6));
    let scene = scene_of(vec![("soft", rounded)]);
    assert!(!errors(&scene).iter().any(|e| e.contains("unbeveled_hard_edge_length_m")), "{:?}", errors(&scene));
}
