use super::paint::SurfacePaint;
use super::part::{Asset, Dimension, Part, PartKind};
use super::sdf::Sdf;
use crate::math::Vec3;

const ANODIZED_BASE: Vec3 = Vec3::new(0.20, 0.20, 0.22);
const ANODIZED_EDGE: Vec3 = Vec3::new(0.62, 0.62, 0.66);
const POLYMER_BASE: Vec3 = Vec3::new(0.13, 0.13, 0.14);
const POLYMER_EDGE: Vec3 = Vec3::new(0.24, 0.24, 0.26);
const PARKERIZED_BASE: Vec3 = Vec3::new(0.26, 0.28, 0.27);
const PARKERIZED_EDGE: Vec3 = Vec3::new(0.64, 0.64, 0.66);
const RUBBER: Vec3 = Vec3::new(0.06, 0.06, 0.065);
const STAGE_GRAY: Vec3 = Vec3::new(0.70, 0.71, 0.73);

const X_AXIS: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const Z_AXIS: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const RAIL_CENTER_HEIGHT: f32 = 0.031;
const RAIL_STEP: f32 = 0.0165;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn revolve_along_x(profile: &[[f32; 2]], start: f32) -> Sdf {
    Sdf::revolve(profile).rotate_z(-90.0).translate(v(start, 0.0, 0.0))
}

fn cylinder_along_z(radius: f32, length: f32, center: Vec3) -> Sdf {
    Sdf::cylinder(radius, length, radius * 0.25).rotate_x(90.0).translate(center)
}

fn picatinny_rail(start: f32, length: f32) -> Sdf {
    let rail = Sdf::round_box(v(length, 0.012, 0.022), 0.002).translate(v(start + length * 0.5, RAIL_CENTER_HEIGHT, 0.0));
    let slots = ((length - 0.016) / RAIL_STEP) as usize;
    let slot = Sdf::round_box(v(0.006, 0.006, 0.03), 0.0005)
        .linear_array_x(slots, RAIL_STEP)
        .translate(v(start + 0.012, RAIL_CENTER_HEIGHT + 0.006, 0.0));
    rail.subtract(slot)
}

fn upper_receiver() -> Sdf {
    let body = Sdf::round_box(v(0.255, 0.052, 0.043), 0.005).translate(v(0.1275, 0.0, 0.0));
    let ejection_port = Sdf::round_box(v(0.052, 0.022, 0.03), 0.003).translate(v(0.115, 0.010, 0.0255));
    let forward_assist = cylinder_along_z(0.0065, 0.014, v(0.17, 0.004, 0.0275));
    let deflector = Sdf::limb(v(0.083, -0.006, 0.021), v(0.098, -0.014, 0.0265), [0.0035, 0.012], [0.0035, 0.012], 0.001);
    let handle_block = Sdf::round_box(v(0.04, 0.012, 0.034), 0.003).translate(v(-0.012, 0.033, 0.0));
    let latch = |side: f32| Sdf::round_box(v(0.03, 0.012, 0.008), 0.002).translate(v(-0.03, 0.03, side * 0.0155));
    let rear_sight_base = Sdf::round_box(v(0.032, 0.02, 0.034), 0.003).translate(v(0.034, 0.0455, 0.0));
    let sight_wing = |side: f32| Sdf::round_box(v(0.024, 0.03, 0.006), 0.002).translate(v(0.034, 0.0555, side * 0.0155));
    let aperture = Sdf::cylinder(0.0035, 0.06, 0.0).rotate_z(90.0).translate(v(0.034, 0.060, 0.0));
    let delta_ring = Sdf::torus(0.0245, 0.0045).rotate_z(90.0).translate(v(0.262, 0.0, 0.0));
    body.subtract(ejection_port)
        .chamfer_union(picatinny_rail(0.0, 0.255), 0.002)
        .chamfer_union(forward_assist, 0.001)
        .chamfer_union(deflector, 0.001)
        .chamfer_union(handle_block, 0.001)
        .chamfer_union(latch(1.0), 0.001)
        .chamfer_union(latch(-1.0), 0.001)
        .chamfer_union(rear_sight_base.chamfer_union(sight_wing(1.0), 0.001).chamfer_union(sight_wing(-1.0), 0.001).subtract(aperture), 0.001)
        .chamfer_union(delta_ring, 0.001)
}

fn lower_receiver() -> Sdf {
    let body = Sdf::round_box(v(0.205, 0.047, 0.040), 0.005).translate(v(0.085, -0.048, 0.0));
    let magazine_well = Sdf::round_box(v(0.060, 0.050, 0.040), 0.004).translate(v(0.147, -0.092, 0.0));
    let rear_mount = Sdf::round_box(v(0.034, 0.036, 0.036), 0.004).translate(v(-0.02, -0.030, 0.0));
    let grip_path = [v(-0.003, -0.068, 0.0), v(-0.0215, -0.118, 0.0), v(-0.04, -0.168, 0.0)];
    let grip_sizes = [[0.0158, 0.019], [0.0155, 0.0175], [0.0148, 0.0165]];
    let grip = Sdf::sweep(&grip_path, &grip_sizes, 0.004, 5, Z_AXIS);
    let finger_groove = |y: f32| {
        let x = -0.003 + 0.368 * (y + 0.068);
        Sdf::sphere(0.0095).translate(v(x + 0.0195, y, 0.0))
    };
    let grooves = finger_groove(-0.092).union(finger_groove(-0.116)).union(finger_groove(-0.140));
    let guard_outer = [[0.052, -0.0715], [0.145, -0.0715], [0.151, -0.083], [0.147, -0.108], [0.062, -0.108], [0.052, -0.098]];
    let guard_inner = [[0.060, -0.0715], [0.138, -0.0715], [0.1415, -0.080], [0.139, -0.100], [0.067, -0.100], [0.060, -0.092]];
    let trigger_guard = Sdf::extrude(&guard_outer, 0.008, 0.0025).subtract(Sdf::extrude(&guard_inner, 0.03, 0.0));
    let trigger_path = [v(0.090, -0.068, 0.0), v(0.085, -0.084, 0.0), v(0.0775, -0.097, 0.0)];
    let trigger = Sdf::sweep(&trigger_path, &[[0.0036, 0.0048], [0.0036, 0.0044], [0.0036, 0.004]], 0.001, 4, Z_AXIS);
    let magazine_release = cylinder_along_z(0.0048, 0.01, v(0.118, -0.066, 0.0222));
    let selector = Sdf::round_box(v(0.024, 0.0065, 0.0055), 0.002).translate(v(0.004, -0.050, -0.0225));
    let takedown_pin = |x: f32, y: f32, side: f32| cylinder_along_z(0.0048, 0.008, v(x, y, side * 0.0205));
    let hardware = takedown_pin(-0.014, -0.052, 1.0)
        .union(takedown_pin(-0.014, -0.052, -1.0))
        .union(takedown_pin(0.158, -0.036, 1.0))
        .union(takedown_pin(0.158, -0.036, -1.0));
    Sdf::smooth_union_all(vec![body, magazine_well, rear_mount, grip.smooth_subtract(grooves, 0.004), trigger_guard, trigger], 0.003)
        .chamfer_union(magazine_release, 0.001)
        .chamfer_union(selector, 0.001)
        .chamfer_union(hardware, 0.001)
}

fn magazine() -> Sdf {
    let path = [v(0.139, -0.07, 0.0), v(0.143, -0.12, 0.0), v(0.153, -0.168, 0.0), v(0.169, -0.208, 0.0)];
    let sizes = [[0.0128, 0.0268], [0.0128, 0.0268], [0.0126, 0.0266], [0.0126, 0.0268]];
    let body = Sdf::sweep(&path, &sizes, 0.0035, 6, Z_AXIS);
    let rib = Sdf::round_box(v(0.0016, 0.07, 0.004), 0.0004).linear_array_x(9, 0.0105);
    let side_ribs = |side: f32| rib.clone().rotate_z(90.0).translate(v(0.153, -0.095, side * 0.0128));
    let base_plate = Sdf::round_box(v(0.058, 0.007, 0.0284), 0.002).translate(v(0.1705, -0.2085, 0.0));
    body.subtract(side_ribs(1.0)).subtract(side_ribs(-1.0)).chamfer_union(base_plate, 0.001)
}

fn buffer_tube() -> Sdf {
    let tube = Sdf::revolve(&[[0.0, 0.0], [0.0175, 0.0], [0.0175, 0.2], [0.0, 0.2]]).rotate_z(90.0).translate(v(0.0, -0.005, 0.0));
    let ring = Sdf::cylinder(0.0205, 0.0012, 0.0).subtract(Sdf::cylinder(0.0168, 0.01, 0.0));
    let rings = ring.rotate_z(90.0).linear_array_x(12, 0.0085).translate(v(-0.03, -0.005, 0.0));
    let castle_nut = Sdf::cylinder(0.0225, 0.01, 0.002).rotate_z(90.0).translate(v(-0.015, -0.005, 0.0));
    tube.subtract(rings).chamfer_union(castle_nut, 0.001)
}

fn stock_body() -> Sdf {
    let path = [v(-0.105, 0.002, 0.0), v(-0.19, -0.001, 0.0), v(-0.25, -0.006, 0.0), v(-0.29, -0.012, 0.0)];
    let sizes = [[0.0205, 0.030], [0.0215, 0.036], [0.0225, 0.043], [0.0228, 0.050]];
    let shell = Sdf::sweep(&path, &sizes, 0.006, 6, Z_AXIS);
    let cheek_slot = Sdf::round_box(v(0.05, 0.02, 0.016), 0.004).translate(v(-0.19, -0.03, 0.0));
    shell.subtract(cheek_slot)
}

fn butt_pad() -> Sdf {
    Sdf::round_box(v(0.016, 0.100, 0.048), 0.006).translate(v(-0.2975, -0.0125, 0.0))
}

fn handguard() -> Sdf {
    let profile = [[0.0, 0.0], [0.0228, 0.0], [0.0228, 0.158], [0.0, 0.158]];
    let vent = Sdf::round_box(v(0.014, 0.1, 0.0075), 0.003).translate(v(0.0228, 0.079, 0.0));
    let tube = Sdf::revolve(&profile).subtract(vent.radial_array(8));
    let end_ridge = |position: f32| Sdf::cylinder(0.0248, 0.006, 0.002).translate(v(0.0, position, 0.0));
    let ridged = tube.chamfer_union(end_ridge(0.004), 0.001).chamfer_union(end_ridge(0.154), 0.001);
    revolve_shape_along_x(ridged, 0.262).chamfer_union(picatinny_rail(0.262, 0.158), 0.002)
}

fn revolve_shape_along_x(shape: Sdf, start: f32) -> Sdf {
    shape.rotate_z(-90.0).translate(v(start, 0.0, 0.0))
}

fn barrel_group() -> Sdf {
    let barrel = revolve_along_x(&[[0.0, 0.0], [0.0105, 0.0], [0.0105, 0.19], [0.0092, 0.19], [0.0092, 0.245], [0.0, 0.245]], 0.262);
    let gas_block = revolve_along_x(&[[0.0, 0.0], [0.0148, 0.0], [0.0148, 0.035], [0.0, 0.035]], 0.42);
    let tower_profile = [[0.426, 0.012], [0.452, 0.012], [0.4455, 0.050], [0.4325, 0.050]];
    let tower = Sdf::extrude(&tower_profile, 0.0075, 0.0012);
    let ear = |side: f32| Sdf::round_box(v(0.014, 0.016, 0.0022), 0.0008).translate(v(0.439, 0.044, side * 0.0085));
    let sight_base = Sdf::round_box(v(0.028, 0.012, 0.022), 0.002).translate(v(0.439, 0.0385, 0.0));
    let post = Sdf::limb(v(0.439, 0.045, 0.0), v(0.439, 0.064, 0.0), [0.0030, 0.0032], [0.0024, 0.0032], 0.0008);
    let slot = Sdf::round_box(v(0.012, 0.026, 0.005), 0.0015).translate(v(0.0125, 0.0255, 0.0));
    let hider_profile = [[0.0, 0.0], [0.0105, 0.0], [0.0117, 0.008], [0.0117, 0.040], [0.0088, 0.046], [0.0, 0.046]];
    let hider = revolve_along_x(&hider_profile, 0.499).subtract(revolve_slots(slot));
    Sdf::smooth_union_all(vec![barrel, gas_block, tower, sight_base, ear(1.0), ear(-1.0), post], 0.0015).chamfer_union(hider, 0.0008)
}

fn revolve_slots(slot: Sdf) -> Sdf {
    slot.radial_array(6).rotate_z(-90.0).translate(v(0.499, 0.0, 0.0))
}

pub fn build_carbine(cell: f32) -> Asset {
    let anodized = SurfacePaint::worn(ANODIZED_BASE, ANODIZED_EDGE).with_surface(0.45, 0.85).with_edge_surface(0.3, 1.0).with_seed(3);
    let polymer = SurfacePaint::worn(POLYMER_BASE, POLYMER_EDGE).with_surface(0.7, 0.0).with_edge_surface(0.5, 0.0).with_grain(0.35, 900.0).with_seed(5);
    let parkerized = SurfacePaint::worn(PARKERIZED_BASE, PARKERIZED_EDGE).with_surface(0.6, 0.7).with_edge_surface(0.35, 1.0).with_grain(0.2, 700.0).with_seed(9);
    let rubber = SurfacePaint::plain(RUBBER).with_grain(0.4, 800.0);

    let mut asset = Asset::new("carbine");
    asset.studio_lighting = true;
    let shapes = [
        ("upper_receiver", upper_receiver(), anodized),
        ("lower_receiver", lower_receiver(), anodized.with_seed(4)),
        ("magazine", magazine(), polymer.with_seed(6)),
        ("buffer_tube", buffer_tube(), anodized.with_seed(7)),
        ("stock", stock_body(), polymer),
        ("butt_pad", butt_pad(), rubber),
        ("handguard", handguard(), polymer.with_seed(8)),
        ("barrel_group", barrel_group(), parkerized),
    ];
    for (name, shape, paint) in shapes {
        asset.parts.push(Part::from_sdf_painted(name, PartKind::Object, &shape, cell, paint, Vec::new()));
    }
    let (low, high) = asset.object_bounds();
    asset.add_stage_floor((low.x + high.x) * 0.5, 0.0, 0.9, 0.02, low.y - 0.03, STAGE_GRAY);
    asset.dimensions = vec![
        Dimension::new("carbine", "overall length", high.x - low.x, 0.74, 0.88),
        Dimension::new("carbine", "overall height", high.y - low.y, 0.20, 0.32),
        Dimension::new("carbine", "width", high.z - low.z, 0.04, 0.075),
        Dimension::new("carbine", "barrel and muzzle length", 0.283, 0.25, 0.42),
    ];
    asset
}
