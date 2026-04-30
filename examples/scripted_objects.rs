//! # scripted_objects
//!
//! Demonstrates the **per-object script** system.
//!
//! Five objects are spawned, each with a different script showing a different
//! aspect of the API:
//!
//! | Object            | Script            | What it does                                        |
//! |-------------------|-------------------|-----------------------------------------------------|
//! | Spinning cube     | `RotateY`         | Y-rotation every `on_update`                        |
//! | Bobbing sphere    | `BobY`            | Sine-wave Y translation using internal phase state  |
//! | Pulse sphere      | `PulseScale`      | Uniform scale pulses via `on_fixed_update`          |
//! | Color-cycle plane | `ColorCycle`      | Hue-cycles RGBA color each frame                    |
//! | Logger cube       | `StartLogger`     | Prints once in `on_start`, then silently continues  |
//!
//! Press **Escape** to toggle between editor mode and play mode.
//! Scripts only run during **play mode**.
//!
//! Run with:
//! ```
//! cargo run --example scripted_objects
//! ```

use std::collections::HashSet;
use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::scene::Scene;
use vertra::script::ObjectScript;
use vertra::transform::Transform;
use vertra::window::{FrameContext, Window};
use vertra::world::World;
use vertra::event::{Event, WindowEvent, DeviceEvent, ElementState};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Continuously rotates an object around the Y axis.
struct RotateY {
    /// Rotation speed in degrees per second.
    speed_deg: f32,
}

impl ObjectScript for RotateY {
    fn on_update(&mut self, id: usize, world: &mut World, dt: f32) {
        if let Some(obj) = world.get_mut(id) {
            obj.transform.rotation[1] += self.speed_deg * dt;
        }
    }
}

/// Moves an object up and down using a sine wave.
struct BobY {
    /// Amplitude of the sine wave in world units.
    amplitude: f32,
    /// Oscillation frequency in Hz.
    frequency: f32,
    /// Accumulated time used as phase input.
    time: f32,
    /// Y position at spawn time — restored each frame to avoid drift.
    base_y: f32,
}

impl BobY {
    fn new(amplitude: f32, frequency: f32) -> Self {
        Self { amplitude, frequency, time: 0.0, base_y: 0.0 }
    }
}

impl ObjectScript for BobY {
    fn on_start(&mut self, id: usize, world: &mut World) {
        // Capture the object's initial Y so we oscillate around it.
        if let Some(obj) = world.get_mut(id) {
            self.base_y = obj.transform.position[1];
        }
        println!("[BobY] on_start for id={id}, base_y={}", self.base_y);
    }

    fn on_update(&mut self, id: usize, world: &mut World, dt: f32) {
        self.time += dt;
        let offset = self.amplitude * (self.time * self.frequency * std::f32::consts::TAU).sin();
        if let Some(obj) = world.get_mut(id) {
            obj.transform.position[1] = self.base_y + offset;
        }
    }
}

/// Pulses the uniform scale of an object in sync with the fixed timestep.
struct PulseScale {
    /// Base scale (resting size).
    base_scale: f32,
    /// Half-amplitude of the pulse.
    amplitude: f32,
    /// Oscillation frequency in Hz.
    frequency: f32,
    /// Accumulated fixed-step time.
    time: f32,
}

impl PulseScale {
    fn new(base_scale: f32, amplitude: f32, frequency: f32) -> Self {
        Self { base_scale, amplitude, frequency, time: 0.0 }
    }
}

impl ObjectScript for PulseScale {
    fn on_fixed_update(&mut self, id: usize, world: &mut World, dt: f32) {
        self.time += dt;
        let s = self.base_scale
            + self.amplitude * (self.time * self.frequency * std::f32::consts::TAU).sin();
        if let Some(obj) = world.get_mut(id) {
            obj.transform.scale = [s, s, s];
        }
    }
}

/// Cycles the RGBA colour of an object through the hue spectrum.
struct ColorCycle {
    /// Current hue in [0.0, 1.0).
    hue: f32,
    /// Hue advance per second.
    speed: f32,
}

impl ColorCycle {
    fn new(speed: f32) -> Self {
        Self { hue: 0.0, speed }
    }
}

/// Converts a hue (0–1) to an RGB triple using the classic 6-sector formula.
fn hue_to_rgb(h: f32) -> [f32; 3] {
    let h6 = h * 6.0;
    let i  = h6 as u32;
    let f  = h6 - i as f32;
    match i % 6 {
        0 => [1.0, f,   0.0],
        1 => [1.0 - f, 1.0, 0.0],
        2 => [0.0, 1.0, f  ],
        3 => [0.0, 1.0 - f, 1.0],
        4 => [f,   0.0, 1.0],
        _ => [1.0, 0.0, 1.0 - f],
    }
}

impl ObjectScript for ColorCycle {
    fn on_update(&mut self, id: usize, world: &mut World, dt: f32) {
        self.hue = (self.hue + self.speed * dt).fract();
        let [r, g, b] = hue_to_rgb(self.hue);
        if let Some(obj) = world.get_mut(id) {
            obj.color = [r, g, b, 1.0];
        }
    }
}

/// Prints a message in `on_start`, then does nothing else.
/// Useful for verifying that `on_start` fires exactly once.
struct StartLogger {
    label: String,
}

impl ObjectScript for StartLogger {
    fn on_start(&mut self, id: usize, _world: &mut World) {
        println!("[StartLogger] \"{}\" (id={id}) — on_start fired!", self.label);
    }
}

struct AppState {
    keys: HashSet<KeyCode>,
}

fn main() {
    Window::new(AppState { keys: HashSet::new() })
        .with_title("Scripted Objects")
        .with_camera(
            Camera::new()
                .with_position([0.0, 4.0, -14.0])
                .with_rotation(90.0, -15.0),
        )
        .on_startup(|_state, scene, _ctx| {
            spawn_scene(scene);
            scene.enable_editor_mode();
        })
        .on_update(|state, scene, ctx| {
            if scene.editor.is_none() {
                scene.camera.handle_default_input(&state.keys, 6.0, ctx);
            }
        })
        .with_event_handler(|state, scene, event, _| {
            handle_input(state, scene, event);
        })
        .create();
}

fn spawn_scene(scene: &mut Scene) {
    // 1. Spinning cube (RotateY)
    let cube_id = scene.spawn(
        Object {
            name:     "SpinningCube".into(),
            str_id:   "spin_cube".into(),
            transform: Transform::from_position(-6.0, 0.0, 0.0),
            geometry: Some(Geometry::Cube { size: 1.5 }),
            color:    [0.9, 0.3, 0.2, 1.0],
            ..Default::default()
        },
        None,
    );
    scene.attach_script(cube_id, Box::new(RotateY { speed_deg: 90.0 }));

    // 2. Bobbing sphere (BobY)
    let bob_id = scene.spawn(
        Object {
            name:     "BobbingSphere".into(),
            str_id:   "bob_sphere".into(),
            transform: Transform::from_position(-2.0, 0.0, 0.0),
            geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 20 }),
            color:    [0.2, 0.8, 0.3, 1.0],
            ..Default::default()
        },
        None,
    );
    scene.attach_script(bob_id, Box::new(BobY::new(1.5, 0.8)));

    // 3. Pulsing sphere (PulseScale via on_fixed_update)
    let pulse_id = scene.spawn(
        Object {
            name:     "PulseSphere".into(),
            str_id:   "pulse_sphere".into(),
            transform: Transform::from_position(2.0, 0.0, 0.0),
            geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 20 }),
            color:    [0.3, 0.5, 1.0, 1.0],
            ..Default::default()
        },
        None,
    );
    scene.attach_script(pulse_id, Box::new(PulseScale::new(1.0, 0.4, 1.5)));

    // 4. Color-cycling plane (ColorCycle)
    let plane_id = scene.spawn(
        Object {
            name:     "ColorPlane".into(),
            str_id:   "color_plane".into(),
            transform: Transform::from_position(6.0, 0.0, 0.0),
            geometry: Some(Geometry::Plane { size: 2.0 }),
            color:    [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        },
        None,
    );
    scene.attach_script(plane_id, Box::new(ColorCycle::new(0.4)));

    // 5. Logger cube (StartLogger)
    //    Spawned as a child of the spinning cube so it orbits around it.
    let logger_id = scene.spawn(
        Object {
            name:     "LoggerCube".into(),
            str_id:   "logger_cube".into(),
            transform: Transform::from_position(2.5, 0.0, 0.0),
            geometry: Some(Geometry::Cube { size: 0.5 }),
            color:    [1.0, 1.0, 0.2, 1.0],
            ..Default::default()
        },
        Some(cube_id),
    );
    scene.attach_script(logger_id, Box::new(StartLogger { label: "LoggerCube".into() }));

    println!(
        "[startup] Spawned {} objects with scripts.",
        scene.world.objects.len()
    );
}

fn handle_input(state: &mut AppState, scene: &mut vertra::scene::Scene, event: Event<()>) {
    match event {
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta: (dx, dy) }, ..
        } => {
            if scene.editor.is_none() {
                scene.camera.rotate(dx as f32 * 0.15, dy as f32 * 0.15, false);
            }
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event: ke, .. }, ..
        } => {
            if let PhysicalKey::Code(code) = ke.physical_key {
                if scene.editor.is_none() {
                    match ke.state {
                        ElementState::Pressed  => { state.keys.insert(code); }
                        ElementState::Released => { state.keys.remove(&code); }
                    }
                } else {
                    state.keys.clear();
                }

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
}

