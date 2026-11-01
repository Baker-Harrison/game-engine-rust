use crate::props::*;
use engine::layout::footprint::{Footprint, Rect};
use engine::layout::{Item, ItemKind, Layout, Purpose};
use engine::math::Vec3;
use engine::model::bake::{bake, BakeSettings};
use engine::model::bvh::Bvh;
use engine::model::industrial::build_prop;
use engine::model::lod::simplify;
use engine::model::mesh::Mesh;
use engine::model::style::Style;
use engine::noise::fractal_noise;
use engine::physics::{Aabb, CollisionWorld};

pub const ARENA_HALF_X: f32 = 44.0;
pub const ARENA_HALF_Z: f32 = 34.0;
pub const DECK_HEIGHT: f32 = 4.2;
const STEP_RISE: f32 = 0.2;
const STEP_RUN: f32 = 0.3;
const RAIL_HEIGHT: f32 = 1.05;

#[derive(Clone, Copy)]
pub struct Spawn {
    pub position: Vec3,
    pub yaw: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PickupKind {
    Health,
    Ammo,
}

#[derive(Clone, Copy)]
pub struct PickupSpot {
    pub kind: PickupKind,
    pub position: Vec3,
}

#[derive(Clone, Copy)]
pub struct MapRect {
    pub bounds: Aabb,
    pub elevated: bool,
}

pub struct Level {
    pub collision: CollisionWorld,
    pub mesh: Mesh,
    pub spawns: Vec<Spawn>,
    pub pickups: Vec<PickupSpot>,
    pub map_rects: Vec<MapRect>,
    pub layout: Layout,
}

#[derive(Clone, Copy)]
enum Heading {
    PlusX,
    MinusX,
    MinusZ,
}

struct Builder {
    collision: CollisionWorld,
    mesh: Mesh,
    spawns: Vec<Spawn>,
    pickups: Vec<PickupSpot>,
    map_rects: Vec<MapRect>,
    flipped: bool,
    prop_cache: PropCache,
    layout: Layout,
}

struct PropCache {
    barrels: Vec<Mesh>,
    tank_large: Mesh,
    tank_small: Mesh,
    containers: Vec<Mesh>,
    pipe_run: Mesh,
    elbow: Mesh,
    derrick: Mesh,
    pump_jack: Mesh,
    lamp: Mesh,
    jersey: Mesh,
    crates: Vec<Mesh>,
    sandbags: Mesh,
}

impl PropCache {
    fn new() -> PropCache {
        let mut built = game_props(vec![
            ("container_40_red", 0.12),
            ("container_40_blue", 0.12),
            ("container_40_sand", 0.12),
            ("derrick", 0.12),
            ("jersey_barrier", 0.08),
            ("sandbag_wall", 0.08),
            ("crate", 0.08),
        ])
        .into_iter();
        let containers: Vec<Mesh> = built.by_ref().take(3).collect();
        let derrick = built.next().expect("derrick");
        let jersey = built.next().expect("jersey");
        let sandbags = built.next().expect("sandbags");
        let crate_mesh = built.next().expect("crate");
        PropCache {
            barrels: vec![barrel(BARREL_RED, 1), barrel(BARREL_BLUE, 2), barrel(Vec3::new(0.78, 0.66, 0.2), 3)],
            tank_large: tank(5.0, 7.0, 1),
            tank_small: tank(3.2, 5.0, 2),
            containers,
            pipe_run: pipe_run(10.0, 0.4, STEEL),
            elbow: pipe_elbow(0.35, 2.0, STEEL),
            derrick,
            pump_jack: pump_jack(),
            lamp: lamp_post(6.0),
            jersey,
            crates: vec![crate_mesh],
            sandbags,
        }
    }
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn game_prop(name: &'static str, cell: f32) -> Mesh {
    let prop = build_prop(name, &Style::desert_industrial()).expect("industrial prop builds");
    simplify(&prop.asset.merged_mesh(&[]), cell)
}

fn game_props(specs: Vec<(&'static str, f32)>) -> Vec<Mesh> {
    let handles: Vec<_> = specs.into_iter().map(|(name, cell)| std::thread::spawn(move || game_prop(name, cell))).collect();
    handles.into_iter().map(|h| h.join().expect("prop thread")).collect()
}

impl Builder {
    fn new() -> Builder {
        Builder {
            collision: CollisionWorld::new(),
            mesh: Mesh::new(),
            spawns: Vec::new(),
            pickups: Vec::new(),
            map_rects: Vec::new(),
            flipped: false,
            prop_cache: PropCache::new(),
            layout: Layout::new("oilyard", Rect::new(-ARENA_HALF_X - 1.5, -ARENA_HALF_Z - 1.5, ARENA_HALF_X + 1.5, ARENA_HALF_Z + 1.5)),
        }
    }

    fn point(&self, p: Vec3) -> Vec3 {
        if self.flipped {
            v(-p.x, p.y, -p.z)
        } else {
            p
        }
    }

    fn register(&mut self, name: &str, kind: ItemKind, reason: &str, x: f32, z: f32, size_x: f32, size_z: f32, y0: f32, y1: f32) -> String {
        let p = self.point(v(x, 0.0, z));
        let name = if self.flipped { format!("{}_m", name) } else { name.to_string() };
        let footprint = Footprint::new(p.x, p.z, size_x, size_z, 0.0, y0, y1);
        let mut item = Item::new(&name, kind, reason, "arena", footprint);
        if kind == ItemKind::Stairs {
            let touching: Vec<String> = self.layout.items.iter().filter(|other| other.footprint.separation(&footprint) <= 0.05).map(|other| other.name.clone()).collect();
            for other in touching {
                item = item.attached(&other);
            }
        }
        self.layout.place(item);
        name
    }

    fn bounds(&self, min: Vec3, max: Vec3) -> Aabb {
        let (a, b) = (self.point(min), self.point(max));
        Aabb::new(a.min(b), a.max(b))
    }

    fn solid(&mut self, min: Vec3, max: Vec3, color: Vec3) {
        let aabb = self.bounds(min, max);
        add_tessellated_box(&mut self.mesh, aabb.center(), aabb.size(), color, 1.0);
        self.collision.add(aabb);
        self.map_rects.push(MapRect { bounds: aabb, elevated: aabb.min.y > 1.5 });
    }

    fn collider_only(&mut self, min: Vec3, max: Vec3) {
        let aabb = self.bounds(min, max);
        self.collision.add(aabb);
        self.map_rects.push(MapRect { bounds: aabb, elevated: aabb.min.y > 1.5 });
    }

    fn visual_only(&mut self, min: Vec3, max: Vec3, color: Vec3) {
        let aabb = self.bounds(min, max);
        self.mesh.add_box(aabb.center(), aabb.size(), color);
    }

    fn place(&mut self, source: &Mesh, position: Vec3, yaw: f32) {
        let mut mesh = source.clone();
        mesh.rotate_y(yaw + if self.flipped { 180.0 } else { 0.0 });
        mesh.translate(self.point(position));
        self.mesh.append(&mesh);
    }

    fn stairs(&mut self, start: Vec3, heading: Heading, width: f32, rise_total: f32, color: Vec3) {
        let steps = (rise_total / STEP_RISE).round() as usize;
        let half = width * 0.5;
        let length = steps as f32 * STEP_RUN;
        let (cx, cz, sx, sz) = match heading {
            Heading::PlusX => (start.x + length * 0.5, start.z, length, width),
            Heading::MinusX => (start.x - length * 0.5, start.z, length, width),
            Heading::MinusZ => (start.x, start.z - length * 0.5, width, length),
        };
        self.register(&format!("stairs_{:.0}_{:.0}", start.x, start.z), ItemKind::Stairs, "stairs giving access to the elevated route", cx, cz, sx, sz, 0.0, start.y + rise_total);
        for i in 0..steps {
            let (d0, d1) = (i as f32 * STEP_RUN, (i + 1) as f32 * STEP_RUN);
            let top = start.y + STEP_RISE * (i + 1) as f32;
            let (min, max) = match heading {
                Heading::PlusX => (v(start.x + d0, 0.0, start.z - half), v(start.x + d1, top, start.z + half)),
                Heading::MinusX => (v(start.x - d1, 0.0, start.z - half), v(start.x - d0, top, start.z + half)),
                Heading::MinusZ => (v(start.x - half, 0.0, start.z - d1), v(start.x + half, top, start.z - d0)),
            };
            let shade = if i % 2 == 0 { 1.0 } else { 0.9 };
            self.solid(min, max, color * shade);
        }
    }

    fn rail(&mut self, a: Vec3, b: Vec3) {
        let (min, max) = (a.min(b), a.max(b));
        let thick = 0.07;
        let (lo, hi) = (v(min.x - thick, min.y, min.z - thick), v(max.x + thick, min.y + RAIL_HEIGHT, max.z + thick));
        self.solid(lo, hi, SAFETY_YELLOW);
    }

    fn deck(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        self.register(&format!("deck_{:.0}_{:.0}_{:.0}", center.x, center.z, min.y), ItemKind::Deck, "elevated walkway giving a vantage over the lanes", center.x, center.z, max.x - min.x, max.z - min.z, min.y - 0.25, min.y);
        self.solid(v(min.x, min.y - 0.25, min.z), v(max.x, min.y, max.z), STEEL);
        self.visual_only(v(min.x, min.y - 0.6, min.z), v(max.x, min.y - 0.25, min.z + 0.12), STEEL_DARK);
        self.visual_only(v(min.x, min.y - 0.6, max.z - 0.12), v(max.x, min.y - 0.25, max.z), STEEL_DARK);
    }

    fn post(&mut self, x: f32, z: f32, top: f32, thickness: f32) {
        self.register(&format!("post_{:.1}_{:.1}_{:.0}", x, z, top), ItemKind::Support, "column carrying the deck above", x, z, thickness, thickness, 0.0, top);
        self.solid(v(x - thickness * 0.5, 0.0, z - thickness * 0.5), v(x + thickness * 0.5, top, z + thickness * 0.5), STEEL_DARK);
    }

    fn container_at(&mut self, x: f32, z: f32, along_x: bool, stack: usize, color: usize) {
        let (length, width, height) = (12.19, 2.44, 2.59);
        let (sx, sz) = if along_x { (length, width) } else { (width, length) };
        self.register(&format!("container_{:.0}_{:.0}", x, z), ItemKind::Solid, "shipping container forming a lane wall", x, z, sx, sz, 0.0, stack as f32 * height);
        for level in 0..stack {
            let base = level as f32 * height;
            let min = v(x - sx * 0.5, base, z - sz * 0.5);
            let max = v(x + sx * 0.5, base + height, z + sz * 0.5);
            self.collider_only(min, max);
            let mesh = self.prop_cache.containers[(color + level) % 3].clone();
            self.place(&mesh, v(x, base, z), if along_x { 0.0 } else { 90.0 });
        }
    }

    fn barrels(&mut self, x: f32, z: f32, count: usize, seed: usize) {
        self.register(&format!("barrels_{:.0}_{:.0}", x, z), ItemKind::Cover, "barrel cluster as low cover", x, z, 2.1, 2.1, 0.0, 0.88);
        for i in 0..count {
            let angle = i as f32 * 2.1 + seed as f32;
            let radius = if i == 0 { 0.0 } else { 0.7 };
            let (bx, bz) = (x + angle.cos() * radius, z + angle.sin() * radius);
            self.collider_only(v(bx - 0.29, 0.0, bz - 0.29), v(bx + 0.29, 0.88, bz + 0.29));
            let mesh = self.prop_cache.barrels[(seed + i) % 3].clone();
            self.place(&mesh, v(bx, 0.0, bz), i as f32 * 37.0);
        }
    }

    fn crates(&mut self, x: f32, z: f32, stack: usize, seed: usize) {
        self.register(&format!("crates_{:.0}_{:.0}", x, z), ItemKind::Cover, "stacked crates as cover", x, z, 1.2, 1.2, 0.0, stack as f32 * 1.2);
        for level in 0..stack {
            let y = level as f32 * 1.2;
            self.collider_only(v(x - 0.6, y, z - 0.6), v(x + 0.6, y + 1.2, z + 0.6));
            let mesh = self.prop_cache.crates[0].clone();
            self.place(&mesh, v(x, y, z), (seed * 23 + level * 11) as f32);
        }
    }

    fn tank_at(&mut self, x: f32, z: f32, radius: f32, height: f32, large: bool) {
        let r = radius;
        self.register(&format!("tank_{:.0}_{:.0}", x, z), ItemKind::Solid, "fuel tank landmark", x, z, r * 2.0, r * 2.0, 0.0, height);
        self.collider_only(v(x - r, 0.0, z - r * 0.45), v(x + r, height, z + r * 0.45));
        self.collider_only(v(x - r * 0.45, 0.0, z - r), v(x + r * 0.45, height, z + r));
        self.collider_only(v(x - r * 0.8, 0.0, z - r * 0.8), v(x + r * 0.8, height, z + r * 0.8));
        let mesh = if large { self.prop_cache.tank_large.clone() } else { self.prop_cache.tank_small.clone() };
        self.place(&mesh, v(x, 0.0, z), 0.0);
    }

    fn pipe_rack(&mut self, x0: f32, x1: f32, z: f32) {
        let length = x1 - x0;
        self.register(&format!("pipe_rack_{:.0}_{:.0}", x0, z), ItemKind::Cover, "pipe rack forming a low lane divider", (x0 + x1) * 0.5, z, length, 1.7, 0.0, 0.85);
        let segments = (length / 10.0).ceil() as usize;
        let step = length / segments as f32;
        for i in 0..segments {
            let cx = x0 + step * (i as f32 + 0.5);
            let mesh = self.prop_cache.pipe_run.clone();
            let mut scaled = mesh;
            for p in scaled.positions.iter_mut() {
                p.x *= step / 10.0;
            }
            for lane in [-0.45f32, 0.45] {
                self.place(&scaled, v(cx, 0.3 + (lane + 0.45) * 0.0, z + lane), 0.0);
            }
        }
        self.collider_only(v(x0, 0.0, z - 0.85), v(x1, 0.85, z + 0.85));
        let posts = (length / 5.0) as usize + 1;
        for i in 0..posts {
            let px = x0 + 0.3 + i as f32 * (length - 0.6) / (posts - 1).max(1) as f32;
            self.visual_only(v(px - 0.15, 0.0, z - 0.9), v(px + 0.15, 0.25, z + 0.9), STEEL_DARK);
        }
    }

    fn building(&mut self, center: Vec3, size_x: f32, size_z: f32, height: f32, doors: &[(char, f32)], windows: &[(char, f32)], color: Vec3) {
        let thick = 0.3;
        let base = center.y;
        self.register(&format!("building_{:.0}_{:.0}", center.x, center.z), ItemKind::Solid, "office building giving interior fighting space", center.x, center.z, size_x + 0.4, size_z + 0.4, base, base + height + 0.3);
        let (x0, x1) = (center.x - size_x * 0.5, center.x + size_x * 0.5);
        let (z0, z1) = (center.z - size_z * 0.5, center.z + size_z * 0.5);
        self.solid(v(x0, base, z0), v(x1, base + 0.12, z1), CONCRETE * 0.85);
        let walls = [('n', x0, z0, x1, z0 + thick), ('s', x0, z1 - thick, x1, z1), ('w', x0, z0, x0 + thick, z1), ('e', x1 - thick, z0, x1, z1)];
        for (side, ax, az, bx, bz) in walls {
            let along_x = side == 'n' || side == 's';
            let (wall_min, wall_len) = if along_x { (ax, bx - ax) } else { (az, bz - az) };
            let mut openings: Vec<(f32, f32, f32, f32)> = Vec::new();
            for (s, offset) in doors {
                if *s == side {
                    openings.push((offset - 0.9, offset + 0.9, 0.0, 2.2));
                }
            }
            for (s, offset) in windows {
                if *s == side {
                    openings.push((offset - 0.9, offset + 0.9, 1.0, 2.1));
                }
            }
            openings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let mut cursor = wall_min;
            let mut pieces: Vec<(f32, f32, f32, f32)> = Vec::new();
            for (lo, hi, sill, head) in &openings {
                if *lo > cursor {
                    pieces.push((cursor, *lo, 0.0, height));
                }
                if *sill > 0.0 {
                    pieces.push((*lo, *hi, 0.0, *sill));
                }
                pieces.push((*lo, *hi, *head, height));
                cursor = *hi;
            }
            if cursor < wall_min + wall_len {
                pieces.push((cursor, wall_min + wall_len, 0.0, height));
            }
            for (from, to, bottom, top) in pieces {
                let (low, high) = (base + bottom.max(0.12), base + top);
                let (min, max) = if along_x { (v(from, low, az), v(to, high, bz)) } else { (v(ax, low, from), v(bx, high, to)) };
                self.solid(min, max, color);
            }
        }
        self.solid(v(x0 - 0.2, base + height, z0 - 0.2), v(x1 + 0.2, base + height + 0.3, z1 + 0.2), CONCRETE * 0.7);
    }

    fn office_furniture(&mut self, center: Vec3) {
        self.solid(center + v(-2.5, 0.12, -1.0), center + v(-0.9, 0.85, 0.2), WOOD);
        self.solid(center + v(1.2, 0.12, 1.4), center + v(3.0, 0.95, 2.2), STEEL);
        self.solid(center + v(1.6, 0.12, -2.4), center + v(2.2, 1.8, -1.4), STEEL_DARK);
    }

    fn catwalk_x(&mut self, from_x: f32, to_x: f32, z: f32, y: f32, rails: (bool, bool)) {
        let (lo, hi) = (from_x.min(to_x), from_x.max(to_x));
        self.deck(v(lo, y, z - 1.0), v(hi, y, z + 1.0));
        if rails.0 {
            self.rail(v(lo, y, z - 0.95), v(hi, y, z - 0.95));
        }
        if rails.1 {
            self.rail(v(lo, y, z + 0.95), v(hi, y, z + 0.95));
        }
        let count = ((hi - lo - 2.0) / 6.0).ceil() as usize + 1;
        for i in 0..count {
            let x = lo + 1.0 + (hi - lo - 2.0) * i as f32 / (count - 1) as f32;
            self.post(x, z, y - 0.25, 0.3);
        }
    }

    fn catwalk_z(&mut self, x: f32, from_z: f32, to_z: f32, y: f32, rails: (bool, bool)) {
        let (lo, hi) = (from_z.min(to_z), from_z.max(to_z));
        self.deck(v(x - 1.0, y, lo), v(x + 1.0, y, hi));
        if rails.0 {
            self.rail(v(x - 0.95, y, lo), v(x - 0.95, y, hi));
        }
        if rails.1 {
            self.rail(v(x + 0.95, y, lo), v(x + 0.95, y, hi));
        }
        let count = ((hi - lo - 2.0) / 6.0).ceil() as usize + 1;
        for i in 0..count {
            let z = lo + 1.0 + (hi - lo - 2.0) * i as f32 / (count - 1) as f32;
            self.post(x, z, y - 0.25, 0.3);
        }
    }
}

fn build_perimeter(b: &mut Builder) {
    let (hx, hz) = (ARENA_HALF_X, ARENA_HALF_Z);
    let wall_height = 5.0;
    let thick = 1.0;
    b.solid(v(-hx - thick, 0.0, -hz - thick), v(hx + thick, wall_height, -hz), CONCRETE * 0.9);
    b.solid(v(-hx - thick, 0.0, hz), v(hx + thick, wall_height, hz + thick), CONCRETE * 0.9);
    b.solid(v(-hx - thick, 0.0, -hz), v(-hx, wall_height, hz), CONCRETE * 0.9);
    b.solid(v(hx, 0.0, -hz), v(hx + thick, wall_height, hz), CONCRETE * 0.9);
    let mut x = -hx + 2.0;
    while x < hx {
        for z in [-hz + 0.02, hz - 0.02] {
            b.visual_only(v(x - 0.12, 0.0, z - 0.25), v(x + 0.12, wall_height + 0.4, z + 0.25), STEEL_DARK);
        }
        x += 4.0;
    }
    let mut z = -hz + 2.0;
    while z < hz {
        for x in [-hx + 0.02, hx - 0.02] {
            b.visual_only(v(x - 0.25, 0.0, z - 0.12), v(x + 0.25, wall_height + 0.4, z + 0.12), STEEL_DARK);
        }
        z += 4.0;
    }
    b.collider_only(v(-hx - 2.0, -2.0, -hz - 2.0), v(hx + 2.0, 0.0, hz + 2.0));
    b.collider_only(v(-hx - 40.0, -3.0, -hz - 40.0), v(hx + 40.0, 0.0, hz + 40.0));
}

fn build_center_tower(b: &mut Builder) {
    let y = DECK_HEIGHT;
    b.deck(v(-4.5, y, -4.5), v(4.5, y, 4.5));
    for (x, z) in [(-3.8, -3.8), (3.8, -3.8), (-3.8, 3.8), (3.8, 3.8)] {
        b.post(x, z, y - 0.25, 0.9);
    }
    b.building(v(0.0, y, 0.0), 5.0, 5.0, 2.8, &[('n', 0.0), ('s', 0.0)], &[('w', 0.0), ('e', 0.0)], OFFICE_WALL);
    b.solid(v(-1.2, y + 0.0, -0.8), v(-0.4, y + 1.0, 0.8), STEEL_DARK);
    b.rail(v(-4.45, y, -4.45), v(-4.45, y, -2.9));
    b.rail(v(-4.45, y, 0.9), v(-4.45, y, 4.45));
    b.rail(v(-4.45, y, -4.45), v(4.45, y, -4.45));
    b.rail(v(-4.45, y, 4.45), v(4.45, y, 4.45));
    b.rail(v(4.45, y, -4.45), v(4.45, y, -3.0));
    b.rail(v(4.45, y, 2.9), v(4.45, y, 4.45));
}

fn build_half(b: &mut Builder) {
    let y = DECK_HEIGHT;
    b.stairs(v(1.5, 0.0, 10.5), Heading::MinusZ, 2.0, y, STEEL);
    b.rail(v(0.45, y, 4.55), v(0.45, y, 10.6));
    b.rail(v(2.55, y, 4.55), v(2.55, y, 10.6));

    b.catwalk_x(-27.0, -4.5, -2.0, y, (false, true));
    b.rail(v(-26.95, y, -3.0), v(-26.95, y, -1.05));
    b.catwalk_z(-26.0, -12.0, -3.0, y, (true, true));
    b.deck(v(-29.0, y, -18.0), v(-23.0, y, -12.0));
    for (px, pz) in [(-28.5, -17.5), (-23.5, -17.5), (-28.5, -12.5), (-23.5, -12.5)] {
        b.post(px, pz, y - 0.25, 0.5);
    }
    b.rail(v(-29.0, y, -18.0), v(-23.0, y, -18.0));
    b.rail(v(-23.0, y, -18.0), v(-23.0, y, -12.0));
    b.rail(v(-29.0, y, -18.0), v(-29.0, y, -16.1));
    b.rail(v(-29.0, y, -13.9), v(-29.0, y, -12.0));
    b.rail(v(-29.0, y, -12.0), v(-27.0, y, -12.0));
    b.rail(v(-25.0, y, -12.0), v(-23.0, y, -12.0));
    b.stairs(v(-35.3, 0.0, -15.0), Heading::PlusX, 2.0, y, STEEL);
    b.visual_only(v(-34.8, 0.0, -14.1), v(-34.7, y, -13.9), SAFETY_YELLOW);

    b.tank_at(-17.0, 17.0, 5.0, 7.0, true);
    b.tank_at(-37.0, 6.0, 3.2, 5.0, false);

    b.container_at(-31.0, -26.5, true, 1, 0);
    b.container_at(-17.0, -28.5, true, 2, 1);
    b.container_at(-6.0, -22.0, false, 1, 2);
    b.container_at(-39.0, -12.0, false, 1, 3);
    b.container_at(14.0, 13.5, true, 1, 4);
    b.stairs(v(-21.0, 0.0, -26.5), Heading::MinusX, 1.6, 2.6, WOOD);
    b.stairs(v(-0.9, 0.0, -20.0), Heading::MinusX, 1.6, 2.6, WOOD);

    b.building(v(-6.0, 0.0, 24.0), 9.0, 7.0, 3.2, &[('n', -6.0), ('e', 24.0)], &[('w', 22.5), ('w', 25.5), ('s', -8.0), ('s', -4.0)], OFFICE_WALL);
    b.office_furniture(v(-6.0, 0.0, 24.0));
    b.stairs(v(-15.8, 0.0, 26.0), Heading::PlusX, 1.8, 3.4, WOOD);

    b.pipe_rack(12.0, 30.0, -6.0);
    let elbow = b.prop_cache.elbow.clone();
    b.place(&elbow, v(30.5, 0.0, -6.5), 0.0);

    b.barrels(-28.0, 10.0, 4, 0);
    b.barrels(8.0, -10.0, 3, 1);
    b.barrels(-2.0, -14.0, 3, 2);
    b.barrels(30.0, 20.0, 5, 0);
    b.crates(-12.0, 10.0, 2, 0);
    b.crates(-10.8, 10.0, 1, 1);
    b.crates(18.0, 4.0, 1, 2);
    b.crates(28.0, 10.0, 2, 3);
    b.crates(-30.0, 26.0, 3, 1);

    let jersey = b.prop_cache.jersey.clone();
    for (x, z, yaw) in [(-8.0, -6.0, 0.0), (-8.0, -9.0, 0.0), (20.0, -26.0, 90.0), (-20.0, 2.5, 90.0), (9.0, 19.0, 0.0)] {
        let (half_x, half_z) = if yaw == 0.0 { (0.3, 1.5) } else { (1.5, 0.3) };
        b.register(&format!("jersey_{:.0}_{:.0}", x, z), ItemKind::Cover, "concrete barrier as cover", x, z, half_x * 2.0, half_z * 2.0, 0.0, 0.8);
        b.collider_only(v(x - half_x, 0.0, z - half_z), v(x + half_x, 0.8, z + half_z));
        b.place(&jersey, v(x, 0.0, z), yaw);
    }
    let sandbags = b.prop_cache.sandbags.clone();
    for (x, z) in [(-24.0, 0.0), (26.0, 30.0)] {
        b.register(&format!("sandbags_{:.0}_{:.0}", x, z), ItemKind::Cover, "sandbag wall as cover", x, z, 3.0, 0.4, 0.0, 0.52);
        b.collider_only(v(x - 1.5, 0.0, z - 0.2), v(x + 1.5, 0.52, z + 0.2));
        b.place(&sandbags, v(x, 0.0, z), 0.0);
    }

    let derrick = b.prop_cache.derrick.clone();
    b.register("derrick", ItemKind::Solid, "derrick landmark in the corner", -41.5, -31.0, 5.9, 5.9, 0.0, 6.0);
    b.place(&derrick, v(-41.5, 0.0, -31.0), 0.0);
    for (dx, dz) in [(-2.4, -2.4), (2.4, -2.4), (-2.4, 2.4), (2.4, 2.4)] {
        b.collider_only(v(-41.5 + dx - 0.35, 0.0, -31.0 + dz - 0.35), v(-41.5 + dx + 0.35, 6.0, -31.0 + dz + 0.35));
    }
    let pump = b.prop_cache.pump_jack.clone();
    b.register("pump_jack", ItemKind::Solid, "working pump jack as dressing and cover", -8.0, -30.0, 2.6, 1.4, 0.0, 1.0);
    b.collider_only(v(-9.3, 0.0, -30.7), v(-6.7, 1.0, -29.3));
    b.place(&pump, v(-8.0, 0.0, -30.0), 0.0);
    let lamp = b.prop_cache.lamp.clone();
    for (x, z) in [(-14.0, -10.0), (-30.0, 16.0), (30.0, -22.0)] {
        b.register(&format!("lamp_{:.0}_{:.0}", x, z), ItemKind::Dressing, "lamp post", x, z, 0.2, 0.2, 0.0, 6.0);
        b.place(&lamp, v(x, 0.0, z), 0.0);
        b.collider_only(v(x - 0.1, 0.0, z - 0.1), v(x + 0.1, 6.0, z + 0.1));
    }

    if !b.flipped {
        b.spawns.push(Spawn { position: v(-36.0, 0.05, -22.0), yaw: -30.0 });
        b.spawns.push(Spawn { position: v(-34.0, 0.05, 14.0), yaw: -100.0 });
        b.spawns.push(Spawn { position: v(-14.0, 0.05, -7.0), yaw: 20.0 });
        b.spawns.push(Spawn { position: v(-2.0, 0.05, 32.0), yaw: 180.0 });
        b.pickups.push(PickupSpot { kind: PickupKind::Health, position: v(-24.0, 0.0, 3.0) });
        b.pickups.push(PickupSpot { kind: PickupKind::Ammo, position: v(-10.0, 0.0, -16.0) });
        b.pickups.push(PickupSpot { kind: PickupKind::Health, position: v(-34.0, 0.0, 20.0) });
        b.pickups.push(PickupSpot { kind: PickupKind::Ammo, position: v(-22.0, y, -2.0) });
    }
}

fn build_scenery(b: &mut Builder) {
    let mesas = [
        (-120.0, -60.0, 28.0, 14.0, 20.0),
        (-130.0, 40.0, 36.0, 12.0, 26.0),
        (-60.0, 110.0, 30.0, 16.0, 18.0),
        (50.0, 120.0, 40.0, 14.0, 24.0),
        (125.0, 50.0, 30.0, 18.0, 22.0),
        (118.0, -60.0, 34.0, 12.0, 30.0),
        (40.0, -115.0, 32.0, 14.0, 20.0),
        (-70.0, -120.0, 26.0, 12.0, 16.0),
    ];
    for (i, (x, z, sx, sz, h)) in mesas.iter().enumerate() {
        let tone = 0.85 + 0.15 * ((i * 5) % 3) as f32 / 2.0;
        let base = Vec3::new(0.66, 0.40, 0.26) * tone;
        let mut mesh = Mesh::new();
        mesh.add_box(v(*x, h * 0.35, *z), v(*sx, h * 0.7, *sz), base);
        mesh.add_box(v(*x + 2.0, h * 0.8, *z - 1.0), v(sx * 0.7, h * 0.4, sz * 0.7), base * 1.12);
        b.mesh.append(&mesh);
    }
}

fn ground_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    let half = 150.0;
    let step = 2.0;
    let count = (half * 2.0 / step) as usize;
    let breaks: Vec<f32> = (0..=count).map(|i| -half + i as f32 * step).collect();
    let mut positions = Vec::new();
    for z in &breaks {
        for x in &breaks {
            let n = fractal_noise(x * 0.07, 0.0, z * 0.07, 3, 5);
            let grain = fractal_noise(x * 0.8, 1.0, z * 0.8, 2, 9);
            let track = (((x * 0.3 + z * 0.12).sin() * 3.0).abs() < 0.4) as i32 as f32;
            let tone = 0.82 + 0.3 * n + 0.08 * grain - 0.1 * track;
            positions.push(SAND * tone);
        }
    }
    let columns = breaks.len();
    let mut indices = vec![0u32; columns * columns];
    for (row, z) in breaks.iter().enumerate() {
        for (column, x) in breaks.iter().enumerate() {
            indices[row * columns + column] = mesh.push_vertex(v(*x, 0.0, *z), v(0.0, 1.0, 0.0), positions[row * columns + column]);
        }
    }
    for row in 0..columns - 1 {
        for column in 0..columns - 1 {
            let (x0, z0) = (breaks[column], breaks[row]);
            let inside = x0 > -ARENA_HALF_X - 2.0 && x0 < ARENA_HALF_X + 2.0 && z0 > -ARENA_HALF_Z - 2.0 && z0 < ARENA_HALF_Z + 2.0;
            if inside {
                continue;
            }
            let a = indices[row * columns + column];
            let bb = indices[row * columns + column + 1];
            let c = indices[(row + 1) * columns + column + 1];
            let d = indices[(row + 1) * columns + column];
            mesh.push_triangle(a, c, bb);
            mesh.push_triangle(a, d, c);
        }
    }
    mesh
}

fn arena_floor(b: &mut Builder) {
    let mut mesh = Mesh::new();
    let step = 1.0;
    let (nx, nz) = ((2.0 * (ARENA_HALF_X + 2.0) / step) as usize, (2.0 * (ARENA_HALF_Z + 2.0) / step) as usize);
    let xs: Vec<f32> = (0..=nx).map(|i| -ARENA_HALF_X - 2.0 + i as f32 * step).collect();
    let zs: Vec<f32> = (0..=nz).map(|i| -ARENA_HALF_Z - 2.0 + i as f32 * step).collect();
    let mut indices = vec![0u32; xs.len() * zs.len()];
    for (row, z) in zs.iter().enumerate() {
        for (column, x) in xs.iter().enumerate() {
            let n = fractal_noise(x * 0.09, 0.0, z * 0.09, 3, 5);
            let grain = fractal_noise(x * 1.1, 1.0, z * 1.1, 2, 9);
            let lane = ((x * 0.31 + z * 0.12).sin() * 3.0).abs() < 0.45;
            let oil = fractal_noise(x * 0.35 + 40.0, 2.0, z * 0.35, 3, 21) > 0.72;
            let mut tone = 0.82 + 0.3 * n + 0.07 * grain - if lane { 0.12 } else { 0.0 };
            let mut color = SAND * tone;
            if oil {
                tone *= 0.45;
                color = Vec3::new(0.30, 0.26, 0.22) * tone * 1.6;
            }
            indices[row * xs.len() + column] = mesh.push_vertex(v(*x, 0.0, *z), v(0.0, 1.0, 0.0), color);
        }
    }
    for row in 0..zs.len() - 1 {
        for column in 0..xs.len() - 1 {
            let a = indices[row * xs.len() + column];
            let bb = indices[row * xs.len() + column + 1];
            let c = indices[(row + 1) * xs.len() + column + 1];
            let d = indices[(row + 1) * xs.len() + column];
            mesh.push_triangle(a, c, bb);
            mesh.push_triangle(a, d, c);
        }
    }
    b.mesh.append(&mesh);
}

pub fn build_level(with_bake: bool) -> Level {
    let mut b = Builder::new();
    build_perimeter(&mut b);
    build_center_tower(&mut b);
    for flipped in [false, true] {
        b.flipped = flipped;
        build_half(&mut b);
    }
    b.flipped = false;
    arena_floor(&mut b);
    b.mesh.append(&ground_mesh());
    build_scenery(&mut b);
    for index in 0..b.spawns.len() {
        let spawn = b.spawns[index];
        b.spawns.push(Spawn { position: v(-spawn.position.x, spawn.position.y, -spawn.position.z), yaw: spawn.yaw + 180.0 });
    }
    for index in 0..b.pickups.len() {
        let pickup = b.pickups[index];
        b.pickups.push(PickupSpot { kind: pickup.kind, position: v(-pickup.position.x, pickup.position.y, -pickup.position.z) });
    }
    b.collision.build_index();
    if with_bake {
        let occluders = Bvh::from_meshes(&[&b.mesh]);
        let settings = BakeSettings { ao_samples: 10, ao_distance: 1.6, ao_strength: 0.8, ..BakeSettings::default() };
        bake(&mut b.mesh, &occluders, &settings);
    }
    let mut layout = b.layout;
    layout.zone("arena", Purpose::Cover, Rect::new(-ARENA_HALF_X - 1.5, -ARENA_HALF_Z - 1.5, ARENA_HALF_X + 1.5, ARENA_HALF_Z + 1.5));
    for (index, spawn) in b.spawns.iter().enumerate() {
        layout.zone(&format!("spawn_{}", index), Purpose::Spawn, Rect::new(spawn.position.x - 1.0, spawn.position.z - 1.0, spawn.position.x + 1.0, spawn.position.z + 1.0));
    }
    let half = b.spawns.len() / 2;
    for index in 0..half {
        layout.link(&format!("spawn_{}", index), &format!("spawn_{}", index + half), 1.0, 2);
    }
    Level { collision: b.collision, mesh: b.mesh, spawns: b.spawns, pickups: b.pickups, map_rects: b.map_rects, layout }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::physics::Body;

    #[test]
    fn layout_passes_the_check_gate() {
        let level = build_level(false);
        let findings = level.layout.check();
        let errors: Vec<_> = findings.iter().filter(|f| f.severity == engine::probe::Severity::Error).collect();
        assert!(errors.is_empty(), "{}", engine::layout::check::report(&findings));
    }

    #[test]
    fn spawns_are_clear_of_solids() {
        let level = build_level(false);
        for spawn in &level.spawns {
            let body = Body::new(spawn.position, 0.3, 1.8);
            assert!(!level.collision.overlaps_any(&body.aabb()), "spawn inside solid at {:?}", spawn.position);
        }
    }

    #[test]
    fn eye_positions_are_not_buried_in_the_mesh() {
        let level = build_level(false);
        for spawn in &level.spawns {
            let eye = spawn.position + Vec3::new(0.0, 1.62, 0.0);
            let mut covering = 0;
            for t in 0..level.mesh.triangle_count() {
                let [a, b, c] = level.mesh.triangle(t);
                let low = a.min(b).min(c);
                let high = a.max(b).max(c);
                if low.x < eye.x + 0.3 && high.x > eye.x - 0.3 && low.z < eye.z + 0.3 && high.z > eye.z - 0.3 && low.y < eye.y + 0.3 && high.y > eye.y - 0.3 {
                    covering += 1;
                }
            }
            assert_eq!(covering, 0, "{} triangles surround the eye at {:?}", covering, eye);
        }
    }
}
