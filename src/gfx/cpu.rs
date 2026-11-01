use super::Renderer;
use crate::camera::Camera;
use crate::platform::Window;
use crate::render::{render, Framebuffer};
use crate::scene::Scene;

pub struct CpuRenderer {
    fb: Framebuffer,
}

impl CpuRenderer {
    pub fn new(width: usize, height: usize) -> CpuRenderer {
        CpuRenderer { fb: Framebuffer::new(width, height) }
    }
}

impl Renderer for CpuRenderer {
    fn name(&self) -> &'static str {
        "cpu"
    }
    fn upload_scene(&mut self, _scene: &Scene) -> Result<(), String> {
        Ok(())
    }
    fn render(&mut self, scene: &Scene, cam: &Camera, window: &mut dyn Window) -> Result<(), String> {
        render(scene, cam, &mut self.fb);
        window.present(&self.fb);
        Ok(())
    }
    fn render_offscreen(&mut self, scene: &Scene, cam: &Camera) -> Result<Framebuffer, String> {
        render(scene, cam, &mut self.fb);
        Ok(self.fb.clone())
    }
}
