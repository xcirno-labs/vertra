use wasm_bindgen::prelude::*;
use crate::objects::Object;
use vertra::world::{World as CoreWorld};

/// The entity management system and scene hierarchy container.
/// Handles the creation, destruction, and retrieval of scene objects.
#[wasm_bindgen]
pub struct World {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreWorld,
}

#[wasm_bindgen]
impl World {
    /// Spawns an object. parent_id can be passed as undefined/null from JS.
    pub fn spawn_object(&mut self, object: &Object, parent_id: Option<usize>) -> usize {
        unsafe {
            (*self.inner).spawn_object((*object.inner).clone(), parent_id)
        }
    }

    /// Creates a new object instance in the world.
    ///
    /// @param {VertraObject} object - The template object to clone into the world.
    /// @param {number | null} [parent_id] - The ID of an existing object to act as the parent.
    /// If null, the object is added to the scene root.
    /// @returns {number} The unique ID assigned to the new instance.
    pub fn delete(&mut self, id: usize) {
        unsafe {
            (*self.inner).delete(id);
        }
    }

    /// Retrieves an existing object from the world by its ID.
    ///
    /// @param {number} id - The unique ID of the object.
    /// @returns {VertraObject | undefined} A reference to the object, or undefined if the ID is invalid.
    /// @note This returns a reference owned by the World. Do not manually destroy this object in JS.
    pub fn get_object(&self, id: usize) -> Option<Object> {
        unsafe {
            (*self.inner).objects.get_mut(&id).map(|obj| Object {
                inner: obj as *mut vertra::objects::Object,
                owned: false,
            })
        }
    }

    /// Returns a list of IDs for objects that have no parent (the root nodes).
    /// @returns {Uint32Array | number[]} An array of object IDs.
    pub fn get_roots(&self) -> Vec<usize> {
        unsafe {
            (*self.inner).roots.clone()
        }
    }
}