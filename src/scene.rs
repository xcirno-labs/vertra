use crate::camera::Camera;
use crate::mesh::Mesh;
use crate::pipeline::Pipeline;

pub struct Scene {
    pub pipeline: Pipeline,
    pub mesh: Mesh,
    pub camera: Camera,
}
