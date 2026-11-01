use crate::bodies::{ARM_OFFSET, HIP_HEIGHT, LEG_OFFSET, SHOULDER_HEIGHT};
use crate::level::PickupKind;
use crate::pose::{push_mesh, push_mesh_bone, push_quad, Placement};
use crate::rig::RigInput;
use crate::state::{Brain, Fighter, Oilyard, Pickup};
use crate::weapons::{spec, WeaponKind};
use engine::math::Vec3;
use engine::physics::Body;
use engine::runtime::Engine;
use engine::scene::Tri;
use engine::worldfile::Transform;

const SHIRT: Vec3 = Vec3::new(0.34, 0.36, 0.28);
const PANTS: Vec3 = Vec3::new(0.2, 0.22, 0.2);
const CORPSE_LINGER: f32 = 2.6;
const DYNAMIC_TRIANGLE_BUDGET: usize = 72_000;

fn lerp_vec(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

impl Oilyard {
    pub fn compose_frame(&mut self, engine: &mut Engine, blend: f32) {
        let seconds = engine.time.seconds();
        let mut camera = engine.scene.camera;
        let bob = (self.ps.bob_phase * 2.0).sin() * 0.022 * (self.ps.speed / 5.0).min(1.5);
        camera.pos = lerp_vec(self.ps.previous_eye, self.ps.eye, blend) + Vec3::new(0.0, bob, 0.0);
        let shake = self.shake * 0.06;
        camera.pos = camera.pos + Vec3::new((seconds * 83.0).sin(), (seconds * 71.0 + 1.3).sin(), (seconds * 59.0 + 2.1).sin()) * shake;
        engine.scene.camera = camera;

        let mut dynamic = std::mem::take(&mut engine.scene.dynamic);
        dynamic.clear();
        self.emit_bots(engine, &mut dynamic);
        self.emit_pickups(engine, seconds, &mut dynamic);
        for grenade in &self.grenades {
            push_mesh(&mut dynamic, &self.models().grenade, &Placement::at(grenade.projectile.position, seconds));
        }
        if dynamic.len() < DYNAMIC_TRIANGLE_BUDGET {
            self.fx.emit(&camera, &mut dynamic);
        }
        self.emit_viewmodel(engine, seconds, &mut dynamic);
        dynamic.truncate(DYNAMIC_TRIANGLE_BUDGET + 4_000);
        engine.scene.dynamic = dynamic;

        engine.ui.clear();
        self.draw_hud(engine);
        engine.scene.overlay = engine.ui.finish();
        engine.sync_listener();
    }

    fn emit_bots(&self, engine: &Engine, out: &mut Vec<Tri>) {
        let model = &self.models().bot;
        for entity in engine.world.with::<Brain>() {
            let (Some(brain), Some(body), Some(fighter)) = (engine.world.get::<Brain>(entity), engine.world.get::<Body>(entity), engine.world.get::<Fighter>(entity)) else { continue };
            if !fighter.alive && fighter.dead_for > CORPSE_LINGER {
                continue;
            }
            let fall = if fighter.alive { 0.0 } else { (fighter.dead_for * 4.0).min(1.45) };
            let base = Placement::at(body.position, brain.yaw).pitch(fall);
            let rig = &model.rig;
            let alive_speed = if fighter.alive { brain.speed_fraction } else { 0.0 };
            let input = RigInput { phase: brain.walk_phase, speed: alive_speed, aim_pitch: 0.0, recoil: if brain.flash > 0 { 1.0 } else { 0.0 } };
            let pose = rig.pose(&input);
            let (hips, chest, head) = (rig.transform(&pose, "hips"), rig.transform(&pose, "chest"), rig.transform(&pose, "head"));
            let (hips_rest, chest_rest, head_rest) = (rig.rest_position("hips"), rig.rest_position("chest"), rig.rest_position("head"));
            push_mesh_bone(out, &model.torso, &hips, hips_rest, &base.tint(SHIRT));
            push_mesh_bone(out, &model.vest, &chest, chest_rest, &base.tint(brain.tint));
            push_mesh_bone(out, &model.head, &head, head_rest, &base);
            push_mesh_bone(out, &model.helmet, &head, head_rest, &base);
            push_mesh_bone(out, &model.rifle, &chest, chest_rest - Vec3::new(0.14, 1.2, -0.3), &base);
            for side in ["r", "l"] {
                let sleeve = base.tint(SHIRT);
                let trousers = base.tint(PANTS);
                push_mesh_bone(out, &rig.upper_arm, &rig.transform(&pose, &format!("shoulder_{}", side)), Vec3::ZERO, &sleeve);
                push_mesh_bone(out, &rig.forearm, &rig.transform(&pose, &format!("elbow_{}", side)), Vec3::ZERO, &sleeve);
                push_mesh_bone(out, &rig.thigh, &rig.transform(&pose, &format!("thigh_{}", side)), Vec3::ZERO, &trousers);
                push_mesh_bone(out, &rig.shin, &rig.transform(&pose, &format!("knee_{}", side)), Vec3::ZERO, &trousers);
            }
            if brain.flash > 0 && fighter.alive {
                let muzzle = base.point(Vec3::new(0.14, 1.215, -0.92));
                let right = Vec3::new(brain.yaw.cos(), 0.0, -brain.yaw.sin());
                push_quad(out, muzzle, right * 0.14, Vec3::new(0.0, 0.14, 0.0), Vec3::new(1.0, 0.85, 0.4));
            }
        }
    }

    fn emit_pickups(&self, engine: &Engine, seconds: f32, out: &mut Vec<Tri>) {
        let models = self.models();
        for entity in engine.world.with::<Pickup>() {
            let (Some(pickup), Some(transform)) = (engine.world.get::<Pickup>(entity), engine.world.get::<Transform>(entity)) else { continue };
            if pickup.ready_in > 0.0 {
                continue;
            }
            let mesh = if pickup.kind == PickupKind::Health { &models.health_pack } else { &models.ammo_pack };
            let hover = Vec3::new(0.0, 0.55 + 0.1 * (seconds * 2.0).sin(), 0.0);
            push_mesh(out, mesh, &Placement::at(transform.position + hover, seconds * 1.3));
        }
    }

    fn emit_viewmodel(&self, engine: &Engine, seconds: f32, out: &mut Vec<Tri>) {
        let alive = self.player.and_then(|p| engine.world.get::<Fighter>(p)).map(|f| f.alive).unwrap_or(false);
        if !alive || self.over {
            return;
        }
        let kind = self.weapon_kind();
        let weapon = spec(kind);
        let camera = engine.scene.camera;
        let ads = self.ps.ads;
        let hip = Vec3::new(0.2, -0.19, -0.44);
        let aimed = Vec3::new(0.0, -0.105, -0.4);
        let mut offset = lerp_vec(hip, aimed, ads);
        let sway = (self.ps.speed / 5.0).min(1.5) * (1.0 - ads);
        offset.x += (self.ps.bob_phase).sin() * 0.012 * sway;
        offset.y += (self.ps.bob_phase * 2.0).sin().abs() * 0.01 * sway - (seconds * 1.1).sin() * 0.002;
        offset.z += self.ps.kick * 0.045;
        if self.ps.sprinting {
            offset.y -= 0.05;
            offset.x += 0.05;
        }
        let switching = self.ps.switch_ticks as f32 / 18.0;
        offset.y -= 0.35 * switching;
        let mut pitch = camera.pitch + self.ps.kick * 0.04;
        let mut roll = 0.0;
        let mut reload = None;
        if self.ps.reload_ticks > 0 {
            let progress = 1.0 - self.ps.reload_ticks as f32 / weapon.reload_ticks as f32;
            let frame = crate::viewrig::reload_frame(kind, progress);
            offset.y -= frame.weapon_drop;
            pitch += frame.weapon_pitch;
            roll = frame.weapon_roll;
            reload = Some(progress);
        }
        let body = Placement::at(camera.pos, camera.yaw).pitch(camera.pitch);
        let placement = Placement::at(camera.pos, camera.yaw).pitch(pitch).roll(roll).offset(offset);
        let models = self.models();
        push_mesh(out, models.viewmodels.weapon(kind), &placement);
        models.arms.emit(out, kind, &placement, &body, reload, kind != WeaponKind::Sidearm);
        if self.ps.flash > 0 {
            let size = (0.06 + 0.03 * (engine.time.tick % 2) as f32) * (1.0 - 0.75 * ads);
            let at = placement.point(weapon.muzzle);
            let color = Vec3::new(1.0, 0.82, 0.4);
            push_quad(out, at, camera.right() * size, camera.up() * size, color);
            push_quad(out, at, camera.right() * size * 0.5, camera.up() * size * 1.6, color);
        }
    }
}
