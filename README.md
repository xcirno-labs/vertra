# Vertra

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](#license)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

Vertra is a lightweight, cross-platform 3D rendering engine for Rust, built on top of `wgpu`.
It provides a streamlined abstraction for hardware-accelerated graphics with a professional
perspective camera, a safe hierarchical scene graph, a built-in static scene editor,
a compact binary scene format (VTR), and a WASM/JavaScript binder layer.

---

## Features

| Feature | Details |
|---|---|
| **Scene Graph & Hierarchy** | Parent-child relationships with inherited world transforms. Safe mutation via `spawn`, `delete`, `reparent`, and scene-graph change events. |
| **Perspective Camera** | Full view and projection matrix implementation (Y-up, left-handed, WGPU depth range). Builder-pattern construction with WASD + mouse-look helpers. |
| **Procedural Geometry** | Built-in `Cube`, `Box`, `Plane`, `Pyramid`, `Sphere`, and `Capsule` primitives. Geometry is generated on demand and batched into a single GPU draw call per texture group. |
| **Texture Support** | Load textures from RGBA data (or a file path on native) and bind them to objects by matching `texture_path`. |
| **Built-in Editor** | Static scene editor with orbit/pan/zoom camera, translate/rotate/scale gizmos, multi-select, group transform, object picker, and a skybox. Activated with `scene.enable_editor_mode()`. |
| **Fixed-Update Loop** | Separate `on_fixed_update` callback running at 60 Hz for physics-stable simulation. |
| **VTR Binary Format** | Compact, deterministic, little-endian binary format for saving and loading complete scenes. Roundtrips camera, hierarchy, transforms, colours, geometry, and texture paths. |
| **Cross-Platform** | `wgpu` backend supports Vulkan, Metal, DX12, WebGL, and WebGPU. |
| **WASM / JS Binder** | `binder/` crate exposes the full API to JavaScript via `wasm-bindgen`, including deferred scene-graph events safe from JS re-entrancy. |
| **Scene-Graph Events** | `World::on_scene_graph_modified` callback fires after every structural mutation (add / delete / reparent). Events are queued and dispatched outside the mutation borrow in the binder. |

---

## Getting Started

Vertra is not yet published to crates.io. Clone the repository and reference it via a path
dependency:

```toml
[dependencies]
vertra = { path = "../path/to/vertra" }
```

---

## Quick Example — Solar System

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
    sun_id: usize,
    planet_id: usize,
}

fn main() {
    Window::new(AppState { pressed_keys: HashSet::new(), sun_id: 0, planet_id: 0 })
        .with_title("Solar System")
        .with_camera(
            Camera::new()
                .with_position([0.0, 8.0, -12.0])
                .with_rotation(90.0, -30.0),
        )
        .with_event_handler(|state, scene, event, _| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput { event: ke, .. }, ..
                } => {
                    if let PhysicalKey::Code(code) = ke.physical_key {
                        match ke.state {
                            ElementState::Pressed  => { state.pressed_keys.insert(code); }
                            ElementState::Released => { state.pressed_keys.remove(&code); }
                        }
                    }
                }
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta }, ..
                } => {
                    scene.camera.rotate(delta.0 as f32 * 0.1, delta.1 as f32 * 0.1, false);
                }
                _ => {}
            }
        })
        .on_startup(|state, scene, _| {
            let sun = Object {
                name: "Sun".to_string(),
                geometry: Some(Geometry::Sphere { radius: 2.0, subdivisions: 32 }),
                color: [1.0, 0.9, 0.2, 1.0],
                ..Default::default()
            };
            state.sun_id = scene.spawn(sun, None);

            let planet = Object {
                name: "Planet".to_string(),
                transform: Transform::from_position(6.0, 0.0, 0.0),
                geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 24 }),
                color: [0.2, 0.5, 1.0, 1.0],
                ..Default::default()
            };
            state.planet_id = scene.spawn(planet, Some(state.sun_id));

            let moon = Object {
                name: "Moon".to_string(),
                transform: Transform::from_position(1.5, 0.0, 0.0),
                geometry: Some(Geometry::Sphere { radius: 0.3, subdivisions: 16 }),
                color: [0.7, 0.7, 0.7, 1.0],
                ..Default::default()
            };
            scene.spawn(moon, Some(state.planet_id));
        })
        .on_update(|state, scene, ctx| {
            scene.camera.handle_default_input(&state.pressed_keys, 3.0, ctx);

            if let Some(sun) = scene.world.get_mut(state.sun_id) {
                sun.transform.rotation[1] += 30.0 * ctx.dt;
            }
            if let Some(planet) = scene.world.get_mut(state.planet_id) {
                planet.transform.rotation[1] += 100.0 * ctx.dt;
            }
        })
        .create();
}
```

---

## Architecture Overview

### Module Map

| Module | Purpose |
|---|---|
| `camera` | Perspective camera: eye/target/up, FOV, clip planes, builder setters, WASD helper |
| `scene` | Root scene container — spawn, texture, VTR save/load, editor integration |
| `world` | Scene-graph — object storage, hierarchy mutations, string/integer ID cache, change events |
| `objects` | `Object` struct — the fundamental scene-graph node (transform, geometry, colour, texture path) |
| `geometry` | Procedural mesh primitives — `Cube`, `Box`, `Plane`, `Pyramid`, `Sphere`, `Capsule` |
| `transform` | TRS transform — position/rotation/scale, matrix conversion, point transformation |
| `mesh` | CPU mesh builder (`MeshData`) and GPU baked mesh (`BakedMesh`) |
| `math` | Column-major `Matrix4` — identity, perspective, look-at, point projection |
| `timer` | Simple countdown timer for use in game logic |
| `window` | Builder-pattern windowing and event-loop host with typed callbacks |
| `editor` | Static scene editor — orbit cam, gizmos, multi-select, inspector |
| `vtr` | Binary `.vtr` scene format — read/write for camera + full object hierarchy |
| `constants` | Engine-wide default values |
| `event` | Re-exports of winit event types |

### Scene Graph

Objects form a tree. Each `Object` stores its parent's and children's integer IDs.
During rendering the engine traverses the tree recursively, combining parent and child
`Transform` matrices so that children automatically inherit position, rotation, and scale.

The `World` type manages the graph and exposes safe mutation methods:
* `spawn_object(object, parent_id)` — insert; unknown parent falls back to root.
* `delete(id)` — remove an object and all its descendants.
* `reparent(id, new_parent)` — move an object in the hierarchy with cycle detection.
* `get_id(str_id)` — resolve a stable string handle to an integer ID (call once, cache the result).
* `on_scene_graph_modified` — optional callback fired after every structural mutation.

### Coordinate System

Y-up, left-handed. The default camera looks along +Z. All rotation angles are in degrees
(Euler, Y → X → Z order).

### Rendering Pipeline

Geometry is **baked** each frame: the scene tree is walked, all object meshes are assembled
into `MeshData` builders grouped by `texture_path`, then uploaded to the GPU as a small
number of batched draw calls. The editor gizmo overlay is rendered as a separate pass.

### Built-in Editor

Enable with `scene.enable_editor_mode()` in `on_startup`. While active:
* `on_update`, `on_fixed_update`, and `on_draw_request` are suppressed.
* Orbit (Alt+drag), pan (middle-drag), and zoom (scroll wheel) control the camera.
* `T` / `R` / `E` switch between translate, rotate, and scale gizmos.
* Left-click picks objects; Ctrl+click multi-selects; `G` selects a subtree.
* `F` focuses the camera on the selection.
* `Escape` exits editor mode and returns to play mode.

Use `Window::on_editor_event` to react to gizmo-mode changes, drag start/end, and selection
changes while the editor is active.

### VTR Binary Format

`.vtr` files store the full camera state and scene hierarchy in a compact little-endian binary
layout (~84 bytes minimum for an empty scene). Use `scene.save_vtr_file` / `scene.load_vtr_file`
on native, or `vtr::write` / `vtr::read` directly on any `Write`/`Read` impl.

---

## License

Copyright 2026 xCirno Labs.

Licensed under the Apache License, Version 2.0.  
http://www.apache.org/licenses/LICENSE-2.0

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion
in the work by you shall be licensed as above, without any additional terms or conditions.
