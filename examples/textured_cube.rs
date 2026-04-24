//! # Textured Cube
//!
//! Demonstrates loading a PNG texture from disk and applying it to a rotating
//! cube.  A second, untextured sphere is placed nearby so you can compare the
//! two rendering modes side-by-side.
//!
//! The texture file must exist at `examples/assets/texture.png` relative to
//! the workspace root.  The cube's vertex colour is `[1.0, 1.0, 1.0, 1.0]`
//! (white) so the texture is displayed at full fidelity; tinting the colour
//! changes the texture's hue.
//!
//! **Run:**
//! ```sh
//! cargo run --example textured_cube
//! ```
//!
//! **Controls (editor mode):**
//! | Key / mouse       | Action                          |
//! |-------------------|---------------------------------|
//! | Alt + left-drag   | Orbit camera                    |
//! | Scroll wheel      | Zoom                            |
//! | Middle-drag       | Pan                             |
//! | W / A / S / D     | Fly camera                      |
//! | T / R / E         | Translate / Rotate / Scale gizmo|
//! | F                 | Focus on selected object        |
//! | Escape            | Switch to play mode             |

use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::transform::Transform;
use vertra::window::Window;

const TEXTURE_PATH: &str = "examples/assets/texture.png";

struct AppState {
    cube_id: Option<usize>,
}

fn main() {
    Window::new(AppState { cube_id: None })
        .with_title("Textured Cube — Vertra")
        .with_camera(
            Camera::new()
                .with_position([0.0, 2.5, -6.0])
                .with_rotation(90.0, -15.0),
        )
        .on_startup(|state, scene, _| {
            // ----------------------------------------------------------------
            // 1.  Load the texture from disk.
            //     The key must match the `texture_path` set on the object.
            // ----------------------------------------------------------------
            #[cfg(not(target_arch = "wasm32"))]
            match scene.load_texture(TEXTURE_PATH) {
                Ok(()) => println!("[info] Texture loaded: {TEXTURE_PATH}"),
                Err(e) => eprintln!("[warn] {e}  – cube will render with vertex colour"),
            }

            // ----------------------------------------------------------------
            // 2.  Textured cube (white vertex colour so the image shows cleanly)
            // ----------------------------------------------------------------
            let cube_id = scene.spawn(
                Object {
                    name: "Textured Cube".to_string(),
                    str_id: "cube".to_string(),
                    geometry: Some(Geometry::Cube { size: 2.0 }),
                    // White vertex colour = texture displayed without tint.
                    // Change this to tint the texture (e.g. [1.0, 0.5, 0.5, 1.0] = reddish).
                    color: [1.0, 1.0, 1.0, 1.0],
                    transform: Transform::from_position(0.0, 1.0, 0.0),
                    texture_path: Some(TEXTURE_PATH.to_string()),
                    ..Default::default()
                },
                None,
            );
            state.cube_id = Some(cube_id);
        })
        .on_update(|state, scene, ctx| {
            // Slowly rotate the cube so all faces of the texture are visible.
            if let Some(cube) = state.cube_id.and_then(|id| scene.world.get_mut(id)) {
                cube.transform.rotation[1] += 40.0 * ctx.dt; // 40°/s around Y
                cube.transform.rotation[0] += 15.0 * ctx.dt; // 15°/s around X
            }
        })
        .create();
}

