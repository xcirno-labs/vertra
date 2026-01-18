use crate::mesh::{MeshData, Vertex};
use crate::transform::Transform;

#[derive(Debug, Copy, Clone)]
pub struct GeometryId(pub usize);

pub enum Geometry {
    Cube { size: f32 },
    Box { width: f32, height: f32, depth: f32 },
    Plane { size: f32 },
    Pyramid { base_size: f32, height: f32 },
    Capsule { radius: f32, height: f32, subdivisions: usize },
    // TODO: add a custom mesh variant
    // Custom { vertices: Vec<Vertex> }
}

impl Geometry {
    pub fn build(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut mesh = MeshData::new();
        let identity = Transform::default();

        // We use a dummy color [1,1,1,1] because baked meshes
        // usually have their colors modified by the Entity color later.
        mesh.add_geometry(self, &identity, [1.0, 1.0, 1.0, 1.0]);

        (mesh.vertices, mesh.indices)
    }
}