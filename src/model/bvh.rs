use super::mesh::Mesh;
use crate::math::Vec3;

const LEAF_SIZE: usize = 4;

struct Node {
    low: Vec3,
    high: Vec3,
    first: u32,
    count: u32,
    right_child: u32,
}

pub struct Bvh {
    nodes: Vec<Node>,
    triangles: Vec<[Vec3; 3]>,
}

impl Bvh {
    pub fn from_meshes(meshes: &[&Mesh]) -> Bvh {
        let mut triangles = Vec::new();
        for mesh in meshes {
            for index in 0..mesh.triangle_count() {
                triangles.push(mesh.triangle(index));
            }
        }
        let mut bvh = Bvh { nodes: Vec::new(), triangles: Vec::new() };
        if !triangles.is_empty() {
            let count = triangles.len();
            bvh.build(&mut triangles, 0, count);
        }
        bvh.triangles = triangles;
        bvh
    }

    fn build(&mut self, triangles: &mut [[Vec3; 3]], start: usize, end: usize) -> usize {
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        let mut centroid_low = Vec3::splat(f32::MAX);
        let mut centroid_high = Vec3::splat(f32::MIN);
        for triangle in &triangles[start..end] {
            let centroid = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
            centroid_low = centroid_low.min(centroid);
            centroid_high = centroid_high.max(centroid);
            for vertex in triangle {
                low = low.min(*vertex);
                high = high.max(*vertex);
            }
        }
        let node_index = self.nodes.len();
        self.nodes.push(Node { low, high, first: start as u32, count: (end - start) as u32, right_child: 0 });
        if end - start <= LEAF_SIZE {
            return node_index;
        }
        let spread = centroid_high - centroid_low;
        let axis = if spread.x >= spread.y && spread.x >= spread.z { 0 } else if spread.y >= spread.z { 1 } else { 2 };
        let key = |triangle: &[Vec3; 3]| {
            let sum = triangle[0] + triangle[1] + triangle[2];
            [sum.x, sum.y, sum.z][axis]
        };
        triangles[start..end].sort_by(|a, b| key(a).total_cmp(&key(b)));
        let middle = (start + end) / 2;
        self.build(triangles, start, middle);
        let right = self.build(triangles, middle, end);
        self.nodes[node_index].count = 0;
        self.nodes[node_index].right_child = right as u32;
        node_index
    }

    pub fn any_hit(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        let inverse = Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z);
        let mut stack = [0usize; 64];
        let mut depth = 1;
        while depth > 0 {
            depth -= 1;
            let node_index = stack[depth];
            let node = &self.nodes[node_index];
            if !hits_box(node.low, node.high, origin, inverse, max_distance) {
                continue;
            }
            if node.count > 0 {
                let range = node.first as usize..(node.first + node.count) as usize;
                if self.triangles[range].iter().any(|t| hits_triangle(t, origin, direction, max_distance)) {
                    return true;
                }
            } else {
                stack[depth] = node_index + 1;
                stack[depth + 1] = node.right_child as usize;
                depth += 2;
            }
        }
        false
    }
}

fn hits_box(low: Vec3, high: Vec3, origin: Vec3, inverse: Vec3, max_distance: f32) -> bool {
    let t1 = (low - origin).mul_elements(inverse);
    let t2 = (high - origin).mul_elements(inverse);
    let near = t1.min(t2);
    let far = t1.max(t2);
    let enter = near.x.max(near.y).max(near.z).max(0.0);
    let exit = far.x.min(far.y).min(far.z).min(max_distance);
    enter <= exit
}

fn hits_triangle(triangle: &[Vec3; 3], origin: Vec3, direction: Vec3, max_distance: f32) -> bool {
    let edge1 = triangle[1] - triangle[0];
    let edge2 = triangle[2] - triangle[0];
    let p = direction.cross(edge2);
    let determinant = edge1.dot(p);
    if determinant.abs() < 1e-9 {
        return false;
    }
    let inverse = 1.0 / determinant;
    let to_origin = origin - triangle[0];
    let u = to_origin.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = to_origin.cross(edge1);
    let v = direction.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = edge2.dot(q) * inverse;
    t > 1e-4 && t < max_distance
}
