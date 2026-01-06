pub mod window {
    pub const MIN_DIMENSION: [u32; 2] = [250, 250];
    pub const DEFAULT_WIDTH: u32 = 800;
    pub const DEFAULT_HEIGHT: u32 = 600;
    // 60 times every second
    pub const FIXED_DELTA: f32 = 1.0 / 60.0;
}

pub mod camera {
    pub const DEFAULT_EYE: [f32; 3] = [0.0, 2.0, 5.0];
    pub const DEFAULT_TARGET: [f32; 3] = [0.0, 0.0, 0.0];
    pub const DEFAULT_ASPECT_RATIO: f32 = 1.0;
    pub const DEFAULT_FOV: f32 = 45.0;
    pub const UP: [f32; 3] = [0.0, 1.0, 0.0];
    pub const NEAR_PLANE: f32 = 0.1;
    pub const FAR_PLANE: f32 = 1000.0;
}

pub mod pipeline {
    pub const INITIAL_VERTEX_LIMIT: u32 = 128;
    pub const INITIAL_INDEX_LIMIT: u32 = 1024;
}