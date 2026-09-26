# 008 — App lifecycle & Metal decision

## Flutter reference

Flutter's [iOS controller](https://github.com/flutter/flutter/blob/master/engine/src/flutter/shell/platform/darwin/ios/framework/Source/FlutterViewController.mm)
maps activation, interruption, background and foreground notifications to
resumed/inactive/paused messages. The essential pieces are lifecycle delivery,
surface availability and ending interrupted contacts. Mel needs no channel
serialization or synthetic cross-platform hidden/detached states yet.

## Decision

Calling code designed before implementation:

```rust,ignore
fn on_lifecycle_changed(&mut self, state: AppLifecycleState) {
    if state == AppLifecycleState::Active {
        self.animation_origin = None;
    }
}
fn on_memory_warning(&mut self) {
    self.cache.clear();
}
```

Add native-free `Active`, `Inactive`, `Background` states and two optional
callbacks to `App`. Keep the classic application delegate for this single
window. Initial state is Inactive; the first activation is delivered. Duplicate
states are ignored. Foreground entry is Inactive, not yet permission to draw.
Unlike Flutter (which can render while inactive), Mel pauses already on
resigning active. Only Active grants frames. A private, host-tested frame-loop
state owns the pending request separately from the display link: requests
coalesce and survive suspension, even if made inside lifecycle callbacks.
Activation always requests one fresh frame, including for static apps.

Commit state and pause the link before invoking any app code. Drain remaining
contacts into Cancel events at their last position before the lifecycle
callback; UIKit's late terminal events are ignored. Cancellation uses current
monotonic time, never earlier than the last sample. Ignore touch delivery
while inactive. Release all shell state borrows before calling app code so
every callback can safely request a frame. Memory warnings are independent of
lifecycle and go directly to the app; no cache infrastructure exists yet.

**Keep CPU raster for now.** ADR 006 measured roughly 2–5 ms CPU submission
against a 16.67 ms Simulator interval for a minimal scene. This supports the
current demos, not a hardware performance claim. The [rust-skia Metal example](https://github.com/rust-skia/rust-skia/blob/master/skia-safe/examples/metal-window/main.rs)
requires a Metal device/queue, drawable acquisition, GPU context, backend
surface wrapping and submission. Mel's current callback exposes a CPU buffer;
a true GPU path also needs a painting-owned drawing seam to avoid leaking
Skia into platform. Adding Metal while retaining that buffer would still
upload CPU-rasterized images. Keep the known allocation/copy cost and record
008-1; reconsider with 009's hardware timing and before Phase 2 Canvas design.

## Alternatives considered

- Only pause on background: leaves interruption-time drawing running. Choose
  a stricter active-only policy and make the difference explicit.
- Derive pending work from `CADisplayLink.isPaused`: cannot distinguish idle
  from suspended; loses animation requests across backgrounding.
- Adopt UIScene now: changes launch/window ownership as well as hooks. Keep
  the tested shell for this feature; re-defer 002-1 to 009's toolchain/device
  compatibility check and the Phase 0 gate. This is not a claim that classic
  lifecycle is supported indefinitely by future SDKs.
- Switch to Metal now: defer until hardware evidence and a suitable painting
  API justify the extra backend and ownership changes.

## Consequences

No dependency edges or UIKit types enter the public API. Existing apps resume
without modification; animation timestamps still advance during suspension,
so apps choose whether to reset their origin. There is no guaranteed shutdown
notification or background execution. Memory warning handlers must be brief
and release disposable app resources; they may request a deferred frame.
The demo animates, logs lifecycle/frame counts and sampled timings, and changes
color after a memory warning. It deliberately requests frames while suspended
to exercise the gate. Earlier examples remain regression checks.

## Review outcome

Reviewed and approved by the user on 2026-09-25. Implementation self-check: Correctness ✅ for the
checks below (native held-touch interruption remains a manual check) ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
Accepted debt: retained classic lifecycle (002-1) and CPU backend (008-1).

Validation on 2026-09-25:

- `make check` passed: fmt, host and Simulator all-targets clippy with warnings
  denied, 19 workspace tests and two motion-example tests. New tests cover
  inactive/queued ticks, retained/coalesced requests, activation of static
  apps, interruption without background, and cancellation of all contacts
  once with last positions, monotonic times and fresh subsequent IDs.
- Device-target `cargo build --locked --target aarch64-apple-ios -p
  mel-platform --example 008_app_lifecycle` passed (compile/link only).
- One-command flow installed/launched 008 on iPhone 17 Pro / iOS 26.5.
  Native Home/reopen delivered Active → Inactive → Background → Inactive →
  Active. The same process held at frame 972 across background and resumed
  at 973; a second cycle held at 10937 and resumed at 10938. Lifecycle
  callbacks requesting frames did not bypass suspension.
- Rotation remained drawable; direct Simulator screenshots confirmed the
  square and honey background, then green after a memory warning. Timing
  samples were typically about 2–5 ms CPU at a 16.67 ms interval, with some
  overruns. These debug/Simulator samples are not a hardware benchmark.
- The Simulator menu had no memory-warning action. LLDB invoked the actual
  delegate selector on the main thread and the app logged warning 1 and
  changed color. A second injection attempted while suspended waited for
  foregrounding; it delivered while Mel still held Background at frame 10937,
  followed by Inactive/Active and frame 10938. This verifies callback routing
  and deferral, not OS pressure generation or background execution guarantees.
  Reproduction steps are in `examples/README.md`.
- Regression 005 launched with draw #1, remained idle, and produced only
  draw #2 after Home/reopen. All earlier examples compile in Simulator
  clippy. Held-touch native cancellation and a short system-overlay
  interruption remain manual checks; the underlying state/tracker logic is
  host-tested. No UIKit/Skia types were added to the public API.
