use super::detail::{build_styled_part, DetailSpec};
use super::ornament::{arch_opening_wall, balustrade, column, cornice, iron_railing, BalustradeSpec, Ornament};
use super::part::{Asset, Part, PartKind};
use super::sdf::Sdf;
use super::style::Style;
use crate::math::Vec3;

const SLAB_WIDTH: f32 = 6.0;
const SLAB_DEPTH: f32 = 5.0;
const SLAB_THICKNESS: f32 = 0.3;
const MODULE_LENGTH: f32 = 2.0;
const WALL_HEIGHT: f32 = 3.4;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn styled(name: &str, ornament: &Ornament, style: &Style, material: &str) -> Result<Part, String> {
    let detail = DetailSpec::new(ornament.smallest_feature).with_budget(900_000);
    build_styled_part(name, PartKind::Object, &ornament.sdf, style, material, detail, Vec::new())
}

fn placed(part: &Part, name: &str, position: Vec3, yaw_degrees: f32) -> Part {
    let mut copy = part.clone();
    copy.name = name.to_string();
    copy.mesh.rotate_y(yaw_degrees);
    copy.mesh.translate(position);
    copy
}

pub fn build_terrace(style: &Style) -> Result<Asset, String> {
    let mut asset = Asset::new("terrace");
    asset.studio_lighting = false;

    let slab = Sdf::round_box(v(SLAB_WIDTH, SLAB_THICKNESS, SLAB_DEPTH), 0.03).translate(v(0.0, -SLAB_THICKNESS * 0.5, 0.0));
    asset.parts.push(build_styled_part("floor", PartKind::Architecture, &slab, style, "cobble", DetailSpec::new(0.09), Vec::new())?);

    let module = styled("balustrade", &balustrade(&BalustradeSpec::terrace(MODULE_LENGTH)), style, "stone_wall")?;
    let front_z = SLAB_DEPTH * 0.5 - 0.15;
    for index in 0..3 {
        let x = -SLAB_WIDTH * 0.5 + index as f32 * MODULE_LENGTH;
        asset.parts.push(placed(&module, &format!("balustrade_front_{}", index), v(x, 0.0, front_z), 0.0));
    }
    let side_x = SLAB_WIDTH * 0.5 - 0.15;
    for index in 0..2 {
        let z = front_z - 0.15 - index as f32 * MODULE_LENGTH;
        asset.parts.push(placed(&module, &format!("balustrade_right_{}", index), v(side_x, 0.0, z), 90.0));
        asset.parts.push(placed(&module, &format!("balustrade_left_{}", index), v(-side_x, 0.0, z), 90.0));
    }

    let wall_z = -SLAB_DEPTH * 0.5 + 0.3;
    let wall = arch_opening_wall(SLAB_WIDTH - 0.6, WALL_HEIGHT, 0.5, 1.7, 2.6);
    asset.parts.push(placed(&styled("wall", &wall, style, "stone_wall")?, "back_wall", v(0.0, 0.0, wall_z), 0.0));

    let crown = cornice(SLAB_WIDTH - 0.4, 0.35, 0.4);
    asset.parts.push(placed(&styled("cornice", &crown, style, "stone_wall")?, "back_cornice", v(-(SLAB_WIDTH - 0.4) * 0.5, WALL_HEIGHT - 0.02, wall_z + 0.28), 0.0));

    let pillar = styled("column", &column(WALL_HEIGHT - 0.1, 0.2, 12), style, "stone_wall")?;
    for (index, x) in [-1.5, 1.5].iter().enumerate() {
        asset.parts.push(placed(&pillar, &format!("column_{}", index), v(*x, 0.0, wall_z + 0.5), 0.0));
    }

    let gate = iron_railing(1.5, 2.1, 0.012, 0.11);
    asset.parts.push(placed(&styled("gate", &gate, style, "wrought_iron")?, "gate", v(-0.75, 0.0, wall_z - 0.05), 0.0));
    Ok(asset)
}
