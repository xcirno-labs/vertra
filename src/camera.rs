use crate::math::Matrix4;

pub struct Camera {
    pub eye: [f32; 3],    // Position of the camera
    pub target: [f32; 3], // Where the camera is looking
    pub up: [f32; 3],     // Usually [0.0, 1.0, 0.0]
    pub aspect: f32,      // width / height
    pub fov: f32,         // Field of view in degrees
    pub znear: f32,       // Near clipping plane (e.g., 0.1)
    pub zfar: f32,        // Far clipping plane (e.g., 100.0)
}

impl Camera {
    pub fn new() -> Self {
        // Default settings
        Self {
            eye: [0.0, 2.0, 5.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            aspect: 1.0,
            fov: 20.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect = aspect;
        self
    }

    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    pub fn with_clip_planes(mut self, znear: f32, zfar: f32) -> Self {
        self.znear = znear;
        self.zfar = zfar;
        self
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4 {
        let view = Matrix4::look_at(self.eye, self.target, self.up);
        let proj = Matrix4::perspective(self.fov, self.aspect, self.znear, self.zfar);

        proj * view
    }
}
