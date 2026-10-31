//! Engine-owned renderer interface. Game code only talks to `Renderer`;
//! each backend (CPU, Metal, Vulkan, D3D12) implements it.
//!
//! Every backend draws the same thing from the same data: a flat-shaded
//! triangle list with a directional sun, depth testing and distance fog. The
//! CPU renderer is the reference the GPU backends are compared against.

pub mod cpu;
#[cfg(target_os = "macos")]
pub mod metal;

use crate::camera::Camera;
use crate::platform::Window;
use crate::render::Framebuffer;
use crate::scene::Scene;

pub const FAR_PLANE: f32 = 500.0;

/// One vertex. Everything is vec4-sized so the same layout works in MSL,
/// GLSL and HLSL without padding surprises (48 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
}

/// Per-frame constants shared by all GPU shaders (128 bytes, vec4 aligned).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameUniforms {
    pub view_proj: [f32; 16],
    pub eye: [f32; 4],
    pub sky: [f32; 4],
    /// xyz = direction towards the sun, w = ambient.
    pub sun: [f32; 4],
    /// x = fog distance.
    pub params: [f32; 4],
}

/// Flattens the scene into a vertex list (3 vertices per triangle).
pub fn build_vertices(scene: &Scene) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(scene.tris.len() * 3);
    for t in &scene.tris {
        let n = (t.v[1] - t.v[0]).cross(t.v[2] - t.v[0]).normalized();
        for v in t.v {
            out.push(Vertex {
                pos: [v.x, v.y, v.z, 1.0],
                normal: [n.x, n.y, n.z, 0.0],
                color: [t.color.x, t.color.y, t.color.z, 1.0],
            });
        }
    }
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
    /// Uploads (or re-uploads) the scene's geometry.
    fn upload_scene(&mut self, scene: &Scene) -> Result<(), String>;
    /// Draws one frame and shows it in `window`.
    fn render(&mut self, scene: &Scene, cam: &Camera, window: &mut dyn Window) -> Result<(), String>;
    /// Draws one frame without a window and returns the pixels (0x00RRGGBB).
    fn render_offscreen(&mut self, scene: &Scene, cam: &Camera) -> Result<Framebuffer, String>;
}

/// Creates the renderer for a window. `auto` picks the best backend for this OS
/// and falls back to the CPU renderer if the GPU backend cannot start.
pub fn create(kind: &str, width: usize, height: usize, window: &dyn Window) -> Result<Box<dyn Renderer>, String> {
    match kind {
        "cpu" => Ok(Box::new(cpu::CpuRenderer::new(width, height))),
        #[cfg(target_os = "macos")]
        "metal" => Ok(Box::new(metal::MetalRenderer::new(window)?)),
        "auto" => {
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

/// Headless renderer for `engine render`. `auto` means the CPU reference renderer.
pub fn create_headless(kind: &str, width: usize, height: usize) -> Result<Box<dyn Renderer>, String> {
    match kind {
        "cpu" | "auto" => Ok(Box::new(cpu::CpuRenderer::new(width, height))),
        #[cfg(target_os = "macos")]
        "metal" => Ok(Box::new(metal::MetalRenderer::new_headless(width, height)?)),
        other => Err(format!("unknown or unavailable renderer '{}' (available here: {})", other, available())),
    }
}

pub fn available() -> &'static str {
    if cfg!(target_os = "macos") {
        "cpu, metal"
    } else {
        "cpu"
    }
}
