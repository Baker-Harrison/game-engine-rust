use crate::bodies::{BODY_RADIUS, CHEST_HEIGHT, HEAD_CENTER, HEAD_RADIUS};
use crate::state::{Brain, FeedLine, Fighter, Grenade, Indicator, Noise, Oilyard, ScorePop, KILL_LIMIT, RESPAWN_SECONDS};
use engine::ecs::Entity;
use engine::math::Vec3;
use engine::physics::{ray_aabb, ray_sphere, Aabb, Body};
use engine::runtime::{Engine, TICK_SECONDS};

const GRENADE_RADIUS: f32 = 7.5;
const GRENADE_DAMAGE: f32 = 140.0;
const BODY_TOP: f32 = 1.5;

#[derive(Clone, Copy)]
pub struct ShotSpec {
    pub pellets: u32,
    pub spread: f32,
    pub damage: f32,
    pub head_multiplier: f32,
    pub falloff_start: f32,
    pub falloff_end: f32,
    pub range: f32,
}

pub struct Target {
    pub entity: Entity,
    pub foot: Vec3,
}

enum Strike {
    Wall { distance: f32, point: Vec3, normal: Vec3 },
    Fighter { entity: Entity, distance: f32, point: Vec3, head: bool },
    Nothing,
}

fn falloff(damage: f32, start: f32, end: f32, distance: f32) -> f32 {
    if distance <= start {
        return damage;
    }
    let t = ((distance - start) / (end - start)).clamp(0.0, 1.0);
    damage * (1.0 - 0.6 * t)
}

fn jitter(direction: Vec3, spread: f32, engine: &mut Engine) -> Vec3 {
    if spread <= 0.0 {
        return direction;
    }
    let right = direction.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
    let up = right.cross(direction);
    let angle = engine.random.unit() * std::f32::consts::TAU;
    let radius = engine.random.unit().sqrt() * spread;
    (direction + right * (angle.cos() * radius) + up * (angle.sin() * radius)).normalized()
}

impl Oilyard {
    pub fn living_targets(&self, engine: &Engine, exclude: Option<Entity>) -> Vec<Target> {
        let mut targets = Vec::new();
        for entity in engine.world.with::<Fighter>() {
            let alive = engine.world.get::<Fighter>(entity).map(|f| f.alive).unwrap_or(false);
            if !alive || Some(entity) == exclude {
                continue;
            }
            if let Some(body) = engine.world.get::<Body>(entity) {
                targets.push(Target { entity, foot: body.position });
            }
        }
        targets
    }

    fn trace(&self, engine: &Engine, targets: &[Target], origin: Vec3, direction: Vec3, range: f32) -> Strike {
        let wall = engine.collision.raycast(origin, direction, range);
        let wall_distance = wall.as_ref().map(|hit| hit.distance).unwrap_or(range);
        let mut best: Option<(f32, Entity, bool)> = None;
        for target in targets {
            let head_center = target.foot + Vec3::new(0.0, HEAD_CENTER, 0.0);
            if let Some(distance) = ray_sphere(origin, direction, head_center, HEAD_RADIUS, wall_distance) {
                if best.map(|b| distance < b.0).unwrap_or(true) {
                    best = Some((distance, target.entity, true));
                }
            }
            let low = target.foot + Vec3::new(-BODY_RADIUS * 0.9, 0.0, -BODY_RADIUS * 0.9);
            let high = target.foot + Vec3::new(BODY_RADIUS * 0.9, BODY_TOP, BODY_RADIUS * 0.9);
            if let Some((distance, _)) = ray_aabb(origin, direction, &Aabb::new(low, high), wall_distance) {
                if best.map(|b| distance < b.0).unwrap_or(true) {
                    best = Some((distance, target.entity, false));
                }
            }
        }
        match (best, wall) {
            (Some((distance, entity, head)), _) => Strike::Fighter { entity, distance, point: origin + direction * distance, head },
            (None, Some(hit)) => Strike::Wall { distance: hit.distance, point: hit.point, normal: hit.normal },
            (None, None) => Strike::Nothing,
        }
    }

    pub fn fire_shot(&mut self, engine: &mut Engine, shooter: Entity, origin: Vec3, direction: Vec3, muzzle: Vec3, shot: &ShotSpec) {
        let targets = self.living_targets(engine, Some(shooter));
        let by_player = Some(shooter) == self.player;
        for _ in 0..shot.pellets {
            let aim = jitter(direction, shot.spread, engine);
            match self.trace(engine, &targets, origin, aim, shot.range) {
                Strike::Fighter { entity, distance, point, head } => {
                    let base = falloff(shot.damage, shot.falloff_start, shot.falloff_end, distance);
                    let amount = if head { base * shot.head_multiplier } else { base };
                    self.fx.tracer(muzzle, point);
                    if by_player {
                        self.ps.shots_hit += 1;
                    }
                    self.hurt(engine, entity, shooter, amount, head, point, aim);
                }
                Strike::Wall { distance, point, normal } => {
                    self.fx.tracer(muzzle, point);
                    self.fx.dust(point, normal, &mut engine.random);
                    if distance < 90.0 {
                        self.fx.bullet_hole(point, normal, 0.035);
                    }
                }
                Strike::Nothing => self.fx.tracer(muzzle, origin + aim * shot.range),
            }
        }
        self.noises.push(Noise { position: origin, source: shooter, radius: 50.0 });
    }

    pub fn hurt(&mut self, engine: &mut Engine, victim: Entity, attacker: Entity, amount: f32, head: bool, point: Vec3, direction: Vec3) {
        let alive = match engine.world.get_mut::<Fighter>(victim) {
            Some(fighter) if fighter.alive => {
                fighter.health -= amount;
                fighter.hurt_timer = 0.0;
                fighter.health > 0.0
            }
            _ => return,
        };
        self.fx.blood(point, direction, &mut engine.random);
        let attacker_position = engine.world.get::<Body>(attacker).map(|b| b.position).unwrap_or(point);
        if Some(victim) == self.player {
            let from = engine.world.get::<Body>(victim).map(|b| b.position).unwrap_or(Vec3::ZERO);
            self.indicators.push(Indicator { direction: attacker_position - from, age: 0.0 });
            self.shake = (self.shake + amount * 0.012).min(0.5);
            self.play_flat(engine, |s| &s.hurt, 0.7, 1.0);
        } else if let Some(brain) = engine.world.get_mut::<Brain>(victim) {
            brain.last_seen = attacker_position;
            brain.memory = brain.memory.max(4.0);
            brain.target = Some(attacker);
        }
        if Some(attacker) == self.player && attacker != victim {
            self.hit_marker = 0.22;
            self.hit_was_kill = false;
            if head {
                self.play_flat(engine, |s| &s.headshot, 0.8, 1.0);
            } else {
                self.play_flat(engine, |s| &s.hit, 0.6, 1.0);
            }
        }
        if !alive {
            self.kill(engine, victim, attacker, head);
        }
    }

    fn name_of(engine: &Engine, entity: Entity) -> String {
        engine.world.get::<Fighter>(entity).map(|f| f.name.clone()).unwrap_or_default()
    }

    fn kill(&mut self, engine: &mut Engine, victim: Entity, attacker: Entity, head: bool) {
        let victim_name = Oilyard::name_of(engine, victim);
        let attacker_name = Oilyard::name_of(engine, attacker);
        if let Some(fighter) = engine.world.get_mut::<Fighter>(victim) {
            fighter.alive = false;
            fighter.health = 0.0;
            fighter.deaths += 1;
            fighter.streak = 0;
            fighter.respawn_timer = RESPAWN_SECONDS;
            fighter.dead_for = 0.0;
        }
        let by_player = Some(attacker) == self.player;
        if victim == attacker {
            if let Some(fighter) = engine.world.get_mut::<Fighter>(attacker) {
                fighter.score -= 100;
            }
            self.push_feed(format!("{} BLEW HIMSELF UP", victim_name), Some(victim) == self.player);
        } else {
            let mut points = 100 + if head { 50 } else { 0 };
            let mut chain_bonus = 0;
            let mut streak_bonus = 0;
            let mut streak = 0;
            let mut chain = 0;
            if let Some(fighter) = engine.world.get_mut::<Fighter>(attacker) {
                fighter.kills += 1;
                fighter.streak += 1;
                fighter.chain = if fighter.chain_timer > 0.0 { fighter.chain + 1 } else { 1 };
                fighter.chain_timer = 4.0;
                chain = fighter.chain;
                chain_bonus = 25 * (fighter.chain as i32 - 1);
                streak = fighter.streak;
                streak_bonus = match fighter.streak {
                    3 => 100,
                    5 => 200,
                    7 => 300,
                    _ => 0,
                };
                points += chain_bonus + streak_bonus;
                fighter.score += points;
            }
            let marker = if head { " [HEADSHOT]" } else { "" };
            self.push_feed(format!("{} > {}{}", attacker_name, victim_name, marker), by_player || Some(victim) == self.player);
            if by_player {
                self.hit_marker = 0.5;
                self.hit_was_kill = true;
                self.play_flat(engine, |s| &s.kill, 0.9, 1.0);
                let label = if head { "HEADSHOT" } else { "ENEMY DOWN" };
                self.pops.push(ScorePop { text: format!("{} +{}", label, 100 + if head { 50 } else { 0 }), age: 0.0, accent: head });
                if chain_bonus > 0 {
                    self.pops.push(ScorePop { text: format!("CHAIN X{} +{}", chain, chain_bonus), age: 0.0, accent: false });
                }
                if streak_bonus > 0 {
                    self.banner = Some((format!("{} KILL STREAK +{}", streak, streak_bonus), 2.5));
                    self.play_flat(engine, |s| &s.streak, 0.8, 1.0);
                }
            }
        }
        if Some(victim) == self.player {
            self.killer_name = if victim == attacker { "YOURSELF".to_string() } else { attacker_name.clone() };
            self.shake = 0.5;
        }
        engine.note(format!("{} eliminated {}{}", attacker_name, victim_name, if head { " (headshot)" } else { "" }));
        let kills = engine.world.get::<Fighter>(attacker).map(|f| f.kills).unwrap_or(0);
        if kills >= KILL_LIMIT {
            self.end_match(engine);
        }
    }

    pub fn push_feed(&mut self, text: String, involves_player: bool) {
        self.feed.push(FeedLine { text, age: 0.0, involves_player });
        if self.feed.len() > 6 {
            self.feed.remove(0);
        }
    }

    pub fn end_match(&mut self, engine: &mut Engine) {
        if self.over {
            return;
        }
        self.over = true;
        let mut best: Option<(u32, i32, String)> = None;
        for entity in engine.world.with::<Fighter>() {
            if let Some(f) = engine.world.get::<Fighter>(entity) {
                let key = (f.kills, f.score);
                if best.as_ref().map(|b| key > (b.0, b.1)).unwrap_or(true) {
                    best = Some((f.kills, f.score, f.name.clone()));
                }
            }
        }
        self.winner = best.map(|b| b.2).unwrap_or_default();
        engine.note(format!("match over, winner {}", self.winner));
        self.play_flat(engine, |s| &s.match_end, 0.9, 1.0);
    }

    pub fn throw_grenade(&mut self, owner: Entity, origin: Vec3, velocity: Vec3) {
        let projectile = engine::physics::Projectile::new(origin, velocity, 0.08, 0.45);
        self.grenades.push(Grenade { projectile, fuse: 2.2, owner });
    }

    pub fn update_grenades(&mut self, engine: &mut Engine) {
        let mut bounces = Vec::new();
        let mut blasts = Vec::new();
        for grenade in &mut self.grenades {
            grenade.fuse -= TICK_SECONDS;
            if grenade.projectile.step(&engine.collision, 18.0, TICK_SECONDS) {
                bounces.push(grenade.projectile.position);
            }
            if grenade.fuse <= 0.0 {
                blasts.push((grenade.projectile.position, grenade.owner));
            }
        }
        self.grenades.retain(|g| g.fuse > 0.0);
        for position in bounces {
            self.play_at(engine, |s| &s.bounce, position, 0.5, 1.0);
        }
        for (position, owner) in blasts {
            self.explode(engine, position, owner);
        }
    }

    fn explode(&mut self, engine: &mut Engine, position: Vec3, owner: Entity) {
        self.fx.explosion(position, &mut engine.random);
        self.play_at(engine, |s| &s.explosion, position, 1.0, 1.0);
        self.noises.push(Noise { position, source: owner, radius: 70.0 });
        let center = position + Vec3::new(0.0, 0.25, 0.0);
        let mut victims = Vec::new();
        for target in self.living_targets(engine, None) {
            let chest = target.foot + Vec3::new(0.0, CHEST_HEIGHT * 0.8, 0.0);
            let distance = (chest - center).length();
            if distance > GRENADE_RADIUS {
                continue;
            }
            if distance > 1.5 && !engine.collision.line_of_sight(center, chest) {
                continue;
            }
            let falloff = (1.0 - distance / GRENADE_RADIUS).powf(1.2);
            victims.push((target.entity, GRENADE_DAMAGE * falloff, chest));
        }
        for (victim, amount, chest) in victims {
            let direction = (chest - center).normalized();
            self.hurt(engine, victim, owner, amount, false, chest, direction);
        }
        let listener = engine.scene.camera.pos;
        let distance = (listener - position).length();
        self.shake = (self.shake + (1.0 - distance / 35.0).max(0.0) * 0.6).min(0.8);
    }
}
