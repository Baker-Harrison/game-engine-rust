
#![allow(non_snake_case, non_camel_case_types, clippy::too_many_arguments)]

use super::{build_dynamic_vertices, build_vertices, frame_uniforms, FrameUniforms, Renderer, Vertex, MAX_DYNAMIC_VERTICES};
use crate::camera::Camera;
use crate::platform::{NativeHandle, Window};
use crate::render::Framebuffer;
use crate::scene::Scene;
use std::ffi::{c_char, c_void, CStr};

type Handle = u64;
type Ptr = *mut c_void;
type VkResult = i32;

use crate::platform::dl;

type Gipa = unsafe extern "system" fn(Ptr, *const c_char) -> *mut c_void;

#[repr(C)] struct ApplicationInfo { s_type: u32, p_next: *const c_void, app_name: *const c_char, app_version: u32, engine_name: *const c_char, engine_version: u32, api_version: u32 }
#[repr(C)] struct InstanceCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, app_info: *const ApplicationInfo, layer_count: u32, layers: *const *const c_char, ext_count: u32, exts: *const *const c_char }
#[repr(C)] struct DeviceQueueCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, family: u32, count: u32, priorities: *const f32 }
#[repr(C)] struct DeviceCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, queue_count: u32, queues: *const DeviceQueueCreateInfo, layer_count: u32, layers: *const *const c_char, ext_count: u32, exts: *const *const c_char, features: *const c_void }
#[repr(C)] #[derive(Clone, Copy)] struct QueueFamilyProperties { flags: u32, count: u32, ts_bits: u32, granularity: [u32; 3] }
#[repr(C)] #[derive(Clone, Copy, Default)] struct MemoryType { flags: u32, heap: u32 }
#[repr(C)] #[derive(Clone, Copy, Default)] struct MemoryHeap { size: u64, flags: u32 }
#[repr(C)] struct MemoryProperties { type_count: u32, types: [MemoryType; 32], heap_count: u32, heaps: [MemoryHeap; 16] }
#[repr(C)] #[derive(Default)] struct MemoryRequirements { size: u64, alignment: u64, type_bits: u32 }
#[repr(C)] struct MemoryAllocateInfo { s_type: u32, p_next: *const c_void, size: u64, type_index: u32 }
#[repr(C)] struct BufferCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, size: u64, usage: u32, sharing: u32, qfi_count: u32, qfi: *const u32 }
#[repr(C)] struct ImageCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, image_type: u32, format: u32, extent: [u32; 3], mips: u32, layers: u32, samples: u32, tiling: u32, usage: u32, sharing: u32, qfi_count: u32, qfi: *const u32, initial_layout: u32 }
#[repr(C)] #[derive(Clone, Copy)] struct SubresourceRange { aspect: u32, base_mip: u32, mips: u32, base_layer: u32, layers: u32 }
#[repr(C)] struct ImageViewCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, image: Handle, view_type: u32, format: u32, components: [u32; 4], range: SubresourceRange }
#[repr(C)] struct ShaderModuleCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, code_size: usize, code: *const u32 }
#[repr(C)] struct AttachmentDescription { flags: u32, format: u32, samples: u32, load: u32, store: u32, sload: u32, sstore: u32, initial: u32, final_: u32 }
#[repr(C)] struct AttachmentReference { attachment: u32, layout: u32 }
#[repr(C)] struct SubpassDescription { flags: u32, bind_point: u32, input_count: u32, input: *const AttachmentReference, color_count: u32, color: *const AttachmentReference, resolve: *const AttachmentReference, depth: *const AttachmentReference, preserve_count: u32, preserve: *const u32 }
#[repr(C)] struct SubpassDependency { src: u32, dst: u32, src_stage: u32, dst_stage: u32, src_access: u32, dst_access: u32, flags: u32 }
#[repr(C)] struct RenderPassCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, att_count: u32, atts: *const AttachmentDescription, sub_count: u32, subs: *const SubpassDescription, dep_count: u32, deps: *const SubpassDependency }
#[repr(C)] struct FramebufferCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, render_pass: Handle, att_count: u32, atts: *const Handle, width: u32, height: u32, layers: u32 }
#[repr(C)] struct StageInfo { s_type: u32, p_next: *const c_void, flags: u32, stage: u32, module: Handle, name: *const c_char, spec: *const c_void }
#[repr(C)] struct VertexBinding { binding: u32, stride: u32, rate: u32 }
#[repr(C)] struct VertexAttr { location: u32, binding: u32, format: u32, offset: u32 }
#[repr(C)] struct VertexInputState { s_type: u32, p_next: *const c_void, flags: u32, bind_count: u32, binds: *const VertexBinding, attr_count: u32, attrs: *const VertexAttr }
#[repr(C)] struct InputAssemblyState { s_type: u32, p_next: *const c_void, flags: u32, topology: u32, restart: u32 }
#[repr(C)] struct Viewport { x: f32, y: f32, w: f32, h: f32, min_depth: f32, max_depth: f32 }
#[repr(C)] #[derive(Clone, Copy)] struct Rect2D { x: i32, y: i32, w: u32, h: u32 }
#[repr(C)] struct ViewportState { s_type: u32, p_next: *const c_void, flags: u32, vp_count: u32, vps: *const Viewport, sc_count: u32, scs: *const Rect2D }
#[repr(C)] struct RasterState { s_type: u32, p_next: *const c_void, flags: u32, clamp: u32, discard: u32, polygon: u32, cull: u32, front: u32, bias: u32, bias_c: f32, bias_clamp: f32, bias_slope: f32, line_width: f32 }
#[repr(C)] struct MultisampleState { s_type: u32, p_next: *const c_void, flags: u32, samples: u32, sample_shading: u32, min_sample: f32, mask: *const u32, a2c: u32, a21: u32 }
#[repr(C)] #[derive(Clone, Copy, Default)] struct StencilOp { fail: u32, pass: u32, depth_fail: u32, compare: u32, cmask: u32, wmask: u32, reference: u32 }
#[repr(C)] struct DepthStencilState { s_type: u32, p_next: *const c_void, flags: u32, test: u32, write: u32, compare: u32, bounds: u32, stencil: u32, front: StencilOp, back: StencilOp, min_depth: f32, max_depth: f32 }
#[repr(C)] struct BlendAttachment { enable: u32, sc: u32, dc: u32, co: u32, sa: u32, da: u32, ao: u32, mask: u32 }
#[repr(C)] struct BlendState { s_type: u32, p_next: *const c_void, flags: u32, logic_en: u32, logic: u32, att_count: u32, atts: *const BlendAttachment, consts: [f32; 4] }
#[repr(C)] struct PushRange { stages: u32, offset: u32, size: u32 }
#[repr(C)] struct PipelineLayoutCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, set_count: u32, sets: *const Handle, pc_count: u32, pcs: *const PushRange }
#[repr(C)] struct GraphicsPipelineCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, stage_count: u32, stages: *const StageInfo, vertex: *const VertexInputState, assembly: *const InputAssemblyState, tess: *const c_void, viewport: *const ViewportState, raster: *const RasterState, ms: *const MultisampleState, ds: *const DepthStencilState, blend: *const BlendState, dynamic: *const c_void, layout: Handle, render_pass: Handle, subpass: u32, base: Handle, base_index: i32 }
#[repr(C)] struct CommandPoolCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, family: u32 }
#[repr(C)] struct CommandBufferAllocateInfo { s_type: u32, p_next: *const c_void, pool: Handle, level: u32, count: u32 }
#[repr(C)] struct CommandBufferBeginInfo { s_type: u32, p_next: *const c_void, flags: u32, inheritance: *const c_void }
#[repr(C)] struct RenderPassBeginInfo { s_type: u32, p_next: *const c_void, render_pass: Handle, framebuffer: Handle, area: Rect2D, clear_count: u32, clears: *const [f32; 4] }
#[repr(C)] struct SubmitInfo { s_type: u32, p_next: *const c_void, wait_count: u32, waits: *const Handle, stages: *const u32, cmd_count: u32, cmds: *const Ptr, signal_count: u32, signals: *const Handle }
#[repr(C)] struct SimpleCreateInfo { s_type: u32, p_next: *const c_void, flags: u32 }
#[repr(C)] struct SubresourceLayers { aspect: u32, mip: u32, base_layer: u32, layers: u32 }
#[repr(C)] struct BufferImageCopy { buffer_offset: u64, row_length: u32, image_height: u32, sub: SubresourceLayers, offset: [i32; 3], extent: [u32; 3] }
#[repr(C)] struct SwapchainCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, surface: Handle, min_images: u32, format: u32, color_space: u32, extent: [u32; 2], layers: u32, usage: u32, sharing: u32, qfi_count: u32, qfi: *const u32, transform: u32, alpha: u32, present_mode: u32, clipped: u32, old: Handle }
#[repr(C)] struct PresentInfo { s_type: u32, p_next: *const c_void, wait_count: u32, waits: *const Handle, sc_count: u32, scs: *const Handle, indices: *const u32, results: *mut i32 }
#[repr(C)] #[derive(Default)] struct SurfaceCaps { min_images: u32, max_images: u32, current: [u32; 2], min_extent: [u32; 2], max_extent: [u32; 2], max_layers: u32, transforms: u32, current_transform: u32, alpha: u32, usage: u32 }
#[repr(C)] #[derive(Clone, Copy, Default)] struct SurfaceFormat { format: u32, color_space: u32 }
#[repr(C)] struct XlibSurfaceCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, dpy: *mut c_void, window: std::ffi::c_ulong }
#[repr(C)] struct Win32SurfaceCreateInfo { s_type: u32, p_next: *const c_void, flags: u32, hinstance: *mut c_void, hwnd: *mut c_void }

macro_rules! vk_table {
    ($name:ident { $($f:ident($($a:ty),*) -> $r:ty;)* }) => {
        struct $name { $($f: unsafe extern "system" fn($($a),*) -> $r),* }
        impl $name {
            unsafe fn load(gipa: Gipa, inst: Ptr) -> Result<$name, String> {
                Ok($name { $($f: {
                    let p = gipa(inst, concat!("vk", stringify!($f), "\0").as_ptr() as *const c_char);
                    if p.is_null() { return Err(format!("Vulkan function vk{} missing", stringify!($f))); }
                    std::mem::transmute::<*mut c_void, unsafe extern "system" fn($($a),*) -> $r>(p)
                }),* })
            }
        }
    };
}

vk_table!(Fns {
    DestroyInstance(Ptr, *const c_void) -> ();
    EnumeratePhysicalDevices(Ptr, *mut u32, *mut Ptr) -> VkResult;
    GetPhysicalDeviceProperties(Ptr, *mut u8) -> ();
    GetPhysicalDeviceQueueFamilyProperties(Ptr, *mut u32, *mut QueueFamilyProperties) -> ();
    GetPhysicalDeviceMemoryProperties(Ptr, *mut MemoryProperties) -> ();
    CreateDevice(Ptr, *const DeviceCreateInfo, *const c_void, *mut Ptr) -> VkResult;
    GetDeviceQueue(Ptr, u32, u32, *mut Ptr) -> ();
    CreateImage(Ptr, *const ImageCreateInfo, *const c_void, *mut Handle) -> VkResult;
    GetImageMemoryRequirements(Ptr, Handle, *mut MemoryRequirements) -> ();
    AllocateMemory(Ptr, *const MemoryAllocateInfo, *const c_void, *mut Handle) -> VkResult;
    BindImageMemory(Ptr, Handle, Handle, u64) -> VkResult;
    CreateImageView(Ptr, *const ImageViewCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreateBuffer(Ptr, *const BufferCreateInfo, *const c_void, *mut Handle) -> VkResult;
    GetBufferMemoryRequirements(Ptr, Handle, *mut MemoryRequirements) -> ();
    BindBufferMemory(Ptr, Handle, Handle, u64) -> VkResult;
    MapMemory(Ptr, Handle, u64, u64, u32, *mut *mut c_void) -> VkResult;
    UnmapMemory(Ptr, Handle) -> ();
    CreateShaderModule(Ptr, *const ShaderModuleCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreateRenderPass(Ptr, *const RenderPassCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreateFramebuffer(Ptr, *const FramebufferCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreatePipelineLayout(Ptr, *const PipelineLayoutCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreateGraphicsPipelines(Ptr, Handle, u32, *const GraphicsPipelineCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreateCommandPool(Ptr, *const CommandPoolCreateInfo, *const c_void, *mut Handle) -> VkResult;
    AllocateCommandBuffers(Ptr, *const CommandBufferAllocateInfo, *mut Ptr) -> VkResult;
    ResetCommandBuffer(Ptr, u32) -> VkResult;
    BeginCommandBuffer(Ptr, *const CommandBufferBeginInfo) -> VkResult;
    EndCommandBuffer(Ptr) -> VkResult;
    CmdBeginRenderPass(Ptr, *const RenderPassBeginInfo, u32) -> ();
    CmdEndRenderPass(Ptr) -> ();
    CmdBindPipeline(Ptr, u32, Handle) -> ();
    CmdBindVertexBuffers(Ptr, u32, u32, *const Handle, *const u64) -> ();
    CmdPushConstants(Ptr, Handle, u32, u32, u32, *const c_void) -> ();
    CmdDraw(Ptr, u32, u32, u32, u32) -> ();
    CmdCopyImageToBuffer(Ptr, Handle, u32, Handle, u32, *const BufferImageCopy) -> ();
    QueueSubmit(Ptr, u32, *const SubmitInfo, Handle) -> VkResult;
    QueueWaitIdle(Ptr) -> VkResult;
    DeviceWaitIdle(Ptr) -> VkResult;
    CreateSemaphore(Ptr, *const SimpleCreateInfo, *const c_void, *mut Handle) -> VkResult;
    CreateFence(Ptr, *const SimpleCreateInfo, *const c_void, *mut Handle) -> VkResult;
    WaitForFences(Ptr, u32, *const Handle, u32, u64) -> VkResult;
    ResetFences(Ptr, u32, *const Handle) -> VkResult;
});

vk_table!(SwapFns {
    DestroySurfaceKHR(Ptr, Handle, *const c_void) -> ();
    GetPhysicalDeviceSurfaceSupportKHR(Ptr, u32, Handle, *mut u32) -> VkResult;
    GetPhysicalDeviceSurfaceCapabilitiesKHR(Ptr, Handle, *mut SurfaceCaps) -> VkResult;
    GetPhysicalDeviceSurfaceFormatsKHR(Ptr, Handle, *mut u32, *mut SurfaceFormat) -> VkResult;
    CreateSwapchainKHR(Ptr, *const SwapchainCreateInfo, *const c_void, *mut Handle) -> VkResult;
    GetSwapchainImagesKHR(Ptr, Handle, *mut u32, *mut Handle) -> VkResult;
    AcquireNextImageKHR(Ptr, Handle, u64, Handle, Handle, *mut u32) -> VkResult;
    QueuePresentKHR(Ptr, *const PresentInfo) -> VkResult;
});

const FORMAT_BGRA8: u32 = 44;
const FORMAT_D32: u32 = 126;
const FORMAT_RGBA32F: u32 = 109;
const LAYOUT_COLOR: u32 = 2;
const LAYOUT_DEPTH: u32 = 3;
const LAYOUT_TRANSFER_SRC: u32 = 6;
const LAYOUT_PRESENT: u32 = 1000001002;
const EXTERNAL: u32 = 0xFFFF_FFFF;
const SPV_VERT: &[u8] = include_bytes!("../../shaders/scene.vert.spv");
const SPV_FRAG: &[u8] = include_bytes!("../../shaders/scene.frag.spv");

macro_rules! chk {
    ($e:expr, $what:expr) => {{
        let r = $e;
        if r != 0 {
            return Err(format!("Vulkan: {} failed (VkResult {})", $what, r));
        }
    }};
}

enum Target {
    Window { surface: Handle, swapchain: Handle, framebuffers: Vec<Handle> },
    Offscreen { image: Handle, framebuffer: Handle, readback: Handle, readback_mem: Handle },
}

pub struct VulkanRenderer {
    f: Fns,
    sf: Option<SwapFns>,
    instance: Ptr,
    device: Ptr,
    queue: Ptr,
    mem: MemoryProperties,
    render_pass: Handle,
    layout: Handle,
    pipeline: Handle,
    cmd: Ptr,
    fence: Handle,
    sem_acquire: Handle,
    sem_render: Handle,
    vertex_buf: Handle,
    vertex_mem: Handle,
    vertex_capacity: u64,
    vertex_count: u32,
    dynamic_count: u32,
    vertex_mapped: *mut u8,
    target: Target,
    w: usize,
    h: usize,
    device_name: String,
}

impl VulkanRenderer {
    pub fn new(window: &dyn Window) -> Result<VulkanRenderer, String> {
        let (w, h) = window.size();
        Self::build(Some(window.native_handle()), w, h)
    }

    pub fn new_headless(w: usize, h: usize) -> Result<VulkanRenderer, String> {
        Self::build(None, w, h)
    }

    fn build(handle: Option<NativeHandle>, w: usize, h: usize) -> Result<VulkanRenderer, String> {
        unsafe {
            #[cfg(unix)]
            let lib = dl::open(&["libvulkan.so.1", "libvulkan.so"]);
            #[cfg(windows)]
            let lib = dl::open(&["vulkan-1.dll"]);
            if lib.is_null() {
                return Err("no Vulkan loader found (install your GPU driver / Vulkan runtime)".into());
            }
            let gipa_ptr = dl::sym(lib, "vkGetInstanceProcAddr");
            if gipa_ptr.is_null() {
                return Err("Vulkan loader has no vkGetInstanceProcAddr".into());
            }
            let gipa: Gipa = std::mem::transmute(gipa_ptr);
            type CreateInstance = unsafe extern "system" fn(*const InstanceCreateInfo, *const c_void, *mut Ptr) -> VkResult;
            let create_instance: CreateInstance = std::mem::transmute(gipa(std::ptr::null_mut(), b"vkCreateInstance\0".as_ptr() as *const c_char));

            let surface_ext: Option<&[u8]> = match handle {
                Some(NativeHandle::X11 { .. }) => Some(b"VK_KHR_xlib_surface\0"),
                Some(NativeHandle::Win32(_)) => Some(b"VK_KHR_win32_surface\0"),
                Some(_) => return Err("Vulkan has no surface for this window type".into()),
                None => None,
            };
            let ext_names: Vec<*const c_char> = match surface_ext {
                Some(s) => vec![b"VK_KHR_surface\0".as_ptr() as *const c_char, s.as_ptr() as *const c_char],
                None => vec![],
            };
            let app = ApplicationInfo {
                s_type: 0, p_next: std::ptr::null(), app_name: b"engine\0".as_ptr() as *const c_char, app_version: 1,
                engine_name: b"engine\0".as_ptr() as *const c_char, engine_version: 1, api_version: 1 << 22,
            };
            let ici = InstanceCreateInfo {
                s_type: 1, p_next: std::ptr::null(), flags: 0, app_info: &app, layer_count: 0, layers: std::ptr::null(),
                ext_count: ext_names.len() as u32, exts: ext_names.as_ptr(),
            };
            let mut instance: Ptr = std::ptr::null_mut();
            let r = create_instance(&ici, std::ptr::null(), &mut instance);
            if r == -9 || r == -6 {
                return Err("no Vulkan driver installed (VK_ERROR_INCOMPATIBLE_DRIVER)".into());
            }
            chk!(r, "vkCreateInstance");
            let f = Fns::load(gipa, instance)?;
            let sf = if handle.is_some() { Some(SwapFns::load(gipa, instance)?) } else { None };

            let mut surface: Handle = 0;
            match handle {
                Some(NativeHandle::X11 { display, window }) => {
                    type Create = unsafe extern "system" fn(Ptr, *const XlibSurfaceCreateInfo, *const c_void, *mut Handle) -> VkResult;
                    let p = gipa(instance, b"vkCreateXlibSurfaceKHR\0".as_ptr() as *const c_char);
                    if p.is_null() { return Err("vkCreateXlibSurfaceKHR missing".into()); }
                    let create: Create = std::mem::transmute(p);
                    let info = XlibSurfaceCreateInfo { s_type: 1000004000, p_next: std::ptr::null(), flags: 0, dpy: display, window: window as std::ffi::c_ulong };
                    chk!(create(instance, &info, std::ptr::null(), &mut surface), "vkCreateXlibSurfaceKHR");
                }
                #[cfg(windows)]
                Some(NativeHandle::Win32(hwnd)) => {
                    type Create = unsafe extern "system" fn(Ptr, *const Win32SurfaceCreateInfo, *const c_void, *mut Handle) -> VkResult;
                    #[link(name = "kernel32")]
                    extern "system" { fn GetModuleHandleW(n: *const u16) -> *mut c_void; }
                    let p = gipa(instance, b"vkCreateWin32SurfaceKHR\0".as_ptr() as *const c_char);
                    if p.is_null() { return Err("vkCreateWin32SurfaceKHR missing".into()); }
                    let create: Create = std::mem::transmute(p);
                    let info = Win32SurfaceCreateInfo { s_type: 1000009000, p_next: std::ptr::null(), flags: 0, hinstance: GetModuleHandleW(std::ptr::null()), hwnd };
                    chk!(create(instance, &info, std::ptr::null(), &mut surface), "vkCreateWin32SurfaceKHR");
                }
                _ => {}
            }

            let mut n = 0u32;
            chk!((f.EnumeratePhysicalDevices)(instance, &mut n, std::ptr::null_mut()), "vkEnumeratePhysicalDevices");
            if n == 0 {
                return Err("no Vulkan device found".into());
            }
            let mut devs = vec![std::ptr::null_mut(); n as usize];
            chk!((f.EnumeratePhysicalDevices)(instance, &mut n, devs.as_mut_ptr()), "vkEnumeratePhysicalDevices");
            let mut best: Option<(i32, Ptr, u32, String)> = None;
            for &d in &devs {
                let mut props = [0u8; 1024];
                (f.GetPhysicalDeviceProperties)(d, props.as_mut_ptr());
                let dtype = u32::from_ne_bytes(props[16..20].try_into().unwrap());
                let name = CStr::from_ptr(props[20..276].as_ptr() as *const c_char).to_string_lossy().into_owned();
                let mut qn = 0u32;
                (f.GetPhysicalDeviceQueueFamilyProperties)(d, &mut qn, std::ptr::null_mut());
                let mut qs = vec![QueueFamilyProperties { flags: 0, count: 0, ts_bits: 0, granularity: [0; 3] }; qn as usize];
                (f.GetPhysicalDeviceQueueFamilyProperties)(d, &mut qn, qs.as_mut_ptr());
                for (i, q) in qs.iter().enumerate() {
                    if q.flags & 1 == 0 {
                        continue;
                    }
                    if let Some(sf) = &sf {
                        let mut ok = 0u32;
                        (sf.GetPhysicalDeviceSurfaceSupportKHR)(d, i as u32, surface, &mut ok);
                        if ok == 0 {
                            continue;
                        }
                    }
                    let score = match dtype { 2 => 4, 1 => 3, 3 => 2, 4 => 1, _ => 0 };
                    if best.as_ref().map_or(true, |b| score > b.0) {
                        best = Some((score, d, i as u32, name.clone()));
                    }
                    break;
                }
            }
            let (_, phys, family, device_name) = best.ok_or("no Vulkan device can draw to this window")?;
            let mut mem = MemoryProperties { type_count: 0, types: [MemoryType::default(); 32], heap_count: 0, heaps: [MemoryHeap::default(); 16] };
            (f.GetPhysicalDeviceMemoryProperties)(phys, &mut mem);

            let prio = 1.0f32;
            let qci = DeviceQueueCreateInfo { s_type: 2, p_next: std::ptr::null(), flags: 0, family, count: 1, priorities: &prio };
            let dev_exts: Vec<*const c_char> = if sf.is_some() { vec![b"VK_KHR_swapchain\0".as_ptr() as *const c_char] } else { vec![] };
            let dci = DeviceCreateInfo {
                s_type: 3, p_next: std::ptr::null(), flags: 0, queue_count: 1, queues: &qci, layer_count: 0, layers: std::ptr::null(),
                ext_count: dev_exts.len() as u32, exts: dev_exts.as_ptr(), features: std::ptr::null(),
            };
            let mut device: Ptr = std::ptr::null_mut();
            chk!((f.CreateDevice)(phys, &dci, std::ptr::null(), &mut device), "vkCreateDevice");
            let mut queue: Ptr = std::ptr::null_mut();
            (f.GetDeviceQueue)(device, family, 0, &mut queue);

            let mut r = VulkanRenderer {
                f, sf, instance, device, queue, mem, render_pass: 0, layout: 0, pipeline: 0, cmd: std::ptr::null_mut(),
                fence: 0, sem_acquire: 0, sem_render: 0, vertex_buf: 0, vertex_mem: 0, vertex_capacity: 0, vertex_count: 0, dynamic_count: 0, vertex_mapped: std::ptr::null_mut(),
                target: Target::Offscreen { image: 0, framebuffer: 0, readback: 0, readback_mem: 0 }, w, h, device_name,
            };
            r.init(phys, surface, family)?;
            Ok(r)
        }
    }

    fn memory_type(&self, bits: u32, flags: u32) -> Result<u32, String> {
        (0..self.mem.type_count)
            .find(|&i| bits & (1 << i) != 0 && self.mem.types[i as usize].flags & flags == flags)
            .ok_or_else(|| "no suitable Vulkan memory type".to_string())
    }

    unsafe fn alloc_image(&self, format: u32, usage: u32, aspect: u32) -> Result<(Handle, Handle, Handle), String> {
        let ci = ImageCreateInfo {
            s_type: 14, p_next: std::ptr::null(), flags: 0, image_type: 1, format, extent: [self.w as u32, self.h as u32, 1],
            mips: 1, layers: 1, samples: 1, tiling: 0, usage, sharing: 0, qfi_count: 0, qfi: std::ptr::null(), initial_layout: 0,
        };
        let mut image = 0;
        chk!((self.f.CreateImage)(self.device, &ci, std::ptr::null(), &mut image), "vkCreateImage");
        let mut req = MemoryRequirements::default();
        (self.f.GetImageMemoryRequirements)(self.device, image, &mut req);
        let ai = MemoryAllocateInfo { s_type: 5, p_next: std::ptr::null(), size: req.size, type_index: self.memory_type(req.type_bits, 1)? };
        let mut mem = 0;
        chk!((self.f.AllocateMemory)(self.device, &ai, std::ptr::null(), &mut mem), "vkAllocateMemory");
        chk!((self.f.BindImageMemory)(self.device, image, mem, 0), "vkBindImageMemory");
        let view = self.make_view(image, format, aspect)?;
        Ok((image, mem, view))
    }

    unsafe fn make_view(&self, image: Handle, format: u32, aspect: u32) -> Result<Handle, String> {
        let vi = ImageViewCreateInfo {
            s_type: 15, p_next: std::ptr::null(), flags: 0, image, view_type: 1, format, components: [0; 4],
            range: SubresourceRange { aspect, base_mip: 0, mips: 1, base_layer: 0, layers: 1 },
        };
        let mut view = 0;
        chk!((self.f.CreateImageView)(self.device, &vi, std::ptr::null(), &mut view), "vkCreateImageView");
        Ok(view)
    }

    unsafe fn alloc_buffer(&self, size: u64, usage: u32) -> Result<(Handle, Handle), String> {
        let bi = BufferCreateInfo { s_type: 12, p_next: std::ptr::null(), flags: 0, size, usage, sharing: 0, qfi_count: 0, qfi: std::ptr::null() };
        let mut buf = 0;
        chk!((self.f.CreateBuffer)(self.device, &bi, std::ptr::null(), &mut buf), "vkCreateBuffer");
        let mut req = MemoryRequirements::default();
        (self.f.GetBufferMemoryRequirements)(self.device, buf, &mut req);
        let ai = MemoryAllocateInfo { s_type: 5, p_next: std::ptr::null(), size: req.size, type_index: self.memory_type(req.type_bits, 2 | 4)? };
        let mut mem = 0;
        chk!((self.f.AllocateMemory)(self.device, &ai, std::ptr::null(), &mut mem), "vkAllocateMemory");
        chk!((self.f.BindBufferMemory)(self.device, buf, mem, 0), "vkBindBufferMemory");
        Ok((buf, mem))
    }

    unsafe fn shader(&self, bytes: &[u8]) -> Result<Handle, String> {
        let words: Vec<u32> = bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        let ci = ShaderModuleCreateInfo { s_type: 16, p_next: std::ptr::null(), flags: 0, code_size: bytes.len(), code: words.as_ptr() };
        let mut m = 0;
        chk!((self.f.CreateShaderModule)(self.device, &ci, std::ptr::null(), &mut m), "vkCreateShaderModule");
        Ok(m)
    }

    unsafe fn init(&mut self, phys: Ptr, surface: Handle, family: u32) -> Result<(), String> {
        let windowed = self.sf.is_some();
        let mut sc_images: Vec<Handle> = Vec::new();
        let mut swapchain = 0;
        if let Some(sf) = &self.sf {
            let mut caps = SurfaceCaps::default();
            chk!((sf.GetPhysicalDeviceSurfaceCapabilitiesKHR)(phys, surface, &mut caps), "surface capabilities");
            let mut fnum = 0u32;
            (sf.GetPhysicalDeviceSurfaceFormatsKHR)(phys, surface, &mut fnum, std::ptr::null_mut());
            let mut fmts = vec![SurfaceFormat::default(); fnum as usize];
            (sf.GetPhysicalDeviceSurfaceFormatsKHR)(phys, surface, &mut fnum, fmts.as_mut_ptr());
            let ok = fmts.iter().any(|x| x.format == FORMAT_BGRA8 && x.color_space == 0) || (fmts.len() == 1 && fmts[0].format == 0);
            if !ok {
                return Err("surface does not support BGRA8 UNORM".into());
            }
            let extent = if caps.current[0] == 0xFFFF_FFFF { [self.w as u32, self.h as u32] } else { caps.current };
            let mut count = caps.min_images.max(2);
            if caps.max_images != 0 {
                count = count.min(caps.max_images);
            }
            let sci = SwapchainCreateInfo {
                s_type: 1000001000, p_next: std::ptr::null(), flags: 0, surface, min_images: count, format: FORMAT_BGRA8, color_space: 0,
                extent, layers: 1, usage: 0x10, sharing: 0, qfi_count: 0, qfi: std::ptr::null(), transform: caps.current_transform,
                alpha: 1, present_mode: 2, clipped: 1, old: 0,
            };
            chk!((sf.CreateSwapchainKHR)(self.device, &sci, std::ptr::null(), &mut swapchain), "vkCreateSwapchainKHR");
            let mut n = 0u32;
            (sf.GetSwapchainImagesKHR)(self.device, swapchain, &mut n, std::ptr::null_mut());
            sc_images = vec![0; n as usize];
            chk!((sf.GetSwapchainImagesKHR)(self.device, swapchain, &mut n, sc_images.as_mut_ptr()), "vkGetSwapchainImagesKHR");
        }

        let atts = [
            AttachmentDescription { flags: 0, format: FORMAT_BGRA8, samples: 1, load: 1, store: 0, sload: 2, sstore: 1, initial: 0,
                final_: if windowed { LAYOUT_PRESENT } else { LAYOUT_TRANSFER_SRC } },
            AttachmentDescription { flags: 0, format: FORMAT_D32, samples: 1, load: 1, store: 1, sload: 2, sstore: 1, initial: 0, final_: LAYOUT_DEPTH },
        ];
        let color_ref = AttachmentReference { attachment: 0, layout: LAYOUT_COLOR };
        let depth_ref = AttachmentReference { attachment: 1, layout: LAYOUT_DEPTH };
        let sub = SubpassDescription {
            flags: 0, bind_point: 0, input_count: 0, input: std::ptr::null(), color_count: 1, color: &color_ref, resolve: std::ptr::null(),
            depth: &depth_ref, preserve_count: 0, preserve: std::ptr::null(),
        };
        let deps = [
            SubpassDependency { src: EXTERNAL, dst: 0, src_stage: 0x400 | 0x100 | 0x200, dst_stage: 0x400 | 0x100 | 0x200,
                src_access: 0x400, dst_access: 0x100 | 0x400, flags: 0 },
            SubpassDependency { src: 0, dst: EXTERNAL, src_stage: 0x400, dst_stage: 0x1000, src_access: 0x100, dst_access: 0x800, flags: 0 },
        ];
        let rpi = RenderPassCreateInfo { s_type: 38, p_next: std::ptr::null(), flags: 0, att_count: 2, atts: atts.as_ptr(), sub_count: 1, subs: &sub, dep_count: 2, deps: deps.as_ptr() };
        chk!((self.f.CreateRenderPass)(self.device, &rpi, std::ptr::null(), &mut self.render_pass), "vkCreateRenderPass");

        let (_, _, depth_view) = self.alloc_image(FORMAT_D32, 0x20, 2)?;

        let make_fb = |this: &Self, color_view: Handle| -> Result<Handle, String> {
            let views = [color_view, depth_view];
            let fi = FramebufferCreateInfo { s_type: 37, p_next: std::ptr::null(), flags: 0, render_pass: this.render_pass, att_count: 2,
                atts: views.as_ptr(), width: this.w as u32, height: this.h as u32, layers: 1 };
            let mut fb = 0;
            chk!((this.f.CreateFramebuffer)(this.device, &fi, std::ptr::null(), &mut fb), "vkCreateFramebuffer");
            Ok(fb)
        };
        if windowed {
            let mut fbs = Vec::new();
            for &img in &sc_images {
                let v = self.make_view(img, FORMAT_BGRA8, 1)?;
                fbs.push(make_fb(self, v)?);
            }
            self.target = Target::Window { surface, swapchain, framebuffers: fbs };
        } else {
            let (image, _, view) = self.alloc_image(FORMAT_BGRA8, 0x10 | 0x01, 1)?;
            let framebuffer = make_fb(self, view)?;
            let (readback, readback_mem) = self.alloc_buffer((self.w * self.h * 4) as u64, 2)?;
            self.target = Target::Offscreen { image, framebuffer, readback, readback_mem };
        }

        let vs = self.shader(SPV_VERT)?;
        let fs = self.shader(SPV_FRAG)?;
        let stages = [
            StageInfo { s_type: 18, p_next: std::ptr::null(), flags: 0, stage: 1, module: vs, name: b"main\0".as_ptr() as *const c_char, spec: std::ptr::null() },
            StageInfo { s_type: 18, p_next: std::ptr::null(), flags: 0, stage: 0x10, module: fs, name: b"main\0".as_ptr() as *const c_char, spec: std::ptr::null() },
        ];
        let binding = VertexBinding { binding: 0, stride: std::mem::size_of::<Vertex>() as u32, rate: 0 };
        let attrs = [
            VertexAttr { location: 0, binding: 0, format: FORMAT_RGBA32F, offset: 0 },
            VertexAttr { location: 1, binding: 0, format: FORMAT_RGBA32F, offset: 16 },
            VertexAttr { location: 2, binding: 0, format: FORMAT_RGBA32F, offset: 32 },
        ];
        let vi = VertexInputState { s_type: 19, p_next: std::ptr::null(), flags: 0, bind_count: 1, binds: &binding, attr_count: 3, attrs: attrs.as_ptr() };
        let ia = InputAssemblyState { s_type: 20, p_next: std::ptr::null(), flags: 0, topology: 3, restart: 0 };
        let vp = Viewport { x: 0.0, y: 0.0, w: self.w as f32, h: self.h as f32, min_depth: 0.0, max_depth: 1.0 };
        let sc = Rect2D { x: 0, y: 0, w: self.w as u32, h: self.h as u32 };
        let vps = ViewportState { s_type: 22, p_next: std::ptr::null(), flags: 0, vp_count: 1, vps: &vp, sc_count: 1, scs: &sc };
        let rs = RasterState { s_type: 23, p_next: std::ptr::null(), flags: 0, clamp: 0, discard: 0, polygon: 0, cull: 0, front: 0, bias: 0, bias_c: 0.0, bias_clamp: 0.0, bias_slope: 0.0, line_width: 1.0 };
        let ms = MultisampleState { s_type: 24, p_next: std::ptr::null(), flags: 0, samples: 1, sample_shading: 0, min_sample: 0.0, mask: std::ptr::null(), a2c: 0, a21: 0 };
        let ds = DepthStencilState { s_type: 25, p_next: std::ptr::null(), flags: 0, test: 1, write: 1, compare: 1, bounds: 0, stencil: 0,
            front: StencilOp::default(), back: StencilOp::default(), min_depth: 0.0, max_depth: 1.0 };
        let ba = BlendAttachment { enable: 1, sc: 6, dc: 7, co: 0, sa: 1, da: 7, ao: 0, mask: 0xF };
        let bs = BlendState { s_type: 26, p_next: std::ptr::null(), flags: 0, logic_en: 0, logic: 0, att_count: 1, atts: &ba, consts: [0.0; 4] };
        let pc = PushRange { stages: 0x11, offset: 0, size: std::mem::size_of::<FrameUniforms>() as u32 };
        let li = PipelineLayoutCreateInfo { s_type: 30, p_next: std::ptr::null(), flags: 0, set_count: 0, sets: std::ptr::null(), pc_count: 1, pcs: &pc };
        chk!((self.f.CreatePipelineLayout)(self.device, &li, std::ptr::null(), &mut self.layout), "vkCreatePipelineLayout");
        let gpi = GraphicsPipelineCreateInfo {
            s_type: 28, p_next: std::ptr::null(), flags: 0, stage_count: 2, stages: stages.as_ptr(), vertex: &vi, assembly: &ia,
            tess: std::ptr::null(), viewport: &vps, raster: &rs, ms: &ms, ds: &ds, blend: &bs, dynamic: std::ptr::null(),
            layout: self.layout, render_pass: self.render_pass, subpass: 0, base: 0, base_index: -1,
        };
        chk!((self.f.CreateGraphicsPipelines)(self.device, 0, 1, &gpi, std::ptr::null(), &mut self.pipeline), "vkCreateGraphicsPipelines");

        let pi = CommandPoolCreateInfo { s_type: 39, p_next: std::ptr::null(), flags: 2, family };
        let mut pool = 0;
        chk!((self.f.CreateCommandPool)(self.device, &pi, std::ptr::null(), &mut pool), "vkCreateCommandPool");
        let ai = CommandBufferAllocateInfo { s_type: 40, p_next: std::ptr::null(), pool, level: 0, count: 1 };
        chk!((self.f.AllocateCommandBuffers)(self.device, &ai, &mut self.cmd), "vkAllocateCommandBuffers");
        let fi = SimpleCreateInfo { s_type: 8, p_next: std::ptr::null(), flags: 1 };
        chk!((self.f.CreateFence)(self.device, &fi, std::ptr::null(), &mut self.fence), "vkCreateFence");
        let si = SimpleCreateInfo { s_type: 9, p_next: std::ptr::null(), flags: 0 };
        chk!((self.f.CreateSemaphore)(self.device, &si, std::ptr::null(), &mut self.sem_acquire), "vkCreateSemaphore");
        chk!((self.f.CreateSemaphore)(self.device, &si, std::ptr::null(), &mut self.sem_render), "vkCreateSemaphore");
        Ok(())
    }

    unsafe fn record(&self, framebuffer: Handle, u: &FrameUniforms, readback: Option<(Handle, Handle)>) -> Result<(), String> {
        let f = &self.f;
        chk!((f.ResetCommandBuffer)(self.cmd, 0), "vkResetCommandBuffer");
        let bi = CommandBufferBeginInfo { s_type: 42, p_next: std::ptr::null(), flags: 1, inheritance: std::ptr::null() };
        chk!((f.BeginCommandBuffer)(self.cmd, &bi), "vkBeginCommandBuffer");
        let clears = [[u.sky[0], u.sky[1], u.sky[2], 1.0], [1.0, 0.0, 0.0, 0.0]];
        let rb = RenderPassBeginInfo {
            s_type: 43, p_next: std::ptr::null(), render_pass: self.render_pass, framebuffer,
            area: Rect2D { x: 0, y: 0, w: self.w as u32, h: self.h as u32 }, clear_count: 2, clears: clears.as_ptr(),
        };
        (f.CmdBeginRenderPass)(self.cmd, &rb, 0);
        if self.vertex_count + self.dynamic_count > 0 {
            (f.CmdBindPipeline)(self.cmd, 0, self.pipeline);
            let off = 0u64;
            (f.CmdBindVertexBuffers)(self.cmd, 0, 1, &self.vertex_buf, &off);
            (f.CmdPushConstants)(self.cmd, self.layout, 0x11, 0, std::mem::size_of::<FrameUniforms>() as u32, u as *const FrameUniforms as *const c_void);
            (f.CmdDraw)(self.cmd, self.vertex_count + self.dynamic_count, 1, 0, 0);
        }
        (f.CmdEndRenderPass)(self.cmd);
        if let (Some((image, buffer)), true) = (readback, true) {
            let region = BufferImageCopy {
                buffer_offset: 0, row_length: 0, image_height: 0, sub: SubresourceLayers { aspect: 1, mip: 0, base_layer: 0, layers: 1 },
                offset: [0, 0, 0], extent: [self.w as u32, self.h as u32, 1],
            };
            (f.CmdCopyImageToBuffer)(self.cmd, image, LAYOUT_TRANSFER_SRC, buffer, 1, &region);
        }
        chk!((f.EndCommandBuffer)(self.cmd), "vkEndCommandBuffer");
        Ok(())
    }
}

impl Renderer for VulkanRenderer {
    fn name(&self) -> &'static str {
        "vulkan"
    }

    fn upload_scene(&mut self, scene: &Scene) -> Result<(), String> {
        let verts = build_vertices(scene);
        let vertex_size = std::mem::size_of::<Vertex>();
        unsafe {
            chk!((self.f.DeviceWaitIdle)(self.device), "vkDeviceWaitIdle");
            self.vertex_count = verts.len() as u32;
            let bytes = ((verts.len() + MAX_DYNAMIC_VERTICES) * vertex_size) as u64;
            if bytes > self.vertex_capacity {
                let (buf, mem) = self.alloc_buffer(bytes, 0x80)?;
                self.vertex_buf = buf;
                self.vertex_mem = mem;
                self.vertex_capacity = bytes;
                let mut p: *mut c_void = std::ptr::null_mut();
                chk!((self.f.MapMemory)(self.device, self.vertex_mem, 0, bytes, 0, &mut p), "vkMapMemory");
                self.vertex_mapped = p as *mut u8;
            }
            std::ptr::copy_nonoverlapping(verts.as_ptr() as *const u8, self.vertex_mapped, verts.len() * vertex_size);
        }
        Ok(())
    }

    fn render(&mut self, scene: &Scene, cam: &Camera, _window: &mut dyn Window) -> Result<(), String> {
        let u = frame_uniforms(scene, cam, self.w as f32 / self.h as f32, true);
        unsafe {
            let (swapchain, framebuffers) = match &self.target {
                Target::Window { swapchain, framebuffers, .. } => (*swapchain, framebuffers.clone()),
                Target::Offscreen { .. } => return Err("headless Vulkan renderer has no window".into()),
            };
            chk!((self.f.WaitForFences)(self.device, 1, &self.fence, 1, u64::MAX), "vkWaitForFences");
            self.write_dynamic(scene);
            let sf = self.sf.as_ref().unwrap();
            let mut index = 0u32;
            let r = (sf.AcquireNextImageKHR)(self.device, swapchain, u64::MAX, self.sem_acquire, 0, &mut index);
            if r != 0 && r != 1000001003 {
                return Err(format!("Vulkan: vkAcquireNextImageKHR failed (VkResult {})", r));
            }
            chk!((self.f.ResetFences)(self.device, 1, &self.fence), "vkResetFences");
            self.record(framebuffers[index as usize], &u, None)?;
            let stage = 0x400u32;
            let si = SubmitInfo { s_type: 4, p_next: std::ptr::null(), wait_count: 1, waits: &self.sem_acquire, stages: &stage, cmd_count: 1,
                cmds: &self.cmd, signal_count: 1, signals: &self.sem_render };
            chk!((self.f.QueueSubmit)(self.queue, 1, &si, self.fence), "vkQueueSubmit");
            let pi = PresentInfo { s_type: 1000001001, p_next: std::ptr::null(), wait_count: 1, waits: &self.sem_render, sc_count: 1,
                scs: &swapchain, indices: &index, results: std::ptr::null_mut() };
            let r = (sf.QueuePresentKHR)(self.queue, &pi);
            if r != 0 && r != 1000001003 {
                return Err(format!("Vulkan: vkQueuePresentKHR failed (VkResult {})", r));
            }
        }
        Ok(())
    }

    fn render_offscreen(&mut self, scene: &Scene, cam: &Camera) -> Result<Framebuffer, String> {
        let u = frame_uniforms(scene, cam, self.w as f32 / self.h as f32, true);
        unsafe {
            let (image, framebuffer, readback, readback_mem) = match &self.target {
                Target::Offscreen { image, framebuffer, readback, readback_mem } => (*image, *framebuffer, *readback, *readback_mem),
                Target::Window { .. } => return Err("windowed Vulkan renderer cannot render offscreen".into()),
            };
            chk!((self.f.WaitForFences)(self.device, 1, &self.fence, 1, u64::MAX), "vkWaitForFences");
            self.write_dynamic(scene);
            chk!((self.f.ResetFences)(self.device, 1, &self.fence), "vkResetFences");
            self.record(framebuffer, &u, Some((image, readback)))?;
            let si = SubmitInfo { s_type: 4, p_next: std::ptr::null(), wait_count: 0, waits: std::ptr::null(), stages: std::ptr::null(), cmd_count: 1,
                cmds: &self.cmd, signal_count: 0, signals: std::ptr::null() };
            chk!((self.f.QueueSubmit)(self.queue, 1, &si, self.fence), "vkQueueSubmit");
            chk!((self.f.WaitForFences)(self.device, 1, &self.fence, 1, u64::MAX), "vkWaitForFences");
            let bytes = (self.w * self.h * 4) as u64;
            let mut p: *mut c_void = std::ptr::null_mut();
            chk!((self.f.MapMemory)(self.device, readback_mem, 0, bytes, 0, &mut p), "vkMapMemory");
            let src = std::slice::from_raw_parts(p as *const u8, bytes as usize);
            let mut fb = Framebuffer::new(self.w, self.h);
            for (px, b) in fb.pixels.iter_mut().zip(src.chunks_exact(4)) {
                *px = (b[2] as u32) << 16 | (b[1] as u32) << 8 | b[0] as u32;
            }
            (self.f.UnmapMemory)(self.device, readback_mem);
            Ok(fb)
        }
    }
}

impl VulkanRenderer {
    fn write_dynamic(&mut self, scene: &Scene) {
        let verts = build_dynamic_vertices(scene);
        let vertex_size = std::mem::size_of::<Vertex>();
        if self.vertex_mapped.is_null() {
            self.dynamic_count = 0;
            return;
        }
        self.dynamic_count = verts.len() as u32;
        unsafe {
            let destination = self.vertex_mapped.add(self.vertex_count as usize * vertex_size);
            std::ptr::copy_nonoverlapping(verts.as_ptr() as *const u8, destination, verts.len() * vertex_size);
        }
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            (self.f.DeviceWaitIdle)(self.device);
            let _ = (self.instance, &self.queue);
        }
    }
}
