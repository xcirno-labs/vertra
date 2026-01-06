# Vertra

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

Vertra is a lightweight, high-performance 3D rendering engine for Rust. Built on top of `wgpu`, 
it provides a streamlined abstraction for hardware-accelerated graphics, featuring a professional 
3D camera system, flight mechanics, and programmable event scripting.

## Features
* **3D Perspective Camera**: Full implementation of Perspective Projection and View matrices (Left-Handed Y-Up).
* **Batch Rendering**: Optimized draw call management via dynamic vertex/index buffering.
* **Cross-Platform**: Leverages wgpu for Vulkan, Metal, DX12, and WebGPU.
* **Declarative Windowing**: Builder-pattern API for window creation.
* **Unified Transform System**: Support for 3D scaling, Euler rotations, and 3D translation.

## Getting Started
Add Vertra to your `Cargo.toml`:

```toml
[dependencies]
vertra = "0.1.0"
```

## Usage Example
The following example demonstrates the State Container Pattern, initializing a window with custom state and 
handling input via a fixed-timestep loop.
```rust
use std::collections::HashSet;
use winit::event::{ElementState, Event, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use vertra::camera::Camera;
use vertra::window::Window;
use vertra::transform::Transform;
use vertra::geometry::Geometry;

struct AppState {
    player_entity_id: Option<usize>,
    pressed_keys: HashSet<KeyCode>,
}

fn main() {
    let initial_state = AppState {
        player_entity_id: None,
        pressed_keys: HashSet::new(),
    };

    Window::new(initial_state)
        .with_camera(
            Camera::new()
                .with_position([0.0, 2.0, -10.0])
                .with_target([0.0, 0.0, 0.0])
        )
        .with_title("A spinning cube!")
        .on_fixed_update(|state, scene| {
            scene.camera.handle_default_input(&state.pressed_keys);
        })
        .with_event_handler(|state, _, event, _| {
            if let Event::WindowEvent { event: WindowEvent::KeyboardInput { event: key_event, .. }, .. } = event {
                if let PhysicalKey::Code(code) = key_event.physical_key {
                    match key_event.state {
                        ElementState::Pressed => { state.pressed_keys.insert(code); },
                        ElementState::Released => { state.pressed_keys.remove(&code); },
                    };
                }
            }
        })
        .on_startup(|state, scene| {
            let entity_id = scene.spawn(
                &Geometry::Cube { size: 1.0 },
                Transform::from_position(0.0, 0.0, 0.0),
                [0.0, 1.0, 0.5, 1.0]
            );
            state.player_entity_id = Some(entity_id);
        })
        .on_update(|state, dt, scene| {
            if let Some(id) = state.player_entity_id {
                if let Some(entity) = scene.world.get_entity_mut(id) {
                    entity.transform.rotation[1] += 45.0 * dt;
                }
            }
        })
        .create();
}
```

## Technical Architecture

### Coordinate System
Vertra utilizes a Y-Up, Left-Handed coordinate system. The engine includes a `Transform` system 
to handle the mapping of world-space coordinates to the GPU clip-space.

### Rendering Pipeline
The engine uses a modern WGSL-based pipeline. Vertex data is automatically batched into shared buffers, 
which dynamically resize to accommodate scene complexity. This minimizes the overhead of CPU-to-GPU transfers 
and draw calls.

## License
Copyright 2026 xCirno.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in 
compliance with the License. You may obtain a copy of the License at:

http://www.apache.org/licenses/LICENSE-2.0

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the 
work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional 
terms or conditions.