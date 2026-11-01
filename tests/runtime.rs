use engine::ecs::World;
use engine::headless::{simulate, SimOptions};
use engine::input::{ActionMap, Input, InputScript, Key};
use engine::math::Vec3;
use engine::mixer::{Mixer, PlayParams, Sound, SAMPLE_RATE};
use engine::nav::{NavGraph, NavSettings};
use engine::physics::{step_body, Aabb, Body, CollisionWorld, MoveParams};
use engine::runtime::{Engine, Game, TICK_SECONDS};
use engine::worldfile::{Component, Fields, Registry, Transform};

fn flat_world() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.add(Aabb::new(Vec3::new(-50.0, -1.0, -50.0), Vec3::new(50.0, 0.0, 50.0)));
    world
}

fn settle(world: &CollisionWorld, body: &mut Body) {
    for _ in 0..120 {
        step_body(world, body, Vec3::ZERO, false, TICK_SECONDS, &MoveParams::default());
    }
}

#[test]
fn entities_have_generations_and_components() {
    let mut world = World::new();
    let a = world.spawn();
    world.insert(a, 5i32);
    world.insert(a, "tag");
    assert_eq!(world.get::<i32>(a), Some(&5));
    world.despawn(a);
    assert!(!world.is_alive(a));
    assert_eq!(world.get::<i32>(a), None);
    let b = world.spawn();
    assert_eq!(a.index(), b.index());
    assert_ne!(a, b);
    assert_eq!(world.get::<i32>(b), None);
    world.insert(b, 9i32);
    assert_eq!(world.with::<i32>(), vec![b]);
    world.set_resource(3.5f32);
    assert_eq!(world.resource::<f32>(), Some(&3.5));
}

#[derive(Clone, PartialEq, Debug)]
struct Health(f32);

impl Component for Health {
    const NAME: &'static str = "health";
    fn from_fields(fields: &Fields) -> Result<Health, String> {
        Ok(Health(fields.f32("value")?))
    }
    fn to_fields(&self) -> Vec<(String, String)> {
        vec![("value".into(), self.0.to_string())]
    }
}

#[test]
fn worlds_round_trip_through_text() {
    let mut registry = Registry::default();
    registry.register::<Health>();
    let mut world = World::new();
    let names = registry
        .load(&mut world, "entity crate\n  transform pos=1,2,3 yaw=90\n  health value=12\nentity lamp\n  transform pos=0,5,0\n")
        .unwrap();
    let crate_entity = names["crate"];
    assert_eq!(world.get::<Health>(crate_entity), Some(&Health(12.0)));
    assert_eq!(world.get::<Transform>(crate_entity).unwrap().position, Vec3::new(1.0, 2.0, 3.0));
    let saved = registry.save(&world);
    let mut copy = World::new();
    registry.load(&mut copy, &saved).unwrap();
    assert_eq!(registry.save(&copy), saved);
    assert_eq!(registry.checksum(&copy), registry.checksum(&world));
    assert!(registry.load(&mut copy, "entity x\n  nonsense a=1\n").unwrap_err().contains("unknown component"));
}

#[test]
fn body_lands_walks_and_is_stopped_by_walls() {
    let mut world = flat_world();
    world.add(Aabb::new(Vec3::new(3.0, 0.0, -5.0), Vec3::new(4.0, 3.0, 5.0)));
    world.build_index();
    let mut body = Body::new(Vec3::new(0.0, 2.0, 0.0), 0.3, 1.8);
    settle(&world, &mut body);
    assert!(body.grounded);
    assert!(body.position.y.abs() < 0.01);
    for _ in 0..240 {
        step_body(&world, &mut body, Vec3::new(5.0, 0.0, 0.0), false, TICK_SECONDS, &MoveParams::default());
    }
    assert!(body.position.x < 2.75 && body.position.x > 2.5, "stopped at the wall, got {}", body.position.x);
}

#[test]
fn body_steps_up_low_ledges_but_not_tall_ones() {
    let mut world = flat_world();
    world.add(Aabb::new(Vec3::new(2.0, 0.0, -5.0), Vec3::new(5.0, 0.3, 5.0)));
    world.add(Aabb::new(Vec3::new(8.0, 0.0, -5.0), Vec3::new(9.0, 1.2, 5.0)));
    world.build_index();
    let mut body = Body::new(Vec3::ZERO, 0.3, 1.8);
    settle(&world, &mut body);
    for _ in 0..60 {
        step_body(&world, &mut body, Vec3::new(4.0, 0.0, 0.0), false, TICK_SECONDS, &MoveParams::default());
    }
    assert!(body.position.x > 2.5 && (body.position.y - 0.3).abs() < 0.02, "on the ledge: {:?}", body.position);
    for _ in 0..240 {
        step_body(&world, &mut body, Vec3::new(4.0, 0.0, 0.0), false, TICK_SECONDS, &MoveParams::default());
    }
    assert!(body.position.x < 7.75, "blocked by the tall wall: {}", body.position.x);
}

#[test]
fn jumping_leaves_and_returns_to_the_ground() {
    let world = flat_world();
    let mut body = Body::new(Vec3::ZERO, 0.3, 1.8);
    settle(&world, &mut body);
    step_body(&world, &mut body, Vec3::ZERO, true, TICK_SECONDS, &MoveParams::default());
    assert!(!body.grounded && body.velocity.y > 0.0);
    let mut peak = 0.0f32;
    for _ in 0..120 {
        step_body(&world, &mut body, Vec3::ZERO, false, TICK_SECONDS, &MoveParams::default());
        peak = peak.max(body.position.y);
    }
    assert!(peak > 0.8 && peak < 1.4, "jump peak {}", peak);
    assert!(body.grounded);
}

#[test]
fn rays_find_the_nearest_box_and_face() {
    let mut world = flat_world();
    world.add(Aabb::new(Vec3::new(5.0, 0.0, -1.0), Vec3::new(6.0, 2.0, 1.0)));
    world.add(Aabb::new(Vec3::new(8.0, 0.0, -1.0), Vec3::new(9.0, 2.0, 1.0)));
    world.build_index();
    let hit = world.raycast(Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0), 100.0).unwrap();
    assert!((hit.distance - 5.0).abs() < 1e-4);
    assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    assert!(!world.line_of_sight(Vec3::new(0.0, 1.0, 0.0), Vec3::new(10.0, 1.0, 0.0)));
    assert!(world.line_of_sight(Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 10.0)));
    let down = world.raycast(Vec3::new(5.5, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 100.0).unwrap();
    assert!((down.point.y - 2.0).abs() < 1e-4);
}

#[test]
fn navigation_routes_around_walls_and_up_stairs() {
    let mut world = flat_world();
    world.add(Aabb::new(Vec3::new(-0.5, 0.0, -10.0), Vec3::new(0.5, 3.0, 4.0)));
    for step in 0..8 {
        let x = 6.0 + step as f32 * 0.5;
        world.add(Aabb::new(Vec3::new(x, 0.0, 6.0), Vec3::new(x + 0.5, 0.2 * (step + 1) as f32, 9.0)));
    }
    world.add(Aabb::new(Vec3::new(10.0, 0.0, 6.0), Vec3::new(14.0, 1.6, 9.0)));
    world.build_index();
    let graph = NavGraph::build(&world, &NavSettings { max_surface_height: 5.0, ..NavSettings::default() });
    let path = graph.find_path(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0)).expect("path around the wall");
    assert!(path.iter().any(|p| p.z > 3.5), "path detours around the wall end");
    let up = graph.find_path(Vec3::new(5.0, 0.0, 7.0), Vec3::new(12.0, 1.6, 7.5)).expect("path up the stairs");
    assert!((up.last().unwrap().y - 1.6).abs() < 0.05);
    assert!(graph.find_path(Vec3::new(-5.0, 0.0, 0.0), Vec3::new(500.0, 0.0, 0.0)).map_or(true, |p| p.last().unwrap().x < 60.0));
}

fn tone(frequency: f32, seconds: f32) -> Sound {
    let count = (SAMPLE_RATE as f32 * seconds) as usize;
    Sound::from_samples((0..count).map(|i| (i as f32 * frequency * std::f32::consts::TAU / SAMPLE_RATE as f32).sin() * 0.5).collect())
}

#[test]
fn mixer_pans_attenuates_and_finishes_voices() {
    let sound = tone(440.0, 0.2);
    let mut mixer = Mixer::new();
    mixer.set_listener(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    mixer.play(&sound, PlayParams::at(Vec3::new(10.0, 0.0, 0.0), 1.0));
    let mut block = vec![0.0f32; 1024];
    mixer.mix(&mut block);
    let energy = |channel: usize| block.iter().skip(channel).step_by(2).map(|s| s * s).sum::<f32>();
    assert!(energy(1) > energy(0) * 4.0, "a source on the right is louder in the right ear");
    let near = {
        let mut m = Mixer::new();
        m.play(&sound, PlayParams::at(Vec3::new(1.0, 0.0, 0.0), 1.0));
        m.mix(&mut block);
        block.iter().map(|s| s * s).sum::<f32>()
    };
    let far = {
        let mut m = Mixer::new();
        m.play(&sound, PlayParams::at(Vec3::new(60.0, 0.0, 0.0), 1.0));
        m.mix(&mut block);
        block.iter().map(|s| s * s).sum::<f32>()
    };
    assert!(near > far * 10.0);
    for _ in 0..20 {
        mixer.mix(&mut block);
    }
    assert_eq!(mixer.voice_count(), 0);
}

#[test]
fn wav_files_round_trip() {
    let original = tone(220.0, 0.05);
    let loaded = Sound::from_wav(&original.to_wav()).unwrap();
    assert_eq!(loaded.samples().len(), original.samples().len());
    assert!((loaded.peak() - original.peak()).abs() < 0.01);
    assert!(Sound::from_wav(b"nope").is_err());
}

#[test]
fn input_scripts_drive_edges_and_actions() {
    let script = InputScript::parse("tick 2 hold w\ntick 4 tap space\ntick 6 look 3 -1 for 2\ntick 8 release w\n").unwrap();
    let actions = ActionMap::parse("jump = space, pad_south\nforward = w\n").unwrap();
    let mut input = Input::new();
    let mut seen = Vec::new();
    for tick in 0..10 {
        input.clear_edges();
        script.apply(tick, &mut input);
        seen.push((actions.down(&input, "forward"), actions.pressed(&input, "jump"), input.mouse_dx));
    }
    assert!(!seen[1].0 && seen[2].0 && seen[7].0 && !seen[8].0);
    assert!(seen[4].1 && !seen[5].1);
    assert_eq!((seen[6].2, seen[7].2, seen[8].2), (3.0, 3.0, 0.0));
    assert!(InputScript::parse("tick 1 hold banana").is_err());
    assert!(Input::new().key_down(Key::W) == false);
}

struct Counter {
    ticks: u32,
    sound: Sound,
}

impl Game for Counter {
    fn name(&self) -> &str {
        "counter"
    }
    fn setup(&mut self, engine: &mut Engine) -> Result<(), String> {
        let entity = engine.world.spawn();
        engine.world.insert(entity, Transform::at(Vec3::ZERO));
        Ok(())
    }
    fn tick(&mut self, engine: &mut Engine, input: &Input) {
        self.ticks += 1;
        let jitter = engine.random.unit();
        for entity in engine.world.with::<Transform>() {
            if let Some(t) = engine.world.get_mut::<Transform>(entity) {
                t.position.x += jitter + if input.key_down(Key::W) { 1.0 } else { 0.0 };
            }
        }
        if self.ticks % 30 == 0 {
            engine.play(&self.sound, PlayParams::flat(0.5));
        }
    }
    fn draw(&mut self, _engine: &mut Engine, _blend: f32) {}
    fn report(&self, _engine: &Engine) -> String {
        format!("ticks seen: {}\n", self.ticks)
    }
}

#[test]
fn headless_simulation_is_deterministic_and_scriptable() {
    let run = |seed: u64, script: &str| {
        let mut game = Counter { ticks: 0, sound: tone(300.0, 0.1) };
        let options = SimOptions { ticks: 200, seed, script: Some(InputScript::parse(script).unwrap()), ..SimOptions::default() };
        simulate(&mut game, &options).unwrap()
    };
    let first = run(7, "tick 10 hold w\n");
    let second = run(7, "tick 10 hold w\n");
    let other_seed = run(8, "tick 10 hold w\n");
    let other_input = run(7, "tick 50 hold w\n");
    assert_eq!(first.checksum, second.checksum);
    assert_ne!(first.checksum, other_seed.checksum);
    assert_ne!(first.checksum, other_input.checksum);
    assert!(first.report.contains("ticks seen: 200"));
    assert!(first.report.contains("audio: peak"));
}

#[test]
fn config_and_saves_persist() {
    let directory = std::env::temp_dir().join(format!("engine-test-{}", std::process::id()));
    let mut config = engine::save::Config::parse("# comment\nvolume = 0.5\nfullscreen = on\n").unwrap();
    config.set("sensitivity", 1.25);
    config.save(&directory.join("config.txt")).unwrap();
    let loaded = engine::save::Config::load(&directory.join("config.txt"));
    assert_eq!(loaded.number("volume", 1.0), 0.5);
    assert!(loaded.flag("fullscreen", false));
    assert_eq!(loaded.number("missing", 3.0), 3.0);

    let registry = Registry::default();
    let mut world = World::new();
    registry.load(&mut world, "entity hero\n  transform pos=1,2,3\n").unwrap();
    engine::save::save_world(&registry, &world, &directory.join("slot1.save")).unwrap();
    let mut restored = World::new();
    restored.spawn();
    engine::save::load_world(&registry, &mut restored, &directory.join("slot1.save")).unwrap();
    assert_eq!(registry.checksum(&restored), registry.checksum(&world));
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn vector_text_is_antialiased_and_scales_with_resolution() {
    use engine::math::Vec3;
    use engine::render::{render, Framebuffer};
    use engine::scene::Scene;
    use engine::ui::{Align, Ui};

    let white = Vec3::new(1.0, 1.0, 1.0);
    let mut partial_counts = Vec::new();
    for (width, height) in [(640usize, 360usize), (1920, 1080)] {
        let mut ui = Ui::new();
        ui.set_target(width, height);
        ui.text(640.0, 300.0, "OIL YARD 0123456789", 6.0, white, Align::Center);
        let mut scene = Scene::empty(Vec3::new(0.0, 0.0, 0.0), 100.0);
        scene.overlay = ui.finish();
        let mut framebuffer = Framebuffer::new(width, height);
        render(&scene, &scene.camera, &mut framebuffer);
        let grey = |pixel: u32| (pixel & 255) as u32;
        let lit = framebuffer.pixels.iter().filter(|p| grey(**p) > 8).count();
        let partial = framebuffer.pixels.iter().filter(|p| grey(**p) > 24 && grey(**p) < 230).count();
        assert!(lit > 200, "text drew nothing at {}x{}", width, height);
        assert!(partial * 25 > lit && partial > 100, "edges are not anti-aliased at {}x{}", width, height);
        partial_counts.push(lit as f32 / (width * height) as f32);
    }
    assert!((partial_counts[0] - partial_counts[1]).abs() < partial_counts[0] * 0.3, "coverage should not depend on resolution");
}

#[test]
fn every_character_the_game_prints_has_an_outline() {
    let font = engine::font::font();
    for character in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,:;!?-+=/%()[]_*\"'<>#|".chars() {
        assert!(!font.glyph(character).strokes.is_empty(), "no outline for {}", character);
    }
    assert!(font.width("WASD") > font.width("IIII") * 0.5);
}
