use wasm_bindgen::prelude::*;

/// Smoothed frame statistics exposed to JavaScript.
#[wasm_bindgen]
#[derive(Clone)]
pub struct FrameStats {
    fps: f32,
    frame_time_ms: f32,
    draw_calls: u32,
    triangle_count: u32,
}

impl From<&vertra::frame_stats::FrameStats> for FrameStats {
    fn from(stats: &vertra::frame_stats::FrameStats) -> Self {
        Self {
            fps: stats.fps,
            frame_time_ms: stats.frame_time_ms,
            draw_calls: stats.draw_calls,
            triangle_count: stats.triangle_count,
        }
    }
}

#[wasm_bindgen]
impl FrameStats {
    /// Frames per second averaged over the last 0.5-second sample window.
    #[wasm_bindgen(getter)]
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Average frame time in milliseconds over the last 0.5-second sample window.
    #[wasm_bindgen(getter)]
    pub fn frame_time_ms(&self) -> f32 {
        self.frame_time_ms
    }

    /// Draw calls issued during the most recently rendered frame.
    #[wasm_bindgen(getter)]
    pub fn draw_calls(&self) -> u32 {
        self.draw_calls
    }

    /// Triangles rendered during the most recently rendered frame.
    #[wasm_bindgen(getter)]
    pub fn triangle_count(&self) -> u32 {
        self.triangle_count
    }
}
