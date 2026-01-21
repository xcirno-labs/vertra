# Vertra

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

> #### ⚠️ **Warning:** Vertra is currently in an **experimental, pre-alpha state**.
> The API is highly unstable, undergoing frequent breaking changes, and is **not suitable for use in production environments**.

Vertra is a lightweight, high-performance 3D rendering engine for Rust. Built on top of `wgpu`,
it provides a streamlined abstraction for hardware-accelerated graphics, featuring a professional
3D camera system, hierarchical scene graph, and programmable event scripting.

## Features
* **Scene Graph & Hierarchy**: Support for parent-child relationships with inherited world transforms.
* **3D Perspective Camera**: Full implementation of Perspective Projection and View matrices (Left-Handed Y-Up).
* **Batch Rendering**: Optimized draw call management via recursive vertex/index "baking."
* **Cross-Platform**: Leverages wgpu for Vulkan, Metal, DX12, and WebGPU.
* **Declarative Windowing**: Builder-pattern API for window creation and state management.
* **Unified Transform System**: Support for 3D scaling, Euler rotations, and 3D translation.

## Getting Started
Vertra is currently in active development and is not yet published to crates.io. To use it, clone the repository and reference it via a path dependency in your project's `Cargo.toml`:

```toml
[dependencies]
vertra = { path = "../path/to/vertra" }
```

## Usage Example
The following example demonstrates creating a parent-child relationship where a "Moon" orbits a "Planet" by inheriting the parent's rotation.

```rust
use std::collections::HashSet;
use winit::event::{DeviceEvent, ElementState, Event, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use vertra::camera::Camera;
use vertra::window::Window;
use vertra::transform::Transform;
use vertra::geometry::Geometry;
use vertra::objects::Object;

struct AppState {
    pressed_keys: HashSet<KeyCode>,
    object_ids: Vec<usize>,
}

fn main() {
    let initial_state = AppState {
        pressed_keys: HashSet::new(),
        object_ids: Vec::new(),
    };

    Window::new(initial_state)
        .with_title("Simple Solar Simulation")
        .with_camera(
            Camera::new()
                .with_position([0.0, 8.0, -12.0])
                .with_rotation(90.0, -30.0)
        )
        // INPUT HANDLING
        .with_event_handler(|state, scene, event, _| {
            match event {
                Event::WindowEvent { event: WindowEvent::KeyboardInput { event: key_event, .. }, .. } => {
                    if let PhysicalKey::Code(code) = key_event.physical_key {
                        match key_event.state {
                            ElementState::Pressed => { state.pressed_keys.insert(code); },
                            ElementState::Released => { state.pressed_keys.remove(&code); },
                        };
                    }
                }
                Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                    scene.camera.rotate(delta.0 as f32 * 0.1, delta.1 as f32 * 0.1, false);
                }
                _ => {}
            }
        })
        .on_startup(|state, scene, _| {
            // 1. The Sun (Center)
            let sun = Object {
                name: "Sun".to_string(),
                transform: Transform::from_position(0.0, 0.0, 0.0),
                geometry: Some(Geometry::Sphere { radius: 2.0, subdivisions: 32 }),
                color: [1.0, 0.9, 0.2, 1.0], // Yellow
                ..Default::default()
            };
            let sun_id = scene.spawn(sun, None);

            // 2. The Planet (Child)
            let planet = Object {
                name: "Planet".to_string(),
                transform: Transform::from_position(6.0, 0.0, 0.0), // 6 units away
                geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 24 }),
                color: [0.2, 0.5, 1.0, 1.0], // Blue
                ..Default::default()
            };
            let planet_id = scene.spawn(planet, Some(sun_id));

            // 3. The Moon (Grandchild)
            let moon = Object {
                name: "Moon".to_string(),
                transform: Transform::from_position(1.5, 0.0, 0.0), // 1.5 units away from planet
                geometry: Some(Geometry::Sphere { radius: 0.3, subdivisions: 16 }),
                color: [0.7, 0.7, 0.7, 1.0], // Gray
                ..Default::default()
            };
            scene.spawn(moon, Some(planet_id));

            state.object_ids.push(sun_id);
            state.object_ids.push(planet_id);
        })
        .on_update(|state, scene, ctx| {
            scene.camera.handle_default_input(&state.pressed_keys, 3.0, ctx);

            // Rotate the Sun (the planet will orbit automatically)
            if let Some(&sun_id) = state.object_ids.get(0) {
                if let Some(sun) = scene.world.get_mut(sun_id) {
                    sun.transform.rotation[1] += 30.0 * ctx.dt;
                }
            }

            // Rotate the Planet (the moon will orbit the planet automatically)
            if let Some(&planet_id) = state.object_ids.get(1) {
                if let Some(planet) = scene.world.get_mut(planet_id) {
                    planet.transform.rotation[1] += 100.0 * ctx.dt;
                }
            }
        })
        .create();
}
```

## Technical Architecture

### Scene Graph & Transforms
Vertra uses a tree-based scene graph. Each `Object` contains a local `Transform`.
During the rendering phase, the engine recursively traverses the tree, combining parent
and child matrices to calculate absolute world-space coordinates. This allows for
complex nested animations (e.g., solar systems, skeletal structures).

### Coordinate System
Vertra utilizes a **Y-Up, Left-Handed** coordinate system. The engine includes a `Transform` system
to handle the mapping of world-space coordinates to the GPU clip-space.

### Rendering Pipeline
The engine uses a modern **WGSL-based pipeline**. To maximize performance, vertex data is "baked"
into shared GPU buffers. The engine flattens the object hierarchy into a single optimized vertex
stream each frame, minimizing the overhead of multiple draw calls and state changes.

## License
Copyright 2026 xCirno.

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in
compliance with the License. You may obtain a copy of the License at:

http://www.apache.org/licenses/LICENSE-2.0

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional
terms or conditions.
