use super::bake::{bake, BakeSettings};
use super::bvh::Bvh;
use super::mesh::Mesh;
use super::mesher::mesh_sdf;
use super::sdf::Sdf;
use crate::camera::Camera;
use crate::math::Vec3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PartKind {
    Object,
    Architecture,
    Stage,
}

#[derive(Clone)]
pub struct Member {
    pub name: String,
    pub min: Vec3,
    pub max: Vec3,
    pub touches_floor: bool,
}

impl Member {
    pub fn from_sdf(name: &str, sdf: &Sdf, touches_floor: bool) -> Member {
        let (min, max) = sdf.bounds();
        Member { name: name.to_string(), min, max, touches_floor }
    }
}

#[derive(Clone)]
pub struct Part {
    pub name: String,
    pub kind: PartKind,
    pub mesh: Mesh,
    pub members: Vec<Member>,
}

impl Part {
    pub fn from_sdf(name: &str, kind: PartKind, sdf: &Sdf, cell: f32, color: Vec3, members: Vec<Member>) -> Part {
        Part { name: name.to_string(), kind, mesh: mesh_sdf(sdf, cell, color), members }
    }

    pub fn from_mesh(name: &str, kind: PartKind, mesh: Mesh) -> Part {
        Part { name: name.to_string(), kind, mesh, members: Vec::new() }
    }

    fn transform(&mut self, position: Vec3, yaw_degrees: f32) {
        self.mesh.rotate_y(yaw_degrees);
        self.mesh.translate(position);
        for member in &mut self.members {
            let (low, high) = (member.min, member.max);
            let mut new_low = Vec3::splat(f32::MAX);
            let mut new_high = Vec3::splat(f32::MIN);
            for x in [low.x, high.x] {
                for y in [low.y, high.y] {
                    for z in [low.z, high.z] {
                        let corner = rotate_about_y(Vec3::new(x, y, z), yaw_degrees) + position;
                        new_low = new_low.min(corner);
                        new_high = new_high.max(corner);
                    }
                }
            }
            member.min = new_low;
            member.max = new_high;
        }
    }
}

pub fn rotate_about_y(point: Vec3, degrees: f32) -> Vec3 {
    let (s, c) = degrees.to_radians().sin_cos();
    Vec3::new(c * point.x + s * point.z, point.y, -s * point.x + c * point.z)
}

#[derive(Clone)]
pub struct Dimension {
    pub subject: String,
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
}

impl Dimension {
    pub fn new(subject: &str, name: &str, value: f32, min: f32, max: f32) -> Dimension {
        Dimension { subject: subject.to_string(), name: name.to_string(), value, min, max }
    }
}

#[derive(Clone)]
pub struct Footprint {
    pub subject: String,
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Footprint {
    pub fn overlaps(&self, other: &Footprint) -> bool {
        self.min_x < other.max_x && self.max_x > other.min_x && self.min_z < other.max_z && self.max_z > other.min_z
    }
}

#[derive(Clone)]
pub struct ClearanceZone {
    pub owner: String,
    pub purpose: String,
    pub area: Footprint,
}

#[derive(Clone)]
pub struct Room {
    pub name: String,
    pub area: Footprint,
    pub ceiling_height: f32,
}

#[derive(Clone)]
pub struct DoorLink {
    pub name: String,
    pub from: String,
    pub to: String,
    pub swing_area: Footprint,
}

#[derive(Clone)]
pub struct View {
    pub name: String,
    pub camera: Camera,
}

#[derive(Clone, Default)]
pub struct Asset {
    pub name: String,
    pub parts: Vec<Part>,
    pub dimensions: Vec<Dimension>,
    pub footprints: Vec<Footprint>,
    pub clearance_zones: Vec<ClearanceZone>,
    pub rooms: Vec<Room>,
    pub doors: Vec<DoorLink>,
    pub views: Vec<View>,
    pub roof_part_names: Vec<String>,
}

impl Asset {
    pub fn new(name: &str) -> Asset {
        Asset { name: name.to_string(), ..Asset::default() }
    }

    pub fn absorb(&mut self, mut other: Asset, prefix: &str, position: Vec3, yaw_degrees: f32) -> Footprint {
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        for part in &mut other.parts {
            part.transform(position, yaw_degrees);
            part.name = format!("{}.{}", prefix, part.name);
            for member in &mut part.members {
                member.name = format!("{}.{}", prefix, member.name);
            }
            if part.kind == PartKind::Object {
                let (part_low, part_high) = part.mesh.bounds();
                low = low.min(part_low);
                high = high.max(part_high);
            }
        }
        for dimension in &mut other.dimensions {
            dimension.subject = format!("{}.{}", prefix, dimension.subject);
        }
        self.parts.append(&mut other.parts);
        self.dimensions.append(&mut other.dimensions);
        Footprint { subject: prefix.to_string(), min_x: low.x, max_x: high.x, min_z: low.z, max_z: high.z }
    }

    pub fn merged_mesh(&self, excluded_parts: &[String]) -> Mesh {
        let mut merged = Mesh::new();
        for part in &self.parts {
            if !excluded_parts.contains(&part.name) {
                merged.append(&part.mesh);
            }
        }
        merged
    }

    pub fn baked_mesh(&self, excluded_parts: &[String], settings: &BakeSettings) -> Mesh {
        let mut merged = self.merged_mesh(excluded_parts);
        let occluders = Bvh::from_meshes(&[&merged]);
        bake(&mut merged, &occluders, settings);
        merged
    }

    pub fn object_bounds(&self) -> (Vec3, Vec3) {
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        for part in self.parts.iter().filter(|part| part.kind != PartKind::Stage) {
            let (part_low, part_high) = part.mesh.bounds();
            low = low.min(part_low);
            high = high.max(part_high);
        }
        (low, high)
    }

    pub fn add_stage_floor(&mut self, center_x: f32, center_z: f32, half_size: f32, step: f32, height: f32, color: Vec3) {
        let mut floor = Mesh::new();
        let breaks = grid_breaks(-half_size, half_size, step);
        floor.add_grid(
            Vec3::new(center_x, height, center_z),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            &breaks,
            &breaks,
            Vec3::new(0.0, 1.0, 0.0),
            color,
            |_, _| true,
        );
        self.parts.push(Part::from_mesh("stage_floor", PartKind::Stage, floor));
    }
}

pub fn grid_breaks(from: f32, to: f32, step: f32) -> Vec<f32> {
    let count = ((to - from) / step).ceil().max(1.0) as usize;
    (0..=count).map(|i| from + (to - from) * i as f32 / count as f32).collect()
}
