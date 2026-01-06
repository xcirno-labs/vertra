use crate::geometry::{Geometry, GeometryId};
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
    pub baked_geometries: Vec<BakedMesh>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self { baked_geometries: Vec::new() }
    }

    pub fn add(&mut self, baked: BakedMesh) -> GeometryId {
        let id = self.baked_geometries.len();
        self.baked_geometries.push(baked);
        GeometryId(id)
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

    pub fn add_geometry(&mut self, geometry: &Geometry, transform: &Transform, color: [f32; 4]) {
        match geometry {
            Geometry::Square { size } => {
                self.add_geometry(
                    &Geometry::Rectangle { width: *size, height: *size }, transform, color
                );
            }
            Geometry::Rectangle { width, height } => {
                let w = width * 0.5;
                let h = height * 0.5;
                
                // Corners relative to center (0,0)
                let p1 = [-w, -h, 0.0];  // Bottom Left
                let p2 = [ w, -h, 0.0];  // Bottom Right
                let p3 = [ w,  h, 0.0];  // Top Right
                let p4 = [-w,  h, 0.0];  // Top Left

                self.add_transformed_quad([p1, p2, p3, p4], transform, color);
            }
            Geometry::Triangle { base, height } => {
                let half_w = base * 0.5;
                let half_h = height * 0.5;
                
                let p1 = [0.0, half_h, 0.0];        // Top
                let p2 = [-half_w, -half_h, 0.0];   // Bottom Left
                let p3 = [half_w, -half_h, 0.0];    // Bottom Right
                
                self.add_transformed_triangle([p1, p2, p3], transform, color);
            }
            Geometry::Cube { size } => {
                let s = size * 0.5;
                let p1 = [-s, -s,  s]; // Front-Bottom-Left
                let p2 = [ s, -s,  s]; // Front-Bottom-Right
                let p3 = [ s,  s,  s]; // Front-Top-Right
                let p4 = [-s,  s,  s]; // Front-Top-Left
                let p5 = [-s, -s, -s]; // Back-Bottom-Left
                let p6 = [ s, -s, -s]; // Back-Bottom-Right
                let p7 = [ s,  s, -s]; // Back-Top-Right
                let p8 = [-s,  s, -s]; // Back-Top-Left

                // Note: Winding order matters for culling!
                self.add_transformed_quad([p1, p2, p3, p4], transform, color); // Front
                self.add_transformed_quad([p6, p5, p8, p7], transform, color); // Back
                self.add_transformed_quad([p5, p1, p4, p8], transform, color); // Left
                self.add_transformed_quad([p2, p6, p7, p3], transform, color); // Right
                self.add_transformed_quad([p4, p3, p7, p8], transform, color); // Top
                self.add_transformed_quad([p5, p6, p2, p1], transform, color); // Bottom
            }
        }
    }

    fn add_transformed_triangle(&mut self, points: [[f32; 3]; 3], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points);
        self.push_triangle(transformed, color);
    }

    fn add_transformed_quad(&mut self, points: [[f32; 3]; 4], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points);
        self.push_quad(transformed, color);
    }

    fn push_quad(&mut self, points: [[f32; 3]; 4], color: [f32; 4]) {
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

    fn push_triangle(&mut self, points: [[f32; 3]; 3], color: [f32; 4]) {
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