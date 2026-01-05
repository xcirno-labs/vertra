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

    pub fn apply<const N: usize>(&self, points: [[f32; 3]; N]) -> [[f32; 3]; N] {
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
            output[i] = [
                rx + self.position[0],
                ry + self.position[1],
                points[i][2] + self.position[2],
            ]

        }
        output
    }
}