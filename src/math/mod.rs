//! Column-major 4x4 matrix used for view, projection, and model transforms.
//!
//! All matrices follow the **column-major** memory layout required by WGSL
//! and the wgpu push-constant / uniform convention: `data[col][row]`.
pub mod matrix4;

pub use matrix4::Matrix4;