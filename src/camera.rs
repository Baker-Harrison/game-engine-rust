use crate::math::Vec3;

/// First-person camera. Right-handed, Y up. At yaw 0, pitch 0 it looks down -Z.
/// Positive yaw turns left (counterclockwise seen from above).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,   // radians
    pub pitch: f32, // radians, clamped to just under +-90 degrees
    pub fov_y: f32, // radians
    pub near: f32,
}

pub const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

impl Camera {
    pub fn new(pos: Vec3) -> Camera {
        Camera { pos, yaw: 0.0, pitch: 0.0, fov_y: 70f32.to_radians(), near: 0.05 }
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    /// Horizontal right vector.
    pub fn right(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(cy, 0.0, -sy)
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    /// Forward direction flattened onto the ground, used for walking.
    pub fn walk_forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(-sy, 0.0, -cy)
    }
}
