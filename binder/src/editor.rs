use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use vertra::editor::InspectorData as CoreInspectorData;
use vertra::editor::EditorEvent;
use vertra::scene::Scene as CoreScene;

#[wasm_bindgen(typescript_custom_section)]
const TS_INSPECTOR: &'static str = r#"
/**
 * Snapshot of a selected scene object returned by `scene.editor.inspector`.
 * All rotation values are in **degrees**.
 */
export interface InspectorData {
    /** Integer scene-graph ID (used for direct object manipulation). */
    id: number;
    /** Human-readable display name. */
    name: string;
    /** Stable string identifier assigned at creation time. */
    str_id: string;
    /** World-space position as `[x, y, z]`. */
    position: [number, number, number];
    /** Euler rotation in degrees as `[rx, ry, rz]`. */
    rotation_deg: [number, number, number];
    /** Per-axis scale factors as `[sx, sy, sz]`. */
    scale: [number, number, number];
    /** RGBA colour as `[r, g, b, a]`. */
    color: [number, number, number, number];
    /** Geometry variant name (e.g. `"Sphere"`), or `null` when no geometry is attached. */
    geometry_type: string | null;
}

/**
 * Platform-agnostic input event forwarded to the editor subsystem.
 *
 * Pass one of these objects to `scene.editor.editor_event(payload)` from your
 * browser event listeners when you need to drive the editor from custom input code.
 *
 * ### Default keybinds (active in editor mode)
 *
 * | Key        | Action                                        |
 * |------------|-----------------------------------------------|
 * | `Escape`   | Exit editor → enter **play mode**             |
 * | `W/A/S/D`  | Fly the editor camera                         |
 * | `Shift`    | 3× camera-speed boost while held              |
 * | `F`        | Focus camera on the current selection         |
 * | `G`        | Expand selection to all descendants           |
 * | `T`        | Switch gizmo to **Translate** mode            |
 * | `R`        | Switch gizmo to **Rotate** mode               |
 * | `E`        | Switch gizmo to **Scale** mode                |
 * | `Alt+drag` | Free-look orbit around the pivot point        |
 * | `Mid-drag` | Pan the camera                                |
 * | `Scroll`   | Zoom towards / away from the pivot            |
 *
 * @example
 * ```ts
 * // Free-look rotation (Alt+drag equivalent)
 * canvas.addEventListener('mousemove', e => {
 *   scene.editor.editor_event({ type: "mouse_motion", dx: e.movementX, dy: e.movementY });
 * });
 *
 * // Scroll zoom
 * canvas.addEventListener('wheel', e => {
 *   scene.editor.editor_event({ type: "scroll", delta: -e.deltaY * 0.01 });
 * });
 *
 * // Modifier keys — call on keydown AND keyup so Alt/Ctrl state stays correct
 * window.addEventListener('keydown', e => {
 *   scene.editor.editor_event({ type: "modifiers", alt: e.altKey, ctrl: e.ctrlKey });
 * });
 *
 * // WASD movement and gizmo shortcuts
 * window.addEventListener('keydown', e => {
 *   scene.editor.editor_event({ type: "key_pressed", code: e.code });
 * });
 * window.addEventListener('keyup', e => {
 *   scene.editor.editor_event({ type: "key_released", code: e.code });
 * });
 * ```
 */
export type EditorEventPayload =
    | { type: "mouse_motion";   dx: number; dy: number }
    | { type: "cursor_moved";   x: number;  y: number  }
    | { type: "mouse_button";   left?: boolean; middle?: boolean; right?: boolean }
    | { type: "scroll";         delta: number }
    | { type: "modifiers";      alt: boolean; ctrl: boolean }
    | { type: "focus_key" }
    | { type: "key_pressed";    code: string }
    | { type: "key_released";   code: string };

/**
 * The engine's current operating mode.
 *
 * - `"editor"` — orbit / pick / gizmo controls active; client
 *   `with_event_handler` callbacks are suppressed.
 * - `"play"`   — editor is inactive; all `with_event_handler` callbacks fire.
 */
export type EngineMode = "editor" | "play";
"#;

/// Serialisable mirror of [`vertra::editor::InspectorData`], exposed to JS
/// via `serde_wasm_bindgen::to_value`.
#[derive(Serialize)]
pub struct JsInspectorData {
    pub id:            usize,
    pub name:          String,
    pub str_id:        String,
    pub position:      [f32; 3],
    pub rotation_deg:  [f32; 3],
    pub scale:         [f32; 3],
    pub color:         [f32; 4],
    pub geometry_type: Option<String>,
}

impl From<&CoreInspectorData> for JsInspectorData {
    fn from(d: &CoreInspectorData) -> Self {
        Self {
            id:            d.id,
            name:          d.name.clone(),
            str_id:        d.str_id.clone(),
            position:      d.position,
            rotation_deg:  d.rotation_deg,
            scale:         d.scale,
            color:         d.color,
            geometry_type: d.geometry_type.clone(),
        }
    }
}

/// Mirrors [`EditorEventType`] TypeScript union; serialised with
/// `serde_wasm_bindgen::to_value` and passed to the `on_editor_event` callback.
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum WebEditorEvent {
    /// Fires when the active gizmo mode changes (T / R / E keys).
    #[serde(rename = "gizmo_mode_changed")]
    GizmoModeChanged {
        /// `"translate"`, `"rotate"`, or `"scale"`.
        mode: String,
    },
    /// Fires when the user starts dragging a gizmo axis handle.
    #[serde(rename = "drag_start")]
    DragStart {
        /// The axis being dragged: `"x"`, `"y"`, or `"z"`.
        axis: String,
    },
    /// Fires when the user releases a gizmo axis drag.
    #[serde(rename = "drag_end")]
    DragEnd,
}

#[derive(Deserialize)]
pub(crate) struct JsEditorEvent {
    #[serde(rename = "type")]
    pub kind: String,
    // mouse_motion
    pub dx: Option<f32>,
    pub dy: Option<f32>,
    // cursor_moved
    pub x: Option<f32>,
    pub y: Option<f32>,
    // mouse_button
    pub left:   Option<bool>,
    pub middle: Option<bool>,
    pub right:  Option<bool>,
    // scroll
    pub delta: Option<f32>,
    // modifiers
    pub alt:  Option<bool>,
    pub ctrl: Option<bool>,
    // key_pressed / key_released  (winit KeyCode debug string, e.g. "KeyW")
    pub code: Option<String>,
}

/// Parse a winit `KeyCode` from its `Debug` string representation
/// (e.g. `"KeyW"` → `KeyCode::KeyW`).
pub(crate) fn parse_key_code(s: &str) -> Result<winit::keyboard::KeyCode, JsValue> {
    use winit::keyboard::KeyCode::*;
    Ok(match s {
        "KeyA" => KeyA, "KeyB" => KeyB, "KeyC" => KeyC, "KeyD" => KeyD,
        "KeyE" => KeyE, "KeyF" => KeyF, "KeyG" => KeyG, "KeyH" => KeyH,
        "KeyI" => KeyI, "KeyJ" => KeyJ, "KeyK" => KeyK, "KeyL" => KeyL,
        "KeyM" => KeyM, "KeyN" => KeyN, "KeyO" => KeyO, "KeyP" => KeyP,
        "KeyQ" => KeyQ, "KeyR" => KeyR, "KeyS" => KeyS, "KeyT" => KeyT,
        "KeyU" => KeyU, "KeyV" => KeyV, "KeyW" => KeyW, "KeyX" => KeyX,
        "KeyY" => KeyY, "KeyZ" => KeyZ,
        "ShiftLeft"   => ShiftLeft,   "ShiftRight"   => ShiftRight,
        "ControlLeft" => ControlLeft, "ControlRight" => ControlRight,
        "AltLeft"     => AltLeft,     "AltRight"     => AltRight,
        "Space"       => Space,       "Escape"       => Escape,
        "ArrowUp"     => ArrowUp,     "ArrowDown"    => ArrowDown,
        "ArrowLeft"   => ArrowLeft,   "ArrowRight"   => ArrowRight,
        other => return Err(JsValue::from_str(&format!("Unknown KeyCode: {other}"))),
    })
}

/// Handle to the editor subsystem for the active [`Scene`].
///
/// Provides access to selection state, inspector data, orbit pivot, and
/// input-event forwarding.  Obtain it via [`Scene::editor`].
///
/// All mutating methods are **no-ops** when editor mode is inactive (play mode).
#[wasm_bindgen]
pub struct Editor {
    #[wasm_bindgen(skip)]
    pub scene: *mut CoreScene,
}

#[wasm_bindgen]
impl Editor {
    /// Returns `true` when the engine is currently in **editor mode**.
    pub fn is_editor_mode(&self) -> bool {
        unsafe { (*self.scene).editor.is_some() }
    }

    /// Returns `true` when the engine is in **play mode** (editor inactive).
    ///
    /// Equivalent to `!scene.editor.is_editor_mode()`.  Useful as a guard in
    /// `on_update` to skip game logic while the editor is open:
    ///
    /// ```ts
    /// window.on_update((state, scene) => {
    ///   if (!scene.editor.is_play_mode()) return;
    ///   // game logic here ...
    /// });
    /// ```
    pub fn is_play_mode(&self) -> bool {
        unsafe { (*self.scene).editor.is_none() }
    }

    /// Returns the current engine mode as a string.
    ///
    /// # Returns
    ///
    /// `"editor"` when editor mode is active, `"play"` otherwise.
    /// Matches the [`EngineMode`] TypeScript union type.
    pub fn mode(&self) -> String {
        if unsafe { (*self.scene).editor.is_some() } {
            "editor".to_string()
        } else {
            "play".to_string()
        }
    }

    /// Returns the currently-selected object's properties, or `undefined` if
    /// nothing is selected or editor mode is inactive.
    ///
    /// # Returns
    ///
    /// An [`InspectorData`] object, or `undefined`.
    pub fn inspector(&self) -> JsValue {
        unsafe {
            match (*self.scene).inspector() {
                Some(data) => {
                    let js = JsInspectorData::from(data);
                    serde_wasm_bindgen::to_value(&js).unwrap_or(JsValue::UNDEFINED)
                }
                None => JsValue::UNDEFINED,
            }
        }
    }

    /// Clears the inspector selection programmatically.
    ///
    /// No-op when editor mode is inactive.
    pub fn clear_inspector(&mut self) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.inspector.clear();
            }
        }
    }

    /// Sets the orbit pivot point used for camera zoom and Alt+drag rotation.
    ///
    /// No-op when editor mode is inactive.
    ///
    /// # Arguments
    ///
    /// * `x`, `y`, `z` - World-space coordinates of the new pivot.
    pub fn set_pivot(&mut self, x: f32, y: f32, z: f32) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.pivot = [x, y, z];
            }
        }
    }

    /// Returns the current orbit pivot as `[x, y, z]`, or `undefined` when
    /// editor mode is inactive.
    pub fn get_pivot(&self) -> JsValue {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => {
                    serde_wasm_bindgen::to_value(&ed.pivot).unwrap_or(JsValue::UNDEFINED)
                }
                None => JsValue::UNDEFINED,
            }
        }
    }

    /// Returns the integer IDs of all currently multi-selected objects.
    ///
    /// Returns an empty array when nothing is selected or editor mode is
    /// inactive.
    ///
    /// # Examples
    ///
    /// ```ts
    /// const ids: number[] = Array.from(scene.editor.multi_selected_ids());
    /// ```
    pub fn multi_selected_ids(&self) -> Vec<usize> {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => ed.multi_selected.clone(),
                None     => Vec::new(),
            }
        }
    }

    /// Returns `true` if the object with the given `id` is in the current
    /// multi-selection.
    ///
    /// # Arguments
    ///
    /// * `id` - The integer ID of the object to check.
    pub fn is_multi_selected(&self, id: usize) -> bool {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => ed.multi_selected.contains(&id),
                None     => false,
            }
        }
    }

    /// Programmatically sets the multi-selection to the supplied list of IDs.
    ///
    /// The last ID in `ids` becomes the inspector's primary selection.
    /// Passing an empty slice clears the selection entirely.
    ///
    /// No-op when editor mode is inactive.
    ///
    /// # Arguments
    ///
    /// * `ids` - The new set of selected object IDs.
    ///
    /// # Examples
    ///
    /// ```ts
    /// scene.editor.set_multi_selected([sunId, planetId]);
    /// ```
    pub fn set_multi_selected(&mut self, ids: Vec<usize>) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.multi_selected = ids.clone();
                ed.group_ids.clear();
                ed.inspector.selected = ids.last()
                    .and_then(|&id| (*self.scene).world.objects.get(&id)
                        .map(|o| vertra::editor::InspectorData::from_object(id, o)));
            }
        }
    }

    /// Clears the entire selection: inspector, multi-select, and group expansion.
    ///
    /// No-op when editor mode is inactive.
    pub fn clear_selection(&mut self) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.inspector.clear();
                ed.multi_selected.clear();
                ed.group_ids.clear();
            }
        }
    }

    /// Returns the IDs that are part of the current `G`-key group expansion.
    ///
    /// Returns an empty array when no group expansion is active or editor mode
    /// is inactive.
    pub fn group_ids(&self) -> Vec<usize> {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => ed.group_ids.clone(),
                None     => Vec::new(),
            }
        }
    }

    /// Sets the editor camera fly speed in world units per second.
    ///
    /// Default is `5.0`.  No-op when editor mode is inactive.
    ///
    /// # Arguments
    ///
    /// * `speed` - The new camera movement speed (world units / second).
    pub fn set_camera_speed(&mut self, speed: f32) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.camera_speed = speed;
            }
        }
    }

    /// Dispatches a platform-agnostic editor input event from JavaScript.
    ///
    /// The `payload` must conform to the [`EditorEventPayload`] TypeScript union.
    /// This is a no-op when editor mode is inactive.
    ///
    /// # Arguments
    ///
    /// * `payload` - A JS object matching one of the [`EditorEventPayload`] variants.
    ///
    /// # Errors
    ///
    /// Returns a [`JsValue`] error string when `payload` cannot be deserialised
    /// into a known event variant (e.g. unknown `type` field or unrecognised key code).
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
                right:  ev.right,
            },
            "scroll" => EditorEvent::Scroll {
                delta: ev.delta.unwrap_or(0.0),
            },
            "modifiers" => EditorEvent::ModifiersChanged {
                alt:  ev.alt.unwrap_or(false),
                ctrl: ev.ctrl.unwrap_or(false),
            },
            "focus_key" => EditorEvent::FocusKey,
            "key_pressed" => {
                let code = parse_key_code(ev.code.as_deref().unwrap_or(""))?;
                EditorEvent::KeyPressed(code)
            }
            "key_released" => {
                let code = parse_key_code(ev.code.as_deref().unwrap_or(""))?;
                EditorEvent::KeyReleased(code)
            }
            other => return Err(JsValue::from_str(&format!("Unknown editor event type: {other}"))),
        };

        unsafe { (*self.scene).handle_editor_event(editor_ev); }
        Ok(())
    }
}
