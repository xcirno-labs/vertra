use wasm_bindgen::prelude::*;
use vertra::objects::{Object as CoreObject};
use crate::geometry::Geometry;
use crate::transform::Transform;

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
    /// Creates a new scene object with a unique name.
    /// @param {string} name - The identifier for this object.
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> Self {
        let obj = Box::new(CoreObject {
            name,
            ..Default::default()
        });
        Self {
            inner: Box::into_raw(obj),
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
