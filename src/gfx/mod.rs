

pub mod cpu;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(not(target_os = "macos"))]
pub mod vulkan;

use crate::camera::Camera;
use crate::platform::Window;
use crate::render::Framebuffer;
use crate::scene::{OverlayRect, Scene, Tri};

pub const FAR_PLANE: f32 = 500.0;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
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
pub const MAX_DYNAMIC_VERTICES: usize = 240_000;

fn push_triangle(out: &mut Vec<Vertex>, triangle: &Tri) {
    let kind = if triangle.unlit { KIND_UNLIT } else { KIND_LIT };
    for i in 0..3 {
        let (v, n, c) = (triangle.v[i], triangle.n[i], triangle.c[i]);
        out.push(Vertex { pos: [v.x, v.y, v.z, 1.0], normal: [n.x, n.y, n.z, kind], color: [c.x, c.y, c.z, 1.0] });
    }
}

fn push_overlay_rect(out: &mut Vec<Vertex>, rect: &OverlayRect, depth: f32) {
    let to_clip = |x: f32, y: f32| [x * 2.0 - 1.0, 1.0 - y * 2.0, depth, 1.0];
    let corner = |x: f32, y: f32| Vertex {
        pos: to_clip(x, y),
        normal: [0.0, 0.0, 1.0, KIND_SCREEN],
        color: [rect.color.x, rect.color.y, rect.color.z, rect.alpha],
    };
    let (a, b, c, d) = (corner(rect.min.0, rect.min.1), corner(rect.max.0, rect.min.1), corner(rect.max.0, rect.max.1), corner(rect.min.0, rect.max.1));
    out.extend_from_slice(&[a, b, c, a, c, d]);
}

pub fn build_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(scene.tris.len() * 3);
    for triangle in &scene.tris {
        push_triangle(&mut out, triangle);
    }
    out
}

pub fn build_dynamic_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(scene.dynamic.len() * 3 + scene.overlay.len() * 6);
    for triangle in &scene.dynamic {
        push_triangle(&mut out, triangle);
    }
    let overlay_budget = scene.overlay.len().min(MAX_DYNAMIC_VERTICES.saturating_sub(out.len()) / 6);
    let steps = scene.overlay.len().max(1) as f32;
    for (index, rect) in scene.overlay.iter().take(overlay_budget).enumerate() {
        push_overlay_rect(&mut out, rect, 0.5 - 0.4 * index as f32 / steps);
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
