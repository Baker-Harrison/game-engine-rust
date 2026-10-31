# Building a Game Engine in Rust: Research and Roadmap

Status: the repo was empty when this was written. Crate versions below are approximate; verify on crates.io before pinning. Bevy has reached at least 0.19 and Fyrox has reached 1.0 (per web search), so this ecosystem moves fast.

## 1. Guiding decision: what kind of engine?

Pick scope before architecture. Recommendation: a **3D-capable, data-oriented, code-first engine** (like Bevy, not Unity/Godot-style editor-first) with an editor added late. "Full engine from scratch" should mean: you own the architecture (ECS, render graph, asset pipeline, scheduling) but buy commodity layers (windowing, GPU abstraction, math, audio decoding, physics) at first and replace them only where it pays off.

## 2. Subsystems, crates, build vs buy

| Subsystem | Buy (recommended) | Build | Notes |
|---|---|---|---|
| Windowing / input | `winit` (+ `raw-window-handle`), `gilrs` for gamepads | Input action mapping layer | winit is the de facto standard; wrap it behind your own `Platform` trait. |
| GPU / rendering API | `wgpu` (Vulkan/Metal/DX12/GL/WebGPU/wasm) | Renderer on top | Do not write raw Vulkan unless you need it. `naga` handles shader translation; WGSL is the native shader language. |
| Renderer | Build | Render graph, frame graph, materials, PBR, shadows, culling, post-processing, glTF scenes | The biggest chunk of work. Reference: Bevy `bevy_render`, Fyrox renderer, `rend3`. |
| Math | `glam` (fast, SIMD), or `nalgebra` if physics integration needs it | none | Use `glam` for the engine; `rapier` uses `glam` or nalgebra depending on features. |
| ECS | Build, or `hecs`/`bevy_ecs`/`flecs-rs` as a stopgap | Archetype or sparse-set ECS, query API, scheduler with parallel system execution, change detection, events, commands | Building one is a well-trodden learning path. `bevy_ecs` is usable standalone if you would rather skip it. |
| Parallelism | `rayon`, or a custom job system | Task graph for systems | Keep the scheduler deterministic where possible. |
| Audio | `cpal` (device), `rodio` or `kira` (mixing/playback), `symphonia` (decoding), `fmod`/`wwise` bindings if commercial middleware is wanted | Spatial audio, mixer graph | `kira` suits game audio best (tweens, clocks, buses). |
| Assets | `gltf`, `image`, `ddsfile`/`basis-universal` (texture compression), `fbxcel` if FBX is needed, `ttf-parser`/`fontdue`/`cosmic-text` for text | Asset server: handles, async load, hot reload, dependency graph, import pipeline, processed cache | Use `notify` for file watching. Design handles and the import/processed split early; it is hard to retrofit. |
| Serialization | `serde` + `ron`/`bincode`/`postcard` | Reflection layer | Reflection (runtime type info) is needed by editor and scripting. Bevy's `bevy_reflect` is the reference; `facet` is a newer alternative. |
| Physics | `rapier` (2D/3D, from Dimforge) | Own only if a very specific need arises | Integrate as an ECS plugin. Writing a good physics engine is a multi-year project. |
| Scripting | Native Rust plugins first; later `mlua`/`rhai`/`wasmtime` | Script API binding layer | Rust hot reload is awkward (`dylib` + `libloading`, or `hot-lib-reloader`). Wasm scripting gives sandboxing and portability. |
| UI (in-game) | `egui` (debug/tools), `taffy` (layout), `cosmic-text` | Game UI on top of `taffy` | `egui` is the fastest route to editor and debug panels. |
| Editor | `egui` + `egui_dock` + `egui_gizmo`/`transform-gizmo`, or a retained UI | Scene hierarchy, inspector (via reflection), asset browser, play-in-editor | Build after the runtime is stable; the editor is a client of the engine, not part of its core. |
| Networking | `quinn` (QUIC), `renet`, `lightyear` | Replication layer | Defer until the core is done. |
| Profiling / debug | `tracing`, `tracy-client`, `puffin`, `renderdoc` | Debug draw | Add profiling markers from day one. |
| Build / tooling | `cargo`, `cargo-nextest`, `cargo-deny`, `cargo-xtask` | CI for Windows/Linux/macOS/wasm | Use a workspace of small crates for compile times. |

## 3. Reference engines and lessons

- **Bevy** (ECS-first, modular, code-first). Lessons: plugins as the unit of composition; ECS scheduler with explicit system ordering; render graph with extract/prepare/queue/render stages and a separate render world; reflection; asset server with processed assets. Costs to avoid: very long compile times, API churn every 3-4 months, and complex generic type signatures.
- **Fyrox** (scene-graph, integrated editor, 1.0 released). Lessons: an integrated editor early gives users a reason to adopt; a scene-graph and editor-first design is easier for non-programmers but couples runtime to editor.
- **Macroquad / ggez / Comfy** (simple 2D). Lesson: low-friction APIs drive adoption; useful as a minimal baseline.
- **Godot (Rust bindings via gdext), Unreal, Unity**: for editor UX and asset pipeline concepts, not code.
- **Others to read:** `rend3`, `Piston` (cautionary: modularity without cohesion), `Stardust`/`Veloren` (voxel/large game on Rust), `Our Machinery` talks (data-oriented engine design), Jason Gregory's *Game Engine Architecture*, Hazel/TheCherno series for the C++ equivalent.

## 4. Suggested architecture

Cargo workspace of crates with a strict dependency direction (lower layers do not know about upper ones):

```
engine_core      (math re-exports, time, logging, config, handles, reflection)
engine_ecs       (world, queries, scheduler, events, commands)
engine_platform  (winit window, input, filesystem, clock)
engine_assets    (asset server, loaders, hot reload, importers)
engine_render    (wgpu device, render graph, materials, meshes, camera, lights, PBR)
engine_audio     (kira/cpal wrapper)
engine_physics   (rapier integration)
engine_scene     (transform hierarchy, serialization of worlds, prefabs)
engine_ui        (taffy-based UI)
engine_script    (plugin/wasm/lua host)
engine_app       (App builder, Plugin trait, main loop)
engine_editor    (egui-based editor; binary)
examples/        (each phase ends with a runnable example)
```

Key design choices:
1. **Plugin-based `App`**: every subsystem is a `Plugin` registering resources and systems. Keeps the core small and testable.
2. **Fixed-timestep simulation + variable render**: separate `FixedUpdate` (physics, gameplay) from `Update`/render, with interpolation.
3. **Render world separate from game world** (extract phase) so rendering can pipeline with the next simulation frame.
4. **Render graph** from the start (even a tiny one); passes declare their inputs and outputs.
5. **Stable handles + generational IDs** for assets and entities.
6. **Data-driven scenes**: serialize the world through reflection; same format used by the editor.
7. **Platform targets**: desktop first; keep wasm/WebGPU possible by avoiding blocking APIs and threads in core paths.
8. **Safety**: stay in safe Rust; confine `unsafe` to the ECS storage and GPU interop, with tests under `miri` where feasible.

## 5. Phased roadmap

Each phase ends with a demo and tests.

0. **Foundations (1-2 weeks):** workspace, CI (fmt, clippy, tests, multi-OS), logging/tracing, `App` and `Plugin`, main loop with timing.
1. **Window + GPU hello triangle (1-2 weeks):** winit window, wgpu surface, resize handling, clear color, triangle, input events.
2. **ECS (3-6 weeks):** entities, components, archetype/sparse storage, queries, schedule with parallelism, events, resources, commands. Benchmarks with `criterion`.
3. **Renderer v1 (6-10 weeks):** camera, transform hierarchy, meshes, textures, depth, unlit then Blinn-Phong, glTF loading, render graph, frustum culling, debug lines.
4. **Asset pipeline (3-5 weeks):** asset server, async loading, hot reload, texture/mesh import and cache, scene serialization.
5. **Gameplay base (4-6 weeks):** input actions, audio playback, rapier integration, 2D sprite renderer and UI via egui, a small playable game (e.g. breakout, then a 3D platformer).
6. **Renderer v2 (8-12 weeks):** PBR + IBL, shadow maps (cascaded), HDR + tonemapping, bloom, MSAA/TAA, skinning and animation, instancing, optional clustered forward or deferred.
7. **Reflection + editor (8-12 weeks):** reflection derive, inspector, hierarchy, asset browser, gizmos, play-in-editor, undo/redo, project files.
8. **Scripting + hot reload (4-8 weeks):** dylib game-code reload or wasm/Lua scripting host.
9. **Polish / advanced (open-ended):** networking, particle system, navmesh/AI, terrain, GPU-driven rendering, compute, profiling tools, packaging for desktop and web.

A realistic solo estimate for phases 0-6 is 6-9 months part-time; a "full" engine with editor is multi-year. Ship small games with it throughout, since a engine built without a game to drive it tends to stall.

## 6. Risks and tips

- **Compile times:** keep crates small, avoid heavy generics in hot crates, use `cargo-nextest`, `mold`/`lld`, and `opt-level=1` for dev with dependencies at `opt-level=3`.
- **Scope creep:** write the demo first, then the subsystem it needs.
- **API churn in dependencies:** wgpu and winit break often; wrap them and pin versions.
- **Testing a renderer:** headless wgpu with golden-image tests.
- **License:** choose MIT/Apache-2.0 dual license (Rust convention) early.
- **Learning resources:** Learn Wgpu (sotrh), Bevy source and cheatbook, *Game Engine Architecture* (Gregory), Sebastian Aaltonen's and Frostbite talks on frame graphs, "Data-Oriented Design" (Fabian), Rust Gamedev community and `arewegameyet.rs`.

## 7. Open questions for the owner

1. 2D, 3D, or both? (Recommendation: 3D core, 2D as a layer.)
2. Target platforms (desktop only vs web/mobile/console)?
3. Learning project vs shippable product (decides how much to buy)?
4. Editor required, and when?
5. Target genre of the first game that drives the engine?
