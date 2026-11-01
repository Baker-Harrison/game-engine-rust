use crate::math::Vec3;
use crate::physics::{Aabb, CollisionWorld};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Clone, Copy, Debug)]
pub struct NavSettings {
    pub cell: f32,
    pub agent_radius: f32,
    pub agent_height: f32,
    pub step_height: f32,
    pub max_surface_height: f32,
}

impl Default for NavSettings {
    fn default() -> NavSettings {
        NavSettings { cell: 0.5, agent_radius: 0.32, agent_height: 1.75, step_height: 0.45, max_surface_height: 40.0 }
    }
}

#[derive(Clone, Debug)]
pub struct NavNode {
    pub position: Vec3,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct NavGraph {
    pub nodes: Vec<NavNode>,
    cell: f32,
    index: HashMap<(i32, i32), Vec<usize>>,
}

fn standing_box(position: Vec3, settings: &NavSettings) -> Aabb {
    let r = settings.agent_radius;
    Aabb::new(
        Vec3::new(position.x - r, position.y + settings.step_height, position.z - r),
        Vec3::new(position.x + r, position.y + settings.agent_height, position.z + r),
    )
}

impl NavGraph {
    pub fn build(world: &CollisionWorld, settings: &NavSettings) -> NavGraph {
        let mut graph = NavGraph { nodes: Vec::new(), cell: settings.cell, index: HashMap::new() };
        let Some(bounds) = world.bounds() else {
            return graph;
        };
        let (cx0, cx1) = ((bounds.min.x / settings.cell).floor() as i32, (bounds.max.x / settings.cell).ceil() as i32);
        let (cz0, cz1) = ((bounds.min.z / settings.cell).floor() as i32, (bounds.max.z / settings.cell).ceil() as i32);
        for cz in cz0..cz1 {
            for cx in cx0..cx1 {
                let x = (cx as f32 + 0.5) * settings.cell;
                let z = (cz as f32 + 0.5) * settings.cell;
                let column = Aabb::new(Vec3::new(x, -1000.0, z), Vec3::new(x, 1000.0, z));
                let mut tops: Vec<f32> = world
                    .candidates(&column)
                    .into_iter()
                    .map(|i| &world.boxes[i])
                    .filter(|b| b.min.x <= x && x <= b.max.x && b.min.z <= z && z <= b.max.z)
                    .map(|b| b.max.y)
                    .filter(|y| *y <= settings.max_surface_height)
                    .collect();
                tops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                tops.dedup_by(|a, b| (*a - *b).abs() < 0.01);
                for y in tops {
                    let position = Vec3::new(x, y, z);
                    if !world.overlaps_any(&standing_box(position, settings)) {
                        graph.index.entry((cx, cz)).or_default().push(graph.nodes.len());
                        graph.nodes.push(NavNode { position, links: Vec::new() });
                    }
                }
            }
        }
        graph.link(world, settings);
        graph
    }

    fn link(&mut self, world: &CollisionWorld, settings: &NavSettings) {
        let keys: Vec<(i32, i32)> = self.index.keys().copied().collect();
        let mut links: Vec<(usize, usize)> = Vec::new();
        for (cx, cz) in keys {
            for (dx, dz) in [(1, 0), (0, 1), (1, 1), (1, -1)] {
                let Some(neighbors) = self.index.get(&(cx + dx, cz + dz)) else {
                    continue;
                };
                for &a in &self.index[&(cx, cz)] {
                    for &b in neighbors {
                        let (pa, pb) = (self.nodes[a].position, self.nodes[b].position);
                        if (pa.y - pb.y).abs() > settings.step_height {
                            continue;
                        }
                        if Self::walkable_between(world, settings, pa, pb) {
                            links.push((a, b));
                        }
                    }
                }
            }
        }
        for (a, b) in links {
            self.nodes[a].links.push(b);
            self.nodes[b].links.push(a);
        }
    }

    fn walkable_between(world: &CollisionWorld, settings: &NavSettings, from: Vec3, to: Vec3) -> bool {
        let top = from.y.max(to.y);
        for t in [0.25, 0.5, 0.75] {
            let p = from.lerp(to, t);
            if world.overlaps_any(&standing_box(Vec3::new(p.x, top, p.z), settings)) {
                return false;
            }
        }
        true
    }

    pub fn nearest(&self, position: Vec3) -> Option<usize> {
        let (cx, cz) = ((position.x / self.cell).floor() as i32, (position.z / self.cell).floor() as i32);
        let mut best: Option<(f32, usize)> = None;
        for radius in 0..6i32 {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs().max(dz.abs()) != radius {
                        continue;
                    }
                    for &i in self.index.get(&(cx + dx, cz + dz)).into_iter().flatten() {
                        let p = self.nodes[i].position;
                        let vertical = (p.y - position.y).abs() * 2.0;
                        let horizontal = ((p.x - position.x).powi(2) + (p.z - position.z).powi(2)).sqrt();
                        let score = vertical + horizontal;
                        if best.map_or(true, |(s, _)| score < s) {
                            best = Some((score, i));
                        }
                    }
                }
            }
            if best.is_some() && radius >= 1 {
                break;
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn largest_component_from(&mut self, seed: Vec3) {
        let Some(start) = self.nearest(seed) else {
            return;
        };
        let mut keep = vec![false; self.nodes.len()];
        let mut stack = vec![start];
        keep[start] = true;
        while let Some(node) = stack.pop() {
            for &next in &self.nodes[node].links {
                if !keep[next] {
                    keep[next] = true;
                    stack.push(next);
                }
            }
        }
        let mut remap = vec![usize::MAX; self.nodes.len()];
        let mut nodes = Vec::new();
        for (i, node) in self.nodes.iter().enumerate() {
            if keep[i] {
                remap[i] = nodes.len();
                nodes.push(node.clone());
            }
        }
        for node in &mut nodes {
            node.links = node.links.iter().map(|l| remap[*l]).collect();
        }
        self.nodes = nodes;
        self.index.clear();
        for (i, node) in self.nodes.iter().enumerate() {
            let key = ((node.position.x / self.cell).floor() as i32, (node.position.z / self.cell).floor() as i32);
            self.index.entry(key).or_default().push(i);
        }
    }

    pub fn find_path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let start = self.nearest(from)?;
        let goal = self.nearest(to)?;
        if start == goal {
            return Some(vec![self.nodes[goal].position]);
        }
        let mut best_cost = vec![f32::INFINITY; self.nodes.len()];
        let mut came_from = vec![usize::MAX; self.nodes.len()];
        let mut open = BinaryHeap::new();
        best_cost[start] = 0.0;
        open.push(Frontier { priority: 0.0, node: start });
        let target = self.nodes[goal].position;
        while let Some(Frontier { node, .. }) = open.pop() {
            if node == goal {
                break;
            }
            let here = self.nodes[node].position;
            for &next in &self.nodes[node].links {
                let there = self.nodes[next].position;
                let cost = best_cost[node] + (there - here).length();
                if cost < best_cost[next] {
                    best_cost[next] = cost;
                    came_from[next] = node;
                    open.push(Frontier { priority: cost + (target - there).length(), node: next });
                }
            }
        }
        if came_from[goal] == usize::MAX {
            return None;
        }
        let mut path = vec![self.nodes[goal].position];
        let mut cursor = goal;
        while came_from[cursor] != usize::MAX {
            cursor = came_from[cursor];
            path.push(self.nodes[cursor].position);
        }
        path.reverse();
        Some(path)
    }

    pub fn random_node(&self, random: &mut crate::random::Random) -> Option<Vec3> {
        if self.nodes.is_empty() {
            return None;
        }
        let index = (random.next_u32() as usize) % self.nodes.len();
        Some(self.nodes[index].position)
    }
}

struct Frontier {
    priority: f32,
    node: usize,
}

impl PartialEq for Frontier {
    fn eq(&self, other: &Frontier) -> bool {
        self.node == other.node && self.priority == other.priority
    }
}

impl Eq for Frontier {}

impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Frontier) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frontier {
    fn cmp(&self, other: &Frontier) -> Ordering {
        other.priority.partial_cmp(&self.priority).unwrap_or(Ordering::Equal).then_with(|| other.node.cmp(&self.node))
    }
}
