use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use vertra::transform::{Transform as CoreTransform};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TransformOptions")]
    pub type JsTransformOptions;
}

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
/** Spatial configuration passed to the `Transform` constructor. */
export interface TransformOptions {
    /** World-space position as `[x, y, z]`. Defaults to `[0, 0, 0]`. */
    position?: [number, number, number];
    /** Euler rotation in degrees as `[rx, ry, rz]`. Defaults to `[0, 0, 0]`. */
    rotation?: [number, number, number];
    /** Per-axis scale factors as `[sx, sy, sz]`. Defaults to `[1, 1, 1]`. */
    scale?: [number, number, number];
}
"#;

/// Manages the spatial state of a scene object: position, rotation, and scale.
///
/// Used to calculate local-to-world matrices and hierarchy transformations.
#[wasm_bindgen]
#[derive(Serialize, Deserialize, Clone)]
pub struct Transform {
    #[wasm_bindgen(skip)]
    pub inner: CoreTransform,
}

// We use a regular Rust struct for the options and let Serde handle the JS object
#[derive(Deserialize)]
pub struct TransformConstructorOptions {
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
}

#[wasm_bindgen]
impl Transform {
    /// Creates a new `Transform` with optional initial spatial values.
    ///
    /// Any omitted fields fall back to their defaults: zero position,
    /// zero rotation, and unit scale `[1, 1, 1]`.
    ///
    /// # Arguments
    ///
    /// * `options` - A [`TransformOptions`] object with optional `position`,
    ///   `rotation`, and `scale` fields.
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsTransformOptions) -> Self {
        let mut inner = CoreTransform::default();
        let val: JsValue = options.into();

        // If the user passed an object, try to parse it
        if !val.is_undefined() && !val.is_null() {
            if let Ok(opts) = serde_wasm_bindgen::from_value::<TransformConstructorOptions>(val) {
                if let Some(pos) = opts.position { inner.position = pos; }
                if let Some(rot) = opts.rotation { inner.rotation = rot; }
                if let Some(scl) = opts.scale { inner.scale = scl; }
            }
        }

        Self { inner }
    }

    /// Returns the world-space position as `[x, y, z]`.
    #[wasm_bindgen(getter)]
    pub fn position(&self) -> Vec<f32> {
        self.inner.position.to_vec()
    }

    /// Sets the position from a 3-element array.
    ///
    /// Silently ignored when the slice does not contain exactly 3 elements.
    #[wasm_bindgen(setter)]
    pub fn set_position(&mut self, val: Vec<f32>) {
        if val.len() == 3 {
            self.inner.position = [val[0], val[1], val[2]];
        }
    }

    /// Returns the Euler rotation as `[rx, ry, rz]` in degrees.
    #[wasm_bindgen(getter)]
    pub fn rotation(&self) -> Vec<f32> {
        self.inner.rotation.to_vec()
    }

    /// Sets the rotation from a 3-element array of Euler angles in degrees.
    ///
    /// Silently ignored when the slice does not contain exactly 3 elements.
    #[wasm_bindgen(setter)]
    pub fn set_rotation(&mut self, val: Vec<f32>) {
        if val.len() == 3 {
            self.inner.rotation = [val[0], val[1], val[2]];
        }
    }

    /// Returns the per-axis scale factors as `[sx, sy, sz]`.
    #[wasm_bindgen(getter)]
    pub fn scale(&self) -> Vec<f32> {
        self.inner.scale.to_vec()
    }

    /// Sets the scale from a 3-element array.
    ///
    /// Silently ignored when the slice does not contain exactly 3 elements.
    #[wasm_bindgen(setter)]
    pub fn set_scale(&mut self, val: Vec<f32>) {
        if val.len() == 3 {
            self.inner.scale = [val[0], val[1], val[2]];
        }
    }

    /// Composes this transform with a child transform, returning a new world-space result.
    ///
    /// Useful for computing the absolute position of a nested object given its
    /// local transform relative to a parent.
    ///
    /// # Arguments
    ///
    /// * `child` - The local-space transform to apply on top of this parent transform.
    ///
    /// # Returns
    ///
    /// A new [`Transform`] representing the composed world-space transformation.
    pub fn combine_wasm(&self, child: &Transform) -> Transform {
        let combined = self.inner.combine(&child.inner);
        Self { inner: combined }
    }
}