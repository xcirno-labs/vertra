use wasm_bindgen::prelude::*;
use vertra::scene::Scene as CoreScene;
use vertra::editor::EditorEvent;
use std::io::Cursor;
use crate::objects::Object;
use crate::world::World;
use crate::camera::Camera;
use crate::editor::JsInspectorData;
use serde::Deserialize;

#[derive(Deserialize)]
struct JsEditorEvent {
    #[serde(rename = "type")]
    kind: String,
    // mouse_motion
    dx: Option<f32>,
    dy: Option<f32>,
    // cursor_moved
    x: Option<f32>,
    y: Option<f32>,
    // mouse_button
    left:   Option<bool>,
    middle: Option<bool>,
    right: Option<bool>,
    // scroll
    delta: Option<f32>,
    // modifiers
    alt: Option<bool>,
}

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

    // Editor mode
    /// Activate static editor mode.
    ///
    /// Spawns the XYZ axis gizmos and enables orbit/pan/zoom camera controls
    /// and object picking.  Call once from `on_startup`.
    pub fn enable_editor_mode(&mut self) {
        unsafe { (*self.inner).enable_editor_mode(); }
    }

    /// Returns `true` when editor mode is active.
    pub fn is_editor_mode(&self) -> bool {
        unsafe { (*self.inner).editor.is_some() }
    }

    /// Returns the currently-selected object's properties as a JS object
    /// (`InspectorData`), or `undefined` if nothing is selected.
    pub fn inspector(&self) -> JsValue {
        unsafe {
            match (*self.inner).inspector() {
                Some(data) => {
                    let js = JsInspectorData::from(data);
                    serde_wasm_bindgen::to_value(&js).unwrap_or(JsValue::UNDEFINED)
                }
                None => JsValue::UNDEFINED,
            }
        }
    }

    /// Clear the inspector selection programmatically.
    pub fn clear_inspector(&mut self) {
        unsafe {
            if let Some(ed) = &mut (*self.inner).editor {
                ed.inspector.clear();
            }
        }
    }

    /// Manually set the orbit pivot point in world space.
    /// @param {number} x
    /// @param {number} y
    /// @param {number} z
    pub fn set_pivot(&mut self, x: f32, y: f32, z: f32) {
        unsafe {
            if let Some(ed) = &mut (*self.inner).editor {
                ed.pivot = [x, y, z];
            }
        }
    }

    /// Returns the current orbit pivot as `[x, y, z]`, or `undefined` when
    /// editor mode is inactive.
    pub fn get_pivot(&self) -> JsValue {
        unsafe {
            match &(*self.inner).editor {
                Some(ed) => {
                    serde_wasm_bindgen::to_value(&ed.pivot).unwrap_or(JsValue::UNDEFINED)
                }
                None => JsValue::UNDEFINED,
            }
        }
    }

    /// Dispatch a platform-agnostic editor event from JavaScript.
    ///
    /// Use this when browser pointer-lock / raw events need to be forwarded
    /// manually (e.g., from a `pointermove` handler outside the canvas).
    ///
    /// The `payload` must match the `EditorEventPayload` TypeScript type:
    /// ```ts
    /// scene.editor_event({ type: "mouse_motion", dx: 3.0, dy: -1.5 });
    /// scene.editor_event({ type: "scroll",       delta: 1.0 });
    /// scene.editor_event({ type: "modifiers",    alt: true  });
    /// ```
    pub fn editor_event(&mut self, payload: JsValue) -> Result<(), JsValue> {
        let ev: JsEditorEvent = serde_wasm_bindgen::from_value(payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let editor_ev = match ev.kind.as_str() {
            "mouse_motion" => EditorEvent::MouseMotionDelta {
                dx: ev.dx.unwrap_or(0.0),
                dy: ev.dy.unwrap_or(0.0),
            },
            "cursor_moved" => EditorEvent::CursorMoved {
                x: ev.x.unwrap_or(0.0),
                y: ev.y.unwrap_or(0.0),
            },
            "mouse_button" => EditorEvent::MouseButton {
                left:   ev.left,
                middle: ev.middle,
                right: ev.right
            },
            "scroll" => EditorEvent::Scroll {
                delta: ev.delta.unwrap_or(0.0),
            },
            "modifiers" => EditorEvent::ModifiersChanged {
                alt: ev.alt.unwrap_or(false),
            },
            "focus_key" => EditorEvent::FocusKey,
            other => return Err(JsValue::from_str(&format!("Unknown editor event type: {other}"))),
        };

        unsafe { (*self.inner).handle_editor_event(editor_ev); }
        Ok(())
    }

    // VTR I/O 
    /// Exports the scene as a VTR binary buffer.
    /// @returns {Uint8Array} The binary data of the scene.
    pub fn save_vtr(&self) -> Result<Vec<u8>, JsValue> {
        unsafe {
            let mut buf = Vec::new();
            vertra::vtr::write(&mut buf, &(*self.inner).camera, &(*self.inner).world)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(buf)
        }
    }

    /// Loads a VTR scene from a binary buffer.
    /// @param {Uint8Array} data - The VTR binary data.
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