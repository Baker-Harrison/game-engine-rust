use crate::camera::MAX_PITCH;
use crate::input::{Input, Key};
use crate::math::Vec3;
use crate::physics::{step_body, Aabb, Body, CollisionWorld, MoveParams};
use crate::runtime::{Engine, Game, TICK_SECONDS};
use crate::scene::Scene;

const WALK_SPEED: f32 = 4.5;
const SPRINT_MULTIPLIER: f32 = 2.0;
const MOUSE_SENSITIVITY: f32 = 0.0025;
const EYE_HEIGHT: f32 = 1.65;

pub struct SceneViewer {
    name: String,
    source: Option<Scene>,
    body: Body,
    params: MoveParams,
}

impl SceneViewer {
    pub fn new(name: &str, scene: Scene) -> SceneViewer {
        let start = scene.camera.pos - Vec3::new(0.0, EYE_HEIGHT, 0.0);
        SceneViewer { name: name.to_string(), source: Some(scene), body: Body::new(start, 0.3, 1.8), params: MoveParams::default() }
    }
}

fn collision_from_scene(scene: &Scene) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    if let Some(ground) = scene.ground {
        world.add(Aabb::new(Vec3::new(-ground.size, -2.0, -ground.size), Vec3::new(ground.size, 0.0, ground.size)));
    }
    for entity in &scene.boxes {
        world.add(Aabb::from_center(entity.center, entity.size));
    }
    world.build_index();
    world
}

impl Game for SceneViewer {
    fn name(&self) -> &str {
        &self.name
    }

    fn setup(&mut self, engine: &mut Engine) -> Result<(), String> {
        let scene = self.source.take().ok_or("scene already consumed")?;
        engine.collision = collision_from_scene(&scene);
        engine.scene = scene;
        engine.scene_dirty = true;
        Ok(())
    }

    fn tick(&mut self, engine: &mut Engine, input: &Input) {
        let camera = &mut engine.scene.camera;
        camera.yaw -= input.mouse_dx * MOUSE_SENSITIVITY;
        camera.pitch = (camera.pitch - input.mouse_dy * MOUSE_SENSITIVITY).clamp(-MAX_PITCH, MAX_PITCH);
        let held = |key: Key| if input.key_down(key) { 1.0 } else { 0.0 };
        let forward = held(Key::W) + held(Key::Up) - held(Key::S) - held(Key::Down);
        let strafe = held(Key::D) + held(Key::Right) - held(Key::A) - held(Key::Left);
        let wish = (camera.walk_forward() * forward + camera.right() * strafe).normalized();
        let speed = WALK_SPEED * if input.key_down(Key::Shift) { SPRINT_MULTIPLIER } else { 1.0 };
        step_body(&engine.collision, &mut self.body, wish * speed, input.key_pressed(Key::Space), TICK_SECONDS, &self.params);
    }

    fn draw(&mut self, engine: &mut Engine, _blend: f32) {
        engine.scene.camera.pos = self.body.eye(EYE_HEIGHT);
    }

    fn report(&self, _engine: &Engine) -> String {
        format!("position: {:.2} {:.2} {:.2}\n", self.body.position.x, self.body.position.y, self.body.position.z)
    }
}
