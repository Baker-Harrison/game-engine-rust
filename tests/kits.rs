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

use engine::model::ornament::{arch_opening_wall, balustrade, column, cornice, finial, iron_railing, BalustradeSpec, Ornament};

fn mesh_ornament(ornament: &Ornament) -> Mesh {
    mesh_sdf(&ornament.sdf, ornament.smallest_feature / 3.0, Vec3::splat(0.5))
}

#[test]
fn ornaments_mesh_to_closed_surfaces() {
    let list = [
        ("finial", finial(0.3, 0.1)),
        ("column", column(2.0, 0.2, 12)),
        ("cornice", cornice(1.0, 0.3, 0.4)),
        ("arch", arch_opening_wall(2.0, 2.6, 0.4, 1.0, 1.9)),
    ];
    for (name, ornament) in list {
        let report = mesh_ornament(&ornament).topology_report();
        assert!(report.watertight, "{} open {} nonmanifold {}", name, report.open_edges, report.nonmanifold_edges);
    }
}

#[test]
fn balustrade_has_a_baluster_every_spacing() {
    let spec = BalustradeSpec::terrace(2.0);
    let ornament = balustrade(&spec);
    let mesh = mesh_sdf(&ornament.sdf, 0.012, Vec3::splat(0.5));
    let bvh = Bvh::from_meshes(&[&mesh]);
    let mut hits = 0;
    let mut x = 0.4;
    while x < 1.6 {
        if bvh.any_hit(Vec3::new(x, 0.5, 1.0), Vec3::new(0.0, 0.0, -1.0), 2.0) {
            hits += 1;
        }
        x += 0.01;
    }
    assert!(hits > 20 && hits < 80, "ray hits {}", hits);
}

#[test]
fn iron_railing_is_a_see_through_silhouette() {
    let ornament = iron_railing(1.0, 1.4, 0.012, 0.11);
    let mesh = mesh_sdf(&ornament.sdf, 0.006, Vec3::splat(0.3));
    let stats = MeshStats::measure(&mesh);
    assert!(stats.silhouette_convexity[2] < 0.5, "{}", stats.silhouette_convexity[2]);
}

use engine::model::building::{stairs, StairSpec, MAX_RISE};

#[test]
fn stairs_follow_rise_and_run_rules() {
    let good = StairSpec { total_rise: 3.0, width: 1.2, run: 0.28 };
    assert!(good.rise() <= MAX_RISE);
    assert!(stairs(&good).is_ok());
    let steep = StairSpec { total_rise: 3.0, width: 1.2, run: 0.15 };
    assert!(stairs(&steep).err().unwrap().contains("run"));
    let awkward = StairSpec { total_rise: 3.0, width: 1.2, run: 0.5 };
    assert!(stairs(&awkward).err().unwrap().contains("comfort"));
}

use engine::texture::Texture;

#[test]
fn baked_maps_tile_seamlessly_and_match_requested_size() {
    let style = Style::overgrown_manor();
    let maps = style.bake_material_maps("stone_wall", 64).unwrap();
    assert_eq!(maps.albedo.texels.len(), 64 * 64);
    let max_step = |texture: &Texture, wrap: bool| {
        let mut worst = 0i32;
        for y in 0..64 {
            let (a, b) = if wrap { (texture.get(63, y), texture.get(0, y)) } else { (texture.get(31, y), texture.get(32, y)) };
            worst = worst.max((a[0] as i32 - b[0] as i32).abs());
        }
        worst
    };
    let seam = max_step(&maps.albedo, true);
    let interior = max_step(&maps.albedo, false);
    assert!(seam <= interior.max(24) + 8, "seam {} interior {}", seam, interior);
    assert!(maps.packed.get(10, 10)[0] > 100);
}

#[test]
fn unknown_material_cannot_be_baked() {
    assert!(Style::overgrown_manor().bake_material_maps("granite", 16).is_err());
}

use engine::model::vegetation::{build_tree, ground_cover, GroundCoverSpec, TreeSpec};

#[test]
fn trees_are_deterministic_and_seed_dependent() {
    let a = build_tree(&TreeSpec::oak(6.0));
    let b = build_tree(&TreeSpec::oak(6.0));
    let mut other = TreeSpec::oak(6.0);
    other.seed = 99;
    let c = build_tree(&other);
    assert_eq!(a.leaf_count, b.leaf_count);
    assert_eq!(a.foliage.positions, b.foliage.positions);
    assert_ne!(a.foliage.positions, c.foliage.positions);
    assert!(a.branch_count > 20 && a.leaf_count > 500);
}

#[test]
fn ground_cover_respects_density_and_clear_zones() {
    let spec = GroundCoverSpec { half_width: 2.0, half_depth: 2.0, blades_per_square_meter: 50.0, blade_height: 0.3, seed: 3 };
    let all = ground_cover(&spec, &|_, _| false);
    let cleared = ground_cover(&spec, &|x, _| x > 0.0);
    assert!(all.triangle_count() as f32 > 16.0 * 50.0 * 2.0 * 0.95);
    assert!(cleared.triangle_count() < all.triangle_count() * 6 / 10);
    assert!(cleared.positions.iter().all(|p| p.x <= 0.5));
}

use engine::model::character::{build_hand_poses, build_viewmodel};
use engine::model::humanoid::{hand, BodySpec, HandPose, Landmarks, FINGER_LENGTH_RATIOS};
use engine::model::instancing::{Prefab, PrefabLibrary};
use engine::model::props::carbine_sockets;

#[test]
fn adult_proportions_pass_all_anatomical_ratio_checks() {
    let spec = BodySpec::adult(1.78);
    let landmarks = Landmarks::for_spec(&spec);
    for check in landmarks.ratio_checks(spec.height) {
        assert!(check.passes(), "{}", check.message());
    }
}

#[test]
fn proportions_scale_with_height() {
    let tall = Landmarks::for_spec(&BodySpec::adult(2.0));
    let short = Landmarks::for_spec(&BodySpec::adult(1.5));
    assert!((tall.head_top.y / short.head_top.y - 2.0 / 1.5).abs() < 1e-4);
    assert!(tall.ratio_checks(2.0).iter().all(|c| c.passes()));
}

#[test]
fn finger_ratios_follow_anatomy() {
    let middle: f32 = FINGER_LENGTH_RATIOS[2].iter().sum();
    let index: f32 = FINGER_LENGTH_RATIOS[1].iter().sum();
    let ring: f32 = FINGER_LENGTH_RATIOS[3].iter().sum();
    let little: f32 = FINGER_LENGTH_RATIOS[4].iter().sum();
    assert!(middle > index && middle > ring && ring > little);
}

#[test]
fn fist_is_more_compact_than_open_hand() {
    let open = mesh_sdf(&hand(0.19, &HandPose::open()), 0.004, Vec3::splat(0.5)).bounds();
    let fist = mesh_sdf(&hand(0.19, &HandPose::fist()), 0.004, Vec3::splat(0.5)).bounds();
    assert!(fist.1.y - fist.0.y < (open.1.y - open.0.y) * 0.9);
}

#[test]
fn grip_sockets_place_hands_on_the_weapon() {
    let sockets = carbine_sockets();
    let placed = sockets.pistol_grip.place(hand(0.19, &HandPose::pistol_grip()));
    let (low, high) = mesh_sdf(&placed, 0.005, Vec3::splat(0.5)).bounds();
    let center = (low + high) * 0.5;
    assert!((center - sockets.pistol_grip.position).length() < 0.15, "{:?}", center);
}

#[test]
fn hand_pose_asset_and_viewmodel_build() {
    let style = Style::overgrown_manor();
    assert_eq!(build_hand_poses(&style, 0.19).unwrap().parts.len(), 5);
    assert!(build_viewmodel(&style, 0.19).unwrap().parts.iter().any(|p| p.name == "hand_support"));
}

#[test]
fn prefab_lods_cut_drawn_triangles_for_distant_instances() {
    let sphere = mesh_sdf(&Sdf::sphere(0.5), 0.015, Vec3::splat(0.5));
    let mut library = PrefabLibrary::default();
    let id = library.add_prefab(Prefab::from_mesh("ball", &sphere, &[0.3, 0.08], &[15.0, 40.0]));
    for index in 0..50 {
        library.place(id, Vec3::new(index as f32 * 3.0, 0.0, 0.0), 0.0);
    }
    let viewer = Vec3::ZERO;
    assert!(library.drawn_triangles(viewer) < sphere.triangle_count() * 50 / 2);
    assert!(library.stored_triangles() < sphere.triangle_count() * 2);
    assert!(library.flatten(viewer).triangle_count() == library.drawn_triangles(viewer));
}

#[test]
fn leaf_texture_has_cutout_alpha_and_glass_is_translucent() {
    let style = Style::overgrown_manor();
    let leaf = style.bake_material_maps("leaf", 64).unwrap();
    let opaque = leaf.albedo.texels.iter().filter(|t| t[3] == 255).count();
    let clear = leaf.albedo.texels.iter().filter(|t| t[3] == 0).count();
    assert!(opaque > 64 * 64 / 5 && clear > 64 * 64 / 5, "opaque {} clear {}", opaque, clear);
    let glass = style.bake_material_maps("glass", 16).unwrap();
    assert!(glass.albedo.texels.iter().all(|t| t[3] > 40 && t[3] < 90));
    assert!(glass.packed.get(3, 3)[0] < 30);
    assert!(style.bake_material_maps("stone_wall", 16).unwrap().albedo.texels.iter().all(|t| t[3] == 255));
}
