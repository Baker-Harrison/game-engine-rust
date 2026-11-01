use crate::bodies::STAND_HEIGHT;
use crate::state::{Brain, Fighter, Oilyard, KILL_LIMIT, MAX_HEALTH};
use crate::weapons::{spec, WeaponKind, WEAPONS};
use engine::math::Vec3;
use engine::physics::Body;
use engine::runtime::Engine;
use engine::ui::{Align, CANVAS_HEIGHT, CANVAS_WIDTH, GREEN, ORANGE, RED, WHITE, YELLOW};

const MAP_ORIGIN: (f32, f32) = (20.0, 190.0);
const MAP_SCALE: f32 = 2.0;
const MAP_HALF: (f32, f32) = (44.0, 34.0);
const PANEL: Vec3 = Vec3::new(0.0, 0.0, 0.0);
const INTRO_SECONDS: f32 = 9.0;

impl Oilyard {
    pub fn draw_hud(&mut self, engine: &mut Engine) {
        let Some(player) = self.player else { return };
        let (alive, health, hurt_timer, respawn_timer) = match engine.world.get::<Fighter>(player) {
            Some(f) => (f.alive, f.health, f.hurt_timer, f.respawn_timer),
            None => return,
        };
        if alive && !self.over {
            self.draw_reticle(engine);
            self.draw_ammo(engine);
        }
        self.draw_damage_feedback(engine, health, hurt_timer);
        self.draw_health(engine, health);
        self.draw_clock_and_score(engine);
        self.draw_scoreboard(engine);
        self.draw_minimap(engine);
        self.draw_feed(engine);
        self.draw_pops(engine);
        if !alive && !self.over {
            let ui = &mut engine.ui;
            ui.fade(PANEL, 0.35);
            ui.text(CANVAS_WIDTH * 0.5, 280.0, &format!("ELIMINATED BY {}", self.killer_name), 4.0, RED, Align::Center);
            ui.text(CANVAS_WIDTH * 0.5, 340.0, &format!("RESPAWN IN {}", respawn_timer.ceil().max(0.0) as i32), 3.0, WHITE, Align::Center);
        }
        if self.intro_time < INTRO_SECONDS && !self.over {
            let ui = &mut engine.ui;
            let lines = ["WASD MOVE   SHIFT SPRINT   SPACE JUMP", "MOUSE AIM   LMB FIRE   RMB AIM DOWN SIGHTS   R RELOAD", "1 2 3 OR WHEEL SWITCH WEAPON   G GRENADE   TAB RELEASE MOUSE"];
            for (i, line) in lines.iter().enumerate() {
                ui.text(CANVAS_WIDTH * 0.5, 540.0 + i as f32 * 26.0, line, 2.0, WHITE, Align::Center);
            }
        }
        if self.over {
            self.draw_match_over(engine);
        }
    }

    fn draw_reticle(&mut self, engine: &mut Engine) {
        let kind = self.weapon_kind();
        let weapon = spec(kind);
        let ui = &mut engine.ui;
        let spread = weapon.spread + (weapon.aim_spread - weapon.spread) * self.ps.ads;
        let gap = 4.0 + spread * 260.0 + self.ps.speed * 0.9 + self.ps.kick * 7.0;
        if self.ps.ads > 0.7 && kind != WeaponKind::Scattergun {
            ui.rect(CANVAS_WIDTH * 0.5 - 1.5, CANVAS_HEIGHT * 0.5 - 1.5, 3.0, 3.0, RED, 1.0);
        } else {
            ui.crosshair(gap, 9.0, 2.0, WHITE);
        }
        if self.hit_marker > 0.0 {
            let color = if self.hit_was_kill { RED } else { WHITE };
            ui.hit_marker(color, (self.hit_marker / 0.15).min(1.0));
        }
    }

    fn draw_ammo(&mut self, engine: &mut Engine) {
        let index = self.ps.weapon;
        let ui = &mut engine.ui;
        let ammo = self.ps.ammo[index];
        let color = if ammo * 4 <= WEAPONS[index].magazine { RED } else { WHITE };
        ui.text(1240.0, 618.0, &format!("{}", ammo), 6.0, color, Align::Right);
        ui.text(1240.0, 668.0, &format!("/ {}   {}", self.ps.reserve[index], WEAPONS[index].name), 2.0, WHITE, Align::Right);
        ui.text(1240.0, 692.0, &format!("GRENADES {}", self.ps.grenades), 2.0, YELLOW, Align::Right);
        if self.ps.reload_ticks > 0 {
            ui.text(CANVAS_WIDTH * 0.5, 440.0, "RELOADING", 3.0, YELLOW, Align::Center);
        } else if ammo == 0 {
            ui.text(CANVAS_WIDTH * 0.5, 440.0, "RELOAD", 3.0, RED, Align::Center);
        }
    }

    fn draw_health(&mut self, engine: &mut Engine, health: f32) {
        let ui = &mut engine.ui;
        let fraction = health / MAX_HEALTH;
        let color = Vec3::new(0.9 - 0.6 * fraction, 0.25 + 0.65 * fraction, 0.2);
        ui.text(40.0, 646.0, &format!("HEALTH {}", health.ceil().max(0.0) as i32), 2.0, WHITE, Align::Left);
        ui.bar(40.0, 672.0, 240.0, 14.0, fraction, color);
    }

    fn draw_damage_feedback(&mut self, engine: &mut Engine, health: f32, hurt_timer: f32) {
        let camera = engine.scene.camera;
        let forward = camera.walk_forward();
        let ui = &mut engine.ui;
        for indicator in &self.indicators {
            let direction = Vec3::new(indicator.direction.x, 0.0, indicator.direction.z).normalized();
            let angle = forward.cross(direction).y.atan2(forward.dot(direction));
            let alpha = (1.0 - indicator.age / 1.5) * 0.8;
            let (cx, cy) = (CANVAS_WIDTH * 0.5 - angle.sin() * 170.0, CANVAS_HEIGHT * 0.5 - angle.cos() * 170.0);
            ui.rect(cx - 16.0, cy - 16.0, 32.0, 32.0, RED, alpha);
        }
        let low = 1.0 - (health / (MAX_HEALTH * 0.4)).clamp(0.0, 1.0);
        ui.vignette(RED, low.max(if hurt_timer < 0.25 { 0.6 } else { 0.0 }));
    }

    fn draw_clock_and_score(&mut self, engine: &mut Engine) {
        let Some(player) = self.player else { return };
        let (kills, score) = engine.world.get::<Fighter>(player).map(|f| (f.kills, f.score)).unwrap_or((0, 0));
        let ui = &mut engine.ui;
        let seconds = self.time_left.ceil() as i32;
        ui.text(CANVAS_WIDTH * 0.5, 16.0, &format!("{}:{:02}", seconds / 60, seconds % 60), 4.0, WHITE, Align::Center);
        ui.text(CANVAS_WIDTH * 0.5, 54.0, &format!("SCORE {}   KILLS {} / {}", score, kills, KILL_LIMIT), 2.0, YELLOW, Align::Center);
        if let Some((text, _)) = &self.banner {
            ui.text(CANVAS_WIDTH * 0.5, 130.0, text, 4.0, ORANGE, Align::Center);
        }
    }

    fn draw_scoreboard(&mut self, engine: &mut Engine) {
        let rows = self.standings(engine);
        let ui = &mut engine.ui;
        ui.rect(14.0, 14.0, 230.0, 24.0 + rows.len() as f32 * 20.0, PANEL, 0.4);
        ui.text(24.0, 18.0, "SCOREBOARD", 2.0, ORANGE, Align::Left);
        for (i, (name, kills, deaths, _, is_player)) in rows.iter().enumerate() {
            let color = if *is_player { YELLOW } else { WHITE };
            ui.text(24.0, 40.0 + i as f32 * 20.0, &format!("{} {}", i + 1, name), 2.0, color, Align::Left);
            ui.text(234.0, 40.0 + i as f32 * 20.0, &format!("{}/{}", kills, deaths), 2.0, color, Align::Right);
        }
    }

    fn draw_minimap(&mut self, engine: &mut Engine) {
        let (ox, oy) = MAP_ORIGIN;
        let (width, height) = (MAP_HALF.0 * 2.0 * MAP_SCALE, MAP_HALF.1 * 2.0 * MAP_SCALE);
        let to_map = |x: f32, z: f32| (ox + (x + MAP_HALF.0) * MAP_SCALE, oy + (z + MAP_HALF.1) * MAP_SCALE);
        let ui = &mut engine.ui;
        ui.rect(ox - 4.0, oy - 4.0, width + 8.0, height + 8.0, PANEL, 0.45);
        for rect in &self.map_rects {
            let (x0, y0) = to_map(rect.bounds.min.x, rect.bounds.min.z);
            let (x1, y1) = to_map(rect.bounds.max.x, rect.bounds.max.z);
            let (x0, y0) = (x0.max(ox), y0.max(oy));
            let (x1, y1) = (x1.min(ox + width), y1.min(oy + height));
            if x1 <= x0 || y1 <= y0 {
                continue;
            }
            let shade = if rect.elevated { 0.62 } else { 0.38 };
            ui.rect(x0, y0, x1 - x0, y1 - y0, Vec3::splat(shade), 0.75);
        }
        for entity in engine.world.with::<Brain>() {
            let (Some(brain), Some(body), Some(fighter)) = (engine.world.get::<Brain>(entity), engine.world.get::<Body>(entity), engine.world.get::<Fighter>(entity)) else { continue };
            if fighter.alive && body.position.y < STAND_HEIGHT * 10.0 {
                let (x, y) = to_map(body.position.x, body.position.z);
                engine.ui.rect(x - 2.5, y - 2.5, 5.0, 5.0, brain.tint, 1.0);
            }
        }
        if let Some(body) = self.player.and_then(|p| engine.world.get::<Body>(p)) {
            let camera = engine.scene.camera;
            let (x, y) = to_map(body.position.x, body.position.z);
            let forward = camera.walk_forward();
            engine.ui.rect(x - 3.0, y - 3.0, 6.0, 6.0, YELLOW, 1.0);
            for step in 1..4 {
                engine.ui.rect(x + forward.x * step as f32 * 4.0 - 1.0, y + forward.z * step as f32 * 4.0 - 1.0, 2.0, 2.0, YELLOW, 1.0);
            }
        }
    }

    fn draw_feed(&mut self, engine: &mut Engine) {
        let ui = &mut engine.ui;
        for (i, line) in self.feed.iter().enumerate() {
            let color = if line.involves_player { YELLOW } else { WHITE };
            ui.text(1250.0, 20.0 + i as f32 * 24.0, &line.text, 2.0, color, Align::Right);
        }
    }

    fn draw_pops(&mut self, engine: &mut Engine) {
        let ui = &mut engine.ui;
        for (i, pop) in self.pops.iter().rev().enumerate() {
            let color = if pop.accent { ORANGE } else { GREEN };
            ui.text(CANVAS_WIDTH * 0.5 + 90.0, 340.0 + i as f32 * 24.0 - pop.age * 18.0, &pop.text, 2.0, color, Align::Left);
        }
    }

    fn draw_match_over(&mut self, engine: &mut Engine) {
        let rows = self.standings(engine);
        let accuracy = if self.ps.shots_fired > 0 { 100.0 * self.ps.shots_hit as f32 / self.ps.shots_fired as f32 } else { 0.0 };
        let ui = &mut engine.ui;
        ui.fade(PANEL, 0.6);
        ui.text(CANVAS_WIDTH * 0.5, 120.0, "MATCH OVER", 8.0, WHITE, Align::Center);
        ui.text(CANVAS_WIDTH * 0.5, 200.0, &format!("WINNER {}", self.winner), 5.0, ORANGE, Align::Center);
        for (i, (name, kills, deaths, score, is_player)) in rows.iter().enumerate() {
            let color = if *is_player { YELLOW } else { WHITE };
            ui.text(430.0, 290.0 + i as f32 * 34.0, &format!("{} {}", i + 1, name), 3.0, color, Align::Left);
            ui.text(850.0, 290.0 + i as f32 * 34.0, &format!("{} K  {} D  {}", kills, deaths, score), 3.0, color, Align::Right);
        }
        ui.text(CANVAS_WIDTH * 0.5, 520.0, &format!("ACCURACY {:.0}%", accuracy), 3.0, WHITE, Align::Center);
        ui.text(CANVAS_WIDTH * 0.5, 600.0, "PRESS ENTER TO PLAY AGAIN", 3.0, GREEN, Align::Center);
    }
}
