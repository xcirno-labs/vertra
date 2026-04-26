//! Engine-wide default constants.
//!
//! Grouped into sub-modules by subsystem:
//! * [`window`] - default window size and fixed-update rate.
//! * [`camera`] - default camera placement and projection parameters.
//! * [`pipeline`] - initial GPU buffer allocation sizes.

/// Default windowing constants.
pub mod window {
    /// Minimum allowed window width and height in physical pixels.
    pub const MIN_DIMENSION: [u32; 2] = [250, 250];
    /// Default window width in physical pixels.
    pub const DEFAULT_WIDTH: u32 = 800;
    /// Default window height in physical pixels.
    pub const DEFAULT_HEIGHT: u32 = 600;
    /// Fixed-update timestep in seconds (1 / 60 or approximately 16.67 ms).
    pub const FIXED_DELTA: f32 = 1.0 / 60.0;
}

/// Default camera constants.
pub mod camera {
    /// Default world-space eye position `[x, y, z]`.
    pub const DEFAULT_EYE: [f32; 3] = [0.0, 2.0, 5.0];
    /// Default look-at target `[x, y, z]`.
    pub const DEFAULT_TARGET: [f32; 3] = [0.0, 0.0, 0.0];
    /// Default aspect ratio (overridden at runtime by the actual window size).
    pub const DEFAULT_ASPECT_RATIO: f32 = 1.0;
    /// Default vertical field of view in degrees.
    pub const DEFAULT_FOV: f32 = 45.0;
    /// World-up vector.
    pub const UP: [f32; 3] = [0.0, 1.0, 0.0];
    /// Near clipping plane distance.
    pub const NEAR_PLANE: f32 = 0.1;
    /// Far clipping plane distance.
    pub const FAR_PLANE: f32 = 1000.0;
    /// Default yaw and pitch rotation in degrees.
    pub const DEFAULT_ROTATION: f32 = 0.0;
}

/// Default GPU pipeline constants.
pub mod pipeline {
    /// Initial capacity of the GPU vertex buffer in vertices.
    pub const INITIAL_VERTEX_LIMIT: u32 = 128;
    /// Initial capacity of the GPU index buffer in indices.
    pub const INITIAL_INDEX_LIMIT: u32 = 1024;
}