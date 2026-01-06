use crate::mesh::{MeshData, Vertex};
use crate::transform::Transform;

#[derive(Debug, Copy, Clone)]
pub struct GeometryId(pub usize);

pub enum Geometry {
    Triangle { base: f32, height: f32 },
    Rectangle { width: f32, height: f32 },
    Square { size: f32 },
    Cube { size: f32 },
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