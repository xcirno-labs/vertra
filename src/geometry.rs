use crate::mesh::{MeshData, Vertex};
use crate::transform::Transform;
use serde::{Serialize, Deserialize};

/// A lightweight opaque handle to a geometry entry in a GPU registry.
///
/// Returned by pipeline-internal registration routines; you do not typically
/// need to construct or inspect this directly.
#[derive(Debug, Copy, Clone)]
pub struct GeometryId(pub usize);

/// Procedural geometry primitives supported by the engine.
///
/// Each variant stores only the parameters needed to describe its shape; the
/// actual vertex data is generated on demand via [`Geometry::build`] or
/// [`Geometry::generate_mesh_data`].
///
/// # Coordinate conventions
/// All dimensions (radii, sizes, heights) are in **world units**.  The
/// geometry is centred at the local origin unless otherwise noted.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Geometry {
    /// A uniform cube centred at the origin.
    ///
    /// `size` is the full side length; half-extents are `size / 2` on each axis.
    Cube { size: f32 },
    /// An axis-aligned rectangular box centred at the origin.
    ///
    /// `width` = X extent, `height` = Y extent, `depth` = Z extent (full, not half).
    Box { width: f32, height: f32, depth: f32 },
    /// A flat, double-sided horizontal plane centred at the origin lying in
    /// the XZ plane.
    ///
    /// `size` is the full side length.
    Plane { size: f32 },
    /// A four-sided pyramid centred at the origin.
    ///
    /// The base is a square with full side `base_size` at `y = -height / 2`;
    /// the apex is at `y = height / 2`.
    Pyramid { base_size: f32, height: f32 },
    /// A capsule (cylinder capped with hemispheres) centred at the origin,
    /// oriented along the Y axis.
    ///
    /// * `radius`       — radius of the cylinder body and hemispherical caps.
    /// * `height`       — length of the **cylindrical** body only (not including caps).
    /// * `subdivisions` — number of horizontal segments; higher values produce
    ///   a smoother silhouette.
    Capsule { radius: f32, height: f32, subdivisions: usize },
    /// A UV sphere centred at the origin.
    ///
    /// * `radius`       — sphere radius.
    /// * `subdivisions` — number of horizontal (longitude) segments; latitude
    ///   segments are derived as `subdivisions / 2`.  Minimum effective value
    ///   is 8 for a reasonable sphere.
    Sphere { radius: f32, subdivisions: usize },
}

impl Geometry {
    /// Build raw vertex and index arrays for this geometry at the world origin
    /// with a neutral white colour.
    ///
    /// Returns `(vertices, indices)`.  The result can be passed directly to
    /// [`crate::pipeline::Pipeline::create_baked_mesh`] for GPU upload.
    pub fn build(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut mesh = MeshData::new();
        let identity = Transform::default();

        // We use a dummy color [1,1,1,1] because baked meshes
        // usually have their colors modified by the Object color later.
        self.generate_mesh_data(&mut mesh, &identity, [1.0, 1.0, 1.0, 1.0]);

        (mesh.vertices, mesh.indices)
    }

    /// Append this geometry's triangles into an existing [`MeshData`] builder,
    /// applying `transform` and `color` to every vertex.
    ///
    /// This is the low-level primitive used by the scene renderer to batch all
    /// objects into a single draw call each frame.
    pub fn generate_mesh_data(&self, mesh_data: &mut MeshData, transform: &Transform, color: [f32; 4]) {
        match self {
            Geometry::Cube { size } => {
                let s = *size * 0.5;
                Geometry::Box { width: s, height: s, depth: s }.generate_mesh_data(
                    mesh_data, transform, color
                );
            }
            Geometry::Box { width, height, depth } => {
                let w = width * 0.5;
                let h = height * 0.5;
                let d = depth * 0.5;

                let p1 = [-w, -h,  d]; // Front-Bottom-Left
                let p2 = [ w, -h,  d]; // Front-Bottom-Right
                let p3 = [ w,  h,  d]; // Front-Top-Right
                let p4 = [-w,  h,  d]; // Front-Top-Left
                let p5 = [-w, -h, -d]; // Back-Bottom-Left
                let p6 = [ w, -h, -d]; // Back-Bottom-Right
                let p7 = [ w,  h, -d]; // Back-Top-Right
                let p8 = [-w,  h, -d]; // Back-Top-Left

                // Note: Winding order matters for culling!
                mesh_data.add_transformed_quad([p1, p4, p3, p2], transform, color); // Front
                mesh_data.add_transformed_quad([p6, p7, p8, p5], transform, color); // Back
                mesh_data.add_transformed_quad([p5, p8, p4, p1], transform, color); // Left
                mesh_data.add_transformed_quad([p2, p3, p7, p6], transform, color); // Right
                mesh_data.add_transformed_quad([p4, p8, p7, p3], transform, color); // Top
                mesh_data.add_transformed_quad([p5, p1, p2, p6], transform, color); // Bottom
            }
            Geometry::Plane { size } => {
                let s = size * 0.5;

                // Since using culling makes the back of the geometry not visible,
                // we can instead make 2 copies of switched vertices.
                let p1 = [-s, 0.0,  s];
                let p2 = [ s, 0.0,  s];
                let p3 = [ s, 0.0, -s];
                let p4 = [-s, 0.0, -s];

                // Push the top face
                mesh_data.add_transformed_quad([p1, p2, p3, p4], transform, color);

                // Push the bottom face (reversed order)
                mesh_data.add_transformed_quad([p4, p3, p2, p1], transform, color);
            }
            Geometry::Pyramid { base_size, height } => {
                let s = base_size * 0.5;
                let h = height * 0.5;

                let tip = [0.0, h, 0.0];
                let b1 = [-s, -h, s]; // Front-Left
                let b2 = [s, -h, s]; // Front-Right
                let b3 = [s, -h, -s]; // Back-Right
                let b4 = [-s, -h, -s]; // Back-Left

                // 4 Sides
                mesh_data.add_transformed_triangle([tip, b1, b2], transform, color); // Front
                mesh_data.add_transformed_triangle([tip, b2, b3], transform, color); // Right
                mesh_data.add_transformed_triangle([tip, b3, b4], transform, color); // Back
                mesh_data.add_transformed_triangle([tip, b4, b1], transform, color); // Left
                // Base
                mesh_data.add_transformed_quad([b4, b3, b2, b1], transform, color);
            }
            Geometry::Capsule { radius, height, subdivisions } => {
                let r = *radius;
                let h = *height;
                let subs = *subdivisions as f32;
                let half_h = h * 0.5;
                // `lat_subs` is the number of vertical vertices. To maintain a "rounded" shape,
                // a minimum of 4 subdivisions is used.
                let lat_subs = (*subdivisions / 2).max(4);

                // `subdivisions` is the number of horizontal vertices
                for i in 0..*subdivisions {
                    let t1 = (i as f32 * 2.0 * std::f32::consts::PI) / subs;
                    let t2 = ((i + 1) as f32 * 2.0 * std::f32::consts::PI) / subs;

                    let x1 = t1.cos();
                    let z1 = t1.sin();
                    let x2 = t2.cos();
                    let z2 = t2.sin();

                    // The body (Cylinder)
                    mesh_data.add_transformed_quad(
                        [
                            [x1 * r, -half_h, z1 * r],
                            [x2 * r, -half_h, z2 * r],
                            [x2 * r,  half_h, z2 * r],
                            [x1 * r,  half_h, z1 * r],
                        ],
                        transform, color
                    );

                    // The 2 hemispheres
                    for j in 0..lat_subs {
                        let phi1 = (j as f32 * std::f32::consts::FRAC_PI_2) / lat_subs as f32;
                        let phi2 = ((j + 1) as f32 * std::f32::consts::FRAC_PI_2) / lat_subs as f32;

                        let r1 = phi1.cos() * r; let y1 = phi1.sin() * r;
                        let r2 = phi2.cos() * r; let y2 = phi2.sin() * r;

                        // TOP CAP (Facing Outwards/Up)
                        mesh_data.add_transformed_quad(
                            [
                                [x1 * r1,  half_h + y1, z1 * r1],
                                [x2 * r1,  half_h + y1, z2 * r1],
                                [x2 * r2,  half_h + y2, z2 * r2],
                                [x1 * r2,  half_h + y2, z1 * r2],
                            ],
                            transform, color
                        );

                        // BOTTOM CAP (Facing Outwards/Down)
                        // To ensure the "base" renders, we reverse the sequence of x1 and x2
                        // so the normal faces DOWN.
                        mesh_data.add_transformed_quad(
                            [
                                [x1 * r1, -half_h - y1, z1 * r1],
                                [x1 * r2, -half_h - y2, z1 * r2],
                                [x2 * r2, -half_h - y2, z2 * r2],
                                [x2 * r1, -half_h - y1, z2 * r1],
                            ],
                            transform, color
                        );
                    }
                }
            }
            Geometry::Sphere { radius, subdivisions } => {
                let r = *radius;
                let subs = *subdivisions as f32;
                let lat_subs = (*subdivisions / 2).max(4);

                for i in 0..*subdivisions {
                    let t1 = (i as f32 * 2.0 * std::f32::consts::PI) / subs;
                    let t2 = ((i + 1) as f32 * 2.0 * std::f32::consts::PI) / subs;

                    let (x1, z1) = (t1.cos(), t1.sin());
                    let (x2, z2) = (t2.cos(), t2.sin());

                    for j in 0..lat_subs {
                        // Angle from bottom (-PI/2) to top (PI/2)
                        let phi1 = (j as f32 * std::f32::consts::PI) / lat_subs as f32 - std::f32::consts::FRAC_PI_2;
                        let phi2 = ((j + 1) as f32 * std::f32::consts::PI) / lat_subs as f32 - std::f32::consts::FRAC_PI_2;

                        let r1 = phi1.cos() * r; let y1 = phi1.sin() * r;
                        let r2 = phi2.cos() * r; let y2 = phi2.sin() * r;

                        mesh_data.add_transformed_quad(
                            [
                                [x1 * r1, y1, z1 * r1],
                                [x2 * r1, y1, z2 * r1],
                                [x2 * r2, y2, z2 * r2],
                                [x1 * r2, y2, z1 * r2],
                            ],
                            transform, color
                        );
                    }
                }
            }
        }
    }
}