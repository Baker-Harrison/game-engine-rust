use crate::math::{Mat4, Vec3};

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

    /// World -> clip matrix for GPU backends. Depth range is [0, 1] (Metal, D3D, Vulkan).
    /// `flip_y` is for Vulkan, whose clip-space Y points down.
    ///
    /// It encodes the same view the CPU rasterizer uses: view-space x is
    /// right, y is up, and depth z is positive in front of the camera.
    pub fn view_proj(&self, aspect: f32, far: f32, flip_y: bool) -> Mat4 {
        let (r, u, f) = (self.right(), self.up(), self.forward());
        let t = (self.fov_y * 0.5).tan();
        let (n, fa) = (self.near, far);
        let sx = 1.0 / (t * aspect);
        let sy = (if flip_y { -1.0 } else { 1.0 }) / t;
        let a = fa / (fa - n);
        let b = -fa * n / (fa - n);
        let e = self.pos;
        // Rows of the final matrix, then written column-major.
        let rows = [
            [r.x * sx, r.y * sx, r.z * sx, -r.dot(e) * sx],
            [u.x * sy, u.y * sy, u.z * sy, -u.dot(e) * sy],
            [f.x * a, f.y * a, f.z * a, -f.dot(e) * a + b],
            [f.x, f.y, f.z, -f.dot(e)],
        ];
        let mut m = [0.0f32; 16];
        for c in 0..4 {
            for rr in 0..4 {
                m[c * 4 + rr] = rows[rr][c];
            }
        }
        Mat4(m)
    }
}
