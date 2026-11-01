use crate::bodies::{CHEST_HEIGHT, EYE_HEIGHT, HEAD_CENTER};
use crate::combat::ShotSpec;
use crate::level::PickupKind;
use crate::state::{BotMode, Brain, Fighter, Oilyard, Pickup};
use engine::ecs::Entity;
use engine::math::Vec3;
use engine::physics::{step_body, Body, MoveParams};
use engine::runtime::{Engine, TICK_SECONDS};
use std::f32::consts::{PI, TAU};

const ROAM_SPEED: f32 = 3.8;
const HUNT_SPEED: f32 = 4.6;
const FLEE_SPEED: f32 = 5.2;
const STRAFE_SPEED: f32 = 2.6;
const SIGHT_RANGE: f32 = 70.0;
const CLOSE_AWARENESS: f32 = 7.0;
const FIELD_OF_VIEW_COS: f32 = 0.3;
const FIRE_INTERVAL_TICKS: u32 = 7;
const BOT_DAMAGE: f32 = 8.0;

pub fn yaw_toward(direction: Vec3) -> f32 {
    (-direction.x).atan2(-direction.z)
}

fn facing(yaw: f32) -> Vec3 {
    Vec3::new(-yaw.sin(), 0.0, -yaw.cos())
}

fn angle_difference(from: f32, to: f32) -> f32 {
    (to - from + PI).rem_euclid(TAU) - PI
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

fn follow_path(engine: &mut Engine, brain: &mut Brain, body: &Body, goal: Vec3) -> Vec3 {
    brain.repath_timer -= TICK_SECONDS;
    let goal_moved = brain.path.last().map(|end| (*end - goal).length() > 2.5).unwrap_or(true);
    let exhausted = brain.path_index >= brain.path.len();
    if brain.repath_timer <= 0.0 && (goal_moved || exhausted) {
        brain.repath_timer = 0.8;
        brain.path = engine.nav.find_path(body.position, goal).unwrap_or_default();
        brain.path_index = 0;
    }
    while brain.path_index < brain.path.len() {
        let waypoint = brain.path[brain.path_index];
        let offset = flat(waypoint - body.position);
        if offset.length() < 0.55 && (waypoint.y - body.position.y).abs() < 1.2 {
            brain.path_index += 1;
        } else {
            return offset.normalized();
        }
    }
    Vec3::ZERO
}

impl Oilyard {
    pub fn update_bots(&mut self, engine: &mut Engine) {
        for entity in engine.world.with::<Brain>() {
            self.update_bot(engine, entity);
        }
    }

    fn pick_flee_goal(&self, engine: &Engine, from: Vec3, danger: Vec3) -> Vec3 {
        let mut best: Option<(f32, Vec3)> = None;
        for entity in engine.world.with::<Pickup>() {
            let (Some(pickup), Some(transform)) = (engine.world.get::<Pickup>(entity), engine.world.get::<engine::worldfile::Transform>(entity)) else { continue };
            if pickup.kind == PickupKind::Health && pickup.ready_in <= 0.0 {
                let distance = (transform.position - from).length();
                if best.map(|b| distance < b.0).unwrap_or(true) {
                    best = Some((distance, transform.position));
                }
            }
        }
        if let Some((distance, position)) = best {
            if distance < 45.0 {
                return position;
            }
        }
        self.spawns.iter().map(|s| s.position).max_by(|a, b| (*a - danger).length().total_cmp(&(*b - danger).length())).unwrap_or(from)
    }

    fn update_bot(&mut self, engine: &mut Engine, entity: Entity) {
        let dt = TICK_SECONDS;
        let (Some(mut brain), Some(mut body)) = (engine.world.get::<Brain>(entity).cloned(), engine.world.get::<Body>(entity).copied()) else { return };
        let health_fraction = match engine.world.get_mut::<Fighter>(entity) {
            Some(fighter) => {
                fighter.chain_timer -= dt;
                fighter.hurt_timer += dt;
                if !fighter.alive {
                    return;
                }
                fighter.health / crate::state::MAX_HEALTH
            }
            None => return,
        };
        let eye = body.position + Vec3::new(0.0, EYE_HEIGHT, 0.0);
        let targets = self.living_targets(engine, Some(entity));
        let mut visible: Option<(Entity, Vec3, f32)> = None;
        for target in &targets {
            let aim_point = target.foot + Vec3::new(0.0, CHEST_HEIGHT, 0.0);
            let to = aim_point - eye;
            let distance = to.length();
            if distance > SIGHT_RANGE || visible.map(|v| distance >= v.2).unwrap_or(false) {
                continue;
            }
            let in_view = flat(to).normalized().dot(facing(brain.yaw)) > FIELD_OF_VIEW_COS || distance < CLOSE_AWARENESS;
            if in_view && engine.collision.line_of_sight(eye, aim_point) {
                visible = Some((target.entity, target.foot, distance));
            }
        }
        match visible {
            Some((target, foot, _)) => {
                brain.target = Some(target);
                brain.seen_for += dt;
                brain.memory = 4.0;
                brain.last_seen = foot;
            }
            None => {
                brain.seen_for = 0.0;
                brain.memory = (brain.memory - dt).max(0.0);
                for noise in &self.noises {
                    let reach = noise.radius * (0.5 + 0.5 * brain.skill);
                    if noise.source != entity && (noise.position - body.position).length() < reach {
                        brain.last_seen = noise.position;
                        brain.memory = brain.memory.max(3.0);
                    }
                }
            }
        }

        let reaction = 0.55 - 0.35 * brain.skill;
        if brain.flee_timer > 0.0 {
            brain.flee_timer -= dt;
        } else if health_fraction < 0.3 && brain.memory > 0.0 {
            brain.flee_timer = 5.0;
            brain.path.clear();
        }
        let mode = if brain.flee_timer > 0.0 {
            BotMode::Flee
        } else if visible.is_some() && brain.seen_for >= reaction {
            BotMode::Engage
        } else if brain.memory > 0.0 {
            BotMode::Hunt
        } else {
            BotMode::Roam
        };
        if mode != brain.mode {
            brain.path.clear();
            brain.repath_timer = 0.0;
        }
        brain.mode = mode;

        let mut wish = Vec3::ZERO;
        let mut speed = ROAM_SPEED;
        match mode {
            BotMode::Roam => {
                if brain.path_index >= brain.path.len() {
                    if let Some(goal) = engine.nav.random_node(&mut engine.random) {
                        brain.path = engine.nav.find_path(body.position, goal).unwrap_or_default();
                        brain.path_index = 0;
                    }
                }
                let goal = brain.path.last().copied().unwrap_or(body.position);
                wish = follow_path(engine, &mut brain, &body, goal);
            }
            BotMode::Hunt => {
                speed = HUNT_SPEED;
                if flat(brain.last_seen - body.position).length() < 2.5 {
                    brain.memory = 0.0;
                } else {
                    let goal = brain.last_seen;
                    wish = follow_path(engine, &mut brain, &body, goal);
                }
            }
            BotMode::Flee => {
                speed = FLEE_SPEED;
                let danger = brain.last_seen;
                let goal = brain.path.last().copied().unwrap_or_else(|| self.pick_flee_goal(engine, body.position, danger));
                wish = follow_path(engine, &mut brain, &body, goal);
            }
            BotMode::Engage => {
                let foot = visible.map(|v| v.1).unwrap_or(brain.last_seen);
                let to = flat(foot - body.position);
                let distance = to.length();
                brain.strafe_timer -= dt;
                if brain.strafe_timer <= 0.0 || body.bumped {
                    brain.strafe = if engine.random.unit() < 0.5 { -1.0 } else { 1.0 };
                    brain.strafe_timer = engine.random.range(0.7, 1.8);
                }
                let side = Vec3::new(-to.z, 0.0, to.x).normalized();
                if distance > 26.0 {
                    speed = HUNT_SPEED;
                    wish = follow_path(engine, &mut brain, &body, foot);
                } else {
                    speed = STRAFE_SPEED;
                    let range_push = if distance < 6.0 { -0.8 } else if distance > 16.0 { 0.5 } else { 0.0 };
                    wish = (side * brain.strafe + to.normalized() * range_push).normalized();
                }
            }
        }

        let target_yaw = match (mode, visible) {
            (BotMode::Engage, Some((_, foot, _))) => yaw_toward(foot - body.position),
            _ if wish.length() > 0.1 => yaw_toward(wish),
            _ => brain.yaw,
        };
        let turn_rate = 3.0 + 4.5 * brain.skill;
        let yaw_error = angle_difference(brain.yaw, target_yaw);
        brain.yaw += yaw_error.clamp(-turn_rate * dt, turn_rate * dt);

        if mode == BotMode::Engage {
            if let Some((_, foot, distance)) = visible {
                let aim_error = angle_difference(brain.yaw, target_yaw).abs();
                self.bot_shoot(engine, entity, &mut brain, eye, foot, distance, aim_error);
            }
        }
        brain.fire_cooldown = brain.fire_cooldown.saturating_sub(1);
        brain.flash = brain.flash.saturating_sub(1);

        let params = MoveParams { jump_speed: 7.0, ..MoveParams::default() };
        step_body(&engine.collision, &mut body, wish * speed, false, dt, &params);
        let horizontal = Vec3::new(body.velocity.x, 0.0, body.velocity.z).length();
        brain.speed_fraction = (horizontal / ROAM_SPEED).min(1.4);
        brain.walk_phase += horizontal * dt * 2.6;

        if wish.length() > 0.1 {
            brain.stuck_timer += dt;
            if brain.stuck_timer > 1.5 {
                if flat(body.position - brain.stuck_origin).length() < 0.4 {
                    brain.path.clear();
                    brain.repath_timer = 0.0;
                    brain.strafe = -brain.strafe;
                    if mode == BotMode::Roam {
                        brain.path_index = 0;
                    }
                }
                brain.stuck_timer = 0.0;
                brain.stuck_origin = body.position;
            }
        } else {
            brain.stuck_timer = 0.0;
            brain.stuck_origin = body.position;
        }

        brain.step_timer -= dt;
        if horizontal > 1.0 && brain.step_timer <= 0.0 && body.grounded {
            brain.step_timer = 0.5;
            let pitch = 0.9 + engine.random.unit() * 0.2;
            self.play_at(engine, |s| &s.step, body.position, 0.4, pitch);
        }

        engine.world.insert(entity, body);
        engine.world.insert(entity, brain);
    }

    fn bot_shoot(&mut self, engine: &mut Engine, entity: Entity, brain: &mut Brain, eye: Vec3, foot: Vec3, distance: f32, yaw_error: f32) {
        if brain.burst_pause > 0.0 {
            brain.burst_pause -= TICK_SECONDS;
            return;
        }
        let aim_height = if brain.skill > 0.75 && engine.random.unit() < 0.3 { HEAD_CENTER } else { CHEST_HEIGHT };
        let aim_point = foot + Vec3::new(0.0, aim_height, 0.0);
        if yaw_error > 0.1 || brain.fire_cooldown > 0 || !engine.collision.line_of_sight(eye, aim_point) {
            return;
        }
        if brain.burst_left == 0 {
            brain.burst_left = 3 + engine.random.next_u32() % 5;
        }
        brain.burst_left -= 1;
        brain.fire_cooldown = FIRE_INTERVAL_TICKS;
        brain.flash = 2;
        if brain.burst_left == 0 {
            brain.burst_pause = 0.3 + (1.0 - brain.skill) * 0.8 + engine.random.unit() * 0.4;
        }
        let to = aim_point - eye;
        let pitch = (to.y / flat(to).length().max(0.01)).atan();
        let direction = Vec3::new(-brain.yaw.sin() * pitch.cos(), pitch.sin(), -brain.yaw.cos() * pitch.cos());
        let right = Vec3::new(brain.yaw.cos(), 0.0, -brain.yaw.sin());
        let muzzle = eye + direction * 0.7 + right * 0.2 - Vec3::new(0.0, 0.25, 0.0);
        let shot = ShotSpec {
            pellets: 1,
            spread: 0.018 + 0.03 * (1.0 - brain.skill) + distance * 0.0004,
            damage: BOT_DAMAGE,
            head_multiplier: 1.5,
            falloff_start: 30.0,
            falloff_end: 80.0,
            range: 120.0,
        };
        self.fire_shot(engine, entity, eye, direction, muzzle, &shot);
        let listener_distance = (engine.scene.camera.pos - muzzle).length();
        let jitter = 0.95 + engine.random.unit() * 0.1;
        if listener_distance > 45.0 {
            self.play_at(engine, |s| &s.distant_shot, muzzle, 0.7, jitter);
        } else {
            self.play_at(engine, |s| &s.rifle, muzzle, 0.6, jitter);
        }
    }
}
