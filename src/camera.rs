use std::collections::HashSet;
use winit::keyboard::KeyCode;
use crate::math::Matrix4;
use crate::constants::camera;
use crate::window::FrameContext;

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub eye: [f32; 3],    // Position of the camera
    pub target: [f32; 3], // Where the camera is looking
    pub up: [f32; 3],     // Usually [0.0, 1.0, 0.0]
    pub aspect: f32,      // width / height
    pub fov: f32,         // Field of view in degrees
    pub znear: f32,       // Near clipping plane (e.g., 0.1)
    pub zfar: f32,        // Far clipping plane (e.g., 100.0)
    pub lr_rot: f32,      // Left/Right rotation
    pub ud_rot: f32,      // Up/Down rotation
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
            lr_rot: camera::DEFAULT_ROTATION,
            ud_rot: camera::DEFAULT_ROTATION,
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

    pub fn with_position(mut self, pos: [f32; 3]) -> Self {
        self.eye = pos;
        self
    }

    pub fn with_rotation(mut self, rotx: f32, roty: f32) -> Self {
        self.lr_rot = rotx;
        self.ud_rot = roty;
        self.update_target_from_angles();
        self
    }

    pub fn update_position(&mut self, new_pos: [f32; 3]) {
        self.eye = new_pos;
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4 {
        let view = Matrix4::look_at(self.eye, self.target, self.up);
        let proj = Matrix4::perspective(self.fov, self.aspect, self.znear, self.zfar);

        proj * view
    }

    fn update_target_from_angles(&mut self) {
        let lr_rad = self.lr_rot.to_radians();
        let ud_rad = self.ud_rot.to_radians();

        // Calculate a direction vector from angles
        let f_x = lr_rad.cos() * ud_rad.cos();
        let f_y = ud_rad.sin();
        let f_z = lr_rad.sin() * ud_rad.cos();


        // The target is just the eye position + the direction vector
        self.target = [
            self.eye[0] + f_x,
            self.eye[1] + f_y,
            self.eye[2] + f_z,
        ];
    }

    pub fn rotate(&mut self, dx: f32, dy: f32, inverted: bool) {
        if !inverted {
            // Moving mouse up, looks up and right, looks right
            self.lr_rot -= dx;
            self.ud_rot -= dy;
        } else {
            self.lr_rot += dx;
            self.ud_rot += dy;
        }

        // Constrain pitch so you can't flip the camera upside down
        self.ud_rot = self.ud_rot.clamp(-89.0, 89.0);

        self.update_target_from_angles();
    }

    pub fn get_directions(&self) -> ([f32; 3], [f32; 3]) {
        // Calculate Forward vector (Target - Eye)
        let f = [
            self.target[0] - self.eye[0],
            self.target[1] - self.eye[1],
            self.target[2] - self.eye[2],
        ];

        // Normalize Forward
        let f_len = (f[0]*f[0] + f[1]*f[1] + f[2]*f[2]).sqrt();
        let forward = [f[0] / f_len, f[1] / f_len, f[2] / f_len];

        // Calculate Right vector using Cross Product: Forward x Up
        // Cross Product Formula:
        let r = [
            forward[2] * self.up[1] - forward[1] * self.up[2],
            forward[0] * self.up[2] - forward[2] * self.up[0],
            forward[1] * self.up[0] - forward[0] * self.up[1],
        ];
        let r_len_sq = r[0]*r[0] + r[1]*r[1] + r[2]*r[2];
        // Normalize Right
        let right = if r_len_sq < 0.0001 {
            [1.0, 0.0, 0.0]
        } else {
            let r_len = r_len_sq.sqrt();
            [r[0] / r_len, r[1] / r_len, r[2] / r_len]
        };

        (forward, right)
    }

    pub fn move_by(&mut self, direction: [f32; 3], amount: f32) {
        let dx = direction[0] * amount;
        let dy = direction[1] * amount;
        let dz = direction[2] * amount;

        // Move the camera position
        self.eye[0] += dx;
        self.eye[1] += dy;
        self.eye[2] += dz;

        // Move the focal point so the camera doesn't "pivot"
        self.target[0] += dx;
        self.target[1] += dy;
        self.target[2] += dz;
    }

    pub fn handle_default_input(&mut self, keys: &HashSet<KeyCode>, speed: f32, ctx: &mut FrameContext) {
        let (f, r) = self.get_directions();
        let mut move_dir = [0.0, 0.0, 0.0];

        if keys.contains(&KeyCode::KeyW) {
            move_dir[0] += f[0]; move_dir[1] += f[1]; move_dir[2] += f[2];
        }
        if keys.contains(&KeyCode::KeyS) {
            move_dir[0] -= f[0]; move_dir[1] -= f[1]; move_dir[2] -= f[2];
        }
        if keys.contains(&KeyCode::KeyD) {
            move_dir[0] += r[0]; move_dir[1] += r[1]; move_dir[2] += r[2];
        }
        if keys.contains(&KeyCode::KeyA) {
            move_dir[0] -= r[0]; move_dir[1] -= r[1]; move_dir[2] -= r[2];
        }

        self.move_by(move_dir, speed * ctx.dt);
    }
}
