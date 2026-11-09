use web_time::{Duration, Instant};

/// Timing for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FrameTime {
    /// Time since the previous frame; zero for the first frame.
    pub(crate) delta: Duration,
    /// Time since the first frame.
    pub(crate) total: Duration,
}

/// Measures frame timing from timestamps supplied by the caller.
#[derive(Default)]
pub(crate) struct FrameClock {
    start: Option<Instant>,
    last: Option<Instant>,
}

impl FrameClock {
    pub(crate) fn tick(&mut self, now: Instant) -> FrameTime {
        let start = *self.start.get_or_insert(now);
        let delta = self
            .last
            .replace(now)
            .map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
        FrameTime {
            delta,
            total: now.saturating_duration_since(start),
        }
    }
}
