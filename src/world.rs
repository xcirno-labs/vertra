use std::collections::HashMap;
use crate::objects::Object;

/// Describes a structural change to the scene hierarchy.
///
/// Fired whenever objects are added, removed, or re-parented.
#[derive(Debug, Clone)]
pub enum SceneGraphEvent {
    /// A new object was inserted into the world.
    ObjectAdded { id: usize, parent_id: Option<usize> },
    /// An object (and all its descendants) was removed.
    ObjectDeleted { id: usize },
    /// An object was moved to a different parent (or to/from root level).
    ObjectReparented { id: usize, old_parent: Option<usize>, new_parent: Option<usize> },
}

/// Newtype wrapper around a `FnMut(SceneGraphEvent)` that satisfies `Debug`.
pub struct SceneGraphCallback(pub Box<dyn FnMut(SceneGraphEvent)>);

impl std::fmt::Debug for SceneGraphCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SceneGraphCallback(<fn>)")
    }
}

#[derive(Debug)]
pub struct World {
    pub objects: HashMap<usize, Object>,
    pub roots: Vec<usize>,
    pub name_handles: HashMap<String, usize>,
    next_id: usize,
    /// Optional callback invoked after every structural scene-graph change.
    pub on_scene_graph_modified: Option<SceneGraphCallback>,
}

impl World {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            roots: Vec::new(),
            next_id: 0,
            name_handles: HashMap::new(),
            on_scene_graph_modified: None,
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
        Self { objects, roots, next_id, name_handles, on_scene_graph_modified: None }
    }

    pub fn spawn_object(&mut self, mut object: Object, parent_id: Option<usize>) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        self.name_handles.insert(object.str_id.clone(), id);
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

        if let Some(cb) = &mut self.on_scene_graph_modified {
            (cb.0)(SceneGraphEvent::ObjectAdded { id, parent_id });
        }
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
    /// ```ignore
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
            self.name_handles.remove(&obj.str_id);
            for child_id in obj.children {
                self.recursive_remove(child_id);
            }
        }
    }

    pub fn delete(&mut self, id: usize) {
        let existed = if let Some(obj) = self.objects.remove(&id) {
            self.name_handles.remove(&obj.str_id);
            if let Some(p_id) = obj.parent {
                if let Some(parent) = self.objects.get_mut(&p_id) {
                    parent.children.retain(|&child_id| child_id != id);
                }
            } else {
                self.roots.retain(|&root_id| root_id != id);
            }
            true
        } else {
            false
        };
        self.recursive_remove(id);

        if existed {
            if let Some(cb) = &mut self.on_scene_graph_modified {
                (cb.0)(SceneGraphEvent::ObjectDeleted { id });
            }
        }
    }

    /// Move `id` to a new parent (or to the scene root when `new_parent` is `None`).
    ///
    /// No-op when `id` does not exist or equals `new_parent`.
    /// The object's children are carried along unchanged.
    pub fn reparent(&mut self, id: usize, new_parent: Option<usize>) {
        if Some(id) == new_parent { return; }

        let old_parent = match self.objects.get(&id) {
            Some(obj) => obj.parent,
            None => return,
        };
        if old_parent == new_parent { return; }

        // Detach from current location
        if let Some(p_id) = old_parent {
            if let Some(p) = self.objects.get_mut(&p_id) {
                p.children.retain(|&c| c != id);
            }
        } else {
            self.roots.retain(|&r| r != id);
        }

        // Attach to new location
        if let Some(p_id) = new_parent {
            if let Some(p) = self.objects.get_mut(&p_id) {
                p.children.push(id);
            }
        } else {
            self.roots.push(id);
        }

        if let Some(obj) = self.objects.get_mut(&id) {
            obj.parent = new_parent;
        }

        if let Some(cb) = &mut self.on_scene_graph_modified {
            (cb.0)(SceneGraphEvent::ObjectReparented { id, old_parent, new_parent });
        }
    }
}