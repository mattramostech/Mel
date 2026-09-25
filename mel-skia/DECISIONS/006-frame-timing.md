# 006 — Frame timing

## Flutter reference

Flutter's [`PlatformDispatcher.onBeginFrame`](https://api.flutter.dev/flutter/dart-ui/PlatformDispatcher/onBeginFrame.html)
delivers a vsync timestamp only after a frame request. Animation uses this
shared time instead of counting frames or sampling a clock in each widget.
The iOS [`VsyncWaiterIOS`](https://github.com/flutter/flutter/blob/master/engine/src/flutter/shell/platform/darwin/ios/framework/Source/vsync_waiter_ios.mm)
receives start and target times, translates the clock domain, and forwards
them to the engine. Mel retains the platform clock domain because its
callbacks all run on the main thread.

Flutter's [`FrameTiming`](https://api.flutter.dev/flutter/dart-ui/FrameTiming-class.html)
separately reports work durations and vsync overhead. The essential distinction
is animation time versus work measurement; its build/raster thread split and
batched reporting are unnecessary for Mel's single-thread CPU renderer.

## Decision

Keep `App::draw(&mut Frame)` and add `Frame::time() -> FrameTime` with
`timestamp` and `target_timestamp` as `std::time::Duration` values from the
same monotonic, unspecified platform epoch. On iOS they come directly from
`CADisplayLink.timestamp` and `targetTimestamp`. They are not wall time, app
uptime, or a Rust `Instant`. Differences are meaningful; the origin is not.
The interval is recomputed every frame, never assumed to be 1/60 second.

Calling code, before implementation:

```rust,ignore
fn draw(&mut self, frame: &mut Frame) {
    let time = frame.time();
    let origin = *self.origin.get_or_insert(time.timestamp);
    let elapsed = time.timestamp.saturating_sub(origin).as_secs_f64();
    // Draw a rectangle at a position calculated from elapsed.
    mel_platform::request_frame();
}

fn on_frame_timing(&mut self, timing: FrameTiming) {
    // Compare timing.cpu_duration with timing.frame_time.interval().
}
```

`FrameTime` is a copyable value carried by the shell-owned `Frame`;
`App::on_frame_timing` is an optional, default-no-op callback after layer
submission. It receives CPU duration (tick entry through `CALayer.contents`
assignment) and deadline overrun against `target_timestamp`. Both samples
use `CACurrentMediaTime`, the display link's clock. This includes viewport
reads, allocation, app drawing and image copies; it excludes the reporting
callback and does **not** claim to measure compositor/GPU/display completion.
A late callback can miss the deadline even when CPU work fits the interval.

Invalid/non-finite or reversed platform timestamps drop that tick and clear
the metrics memo, like a degenerate viewport. No fabricated 60 Hz fallback.
The paused-on-entry, re-request-from-callback contract of 005 stays intact.
The demo logs a sample about once per timestamp-second to avoid per-frame
console traffic. The library never logs unconditionally.

## Alternatives considered

- Change every `draw` signature or add a second begin-frame callback:
  the frame already groups immutable per-frame metadata with its pixels.
- Frame counters/fixed deltas: animation speed would depend on refresh rate
  and dropped frames. The example computes position from absolute elapsed time.
- Measure only `App::draw`: hides allocation and presentation-copy costs that
  feature 008 needs to evaluate. Measure the whole shell submission instead.
- Build a clock abstraction, ticker registry, or performance overlay now:
  those belong to 016, 036, and 131 respectively.

## Consequences

Existing examples keep their signatures and idle behavior. Timing types are
platform-owned bootstrap carriers, like safe-area insets; 016 can unify time
conventions later without adding an upward dependency today. CPU rendering
and Simulator timing are not real-device performance evidence (008/009).
After a long idle gap, timestamps advance naturally; apps choose whether to
reset their animation origin on resume. The demo uses a four-second cosine
cycle, remains within the safe area after rotation, and requests another
frame only while running. A tap toggles animation; resuming resets its origin.

## Review outcome

Reviewed and approved 2026-09-25. Correctness ✅ ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No new debt proposed; existing lifecycle debt 002-1 still belongs to 008.

Validation on 2026-09-19:

- `make check` passed: formatting, host and Simulator clippy with warnings
  denied, 11 workspace unit tests and two deterministic example tests.
- `cargo build --locked --target aarch64-apple-ios -p mel-platform --example
  006_frame_timing` passed (compile/link only; physical-device run is 009).
- The one-command Simulator flow built, installed and launched 006 on an
  iPhone 16 Pro / iOS 18.4. Observed motion, pause/console silence, rotation
  while paused, and successful restart in landscape within the safe area.
- Debug timing samples showed a 16.67 ms interval and typically about
  2–5 ms CPU submission cost. Some callbacks arrived late and reported a
  positive deadline overrun despite short CPU work, as intended. These are
  sampled Simulator observations, not a benchmark or proof of display latency.
- Regression demo 005 still rendered once per tap and stayed silent when
  idle; rotation requested its own frame. Earlier examples compile through
  the all-targets Simulator clippy check.
