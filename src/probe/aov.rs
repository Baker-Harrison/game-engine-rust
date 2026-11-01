use crate::camera::Camera;
use crate::math::{clamp, Vec3};
use crate::model::mesh::Mesh;
use crate::render::{AMBIENT, SUN};

pub const NO_TRIANGLE: u32 = u32::MAX;

pub struct Aov {
    pub width: usize,
    pub height: usize,
    pub depth: Vec<f32>,
    pub triangle: Vec<u32>,
    pub normal: Vec<Vec3>,
    pub albedo: Vec<Vec3>,
    pub luminance: Vec<f32>,
}

impl Aov {
    fn new(width: usize, height: usize) -> Aov {
        let count = width * height;
        Aov {
            width,
            height,
            depth: vec![f32::INFINITY; count],
            triangle: vec![NO_TRIANGLE; count],
            normal: vec![Vec3::ZERO; count],
            albedo: vec![Vec3::ZERO; count],
            luminance: vec![0.0; count],
        }
    }

    pub fn covered(&self, index: usize) -> bool {
        self.triangle[index] != NO_TRIANGLE
    }
}

pub fn luminance_of(color: Vec3) -> f32 {
    0.2126 * color.x + 0.7152 * color.y + 0.0722 * color.z
}

#[derive(Clone, Copy)]
struct ViewVertex {
    position: Vec3,
    normal: Vec3,
    color: Vec3,
}

impl ViewVertex {
    fn lerp(self, other: ViewVertex, t: f32) -> ViewVertex {
        ViewVertex {
            position: self.position.lerp(other.position, t),
            normal: self.normal.lerp(other.normal, t),
            color: self.color.lerp(other.color, t),
        }
    }
}

#[derive(Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    inverse_depth: f32,
    normal_over_depth: Vec3,
    color_over_depth: Vec3,
}

struct ViewFrame {
    camera: Camera,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    tan_half: f32,
    aspect: f32,
    sun: Vec3,
}

impl ViewFrame {
    fn new(camera: &Camera, width: usize, height: usize) -> ViewFrame {
        let (right, up, forward) = (camera.right(), camera.up(), camera.forward());
        let sun = SUN.normalized();
        ViewFrame {
            camera: *camera,
            right,
            up,
            forward,
            tan_half: (camera.fov_y * 0.5).tan(),
            aspect: width as f32 / height as f32,
            sun: Vec3::new(sun.dot(right), sun.dot(up), sun.dot(forward)),
        }
    }

    fn to_view(&self, point: Vec3) -> Vec3 {
        let offset = point - self.camera.pos;
        Vec3::new(offset.dot(self.right), offset.dot(self.up), offset.dot(self.forward))
    }

    fn direction_to_view(&self, direction: Vec3) -> Vec3 {
        Vec3::new(direction.dot(self.right), direction.dot(self.up), direction.dot(self.forward))
    }

    fn view_to_world(&self, direction: Vec3) -> Vec3 {
        self.right * direction.x + self.up * direction.y + self.forward * direction.z
    }

    fn project(&self, vertex: ViewVertex, width: f32, height: f32) -> ScreenVertex {
        let inverse_depth = 1.0 / vertex.position.z;
        ScreenVertex {
            x: (vertex.position.x * inverse_depth / (self.tan_half * self.aspect) * 0.5 + 0.5) * width,
            y: (0.5 - vertex.position.y * inverse_depth / self.tan_half * 0.5) * height,
            inverse_depth,
            normal_over_depth: vertex.normal * inverse_depth,
            color_over_depth: vertex.color * inverse_depth,
        }
    }
}

pub fn rasterize(mesh: &Mesh, camera: &Camera, width: usize, height: usize) -> Aov {
    let mut aov = Aov::new(width, height);
    let frame = ViewFrame::new(camera, width, height);
    for triangle in 0..mesh.triangle_count() {
        rasterize_triangle(mesh, triangle, &frame, &mut aov);
    }
    aov
}

fn rasterize_triangle(mesh: &Mesh, triangle: usize, frame: &ViewFrame, aov: &mut Aov) {
    let [a, b, c] = mesh.triangle(triangle);
    let facing_away = (b - a).cross(c - a).dot(frame.camera.pos - a) < 0.0;
    let sign = if facing_away { -1.0 } else { 1.0 };
    let corners: [ViewVertex; 3] = std::array::from_fn(|k| {
        let index = mesh.indices[triangle * 3 + k] as usize;
        ViewVertex {
            position: frame.to_view([a, b, c][k]),
            normal: frame.direction_to_view(mesh.normals[index]) * sign,
            color: mesh.colors[index],
        }
    });
    let (polygon, count) = clip_to_near(&corners, frame.camera.near);
    if count < 3 {
        return;
    }
    let (width, height) = (aov.width as f32, aov.height as f32);
    let first = frame.project(polygon[0], width, height);
    for k in 1..count - 1 {
        let second = frame.project(polygon[k], width, height);
        let third = frame.project(polygon[k + 1], width, height);
        fill(aov, frame, triangle as u32, first, second, third);
    }
}

fn clip_to_near(corners: &[ViewVertex; 3], near: f32) -> ([ViewVertex; 4], usize) {
    let mut polygon = [corners[0]; 4];
    let mut count = 0;
    for k in 0..3 {
        let (current, next) = (corners[k], corners[(k + 1) % 3]);
        let (current_in, next_in) = (current.position.z >= near, next.position.z >= near);
        if current_in {
            polygon[count] = current;
            count += 1;
        }
        if current_in != next_in {
            let t = (near - current.position.z) / (next.position.z - current.position.z);
            polygon[count] = current.lerp(next, t);
            count += 1;
        }
    }
    (polygon, count)
}

fn fill(aov: &mut Aov, frame: &ViewFrame, triangle: u32, a: ScreenVertex, b: ScreenVertex, c: ScreenVertex) {
    let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area.abs() < 1e-8 {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(aov.width as f32 - 1.0) as i32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(aov.height as f32 - 1.0) as i32;
    let inverse_area = 1.0 / area;
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) * inverse_area;
            let w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) * inverse_area;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let inverse_depth = w0 * a.inverse_depth + w1 * b.inverse_depth + w2 * c.inverse_depth;
            let depth = 1.0 / inverse_depth;
            let index = y as usize * aov.width + x as usize;
            if depth >= aov.depth[index] {
                continue;
            }
            let view_normal = ((a.normal_over_depth * w0 + b.normal_over_depth * w1 + c.normal_over_depth * w2) * depth).normalized();
            let albedo = (a.color_over_depth * w0 + b.color_over_depth * w1 + c.color_over_depth * w2) * depth;
            let shade = AMBIENT + (1.0 - AMBIENT) * view_normal.dot(frame.sun).max(0.0);
            aov.depth[index] = depth;
            aov.triangle[index] = triangle;
            aov.normal[index] = frame.view_to_world(view_normal);
            aov.albedo[index] = albedo;
            aov.luminance[index] = clamp(luminance_of(albedo) * shade, 0.0, 1.0);
        }
    }
}
