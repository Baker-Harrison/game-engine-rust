
use super::{build_dynamic_vertices, build_vertices, draw_counts, draw_plan, frame_uniforms, pack_materials, shadow_uniforms, DrawRange, ShadowUniforms, SHADOW_SIZE, FrameUniforms, MaterialArrays, MaterialLevel, Renderer, Vertex, MAX_DYNAMIC_VERTICES};
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
const RENDER_AND_READ: usize = 5;

const SHADER: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Vertex { float4 pos; float4 normal; float4 color; float4 uv; };
struct U { float4x4 view_proj; float4 eye; float4 sky; float4 sun; float4 params; };
struct ShadowU { float4x4 light_view_proj; float4 params; };
struct VOut {
    float4 position [[position]];
    float3 world;
    float3 normal;
    float4 color;
    float depth;
    float kind;
    float2 uv;
    float layer [[flat]];
    float mode [[flat]];
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
    o.uv = v.uv.xy;
    o.layer = v.uv.z;
    o.mode = v.uv.w;
    return o;
}

vertex float4 vs_shadow(uint vid [[vertex_id]], constant Vertex* verts [[buffer(0)]], constant ShadowU& shadow [[buffer(2)]]) {
    return shadow.light_view_proj * float4(verts[vid].pos.xyz, 1.0);
}

float sun_visibility(float3 position, float3 normal, constant ShadowU& shadow, depth2d<float> shadow_map, sampler shadow_sampler) {
    if (shadow.params.x < 0.5) return 1.0;
    float3 offset_position = position + normal * (shadow.params.y * 1.5);
    float4 clip = shadow.light_view_proj * float4(offset_position, 1.0);
    float2 uv = float2(clip.x * 0.5 + 0.5, 0.5 - clip.y * 0.5);
    float texel = 1.0 / 2048.0;
    float lit = 0.0;
    for (int y = -1; y <= 1; y++) {
        for (int x = -1; x <= 1; x++) {
            lit += shadow_map.sample_compare(shadow_sampler, uv + float2(float(x), float(y)) * texel, clip.z - 0.0006);
        }
    }
    return lit / 9.0;
}

fragment float4 fs(VOut i [[stage_in]], constant U& u [[buffer(1)]],
                   texture2d_array<float> albedo_tex [[texture(0)]],
                   texture2d_array<float> normal_tex [[texture(1)]],
                   texture2d_array<float> packed_tex [[texture(2)]],
                   constant ShadowU& shadow_u [[buffer(2)]],
                   depth2d<float> shadow_map [[texture(3)]],
                   sampler material_sampler [[sampler(0)]],
                   sampler shadow_sampler [[sampler(1)]]) {
    if (i.kind > 1.5) return i.color;
    float3 n = normalize(i.normal);
    float3 to_eye = normalize(u.eye.xyz - i.world);
    if (dot(n, to_eye) < 0.0) n = -n;
    float3 base = i.color.xyz;
    float roughness = 0.85;
    float metallic = 0.0;
    float occlusion = 1.0;
    float alpha = 1.0;
    bool textured = i.layer > 0.5 && i.kind < 0.5;
    float3 dp1 = dfdx(i.world);
    float3 dp2 = dfdy(i.world);
    float2 duv1 = dfdx(i.uv);
    float2 duv2 = dfdy(i.uv);
    if (textured) {
        uint slice = uint(i.layer - 1.0 + 0.5);
        float3 dp2perp = cross(dp2, n);
        float3 dp1perp = cross(n, dp1);
        float3 tangent = dp2perp * duv1.x + dp1perp * duv2.x;
        float3 bitangent = dp2perp * duv1.y + dp1perp * duv2.y;
        float inverse_scale = rsqrt(max(max(dot(tangent, tangent), dot(bitangent, bitangent)), 1e-20));
        float3 texel_normal = normal_tex.sample(material_sampler, i.uv, slice).xyz * 2.0 - 1.0;
        float3 perturbed = tangent * inverse_scale * texel_normal.x + bitangent * inverse_scale * texel_normal.y + n * texel_normal.z;
        if (dot(perturbed, perturbed) > 1e-8) n = normalize(perturbed);
        float4 albedo = albedo_tex.sample(material_sampler, i.uv, slice);
        base = albedo.xyz;
        if (i.mode > 0.5 && i.mode < 1.5 && albedo.a < 0.5) discard_fragment();
        if (i.mode > 1.5) alpha = albedo.a;
        float3 surface = packed_tex.sample(material_sampler, i.uv, slice).xyz;
        roughness = max(surface.x, 0.04);
        metallic = surface.y;
        occlusion = surface.z;
    }
    float ambient = u.sun.w;
    float facing = max(dot(n, u.sun.xyz), 0.0);
    float shadow = i.kind > 0.5 ? 1.0 : sun_visibility(i.world, n, shadow_u, shadow_map, shadow_sampler);
    float shade = ambient * occlusion + (1.0 - ambient) * facing * shadow;
    if (i.kind > 0.5) shade = 1.0;
    float3 lit = base * shade;
    if (textured) {
        float3 half_vector = normalize(u.sun.xyz + to_eye);
        float r4 = max(roughness * roughness * roughness * roughness, 0.0004);
        float sharpness = min(2.0 / r4 - 2.0, 2000.0);
        float highlight = pow(max(dot(n, half_vector), 0.0), sharpness);
        float strength = highlight * facing * (1.0 - roughness) * 0.6 * shadow;
        float3 tint = mix(float3(1.0), base, metallic);
        lit = base * (1.0 - metallic * 0.8) * shade + tint * strength;
    }
    float t = clamp(i.depth / u.params.x, 0.0, 1.0);
    t = t * t;
    return float4(mix(lit, u.sky.xyz, t), alpha);
}
"#;

pub struct MetalRenderer {
    device: Id,
    queue: Id,
    pipeline: Id,
    depth_state: Id,
    blend_depth_state: Id,
    shadow_pipeline: Id,
    shadow_texture: Id,
    shadow_sampler: Id,
    shadow_draws: Vec<DrawRange>,
    static_opaque: usize,
    draws: Vec<DrawRange>,
    depth_tex: Id,
    layer: Id,
    vertex_buf: Id,
    vertex_count: usize,
    dynamic_buffers: [Id; 2],
    dynamic_count: usize,
    material_textures: [Id; 3],
    material_sampler: Id,
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
            send!((); layer, "setContentsScale:", f64 = window.scale_factor() as f64);
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

        send!((); dd, "setDepthWriteEnabled:", bool = false);
        let blend_depth_state = send!(Id; device, "newDepthStencilStateWithDescriptor:", Id = dd);
        let queue = send!(Id; device, "newCommandQueue");
        if queue.is_null() || depth_state.is_null() || blend_depth_state.is_null() {
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
        let mut renderer = MetalRenderer {
            device,
            queue,
            pipeline,
            depth_state,
            blend_depth_state,
            shadow_pipeline: std::ptr::null_mut(),
            shadow_texture: std::ptr::null_mut(),
            shadow_sampler: std::ptr::null_mut(),
            shadow_draws: Vec::new(),
            static_opaque: 0,
            draws: Vec::new(),
            depth_tex,
            layer,
            vertex_buf: std::ptr::null_mut(),
            vertex_count: 0,
            dynamic_buffers,
            dynamic_count: 0,
            material_textures: [std::ptr::null_mut(); 3],
            material_sampler: std::ptr::null_mut(),
            in_flight: [std::ptr::null_mut(); 2],
            frame: 0,
            w,
            h,
        };
        let sampler_desc = send!(Id; send!(Id; class("MTLSamplerDescriptor"), "alloc"), "init");
        send!((); sampler_desc, "setMinFilter:", usize = 1);
        send!((); sampler_desc, "setMagFilter:", usize = 1);
        send!((); sampler_desc, "setMipFilter:", usize = 2);
        send!((); sampler_desc, "setSAddressMode:", usize = 2);
        send!((); sampler_desc, "setTAddressMode:", usize = 2);
        renderer.material_sampler = send!(Id; device, "newSamplerStateWithDescriptor:", Id = sampler_desc);
        if renderer.material_sampler.is_null() {
            return Err("Metal sampler creation failed".into());
        }
        renderer.upload_materials(None)?;

        let shadow_vs = send!(Id; lib, "newFunctionWithName:", Id = nsstring("vs_shadow"));
        let shadow_desc = send!(Id; send!(Id; class("MTLRenderPipelineDescriptor"), "alloc"), "init");
        send!((); shadow_desc, "setVertexFunction:", Id = shadow_vs);
        send!((); shadow_desc, "setDepthAttachmentPixelFormat:", usize = DEPTH32);
        err = std::ptr::null_mut();
        renderer.shadow_pipeline = send!(Id; device, "newRenderPipelineStateWithDescriptor:error:", Id = shadow_desc, *mut Id = &mut err);
        if renderer.shadow_pipeline.is_null() {
            return Err(format!("Metal shadow pipeline creation failed: {}", error_text(err)));
        }
        let shadow_tex_desc = send!(Id; class("MTLTextureDescriptor"),
            "texture2DDescriptorWithPixelFormat:width:height:mipmapped:",
            usize = DEPTH32, usize = SHADOW_SIZE, usize = SHADOW_SIZE, bool = false);
        send!((); shadow_tex_desc, "setUsage:", usize = RENDER_AND_READ);
        send!((); shadow_tex_desc, "setStorageMode:", usize = 2);
        renderer.shadow_texture = send!(Id; device, "newTextureWithDescriptor:", Id = shadow_tex_desc);
        let shadow_sampler_desc = send!(Id; send!(Id; class("MTLSamplerDescriptor"), "alloc"), "init");
        send!((); shadow_sampler_desc, "setMinFilter:", usize = 1);
        send!((); shadow_sampler_desc, "setMagFilter:", usize = 1);
        send!((); shadow_sampler_desc, "setSAddressMode:", usize = 0);
        send!((); shadow_sampler_desc, "setTAddressMode:", usize = 0);
        send!((); shadow_sampler_desc, "setCompareFunction:", usize = 3);
        renderer.shadow_sampler = send!(Id; device, "newSamplerStateWithDescriptor:", Id = shadow_sampler_desc);
        if renderer.shadow_texture.is_null() || renderer.shadow_sampler.is_null() {
            return Err("Metal shadow map creation failed".into());
        }
        Ok(renderer)
    }

    unsafe fn upload_materials(&mut self, arrays: Option<&MaterialArrays>) -> Result<(), String> {
        let fallback = MaterialArrays {
            layers: 1,
            levels: vec![MaterialLevel { width: 1, height: 1, albedo: vec![255; 4], normal: vec![128, 128, 255, 255], packed: vec![220, 0, 255, 255] }],
        };
        let arrays = arrays.unwrap_or(&fallback);
        let storage = if cfg!(target_arch = "aarch64") { 0 } else { 1 };
        let base = &arrays.levels[0];
        let kinds: [fn(&MaterialLevel) -> &Vec<u8>; 3] = [|l| &l.albedo, |l| &l.normal, |l| &l.packed];
        for (index, kind) in kinds.iter().enumerate() {
            let desc = send!(Id; send!(Id; class("MTLTextureDescriptor"), "alloc"), "init");
            send!((); desc, "setTextureType:", usize = 3);
            send!((); desc, "setPixelFormat:", usize = 70);
            send!((); desc, "setWidth:", usize = base.width);
            send!((); desc, "setHeight:", usize = base.height);
            send!((); desc, "setMipmapLevelCount:", usize = arrays.levels.len());
            send!((); desc, "setArrayLength:", usize = arrays.layers);
            send!((); desc, "setUsage:", usize = 1);
            send!((); desc, "setStorageMode:", usize = storage);
            let texture = send!(Id; self.device, "newTextureWithDescriptor:", Id = desc);
            if texture.is_null() {
                return Err("Metal material texture creation failed".into());
            }
            for (level_index, level) in arrays.levels.iter().enumerate() {
                let data = kind(level);
                let layer_bytes = level.width * level.height * 4;
                for layer in 0..arrays.layers {
                    send!((); texture, "replaceRegion:mipmapLevel:slice:withBytes:bytesPerRow:bytesPerImage:",
                        Region = Region { origin: [0, 0, 0], size: [level.width, level.height, 1] },
                        usize = level_index, usize = layer,
                        *const c_void = data.as_ptr().add(layer * layer_bytes) as *const c_void,
                        usize = level.width * 4, usize = layer_bytes);
                }
            }
            if !self.material_textures[index].is_null() {
                send!((); self.material_textures[index], "release");
            }
            self.material_textures[index] = texture;
        }
        Ok(())
    }

    unsafe fn encode(&self, target: Id, u: &FrameUniforms, shadow: &ShadowUniforms, slot: usize) -> Id {
        let cb = send!(Id; self.queue, "commandBuffer");
        let shadow_pass = send!(Id; class("MTLRenderPassDescriptor"), "renderPassDescriptor");
        let shadow_depth = send!(Id; shadow_pass, "depthAttachment");
        send!((); shadow_depth, "setTexture:", Id = self.shadow_texture);
        send!((); shadow_depth, "setLoadAction:", usize = 2);
        send!((); shadow_depth, "setStoreAction:", usize = 1);
        send!((); shadow_depth, "setClearDepth:", f64 = 1.0);
        let shadow_enc = send!(Id; cb, "renderCommandEncoderWithDescriptor:", Id = shadow_pass);
        if shadow.params[0] > 0.5 {
            send!((); shadow_enc, "setRenderPipelineState:", Id = self.shadow_pipeline);
            send!((); shadow_enc, "setDepthStencilState:", Id = self.depth_state);
            send!((); shadow_enc, "setVertexBytes:length:atIndex:", *const c_void = shadow as *const ShadowUniforms as *const c_void, usize = std::mem::size_of::<ShadowUniforms>(), usize = 2);
            for range in &self.shadow_draws {
                let buffer = if range.dynamic { self.dynamic_buffers[slot] } else { self.vertex_buf };
                if buffer.is_null() {
                    continue;
                }
                send!((); shadow_enc, "setVertexBuffer:offset:atIndex:", Id = buffer, usize = 0, usize = 0);
                send!((); shadow_enc, "drawPrimitives:vertexStart:vertexCount:", usize = 3, usize = range.first, usize = range.count);
            }
        }
        send!((); shadow_enc, "endEncoding");

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

        let enc = send!(Id; cb, "renderCommandEncoderWithDescriptor:", Id = pass);
        send!((); enc, "setRenderPipelineState:", Id = self.pipeline);
        send!((); enc, "setDepthStencilState:", Id = self.depth_state);
        let uniforms = u as *const FrameUniforms as *const c_void;
        let uniforms_size = std::mem::size_of::<FrameUniforms>();
        send!((); enc, "setVertexBytes:length:atIndex:", *const c_void = uniforms, usize = uniforms_size, usize = 1);
        send!((); enc, "setFragmentBytes:length:atIndex:", *const c_void = uniforms, usize = uniforms_size, usize = 1);
        for (index, texture) in self.material_textures.iter().enumerate() {
            send!((); enc, "setFragmentTexture:atIndex:", Id = *texture, usize = index);
        }
        send!((); enc, "setFragmentSamplerState:atIndex:", Id = self.material_sampler, usize = 0);
        send!((); enc, "setFragmentBytes:length:atIndex:", *const c_void = shadow as *const ShadowUniforms as *const c_void, usize = std::mem::size_of::<ShadowUniforms>(), usize = 2);
        send!((); enc, "setFragmentTexture:atIndex:", Id = self.shadow_texture, usize = 3);
        send!((); enc, "setFragmentSamplerState:atIndex:", Id = self.shadow_sampler, usize = 1);
        for range in &self.draws {
            let buffer = if range.dynamic { self.dynamic_buffers[slot] } else { self.vertex_buf };
            if buffer.is_null() {
                continue;
            }
            let state = if range.blend_pass { self.blend_depth_state } else { self.depth_state };
            send!((); enc, "setDepthStencilState:", Id = state);
            send!((); enc, "setVertexBuffer:offset:atIndex:", Id = buffer, usize = 0, usize = 0);
            send!((); enc, "drawPrimitives:vertexStart:vertexCount:", usize = 3, usize = range.first, usize = range.count);
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
        let mut counts = draw_counts(scene, self.vertex_count, verts.len());
        counts.static_opaque = self.static_opaque.min(self.vertex_count);
        self.draws = draw_plan(&counts);
        self.shadow_draws = [
            DrawRange { blend_pass: false, dynamic: false, first: 0, count: counts.static_opaque },
            DrawRange { blend_pass: false, dynamic: true, first: 0, count: counts.dynamic_opaque },
        ]
        .into_iter()
        .filter(|r| r.count > 0)
        .collect();
    }
}

impl Renderer for MetalRenderer {
    fn name(&self) -> &'static str {
        "metal"
    }

    fn upload_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let verts = build_vertices(scene);
        unsafe {
            let arrays = pack_materials(scene);
            self.upload_materials(arrays.as_ref())?;
            self.vertex_count = verts.len();
            self.static_opaque = super::static_opaque_vertex_count(scene).min(verts.len());
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
                let shadow = shadow_uniforms(scene, false, SHADOW_SIZE);
                let cb = self.encode(tex, &u, &shadow, slot);
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
            let shadow = shadow_uniforms(scene, false, SHADOW_SIZE);
            let cb = self.encode(tex, &u, &shadow, 0);
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
