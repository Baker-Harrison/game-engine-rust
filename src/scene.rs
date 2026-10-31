use crate::camera::Camera;
use crate::math::Vec3;

/// A world-space triangle with a flat base color.
#[derive(Clone, Copy)]
pub struct Tri {
    pub v: [Vec3; 3],
    pub color: Vec3,
}

#[derive(Clone, Copy)]
pub struct BoxEnt {
    pub center: Vec3,
    pub size: Vec3,
    pub color: Vec3,
}

#[derive(Clone, Copy)]
pub struct Ground {
    pub size: f32,
    pub tile: f32,
    pub color_a: Vec3,
    pub color_b: Vec3,
}

pub struct Scene {
    pub sky: Vec3,
    pub fog: f32,
    pub ground: Option<Ground>,
    pub boxes: Vec<BoxEnt>,
    pub camera: Camera,
    pub tris: Vec<Tri>,
}

impl Scene {
    pub fn parse(src: &str) -> Result<Scene, String> {
        let mut s = Scene {
            sky: Vec3::new(0.53, 0.74, 0.93),
            fog: 90.0,
            ground: None,
            boxes: Vec::new(),
            camera: Camera::new(Vec3::new(0.0, 1.7, 0.0)),
            tris: Vec::new(),
        };
        for (i, raw) in src.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let t: Vec<&str> = line.split_whitespace().collect();
            let err = |m: &str| format!("scene line {}: {}", i + 1, m);
            match t[0] {
                "sky" => s.sky = vec3_at(&t, 1).ok_or_else(|| err("expected: sky r g b"))?,
                "fog" => {
                    s.fog = t.get(1).and_then(|v| v.parse().ok()).ok_or_else(|| err("expected: fog distance"))?
                }
                "ground" => {
                    s.ground = Some(Ground {
                        size: kv_f32(&t, "size").unwrap_or(100.0),
                        tile: kv_f32(&t, "tile").unwrap_or(2.0),
                        color_a: kv_vec3(&t, "color_a").unwrap_or(Vec3::new(0.34, 0.47, 0.30)),
                        color_b: kv_vec3(&t, "color_b").unwrap_or(Vec3::new(0.29, 0.41, 0.26)),
                    })
                }
                "camera" => {
                    if let Some(p) = kv_vec3(&t, "pos") {
                        s.camera.pos = p;
                    }
                    if let Some(y) = kv_f32(&t, "yaw") {
                        s.camera.yaw = y.to_radians();
                    }
                    if let Some(p) = kv_f32(&t, "pitch") {
                        s.camera.pitch = p.to_radians();
                    }
                    if let Some(f) = kv_f32(&t, "fov") {
                        s.camera.fov_y = f.to_radians();
                    }
                }
                "box" => s.boxes.push(BoxEnt {
                    center: kv_vec3(&t, "pos").ok_or_else(|| err("box needs pos x y z"))?,
                    size: kv_vec3(&t, "size").ok_or_else(|| err("box needs size x y z"))?,
                    color: kv_vec3(&t, "color").unwrap_or(Vec3::new(0.7, 0.7, 0.7)),
                }),
                other => return Err(err(&format!("unknown entity '{}'", other))),
            }
        }
        s.build_mesh();
        Ok(s)
    }

    fn build_mesh(&mut self) {
        let mut tris = Vec::new();
        if let Some(g) = self.ground {
            let n = ((g.size / g.tile).ceil() as i32).max(1);
            let half = n as f32 * g.tile * 0.5;
            for iz in 0..n {
                for ix in 0..n {
                    let x0 = -half + ix as f32 * g.tile;
                    let z0 = -half + iz as f32 * g.tile;
                    let (x1, z1) = (x0 + g.tile, z0 + g.tile);
                    let c = if (ix + iz) % 2 == 0 { g.color_a } else { g.color_b };
                    let (a, b, d, e) = (
                        Vec3::new(x0, 0.0, z0),
                        Vec3::new(x1, 0.0, z0),
                        Vec3::new(x1, 0.0, z1),
                        Vec3::new(x0, 0.0, z1),
                    );
                    tris.push(Tri { v: [a, e, d], color: c });
                    tris.push(Tri { v: [a, d, b], color: c });
                }
            }
        }
        for b in &self.boxes {
            let h = b.size * 0.5;
            let c = b.center;
            let p = |sx: f32, sy: f32, sz: f32| Vec3::new(c.x + sx * h.x, c.y + sy * h.y, c.z + sz * h.z);
            let corners = [
                p(-1.0, -1.0, -1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(-1.0, 1.0, -1.0),
                p(-1.0, -1.0, 1.0),
                p(1.0, -1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(-1.0, 1.0, 1.0),
            ];
            let faces = [[0, 1, 2, 3], [5, 4, 7, 6], [4, 0, 3, 7], [1, 5, 6, 2], [3, 2, 6, 7], [4, 5, 1, 0]];
            for f in faces {
                tris.push(Tri { v: [corners[f[0]], corners[f[1]], corners[f[2]]], color: b.color });
                tris.push(Tri { v: [corners[f[0]], corners[f[2]], corners[f[3]]], color: b.color });
            }
        }
        self.tris = tris;
    }

    /// Moves a circle (the player) in the XZ plane out of any box it overlaps.
    /// Boxes whose top is below `step` are walkable and ignored.
    pub fn blocked(&self, x: f32, z: f32, radius: f32) -> bool {
        self.boxes.iter().any(|b| {
            let top = b.center.y + b.size.y * 0.5;
            if top < 0.3 {
                return false;
            }
            let hx = b.size.x * 0.5 + radius;
            let hz = b.size.z * 0.5 + radius;
            (x - b.center.x).abs() < hx && (z - b.center.z).abs() < hz
        })
    }
}

fn vec3_at(t: &[&str], i: usize) -> Option<Vec3> {
    let g = |k: usize| t.get(k).and_then(|v| v.parse::<f32>().ok());
    Some(Vec3::new(g(i)?, g(i + 1)?, g(i + 2)?))
}

fn kv_vec3(t: &[&str], key: &str) -> Option<Vec3> {
    let i = t.iter().position(|w| *w == key)?;
    vec3_at(t, i + 1)
}

fn kv_f32(t: &[&str], key: &str) -> Option<f32> {
    let i = t.iter().position(|w| *w == key)?;
    t.get(i + 1)?.parse().ok()
}
