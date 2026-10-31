use engine::app;
use engine::math::Vec3;
use engine::render::{render, Framebuffer};
use engine::scene::Scene;
use std::process::exit;

const DEFAULT_SCENE: &str = include_str!("../scenes/default.scene");

const HELP: &str = "engine - agent-first game engine (std only, no external crates)

USAGE:
  engine run    [SCENE] [--width N] [--height N]
  engine render [SCENE] --out FILE.ppm [--width N] [--height N]
                [--pos X,Y,Z] [--yaw DEG] [--pitch DEG]
  engine help

`run` opens a window: WASD move, mouse look, Shift sprint, Tab toggles mouse
capture, Esc quits. `render` draws one frame headlessly to a PPM file so an
agent can inspect its own output. SCENE is a text file (see scenes/default.scene);
the built-in default scene is used when omitted.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = real_main(&args) {
        eprintln!("error: {}", e);
        exit(1);
    }
}

fn real_main(args: &[String]) -> Result<(), String> {
    let cmd = args.first().map(String::as_str).unwrap_or("run");
    if matches!(cmd, "help" | "--help" | "-h") {
        println!("{}", HELP);
        return Ok(());
    }
    if !matches!(cmd, "run" | "render") {
        return Err(format!("unknown command '{}'\n\n{}", cmd, HELP));
    }
    let rest = if args.is_empty() { &args[..] } else { &args[1..] };

    let mut scene_path: Option<&str> = None;
    let (mut width, mut height) = (960usize, 540usize);
    let mut out: Option<&str> = None;
    let (mut pos, mut yaw, mut pitch) = (None, None, None);
    let mut i = 0;
    while i < rest.len() {
        let a = rest[i].as_str();
        let mut val = || -> Result<&str, String> {
            i += 1;
            rest.get(i).map(String::as_str).ok_or_else(|| format!("{} needs a value", a))
        };
        match a {
            "--width" => width = val()?.parse().map_err(|_| "bad --width")?,
            "--height" => height = val()?.parse().map_err(|_| "bad --height")?,
            "--out" => out = Some(val()?),
            "--yaw" => yaw = Some(val()?.parse::<f32>().map_err(|_| "bad --yaw")?),
            "--pitch" => pitch = Some(val()?.parse::<f32>().map_err(|_| "bad --pitch")?),
            "--pos" => {
                let v: Vec<f32> = val()?.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if v.len() != 3 {
                    return Err("--pos expects X,Y,Z".into());
                }
                pos = Some(Vec3::new(v[0], v[1], v[2]));
            }
            s if s.starts_with("--") => return Err(format!("unknown flag {}", s)),
            s => scene_path = Some(s),
        }
        i += 1;
    }
    if width == 0 || height == 0 {
        return Err("width and height must be positive".into());
    }

    let src = match scene_path {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{}: {}", p, e))?,
        None => DEFAULT_SCENE.to_string(),
    };
    let mut scene = Scene::parse(&src)?;
    if let Some(p) = pos { scene.camera.pos = p; }
    if let Some(y) = yaw { scene.camera.yaw = y.to_radians(); }
    if let Some(p) = pitch { scene.camera.pitch = p.to_radians(); }

    if cmd == "render" {
        let out = out.ok_or("render needs --out FILE.ppm")?;
        let mut fb = Framebuffer::new(width, height);
        render(&scene, &scene.camera, &mut fb);
        fb.write_ppm(out).map_err(|e| format!("{}: {}", out, e))?;
        println!("wrote {} ({}x{})", out, width, height);
        Ok(())
    } else {
        app::run(scene, width, height)
    }
}
