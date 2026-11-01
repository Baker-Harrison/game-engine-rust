use crate::bodies::EYE_HEIGHT;
use crate::combat::ShotSpec;
use crate::state::{Fighter, Oilyard, MAX_HEALTH};
use crate::weapons::{spec, WeaponKind, CARBINE_RECOIL_PATTERN, WEAPONS};
use engine::camera::MAX_PITCH;
use engine::input::{Button, Input, Key};
use engine::math::Vec3;
use engine::physics::{step_body, Body, MoveParams};
use engine::runtime::{Engine, TICK_SECONDS};

pub const BASE_FOV: f32 = 1.2;
const MOUSE_SENSITIVITY: f32 = 0.0022;
const PAD_LOOK_SPEED: f32 = 2.8;
const WALK_SPEED: f32 = 5.0;
const SPRINT_MULTIPLIER: f32 = 1.5;
const SWITCH_TICKS: u32 = 18;
const REGEN_DELAY: f32 = 5.0;
const REGEN_RATE: f32 = 22.0;
const THROW_SPEED: f32 = 16.0;

pub fn move_params() -> MoveParams {
    MoveParams { gravity: 24.0, jump_speed: 7.4, ..MoveParams::default() }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Oilyard {
    pub fn weapon_kind(&self) -> WeaponKind {
        WEAPONS[self.ps.weapon].kind
    }

    fn select_weapon(&mut self, index: usize) {
        if index == self.ps.weapon || index >= WEAPONS.len() {
            return;
        }
        self.ps.weapon = index;
        self.ps.reload_ticks = 0;
        self.ps.switch_ticks = SWITCH_TICKS;
        self.ps.recoil_index = 0;
        self.ps.cooldown = 0;
    }

    fn start_reload(&mut self, engine: &Engine) {
        let index = self.ps.weapon;
        let full = self.ps.ammo[index] >= WEAPONS[index].magazine;
        if self.ps.reload_ticks > 0 || full || self.ps.reserve[index] == 0 {
            return;
        }
        self.ps.reload_ticks = WEAPONS[index].reload_ticks;
        self.play_flat(engine, |s| &s.reload, 0.6, 1.0);
    }

    fn finish_reload(&mut self) {
        let index = self.ps.weapon;
        let need = WEAPONS[index].magazine - self.ps.ammo[index];
        let take = need.min(self.ps.reserve[index]);
        self.ps.ammo[index] += take;
        self.ps.reserve[index] -= take;
    }

    pub fn update_player(&mut self, engine: &mut Engine, input: &Input) {
        let entity = self.player_entity();
        let dt = TICK_SECONDS;
        self.ps.previous_eye = self.ps.eye;
        let alive = match engine.world.get_mut::<Fighter>(entity) {
            Some(fighter) => {
                fighter.chain_timer -= dt;
                fighter.hurt_timer += dt;
                if fighter.alive && fighter.hurt_timer > REGEN_DELAY && fighter.health < MAX_HEALTH {
                    fighter.health = (fighter.health + REGEN_RATE * dt).min(MAX_HEALTH);
                }
                fighter.alive
            }
            None => return,
        };
        if !alive {
            let drop = self.ps.eye.y - (engine.world.get::<Body>(entity).map(|b| b.position.y).unwrap_or(0.0) + 0.35);
            self.ps.eye.y -= drop.max(0.0).min(3.0 * dt);
            self.ps.ads = 0.0;
            self.ps.reload_ticks = 0;
            return;
        }

        let sensitivity = MOUSE_SENSITIVITY * (1.0 - 0.45 * self.ps.ads);
        let camera = &mut engine.scene.camera;
        camera.yaw -= input.mouse_dx * sensitivity + input.pad.right_x * PAD_LOOK_SPEED * dt;
        camera.pitch = (camera.pitch - input.mouse_dy * sensitivity - input.pad.right_y * PAD_LOOK_SPEED * dt).clamp(-MAX_PITCH, MAX_PITCH);
        let (walk_forward, right) = (camera.walk_forward(), camera.right());

        let held = |key: Key| if input.key_down(key) { 1.0 } else { 0.0 };
        let forward_axis = held(Key::W) - held(Key::S) - input.pad.left_y;
        let strafe_axis = held(Key::D) - held(Key::A) + input.pad.left_x;
        let mut wish = walk_forward * forward_axis + right * strafe_axis;
        if wish.length() > 1.0 {
            wish = wish.normalized();
        }
        self.ps.sprinting = input.key_down(Key::Shift) && forward_axis > 0.5 && self.ps.ads < 0.3;
        let kind = self.weapon_kind();
        let weapon = spec(kind);
        let speed = WALK_SPEED
            * if self.ps.sprinting { SPRINT_MULTIPLIER } else { 1.0 }
            * (1.0 - 0.4 * self.ps.ads)
            * weapon.speed_factor;
        let mut body = engine.world.get::<Body>(entity).copied().unwrap_or(Body::new(Vec3::ZERO, 0.3, 1.8));
        step_body(&engine.collision, &mut body, wish * speed, input.key_pressed(Key::Space), dt, &move_params());
        self.ps.speed = Vec3::new(body.velocity.x, 0.0, body.velocity.z).length();
        let grounded = body.grounded;
        let feet = body.position;
        engine.world.insert(entity, body);
        self.ps.eye = feet + Vec3::new(0.0, EYE_HEIGHT, 0.0);

        self.update_weapon_controls(engine, input);
        self.update_firing(engine, input, grounded);
        self.update_aim_and_recoil(engine, input);
        self.update_footsteps(engine, grounded);
    }

    fn update_weapon_controls(&mut self, engine: &mut Engine, input: &Input) {
        let digits = [Key::Digit1, Key::Digit2, Key::Digit3];
        for (index, key) in digits.iter().enumerate() {
            if input.key_pressed(*key) {
                self.select_weapon(index);
            }
        }
        if input.wheel.abs() > 0.5 {
            let count = WEAPONS.len() as i32;
            let step = if input.wheel > 0.0 { -1 } else { 1 };
            self.select_weapon(((self.ps.weapon as i32 + step + count) % count) as usize);
        }
        if input.key_pressed(Key::R) || input.pad_pressed(engine::input::PadButton::West) {
            self.start_reload(engine);
        }
        if self.ps.reload_ticks > 0 {
            self.ps.reload_ticks -= 1;
            if self.ps.reload_ticks == 0 {
                self.finish_reload();
            }
        }
        self.ps.cooldown = self.ps.cooldown.saturating_sub(1);
        self.ps.switch_ticks = self.ps.switch_ticks.saturating_sub(1);
        self.ps.flash = self.ps.flash.saturating_sub(1);
        self.ps.grenade_cooldown = self.ps.grenade_cooldown.saturating_sub(1);
        self.ps.kick = (self.ps.kick - TICK_SECONDS * 7.0).max(0.0);
        if (input.key_pressed(Key::G) || input.pad_pressed(engine::input::PadButton::RightShoulder)) && self.ps.grenades > 0 && self.ps.grenade_cooldown == 0 {
            let camera = engine.scene.camera;
            let origin = camera.pos + camera.forward() * 0.5 + Vec3::new(0.0, -0.1, 0.0);
            let velocity = camera.forward() * THROW_SPEED + Vec3::new(0.0, 3.5, 0.0);
            let owner = self.player_entity();
            self.throw_grenade(owner, origin, velocity);
            self.ps.grenades -= 1;
            self.ps.grenade_cooldown = 40;
        }
    }

    fn update_firing(&mut self, engine: &mut Engine, input: &Input, grounded: bool) {
        let kind = self.weapon_kind();
        let weapon = spec(kind);
        let index = self.ps.weapon;
        let pad_trigger = input.pad.right_trigger > 0.5;
        let wants_fire = if weapon.automatic {
            input.button_down(Button::Left) || pad_trigger
        } else {
            input.button_pressed(Button::Left) || pad_trigger
        };
        let ready = self.ps.cooldown == 0 && self.ps.reload_ticks == 0 && self.ps.switch_ticks == 0 && !self.ps.sprinting;
        if !wants_fire || !ready {
            return;
        }
        if self.ps.ammo[index] == 0 {
            if input.button_pressed(Button::Left) {
                self.play_flat(engine, |s| &s.dry_fire, 0.5, 1.0);
            }
            self.start_reload(engine);
            return;
        }
        self.ps.ammo[index] -= 1;
        self.ps.cooldown = weapon.fire_interval_ticks;
        self.ps.flash = 2;
        self.ps.kick = 1.0;
        self.ps.shots_fired += 1;
        self.ps.idle_time = 0.0;
        let jitter = 1.0 + (engine.random.unit() - 0.5) * 0.06;
        match kind {
            WeaponKind::Carbine => self.play_flat(engine, |s| &s.rifle, 0.8, jitter),
            WeaponKind::Scattergun => self.play_flat(engine, |s| &s.shotgun, 0.9, jitter),
            WeaponKind::Sidearm => self.play_flat(engine, |s| &s.pistol, 0.75, jitter),
        }
        let mut spread = lerp(weapon.spread, weapon.aim_spread, self.ps.ads);
        spread *= 1.0 + self.ps.speed / WALK_SPEED * 0.8 + if grounded { 0.0 } else { 1.2 };
        let camera = engine.scene.camera;
        let direction = camera.forward();
        let hip = 1.0 - self.ps.ads;
        let muzzle = self.ps.eye + direction * 0.7 + camera.right() * (0.14 * hip) - camera.up() * (0.11 * hip);
        let shot = ShotSpec {
            pellets: weapon.pellets,
            spread,
            damage: weapon.damage,
            head_multiplier: weapon.head_multiplier,
            falloff_start: weapon.falloff_start,
            falloff_end: weapon.falloff_end,
            range: 220.0,
        };
        let shooter = self.player_entity();
        let eye = self.ps.eye;
        self.fire_shot(engine, shooter, eye, direction, muzzle, &shot);
        self.apply_recoil(engine);
    }

    fn apply_recoil(&mut self, engine: &mut Engine) {
        let weapon = spec(self.weapon_kind());
        let aim_scale = 1.0 - 0.4 * self.ps.ads;
        let yaw_pattern = if weapon.kind == WeaponKind::Carbine {
            CARBINE_RECOIL_PATTERN[self.ps.recoil_index % CARBINE_RECOIL_PATTERN.len()]
        } else {
            engine.random.range(-1.0, 1.0)
        };
        self.ps.recoil_index += 1;
        let pitch_kick = weapon.recoil_pitch * aim_scale * (1.0 + engine.random.range(-0.15, 0.15));
        let camera = &mut engine.scene.camera;
        camera.pitch = (camera.pitch + pitch_kick).min(MAX_PITCH);
        camera.yaw -= yaw_pattern * weapon.recoil_yaw * aim_scale;
        self.ps.unrecovered_pitch += pitch_kick;
    }

    fn update_aim_and_recoil(&mut self, engine: &mut Engine, input: &Input) {
        let dt = TICK_SECONDS;
        let wants_aim = (input.button_down(Button::Right) || input.pad.left_trigger > 0.5) && self.ps.reload_ticks == 0 && self.ps.switch_ticks == 0 && !self.ps.sprinting;
        let target = if wants_aim { 1.0 } else { 0.0 };
        let step = 7.0 * dt;
        self.ps.ads += (target - self.ps.ads).clamp(-step, step);
        engine.scene.camera.fov_y = lerp(BASE_FOV, spec(self.weapon_kind()).aim_fov, self.ps.ads);
        self.ps.idle_time += dt;
        if self.ps.idle_time > 0.12 {
            let recover = (0.45 * dt).min(self.ps.unrecovered_pitch);
            engine.scene.camera.pitch -= recover;
            self.ps.unrecovered_pitch -= recover;
        }
        if self.ps.idle_time > 0.35 {
            self.ps.recoil_index = 0;
        }
    }

    fn update_footsteps(&mut self, engine: &mut Engine, grounded: bool) {
        if !grounded || self.ps.speed < 0.8 {
            self.ps.step_timer = 0.2;
            return;
        }
        self.ps.bob_phase += self.ps.speed * TICK_SECONDS * 1.7;
        self.ps.step_timer -= TICK_SECONDS;
        if self.ps.step_timer <= 0.0 {
            self.ps.step_timer = if self.ps.sprinting { 0.32 } else { 0.46 };
            let pitch = 0.9 + engine.random.unit() * 0.2;
            self.play_flat(engine, |s| &s.step, 0.3, pitch);
        }
    }
}
