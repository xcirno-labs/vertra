use wasm_bindgen::prelude::*;
use vertra::objects::{Object as CoreObject, ObjectConstructor};
use crate::geometry::Geometry;
use crate::transform::Transform;
use serde::Deserialize;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "JsObjectOptions")]
    pub type JsObjectOptions;
}

#[wasm_bindgen(typescript_custom_section)]
const TS_OBJECT_CONTENT: &'static str = r#"
/**
 * Configuration options for initializing a new VertraObject.
 */
export interface JsObjectOptions {
    /** * A unique string identifier. If provided, this will be used for World lookups.
     * If omitted, a random UUID will be generated automatically.
     */
    str_id?: string;
    /** The initial color of the object [r, g, b, a]. */
    color?: [number, number, number, number];
}
"#;

#[derive(Deserialize, Default)]
struct InternalObjectOptions {
    str_id: Option<String>,
    color: Option<[f32; 4]>,
}

/// Represents a node in the 3D scene graph.
/// Objects hold a name, a transform (position/rotation/scale), and optional geometry/color data.
#[wasm_bindgen(js_name = VertraObject)]
pub struct Object {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreObject,
    #[wasm_bindgen(skip)]
    pub owned: bool,
}

#[wasm_bindgen(js_name = VertraObject)]
impl Object {
    /// Creates a new scene object.
    /// @param {string} name - The display name.
    /// @param {JsObjectOptions} [options] - Initial configuration (str_id, color, etc).
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, options: Option<JsValue>) -> Self {
        let opts: InternalObjectOptions = options
            .and_then(|val| {
                if val.is_undefined() || val.is_null() {
                    None
                } else {
                    serde_wasm_bindgen::from_value(val).ok()
                }
            })
            .unwrap_or_default();
        
        let core_obj = CoreObject::new(ObjectConstructor {
            name,
            color: opts.color,
            str_id: opts.str_id,
            transform: None,
            geometry: None,
        });

        Self {
            inner: Box::into_raw(Box::new(core_obj)),
            owned: true,
        }
    }
    /// Sets the name of the object.
    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: String) {
        unsafe {(*self.inner).name = name}
    }

    /// Gets the current name of the object.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        unsafe { (*self.inner).name.clone() }
    }

    /// Updates the object's spatial properties (position, rotation, scale).
    /// @param {Transform} transform - The new transform state.
    #[wasm_bindgen(setter)]
    pub fn set_transform(&mut self, transform: &Transform) {
        unsafe {
            // This updates the memory DIRECTLY inside the HashMap
            (*self.inner).transform = transform.inner.clone();
        }
    }

    /// Returns a copy of the object's current transform.
    #[wasm_bindgen(getter)]
    pub fn transform(&self) -> Transform {
        unsafe {
            Transform { inner: (*self.inner).transform.clone() }
        }
    }

    /// Sets the RGBA color of the object.
    /// @param {Float32Array | number[]} color - An array of 4 numbers [r, g, b, a] ranging from 0.0 to 1.0.
    pub fn set_color(&mut self, color: Vec<f32>) {
        unsafe {
            if color.len() == 4 {
                (*self.inner).color = [color[0], color[1], color[2], color[3]];
            }
        }
    }

    /// Attaches a mesh geometry to this object for rendering.
    /// @param {Geometry} geometry - The geometry to be applied.
    pub fn set_geometry(&mut self, geometry: &Geometry) {
        unsafe {
            (*self.inner).geometry = Some(geometry.inner.clone());
        }
    }

    /// Returns the ID of the parent object, if one exists.
    #[wasm_bindgen(getter)]
    pub fn parent(&self) -> Option<usize> {
        unsafe {
            (*self.inner).parent
        }
    }

    /// Returns the unique string identifier for this object.
    /// This is assigned at creation and cannot be changed.
    #[wasm_bindgen(getter)]
    pub fn str_id(&self) -> String {
        unsafe { (*self.inner).str_id.clone() }
    }

    /// Returns the number of direct children attached to this object.
    #[wasm_bindgen(getter)]
    pub fn children_count(&self) -> usize {
        unsafe {
            (*self.inner).children.len()
        }
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        if self.owned && !self.inner.is_null() {
            unsafe {
                let _ = Box::from_raw(self.inner);
            }
        }
    }
}
