use crate::random::hash_to_unit;

fn lattice(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mixed = (x as u32)
        .wrapping_mul(73856093)
        ^ (y as u32).wrapping_mul(19349663)
        ^ (z as u32).wrapping_mul(83492791)
        ^ seed.wrapping_mul(2654435761);
    hash_to_unit(mixed) * 2.0 - 1.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn value_noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
    let (ix, iy, iz) = (fx as i32, fy as i32, fz as i32);
    let (tx, ty, tz) = (smooth(x - fx), smooth(y - fy), smooth(z - fz));
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let corner = |dx: i32, dy: i32, dz: i32| lattice(ix + dx, iy + dy, iz + dz, seed);
    let along_x = |dy: i32, dz: i32| mix(corner(0, dy, dz), corner(1, dy, dz), tx);
    let along_y = |dz: i32| mix(along_x(0, dz), along_x(1, dz), ty);
    mix(along_y(0), along_y(1), tz)
}

pub fn fractal_noise(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut total = 0.0;
    let mut weight = 0.0;
    for octave in 0..octaves {
        total += amplitude * value_noise(x * frequency, y * frequency, z * frequency, seed.wrapping_add(octave));
        weight += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total / weight
}
