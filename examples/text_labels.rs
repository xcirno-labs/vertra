//! # Text Labels
//!
//! Demonstrates the full text-label API:
//!
//! * All three [`HorizontalAlignment`] modes — Left, Center, Right.
//! * The three bundled font faces: **sans**, **mono**, and **serif**.
//! * Dynamic label updates (frame counter in the top-left corner).
//!
//! **Run:**
//! ```sh
//! cargo run --example text_labels --features default-fonts
//! ```
//!
//! **Controls:**
//! * `Escape` - toggle the built-in scene editor (drag labels with the mouse).
//! * Close the window to exit.

use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::text_overlay::{HorizontalAlignment, VerticalAlignment};
use vertra::transform::Transform;
use vertra::window::Window;

struct AppState {
    /// Handle to the live frame-counter label so we can update it every frame.
    counter_id: Option<vertra::text_overlay::TextLabelHandle>,
    /// Total frames rendered since startup.
    frame: u64,
}

fn main() {
    Window::new(AppState { counter_id: None, frame: 0 })
        .with_title("Text Labels Demo")
        .with_camera(
            Camera::new()
                .with_position([0.0, 3.0, -6.0])
                .with_rotation(90.0, -20.0),
        )
        .on_startup(|state, scene, _| {
            // 3D object in the background
            scene.spawn(
                Object {
                    name:     "Showcase Sphere".to_string(),
                    str_id:   "sphere".to_string(),
                    geometry: Some(Geometry::Sphere { radius: 1.5, subdivisions: 32 }),
                    color:    [0.3, 0.55, 0.9, 1.0],
                    transform: Transform::from_position(0.0, 0.0, 0.0),
                    ..Default::default()
                },
                None,
            );
            scene.text_overlay.load_default_fonts();
            // Left-aligned label (default, anchored to left edge)
            scene.text_overlay
                .add_label("Left-aligned  (sans)")
                .at(20.0, 60.0)
                .with_font_size(22.0)
                .with_font("sans")
                .with_color([0.3, 1.0, 0.6, 1.0])
                .with_alignment(HorizontalAlignment::Left)
                .with_zindex(10)
                .build();

            // Center-aligned label (always centred in viewport)
            scene.text_overlay
                .add_label("Centered  (mono)")
                .at(0.0, 100.0)
                .with_font_size(22.0)
                .with_font("mono")
                .with_color([1.0, 1.0, 0.3, 1.0])
                .with_alignment(HorizontalAlignment::Free)
                .with_zindex(11)
                .build();

            // Right-aligned label (anchored to right edge)
            scene.text_overlay
                .add_label("Right-aligned  (serif)")
                .at(20.0, 140.0)
                .with_font_size(22.0)
                .with_font("serif")
                .with_color([1.0, 0.5, 0.3, 1.0])
                .with_alignment(HorizontalAlignment::Right)
                .with_zindex(12)
                .build();

            // Bottom-center label (vertical + horizontal alignment)
            scene.text_overlay
                .add_label("Bottom-center  (sans)")
                .at(0.0, 10.0)   // y = bottom margin; x ignored for Center
                .with_font_size(20.0)
                .with_font("sans")
                .with_color([0.7, 0.4, 1.0, 1.0])
                .with_alignment(HorizontalAlignment::Center)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_zindex(13)
                .build();

            // Hidden label revealed after first frame
            let hint = scene.text_overlay
                .add_label("Press Escape to toggle editor")
                .at(20.0, 180.0)
                .with_font_size(16.0)
                .with_font("sans")
                .with_color([0.7, 0.7, 0.7, 1.0])
                .with_alignment(HorizontalAlignment::Left)
                .with_zindex(5)
                .hidden()
                .build();
            hint.show(&mut scene.text_overlay);

            // Live frame counter (updated every frame in on_update)
            let counter = scene.text_overlay
                .add_label("Frame: 0")
                .at(20.0, 20.0)
                .with_font_size(18.0)
                .with_font("mono")
                .with_color([0.9, 0.9, 0.9, 1.0])
                .with_alignment(HorizontalAlignment::Left)
                .with_zindex(20)
                .build();

            state.counter_id = Some(counter);

        })
        .on_update(|state, scene, _ctx| {
            state.frame += 1;
            if let Some(handle) = state.counter_id {
                handle.set_text(
                    &mut scene.text_overlay,
                    format!("Frame: {}", state.frame),
                );
            }
        }).with_event_handler(|_, scene, event, _| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event: ke, .. }, ..
                } => {
                    if let PhysicalKey::Code(code) = ke.physical_key {
                        if code == KeyCode::Escape && ke.state == ElementState::Pressed {
                            if scene.editor.is_some() {
                                scene.disable_editor_mode();
                            } else {
                                scene.enable_editor_mode();
                            }
                        }
                    }
                }
                _ => {}
            }
        })
        .create();
}
