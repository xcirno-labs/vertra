use crate::camera::Camera;
use crate::mesh::{MeshRegistry};
use crate::pipeline::Pipeline;
use crate::world::World;
use crate::geometry::{Geometry, GeometryId};
use crate::transform::Transform;

pub struct Scene {
    pub pipeline: Pipeline,
    pub mesh_registry: MeshRegistry,
    pub camera: Camera,
    pub world: World
}

impl Scene {
    pub fn _register(&mut self, geometry: &Geometry) -> GeometryId {
        // Convert Geometry (Blueprint) to raw Vertex/Index data
        let (verts, indices) = geometry.build();

        // Upload that raw data to the GPU and get the Buffer handles
        let baked = self.pipeline.create_baked_mesh(&verts, &indices);

        // Store the BakedMesh in our internal list and return the ID
        self.mesh_registry.add(baked)
    }

    pub fn spawn(&mut self, geometry: &Geometry, transform: Transform, color: [f32; 4]) -> usize {
        // Bake the geometry into the MeshRegistry
        let geometry_id = self._register(geometry);

        // Add the entity to the World (Talks to Logic)
        self.world.spawn(geometry_id, transform, color)
    }

    pub fn draw_world(&mut self) {
        // We pass 'self.mesh_registry' because it contains the 'baked_geometries'
        // (the actual GPU buffers) that 'world' entities reference by ID.
        self.pipeline.render_world(&self.world, &self.mesh_registry, &self.camera);
    }
}