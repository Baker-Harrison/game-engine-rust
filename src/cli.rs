use crate::headless::{simulate, SimOptions};
use crate::input::InputScript;
use crate::runner::{run, RunOptions};
use crate::runtime::{Engine, Game};
use crate::png;
use crate::render::Framebuffer;

const GAME_HELP: &str = "USAGE:
  GAME play   [--width N] [--height N] [--renderer auto|cpu|metal|vulkan] [--frames N] [--no-audio] [--seed N]
  GAME sim    [--ticks N] [--seed N] [--script FILE] [--out DIR] [--every N] [--at N,N] [--wav FILE]
              [--width N] [--height N] [--renderer cpu|metal|vulkan]
  GAME shot   --out FILE.png [--ticks N] [--seed N] [--script FILE] [--width N] [--height N] [--renderer cpu|metal|vulkan]
  GAME world  [--seed N]
  GAME help

`play` opens a window. `sim` runs the game headlessly at a fixed 60 Hz with scripted input and prints a
report (state summary, event log, audio levels, determinism checksum); --every/--at also write frames.
`shot` runs N ticks and writes one frame. `world` prints the entity world as text.";

struct Flags {
    values: Vec<(String, String)>,
    switches: Vec<String>,
}

impl Flags {
    fn parse(args: &[String]) -> Result<Flags, String> {
        let mut flags = Flags { values: Vec::new(), switches: Vec::new() };
        let mut i = 0;
        while i < args.len() {
            let name = args[i].as_str();
            match name {
                "--no-audio" => flags.switches.push(name.to_string()),
                s if s.starts_with("--") => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| format!("{} needs a value", s))?;
                    flags.values.push((s.to_string(), value.clone()));
                }
                other => return Err(format!("unexpected argument '{}'\n\n{}", other, GAME_HELP)),
            }
            i += 1;
        }
        Ok(flags)
    }

    fn text(&self, name: &str) -> Option<&str> {
        self.values.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn number<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        match self.text(name) {
            Some(value) => value.parse().map_err(|_| format!("{} expects a number, got '{}'", name, value)),
            None => Ok(default),
        }
    }

    fn switch(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }
}

pub fn game_main(make_game: fn() -> Box<dyn Game>) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = dispatch(make_game, &args) {
        eprintln!("error: {}", error);
        std::process::exit(1);
    }
}

fn load_script(flags: &Flags) -> Result<Option<InputScript>, String> {
    match flags.text("--script") {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?;
            Ok(Some(InputScript::parse(&text)?))
        }
        None => Ok(None),
    }
}

fn dispatch(make_game: fn() -> Box<dyn Game>, args: &[String]) -> Result<(), String> {
    let command = args.first().map(String::as_str).unwrap_or("play");
    let flags = Flags::parse(if args.is_empty() { args } else { &args[1..] })?;
    let mut game = make_game();
    match command {
        "help" | "--help" | "-h" => {
            println!("{}", GAME_HELP);
            Ok(())
        }
        "play" => {
            let defaults = RunOptions::default();
            let options = RunOptions {
                width: flags.number("--width", defaults.width)?,
                height: flags.number("--height", defaults.height)?,
                renderer: flags.text("--renderer").unwrap_or("auto").to_string(),
                max_frames: flags.text("--frames").map(|v| v.parse().map_err(|_| "bad --frames")).transpose()?,
                audio: !flags.switch("--no-audio"),
                seed: flags.number("--seed", defaults.seed)?,
            };
            run(game.as_mut(), &options)
        }
        "sim" | "shot" => {
            let defaults = SimOptions::default();
            let mut options = SimOptions {
                ticks: flags.number("--ticks", if command == "shot" { 120 } else { defaults.ticks })?,
                seed: flags.number("--seed", defaults.seed)?,
                script: load_script(&flags)?,
                capture_every: flags.text("--every").map(|v| v.parse().map_err(|_| "bad --every")).transpose()?,
                capture_at: flags
                    .text("--at")
                    .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
                    .unwrap_or_default(),
                output_dir: flags.text("--out").unwrap_or("sim").to_string(),
                width: flags.number("--width", defaults.width)?,
                height: flags.number("--height", defaults.height)?,
                renderer: flags.text("--renderer").unwrap_or("cpu").to_string(),
                wav_path: flags.text("--wav").map(String::from),
            };
            if command == "shot" {
                let out = flags.text("--out").ok_or("shot needs --out FILE.png")?.to_string();
                options.capture_at = vec![options.ticks];
                options.output_dir = std::env::temp_dir().join(format!("shot-{}", std::process::id())).to_string_lossy().into_owned();
                let result = simulate(game.as_mut(), &options)?;
                let frame = result.files.first().ok_or("no frame was rendered")?;
                std::fs::copy(frame, &out).map_err(|e| format!("{}: {}", out, e))?;
                let _ = std::fs::remove_dir_all(&options.output_dir);
                println!("{}", result.report);
                println!("wrote {}", out);
                return Ok(());
            }
            let result = simulate(game.as_mut(), &options)?;
            println!("{}", result.report);
            for file in &result.files {
                println!("wrote {}", file);
            }
            Ok(())
        }
        "world" => {
            let mut engine = Engine::new(flags.number("--seed", 1u64)?);
            game.setup(&mut engine)?;
            print!("{}", engine.registry.save(&engine.world));
            Ok(())
        }
        other => Err(format!("unknown command '{}'\n\n{}", other, GAME_HELP)),
    }
}

pub fn write_frame(framebuffer: &Framebuffer, path: &str) -> Result<(), String> {
    png::write(framebuffer, path)
}
