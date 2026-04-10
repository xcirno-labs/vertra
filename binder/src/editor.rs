use wasm_bindgen::prelude::*;
use serde::Serialize;
use vertra::editor::InspectorData as CoreInspectorData;

#[wasm_bindgen(typescript_custom_section)]
const TS_INSPECTOR: &'static str = r#"
/**
 * Snapshot of a selected scene object returned by `scene.inspector`.
 * All rotation values are in **degrees**.
 */
export interface InspectorData {
    /** Integer scene-graph ID (used for direct object manipulation). */
    id: number;
    /** Display name. */
    name: string;
    /** Stable string identifier. */
    str_id: string;
    /** World-space position [x, y, z]. */
    position: [number, number, number];
    /** Euler rotation in degrees [rx, ry, rz]. */
    rotation_deg: [number, number, number];
    /** Scale factors [sx, sy, sz]. */
    scale: [number, number, number];
    /** RGBA colour [r, g, b, a]. */
    color: [number, number, number, number];
    /** Geometry variant name, e.g. "Sphere", or null if no geometry. */
    geometry_type: string | null;
}

/**
 * Platform-agnostic editor input event.
 * Pass to `scene.editor_event(…)` from browser event listeners when
 * the engine canvas does not receive raw pointer events automatically.
 */
export type EditorEventPayload =
    | { type: "mouse_motion";   dx: number; dy: number }
    | { type: "cursor_moved";   x: number;  y: number  }
    | { type: "mouse_button";   left?: boolean; middle?: boolean }
    | { type: "scroll";         delta: number }
    | { type: "modifiers";      alt: boolean }
    | { type: "focus_key" };
"#;

/// Serializable mirror of [`vertra::editor::InspectorData`], exposed to JS
/// via `serde_wasm_bindgen::to_value`.
#[derive(Serialize)]
pub struct JsInspectorData {
    pub id:           usize,
    pub name:         String,
    pub str_id:       String,
    pub position:     [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale:        [f32; 3],
    pub color:        [f32; 4],
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
