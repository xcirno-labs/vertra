pub enum Geometry {
    Triangle { base: f32, height: f32 },
    Rectangle { width: f32, height: f32 },
    Square { size: f32 },
    // TODO: add a custom mesh variant
    // Custom { vertices: Vec<Vertex> }
}