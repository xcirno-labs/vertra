use crate::math::Matrix4;
use crate::constants::camera;

#[derive(Debug, Copy, Clone)]
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
        Self {
            eye: camera::DEFAULT_EYE,
            target: camera::DEFAULT_TARGET,
            up: camera::UP,
            aspect: camera::DEFAULT_ASPECT_RATIO,
            fov: camera::DEFAULT_FOV,
            znear: camera::NEAR_PLANE,
            zfar: camera::FAR_PLANE,
        }
    }
    pub fn with_target(mut self, target: [f32; 3]) -> Self {
        self.target = target;
        self
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

    pub fn with_position(mut self, pos: [f32; 3]) -> Self {
        self.eye = pos;
        self
    }

    pub fn update_position(&mut self, new_pos: [f32; 3]) {
        self.eye = new_pos;
    }

    pub fn set_target(&mut self, new_target: [f32; 3]) {
        self.target = new_target;
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4 {
        let view = Matrix4::look_at(self.eye, self.target, self.up);
        let proj = Matrix4::perspective(self.fov, self.aspect, self.znear, self.zfar);

        proj * view
    }
}
