use std::collections::HashMap;
use crate::transform::Transform;
use crate::geometry::GeometryId;

pub struct Entity {
    pub geometry_id: GeometryId,
    pub transform: Transform,
    pub color: [f32; 4],
}

pub struct World {
    pub entities: HashMap<usize, Entity>,
    next_entity_id: usize,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_entity_id: 0,
        }
    }

    pub fn spawn(&mut self, geometry_id: GeometryId, transform: Transform, color: [f32; 4]) -> usize {
        let id = self.next_entity_id;
        self.entities.insert(id, Entity {
            geometry_id,
            transform,
            color,
        });
        self.next_entity_id += 1;
        id
    }

    pub fn despawn(&mut self, entity_id: usize) {
        self.entities.remove(&entity_id);
    }

    pub fn get_entity_mut(&mut self, entity_id: usize) -> Option<&mut Entity> {
        self.entities.get_mut(&entity_id)
    }
}
