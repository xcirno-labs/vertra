use crate::geometry::Geometry;
use crate::transform::Transform;
use crate::viewport::Viewport;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub viewport: Viewport,
}

impl Mesh {
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self {
        let viewport = Viewport {
            width: viewport_width,
            height: viewport_height,
        };
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            viewport
        }
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
        }
    }
    fn add_transformed_triangle(&mut self, points: [[f32; 3]; 3], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points, self.viewport);
        self.push_triangle([transformed[0], transformed[1], transformed[2]], color);
    }
    fn add_transformed_quad(&mut self, points: [[f32; 3]; 4], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points, self.viewport);
        self.push_quad([transformed[0], transformed[1], transformed[2], transformed[3]], color);
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