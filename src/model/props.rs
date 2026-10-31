use super::part::{Asset, Dimension, Part, PartKind};
use super::sdf::Sdf;
use crate::math::Vec3;

const BLACK_METAL: Vec3 = Vec3::new(0.24, 0.24, 0.26);
const STEEL: Vec3 = Vec3::new(0.46, 0.46, 0.50);
const POLYMER: Vec3 = Vec3::new(0.17, 0.17, 0.18);

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn along_x(shape: Sdf, start: f32) -> Sdf {
    shape.rotate_z(-90.0).translate(v(start, 0.0, 0.0))
}

fn rail_with_grooves(start: f32, length: f32, height: f32) -> Sdf {
    let rail = Sdf::round_box(v(length, 0.012, 0.022), 0.002).translate(v(start + length * 0.5, height, 0.0));
    let step = 0.0165;
    let count = ((length - 0.02) / step) as usize;
    let groove = Sdf::round_box(v(0.006, 0.006, 0.03), 0.0005)
        .linear_array_x(count, step)
        .translate(v(start + 0.01, height + 0.006, 0.0));
    rail.subtract(groove)
}

pub fn build_carbine(cell: f32) -> Asset {
    let upper_body = Sdf::round_box(v(0.25, 0.05, 0.042), 0.006).translate(v(0.125, 0.0, 0.0));
    let ejection_port = Sdf::round_box(v(0.05, 0.02, 0.03), 0.003).translate(v(0.12, 0.012, 0.022));
    let rear_sight = Sdf::round_box(v(0.022, 0.03, 0.026), 0.003)
        .translate(v(0.02, 0.05, 0.0))
        .subtract(Sdf::cylinder(0.006, 0.05, 0.0).rotate_z(90.0).translate(v(0.02, 0.06, 0.0)));
    let upper = upper_body
        .subtract(ejection_port)
        .chamfer_union(rail_with_grooves(0.0, 0.30, 0.031), 0.002)
        .chamfer_union(rear_sight, 0.002);

    let lower_body = Sdf::round_box(v(0.20, 0.045, 0.036), 0.006).translate(v(0.09, -0.0475, 0.0));
    let magazine_well = Sdf::round_box(v(0.052, 0.05, 0.036), 0.004).translate(v(0.14, -0.085, 0.0));
    let grip_profile = [[-0.02, 0.0], [0.02, 0.0], [0.003, -0.105], [-0.04, -0.105]];
    let grip = Sdf::extrude(&grip_profile, 0.032, 0.005).translate(v(0.0, -0.065, 0.0));
    let guard_outer = [[0.045, -0.065], [0.135, -0.065], [0.135, -0.105], [0.045, -0.105]];
    let guard_inner = [[0.056, -0.065], [0.124, -0.065], [0.124, -0.092], [0.056, -0.092]];
    let trigger_guard = Sdf::extrude(&guard_outer, 0.008, 0.002).subtract(Sdf::extrude(&guard_inner, 0.03, 0.0));
    let trigger = Sdf::limb(v(0.085, -0.066, 0.0), v(0.078, -0.092, 0.0), [0.003, 0.0035], [0.002, 0.003], 0.001);
    let lower = Sdf::smooth_union_all(vec![lower_body, magazine_well, grip, trigger_guard, trigger], 0.004);

    let magazine_profile = [[-0.014, 0.0], [0.014, 0.0], [0.024, -0.07], [0.038, -0.135], [0.006, -0.135], [0.002, -0.07]];
    let magazine = Sdf::extrude(&magazine_profile, 0.026, 0.003).translate(v(0.14, -0.075, 0.0));

    let tube = Sdf::revolve(&[[0.0, 0.0], [0.0175, 0.0], [0.0175, 0.19], [0.0, 0.19]])
        .rotate_z(90.0)
        .translate(v(0.0, -0.005, 0.0));
    let stock_profile = [[-0.11, 0.036], [-0.30, 0.045], [-0.315, -0.075], [-0.25, -0.08], [-0.11, -0.012]];
    let stock = Sdf::extrude(&stock_profile, 0.046, 0.009);
    let stock_group = stock.smooth_union(tube, 0.006);

    let handguard_profile = [[0.0, 0.0], [0.0215, 0.0], [0.0215, 0.17], [0.0, 0.17]];
    let vent = Sdf::round_box(v(0.012, 0.12, 0.007), 0.002).translate(v(0.0215, 0.085, 0.0));
    let handguard = along_x(Sdf::revolve(&handguard_profile).subtract(vent.radial_array(8)), 0.23);

    let barrel_profile = [[0.0, 0.0], [0.0095, 0.0], [0.0095, 0.23], [0.0, 0.23]];
    let barrel = along_x(Sdf::revolve(&barrel_profile), 0.23);
    let gas_block = along_x(Sdf::revolve(&[[0.0, 0.0], [0.0145, 0.0], [0.0145, 0.03], [0.0, 0.03]]), 0.385);
    let sight_post = Sdf::limb(v(0.395, 0.01, 0.0), v(0.395, 0.052, 0.0), [0.005, 0.006], [0.003, 0.004], 0.0015);
    let sight_base = Sdf::round_box(v(0.03, 0.014, 0.024), 0.003).translate(v(0.395, 0.018, 0.0));
    let slot = Sdf::round_box(v(0.012, 0.036, 0.0046), 0.0012).translate(v(0.0125, 0.031, 0.0));
    let flash_hider_profile = [[0.0, 0.0], [0.0105, 0.0], [0.0115, 0.01], [0.0115, 0.055], [0.0085, 0.062], [0.0, 0.062]];
    let flash_hider = along_x(Sdf::revolve(&flash_hider_profile).subtract(slot.radial_array(6)), 0.46);
    let barrel_group = Sdf::smooth_union_all(vec![barrel, gas_block, sight_post, sight_base], 0.002).chamfer_union(flash_hider, 0.001);

    let mut asset = Asset::new("carbine");
    let parts = [
        ("upper_receiver", upper, BLACK_METAL),
        ("lower_receiver", lower, BLACK_METAL),
        ("magazine", magazine, POLYMER),
        ("stock", stock_group, POLYMER),
        ("handguard", handguard, POLYMER),
        ("barrel_group", barrel_group, STEEL),
    ];
    for (name, shape, color) in parts {
        asset.parts.push(Part::from_sdf(name, PartKind::Object, &shape, cell, color, Vec::new()));
    }
    let (low, high) = asset.object_bounds();
    asset.dimensions = vec![
        Dimension::new("carbine", "overall length", high.x - low.x, 0.74, 0.86),
        Dimension::new("carbine", "overall height", high.y - low.y, 0.20, 0.32),
        Dimension::new("carbine", "width", high.z - low.z, 0.04, 0.07),
        Dimension::new("carbine", "barrel length", 0.29, 0.25, 0.42),
    ];
    asset
}
