use wasm_bindgen::prelude::*;
use js_sys::Function;
use crate::objects::Object;
use vertra::world::World as CoreWorld;
use crate::internals::mutation::{
    is_script_borrow_active,
    queue_mutation,
    drain_scene_graph_events,
    Mutation,
};


pub(crate) use crate::internals::mutation::register_scene_graph_cb;
pub(crate) use crate::internals::mutation::attach_scene_graph_cb;

/// The entity management system and scene hierarchy container.
///
/// Handles creation, destruction, and retrieval of scene objects.
/// Obtain the world for the active scene via [`Scene::world`].
///
/// # Script-callback safety
///
/// All mutation methods (`spawn_object`, `delete`, `reparent`,
/// `rename_str_id`) are safe to call from inside an `on_start`,
/// `on_update`, or `on_fixed_update` script callback.  When called during a
/// callback the operation is **silently deferred**: it is placed on an
/// internal queue and replayed against the real world the instant the
/// callback returns.  The JS caller receives the correct return value
/// immediately (e.g. the pre-allocated spawn ID).
#[wasm_bindgen]
pub struct World {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreWorld,
}

#[wasm_bindgen]
impl World {
    /// Spawns an object into the world, optionally as a child of an existing
    /// object.
    ///
    /// When called **inside a script callback** the spawn is deferred until
    /// the callback returns; the returned ID is pre-allocated and will be
    /// valid immediately after the callback.  Sequential deferred spawns
    /// receive sequential IDs.
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
        if is_script_borrow_active() {
            let pre_id   = unsafe { (*self.inner).alloc_id() };
            let core_obj = unsafe { (*object.inner).clone() };
            queue_mutation(Mutation::Spawn { id: pre_id, object: core_obj, parent: parent_id });
            return pre_id;
        }
        let id = unsafe { (*self.inner).spawn_object((*object.inner).clone(), parent_id) };
        drain_scene_graph_events();
        id
    }

    /// Removes an object and all of its descendants from the world.
    ///
    /// When called **inside a script callback** the deletion is deferred until
    /// the callback returns.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique integer ID of the root object to remove.
    pub fn delete(&mut self, id: usize) {
        if is_script_borrow_active() {
            queue_mutation(Mutation::Delete(id));
            return;
        }
        unsafe { (*self.inner).delete(id); }
        drain_scene_graph_events();
    }

    /// Moves an object to a new parent in the scene hierarchy.
    ///
    /// When called **inside a script callback** the reparent is deferred.
    /// `true` is returned optimistically; the actual outcome is determined
    /// when the mutation is flushed after the callback.
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
        if is_script_borrow_active() {
            queue_mutation(Mutation::Reparent { id, new_parent: new_parent_id });
            return true;
        }
        let result = unsafe { (*self.inner).reparent(id, new_parent_id) };
        drain_scene_graph_events();
        result
    }

    /// Retrieves a live reference to an object by its integer ID.
    ///
    /// The returned [`Object`] is **owned by the world**. Do not manually
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
        unsafe { (*self.inner).roots.clone() }
    }

    /// Renames the stable string identifier of a live world object and keeps
    /// the internal name-handle cache in sync.
    ///
    /// Prefer this over writing to `object.str_id` directly when the object is
    /// already part of the world, as the world maintains an internal
    /// `str_id -> integer id` lookup table that must stay consistent.
    ///
    /// When called **inside a script callback** the rename is deferred.
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
        if is_script_borrow_active() {
            queue_mutation(Mutation::Rename { id, new_str_id });
            return true;
        }
        unsafe { (*self.inner).rename_str_id(id, new_str_id) }
    }

    /// Registers a callback fired whenever the scene graph changes structurally
    /// (object added, deleted, or re-parented).
    ///
    /// This installs the internal Rust hook on the underlying world **and**
    /// stores the JS handler, both steps happen in a single call, so you can
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