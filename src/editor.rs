//! Static scene editor — orbit camera, transform gizmos, object inspector.
//!
//! # Architecture
//!
//! The editor subsystem is organised into four sub-modules:
//!
//! | Module              | Contents                                                           |
//! |---------------------|--------------------------------------------------------------------|
//! | [`types`]           | Data types and events ([`EditorEvent`], [`EditorStateEvent`], …)  |
//! | [`state`]           | [`EditorState`] — all runtime state and input-processing logic    |
//! | [`gizmo`]           | Translate / rotate / scale gizmo and selection-box mesh builders  |
//! | `math` *(internal)* | Ray-cast, AABB, hierarchy, and vector helpers                      |
//!
//! # Quick start
//!
//! ```rust,ignore
//! // Enable editor mode from on_startup:
//! scene.enable_editor_mode();
//! ```
//!
//! # Responding to editor state changes
//!
//! Register a callback with [`crate::window::Window::on_editor_event`]:
//!
//! ```rust
//! .on_editor_event(|_state, _scene, event, obj| {
//!     let name = obj.as_ref().map(|o| o.name.as_str()).unwrap_or("–");
//!     match event {
//!         EditorStateEvent::GizmoModeChanged(mode) =>
//!             println!("Gizmo → {mode:?}  (selected: {name})"),
//!         EditorStateEvent::DragStart { axis } =>
//!             println!("Drag on {axis:?}  (obj: {name})"),
//!         EditorStateEvent::DragEnd =>
//!             println!("Drag ended  (obj: {name})"),
//!     }
//! })
//! ```
//!
//! > **Note:** [`on_update`](crate::window::Window::on_update),
//! > [`on_fixed_update`](crate::window::Window::on_fixed_update), and
//! > [`on_draw_request`](crate::window::Window::on_draw_request) are
//! > automatically suppressed while editor mode is active.
pub mod types;
pub mod state;
pub mod gizmo;
pub(crate) mod math;

pub use types::{
    EditorEvent,
    EditorInput,
    EditorStateEvent,
    GizmoMode,
    DragAxis,
    DragKind,
    DragState,
    // Inspector data
    InspectorData,
    Inspector,
};
pub use state::EditorState;
pub use gizmo::{
    build_gizmo_mesh_data,
    build_rotate_gizmo_mesh_data,
    build_scale_gizmo_mesh_data,
    build_selection_box,
    build_skybox_mesh,
};