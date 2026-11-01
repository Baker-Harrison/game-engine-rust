use crate::camera::Camera;
use crate::math::Vec3;
use crate::model::bvh::{Bvh, ClosestHit};
use crate::overlay::pack_rgb;
use crate::parallel::for_each_chunk;
use crate::random::Random;
use crate::render::{Framebuffer, SUN};
use crate::scene::{Scene, Tri};

#[derive(Clone, Copy)]
pub struct TraceSettings {
    pub samples_per_pixel: usize,
    pub shadow_samples: usize,
    pub indirect_samples: usize,
    pub reflection_samples: usize,
    pub sun_angular_radius: f32,
    pub exposure: f32,
}

impl Default for TraceSettings {
    fn default() -> TraceSettings {
        TraceSettings { samples_per_pixel: 4, shadow_samples: 3, indirect_samples: 6, reflection_samples: 1, sun_angular_radius: 0.035, exposure: 0.6 }
    }
}

const SUN_INTENSITY: f32 = 1.35;
const SKY_INTENSITY: f32 = 1.0;
const BOUNCE_ALBEDO_SCALE: f32 = 0.3;
const RAY_OFFSET: f32 = 0.002;
const MAX_DISTANCE: f32 = 1000.0;
const INDIRECT_DISTANCE: f32 = 6.0;

struct Surface {
    position: Vec3,
    normal: Vec3,
    albedo: Vec3,
    roughness: f32,
    metallic: f32,
}

pub struct Tracer<'a> {
    bvh: Bvh,
    triangles: &'a [Tri],
    sky_horizon: Vec3,
    sky_zenith: Vec3,
    sun: Vec3,
    settings: TraceSettings,
}

impl<'a> Tracer<'a> {
    pub fn new(scene: &'a Scene, settings: TraceSettings) -> Tracer<'a> {
        let geometry = scene.tris.iter().map(|tri| tri.v).collect();
        Tracer {
            bvh: Bvh::from_triangles(geometry),
            triangles: &scene.tris,
            sky_horizon: scene.sky,
            sky_zenith: scene.sky.mul_elements(Vec3::new(0.55, 0.7, 1.0)),
            sun: SUN.normalized(),
            settings,
        }
    }

    fn sky(&self, direction: Vec3) -> Vec3 {
        let up = direction.y;
        let base = if up >= 0.0 {
            self.sky_horizon.lerp(self.sky_zenith, up.powf(0.6).min(1.0))
        } else {
            self.sky_horizon * 0.4
        };
        let glow = direction.dot(self.sun).max(0.0);
        base * SKY_INTENSITY + Vec3::new(1.0, 0.9, 0.7) * (glow.powi(256) * 6.0 + glow.powi(12) * 0.12)
    }

    fn surface_at(&self, hit: &ClosestHit, direction: Vec3) -> Surface {
        let tri = &self.triangles[hit.triangle];
        let weights = [1.0 - hit.u - hit.v, hit.u, hit.v];
        let blend = |values: [Vec3; 3]| values[0] * weights[0] + values[1] * weights[1] + values[2] * weights[2];
        let position = blend(tri.v);
        let mut normal = blend(tri.n).normalized();
        if normal.dot(direction) > 0.0 {
            normal = -normal;
        }
        let surface_value = |channel: usize| tri.s[0][channel] * weights[0] + tri.s[1][channel] * weights[1] + tri.s[2][channel] * weights[2];
        Surface { position, normal, albedo: blend(tri.c), roughness: surface_value(0).clamp(0.04, 1.0), metallic: surface_value(1).clamp(0.0, 1.0) }
    }

    fn sun_visibility(&self, point: Vec3, normal: Vec3, random: &mut Random) -> f32 {
        let samples = self.settings.shadow_samples.max(1);
        let mut visible = 0;
        let (tangent, bitangent) = basis(self.sun);
        for _ in 0..samples {
            let (a, b) = disc_sample(random);
            let direction = (self.sun + tangent * (a * self.settings.sun_angular_radius) + bitangent * (b * self.settings.sun_angular_radius)).normalized();
            if !self.bvh.any_hit(point + normal * RAY_OFFSET, direction, MAX_DISTANCE) {
                visible += 1;
            }
        }
        visible as f32 / samples as f32
    }

    fn indirect_light(&self, surface: &Surface, random: &mut Random) -> Vec3 {
        let samples = self.settings.indirect_samples.max(1);
        let (tangent, bitangent) = basis(surface.normal);
        let mut total = Vec3::ZERO;
        for _ in 0..samples {
            let direction = cosine_sample(surface.normal, tangent, bitangent, random);
            let origin = surface.position + surface.normal * RAY_OFFSET;
            total += match self.bvh.closest_hit(origin, direction, INDIRECT_DISTANCE) {
                None => self.sky(direction) * 0.9,
                Some(hit) => {
                    let other = self.surface_at(&hit, direction);
                    let lit = 0.35 + 0.65 * other.normal.dot(self.sun).max(0.0);
                    other.albedo * (lit * BOUNCE_ALBEDO_SCALE * 1.4)
                }
            };
        }
        total / samples as f32
    }

    fn reflection(&self, surface: &Surface, view: Vec3, random: &mut Random) -> Vec3 {
        let samples = self.settings.reflection_samples.max(1);
        let mirror = view - surface.normal * (2.0 * view.dot(surface.normal));
        let (tangent, bitangent) = basis(mirror);
        let spread = surface.roughness * surface.roughness;
        let mut total = Vec3::ZERO;
        for _ in 0..samples {
            let (a, b) = disc_sample(random);
            let direction = (mirror + tangent * (a * spread) + bitangent * (b * spread)).normalized();
            let origin = surface.position + surface.normal * RAY_OFFSET;
            total += match self.bvh.closest_hit(origin, direction, MAX_DISTANCE) {
                None => self.sky(direction),
                Some(hit) => {
                    let other = self.surface_at(&hit, direction);
                    let lit = 0.3 + 0.7 * other.normal.dot(self.sun).max(0.0);
                    other.albedo * (lit * 0.8)
                }
            };
        }
        total / samples as f32
    }

    fn shade(&self, surface: &Surface, view: Vec3, random: &mut Random) -> Vec3 {
        let n_dot_v = surface.normal.dot(-view).max(1e-3);
        let n_dot_l = surface.normal.dot(self.sun).max(0.0);
        let visibility = if n_dot_l > 0.0 { self.sun_visibility(surface.position, surface.normal, random) } else { 0.0 };
        let diffuse_color = surface.albedo * (1.0 - surface.metallic);
        let f0 = Vec3::splat(0.04).lerp(surface.albedo, surface.metallic);
        let fresnel = f0 + (Vec3::splat(1.0) - f0) * (1.0 - n_dot_v).powi(5);

        let direct = diffuse_color * (SUN_INTENSITY * n_dot_l * visibility);
        let indirect = diffuse_color.mul_elements(self.indirect_light(surface, random));

        let half = (self.sun - view).normalized();
        let n_dot_h = surface.normal.dot(half).max(0.0);
        let alpha = (surface.roughness * surface.roughness).max(0.002);
        let denominator = n_dot_h * n_dot_h * (alpha * alpha - 1.0) + 1.0;
        let distribution = alpha * alpha / (std::f32::consts::PI * denominator * denominator);
        let geometry = 1.0 / ((n_dot_v + (alpha + (1.0 - alpha) * n_dot_v * n_dot_v).sqrt()) * (n_dot_l + (alpha + (1.0 - alpha) * n_dot_l * n_dot_l).sqrt()) + 1e-4);
        let sun_specular = fresnel * (SUN_INTENSITY * distribution * geometry * n_dot_l * visibility);

        let smoothness = (1.0 - surface.roughness).powi(2);
        let environment = if smoothness > 0.02 { fresnel.mul_elements(self.reflection(surface, view, random)) * smoothness } else { Vec3::ZERO };
        direct + indirect + sun_specular + environment
    }

    fn radiance(&self, origin: Vec3, direction: Vec3, random: &mut Random) -> Vec3 {
        match self.bvh.closest_hit(origin, direction, MAX_DISTANCE) {
            None => self.sky(direction),
            Some(hit) => {
                let surface = self.surface_at(&hit, direction);
                self.shade(&surface, direction, random)
            }
        }
    }
}

pub fn render(scene: &Scene, camera: &Camera, width: usize, height: usize, settings: TraceSettings) -> Framebuffer {
    let tracer = Tracer::new(scene, settings);
    let mut frame = Framebuffer::new(width, height);
    let forward = camera.forward();
    let right = camera.right();
    let up = camera.up();
    let tan_half = (camera.fov_y * 0.5).tan();
    let aspect = width as f32 / height as f32;
    let samples = settings.samples_per_pixel.max(1);
    let grid = (samples as f32).sqrt().ceil() as usize;
    let tracer = &tracer;

    for_each_chunk(&mut frame.pixels, width, |start, chunk| {
        for (offset, pixel) in chunk.iter_mut().enumerate() {
            let index = start + offset;
            let (x, y) = (index % width, index / width);
            let mut random = Random::new(index as u64 * 2654435761 + 17);
            let mut total = Vec3::ZERO;
            for sample in 0..samples {
                let (jx, jy) = ((sample % grid) as f32 + random.unit(), (sample / grid) as f32 + random.unit());
                let (jx, jy) = (jx / grid as f32, jy / grid as f32);
                let ndc_x = (x as f32 + jx) / width as f32 * 2.0 - 1.0;
                let ndc_y = 1.0 - (y as f32 + jy) / height as f32 * 2.0;
                let direction = (forward + right * (ndc_x * tan_half * aspect) + up * (ndc_y * tan_half)).normalized();
                let color = tracer.radiance(camera.pos, direction, &mut random);
                total += Vec3::new(color.x.min(8.0), color.y.min(8.0), color.z.min(8.0));
            }
            *pixel = tone_map(total / samples as f32 * settings.exposure);
        }
    });
    frame
}

fn tone_map(color: Vec3) -> u32 {
    let map = |value: f32| {
        let x = value.max(0.0) * 0.85;
        let mapped = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
        (mapped.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
    };
    pack_rgb(map(color.x), map(color.y), map(color.z))
}

fn basis(direction: Vec3) -> (Vec3, Vec3) {
    let helper = if direction.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) };
    let tangent = direction.cross(helper).normalized();
    (tangent, direction.cross(tangent))
}

fn disc_sample(random: &mut Random) -> (f32, f32) {
    let radius = random.unit().sqrt();
    let angle = random.unit() * std::f32::consts::TAU;
    (radius * angle.cos(), radius * angle.sin())
}

fn cosine_sample(normal: Vec3, tangent: Vec3, bitangent: Vec3, random: &mut Random) -> Vec3 {
    let (a, b) = disc_sample(random);
    let up = (1.0 - a * a - b * b).max(0.0).sqrt();
    (tangent * a + bitangent * b + normal * up).normalized()
}
