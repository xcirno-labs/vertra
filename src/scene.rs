use crate::camera::Camera;
use crate::mesh::{MeshRegistry};
use crate::pipeline::Pipeline;
use crate::world::World;
use crate::objects::Object;
use crate::transform::Transform;
use crate::vtr::{self, VtrError};

pub struct Scene {
    pub pipeline: Pipeline,
    pub mesh_registry: MeshRegistry,
    pub camera: Camera,
    pub world: World
}

impl Scene {
    pub fn spawn(&mut self, object: Object, parent_id: Option<usize>) -> usize {
        self.world.spawn_object(object, parent_id)
    }

    pub fn draw_world(&mut self) {
        let mut mesh_data = crate::mesh::MeshData::new();
        let identity = Transform::default();

        // Flatten the entire world hierarchy into vertices
        // This visits every Object and combines their Transforms
        for &root_id in &self.world.roots {
            mesh_data.add_object(&self.world, root_id, &identity);
        }

        // Bake this frame's geometry to the GPU
        // TODO: Reuse buffers instead of creating new ones
        let world_baked = mesh_data.bake(&self.pipeline);

        self.pipeline.render_baked_mesh(&world_baked, &self.camera);
    }

    /// Serialize the current camera and world to a `.vtr` binary file.
    ///
    /// Creates or truncates the file at `path`.
    ///
    /// # Errors
    /// Returns a [`VtrError`] on I/O failure or serialization problems.
    pub fn save_vtr_file(&self, path: &std::path::Path) -> Result<(), VtrError> {
        vtr::write_to_file(path, &self.camera, &self.world)
    }

    /// Replace the current camera and world with the contents of a `.vtr` file.
    ///
    /// The GPU pipeline is **not** affected — only the logical scene state
    /// (camera, objects, hierarchy) is replaced.
    ///
    /// # Errors
    /// Returns a [`VtrError`] on I/O failure, bad magic bytes, unsupported
    /// format version, or any other parse error.
    pub fn load_vtr_file(&mut self, path: &std::path::Path) -> Result<(), VtrError> {
        let data = vtr::read_from_file(path)?;
        self.camera = data.camera;
        self.world = data.world;
        Ok(())
    }
}