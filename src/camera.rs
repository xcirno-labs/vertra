use std::collections::HashSet;
use winit::keyboard::KeyCode;
use crate::math::Matrix4;
use crate::constants::camera;
use crate::window::FrameContext;

/// A perspective camera that defines the observer's position and orientation
/// in world space, and supplies the view-projection matrix used by the
/// rendering pipeline.
///
/// # Coordinate system
/// Vertra uses a **Y-up, left-handed** system.  The camera looks along the
/// positive Z axis by default.
///
/// # Builder pattern
/// Construct with [`Camera::new`] and then chain the `with_*` setters:
///
/// ```rust,ignore
/// let cam = Camera::new()
///     .with_position([0.0, 5.0, -10.0])
///     .with_fov(60.0)
///     .with_rotation(90.0, -20.0);
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Camera {
    /// World-space position of the camera (the "eye" point).
    pub eye: [f32; 3],
    /// World-space point the camera is looking at.
    pub target: [f32; 3],
    /// The world-up vector - almost always `[0.0, 1.0, 0.0]`.
    pub up: [f32; 3],
    /// Viewport aspect ratio (`width / height`).  Updated automatically on
    /// window resize.
    pub aspect: f32,
    /// Vertical field of view in **degrees**.
    pub fov: f32,
    /// Distance to the near clipping plane.  Objects closer than this are not
    /// rendered.
    pub znear: f32,
    /// Distance to the far clipping plane.  Objects farther than this are not
    /// rendered.
    pub zfar: f32,
    /// Horizontal (yaw) angle in degrees.  Drives the `target` direction via
    /// [`Camera::update_position`] / [`Camera::rotate`].
    pub lr_rot: f32,
    /// Vertical (pitch) angle in degrees, clamped to `(-89°, 89°)` to prevent
    /// gimbal flip.
    pub ud_rot: f32,
}

impl Camera {
    /// Create a camera with sensible defaults (eye at `[0, 2, 5]`, looking at
    /// the origin, 45° FOV, 0.1–1000 clip range).
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

    /// Override the aspect ratio (`width / height`).
    ///
    /// Called automatically by [`crate::window::Window`] when the viewport is
    /// resized, but you can also set it during initial setup.
    pub fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect = aspect;
        self
    }

    /// Set the vertical field of view in **degrees**.
    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    /// Set the near and far clipping planes.
    ///
    /// * `znear` - objects closer than this distance are clipped.
    /// * `zfar`  - objects farther than this distance are clipped.
    pub fn with_clip_planes(mut self, znear: f32, zfar: f32) -> Self {
        self.znear = znear;
        self.zfar = zfar;
        self
    }

    /// Set the world-space eye position.
    pub fn with_position(mut self, pos: [f32; 3]) -> Self {
        self.eye = pos;
        self
    }

    /// Set the yaw (`rotx`) and pitch (`roty`) angles in degrees and
    /// recompute [`Camera::target`] accordingly.
    pub fn with_rotation(mut self, rotx: f32, roty: f32) -> Self {
        self.lr_rot = rotx;
        self.ud_rot = roty;
        self.update_target_from_angles();
        self
    }

    /// Teleport the camera eye to `new_pos` without changing the look
    /// direction.
    pub fn update_position(&mut self, new_pos: [f32; 3]) {
        self.eye = new_pos;
    }

    /// Compute the combined view-projection matrix for the current camera
    /// state and return it as a [`Matrix4`].
    ///
    /// Used by the pipeline each frame to transform world-space vertices into
    /// NDC clip space.
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

    /// Apply a mouse-delta rotation.
    ///
    /// * `dx` - horizontal delta (positive = right in non-inverted mode).
    /// * `dy` - vertical delta (positive = down in non-inverted mode).
    /// * `inverted` - when `true`, both axes are reversed.
    ///
    /// Pitch is clamped to `±89°` to prevent the camera from flipping.
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

    /// Return the normalised **forward** and **right** vectors for the current
    /// camera orientation.
    ///
    /// Useful for computing movement directions in response to WASD input.
    ///
    /// # Returns
    /// `(forward, right)` - both unit-length, perpendicular to each other and
    /// to [`Camera::up`].
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

    /// Translate the camera (eye **and** target) by `direction * amount`.
    ///
    /// Moving both points together preserves the look direction.
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

    /// Process WASD keyboard movement for the current frame.
    ///
    /// Reads `W/A/S/D` from `keys` and moves the camera along the forward /
    /// right axes scaled by `speed * ctx.dt`.
    pub fn handle_default_input(&mut self, keys: &HashSet<KeyCode>, speed: f32, ctx: &mut FrameContext<'_>) {
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
