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
use vertra::window::Window;
use vertra::transform::Transform;
use vertra::geometry::Geometry;
use vertra::timer::Timer;

struct AppState {
    timer: Timer,
    square_transform: Transform,
    rotation: f32,
}

fn main() {
    let state = Rc::new(RefCell::new(AppState {
        timer: Timer::new(2.0),
        square_transform: Transform::from_position(0.0, 0.0, 0.0),
        rotation: 0.0,
    }));

    let u_state = Rc::clone(&state);
    let d_state = Rc::clone(&state);

    Window::new()
        .with_title("Vertra Example")
        .with_dimensions(800, 600)
        .on_update(move |dt| {
            let mut s = u_state.borrow_mut();
            s.rotation += 45.0 * dt;
            s.square_transform.rotation = s.rotation;
        })
        .on_draw_request(move |batch| {
            let s = d_state.borrow();
            batch.clear();
            
            batch.add_geometry(
                &Geometry::Rectangle { width: 0.5, height: 0.5 },
                &s.square_transform,
                [0.2, 0.6, 1.0, 1.0]
            );
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