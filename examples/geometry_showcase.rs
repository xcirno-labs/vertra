//! # Geometry Showcase
//!
//! Displays all six built-in geometry types side by side in a static editor
//! scene so you can visually inspect and compare them.
//!
//! | Position | Shape    | Geometry variant                           |
//! |----------|----------|--------------------------------------------|
//! | x = -7.5 | Cube     | `Geometry::Cube { size }`                  |
//! | x = -4.5 | Box      | `Geometry::Box { width, height, depth }`   |
//! | x = -1.5 | Plane    | `Geometry::Plane { size }`                 |
//! | x =  1.5 | Pyramid  | `Geometry::Pyramid { base_size, height }`  |
//! | x =  4.5 | Capsule  | `Geometry::Capsule { radius, height, … }`  |
//! | x =  7.5 | Sphere   | `Geometry::Sphere { radius, subdivisions }`|
//!
//! **Run:**
//! ```sh
//! cargo run --example geometry_showcase
//! ```
//!
//! Editor mode is active: click any object to select it and inspect its
//! transform in the gizmo overlay.

use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::transform::Transform;
use vertra::window::Window;

fn main() {
    Window::new(())
        .with_title("Geometry Showcase — Vertra example")
        .with_camera(
            Camera::new()
                .with_position([0.0, 6.0, -18.0])
                .with_rotation(90.0, -18.0),
        )
        .on_startup(|_, scene, _| {
            let shapes: &[(&str, &str, Geometry, [f32; 4])] = &[
                (
                    "Cube",
                    "geo_cube",
                    Geometry::Cube { size: 1.8 },
                    [0.9, 0.3, 0.3, 1.0], // red
                ),
                (
                    "Box",
                    "geo_box",
                    Geometry::Box {
                        width: 1.2,
                        height: 2.0,
                        depth: 0.8,
                    },
                    [0.9, 0.6, 0.2, 1.0], // orange
                ),
                (
                    "Plane",
                    "geo_plane",
                    Geometry::Plane { size: 2.0 },
                    [0.9, 0.9, 0.2, 1.0], // yellow
                ),
                (
                    "Pyramid",
                    "geo_pyramid",
                    Geometry::Pyramid {
                        base_size: 1.8,
                        height: 2.5,
                    },
                    [0.3, 0.8, 0.3, 1.0], // green
                ),
                (
                    "Capsule",
                    "geo_capsule",
                    Geometry::Capsule {
                        radius: 0.6,
                        height: 1.6,
                        subdivisions: 20,
                    },
                    [0.2, 0.5, 1.0, 1.0], // blue
                ),
                (
                    "Sphere",
                    "geo_sphere",
                    Geometry::Sphere {
                        radius: 1.0,
                        subdivisions: 24,
                    },
                    [0.7, 0.2, 0.9, 1.0], // purple
                ),
            ];

            // Lay out objects evenly along the X-axis, centred at the origin.
            let count = shapes.len() as f32;
            let spacing = 3.0_f32;
            let start_x = -((count - 1.0) * spacing * 0.5);

            for (i, (name, str_id, geometry, color)) in shapes.iter().enumerate() {
                let x = start_x + i as f32 * spacing;
                scene.spawn(
                    Object {
                        name: name.to_string(),
                        str_id: str_id.to_string(),
                        transform: Transform::from_position(x, 0.0, 0.0),
                        geometry: Some(geometry.clone()),
                        color: *color,
                        ..Default::default()
                    },
                    None,
                );
            }

            scene.enable_editor_mode();
        })
        .create();
}

