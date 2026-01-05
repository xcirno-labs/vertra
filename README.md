# Vertra

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

Vertra is a lightweight, high-performance 2D batch rendering engine for Rust. Built on top of `wgpu`, 
it provides a streamlined abstraction for creating hardware-accelerated 2D applications without the 
complexity of raw graphics API management.

## Features

- **Batch Rendering**: Optimized draw call management through dynamic vertex and index buffering.
- **Hardware Acceleration**: Leverages `wgpu` for cross-platform support (Vulkan, Metal, DX12, and WebGPU).
- **Declarative Windowing**: A builder-pattern API for window creation and event loop management via `winit`.
- **Integrated Transform System**: Built-in support for 2D scaling, rotation, and translation.
- **Primitive Geometry**: Native support for Triangles, Rectangles, and Squares with automated mesh generation.
- **Timer Abstraction**: Frame-independent timing utilities for smooth animations and logic updates.

## Getting Started

Add Vertra to your `Cargo.toml`:

```toml
[dependencies]
vertra = "0.1.0"
```

## Usage Example

The following example demonstrates how to initialize a window, update state using the `Timer` abstraction, 
and render batched geometry.

```rust
use std::rc::Rc;
use std::cell::RefCell;
use vertra::camera::Camera;
use vertra::window::Window;
use vertra::transform::{Transform};
use vertra::geometry::Geometry;
use vertra::timer::Timer;

struct AppState {
    visibility_timer: Timer,
    triangle_pos: Transform,
    square_pos: Transform,
    show_triangle: bool,
}

fn main() {
    // Initialize State with Timer
    let state = Rc::new(RefCell::new(AppState {
        visibility_timer: Timer::new(2.0), // Wait 2 seconds
        triangle_pos: Transform::from_position(-250.0, 0.0, 0.0),
        square_pos: Transform::from_position(250.0, 0.0, 0.0),
        show_triangle: true,
    }));

    let u_state = Rc::clone(&state);
    let d_state = Rc::clone(&state);

    Window::new()
        .with_title("Vertra with Timer")
        // Set up the camera
        .with_camera(Camera::new().with_fov(20.0))
        .on_update(move |dt| {
            let mut s = u_state.borrow_mut();

            // Use Timer's update method
            s.visibility_timer.update(dt);

            // Rotate the square every frame
            s.square_pos.rotation += 60.0 * dt;

            // Change triangle visibility
            if s.visibility_timer.is_finished() {
                s.show_triangle = !s.show_triangle;
                s.visibility_timer.reset(); // Reset for the next 2-second cycle
            }
        })
        .on_draw_request(move |scene| {
            let s = d_state.borrow();

            // Clear the buffer to draw fresh this frame
            scene.mesh.clear();

            // Draw Square
            scene.mesh.add_geometry(
                &Geometry::Rectangle { height: 100.0, width: 100.0 },
                &s.square_pos,
                [0.0, 0.8, 1.0, 1.0]
            );

            // Draw Triangle only if toggle is true
            if s.show_triangle {
                scene.mesh.add_geometry(
                    &Geometry::Triangle { base: 100.0, height: 100.0 },
                    &s.triangle_pos,
                    [1.0, 0.2, 0.5, 1.0]
                );
            }
        })
        .create();
}
```

## Technical Architecture

### Coordinate System
Vertra utilizes Normalized Device Coordinates (NDC) by default, where the screen extends from `-1.0` to `1.0` 
on both axes. The library includes a `Transform` system to map world-space coordinates to the GPU clip-space.

### Rendering Pipeline
The engine uses a modern WGSL-based pipeline. Vertex data is automatically batched into shared buffers, 
which dynamically resize to accommodate scene complexity. This minimizes the overhead of CPU-to-GPU transfers 
and draw calls.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.