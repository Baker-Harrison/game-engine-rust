use crate::camera::Camera;
use crate::ecs::World;
use crate::input::{ActionMap, Input};
use crate::math::Vec3;
use crate::mixer::{Mixer, PlayParams, SharedMixer, Sound, VoiceId};
use crate::nav::NavGraph;
use crate::physics::CollisionWorld;
use crate::random::Random;
use crate::scene::Scene;
use crate::ui::Ui;
use crate::worldfile::Registry;

pub const TICK_RATE: u32 = 60;
pub const TICK_SECONDS: f32 = 1.0 / TICK_RATE as f32;
const LOG_LIMIT: usize = 400;

pub struct Time {
    pub tick: u64,
}

impl Time {
    pub fn seconds(&self) -> f32 {
        self.tick as f32 * TICK_SECONDS
    }
}

pub struct Engine {
    pub world: World,
    pub registry: Registry,
    pub scene: Scene,
    pub scene_dirty: bool,
    pub collision: CollisionWorld,
    pub nav: NavGraph,
    pub mixer: SharedMixer,
    pub ui: Ui,
    pub random: Random,
    pub actions: ActionMap,
    pub time: Time,
    pub log: Vec<String>,
}

impl Engine {
    pub fn new(seed: u64) -> Engine {
        Engine {
            world: World::new(),
            registry: Registry::default(),
            scene: Scene::empty(Vec3::new(0.53, 0.74, 0.93), 200.0),
            scene_dirty: true,
            collision: CollisionWorld::new(),
            nav: NavGraph::default(),
            mixer: Mixer::shared(),
            ui: Ui::new(),
            random: Random::new(seed),
            actions: ActionMap::default(),
            time: Time { tick: 0 },
            log: Vec::new(),
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.scene.camera
    }

    pub fn play(&self, sound: &Sound, params: PlayParams) -> Option<VoiceId> {
        self.mixer.lock().ok().map(|mut mixer| mixer.play(sound, params))
    }

    pub fn sync_listener(&self) {
        let camera = self.scene.camera;
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_listener(camera.pos, camera.right());
        }
    }

    pub fn note(&mut self, line: String) {
        if self.log.len() < LOG_LIMIT {
            self.log.push(format!("[{:7.2}s] {}", self.time.seconds(), line));
        }
    }

    pub fn reseed(&mut self, seed: u64) {
        self.random = Random::new(seed);
    }
}

pub trait Game {
    fn name(&self) -> &str;

    fn setup(&mut self, engine: &mut Engine) -> Result<(), String>;

    fn tick(&mut self, engine: &mut Engine, input: &Input);

    fn draw(&mut self, engine: &mut Engine, blend: f32);

    fn report(&self, _engine: &Engine) -> String {
        String::new()
    }

    fn finished(&self) -> bool {
        false
    }
}
