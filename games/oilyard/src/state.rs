use crate::bodies::BotModel;
use crate::effects::Effects;
use crate::level::{MapRect, PickupKind, Spawn};
use crate::sfx::Sounds;
use crate::weapons::{Viewmodels, WEAPONS};
use engine::ecs::Entity;
use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::physics::Projectile;
use engine::worldfile::{Component, Fields};

pub const KILL_LIMIT: u32 = 25;
pub const MATCH_SECONDS: f32 = 240.0;
pub const BOT_COUNT: usize = 5;
pub const RESPAWN_SECONDS: f32 = 3.0;
pub const MAX_HEALTH: f32 = 100.0;
pub const BOT_NAMES: [&str; BOT_COUNT] = ["HAWK", "ROOK", "VIPER", "DUNE", "GHOST"];
pub const BOT_COLORS: [Vec3; BOT_COUNT] = [
    Vec3::new(0.85, 0.25, 0.15),
    Vec3::new(0.2, 0.45, 0.85),
    Vec3::new(0.25, 0.7, 0.3),
    Vec3::new(0.9, 0.7, 0.15),
    Vec3::new(0.6, 0.3, 0.75),
];

#[derive(Clone, Debug)]
pub struct Fighter {
    pub name: String,
    pub health: f32,
    pub kills: u32,
    pub deaths: u32,
    pub score: i32,
    pub streak: u32,
    pub chain: u32,
    pub chain_timer: f32,
    pub alive: bool,
    pub respawn_timer: f32,
    pub dead_for: f32,
    pub hurt_timer: f32,
}

impl Fighter {
    pub fn new(name: &str) -> Fighter {
        Fighter {
            name: name.to_string(),
            health: MAX_HEALTH,
            kills: 0,
            deaths: 0,
            score: 0,
            streak: 0,
            chain: 0,
            chain_timer: 0.0,
            alive: true,
            respawn_timer: 0.0,
            dead_for: 0.0,
            hurt_timer: 99.0,
        }
    }
}

impl Component for Fighter {
    const NAME: &'static str = "fighter";

    fn from_fields(fields: &Fields) -> Result<Fighter, String> {
        let mut fighter = Fighter::new(fields.text_or("name", "unnamed"));
        fighter.health = fields.f32_or("health", MAX_HEALTH);
        fighter.kills = fields.f32_or("kills", 0.0) as u32;
        fighter.deaths = fields.f32_or("deaths", 0.0) as u32;
        fighter.score = fields.f32_or("score", 0.0) as i32;
        Ok(fighter)
    }

    fn to_fields(&self) -> Vec<(String, String)> {
        vec![
            ("name".into(), self.name.clone()),
            ("health".into(), format!("{:.0}", self.health)),
            ("kills".into(), self.kills.to_string()),
            ("deaths".into(), self.deaths.to_string()),
            ("score".into(), self.score.to_string()),
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotMode {
    Roam,
    Hunt,
    Engage,
    Flee,
}

#[derive(Clone, Debug)]
pub struct Brain {
    pub mode: BotMode,
    pub path: Vec<Vec3>,
    pub path_index: usize,
    pub repath_timer: f32,
    pub target: Option<Entity>,
    pub last_seen: Vec3,
    pub seen_for: f32,
    pub memory: f32,
    pub yaw: f32,
    pub fire_cooldown: u32,
    pub burst_left: u32,
    pub burst_pause: f32,
    pub strafe: f32,
    pub strafe_timer: f32,
    pub walk_phase: f32,
    pub speed_fraction: f32,
    pub stuck_timer: f32,
    pub stuck_origin: Vec3,
    pub step_timer: f32,
    pub flee_timer: f32,
    pub flash: u32,
    pub skill: f32,
    pub tint: Vec3,
}

impl Brain {
    pub fn new(skill: f32, tint: Vec3, yaw: f32) -> Brain {
        Brain {
            mode: BotMode::Roam,
            path: Vec::new(),
            path_index: 0,
            repath_timer: 0.0,
            target: None,
            last_seen: Vec3::ZERO,
            seen_for: 0.0,
            memory: 0.0,
            yaw,
            fire_cooldown: 0,
            burst_left: 0,
            burst_pause: 0.0,
            strafe: 1.0,
            strafe_timer: 0.0,
            walk_phase: 0.0,
            speed_fraction: 0.0,
            stuck_timer: 0.0,
            stuck_origin: Vec3::ZERO,
            step_timer: 0.0,
            flee_timer: 0.0,
            flash: 0,
            skill,
            tint,
        }
    }

    pub fn reset(&mut self) {
        self.mode = BotMode::Roam;
        self.path.clear();
        self.path_index = 0;
        self.target = None;
        self.seen_for = 0.0;
        self.memory = 0.0;
        self.burst_left = 0;
        self.flee_timer = 0.0;
    }
}

#[derive(Clone, Debug)]
pub struct Pickup {
    pub kind: PickupKind,
    pub ready_in: f32,
}

impl Component for Pickup {
    const NAME: &'static str = "pickup";

    fn from_fields(fields: &Fields) -> Result<Pickup, String> {
        let kind = match fields.text_or("kind", "health") {
            "ammo" => PickupKind::Ammo,
            _ => PickupKind::Health,
        };
        Ok(Pickup { kind, ready_in: fields.f32_or("ready_in", 0.0) })
    }

    fn to_fields(&self) -> Vec<(String, String)> {
        let kind = if self.kind == PickupKind::Ammo { "ammo" } else { "health" };
        vec![("kind".into(), kind.into()), ("ready_in".into(), format!("{:.1}", self.ready_in))]
    }
}

pub struct Grenade {
    pub projectile: Projectile,
    pub fuse: f32,
    pub owner: Entity,
}

pub struct FeedLine {
    pub text: String,
    pub age: f32,
    pub involves_player: bool,
}

pub struct ScorePop {
    pub text: String,
    pub age: f32,
    pub accent: bool,
}

pub struct Noise {
    pub position: Vec3,
    pub source: Entity,
    pub radius: f32,
}

pub struct Indicator {
    pub direction: Vec3,
    pub age: f32,
}

pub struct PlayerState {
    pub weapon: usize,
    pub ammo: [u32; 3],
    pub reserve: [u32; 3],
    pub reload_ticks: u32,
    pub cooldown: u32,
    pub switch_ticks: u32,
    pub ads: f32,
    pub recoil_index: usize,
    pub idle_time: f32,
    pub unrecovered_pitch: f32,
    pub grenades: u32,
    pub grenade_cooldown: u32,
    pub step_timer: f32,
    pub bob_phase: f32,
    pub speed: f32,
    pub kick: f32,
    pub flash: u32,
    pub previous_eye: Vec3,
    pub eye: Vec3,
    pub sprinting: bool,
    pub shots_fired: u32,
    pub shots_hit: u32,
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            weapon: 0,
            ammo: [WEAPONS[0].magazine, WEAPONS[1].magazine, WEAPONS[2].magazine],
            reserve: [WEAPONS[0].reserve, WEAPONS[1].reserve, WEAPONS[2].reserve],
            reload_ticks: 0,
            cooldown: 0,
            switch_ticks: 0,
            ads: 0.0,
            recoil_index: 0,
            idle_time: 9.0,
            unrecovered_pitch: 0.0,
            grenades: 2,
            grenade_cooldown: 0,
            step_timer: 0.0,
            bob_phase: 0.0,
            speed: 0.0,
            kick: 0.0,
            flash: 0,
            previous_eye: Vec3::ZERO,
            eye: Vec3::ZERO,
            sprinting: false,
            shots_fired: 0,
            shots_hit: 0,
        }
    }
}

pub struct Models {
    pub bot: BotModel,
    pub viewmodels: Viewmodels,
    pub arms: crate::viewrig::ViewArms,
    pub grenade: Mesh,
    pub health_pack: Mesh,
    pub ammo_pack: Mesh,
}

pub struct Oilyard {
    pub models: Option<Models>,
    pub sounds: Option<Sounds>,
    pub spawns: Vec<Spawn>,
    pub map_rects: Vec<MapRect>,
    pub player: Option<Entity>,
    pub ps: PlayerState,
    pub grenades: Vec<Grenade>,
    pub fx: Effects,
    pub feed: Vec<FeedLine>,
    pub pops: Vec<ScorePop>,
    pub noises: Vec<Noise>,
    pub indicators: Vec<Indicator>,
    pub time_left: f32,
    pub over: bool,
    pub winner: String,
    pub hit_marker: f32,
    pub hit_was_kill: bool,
    pub shake: f32,
    pub banner: Option<(String, f32)>,
    pub killer_name: String,
    pub match_number: u32,
    pub intro_time: f32,
}

impl Oilyard {
    pub fn new() -> Oilyard {
        Oilyard {
            models: None,
            sounds: None,
            spawns: Vec::new(),
            map_rects: Vec::new(),
            player: None,
            ps: PlayerState::new(),
            grenades: Vec::new(),
            fx: Effects::new(),
            feed: Vec::new(),
            pops: Vec::new(),
            noises: Vec::new(),
            indicators: Vec::new(),
            time_left: MATCH_SECONDS,
            over: false,
            winner: String::new(),
            hit_marker: 0.0,
            hit_was_kill: false,
            shake: 0.0,
            banner: None,
            killer_name: String::new(),
            match_number: 1,
            intro_time: 0.0,
        }
    }

    pub fn sounds(&self) -> &Sounds {
        self.sounds.as_ref().expect("setup builds sounds")
    }

    pub fn models(&self) -> &Models {
        self.models.as_ref().expect("setup builds models")
    }

    pub fn player_entity(&self) -> Entity {
        self.player.expect("setup spawns the player")
    }
}
