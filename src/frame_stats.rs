//! Per-frame performance statistics with a given sampling window.
//!
//! [`FrameStats`] counts rendered frames and uses real elapsed wall-clock time
//! for its given sample window. Every given seconds, the accumulated data is
//! "committed" into the public fields so callers see a stable value rather
//! than per-frame jitter.

use crate::constants::frame_stats::SAMPLE_WINDOW_SECS_DEFAULT;

/// Smoothed performance counters.
///
/// Access these through [`FrameContext::stats`](crate::window::FrameContext).
#[derive(Debug, Clone)]
pub struct FrameStats {
    /// Frames per second, averaged over the last given window.
    pub fps: f32,
    /// Average frame time in milliseconds over the last given window.
    pub frame_time_ms: f32,
    /// Number of draw calls issued in the most recent committed window.
    pub draw_calls: u32,
    /// Number of triangles rendered in the most recent committed window.
    pub triangle_count: u32,

    /// Timestamp of the start of the current accumulation window.
    pub(crate) last_sample_time: web_time::Instant,
    /// Number of frames collected since the last commit.
    pub(crate) frames_collected: u32,
    /// Sleep time between frame stats.
    pub(crate) sample_window_secs: f32,
}

impl FrameStats {
    /// Create a new `FrameStats` with all counters zeroed.
    pub fn new() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            draw_calls: 0,
            triangle_count: 0,
            last_sample_time: web_time::Instant::now(),
            frames_collected: 0,
            sample_window_secs: SAMPLE_WINDOW_SECS_DEFAULT,
        }
    }
    /// Builder method to override the default sample window
    pub fn with_sample_window(mut self, secs: f32) -> Self {
        self.sample_window_secs = secs;
        self
    }

    /// Record one frame with the given delta-time `dt` (seconds).
    ///
    /// When the accumulated window exceeds [`SAMPLE_WINDOW_SECS`] the public
    /// fields are updated and the window resets.
    pub fn tick(&mut self, _dt: f32) {
        let now = web_time::Instant::now();
        self.frames_collected += 1;
        let sample_elapsed = now.duration_since(self.last_sample_time).as_secs_f32();

        if self.frames_collected > 0 && sample_elapsed >= self.sample_window_secs {
            self.fps = self.frames_collected as f32 / sample_elapsed;
            self.frame_time_ms = (sample_elapsed / self.frames_collected as f32) * 1000.0;
            self.frames_collected = 0;
            self.last_sample_time = now;
        }
    }

    /// Notify the stats of GPU work performed during a frame.
    ///
    /// These are passed through directly (not smoothed) and reflect the last
    /// frame's GPU workload as reported by the renderer.
    pub fn set_gpu_stats(&mut self, draw_calls: u32, triangle_count: u32) {
        self.draw_calls     = draw_calls;
        self.triangle_count = triangle_count;
    }
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new()
    }
}


