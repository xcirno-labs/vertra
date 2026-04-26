use wasm_bindgen::prelude::*;
use js_sys::Function;
use serde::Serialize;
use crate::objects::Object;
use vertra::world::{World as CoreWorld, SceneGraphEvent, SceneGraphCallback};

thread_local! {
    static SCENE_GRAPH_CB: std::cell::RefCell<Option<Function>> =
        std::cell::RefCell::new(None);
    /// Pending events accumulated during a world-mutation call.
    /// They are drained and dispatched to JS only after the `*mut CoreWorld`
    /// borrow has been fully released, preventing JS re-entrancy from aliasing
    /// the same pointer as a second `&mut`.
    static SCENE_GRAPH_QUEUE: std::cell::RefCell<Vec<SceneGraphModifiedEvent>> =
        std::cell::RefCell::new(Vec::new());
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
    // Push into the queue; the caller is responsible for draining it once the
    // mutable world borrow has been released (see `drain_scene_graph_events`).
    SCENE_GRAPH_QUEUE.with(|q| q.borrow_mut().push(ev));
}

/// Drain the pending scene-graph event queue and dispatch each event to JS.
///
/// Must be called **after** every world-mutating binder call (`spawn_object`,
/// `delete`, `reparent`) so that the `*mut CoreWorld` raw pointer is no longer
/// borrowed when JS receives the callback and can potentially re-enter WASM.
pub(crate) fn drain_scene_graph_events() {
    let events: Vec<SceneGraphModifiedEvent> =
        SCENE_GRAPH_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));

    if events.is_empty() { return; }

    SCENE_GRAPH_CB.with(|c| {
        if let Some(cb) = c.borrow().as_ref() {
            for ev in events {
                if let Ok(js) = serde_wasm_bindgen::to_value(&ev) {
                    let _ = cb.call1(&JsValue::UNDEFINED, &js);
                }
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
        let id = unsafe {
            (*self.inner).spawn_object((*object.inner).clone(), parent_id)
        };
        drain_scene_graph_events();
        id
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
        drain_scene_graph_events();
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
        let result = unsafe {
            (*self.inner).reparent(id, new_parent_id)
        };
        drain_scene_graph_events();
        result
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

    /// Registers a callback fired whenever the scene graph changes structurally
    /// (object added, deleted, or re-parented).
    ///
    /// This installs the internal Rust hook on the underlying world **and**
    /// stores the JS handler — both steps happen in a single call, so you can
    /// wire it up directly from `on_startup` without touching `WebWindow`:
    ///
    /// ```js
    /// window.on_startup((state, scene) => {
    ///   scene.world.on_scene_graph_modified(ev => console.log(ev));
    /// });
    /// ```
    ///
    /// The event object is a tagged union:
    /// - `{ type: "object_added",      data: { id, parent_id } }`
    /// - `{ type: "object_deleted",    data: { id } }`
    /// - `{ type: "object_reparented", data: { id, old_parent, new_parent } }`
    ///
    /// Pass `undefined` / `null` to unregister a previously set callback.
    ///
    /// Callback signature: `(event: SceneGraphModifiedEvent) => void`
    pub fn on_scene_graph_modified(&mut self, f: Option<Function>) {
        register_scene_graph_cb(f);
        // Install the Rust -> JS bridge on the core world if not already present.
        // Calling this more than once is harmless; it simply replaces the
        // existing callback closure with an identical one.
        unsafe { attach_scene_graph_cb(&mut *self.inner); }
    }
}