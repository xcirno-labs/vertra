#[derive(Copy, Clone)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    pub fn aspect_ratio(&self) -> f32 {
        (self.width / self.height) as f32
    }
}