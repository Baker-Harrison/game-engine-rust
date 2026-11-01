use super::mesh::Mesh;
use super::sdf::Sdf;
use crate::math::Vec3;
use crate::parallel::for_each_chunk;

const BLOCK_SIZE: usize = 8;
const SKIP_SAFETY_FACTOR: f32 = 1.4;
const NO_VERTEX: u32 = u32::MAX;

struct Grid {
    origin: Vec3,
    cell: f32,
    nx: usize,
    ny: usize,
    nz: usize,
}

impl Grid {
    fn sample_index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.ny + y) * self.nx + x
    }

    fn cell_index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * (self.ny - 1) + y) * (self.nx - 1) + x
    }

    fn position(&self, x: usize, y: usize, z: usize) -> Vec3 {
        self.origin + Vec3::new(x as f32, y as f32, z as f32) * self.cell
    }
}

pub struct SurfaceSample {
    pub position: Vec3,
    pub normal: Vec3,
    pub curvature: f32,
}

pub type Paint<'a> = &'a (dyn Fn(&SurfaceSample) -> Vec3 + Sync);

pub fn mesh_sdf(sdf: &Sdf, cell: f32, color: Vec3) -> Mesh {
    mesh_sdf_painted(sdf, cell, &|_| color)
}

pub fn mesh_sdf_painted(sdf: &Sdf, cell: f32, paint: Paint) -> Mesh {
    let (low, high) = sdf.bounds();
    if high.x <= low.x || high.y <= low.y || high.z <= low.z {
        return Mesh::new();
    }
    let padding = Vec3::splat(cell * 3.0);
    let origin = low - padding;
    let size = (high + padding) - origin;
    let count = |extent: f32| (extent / cell).ceil() as usize + 2;
    let grid = Grid { origin, cell, nx: count(size.x), ny: count(size.y), nz: count(size.z) };

    let field = sample_field(sdf, &grid);
    let (cell_vertices, vertex_cells) = place_vertices(&grid, &field);
    let refined = refine_vertices(sdf, &grid, &vertex_cells, paint);

    let mut mesh = Mesh::new();
    for (position, normal, color) in &refined {
        mesh.push_vertex(*position, *normal, *color);
    }
    emit_quads(&grid, &field, &cell_vertices, &mut mesh);
    mesh
}

fn sample_field(sdf: &Sdf, grid: &Grid) -> Vec<f32> {
    let blocks = |n: usize| n.div_ceil(BLOCK_SIZE);
    let (bx, by, bz) = (blocks(grid.nx), blocks(grid.ny), blocks(grid.nz));
    let mut coarse = vec![0.0f32; bx * by * bz];
    let block_extent = BLOCK_SIZE as f32 * grid.cell;
    let skip_distance = block_extent * 0.866 * SKIP_SAFETY_FACTOR;

    for_each_chunk(&mut coarse, 1, |start, chunk| {
        for (offset, value) in chunk.iter_mut().enumerate() {
            let index = start + offset;
            let (x, y, z) = (index % bx, (index / bx) % by, index / (bx * by));
            let center = grid.origin + (Vec3::new(x as f32, y as f32, z as f32) + Vec3::splat(0.5)) * block_extent;
            *value = sdf.distance(center);
        }
    });

    let mut field = vec![0.0f32; grid.nx * grid.ny * grid.nz];
    let layer = grid.nx * grid.ny;
    let coarse = &coarse;
    for_each_chunk(&mut field, layer, |start, chunk| {
        for (offset, value) in chunk.iter_mut().enumerate() {
            let index = start + offset;
            let (x, y, z) = (index % grid.nx, (index / grid.nx) % grid.ny, index / layer);
            let block = coarse[((z / BLOCK_SIZE) * by + y / BLOCK_SIZE) * bx + x / BLOCK_SIZE];
            *value = if block.abs() > skip_distance { block } else { sdf.distance(grid.position(x, y, z)) };
        }
    });
    field
}

fn place_vertices(grid: &Grid, field: &[f32]) -> (Vec<u32>, Vec<Vec3>) {
    let mut cell_vertices = vec![NO_VERTEX; (grid.nx - 1) * (grid.ny - 1) * (grid.nz - 1)];
    let mut vertex_cells = Vec::new();
    const EDGES: [([usize; 3], [usize; 3]); 12] = [
        ([0, 0, 0], [1, 0, 0]),
        ([0, 1, 0], [1, 1, 0]),
        ([0, 0, 1], [1, 0, 1]),
        ([0, 1, 1], [1, 1, 1]),
        ([0, 0, 0], [0, 1, 0]),
        ([1, 0, 0], [1, 1, 0]),
        ([0, 0, 1], [0, 1, 1]),
        ([1, 0, 1], [1, 1, 1]),
        ([0, 0, 0], [0, 0, 1]),
        ([1, 0, 0], [1, 0, 1]),
        ([0, 1, 0], [0, 1, 1]),
        ([1, 1, 0], [1, 1, 1]),
    ];
    for z in 0..grid.nz - 1 {
        for y in 0..grid.ny - 1 {
            for x in 0..grid.nx - 1 {
                let corner = |c: [usize; 3]| field[grid.sample_index(x + c[0], y + c[1], z + c[2])];
                let mut inside = 0;
                for c in 0..8 {
                    if corner([c & 1, (c >> 1) & 1, (c >> 2) & 1]) < 0.0 {
                        inside += 1;
                    }
                }
                if inside == 0 || inside == 8 {
                    continue;
                }
                let mut sum = Vec3::ZERO;
                let mut crossings = 0.0;
                for (from, to) in EDGES {
                    let (d0, d1) = (corner(from), corner(to));
                    if (d0 < 0.0) == (d1 < 0.0) {
                        continue;
                    }
                    let t = d0 / (d0 - d1);
                    let p0 = Vec3::new(from[0] as f32, from[1] as f32, from[2] as f32);
                    let p1 = Vec3::new(to[0] as f32, to[1] as f32, to[2] as f32);
                    sum += p0.lerp(p1, t);
                    crossings += 1.0;
                }
                let local = sum / crossings;
                let position = grid.position(x, y, z) + local * grid.cell;
                cell_vertices[grid.cell_index(x, y, z)] = vertex_cells.len() as u32;
                vertex_cells.push(position);
            }
        }
    }
    (cell_vertices, vertex_cells)
}

fn refine_vertices(sdf: &Sdf, grid: &Grid, positions: &[Vec3], paint: Paint) -> Vec<(Vec3, Vec3, Vec3)> {
    let mut refined: Vec<(Vec3, Vec3, Vec3)> = positions.iter().map(|p| (*p, Vec3::ZERO, Vec3::ZERO)).collect();
    let step = grid.cell * 0.5;
    for_each_chunk(&mut refined, 1, |_, chunk| {
        for (position, normal, color) in chunk.iter_mut() {
            let distance = sdf.distance(*position).clamp(-grid.cell, grid.cell);
            let first_normal = sdf.gradient(*position, step);
            *position = *position - first_normal * distance;
            *normal = sdf.gradient(*position, step);
            let sample = SurfaceSample { position: *position, normal: *normal, curvature: curvature_at(sdf, *position, grid.cell) };
            *color = paint(&sample);
        }
    });
    refined
}

fn curvature_at(sdf: &Sdf, position: Vec3, step: f32) -> f32 {
    let offsets = [
        Vec3::new(step, 0.0, 0.0),
        Vec3::new(-step, 0.0, 0.0),
        Vec3::new(0.0, step, 0.0),
        Vec3::new(0.0, -step, 0.0),
        Vec3::new(0.0, 0.0, step),
        Vec3::new(0.0, 0.0, -step),
    ];
    let around: f32 = offsets.iter().map(|offset| sdf.distance(position + *offset)).sum();
    (around - 6.0 * sdf.distance(position)) / (step * step)
}

fn emit_quads(grid: &Grid, field: &[f32], cell_vertices: &[u32], mesh: &mut Mesh) {
    for z in 1..grid.nz - 1 {
        for y in 1..grid.ny - 1 {
            for x in 1..grid.nx - 1 {
                let here = field[grid.sample_index(x, y, z)] < 0.0;
                let offsets: [([usize; 3], [[usize; 3]; 4]); 3] = [
                    ([1, 0, 0], [[0, 1, 1], [0, 0, 1], [0, 0, 0], [0, 1, 0]]),
                    ([0, 1, 0], [[1, 0, 1], [0, 0, 1], [0, 0, 0], [1, 0, 0]]),
                    ([0, 0, 1], [[1, 1, 0], [0, 1, 0], [0, 0, 0], [1, 0, 0]]),
                ];
                for (axis, cells) in offsets {
                    let neighbor = field[grid.sample_index(x + axis[0], y + axis[1], z + axis[2])] < 0.0;
                    if here == neighbor {
                        continue;
                    }
                    let mut ids = [0u32; 4];
                    let mut complete = true;
                    for (slot, delta) in cells.iter().enumerate() {
                        let (cx, cy, cz) = (x - delta[0], y - delta[1], z - delta[2]);
                        let id = cell_vertices[grid.cell_index(cx, cy, cz)];
                        if id == NO_VERTEX {
                            complete = false;
                            break;
                        }
                        ids[slot] = id;
                    }
                    if complete {
                        emit_oriented_quad(mesh, ids);
                    }
                }
            }
        }
    }
}

fn emit_oriented_quad(mesh: &mut Mesh, ids: [u32; 4]) {
    let p = |i: usize| mesh.positions[ids[i] as usize];
    let face_normal = (p(1) - p(0)).cross(p(2) - p(0));
    let average_normal = (0..4).fold(Vec3::ZERO, |sum, i| sum + mesh.normals[ids[i] as usize]);
    if face_normal.dot(average_normal) >= 0.0 {
        mesh.push_triangle(ids[0], ids[1], ids[2]);
        mesh.push_triangle(ids[0], ids[2], ids[3]);
    } else {
        mesh.push_triangle(ids[0], ids[2], ids[1]);
        mesh.push_triangle(ids[0], ids[3], ids[2]);
    }
}
