use std::collections::HashMap;
use crate::objects::Object;

pub struct World {
    pub objects: HashMap<usize, Object>,
    pub roots: Vec<usize>,
    next_id: usize,
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            roots: Vec::new(),
            next_id: 0,
        }
    }

    pub fn spawn_object(&mut self, mut object: Object, parent_id: Option<usize>) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        object.parent = parent_id;

        // If it has a parent, link the child to the parent
        if let Some(p_id) = parent_id {
            if let Some(parent_obj) = self.objects.get_mut(&p_id) {
                parent_obj.children.push(id);
            }
        } else {
            // If no parent, it's a root object
            self.roots.push(id);
        }

        self.objects.insert(id, object);
        id
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }

    fn recursive_remove(&mut self, id: usize) {
        // Remove the object and take ownership of its children list
        if let Some(obj) = self.objects.remove(&id) {
            for child_id in obj.children {
                self.recursive_remove(child_id);
            }
        }
    }

    pub fn delete(&mut self, id: usize) {
        if let Some(obj) = self.objects.remove(&id) {
            if let Some(p_id) = obj.parent {
                if let Some(parent) = self.objects.get_mut(&p_id) {
                    parent.children.retain(|&child_id| child_id != id);
                }
            } else {
                self.roots.retain(|&root_id| root_id != id);
            }
        }
        self.recursive_remove(id);
    }
}