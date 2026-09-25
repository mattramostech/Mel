# 007 — Touch input plumbing

## Flutter reference

Flutter's [iOS controller](https://github.com/flutter/engine/blob/main/shell/platform/darwin/ios/framework/Source/FlutterViewController.mm)
translates the four UIKit touch callbacks into pointer packets with phase,
position, timestamp, and touch identity. The converter assigns interaction
IDs. Framework [PointerEvent](https://api.flutter.dev/flutter/gestures/PointerEvent-class.html)
positions use logical pixels; each down gets a new ID, preserved through
move and up/cancel. Essential: independent contact lifetimes and a native
type boundary. Packets, physical-pixel transport, and Dart subclasses are
unnecessary for our single-threaded shell.

## Decision

Replace the temporary `App::on_touch` callback with this calling API:

```rust,ignore
fn on_pointer_event(&mut self, event: PointerEvent) {
    match event.phase {
        PointerPhase::Down | PointerPhase::Move => {
            self.contacts.insert(event.id, event.position);
        }
        PointerPhase::Up | PointerPhase::Cancel => {
            self.contacts.remove(&event.id);
        }
    }
    mel_platform::request_frame();
}
```

`PointerEvent`, `PointerId`, `PointerPhase`, and `PointerPosition` are plain
Rust carriers in `mel-platform`, like `FrameTime` and `SafeAreaInsets`.
Position is in logical points from the full viewport's top-left, including
the safe area; no scale multiplication, inset subtraction, or clamping.
`PointerPosition { x, y }` is temporary until foundation's Offset (010).
`timestamp: Duration` carries UITouch's monotonic seconds since system
startup; its origin is not wall time. No clock abstraction is introduced.

The main-thread shell owns a private tracker mapping native touch addresses
to monotonically allocated `PointerId(u64)` values. Addresses are identity
keys only, never dereferenced or exposed; UIKit keeps each touch alive for
its sequence. Up/cancel removes the mapping, so address reuse gets a fresh
ID. Duplicate downs and orphan move/up/cancel are ignored. ID exhaustion
fails explicitly instead of reusing an ID. Unit tests exercise this logic
without UIKit. Each callback's set contains changed touches only; unordered
iteration cannot change an existing contact's ID, and no ordering between
simultaneous contacts is promised.

Enable `multipleTouchEnabled` on the root view. The controller consumes all
four callbacks, translates every changed touch, releases the tracker borrow,
and calls the app. Drawing remains exclusively on request at vsync (005).
Examples 005/006 react only to Down; 007 logs events and paints a colored
square for each active contact, removing it on Up/Cancel.

## Alternatives considered

- Expose native addresses as IDs: reuse across interactions and a platform
  detail leaking through the public contract.
- Retain UIKit objects in the tracker: unnecessary while UIKit owns active
  sequences; keeping the tracker native-free makes host testing straightforward.
- Add hit testing, capture, gesture recognition, pressure/stylus metadata,
  predicted/coalesced samples, or add/remove/hover events: beyond 007's touch
  transport. Target dispatch and gesture recognition belong to Phases 5/8.

## Consequences

No library dependency edge is added. Public APIs remain UIKit-free. The
`on_touch` bootstrap is removed, and its two callers migrate in this diff.
Two fingers can independently move/end; neither ending nor cancellation
disturbs the other contact. Lifecycle-triggered cancellation beyond UIKit's
normal callbacks remains part of 008's lifecycle integration.

## Review outcome

Pending human review. Implementation self-check: Architecture ✅ · API
ergonomics ✅ · Understandability ✅ · Scope discipline ✅. Correctness:
automated checks pass; sustained native Move/Cancel and the visual contact
markers remain for manual review (see the validation limits below).

Validation on 2026-09-25:

- `make check` passed: formatting, host and Simulator clippy with warnings
  denied, 15 workspace tests (four new pointer tests), and two motion tests.
- `cargo build --locked --target aarch64-apple-ios -p mel-platform --example
  007_touch_input` passed (compile/link only; hardware deployment is 009).
- The one-command flow built, installed and launched 007 on iPhone 17 Pro /
  iOS 26.5. Logs showed two simultaneous Down contacts with distinct IDs,
  `active=2`, matching Up IDs, and `active=0`. The next contact got a fresh ID.
  The automation's short drag emitted Down/Up without intermediate Move;
  sustained native Move and interruption-driven Cancel were not observed.
  Host tests cover interleaved movement, independent cancellation, ID reuse,
  duplicate/orphan rejection, and preservation of sample data.
- Also installed/launched on iPhone 16 Pro / iOS 18.4. A direct `simctl`
  screenshot confirmed the honey frame. The UI automation screenshot had
  appeared white for both 007 and regression example 005; this was a capture
  limitation, not evidence of a blank rendered surface. Active markers were
  too brief to inspect with the automation's instant input actions.
- Examples 002–006 compile through Simulator all-targets clippy; 005 also
  launched and logged its initial draw. Manual review steps for 007 are in
  `examples/README.md`. No new product debt proposed; 002-1 remains for 008.
