use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub fn index(self) -> u32 {
        self.index
    }

    pub fn id(self) -> u64 {
        ((self.generation as u64) << 32) | self.index as u64
    }

    pub fn from_id(id: u64) -> Entity {
        Entity { index: id as u32, generation: (id >> 32) as u32 }
    }
}

trait Store: Any {
    fn remove_index(&mut self, index: usize);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct Column<T> {
    items: Vec<Option<T>>,
}

impl<T: 'static> Store for Column<T> {
    fn remove_index(&mut self, index: usize) {
        if let Some(slot) = self.items.get_mut(index) {
            *slot = None;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
    columns: HashMap<TypeId, Box<dyn Store>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            self.alive[index as usize] = true;
            return Entity { index, generation: self.generations[index as usize] };
        }
        self.generations.push(0);
        self.alive.push(true);
        Entity { index: self.generations.len() as u32 - 1, generation: 0 }
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations.get(entity.index as usize) == Some(&entity.generation) && self.alive[entity.index as usize]
    }

    pub fn despawn(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
            return;
        }
        for column in self.columns.values_mut() {
            column.remove_index(entity.index as usize);
        }
        self.alive[entity.index as usize] = false;
        self.generations[entity.index as usize] += 1;
        self.free.push(entity.index);
    }

    pub fn entity_count(&self) -> usize {
        self.alive.iter().filter(|a| **a).count()
    }

    pub fn entities(&self) -> Vec<Entity> {
        (0..self.alive.len())
            .filter(|i| self.alive[*i])
            .map(|i| Entity { index: i as u32, generation: self.generations[i] })
            .collect()
    }

    pub fn insert<T: 'static>(&mut self, entity: Entity, component: T) {
        if !self.is_alive(entity) {
            return;
        }
        let column = self
            .columns
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Column::<T> { items: Vec::new() }));
        let column = column.as_any_mut().downcast_mut::<Column<T>>().expect("column type");
        let index = entity.index as usize;
        if column.items.len() <= index {
            column.items.resize_with(index + 1, || None);
        }
        column.items[index] = Some(component);
    }

    pub fn remove<T: 'static>(&mut self, entity: Entity) {
        if let Some(column) = self.columns.get_mut(&TypeId::of::<T>()) {
            column.remove_index(entity.index as usize);
        }
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) {
            return None;
        }
        let column = self.columns.get(&TypeId::of::<T>())?.as_any().downcast_ref::<Column<T>>()?;
        column.items.get(entity.index as usize)?.as_ref()
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) {
            return None;
        }
        let column = self.columns.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut::<Column<T>>()?;
        column.items.get_mut(entity.index as usize)?.as_mut()
    }

    pub fn has<T: 'static>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    pub fn with<T: 'static>(&self) -> Vec<Entity> {
        let Some(column) = self.columns.get(&TypeId::of::<T>()) else {
            return Vec::new();
        };
        let column = column.as_any().downcast_ref::<Column<T>>().expect("column type");
        column
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_some())
            .map(|(i, _)| Entity { index: i as u32, generation: self.generations[i] })
            .collect()
    }

    pub fn count<T: 'static>(&self) -> usize {
        self.with::<T>().len()
    }

    pub fn each<T: 'static>(&self, mut visit: impl FnMut(Entity, &T)) {
        for entity in self.with::<T>() {
            if let Some(component) = self.get::<T>(entity) {
                visit(entity, component);
            }
        }
    }

    pub fn each_mut<T: 'static>(&mut self, mut visit: impl FnMut(Entity, &mut T)) {
        for entity in self.with::<T>() {
            if let Some(component) = self.get_mut::<T>(entity) {
                visit(entity, component);
            }
        }
    }

    pub fn set_resource<T: 'static>(&mut self, value: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn resource<T: 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }
}
