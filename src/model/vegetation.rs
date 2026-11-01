use super::mesh::Mesh;
use super::sdf::Sdf;
use crate::math::Vec3;
use crate::random::Random;

const MIN_BRANCH_RADIUS: f32 = 0.03;

#[derive(Clone, Copy)]
pub struct TreeSpec {
    pub height: f32,
    pub trunk_radius: f32,
    pub branch_levels: usize,
    pub branches_per_level: usize,
    pub spread_degrees: f32,
    pub leaf_size: f32,
    pub leaves_per_twig: usize,
    pub seed: u64,
}

impl TreeSpec {
    pub fn oak(height: f32) -> TreeSpec {
        TreeSpec { height, trunk_radius: height * 0.035, branch_levels: 4, branches_per_level: 4, spread_degrees: 38.0, leaf_size: 0.3, leaves_per_twig: 40, seed: 7 }
    }
}

pub struct Tree {
    pub wood: Sdf,
    pub foliage: Mesh,
    pub branch_count: usize,
    pub leaf_count: usize,
    pub smallest_feature: f32,
}

struct Branch {
    start: Vec3,
    end: Vec3,
    radius: f32,
}

fn rotate_toward(direction: Vec3, spread_degrees: f32, random: &mut Random) -> Vec3 {
    let up_reference = if direction.y.abs() > 0.95 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let side = direction.cross(up_reference).normalized();
    let other = direction.cross(side).normalized();
    let around = random.range(0.0, std::f32::consts::TAU);
    let tilt = random.range(0.6, 1.0) * spread_degrees.to_radians();
    let lateral = side * around.cos() + other * around.sin();
    (direction * tilt.cos() + lateral * tilt.sin() + Vec3::new(0.0, 0.12, 0.0)).normalized()
}

fn grow(spec: &TreeSpec, random: &mut Random, start: Vec3, direction: Vec3, length: f32, radius: f32, level: usize, branches: &mut Vec<Branch>, twigs: &mut Vec<(Vec3, Vec3)>) {
    let end = start + direction * length;
    branches.push(Branch { start, end, radius: radius.max(MIN_BRANCH_RADIUS) });
    if level >= 2 {
        twigs.push((end, direction));
    }
    if level >= spec.branch_levels {
        return;
    }
    let count = if level == 0 { spec.branches_per_level + 1 } else { spec.branches_per_level.max(2) - 1 };
    for _ in 0..count {
        let child_direction = rotate_toward(direction, spec.spread_degrees, random);
        let child_length = length * random.range(0.62, 0.8);
        grow(spec, random, end, child_direction, child_length, radius * 0.62, level + 1, branches, twigs);
    }
}

pub fn build_tree(spec: &TreeSpec) -> Tree {
    let mut random = Random::new(spec.seed);
    let trunk_length = spec.height * 0.38;
    let mut branches = Vec::new();
    let mut twigs = Vec::new();
    grow(spec, &mut random, Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), trunk_length, spec.trunk_radius, 0, &mut branches, &mut twigs);

    let limbs: Vec<Sdf> = branches
        .iter()
        .map(|b| {
            let taper = b.radius * 0.62;
            Sdf::limb(b.start, b.end, [b.radius, b.radius], [taper, taper], b.radius * 0.9)
        })
        .collect();
    let flare = Sdf::cylinder(spec.trunk_radius * 1.5, spec.trunk_radius * 0.6, spec.trunk_radius * 0.5).translate(Vec3::new(0.0, spec.trunk_radius * 0.3, 0.0));
    let mut all = limbs;
    all.push(flare);
    let wood = Sdf::smooth_union_all(all, spec.trunk_radius * 0.5);

    let mut foliage = Mesh::new();
    let mut leaf_count = 0;
    for (tip, direction) in &twigs {
        for _ in 0..spec.leaves_per_twig {
            let offset = Vec3::new(random.range(-1.0, 1.0), random.range(-0.7, 1.0), random.range(-1.0, 1.0)) * (spec.height * 0.11);
            let center = *tip + *direction * (spec.height * 0.04) + offset;
            add_leaf(&mut foliage, &mut random, center, spec.leaf_size, tip.y / spec.height, (tip.y / spec.height).clamp(0.2, 1.0));
            leaf_count += 1;
        }
    }
    Tree { wood, foliage, branch_count: branches.len(), leaf_count, smallest_feature: spec.trunk_radius * 0.62f32.powi(spec.branch_levels as i32) * 2.0 }
}

pub fn add_leaf(mesh: &mut Mesh, random: &mut Random, center: Vec3, size: f32, height_fraction: f32, sway: f32) {
    let yaw = random.range(0.0, std::f32::consts::TAU);
    let pitch = random.range(-0.9, 0.9);
    let along = Vec3::new(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos());
    let across = along.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
    let across = if across.length() < 0.1 { Vec3::new(1.0, 0.0, 0.0) } else { across };
    let normal = along.cross(across).normalized();
    let tone = random.range(0.75, 1.15);
    let color = Vec3::new(0.10, 0.20, 0.05) * tone + Vec3::new(0.05, 0.04, 0.0) * height_fraction;
    let (length, width) = (size, size * 0.55);
    let tip = center + along * length;
    let base = center;
    let left = center + along * (length * 0.45) - across * (width * 0.5);
    let right = center + along * (length * 0.45) + across * (width * 0.5);
    let first_vertex = mesh.vertex_count();
    for facing in [normal, normal * -1.0] {
        let a = mesh.push_surface_vertex(base, facing, color * 0.8, [0.7, 0.0]);
        let b = mesh.push_surface_vertex(left, facing, color, [0.7, 0.0]);
        let c = mesh.push_surface_vertex(tip, facing, color * 1.1, [0.7, 0.0]);
        let d = mesh.push_surface_vertex(right, facing, color, [0.7, 0.0]);
        mesh.uvs[a as usize] = [0.5, 0.0];
        mesh.uvs[b as usize] = [0.0, 0.45];
        mesh.uvs[c as usize] = [0.5, 1.0];
        mesh.uvs[d as usize] = [1.0, 0.45];
        if facing.dot(normal) > 0.0 {
            mesh.push_triangle(a, b, c);
            mesh.push_triangle(a, c, d);
        } else {
            mesh.push_triangle(a, c, b);
            mesh.push_triangle(a, d, c);
        }
    }
    for index in first_vertex..mesh.vertex_count() {
        mesh.sway[index] = sway;
    }
}

pub struct GroundCoverSpec {
    pub half_width: f32,
    pub half_depth: f32,
    pub blades_per_square_meter: f32,
    pub blade_height: f32,
    pub seed: u64,
}

pub fn ground_cover(spec: &GroundCoverSpec, keep_clear: &dyn Fn(f32, f32) -> bool) -> Mesh {
    let mut random = Random::new(spec.seed);
    let mut mesh = Mesh::new();
    let count = (spec.half_width * 2.0 * spec.half_depth * 2.0 * spec.blades_per_square_meter) as usize;
    for _ in 0..count {
        let x = random.range(-spec.half_width, spec.half_width);
        let z = random.range(-spec.half_depth, spec.half_depth);
        if keep_clear(x, z) {
            continue;
        }
        let height = spec.blade_height * random.range(0.5, 1.3);
        let lean = Vec3::new(random.range(-0.4, 0.4), 0.0, random.range(-0.4, 0.4)) * height;
        let facing = random.range(0.0, std::f32::consts::PI);
        let half_width = Vec3::new(facing.cos(), 0.0, facing.sin()) * (spec.blade_height * 0.04);
        let base = Vec3::new(x, 0.0, z);
        let tone = random.range(0.7, 1.2);
        let color = Vec3::new(0.14, 0.26, 0.07) * tone;
        let normal = Vec3::new(-facing.sin(), 0.3, facing.cos()).normalized();
        let a = mesh.push_surface_vertex(base - half_width, normal, color * 0.6, [0.8, 0.0]);
        let b = mesh.push_surface_vertex(base + half_width, normal, color * 0.6, [0.8, 0.0]);
        let tip = mesh.push_surface_vertex(base + lean + Vec3::new(0.0, height, 0.0), normal, color * 1.2, [0.8, 0.0]);
        mesh.push_triangle(a, b, tip);
        mesh.push_triangle(a, tip, b);
        mesh.sway[tip as usize] = 1.0;
    }
    mesh
}
