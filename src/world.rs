use std::collections::HashMap;
use crate::objects::Object;

#[derive(Debug)]
pub struct World {
    pub objects: HashMap<usize, Object>,
    pub roots: Vec<usize>,
    pub name_handles: HashMap<String, usize>,
    next_id: usize,
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            roots: Vec::new(),
            next_id: 0,
            name_handles: HashMap::new(),
        }
    }

    /// Reconstruct a `World` directly from its constituent parts.
    ///
    /// Used by the VTR deserializer to rebuild a world without going through
    /// `spawn_object`, so that the original IDs and hierarchy are preserved
    /// exactly.
    ///
    /// `next_id` should be set to `max(existing_ids) + 1` so that future
    /// calls to `spawn_object` never collide with the loaded objects.
    pub fn from_parts(
        objects: HashMap<usize, Object>,
        roots: Vec<usize>,
        next_id: usize,
    ) -> Self {
        let mut name_handles = HashMap::with_capacity(objects.len());

        for (&id, obj) in &objects {
            name_handles.insert(obj.str_id.clone(), id);
        }
        Self { objects, roots, next_id, name_handles }
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

    /// Returns the unique integer ID associated with a given string identifier (`str_id`).
    ///
    /// This method performs a lookup in the internal handle cache. While the lookup is
    /// technically $O(1)$ on average, it involves hashing the input string and searching
    /// a `HashMap`.
    ///
    /// # Performance Warning
    ///
    /// **Do not use this method inside `on_update` or other high-frequency loops.**
    ///
    /// Calling this every frame for multiple objects will cause significant performance
    /// degradation due to repeated string hashing and cache misses. Instead, "memoize"
    /// the ID: call this method once during `on_startup`, store the resulting `usize`
    /// in your application state, and use that integer ID for direct access during updates.
    ///
    /// # Examples
    ///
    /// ```
    /// // Correct: Resolve once during initialization
    /// let sun_id = scene.get_id("sun_center").expect("Sun not found in scene!");
    /// state.sun_id = Some(sun_id);
    /// ```
    pub fn get_id(&self, str_id: &str) -> Option<usize> {
        self.name_handles.get(str_id).copied()
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