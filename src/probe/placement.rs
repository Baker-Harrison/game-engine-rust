use super::targets::Targets;
use super::{Finding, ProbeScene, Severity};
use crate::math::Vec3;
use std::collections::{HashMap, HashSet};

const GRID_CELL: f32 = 0.25;
const MAX_CELLS_PER_TRIANGLE: i64 = 4096;
const SEPARATION_EPSILON: f32 = 1e-4;

#[derive(Clone, Copy)]
struct Overlap {
    pairs: usize,
    max_depth: f32,
    low: Vec3,
    high: Vec3,
}

pub fn findings(scene: &ProbeScene, targets: &Targets) -> Vec<Finding> {
    let mut overlaps: HashMap<(usize, usize), Overlap> = HashMap::new();
    for (first, second) in candidate_pairs(scene) {
        let (a, b) = (scene.mesh.triangle(first), scene.mesh.triangle(second));
        let Some(crossing) = proper_crossing(&a, &b) else { continue };
        let (owner_a, owner_b) = (scene.object_of_triangle[first] as usize, scene.object_of_triangle[second] as usize);
        let key = (owner_a.min(owner_b), owner_a.max(owner_b));
        let entry = overlaps.entry(key).or_insert(Overlap { pairs: 0, max_depth: 0.0, low: crossing.low, high: crossing.high });
        entry.pairs += 1;
        entry.max_depth = entry.max_depth.max(crossing.depth);
        entry.low = entry.low.min(crossing.low);
        entry.high = entry.high.max(crossing.high);
    }
    let mut keys: Vec<_> = overlaps.keys().copied().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| {
            let overlap = overlaps[&key];
            (overlap.max_depth > targets.penetration_max).then(|| {
                Finding::new(
                    Severity::Error,
                    &format!("{} / {}", scene.objects[key.0].name, scene.objects[key.1].name),
                    "interpenetration_depth_m",
                    overlap.max_depth,
                    format!("<= {:.3}", targets.penetration_max),
                    &format!(
                        "{} triangle pairs cross inside x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2}; move or resize one object",
                        overlap.pairs, overlap.low.x, overlap.high.x, overlap.low.y, overlap.high.y, overlap.low.z, overlap.high.z
                    ),
                )
            })
        })
        .collect()
}

fn triangle_box(corners: &[Vec3; 3]) -> (Vec3, Vec3) {
    (corners[0].min(corners[1]).min(corners[2]), corners[0].max(corners[1]).max(corners[2]))
}

fn candidate_pairs(scene: &ProbeScene) -> Vec<(usize, usize)> {
    let mut cells: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
    for triangle in 0..scene.mesh.triangle_count() {
        let (low, high) = triangle_box(&scene.mesh.triangle(triangle));
        let (lo, hi) = (cell_of(low), cell_of(high));
        let count = (hi.0 - lo.0 + 1) as i64 * (hi.1 - lo.1 + 1) as i64 * (hi.2 - lo.2 + 1) as i64;
        if count > MAX_CELLS_PER_TRIANGLE {
            continue;
        }
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                for z in lo.2..=hi.2 {
                    cells.entry((x, y, z)).or_default().push(triangle as u32);
                }
            }
        }
    }
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    let mut pairs = Vec::new();
    for members in cells.values() {
        for (index, &first) in members.iter().enumerate() {
            for &second in &members[index + 1..] {
                let (owner_a, owner_b) = (scene.object_of_triangle[first as usize] as usize, scene.object_of_triangle[second as usize] as usize);
                if owner_a == owner_b || scene.overlap_allowed(owner_a, owner_b) {
                    continue;
                }
                if boxes_apart(scene, first as usize, second as usize) {
                    continue;
                }
                if seen.insert((first.min(second), first.max(second))) {
                    pairs.push((first as usize, second as usize));
                }
            }
        }
    }
    pairs
}

fn boxes_apart(scene: &ProbeScene, first: usize, second: usize) -> bool {
    let (a_low, a_high) = triangle_box(&scene.mesh.triangle(first));
    let (b_low, b_high) = triangle_box(&scene.mesh.triangle(second));
    a_high.x < b_low.x - SEPARATION_EPSILON
        || b_high.x < a_low.x - SEPARATION_EPSILON
        || a_high.y < b_low.y - SEPARATION_EPSILON
        || b_high.y < a_low.y - SEPARATION_EPSILON
        || a_high.z < b_low.z - SEPARATION_EPSILON
        || b_high.z < a_low.z - SEPARATION_EPSILON
}

fn cell_of(point: Vec3) -> (i32, i32, i32) {
    ((point.x / GRID_CELL).floor() as i32, (point.y / GRID_CELL).floor() as i32, (point.z / GRID_CELL).floor() as i32)
}

struct Crossing {
    depth: f32,
    low: Vec3,
    high: Vec3,
}

fn plane_distances(corners: &[Vec3; 3], origin: Vec3, normal: Vec3) -> [f32; 3] {
    corners.map(|c| (c - origin).dot(normal))
}

fn straddles(distances: &[f32; 3]) -> bool {
    distances.iter().any(|d| *d > SEPARATION_EPSILON) && distances.iter().any(|d| *d < -SEPARATION_EPSILON)
}

fn poke_depth(distances: &[f32; 3]) -> f32 {
    let beyond = distances.iter().fold(0.0f32, |m, d| m.max(*d));
    let behind = distances.iter().fold(0.0f32, |m, d| m.max(-*d));
    beyond.min(behind)
}

fn plane_cut(corners: &[Vec3; 3], distances: &[f32; 3], direction: Vec3) -> Vec<(f32, Vec3)> {
    let mut cuts = Vec::new();
    for k in 0..3 {
        let j = (k + 1) % 3;
        let (d0, d1) = (distances[k], distances[j]);
        if (d0 > 0.0) != (d1 > 0.0) && (d0 - d1).abs() > 1e-12 {
            let point = corners[k] + (corners[j] - corners[k]) * (d0 / (d0 - d1));
            cuts.push((point.dot(direction), point));
        }
    }
    cuts
}

fn proper_crossing(a: &[Vec3; 3], b: &[Vec3; 3]) -> Option<Crossing> {
    let normal_a = (a[1] - a[0]).cross(a[2] - a[0]).normalized();
    let normal_b = (b[1] - b[0]).cross(b[2] - b[0]).normalized();
    let direction = normal_a.cross(normal_b);
    if direction.length() < 1e-6 {
        return None;
    }
    let direction = direction.normalized();
    let a_to_b = plane_distances(a, b[0], normal_b);
    let b_to_a = plane_distances(b, a[0], normal_a);
    if !straddles(&a_to_b) || !straddles(&b_to_a) {
        return None;
    }
    let (cut_a, cut_b) = (plane_cut(a, &a_to_b, direction), plane_cut(b, &b_to_a, direction));
    if cut_a.len() < 2 || cut_b.len() < 2 {
        return None;
    }
    let ordered = |cuts: &[(f32, Vec3)]| if cuts[0].0 <= cuts[1].0 { (cuts[0], cuts[1]) } else { (cuts[1], cuts[0]) };
    let ((a_low, a_high), (b_low, b_high)) = (ordered(&cut_a), ordered(&cut_b));
    let start = if a_low.0 >= b_low.0 { a_low } else { b_low };
    let end = if a_high.0 <= b_high.0 { a_high } else { b_high };
    if end.0 - start.0 <= SEPARATION_EPSILON {
        return None;
    }
    Some(Crossing {
        depth: poke_depth(&a_to_b).min(poke_depth(&b_to_a)),
        low: start.1.min(end.1),
        high: start.1.max(end.1),
    })
}
