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
/** Configuration options for initialising a new `VertraObject`. */
export interface JsObjectOptions {
    /**
     * A stable string identifier used for world lookups via `World.get_id`.
     * If omitted, a random UUID is generated automatically.
     */
    str_id?: string;
    /** Initial RGBA colour of the object as `[r, g, b, a]` in the range `0.0–1.0`. */
    color?: [number, number, number, number];
}
"#;

#[derive(Deserialize, Default)]
struct InternalObjectOptions {
    str_id: Option<String>,
    color: Option<[f32; 4]>,
    texture_path: Option<String>,
}

/// Represents a node in the 3D scene graph.
///
/// Objects hold a name, a [`Transform`] (position / rotation / scale), and
/// optional geometry and colour data.  Spawn them into a scene with
/// [`Scene::spawn`] or [`World::spawn_object`].
#[wasm_bindgen(js_name = VertraObject)]
pub struct Object {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreObject,
    #[wasm_bindgen(skip)]
    pub owned: bool,
}

#[wasm_bindgen(js_name = VertraObject)]
impl Object {
    /// Creates a new scene object template.
    ///
    /// The object is not yet part of any scene; call [`Scene::spawn`] or
    /// [`World::spawn_object`] to add it to the world.
    ///
    /// # Arguments
    ///
    /// * `name`    - Human-readable display name shown in the inspector.
    /// * `options` - Optional [`JsObjectOptions`] with `str_id` and/or `color`.
    ///   Pass `undefined` or `null` to use defaults (random UUID, white colour).
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
            texture_path: opts.texture_path,
        });

        Self {
            inner: Box::into_raw(Box::new(core_obj)),
            owned: true,
        }
    }

    /// Sets the display name of the object.
    #[wasm_bindgen(setter)]
    pub fn set_name(&mut self, name: String) {
        unsafe { (*self.inner).name = name }
    }

    /// Returns the current display name of the object.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        unsafe { (*self.inner).name.clone() }
    }

    /// Updates the object's spatial properties (position, rotation, and scale).
    ///
    /// For objects fetched from the world via [`World::get_object`] this mutates
    /// the live world-backed data.  For JS-owned objects created with `new` it
    /// mutates the standalone template before it is spawned.
    ///
    /// # Arguments
    ///
    /// * `transform` - The new transform state to apply.
    #[wasm_bindgen(setter)]
    pub fn set_transform(&mut self, transform: &Transform) {
        unsafe {
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

    /// Sets the RGBA colour of the object.
    ///
    /// # Arguments
    ///
    /// * `color` - A 4-element `[r, g, b, a]` array with values in `0.0 ..= 1.0`.
    ///   Silently ignored when the slice does not contain exactly 4 elements.
    pub fn set_color(&mut self, color: Vec<f32>) {
        unsafe {
            if color.len() == 4 {
                (*self.inner).color = [color[0], color[1], color[2], color[3]];
            }
        }
    }

    /// Attaches a mesh geometry to this object for rendering.
    ///
    /// # Arguments
    ///
    /// * `geometry` - The geometry variant to attach (cube, sphere, plane, etc.).
    pub fn set_geometry(&mut self, geometry: &Geometry) {
        unsafe {
            (*self.inner).geometry = Some(geometry.inner.clone());
        }
    }

    /// Returns the integer ID of the parent object, or `undefined` if this is a root object.
    #[wasm_bindgen(getter)]
    pub fn parent(&self) -> Option<usize> {
        unsafe { (*self.inner).parent }
    }

    /// Returns the stable string identifier assigned at creation time.
    ///
    /// This value cannot be changed after construction.  Use it with
    /// [`World::get_id`] to resolve back to the integer ID at runtime.
    #[wasm_bindgen(getter)]
    pub fn str_id(&self) -> String {
        unsafe { (*self.inner).str_id.clone() }
    }

    /// Returns the number of direct children attached to this object.
    #[wasm_bindgen(getter)]
    pub fn children_count(&self) -> usize {
        unsafe { (*self.inner).children.len() }
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
