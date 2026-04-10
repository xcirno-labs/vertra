use wasm_bindgen::prelude::*;
use serde::Deserialize;
use vertra::camera::Camera as CoreCamera;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "JsCameraOptions")]
    pub type JsCameraOptions;
}

#[wasm_bindgen(typescript_custom_section)]
const TS_CAMERA_CONTENT: &'static str = r#"
/**
 * Configuration options for initializing a new Camera instance.
 */
export interface JsCameraOptions {
/** The aspect ratio of the viewport (width / height). */
    aspect?: number;
    /** Vertical field of view in degrees. */
    fov?: number;
    /** Distance to the near clipping plane. */
    znear?: number;
    /** Distance to the far clipping plane. */
    zfar?: number;
    /** Initial left-right (yaw) rotation in degrees. */
    lr_rot?: number;
    /** Initial up-down (pitch) rotation in degress. */
    ud_rot?: number;
    /** The initial [x, y, z] position of the camera in world space. */
    position?: [number, number, number];
}
"#;

#[derive(Deserialize)]
pub struct CameraConstructorOptions {
    pub aspect: Option<f32>,
    pub fov: Option<f32>,
    pub znear: Option<f32>,
    pub zfar: Option<f32>,
    pub lr_rot: Option<f32>,
    pub ud_rot: Option<f32>,
    pub position: Option<[f32; 3]>,
}

/// A 3D camera controller for navigating world space.
/// Manages projection matrices, position, and orientation.
#[wasm_bindgen]
pub struct Camera {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreCamera,
    #[wasm_bindgen(skip)]
    pub owned: bool,
}

#[wasm_bindgen]
impl Camera {
    /// Creates a new Camera instance.
    /// @param {JsCameraOptions} options - Initial configuration for the camera.
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsCameraOptions) -> Self {
        let mut camera = Box::new(CoreCamera::new());
        let val: JsValue = options.into();

        if !val.is_undefined() && !val.is_null() {
            if let Ok(opts) = serde_wasm_bindgen::from_value::<CameraConstructorOptions>(val) {
                if let Some(a) = opts.aspect { camera.aspect = a; }
                if let Some(f) = opts.fov { camera.fov = f; }
                if let Some(zn) = opts.znear { camera.znear = zn; }
                if let Some(zf) = opts.zfar { camera.zfar = zf; }

                // Handle rotation
                if let (Some(lr), Some(ud)) = (opts.lr_rot, opts.ud_rot) {
                    *camera = camera.with_rotation(lr, ud);
                }

                // New: Handle position directly in constructor
                if let Some(pos) = opts.position {
                    camera.eye = pos;
                }
            }
        }

        Self { inner: Box::into_raw(camera), owned: true }
    }

    /// Updates the aspect ratio of the camera.
    /// Typically called when the window or canvas is resized.
    /// @param {number} aspect - The aspect ratio in decimal.
    pub fn set_aspect(&mut self, aspect: f32) {
        unsafe {
            (*self.inner).aspect = aspect;
        }
    }

    /// Rotates the camera based on screen-space movement deltas.
    /// @param {number} dx - Relative mouse movement on the X axis.
    /// @param {number} dy - Relative mouse movement on the Y axis.
    /// @param {boolean} inverted - Whether to invert the vertical look axis.
    pub fn rotate(&mut self, dx: f32, dy: f32, inverted: bool) {
        unsafe {
            (*self.inner).rotate(dx, dy, inverted);
        }
    }

    /// Moves the camera in a specific 3D direction.
    /// @param {Float32Array | number[]} direction - A 3-element array representing the direction vector.
    /// @param {number} amount - The distance to move.
    pub fn move_by(&mut self, direction: Vec<f32>, amount: f32) -> Result<(), JsError> {
        if direction.len() != 3 {
            return Err(JsError::new("Direction must be an array of 3 numbers"));
        }
        unsafe {
            (*self.inner).move_by(
                [direction[0], direction[1], direction[2]],
                amount
            );
        }
        Ok(())
    }

    /// Processes keyboard input to move the camera.
    /// Recognizes WASD and Arrow keys.
    /// @param {string[]} pressed_keys - An array of active key strings (e.g., from KeyboardEvent.code).
    /// @param {number} speed - Movement units per second.
    /// @param {number} dt - Delta time since the last frame in seconds.
    pub fn handle_input_default(&mut self, pressed_keys: Vec<String>, speed: f32, dt: f32) {
        unsafe {
            let (f, r) = (*self.inner).get_directions();
            let mut move_dir = [0.0, 0.0, 0.0];

            for key in pressed_keys {
                match key.as_ref() {
                    "KeyW" | "w" | "ArrowUp" => {
                        move_dir[0] += f[0]; move_dir[1] += f[1]; move_dir[2] += f[2];
                    }
                    "KeyS" | "s" | "ArrowDown" => {
                        move_dir[0] -= f[0]; move_dir[1] -= f[1]; move_dir[2] -= f[2];
                    }
                    "KeyD" | "d" | "ArrowRight" => {
                        move_dir[0] += r[0]; move_dir[1] += r[1]; move_dir[2] += r[2];
                    }
                    "KeyA" | "a" | "ArrowLeft" => {
                        move_dir[0] -= r[0]; move_dir[1] -= r[1]; move_dir[2] -= r[2];
                    }
                    _ => {}
                }
            }
            (*self.inner).move_by(move_dir, speed * dt);
        }
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        if self.owned && !self.inner.is_null() {
            unsafe { let _ = Box::from_raw(self.inner); }
        }
    }
}
