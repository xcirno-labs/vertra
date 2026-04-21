//! # Fixed Update — Bouncing Ball
//!
//! Demonstrates [`Window::on_fixed_update`]: a sphere simulates simple
//! vertical physics (gravity + elastic ground bounce) using a **constant**
//! timestep so the simulation is framerate-independent.
//!
//! A second, purely visual cube spins in `on_update` (variable dt) for
//! comparison.  Both share the same scene, showing how `on_update` and
//! `on_fixed_update` coexist.
//!
//! **Run:**
//! ```sh
//! cargo run --example fixed_update
//! ```
//!
//! **How it works:**
//!
//! * `on_fixed_update` is called at a fixed 60 Hz cadence regardless of the
//!   rendering frame rate.  Use it for physics, AI ticks, or any logic that
//!   must not drift with framerate.
//! * `on_update` is called once per rendered frame with a variable `dt`.
//!   Use it for visuals and input handling.
//!
//! > **Note:** both callbacks are **suppressed** while editor mode is active.
//! > This example intentionally does *not* enable editor mode so you can watch
//! > the simulation run.

use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::transform::Transform;
use vertra::window::Window;

/// Simulation and render state.
struct AppState {
    /// Numeric ID of the bouncing sphere.
    ball_id: Option<usize>,
    /// Numeric ID of the spinning cube.
    cube_id: Option<usize>,
    /// Current vertical velocity of the ball (m/s, world-space).
    ball_vy: f32,
}

fn main() {
    Window::new(AppState {
        ball_id: None,
        cube_id: None,
        ball_vy: 8.0, // initial upward kick
    })
    .with_title("Fixed Update — Bouncing Ball")
    .with_camera(
        Camera::new()
            .with_position([0.0, 3.0, -8.0])
            .with_rotation(90.0, -15.0),
    )
    .on_startup(|state, scene, _| {
        // Bouncing sphere
        let ball_id = scene.spawn(
            Object {
                name: "Ball".to_string(),
                str_id: "ball".to_string(),
                geometry: Some(Geometry::Sphere {
                    radius: 0.5,
                    subdivisions: 20,
                }),
                color: [0.3, 0.7, 1.0, 1.0],
                transform: Transform::from_position(0.0, 4.0, 0.0),
                ..Default::default()
            },
            None,
        );

        // Ground plane
        scene.spawn(
            Object {
                name: "Ground".to_string(),
                str_id: "ground".to_string(),
                geometry: Some(Geometry::Plane { size: 12.0 }),
                color: [0.3, 0.6, 0.3, 1.0],
                transform: Transform::from_position(0.0, 0.0, 0.0),
                ..Default::default()
            },
            None,
        );

        // Spinning cube for on_update comparison
        let cube_id = scene.spawn(
            Object {
                name: "Spinner".to_string(),
                str_id: "spinner".to_string(),
                geometry: Some(Geometry::Cube { size: 1.0 }),
                color: [1.0, 0.5, 0.2, 1.0],
                transform: Transform::from_position(3.5, 1.0, 0.0),
                ..Default::default()
            },
            None,
        );

        state.ball_id = Some(ball_id);
        state.cube_id = Some(cube_id);
    })
    // ── Physics tick (fixed 60 Hz) ────────────────────────────────────────────
    .on_fixed_update(|state, scene, ctx| {
        const GRAVITY: f32 = -12.0;       // m/s²
        const RESTITUTION: f32 = 0.78;    // energy retained on bounce (0–1)
        const GROUND_Y: f32 = 0.5;        // ball radius — lowest allowed centre

        if let Some(id) = state.ball_id {
            // Integrate gravity.
            state.ball_vy += GRAVITY * ctx.dt;

            if let Some(ball) = scene.world.get_mut(id) {
                ball.transform.position[1] += state.ball_vy * ctx.dt;

                // Ground collision: reflect and damp.
                if ball.transform.position[1] < GROUND_Y {
                    ball.transform.position[1] = GROUND_Y;
                    state.ball_vy = state.ball_vy.abs() * RESTITUTION;

                    // Stop micro-bounces that would never settle.
                    if state.ball_vy < 0.2 {
                        state.ball_vy = 0.0;
                    }
                }
            }
        }
    })
    // ── Visual update (variable dt) ───────────────────────────────────────────
    .on_update(|state, scene, ctx| {
        // Spin the reference cube at 90 °/s — purely cosmetic.
        if let Some(spinner) = state.cube_id.and_then(|id| scene.world.get_mut(id)) {
            spinner.transform.rotation[1] += 90.0 * ctx.dt;
            spinner.transform.rotation[0] += 45.0 * ctx.dt;
        }
    })
    .create();
}

