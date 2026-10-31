use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Params {
    values: BTreeMap<&'static str, f32>,
}

impl Params {
    pub fn with_defaults(defaults: &[(&'static str, f32)]) -> Params {
        Params { values: defaults.iter().cloned().collect() }
    }

    pub fn set(&mut self, name: &str, value: f32) -> Result<(), String> {
        match self.values.iter_mut().find(|(key, _)| **key == name) {
            Some((_, slot)) => {
                *slot = value;
                Ok(())
            }
            None => {
                let known: Vec<&str> = self.values.keys().copied().collect();
                Err(format!("unknown parameter '{}' (known: {})", name, known.join(", ")))
            }
        }
    }

    pub fn get(&self, name: &str) -> f32 {
        *self.values.get(name).unwrap_or_else(|| panic!("parameter '{}' is not declared", name))
    }

    pub fn flag(&self, name: &str) -> bool {
        self.get(name) >= 0.5
    }

    pub fn entries(&self) -> Vec<(&'static str, f32)> {
        self.values.iter().map(|(name, value)| (*name, *value)).collect()
    }
}
