use engine::math::Vec3;
use engine::gfx;
use engine::scene::Scene;
use std::process::exit;

const DEFAULT_SCENE: &str = include_str!("../scenes/default.scene");

const HELP: &str = "engine - agent-first game engine (std only, no external crates)

USAGE:
  engine run    [SCENE] [--width N] [--height N] [--renderer auto|cpu|metal|vulkan] [--frames N]
  engine render [SCENE] --out FILE.ppm [--width N] [--height N]
                [--pos X,Y,Z] [--yaw DEG] [--pitch DEG] [--renderer cpu|metal]
  engine new NAME
  engine package NAME
  engine asset list
  engine asset NAME [key=value ...] [--obj FILE.obj]
  engine inspect NAME [key=value ...] --out DIR
  engine check  NAME [key=value ...]
  engine probe  NAME [key=value ...] [--json] [--out FILE]
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
    if cmd == "new" || cmd == "package" {
        let name = args.get(1).ok_or_else(|| format!("{} needs a game name", cmd))?;
        let root = std::env::current_dir().map_err(|e| e.to_string())?;
        if cmd == "new" {
            for file in engine::tools::new_game(&root, name)? {
                println!("wrote {}", file);
            }
            println!("run it with: cargo run --release -p {} -- play", name);
        } else {
            println!("{}", engine::tools::package_game(&root, name)?);
        }
        return Ok(());
    }
    if cmd == "asset" || cmd == "inspect" || cmd == "probe" || cmd == "check" {
        return asset_command(cmd, &args[1..]);
    }
    if !matches!(cmd, "run" | "render") {
        return Err(format!("unknown command '{}'\n\n{}", cmd, HELP));
    }
    let rest = if args.is_empty() { &args[..] } else { &args[1..] };

    let mut scene_path: Option<&str> = None;
    let (mut width, mut height) = (960usize, 540usize);
    let mut out: Option<&str> = None;
    let mut renderer = "auto";
    let mut max_frames: Option<usize> = None;
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
            "--renderer" => renderer = val()?,
            "--frames" => max_frames = Some(val()?.parse().map_err(|_| "bad --frames")?),
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
        let mut r = gfx::create_headless(renderer, width, height)?;
        r.upload_scene(&scene)?;
        let fb = r.render_offscreen(&scene, &scene.camera)?;
        fb.write_ppm(out).map_err(|e| format!("{}: {}", out, e))?;
        println!("wrote {} ({}x{})", out, width, height);
        Ok(())
    } else {
        let mut viewer = engine::viewer::SceneViewer::new("engine", scene);
        let options = engine::runner::RunOptions { width, height, renderer: renderer.to_string(), max_frames, audio: false, seed: 1 };
        engine::runner::run(&mut viewer, &options)
    }
}

fn asset_command(command: &str, rest: &[String]) -> Result<(), String> {
    if command == "asset" && rest.first().map(String::as_str) == Some("list") {
        for definition in engine::assets::definitions() {
            println!("{}  {}", definition.name, definition.summary);
            let parameters: Vec<String> = definition.parameters.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
            println!("    {}", parameters.join(" "));
        }
        return Ok(());
    }
    let name = rest.first().ok_or("expected an asset name; try: engine asset list")?;
    let mut overrides = Vec::new();
    let mut out: Option<&str> = None;
    let mut obj: Option<&str> = None;
    let mut json = false;
    let mut i = 1;
    while i < rest.len() {
        match rest[i].as_str() {
            "--out" => {
                i += 1;
                out = Some(rest.get(i).ok_or("--out needs a value")?);
            }
            "--json" => json = true,
            "--obj" => {
                i += 1;
                obj = Some(rest.get(i).ok_or("--obj needs a value")?);
            }
            other => overrides.push(engine::assets::parse_override(other)?),
        }
        i += 1;
    }
    let asset = engine::assets::build(name, &overrides)?;
    if command == "check" {
        let validation = engine::model::validate::validate(&asset);
        let validation_errors: Vec<String> = validation.iter().filter(|f| f.severity == engine::model::validate::Severity::Error).map(|f| f.format()).collect();
        let views = if asset.views.is_empty() { engine::inspect::automatic_views(&asset) } else { asset.views.clone() };
        let scene = engine::probe::ProbeScene::from_asset(&asset, views);
        let report = engine::probe::run(&scene, &engine::probe::targets::Targets::default());
        for line in &validation_errors {
            println!("{}", line);
        }
        println!("{}", report.text());
        let total = validation_errors.len() + report.error_count();
        return if total > 0 { Err(format!("{} check errors", total)) } else { println!("check passed"); Ok(()) };
    }
    if command == "probe" {
        let views = if asset.views.is_empty() { engine::inspect::automatic_views(&asset) } else { asset.views.clone() };
        let scene = engine::probe::ProbeScene::from_asset(&asset, views);
        let report = engine::probe::run(&scene, &engine::probe::targets::Targets::default());
        let text = if json { report.json() } else { report.text() };
        match out {
            Some(path) => std::fs::write(path, &text).map_err(|e| format!("{}: {}", path, e))?,
            None => println!("{}", text),
        }
        return if report.error_count() > 0 { Err(format!("{} probe errors", report.error_count())) } else { Ok(()) };
    }
    if command == "inspect" {
        let out = out.ok_or("inspect needs --out DIR")?;
        let result = engine::inspect::inspect(&asset, out)?;
        println!("{}", result.report);
        for file in result.files {
            println!("wrote {}", file);
        }
        return Ok(());
    }
    let mesh = asset.merged_mesh(&[]);
    println!("{}: {} triangles", name, mesh.triangle_count());
    if let Some(path) = obj {
        mesh.write_obj(path)?;
        println!("wrote {}", path);
    }
    Ok(())
}
