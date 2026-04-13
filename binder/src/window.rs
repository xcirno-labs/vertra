use wasm_bindgen::prelude::*;
use vertra::window::Window;
use vertra::event::{DeviceEvent, ElementState, PhysicalKey};
use crate::scene::Scene;
use vertra::event::{Event, WindowEvent};
use js_sys::Function;
use serde::Serialize;
use winit::event::{MouseButton, MouseScrollDelta};
use vertra::editor::EditorEvent;
use crate::camera::Camera;
use crate::editor::WebEditorEvent;

#[wasm_bindgen(start)]
pub fn main_js() {
    // This ensures any Rust panic is printed to the JS console
    console_error_panic_hook::set_once();
}

/// Contains information about the current frame.
#[wasm_bindgen]
pub struct FrameContext {
    /// Time elapsed since the last frame in seconds.
    pub dt: f32,
}

/// Represents an input event sent from the engine to the JavaScript handler.
/// Serialized as a tagged union in JS: { type: "keydown", data: { ... } }
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WebEvent {
    #[serde(rename = "keydown")]
    KeyDown { code: String, repeat: bool },
    #[serde(rename = "keyup")]
    KeyUp { code: String },
    #[serde(rename = "mousemove")]
    MouseMove { x: f64, y: f64 },
    #[serde(rename = "mousemotion")]
    MouseMotion { dx: f64, dy: f64 },
    #[serde(rename = "mousedown")]
    MouseDown { button: String },
    #[serde(rename = "mouseup")]
    MouseUp { button: String },
    #[serde(rename = "mousemove")]
    Wheel { delta: f32 }
}

/// The main application controller that manages the canvas and the render loop.
#[wasm_bindgen]
pub struct WebWindow {
    state: JsValue,
    camera: Camera,
    on_update: Option<Function>,
    on_draw_request: Option<Function>,
    on_startup: Option<Function>,
    on_select: Option<Function>,
    on_editor_event: Option<Function>,
    with_event_handler: Option<Function>,
}

#[wasm_bindgen]
impl WebWindow {
    /// Creates a new WebWindow.
    /// @param {Camera} camera - The initial camera for the scene.
    /// @param {any} [state] - Initial state object passed to every callback.
    #[wasm_bindgen(constructor)]
    pub fn new(camera: Camera, state: Option<JsValue>) -> Self {
        console_error_panic_hook::set_once();
        Self {
            state: state.unwrap_or(JsValue::NULL),
            camera,
            on_update: None,
            on_draw_request: None,
            on_startup: None,
            on_select: None,
            on_editor_event: None,
            with_event_handler: None,
        }
    }

    /// Sets the function to call every frame for logic updates.
    /// Callback signature: (state, scene, frameContext) => void
    pub fn on_update(&mut self, f: Function) { self.on_update = Some(f); }

    /// Sets the function to call once before the first frame.
    /// Callback signature: (state, scene, frameContext) => void
    pub fn on_startup(&mut self, f: Function) { self.on_startup = Some(f); }

    /// Sets the function to call when the scene needs to be re-rendered.
    /// Callback signature: (state, scene, frameContext) => void
    pub fn on_draw_request(&mut self, f: Function) { self.on_draw_request = Some(f); }

    /// Registers a handler for input events (keyboard/mouse).
    /// Callback signature: (state, scene, event) => void
    /// The event is an object: { type: string, data: any }
    pub fn with_event_handler(&mut self, f: Function) { self.with_event_handler = Some(f); }

    /// Registers a callback fired whenever the inspector selection changes
    /// (editor mode only).
    /// Callback signature: `(data: InspectorData | undefined) => void`
    /// `data` is `undefined` when the selection is cleared.
    pub fn on_select(&mut self, f: Function) { self.on_select = Some(f); }

    /// Registers a callback fired when editor state changes that are relevant
    /// to the scene editor UI (gizmo mode switches, drag start/end).
    ///
    /// The callback receives a single [`EditorEventType`] object.
    ///
    /// # Example
    ///
    /// ```ts
    /// window.on_editor_event(ev => {
    ///   if (ev.type === "gizmo_mode_changed") setActiveGizmo(ev.mode);
    ///   if (ev.type === "drag_start")  console.log("dragging", ev.axis);
    ///   if (ev.type === "drag_end")    console.log("drag released");
    /// });
    /// ```
    ///
    /// Callback signature: `(event: EditorEventType) => void`
    pub fn on_editor_event(&mut self, f: Function) { self.on_editor_event = Some(f); }

    /// Initializes the engine and starts the RequestAnimationFrame loop.
    /// @param {string} canvas_id - The ID of the HTMLCanvasElement to target.
    pub fn start(mut self, canvas_id: String) {
        // Initialize the engine window with JsValue as the state type S
        let camera_val = unsafe {
            if self.camera.owned {
                self.camera.owned = false;
                *Box::from_raw(self.camera.inner)
            } else {
                core::ptr::read(self.camera.inner)
            }
        };

        let mut engine_window = Window::new(self.state)
            .with_title("Vertra Web")
            .with_canvas_id(canvas_id)
            .with_camera(camera_val);

        // We can't move a reference into an owned Wasm struct.
        unsafe fn wrap_scene(scene: &mut vertra::scene::Scene) -> Scene {
            Scene { inner: scene as *mut vertra::scene::Scene }
        }

        if let Some(f) = self.on_startup {
            engine_window = engine_window.on_startup(move |state, scene, _ctx| {
                let frame_ctx = FrameContext { dt: _ctx.dt };
                let _ = f.call3(
                    &JsValue::UNDEFINED,
                    state,
                    &JsValue::from(unsafe { wrap_scene(scene) }),
                    &JsValue::from(frame_ctx)
                );
            });
        }

        if let Some(f) = self.on_update {
            engine_window = engine_window.on_update(move |state, scene, _ctx| {
                let frame_ctx = FrameContext { dt: _ctx.dt };
                let _ = f.call3(
                    &JsValue::UNDEFINED,
                    state,
                    &JsValue::from(unsafe { wrap_scene(scene) }),
                    &JsValue::from(frame_ctx)
                );
            });
        }

        if let Some(f) = self.on_draw_request {
            engine_window = engine_window.on_draw_request(move |state, scene, _ctx| {
                let frame_ctx = FrameContext { dt: _ctx.dt };
                let _ = f.call3(
                    &JsValue::UNDEFINED,
                    state,
                    &JsValue::from(unsafe { wrap_scene(scene) }),
                    &JsValue::from(frame_ctx)
                );
            });
        }

        // Register a combined event handler whenever any of the three
        // event-driven callbacks is set.  This decouples on_select and
        // on_editor_event from requiring with_event_handler to be set.
        let needs_event_handler = self.with_event_handler.is_some()
            || self.on_select.is_some()
            || self.on_editor_event.is_some();

        if needs_event_handler {
            let on_select_fn      = self.on_select.clone();
            let on_editor_ev_fn   = self.on_editor_event.clone();
            let user_event_fn     = self.with_event_handler;

            // State tracking for on_editor_event – captured by value into the
            // FnMut closure so they persist across frames.
            let mut prev_gizmo_mode  = vertra::editor::GizmoMode::Translate;
            let mut prev_is_dragging = false;

            engine_window = engine_window.with_event_handler(move |state, scene, event, _elwt| {

                if scene.editor.is_some() {
                    let ed_ev: Option<EditorEvent> = match &event {
                        Event::WindowEvent { event: wev, .. } => match wev {
                            WindowEvent::CursorMoved { position, .. } => Some(EditorEvent::CursorMoved {
                                x: position.x as f32, y: position.y as f32,
                            }),
                            WindowEvent::MouseInput { state: s, button, .. } => {
                                let pressed = *s == ElementState::Pressed;
                                Some(EditorEvent::MouseButton {
                                    left:   (*button == MouseButton::Left).then_some(pressed),
                                    middle: (*button == MouseButton::Middle).then_some(pressed),
                                    right:  (*button == MouseButton::Right).then_some(pressed),
                                })
                            }
                            WindowEvent::MouseWheel { delta, .. } => {
                                let scroll = match delta {
                                    MouseScrollDelta::LineDelta(_, y) => *y,
                                    MouseScrollDelta::PixelDelta(p)   => p.y as f32 * 0.1,
                                };
                                Some(EditorEvent::Scroll { delta: scroll })
                            }
                            WindowEvent::ModifiersChanged(mods) => {
                                let s = mods.state();
                                Some(EditorEvent::ModifiersChanged {
                                    alt:  s.alt_key(),
                                    ctrl: s.control_key(),
                                })
                            }
                            WindowEvent::KeyboardInput { event: ke, .. } => {
                                use vertra::event::PhysicalKey;
                                use winit::keyboard::KeyCode;
                                if let PhysicalKey::Code(code) = ke.physical_key {
                                    let ev = match ke.state {
                                        ElementState::Pressed  => EditorEvent::KeyPressed(code),
                                        ElementState::Released => EditorEvent::KeyReleased(code),
                                    };
                                    if ke.state == ElementState::Pressed && code == KeyCode::KeyF {
                                        scene.handle_editor_event(EditorEvent::KeyPressed(code));
                                        Some(EditorEvent::FocusKey)
                                    } else {
                                        Some(ev)
                                    }
                                } else { None }
                            }
                            _ => None,
                        },
                        Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                            Some(EditorEvent::MouseMotionDelta {
                                dx: delta.0 as f32, dy: delta.1 as f32,
                            })
                        }
                        _ => None,
                    };

                    let prev_id = scene.inspector().map(|d| d.id);

                    if let Some(e) = ed_ev {
                        scene.handle_editor_event(e);
                    }

                    // ── on_select: fire when selection changes ────────────────
                    let curr_id = scene.inspector().map(|d| d.id);
                    if curr_id != prev_id {
                        if let Some(sel_fn) = &on_select_fn {
                            let js_val = scene.inspector()
                                .and_then(|d| {
                                    let js = crate::editor::JsInspectorData::from(d);
                                    serde_wasm_bindgen::to_value(&js).ok()
                                })
                                .unwrap_or(JsValue::UNDEFINED);
                            let _ = sel_fn.call1(&JsValue::UNDEFINED, &js_val);
                        }
                    }

                    if let Some(ed) = &scene.editor {
                        let curr_mode = ed.gizmo_mode;
                        if curr_mode != prev_gizmo_mode {
                            prev_gizmo_mode = curr_mode;
                            if let Some(cb) = &on_editor_ev_fn {
                                let mode_str = match curr_mode {
                                    vertra::editor::GizmoMode::Translate => "translate",
                                    vertra::editor::GizmoMode::Rotate    => "rotate",
                                    vertra::editor::GizmoMode::Scale     => "scale",
                                };
                                let ev = WebEditorEvent::GizmoModeChanged {
                                    mode: mode_str.to_string(),
                                };
                                if let Ok(js) = serde_wasm_bindgen::to_value(&ev) {
                                    let _ = cb.call1(&JsValue::UNDEFINED, &js);
                                }
                            }
                        }

                        let curr_dragging = ed.drag.is_some();
                        if curr_dragging != prev_is_dragging {
                            prev_is_dragging = curr_dragging;
                            if let Some(cb) = &on_editor_ev_fn {
                                let web_ev = if curr_dragging {
                                    let axis_str = match ed.drag.as_ref().map(|d| d.axis) {
                                        Some(vertra::editor::DragAxis::X) => "x",
                                        Some(vertra::editor::DragAxis::Y) => "y",
                                        Some(vertra::editor::DragAxis::Z) => "z",
                                        None => "x",
                                    };
                                    WebEditorEvent::DragStart { axis: axis_str.to_string() }
                                } else {
                                    WebEditorEvent::DragEnd
                                };
                                if let Ok(js) = serde_wasm_bindgen::to_value(&web_ev) {
                                    let _ = cb.call1(&JsValue::UNDEFINED, &js);
                                }
                            }
                        }
                    } else {
                        // Editor was deactivated (e.g. Escape); reset tracking state.
                        prev_gizmo_mode  = vertra::editor::GizmoMode::Translate;
                        prev_is_dragging = false;
                    }
                }

                let web_event = match event {
                    Event::WindowEvent { event: WindowEvent::KeyboardInput { event: key_event, .. }, .. } => {
                        if let PhysicalKey::Code(code) = key_event.physical_key {
                            let code_str = format!("{:?}", code);
                            match key_event.state {
                                ElementState::Pressed => Some(WebEvent::KeyDown {
                                    code: code_str, repeat: key_event.repeat
                                }),
                                ElementState::Released => Some(WebEvent::KeyUp { code: code_str }),
                            }
                        } else { None }
                    }
                    Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                        Some(WebEvent::MouseMotion { dx: delta.0, dy: delta.1 })
                    }
                    Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                        Some(WebEvent::MouseMove { x: position.x, y: position.y })
                    }
                    Event::WindowEvent { event: WindowEvent::MouseInput { state: s, button, .. }, .. } => {
                        let btn_str = match button {
                            MouseButton::Left   => "left",
                            MouseButton::Right  => "right",
                            MouseButton::Middle => "middle",
                            _                   => "other",
                        }.to_string();
                        match s {
                            ElementState::Pressed  => Some(WebEvent::MouseDown { button: btn_str }),
                            ElementState::Released => Some(WebEvent::MouseUp   { button: btn_str }),
                        }
                    }
                    Event::WindowEvent { event: WindowEvent::MouseWheel { delta, .. }, .. } => {
                        let scroll = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(p)   => p.y as f32 * 0.1,
                        };
                        Some(WebEvent::Wheel { delta: scroll })
                    }
                    _ => None,
                };

                // In editor mode the engine owns all input; the client handler
                // is intentionally suppressed so editor operations (orbit,
                // pick, gizmo drag) don't bleed through to game logic.
                // Once `scene.disable_editor_mode()` is called the block below
                // becomes active and all events are forwarded as usual.
                if scene.editor.is_none() {
                    if let (Some(e), Some(f)) = (web_event, &user_event_fn) {
                        if let Ok(js_event_obj) = serde_wasm_bindgen::to_value(&e) {
                            let _ = f.call3(
                                &JsValue::UNDEFINED,
                                state,
                                &JsValue::from(unsafe { wrap_scene(scene) }),
                                &js_event_obj,
                            );
                        }
                    }
                }
            });
        }
        engine_window.create();
    }
}