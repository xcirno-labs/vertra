use {crate::mesh::Vertex};

pub const SQUARE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5,  0.5, 0.0], color: [1.0, 0.0, 0.0] },   // Top Left
    Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },   // Bottom Left
    Vertex { position: [ 0.5,  -0.5, 0.0], color: [0.0, 0.0, 1.0] },  // Bottom Right

    Vertex { position: [ 0.5,  -0.5, 0.0], color: [1.0, 1.0, 1.0] },  // Bottom Right
    Vertex { position: [ 0.5,  0.5, 0.0], color: [1.0, 0.0, 1.0] },   // Top Right
    Vertex { position: [ -0.5,  0.5, 0.0], color: [0.0, 1.0, 1.0] },  // Top Left
];

pub const TRIANGLES_VERTICES: &[Vertex] = &[];