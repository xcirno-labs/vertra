pub struct Timer {
    pub elapsed: f32,
    duration: f32,
    finished: bool,
}

impl Timer {
    pub fn new(seconds: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration: seconds,
            finished: false,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.finished {
            self.elapsed += dt;
            if self.elapsed >= self.duration {
                self.finished = true;
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
    }
}
