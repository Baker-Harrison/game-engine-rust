

pub mod cpu;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(not(target_os = "macos"))]
pub mod vulkan;

use crate::camera::Camera;
use crate::platform::Window;
use crate::render::Framebuffer;
use crate::scene::{OverlayItem, OverlayRect, OverlayTri, Scene, Tri};

pub const FAR_PLANE: f32 = 500.0;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
    pub uv: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameUniforms {
    pub view_proj: [f32; 16],
    pub eye: [f32; 4],
    pub sky: [f32; 4],
    pub sun: [f32; 4],
    pub params: [f32; 4],
}

pub const KIND_LIT: f32 = 0.0;
pub const KIND_UNLIT: f32 = 1.0;
pub const KIND_SCREEN: f32 = 2.0;
pub const MAX_DYNAMIC_VERTICES: usize = 400_000;

fn push_triangle(out: &mut Vec<Vertex>, triangle: &Tri) {
    let kind = if triangle.unlit { KIND_UNLIT } else { KIND_LIT };
    for i in 0..3 {
        let (v, n, c) = (triangle.v[i], triangle.n[i], triangle.c[i]);
        let layer = if triangle.unlit { 0.0 } else { triangle.material as f32 };
        out.push(Vertex {
            pos: [v.x, v.y, v.z, 1.0],
            normal: [n.x, n.y, n.z, kind],
            color: [c.x, c.y, c.z, 1.0],
            uv: [triangle.t[i][0], triangle.t[i][1], layer, 0.0],
        });
    }
}

fn overlay_vertex(x: f32, y: f32, depth: f32, color: crate::math::Vec3, alpha: f32) -> Vertex {
    Vertex { pos: [x * 2.0 - 1.0, 1.0 - y * 2.0, depth, 1.0], normal: [0.0, 0.0, 1.0, KIND_SCREEN], color: [color.x, color.y, color.z, alpha], uv: [0.0; 4] }
}

fn push_overlay_rect(out: &mut Vec<Vertex>, rect: &OverlayRect, depth: f32) {
    let corner = |x: f32, y: f32| overlay_vertex(x, y, depth, rect.color, rect.alpha);
    let (a, b, c, d) = (corner(rect.min.0, rect.min.1), corner(rect.max.0, rect.min.1), corner(rect.max.0, rect.max.1), corner(rect.min.0, rect.max.1));
    out.extend_from_slice(&[a, b, c, a, c, d]);
}

fn push_overlay_tri(out: &mut Vec<Vertex>, tri: &OverlayTri, depth: f32) {
    for i in 0..3 {
        out.push(overlay_vertex(tri.points[i].0, tri.points[i].1, depth, tri.color, tri.alpha[i]));
    }
}

fn overlay_vertex_count(item: &OverlayItem) -> usize {
    match item {
        OverlayItem::Rect(_) => 6,
        OverlayItem::Tri(_) => 3,
    }
}

pub fn build_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(scene.tris.len() * 3);
    for triangle in &scene.tris {
        push_triangle(&mut out, triangle);
    }
    out
}

pub fn build_dynamic_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(scene.dynamic.len() * 3 + scene.overlay.len() * 4);
    for triangle in &scene.dynamic {
        push_triangle(&mut out, triangle);
    }
    let steps = scene.overlay.len().max(1) as f32;
    for (index, item) in scene.overlay.iter().enumerate() {
        if out.len() + overlay_vertex_count(item) > MAX_DYNAMIC_VERTICES {
            break;
        }
        let depth = 0.5 - 0.4 * index as f32 / steps;
        match item {
            OverlayItem::Rect(rect) => push_overlay_rect(&mut out, rect, depth),
            OverlayItem::Tri(tri) => push_overlay_tri(&mut out, tri, depth),
        }
    }
    out.truncate(MAX_DYNAMIC_VERTICES);
    out
}

pub fn frame_uniforms(scene: &Scene, cam: &Camera, aspect: f32, flip_y: bool) -> FrameUniforms {
    let sun = crate::render::SUN.normalized();
    FrameUniforms {
        view_proj: cam.view_proj(aspect, FAR_PLANE, flip_y).0,
        eye: [cam.pos.x, cam.pos.y, cam.pos.z, 1.0],
        sky: [scene.sky.x, scene.sky.y, scene.sky.z, 1.0],
        sun: [sun.x, sun.y, sun.z, crate::render::AMBIENT],
        params: [scene.fog, 0.0, 0.0, 0.0],
    }
}

pub trait Renderer {
    fn name(&self) -> &'static str;
    fn upload_scene(&mut self, scene: &Scene) -> Result<(), String>;
    fn render(&mut self, scene: &Scene, cam: &Camera, window: &mut dyn Window) -> Result<(), String>;
    fn render_offscreen(&mut self, scene: &Scene, cam: &Camera) -> Result<Framebuffer, String>;
}
pub fn create(kind: &str, width: usize, height: usize, window: &dyn Window) -> Result<Box<dyn Renderer>, String> {
    match kind {
        "cpu" => Ok(Box::new(cpu::CpuRenderer::new(width, height))),
        #[cfg(target_os = "macos")]
        "metal" => Ok(Box::new(metal::MetalRenderer::new(window)?)),
        #[cfg(not(target_os = "macos"))]
        "vulkan" => Ok(Box::new(vulkan::VulkanRenderer::new(window)?)),
        "auto" => {
            #[cfg(not(target_os = "macos"))]
            match vulkan::VulkanRenderer::new(window) {
                Ok(r) => return Ok(Box::new(r)),
                Err(e) => eprintln!("warning: Vulkan unavailable ({}), using the CPU renderer", e),
            }
            #[cfg(target_os = "macos")]
            match metal::MetalRenderer::new(window) {
                Ok(r) => return Ok(Box::new(r)),
                Err(e) => eprintln!("warning: Metal unavailable ({}), using the CPU renderer", e),
            }
            let _ = window;
            create("cpu", width, height, window)
        }
        other => Err(format!("unknown or unavailable renderer '{}' (available here: {})", other, available())),
    }
}
pub fn create_headless(kind: &str, width: usize, height: usize) -> Result<Box<dyn Renderer>, String> {
    match kind {
        "cpu" | "auto" => Ok(Box::new(cpu::CpuRenderer::new(width, height))),
        #[cfg(target_os = "macos")]
        "metal" => Ok(Box::new(metal::MetalRenderer::new_headless(width, height)?)),
        #[cfg(not(target_os = "macos"))]
        "vulkan" => Ok(Box::new(vulkan::VulkanRenderer::new_headless(width, height)?)),
        other => Err(format!("unknown or unavailable renderer '{}' (available here: {})", other, available())),
    }
}

pub fn available() -> &'static str {
    if cfg!(target_os = "macos") {
        "cpu, metal"
    } else {
        "cpu, vulkan"
    }
}

pub struct MaterialLevel {
    pub width: usize,
    pub height: usize,
    pub albedo: Vec<u8>,
    pub normal: Vec<u8>,
    pub packed: Vec<u8>,
}

pub struct MaterialArrays {
    pub layers: usize,
    pub levels: Vec<MaterialLevel>,
}

fn fit_level(source: &crate::mips::MipTexture, width: usize, height: usize, level: usize) -> Vec<u8> {
    let texture = &source.levels[level.min(source.levels.len() - 1)];
    let mut out = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let sx = x * texture.width / width;
            let sy = y * texture.height / height;
            out.extend_from_slice(&texture.get(sx, sy));
        }
    }
    out
}

pub fn pack_materials(scene: &Scene) -> Option<MaterialArrays> {
    if scene.materials.is_empty() {
        return None;
    }
    let (base_width, base_height) = scene.materials.iter().fold((1, 1), |size, maps| {
        let (w, h) = maps.albedo.base_size();
        (size.0.max(w), size.1.max(h))
    });
    let level_count = (base_width.max(base_height) as f32).log2().floor() as usize + 1;
    let mut levels = Vec::with_capacity(level_count);
    for level in 0..level_count {
        let (width, height) = ((base_width >> level).max(1), (base_height >> level).max(1));
        let mut packed_level = MaterialLevel { width, height, albedo: Vec::new(), normal: Vec::new(), packed: Vec::new() };
        for maps in &scene.materials {
            let (own_width, _) = maps.albedo.base_size();
            let source_level = (own_width as f32 / width as f32).max(1.0).log2().floor() as usize;
            packed_level.albedo.extend(fit_level(&maps.albedo, width, height, source_level));
            packed_level.normal.extend(fit_level(&maps.normal, width, height, source_level));
            packed_level.packed.extend(fit_level(&maps.packed, width, height, source_level));
        }
        levels.push(packed_level);
    }
    Some(MaterialArrays { layers: scene.materials.len(), levels })
}
