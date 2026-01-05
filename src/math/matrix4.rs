use std::ops::Mul;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Matrix4 {
    pub data: [[f32; 4]; 4],
}

impl Matrix4 {
    pub fn identity() -> Self {
        Self {
            data: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn mul_vec4(&self, v: [f32; 4]) -> [f32; 4] {
        let mut res = [0.0; 4];
        for row in 0..4 {
            res[row] = self.data[0][row] * v[0] +
                self.data[1][row] * v[1] +
                self.data[2][row] * v[2] +
                self.data[3][row] * v[3];
        }
        res
    }

    pub fn perspective(fov_deg: f32, aspect: f32, near: f32, far: f32) -> Self {
        // We are using WGPU-compatible version of this camera perspective projection formula:
        // https://jsantell.com/3d-projection/#field-of-view. We use Column-Major version of
        // the formula, that is why the matrix will be flipped.
        let fov_rad = fov_deg.to_radians();
        let g = 1.0 / (fov_rad / 2.0).tan();

        let mut data = [[0.0; 4]; 4];
        data[0][0] = g / aspect;
        data[1][1] = g;
        // WGPU version (strictly positive)
        data[2][2] = far / (far - near);
        // We use 1.0 for Left-Handed system where +Z is forward, the URL uses Right-Handed
        // system where -Z is forward.
        data[2][3] = 1.0;
        // WGPU uses 0.0 to 1.0 for depth range, that means we do not need to multiply it by 2
        // like in the previous url, since they are using OpenGL which has depth range -1.0 to 1.0.
        data[3][2] = far * near / (near - far);
        data[3][3] = 0.0;

        Self { data }
    }

    pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Self {
        // The 'Forward' vector (Forward = Target - Eye)
        let f = {
            let d = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
            let len = (d[0]*d[0] + d[1]*d[1] + d[2]*d[2]).sqrt();
            [d[0]/len, d[1]/len, d[2]/len]
        };

        // The 'Right' vector (Right = Forward x Up)
        let r = {
            let d = [f[1]*up[2] - f[2]*up[1], f[2]*up[0] - f[0]*up[2], f[0]*up[1] - f[1]*up[0]];
            let len = (d[0]*d[0] + d[1]*d[1] + d[2]*d[2]).sqrt();
            [d[0]/len, d[1]/len, d[2]/len]
        };

        // The 'Up' vector (Up = Right x Forward)
        let u = [r[1]*f[2] - r[2]*f[1], r[2]*f[0] - r[0]*f[2], r[0]*f[1] - r[1]*f[0]];

        let mut res = Self::identity();

        // Orientation part (Rows of the rotation part of the matrix)
        res.data[0][0] = r[0]; res.data[1][0] = r[1]; res.data[2][0] = r[2];
        res.data[0][1] = u[0]; res.data[1][1] = u[1]; res.data[2][1] = u[2];
        res.data[0][2] = f[0]; res.data[1][2] = f[1]; res.data[2][2] = f[2];

        // Translation part (Camera position offset)
        res.data[3][0] = -(r[0]*eye[0] + r[1]*eye[1] + r[2]*eye[2]);
        res.data[3][1] = -(u[0]*eye[0] + u[1]*eye[1] + u[2]*eye[2]);
        res.data[3][2] = -(f[0]*eye[0] + f[1]*eye[1] + f[2]*eye[2]);

        res
    }

    pub fn project_point(&self, p: [f32; 3]) -> [f32; 3] {
        let v = self.mul_vec4([p[0], p[1], p[2], 1.0]);

        // Perspective Divide: [x/w, y/w, z/w]
        [v[0] / v[3], v[1] / v[3], v[2] / v[3]]
    }
}


impl Mul for Matrix4 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        let mut res = [[0.0; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                res[col][row] = (0..4).map(|i| self.data[i][row] * other.data[col][i]).sum();
            }
        }
        Self { data: res }
    }
}