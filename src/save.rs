use crate::ecs::World;
use crate::worldfile::Registry;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub fn data_dir(app: &str) -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library").join("Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home.map(|h| h.join(".config")))
    };
    base.unwrap_or_else(std::env::temp_dir).join(app)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    values: BTreeMap<String, String>,
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let mut config = Config::default();
        for (number, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once('=').ok_or_else(|| format!("config line {}: expected key = value", number + 1))?;
            config.values.insert(key.trim().to_string(), value.trim().to_string());
        }
        Ok(config)
    }

    pub fn to_text(&self) -> String {
        self.values.iter().map(|(k, v)| format!("{} = {}\n", k, v)).collect()
    }

    pub fn load(path: &std::path::Path) -> Config {
        std::fs::read_to_string(path).ok().and_then(|text| Config::parse(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {}", parent.display(), e))?;
        }
        std::fs::write(path, self.to_text()).map_err(|e| format!("{}: {}", path.display(), e))
    }

    pub fn set(&mut self, key: &str, value: impl ToString) {
        self.values.insert(key.to_string(), value.to_string());
    }

    pub fn text(&self, key: &str, default: &str) -> String {
        self.values.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn number(&self, key: &str, default: f32) -> f32 {
        self.values.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn flag(&self, key: &str, default: bool) -> bool {
        match self.values.get(key).map(String::as_str) {
            Some("true") | Some("1") | Some("on") => true,
            Some("false") | Some("0") | Some("off") => false,
            _ => default,
        }
    }
}

const SAVE_HEADER: &str = "engine-save 1";

pub fn save_world(registry: &Registry, world: &World, path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {}", parent.display(), e))?;
    }
    let text = format!("# {}\n{}", SAVE_HEADER, registry.save(world));
    std::fs::write(path, text).map_err(|e| format!("{}: {}", path.display(), e))
}

pub fn load_world(registry: &Registry, world: &mut World, path: &std::path::Path) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    if !text.starts_with(&format!("# {}", SAVE_HEADER)) {
        return Err(format!("{}: not an engine save file", path.display()));
    }
    for entity in world.entities() {
        world.despawn(entity);
    }
    registry.load(world, &text).map(|_| ())
}
