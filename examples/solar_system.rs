//! # Solar System
//!
//! A three-level parent-child hierarchy: **Sun → Planet → Moon**.
//! The Sun drives the entire system — rotating the Sun automatically
//! carries the Planet (and Moon) around it, demonstrating how Vertra
//! propagates world transforms down the object tree.
//!
//! Editor mode is enabled on startup so you can inspect and manipulate
//! any object with the gizmo.  The `on_editor_event` callback logs every
//! gizmo mode change and axis drag to stdout.
//!
//! **Run:**
//! ```sh
//! cargo run --example solar_system
//! ```
//!
//! **Editor keybinds:**
//! | Key        | Action                               |
//! |------------|--------------------------------------|
//! | `T`        | Switch to Translate gizmo            |
//! | `R`        | Switch to Rotate gizmo               |
//! | `E`        | Switch to Scale gizmo                |
//! | `F`        | Frame / focus on selected object     |
//! | `Escape`   | Exit editor mode (enter play mode)   |
//! | `RMB drag` | Orbit camera                         |
//! | `MMB drag` | Pan camera                           |
//! | Scroll     | Dolly in / out                       |

use vertra::camera::Camera;
use vertra::window::Window;
use vertra::transform::Transform;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::editor::{EditorStateEvent, GizmoMode, DragAxis};

struct AppState {
    sun_id: Option<usize>,
    earth_id: Option<usize>,
    moon_id: Option<usize>,
}

fn main() {
    let initial_state = AppState {
        sun_id: None,
        earth_id: None,
        moon_id: None,
    };

    Window::new(initial_state)
        .with_title("Simple Solar Simulation")
        .with_camera(
            Camera::new()
                .with_position([0.0, 8.0, -12.0])
                .with_rotation(90.0, -30.0)
        )
        .on_startup(|state, scene, _| {
            // 1. The Sun (Center)
            let sun = Object {
                name: "Sun".to_string(),
                str_id: "sun".to_string (),
                transform: Transform::from_position(0.0, 0.0, 0.0),
                geometry: Some(Geometry::Cube { size: 2.0 }),
                color: [1.0, 0.9, 0.2, 1.0],
                ..Default::default()
            };
            let sun_id = scene.spawn(sun, None);

            // 2. The Planet (Child)
            let planet = Object {
                name: "Planet".to_string(),
                str_id: "earth".to_string(),
                transform: Transform::from_position(6.0, 0.0, 0.0),
                geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 24 }),
                color: [0.2, 0.5, 1.0, 1.0],
                ..Default::default()
            };
            let planet_id = scene.spawn(planet, Some(sun_id));

            // 3. The Moon (Grandchild)
            let moon = Object {
                name: "Moon".to_string(),
                str_id: "moon".to_string(),
                transform: Transform::from_position(1.5, 0.0, 0.0),
                geometry: Some(Geometry::Sphere { radius: 0.3, subdivisions: 16 }),
                color: [0.7, 0.7, 0.7, 1.0],
                ..Default::default()
            };
            scene.spawn(moon, Some(planet_id));

            state.sun_id    = scene.world.get_id("sun");
            state.earth_id  = scene.world.get_id("earth");
            state.moon_id   = scene.world.get_id("moon");
            scene.enable_editor_mode();
        })
        .on_update(|state, scene, ctx| {
            // Rotate the Sun (the planet will orbit automatically)
            if let Some(sun) = state.sun_id.and_then(|id| scene.world.get_mut(id)) {
                sun.transform.rotation[1] += 30.0 * ctx.dt;
            }

            // Rotate the Planet (Earth)
            if let Some(planet) = state.earth_id.and_then(|id| scene.world.get_mut(id)) {
                planet.transform.rotation[1] += 100.0 * ctx.dt;
            }
        })
        // on_editor_event fires whenever the editor's internal state changes:
        //   • T / R / E keys  →  GizmoModeChanged
        //   • Gizmo axis drag →  DragStart / DragEnd
        // on_update is suppressed in editor mode, so game logic here is safe.
        .on_editor_event(|_state, _scene, event, object| {
            match event {
                EditorStateEvent::GizmoModeChanged(mode) => {
                    let label = match mode {
                        GizmoMode::Translate => "Translate",
                        GizmoMode::Rotate    => "Rotate",
                        GizmoMode::Scale     => "Scale",
                    };
                    println!("[Editor] Gizmo mode → {label} {object:?}");
                }
                EditorStateEvent::DragStart { axis } => {
                    let label = match axis {
                        DragAxis::X => "X",
                        DragAxis::Y => "Y",
                        DragAxis::Z => "Z",
                    };
                    println!("[Editor] Drag started — axis: {label} {object:?}");
                }
                EditorStateEvent::DragEnd => {
                    println!("[Editor] Drag ended {object:?}");
                }
                EditorStateEvent::SelectionChanged => {
                    println!("[Editor] Selection changed {object:?}");
                }
            }
        })
        .create();
}