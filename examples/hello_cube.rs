//! # Hello Cube
//!
//! The simplest possible Vertra scene: a single orange cube that slowly
//! rotates in place.  This example shows the minimum boilerplate required
//! to open a window and render geometry.
//!
//! **Run:**
//! ```sh
//! cargo run --example hello_cube
//! ```
//!
//! **Controls:** close the window to exit.

use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::transform::Transform;
use vertra::window::Window;

/// Application state — we cache the numeric ID resolved during startup so that
/// `on_update` never pays the cost of a string-hash lookup every frame.
struct AppState {
    cube_id: Option<usize>,
}

fn main() {
    Window::new(AppState { cube_id: None })
        .with_title("Hello, Cube!")
        .with_camera(
            Camera::new()
                .with_position([0.0, 2.0, -5.0])
                .with_rotation(90.0, -15.0),
        )
        .on_startup(|state, scene, _| {
            // Spawn a single cube at the world origin.
            let id = scene.spawn(
                Object {
                    name: "Cube".to_string(),
                    str_id: "cube".to_string(),
                    geometry: Some(Geometry::Cube { size: 1.5 }),
                    color: [0.9, 0.5, 0.2, 1.0], // warm orange
                    transform: Transform::default(),
                    ..Default::default()
                },
                None, // no parent — this is a root object
            );
            // Cache the ID so on_update can find it cheaply.
            state.cube_id = Some(id);
        })
        .on_update(|state, scene, ctx| {
            // Rotate 45° per second around the Y-axis.
            if let Some(cube) = state.cube_id.and_then(|id| scene.world.get_mut(id)) {
                cube.transform.rotation[1] += 45.0 * ctx.dt;
            }
        })
        .create();
}

