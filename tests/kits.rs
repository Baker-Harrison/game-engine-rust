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
