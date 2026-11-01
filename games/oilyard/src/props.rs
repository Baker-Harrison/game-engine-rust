use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::material::SurfaceSample;
use engine::model::mesher::{mesh_sdf, mesh_sdf_painted};
use engine::model::sdf::Sdf;
use engine::noise::fractal_noise;

pub const SAND: Vec3 = Vec3::new(0.78, 0.64, 0.42);
pub const CONCRETE: Vec3 = Vec3::new(0.62, 0.60, 0.56);
pub const STEEL: Vec3 = Vec3::new(0.34, 0.37, 0.40);
pub const STEEL_DARK: Vec3 = Vec3::new(0.18, 0.20, 0.22);
pub const SAFETY_YELLOW: Vec3 = Vec3::new(0.92, 0.72, 0.10);
pub const RUST: Vec3 = Vec3::new(0.52, 0.27, 0.13);
pub const BARREL_RED: Vec3 = Vec3::new(0.68, 0.18, 0.12);
pub const BARREL_BLUE: Vec3 = Vec3::new(0.14, 0.30, 0.55);
pub const TANK_WHITE: Vec3 = Vec3::new(0.84, 0.82, 0.76);
pub const CONTAINER_COLORS: [Vec3; 5] = [
    Vec3::new(0.62, 0.20, 0.14),
    Vec3::new(0.16, 0.34, 0.50),
    Vec3::new(0.24, 0.45, 0.28),
    Vec3::new(0.78, 0.58, 0.16),
    Vec3::new(0.40, 0.40, 0.42),
];
pub const WOOD: Vec3 = Vec3::new(0.52, 0.38, 0.22);
pub const OFFICE_WALL: Vec3 = Vec3::new(0.80, 0.74, 0.62);

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn weathered(base: Vec3, seed: u32, amount: f32) -> impl Fn(&SurfaceSample) -> Vec3 {
    move |sample: &SurfaceSample| {
        let p = sample.position;
        let dirt = fractal_noise(p.x * 5.0, p.y * 5.0, p.z * 5.0, 3, seed);
        let streak = fractal_noise(p.x * 9.0, p.y * 0.8, p.z * 9.0, 2, seed + 5);
        let low = (1.0 - (p.y / 2.5).clamp(0.0, 1.0)) * 0.25;
        let wear = (sample.curvature * 0.02).clamp(0.0, 1.0);
        let shade = 1.0 - amount * (0.16 * dirt + 0.14 * streak + low * 0.7);
        (base * shade).lerp(RUST, wear * 0.5 * amount)
    }
}

pub fn add_tessellated_box(mesh: &mut Mesh, center: Vec3, size: Vec3, color: Vec3, cell: f32) {
    let half = size * 0.5;
    let breaks = |length: f32| -> Vec<f32> {
        let count = ((length / cell).ceil() as usize).max(1);
        (0..=count).map(|i| -length * 0.5 + length * i as f32 / count as f32).collect()
    };
    let faces = [
        (v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0), v(0.0, 0.0, 1.0), half.x, size.y, size.z),
        (v(-1.0, 0.0, 0.0), v(0.0, 1.0, 0.0), v(0.0, 0.0, 1.0), half.x, size.y, size.z),
        (v(0.0, 1.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), half.y, size.x, size.z),
        (v(0.0, -1.0, 0.0), v(1.0, 0.0, 0.0), v(0.0, 0.0, 1.0), half.y, size.x, size.z),
        (v(0.0, 0.0, 1.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0), half.z, size.x, size.y),
        (v(0.0, 0.0, -1.0), v(1.0, 0.0, 0.0), v(0.0, 1.0, 0.0), half.z, size.x, size.y),
    ];
    for (normal, u_axis, v_axis, offset, u_length, v_length) in faces {
        let origin = center + normal * offset;
        mesh.add_grid(origin, u_axis, v_axis, &breaks(u_length), &breaks(v_length), normal, color, |_, _| true);
    }
}

pub fn barrel(color: Vec3, seed: u32) -> Mesh {
    let body = Sdf::cylinder(0.29, 0.88, 0.035);
    let rib = |y: f32| Sdf::torus(0.285, 0.018).translate(v(0.0, y, 0.0));
    let dish = Sdf::cylinder(0.2, 0.08, 0.0).translate(v(0.0, 0.44, 0.0));
    let sdf = body.chamfer_union(rib(-0.2), 0.01).chamfer_union(rib(0.2), 0.01).subtract(dish).translate(v(0.0, 0.44, 0.0));
    mesh_sdf_painted(&sdf, 0.07, &weathered(color, seed, 0.8))
}

pub fn tank(radius: f32, height: f32, seed: u32) -> Mesh {
    let cell = if radius > 4.0 { 0.28 } else { 0.25 };
    let shoulder = radius * 0.92;
    let profile = [
        [0.0, 0.0],
        [radius, 0.0],
        [radius, height],
        [shoulder, height + 0.35],
        [radius * 0.55, height + 0.8],
        [radius * 0.12, height + 1.0],
        [0.0, height + 1.0],
    ];
    let body = Sdf::revolve(&profile);
    let vent = Sdf::cylinder(0.35, 0.5, 0.05).translate(v(radius * 0.3, height + 1.1, 0.0));
    let sdf = body.chamfer_union(vent, 0.08);
    let mut mesh = mesh_sdf_painted(&sdf, cell, &|sample: &SurfaceSample| {
        let p = sample.position;
        let band = ((p.y / 2.2).fract() < 0.06) as i32 as f32;
        let base = weathered(TANK_WHITE, seed, 1.0)(sample);
        let rust_run = fractal_noise(p.x * 2.0, p.y * 0.35, p.z * 2.0, 3, seed + 9);
        let top = (p.y / (height + 1.0)).clamp(0.0, 1.0);
        let rusty = (rust_run - 0.62).max(0.0) * 1.6 * top;
        base.lerp(RUST, rusty.min(0.8)) * (1.0 - band * 0.2)
    });
    let ladder_x = radius + 0.04;
    for rung in 0..((height / 0.33) as usize) {
        mesh.add_box(v(ladder_x, 0.4 + rung as f32 * 0.33, 0.0), v(0.08, 0.04, 0.5), STEEL_DARK);
    }
    mesh.add_box(v(ladder_x, height * 0.5, 0.26), v(0.06, height, 0.05), STEEL_DARK);
    mesh.add_box(v(ladder_x, height * 0.5, -0.26), v(0.06, height, 0.05), STEEL_DARK);
    mesh
}

pub fn container(length: f32, color: Vec3, seed: u32) -> Mesh {
    let width = 2.44;
    let height = 2.59;
    let mut mesh = Mesh::new();
    let jitter = 0.9 + 0.1 * engine::random::hash_to_unit(seed);
    let body_color = color * jitter;
    add_tessellated_box(&mut mesh, v(0.0, height * 0.5, 0.0), v(length, height, width), body_color, 0.8);
    let ribs = (length / 0.3) as usize;
    let rib_color = body_color * 0.78;
    for i in 0..ribs {
        let x = -length * 0.5 + 0.25 + i as f32 * (length - 0.5) / ribs as f32;
        for side in [-1.0, 1.0] {
            mesh.add_box(v(x, height * 0.5, side * (width * 0.5 + 0.015)), v(0.12, height - 0.35, 0.03), rib_color);
        }
    }
    for corner_x in [-1.0f32, 1.0] {
        for corner_z in [-1.0f32, 1.0] {
            mesh.add_box(v(corner_x * (length * 0.5 - 0.09), height * 0.5, corner_z * (width * 0.5 - 0.09)), v(0.2, height + 0.04, 0.2), STEEL_DARK);
        }
    }
    for rail_y in [0.06, height - 0.06] {
        mesh.add_box(v(0.0, rail_y, 0.0), v(length + 0.04, 0.12, width + 0.04), STEEL_DARK);
    }
    for side in [-1.0f32, 1.0] {
        for bar in [-0.55f32, 0.55] {
            mesh.add_box(v(side * (length * 0.5 + 0.04), height * 0.5, bar), v(0.05, height - 0.3, 0.06), STEEL);
        }
        mesh.add_box(v(side * (length * 0.5 + 0.03), height * 0.5, 0.0), v(0.04, 0.12, width - 0.3), STEEL);
    }
    mesh
}

pub fn pipe_run(length: f32, radius: f32, color: Vec3) -> Mesh {
    let pipe = Sdf::cylinder(radius, length * 0.5, radius * 0.1).rotate_z(90.0);
    let flange = |x: f32| Sdf::cylinder(radius * 1.25, 0.04, 0.01).rotate_z(90.0).translate(v(x, 0.0, 0.0));
    let mut sdf = pipe;
    let mut x = -length * 0.5 + 1.6;
    while x < length * 0.5 - 0.5 {
        sdf = sdf.chamfer_union(flange(x), 0.02);
        x += 2.4;
    }
    mesh_sdf_painted(&sdf.translate(v(0.0, radius, 0.0)), 0.14, &weathered(color, 7, 0.7))
}

pub fn pipe_elbow(radius: f32, bend: f32, color: Vec3) -> Mesh {
    let path = [v(0.0, 0.0, 0.0), v(bend * 0.8, 0.0, 0.0), v(bend, 0.0, bend * 0.2), v(bend, 0.0, bend)];
    let sizes = [[radius, radius]; 4];
    let sdf = Sdf::sweep(&path, &sizes, radius * 0.9, 6, v(0.0, 1.0, 0.0));
    mesh_sdf(&sdf.translate(v(0.0, radius, 0.0)), 0.1, color)
}

pub fn derrick(height: f32) -> Mesh {
    let base = 2.4;
    let top = 0.5;
    let leg = |x: f32, z: f32| {
        Sdf::limb(v(x * base, 0.0, z * base), v(x * top, height, z * top), [0.2, 0.2], [0.11, 0.11], 0.01)
    };
    let mut sdf = leg(1.0, 1.0).union(leg(-1.0, 1.0)).union(leg(1.0, -1.0)).union(leg(-1.0, -1.0));
    let levels = 7;
    for level in 1..levels {
        let t = level as f32 / levels as f32;
        let y = t * height;
        let half = base + (top - base) * t;
        let brace = |a: Vec3, b: Vec3| Sdf::limb(a, b, [0.04, 0.04], [0.04, 0.04], 0.005);
        let corners = [v(half, y, half), v(-half, y, half), v(-half, y, -half), v(half, y, -half)];
        for i in 0..4 {
            sdf = sdf.union(brace(corners[i], corners[(i + 1) % 4]));
        }
        let t2 = (level + 1) as f32 / levels as f32;
        let half2 = base + (top - base) * t2;
        let y2 = t2 * height;
        if level + 1 <= levels {
            sdf = sdf.union(brace(v(half, y, half), v(-half2, y2, half2)));
            sdf = sdf.union(brace(v(half, y, -half), v(half2, y2, half2)));
            sdf = sdf.union(brace(v(-half, y, -half), v(half2, y2, -half2)));
            sdf = sdf.union(brace(v(-half, y, half), v(-half2, y2, -half2)));
        }
    }
    let crown = Sdf::round_box(v(1.4, 0.5, 1.4), 0.05).translate(v(0.0, height + 0.25, 0.0));
    mesh_sdf_painted(&sdf.union(crown), 0.11, &weathered(Vec3::new(0.72, 0.30, 0.12), 3, 0.5))
}

pub fn pump_jack() -> Mesh {
    let base = Sdf::round_box(v(2.6, 0.4, 1.4), 0.05).translate(v(0.0, 0.2, 0.0));
    let post = |z: f32| Sdf::limb(v(-0.2, 0.4, z), v(0.3, 2.9, z), [0.09, 0.1], [0.07, 0.08], 0.01);
    let beam = Sdf::limb(v(-1.9, 3.5, 0.0), v(1.9, 2.5, 0.0), [0.14, 0.2], [0.14, 0.26], 0.02);
    let head = Sdf::extrude(&[[1.9, 2.1], [2.4, 2.1], [2.5, 2.9], [1.9, 3.1]], 0.2, 0.03);
    let counterweight = Sdf::cylinder(0.75, 0.12, 0.02).rotate_x(90.0).translate(v(-1.3, 1.3, 0.35));
    let crank = Sdf::limb(v(-1.3, 1.3, 0.35), v(-1.9, 3.3, 0.0), [0.05, 0.05], [0.05, 0.05], 0.01);
    let sdf = base
        .union(post(0.5))
        .union(post(-0.5))
        .union(beam)
        .union(head)
        .union(counterweight)
        .union(crank);
    mesh_sdf_painted(&sdf, 0.1, &weathered(Vec3::new(0.78, 0.45, 0.12), 4, 0.6))
}

pub fn lamp_post(height: f32) -> Mesh {
    let pole = Sdf::limb(v(0.0, 0.0, 0.0), v(0.0, height, 0.0), [0.08, 0.08], [0.05, 0.05], 0.01);
    let arm = Sdf::limb(v(0.0, height, 0.0), v(0.8, height + 0.1, 0.0), [0.04, 0.04], [0.04, 0.04], 0.005);
    let head = Sdf::round_box(v(0.5, 0.1, 0.25), 0.03).translate(v(0.85, height + 0.08, 0.0));
    mesh_sdf(&pole.union(arm).union(head), 0.07, STEEL)
}

pub fn prism(profile: &[[f32; 2]], length: f32, color: Vec3) -> Mesh {
    let mut mesh = Mesh::new();
    let half = length * 0.5;
    let point = |p: [f32; 2], z: f32| v(p[0], p[1], z);
    for i in 0..profile.len() {
        let (a, b) = (profile[i], profile[(i + 1) % profile.len()]);
        mesh.add_quad(point(a, -half), point(b, -half), point(b, half), point(a, half), color);
    }
    for i in 1..profile.len() - 1 {
        mesh.add_flat_triangle(point(profile[0], half), point(profile[i], half), point(profile[i + 1], half), color);
        mesh.add_flat_triangle(point(profile[0], -half), point(profile[i + 1], -half), point(profile[i], -half), color);
    }
    mesh
}

pub fn jersey_barrier() -> Mesh {
    let profile = [[-0.3, 0.0], [0.3, 0.0], [0.3, 0.2], [0.14, 0.8], [-0.14, 0.8], [-0.3, 0.2]];
    let mut mesh = prism(&profile, 3.0, CONCRETE);
    mesh.rotate_y(90.0);
    mesh
}

pub fn crate_box(size: f32, seed: u32) -> Mesh {
    let jitter = 0.88 + 0.12 * engine::random::hash_to_unit(seed);
    let mut mesh = Mesh::new();
    add_tessellated_box(&mut mesh, v(0.0, size * 0.5, 0.0), v(size, size, size), WOOD * jitter, 0.4);
    let slat = size * 0.12;
    for level in 0..3 {
        let y = size * (0.16 + 0.34 * level as f32);
        for (dx, dz) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let along = if dx != 0.0 { v(0.03, slat, size * 1.01) } else { v(size * 1.01, slat, 0.03) };
            mesh.add_box(v(dx * size * 0.5, y, dz * size * 0.5), along, WOOD * jitter * 0.72);
        }
    }
    for (dx, dz) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        mesh.add_box(v(dx * size * 0.5, size * 0.5, dz * size * 0.5), v(0.07, size * 1.01, 0.07), WOOD * jitter * 0.6);
    }
    mesh
}

pub fn sandbag_wall(length: f32) -> Mesh {
    let count = (length / 0.45) as usize;
    let mut mesh = Mesh::new();
    for row in 0..3 {
        for i in 0..count {
            let offset = if row % 2 == 0 { 0.0 } else { 0.22 };
            let x = -length * 0.5 + 0.22 + i as f32 * 0.45 + offset;
            if x > length * 0.5 - 0.1 {
                continue;
            }
            let tint = 0.9 + 0.1 * ((i * 7 + row * 3) % 5) as f32 / 4.0;
            let bag = Sdf::round_box(v(0.42, 0.18, 0.3), 0.07).translate(v(x, 0.1 + row as f32 * 0.17, 0.0));
            mesh.append(&mesh_sdf(&bag, 0.07, Vec3::new(0.66, 0.58, 0.40) * tint));
        }
    }
    mesh
}
