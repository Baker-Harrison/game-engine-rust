use crate::worldfile::fnv1a;
use std::path::{Path, PathBuf};
use std::process::Command;

const GAME_MANIFEST: &str = "[package]
name = \"{name}\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
engine = { path = \"../..\" }
";

const GAME_SOURCE: &str = "use engine::camera::Camera;
use engine::input::Input;
use engine::math::Vec3;
use engine::runtime::{Engine, Game};
use engine::scene::Tri;
use engine::ui::{Align, WHITE};

struct {type_name} {
    spin: f32,
}

impl Game for {type_name} {
    fn name(&self) -> &str {
        \"{name}\"
    }

    fn setup(&mut self, engine: &mut Engine) -> Result<(), String> {
        engine.scene = engine::scene::Scene::parse(\"ground size=60\\n\")?;
        engine.scene.camera = Camera::new(Vec3::new(0.0, 2.0, 6.0));
        engine.scene_dirty = true;
        Ok(())
    }

    fn tick(&mut self, _engine: &mut Engine, _input: &Input) {
        self.spin += 0.02;
    }

    fn draw(&mut self, engine: &mut Engine, _blend: f32) {
        let (sin, cos) = self.spin.sin_cos();
        let corner = |x: f32, z: f32| Vec3::new(x * cos - z * sin, 0.0, x * sin + z * cos);
        let tip = Vec3::new(0.0, 2.0, 0.0);
        engine.scene.dynamic.clear();
        engine.scene.dynamic.push(Tri::flat([corner(-1.0, -1.0), corner(1.0, -1.0), tip], Vec3::new(0.9, 0.5, 0.2)));
        engine.scene.dynamic.push(Tri::flat([corner(1.0, -1.0), corner(1.0, 1.0), tip], Vec3::new(0.3, 0.6, 0.9)));
        engine.scene.dynamic.push(Tri::flat([corner(1.0, 1.0), corner(-1.0, 1.0), tip], Vec3::new(0.5, 0.8, 0.4)));
        engine.scene.dynamic.push(Tri::flat([corner(-1.0, 1.0), corner(-1.0, -1.0), tip], Vec3::new(0.8, 0.4, 0.6)));
        engine.ui.clear();
        engine.ui.text(640.0, 40.0, \"{name}\", 4.0, WHITE, Align::Center);
        engine.scene.overlay = engine.ui.finish();
    }
}

fn main() {
    engine::cli::game_main(|| Box::new({type_name} { spin: 0.0 }));
}
";

fn type_name_for(name: &str) -> String {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map(|first| first.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
        })
        .collect()
}

pub fn new_game(root: &Path, name: &str) -> Result<Vec<String>, String> {
    let valid = !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid || name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("game names are lowercase letters, digits, '-' and '_' and must not start with a digit".into());
    }
    let directory = root.join("games").join(name);
    if directory.exists() {
        return Err(format!("{} already exists", directory.display()));
    }
    std::fs::create_dir_all(directory.join("src")).map_err(|e| e.to_string())?;
    let manifest = GAME_MANIFEST.replace("{name}", name);
    let source = GAME_SOURCE.replace("{name}", name).replace("{type_name}", &type_name_for(name));
    let files = [(directory.join("Cargo.toml"), manifest), (directory.join("src").join("main.rs"), source)];
    let mut written = Vec::new();
    for (path, text) in files {
        std::fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), e))?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

fn copy_tree(from: &Path, to: &Path, copied: &mut Vec<PathBuf>) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {}", to.display(), e))?;
    for entry in std::fs::read_dir(from).map_err(|e| format!("{}: {}", from.display(), e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target, copied)?;
        } else {
            std::fs::copy(entry.path(), &target).map_err(|e| format!("{}: {}", target.display(), e))?;
            copied.push(target);
        }
    }
    Ok(())
}

pub fn package_game(root: &Path, name: &str) -> Result<String, String> {
    if !root.join("games").join(name).join("Cargo.toml").exists() {
        return Err(format!("no game named '{}' under games/", name));
    }
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", name])
        .current_dir(root)
        .status()
        .map_err(|e| format!("could not run cargo: {}", e))?;
    if !status.success() {
        return Err("cargo build failed".into());
    }
    let built = root.join("target").join("release").join(executable_name(name));
    let output = root.join("dist").join(name);
    if output.exists() {
        std::fs::remove_dir_all(&output).map_err(|e| format!("{}: {}", output.display(), e))?;
    }
    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    let binary = output.join(executable_name(name));
    std::fs::copy(&built, &binary).map_err(|e| format!("{}: {}", built.display(), e))?;
    files.push(binary);
    let assets = root.join("games").join(name).join("assets");
    if assets.exists() {
        copy_tree(&assets, &output.join("assets"), &mut files)?;
    }
    let launcher_name = if cfg!(windows) { "play.bat" } else { "play.sh" };
    let launcher = if cfg!(windows) {
        format!("@echo off\r\ncd /d \"%~dp0\"\r\n{} play %*\r\n", executable_name(name))
    } else {
        format!("#!/bin/sh\ncd \"$(dirname \"$0\")\"\nexec ./{} play \"$@\"\n", name)
    };
    let launcher_path = output.join(launcher_name);
    std::fs::write(&launcher_path, launcher).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&launcher_path, std::fs::Permissions::from_mode(0o755));
    }
    files.push(launcher_path);
    let mut manifest = String::new();
    for file in &files {
        let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
        let relative = file.strip_prefix(&output).unwrap_or(file).display().to_string().replace('\\', "/");
        manifest.push_str(&format!("{}  {} bytes  {:016x}\n", relative, bytes.len(), fnv1a(&bytes)));
    }
    std::fs::write(output.join("manifest.txt"), &manifest).map_err(|e| e.to_string())?;
    Ok(format!("packaged {} into {}\n{}", name, output.display(), manifest))
}
