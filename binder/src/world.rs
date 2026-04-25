use wasm_bindgen::prelude::*;
use js_sys::Function;
use serde::Serialize;
use crate::objects::Object;
use vertra::world::{World as CoreWorld, SceneGraphEvent, SceneGraphCallback};

thread_local! {
    static SCENE_GRAPH_CB: std::cell::RefCell<Option<Function>> =
        std::cell::RefCell::new(None);
}

/// Register (or clear) the JS function that receives scene-graph change events.
pub(crate) fn register_scene_graph_cb(f: Option<Function>) {
    SCENE_GRAPH_CB.with(|c| *c.borrow_mut() = f);
}

/// Serialisable event sent to JavaScript.
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum SceneGraphModifiedEvent {
    #[serde(rename = "object_added")]
    ObjectAdded { id: usize, parent_id: Option<usize> },
    #[serde(rename = "object_deleted")]
    ObjectDeleted { id: usize },
    #[serde(rename = "object_reparented")]
    ObjectReparented { id: usize, old_parent: Option<usize>, new_parent: Option<usize> },
}

fn fire_scene_graph_event(ev: SceneGraphModifiedEvent) {
    SCENE_GRAPH_CB.with(|c| {
        if let Some(cb) = c.borrow().as_ref() {
            if let Ok(js) = serde_wasm_bindgen::to_value(&ev) {
                let _ = cb.call1(&JsValue::UNDEFINED, &js);
            }
        }
    });
}

/// Install the scene-graph callback on a `CoreWorld` so every structural
/// mutation fires the registered JS handler.
pub(crate) fn attach_scene_graph_cb(world: &mut CoreWorld) {
    world.on_scene_graph_modified = Some(SceneGraphCallback(Box::new(|ev: SceneGraphEvent| {
        let web_ev = match ev {
            SceneGraphEvent::ObjectAdded { id, parent_id } =>
                SceneGraphModifiedEvent::ObjectAdded { id, parent_id },
            SceneGraphEvent::ObjectDeleted { id } =>
                SceneGraphModifiedEvent::ObjectDeleted { id },
            SceneGraphEvent::ObjectReparented { id, old_parent, new_parent } =>
                SceneGraphModifiedEvent::ObjectReparented { id, old_parent, new_parent },
        };
        fire_scene_graph_event(web_ev);
    })));
}

/// The entity management system and scene hierarchy container.
///
/// Handles creation, destruction, and retrieval of scene objects.
/// Obtain the world for the active scene via [`Scene::world`].
#[wasm_bindgen]
pub struct World {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreWorld,
}

#[wasm_bindgen]
impl World {
    /// Spawns an object into the world, optionally as a child of an existing object.
    ///
    /// # Arguments
    ///
    /// * `object`    - The template object to clone into the world.
    /// * `parent_id` - ID of the parent object.  Pass `undefined` / `null` to
    ///   add the object at the scene root.
    ///
    /// # Returns
    ///
    /// The unique integer ID assigned to the new object instance.
    pub fn spawn_object(&mut self, object: &Object, parent_id: Option<usize>) -> usize {
        unsafe {
            (*self.inner).spawn_object((*object.inner).clone(), parent_id)
        }
    }

    /// Removes an object and all of its descendants from the world.
    ///
    /// Any integer IDs or [`Object`] references held in JavaScript that
    /// point to the deleted object or its children become dangling after this
    /// call; do not use them for further world queries.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique integer ID of the root object to remove.
    pub fn delete(&mut self, id: usize) {
        unsafe {
            (*self.inner).delete(id);
        }
    }

    /// Moves an object to a new parent in the scene hierarchy.
    ///
    /// Pass `undefined` / `null` as `new_parent_id` to move the object to the
    /// scene root.  The object's children are carried along unchanged.
    ///
    /// Returns `false` and leaves the hierarchy unchanged when any of these
    /// conditions hold:
    /// - `id` does not exist.
    /// - `new_parent_id` does not exist (and is not `null`).
    /// - `new_parent_id` equals `id` (self-parenting).
    /// - `new_parent_id` is already the current parent.
    /// - `new_parent_id` is a descendant of `id` (would create a cycle).
    ///
    /// # Arguments
    ///
    /// * `id`            - Integer ID of the object to move.
    /// * `new_parent_id` - ID of the new parent, or `undefined` / `null` for root.
    ///
    /// # Returns
    ///
    /// `true` if the reparent was applied; `false` if it was rejected.
    pub fn reparent(&mut self, id: usize, new_parent_id: Option<usize>) -> bool {
        unsafe {
            (*self.inner).reparent(id, new_parent_id)
        }
    }

    /// Retrieves a live reference to an object by its integer ID.
    ///
    /// The returned [`Object`] is **owned by the world** — do not manually
    /// destroy it on the JS side, and do not retain it across calls to
    /// [`World::delete`] with the same ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique integer ID of the object to fetch.
    ///
    /// # Returns
    ///
    /// The object, or `undefined` when no object with that ID exists in the world.
    pub fn get_object(&self, id: usize) -> Option<Object> {
        unsafe {
            (*self.inner).objects.get_mut(&id).map(|obj| Object {
                inner: obj as *mut vertra::objects::Object,
                owned: false,
            })
        }
    }

    /// Resolves a stable string identifier (`str_id`) to its integer ID.
    ///
    /// This is an O(1) hash-map lookup but still incurs string-hashing overhead.
    ///
    /// > **Performance note:** do not call this inside `on_update` or other
    /// > high-frequency loops.  Call it once during `on_startup`, store the
    /// > resulting integer ID, and use that directly during updates.
    ///
    /// # Arguments
    ///
    /// * `str_id` - The string handle assigned at object creation (e.g. `"player"`).
    ///
    /// # Returns
    ///
    /// The integer ID, or `undefined` when no object with that `str_id` exists.
    pub fn get_id(&self, str_id: &str) -> Option<usize> {
        unsafe {
            (*self.inner).get_id(str_id)
        }
    }

    /// Returns the integer IDs of all root-level objects (objects with no parent).
    ///
    /// # Returns
    ///
    /// An array of integer IDs, one for each root object.
    pub fn get_roots(&self) -> Vec<usize> {
        unsafe {
            (*self.inner).roots.clone()
        }
    }

    /// Renames the stable string identifier of a live world object and keeps
    /// the internal name-handle cache in sync.
    ///
    /// Prefer this over writing to `object.str_id` directly when the object is
    /// already part of the world, as the world maintains an internal
    /// `str_id → integer id` lookup table that must stay consistent.
    ///
    /// # Arguments
    ///
    /// * `id`         - Integer ID of the object to rename.
    /// * `new_str_id` - The replacement string identifier (should be unique
    ///   within this world).
    ///
    /// # Returns
    ///
    /// `true` if the rename succeeded; `false` when `id` does not exist.
    pub fn rename_str_id(&mut self, id: usize, new_str_id: String) -> bool {
        unsafe {
            (*self.inner).rename_str_id(id, new_str_id)
        }
    }
}