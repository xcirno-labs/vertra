use crate::math::Matrix4;

pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 3],  // All rotation-related data are measured in degrees
    pub scale: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
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
    
    pub fn to_matrix(&self) -> Matrix4 {
        // Create Translation Matrix
        let mut translation = Matrix4::identity();
        translation.data[3][0] = self.position[0];
        translation.data[3][1] = self.position[1];
        translation.data[3][2] = self.position[2];

        let rx = self.rotation[0].to_radians();
        let ry = self.rotation[1].to_radians();
        let rz = self.rotation[2].to_radians();

        // Create Rotation Matrices
        // Reference: https://en.wikipedia.org/wiki/Rotation_matrix
        let mut rot_x = Matrix4::identity();
        let (sx, cx) = rx.sin_cos();
        rot_x.data[1][1] = cx;
        rot_x.data[1][2] = sx;
        rot_x.data[2][1] = -sx;
        rot_x.data[2][2] = cx;

        let mut rot_y = Matrix4::identity();
        let (sy, cy) = ry.sin_cos();
        rot_y.data[0][0] = cy;  rot_y.data[0][2] = -sy;
        rot_y.data[2][0] = sy;  rot_y.data[2][2] = cy;

        let mut rot_z = Matrix4::identity();
        let (sz, cz) = rz.sin_cos();
        rot_z.data[0][0] = cz;  rot_z.data[0][1] = sz;
        rot_z.data[1][0] = -sz; rot_z.data[1][1] = cz;

        // Combine Rotations
        let rotation = rot_y * rot_x * rot_z;

        // Create Scale Matrix
        let mut scale = Matrix4::identity();
        scale.data[0][0] = self.scale[0];
        scale.data[1][1] = self.scale[1];
        scale.data[2][2] = self.scale[2];

        // Combine them: Model = Translation * Rotation * Scale
        translation * rotation * scale
    }
    
    pub fn apply<const N: usize>(&self, points: [[f32; 3]; N]) -> [[f32; 3]; N] {
        let model_matrix = self.to_matrix();

        // Apply to all points
        let mut output = [[0.0; 3]; N];
        for i in 0..N {
            // Convert [f32; 3] to [f32; 4] for the matrix math
            let v4 = [points[i][0], points[i][1], points[i][2], 1.0];
            let transformed = model_matrix.mul_vec4(v4);

            // Drop the w component to return to [f32; 3]
            output[i] = [transformed[0], transformed[1], transformed[2]];
        }
        output
    }
    
    pub fn combine(&self, child: &Transform) -> Self {
        let parent_m = self.to_matrix();
        let child_m = child.to_matrix();
        let combined_m = parent_m * child_m;

        let mut t = Transform::default();
        t.position = [
            combined_m.data[3][0],
            combined_m.data[3][1],
            combined_m.data[3][2],
        ];
        t.rotation = [
            self.rotation[0] + child.rotation[0],
            self.rotation[1] + child.rotation[1],
            self.rotation[2] + child.rotation[2],
        ];
        t.scale = [
            self.scale[0] * child.scale[0],
            self.scale[1] * child.scale[1],
            self.scale[2] * child.scale[2],
        ];
        t
    }
}