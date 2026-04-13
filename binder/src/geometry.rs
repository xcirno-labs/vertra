use wasm_bindgen::prelude::*;
use vertra::geometry::Geometry as CoreGeometry;

/// Represents a 3D mesh definition that can be attached to a scene object for rendering.
#[wasm_bindgen]
pub struct Geometry {
    pub(crate) inner: CoreGeometry,
}

#[wasm_bindgen]
impl Geometry {
    /// Creates a cube where all sides are equal in length.
    ///
    /// # Arguments
    ///
    /// * `size` - The length of each side of the cube in world units.
    #[wasm_bindgen]
    pub fn cube(size: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Cube { size } }
    }

    /// Creates a rectangular box with independent dimensions on each axis.
    ///
    /// # Arguments
    ///
    /// * `width`  - Size along the X-axis.
    /// * `height` - Size along the Y-axis.
    /// * `depth`  - Size along the Z-axis.
    #[wasm_bindgen(js_name = box)]
    pub fn box_geo(width: f32, height: f32, depth: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Box { width, height, depth } }
    }

    /// Creates a flat, square surface lying on the XZ plane.
    ///
    /// # Arguments
    ///
    /// * `size` - The side length of the square plane in world units.
    #[wasm_bindgen]
    pub fn plane(size: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Plane { size } }
    }

    /// Creates a spherical mesh.
    ///
    /// # Arguments
    ///
    /// * `radius`       - Distance from the centre to the surface in world units.
    /// * `subdivisions` - Smoothness level; higher values produce more triangles.
    #[wasm_bindgen]
    pub fn sphere(radius: f32, subdivisions: usize) -> Geometry {
        Geometry { inner: CoreGeometry::Sphere { radius, subdivisions } }
    }

    /// Creates a four-sided pyramid with a square base.
    ///
    /// # Arguments
    ///
    /// * `base_size` - The side length of the square base in world units.
    /// * `height`    - Vertical distance from the base to the apex.
    #[wasm_bindgen]
    pub fn pyramid(base_size: f32, height: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Pyramid { base_size, height } }
    }
}