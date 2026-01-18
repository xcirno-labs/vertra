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
            Geometry::Cube { size } => {
                let s = *size * 0.5;
                self.add_geometry(
                    &Geometry::Box { width: s, height: s, depth: s }, transform, color
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
                self.add_transformed_quad([p1, p2, p3, p4], transform, color); // Front
                self.add_transformed_quad([p6, p5, p8, p7], transform, color); // Back
                self.add_transformed_quad([p5, p1, p4, p8], transform, color); // Left
                self.add_transformed_quad([p2, p6, p7, p3], transform, color); // Right
                self.add_transformed_quad([p4, p3, p7, p8], transform, color); // Top
                self.add_transformed_quad([p5, p6, p2, p1], transform, color); // Bottom
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
                self.add_transformed_quad([p1, p2, p3, p4], transform, color);

                // Push the bottom face (reversed order)
                self.add_transformed_quad([p4, p3, p2, p1], transform, color);
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
                self.add_transformed_triangle([tip, b1, b2], transform, color); // Front
                self.add_transformed_triangle([tip, b2, b3], transform, color); // Right
                self.add_transformed_triangle([tip, b3, b4], transform, color); // Back
                self.add_transformed_triangle([tip, b4, b1], transform, color); // Left
                // Base
                self.add_transformed_quad([b4, b3, b2, b1], transform, color);
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
                    self.add_transformed_quad(
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
                        self.add_transformed_quad(
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
                        self.add_transformed_quad(
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