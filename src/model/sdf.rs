use crate::math::Vec3;

pub type Bounds = (Vec3, Vec3);

#[derive(Clone, Copy)]
pub struct Rotation {
    rows: [Vec3; 3],
}

impl Rotation {
    fn about_y(degrees: f32) -> Rotation {
        let (s, c) = degrees.to_radians().sin_cos();
        Rotation { rows: [Vec3::new(c, 0.0, s), Vec3::new(0.0, 1.0, 0.0), Vec3::new(-s, 0.0, c)] }
    }
    fn about_x(degrees: f32) -> Rotation {
        let (s, c) = degrees.to_radians().sin_cos();
        Rotation { rows: [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, c, -s), Vec3::new(0.0, s, c)] }
    }
    fn about_z(degrees: f32) -> Rotation {
        let (s, c) = degrees.to_radians().sin_cos();
        Rotation { rows: [Vec3::new(c, -s, 0.0), Vec3::new(s, c, 0.0), Vec3::new(0.0, 0.0, 1.0)] }
    }
    pub fn apply(&self, p: Vec3) -> Vec3 {
        Vec3::new(self.rows[0].dot(p), self.rows[1].dot(p), self.rows[2].dot(p))
    }
    pub fn apply_inverse(&self, p: Vec3) -> Vec3 {
        self.rows[0] * p.x + self.rows[1] * p.y + self.rows[2] * p.z
    }
}

#[derive(Clone)]
pub struct Limb {
    start: Vec3,
    end: Vec3,
    axis: Vec3,
    side: Vec3,
    depth: Vec3,
    length: f32,
    half_start: [f32; 2],
    half_end: [f32; 2],
    radius: f32,
}

impl Limb {
    pub fn new(start: Vec3, end: Vec3, half_start: [f32; 2], half_end: [f32; 2], radius: f32) -> Limb {
        let span = end - start;
        let length = span.length();
        let axis = span.normalized();
        let reference = if axis.y.abs() > 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
        let side = reference.cross(axis).normalized();
        let depth = axis.cross(side);
        Limb { start, end, axis, side, depth, length, half_start, half_end, radius }
    }

    fn distance(&self, p: Vec3) -> f32 {
        let offset = p - self.start;
        let along = offset.dot(self.axis);
        let fraction = (along / self.length).clamp(0.0, 1.0);
        let half_side = self.half_start[0] + (self.half_end[0] - self.half_start[0]) * fraction;
        let half_depth = self.half_start[1] + (self.half_end[1] - self.half_start[1]) * fraction;
        let q = Vec3::new(
            offset.dot(self.side).abs() - half_side + self.radius,
            (along - self.length * 0.5).abs() - self.length * 0.5 + self.radius,
            offset.dot(self.depth).abs() - half_depth + self.radius,
        );
        q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - self.radius
    }

    fn bounds(&self) -> Bounds {
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        for (center, half) in [(self.start, self.half_start), (self.end, self.half_end)] {
            for side_sign in [-1.0, 1.0] {
                for depth_sign in [-1.0, 1.0] {
                    let corner = center + self.side * (half[0] * side_sign) + self.depth * (half[1] * depth_sign);
                    low = low.min(corner);
                    high = high.max(corner);
                }
            }
        }
        (low, high)
    }
}

#[derive(Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Clone)]
pub enum Sdf {
    Sphere { radius: f32 },
    RoundBox { half: Vec3, radius: f32 },
    Cylinder { radius: f32, half_height: f32, rounding: f32 },
    Limb(Limb),
    Union(Box<Sdf>, Box<Sdf>),
    SmoothUnion(Box<Sdf>, Box<Sdf>, f32),
    Subtract(Box<Sdf>, Box<Sdf>),
    SmoothSubtract(Box<Sdf>, Box<Sdf>, f32),
    Intersect(Box<Sdf>, Box<Sdf>),
    Offset(Box<Sdf>, f32),
    Shell(Box<Sdf>, f32),
    Translate(Box<Sdf>, Vec3),
    Rotate(Box<Sdf>, Rotation),
    Mirror(Box<Sdf>, Axis),
    Torus { major_radius: f32, minor_radius: f32 },
    HalfSpace { normal: Vec3, offset: f32 },
    Extrude { points: Vec<[f32; 2]>, half_depth: f32, rounding: f32 },
    Revolve { points: Vec<[f32; 2]> },
    LinearArrayX(Box<Sdf>, usize, f32),
    RadialArray(Box<Sdf>, usize),
    Scale(Box<Sdf>, f32),
    ChamferUnion(Box<Sdf>, Box<Sdf>, f32),
    Displace { child: Box<Sdf>, amplitude: f32, frequency: f32, seed: u32 },
}

impl Sdf {
    pub fn sphere(radius: f32) -> Sdf {
        Sdf::Sphere { radius }
    }

    pub fn round_box(size: Vec3, radius: f32) -> Sdf {
        let half = size * 0.5;
        Sdf::RoundBox { half, radius: radius.min(half.x).min(half.y).min(half.z) }
    }

    pub fn cylinder(radius: f32, height: f32, rounding: f32) -> Sdf {
        Sdf::Cylinder { radius, half_height: height * 0.5, rounding: rounding.min(radius).min(height * 0.5) }
    }

    pub fn limb(start: Vec3, end: Vec3, half_start: [f32; 2], half_end: [f32; 2], radius: f32) -> Sdf {
        Sdf::Limb(Limb::new(start, end, half_start, half_end, radius))
    }

    pub fn union(self, other: Sdf) -> Sdf {
        Sdf::Union(Box::new(self), Box::new(other))
    }

    pub fn smooth_union(self, other: Sdf, blend: f32) -> Sdf {
        Sdf::SmoothUnion(Box::new(self), Box::new(other), blend)
    }

    pub fn subtract(self, other: Sdf) -> Sdf {
        Sdf::Subtract(Box::new(self), Box::new(other))
    }

    pub fn smooth_subtract(self, other: Sdf, blend: f32) -> Sdf {
        Sdf::SmoothSubtract(Box::new(self), Box::new(other), blend)
    }

    pub fn intersect(self, other: Sdf) -> Sdf {
        Sdf::Intersect(Box::new(self), Box::new(other))
    }

    pub fn offset(self, distance: f32) -> Sdf {
        Sdf::Offset(Box::new(self), distance)
    }

    pub fn shell(self, thickness: f32) -> Sdf {
        Sdf::Shell(Box::new(self), thickness)
    }

    pub fn translate(self, by: Vec3) -> Sdf {
        Sdf::Translate(Box::new(self), by)
    }

    pub fn rotate_y(self, degrees: f32) -> Sdf {
        Sdf::Rotate(Box::new(self), Rotation::about_y(degrees))
    }

    pub fn rotate_x(self, degrees: f32) -> Sdf {
        Sdf::Rotate(Box::new(self), Rotation::about_x(degrees))
    }

    pub fn rotate_z(self, degrees: f32) -> Sdf {
        Sdf::Rotate(Box::new(self), Rotation::about_z(degrees))
    }

    pub fn mirror(self, axis: Axis) -> Sdf {
        Sdf::Mirror(Box::new(self), axis)
    }

    pub fn torus(major_radius: f32, minor_radius: f32) -> Sdf {
        Sdf::Torus { major_radius, minor_radius }
    }

    pub fn half_space(normal: Vec3, offset: f32) -> Sdf {
        Sdf::HalfSpace { normal: normal.normalized(), offset }
    }

    pub fn extrude(points: &[[f32; 2]], depth: f32, rounding: f32) -> Sdf {
        Sdf::Extrude { points: points.to_vec(), half_depth: depth * 0.5, rounding: rounding.min(depth * 0.5) }
    }

    pub fn revolve(points: &[[f32; 2]]) -> Sdf {
        Sdf::Revolve { points: points.to_vec() }
    }

    pub fn linear_array_x(self, count: usize, step: f32) -> Sdf {
        Sdf::LinearArrayX(Box::new(self), count.max(1), step)
    }

    pub fn radial_array(self, count: usize) -> Sdf {
        Sdf::RadialArray(Box::new(self), count.max(1))
    }

    pub fn scale(self, factor: f32) -> Sdf {
        Sdf::Scale(Box::new(self), factor)
    }

    pub fn chamfer_union(self, other: Sdf, size: f32) -> Sdf {
        Sdf::ChamferUnion(Box::new(self), Box::new(other), size)
    }

    pub fn displace(self, amplitude: f32, frequency: f32, seed: u32) -> Sdf {
        Sdf::Displace { child: Box::new(self), amplitude, frequency, seed }
    }

    pub fn smooth_union_all(items: Vec<Sdf>, blend: f32) -> Sdf {
        let mut iterator = items.into_iter();
        let first = iterator.next().expect("smooth_union_all needs at least one shape");
        iterator.fold(first, |accumulated, item| accumulated.smooth_union(item, blend))
    }

    pub fn distance(&self, p: Vec3) -> f32 {
        match self {
            Sdf::Sphere { radius } => p.length() - radius,
            Sdf::RoundBox { half, radius } => {
                let q = p.abs() - *half + Vec3::splat(*radius);
                q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - radius
            }
            Sdf::Cylinder { radius, half_height, rounding } => {
                let horizontal = (p.x * p.x + p.z * p.z).sqrt() - radius + rounding;
                let vertical = p.y.abs() - half_height + rounding;
                let outside = (horizontal.max(0.0).powi(2) + vertical.max(0.0).powi(2)).sqrt();
                outside + horizontal.max(vertical).min(0.0) - rounding
            }
            Sdf::Limb(limb) => limb.distance(p),
            Sdf::Union(a, b) => a.distance(p).min(b.distance(p)),
            Sdf::SmoothUnion(a, b, k) => smooth_min(a.distance(p), b.distance(p), *k),
            Sdf::Subtract(a, b) => a.distance(p).max(-b.distance(p)),
            Sdf::SmoothSubtract(a, b, k) => {
                let (base, cut) = (a.distance(p), b.distance(p));
                let h = (0.5 - 0.5 * (base + cut) / k).clamp(0.0, 1.0);
                base + (-cut - base) * h + k * h * (1.0 - h)
            }
            Sdf::Intersect(a, b) => a.distance(p).max(b.distance(p)),
            Sdf::Offset(a, d) => a.distance(p) - d,
            Sdf::Shell(a, t) => a.distance(p).abs() - t * 0.5,
            Sdf::Translate(a, by) => a.distance(p - *by),
            Sdf::Rotate(a, rotation) => a.distance(rotation.apply_inverse(p)),
            Sdf::Torus { major_radius, minor_radius } => {
                let ring = (p.x * p.x + p.z * p.z).sqrt() - major_radius;
                (ring * ring + p.y * p.y).sqrt() - minor_radius
            }
            Sdf::HalfSpace { normal, offset } => p.dot(*normal) - offset,
            Sdf::Extrude { points, half_depth, rounding } => {
                let profile = polygon_distance(points, [p.x, p.y]) + rounding;
                let depth = p.z.abs() - half_depth + rounding;
                profile.max(depth).min(0.0) + (profile.max(0.0).powi(2) + depth.max(0.0).powi(2)).sqrt() - rounding
            }
            Sdf::Revolve { points } => polygon_distance(points, [(p.x * p.x + p.z * p.z).sqrt(), p.y]),
            Sdf::LinearArrayX(child, count, step) => {
                let index = (p.x / step).round().clamp(0.0, (*count - 1) as f32);
                child.distance(Vec3::new(p.x - index * step, p.y, p.z))
            }
            Sdf::RadialArray(child, count) => {
                let sector = std::f32::consts::TAU / *count as f32;
                let angle = p.z.atan2(p.x);
                let folded = (angle + sector * 0.5).rem_euclid(sector) - sector * 0.5;
                let radius = (p.x * p.x + p.z * p.z).sqrt();
                child.distance(Vec3::new(radius * folded.cos(), p.y, radius * folded.sin()))
            }
            Sdf::Scale(child, factor) => child.distance(p / *factor) * factor,
            Sdf::ChamferUnion(a, b, size) => {
                let (da, db) = (a.distance(p), b.distance(p));
                da.min(db).min((da - size + db) * std::f32::consts::FRAC_1_SQRT_2)
            }
            Sdf::Displace { child, amplitude, frequency, seed } => {
                child.distance(p) + amplitude * crate::noise::value_noise(p.x * frequency, p.y * frequency, p.z * frequency, *seed)
            }
            Sdf::Mirror(a, axis) => {
                let mirrored = match axis {
                    Axis::X => Vec3::new(p.x.abs(), p.y, p.z),
                    Axis::Y => Vec3::new(p.x, p.y.abs(), p.z),
                    Axis::Z => Vec3::new(p.x, p.y, p.z.abs()),
                };
                a.distance(mirrored)
            }
        }
    }

    pub fn bounds(&self) -> Bounds {
        match self {
            Sdf::Sphere { radius } => (Vec3::splat(-radius), Vec3::splat(*radius)),
            Sdf::RoundBox { half, .. } => (-*half, *half),
            Sdf::Cylinder { radius, half_height, .. } => {
                (Vec3::new(-radius, -half_height, -radius), Vec3::new(*radius, *half_height, *radius))
            }
            Sdf::Limb(limb) => limb.bounds(),
            Sdf::Union(a, b) | Sdf::SmoothUnion(a, b, _) => {
                let (a, b) = (a.bounds(), b.bounds());
                let blend = match self {
                    Sdf::SmoothUnion(_, _, k) => *k * 0.25,
                    _ => 0.0,
                };
                (a.0.min(b.0) - Vec3::splat(blend), a.1.max(b.1) + Vec3::splat(blend))
            }
            Sdf::Subtract(a, _) | Sdf::SmoothSubtract(a, _, _) => a.bounds(),
            Sdf::Intersect(a, b) => {
                let (a, b) = (a.bounds(), b.bounds());
                (a.0.max(b.0), a.1.min(b.1))
            }
            Sdf::Offset(a, d) => {
                let (low, high) = a.bounds();
                let grow = d.max(0.0);
                (low - Vec3::splat(grow), high + Vec3::splat(grow))
            }
            Sdf::Shell(a, t) => {
                let (low, high) = a.bounds();
                (low - Vec3::splat(t * 0.5), high + Vec3::splat(t * 0.5))
            }
            Sdf::Translate(a, by) => {
                let (low, high) = a.bounds();
                (low + *by, high + *by)
            }
            Sdf::Rotate(a, rotation) => {
                let (low, high) = a.bounds();
                let mut new_low = Vec3::splat(f32::MAX);
                let mut new_high = Vec3::splat(f32::MIN);
                for x in [low.x, high.x] {
                    for y in [low.y, high.y] {
                        for z in [low.z, high.z] {
                            let corner = rotation.apply(Vec3::new(x, y, z));
                            new_low = new_low.min(corner);
                            new_high = new_high.max(corner);
                        }
                    }
                }
                (new_low, new_high)
            }
            Sdf::Torus { major_radius, minor_radius } => {
                let reach = major_radius + minor_radius;
                (Vec3::new(-reach, -minor_radius, -reach), Vec3::new(reach, *minor_radius, reach))
            }
            Sdf::HalfSpace { .. } => (Vec3::splat(-1000.0), Vec3::splat(1000.0)),
            Sdf::Extrude { points, half_depth, .. } => {
                let (low, high) = profile_bounds(points);
                (Vec3::new(low[0], low[1], -half_depth), Vec3::new(high[0], high[1], *half_depth))
            }
            Sdf::Revolve { points } => {
                let (low, high) = profile_bounds(points);
                let reach = high[0].max(low[0].abs());
                (Vec3::new(-reach, low[1], -reach), Vec3::new(reach, high[1], reach))
            }
            Sdf::LinearArrayX(child, count, step) => {
                let (low, high) = child.bounds();
                (low, high + Vec3::new(step * (*count - 1) as f32, 0.0, 0.0))
            }
            Sdf::RadialArray(child, _) => {
                let (low, high) = child.bounds();
                let reach = low.x.abs().max(high.x.abs()).max(low.z.abs()).max(high.z.abs()) * std::f32::consts::SQRT_2;
                (Vec3::new(-reach, low.y, -reach), Vec3::new(reach, high.y, reach))
            }
            Sdf::Scale(child, factor) => {
                let (low, high) = child.bounds();
                (low * *factor, high * *factor)
            }
            Sdf::ChamferUnion(a, b, size) => {
                let (a, b) = (a.bounds(), b.bounds());
                (a.0.min(b.0) - Vec3::splat(*size), a.1.max(b.1) + Vec3::splat(*size))
            }
            Sdf::Displace { child, amplitude, .. } => {
                let (low, high) = child.bounds();
                (low - Vec3::splat(*amplitude), high + Vec3::splat(*amplitude))
            }
            Sdf::Mirror(a, axis) => {
                let (low, high) = a.bounds();
                let extent = |l: f32, h: f32| l.abs().max(h.abs());
                match axis {
                    Axis::X => {
                        let e = extent(low.x, high.x);
                        (Vec3::new(-e, low.y, low.z), Vec3::new(e, high.y, high.z))
                    }
                    Axis::Y => {
                        let e = extent(low.y, high.y);
                        (Vec3::new(low.x, -e, low.z), Vec3::new(high.x, e, high.z))
                    }
                    Axis::Z => {
                        let e = extent(low.z, high.z);
                        (Vec3::new(low.x, low.y, -e), Vec3::new(high.x, high.y, e))
                    }
                }
            }
        }
    }

    pub fn gradient(&self, p: Vec3, step: f32) -> Vec3 {
        let dx = Vec3::new(step, 0.0, 0.0);
        let dy = Vec3::new(0.0, step, 0.0);
        let dz = Vec3::new(0.0, 0.0, step);
        Vec3::new(
            self.distance(p + dx) - self.distance(p - dx),
            self.distance(p + dy) - self.distance(p - dy),
            self.distance(p + dz) - self.distance(p - dz),
        )
        .normalized()
    }
}

fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

fn profile_bounds(points: &[[f32; 2]]) -> ([f32; 2], [f32; 2]) {
    let mut low = [f32::MAX; 2];
    let mut high = [f32::MIN; 2];
    for point in points {
        for axis in 0..2 {
            low[axis] = low[axis].min(point[axis]);
            high[axis] = high[axis].max(point[axis]);
        }
    }
    (low, high)
}

fn polygon_distance(points: &[[f32; 2]], p: [f32; 2]) -> f32 {
    let count = points.len();
    let first = points[0];
    let mut nearest = (p[0] - first[0]).powi(2) + (p[1] - first[1]).powi(2);
    let mut sign = 1.0;
    for i in 0..count {
        let j = (i + count - 1) % count;
        let (vi, vj) = (points[i], points[j]);
        let edge = [vj[0] - vi[0], vj[1] - vi[1]];
        let offset = [p[0] - vi[0], p[1] - vi[1]];
        let along = ((offset[0] * edge[0] + offset[1] * edge[1]) / (edge[0] * edge[0] + edge[1] * edge[1]).max(1e-12)).clamp(0.0, 1.0);
        let closest = [offset[0] - edge[0] * along, offset[1] - edge[1] * along];
        nearest = nearest.min(closest[0] * closest[0] + closest[1] * closest[1]);
        let crosses = [p[1] >= vi[1], p[1] < vj[1], edge[0] * offset[1] > edge[1] * offset[0]];
        if crosses.iter().all(|c| *c) || crosses.iter().all(|c| !*c) {
            sign = -sign;
        }
    }
    sign * nearest.sqrt()
}
