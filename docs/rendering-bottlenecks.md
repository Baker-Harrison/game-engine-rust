# Why 3D Rendering Is Expensive, and How to Make It Cheaper

Status: written from established knowledge of the graphics literature and shipping engines (not freshly web-verified). Numbers are rough orders of magnitude. Verify specifics (API features, extension support) before committing to an implementation.

## 0. The one-paragraph mental model

A frame is a pipeline: the CPU decides *what* to draw and tells the GPU; the GPU transforms vertices, rasterizes triangles into pixels, shades pixels, and writes results to memory. Every stage has a throughput limit, and the slowest one sets frame time. Almost every "fundamental" improvement is one of four moves:

1. **Do less work** (don't process what isn't visible or isn't noticeable): culling, LOD, occlusion.
2. **Do the work once** (reuse across pixels, frames, or lights): caching, temporal reuse, baking, deferred shading.
3. **Do the work at lower resolution or rate** where the eye can't tell: upscaling, variable rate shading, foveation.
4. **Move the decision to where the data is** (GPU decides what to draw, not the CPU): GPU-driven rendering.

The rest of this document maps each bottleneck to the moves that attack it.

## 1. The bottlenecks

### 1.1 CPU submission and draw calls
Each draw call costs CPU time: validating state, binding resources, writing command buffers. Old APIs (GL, DX11) cost roughly 10-50 µs per draw; modern APIs (Vulkan, DX12, Metal, WebGPU) cut this but don't remove it. At 10,000 objects x several passes (depth, shadow cascades, main, transparency) you reach hundreds of thousands of calls per frame, which is far more than a single thread can encode in 16 ms. Scene traversal, culling, sorting and material lookup add CPU cost on top.

### 1.2 State changes
Switching pipelines, shaders, render targets, or bindings stalls or flushes parts of the GPU and costs CPU time to validate. Engines sort draws by pipeline/material to minimize changes, which conflicts with sorting by depth (needed for transparency and for early-Z efficiency).

### 1.3 Vertex and geometry throughput
Vertex shaders run per vertex, and with indexed meshes a post-transform cache helps only partly. Real costs: triangles smaller than a pixel (the rasterizer works in 2x2 pixel quads, so tiny triangles waste up to 75% of lanes and many fixed-function units are underfed), vertex attribute fetch bandwidth, and the classic fixed-function bottleneck of triangle setup/primitive assembly. Film-quality assets (millions of triangles per mesh) overwhelm this.

### 1.4 Overdraw and fill rate (ROP/pixel throughput)
Every pixel shaded that is later overwritten is wasted. Overdraw of 2-4x is typical in cluttered scenes; transparent particles and foliage can reach 10x+. Fill rate is also bounded by render-target resolution: 4K is 4x the pixels of 1080p, so cost scales with resolution, which is why upscaling is so effective.

### 1.5 Shading cost (ALU)
Pixel shader cost = pixels shaded x instructions per pixel. PBR with many lights, normal mapping, parallax, subsurface scattering, and complex materials can mean hundreds to thousands of instructions per pixel. Divergence (neighboring lanes taking different branches) and register pressure (which reduces occupancy and latency hiding) make the real cost worse than instruction counts suggest.

### 1.6 Memory bandwidth and latency
Often the true limiter, especially on mobile and integrated GPUs, where the GPU shares DRAM with the CPU. G-buffers of 4-6 render targets at 4K read and written multiple times per frame are enormous traffic. Texture sampling is bandwidth-bound; cache misses from poor locality or uncompressed textures hurt. Moving a byte from DRAM costs orders of magnitude more energy than an arithmetic op on it.

### 1.7 Lighting
Naive forward shading costs O(pixels x lights). Hundreds of dynamic lights are impossible without a data structure to limit which lights touch which pixel. Global illumination (indirect light) is the hardest: physically correct GI is a multi-bounce integral over the whole scene.

### 1.8 Shadows
Shadow maps re-render the scene from each light's point of view, so each shadowed light multiplies geometry cost. Cascaded shadow maps for the sun (3-4 cascades) can mean rendering the world 4 extra times. Resolution, aliasing, filtering (PCF/PCSS samples) all add cost. Ray-traced shadows trade this for BVH traversal cost.

### 1.9 Visibility determination
Deciding which objects/triangles are visible is itself work. Frustum culling is cheap; occlusion culling (is this object hidden behind others?) is hard because it depends on the rendered depth. It's a chicken-and-egg problem that drives two-pass algorithms.

### 1.10 Transparency
Order-dependent blending forbids simple depth-buffer sorting and early-Z, forces back-to-front sorting (CPU cost, and sort errors), and causes large overdraw. Order-independent transparency (OIT) has its own memory/bandwidth costs.

### 1.11 Anti-aliasing and temporal stability
MSAA multiplies bandwidth and memory. Supersampling multiplies shading. Cheaper post-process AA (FXAA, SMAA) is blurry; TAA is cheap and good but introduces ghosting and needs motion vectors.

### 1.12 Data movement CPU->GPU and asset streaming
Uploading buffers/textures, streaming megatextures or huge meshes, and shader compilation hitches ("stutter") are frame-time bottlenecks that don't show in steady-state GPU time. PSO (pipeline state object) compilation on first use is a notorious source of hitches.

### 1.13 Hardware-specific: tile-based GPUs
Mobile GPUs (and Apple Silicon) shade in on-chip tiles. Bandwidth to DRAM is minimized if you keep render targets in tile memory; breaking render passes forces a store/load. Algorithms that are fine on desktop (large G-buffer, many passes) can be disastrous here.

## 2. Techniques that reduce cost, by bottleneck

### 2.1 Culling (don't draw what can't be seen)
- **Frustum culling**: test bounding volumes (AABB/sphere) against 6 planes. Do it in SIMD on CPU or in a compute shader. Cost is tiny vs. savings.
- **Backface / cone culling**: per-meshlet normal cones discard clusters facing away.
- **Occlusion culling**: *Hierarchical Z-buffer (HZB)* occlusion: build a depth mip pyramid from last frame's (or a depth prepass's) depth, test object/cluster bounds against it in a compute shader. The standard two-pass approach: (1) draw what was visible last frame, rebuild HZB, (2) test everything else against the new HZB and draw the newly visible. This is exact-ish, temporally coherent, and used by Nanite, Unreal 5, Assassin's Creed Unity and many others.
- **Portals / PVS / BSP**: precomputed visibility for static indoor levels. Cheap at runtime, but needs a bake step and doesn't handle dynamic scenes.
- **Software occlusion rasterization** (e.g. Intel's masked occlusion culling): tiny CPU depth rasterizer for occluders, feeding CPU culling when GPU readback latency is unacceptable.
- **Contribution culling**: skip objects whose screen footprint is below a threshold.

Why it's fundamental: visible geometry is O(pixels), not O(scene size). Good visibility gets you from "cost scales with the world" to "cost scales with the screen".

### 2.2 Level of detail (LOD)
- **Discrete LODs**: pre-simplified meshes (meshoptimizer/quadric edge collapse); simple, popping at transitions (mask with dithered cross-fade).
- **Continuous / hierarchical LOD (HLOD)**: merge distant groups of objects into single proxy meshes, which also cuts draw calls.
- **Impostors/billboards**: replace distant complex objects (trees) with textured quads or octahedral impostors.
- **Material / shader LOD**: cheaper shaders at distance; drop normal maps, fewer light taps.
- **Texture LOD**: mipmapping is the original LOD and is universal; sampler feedback / virtual texturing streams only the mips and tiles needed.

### 2.3 Virtualized geometry (Nanite-style)
The aim: triangle count that is *independent of scene complexity* and bounded by screen pixels, with roughly one triangle per pixel. Core ideas:
1. **Meshlet clusters**: split every mesh into ~128-triangle clusters.
2. **Cluster hierarchy (DAG)**: group clusters, simplify the group, split again, building a hierarchy where parent clusters are coarser. Locked shared boundaries between neighboring clusters at different LODs avoid cracks.
3. **Per-cluster LOD selection on the GPU**: choose, per cluster, the cut through the DAG whose projected error is under a pixel. No popping because error is sub-pixel.
4. **GPU culling** (frustum + HZB occlusion) at cluster granularity.
5. **Hybrid rasterizer**: large triangles go through the hardware rasterizer; tiny (sub-pixel to ~few pixels) triangles are rasterized in a *software compute rasterizer* because the HW rasterizer's quad overhead is inefficient there.
6. **Visibility buffer** output (see 2.6) so material shading happens once per pixel.
7. **Streaming**: cluster pages streamed from disk on demand, with compressed, quantized geometry.

Costs: heavy offline preprocessing, complexity, a requirement for mesh shaders or compute-based rasterization, and weak support for deforming/skinned meshes, foliage with alpha test and so on (though later UE5 versions extend it). Open implementations to study: `meshoptimizer` (clusterization, simplification, `meshopt_buildMeshlets`), Arseny Kapoulkine's niagara renderer, Bevy's virtual geometry (`bevy_pbr` meshlet feature, experimental), `nanite-rs`-style hobby work.

### 2.4 Mesh/task shaders and GPU-driven rendering
- **Mesh shaders** (task + mesh stages) replace the fixed-function input assembler + vertex stages with compute-like workgroups that emit small meshlets. Benefits: per-meshlet culling before rasterization, no index-buffer vertex-cache dependency, amplification (task shader decides how many mesh groups to launch). Availability: DX12 Ultimate, Vulkan `VK_EXT_mesh_shader`, Metal 3; **not available in WebGPU/wgpu yet** (verify current status), so a wgpu-based engine would emulate with compute culling + indirect draws.
- **GPU-driven rendering**: put scene data (transforms, bounds, material IDs) in GPU buffers, run culling and LOD in compute, write draw arguments to a buffer, and issue **indirect draws** (`multi-draw-indirect`, `ExecuteIndirect`, or `draw_indexed_indirect_count`). The CPU issues O(1) calls per pass regardless of object count. Combined with **bindless** resources (descriptor indexing / texture arrays), draws no longer need per-draw binding, collapsing state changes and draw calls together.
- **Instancing**: the simple version of the same idea; one call, many copies.
- **Persistent buffers, ring allocators, and multithreaded command encoding** address CPU-side costs where GPU-driven isn't possible.

Why it's fundamental: it removes the CPU->GPU round trip from visibility decisions, so draw-call count stops being the bottleneck.

### 2.5 Shading architectures: forward, deferred, clustered, visibility buffer
| Architecture | Idea | Pros | Cons |
|---|---|---|---|
| Forward | Shade each fragment as it is rasterized, looping over lights | Cheap bandwidth, MSAA works, transparency natural | Overdraw shades pixels repeatedly; light count limited |
| Deferred | Rasterize attributes to a G-buffer, shade once per pixel in a later pass | Shading cost decoupled from geometry complexity and overdraw; many lights | Big bandwidth, no MSAA, transparency needs a separate path |
| Tiled deferred/forward+ | Divide screen into tiles (16x16), compute per-tile light lists in compute | Light cost ~ lights touching a tile | Depth discontinuities inside a tile give false positives |
| Clustered | 3D froxel grid (tiles x depth slices) with per-cluster light lists | Handles depth discontinuities, fast for thousands of lights; works with forward | Light list build cost, memory |
| Visibility buffer | Raster pass writes only (instance ID, triangle ID) in one 32-64 bit target; a later compute pass reconstructs attributes, does material/texture lookups and shading | Minimal raster bandwidth, perfect shading coherence, shades each pixel once, great for tiny triangles and many materials | Needs bindless + GPU attribute fetch, derivatives by hand, harder transparency, MSAA complicated |

The trend in high-end engines: **visibility buffer + clustered lighting + GPU-driven culling**, because together they decouple three formerly multiplied costs (geometry, materials, lights). Forward+/clustered forward remains the pragmatic choice for mobile and for engines that need MSAA/XR.

### 2.6 Reducing overdraw
- **Depth pre-pass / Z-prepass** (+ early-Z): render depth first with a cheap shader, then shade only visible fragments. Cost: geometry twice. Often worth it for expensive shaders.
- **Front-to-back sorting of opaque** to maximize early-Z rejection; tile-based GPUs do hidden surface removal in hardware.
- **Visibility buffer / deferred** (exactly one shade per pixel).
- **Meshlet-level occlusion culling** to avoid raster of hidden clusters at all.
- **Tight transparent geometry** (cut particle quads to the visible shape), half- or quarter-resolution particle rendering, soft-particle tricks, alpha-to-coverage instead of blending for foliage.

### 2.7 Lighting and shadows
- **Lights**: clustered/tiled culling (above); limit light radius; baked light for static contributions; light probes and irradiance volumes for dynamic objects.
- **Shadow maps**: tighter cascade fitting, **virtual shadow maps** (high-resolution page-based shadow maps allocated only where visible pixels need them, UE5), shadow-map caching (re-render only pages that contain moving objects or changed lights), contact-hardening filters, static/dynamic shadow split.
- **Ray-traced shadows** (hardware RT): correct, no resolution problem, but BVH build/refit and traversal costs; typically used one ray per pixel plus denoising.
- **Global illumination approximations**: baked lightmaps (cheapest, static only); light probes (SH/ambient cube); **voxel cone tracing**; **screen-space GI/AO** (SSAO/GTAO, SSGI: cheap but only sees what's on screen); **radiance cascades** (a recent, promising 2D/3D technique with cost bounded by screen/probe resolution, used in Path of Exile 2 and getting research attention); **DDGI** (probe grid with ray-traced updates, amortized over time); **Lumen** (screen-space tracing + SDF tracing + radiance caching, with software and hardware RT paths); **ReSTIR** (reservoir resampling, which reuses light samples spatially and temporally, enabling many-light real-time path tracing).

### 2.8 Temporal reuse, upscaling and frame generation
The strongest practical cost reducer of the last decade: the information in the previous frame is nearly the same as in this one.
- **TAA**: jitter the camera sub-pixel each frame, reproject history using motion vectors, accumulate. Gives AA nearly free and the foundation for everything below.
- **Temporal upscaling**: render at e.g. 50-67% scale per axis (25-45% of the pixels), reconstruct to the output resolution using history. Examples: FSR 2/3 (open, Rust-bindable via C API), **DLSS** (ML, NVIDIA-only), **XeSS** (Intel), **TSR** (UE5), and Apple MetalFX. This is why 4K is feasible today.
- **Frame generation**: interpolate or extrapolate whole frames using optical flow + motion vectors (DLSS FG, FSR 3 FG). Doubles displayed frame rate but not input latency; needs a latency-reduction scheme.
- **Temporal denoising** for ray-traced effects: 1 sample per pixel per frame becomes decent after accumulation (SVGF, ReBLUR/NRD, ML denoisers).
- **Amortization**: update expensive things (probe GI, shadow pages, cloud rendering, SSAO) over several frames in a checkerboard or round-robin pattern.
- **Engine requirement**: clean motion vectors, jitter support in the projection matrix, per-object velocity, exposure/alpha handling. Designing for this from the start is much cheaper than retrofitting.

### 2.9 Variable rate shading (VRS) and foveation
Shade at one invocation per 2x2 or 4x4 pixels where the eye won't notice (flat areas, motion blur regions, peripheral vision, UI-obscured areas). Hardware VRS (DX12 Tier 2, Vulkan `VK_KHR_fragment_shading_rate`) works per-draw, per-primitive or via a screen-space rate image. In VR, **foveated rendering** (with eye tracking, dynamic) cuts shaded pixels 2-4x. A related software approach: **decoupled shading / texture-space shading** — shade in texture or object space at a rate independent of screen resolution, cache and reuse across frames and across both eyes.

### 2.10 Caching and baking
- Baked lightmaps / probes / reflection probes for static content.
- Texture-space or **object-space shading caches** (reuse shading across frames; used in some mobile and VR engines).
- **Virtual texturing / sparse resources** to keep only needed texture tiles resident; reduces memory footprint and bandwidth.
- **Texture compression** (BCn, ASTC, Basis Universal/KTX2) cuts bandwidth by 4-8x and is basically free; hardware decompresses in the texture unit.
- **Vertex compression**: quantized positions, octahedral normals, packed tangents, meshlet-local indices.
- **Pipeline cache / shader precompilation** to remove first-use stutter.

### 2.11 Cheaper transparency
Weighted blended OIT (single pass, approximate), depth peeling (exact, expensive), per-pixel linked lists (exact, memory heavy), alpha-to-coverage + dithering + TAA ("stochastic transparency"), and render particles at lower resolution then composite.

### 2.12 Bandwidth strategies
Pack G-buffers tightly (octahedral normals, 8-10 bit formats where OK), render-pass merging on tile GPUs (subpass inputs / memoryless attachments), compute-based post-processing with LDS/shared memory, avoid full-res passes, use half precision (FP16) where precision permits, and prefer arithmetic over memory when ALU is cheap and bandwidth is expensive.

## 3. Speculative and "fundamental" directions

These attack the structure of the problem rather than tuning it. Maturity varies; treat them as research bets.

1. **Neural rendering.** Replace parts of the pipeline with learned models: neural upscalers and denoisers (shipping), **neural radiance caching** (NVIDIA, learning indirect light online; shipping in some titles), **neural texture compression** (BCn alternatives with several-times-higher ratios), **neural materials** (a net replaces a layered BRDF evaluation). Cost shifts from shader ALU to tensor/matrix units. Needs vendor-specific or cooperative-matrix hardware paths, so portability is poor today.
2. **3D Gaussian splatting / point-based and radiance-field rendering.** Scenes represented as millions of anisotropic Gaussians rendered by sorting and alpha-blending splats; real-time, photoreal for captured scenes. Weak at dynamics, relighting and editing; active research (relightable, animated, compressed variants). Could be a niche rendering mode for captured environments.
3. **Fully ray/path-traced pipelines** with ReSTIR + neural denoising and neural radiance caching: unifies shadows, reflections, GI and transparency into one mechanism, at the cost of BVH build/refit and ~1-2 samples/pixel budgets. Becomes viable as RT hardware scales. A BVH is also a visibility structure: the same data answers occlusion queries and GI.
4. **SDF-based and implicit scene representations.** Signed distance fields give cheap soft shadows, AO, and large-scale GI tracing (Lumen uses mesh and global SDFs). Compact and easily blended, but expensive to update for dynamic geometry.
5. **Voxel / sparse-voxel rendering (SVO, brick maps)**: geometry, lighting and LOD all in one hierarchical structure; the ray cost is logarithmic in scene size. Studied for decades; limited by memory and animation.
6. **Radiance cascades / probe hierarchies**: GI whose cost is bounded by a fixed probe budget rather than scene complexity, with penumbra-correct soft shadows nearly free. A rare algorithmic win of recent years; worth prototyping.
7. **Decoupled / object-space / texture-space shading** (see 2.9) taken to the extreme: shade surfaces at the rate they change (e.g. diffuse lighting at low rate, specular at screen rate), sharing results across frames and across views (stereo, split-screen, shadows).
8. **Foveated and gaze-contingent rendering**, with eye-tracking in VR/AR and perceptual models: spend samples where the human visual system resolves detail. A few-fold savings in the limit.
9. **Perceptually driven adaptive quality**: use contrast sensitivity, saccadic masking, and motion to modulate shading rate, LOD and temporal accumulation.
10. **Cloud/remote rendering and streaming**: move cost off the client; shifts the bottleneck to bandwidth and latency. Not an engine-level concern unless targeted.
11. **Work graphs / GPU-resident scheduling** (DX12 work graphs, Vulkan device-generated commands, `VK_EXT_device_generated_commands`): the GPU launches its own work, so entire frames (culling -> LOD -> raster -> shade -> post) can run with minimal CPU involvement. This is the logical end-point of GPU-driven rendering.
12. **Procedural and generative assets at runtime**: generate detail from compact descriptions or neural decoders, trading compute for storage and streaming bandwidth.
13. **Differentiable / learned LOD and impostors** (neural impostors, learned mesh simplification) tuned for perceptual error instead of geometric error.

## 4. Mapping bottlenecks to fixes (cheat sheet)

| Bottleneck | Cheapest fix | Fundamental fix |
|---|---|---|
| Draw calls / CPU submit | Instancing, batching, multithreaded encoding | GPU-driven rendering + indirect draws + bindless |
| State changes | Sort by pipeline/material, fewer permutations | Bindless + uber-shaders / visibility buffer material pass |
| Vertex / geometry throughput | Mesh optimization (vertex cache, quantization), discrete LOD | Meshlets + cluster LOD DAG + software rasterization of small triangles |
| Overdraw | Front-to-back sort, Z-prepass | HZB occlusion culling, visibility buffer |
| Pixel shading ALU | Cheaper materials, LOD shaders, FP16 | Deferred / visibility buffer, VRS, temporal upscaling, shading caches |
| Memory bandwidth | Texture + vertex compression, packed G-buffer | Visibility buffer, tile-memory passes, virtual texturing, neural compression |
| Many lights | Tiled / clustered lists | ReSTIR, light BVH, importance sampling |
| Shadows | Cascades, caching, culling per light | Virtual shadow maps, hardware RT shadows + denoise |
| Global illumination | Baked + probes + SSAO | Radiance cascades, DDGI, Lumen-style cache, neural radiance cache |
| Visibility | Frustum culling, portals | HZB two-pass GPU occlusion, cluster-level culling |
| Transparency | Alpha-to-coverage, weighted OIT | Stochastic transparency + TAA, ray-traced hybrid |
| Resolution cost | Dynamic resolution | Temporal upscaling (FSR/DLSS/XeSS), frame generation |
| Stutter | PSO caches, async compile | Precompiled pipeline libraries, ubershader fallbacks |

## 5. What this means for the Rust engine

Constraints first: the project is assumed to use `wgpu` (see `docs/research.md`). wgpu is a portable subset: **no mesh shaders** yet, limited bindless (`binding_array`/texture arrays are feature-gated and uneven across backends), `multi_draw_indirect` and `multi_draw_indirect_count` are native-only features, and no direct work graphs. Verify each against the wgpu version you pin. These limits push the design toward compute-driven culling + indirect draws, which is also the portable GPU-driven design.

### Recommended adoption order (highest payoff per complexity first)

**Phase A: Baseline that doesn't paint you into a corner**
1. Render graph / frame graph with explicit resource lifetimes (already planned). It enables pass merging, transient aliasing, and later async compute.
2. Data layout designed for GPU-driven rendering from day one: a persistent GPU scene buffer (instance transforms, bounds, mesh and material IDs), mesh data in large shared vertex/index buffers (suballocated), materials as indexed records. Retrofitting this is the most expensive mistake available.
3. CPU frustum culling + instancing + sorted draw lists, multithreaded via the ECS extract/queue model. Good enough for the early engine.
4. Texture compression (KTX2/Basis, BCn/ASTC) in the asset pipeline from the beginning; vertex quantization and `meshoptimizer` (Rust bindings exist: `meshopt`) for vertex cache / overdraw optimization.
5. Clean jitter + motion-vector support in the camera and material paths, even if TAA ships later.
6. Profiling: GPU timestamp queries per pass (wgpu supports them), `tracy`, and RenderDoc/PIX captures. Measure before choosing: the bottleneck differs per scene and per GPU.

**Phase B: Make cost scale with the screen, not the scene**
1. GPU culling in compute (frustum, then HZB occlusion with the two-pass scheme) writing indirect draw args; `multi_draw_indirect` where available, fallback loops otherwise.
2. Clustered forward lighting (works with MSAA/transparency and tile GPUs) or a deferred path; decide per target. Clustered forward is the best default for a portable wgpu engine.
3. Cascaded shadow maps with caching and tight fitting; add point/spot shadow atlases with culling.
4. TAA, then temporal upscaling (integrate FSR via its C library, or write a simple one first), dynamic resolution.
5. Discrete LOD + HLOD pipeline, offline-generated with `meshopt` simplification.

**Phase C: Virtualized geometry and visibility buffer**
1. Meshlet generation in the asset importer (`meshopt` meshlets), meshlet culling in compute, then cluster LOD DAG with error metrics and boundary locking. Bevy's experimental meshlet renderer is the closest Rust reference.
2. Visibility buffer path: ID raster pass + compute material evaluation. Needs bindless textures; check wgpu support, otherwise use a texture atlas or array fallback.
3. Software rasterization of small triangles in compute (using 64-bit atomics where available; check support in wgpu) for the sub-pixel regime.
4. Virtual shadow maps, caching, and virtual texturing as follow-ons.

**Phase D: Global illumination and speculative**
1. Start cheap: baked lightmaps/probes + GTAO + SSR.
2. Prototype radiance cascades or DDGI-style probes as a dynamic GI option. Radiance cascades are comparatively easy to implement in compute and don't require hardware RT.
3. Treat hardware RT, neural methods and Gaussian splatting as optional experimental backends behind feature flags, not core architecture. They are vendor- and API-dependent, and wgpu's ray-tracing support is experimental.
4. Keep a native-Vulkan/DX12 backend escape hatch in mind (mesh shaders, work graphs, device-generated commands) if wgpu becomes the limit. Abstract at the render-graph level, not at the wgpu call level, so this stays possible.

### Rust-specific notes
- Rust helps here mostly on the CPU side: data-oriented ECS iteration for culling/extraction, safe multithreaded command encoding (`rayon`, scoped threads), and `bytemuck`/`encase` for GPU-layout-safe structs. Use `#[repr(C)]` + `bytemuck::Pod` for every GPU struct and test layouts.
- Write shaders in WGSL (native to wgpu) or use `rust-gpu` (shared Rust code between CPU and GPU, but a moving target) / `slang` (rich, increasingly supported, and compiles to SPIR-V/WGSL). Decide early; shader language choice is a long-lived commitment.
- Hot reload of shaders and automatic pipeline cache (`wgpu::PipelineCache` where supported) improve iteration and reduce stutter.

### Decisions to make now
1. **Target hardware**: desktop-only vs. mobile/web. This decides forward-clustered vs. visibility-buffer priority and whether tile-memory-friendly passes matter.
2. **Static vs. dynamic worlds**: heavy baking (cheap runtime) vs. fully dynamic (GI and shadow cost).
3. **How much to depend on wgpu**: portability and safety vs. access to mesh shaders and bindless.
4. **Material model**: a small set of parameterized uber-materials fits the visibility-buffer and GPU-driven paths far better than arbitrary per-material shader graphs.

## 6. Reading list
- Haar & Aaltonen, *GPU-Driven Rendering Pipelines* (SIGGRAPH 2015, Assassin's Creed Unity).
- Karis et al., *A Deep Dive into Nanite Virtualized Geometry* (SIGGRAPH 2021), and Brian Karis's earlier *High-Quality Temporal Supersampling*.
- Burns & Hunt, *The Visibility Buffer* (JCGT 2013); Schied & Dachsbacher, *Deferred Attribute Interpolation Shading*.
- Olsson, Billeter, Assarsson, *Clustered Deferred and Forward Shading* (HPG 2012).
- Bitterli et al., *Spatiotemporal Reservoir Resampling (ReSTIR)* (SIGGRAPH 2020).
- Sannikov, *Radiance Cascades* (2023) and community write-ups.
- Majercik et al., *Dynamically Diffuse Global Illumination (DDGI)*.
- Kerbl et al., *3D Gaussian Splatting for Real-Time Radiance Field Rendering* (2023).
- Arseny Kapoulkine's *meshoptimizer* and the *niagara* Vulkan renderer streams.
- Wihlidal, *Optimizing the Graphics Pipeline with Compute* (GDC 2016).
- Real-Time Rendering, 4th ed. (Akenine-Möller, Haines, Hoffman), the general reference.
- Bevy's `bevy_pbr` meshlet and render-graph code as a Rust implementation reference.
