use super::detail::{build_styled_part, DetailSpec};
use super::part::{Asset, Part, PartKind};
use super::style::Style;
use super::vegetation::{build_tree, ground_cover, GroundCoverSpec, TreeSpec};
use crate::math::Vec3;

pub fn build_garden(style: &Style, tree: &TreeSpec) -> Result<Asset, String> {
    let mut asset = Asset::new("garden");
    asset.add_stage_floor(0.0, 0.0, 6.0, 0.25, 0.0, Vec3::new(0.12, 0.18, 0.07));
    let grown = build_tree(tree);
    let detail = DetailSpec::new(grown.smallest_feature.max(0.05)).with_budget(900_000);
    asset.parts.push(build_styled_part("trunk", PartKind::Object, &grown.wood, style, "bark", detail, Vec::new())?);
    asset.parts.push(Part::from_mesh("foliage", PartKind::Object, grown.foliage));
    let trunk_clearance = tree.trunk_radius * 2.5;
    let cover = ground_cover(
        &GroundCoverSpec { half_width: 5.0, half_depth: 5.0, blades_per_square_meter: 60.0, blade_height: 0.35, seed: tree.seed + 1 },
        &|x, z| x * x + z * z < trunk_clearance * trunk_clearance,
    );
    asset.parts.push(Part::from_mesh("grass", PartKind::Object, cover));
    Ok(asset)
}
