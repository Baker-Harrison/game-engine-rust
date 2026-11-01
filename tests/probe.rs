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

fn overlapping_pair(mutate: impl Fn(&mut Asset)) -> Vec<String> {
    let mut asset = Asset::new("test");
    asset.parts.push(Part::from_mesh("a", PartKind::Object, boxed(v(0.0, 0.5, 0.0), v(1.0, 1.0, 1.0))));
    asset.parts.push(Part::from_mesh("b", PartKind::Object, boxed(v(0.6, 0.9, 0.2), v(1.0, 1.0, 1.0))));
    mutate(&mut asset);
    let scene = ProbeScene::from_asset(&asset, Vec::new());
    errors(&scene).into_iter().filter(|e| e.contains("interpenetration")).collect()
}

#[test]
fn declared_overlap_is_allowed_in_either_order() {
    assert_eq!(overlapping_pair(|_| {}).len(), 1);
    assert!(overlapping_pair(|asset| asset.declare_overlap("a", "b")).is_empty());
    assert!(overlapping_pair(|asset| asset.declare_overlap("b", "a")).is_empty());
}

#[test]
fn foliage_parts_may_overlap_and_skip_surface_gates() {
    let errors = overlapping_pair(|asset| asset.parts[1].kind = PartKind::Foliage);
    assert!(errors.is_empty());
}

#[test]
fn foliage_open_surfaces_pass_model_validation() {
    let mut card = Mesh::new();
    card.add_flat_triangle(v(0.0, 0.0, 0.0), v(0.2, 0.0, 0.0), v(0.0, 0.2, 0.0), v(0.2, 0.5, 0.2));
    let mut asset = Asset::new("bush");
    asset.parts.push(Part::from_mesh("bush.leaves", PartKind::Foliage, card.clone()));
    assert!(engine::model::validate::validate(&asset).iter().all(|f| f.severity != engine::model::validate::Severity::Error));
    asset.parts[0].kind = PartKind::Object;
    assert!(engine::model::validate::validate(&asset).iter().any(|f| f.severity == engine::model::validate::Severity::Error));
}
