pub mod check;
pub mod footprint;
pub mod nav;
pub mod solver;

use footprint::{Footprint, Rect};

pub const WALK_CLEARANCE: f32 = 1.0;
pub const FLUSH_TOLERANCE: f32 = 0.05;
pub const OVERLAP_TOLERANCE: f32 = 0.02;
pub const HEAD_CLEARANCE: f32 = 2.0;
pub const STEP_HEIGHT: f32 = 0.4;
pub const MIN_REASON_LENGTH: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Purpose {
    Spawn,
    Objective,
    Cover,
    Vantage,
    Route,
    Landmark,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    Solid,
    Cover,
    Deck,
    Support,
    Stairs,
    Dressing,
}

impl ItemKind {
    pub fn blocks_ground(self, footprint: &Footprint) -> bool {
        match self {
            ItemKind::Dressing | ItemKind::Stairs => false,
            _ => footprint.y_min < HEAD_CLEARANCE && footprint.y_max > STEP_HEIGHT,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub name: String,
    pub purpose: Purpose,
    pub bounds: Rect,
    pub level: usize,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub kind: ItemKind,
    pub reason: String,
    pub zone: String,
    pub footprint: Footprint,
    pub attached_to: Vec<String>,
}

impl Item {
    pub fn new(name: &str, kind: ItemKind, reason: &str, zone: &str, footprint: Footprint) -> Item {
        Item { name: name.to_string(), kind, reason: reason.to_string(), zone: zone.to_string(), footprint, attached_to: Vec::new() }
    }

    pub fn attached(mut self, other: &str) -> Item {
        self.attached_to.push(other.to_string());
        self
    }

    pub fn is_attached_to(&self, other: &Item) -> bool {
        self.attached_to.contains(&other.name) || other.attached_to.contains(&self.name)
    }
}

#[derive(Clone, Debug)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub width: f32,
    pub min_routes: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Portal {
    pub ground: (f32, f32),
    pub deck: (f32, f32),
}

#[derive(Clone, Debug)]
pub struct Request {
    pub name: String,
    pub kind: ItemKind,
    pub reason: String,
    pub zone: String,
    pub size: (f32, f32, f32),
    pub base_height: f32,
    pub count: usize,
    pub min_spacing: f32,
    pub yaw_choices: Vec<f32>,
    pub keep_out: Vec<((f32, f32), f32)>,
}

impl Request {
    pub fn new(name: &str, kind: ItemKind, reason: &str, zone: &str, size: (f32, f32, f32)) -> Request {
        Request {
            name: name.to_string(),
            kind,
            reason: reason.to_string(),
            zone: zone.to_string(),
            size,
            base_height: 0.0,
            count: 1,
            min_spacing: 0.0,
            yaw_choices: vec![0.0, 90.0],
            keep_out: Vec::new(),
        }
    }

    pub fn count(mut self, count: usize) -> Request {
        self.count = count;
        self
    }

    pub fn spaced(mut self, spacing: f32) -> Request {
        self.min_spacing = spacing;
        self
    }

    pub fn yaws(mut self, yaws: &[f32]) -> Request {
        self.yaw_choices = yaws.to_vec();
        self
    }

    pub fn base_height(mut self, height: f32) -> Request {
        self.base_height = height;
        self
    }

    pub fn keep_out(mut self, center: (f32, f32), radius: f32) -> Request {
        self.keep_out.push((center, radius));
        self
    }
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub name: String,
    pub bounds: Rect,
    pub zones: Vec<Zone>,
    pub items: Vec<Item>,
    pub links: Vec<Link>,
    pub portals: Vec<Portal>,
    pub keep_clear: Vec<((f32, f32), f32)>,
}

impl Layout {
    pub fn new(name: &str, bounds: Rect) -> Layout {
        Layout { name: name.to_string(), bounds, zones: Vec::new(), items: Vec::new(), links: Vec::new(), portals: Vec::new(), keep_clear: Vec::new() }
    }

    pub fn zone(&mut self, name: &str, purpose: Purpose, bounds: Rect) {
        self.zones.push(Zone { name: name.to_string(), purpose, bounds, level: 0 });
    }

    pub fn deck_zone(&mut self, name: &str, purpose: Purpose, bounds: Rect) {
        self.zones.push(Zone { name: name.to_string(), purpose, bounds, level: 1 });
    }

    pub fn place(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn link(&mut self, from: &str, to: &str, width: f32, min_routes: usize) {
        self.links.push(Link { from: from.to_string(), to: to.to_string(), width, min_routes });
    }

    pub fn portal(&mut self, ground: (f32, f32), deck: (f32, f32)) {
        self.portals.push(Portal { ground, deck });
    }

    pub fn keep_clear(&mut self, center: (f32, f32), radius: f32) {
        self.keep_clear.push((center, radius));
    }

    pub fn solve(&mut self, requests: &[Request], seed: u64) -> Vec<crate::probe::Finding> {
        solver::solve(self, requests, seed)
    }

    pub fn check(&self) -> Vec<crate::probe::Finding> {
        check::check(self)
    }

    pub fn zone_named(&self, name: &str) -> Option<&Zone> {
        self.zones.iter().find(|z| z.name == name)
    }

    pub fn item_named(&self, name: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.name == name)
    }
}
