use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::model::mesher::mesh_sdf;
use engine::model::sdf::Sdf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Carbine,
    Scattergun,
    Sidearm,
}

pub struct WeaponSpec {
    pub kind: WeaponKind,
    pub name: &'static str,
    pub damage: f32,
    pub head_multiplier: f32,
    pub fire_interval_ticks: u32,
    pub automatic: bool,
    pub pellets: u32,
    pub spread: f32,
    pub aim_spread: f32,
    pub magazine: u32,
    pub reserve: u32,
    pub reload_ticks: u32,
    pub recoil_pitch: f32,
    pub recoil_yaw: f32,
    pub falloff_start: f32,
    pub falloff_end: f32,
    pub aim_fov: f32,
    pub speed_factor: f32,
    pub muzzle: Vec3,
}

pub const WEAPONS: [WeaponSpec; 3] = [
    WeaponSpec {
        kind: WeaponKind::Carbine,
        name: "M4 CARBINE",
        damage: 24.0,
        head_multiplier: 1.8,
        fire_interval_ticks: 6,
        automatic: true,
        pellets: 1,
        spread: 0.022,
        aim_spread: 0.004,
        magazine: 30,
        reserve: 120,
        reload_ticks: 130,
        recoil_pitch: 0.0085,
        recoil_yaw: 0.004,
        falloff_start: 25.0,
        falloff_end: 70.0,
        aim_fov: 0.62,
        speed_factor: 1.0,
        muzzle: Vec3::new(0.0, -0.01, -0.58),
    },
    WeaponSpec {
        kind: WeaponKind::Scattergun,
        name: "SCATTERGUN",
        damage: 13.0,
        head_multiplier: 1.4,
        fire_interval_ticks: 48,
        automatic: false,
        pellets: 9,
        spread: 0.075,
        aim_spread: 0.05,
        magazine: 6,
        reserve: 30,
        reload_ticks: 150,
        recoil_pitch: 0.05,
        recoil_yaw: 0.01,
        falloff_start: 6.0,
        falloff_end: 22.0,
        aim_fov: 0.78,
        speed_factor: 0.95,
        muzzle: Vec3::new(0.0, 0.0, -0.9),
    },
    WeaponSpec {
        kind: WeaponKind::Sidearm,
        name: "P9 SIDEARM",
        damage: 30.0,
        head_multiplier: 2.0,
        fire_interval_ticks: 11,
        automatic: false,
        pellets: 1,
        spread: 0.014,
        aim_spread: 0.003,
        magazine: 12,
        reserve: 60,
        reload_ticks: 95,
        recoil_pitch: 0.02,
        recoil_yaw: 0.004,
        falloff_start: 20.0,
        falloff_end: 55.0,
        aim_fov: 0.7,
        speed_factor: 1.05,
        muzzle: Vec3::new(0.0, 0.01, -0.28),
    },
];

pub const CARBINE_RECOIL_PATTERN: [f32; 12] = [0.0, 0.3, -0.5, 0.8, 0.4, -0.6, -1.0, -0.4, 0.5, 1.0, 0.7, -0.3];

pub fn spec(kind: WeaponKind) -> &'static WeaponSpec {
    &WEAPONS[kind as usize]
}

fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn barrel(start: Vec3, end: Vec3, radius: f32) -> Sdf {
    Sdf::limb(start, end, [radius, radius], [radius, radius], radius * 0.95)
}

fn block(center: Vec3, size: Vec3, rounding: f32) -> Sdf {
    Sdf::round_box(size, rounding).translate(center)
}

pub struct Viewmodels {
    pub carbine: Mesh,
    pub scattergun: Mesh,
    pub sidearm: Mesh,
    pub right_arm: Mesh,
    pub left_arm: Mesh,
}

impl Viewmodels {
    pub fn build() -> Result<Viewmodels, String> {
        Ok(Viewmodels {
            carbine: carbine()?,
            scattergun: scattergun(),
            sidearm: sidearm(),
            right_arm: arm(v(0.17, -0.34, 0.34), v(0.0, -0.16, 0.06)),
            left_arm: arm(v(-0.22, -0.36, 0.2), v(-0.01, -0.07, -0.26)),
        })
    }

    pub fn weapon(&self, kind: WeaponKind) -> &Mesh {
        match kind {
            WeaponKind::Carbine => &self.carbine,
            WeaponKind::Scattergun => &self.scattergun,
            WeaponKind::Sidearm => &self.sidearm,
        }
    }
}

fn arm(shoulder: Vec3, hand: Vec3) -> Mesh {
    let sleeve = Sdf::limb(shoulder, hand, [0.055, 0.055], [0.04, 0.04], 0.02);
    let sleeve_mesh = mesh_sdf(&sleeve, 0.012, v(0.24, 0.27, 0.19));
    let glove = Sdf::round_box(v(0.085, 0.07, 0.11), 0.03).translate(hand);
    let mut mesh = sleeve_mesh;
    mesh.append(&mesh_sdf(&glove, 0.012, v(0.1, 0.1, 0.11)));
    mesh
}

fn carbine() -> Result<Mesh, String> {
    let asset = engine::assets::build("carbine", &[("cell".to_string(), 0.007)])?;
    let mut mesh = asset.merged_mesh(&["stage_floor".to_string()]);
    mesh.rotate_y(90.0);
    mesh.translate(v(0.0, 0.03, -0.02));
    Ok(mesh)
}

fn scattergun() -> Mesh {
    let steel = v(0.2, 0.21, 0.23);
    let wood = v(0.38, 0.25, 0.14);
    let receiver = block(v(0.0, 0.0, -0.1), v(0.05, 0.075, 0.28), 0.015);
    let barrel_top = barrel(v(0.0, 0.015, -0.2), v(0.0, 0.015, -0.9), 0.019);
    let magazine_tube = barrel(v(0.0, -0.035, -0.22), v(0.0, -0.035, -0.82), 0.016);
    let metal = receiver.union(barrel_top).union(magazine_tube);
    let pump = block(v(0.0, -0.045, -0.5), v(0.052, 0.06, 0.26), 0.02);
    let stock = Sdf::limb(v(0.0, 0.0, 0.04), v(0.0, -0.07, 0.5), [0.03, 0.045], [0.03, 0.09], 0.01);
    let grip = block(v(0.0, -0.07, 0.0), v(0.04, 0.1, 0.05), 0.012);
    let mut mesh = mesh_sdf(&metal, 0.008, steel);
    mesh.append(&mesh_sdf(&pump.union(stock).union(grip), 0.008, wood));
    mesh
}

fn sidearm() -> Mesh {
    let slide = block(v(0.0, 0.03, -0.1), v(0.035, 0.05, 0.28), 0.012);
    let frame = block(v(0.0, -0.005, -0.08), v(0.034, 0.03, 0.22), 0.01);
    let grip = Sdf::limb(v(0.0, -0.01, 0.0), v(0.0, -0.14, 0.03), [0.017, 0.028], [0.018, 0.03], 0.008);
    let mut mesh = mesh_sdf(&slide, 0.005, v(0.13, 0.14, 0.16));
    mesh.append(&mesh_sdf(&frame.union(grip), 0.005, v(0.07, 0.07, 0.08)));
    mesh
}
