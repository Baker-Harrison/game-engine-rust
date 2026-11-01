use engine::math::Vec3;
use engine::model::mesh::Mesh;
use engine::scene::Tri;
use engine::skeleton::JointTransform;

#[derive(Clone, Copy)]
pub struct Placement {
    pub origin: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub offset: Vec3,
    pub swing: f32,
    pub tint: Vec3,
}

impl Placement {
    pub fn at(origin: Vec3, yaw: f32) -> Placement {
        Placement { origin, yaw, pitch: 0.0, roll: 0.0, offset: Vec3::ZERO, swing: 0.0, tint: Vec3::splat(1.0) }
    }

    pub fn offset(mut self, offset: Vec3) -> Placement {
        self.offset = offset;
        self
    }

    pub fn swing(mut self, swing: f32) -> Placement {
        self.swing = swing;
        self
    }

    pub fn pitch(mut self, pitch: f32) -> Placement {
        self.pitch = pitch;
        self
    }

    pub fn roll(mut self, roll: f32) -> Placement {
        self.roll = roll;
        self
    }

    pub fn tint(mut self, tint: Vec3) -> Placement {
        self.tint = tint;
        self
    }

    pub fn point(&self, p: Vec3) -> Vec3 {
        rotate_y(rotate_x(rotate_x(rotate_z(p, self.roll), self.swing) + self.offset, self.pitch), self.yaw) + self.origin
    }

    pub fn direction(&self, d: Vec3) -> Vec3 {
        rotate_y(rotate_x(rotate_x(rotate_z(d, self.roll), self.swing), self.pitch), self.yaw)
    }
}

pub fn rotate_x(p: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(p.x, c * p.y - s * p.z, s * p.y + c * p.z)
}

pub fn rotate_z(p: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z)
}

pub fn rotate_y(p: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z)
}

pub fn push_mesh(out: &mut Vec<Tri>, mesh: &Mesh, placement: &Placement) {
    let points: Vec<Vec3> = mesh.positions.iter().map(|p| placement.point(*p)).collect();
    let normals: Vec<Vec3> = mesh.normals.iter().map(|n| placement.direction(*n)).collect();
    for triangle in mesh.indices.chunks(3) {
        let at = |k: usize| triangle[k] as usize;
        out.push(Tri {
            v: [points[at(0)], points[at(1)], points[at(2)]],
            n: [normals[at(0)], normals[at(1)], normals[at(2)]],
            c: [
                mesh.colors[at(0)].mul_elements(placement.tint),
                mesh.colors[at(1)].mul_elements(placement.tint),
                mesh.colors[at(2)].mul_elements(placement.tint),
            ],
            s: [mesh.surfaces[at(0)], mesh.surfaces[at(1)], mesh.surfaces[at(2)]],
            t: [[0.0; 2]; 3],
            material: 0,
            unlit: false,
        });
    }
}

pub fn push_mesh_bone(out: &mut Vec<Tri>, mesh: &Mesh, bone: &JointTransform, local_origin: Vec3, base: &Placement) {
    let points: Vec<Vec3> = mesh.positions.iter().map(|p| base.point(bone.position + bone.rotation.rotate(*p - local_origin))).collect();
    let normals: Vec<Vec3> = mesh.normals.iter().map(|n| base.direction(bone.rotation.rotate(*n))).collect();
    for triangle in mesh.indices.chunks(3) {
        let at = |k: usize| triangle[k] as usize;
        out.push(Tri {
            v: [points[at(0)], points[at(1)], points[at(2)]],
            n: [normals[at(0)], normals[at(1)], normals[at(2)]],
            c: [
                mesh.colors[at(0)].mul_elements(base.tint),
                mesh.colors[at(1)].mul_elements(base.tint),
                mesh.colors[at(2)].mul_elements(base.tint),
            ],
            s: [mesh.surfaces[at(0)], mesh.surfaces[at(1)], mesh.surfaces[at(2)]],
            t: [[0.0; 2]; 3],
            material: 0,
            unlit: false,
        });
    }
}

pub fn push_quad(out: &mut Vec<Tri>, center: Vec3, right: Vec3, up: Vec3, color: Vec3) {
    let corners = [center - right - up, center + right - up, center + right + up, center - right + up];
    out.push(Tri::glowing([corners[0], corners[1], corners[2]], color));
    out.push(Tri::glowing([corners[0], corners[2], corners[3]], color));
}

pub fn push_lit_quad(out: &mut Vec<Tri>, center: Vec3, right: Vec3, up: Vec3, color: Vec3) {
    let corners = [center - right - up, center + right - up, center + right + up, center - right + up];
    out.push(Tri::flat([corners[0], corners[1], corners[2]], color));
    out.push(Tri::flat([corners[0], corners[2], corners[3]], color));
}
