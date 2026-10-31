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
- `src/platform/`: `macos.rs` (AppKit via the Objective-C runtime), `win32.rs`, `x11.rs`
- `src/app.rs`: game loop and first-person movement with box collision

## Status

Verified: headless render and the X11 backend (Linux, under Xvfb). The macOS and
Windows backends type-check for their targets but were not run by the author.
