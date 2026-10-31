use super::part::{Asset, Dimension, Member, Part, PartKind};
use super::sdf::Sdf;
use crate::math::Vec3;
use crate::random::Random;

pub const WOOD: Vec3 = Vec3::new(0.47, 0.30, 0.17);
pub const DARK_WOOD: Vec3 = Vec3::new(0.30, 0.19, 0.12);
pub const FABRIC_BLUE: Vec3 = Vec3::new(0.34, 0.41, 0.52);
pub const LINEN: Vec3 = Vec3::new(0.86, 0.84, 0.78);
pub const DUVET_GREY: Vec3 = Vec3::new(0.52, 0.56, 0.60);

const SMOOTH_JOINT: f32 = 0.012;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub struct ChairSpec {
    pub seat_height: f32,
    pub seat_width: f32,
    pub seat_depth: f32,
    pub seat_thickness: f32,
    pub seat_dish: f32,
    pub back_height: f32,
    pub back_rake_degrees: f32,
    pub slat_count: usize,
    pub leg_splay: f32,
    pub edge_radius: f32,
    pub stretchers: bool,
    pub raised_leg_height: f32,
    pub cell: f32,
}

impl ChairSpec {
    pub fn dining() -> ChairSpec {
        ChairSpec {
            seat_height: 0.46,
            seat_width: 0.44,
            seat_depth: 0.42,
            seat_thickness: 0.04,
            seat_dish: 0.008,
            back_height: 0.44,
            back_rake_degrees: 8.0,
            slat_count: 4,
            leg_splay: 0.015,
            edge_radius: 0.012,
            stretchers: true,
            raised_leg_height: 0.0,
            cell: 0.005,
        }
    }
}

pub fn build_chair(spec: &ChairSpec) -> Asset {
    let half_width = spec.seat_width * 0.5;
    let half_depth = spec.seat_depth * 0.5;
    let leg_inset = 0.034;
    let leg_x = half_width - leg_inset;
    let front_z = half_depth - leg_inset;
    let rear_z = -(half_depth - leg_inset);
    let leg_top = spec.seat_height - spec.seat_thickness;
    let rake = spec.back_rake_degrees.to_radians().tan();
    let post_z = |height: f32| rear_z - (height - spec.seat_height) * rake;

    let mut seat = Sdf::round_box(v(spec.seat_width, spec.seat_thickness, spec.seat_depth), spec.edge_radius)
        .translate(v(0.0, spec.seat_height - spec.seat_thickness * 0.5, 0.0));
    if spec.seat_dish > 0.0 {
        let chord = spec.seat_width * 0.4;
        let radius = chord * chord / (2.0 * spec.seat_dish);
        let cutter = Sdf::sphere(radius).translate(v(0.0, spec.seat_height + radius - spec.seat_dish, 0.0));
        seat = seat.smooth_subtract(cutter, 0.02);
    }

    let mut limbs = Vec::new();
    let mut members = Vec::new();
    let leg_names = [
        ("leg_front_left", -1.0, 1.0),
        ("leg_front_right", 1.0, 1.0),
        ("leg_rear_left", -1.0, -1.0),
        ("leg_rear_right", 1.0, -1.0),
    ];
    for (name, side, end) in leg_names {
        let top_z = if end > 0.0 { front_z } else { rear_z };
        let foot = v(side * (leg_x + spec.leg_splay * 0.6), 0.0, top_z + end * spec.leg_splay);
        let top = v(side * leg_x, leg_top + 0.01, top_z);
        let raised = if name == "leg_front_left" { spec.raised_leg_height } else { 0.0 };
        let foot = v(foot.x, raised, foot.z);
        let leg = Sdf::limb(foot, top, [0.014, 0.014], [0.021, 0.021], 0.004);
        members.push(Member::from_sdf(name, &leg, true));
        limbs.push(leg);
    }
    for (name, side) in [("back_post_left", -1.0), ("back_post_right", 1.0)] {
        let top_height = spec.seat_height + spec.back_height;
        let post = Sdf::limb(
            v(side * leg_x, leg_top, rear_z),
            v(side * leg_x, top_height, post_z(top_height)),
            [0.019, 0.019],
            [0.015, 0.015],
            0.005,
        );
        members.push(Member::from_sdf(name, &post, false));
        limbs.push(post);
    }

    let apron_center = leg_top - 0.035;
    let apron_half = [0.011, 0.035];
    limbs.push(Sdf::limb(v(-leg_x, apron_center, front_z), v(leg_x, apron_center, front_z), apron_half, apron_half, 0.004));
    limbs.push(Sdf::limb(v(-leg_x, apron_center, rear_z), v(leg_x, apron_center, rear_z), apron_half, apron_half, 0.004));
    for side in [-1.0, 1.0] {
        limbs.push(Sdf::limb(
            v(side * leg_x, apron_center, rear_z),
            v(side * leg_x, apron_center, front_z),
            apron_half,
            apron_half,
            0.004,
        ));
    }

    if spec.stretchers {
        let stretcher_height = 0.17;
        let blend = 1.0 - stretcher_height / leg_top;
        let x = leg_x + spec.leg_splay * 0.6 * blend;
        let front = front_z + spec.leg_splay * blend;
        let rear = rear_z - spec.leg_splay * blend;
        let half = [0.009, 0.012];
        for side in [-1.0, 1.0] {
            limbs.push(Sdf::limb(v(side * x, stretcher_height, rear), v(side * x, stretcher_height, front), half, half, 0.004));
        }
        limbs.push(Sdf::limb(v(-x, stretcher_height, 0.0), v(x, stretcher_height, 0.0), half, half, 0.004));
    }

    let top_rail_height = spec.seat_height + spec.back_height - 0.045;
    let lower_rail_height = spec.seat_height + 0.11;
    let top_rail = Sdf::limb(
        v(-leg_x, top_rail_height, post_z(top_rail_height)),
        v(leg_x, top_rail_height, post_z(top_rail_height)),
        [0.012, 0.045],
        [0.012, 0.045],
        0.005,
    );
    let lower_rail = Sdf::limb(
        v(-leg_x, lower_rail_height, post_z(lower_rail_height)),
        v(leg_x, lower_rail_height, post_z(lower_rail_height)),
        [0.011, 0.02],
        [0.011, 0.02],
        0.004,
    );
    members.push(Member::from_sdf("back_top_rail", &top_rail, false));
    limbs.push(top_rail);
    limbs.push(lower_rail);
    for index in 0..spec.slat_count {
        let fraction = (index + 1) as f32 / (spec.slat_count + 1) as f32;
        let x = -leg_x + 2.0 * leg_x * fraction;
        limbs.push(Sdf::limb(
            v(x, lower_rail_height, post_z(lower_rail_height)),
            v(x, top_rail_height, post_z(top_rail_height)),
            [0.007, 0.017],
            [0.007, 0.017],
            0.004,
        ));
    }

    let frame = Sdf::smooth_union_all(limbs, SMOOTH_JOINT);
    let seat_member = Member::from_sdf("seat", &seat, false);

    let mut asset = Asset::new("chair");
    asset.parts.push(Part::from_sdf("frame", PartKind::Object, &frame, spec.cell, WOOD, members));
    asset.parts.push(Part::from_sdf("seat", PartKind::Object, &seat, spec.cell, FABRIC_BLUE, vec![seat_member]));
    asset.dimensions = vec![
        Dimension::new("chair", "seat height", spec.seat_height, 0.40, 0.50),
        Dimension::new("chair", "seat width", spec.seat_width, 0.38, 0.52),
        Dimension::new("chair", "seat depth", spec.seat_depth, 0.38, 0.50),
        Dimension::new("chair", "back height above seat", spec.back_height, 0.30, 0.55),
        Dimension::new("chair", "seat thickness", spec.seat_thickness, 0.02, 0.07),
        Dimension::new("chair", "leg width at floor", 0.028, 0.02, 0.05),
    ];
    asset
}

pub struct StoolSpec {
    pub seat_height: f32,
    pub seat_radius: f32,
    pub leg_splay: f32,
    pub cell: f32,
}

impl StoolSpec {
    pub fn bar_stool() -> StoolSpec {
        StoolSpec { seat_height: 0.62, seat_radius: 0.19, leg_splay: 0.07, cell: 0.005 }
    }
}

pub fn build_stool(spec: &StoolSpec) -> Asset {
    let thickness = 0.045;
    let seat = Sdf::cylinder(spec.seat_radius, thickness, 0.016)
        .translate(v(0.0, spec.seat_height - thickness * 0.5, 0.0));
    let leg_top_radius = spec.seat_radius * 0.62;
    let leg_top = spec.seat_height - thickness;
    let mut limbs = Vec::new();
    let mut members = Vec::new();
    let mut ring_points = Vec::new();
    let ring_height = 0.26;
    for index in 0..4 {
        let angle = (index as f32 * 90.0 + 45.0).to_radians();
        let (sin, cos) = angle.sin_cos();
        let top = v(cos * leg_top_radius, leg_top + 0.01, sin * leg_top_radius);
        let foot = v(cos * (leg_top_radius + spec.leg_splay), 0.0, sin * (leg_top_radius + spec.leg_splay));
        let leg = Sdf::limb(foot, top, [0.014, 0.014], [0.02, 0.02], 0.005);
        members.push(Member::from_sdf(&format!("leg_{}", index), &leg, true));
        limbs.push(leg);
        let blend = 1.0 - ring_height / leg_top;
        let ring_radius = leg_top_radius + spec.leg_splay * blend;
        ring_points.push(v(cos * ring_radius, ring_height, sin * ring_radius));
    }
    for index in 0..4 {
        let (a, b) = (ring_points[index], ring_points[(index + 1) % 4]);
        limbs.push(Sdf::limb(a, b, [0.009, 0.009], [0.009, 0.009], 0.004));
    }
    let frame = Sdf::smooth_union_all(limbs, SMOOTH_JOINT);
    let mut asset = Asset::new("stool");
    asset.parts.push(Part::from_sdf("frame", PartKind::Object, &frame, spec.cell, DARK_WOOD, members));
    let seat_member = Member::from_sdf("seat", &seat, false);
    asset.parts.push(Part::from_sdf("seat", PartKind::Object, &seat, spec.cell, FABRIC_BLUE, vec![seat_member]));
    asset.dimensions = vec![
        Dimension::new("stool", "seat height", spec.seat_height, 0.45, 0.80),
        Dimension::new("stool", "seat diameter", spec.seat_radius * 2.0, 0.28, 0.45),
    ];
    asset
}

pub struct TableSpec {
    pub width: f32,
    pub depth: f32,
    pub height: f32,
    pub top_thickness: f32,
    pub cell: f32,
}

impl TableSpec {
    pub fn dining() -> TableSpec {
        TableSpec { width: 1.4, depth: 0.8, height: 0.74, top_thickness: 0.03, cell: 0.006 }
    }
}

pub fn build_table(spec: &TableSpec) -> Asset {
    let top = Sdf::round_box(v(spec.width, spec.top_thickness, spec.depth), 0.01)
        .translate(v(0.0, spec.height - spec.top_thickness * 0.5, 0.0));
    let leg_top = spec.height - spec.top_thickness;
    let inset = 0.05;
    let leg_x = spec.width * 0.5 - inset;
    let leg_z = spec.depth * 0.5 - inset;
    let mut limbs = Vec::new();
    let mut members = Vec::new();
    for (name, sx, sz) in [("leg_front_left", -1.0, 1.0), ("leg_front_right", 1.0, 1.0), ("leg_rear_left", -1.0, -1.0), ("leg_rear_right", 1.0, -1.0)] {
        let leg = Sdf::limb(v(sx * leg_x, 0.0, sz * leg_z), v(sx * leg_x, leg_top + 0.01, sz * leg_z), [0.022, 0.022], [0.03, 0.03], 0.005);
        members.push(Member::from_sdf(name, &leg, true));
        limbs.push(leg);
    }
    let apron_center = leg_top - 0.045;
    let apron_half = [0.012, 0.045];
    for sz in [-1.0, 1.0] {
        limbs.push(Sdf::limb(v(-leg_x, apron_center, sz * leg_z), v(leg_x, apron_center, sz * leg_z), apron_half, apron_half, 0.004));
    }
    for sx in [-1.0, 1.0] {
        limbs.push(Sdf::limb(v(sx * leg_x, apron_center, -leg_z), v(sx * leg_x, apron_center, leg_z), apron_half, apron_half, 0.004));
    }
    let frame = Sdf::smooth_union_all(limbs, SMOOTH_JOINT);
    let mut asset = Asset::new("table");
    asset.parts.push(Part::from_sdf("frame", PartKind::Object, &frame, spec.cell, WOOD, members));
    let top_member = Member::from_sdf("top", &top, false);
    asset.parts.push(Part::from_sdf("top", PartKind::Object, &top, spec.cell, DARK_WOOD, vec![top_member]));
    asset.dimensions = vec![
        Dimension::new("table", "height", spec.height, 0.72, 0.76),
        Dimension::new("table", "width", spec.width, 0.80, 2.60),
        Dimension::new("table", "depth", spec.depth, 0.70, 1.10),
        Dimension::new("table", "top thickness", spec.top_thickness, 0.02, 0.05),
    ];
    asset
}

pub struct BedSpec {
    pub width: f32,
    pub length: f32,
    pub mattress_top: f32,
    pub cell: f32,
}

impl BedSpec {
    pub fn double() -> BedSpec {
        BedSpec { width: 1.6, length: 2.0, mattress_top: 0.55, cell: 0.008 }
    }
}

pub fn build_bed(spec: &BedSpec) -> Asset {
    let mattress_height = 0.22;
    let base_top = spec.mattress_top - mattress_height;
    let frame_width = spec.width + 0.1;
    let frame_length = spec.length + 0.1;
    let base = Sdf::round_box(v(frame_width, base_top - 0.1, frame_length), 0.012)
        .translate(v(0.0, 0.1 + (base_top - 0.1) * 0.5, 0.0));
    let head_z = -frame_length * 0.5 + 0.03;
    let headboard = Sdf::round_box(v(frame_width, 0.95, 0.06), 0.015).translate(v(0.0, 0.45, head_z));
    let foot_board = Sdf::round_box(v(frame_width, 0.35, 0.05), 0.015).translate(v(0.0, 0.275, frame_length * 0.5 - 0.025));
    let mut shapes = vec![base, headboard, foot_board];
    let mut members = Vec::new();
    for (name, sx, sz) in [("foot_front_left", -1.0, 1.0), ("foot_front_right", 1.0, 1.0), ("foot_rear_left", -1.0, -1.0), ("foot_rear_right", 1.0, -1.0)] {
        let foot = Sdf::limb(
            v(sx * (frame_width * 0.5 - 0.05), 0.0, sz * (frame_length * 0.5 - 0.05)),
            v(sx * (frame_width * 0.5 - 0.05), 0.14, sz * (frame_length * 0.5 - 0.05)),
            [0.03, 0.03],
            [0.035, 0.035],
            0.008,
        );
        members.push(Member::from_sdf(name, &foot, true));
        shapes.push(foot);
    }
    let frame = Sdf::smooth_union_all(shapes, 0.01);
    members.push(Member::from_sdf("platform", &frame, false));

    let mattress = Sdf::round_box(v(spec.width, mattress_height, spec.length), 0.06)
        .translate(v(0.0, base_top + mattress_height * 0.5, 0.02));
    let duvet_length = spec.length * 0.68;
    let duvet = Sdf::round_box(v(spec.width + 0.08, 0.1, duvet_length), 0.045)
        .translate(v(0.0, spec.mattress_top - 0.035, spec.length * 0.5 - duvet_length * 0.5 - 0.02));
    let pillow_shape = |x: f32, yaw: f32| {
        Sdf::round_box(v(0.58, 0.14, 0.38), 0.06)
            .rotate_y(yaw)
            .translate(v(x, spec.mattress_top + 0.05, -spec.length * 0.5 + 0.32))
    };
    let pillows = pillow_shape(-spec.width * 0.25, 2.0).union(pillow_shape(spec.width * 0.25, -3.0));

    let mut asset = Asset::new("bed");
    asset.parts.push(Part::from_sdf("frame", PartKind::Object, &frame, spec.cell, DARK_WOOD, members));
    let mattress_member = Member::from_sdf("mattress", &mattress, false);
    asset.parts.push(Part::from_sdf("mattress", PartKind::Object, &mattress, spec.cell, LINEN, vec![mattress_member]));
    let duvet_member = Member::from_sdf("duvet", &duvet, false);
    asset.parts.push(Part::from_sdf("duvet", PartKind::Object, &duvet, spec.cell, DUVET_GREY, vec![duvet_member]));
    let pillow_member = Member::from_sdf("pillows", &pillows, false);
    asset.parts.push(Part::from_sdf("pillows", PartKind::Object, &pillows, spec.cell, LINEN, vec![pillow_member]));
    asset.dimensions = vec![
        Dimension::new("bed", "mattress top height", spec.mattress_top, 0.45, 0.65),
        Dimension::new("bed", "mattress width", spec.width, 0.90, 2.00),
        Dimension::new("bed", "mattress length", spec.length, 1.90, 2.20),
    ];
    asset
}

pub struct ShelfSpec {
    pub width: f32,
    pub depth: f32,
    pub height: f32,
    pub shelf_count: usize,
    pub books: bool,
    pub cell: f32,
}

impl ShelfSpec {
    pub fn bookcase() -> ShelfSpec {
        ShelfSpec { width: 0.9, depth: 0.3, height: 1.8, shelf_count: 5, books: true, cell: 0.006 }
    }
}

pub fn build_shelf(spec: &ShelfSpec) -> Asset {
    let panel = 0.025;
    let plinth_height = 0.08;
    let inner_width = spec.width - panel * 2.0;
    let mut shapes = Vec::new();
    for side in [-1.0, 1.0] {
        shapes.push(
            Sdf::round_box(v(panel, spec.height, spec.depth), 0.005).translate(v(side * (spec.width * 0.5 - panel * 0.5), spec.height * 0.5, 0.0)),
        );
    }
    shapes.push(Sdf::round_box(v(spec.width, panel, spec.depth), 0.005).translate(v(0.0, spec.height - panel * 0.5, 0.0)));
    shapes.push(Sdf::round_box(v(inner_width, plinth_height, spec.depth - 0.03), 0.004).translate(v(0.0, plinth_height * 0.5, 0.0)));
    shapes.push(Sdf::round_box(v(inner_width, spec.height - 0.02, 0.02), 0.003).translate(v(0.0, spec.height * 0.5, -spec.depth * 0.5 + 0.01)));
    let usable = spec.height - plinth_height - panel;
    let spacing = usable / spec.shelf_count as f32;
    let mut shelf_tops = Vec::new();
    for index in 0..spec.shelf_count {
        let shelf_y = plinth_height + spacing * index as f32;
        if index > 0 {
            shapes.push(Sdf::round_box(v(inner_width, panel, spec.depth - 0.02), 0.004).translate(v(0.0, shelf_y, 0.01)));
        }
        shelf_tops.push(if index == 0 { plinth_height } else { shelf_y + panel * 0.5 });
    }
    let carcass = Sdf::smooth_union_all(shapes, 0.004);
    let members = vec![
        Member::from_sdf("carcass", &carcass, true),
    ];
    let mut asset = Asset::new("shelf");
    asset.parts.push(Part::from_sdf("carcass", PartKind::Object, &carcass, spec.cell, WOOD, members));

    if spec.books {
        let mut random = Random::new(7);
        let palette = [
            v(0.55, 0.16, 0.14),
            v(0.16, 0.30, 0.45),
            v(0.20, 0.38, 0.24),
            v(0.72, 0.62, 0.30),
            v(0.30, 0.22, 0.38),
            v(0.78, 0.74, 0.66),
        ];
        let mut books = super::mesh::Mesh::new();
        for (index, top) in shelf_tops.iter().enumerate().take(spec.shelf_count.saturating_sub(1)) {
            if index % 2 == 1 {
                continue;
            }
            let mut x = -inner_width * 0.5 + 0.02;
            let limit = inner_width * 0.5 - 0.1 - random.range(0.0, 0.25);
            while x < limit {
                let thickness = random.range(0.022, 0.05);
                let height = random.range(0.19, 0.30).min(spacing - 0.06);
                let depth = random.range(0.18, 0.23);
                let color = palette[(random.next_u32() as usize) % palette.len()];
                let book = Sdf::round_box(v(thickness, height, depth), 0.003).translate(v(x + thickness * 0.5, top + height * 0.5, 0.0));
                books.append(&super::mesher::mesh_sdf(&book, spec.cell, color));
                x += thickness + 0.002;
            }
        }
        asset.parts.push(Part::from_mesh("books", PartKind::Object, books));
    }
    asset.dimensions = vec![
        Dimension::new("shelf", "height", spec.height, 1.20, 2.30),
        Dimension::new("shelf", "depth", spec.depth, 0.25, 0.50),
        Dimension::new("shelf", "width", spec.width, 0.60, 1.20),
        Dimension::new("shelf", "shelf spacing", spacing, 0.25, 0.45),
    ];
    asset
}
