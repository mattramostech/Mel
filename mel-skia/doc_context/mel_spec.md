# Mel — Implementation Spec (current state)

A snapshot of **what is actually implemented right now**: what exists, where
it lives, how to run it, and which decisions are in force. Read it to decide
what to do next or to jump straight to the code you need to change, without
re-deriving the project state from git history.

Rules of this file:

- **Update it as part of every feature's diff** (it describes "today", so a
  feature that changes "today" must update it).
- It never replaces the sources of truth: status lives in
  [ROADMAP.md](ROADMAP.md) checkboxes, process in [CONTEXT.md](CONTEXT.md),
  rationale in `DECISIONS/`. On any conflict, those win — then fix this file.

_Last updated: 2026-09-19, after building feature 006 (pending review)._

---

## 1. Status at a glance

- **Done (reviewed):** 001 (workspace & toolchain), 002 (app shell & blank
  app), 003 (Skia surface on screen), 004 (scale factor, safe areas &
  rotation), 005 (redraw-on-demand loop).
- **In progress:** 006 — Frame timing (built, pending review).
- **Next up:** 007 — Touch input plumbing.
- **Open debt:** 002-1 (classic lifecycle vs `UIScene` — see [DEBT.md](../DEBT.md)).

## 2. What runs today

```sh
scripts/run-sim.sh 006_frame_timing   # or: make run
```

launches a honey background with an espresso rectangle moving horizontally
inside the safe area. Position comes exclusively from display-link timestamps,
with a four-second cycle independent of refresh rate or skipped frames. A tap
pauses; another tap restarts the cycle. Rotation updates the physical buffer
and safe-area travel range. The console samples CPU submission time versus
refresh interval and deadline about once per timestamp-second (ADR 006).

The app remains pure Rust/UIKit, CPU-rasterized through Skia (no Xcode project,
Swift, or winit). Frames still exist only on demand (ADR 005): the animation
requests the next frame from `draw`; pausing stops requests and lets the
`CADisplayLink` go idle after any pending frame. Examples 002–005 remain
runnable regression checks; 005 still demonstrates one repaint per tap.

## 3. Repository map (real vs. placeholder)

```text
mel-skia/
├── doc_context/          CONTEXT.md (process) · ROADMAP.md (status) · this file
├── DECISIONS/            001 (layout) · 002 (app shell) · 003 (Skia surface) · 004 (scale/insets/rotation) · 005 (redraw on demand) · 006 (frame timing)
├── DEBT.md               one open item (002-1)
├── examples/             002_blank_app.rs · 003_skia_surface.rs · 004_scale_rotation.rs · 005_redraw_on_demand.rs · 006_frame_timing.rs + README
├── scripts/run-sim.sh    the one-command Simulator flow (ADR 002)
├── rust-toolchain.toml   stable · edition 2024 · both iOS targets pinned
├── .github/workflows/    CI exists but is disabled/commented (ADR 001 amendment)
└── crates/               one crate per layer (ADR 001)
```

| Crate | Layer (CONTEXT.md §5) | State |
| --- | --- | --- |
| `mel-foundation` | geometry, color, ids | empty placeholder |
| `mel-painting` | Canvas/Paint over Skia — the only crate ever allowed `skia-safe` | **bootstraps `fill()` (003) + `fill_rect()` (004)** — see §5 |
| `mel-compositing` | layer tree, scenes | empty placeholder |
| `mel-scheduling` | frame pipeline, tickers | empty placeholder |
| `mel-rendering` | RenderObject, layout | empty placeholder |
| `mel-widgets` | Widget/Element trees | empty placeholder |
| `mel-platform` | iOS app shell | **implemented (002–006)** — see §4 |

Inter-crate dependency edges are added only when a feature first needs them.
Exactly one exists: `mel-platform` → `mel-painting` as a **dev-dependency**
(examples are app-level code above every layer — ADR 003); the libraries
themselves still have zero edges, and `cargo tree -i skia-safe` shows Skia
confined to `mel-painting`.

## 4. `mel-platform` — the app shell (features 002–006)

Public API — this is everything layers above can see:

```rust
pub trait App {                                // the platform↔app seam (ADR 005)
    fn draw(&mut self, frame: &mut Frame);     // once per granted frame request
    fn on_frame_timing(&mut self, timing: FrameTiming) {} // after CPU submission
    fn on_touch(&mut self) {}                  // touch-began; info-free stub until 007
}

// iOS-only (#[cfg(target_os = "ios")]):
mel_platform::run(app: impl App + 'static) -> !
mel_platform::request_frame()                  // schedule exactly one draw at next vsync;
                                               // idempotent until granted (coalesces);
                                               // main-thread only

pub struct SafeAreaInsets { pub left/top/right/bottom: f64 } // logical points

pub struct FrameTime {
    pub timestamp: std::time::Duration,        // monotonic platform epoch
    pub target_timestamp: std::time::Duration, // expected next refresh
}
impl FrameTime { pub fn interval(self) -> std::time::Duration; }
pub struct FrameTiming {
    pub frame_time: FrameTime,
    pub cpu_duration: std::time::Duration,     // tick entry through submission
    pub deadline_overrun: std::time::Duration, // zero if submitted by target
}

pub struct Frame { /* physical-pixel BGRA buffer + viewport metrics + time */ }
impl Frame {
    pub fn time(&self) -> FrameTime;      // immutable per-frame timestamps
    pub fn physical_width(&self) -> u32;   // buffer dimensions
    pub fn physical_height(&self) -> u32;
    pub fn scale_factor(&self) -> f64;     // Flutter's devicePixelRatio
    pub fn logical_width(&self) -> f64;    // physical ÷ scale
    pub fn logical_height(&self) -> f64;
    pub fn safe_area(&self) -> SafeAreaInsets;
    pub fn pixels(&self) -> &[u8];
    pub fn pixels_mut(&mut self) -> &mut [u8]; // BGRA8888, premul, stride = w*4
}
```

- [`src/lib.rs`](../crates/mel-platform/src/lib.rs) — the API above; no UIKit
  type leaks upward (CONTEXT.md §5). `Frame` carries the viewport metrics it
  was sized for (ADR 004); `mel_painting`'s tests pin the pixel contract.
- [`src/timing.rs`](../crates/mel-platform/src/timing.rs) — `FrameTime` and
  `FrameTiming`, platform-independent duration math and tests. iOS converts
  `timestamp`/`targetTimestamp` to durations from their shared monotonic epoch.
  Subtract timestamps for animation; do not compare with wall time or `Instant`.
  Invalid platform samples are rejected rather than replaced with a fixed rate.
- [`src/ios.rs`](../crates/mel-platform/src/ios.rs) — the only file that
  speaks Objective-C. `define_class!` registers two Rust-implemented classes:
  `MelAppDelegate` (creates the `UIWindow`, installs the root VC, keeps the
  window alive in its ivars) and `MelViewController`, which owns the frame
  loop (ADR 005, mirroring Flutter's `VsyncWaiterIOS`): a `CADisplayLink`
  created **paused** in `viewDidLoad` (main run loop, common modes) is the
  only path to the screen. Events _request_ frames — `viewDidLayoutSubviews`
  (first display, rotation, safe-area changes) memoizes the metrics snapshot
  and requests on change; `touchesBegan` forwards to `App::on_touch`, where
  the app may call `request_frame()`. The tick (`onDisplayLink:`) repauses
  first (so `draw` can re-request — continuous animation), re-reads bounds
  × `window.screen.scale` + `safeAreaInsets` fresh (not `contentScaleFactor` —
  it stays 1.0 for views UIKit never draws into; ADR 004's Alternatives),
  rasters a physical-pixel `Frame` through `App::draw` (the app is parked in
  a main-thread `thread_local` by `run()`), and presents it as the layer's
  contents with `contentsScale` set (CPU raster — ADR 003). The tick also
  attaches the display-link times and samples `CACurrentMediaTime` at entry
  and after submission, then calls `App::on_frame_timing` outside `draw`'s
  mutable borrow. CPU cost includes allocation/drawing/copies, excludes the
  reporting callback and actual compositor/display completion. Deadline
  overrun compares submission to `targetTimestamp`, so late ticks are visible
  even when their CPU work fits the refresh interval. Idle = link paused.
- **Lifecycle model:** classic `UIApplicationDelegate`/`UIWindow`, _not_
  scene-based (`UIScene`) — debt 002-1, re-evaluated at 008. The
  `#[expect(deprecated)]` on `UIScreen::mainScreen` / `UIWindow::initWithFrame`
  is deliberate: those are scenes-era deprecations that don't apply to a
  scene-less app. Don't "fix" them independently of 002-1.
- **Dependencies** ([Cargo.toml](../crates/mel-platform/Cargo.toml)):
  `objc2`, `objc2-foundation`, `objc2-ui-kit`, and (since 003)
  `objc2-core-foundation`, `objc2-core-graphics`, `objc2-quartz-core` —
  iOS-target-only (`[target.'cfg(target_os = "ios")'.dependencies]`), with
  minimal explicit feature lists. Touching a new UIKit/CoreGraphics class
  usually means adding its feature flag there.

**Examples wiring (ADR 002):** demo files live at `examples/NNN_slug.rs`
(repo convention) and are declared as `[[example]]` targets of `mel-platform`
with an explicit `path`. Each example guards `main` with
`#[cfg(target_os = "ios")]` plus a host-side stub so workspace-wide
clippy/test stay green on the Mac.

## 5. `mel-painting` — bootstrap Skia raster (features 003–004)

Two functions, deliberately temporary until Phase 2's real Canvas/Paint:

```rust
mel_painting::fill(pixels: &mut [u8], width: u32, height: u32, color: [f32; 4])
mel_painting::fill_rect(pixels, width, height, rect: [f32; 4] /* x,y,w,h */, color)
```

Both wrap the caller's buffer in a Skia raster surface
(`surfaces::wrap_pixels`, `BGRA8888` + premultiplied alpha) and draw through
a Skia canvas: `fill` clears everything, `fill_rect` fills one sub-rectangle
(clipped to the buffer, anti-aliasing off so edges are measurable — ADR
004). No Skia type appears in the signatures. Unit-tested on the host (byte
order, premultiplication, clipping, size validation).

## 6. Build, check & run flows

Host (must be clean before every review — CONTEXT.md §6):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p mel-platform --example 006_frame_timing
```

iOS (the shell code only compiles for iOS targets):

```sh
cargo clippy --target aarch64-apple-ios-sim -p mel-platform --all-targets -- -D warnings
cargo build --target aarch64-apple-ios -p mel-platform --example 006_frame_timing
```

First builds are slow: `skia-safe` downloads prebuilt Skia binaries per
target (host, `-sim`, device) and compiles its bindings once each.

Run on the Simulator: `scripts/run-sim.sh <example>` — builds (debug) for
`aarch64-apple-ios-sim`, assembles `target/sim/<example>.app` (generated
`Info.plist`; `UILaunchScreen = {}` is what makes it full-screen without a
storyboard), boots an iPhone Simulator if none is booted, `simctl install` +
`launch --console-pty` (app stdout streams to the terminal — the only
"logging" channel until Phase 11). Apple Silicon only.

A thin [Makefile](../Makefile) wraps both flows: `make run`
(`EXAMPLE=NNN_slug` to pick one, defaults to the latest) and `make check`
(host/iOS checks plus the motion example tests).

## 7. Decisions in force (details in `DECISIONS/`)

- **ADR 001** — one Cargo workspace, one crate per layer; layering enforced
  by Cargo (no cycles, edges reviewed); toolchain pinned; CI disabled until
  it pays for itself.
- **ADR 002** — pure-Rust UIKit shell via `objc2`; no Xcode project, no
  winit; classic lifecycle (debt 002-1); one-command flow is a shell script
  (revisit xtask at 009/135); examples attach to `mel-platform`. Feature 009
  (real device) may reopen the Xcode-host question for codesigning.
- **ADR 003** — CPU raster, no Metal yet (GPU decision deliberately deferred
  to 008): app draws via `mel_painting::fill` into a shell-owned BGRA
  buffer; shell presents it as `CALayer.contents` through a `CGImage`.
  Painting/platform stay decoupled — they share only a documented, tested
  pixel-format contract; the sole inter-crate edge is a dev-dependency for
  examples.
- **ADR 004** — `viewDidLayoutSubviews` on a Mel-owned root view controller
  is the metrics source (launch, rotation, safe-area changes — since 005 it
  requests frames instead of rendering them); the buffer is
  physical-pixel-sized with `layer.contentsScale` set (no resampling);
  `Frame` names its units (`physical_*`/`logical_*`/`scale_factor`);
  `SafeAreaInsets` is a platform-owned carrier in logical points, absorbed
  by Phase 1's `EdgeInsets` (014) later.
- **ADR 005** — frames are pull-based through a paused `CADisplayLink`:
  `request_frame()` unpauses, the tick repauses before drawing (so requests
  coalesce and `draw` can re-request), and nothing presents synchronously
  from an event callback. `run()` takes the `App` trait — the platform↔app
  seam that 006 (timestamps), 007 (pointer events) and 008 (lifecycle)
  extend; `App::on_touch` is an information-free touch-began stub that 007
  replaces.
- **ADR 006** — immutable `FrameTime` in each `Frame`, with monotonic display
  timestamps and a per-frame interval; optional `on_frame_timing` reports
  CPU submission duration and deadline overrun. No unconditional engine
  logging. The demo samples logs and uses elapsed time for animation.
- `unsafe` only in the platform/painting layers, each use justified
  (CONTEXT.md §5).

## 8. What does NOT exist yet — don't assume it does

- No GPU/Metal rendering: still CPU raster (ADR 003), one buffer alloc +
  two copies per presented frame.
- No real touch input (007): `App::on_touch` says only "a touch began" —
  no position, no phase, no pointer IDs, no moved/ended/cancelled.
- No lifecycle/background hooks (008), no real-device or codesigning flow
  (009).
- No Mel `Canvas`/`Paint`/`Color` types — `fill()`/`fill_rect()` are the
  entire painting API until Phase 2; five of the seven crates are still
  empty.
- No logging/debug tooling beyond `println!` through the attached console.

## 9. Where the next features land

- **007 (touches), 008 (lifecycle + GPU decision —
  revisit ADR 003's CPU raster and 004/005's per-present copies):** all live
  in `mel-platform`'s `ios` module, translating UIKit concepts into Mel
  types (which will start populating `mel-foundation` from Phase 1). 007
  replaces `App::on_touch` with Mel-typed
  pointer events on the same trait; 008 pauses/resumes the link with the
  app lifecycle.

## 10. Working rules recap (full text: CONTEXT.md)

- One feature per pass: Study → Design (ADR) → Build → Demo → stop for
  human review. Never mark `[x]` before review passes.
- Never implement ahead of the roadmap; split or stub + file debt instead.
- **AI agents never commit** — leave everything in the working tree.
