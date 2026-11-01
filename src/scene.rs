use crate::camera::Camera;
use crate::math::Vec3;

#[derive(Clone, Copy)]
pub struct Tri {
    pub v: [Vec3; 3],
    pub n: [Vec3; 3],
    pub c: [Vec3; 3],
    pub s: [[f32; 2]; 3],
    pub t: [[f32; 2]; 3],
    pub material: u16,
    pub unlit: bool,
}

impl Tri {
    pub fn flat(v: [Vec3; 3], color: Vec3) -> Tri {
        let normal = (v[1] - v[0]).cross(v[2] - v[0]).normalized();
        Tri { v, n: [normal; 3], c: [color; 3], s: [[0.85, 0.0]; 3], t: [[0.0; 2]; 3], material: 0, unlit: false }
    }

    pub fn glowing(v: [Vec3; 3], color: Vec3) -> Tri {
        Tri { unlit: true, ..Tri::flat(v, color) }
    }
}

#[derive(Clone, Copy)]
pub struct OverlayRect {
    pub min: (f32, f32),
    pub max: (f32, f32),
    pub color: Vec3,
    pub alpha: f32,
}

#[derive(Clone, Copy)]
pub struct OverlayTri {
    pub points: [(f32, f32); 3],
    pub alpha: [f32; 3],
    pub color: Vec3,
}

#[derive(Clone, Copy)]
pub enum OverlayItem {
    Rect(OverlayRect),
    Tri(OverlayTri),
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AlphaMode {
    Opaque,
    Cutout,
    Blend,
}

pub struct SurfaceMaps {
    pub alpha_mode: AlphaMode,
    pub albedo: crate::mips::MipTexture,
    pub normal: crate::mips::MipTexture,
    pub packed: crate::mips::MipTexture,
}

pub struct Scene {
    pub materials: Vec<SurfaceMaps>,
    pub sky: Vec3,
    pub fog: f32,
    pub ground: Option<Ground>,
    pub boxes: Vec<BoxEnt>,
    pub camera: Camera,
    pub tris: Vec<Tri>,
    pub placed_meshes: Vec<crate::model::mesh::Mesh>,
    pub dynamic: Vec<Tri>,
    pub overlay: Vec<OverlayItem>,
}

impl Scene {
    pub fn empty(sky: Vec3, fog: f32) -> Scene {
        Scene {
            materials: Vec::new(),
            sky,
            fog,
            ground: None,
            boxes: Vec::new(),
            camera: Camera::new(Vec3::new(0.0, 1.7, 0.0)),
            tris: Vec::new(),
            placed_meshes: Vec::new(),
            dynamic: Vec::new(),
            overlay: Vec::new(),
        }
    }

    pub fn from_mesh(mesh: &crate::model::mesh::Mesh, sky: Vec3) -> Scene {
        let mut scene = Scene::empty(sky, 1.0e6);
        scene.add_mesh(mesh);
        scene
    }

    pub fn add_material(&mut self, albedo: &crate::texture::Texture, normal: &crate::texture::Texture, packed: &crate::texture::Texture) -> u16 {
        self.add_material_alpha(albedo, normal, packed, AlphaMode::Opaque)
    }

    pub fn add_material_alpha(&mut self, albedo: &crate::texture::Texture, normal: &crate::texture::Texture, packed: &crate::texture::Texture, alpha_mode: AlphaMode) -> u16 {
        self.materials.push(SurfaceMaps {
            alpha_mode,
            albedo: crate::mips::MipTexture::new(albedo),
            normal: crate::mips::MipTexture::new(normal),
            packed: crate::mips::MipTexture::new(packed),
        });
        self.materials.len() as u16
    }

    pub fn is_blended(&self, tri: &Tri) -> bool {
        !tri.unlit && tri.material > 0 && self.materials.get(tri.material as usize - 1).map_or(false, |m| m.alpha_mode == AlphaMode::Blend)
    }

    pub fn alpha_mode_of(&self, tri: &Tri) -> AlphaMode {
        if tri.unlit || tri.material == 0 {
            return AlphaMode::Opaque;
        }
        self.materials.get(tri.material as usize - 1).map_or(AlphaMode::Opaque, |m| m.alpha_mode)
    }

    pub fn add_mesh(&mut self, mesh: &crate::model::mesh::Mesh) {
        self.add_mesh_textured(mesh, 0);
    }

    pub fn add_mesh_textured(&mut self, mesh: &crate::model::mesh::Mesh, material: u16) {
        let uv = |i: usize| mesh.uvs.get(i).copied().unwrap_or([0.0, 0.0]);
        for triangle in mesh.indices.chunks(3) {
            let at = |k: usize| triangle[k] as usize;
            self.tris.push(Tri {
                v: [mesh.positions[at(0)], mesh.positions[at(1)], mesh.positions[at(2)]],
                n: [mesh.normals[at(0)], mesh.normals[at(1)], mesh.normals[at(2)]],
                c: [mesh.colors[at(0)], mesh.colors[at(1)], mesh.colors[at(2)]],
                s: [mesh.surfaces[at(0)], mesh.surfaces[at(1)], mesh.surfaces[at(2)]],
                t: [uv(at(0)), uv(at(1)), uv(at(2))],
                material,
                unlit: false,
            });
        }
    }

    pub fn parse(src: &str) -> Result<Scene, String> {
        let mut s = Scene::empty(Vec3::new(0.53, 0.74, 0.93), 90.0);
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
                "asset" => {
                    let name = t.get(1).ok_or_else(|| err("expected: asset NAME [pos x y z] [yaw deg] [key=value ...]"))?;
                    let overrides = t[2..]
                        .iter()
                        .filter(|word| word.contains('='))
                        .map(|word| crate::assets::parse_override(word))
                        .collect::<Result<Vec<_>, String>>()
                        .map_err(|e| err(&e))?;
                    let asset = crate::assets::build(name, &overrides).map_err(|e| err(&e))?;
                    let mut mesh = asset.baked_mesh(&["stage_floor".to_string()], &asset.default_bake_settings());
                    mesh.rotate_y(kv_f32(&t, "yaw").unwrap_or(0.0));
                    mesh.translate(kv_vec3(&t, "pos").unwrap_or(Vec3::ZERO));
                    s.placed_meshes.push(mesh);
                }
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
                    tris.push(Tri::flat([a, e, d], c));
                    tris.push(Tri::flat([a, d, b], c));
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
                tris.push(Tri::flat([corners[f[0]], corners[f[1]], corners[f[2]]], b.color));
                tris.push(Tri::flat([corners[f[0]], corners[f[2]], corners[f[3]]], b.color));
            }
        }
        self.tris = tris;
        let placed = std::mem::take(&mut self.placed_meshes);
        for mesh in &placed {
            self.add_mesh(mesh);
        }
        self.placed_meshes = placed;
    }
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
