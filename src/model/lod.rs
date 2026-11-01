use super::mesh::Mesh;
use crate::math::Vec3;
use std::collections::HashMap;

const SOLVE_DETERMINANT_LIMIT: f32 = 1e-7;
const SEARCH_STEPS: usize = 14;

#[derive(Clone, Default)]
struct Quadric {
    xx: f32,
    xy: f32,
    xz: f32,
    yy: f32,
    yz: f32,
    zz: f32,
    bx: f32,
    by: f32,
    bz: f32,
    constant: f32,
}

impl Quadric {
    fn add_plane(&mut self, normal: Vec3, offset: f32, weight: f32) {
        self.xx += weight * normal.x * normal.x;
        self.xy += weight * normal.x * normal.y;
        self.xz += weight * normal.x * normal.z;
        self.yy += weight * normal.y * normal.y;
        self.yz += weight * normal.y * normal.z;
        self.zz += weight * normal.z * normal.z;
        self.bx += weight * normal.x * offset;
        self.by += weight * normal.y * offset;
        self.bz += weight * normal.z * offset;
        self.constant += weight * offset * offset;
    }

    fn optimal_point(&self) -> Option<Vec3> {
        let determinant = self.xx * (self.yy * self.zz - self.yz * self.yz) - self.xy * (self.xy * self.zz - self.yz * self.xz)
            + self.xz * (self.xy * self.yz - self.yy * self.xz);
        let scale = (self.xx + self.yy + self.zz).max(1e-12);
        if determinant.abs() < SOLVE_DETERMINANT_LIMIT * scale * scale * scale {
            return None;
        }
        let (rx, ry, rz) = (-self.bx, -self.by, -self.bz);
        let x = rx * (self.yy * self.zz - self.yz * self.yz) - self.xy * (ry * self.zz - self.yz * rz) + self.xz * (ry * self.yz - self.yy * rz);
        let y = self.xx * (ry * self.zz - self.yz * rz) - rx * (self.xy * self.zz - self.yz * self.xz) + self.xz * (self.xy * rz - ry * self.xz);
        let z = self.xx * (self.yy * rz - ry * self.yz) - self.xy * (self.xy * rz - ry * self.xz) + rx * (self.xy * self.yz - self.yy * self.xz);
        Some(Vec3::new(x / determinant, y / determinant, z / determinant))
    }
}

#[derive(Default)]
struct Cluster {
    quadric: Quadric,
    position_sum: Vec3,
    normal_sum: Vec3,
    color_sum: Vec3,
    surface_sum: [f32; 2],
    uv_sum: [f32; 2],
    count: f32,
    origin: Vec3,
}

pub fn simplify(mesh: &Mesh, cell: f32) -> Mesh {
    let key = |p: Vec3| ((p.x / cell).floor() as i32, (p.y / cell).floor() as i32, (p.z / cell).floor() as i32);
    let mut ids: HashMap<(i32, i32, i32), usize> = HashMap::new();
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut vertex_cluster = Vec::with_capacity(mesh.vertex_count());
    for index in 0..mesh.vertex_count() {
        let position = mesh.positions[index];
        let cell_key = key(position);
        let id = *ids.entry(cell_key).or_insert_with(|| {
            clusters.push(Cluster {
                origin: Vec3::new(cell_key.0 as f32 * cell, cell_key.1 as f32 * cell, cell_key.2 as f32 * cell),
                ..Cluster::default()
            });
            clusters.len() - 1
        });
        let cluster = &mut clusters[id];
        cluster.position_sum = cluster.position_sum + position;
        cluster.normal_sum = cluster.normal_sum + mesh.normals[index];
        cluster.color_sum = cluster.color_sum + mesh.colors[index];
        cluster.surface_sum[0] += mesh.surfaces[index][0];
        cluster.surface_sum[1] += mesh.surfaces[index][1];
        cluster.uv_sum[0] += mesh.uvs[index][0];
        cluster.uv_sum[1] += mesh.uvs[index][1];
        cluster.count += 1.0;
        vertex_cluster.push(id);
    }
    for triangle in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(triangle);
        let cross = (b - a).cross(c - a);
        let area = cross.length() * 0.5;
        if area < 1e-12 {
            continue;
        }
        let normal = cross.normalized();
        let offset = -normal.dot(a);
        for corner in 0..3 {
            let id = vertex_cluster[mesh.indices[triangle * 3 + corner] as usize];
            clusters[id].quadric.add_plane(normal, offset, area);
        }
    }

    let mut result = Mesh::new();
    for cluster in &clusters {
        let mean = cluster.position_sum * (1.0 / cluster.count);
        let low = cluster.origin - Vec3::splat(cell * 0.25);
        let high = cluster.origin + Vec3::splat(cell * 1.25);
        let position = match cluster.quadric.optimal_point() {
            Some(p) if p.x >= low.x && p.x <= high.x && p.y >= low.y && p.y <= high.y && p.z >= low.z && p.z <= high.z => p,
            _ => mean,
        };
        let id = result.push_surface_vertex(
            position,
            cluster.normal_sum.normalized(),
            cluster.color_sum * (1.0 / cluster.count),
            [cluster.surface_sum[0] / cluster.count, cluster.surface_sum[1] / cluster.count],
        );
        result.uvs[id as usize] = [cluster.uv_sum[0] / cluster.count, cluster.uv_sum[1] / cluster.count];
    }
    for triangle in 0..mesh.triangle_count() {
        let ids = [0, 1, 2].map(|k| vertex_cluster[mesh.indices[triangle * 3 + k] as usize] as u32);
        if ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2] {
            result.push_triangle(ids[0], ids[1], ids[2]);
        }
    }
    result
}

pub struct Lod {
    pub level: usize,
    pub mesh: Mesh,
    pub triangle_ratio: f32,
    pub cluster_cell: f32,
}

pub fn build_lod_chain(mesh: &Mesh, triangle_ratios: &[f32]) -> Vec<Lod> {
    let (low, high) = mesh.bounds();
    let diagonal = (high - low).length().max(1e-6);
    let mut chain = vec![Lod { level: 0, mesh: mesh.clone(), triangle_ratio: 1.0, cluster_cell: 0.0 }];
    for (offset, ratio) in triangle_ratios.iter().enumerate() {
        let target = (mesh.triangle_count() as f32 * ratio) as usize;
        let (mut small, mut large) = (diagonal * 1e-4, diagonal * 0.5);
        for _ in 0..SEARCH_STEPS {
            let middle = (small * large).sqrt();
            if simplify(mesh, middle).triangle_count() > target {
                small = middle;
            } else {
                large = middle;
            }
        }
        let simplified = simplify(mesh, large);
        chain.push(Lod {
            level: offset + 1,
            triangle_ratio: simplified.triangle_count() as f32 / mesh.triangle_count().max(1) as f32,
            mesh: simplified,
            cluster_cell: large,
        });
    }
    chain
}
