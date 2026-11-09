use web_time::Instant;

/// Measures the time between frames and since the clock started.
pub struct FrameClock {
    start: Instant,
    last: Instant,
}

impl FrameClock {
    pub fn new(now: Instant) -> Self {
        Self {
            start: now,
            last: now,
        }
    }

    /// Advances to `now` and returns the frame-time and elapsed-time debug labels.
    pub fn tick(&mut self, now: Instant) -> Vec<String> {
        let frame = now.saturating_duration_since(self.last);
        self.last = now;
        let elapsed = now.saturating_duration_since(self.start);
        vec![
            format!("FRAME {:.2} MS", frame.as_secs_f64() * 1000.0),
            format!("ELAPSED {:.2} S", elapsed.as_secs_f64()),
        ]
    }
}
