use super::material::{Material, Surface, SurfaceSample};
use crate::math::Vec3;
use std::collections::HashMap;

pub const DEFAULT_SURFACE: [f32; 2] = [0.8, 0.0];

#[derive(Clone, Default)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub colors: Vec<Vec3>,
    pub surfaces: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub struct TopologyReport {
    pub watertight: bool,
    pub open_edges: usize,
    pub nonmanifold_edges: usize,
    pub components: usize,
    pub max_edge_angle_degrees: f32,
    pub sharp_edge_fraction: f32,
}

const WELD_PRECISION: f32 = 10_000.0;
const SHARP_EDGE_DEGREES: f32 = 35.0;

impl Mesh {
    pub fn new() -> Mesh {
        Mesh::default()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn push_vertex(&mut self, position: Vec3, normal: Vec3, color: Vec3) -> u32 {
        self.push_surface_vertex(position, normal, color, DEFAULT_SURFACE)
    }

    pub fn push_surface_vertex(&mut self, position: Vec3, normal: Vec3, color: Vec3, surface: [f32; 2]) -> u32 {
        self.positions.push(position);
        self.normals.push(normal);
        self.colors.push(color);
        self.surfaces.push(surface);
        self.uvs.push([0.0, 0.0]);
        (self.positions.len() - 1) as u32
    }

    pub fn set_surface(&mut self, roughness: f32, metallic: f32) {
        self.surfaces.iter_mut().for_each(|surface| *surface = [roughness, metallic]);
    }

    pub fn apply_surface(&mut self, surface: &dyn Surface) {
        for index in 0..self.positions.len() {
            let sample = SurfaceSample { position: self.positions[index], normal: self.normals[index], curvature: 0.0 };
            let material = surface.evaluate(&sample);
            self.colors[index] = material.albedo;
            self.surfaces[index] = [material.roughness, material.metallic];
        }
    }

    pub fn apply_material_map(&mut self, map: &dyn Fn(&SurfaceSample, Vec3) -> Material) {
        for index in 0..self.positions.len() {
            let sample = SurfaceSample { position: self.positions[index], normal: self.normals[index], curvature: 0.0 };
            let material = map(&sample, self.colors[index]);
            self.colors[index] = material.albedo;
            self.surfaces[index] = [material.roughness, material.metallic];
        }
    }

    pub fn generate_uvs(&mut self, meters_per_tile: f32) {
        let source = std::mem::take(self);
        let mut chart_vertices: HashMap<(u32, usize), u32> = HashMap::new();
        for triangle in 0..source.triangle_count() {
            let [a, b, c] = source.triangle(triangle);
            let normal = (b - a).cross(c - a);
            let axis = dominant_axis(normal);
            for corner in 0..3 {
                let original = source.indices[triangle * 3 + corner];
                let index = *chart_vertices.entry((original, axis)).or_insert_with(|| {
                    let position = source.positions[original as usize];
                    let created = self.push_surface_vertex(
                        position,
                        source.normals[original as usize],
                        source.colors[original as usize],
                        source.surfaces[original as usize],
                    );
                    self.uvs[created as usize] = chart_uv(position, axis, meters_per_tile);
                    created
                });
                self.indices.push(index);
            }
        }
    }

    pub fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend_from_slice(&[a, b, c]);
    }

    pub fn triangle(&self, index: usize) -> [Vec3; 3] {
        let i = index * 3;
        [
            self.positions[self.indices[i] as usize],
            self.positions[self.indices[i + 1] as usize],
            self.positions[self.indices[i + 2] as usize],
        ]
    }

    pub fn append(&mut self, other: &Mesh) {
        let offset = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.colors.extend_from_slice(&other.colors);
        self.surfaces.extend_from_slice(&other.surfaces);
        self.uvs.extend_from_slice(&other.uvs);
        self.indices.extend(other.indices.iter().map(|i| i + offset));
    }

    pub fn translate(&mut self, by: Vec3) {
        self.positions.iter_mut().for_each(|p| *p = *p + by);
    }

    pub fn rotate_y(&mut self, degrees: f32) {
        let (s, c) = degrees.to_radians().sin_cos();
        let rotate = |v: Vec3| Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z);
        self.positions.iter_mut().for_each(|p| *p = rotate(*p));
        self.normals.iter_mut().for_each(|n| *n = rotate(*n));
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        for p in &self.positions {
            low = low.min(*p);
            high = high.max(*p);
        }
        if self.positions.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        (low, high)
    }

    pub fn surface_area(&self) -> f32 {
        (0..self.triangle_count())
            .map(|i| {
                let [a, b, c] = self.triangle(i);
                (b - a).cross(c - a).length() * 0.5
            })
            .sum()
    }

    pub fn volume(&self) -> f32 {
        (0..self.triangle_count())
            .map(|i| {
                let [a, b, c] = self.triangle(i);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    pub fn centroid_of_volume(&self) -> Vec3 {
        let mut weighted = Vec3::ZERO;
        let mut total = 0.0;
        for i in 0..self.triangle_count() {
            let [a, b, c] = self.triangle(i);
            let signed_volume = a.dot(b.cross(c)) / 6.0;
            weighted += (a + b + c) * (0.25 * signed_volume);
            total += signed_volume;
        }
        if total.abs() < 1e-12 {
            let (low, high) = self.bounds();
            return (low + high) * 0.5;
        }
        weighted / total
    }

    pub fn add_flat_triangle(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Vec3) {
        let normal = (b - a).cross(c - a).normalized();
        let ia = self.push_vertex(a, normal, color);
        let ib = self.push_vertex(b, normal, color);
        let ic = self.push_vertex(c, normal, color);
        self.push_triangle(ia, ib, ic);
    }

    pub fn add_quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Vec3) {
        self.add_flat_triangle(a, b, c, color);
        self.add_flat_triangle(a, c, d, color);
    }

    pub fn add_box(&mut self, center: Vec3, size: Vec3, color: Vec3) {
        let h = size * 0.5;
        let corner = |sx: f32, sy: f32, sz: f32| center + Vec3::new(sx * h.x, sy * h.y, sz * h.z);
        let faces = [
            [corner(-1.0, -1.0, 1.0), corner(1.0, -1.0, 1.0), corner(1.0, 1.0, 1.0), corner(-1.0, 1.0, 1.0)],
            [corner(1.0, -1.0, -1.0), corner(-1.0, -1.0, -1.0), corner(-1.0, 1.0, -1.0), corner(1.0, 1.0, -1.0)],
            [corner(-1.0, -1.0, -1.0), corner(-1.0, -1.0, 1.0), corner(-1.0, 1.0, 1.0), corner(-1.0, 1.0, -1.0)],
            [corner(1.0, -1.0, 1.0), corner(1.0, -1.0, -1.0), corner(1.0, 1.0, -1.0), corner(1.0, 1.0, 1.0)],
            [corner(-1.0, 1.0, 1.0), corner(1.0, 1.0, 1.0), corner(1.0, 1.0, -1.0), corner(-1.0, 1.0, -1.0)],
            [corner(-1.0, -1.0, -1.0), corner(1.0, -1.0, -1.0), corner(1.0, -1.0, 1.0), corner(-1.0, -1.0, 1.0)],
        ];
        for [a, b, c, d] in faces {
            self.add_quad(a, b, c, d, color);
        }
    }

    pub fn add_grid(
        &mut self,
        origin: Vec3,
        u_axis: Vec3,
        v_axis: Vec3,
        u_breaks: &[f32],
        v_breaks: &[f32],
        facing: Vec3,
        color: Vec3,
        keep_cell: impl Fn(f32, f32) -> bool,
    ) {
        let mut normal = u_axis.cross(v_axis).normalized();
        let flipped = normal.dot(facing) < 0.0;
        if flipped {
            normal = -normal;
        }
        let columns = u_breaks.len();
        let rows = v_breaks.len();
        let mut indices = vec![0u32; columns * rows];
        for (row, v) in v_breaks.iter().enumerate() {
            for (column, u) in u_breaks.iter().enumerate() {
                indices[row * columns + column] = self.push_vertex(origin + u_axis * *u + v_axis * *v, normal, color);
            }
        }
        for row in 0..rows - 1 {
            for column in 0..columns - 1 {
                let center_u = (u_breaks[column] + u_breaks[column + 1]) * 0.5;
                let center_v = (v_breaks[row] + v_breaks[row + 1]) * 0.5;
                if !keep_cell(center_u, center_v) {
                    continue;
                }
                let a = indices[row * columns + column];
                let b = indices[row * columns + column + 1];
                let c = indices[(row + 1) * columns + column + 1];
                let d = indices[(row + 1) * columns + column];
                if flipped {
                    self.push_triangle(a, c, b);
                    self.push_triangle(a, d, c);
                } else {
                    self.push_triangle(a, b, c);
                    self.push_triangle(a, c, d);
                }
            }
        }
    }

    pub fn topology_report(&self) -> TopologyReport {
        let mut vertex_ids: HashMap<(i32, i32, i32), u32> = HashMap::new();
        let welded: Vec<u32> = self
            .positions
            .iter()
            .map(|p| {
                let key = (
                    (p.x * WELD_PRECISION).round() as i32,
                    (p.y * WELD_PRECISION).round() as i32,
                    (p.z * WELD_PRECISION).round() as i32,
                );
                let next = vertex_ids.len() as u32;
                *vertex_ids.entry(key).or_insert(next)
            })
            .collect();

        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        let mut parents: Vec<u32> = (0..vertex_ids.len() as u32).collect();
        for triangle in 0..self.triangle_count() {
            let ids = [
                welded[self.indices[triangle * 3] as usize],
                welded[self.indices[triangle * 3 + 1] as usize],
                welded[self.indices[triangle * 3 + 2] as usize],
            ];
            if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
                continue;
            }
            for k in 0..3 {
                let (a, b) = (ids[k], ids[(k + 1) % 3]);
                edge_faces.entry((a.min(b), a.max(b))).or_default().push(triangle);
                union_sets(&mut parents, a, b);
            }
        }

        let used_vertices: std::collections::HashSet<u32> = edge_faces.keys().flat_map(|(a, b)| [*a, *b]).collect();
        let components = used_vertices
            .iter()
            .map(|v| find_root(&mut parents, *v))
            .collect::<std::collections::HashSet<_>>()
            .len();

        let mut open_edges = 0;
        let mut nonmanifold_edges = 0;
        let mut sharp = 0;
        let mut shared = 0;
        let mut max_angle = 0.0f32;
        for faces in edge_faces.values() {
            match faces.len() {
                1 => open_edges += 1,
                2 => {
                    shared += 1;
                    let n0 = self.face_normal(faces[0]);
                    let n1 = self.face_normal(faces[1]);
                    let angle = n0.dot(n1).clamp(-1.0, 1.0).acos().to_degrees();
                    max_angle = max_angle.max(angle);
                    if angle > SHARP_EDGE_DEGREES {
                        sharp += 1;
                    }
                }
                _ => nonmanifold_edges += 1,
            }
        }
        TopologyReport {
            watertight: open_edges == 0 && nonmanifold_edges == 0 && !edge_faces.is_empty(),
            open_edges,
            nonmanifold_edges,
            components,
            max_edge_angle_degrees: max_angle,
            sharp_edge_fraction: if shared == 0 { 0.0 } else { sharp as f32 / shared as f32 },
        }
    }

    pub fn edge_dihedral_angles(&self) -> Vec<f32> {
        let mut vertex_ids: HashMap<(i32, i32, i32), u32> = HashMap::new();
        let welded: Vec<u32> = self
            .positions
            .iter()
            .map(|p| {
                let key = ((p.x * WELD_PRECISION).round() as i32, (p.y * WELD_PRECISION).round() as i32, (p.z * WELD_PRECISION).round() as i32);
                let next = vertex_ids.len() as u32;
                *vertex_ids.entry(key).or_insert(next)
            })
            .collect();
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for triangle in 0..self.triangle_count() {
            let ids = [welded[self.indices[triangle * 3] as usize], welded[self.indices[triangle * 3 + 1] as usize], welded[self.indices[triangle * 3 + 2] as usize]];
            if ids[0] == ids[1] || ids[1] == ids[2] || ids[0] == ids[2] {
                continue;
            }
            for k in 0..3 {
                let (a, b) = (ids[k], ids[(k + 1) % 3]);
                edge_faces.entry((a.min(b), a.max(b))).or_default().push(triangle);
            }
        }
        edge_faces
            .values()
            .filter(|faces| faces.len() == 2)
            .map(|faces| self.face_normal(faces[0]).dot(self.face_normal(faces[1])).clamp(-1.0, 1.0).acos().to_degrees())
            .collect()
    }

    fn face_normal(&self, triangle: usize) -> Vec3 {
        let [a, b, c] = self.triangle(triangle);
        (b - a).cross(c - a).normalized()
    }

    pub fn write_obj(&self, path: &str) -> Result<(), String> {
        use std::fmt::Write;
        let mut text = String::new();
        for p in &self.positions {
            writeln!(text, "v {:.5} {:.5} {:.5}", p.x, p.y, p.z).unwrap();
        }
        for n in &self.normals {
            writeln!(text, "vn {:.4} {:.4} {:.4}", n.x, n.y, n.z).unwrap();
        }
        for t in self.indices.chunks(3) {
            writeln!(text, "f {0}//{0} {1}//{1} {2}//{2}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
        }
        std::fs::write(path, text).map_err(|error| format!("{}: {}", path, error))
    }
}

fn find_root(parents: &mut [u32], mut node: u32) -> u32 {
    while parents[node as usize] != node {
        parents[node as usize] = parents[parents[node as usize] as usize];
        node = parents[node as usize];
    }
    node
}

fn union_sets(parents: &mut [u32], a: u32, b: u32) {
    let (root_a, root_b) = (find_root(parents, a), find_root(parents, b));
    if root_a != root_b {
        parents[root_a as usize] = root_b;
    }
}

fn dominant_axis(normal: Vec3) -> usize {
    let (x, y, z) = (normal.x.abs(), normal.y.abs(), normal.z.abs());
    if y >= x && y >= z {
        1
    } else if x >= z {
        0
    } else {
        2
    }
}

fn chart_uv(position: Vec3, axis: usize, meters_per_tile: f32) -> [f32; 2] {
    let (u, v) = match axis {
        0 => (position.z, position.y),
        1 => (position.x, position.z),
        _ => (position.x, position.y),
    };
    [u / meters_per_tile, v / meters_per_tile]
}
