use wasm_bindgen::prelude::*;
use vertra::camera::Camera as CoreCamera;
use serde::Deserialize;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "JsCameraOptions")]
    pub type JsCameraOptions;
}

#[wasm_bindgen(typescript_custom_section)]
const TS_CAMERA_CONTENT: &'static str = r#"
/** Configuration options for initialising a new `Camera` instance. */
export interface JsCameraOptions {
    /** The aspect ratio of the viewport (`width / height`). */
    aspect?: number;
    /** Vertical field of view in degrees. */
    fov?: number;
    /** Distance to the near clipping plane. */
    znear?: number;
    /** Distance to the far clipping plane. */
    zfar?: number;
    /** Initial left-right (yaw) rotation in degrees. */
    lr_rot?: number;
    /** Initial up-down (pitch) rotation in degrees. */
    ud_rot?: number;
    /** Initial world-space position as `[x, y, z]`. */
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

/// A 3D camera that controls the viewpoint and projection used to render the scene.
///
/// Manages the view and projection matrices, world-space position, and look direction.
/// The engine owns one camera per [`Scene`]; obtain it via [`Scene::camera`].
#[wasm_bindgen]
pub struct Camera {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreCamera,
    #[wasm_bindgen(skip)]
    pub owned: bool,
}

#[wasm_bindgen]
impl Camera {
    /// Creates a new `Camera` from optional configuration values.
    ///
    /// Any omitted fields fall back to engine defaults (aspect `1.0`, fov `45°`,
    /// znear `0.1`, zfar `1000.0`).
    ///
    /// # Arguments
    ///
    /// * `options` - A [`JsCameraOptions`] object; all fields are optional.
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

                if let (Some(lr), Some(ud)) = (opts.lr_rot, opts.ud_rot) {
                    *camera = camera.with_rotation(lr, ud);
                }

                if let Some(pos) = opts.position {
                    camera.eye = pos;
                }
            }
        }

        Self { inner: Box::into_raw(camera), owned: true }
    }

    /// Updates the aspect ratio used by the projection matrix.
    ///
    /// Call this whenever the canvas or window is resized so that the projection
    /// stays geometrically correct.
    ///
    /// # Arguments
    ///
    /// * `aspect` - The new aspect ratio (`viewport_width / viewport_height`).
    pub fn set_aspect(&mut self, aspect: f32) {
        unsafe {
            (*self.inner).aspect = aspect;
        }
    }

    /// Rotates the camera by applying yaw and pitch deltas.
    ///
    /// Typically called with the raw `movementX` / `movementY` values from a
    /// browser `mousemove` event.
    ///
    /// # Arguments
    ///
    /// * `dx`       - Horizontal mouse delta in pixels (positive = look right).
    /// * `dy`       - Vertical mouse delta in pixels (positive = look down).
    /// * `inverted` - When `true` the pitch direction is flipped (inverted Y).
    pub fn rotate(&mut self, dx: f32, dy: f32, inverted: bool) {
        unsafe {
            (*self.inner).rotate(dx, dy, inverted);
        }
    }

    /// Moves the camera along an arbitrary world-space direction vector.
    ///
    /// # Arguments
    ///
    /// * `direction` - A 3-element `[x, y, z]` direction vector.
    ///   Does not need to be normalised.
    /// * `amount` - Distance to travel in world units.
    ///
    /// # Errors
    ///
    /// Returns a [`JsError`] when `direction` does not contain exactly 3 elements.
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

    /// Moves the camera using keyboard input from a set of currently-held keys.
    ///
    /// Recognises WASD and Arrow key codes as returned by `KeyboardEvent.code`.
    /// This is a convenience helper for play-mode first-person navigation;
    /// the editor uses its own internal WASD handler.
    ///
    /// # Arguments
    ///
    /// * `pressed_keys` - Slice of active key-code strings
    ///   (e.g. `["KeyW", "ShiftLeft"]`).
    /// * `speed` - Movement speed in world units per second.
    /// * `dt`    - Delta time since the last frame, in seconds.
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
