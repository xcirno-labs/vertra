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

        // Validate parent: if the requested parent does not exist, fall back to
        // root-level placement so the new object is always reachable for
        // traversal/rendering and the hierarchy stays consistent.
        let resolved_parent = parent_id.filter(|p_id| self.objects.contains_key(p_id));
        if parent_id.is_some() && resolved_parent.is_none() {
            eprintln!(
                "spawn_object: parent_id {:?} does not exist; spawning '{}' at root instead",
                parent_id,
                object.str_id
            );
        }

        self.name_handles.insert(object.str_id.clone(), id);
        object.parent = resolved_parent;
        // If it has a parent, link the child to the parent
        if let Some(p_id) = resolved_parent {
            if let Some(parent_obj) = self.objects.get_mut(&p_id) {
                parent_obj.children.push(id);
            }
        } else {
            // If no parent, it's a root object
            self.roots.push(id);
        }

        self.objects.insert(id, object);

        if let Some(cb) = &mut self.on_scene_graph_modified {
            (cb.0)(SceneGraphEvent::ObjectAdded { id, parent_id: resolved_parent });
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

    /// Rename the stable string identifier of a live object and keep the
    /// internal `name_handles` cache in sync.
    ///
    /// **Always prefer this over writing to `object.str_id` directly** when the
    /// object is already inside a `World`.  Direct field assignment bypasses the
    /// cache and will silently break every subsequent [`World::get_id`] call for
    /// the old or new identifier.
    ///
    /// Returns `false` (no-op) when `id` does not exist.
    pub fn rename_str_id(&mut self, id: usize, new_str_id: String) -> bool {
        if let Some(obj) = self.objects.get_mut(&id) {
            let old = std::mem::replace(&mut obj.str_id, new_str_id.clone());
            self.name_handles.remove(&old);
            self.name_handles.insert(new_str_id, id);
            true
        } else {
            false
        }
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
        // Remove the root object first so we can read its children list and
        // parent link before they disappear.
        let obj = match self.objects.remove(&id) {
            Some(o) => o,
            None    => return,   // nothing to do; fire no event
        };

        self.name_handles.remove(&obj.str_id);

        // Unlink from parent / root list
        if let Some(p_id) = obj.parent {
            if let Some(parent) = self.objects.get_mut(&p_id) {
                parent.children.retain(|&c| c != id);
            }
        } else {
            self.roots.retain(|&r| r != id);
        }

        // Recursively destroy all descendants (they were already children of `id`)
        for child_id in obj.children {
            self.recursive_remove(child_id);
        }

        if let Some(cb) = &mut self.on_scene_graph_modified {
            (cb.0)(SceneGraphEvent::ObjectDeleted { id });
        }
    }

    /// Returns `true` when `ancestor` is `node` itself or any ancestor of `node`
    /// in `node`'s subtree — i.e. when making `ancestor` a child of `node` would
    /// create a cycle.
    ///
    /// Walks *down* from `node` through children, so the name "is_in_subtree"
    /// means "`candidate` appears somewhere inside the subtree rooted at `node`".
    fn is_in_subtree(&self, node: usize, candidate: usize) -> bool {
        if node == candidate { return true; }
        if let Some(obj) = self.objects.get(&node) {
            for &child in &obj.children {
                if self.is_in_subtree(child, candidate) {
                    return true;
                }
            }
        }
        false
    }

    /// Move `id` to a new parent (or to the scene root when `new_parent` is `None`).
    ///
    /// # No-op conditions
    /// - `id` does not exist in the world.
    /// - `new_parent` is the same as the current parent.
    /// - `new_parent == Some(id)` (self-parenting).
    /// - `new_parent` does not exist in the world (guards against dangling links).
    /// - `new_parent` is a descendant of `id` (would create a cycle).
    ///
    /// The object's children are carried along unchanged.
    pub fn reparent(&mut self, id: usize, new_parent: Option<usize>) -> bool {
        // Self-parenting
        if Some(id) == new_parent { return false; }

        // Object must exist
        let old_parent = match self.objects.get(&id) {
            Some(obj) => obj.parent,
            None => return false,
        };

        // Already there
        if old_parent == new_parent { return false; }

        // Target parent must exist (unless moving to root)
        if let Some(p_id) = new_parent {
            if !self.objects.contains_key(&p_id) { return false; }
        }

        // Cycle guard: new_parent must not be inside id's subtree
        // TODO: Instead of failing, we can instead switch the position of those objects
        if let Some(p_id) = new_parent {
            if self.is_in_subtree(id, p_id) { return false; }
        }

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
        true
    }
}