use std::time::Duration;

use web_time::Instant;

use super::FrameClock;

#[test]
fn reports_frame_and_elapsed_time() {
    let start = Instant::now();
    let mut clock = FrameClock::new(start);
    assert_eq!(
        clock.tick(start + Duration::from_millis(1000)),
        ["FRAME 1000.00 MS", "ELAPSED 1.00 S"]
    );
    assert_eq!(
        clock.tick(start + Duration::from_micros(1_016_667)),
        ["FRAME 16.67 MS", "ELAPSED 1.02 S"]
    );
}
