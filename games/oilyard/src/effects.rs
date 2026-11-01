use crate::pose::{push_lit_quad, push_quad};
use engine::camera::Camera;
use engine::math::Vec3;
use engine::random::Random;
use engine::scene::Tri;

const MAX_PARTICLES: usize = 700;
const MAX_TRACERS: usize = 80;
const MAX_DECALS: usize = 90;
const TRACER_SPEED: f32 = 260.0;
const TRACER_LENGTH: f32 = 5.0;

pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub life: f32,
    pub size: f32,
    pub color: Vec3,
    pub gravity: f32,
    pub drag: f32,
}

pub struct Tracer {
    pub from: Vec3,
    pub to: Vec3,
    pub age: f32,
}

pub struct Decal {
    pub position: Vec3,
    pub normal: Vec3,
    pub size: f32,
}

pub struct Effects {
    pub particles: Vec<Particle>,
    pub tracers: Vec<Tracer>,
    pub decals: Vec<Decal>,
}

fn random_direction(random: &mut Random) -> Vec3 {
    loop {
        let candidate = Vec3::new(random.range(-1.0, 1.0), random.range(-1.0, 1.0), random.range(-1.0, 1.0));
        let length = candidate.length();
        if length > 0.1 && length <= 1.0 {
            return candidate * (1.0 / length);
        }
    }
}

impl Effects {
    pub fn new() -> Effects {
        Effects { particles: Vec::new(), tracers: Vec::new(), decals: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.tracers.clear();
        self.decals.clear();
    }

    pub fn step(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.age += dt;
            particle.velocity.y -= particle.gravity * dt;
            particle.velocity = particle.velocity * (1.0 - particle.drag * dt).max(0.0);
            particle.position = particle.position + particle.velocity * dt;
        }
        self.particles.retain(|p| p.age < p.life);
        for tracer in &mut self.tracers {
            tracer.age += dt;
        }
        let expired = |t: &Tracer| t.age * TRACER_SPEED > (t.to - t.from).length() + TRACER_LENGTH;
        self.tracers.retain(|t| !expired(t));
    }

    fn add(&mut self, particle: Particle) {
        if self.particles.len() < MAX_PARTICLES {
            self.particles.push(particle);
        }
    }

    pub fn tracer(&mut self, from: Vec3, to: Vec3) {
        if self.tracers.len() < MAX_TRACERS {
            self.tracers.push(Tracer { from, to, age: 0.0 });
        }
    }

    pub fn bullet_hole(&mut self, position: Vec3, normal: Vec3, size: f32) {
        if self.decals.len() >= MAX_DECALS {
            self.decals.remove(0);
        }
        self.decals.push(Decal { position: position + normal * 0.012, normal, size });
    }

    pub fn dust(&mut self, position: Vec3, normal: Vec3, random: &mut Random) {
        for _ in 0..7 {
            let velocity = (normal * 1.6 + random_direction(random)).normalized() * random.range(0.8, 2.6);
            let shade = random.range(0.85, 1.1);
            self.add(Particle {
                position,
                velocity,
                age: 0.0,
                life: random.range(0.35, 0.8),
                size: random.range(0.04, 0.09),
                color: Vec3::new(0.78, 0.66, 0.48) * shade,
                gravity: 1.5,
                drag: 2.5,
            });
        }
        for _ in 0..3 {
            let velocity = (normal + random_direction(random) * 0.6).normalized() * random.range(3.0, 7.0);
            self.add(Particle {
                position,
                velocity,
                age: 0.0,
                life: random.range(0.12, 0.28),
                size: 0.018,
                color: Vec3::new(1.0, 0.8, 0.4),
                gravity: 9.0,
                drag: 0.0,
            });
        }
    }

    pub fn blood(&mut self, position: Vec3, direction: Vec3, random: &mut Random) {
        for _ in 0..9 {
            let velocity = (direction * -0.6 + random_direction(random)).normalized() * random.range(1.0, 3.5);
            self.add(Particle {
                position,
                velocity,
                age: 0.0,
                life: random.range(0.3, 0.6),
                size: random.range(0.03, 0.06),
                color: Vec3::new(0.7, 0.05, 0.04),
                gravity: 9.0,
                drag: 1.0,
            });
        }
    }

    pub fn explosion(&mut self, position: Vec3, random: &mut Random) {
        for _ in 0..70 {
            let velocity = random_direction(random) * random.range(2.0, 11.0);
            let heat = random.unit();
            self.add(Particle {
                position,
                velocity,
                age: 0.0,
                life: random.range(0.3, 0.9),
                size: random.range(0.2, 0.55) * (1.2 - heat * 0.5),
                color: Vec3::new(1.0, 0.45 + 0.4 * heat, 0.1 + 0.2 * heat),
                gravity: -1.0,
                drag: 2.0,
            });
        }
        for _ in 0..50 {
            let mut velocity = random_direction(random) * random.range(1.0, 7.0);
            velocity.y = velocity.y.abs() + 1.0;
            self.add(Particle {
                position,
                velocity,
                age: 0.0,
                life: random.range(1.0, 2.2),
                size: random.range(0.35, 0.8),
                color: Vec3::splat(random.range(0.18, 0.35)),
                gravity: -0.5,
                drag: 1.2,
            });
        }
        for _ in 0..30 {
            let velocity = random_direction(random) * random.range(8.0, 18.0);
            self.add(Particle {
                position,
                velocity,
                age: 0.0,
                life: random.range(0.4, 0.9),
                size: 0.04,
                color: Vec3::new(1.0, 0.8, 0.35),
                gravity: 14.0,
                drag: 0.4,
            });
        }
    }

    pub fn emit(&self, camera: &Camera, out: &mut Vec<Tri>) {
        let right = camera.right();
        let up = camera.up();
        for particle in &self.particles {
            let fade = 1.0 - particle.age / particle.life;
            let size = particle.size * (0.4 + 0.6 * fade);
            push_quad(out, particle.position, right * size, up * size, particle.color * (0.35 + 0.65 * fade));
        }
        for decal in &self.decals {
            let tangent = decal.normal.cross(Vec3::new(0.0, 1.0, 0.0));
            let tangent = if tangent.length() < 0.1 { Vec3::new(1.0, 0.0, 0.0) } else { tangent.normalized() };
            let bitangent = decal.normal.cross(tangent);
            push_lit_quad(out, decal.position, tangent * decal.size, bitangent * decal.size, Vec3::splat(0.05));
        }
        for tracer in &self.tracers {
            let direction = (tracer.to - tracer.from).normalized();
            let length = (tracer.to - tracer.from).length();
            let head = (tracer.age * TRACER_SPEED).min(length);
            let tail = (head - TRACER_LENGTH).max(0.0);
            if head <= tail {
                continue;
            }
            let (start, end) = (tracer.from + direction * tail, tracer.from + direction * head);
            let to_camera = (camera.pos - (start + end) * 0.5).normalized();
            let side = direction.cross(to_camera).normalized() * 0.012;
            let color = Vec3::new(1.0, 0.85, 0.5);
            let corners = [start - side, start + side, end + side, end - side];
            out.push(Tri::glowing([corners[0], corners[1], corners[2]], color));
            out.push(Tri::glowing([corners[0], corners[2], corners[3]], color));
        }
    }
}
