use super::aov::{Aov, NO_TRIANGLE};
use super::targets::Targets;
use super::{Finding, ProbeObject, ProbeScene, Severity};
use crate::math::Vec3;
use crate::model::mesh::Mesh;
use std::collections::HashMap;

const PLANAR_DOT: f32 = 0.9998;
const HARD_EDGE_DEGREES: f32 = 60.0;
const MIN_RAW_EDGE_LENGTH: f32 = 0.05;
const WELD_PRECISION: f32 = 10_000.0;
const MIN_VISIBLE_PIXELS: usize = 150;

pub struct ObjectGeometry {
    pub planar_fraction: f32,
    pub raw_hard_edge_length: f32,
    pub extent: Vec3,
    pub area: f32,
}

#[derive(Clone, Copy, Default)]
pub struct ObjectView {
    pub pixels: usize,
    pub visible_triangles: usize,
    pub solidity: f32,
}

pub fn measure_object(mesh: &Mesh, object: &ProbeObject) -> ObjectGeometry {
    let mut area = 0.0;
    let mut planar_area = 0.0;
    let mut low = Vec3::splat(f32::MAX);
    let mut high = Vec3::splat(f32::MIN);
    for triangle in object.first_triangle..object.first_triangle + object.triangle_count {
        let corners = mesh.triangle(triangle);
        let cross = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
        let triangle_area = cross.length() * 0.5;
        area += triangle_area;
        if triangle_is_planar(mesh, triangle, cross.normalized()) {
            planar_area += triangle_area;
        }
        for corner in corners {
            low = low.min(corner);
            high = high.max(corner);
        }
    }
    ObjectGeometry { planar_fraction: planar_area / area.max(1e-9), raw_hard_edge_length: raw_hard_edge_length(mesh, object), extent: high - low, area }
}

fn raw_hard_edge_length(mesh: &Mesh, object: &ProbeObject) -> f32 {
    type Key = (i32, i32, i32);
    let weld = |p: Vec3| ((p.x * WELD_PRECISION).round() as i32, (p.y * WELD_PRECISION).round() as i32, (p.z * WELD_PRECISION).round() as i32);
    let mut edges: HashMap<(Key, Key), (f32, Vec<Vec3>)> = HashMap::new();
    for triangle in object.first_triangle..object.first_triangle + object.triangle_count {
        let corners = mesh.triangle(triangle);
        let normal = (corners[1] - corners[0]).cross(corners[2] - corners[0]).normalized();
        for k in 0..3 {
            let (from, to) = (corners[k], corners[(k + 1) % 3]);
            let (a, b) = (weld(from), weld(to));
            let entry = edges.entry((a.min(b), a.max(b))).or_insert((0.0, Vec::new()));
            entry.0 = (to - from).length();
            entry.1.push(normal);
        }
    }
    edges
        .values()
        .filter(|(length, normals)| normals.len() == 2 && *length >= MIN_RAW_EDGE_LENGTH)
        .filter(|(_, normals)| normals[0].dot(normals[1]).clamp(-1.0, 1.0).acos().to_degrees() > HARD_EDGE_DEGREES)
        .map(|(length, _)| *length)
        .sum()
}

fn triangle_is_planar(mesh: &Mesh, triangle: usize, face_normal: Vec3) -> bool {
    (0..3).all(|k| mesh.normals[mesh.indices[triangle * 3 + k] as usize].normalized().dot(face_normal).abs() > PLANAR_DOT)
}

pub fn measure_view(scene: &ProbeScene, aov: &Aov) -> Vec<ObjectView> {
    let mut views = vec![ObjectView::default(); scene.objects.len()];
    let mut seen = vec![false; scene.mesh.triangle_count()];
    let mut hull_points: Vec<Vec<(f32, f32)>> = vec![Vec::new(); scene.objects.len()];
    let mut row_bounds: Vec<Option<(f32, f32)>> = vec![None; scene.objects.len()];
    for y in 0..aov.height {
        row_bounds.iter_mut().for_each(|b| *b = None);
        for x in 0..aov.width {
            let triangle = aov.triangle[y * aov.width + x];
            if triangle == NO_TRIANGLE {
                continue;
            }
            let owner = scene.object_of_triangle[triangle as usize] as usize;
            views[owner].pixels += 1;
            if !seen[triangle as usize] {
                seen[triangle as usize] = true;
                views[owner].visible_triangles += 1;
            }
            let x = x as f32;
            row_bounds[owner] = Some(match row_bounds[owner] {
                Some((low, high)) => (low.min(x), high.max(x + 1.0)),
                None => (x, x + 1.0),
            });
        }
        for (owner, bounds) in row_bounds.iter().enumerate() {
            if let Some((low, high)) = bounds {
                hull_points[owner].push((*low, y as f32));
                hull_points[owner].push((*high, y as f32 + 1.0));
            }
        }
    }
    for (owner, view) in views.iter_mut().enumerate() {
        let hull = convex_hull_area(&mut hull_points[owner]);
        view.solidity = if hull > 0.0 { (view.pixels as f32 / hull).min(1.0) } else { 0.0 };
    }
    views
}

fn convex_hull_area(points: &mut Vec<(f32, f32)>) -> f32 {
    if points.len() < 3 {
        return 0.0;
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut hull: Vec<(f32, f32)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let sequence: Vec<(f32, f32)> = if pass == 0 { points.clone() } else { points.iter().rev().copied().collect() };
        for point in sequence {
            while hull.len() >= start + 2 && turn(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0 {
                hull.pop();
            }
            hull.push(point);
        }
        hull.pop();
    }
    let mut twice_area = 0.0;
    for k in 0..hull.len() {
        let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
        twice_area += a.0 * b.1 - b.0 * a.1;
    }
    twice_area.abs() * 0.5
}

fn turn(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

pub struct ObjectReport {
    pub geometry: ObjectGeometry,
    pub best_view: ObjectView,
}

pub fn findings(scene: &ProbeScene, reports: &[ObjectReport], targets: &Targets) -> Vec<Finding> {
    let mut out = Vec::new();
    for (object, report) in scene.objects.iter().zip(reports) {
        let geometry = &report.geometry;
        if geometry.extent.max_element() >= targets.box_like_min_extent && geometry.raw_hard_edge_length > targets.raw_hard_edge_length_max {
            out.push(Finding::new(
                Severity::Error,
                &format!("object {}", object.name),
                "unbeveled_hard_edge_length_m",
                geometry.raw_hard_edge_length,
                format!("<= {:.2}", targets.raw_hard_edge_length_max),
                "long sharp edges with no bevel: round them (radius >= 0.02 m) or build from curved kit parts",
            ));
        }
        let view = report.best_view;
        if view.pixels < MIN_VISIBLE_PIXELS || view.visible_triangles == 0 {
            continue;
        }
        let pixels_per_triangle = view.pixels as f32 / view.visible_triangles as f32;
        if geometry.planar_fraction < 0.9 && pixels_per_triangle > targets.faceted_pixels_per_triangle_max {
            out.push(Finding::new(
                Severity::Warn,
                &format!("object {}", object.name),
                "pixels_per_visible_triangle",
                pixels_per_triangle,
                format!("<= {:.0} on curved surfaces", targets.faceted_pixels_per_triangle_max),
                "curved surface is faceted: reduce mesh cell size",
            ));
        }
        if pixels_per_triangle < targets.overdense_pixels_per_triangle_min {
            out.push(Finding::new(
                Severity::Warn,
                &format!("object {}", object.name),
                "pixels_per_visible_triangle",
                pixels_per_triangle,
                format!(">= {:.1}", targets.overdense_pixels_per_triangle_min),
                "triangles are smaller than a pixel: increase cell size or add LOD",
            ));
        }
    }
    out
}
