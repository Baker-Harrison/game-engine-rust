use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;
use engine::model::stats::{MeshStats, TEXTURE_SIZE};
use engine::model::style::{build_swatches, Style};

#[test]
fn raw_box_has_no_bevel_coverage_and_rounded_box_does() {
    let mut raw = Mesh::new();
    raw.add_box(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0), Vec3::splat(0.5));
    let rounded = mesh_sdf(&Sdf::round_box(Vec3::new(1.0, 1.0, 1.0), 0.06), 0.02, Vec3::splat(0.5));
    let raw_stats = MeshStats::measure(&raw);
    let rounded_stats = MeshStats::measure(&rounded);
    assert!(raw_stats.bevel_coverage < 0.05, "raw {}", raw_stats.bevel_coverage);
    assert!(rounded_stats.bevel_coverage > 0.9, "rounded {}", rounded_stats.bevel_coverage);
}

#[test]
fn silhouette_convexity_separates_box_from_ring() {
    let mut cube = Mesh::new();
    cube.add_box(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0), Vec3::splat(0.5));
    let ring = mesh_sdf(&Sdf::torus(0.5, 0.1), 0.03, Vec3::splat(0.5));
    assert!(MeshStats::measure(&cube).silhouette_convexity[2] > 0.95);
    assert!(MeshStats::measure(&ring).silhouette_convexity[1] < 0.7);
}

#[test]
fn generated_uvs_hit_requested_texel_density() {
    let mut mesh = mesh_sdf(&Sdf::round_box(Vec3::new(1.0, 1.0, 1.0), 0.05), 0.03, Vec3::splat(0.5));
    mesh.generate_uvs(2.0);
    let density = MeshStats::measure(&mesh).texel_density;
    assert!((density - TEXTURE_SIZE / 2.0).abs() < 16.0, "density {}", density);
}

#[test]
fn style_reports_missing_names_with_alternatives() {
    let style = Style::overgrown_manor();
    assert!(style.material("stone_wall").is_ok());
    let error = style.material("granite").err().unwrap();
    assert!(error.contains("stone_wall"));
    assert!(style.color("moss").is_ok());
}

#[test]
fn swatches_build_one_part_per_style_material() {
    let style = Style::overgrown_manor();
    let asset = build_swatches(&style, 0.05);
    assert_eq!(asset.parts.len(), style.materials.len());
}

use engine::model::bvh::Bvh;
use engine::model::detail::{build_styled_mesh_part, build_styled_part, DetailSpec};
use engine::model::lod::build_lod_chain;
use engine::model::part::PartKind;

#[test]
fn lod_chain_reduces_triangles_and_stays_near_the_surface() {
    let sphere = mesh_sdf(&Sdf::sphere(0.5), 0.01, Vec3::splat(0.5));
    let chain = build_lod_chain(&sphere, &[0.25, 0.06]);
    assert_eq!(chain.len(), 3);
    assert!(chain[1].mesh.triangle_count() < chain[0].mesh.triangle_count() / 2);
    assert!(chain[2].mesh.triangle_count() < chain[1].mesh.triangle_count());
    let reference = Bvh::from_meshes(&[&sphere]);
    for position in &chain[2].mesh.positions {
        let toward_center = (Vec3::ZERO - *position).normalized();
        let outside = *position - toward_center * 0.0;
        let distance_from_surface = (outside.length() - 0.5).abs();
        assert!(distance_from_surface < 0.04, "{}", distance_from_surface);
    }
    assert!(reference.any_hit(Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, -1.0), 10.0));
}

#[test]
fn styled_part_builds_with_uvs_and_material() {
    let style = Style::overgrown_manor();
    let block = Sdf::round_box(Vec3::new(1.0, 0.5, 0.4), 0.03);
    let part = build_styled_part("wall", PartKind::Architecture, &block, &style, "stone_wall", DetailSpec::new(0.03), Vec::new()).unwrap();
    assert!(part.mesh.uvs.iter().any(|uv| uv[0] != 0.0 || uv[1] != 0.0));
}

#[test]
fn raw_box_mesh_is_rejected_by_the_bevel_floor() {
    let style = Style::overgrown_manor();
    let mut raw = Mesh::new();
    raw.add_box(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0), Vec3::splat(0.5));
    let error = build_styled_mesh_part("wall", PartKind::Architecture, raw, &style, "stone_wall", Vec::new()).err().unwrap();
    assert!(error.contains("bevel coverage"), "{}", error);
}

#[test]
fn triangle_budget_is_enforced() {
    let style = Style::overgrown_manor();
    let block = Sdf::round_box(Vec3::new(2.0, 2.0, 2.0), 0.05);
    let error = build_styled_part("wall", PartKind::Architecture, &block, &style, "stone_wall", DetailSpec::new(0.03).with_budget(1000), Vec::new()).err().unwrap();
    assert!(error.contains("budget"), "{}", error);
}
