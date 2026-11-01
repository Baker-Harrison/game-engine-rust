use super::footprint::Footprint;
use super::{Layout, Portal};
use std::collections::VecDeque;

pub const CELL: f32 = 0.5;
const ANCHOR_SEARCH_RADIUS: f32 = 3.0;
const ROUTE_SEPARATION: f32 = 2.5;
const ENDPOINT_FREE_RADIUS: f32 = 4.0;

pub struct NavGrid {
    pub columns: usize,
    pub rows: usize,
    origin: (f32, f32),
    blocked: Vec<bool>,
    deck: Vec<bool>,
    portals: Vec<(usize, usize)>,
}

impl NavGrid {
    pub fn build(layout: &Layout, agent_width: f32) -> NavGrid {
        let columns = ((layout.bounds.max_x - layout.bounds.min_x) / CELL).ceil() as usize;
        let rows = ((layout.bounds.max_z - layout.bounds.min_z) / CELL).ceil() as usize;
        let mut grid = NavGrid {
            columns,
            rows,
            origin: (layout.bounds.min_x, layout.bounds.min_z),
            blocked: vec![false; columns * rows],
            deck: vec![false; columns * rows],
            portals: Vec::new(),
        };
        let margin = agent_width * 0.5;
        let blockers: Vec<&Footprint> = layout.items.iter().filter(|i| i.kind.blocks_ground(&i.footprint)).map(|i| &i.footprint).collect();
        let decks: Vec<&Footprint> = layout.items.iter().filter(|i| i.kind == super::ItemKind::Deck).map(|i| &i.footprint).collect();
        for row in 0..rows {
            for column in 0..columns {
                let (x, z) = grid.center(column, row);
                let index = row * columns + column;
                grid.blocked[index] = blockers.iter().any(|f| f.distance_to_point(x, z) < margin);
                grid.deck[index] = decks.iter().any(|f| f.contains_point(x, z));
            }
        }
        grid.portals = layout.portals.iter().filter_map(|p| grid.portal_cells(p)).collect();
        grid
    }

    fn portal_cells(&self, portal: &Portal) -> Option<(usize, usize)> {
        let ground = self.nearest_free(portal.ground, false)?;
        let deck = self.nearest_free(portal.deck, true)?;
        Some((ground, deck))
    }

    fn center(&self, column: usize, row: usize) -> (f32, f32) {
        (self.origin.0 + (column as f32 + 0.5) * CELL, self.origin.1 + (row as f32 + 0.5) * CELL)
    }

    fn cell_of(&self, point: (f32, f32)) -> Option<(usize, usize)> {
        let column = ((point.0 - self.origin.0) / CELL).floor();
        let row = ((point.1 - self.origin.1) / CELL).floor();
        (column >= 0.0 && row >= 0.0 && (column as usize) < self.columns && (row as usize) < self.rows).then(|| (column as usize, row as usize))
    }

    fn node(&self, level: usize, cell: usize) -> usize {
        level * self.columns * self.rows + cell
    }

    fn walkable(&self, level: usize, cell: usize) -> bool {
        if level == 0 {
            !self.blocked[cell]
        } else {
            self.deck[cell]
        }
    }

    pub fn nearest_free(&self, point: (f32, f32), on_deck: bool) -> Option<usize> {
        let (column, row) = self.cell_of(point)?;
        let radius = (ANCHOR_SEARCH_RADIUS / CELL) as i32;
        let mut best: Option<(i32, usize)> = None;
        for dr in -radius..=radius {
            for dc in -radius..=radius {
                let (c, r) = (column as i32 + dc, row as i32 + dr);
                if c < 0 || r < 0 || c as usize >= self.columns || r as usize >= self.rows {
                    continue;
                }
                let cell = r as usize * self.columns + c as usize;
                if self.walkable(if on_deck { 1 } else { 0 }, cell) && best.map(|(d, _)| dc * dc + dr * dr < d).unwrap_or(true) {
                    best = Some((dc * dc + dr * dr, cell));
                }
            }
        }
        best.map(|(_, cell)| cell)
    }

    fn neighbors(&self, node: usize) -> Vec<usize> {
        let cells = self.columns * self.rows;
        let (level, cell) = (node / cells, node % cells);
        let (column, row) = (cell % self.columns, cell / self.columns);
        let mut out = Vec::new();
        for (dc, dr) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (c, r) = (column as i32 + dc, row as i32 + dr);
            if c >= 0 && r >= 0 && (c as usize) < self.columns && (r as usize) < self.rows {
                let next = r as usize * self.columns + c as usize;
                if self.walkable(level, next) {
                    out.push(self.node(level, next));
                }
            }
        }
        for (ground, deck) in &self.portals {
            if cell == *ground && level == 0 {
                out.push(self.node(1, *deck));
            }
            if cell == *deck && level == 1 {
                out.push(self.node(0, *ground));
            }
        }
        out
    }

    pub fn route(&self, start: usize, goal: usize, avoid: &[bool]) -> Option<Vec<usize>> {
        let mut previous = vec![usize::MAX; self.columns * self.rows * 2];
        let mut queue = VecDeque::new();
        previous[start] = start;
        queue.push_back(start);
        while let Some(node) = queue.pop_front() {
            if node == goal {
                let mut path = vec![node];
                let mut at = node;
                while at != start {
                    at = previous[at];
                    path.push(at);
                }
                return Some(path);
            }
            for next in self.neighbors(node) {
                if previous[next] == usize::MAX && (next == goal || !avoid[next]) {
                    previous[next] = node;
                    queue.push_back(next);
                }
            }
        }
        None
    }

    pub fn count_routes(&self, start: usize, goal: usize, wanted: usize) -> usize {
        let nodes = self.columns * self.rows * 2;
        let mut avoid = vec![false; nodes];
        let mut routes = 0;
        for _ in 0..wanted.max(1) {
            let Some(path) = self.route(start, goal, &avoid) else { break };
            routes += 1;
            let start_point = self.node_point(start);
            let goal_point = self.node_point(goal);
            for node in &path {
                let point = self.node_point(*node);
                if distance(point, start_point) < ENDPOINT_FREE_RADIUS || distance(point, goal_point) < ENDPOINT_FREE_RADIUS {
                    continue;
                }
                self.avoid_around(*node, &mut avoid);
            }
        }
        routes
    }

    fn node_point(&self, node: usize) -> (f32, f32) {
        let cell = node % (self.columns * self.rows);
        self.center(cell % self.columns, cell / self.columns)
    }

    fn avoid_around(&self, node: usize, avoid: &mut [bool]) {
        let cells = self.columns * self.rows;
        let (level, cell) = (node / cells, node % cells);
        let (column, row) = ((cell % self.columns) as i32, (cell / self.columns) as i32);
        let radius = (ROUTE_SEPARATION / CELL) as i32;
        for dr in -radius..=radius {
            for dc in -radius..=radius {
                let (c, r) = (column + dc, row + dr);
                if c >= 0 && r >= 0 && (c as usize) < self.columns && (r as usize) < self.rows {
                    avoid[self.node(level, r as usize * self.columns + c as usize)] = true;
                }
            }
        }
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

pub fn anchor_node(grid: &NavGrid, layout: &Layout, zone: &str) -> Option<usize> {
    let zone = layout.zone_named(zone)?;
    let node = grid.nearest_free(zone.bounds.center(), zone.level == 1)?;
    Some(grid.node(zone.level, node))
}
