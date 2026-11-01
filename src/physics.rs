use crate::math::Vec3;
use std::collections::HashMap;

const EPSILON: f32 = 1.0e-4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }

    pub fn from_center(center: Vec3, size: Vec3) -> Aabb {
        let half = size * 0.5;
        Aabb { min: center - half, max: center + half }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x < other.max.x - EPSILON
            && self.max.x > other.min.x + EPSILON
            && self.min.y < other.max.y - EPSILON
            && self.max.y > other.min.y + EPSILON
            && self.min.z < other.max.z - EPSILON
            && self.max.z > other.min.z + EPSILON
    }

    pub fn contains_point(&self, p: Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y && p.z >= self.min.z && p.z <= self.max.z
    }

    pub fn expanded(&self, amount: f32) -> Aabb {
        let grow = Vec3::new(amount, amount, amount);
        Aabb { min: self.min - grow, max: self.max + grow }
    }

    pub fn translated(&self, delta: Vec3) -> Aabb {
        Aabb { min: self.min + delta, max: self.max + delta }
    }

    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb { min: self.min.min(other.min), max: self.max.max(other.max) }
    }
}

fn component(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

fn with_component(v: Vec3, axis: usize, value: f32) -> Vec3 {
    match axis {
        0 => Vec3::new(value, v.y, v.z),
        1 => Vec3::new(v.x, value, v.z),
        _ => Vec3::new(v.x, v.y, value),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RayHit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub collider: usize,
}

pub fn ray_aabb(origin: Vec3, direction: Vec3, aabb: &Aabb, max_distance: f32) -> Option<(f32, Vec3)> {
    let mut t_near = 0.0f32;
    let mut t_far = max_distance;
    let mut normal = Vec3::ZERO;
    for axis in 0..3 {
        let o = component(origin, axis);
        let d = component(direction, axis);
        let (lo, hi) = (component(aabb.min, axis), component(aabb.max, axis));
        if d.abs() < 1.0e-9 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (mut t0, mut t1) = ((lo - o) / d, (hi - o) / d);
        let mut face = -1.0;
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
            face = 1.0;
        }
        if t0 > t_near {
            t_near = t0;
            normal = with_component(Vec3::ZERO, axis, face);
        }
        t_far = t_far.min(t1);
        if t_near > t_far {
            return None;
        }
    }
    Some((t_near, normal))
}

pub fn ray_sphere(origin: Vec3, direction: Vec3, center: Vec3, radius: f32, max_distance: f32) -> Option<f32> {
    let offset = origin - center;
    let b = offset.dot(direction);
    let c = offset.dot(offset) - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = -b - root;
    let far = -b + root;
    let t = if near >= 0.0 { near } else { far };
    if t >= 0.0 && t <= max_distance {
        Some(t)
    } else {
        None
    }
}

const GRID_CELL: f32 = 4.0;

#[derive(Clone, Default)]
pub struct CollisionWorld {
    pub boxes: Vec<Aabb>,
    grid: HashMap<(i32, i32), Vec<usize>>,
    indexed: bool,
}

fn cell_range(min: f32, max: f32) -> std::ops::RangeInclusive<i32> {
    (min / GRID_CELL).floor() as i32..=(max / GRID_CELL).floor() as i32
}

impl CollisionWorld {
    pub fn new() -> CollisionWorld {
        CollisionWorld::default()
    }

    pub fn add(&mut self, aabb: Aabb) -> usize {
        self.boxes.push(aabb);
        self.indexed = false;
        self.boxes.len() - 1
    }

    pub fn build_index(&mut self) {
        self.grid.clear();
        for (index, aabb) in self.boxes.iter().enumerate() {
            for cx in cell_range(aabb.min.x, aabb.max.x) {
                for cz in cell_range(aabb.min.z, aabb.max.z) {
                    self.grid.entry((cx, cz)).or_default().push(index);
                }
            }
        }
        self.indexed = true;
    }

    pub fn candidates(&self, region: &Aabb) -> Vec<usize> {
        if !self.indexed {
            return (0..self.boxes.len()).collect();
        }
        let mut found = Vec::new();
        for cx in cell_range(region.min.x, region.max.x) {
            for cz in cell_range(region.min.z, region.max.z) {
                if let Some(list) = self.grid.get(&(cx, cz)) {
                    found.extend_from_slice(list);
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    pub fn add_mesh_bounds(&mut self, mesh: &crate::model::mesh::Mesh) -> Option<usize> {
        let first = *mesh.positions.first()?;
        let bounds = mesh.positions.iter().fold(Aabb::new(first, first), |b, p| Aabb::new(b.min.min(*p), b.max.max(*p)));
        Some(self.add(bounds))
    }

    pub fn bounds(&self) -> Option<Aabb> {
        let mut iter = self.boxes.iter();
        let first = *iter.next()?;
        Some(iter.fold(first, |b, other| b.union(other)))
    }

    pub fn overlaps_any(&self, aabb: &Aabb) -> bool {
        self.candidates(aabb).into_iter().any(|i| self.boxes[i].overlaps(aabb))
    }

    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<RayHit> {
        let direction = direction.normalized();
        let mut best: Option<RayHit> = None;
        for (index, aabb) in self.boxes.iter().enumerate() {
            let limit = best.map_or(max_distance, |h| h.distance);
            if let Some((distance, normal)) = ray_aabb(origin, direction, aabb, limit) {
                if distance <= limit {
                    best = Some(RayHit { distance, point: origin + direction * distance, normal, collider: index });
                }
            }
        }
        best
    }

    pub fn line_of_sight(&self, from: Vec3, to: Vec3) -> bool {
        let delta = to - from;
        let distance = delta.length();
        if distance < 1.0e-4 {
            return true;
        }
        self.raycast(from, delta, distance - 0.01).is_none()
    }

    pub fn ground_height_at(&self, x: f32, z: f32, from_y: f32) -> Option<f32> {
        self.raycast(Vec3::new(x, from_y, z), Vec3::new(0.0, -1.0, 0.0), 1000.0).map(|h| h.point.y)
    }

    fn clamp_axis(&self, aabb: &Aabb, axis: usize, delta: f32) -> f32 {
        if delta == 0.0 {
            return 0.0;
        }
        let mut allowed = delta;
        let sweep = aabb.union(&aabb.translated(with_component(Vec3::ZERO, axis, delta)));
        for index in self.candidates(&sweep) {
            let other = &self.boxes[index];
            let overlaps_others = (0..3).filter(|a| *a != axis).all(|a| {
                component(aabb.min, a) < component(other.max, a) - EPSILON && component(aabb.max, a) > component(other.min, a) + EPSILON
            });
            if !overlaps_others {
                continue;
            }
            if delta > 0.0 {
                let gap = component(other.min, axis) - component(aabb.max, axis);
                if gap >= -EPSILON && gap < allowed {
                    allowed = gap.max(0.0);
                }
            } else {
                let gap = component(other.max, axis) - component(aabb.min, axis);
                if gap <= EPSILON && gap > allowed {
                    allowed = gap.min(0.0);
                }
            }
        }
        allowed
    }

    pub fn move_aabb(&self, aabb: &mut Aabb, axis: usize, delta: f32) -> f32 {
        let allowed = self.clamp_axis(aabb, axis, delta);
        *aabb = aabb.translated(with_component(Vec3::ZERO, axis, allowed));
        allowed
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoveParams {
    pub gravity: f32,
    pub jump_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    pub step_height: f32,
    pub max_fall_speed: f32,
}

impl Default for MoveParams {
    fn default() -> MoveParams {
        MoveParams {
            gravity: 22.0,
            jump_speed: 7.2,
            ground_acceleration: 70.0,
            air_acceleration: 14.0,
            step_height: 0.45,
            max_fall_speed: 40.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub height: f32,
    pub grounded: bool,
    pub bumped: bool,
}

impl Body {
    pub fn new(position: Vec3, radius: f32, height: f32) -> Body {
        Body { position, velocity: Vec3::ZERO, radius, height, grounded: false, bumped: false }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb {
            min: Vec3::new(self.position.x - self.radius, self.position.y, self.position.z - self.radius),
            max: Vec3::new(self.position.x + self.radius, self.position.y + self.height, self.position.z + self.radius),
        }
    }

    pub fn eye(&self, eye_height: f32) -> Vec3 {
        self.position + Vec3::new(0.0, eye_height, 0.0)
    }
}

pub fn step_body(world: &CollisionWorld, body: &mut Body, wish_velocity: Vec3, jump: bool, dt: f32, params: &MoveParams) {
    let acceleration = if body.grounded { params.ground_acceleration } else { params.air_acceleration };
    let wish = Vec3::new(wish_velocity.x, 0.0, wish_velocity.z);
    let current = Vec3::new(body.velocity.x, 0.0, body.velocity.z);
    let difference = wish - current;
    let max_change = acceleration * dt;
    let change = if difference.length() > max_change { difference.normalized() * max_change } else { difference };
    body.velocity.x += change.x;
    body.velocity.z += change.z;

    if body.grounded && jump {
        body.velocity.y = params.jump_speed;
        body.grounded = false;
    }
    body.velocity.y = (body.velocity.y - params.gravity * dt).max(-params.max_fall_speed);

    let mut aabb = body.aabb();
    body.bumped = false;
    let (dx, dz) = (body.velocity.x * dt, body.velocity.z * dt);
    let was_grounded = body.grounded;
    let (moved_x, moved_z) = slide_horizontal(world, &mut aabb, dx, dz);
    let blocked = (dx - moved_x).abs() > 1.0e-4 || (dz - moved_z).abs() > 1.0e-4;
    if blocked {
        body.bumped = true;
        if was_grounded && body.velocity.y <= 0.0 {
            let attempt = step_up(world, &aabb, dx, dz, params.step_height);
            if let Some((stepped, travel_x, travel_z)) = attempt {
                if travel_x * travel_x + travel_z * travel_z > moved_x * moved_x + moved_z * moved_z + 1.0e-6 {
                    aabb = stepped;
                    body.velocity.x = travel_x / dt;
                    body.velocity.z = travel_z / dt;
                } else {
                    reflect_blocked(body, dx, dz, moved_x, moved_z, dt);
                }
            } else {
                reflect_blocked(body, dx, dz, moved_x, moved_z, dt);
            }
        } else {
            reflect_blocked(body, dx, dz, moved_x, moved_z, dt);
        }
    }

    let dy = body.velocity.y * dt;
    let moved_y = world.move_aabb(&mut aabb, 1, dy);
    body.grounded = false;
    if (moved_y - dy).abs() > 1.0e-6 {
        if dy < 0.0 {
            body.grounded = true;
        }
        body.velocity.y = 0.0;
    } else if was_grounded && body.velocity.y <= 0.0 {
        let snap = world.move_aabb(&mut aabb, 1, -params.step_height.min(0.15));
        if snap < -1.0e-5 {
            body.grounded = true;
            body.velocity.y = 0.0;
        } else {
            let probe = world.clamp_axis(&aabb, 1, -0.02);
            body.grounded = probe > -0.02 + 1.0e-6;
        }
    } else {
        let probe = world.clamp_axis(&aabb, 1, -0.02);
        body.grounded = probe > -0.02 + 1.0e-6 && body.velocity.y <= 0.0;
    }
    body.position = Vec3::new((aabb.min.x + aabb.max.x) * 0.5, aabb.min.y, (aabb.min.z + aabb.max.z) * 0.5);
}

fn reflect_blocked(body: &mut Body, dx: f32, dz: f32, moved_x: f32, moved_z: f32, dt: f32) {
    if (dx - moved_x).abs() > 1.0e-4 {
        body.velocity.x = 0.0;
    }
    if (dz - moved_z).abs() > 1.0e-4 {
        body.velocity.z = 0.0;
    }
    let _ = dt;
}

fn slide_horizontal(world: &CollisionWorld, aabb: &mut Aabb, dx: f32, dz: f32) -> (f32, f32) {
    let moved_x = world.move_aabb(aabb, 0, dx);
    let moved_z = world.move_aabb(aabb, 2, dz);
    (moved_x, moved_z)
}

fn step_up(world: &CollisionWorld, aabb: &Aabb, dx: f32, dz: f32, step_height: f32) -> Option<(Aabb, f32, f32)> {
    let mut raised = *aabb;
    let lift = world.move_aabb(&mut raised, 1, step_height);
    if lift < 0.02 {
        return None;
    }
    let (travel_x, travel_z) = slide_horizontal(world, &mut raised, dx, dz);
    let drop = world.move_aabb(&mut raised, 1, -lift);
    let _ = drop;
    if raised.min.y - aabb.min.y > step_height + 1.0e-3 {
        return None;
    }
    Some((raised, travel_x, travel_z))
}

#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub bounciness: f32,
    pub resting: bool,
}

impl Projectile {
    pub fn new(position: Vec3, velocity: Vec3, radius: f32, bounciness: f32) -> Projectile {
        Projectile { position, velocity, radius, bounciness, resting: false }
    }

    pub fn step(&mut self, world: &CollisionWorld, gravity: f32, dt: f32) -> bool {
        if self.resting {
            return false;
        }
        self.velocity.y -= gravity * dt;
        let travel = self.velocity * dt;
        let distance = travel.length();
        let mut bounced = false;
        if distance > 1.0e-6 {
            let direction = travel * (1.0 / distance);
            if let Some(hit) = world.raycast(self.position, direction, distance + self.radius) {
                if hit.distance <= distance + self.radius {
                    let along = (hit.distance - self.radius).max(0.0);
                    self.position = self.position + direction * along + hit.normal * 0.001;
                    let into = self.velocity.dot(hit.normal);
                    self.velocity = (self.velocity - hit.normal * (into * (1.0 + self.bounciness))) * 0.85;
                    bounced = true;
                    if self.velocity.length() < 0.8 && hit.normal.y > 0.7 {
                        self.resting = true;
                        self.velocity = Vec3::ZERO;
                    }
                } else {
                    self.position = self.position + travel;
                }
            } else {
                self.position = self.position + travel;
            }
        }
        bounced
    }
}
