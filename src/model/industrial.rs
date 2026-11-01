use super::building::{stairs, StairSpec};
use super::detail::{build_styled_part, DetailSpec};
use super::part::{Asset, PartKind};
use super::sdf::Sdf;
use super::style::Style;
use crate::math::Vec3;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Detail {
    Game,
    Hero,
}

impl Detail {
    fn cells_per_feature(self) -> f32 {
        match self {
            Detail::Game => 2.2,
            Detail::Hero => 3.0,
        }
    }

    fn minimum_bar_radius(self) -> f32 {
        match self {
            Detail::Game => 0.05,
            Detail::Hero => 0.0,
        }
    }
}

thread_local! {
    static BAR_RADIUS_FLOOR: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

pub struct Prop {
    pub name: String,
    pub asset: Asset,
    pub footprint_half: (f32, f32),
    pub top_height: f32,
}

impl Prop {
    pub fn prefab(&self, triangle_ratios: &[f32], switch_distances: &[f32]) -> super::instancing::Prefab {
        super::instancing::Prefab::from_mesh(&self.name, &self.asset.merged_mesh(&[]), triangle_ratios, switch_distances)
    }
}

pub fn prop_names() -> Vec<&'static str> {
    vec![
        "container_20_red",
        "container_20_blue",
        "container_20_sand",
        "container_40_red",
        "container_40_blue",
        "container_40_sand",
        "fuel_tank",
        "derrick",
        "pipe_rack",
        "office",
        "jersey_barrier",
        "sandbag_wall",
        "crate",
        "pallet_stack",
        "catwalk",
        "catwalk_stairs",
        "water_tower",
        "wreck_truck",
    ]
}

struct Builder<'a> {
    style: &'a Style,
    asset: Asset,
    detail: Detail,
}

impl<'a> Builder<'a> {
    fn new(name: &str, style: &'a Style) -> Builder<'a> {
        Builder { style, asset: Asset::new(name), detail: current_detail() }
    }

    fn spec(&self, feature: f32) -> DetailSpec {
        DetailSpec { cells_per_feature: self.detail.cells_per_feature(), ..DetailSpec::new(feature).with_budget(700_000) }
    }

    fn add(&mut self, name: &str, sdf: &Sdf, material: &str, feature: f32) -> Result<(), String> {
        let detail = self.spec(feature);
        self.asset.parts.push(build_styled_part(name, PartKind::Object, sdf, self.style, material, detail, Vec::new())?);
        Ok(())
    }

    fn add_flat(&mut self, name: &str, sdf: &Sdf, material: &str, feature: f32) -> Result<(), String> {
        let detail = self.spec(feature);
        self.asset.parts.push(build_styled_part(name, PartKind::Object, sdf, self.style, material, detail, Vec::new())?);
        Ok(())
    }

    fn finish(self) -> Prop {
        let (low, high) = self.asset.object_bounds();
        Prop {
            name: self.asset.name.clone(),
            footprint_half: (((high.x - low.x) * 0.5), ((high.z - low.z) * 0.5)),
            top_height: high.y,
            asset: self.asset,
        }
    }
}

fn current_detail() -> Detail {
    if BAR_RADIUS_FLOOR.with(|floor| floor.get()) > 0.0 { Detail::Game } else { Detail::Hero }
}

fn bar(a: Vec3, b: Vec3, radius: f32) -> Sdf {
    let radius = radius.max(BAR_RADIUS_FLOOR.with(|floor| floor.get()));
    Sdf::limb(a, b, [radius, radius], [radius, radius], radius * 0.8)
}

fn zigzag_panel(length: f32, height: f32, pitch: f32, amplitude: f32) -> Sdf {
    let teeth = (length / pitch).floor() as usize;
    let step = length / teeth as f32;
    let mut points = Vec::new();
    for tooth in 0..teeth {
        let x = -length * 0.5 + tooth as f32 * step;
        points.push([x, 0.0]);
        points.push([x + step * 0.25, amplitude]);
        points.push([x + step * 0.5, amplitude]);
        points.push([x + step * 0.75, 0.0]);
    }
    points.push([length * 0.5, 0.0]);
    points.push([length * 0.5, -0.03]);
    points.push([-length * 0.5, -0.03]);
    Sdf::extrude(&points, height, 0.005).rotate_x(-90.0).translate(v(0.0, height * 0.5, 0.0))
}

fn container(style: &Style, length: f32, paint: &str) -> Result<Prop, String> {
    let (w, h) = (2.44, 2.59);
    let mut b = Builder::new(&format!("container_{}_{}", if length > 8.0 { 40 } else { 20 }, paint.trim_start_matches("container_")), style);
    let panel_height = h - 0.24;
    let side = zigzag_panel(length - 0.2, panel_height, 0.16, 0.04).translate(v(0.0, 0.12, w * 0.5 - 0.05));
    let other = zigzag_panel(length - 0.2, panel_height, 0.16, 0.04).rotate_y(180.0).translate(v(0.0, 0.12, -w * 0.5 + 0.05));
    let roof = Sdf::round_box(v(length - 0.1, 0.08, w - 0.1), 0.02).translate(v(0.0, h - 0.05, 0.0));
    let floor = Sdf::round_box(v(length - 0.1, 0.12, w - 0.1), 0.02).translate(v(0.0, 0.18, 0.0));
    let post = |x: f32, z: f32| Sdf::round_box(v(0.16, h, 0.16), 0.02).translate(v(x, h * 0.5, z));
    let rail = |y: f32, z: f32| Sdf::round_box(v(length, 0.14, 0.14), 0.02).translate(v(0.0, y, z));
    let mut frame = vec![roof, floor, post(length * 0.5 - 0.08, w * 0.5 - 0.08), post(length * 0.5 - 0.08, -w * 0.5 + 0.08), post(-length * 0.5 + 0.08, w * 0.5 - 0.08), post(-length * 0.5 + 0.08, -w * 0.5 + 0.08)];
    for z in [w * 0.5 - 0.08, -w * 0.5 + 0.08] {
        frame.push(rail(h - 0.07, z));
        frame.push(rail(0.07, z));
    }
    let end_wall = |x: f32| Sdf::round_box(v(0.1, h - 0.1, w - 0.2), 0.02).translate(v(x, h * 0.5, 0.0));
    frame.push(end_wall(-length * 0.5 + 0.05));
    frame.push(end_wall(length * 0.5 - 0.05));
    let door_x = length * 0.5;
    let mut door_detail = Vec::new();
    for offset in [-0.55, -0.18, 0.18, 0.55] {
        door_detail.push(bar(v(door_x + 0.06, 0.25, offset), v(door_x + 0.06, h - 0.25, offset), 0.018));
        door_detail.push(Sdf::round_box(v(0.04, 0.12, 0.08), 0.01).translate(v(door_x + 0.06, 1.3, offset)));
    }
    door_detail.push(Sdf::round_box(v(0.04, h - 0.4, 0.02), 0.005).translate(v(door_x + 0.05, h * 0.5, 0.0)));
    b.add_flat("frame", &Sdf::smooth_union_all(frame, 0.01), paint, 0.09)?;
    b.add("side_a", &side, paint, 0.1)?;
    b.add("side_b", &other, paint, 0.1)?;
    b.add("door_hardware", &Sdf::smooth_union_all(door_detail, 0.006), "steel", 0.04)?;
    Ok(b.finish())
}

fn fuel_tank(style: &Style) -> Result<Prop, String> {
    let (radius, height) = (5.0, 8.0);
    let mut b = Builder::new("fuel_tank", style);
    let shell = Sdf::cylinder(radius, height * 0.5, 0.08).translate(v(0.0, height * 0.5, 0.0));
    let rim = Sdf::cylinder(radius + 0.12, 0.1, 0.04).translate(v(0.0, height + 0.08, 0.0));
    let seam = |y: f32| Sdf::cylinder(radius + 0.05, 0.1, 0.03).translate(v(0.0, y, 0.0));
    let mut tank = vec![shell, rim];
    for level in 1..4 {
        tank.push(seam(level as f32 * height * 0.25));
    }
    b.add_flat("shell", &Sdf::smooth_union_all(tank, 0.02), "steel", 0.2)?;
    let rung_count = 24usize;
    let ladder_x = radius + 0.28;
    let rails = bar(v(ladder_x, 0.2, -0.25), v(ladder_x, height + 0.9, -0.25), 0.025).union(bar(v(ladder_x, 0.2, 0.25), v(ladder_x, height + 0.9, 0.25), 0.025));
    let rungs = bar(v(ladder_x, 0.0, -0.25), v(ladder_x, 0.0, 0.25), 0.018).translate(v(0.0, 0.0, 0.0));
    let mut ladder = vec![rails];
    for i in 0..rung_count {
        ladder.push(rungs.clone().translate(v(0.0, 0.4 + i as f32 * 0.38, 0.0)));
    }
    let standoff = |y: f32| bar(v(radius, y, -0.25), v(ladder_x, y, -0.25), 0.02).union(bar(v(radius, y, 0.25), v(ladder_x, y, 0.25), 0.02));
    ladder.push(standoff(1.5));
    ladder.push(standoff(4.5));
    ladder.push(standoff(7.5));
    b.add("ladder", &Sdf::smooth_union_all(ladder, 0.01), "rusted_steel", 0.1)?;
    let stub = Sdf::cylinder(0.22, 0.6, 0.02).rotate_z(90.0).translate(v(-radius - 0.5, 0.7, 0.0));
    let flange = Sdf::cylinder(0.32, 0.05, 0.01).rotate_z(90.0).translate(v(-radius - 1.05, 0.7, 0.0));
    let valve = Sdf::cylinder(0.12, 0.15, 0.02).translate(v(-radius - 0.5, 1.05, 0.0));
    b.add("pipe_stub", &Sdf::smooth_union_all(vec![stub, flange, valve], 0.02), "rusted_steel", 0.1)?;
    Ok(b.finish())
}

fn derrick(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("derrick", style);
    let (base, top, height, deck) = (2.6, 0.7, 14.0, 6.0);
    let half_at = |y: f32| base * 0.5 + (top * 0.5 - base * 0.5) * (y / height);
    let corner = |sx: f32, sz: f32, y: f32| v(sx * half_at(y), y, sz * half_at(y));
    let mut members = Vec::new();
    let corners = [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)];
    for (sx, sz) in corners {
        members.push(bar(corner(sx, sz, 0.0), corner(sx, sz, height), 0.09));
    }
    let levels = [0.0, 3.0, 6.0, 9.0, 12.0, 14.0];
    for pair in levels.windows(2) {
        let (y0, y1) = (pair[0], pair[1]);
        for i in 0..4 {
            let (a, c) = (corners[i], corners[(i + 1) % 4]);
            members.push(bar(corner(a.0, a.1, y0), corner(c.0, c.1, y1), 0.04));
            members.push(bar(corner(c.0, c.1, y0), corner(a.0, a.1, y1), 0.04));
            members.push(bar(corner(a.0, a.1, y1), corner(c.0, c.1, y1), 0.05));
        }
    }
    b.add("lattice", &Sdf::smooth_union_all(members, 0.02), "olive_metal", 0.1)?;
    let deck_half = half_at(deck) + 0.6;
    let platform = Sdf::round_box(v(deck_half * 2.0, 0.12, deck_half * 2.0), 0.02).translate(v(0.0, deck, 0.0));
    let mut railing = Vec::new();
    for (sx, sz) in corners {
        railing.push(bar(v(sx * deck_half, deck, sz * deck_half), v(sx * deck_half, deck + 1.1, sz * deck_half), 0.03));
    }
    for i in 0..4 {
        let (a, c) = (corners[i], corners[(i + 1) % 4]);
        for height_fraction in [0.55, 1.1] {
            railing.push(bar(v(a.0 * deck_half, deck + height_fraction, a.1 * deck_half), v(c.0 * deck_half, deck + height_fraction, c.1 * deck_half), 0.025));
        }
    }
    b.add("deck", &platform, "steel", 0.1)?;
    b.add("railing", &Sdf::smooth_union_all(railing, 0.01), "steel", 0.1)?;
    let crown = Sdf::round_box(v(1.0, 0.6, 1.0), 0.05).translate(v(0.0, height + 0.3, 0.0)).union(Sdf::cylinder(0.3, 0.15, 0.03).rotate_x(90.0).translate(v(0.0, height + 0.9, 0.0)));
    b.add("crown", &crown, "rusted_steel", 0.1)?;
    Ok(b.finish())
}

fn pipe_rack(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("pipe_rack", style);
    let span = 4.0;
    let mut frame = Vec::new();
    for x in [-span * 0.5, span * 0.5] {
        for z in [-0.9, 0.9] {
            frame.push(Sdf::round_box(v(0.22, 3.0, 0.22), 0.03).translate(v(x, 1.5, z)));
        }
        for y in [1.0, 2.2, 3.0] {
            frame.push(Sdf::round_box(v(0.2, 0.2, 2.1), 0.03).translate(v(x, y, 0.0)));
        }
        frame.push(bar(v(x, 0.2, -0.9), v(x, 2.2, 0.9), 0.035));
        frame.push(bar(v(x, 0.2, 0.9), v(x, 2.2, -0.9), 0.035));
    }
    frame.push(Sdf::round_box(v(span + 0.3, 0.16, 0.16), 0.02).translate(v(0.0, 3.0, -0.9)));
    frame.push(Sdf::round_box(v(span + 0.3, 0.16, 0.16), 0.02).translate(v(0.0, 3.0, 0.9)));
    b.add_flat("frame", &Sdf::smooth_union_all(frame, 0.01), "steel", 0.1)?;
    let mut pipes = Vec::new();
    for (index, (y, z)) in [(1.2, -0.45), (1.2, 0.45), (2.4, 0.0)].iter().enumerate() {
        let radius = 0.2;
        pipes.push(Sdf::cylinder(radius, span * 0.5 + 0.4, 0.02).rotate_z(90.0).translate(v(0.0, *y + 0.2, *z)));
        for x in [-span * 0.5, 0.0, span * 0.5] {
            pipes.push(Sdf::cylinder(radius + 0.07, 0.04, 0.01).rotate_z(90.0).translate(v(x + 0.1 * index as f32, *y + 0.2, *z)));
        }
    }
    b.add("pipes", &Sdf::smooth_union_all(pipes, 0.01), "rusted_steel", 0.08)?;
    Ok(b.finish())
}

fn office(style: &Style) -> Result<Prop, String> {
    use super::building::{wall_with_openings, window_frame, window_glass, Opening};
    let (w, d, h) = (8.0, 6.0, 3.2);
    let mut b = Builder::new("office", style);
    let door = Opening { x: 2.0, sill: 0.0, width: 1.0, height: 2.1, arched: false };
    let window = |x: f32| Opening { x, sill: 0.95, width: 1.4, height: 1.2, arched: false };
    let front = wall_with_openings(w, h, &[door, window(5.4)]);
    let back = wall_with_openings(w, h, &[window(2.0), window(6.0)]);
    let side = wall_with_openings(d, h, &[window(3.0)]);
    let side_plain = wall_with_openings(d, h, &[]);
    let mut place = |name: &str, ornament: &super::ornament::Ornament, position: Vec3, yaw: f32| -> Result<(), String> {
        let detail = DetailSpec { cells_per_feature: 2.2, ..DetailSpec::new(ornament.smallest_feature.max(0.12)).with_budget(700_000) };
        let mut part = build_styled_part(name, PartKind::Architecture, &ornament.sdf, style, "office_wall", detail, Vec::new())?;
        part.mesh.rotate_y(yaw);
        part.mesh.translate(position);
        b.asset.parts.push(part);
        Ok(())
    };
    place("wall_front", &front, v(-w * 0.5, 0.0, d * 0.5), 0.0)?;
    place("wall_back", &back, v(w * 0.5, 0.0, -d * 0.5), 180.0)?;
    place("wall_left", &side, v(-w * 0.5 + 0.225, 0.0, d * 0.5), 90.0)?;
    place("wall_right", &side_plain, v(w * 0.5 - 0.225, 0.0, -d * 0.5), -90.0)?;
    let openings: [(&str, Opening, Vec3, f32); 4] = [
        ("front_window", window(5.4), v(-w * 0.5, 0.0, d * 0.5), 0.0),
        ("back_window_a", window(2.0), v(w * 0.5, 0.0, -d * 0.5), 180.0),
        ("back_window_b", window(6.0), v(w * 0.5, 0.0, -d * 0.5), 180.0),
        ("left_window", window(3.0), v(-w * 0.5 + 0.225, 0.0, d * 0.5), 90.0),
    ];
    for (name, opening, origin, yaw) in openings {
        for (kind, ornament, material) in [("frame", window_frame(&opening), "timber"), ("glass", window_glass(&opening), "glass")] {
            let detail = DetailSpec { cells_per_feature: 2.2, ..DetailSpec::new(ornament.smallest_feature).with_budget(200_000) };
            let mut part = build_styled_part(&format!("{}_{}", name, kind), PartKind::Architecture, &ornament.sdf, style, material, detail, Vec::new())?;
            part.mesh.translate(v(opening.x, opening.sill, 0.0));
            part.mesh.rotate_y(yaw);
            part.mesh.translate(origin);
            b.asset.parts.push(part);
        }
    }
    let door_leaf = Sdf::round_box(v(0.9, 2.05, 0.06), 0.015).translate(v(0.0, 1.03, 0.0));
    let detail = DetailSpec::new(0.06).with_budget(100_000);
    let mut leaf = build_styled_part("door", PartKind::Architecture, &door_leaf, style, "timber", detail, Vec::new())?;
    leaf.mesh.translate(v(-w * 0.5 + 2.0, 0.0, d * 0.5 - 0.05));
    b.asset.parts.push(leaf);
    let roof = Sdf::round_box(v(w + 0.1, 0.3, d + 0.1), 0.03).translate(v(0.0, h + 0.15, 0.0));
    let parapet = Sdf::round_box(v(w + 0.1, 0.6, d + 0.1), 0.03).subtract(Sdf::round_box(v(w - 0.3, 0.8, d - 0.3), 0.0)).translate(v(0.0, h + 0.5, 0.0));
    b.add_flat("roof", &roof.union(parapet), "concrete", 0.1)?;
    let rise = h + 0.3;
    let stair_spec = StairSpec { total_rise: rise, width: 1.0, run: 0.28 };
    let steps = stairs(&stair_spec)?;
    let detail = DetailSpec { cells_per_feature: 2.2, ..DetailSpec::new(steps.smallest_feature).with_budget(300_000) };
    let mut flight = build_styled_part("roof_stairs", PartKind::Object, &steps.sdf, style, "concrete", detail, Vec::new())?;
    flight.mesh.rotate_y(90.0);
    flight.mesh.translate(v(w * 0.5 + 0.5 + stair_spec.length(), 0.0, 0.0));
    b.asset.parts.push(flight);
    Ok(b.finish())
}

fn jersey_barrier(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("jersey_barrier", style);
    let profile = [[-0.30, 0.0], [0.30, 0.0], [0.24, 0.20], [0.12, 0.80], [-0.12, 0.80], [-0.24, 0.20]];
    let body = Sdf::extrude(&profile, 3.0, 0.03).rotate_y(90.0);
    b.add("body", &body, "concrete", 0.09)?;
    Ok(b.finish())
}

fn sandbag_wall(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("sandbag_wall", style);
    let (bag_length, bag_height, bag_depth) = (0.45, 0.17, 0.30);
    let rows = 6usize;
    let mut bags = Vec::new();
    for row in 0..rows {
        let offset = if row % 2 == 0 { 0.0 } else { bag_length * 0.5 };
        let count = 4usize;
        for column in 0..count {
            let jitter = ((row * 7 + column * 3) % 5) as f32 * 0.004;
            let x = -1.0 + bag_length * 0.5 + offset + column as f32 * (bag_length + 0.01) + jitter;
            bags.push(Sdf::round_box(v(bag_length, bag_height, bag_depth), 0.06).translate(v(x - 0.1, bag_height * 0.5 + row as f32 * (bag_height - 0.015), jitter * 4.0)));
        }
    }
    b.add("bags", &Sdf::smooth_union_all(bags, 0.015), "sandbag", 0.1)?;
    Ok(b.finish())
}

fn crate_prop(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("crate", style);
    let size = 1.2;
    let slat = |y: f32| Sdf::round_box(v(size, 0.12, size), 0.015).translate(v(0.0, y, 0.0));
    let mut parts = Vec::new();
    for i in 0..5 {
        parts.push(slat(0.1 + i as f32 * 0.25));
    }
    for (x, z) in [(-0.55, -0.55), (0.55, -0.55), (-0.55, 0.55), (0.55, 0.55)] {
        parts.push(Sdf::round_box(v(0.1, size, 0.1), 0.015).translate(v(x, size * 0.5, z)));
    }
    b.add("body", &Sdf::smooth_union_all(parts, 0.005), "pallet_wood", 0.1)?;
    Ok(b.finish())
}

fn pallet(level_y: f32) -> Sdf {
    let mut parts = Vec::new();
    for x in [-0.5, 0.0, 0.5] {
        parts.push(Sdf::round_box(v(0.1, 0.1, 1.0), 0.01).translate(v(x, level_y + 0.05, 0.0)));
    }
    for i in 0..5 {
        parts.push(Sdf::round_box(v(1.2, 0.022, 0.14), 0.005).translate(v(0.0, level_y + 0.111, -0.45 + i as f32 * 0.225)));
    }
    for i in 0..3 {
        parts.push(Sdf::round_box(v(1.2, 0.022, 0.14), 0.005).translate(v(0.0, level_y + 0.0, -0.4 + i as f32 * 0.4)));
    }
    Sdf::smooth_union_all(parts, 0.004)
}

fn pallet_stack(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("pallet_stack", style);
    let stack = Sdf::smooth_union_all(vec![pallet(0.0), pallet(0.14), pallet(0.28), pallet(0.42)], 0.004);
    b.add("pallets", &stack, "pallet_wood", 0.044)?;
    Ok(b.finish())
}

fn catwalk(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("catwalk", style);
    let (length, width) = (4.0, 1.2);
    let deck = Sdf::round_box(v(length, 0.06, width), 0.008).translate(v(0.0, 0.0, 0.0));
    let slot = Sdf::round_box(v(0.06, 0.2, 0.9), 0.0).linear_array_x((length / 0.1) as usize - 2, 0.1).translate(v(-length * 0.5 + 0.15, 0.0, 0.0));
    let grating = deck.subtract(slot);
    let mut frame = vec![
        Sdf::round_box(v(length, 0.14, 0.07), 0.01).translate(v(0.0, -0.1, width * 0.5 - 0.03)),
        Sdf::round_box(v(length, 0.14, 0.07), 0.01).translate(v(0.0, -0.1, -width * 0.5 + 0.03)),
    ];
    for x in [-length * 0.5 + 0.1, 0.0, length * 0.5 - 0.1] {
        frame.push(Sdf::round_box(v(0.06, 0.12, width), 0.01).translate(v(x, -0.1, 0.0)));
    }
    let mut railing = Vec::new();
    for z in [width * 0.5 - 0.03, -width * 0.5 + 0.03] {
        for i in 0..5 {
            let x = -length * 0.5 + i as f32 * length * 0.25;
            railing.push(bar(v(x, 0.0, z), v(x, 1.1, z), 0.025));
        }
        railing.push(bar(v(-length * 0.5, 1.1, z), v(length * 0.5, 1.1, z), 0.025));
        railing.push(bar(v(-length * 0.5, 0.55, z), v(length * 0.5, 0.55, z), 0.02));
        railing.push(Sdf::round_box(v(length, 0.12, 0.012), 0.004).translate(v(0.0, 0.09, z)));
    }
    b.add("grating", &grating, "steel", 0.04)?;
    b.add("frame", &Sdf::smooth_union_all(frame, 0.008), "steel", 0.06)?;
    b.add("railing", &Sdf::smooth_union_all(railing, 0.008), "steel", 0.1)?;
    Ok(b.finish())
}

fn catwalk_stairs(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("catwalk_stairs", style);
    let spec = StairSpec { total_rise: 3.0, width: 1.2, run: 0.28 };
    let steps = stairs(&spec)?;
    b.add("steps", &steps.sdf, "steel", steps.smallest_feature)?;
    let length = spec.length();
    let mut rails = Vec::new();
    for z in [0.55, -0.55] {
        rails.push(bar(v(0.0, 1.0, z), v(-length, 1.0 + spec.total_rise, z), 0.025));
        for i in 0..4 {
            let t = i as f32 / 3.0;
            let (x, y) = (-length * t, spec.total_rise * t);
            rails.push(bar(v(x, y, z), v(x, y + 1.0 + spec.total_rise / spec.step_count() as f32, z), 0.025));
        }
    }
    b.add("handrails", &Sdf::smooth_union_all(rails, 0.008), "steel", 0.05)?;
    Ok(b.finish())
}

fn water_tower(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("water_tower", style);
    let (leg_height, tank_radius) = (8.0, 2.0);
    let mut legs = Vec::new();
    let leg_positions = [(1.6, 1.6), (1.6, -1.6), (-1.6, -1.6), (-1.6, 1.6)];
    for (x, z) in leg_positions {
        legs.push(bar(v(x * 1.3, 0.0, z * 1.3), v(x, leg_height, z), 0.12));
    }
    for level in [2.5, 5.0] {
        let scale = 1.3 - 0.3 * level / leg_height;
        for i in 0..4 {
            let (a, c) = (leg_positions[i], leg_positions[(i + 1) % 4]);
            legs.push(bar(v(a.0 * scale, level, a.1 * scale), v(c.0 * scale, level, c.1 * scale), 0.05));
            let up = 1.3 - 0.3 * (level + 2.5) / leg_height;
            legs.push(bar(v(a.0 * scale, level, a.1 * scale), v(c.0 * up, level + 2.5, c.1 * up), 0.04));
        }
    }
    b.add("legs", &Sdf::smooth_union_all(legs, 0.02), "olive_metal", 0.1)?;
    let tank = Sdf::cylinder(tank_radius, 1.8, 0.1).translate(v(0.0, leg_height + 1.8, 0.0));
    let roof = Sdf::revolve(&[[0.0, 0.0], [tank_radius + 0.1, 0.0], [0.15, 1.2], [0.0, 1.2]]).translate(v(0.0, leg_height + 3.6, 0.0));
    let ring = Sdf::cylinder(tank_radius + 0.06, 0.06, 0.02).translate(v(0.0, leg_height + 2.4, 0.0));
    b.add_flat("tank", &Sdf::smooth_union_all(vec![tank, roof, ring], 0.02), "container_blue", 0.12)?;
    let mut ladder = vec![bar(v(2.3, 0.0, -0.25), v(2.3, leg_height + 3.0, -0.25), 0.025), bar(v(2.3, 0.0, 0.25), v(2.3, leg_height + 3.0, 0.25), 0.025)];
    for i in 0..30 {
        ladder.push(bar(v(2.3, 0.3 + i as f32 * 0.37, -0.25), v(2.3, 0.3 + i as f32 * 0.37, 0.25), 0.018));
    }
    b.add("ladder", &Sdf::smooth_union_all(ladder, 0.008), "rusted_steel", 0.1)?;
    Ok(b.finish())
}

fn wreck_truck(style: &Style) -> Result<Prop, String> {
    let mut b = Builder::new("wreck_truck", style);
    let cab = Sdf::round_box(v(1.7, 1.4, 2.0), 0.12).translate(v(1.6, 1.25, 0.0));
    let windscreen = Sdf::round_box(v(0.1, 0.7, 1.7), 0.02).translate(v(2.45, 1.55, 0.0));
    let cab = cab.smooth_subtract(windscreen, 0.02);
    let hood = Sdf::round_box(v(1.1, 0.7, 1.9), 0.1).translate(v(2.5, 0.95, 0.0));
    let bed = Sdf::round_box(v(2.8, 0.15, 2.1), 0.03).translate(v(-1.3, 0.78, 0.0));
    let side = |z: f32| Sdf::round_box(v(2.8, 0.7, 0.08), 0.02).translate(v(-1.3, 1.15, z));
    let tail = Sdf::round_box(v(0.08, 0.7, 2.1), 0.02).translate(v(-2.7, 1.15, 0.0)).rotate_z(8.0);
    let frame = Sdf::round_box(v(5.0, 0.2, 1.2), 0.04).translate(v(0.2, 0.6, 0.0));
    let wheel = |x: f32, z: f32| Sdf::cylinder(0.45, 0.14, 0.05).rotate_x(90.0).translate(v(x, 0.45, z));
    let mut body = vec![cab, hood, bed, side(1.0), side(-1.0), tail, frame];
    body.push(wheel(1.8, 1.0));
    body.push(wheel(-1.7, -1.0));
    body.push(wheel(-1.7, 1.0));
    b.add_flat("body", &Sdf::smooth_union_all(body, 0.03), "rusted_steel", 0.14)?;
    Ok(b.finish())
}

pub fn build_prop(name: &str, style: &Style) -> Result<Prop, String> {
    build_prop_with_detail(name, style, Detail::Game)
}

pub fn build_prop_with_detail(name: &str, style: &Style, detail: Detail) -> Result<Prop, String> {
    BAR_RADIUS_FLOOR.with(|floor| floor.set(detail.minimum_bar_radius()));
    let result = build_named(name, style);
    BAR_RADIUS_FLOOR.with(|floor| floor.set(0.0));
    result
}

fn build_named(name: &str, style: &Style) -> Result<Prop, String> {
    match name {
        "container_20_red" => container(style, 6.06, "container_red"),
        "container_20_blue" => container(style, 6.06, "container_blue"),
        "container_20_sand" => container(style, 6.06, "container_sand"),
        "container_40_red" => container(style, 12.2, "container_red"),
        "container_40_blue" => container(style, 12.2, "container_blue"),
        "container_40_sand" => container(style, 12.2, "container_sand"),
        "fuel_tank" => fuel_tank(style),
        "derrick" => derrick(style),
        "pipe_rack" => pipe_rack(style),
        "office" => office(style),
        "jersey_barrier" => jersey_barrier(style),
        "sandbag_wall" => sandbag_wall(style),
        "crate" => crate_prop(style),
        "pallet_stack" => pallet_stack(style),
        "catwalk" => catwalk(style),
        "catwalk_stairs" => catwalk_stairs(style),
        "water_tower" => water_tower(style),
        "wreck_truck" => wreck_truck(style),
        other => Err(format!("unknown prop '{}'; known: {}", other, prop_names().join(", "))),
    }
}

