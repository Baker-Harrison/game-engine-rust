# engine

Agent-first game engine in Rust. CLI-first, **zero external crates** (std only).
Window, input and graphics are raw FFI written in-house.

## Run it

Install Rust (https://rustup.rs), then from this folder:

```
cargo run --release -- run
```

Controls: **WASD** move, **mouse** look, **Shift** sprint, **Tab** release or recapture the mouse, **Esc** quit.

macOS: the first launch may ask for no permissions; if the window opens behind the terminal, click its Dock icon.

## Renderers

`--renderer auto|cpu|metal|vulkan` (default `auto`):

- macOS: Metal. Linux and Windows: Vulkan. `auto` falls back to the CPU renderer if the GPU backend cannot start.
- The CPU renderer is the reference. `cargo test` renders the same frames with the CPU and GPU backends and checks they match.
- The Vulkan loader is opened at runtime, so building needs no Vulkan SDK. Shaders are checked-in SPIR-V; regenerate with `shaders/build.sh` (needs `glslangValidator`).
- `engine run --frames N` exits after N frames (used by CI smoke tests).
- Linux: libX11 and the Vulkan loader are opened at runtime, so building needs no dev packages.
- Headless GPU render: `engine render --renderer vulkan --out frame.ppm`.

## For agents

```
cargo run --release -- render --out frame.ppm --pos 0,1.7,8 --yaw 30 --pitch -5
cargo run --release -- run path/to/my.scene
```

`render` draws one frame headlessly to a PPM so an agent can check its own output.
Scenes are plain text, see `scenes/default.scene`.

## Layout

- `src/math.rs`, `camera.rs`, `scene.rs`, `input.rs`: platform-independent core
- `src/render.rs`: software rasterizer (clipping, z-buffer, shading, fog)
- `src/gfx/`: engine-owned `Renderer` trait with `cpu.rs`, `metal.rs`, `vulkan.rs`
- `src/platform/`: `macos.rs` (AppKit via the Objective-C runtime), `win32.rs`, `x11.rs`
- `src/app.rs`: game loop and first-person movement with box collision

## Status

Verified on Linux: CPU renderer, X11 window, and the Vulkan backend (software Vulkan, window and headless, matching the CPU renderer). The macOS (window + Metal) and Windows backends type-check but have not been run on real machines. There is no D3D12 backend yet; Windows GPUs ship Vulkan drivers, so Vulkan covers them.
