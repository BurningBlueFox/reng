use std::time::Duration;

use web_time::Instant;

/// Measures the time between frames and since the clock started.
pub struct FrameClock {
    start: Instant,
    last: Instant,
    frame: Duration,
}

impl FrameClock {
    pub fn new(now: Instant) -> Self {
        Self {
            start: now,
            last: now,
            frame: Duration::ZERO,
        }
    }

    /// Advances to `now` and returns the frame-time and elapsed-time debug labels.
    pub fn tick(&mut self, now: Instant) -> Vec<String> {
        self.frame = now.saturating_duration_since(self.last);
        self.last = now;
        vec![
            format!("FRAME {:.2} MS", self.frame.as_secs_f64() * 1000.0),
            format!("ELAPSED {:.2} S", self.elapsed().as_secs_f64()),
        ]
    }

    /// Time between the latest two ticks.
    pub fn frame(&self) -> Duration {
        self.frame
    }

    /// Time from the clock's start to its latest tick.
    pub fn elapsed(&self) -> Duration {
        self.last.saturating_duration_since(self.start)
    }
}
