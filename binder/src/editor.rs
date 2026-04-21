use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use vertra::editor::InspectorData as CoreInspectorData;
use vertra::editor::EditorEvent;
use vertra::scene::Scene as CoreScene;
use std::cell::RefCell;

thread_local! {
    static EDITOR_EVENT_CB: RefCell<Option<js_sys::Function>> = RefCell::new(None);
}
pub(crate) fn register_editor_event_cb(f: Option<js_sys::Function>) {
    EDITOR_EVENT_CB.with(|cb| *cb.borrow_mut() = f);
}
pub(crate) fn fire_selection_changed(scene: *mut CoreScene) {
    EDITOR_EVENT_CB.with(|cb| {
        if let Some(f) = cb.borrow().as_ref() {
            let data = unsafe {
                (*scene).inspector().map(|d| JsInspectorData::from(d))
            };
            let ev = WebEditorEvent::SelectionChanged { data };
            if let Ok(js) = serde_wasm_bindgen::to_value(&ev) {
                let _ = f.call1(&JsValue::UNDEFINED, &js);
            }
        }
    });
}

#[wasm_bindgen(typescript_custom_section)]
const TS_INSPECTOR: &'static str = r#"
export interface InspectorData {
    id: number;
    name: string;
    str_id: string;
    position: [number, number, number];
    rotation_deg: [number, number, number];
    scale: [number, number, number];
    color: [number, number, number, number];
    geometry_type: string | null;
}
export type EditorEventPayload =
    | { type: "mouse_motion";   dx: number; dy: number }
    | { type: "cursor_moved";   x: number;  y: number  }
    | { type: "mouse_button";   left?: boolean; middle?: boolean; right?: boolean }
    | { type: "scroll";         delta: number }
    | { type: "modifiers";      alt: boolean; ctrl: boolean }
    | { type: "focus_key" }
    | { type: "key_pressed";    code: string }
    | { type: "key_released";   code: string };
export type EditorEventType =
    | { type: "gizmo_mode_changed"; mode: string }
    | { type: "drag_start";         axis: string }
    | { type: "drag_end" }
    | { type: "selection_changed";  data: InspectorData | null };
export type EngineMode = "editor" | "play";
"#;
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
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum WebEditorEvent {
    #[serde(rename = "gizmo_mode_changed")]
    GizmoModeChanged { mode: String },
    #[serde(rename = "drag_start")]
    DragStart { axis: String },
    #[serde(rename = "drag_end")]
    DragEnd,
    #[serde(rename = "selection_changed")]
    SelectionChanged { data: Option<JsInspectorData> },
}
#[derive(Deserialize)]
pub(crate) struct JsEditorEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub dx: Option<f32>,
    pub dy: Option<f32>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub left:   Option<bool>,
    pub middle: Option<bool>,
    pub right:  Option<bool>,
    pub delta: Option<f32>,
    pub alt:  Option<bool>,
    pub ctrl: Option<bool>,
    pub code: Option<String>,
}
pub(crate) fn parse_key_code(s: &str) -> Option<winit::keyboard::KeyCode> {
    use winit::keyboard::KeyCode::*;
    Some(match s {
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
        _ => return None,
    })
}
#[wasm_bindgen]
pub struct Editor {
    #[wasm_bindgen(skip)]
    pub scene: *mut CoreScene,
}
#[wasm_bindgen]
impl Editor {
    pub fn is_editor_mode(&self) -> bool {
        unsafe { (*self.scene).editor.is_some() }
    }
    pub fn is_play_mode(&self) -> bool {
        unsafe { (*self.scene).editor.is_none() }
    }
    pub fn mode(&self) -> String {
        if unsafe { (*self.scene).editor.is_some() } {
            "editor".to_string()
        } else {
            "play".to_string()
        }
    }
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
    pub fn clear_inspector(&mut self) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.inspector.clear();
            }
        }
        fire_selection_changed(self.scene);
    }
    pub fn set_pivot(&mut self, x: f32, y: f32, z: f32) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.pivot = [x, y, z];
            }
        }
    }
    pub fn get_pivot(&self) -> JsValue {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => serde_wasm_bindgen::to_value(&ed.pivot).unwrap_or(JsValue::UNDEFINED),
                None => JsValue::UNDEFINED,
            }
        }
    }
    pub fn multi_selected_ids(&self) -> Vec<usize> {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => ed.multi_selected.clone(),
                None     => Vec::new(),
            }
        }
    }
    pub fn is_multi_selected(&self, id: usize) -> bool {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => ed.multi_selected.contains(&id),
                None     => false,
            }
        }
    }
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
        fire_selection_changed(self.scene);
    }
    pub fn clear_selection(&mut self) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.inspector.clear();
                ed.multi_selected.clear();
                ed.group_ids.clear();
            }
        }
        fire_selection_changed(self.scene);
    }
    pub fn group_ids(&self) -> Vec<usize> {
        unsafe {
            match &(*self.scene).editor {
                Some(ed) => ed.group_ids.clone(),
                None     => Vec::new(),
            }
        }
    }
    pub fn set_camera_speed(&mut self, speed: f32) {
        unsafe {
            if let Some(ed) = &mut (*self.scene).editor {
                ed.camera_speed = speed;
            }
        }
    }
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
                match parse_key_code(ev.code.as_deref().unwrap_or("")) {
                    Some(code) => EditorEvent::KeyPressed(code),
                    None => return Ok(()),
                }
            }
            "key_released" => {
                match parse_key_code(ev.code.as_deref().unwrap_or("")) {
                    Some(code) => EditorEvent::KeyReleased(code),
                    None => return Ok(()),
                }
            }
            other => return Err(JsValue::from_str(&format!("Unknown editor event type: {other}"))),
        };
        unsafe { (*self.scene).handle_editor_event(editor_ev); }
        Ok(())
    }
}
