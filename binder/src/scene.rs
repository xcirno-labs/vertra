use wasm_bindgen::prelude::*;
use crate::objects::Object;
use crate::world::World;
use vertra::scene::{Scene as CoreScene};
use crate::camera::Camera;

/// The root container for a 3D environment.
/// Manages the object lifecycle, scene hierarchy, and the active viewport camera.
#[wasm_bindgen]
pub struct Scene {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreScene,
}

#[wasm_bindgen]
impl Scene {
    /// Spawns a new object into the scene.
    ///
    /// @param {VertraObject} object - The object template to add to the scene.
    /// @param {number | null} [parent_id] - The ID of the parent object. If null, it is added to the scene root.
    /// @returns {number} The unique ID assigned to this object instance within the scene.
    pub fn spawn(&mut self, object: &Object, parent_id: Option<usize>) -> usize {
        // We clone the inner object to move it into the world
        unsafe {
            (*self.inner).spawn((*object.inner).clone(), parent_id)
        }
    }

    /// Accesses the underlying World data structure.
    /// Use this to query entities or batch-update transforms.
    #[wasm_bindgen(getter)]
    pub fn world(&self) -> World {
        unsafe {
            World {
                inner: &mut (*self.inner).world as *mut vertra::world::World
            }
        }
    }

    /// Returns the primary camera used to render this scene.
    /// Note: This camera is owned by the Scene; do not attempt to manually destroy it.
    #[wasm_bindgen(getter)]
    pub fn camera(&self) -> Camera {
        unsafe {
            Camera {
                inner: &mut (*self.inner).camera as *mut vertra::camera::Camera,
                owned: false,
            }
        }
    }
}