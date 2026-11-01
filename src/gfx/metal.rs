
use super::{build_dynamic_vertices, build_vertices, frame_uniforms, FrameUniforms, Renderer, Vertex, MAX_DYNAMIC_VERTICES};
use crate::camera::Camera;
use crate::platform::objc::*;
use crate::platform::{NativeHandle, Window};
use crate::render::Framebuffer;
use crate::scene::Scene;
use std::ffi::c_void;

#[link(name = "Metal", kind = "framework")]
extern "C" {
    fn MTLCreateSystemDefaultDevice() -> Id;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    w: f64,
    h: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct ClearColor {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Region {
    origin: [usize; 3],
    size: [usize; 3],
}

const BGRA8: usize = 80;
const DEPTH32: usize = 252;

const SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Vertex { float4 pos; float4 normal; float4 color; };
struct U { float4x4 view_proj; float4 eye; float4 sky; float4 sun; float4 params; };
struct VOut {
    float4 position [[position]];
    float3 world;
    float3 normal;
    float4 color;
    float depth;
    float kind;
};

vertex VOut vs(uint vid [[vertex_id]], constant Vertex* verts [[buffer(0)]], constant U& u [[buffer(1)]]) {
    Vertex v = verts[vid];
    VOut o;
    o.position = v.normal.w > 1.5 ? float4(v.pos.x, v.pos.y, v.pos.z, 1.0) : u.view_proj * float4(v.pos.xyz, 1.0);
    o.world = v.pos.xyz;
    o.normal = v.normal.xyz;
    o.color = v.color;
    o.kind = v.normal.w;
    o.depth = o.position.w;
    return o;
}

fragment float4 fs(VOut i [[stage_in]], constant U& u [[buffer(1)]]) {
    if (i.kind > 1.5) return i.color;
    float3 n = normalize(i.normal);
    if (dot(n, u.eye.xyz - i.world) < 0.0) n = -n;
    float shade = u.sun.w + (1.0 - u.sun.w) * max(dot(n, u.sun.xyz), 0.0);
    if (i.kind > 0.5) shade = 1.0;
    float t = clamp(i.depth / u.params.x, 0.0, 1.0);
    t = t * t;
    return float4(mix(i.color.xyz * shade, u.sky.xyz, t), 1.0);
}
"#;

pub struct MetalRenderer {
    device: Id,
    queue: Id,
    pipeline: Id,
    depth_state: Id,
    depth_tex: Id,
    layer: Id,
    vertex_buf: Id,
    vertex_count: usize,
    dynamic_buffers: [Id; 2],
    dynamic_count: usize,
    in_flight: [Id; 2],
    frame: usize,
    w: usize,
    h: usize,
}

unsafe fn make_depth_texture(device: Id, w: usize, h: usize) -> Id {
    let desc = send!(Id; class("MTLTextureDescriptor"),
        "texture2DDescriptorWithPixelFormat:width:height:mipmapped:",
        usize = DEPTH32, usize = w, usize = h, bool = false);
    send!((); desc, "setUsage:", usize = 4);
    send!((); desc, "setStorageMode:", usize = 2);
    send!(Id; device, "newTextureWithDescriptor:", Id = desc)
}

impl MetalRenderer {
    pub fn new(window: &dyn Window) -> Result<MetalRenderer, String> {
        let view = match window.native_handle() {
            NativeHandle::MacView(v) => v,
            _ => return Err("Metal needs a macOS window".into()),
        };
        let (w, h) = window.size();
        unsafe {
            let device = MTLCreateSystemDefaultDevice();
            if device.is_null() {
                return Err("no Metal device available".into());
            }
            let layer = send!(Id; class("CAMetalLayer"), "layer");
            send!((); layer, "setDevice:", Id = device);
            send!((); layer, "setPixelFormat:", usize = BGRA8);
            send!((); layer, "setFramebufferOnly:", bool = true);
            send!((); layer, "setDrawableSize:", CGSize = CGSize { w: w as f64, h: h as f64 });
            send!((); view, "setLayer:", Id = layer);
            send!((); view, "setWantsLayer:", bool = true);
            Self::build(device, layer, w, h)
        }
    }

    pub fn new_headless(w: usize, h: usize) -> Result<MetalRenderer, String> {
        unsafe {
            let device = MTLCreateSystemDefaultDevice();
            if device.is_null() {
                return Err("no Metal device available".into());
            }
            Self::build(device, std::ptr::null_mut(), w, h)
        }
    }

    unsafe fn build(device: Id, layer: Id, w: usize, h: usize) -> Result<MetalRenderer, String> {
        let mut err: Id = std::ptr::null_mut();
        let lib = send!(Id; device, "newLibraryWithSource:options:error:",
            Id = nsstring(SHADER), Id = std::ptr::null_mut(), *mut Id = &mut err);
        if lib.is_null() {
            return Err(format!("Metal shader compile failed: {}", error_text(err)));
        }
        let vs = send!(Id; lib, "newFunctionWithName:", Id = nsstring("vs"));
        let fs = send!(Id; lib, "newFunctionWithName:", Id = nsstring("fs"));

        let desc = send!(Id; send!(Id; class("MTLRenderPipelineDescriptor"), "alloc"), "init");
        send!((); desc, "setVertexFunction:", Id = vs);
        send!((); desc, "setFragmentFunction:", Id = fs);
        let attachments = send!(Id; desc, "colorAttachments");
        let c0 = send!(Id; attachments, "objectAtIndexedSubscript:", usize = 0);
        send!((); c0, "setPixelFormat:", usize = BGRA8);
        send!((); c0, "setBlendingEnabled:", bool = true);
        send!((); c0, "setSourceRGBBlendFactor:", usize = 4);
        send!((); c0, "setDestinationRGBBlendFactor:", usize = 5);
        send!((); c0, "setSourceAlphaBlendFactor:", usize = 1);
        send!((); c0, "setDestinationAlphaBlendFactor:", usize = 5);
        send!((); desc, "setDepthAttachmentPixelFormat:", usize = DEPTH32);
        err = std::ptr::null_mut();
        let pipeline = send!(Id; device, "newRenderPipelineStateWithDescriptor:error:", Id = desc, *mut Id = &mut err);
        if pipeline.is_null() {
            return Err(format!("Metal pipeline creation failed: {}", error_text(err)));
        }

        let dd = send!(Id; send!(Id; class("MTLDepthStencilDescriptor"), "alloc"), "init");
        send!((); dd, "setDepthCompareFunction:", usize = 1);
        send!((); dd, "setDepthWriteEnabled:", bool = true);
        let depth_state = send!(Id; device, "newDepthStencilStateWithDescriptor:", Id = dd);

        let queue = send!(Id; device, "newCommandQueue");
        if queue.is_null() || depth_state.is_null() {
            return Err("Metal queue creation failed".into());
        }
        let depth_tex = make_depth_texture(device, w, h);
        let dynamic_bytes = MAX_DYNAMIC_VERTICES * std::mem::size_of::<Vertex>();
        let dynamic_buffers = [
            send!(Id; device, "newBufferWithLength:options:", usize = dynamic_bytes, usize = 0),
            send!(Id; device, "newBufferWithLength:options:", usize = dynamic_bytes, usize = 0),
        ];
        if dynamic_buffers.iter().any(|b| b.is_null()) {
            return Err("Metal dynamic vertex buffer allocation failed".into());
        }
        Ok(MetalRenderer {
            device,
            queue,
            pipeline,
            depth_state,
            depth_tex,
            layer,
            vertex_buf: std::ptr::null_mut(),
            vertex_count: 0,
            dynamic_buffers,
            dynamic_count: 0,
            in_flight: [std::ptr::null_mut(); 2],
            frame: 0,
            w,
            h,
        })
    }

    unsafe fn encode(&self, target: Id, u: &FrameUniforms, slot: usize) -> Id {
        let pass = send!(Id; class("MTLRenderPassDescriptor"), "renderPassDescriptor");
        let color = send!(Id; send!(Id; pass, "colorAttachments"), "objectAtIndexedSubscript:", usize = 0);
        send!((); color, "setTexture:", Id = target);
        send!((); color, "setLoadAction:", usize = 2);
        send!((); color, "setStoreAction:", usize = 1);
        send!((); color, "setClearColor:", ClearColor = ClearColor {
            r: u.sky[0] as f64, g: u.sky[1] as f64, b: u.sky[2] as f64, a: 1.0 });
        let depth = send!(Id; pass, "depthAttachment");
        send!((); depth, "setTexture:", Id = self.depth_tex);
        send!((); depth, "setLoadAction:", usize = 2);
        send!((); depth, "setStoreAction:", usize = 0);
        send!((); depth, "setClearDepth:", f64 = 1.0);

        let cb = send!(Id; self.queue, "commandBuffer");
        let enc = send!(Id; cb, "renderCommandEncoderWithDescriptor:", Id = pass);
        send!((); enc, "setRenderPipelineState:", Id = self.pipeline);
        send!((); enc, "setDepthStencilState:", Id = self.depth_state);
        let uniforms = u as *const FrameUniforms as *const c_void;
        let uniforms_size = std::mem::size_of::<FrameUniforms>();
        send!((); enc, "setVertexBytes:length:atIndex:", *const c_void = uniforms, usize = uniforms_size, usize = 1);
        send!((); enc, "setFragmentBytes:length:atIndex:", *const c_void = uniforms, usize = uniforms_size, usize = 1);
        for (buffer, count) in [(self.vertex_buf, self.vertex_count), (self.dynamic_buffers[slot], self.dynamic_count)] {
            if count > 0 && !buffer.is_null() {
                send!((); enc, "setVertexBuffer:offset:atIndex:", Id = buffer, usize = 0, usize = 0);
                send!((); enc, "drawPrimitives:vertexStart:vertexCount:", usize = 3, usize = 0, usize = count);
            }
        }
        send!((); enc, "endEncoding");
        cb
    }
}

impl MetalRenderer {
    unsafe fn wait_for_slot(&mut self, slot: usize) {
        let previous = self.in_flight[slot];
        if !previous.is_null() {
            send!((); previous, "waitUntilCompleted");
            send!((); previous, "release");
            self.in_flight[slot] = std::ptr::null_mut();
        }
    }

    unsafe fn write_dynamic(&mut self, scene: &Scene, slot: usize) {
        let verts = build_dynamic_vertices(scene);
        let destination = send!(Id; self.dynamic_buffers[slot], "contents") as *mut u8;
        std::ptr::copy_nonoverlapping(verts.as_ptr() as *const u8, destination, verts.len() * std::mem::size_of::<Vertex>());
        self.dynamic_count = verts.len();
    }
}

impl Renderer for MetalRenderer {
    fn name(&self) -> &'static str {
        "metal"
    }

    fn upload_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let verts = build_vertices(scene);
        unsafe {
            self.vertex_count = verts.len();
            if verts.is_empty() {
                return Ok(());
            }
            if !self.vertex_buf.is_null() {
                send!((); self.vertex_buf, "release");
            }
            self.vertex_buf = send!(Id; self.device, "newBufferWithBytes:length:options:",
                *const c_void = verts.as_ptr() as *const c_void,
                usize = verts.len() * std::mem::size_of::<Vertex>(), usize = 0);
            if self.vertex_buf.is_null() {
                return Err("Metal vertex buffer allocation failed".into());
            }
        }
        Ok(())
    }

    fn render(&mut self, scene: &Scene, cam: &Camera, _window: &mut dyn Window) -> Result<(), String> {
        if self.layer.is_null() {
            return Err("headless Metal renderer has no window".into());
        }
        let u = frame_uniforms(scene, cam, self.w as f32 / self.h as f32, false);
        unsafe {
            let pool = objc_autoreleasePoolPush();
            let drawable = send!(Id; self.layer, "nextDrawable");
            if !drawable.is_null() {
                let tex = send!(Id; drawable, "texture");
                let slot = self.frame % 2;
                self.frame += 1;
                self.wait_for_slot(slot);
                self.write_dynamic(scene, slot);
                let cb = self.encode(tex, &u, slot);
                send!((); cb, "presentDrawable:", Id = drawable);
                send!(Id; cb, "retain");
                self.in_flight[slot] = cb;
                send!((); cb, "commit");
            }
            objc_autoreleasePoolPop(pool);
        }
        Ok(())
    }

    fn render_offscreen(&mut self, scene: &Scene, cam: &Camera) -> Result<Framebuffer, String> {
        let (w, h) = (self.w, self.h);
        let u = frame_uniforms(scene, cam, w as f32 / h as f32, false);
        unsafe {
            let pool = objc_autoreleasePoolPush();
            let desc = send!(Id; class("MTLTextureDescriptor"),
                "texture2DDescriptorWithPixelFormat:width:height:mipmapped:",
                usize = BGRA8, usize = w, usize = h, bool = false);
            send!((); desc, "setUsage:", usize = 4 | 1);
            let storage = if cfg!(target_arch = "aarch64") { 0 } else { 1 };
            send!((); desc, "setStorageMode:", usize = storage);
            let tex = send!(Id; self.device, "newTextureWithDescriptor:", Id = desc);
            if tex.is_null() {
                objc_autoreleasePoolPop(pool);
                return Err("could not create offscreen texture".into());
            }
            self.wait_for_slot(0);
            self.write_dynamic(scene, 0);
            let cb = self.encode(tex, &u, 0);
            send!((); cb, "commit");
            if storage == 1 {
                let cb2 = send!(Id; self.queue, "commandBuffer");
                let blit = send!(Id; cb2, "blitCommandEncoder");
                send!((); blit, "synchronizeResource:", Id = tex);
                send!((); blit, "endEncoding");
                send!((); cb2, "commit");
                send!((); cb2, "waitUntilCompleted");
            }
            send!((); cb, "waitUntilCompleted");
            let mut bytes = vec![0u8; w * h * 4];
            send!((); tex, "getBytes:bytesPerRow:fromRegion:mipmapLevel:",
                *mut c_void = bytes.as_mut_ptr() as *mut c_void, usize = w * 4,
                Region = Region { origin: [0, 0, 0], size: [w, h, 1] }, usize = 0);
            objc_autoreleasePoolPop(pool);
            let mut fb = Framebuffer::new(w, h);
            for (p, b) in fb.pixels.iter_mut().zip(bytes.chunks_exact(4)) {
                *p = (b[2] as u32) << 16 | (b[1] as u32) << 8 | b[0] as u32;
            }
            Ok(fb)
        }
    }
}
