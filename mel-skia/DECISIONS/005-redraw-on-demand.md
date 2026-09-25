# 005 — Redraw-on-demand loop

## Flutter reference

Flutter never paints "because the screen refreshed" — it paints because
someone *asked* for a frame. The request side is
`SchedulerBinding.scheduleFrame()` → `PlatformDispatcher.scheduleFrame()` →
`Animator::RequestFrame()` in the engine; the grant side is the platform's
vsync channel. On iOS that channel is `VsyncWaiterIOS`
(`shell/platform/darwin/ios/framework/Source/vsync_waiter_ios.mm`), a thin
wrapper around a `CADisplayLink`:

- the display link is added to the **main run loop in common modes** (so it
  keeps firing while UIKit tracks touches) and starts **paused**;
- `AwaitVSync()` — one frame was requested — sets `paused = NO`;
- the tick callback sets `paused = YES` **first**, then reports the frame to
  the engine, which runs build/layout/paint and presents.

Three properties fall out of that tiny state machine, and they are the whole
feature:

1. **Idle costs nothing.** No request → link paused → the process burns no
   CPU. This is why an idle Flutter app sits at 0%.
2. **Requests coalesce.** N `scheduleFrame()` calls before the next vsync
   unpause an already-unpaused link — one tick, one frame. "Exactly one
   repaint per tap" is this property.
3. **Re-requesting from inside a frame works.** Pausing *before* invoking
   the callback means an animation that calls `scheduleFrame()` while
   drawing re-arms the link instead of having its request swallowed —
   continuous rendering without a special mode.

Events feed the loop from the app side: a `UITouch` becomes a framework
pointer event, a handler calls `setState`, `setState` calls
`scheduleFrame()`. The platform never decides that an event is worth a
frame; the app does.

Essential: paused-by-default link; request = unpause; tick = pause, then
callback; the app owns the "when to repaint" policy. Incidental: Flutter's
UI/raster thread ping-pong and the `Animator`'s pipeline depth (Mel is
main-thread-only until further notice), and the frame-timestamps payload —
that is feature 006.

## Decision

**A paused `CADisplayLink` becomes the only path to the screen;
`mel_platform::request_frame()` unpauses it; `run()` takes an `App` trait
instead of a bare draw closure so events can reach the app.**

- **`trait App { fn draw(&mut self, &mut Frame); fn on_touch(&mut self) {} }`**
  is the new platform↔app seam (`run(app: impl App + 'static)`). 005 is the
  moment the shell grows its second callback, and two `'static` closures
  sharing state would force `Rc<Cell>` plumbing into every stateful app; a
  struct with `&mut self` methods is the idiomatic Rust shape and mirrors
  Flutter's embedder interface (a table of callbacks). Examples 002–004
  adapt (they are regression checks, not frozen call sites — 004's rename
  set the precedent).
- **`on_touch()` is deliberately information-free** — no position, no phase,
  no pointer ID. It fires once per `touchesBegan:` and exists so an app can
  answer "the user poked the screen" with `request_frame()`, which is
  exactly what 005's "Done when" needs. Real pointer plumbing (Mel-typed
  events, stable IDs, began/moved/ended/cancelled) is feature **007**, which
  replaces this method — scheduled, not debt (the ADR-004
  `SafeAreaInsets`-vs-`EdgeInsets` precedent).
- **`request_frame()`** is a free function (Flutter's `scheduleFrame` is a
  singleton method; a free function is the Rust spelling of that). Main
  thread only, asserted loudly. Idempotent until the frame is granted:
  requests between two vsyncs coalesce into one draw. Calling it before
  `run()` is a documented no-op — the launch layout pass always requests
  the first frame anyway.
- **The link is created in `viewDidLoad`** — paused, targeted at the view
  controller (`onDisplayLink:` selector), added to the main run loop in
  common modes (Flutter's choice: ticks must not starve while UIKit tracks
  a touch). It is parked in a main-thread `thread_local` next to the app
  callback, because `request_frame()` — a free function — must reach it.
- **`viewDidLayoutSubviews` no longer renders.** It reads the metrics,
  memoizes them (unchanged from 004 — UIKit fires layout passes liberally),
  and on change calls `request_frame()`. Launch, rotation and safe-area
  changes are now just three more frame requesters, upholding the feature's
  `Platform events → App callback → Present` separation: events *request*,
  the tick *draws and presents*, nothing presents synchronously from an
  event handler anymore.
- **The tick (`onDisplayLink:`) pauses first, then renders**: re-read the
  metrics fresh from the view (never from a stale snapshot), build the
  `Frame`, run `App::draw`, wrap the pixels in a `CGImage`, install it as
  the layer contents — the 003/004 presentation path, moved verbatim from
  the layout callback into the tick.
- **Degenerate ticks drop the request but clear the metrics memo.** If the
  view is zero-sized or detached when the tick lands (possible
  mid-transition), there is nothing to present to; pausing anyway keeps
  idle-means-paused true, and clearing `LAST_METRICS` guarantees the next
  real layout pass re-requests the dropped frame instead of being memoized
  away.

## Alternatives considered

- **Keep 004's synchronous render in `viewDidLayoutSubviews`, add the link
  only for app-requested frames** — two render paths to keep consistent,
  and layout passes would still present from inside an event callback,
  which is exactly what the feature's separation forbids. One path via the
  link covers all requesters.
- **Shell auto-requests a frame on every touch** (no `App` trait needed) —
  fails the architecture: the platform would own repaint policy the app
  should own, and 007's design (app receives events, app decides) would be
  prefigured backwards. Rejected even though it is less code today.
- **`run(draw, on_touch)` as two closures** — state shared between them
  (the demo's tap counter) needs `Rc<Cell>` in every app. The trait's
  `&mut self` gives both methods the same state for free.
- **Blanket `impl<F: FnMut(&mut Frame)> App for F`** to keep the closure
  call sites — two spellings for one concept, and it breaks the moment
  `App` gains a second *required* method. One way to write an app.
- **Always-running link + dirty flag checked per tick** — the classic game
  loop. Ticks at 60/120 Hz while idle is precisely the CPU burn this
  feature exists to eliminate.
- **`NSTimer`/`CFRunLoopTimer`/background thread instead of
  `CADisplayLink`** — not vsync-aligned: tears, judder, and drift.
  `CADisplayLink` *is* the platform's vsync channel, and what Flutter uses.
- **Store the link in the view controller's ivars** — `request_frame()` is
  a free function and needs thread-local access anyway; a single owner (the
  thread local) beats two. The link↔target retain cycle this creates is
  moot either way: the root view controller lives for the process, and
  `run()` never returns.

## Consequences

- **006 slots straight in**: `CADisplayLink` carries `timestamp`/
  `targetTimestamp`; the tick hands them to `draw` through the same
  callback — no new channel, as ADR 004 predicted.
- **007 replaces `on_touch`** with typed pointer events on the same `App`
  trait; 008's lifecycle hooks land there too. The trait is now the seam
  the roadmap keeps extending.
- **Rotation renders vsync-paced**: each layout pass with new metrics
  re-arms the link, so the rotation animation produces at most one draw per
  refresh instead of one per layout pass. End state unchanged and crisp.
- **First frame is one vsync after the first layout pass** (imperceptible;
  Flutter behaves identically).
- **"Exactly one repaint" is per *granted* request**: two taps inside one
  vsync interval coalesce into one draw. That is Flutter's semantics too,
  and the correct one — the second tap's state is drawn, just not twice.
- A frame's presentation cost (buffer alloc + two copies into the
  `CGImage`) is unchanged from 004, now paid once per requested frame; the
  GPU decision (008) owns that question.
- `request_frame()` must never call into the `App` trait object (the app
  may be mid-`draw`, mutably borrowed from the same thread local). It only
  touches the link — keep it that way.

## Review outcome

Reviewed and approved by the user on 2026-09-19. No additional debt reported.
