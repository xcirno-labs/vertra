use crate::viewport::Viewport;

pub struct Transform {
    pub position: [f32; 3],
    pub rotation: f32,  // All rotation-related data are measured in degrees
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Transform {
    pub fn from_position(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            ..Default::default()
        }
    }

    pub fn apply<const N: usize>(&self, points: [[f32; 3]; N], viewport: Viewport) -> [[f32; 3]; N] {
        let aspect_ratio = viewport.aspect_ratio();

        // We define our world units based on width (always -500 to +500).
        // Height will scale based on aspect ratio to prevent stretching.
        let virtual_width = 1000.0;
        let virtual_height = virtual_width / aspect_ratio;
        let mut output = [[0.0; 3]; N];

        for i in 0..N {
            // Scale
            let x = points[i][0] * self.scale[0];
            let y = points[i][1] * self.scale[1];

            // Rotate
            let rad = self.rotation.to_radians();
            let (sin_r, cos_r) = rad.sin_cos();
            let rx = x * cos_r - y * sin_r;
            let ry = x * sin_r + y * cos_r;

            // Translate
            let rx = rx + self.position[0];
            let ry = ry + self.position[1];
            // Normalize coordinate from Coordinate System to NDC (-1.0 to 1.0).
            // We scale by 0.5 since height and with has both negative and positive value.
            let nx = rx / (virtual_width * 0.5);
            let ny = ry / (virtual_height * 0.5);

            output[i] = [nx, ny, points[i][2] + self.position[2]]

        }
        output
    }
}