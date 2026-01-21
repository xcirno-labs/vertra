use crate::pipeline::Pipeline;
use crate::transform::Transform;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

// GPU Side: The actual buffers living in VRAM
pub struct BakedMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

// CPU Side: A "Builder" used to assemble vertices before baking
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

// The Collection: Stored in the Scene to keep track of all unique baked shapes
pub struct MeshRegistry {
    pub world_mesh: Option<BakedMesh>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self { world_mesh: None }
    }

    pub fn update_world_mesh(&mut self, baked: BakedMesh) {
        self.world_mesh = Some(baked);
    }
}

impl MeshData {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    // Takes the current CPU data and uploads it to the GPU
    pub fn bake(&self, pipeline: &Pipeline) -> BakedMesh {
        pipeline.create_baked_mesh(&self.vertices, &self.indices)
    }
    pub fn add_object(&mut self, world: &crate::world::World, object_id: usize, parent_transform: &Transform) {
        if let Some(obj) = world.objects.get(&object_id) {
            // Combine the parent's world transform with this object's local transform
            let world_transform = parent_transform.combine(&obj.transform);

            // If this object has a physical shape, add its vertices
            if let Some(geo) = &obj.geometry {
                geo.generate_mesh_data(self, &world_transform, obj.color);
            }

            // Process all children
            for &child_id in &obj.children {
                self.add_object(world, child_id, &world_transform);
            }
        }
    }
    pub fn add_transformed_triangle(&mut self, points: [[f32; 3]; 3], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points);
        self.push_triangle(transformed, color);
    }

    pub fn add_transformed_quad(&mut self, points: [[f32; 3]; 4], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points);
        self.push_quad(transformed, color);
    }

    pub fn push_quad(&mut self, points: [[f32; 3]; 4], color: [f32; 4]) {
        let start_index = self.vertices.len() as u32;
        // TODO: Implement alpha channel
        let c = [color[0], color[1], color[2]];

        // Push 4 vertices
        for p in points {
            self.vertices.push(Vertex { position: p, color: c });
        }

        // Push 6 indices to form 3 triangles, e.g.
        // Triangle 1: [0, 1, 2], Triangle 2: [0, 2, 3]
        self.indices.extend_from_slice(&[
            start_index + 0, start_index + 1, start_index + 2,
            start_index + 0, start_index + 2, start_index + 3,
        ]);
    }

    pub fn push_triangle(&mut self, points: [[f32; 3]; 3], color: [f32; 4]) {
        let start_index = self.vertices.len() as u32;
        let c = [color[0], color[1], color[2]];
        
        for p in points {
            self.vertices.push(Vertex { position: p, color: c });
        }
        self.indices.extend_from_slice(&[
            start_index + 0, start_index + 1, start_index + 2
        ]);
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}