use wasm_bindgen::prelude::*;
use crate::objects::Object;
use vertra::world::{World as CoreWorld};

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
    /// Any integer IDs or [`VertraObject`] references held in JavaScript that
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

    /// Retrieves a live reference to an object by its integer ID.
    ///
    /// The returned [`VertraObject`] is **owned by the world** — do not manually
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
}