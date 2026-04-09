use wasm_bindgen::prelude::*;
use vertra::geometry::Geometry as CoreGeometry;

/// Represents a 3D mesh definition that can be used to render objects.
#[wasm_bindgen]
pub struct Geometry {
    pub(crate) inner: CoreGeometry,
}

/// Creates a cube where all sides (width, height, and depth) are equal.
/// @param {number} size - The length of each side of the cube.
#[wasm_bindgen]
impl Geometry {
    /// Creates a cube where all sides (width, height, and depth) are equal.
    /// @param {number} size - The length of each side of the cube.
    #[wasm_bindgen]
    pub fn cube(size: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Cube { size } }
    }

    /// Creates a rectangular box with independent dimensions.
    /// @param {number} width - Size along the X-axis.
    /// @param {number} height - Size along the Y-axis.
    /// @param {number} depth - Size along the Z-axis.
    #[wasm_bindgen]
    pub fn box_geo(width: f32, height: f32, depth: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Box { width, height, depth } }
    }

    /// Creates a flat, square surface on the XZ plane.
    /// @param {number} size - The side length of the square plane.
    #[wasm_bindgen]
    pub fn plane(size: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Plane { size } }
    }

    /// Creates a spherical mesh (usually a UV sphere or Ico-sphere depending on implementation).
    /// @param {number} radius - The distance from the center to the surface.
    /// @param {number} subdivisions - Controls the smoothness of the sphere. Higher values create more triangles.
    #[wasm_bindgen]
    pub fn sphere(radius: f32, subdivisions: usize) -> Geometry {
        Geometry { inner: CoreGeometry::Sphere { radius, subdivisions } }
    }

    /// Creates a four-sided pyramid with a square base.
    /// @param {number} base_size - The side length of the square base.
    /// @param {number} height - The vertical distance from the base to the apex.
    #[wasm_bindgen]
    pub fn pyramid(base_size: f32, height: f32) -> Geometry {
        Geometry { inner: CoreGeometry::Pyramid { base_size, height } }
    }
}