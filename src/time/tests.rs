use web_time::{Duration, Instant};

use super::{FrameClock, clock::FrameTime};

#[test]
fn first_tick_starts_at_zero() {
    let mut clock = FrameClock::default();
    assert_eq!(
        clock.tick(Instant::now()),
        FrameTime {
            delta: Duration::ZERO,
            total: Duration::ZERO,
        }
    );
}

#[test]
fn ticks_report_delta_and_total() {
    let start = Instant::now();
    let mut clock = FrameClock::default();
    clock.tick(start);
    clock.tick(start + Duration::from_millis(16));
    assert_eq!(
        clock.tick(start + Duration::from_millis(50)),
        FrameTime {
            delta: Duration::from_millis(34),
            total: Duration::from_millis(50),
        }
    );
}

#[test]
fn earlier_timestamps_saturate_to_zero() {
    let start = Instant::now();
    let mut clock = FrameClock::default();
    clock.tick(start + Duration::from_millis(10));
    let time = clock.tick(start);
    assert_eq!(time.delta, Duration::ZERO);
    assert_eq!(time.total, Duration::ZERO);
}
