// web-time, the engine clock, calls performance.now() on every Instant::now() through a dynamic
// method lookup, so replacing it freezes the animation and labels without test hooks in
// production code. page.clock would also fake requestAnimationFrame, which drives winit redraws.
// Call window.rengSetTime(milliseconds) in the page to move the frozen clock.
exports.freezeClock = (page) => page.addInitScript(() => {
  let now = 0;
  performance.now = () => now;
  window.rengSetTime = (milliseconds) => { now = milliseconds; };
});
