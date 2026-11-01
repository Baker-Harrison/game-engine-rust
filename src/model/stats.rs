use super::mesh::Mesh;
use crate::math::Vec3;
use std::fmt::Write;

pub const TEXTURE_SIZE: f32 = 512.0;
const SILHOUETTE_GRID: usize = 128;
const BEVEL_MIN_DEGREES: f32 = 5.0;
const BEVEL_MAX_DEGREES: f32 = 60.0;
const HISTOGRAM_EDGES: [f32; 5] = [5.0, 15.0, 35.0, 60.0, 89.0];

#[derive(Clone, Debug)]
pub struct MeshStats {
    pub triangles: usize,
    pub surface_area: f32,
    pub size: Vec3,
    pub dihedral_histogram: [f32; 6],
    pub bevel_coverage: f32,
    pub silhouette_convexity: [f32; 3],
    pub texel_density: f32,
    pub triangles_per_square_meter: f32,
}

impl MeshStats {
    pub fn measure(mesh: &Mesh) -> MeshStats {
        let (low, high) = mesh.bounds();
        let surface_area = mesh.surface_area();
        let (dihedral_histogram, bevel_coverage) = dihedral_statistics(mesh);
        MeshStats {
            triangles: mesh.triangle_count(),
            surface_area,
            size: high - low,
            dihedral_histogram,
            bevel_coverage,
            silhouette_convexity: [silhouette_convexity(mesh, 0), silhouette_convexity(mesh, 1), silhouette_convexity(mesh, 2)],
            texel_density: texel_density(mesh),
            triangles_per_square_meter: if surface_area > 0.0 { mesh.triangle_count() as f32 / surface_area } else { 0.0 },
        }
    }

    pub fn mean_silhouette_convexity(&self) -> f32 {
        self.silhouette_convexity.iter().sum::<f32>() / 3.0
    }

    pub fn report(&self, label: &str) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "{}: {} tris, {:.2} m^2, size {:.2} x {:.2} x {:.2} m", label, self.triangles, self.surface_area, self.size.x, self.size.y, self.size.z);
        let _ = writeln!(
            text,
            "  dihedral <5/15/35/60/89/more: {}",
            self.dihedral_histogram.iter().map(|f| format!("{:.0}%", f * 100.0)).collect::<Vec<_>>().join(" ")
        );
        let _ = writeln!(text, "  bevel coverage {:.2}, silhouette convexity {:.2}", self.bevel_coverage, self.mean_silhouette_convexity());
        let _ = writeln!(text, "  texel density {:.0} px/m, {:.0} tris/m^2", self.texel_density, self.triangles_per_square_meter);
        text
    }
}

fn dihedral_statistics(mesh: &Mesh) -> ([f32; 6], f32) {
    let angles = mesh.edge_dihedral_angles();
    let mut counts = [0usize; 6];
    let mut bevels = 0usize;
    let mut creases = 0usize;
    for angle in &angles {
        let bin = HISTOGRAM_EDGES.iter().position(|limit| angle < limit).unwrap_or(5);
        counts[bin] += 1;
        if *angle >= BEVEL_MIN_DEGREES {
            creases += 1;
            if *angle <= BEVEL_MAX_DEGREES {
                bevels += 1;
            }
        }
    }
    let total = angles.len().max(1) as f32;
    let histogram = counts.map(|c| c as f32 / total);
    let coverage = if creases == 0 { 1.0 } else { bevels as f32 / creases as f32 };
    (histogram, coverage)
}

fn project(point: Vec3, axis: usize) -> (f32, f32) {
    match axis {
        0 => (point.z, point.y),
        1 => (point.x, point.z),
        _ => (point.x, point.y),
    }
}

fn silhouette_convexity(mesh: &Mesh, axis: usize) -> f32 {
    if mesh.triangle_count() == 0 {
        return 1.0;
    }
    let points: Vec<(f32, f32)> = mesh.positions.iter().map(|p| project(*p, axis)).collect();
    let hull = convex_hull(points.clone());
    let hull_area = polygon_area(&hull);
    if hull_area <= 1e-9 {
        return 1.0;
    }
    let (min_u, max_u, min_v, max_v) = points.iter().fold((f32::MAX, f32::MIN, f32::MAX, f32::MIN), |acc, p| {
        (acc.0.min(p.0), acc.1.max(p.0), acc.2.min(p.1), acc.3.max(p.1))
    });
    let cell_u = (max_u - min_u).max(1e-6) / SILHOUETTE_GRID as f32;
    let cell_v = (max_v - min_v).max(1e-6) / SILHOUETTE_GRID as f32;
    let mut covered = vec![false; SILHOUETTE_GRID * SILHOUETTE_GRID];
    for triangle in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(triangle).map(|p| project(p, axis));
        fill_triangle(&mut covered, a, b, c, (min_u, min_v), (cell_u, cell_v));
    }
    let area = covered.iter().filter(|c| **c).count() as f32 * cell_u * cell_v;
    (area / hull_area).clamp(0.0, 1.0)
}

fn fill_triangle(covered: &mut [bool], a: (f32, f32), b: (f32, f32), c: (f32, f32), origin: (f32, f32), cell: (f32, f32)) {
    let to_grid = |p: (f32, f32)| ((p.0 - origin.0) / cell.0, (p.1 - origin.1) / cell.1);
    let (a, b, c) = (to_grid(a), to_grid(b), to_grid(c));
    let low_x = a.0.min(b.0).min(c.0).floor().max(0.0) as usize;
    let high_x = (a.0.max(b.0).max(c.0).ceil() as usize).min(SILHOUETTE_GRID - 1);
    let low_y = a.1.min(b.1).min(c.1).floor().max(0.0) as usize;
    let high_y = (a.1.max(b.1).max(c.1).ceil() as usize).min(SILHOUETTE_GRID - 1);
    for y in low_y..=high_y {
        for x in low_x..=high_x {
            let center = (x as f32 + 0.5, y as f32 + 0.5);
            if inside_triangle(center, a, b, c) {
                covered[y * SILHOUETTE_GRID + x] = true;
            }
        }
    }
}

fn inside_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let sign = |p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)| (p1.0 - p3.0) * (p2.1 - p3.1) - (p2.0 - p3.0) * (p1.1 - p3.1);
    let (d1, d2, d3) = (sign(p, a, b), sign(p, b, c), sign(p, c, a));
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

fn convex_hull(mut points: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut hull: Vec<(f32, f32)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iterator: Box<dyn Iterator<Item = &(f32, f32)>> = if pass == 0 { Box::new(points.iter()) } else { Box::new(points.iter().rev()) };
        for point in iterator {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], *point) <= 0.0 {
                hull.pop();
            }
            hull.push(*point);
        }
        hull.pop();
    }
    hull
}

fn polygon_area(polygon: &[(f32, f32)]) -> f32 {
    let mut sum = 0.0;
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        sum += a.0 * b.1 - b.0 * a.1;
    }
    sum.abs() * 0.5
}

fn texel_density(mesh: &Mesh) -> f32 {
    let mut ratios: Vec<f32> = Vec::new();
    for triangle in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(triangle);
        let world_area = (b - a).cross(c - a).length() * 0.5;
        if world_area < 1e-9 {
            continue;
        }
        let uv = |k: usize| mesh.uvs[mesh.indices[triangle * 3 + k] as usize];
        let (p, q, r) = (uv(0), uv(1), uv(2));
        let uv_area = ((q[0] - p[0]) * (r[1] - p[1]) - (r[0] - p[0]) * (q[1] - p[1])).abs() * 0.5;
        ratios.push((uv_area / world_area).sqrt());
    }
    if ratios.is_empty() {
        return 0.0;
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ratios[ratios.len() / 2] * TEXTURE_SIZE
}
