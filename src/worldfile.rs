use crate::ecs::{Entity, World};
use crate::math::Vec3;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Fields {
    values: HashMap<String, String>,
    order: Vec<String>,
}

impl Fields {
    pub fn parse(words: &[&str]) -> Result<Fields, String> {
        let mut fields = Fields::default();
        for word in words {
            let (key, value) = word.split_once('=').ok_or_else(|| format!("expected key=value, got '{}'", word))?;
            fields.order.push(key.to_string());
            fields.values.insert(key.to_string(), value.to_string());
        }
        Ok(fields)
    }

    pub fn text(&self, key: &str) -> Result<&str, String> {
        self.values.get(key).map(String::as_str).ok_or_else(|| format!("missing field '{}'", key))
    }

    pub fn text_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.values.get(key).map(String::as_str).unwrap_or(default)
    }

    pub fn f32(&self, key: &str) -> Result<f32, String> {
        self.text(key)?.parse().map_err(|_| format!("field '{}' is not a number", key))
    }

    pub fn f32_or(&self, key: &str, default: f32) -> f32 {
        self.f32(key).unwrap_or(default)
    }

    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        match self.values.get(key).map(String::as_str) {
            Some("true") | Some("1") => true,
            Some("false") | Some("0") => false,
            _ => default,
        }
    }

    pub fn vec3(&self, key: &str) -> Result<Vec3, String> {
        parse_vec3(self.text(key)?).ok_or_else(|| format!("field '{}' is not x,y,z", key))
    }

    pub fn vec3_or(&self, key: &str, default: Vec3) -> Vec3 {
        self.vec3(key).unwrap_or(default)
    }
}

pub fn parse_vec3(text: &str) -> Option<Vec3> {
    let parts: Vec<f32> = text.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    if parts.len() == 3 {
        Some(Vec3::new(parts[0], parts[1], parts[2]))
    } else {
        None
    }
}

pub fn format_vec3(v: Vec3) -> String {
    format!("{},{},{}", format_f32(v.x), format_f32(v.y), format_f32(v.z))
}

pub fn format_f32(value: f32) -> String {
    let rounded = (value * 10000.0).round() / 10000.0;
    format!("{}", rounded)
}

pub trait Component: Clone + 'static {
    const NAME: &'static str;
    fn from_fields(fields: &Fields) -> Result<Self, String>;
    fn to_fields(&self) -> Vec<(String, String)>;
}

type InsertFn = Box<dyn Fn(&mut World, Entity, &Fields) -> Result<(), String>>;
type WriteFn = Box<dyn Fn(&World, Entity) -> Option<Vec<(String, String)>>>;

struct Kind {
    name: &'static str,
    insert: InsertFn,
    write: WriteFn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Name(pub String);

impl Component for Name {
    const NAME: &'static str = "name";

    fn from_fields(fields: &Fields) -> Result<Name, String> {
        Ok(Name(fields.text("value")?.to_string()))
    }

    fn to_fields(&self) -> Vec<(String, String)> {
        vec![("value".into(), self.0.clone())]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub yaw: f32,
    pub scale: f32,
}

impl Transform {
    pub fn at(position: Vec3) -> Transform {
        Transform { position, yaw: 0.0, scale: 1.0 }
    }
}

impl Component for Transform {
    const NAME: &'static str = "transform";

    fn from_fields(fields: &Fields) -> Result<Transform, String> {
        Ok(Transform {
            position: fields.vec3_or("pos", Vec3::ZERO),
            yaw: fields.f32_or("yaw", 0.0),
            scale: fields.f32_or("scale", 1.0),
        })
    }

    fn to_fields(&self) -> Vec<(String, String)> {
        vec![
            ("pos".into(), format_vec3(self.position)),
            ("yaw".into(), format_f32(self.yaw)),
            ("scale".into(), format_f32(self.scale)),
        ]
    }
}

pub struct Registry {
    kinds: Vec<Kind>,
}

impl Default for Registry {
    fn default() -> Registry {
        let mut registry = Registry { kinds: Vec::new() };
        registry.register::<Name>();
        registry.register::<Transform>();
        registry
    }
}

impl Registry {
    pub fn register<T: Component>(&mut self) {
        self.kinds.push(Kind {
            name: T::NAME,
            insert: Box::new(|world, entity, fields| {
                world.insert(entity, T::from_fields(fields)?);
                Ok(())
            }),
            write: Box::new(|world, entity| world.get::<T>(entity).map(|c| c.to_fields())),
        });
    }

    pub fn component_names(&self) -> Vec<&'static str> {
        self.kinds.iter().map(|k| k.name).collect()
    }

    pub fn load(&self, world: &mut World, text: &str) -> Result<HashMap<String, Entity>, String> {
        let mut names = HashMap::new();
        let mut current: Option<Entity> = None;
        for (number, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let fail = |message: String| format!("world line {}: {}", number + 1, message);
            if words[0] == "entity" {
                let entity = world.spawn();
                if let Some(name) = words.get(1) {
                    names.insert(name.to_string(), entity);
                    world.insert(entity, Name(name.to_string()));
                }
                current = Some(entity);
                continue;
            }
            let entity = current.ok_or_else(|| fail("component before any 'entity' line".into()))?;
            let kind = self
                .kinds
                .iter()
                .find(|k| k.name == words[0])
                .ok_or_else(|| fail(format!("unknown component '{}' (known: {})", words[0], self.component_names().join(", "))))?;
            let fields = Fields::parse(&words[1..]).map_err(&fail)?;
            (kind.insert)(world, entity, &fields).map_err(&fail)?;
        }
        Ok(names)
    }

    pub fn save(&self, world: &World) -> String {
        let mut out = String::new();
        for entity in world.entities() {
            let name = world.get::<Name>(entity).map(|n| n.0.clone()).unwrap_or_else(|| format!("e{}", entity.index()));
            out.push_str(&format!("entity {}\n", name));
            for kind in &self.kinds {
                if kind.name == Name::NAME {
                    continue;
                }
                if let Some(fields) = (kind.write)(world, entity) {
                    let pairs: Vec<String> = fields.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                    out.push_str(&format!("  {} {}\n", kind.name, pairs.join(" ")));
                }
            }
        }
        out
    }

    pub fn checksum(&self, world: &World) -> u64 {
        fnv1a(self.save(world).as_bytes())
    }
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
