use crate::math::{Mat4, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
    pub near: f32,
}

pub const MAX_PITCH: f32 = 89.0 * std::f32::consts::PI / 180.0;

impl Camera {
    pub fn new(pos: Vec3) -> Camera {
        Camera { pos, yaw: 0.0, pitch: 0.0, fov_y: 70f32.to_radians(), near: 0.05 }
    }

    pub fn look_at(pos: Vec3, target: Vec3, fov_degrees: f32) -> Camera {
        let direction = (target - pos).normalized();
        let mut camera = Camera::new(pos);
        camera.pitch = direction.y.clamp(-1.0, 1.0).asin().clamp(-MAX_PITCH, MAX_PITCH);
        camera.yaw = (-direction.x).atan2(-direction.z);
        camera.fov_y = fov_degrees.to_radians();
        camera
    }

    pub fn project_to_screen(&self, point: Vec3, width: usize, height: usize) -> Option<(f32, f32)> {
        let offset = point - self.pos;
        let depth = offset.dot(self.forward());
        if depth < self.near {
            return None;
        }
        let tan_half = (self.fov_y * 0.5).tan();
        let aspect = width as f32 / height as f32;
        let x = offset.dot(self.right()) / depth / (tan_half * aspect);
        let y = offset.dot(self.up()) / depth / tan_half;
        Some(((x * 0.5 + 0.5) * width as f32, (0.5 - y * 0.5) * height as f32))
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }

    pub fn right(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(cy, 0.0, -sy)
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    pub fn walk_forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        Vec3::new(-sy, 0.0, -cy)
    }

    pub fn view_proj(&self, aspect: f32, far: f32, flip_y: bool) -> Mat4 {
        let (r, u, f) = (self.right(), self.up(), self.forward());
        let t = (self.fov_y * 0.5).tan();
        let (n, fa) = (self.near, far);
        let sx = 1.0 / (t * aspect);
        let sy = (if flip_y { -1.0 } else { 1.0 }) / t;
        let a = fa / (fa - n);
        let b = -fa * n / (fa - n);
        let e = self.pos;
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
