use crate::pipeline::Pipeline;
use crate::transform::Transform;

/// A single GPU-ready vertex.
///
/// `repr(C)` and `bytemuck`-derived so it can be safely cast to raw bytes and
/// uploaded to a `wgpu` vertex buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// XYZ position in world space (before the shader applies the MVP matrix).
    pub position: [f32; 3],
    /// RGB vertex colour in `[0.0, 1.0]` linear space.
    pub color: [f32; 3],
    /// UV texture coordinates (default `[0.0, 0.0]` for untextured geometry).
    pub uv: [f32; 2],
}

/// A fully uploaded mesh living in GPU (VRAM) memory.
///
/// Created by [`MeshData::bake`] or
/// [`crate::pipeline::Pipeline::create_baked_mesh`].  The buffers are owned
/// by this struct and released when it is dropped.
pub struct BakedMesh {
    /// wgpu vertex buffer containing the packed [`Vertex`] data.
    pub vertex_buffer: wgpu::Buffer,
    /// wgpu index buffer containing `u32` triangle indices.
    pub index_buffer: wgpu::Buffer,
    /// Number of indices; used as the `index_count` argument in draw calls.
    pub index_count: u32,
}

/// CPU-side mesh builder that accumulates vertices and indices before uploading
/// to the GPU via [`MeshData::bake`].
///
/// This is the primary way to assemble geometry during the render phase.  The
/// scene renderer creates one `MeshData` per texture group each frame,
/// populates it by walking the scene graph, then bakes the result once.
pub struct MeshData {
    /// Accumulated vertex list.
    pub vertices: Vec<Vertex>,
    /// Accumulated index list (triangle list topology).
    pub indices: Vec<u32>,
}

/// Registry that keeps track of the current world mesh inside a [`crate::scene::Scene`].
pub struct MeshRegistry {
    /// The most recently baked world geometry, or `None` before the first frame.
    pub world_mesh: Option<BakedMesh>,
}

impl MeshRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { world_mesh: None }
    }

    /// Replace the stored world mesh with a freshly baked one.
    pub fn update_world_mesh(&mut self, baked: BakedMesh) {
        self.world_mesh = Some(baked);
    }
}

impl MeshData {
    /// Create an empty mesh builder.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Upload the accumulated CPU data to GPU memory and return a [`BakedMesh`].
    ///
    /// The `MeshData` itself is left unchanged; you can bake multiple times if
    /// needed (though typically you bake once and discard).
    pub fn bake(&self, pipeline: &Pipeline) -> BakedMesh {
        pipeline.create_baked_mesh(&self.vertices, &self.indices)
    }

    /// Recursively add an object and all its descendants to this mesh builder.
    ///
    /// `parent_transform` is the accumulated world transform of the caller's
    /// parent; the object's local transform is combined with it before
    /// generating geometry so that child objects inherit parent positions,
    /// rotations, and scales.
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

    /// Apply `transform` to three points and append a transformed triangle.
    pub fn add_transformed_triangle(&mut self, points: [[f32; 3]; 3], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points);
        self.push_triangle(transformed, color);
    }

    /// Apply `transform` to four points and append two transformed triangles
    /// (a quad split along its diagonal).
    pub fn add_transformed_quad(&mut self, points: [[f32; 3]; 4], transform: &Transform, color: [f32; 4]) {
        let transformed = transform.apply(points);
        self.push_quad(transformed, color);
    }

    /// Append a planar quad (four points → two triangles) with the given color.
    ///
    /// UV coordinates are assigned in bottom-left → bottom-right → top-right →
    /// top-left order, matching standard texture-mapping conventions.
    pub fn push_quad(&mut self, points: [[f32; 3]; 4], color: [f32; 4]) {
        let start_index = self.vertices.len() as u32;
        // TODO: Implement alpha channel
        let c = [color[0], color[1], color[2]];
        // Planar face UVs: bottom-left -> bottom-right -> top-right -> top-left
        let uvs: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        for (p, uv) in points.iter().zip(uvs.iter()) {
            self.vertices.push(Vertex { position: *p, color: c, uv: *uv });
        }

        self.indices.extend_from_slice(&[
            start_index,     start_index + 1, start_index + 2,
            start_index,     start_index + 2, start_index + 3,
        ]);
    }

    /// Append a single triangle with the given color.
    pub fn push_triangle(&mut self, points: [[f32; 3]; 3], color: [f32; 4]) {
        let start_index = self.vertices.len() as u32;
        let c = [color[0], color[1], color[2]];
        let uvs: [[f32; 2]; 3] = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];

        for (p, uv) in points.iter().zip(uvs.iter()) {
            self.vertices.push(Vertex { position: *p, color: c, uv: *uv });
        }
        self.indices.extend_from_slice(&[start_index, start_index + 1, start_index + 2]);
    }

    /// Clear all accumulated vertices and indices, resetting the builder.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}