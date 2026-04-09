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
export interface JsTransformOptions {
    position?: [number, number, number];
    rotation?: [number, number, number];
    scale?: [number, number, number];
}
"#;

/// Manages spatial data including position, rotation, and scale.
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
    /// Creates a new Transform.
    /// @param {ITransformOptions} options - Initial spatial values.
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

    /// Returns the [x, y, z] position.
    #[wasm_bindgen(getter)]
    pub fn position(&self) -> Vec<f32> {
        self.inner.position.to_vec()
    }

    /// Sets the position. Expects a 3-element array.
    #[wasm_bindgen(setter)]
    pub fn set_position(&mut self, val: Vec<f32>) {
        if val.len() == 3 {
            self.inner.position = [val[0], val[1], val[2]];
        }
    }

    /// Returns the [x, y, z] Euler rotation.
    #[wasm_bindgen(getter)]
    pub fn rotation(&self) -> Vec<f32> {
        self.inner.rotation.to_vec()
    }

    /// Sets the rotation. Expects a 3-element array of Euler angles (radians).
    #[wasm_bindgen(setter)]
    pub fn set_rotation(&mut self, val: Vec<f32>) {
        if val.len() == 3 {
            self.inner.rotation = [val[0], val[1], val[2]];
        }
    }

    /// Returns the [x, y, z] scale factors.
    #[wasm_bindgen(getter)]
    pub fn scale(&self) -> Vec<f32> {
        self.inner.scale.to_vec()
    }

    /// Sets the scale. Expects a 3-element array.
    #[wasm_bindgen(setter)]
    pub fn set_scale(&mut self, val: Vec<f32>) {
        if val.len() == 3 {
            self.inner.scale = [val[0], val[1], val[2]];
        }
    }

    /// Combines this transform with a child transform, returning a new local-to-world result.
    /// Useful for calculating the absolute position of a nested object.
    /// @param {Transform} child - The local transform to apply to this parent transform.
    pub fn combine_wasm(&self, child: &Transform) -> Transform {
        // Since CoreTransform implements Clone, we can just use the inner value
        let combined = self.inner.combine(&child.inner);
        Self { inner: combined }
    }
}