use super::material::{Concrete, Fabric, Plaster, Surface, WoodGrain};
use super::furniture::{
    build_bed, build_chair, build_shelf, build_stool, build_table, BedSpec, ChairSpec, ShelfSpec, StoolSpec, TableSpec,
};
use super::mesh::Mesh;
use super::part::{grid_breaks, Asset, ClearanceZone, Dimension, DoorLink, Footprint, Part, PartKind, Room, View};
use crate::camera::Camera;
use crate::math::Vec3;

const PLASTER: Vec3 = Vec3::new(0.88, 0.85, 0.79);
const STUCCO: Vec3 = Vec3::new(0.76, 0.72, 0.65);
const REVEAL: Vec3 = Vec3::new(0.82, 0.80, 0.75);
const WALL_TOP: Vec3 = Vec3::new(0.70, 0.68, 0.64);
const TRIM_WHITE: Vec3 = Vec3::new(0.92, 0.91, 0.88);
const DOOR_WOOD: Vec3 = Vec3::new(0.36, 0.25, 0.17);
const LIVING_FLOOR: Vec3 = Vec3::new(0.52, 0.37, 0.24);
const BEDROOM_FLOOR: Vec3 = Vec3::new(0.60, 0.58, 0.55);
const GRASS: Vec3 = Vec3::new(0.34, 0.45, 0.27);
const ROOF_TOP: Vec3 = Vec3::new(0.32, 0.32, 0.34);
const METAL_HANDLE: Vec3 = Vec3::new(0.7, 0.68, 0.6);

const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const GRID_STEP: f32 = 0.25;
const BASEBOARD_HEIGHT: f32 = 0.10;
const BASEBOARD_DEPTH: f32 = 0.015;
const CASING_WIDTH: f32 = 0.07;
const CASING_DEPTH: f32 = 0.02;
const DOOR_OPEN_DEGREES: f32 = 20.0;

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

#[derive(Clone, Copy, PartialEq)]
enum OpeningKind {
    Door,
    Window,
}

struct Opening {
    kind: OpeningKind,
    from: f32,
    to: f32,
    bottom: f32,
    top: f32,
}

struct Wall {
    start: Vec3,
    end: Vec3,
    thickness: f32,
    openings: Vec<Opening>,
}

struct WallFrame {
    origin: Vec3,
    direction: Vec3,
    across: Vec3,
    extension: f32,
    length: f32,
    thickness: f32,
}

impl WallFrame {
    fn of(wall: &Wall) -> WallFrame {
        let span = wall.end - wall.start;
        let direction = span.normalized();
        let extension = wall.thickness * 0.5;
        WallFrame {
            origin: wall.start - direction * extension,
            direction,
            across: v(-direction.z, 0.0, direction.x),
            extension,
            length: span.length() + extension * 2.0,
            thickness: wall.thickness,
        }
    }

    fn point(&self, along: f32, across_offset: f32, height: f32) -> Vec3 {
        self.origin + self.direction * along + self.across * across_offset + UP * height
    }

    fn opening_range(&self, opening: &Opening) -> (f32, f32) {
        (self.extension + opening.from, self.extension + opening.to)
    }

    fn box_size(&self, along: f32, height: f32, across: f32) -> Vec3 {
        if self.direction.x.abs() > 0.5 {
            v(along, height, across)
        } else {
            v(across, height, along)
        }
    }
}

struct Footprint2 {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
}

impl Footprint2 {
    fn contains_strictly(&self, point: Vec3) -> bool {
        point.x > self.min_x + 0.01 && point.x < self.max_x - 0.01 && point.z > self.min_z + 0.01 && point.z < self.max_z - 0.01
    }
}

fn breaks_with(length: f32, extra: &[f32]) -> Vec<f32> {
    let mut breaks = grid_breaks(0.0, length, GRID_STEP);
    breaks.extend_from_slice(extra);
    breaks.sort_by(|a, b| a.total_cmp(b));
    breaks.dedup_by(|a, b| (*a - *b).abs() < 0.02);
    breaks
}

fn build_wall(wall: &Wall, height: f32, building: &Footprint2, mesh: &mut Mesh, trim: &mut Mesh, doors: &mut Mesh) {
    let frame = WallFrame::of(wall);
    let along_breaks: Vec<f32> = wall
        .openings
        .iter()
        .flat_map(|o| {
            let (a, b) = frame.opening_range(o);
            [a, b]
        })
        .collect();
    let height_breaks: Vec<f32> = wall.openings.iter().flat_map(|o| [o.bottom, o.top]).collect();
    let u_breaks = breaks_with(frame.length, &along_breaks);
    let v_breaks = breaks_with(height, &height_breaks);

    for side in [-1.0, 1.0] {
        let offset = side * frame.thickness * 0.5;
        let face_center = frame.point(frame.length * 0.5, offset, 1.0);
        let color = if building.contains_strictly(face_center) { PLASTER } else { STUCCO };
        let in_opening = |u: f32, height_value: f32| {
            wall.openings.iter().any(|o| {
                let (a, b) = frame.opening_range(o);
                u > a && u < b && height_value > o.bottom && height_value < o.top
            })
        };
        mesh.add_grid(
            frame.point(0.0, offset, 0.0),
            frame.direction,
            UP,
            &u_breaks,
            &v_breaks,
            frame.across * side,
            color,
            |u, h| !in_opening(u, h),
        );
        if building.contains_strictly(face_center) {
            add_baseboards(wall, &frame, side, trim);
        }
        for opening in &wall.openings {
            add_casing(&frame, opening, side, trim);
        }
    }

    let thickness_breaks = grid_breaks(0.0, frame.thickness, GRID_STEP);
    mesh.add_grid(
        frame.point(0.0, -frame.thickness * 0.5, height),
        frame.direction,
        frame.across,
        &u_breaks,
        &thickness_breaks,
        UP,
        WALL_TOP,
        |_, _| true,
    );

    for opening in &wall.openings {
        add_reveals(&frame, opening, mesh);
        match opening.kind {
            OpeningKind::Door => add_door_leaf(&frame, opening, doors),
            OpeningKind::Window => add_window_frame(&frame, opening, trim),
        }
    }
}

fn add_reveals(frame: &WallFrame, opening: &Opening, mesh: &mut Mesh) {
    let (a, b) = frame.opening_range(opening);
    let across_breaks = grid_breaks(0.0, frame.thickness, GRID_STEP);
    let height_breaks = grid_breaks(opening.bottom, opening.top, GRID_STEP);
    let along_breaks = grid_breaks(a, b, GRID_STEP);
    mesh.add_grid(frame.point(a, -frame.thickness * 0.5, 0.0), frame.across, UP, &across_breaks, &height_breaks, frame.direction, REVEAL, |_, _| true);
    mesh.add_grid(frame.point(b, -frame.thickness * 0.5, 0.0), frame.across, UP, &across_breaks, &height_breaks, -frame.direction, REVEAL, |_, _| true);
    mesh.add_grid(frame.point(0.0, -frame.thickness * 0.5, opening.top), frame.direction, frame.across, &along_breaks, &across_breaks, -UP, REVEAL, |_, _| true);
    if opening.bottom > 0.01 {
        mesh.add_grid(frame.point(0.0, -frame.thickness * 0.5, opening.bottom), frame.direction, frame.across, &along_breaks, &across_breaks, UP, REVEAL, |_, _| true);
    }
}

fn add_baseboards(wall: &Wall, frame: &WallFrame, side: f32, trim: &mut Mesh) {
    let mut gaps: Vec<(f32, f32)> = wall
        .openings
        .iter()
        .filter(|o| o.kind == OpeningKind::Door)
        .map(|o| frame.opening_range(o))
        .collect();
    gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut cursor = 0.0;
    let mut segments = Vec::new();
    for (start, end) in gaps {
        segments.push((cursor, start));
        cursor = end;
    }
    segments.push((cursor, frame.length));
    for (start, end) in segments {
        if end - start < 0.05 {
            continue;
        }
        let center = frame.point((start + end) * 0.5, side * (frame.thickness * 0.5 + BASEBOARD_DEPTH * 0.5), BASEBOARD_HEIGHT * 0.5);
        trim.add_box(center, frame.box_size(end - start, BASEBOARD_HEIGHT, BASEBOARD_DEPTH), TRIM_WHITE);
    }
}

fn add_casing(frame: &WallFrame, opening: &Opening, side: f32, trim: &mut Mesh) {
    let (a, b) = frame.opening_range(opening);
    let across_offset = side * (frame.thickness * 0.5 + CASING_DEPTH * 0.5);
    let height = opening.top - opening.bottom;
    for edge in [a - CASING_WIDTH * 0.5, b + CASING_WIDTH * 0.5] {
        let center = frame.point(edge, across_offset, opening.bottom + height * 0.5);
        trim.add_box(center, frame.box_size(CASING_WIDTH, height, CASING_DEPTH), TRIM_WHITE);
    }
    let head = frame.point((a + b) * 0.5, across_offset, opening.top + CASING_WIDTH * 0.5);
    trim.add_box(head, frame.box_size(b - a + CASING_WIDTH * 2.0, CASING_WIDTH, CASING_DEPTH), TRIM_WHITE);
    if opening.kind == OpeningKind::Window {
        let sill = frame.point((a + b) * 0.5, across_offset, opening.bottom - 0.02);
        trim.add_box(sill, frame.box_size(b - a + CASING_WIDTH * 2.0, 0.04, CASING_DEPTH + 0.03), TRIM_WHITE);
    }
}

fn add_window_frame(frame: &WallFrame, opening: &Opening, trim: &mut Mesh) {
    let (a, b) = frame.opening_range(opening);
    let bar = 0.05;
    let depth = 0.06;
    let height = opening.top - opening.bottom;
    let middle = (a + b) * 0.5;
    let center_height = opening.bottom + height * 0.5;
    let bars = [
        (frame.point(a + bar * 0.5, 0.0, center_height), frame.box_size(bar, height, depth)),
        (frame.point(b - bar * 0.5, 0.0, center_height), frame.box_size(bar, height, depth)),
        (frame.point(middle, 0.0, opening.bottom + bar * 0.5), frame.box_size(b - a, bar, depth)),
        (frame.point(middle, 0.0, opening.top - bar * 0.5), frame.box_size(b - a, bar, depth)),
        (frame.point(middle, 0.0, center_height), frame.box_size(0.03, height, depth * 0.7)),
        (frame.point(middle, 0.0, center_height), frame.box_size(b - a, 0.03, depth * 0.7)),
    ];
    for (center, size) in bars {
        trim.add_box(center, size, TRIM_WHITE);
    }
}

fn add_door_leaf(frame: &WallFrame, opening: &Opening, doors: &mut Mesh) {
    let (a, b) = frame.opening_range(opening);
    let width = b - a - 0.012;
    let height = opening.top - 0.012;
    let thickness = 0.04;
    let mut leaf = Mesh::new();
    leaf.add_box(v(width * 0.5, height * 0.5, 0.0), v(width, height, thickness), DOOR_WOOD);
    for side in [-1.0, 1.0] {
        leaf.add_box(v(width - 0.07, 1.0, side * (thickness * 0.5 + 0.02)), v(0.12, 0.022, 0.03), METAL_HANDLE);
    }
    leaf.rotate_y(DOOR_OPEN_DEGREES);
    leaf.rotate_y((-frame.direction.z).atan2(frame.direction.x).to_degrees());
    leaf.translate(frame.point(a + 0.006, 0.0, 0.0));
    doors.append(&leaf);
}

fn add_flat_slab(mesh: &mut Mesh, min: Vec3, max: Vec3, thickness: f32, underside: Vec3, top: Vec3, edge: Vec3) {
    let x_breaks = grid_breaks(0.0, max.x - min.x, GRID_STEP);
    let z_breaks = grid_breaks(0.0, max.z - min.z, GRID_STEP);
    let x_axis = v(1.0, 0.0, 0.0);
    let z_axis = v(0.0, 0.0, 1.0);
    mesh.add_grid(v(min.x, min.y, min.z), x_axis, z_axis, &x_breaks, &z_breaks, -UP, underside, |_, _| true);
    mesh.add_grid(v(min.x, min.y + thickness, min.z), x_axis, z_axis, &x_breaks, &z_breaks, UP, top, |_, _| true);
    let sides = [
        (v(min.x, min.y, min.z), x_axis, v(0.0, 0.0, -1.0)),
        (v(min.x, min.y, max.z), x_axis, v(0.0, 0.0, 1.0)),
        (v(min.x, min.y, min.z), z_axis, v(-1.0, 0.0, 0.0)),
        (v(max.x, min.y, min.z), z_axis, v(1.0, 0.0, 0.0)),
    ];
    for (origin, axis, facing) in sides {
        let length = if axis.x > 0.5 { max.x - min.x } else { max.z - min.z };
        mesh.add_grid(origin, axis, UP, &[0.0, length], &[0.0, thickness], facing, edge, |_, _| true);
    }
}

pub struct HouseSpec {
    pub ceiling_height: f32,
    pub furniture_cell: f32,
    pub furnished: bool,
}

impl HouseSpec {
    pub fn two_room() -> HouseSpec {
        HouseSpec { ceiling_height: 2.5, furniture_cell: 0.008, furnished: true }
    }
}

const HOUSE_WIDTH: f32 = 8.0;
const HOUSE_DEPTH: f32 = 5.0;
const PARTITION_X: f32 = 4.8;
const EXTERIOR_THICKNESS: f32 = 0.3;
const INTERIOR_THICKNESS: f32 = 0.12;
const DOOR_WIDTH: f32 = 0.9;
const BEDROOM_DOOR_START: f32 = 3.2;
const DOOR_HEIGHT: f32 = 2.05;
const WINDOW_SILL: f32 = 0.9;
const WINDOW_HEAD: f32 = 2.1;

fn door(from: f32) -> Opening {
    Opening { kind: OpeningKind::Door, from, to: from + DOOR_WIDTH, bottom: 0.0, top: DOOR_HEIGHT }
}

fn window(from: f32, to: f32) -> Opening {
    Opening { kind: OpeningKind::Window, from, to, bottom: WINDOW_SILL, top: WINDOW_HEAD }
}

fn area(subject: &str, min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> Footprint {
    Footprint { subject: subject.to_string(), min_x, max_x, min_z, max_z }
}

pub fn build_house(spec: &HouseSpec) -> Asset {
    let height = spec.ceiling_height;
    let building = Footprint2 { min_x: 0.0, max_x: HOUSE_WIDTH, min_z: 0.0, max_z: HOUSE_DEPTH };
    let walls = vec![
        Wall { start: v(0.0, 0.0, 0.0), end: v(HOUSE_WIDTH, 0.0, 0.0), thickness: EXTERIOR_THICKNESS, openings: vec![window(1.2, 2.8)] },
        Wall {
            start: v(HOUSE_WIDTH, 0.0, 0.0),
            end: v(HOUSE_WIDTH, 0.0, HOUSE_DEPTH),
            thickness: EXTERIOR_THICKNESS,
            openings: vec![window(1.6, 3.0)],
        },
        Wall {
            start: v(HOUSE_WIDTH, 0.0, HOUSE_DEPTH),
            end: v(0.0, 0.0, HOUSE_DEPTH),
            thickness: EXTERIOR_THICKNESS,
            openings: vec![door(5.6), window(3.8, 4.8)],
        },
        Wall {
            start: v(0.0, 0.0, HOUSE_DEPTH),
            end: v(0.0, 0.0, 0.0),
            thickness: EXTERIOR_THICKNESS,
            openings: vec![window(1.9, 3.4)],
        },
        Wall {
            start: v(PARTITION_X, 0.0, 0.0),
            end: v(PARTITION_X, 0.0, HOUSE_DEPTH),
            thickness: INTERIOR_THICKNESS,
            openings: vec![door(BEDROOM_DOOR_START)],
        },
    ];

    let mut shell = Mesh::new();
    let mut trim = Mesh::new();
    let mut doors = Mesh::new();
    for wall in &walls {
        build_wall(wall, height, &building, &mut shell, &mut trim, &mut doors);
    }

    let mut floor = Mesh::new();
    let floor_x_breaks = breaks_with(HOUSE_WIDTH, &[PARTITION_X]);
    let floor_z_breaks = grid_breaks(0.0, HOUSE_DEPTH, GRID_STEP);
    floor.add_grid(Vec3::ZERO, v(1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), &floor_x_breaks, &floor_z_breaks, UP, LIVING_FLOOR, |u, _| u < PARTITION_X);
    floor.add_grid(Vec3::ZERO, v(1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), &floor_x_breaks, &floor_z_breaks, UP, BEDROOM_FLOOR, |u, _| u > PARTITION_X);

    let overhang = EXTERIOR_THICKNESS * 0.5 + 0.2;
    let mut roof = Mesh::new();
    add_flat_slab(
        &mut roof,
        v(-overhang, height, -overhang),
        v(HOUSE_WIDTH + overhang, height, HOUSE_DEPTH + overhang),
        0.25,
        TRIM_WHITE,
        ROOF_TOP,
        ROOF_TOP,
    );

    shell.apply_material_map(&|sample, color| Plaster { base: color, variation: 0.06, seed: 11 }.evaluate(sample));
    trim.apply_material_map(&|sample, color| Plaster { base: color, variation: 0.02, seed: 12 }.evaluate(sample).with_roughness(0.45));
    doors.apply_material_map(&|sample, color| {
        let wood = WoodGrain { light: color * 1.2, dark: color * 0.7, grain_axis: UP, scale: 30.0, roughness: 0.5, seed: 13 };
        wood.evaluate(sample)
    });
    floor.apply_material_map(&|sample, color| {
        if color.x > color.z * 1.3 {
            WoodGrain { light: color * 1.15, dark: color * 0.75, grain_axis: Vec3::new(1.0, 0.0, 0.0), scale: 22.0, roughness: 0.4, seed: 14 }.evaluate(sample)
        } else {
            Fabric { base: color, weave_frequency: 500.0, seed: 15 }.evaluate(sample)
        }
    });
    roof.apply_material_map(&|sample, color| Concrete { base: color, stain: 0.2, seed: 16 }.evaluate(sample));

    let mut asset = Asset::new("house");
    asset.add_stage_floor(HOUSE_WIDTH * 0.5, HOUSE_DEPTH * 0.5, 14.0, 0.5, -0.02, GRASS);
    asset.parts.push(Part::from_mesh("walls", PartKind::Architecture, shell));
    asset.parts.push(Part::from_mesh("trim", PartKind::Architecture, trim));
    asset.parts.push(Part::from_mesh("doors", PartKind::Architecture, doors));
    asset.parts.push(Part::from_mesh("floor", PartKind::Architecture, floor));
    asset.parts.push(Part::from_mesh("roof", PartKind::Architecture, roof));
    asset.roof_part_names = vec!["roof".to_string()];

    let inner = EXTERIOR_THICKNESS * 0.5;
    let partition_inner = INTERIOR_THICKNESS * 0.5;
    asset.rooms = vec![
        Room { name: "living".into(), area: area("living", inner, PARTITION_X - partition_inner, inner, HOUSE_DEPTH - inner), ceiling_height: height },
        Room { name: "bedroom".into(), area: area("bedroom", PARTITION_X + partition_inner, HOUSE_WIDTH - inner, inner, HOUSE_DEPTH - inner), ceiling_height: height },
    ];
    asset.doors = vec![
        DoorLink {
            name: "entry door".into(),
            from: "outside".into(),
            to: "living".into(),
            swing_area: area("entry door", HOUSE_WIDTH - 5.6 - DOOR_WIDTH, HOUSE_WIDTH - 5.6, HOUSE_DEPTH - inner - DOOR_WIDTH, HOUSE_DEPTH - inner),
        },
        DoorLink {
            name: "bedroom door".into(),
            from: "bedroom".into(),
            to: "living".into(),
            swing_area: area("bedroom door", PARTITION_X + partition_inner, PARTITION_X + partition_inner + DOOR_WIDTH, BEDROOM_DOOR_START, BEDROOM_DOOR_START + DOOR_WIDTH),
        },
    ];
    asset.dimensions = vec![
        Dimension::new("house", "ceiling height", height, 2.40, 3.00),
        Dimension::new("house", "door width", DOOR_WIDTH, 0.80, 1.00),
        Dimension::new("house", "door height", DOOR_HEIGHT, 1.95, 2.30),
        Dimension::new("house", "window sill height", WINDOW_SILL, 0.60, 1.10),
        Dimension::new("house", "exterior wall thickness", EXTERIOR_THICKNESS, 0.20, 0.50),
        Dimension::new("house", "interior wall thickness", INTERIOR_THICKNESS, 0.08, 0.20),
        Dimension::new("living", "shortest side", asset.rooms[0].area.max_x - asset.rooms[0].area.min_x, 2.40, 12.0),
        Dimension::new("bedroom", "shortest side", asset.rooms[1].area.max_x - asset.rooms[1].area.min_x, 2.40, 12.0),
    ];
    if spec.furnished {
        furnish(&mut asset, spec.furniture_cell);
    }
    asset.views = house_views(height);
    asset
}

fn furnish(asset: &mut Asset, cell: f32) {
    let mut chair_spec = ChairSpec::dining();
    chair_spec.cell = cell;
    let chair = build_chair(&chair_spec);
    let mut table_spec = TableSpec::dining();
    table_spec.cell = cell * 1.5;
    let table = build_table(&table_spec);
    let mut shelf_spec = ShelfSpec::bookcase();
    shelf_spec.cell = cell;
    let shelf = build_shelf(&shelf_spec);
    let mut bed_spec = BedSpec::double();
    bed_spec.cell = cell * 2.5;
    let bed = build_bed(&bed_spec);
    let mut stool_spec = StoolSpec::bar_stool();
    stool_spec.cell = cell;
    let stool = build_stool(&stool_spec);

    let mut footprints = vec![asset.absorb(table, "dining_table", v(2.2, 0.0, 1.6), 0.0)];
    let chair_positions = [
        ("chair_north_west", 1.85, 0.87, 0.0),
        ("chair_north_east", 2.55, 0.87, 0.0),
        ("chair_south_west", 1.85, 2.33, 180.0),
        ("chair_south_east", 2.55, 2.33, 180.0),
    ];
    for (name, x, z, yaw) in chair_positions {
        footprints.push(asset.absorb(chair.clone(), name, v(x, 0.0, z), yaw));
    }
    footprints.push(asset.absorb(shelf.clone(), "living_bookcase", v(3.9, 0.0, 0.30), 0.0));
    footprints.push(asset.absorb(stool, "window_stool", v(0.6, 0.0, 2.65), 0.0));
    footprints.push(asset.absorb(bed, "bed", v(6.355, 0.0, 1.215), 0.0));
    footprints.push(asset.absorb(shelf, "bedroom_bookcase", v(6.3, 0.0, 4.7), 180.0));

    let bed_footprint = footprints.iter().find(|f| f.subject == "bed").unwrap().clone();
    let zones = vec![
        ("bed", "left side", bed_footprint.min_x - 0.6, bed_footprint.min_x, bed_footprint.min_z + 0.3, bed_footprint.max_z),
        ("bed", "right side", bed_footprint.max_x, bed_footprint.max_x + 0.6, bed_footprint.min_z + 0.3, bed_footprint.max_z),
        ("bed", "foot", bed_footprint.min_x, bed_footprint.max_x, bed_footprint.max_z, bed_footprint.max_z + 0.6),
        ("living_bookcase", "front", 3.45, 4.35, 0.45, 1.05),
        ("bedroom_bookcase", "front", 5.85, 6.75, 3.95, 4.55),
    ];
    for (owner, purpose, min_x, max_x, min_z, max_z) in zones {
        asset.clearance_zones.push(ClearanceZone {
            owner: owner.to_string(),
            purpose: purpose.to_string(),
            area: area(&format!("{} {}", owner, purpose), min_x, max_x, min_z, max_z),
        });
    }
    asset.footprints = footprints;
}

fn house_views(height: f32) -> Vec<View> {
    let eye = 1.55;
    vec![
        View { name: "dollhouse".into(), camera: Camera::look_at(v(4.0, 11.5, 11.5), v(4.0, 0.3, 2.4), 42.0) },
        View { name: "plan".into(), camera: Camera::look_at(v(4.0, 26.0, 2.52), v(4.0, 0.0, 2.5), 24.0) },
        View { name: "front".into(), camera: Camera::look_at(v(4.0, 4.0, 19.0), v(4.0, height * 0.45, 2.5), 36.0) },
        View { name: "living".into(), camera: Camera::look_at(v(0.95, eye, 4.55), v(3.0, 1.0, 0.9), 85.0) },
        View { name: "bedroom".into(), camera: Camera::look_at(v(7.5, eye, 4.6), v(5.9, 0.9, 1.2), 85.0) },
        View { name: "dining".into(), camera: Camera::look_at(v(3.9, 1.7, 3.6), v(2.2, 0.7, 1.4), 70.0) },
    ]
}
