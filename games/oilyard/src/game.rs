use crate::bodies::{BotModel, EYE_HEIGHT};
use crate::level::{build_level, PickupKind, Spawn};
use crate::player::BASE_FOV;
use crate::sfx::Sounds;
use crate::state::*;
use crate::weapons::{Viewmodels, WEAPONS};
use engine::camera::Camera;
use engine::input::{Input, Key};
use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;
use engine::nav::{NavGraph, NavSettings};
use engine::physics::Body;
use engine::runtime::{Engine, Game, TICK_SECONDS};
use engine::scene::Scene;
use engine::worldfile::{Name, Transform};

const SKY: Vec3 = Vec3::new(0.62, 0.78, 0.95);
const FOG_DISTANCE: f32 = 260.0;
const PICKUP_RADIUS: f32 = 1.1;
const PICKUP_COOLDOWN: f32 = 20.0;
const HEALTH_PICKUP: f32 = 50.0;
const MAX_GRENADES: u32 = 3;
const BOT_SKILLS: [f32; BOT_COUNT] = [0.55, 0.65, 0.75, 0.6, 0.85];

fn health_pack_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    mesh.add_box(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.46, 0.28, 0.32), Vec3::new(0.9, 0.9, 0.88));
    mesh.add_box(Vec3::new(0.0, 0.0, 0.17), Vec3::new(0.26, 0.08, 0.02), Vec3::new(0.85, 0.1, 0.08));
    mesh.add_box(Vec3::new(0.0, 0.0, 0.17), Vec3::new(0.08, 0.2, 0.021), Vec3::new(0.85, 0.1, 0.08));
    mesh.add_box(Vec3::new(0.0, 0.0, -0.17), Vec3::new(0.26, 0.08, 0.02), Vec3::new(0.85, 0.1, 0.08));
    mesh.add_box(Vec3::new(0.0, 0.0, -0.17), Vec3::new(0.08, 0.2, 0.021), Vec3::new(0.85, 0.1, 0.08));
    mesh
}

fn ammo_pack_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    mesh.add_box(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.5, 0.26, 0.3), Vec3::new(0.25, 0.3, 0.18));
    mesh.add_box(Vec3::new(0.0, 0.135, 0.0), Vec3::new(0.42, 0.03, 0.22), Vec3::new(0.18, 0.22, 0.12));
    for k in 0..4 {
        mesh.add_box(Vec3::new(-0.15 + k as f32 * 0.1, 0.17, 0.0), Vec3::new(0.05, 0.06, 0.05), Vec3::new(0.85, 0.65, 0.2));
    }
    mesh
}

fn grenade_mesh() -> Mesh {
    let body = Sdf::sphere(0.055).union(Sdf::cylinder(0.018, 0.05, 0.004).translate(Vec3::new(0.0, 0.06, 0.0)));
    mesh_sdf(&body, 0.012, Vec3::new(0.2, 0.26, 0.15))
}

impl Oilyard {
    fn spawn_fighter(&mut self, engine: &mut Engine, name: &str, spawn: &Spawn) -> engine::ecs::Entity {
        let entity = engine.world.spawn();
        engine.world.insert(entity, Name(name.to_string()));
        engine.world.insert(entity, Fighter::new(name));
        engine.world.insert(entity, Body::new(spawn.position, crate::bodies::BODY_RADIUS, crate::bodies::STAND_HEIGHT));
        engine.world.insert(entity, Transform { position: spawn.position, yaw: spawn.yaw, scale: 1.0 });
        entity
    }

    pub fn pick_spawn(&self, engine: &Engine) -> Spawn {
        let others = self.living_targets(engine, None);
        let clearance = |spawn: &Spawn| others.iter().map(|t| (t.foot - spawn.position).length()).fold(f32::MAX, f32::min);
        *self.spawns.iter().max_by(|a, b| clearance(a).total_cmp(&clearance(b))).expect("level has spawns")
    }

    fn respawn(&mut self, engine: &mut Engine, entity: engine::ecs::Entity) {
        let spawn = self.pick_spawn(engine);
        engine.world.insert(entity, Body::new(spawn.position, crate::bodies::BODY_RADIUS, crate::bodies::STAND_HEIGHT));
        if let Some(fighter) = engine.world.get_mut::<Fighter>(entity) {
            fighter.alive = true;
            fighter.health = MAX_HEALTH;
            fighter.hurt_timer = 99.0;
            fighter.respawn_timer = 0.0;
        }
        if let Some(brain) = engine.world.get_mut::<Brain>(entity) {
            brain.reset();
            brain.yaw = spawn.yaw;
        }
        if Some(entity) == self.player {
            let fresh = PlayerState::new();
            self.ps.ammo = fresh.ammo;
            self.ps.reserve = fresh.reserve;
            self.ps.grenades = fresh.grenades;
            self.ps.reload_ticks = 0;
            self.ps.weapon = 0;
            self.ps.eye = spawn.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
            self.ps.previous_eye = self.ps.eye;
            engine.scene.camera.yaw = spawn.yaw;
            engine.scene.camera.pitch = 0.0;
            self.play_flat(engine, |s| &s.respawn, 0.6, 1.0);
        }
    }

    fn update_respawns(&mut self, engine: &mut Engine) {
        for entity in engine.world.with::<Fighter>() {
            let ready = match engine.world.get_mut::<Fighter>(entity) {
                Some(fighter) if !fighter.alive => {
                    fighter.respawn_timer -= TICK_SECONDS;
                    fighter.dead_for += TICK_SECONDS;
                    fighter.respawn_timer <= 0.0
                }
                _ => false,
            };
            if ready {
                self.respawn(engine, entity);
            }
        }
    }

    fn update_pickups(&mut self, engine: &mut Engine) {
        let fighters = self.living_targets(engine, None);
        for entity in engine.world.with::<Pickup>() {
            let Some(position) = engine.world.get::<Transform>(entity).map(|t| t.position) else { continue };
            let Some(pickup) = engine.world.get_mut::<Pickup>(entity) else { continue };
            if pickup.ready_in > 0.0 {
                pickup.ready_in -= TICK_SECONDS;
                continue;
            }
            let kind = pickup.kind;
            let taker = fighters.iter().find(|f| {
                let flat = Vec3::new(f.foot.x - position.x, 0.0, f.foot.z - position.z).length();
                flat < PICKUP_RADIUS && (f.foot.y - position.y).abs() < 1.6
            });
            let Some(taker) = taker else { continue };
            let is_player = Some(taker.entity) == self.player;
            let used = match kind {
                PickupKind::Health => match engine.world.get_mut::<Fighter>(taker.entity) {
                    Some(fighter) if fighter.health < MAX_HEALTH => {
                        fighter.health = (fighter.health + HEALTH_PICKUP).min(MAX_HEALTH);
                        true
                    }
                    _ => false,
                },
                PickupKind::Ammo => is_player && self.restock_player(),
            };
            if used {
                if let Some(pickup) = engine.world.get_mut::<Pickup>(entity) {
                    pickup.ready_in = PICKUP_COOLDOWN;
                }
                if is_player {
                    self.play_flat(engine, |s| &s.pickup, 0.7, 1.0);
                } else {
                    self.play_at(engine, |s| &s.pickup, position, 0.5, 1.0);
                }
            }
        }
    }

    fn restock_player(&mut self) -> bool {
        let mut changed = false;
        for (index, weapon) in WEAPONS.iter().enumerate() {
            if self.ps.reserve[index] < weapon.reserve {
                self.ps.reserve[index] = weapon.reserve;
                changed = true;
            }
        }
        if self.ps.grenades < MAX_GRENADES {
            self.ps.grenades += 1;
            changed = true;
        }
        changed
    }

    fn age_timers(&mut self) {
        let dt = TICK_SECONDS;
        self.hit_marker = (self.hit_marker - dt).max(0.0);
        self.shake = (self.shake - dt * 1.4).max(0.0);
        for line in &mut self.feed {
            line.age += dt;
        }
        self.feed.retain(|line| line.age < 5.5);
        for pop in &mut self.pops {
            pop.age += dt;
        }
        self.pops.retain(|pop| pop.age < 1.6);
        for indicator in &mut self.indicators {
            indicator.age += dt;
        }
        self.indicators.retain(|indicator| indicator.age < 1.5);
        if let Some((_, remaining)) = &mut self.banner {
            *remaining -= dt;
        }
        if self.banner.as_ref().map(|b| b.1 <= 0.0).unwrap_or(false) {
            self.banner = None;
        }
    }

    fn sync_transforms(&self, engine: &mut Engine) {
        for entity in engine.world.with::<Body>() {
            let Some(body) = engine.world.get::<Body>(entity).copied() else { continue };
            let yaw = engine.world.get::<Brain>(entity).map(|b| b.yaw).unwrap_or(engine.scene.camera.yaw);
            if let Some(transform) = engine.world.get_mut::<Transform>(entity) {
                transform.position = body.position;
                transform.yaw = yaw;
            }
        }
    }

    pub fn restart(&mut self, engine: &mut Engine) {
        self.match_number += 1;
        self.over = false;
        self.time_left = MATCH_SECONDS;
        self.grenades.clear();
        self.fx.clear();
        self.feed.clear();
        self.pops.clear();
        self.indicators.clear();
        self.banner = None;
        self.ps = PlayerState::new();
        for entity in engine.world.with::<Fighter>() {
            let name = engine.world.get::<Fighter>(entity).map(|f| f.name.clone()).unwrap_or_default();
            engine.world.insert(entity, Fighter::new(&name));
        }
        for entity in engine.world.with::<Pickup>() {
            if let Some(pickup) = engine.world.get_mut::<Pickup>(entity) {
                pickup.ready_in = 0.0;
            }
        }
        let fighters = engine.world.with::<Fighter>();
        for (index, entity) in fighters.iter().enumerate() {
            let spawn = self.spawns[index % self.spawns.len()];
            engine.world.insert(*entity, Body::new(spawn.position, crate::bodies::BODY_RADIUS, crate::bodies::STAND_HEIGHT));
            if let Some(brain) = engine.world.get_mut::<Brain>(*entity) {
                brain.reset();
                brain.yaw = spawn.yaw;
            }
            if Some(*entity) == self.player {
                engine.scene.camera.yaw = spawn.yaw;
                engine.scene.camera.pitch = 0.0;
                self.ps.eye = spawn.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
                self.ps.previous_eye = self.ps.eye;
            }
        }
        engine.note(format!("match {} started", self.match_number));
    }

    pub fn standings(&self, engine: &Engine) -> Vec<(String, u32, u32, i32, bool)> {
        let mut rows: Vec<(String, u32, u32, i32, bool)> = engine
            .world
            .with::<Fighter>()
            .into_iter()
            .filter_map(|e| engine.world.get::<Fighter>(e).map(|f| (f.name.clone(), f.kills, f.deaths, f.score, Some(e) == self.player)))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(b.3.cmp(&a.3)).then(a.0.cmp(&b.0)));
        rows
    }
}

impl Game for Oilyard {
    fn name(&self) -> &str {
        "oilyard"
    }

    fn setup(&mut self, engine: &mut Engine) -> Result<(), String> {
        engine.registry.register::<Fighter>();
        engine.registry.register::<Pickup>();
        let level = build_level(true);
        if level.spawns.len() < BOT_COUNT + 1 {
            return Err("level needs at least one spawn per fighter".into());
        }
        let facing_center = |spawn: &Spawn| Spawn { position: spawn.position, yaw: crate::bots::yaw_toward(Vec3::ZERO - spawn.position) };
        let spawns: Vec<Spawn> = level.spawns.iter().map(facing_center).collect();
        self.spawns = spawns.clone();
        self.map_rects = level.map_rects;
        let mut scene = Scene::from_mesh(&level.mesh, SKY);
        scene.fog = FOG_DISTANCE;
        scene.camera = Camera::new(spawns[0].position + Vec3::new(0.0, EYE_HEIGHT, 0.0));
        scene.camera.yaw = spawns[0].yaw;
        scene.camera.fov_y = BASE_FOV;
        engine.scene = scene;
        engine.scene_dirty = true;
        engine.collision = level.collision;
        engine.nav = NavGraph::build(&engine.collision, &NavSettings::default());
        engine.nav.largest_component_from(spawns[0].position);

        self.sounds = Some(Sounds::build());
        self.models = Some(Models {
            bot: BotModel::build(),
            viewmodels: Viewmodels::build()?,
            arms: crate::viewrig::ViewArms::build(),
            grenade: grenade_mesh(),
            health_pack: health_pack_mesh(),
            ammo_pack: ammo_pack_mesh(),
        });

        let player_spawn = spawns[0];
        let player = self.spawn_fighter(engine, "YOU", &player_spawn);
        self.player = Some(player);
        self.ps.eye = player_spawn.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        self.ps.previous_eye = self.ps.eye;
        for index in 0..BOT_COUNT {
            let spawn = spawns[(index + 1) % spawns.len()];
            let bot = self.spawn_fighter(engine, BOT_NAMES[index], &spawn);
            engine.world.insert(bot, Brain::new(BOT_SKILLS[index], BOT_COLORS[index], spawn.yaw));
        }
        for spot in &level.pickups {
            let entity = engine.world.spawn();
            engine.world.insert(entity, Pickup { kind: spot.kind, ready_in: 0.0 });
            engine.world.insert(entity, Transform::at(spot.position));
        }
        self.start_ambience(engine);
        engine.note("match 1 started".to_string());
        Ok(())
    }

    fn tick(&mut self, engine: &mut Engine, input: &Input) {
        self.intro_time += TICK_SECONDS;
        self.age_timers();
        self.fx.step(TICK_SECONDS);
        if self.over {
            if input.key_pressed(Key::Enter) {
                self.restart(engine);
            }
            return;
        }
        self.time_left -= TICK_SECONDS;
        if self.time_left <= 0.0 {
            self.time_left = 0.0;
            self.end_match(engine);
            return;
        }
        self.update_player(engine, input);
        self.update_bots(engine);
        self.update_grenades(engine);
        self.update_pickups(engine);
        self.update_respawns(engine);
        self.noises.clear();
        self.sync_transforms(engine);
    }

    fn draw(&mut self, engine: &mut Engine, blend: f32) {
        self.compose_frame(engine, blend);
    }

    fn report(&self, engine: &Engine) -> String {
        let mut text = String::new();
        let time = engine.time.seconds();
        text.push_str(&format!("match {} | {:.1}s elapsed | {:.1}s left | over={}\n", self.match_number, time, self.time_left, self.over));
        if let Some(body) = self.player.and_then(|p| engine.world.get::<Body>(p)) {
            text.push_str(&format!("player at {:.1} {:.1} {:.1}\n", body.position.x, body.position.y, body.position.z));
        }
        text.push_str(&format!(
            "weapon {} ammo {}/{} grenades {} shots {} hits {}\n",
            WEAPONS[self.ps.weapon].name, self.ps.ammo[self.ps.weapon], self.ps.reserve[self.ps.weapon], self.ps.grenades, self.ps.shots_fired, self.ps.shots_hit
        ));
        text.push_str("standings:\n");
        for (name, kills, deaths, score, _) in self.standings(engine) {
            text.push_str(&format!("  {:6} kills {:2} deaths {:2} score {}\n", name, kills, deaths, score));
        }
        for entity in engine.world.with::<Brain>() {
            if let (Some(brain), Some(body), Some(fighter)) = (engine.world.get::<Brain>(entity), engine.world.get::<Body>(entity), engine.world.get::<Fighter>(entity)) {
                text.push_str(&format!(
                    "  bot {:6} {:?} hp {:3.0} alive {} at {:.1} {:.1} {:.1}\n",
                    fighter.name, brain.mode, fighter.health, fighter.alive, body.position.x, body.position.y, body.position.z
                ));
            }
        }
        text.push_str(&format!("nav nodes {} | dynamic triangles ready\n", engine.nav.nodes.len()));
        text.push_str("events:\n");
        for line in engine.log.iter().rev().take(14).rev() {
            text.push_str(&format!("  {}\n", line));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::headless::{simulate, SimOptions};
    use engine::physics::CollisionWorld;

    fn nav_for(collision: &CollisionWorld, start: Vec3) -> NavGraph {
        let mut nav = NavGraph::build(collision, &NavSettings::default());
        nav.largest_component_from(start);
        nav
    }

    #[test]
    fn every_spawn_and_pickup_is_reachable_on_the_nav_graph() {
        let level = build_level(false);
        let nav = nav_for(&level.collision, level.spawns[0].position);
        let near_node = |position: Vec3| {
            nav.nearest(position)
                .map(|index| {
                    let node = nav.nodes[index].position;
                    Vec3::new(node.x - position.x, 0.0, node.z - position.z).length() < 1.2 && (node.y - position.y).abs() < 1.5
                })
                .unwrap_or(false)
        };
        for spawn in &level.spawns {
            assert!(near_node(spawn.position), "spawn not on nav graph: {:?}", spawn.position);
        }
        for pickup in &level.pickups {
            assert!(near_node(pickup.position), "pickup not on nav graph: {:?}", pickup.position);
        }
        let vantage_points = [
            Vec3::new(0.0, 4.2, 0.0),
            Vec3::new(-26.0, 4.2, -15.0),
            Vec3::new(-26.0, 4.2, -8.0),
            Vec3::new(-31.0, 2.6, -26.5),
            Vec3::new(-6.0, 3.5, 24.0),
            Vec3::new(26.0, 4.2, 15.0),
        ];
        for point in vantage_points {
            assert!(near_node(point), "vantage point not reachable: {:?}", point);
        }
        for pair in level.spawns.windows(2) {
            assert!(nav.find_path(pair[0].position, pair[1].position).is_some(), "no path between spawns");
        }
    }

    #[test]
    fn bot_match_is_deterministic_and_produces_kills() {
        let options = SimOptions { ticks: 1500, seed: 7, ..SimOptions::default() };
        let first = simulate(&mut Oilyard::new(), &options).expect("first run");
        let second = simulate(&mut Oilyard::new(), &options).expect("second run");
        assert_eq!(first.checksum, second.checksum);
        assert!(first.report.contains("eliminated"), "bots never scored:\n{}", first.report);
        let other = simulate(&mut Oilyard::new(), &SimOptions { seed: 8, ..options }).expect("other seed");
        assert_ne!(first.checksum, other.checksum);
    }
}
