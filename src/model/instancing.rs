use super::lod::{build_lod_chain, Lod};
use super::mesh::Mesh;
use crate::math::Vec3;

pub struct Prefab {
    pub name: String,
    pub lods: Vec<Lod>,
    pub switch_distances: Vec<f32>,
}

#[derive(Clone, Copy)]
pub struct Instance {
    pub prefab: usize,
    pub position: Vec3,
    pub yaw_degrees: f32,
}

#[derive(Default)]
pub struct PrefabLibrary {
    pub prefabs: Vec<Prefab>,
    pub instances: Vec<Instance>,
}

impl Prefab {
    pub fn from_mesh(name: &str, mesh: &Mesh, triangle_ratios: &[f32], switch_distances: &[f32]) -> Prefab {
        Prefab { name: name.to_string(), lods: build_lod_chain(mesh, triangle_ratios), switch_distances: switch_distances.to_vec() }
    }

    pub fn lod_for_distance(&self, distance: f32) -> &Lod {
        let level = self.switch_distances.iter().filter(|limit| distance > **limit).count();
        &self.lods[level.min(self.lods.len() - 1)]
    }
}

impl PrefabLibrary {
    pub fn add_prefab(&mut self, prefab: Prefab) -> usize {
        self.prefabs.push(prefab);
        self.prefabs.len() - 1
    }

    pub fn place(&mut self, prefab: usize, position: Vec3, yaw_degrees: f32) {
        self.instances.push(Instance { prefab, position, yaw_degrees });
    }

    pub fn flatten(&self, viewer: Vec3) -> Mesh {
        let mut result = Mesh::new();
        for instance in &self.instances {
            let distance = (instance.position - viewer).length();
            let lod = self.prefabs[instance.prefab].lod_for_distance(distance);
            let mut copy = lod.mesh.clone();
            copy.rotate_y(instance.yaw_degrees);
            copy.translate(instance.position);
            result.append(&copy);
        }
        result
    }

    pub fn stored_triangles(&self) -> usize {
        self.prefabs.iter().map(|p| p.lods.iter().map(|l| l.mesh.triangle_count()).sum::<usize>()).sum()
    }

    pub fn drawn_triangles(&self, viewer: Vec3) -> usize {
        self.instances
            .iter()
            .map(|i| self.prefabs[i.prefab].lod_for_distance((i.position - viewer).length()).mesh.triangle_count())
            .sum()
    }
}
