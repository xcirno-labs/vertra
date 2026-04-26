/// A simple countdown timer for use in game logic.
///
/// Tracks how much time has elapsed since it was last reset and exposes a
/// one-shot `is_finished` flag that stays `true` until explicitly reset.
///
/// Call [`Timer::update`] once per frame with the frame delta-time.
///
/// # Example
/// ```rust,ignore
/// let mut spawn_timer = Timer::new(2.0); // fires after 2 seconds
///
/// fn on_update(state: &mut State, _scene: &mut Scene, ctx: &mut FrameContext) {
///     spawn_timer.update(ctx.dt);
///     if spawn_timer.is_finished() {
///         // spawn something …
///         spawn_timer.reset();
///     }
/// }
/// ```
pub struct Timer {
    /// Total time elapsed since the last [`Timer::reset`], in seconds.
    pub elapsed: f32,
    duration: f32,
    finished: bool,
}

impl Timer {
    /// Create a new timer that fires after `seconds` have elapsed.
    pub fn new(seconds: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration: seconds,
            finished: false,
        }
    }

    /// Advance the timer by `dt` seconds.
    ///
    /// Once the accumulated elapsed time reaches or exceeds the duration the
    /// timer is marked as finished and stops advancing until [`Timer::reset`]
    /// is called.
    pub fn update(&mut self, dt: f32) {
        if !self.finished {
            self.elapsed += dt;
            if self.elapsed >= self.duration {
                self.finished = true;
            }
        }
    }

    /// Returns `true` if the timer has reached its duration.
    ///
    /// The flag remains `true` until [`Timer::reset`] is called.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Reset the timer to zero elapsed time and clear the finished flag.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
    }
}
