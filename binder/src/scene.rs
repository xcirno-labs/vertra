use wasm_bindgen::prelude::*;
use vertra::scene::Scene as CoreScene;
use std::io::Cursor;
use crate::objects::Object;
use crate::world::World;
use crate::camera::Camera;
use crate::editor::Editor;

/// The root container for a 3D environment.
///
/// Manages the object lifecycle, scene hierarchy, GPU pipeline, and the active
/// viewport camera.  One `Scene` exists per [`WebWindow`] and is passed to
/// every callback (`on_startup`, `on_update`, `on_draw_request`).
#[wasm_bindgen]
pub struct Scene {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreScene,
}

#[wasm_bindgen]
impl Scene {
    /// Spawns a new object into the scene hierarchy.
    ///
    /// # Arguments
    ///
    /// * `object`    - The object template to clone into the scene.
    /// * `parent_id` - ID of an existing object to attach this object to as a
    ///   child.  Pass `undefined` / `null` to add the object at the scene root.
    ///
    /// # Returns
    ///
    /// The unique integer ID assigned to the new object instance.
    pub fn spawn(&mut self, object: &Object, parent_id: Option<usize>) -> usize {
        unsafe {
            (*self.inner).spawn((*object.inner).clone(), parent_id)
        }
    }

    /// Returns a handle to the underlying [`World`] data structure.
    ///
    /// Use this to query entities, batch-update transforms, or delete objects.
    #[wasm_bindgen(getter)]
    pub fn world(&self) -> World {
        unsafe {
            World {
                inner: &mut (*self.inner).world as *mut vertra::world::World
            }
        }
    }

    /// Returns the primary camera used to render this scene.
    ///
    /// The camera is owned by the scene; do not attempt to manually destroy it
    /// on the JavaScript side.
    #[wasm_bindgen(getter)]
    pub fn camera(&self) -> Camera {
        unsafe {
            Camera {
                inner: &mut (*self.inner).camera as *mut vertra::camera::Camera,
                owned: false,
            }
        }
    }

    /// Returns a handle to the editor subsystem.
    ///
    /// Use this to query and mutate selection state, dispatch input events,
    /// check the current engine mode, and so on.  All mutating methods are
    /// no-ops when editor mode is not active.
    #[wasm_bindgen(getter)]
    pub fn editor(&self) -> Editor {
        unsafe {
            Editor { scene: self.inner }
        }
    }

    // ── Engine mode ───────────────────────────────────────────────────────────

    /// Returns `true` when the scene is currently in **editor mode**, `false`
    /// when in play mode.
    ///
    /// Shorthand for `scene.editor.is_editor_mode()`.
    pub fn is_editor_mode(&self) -> bool {
        unsafe { (*self.inner).editor.is_some() }
    }

    /// Activates static editor mode.
    ///
    /// Spawns the XYZ axis gizmos and enables orbit / pan / zoom camera
    /// controls and object picking.  Call once from `on_startup`.
    pub fn enable_editor_mode(&mut self) {
        unsafe { (*self.inner).enable_editor_mode(); }
    }

    /// Exits editor mode and switches to **play mode**.
    ///
    /// Drops all editor state (selection, gizmos, skybox, orbit pivot).
    /// After this call [`Scene::is_editor_mode`] returns `false` and all
    /// `with_event_handler` callbacks start receiving raw input events again.
    ///
    /// > **Keybind:** pressing `Escape` while in editor mode calls this
    /// > automatically and also fires the [`WebWindow::on_play`] callback.
    pub fn disable_editor_mode(&mut self) {
        unsafe { (*self.inner).disable_editor_mode(); }
    }
    
    /// Exports the entire scene (camera + world) as a VTR binary buffer.
    ///
    /// The buffer can be stored, transferred, and later reloaded with
    /// [`Scene::load_vtr`].
    ///
    /// # Returns
    ///
    /// A `Uint8Array` containing the serialised scene data.
    ///
    /// # Errors
    ///
    /// Returns a [`JsValue`] error string on serialisation failure.
    pub fn save_vtr(&self) -> Result<Vec<u8>, JsValue> {
        unsafe {
            let mut buf = Vec::new();
            vertra::vtr::write(&mut buf, &(*self.inner).camera, &(*self.inner).world)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(buf)
        }
    }

    /// Replaces the current camera and world state from a VTR binary buffer.
    ///
    /// The GPU pipeline is unaffected — only the logical scene state (camera,
    /// objects, hierarchy) is replaced.
    ///
    /// # Arguments
    ///
    /// * `data` - A `Uint8Array` previously produced by [`Scene::save_vtr`].
    ///
    /// # Errors
    ///
    /// Returns a [`JsValue`] error string when the data is corrupt, truncated,
    /// or written by an incompatible format version.
    pub fn load_vtr(&mut self, data: &[u8]) -> Result<(), JsValue> {
        unsafe {
            let mut cur = Cursor::new(data);
            let scene_data = vertra::vtr::read(&mut cur)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            (*self.inner).camera = scene_data.camera;
            (*self.inner).world  = scene_data.world;
            Ok(())
        }
    }
}