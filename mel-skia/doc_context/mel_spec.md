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

_Last updated: 2026-09-26, after feature 012 approval and feature 013 implementation (pending review)._

---

## 1. Status at a glance

- **Done (reviewed):** 001 (workspace & toolchain), 002 (app shell & blank
  app), 003 (Skia surface on screen), 004 (scale factor, safe areas &
  rotation), 005 (redraw-on-demand loop), 006 (frame timing), 007 (touch input),
  008 (app lifecycle & Metal decision), 009 (real-device deployment),
  010 (Offset & Size), 011 (Rect), 012 (Radius & RRect).
- **Phase 0 gate:** regression sweep, debt review and retrospective completed.
- **In progress:** 013 — Color (built, pending review).
- **Next up:** 014 — Alignment & EdgeInsets, after 013 review.
- **Open debt:** 002-1 (classic lifecycle vs `UIScene`) and 008-1
  (CPU raster retained — see [DEBT.md](../DEBT.md)).

## 2. What runs today

```sh
scripts/run-sim.sh 008_app_lifecycle   # or: make run
```

launches a honey background with a timestamp-driven moving square. Going
Home pauses drawing before Inactive/Background callbacks; reopening emits
Inactive/Active, repaints and restarts the demo's motion. Lifecycle logs show
a frame counter that stays fixed while suspended. The demo deliberately
requests frames in every lifecycle callback to verify they cannot bypass
this gate. Memory warnings log a counter and toggle the background color.

`make run-device` builds/signs/installs the existing 006 motion demo on an
explicitly selected physical iPhone. Set `DEVICE_ID` and `PROVISIONING_PROFILE`
first; see §6 and the examples README. The app remains pure Rust/UIKit,
CPU-rasterized through Skia (ADRs 008/009).
Frames exist only on demand and while Active; requests coalesce and survive
suspension. Each activation also schedules one fresh frame for static apps.
Examples 002–007 remain runnable. Touch input is cancelled on interruption;
late terminal events are ignored. Rotation updates viewport metrics.

## 3. Repository map (real vs. placeholder)

```text
mel-skia/
├── doc_context/          CONTEXT.md (process) · ROADMAP.md (status) · this file
├── DECISIONS/            001–013 + Phase 0 retrospective
├── DEBT.md               002-1 and 008-1 retained
├── examples/             002–008 iOS demos · 010–013 host foundation demos + README
├── scripts/              run-sim.sh (002) · run-device.py + signing tests (009)
├── rust-toolchain.toml   stable · edition 2024 · both iOS targets pinned
├── .github/workflows/    CI exists but is disabled/commented (ADR 001 amendment)
└── crates/               one crate per layer (ADR 001)
```

| Crate | Layer (CONTEXT.md §5) | State |
| --- | --- | --- |
| `mel-foundation` | geometry, color, ids | **Offset & Size (010), Rect (011), Radius & RRect (012), Color (013)** — see below |
| `mel-painting` | Canvas/Paint over Skia — the only crate ever allowed `skia-safe` | **bootstraps `fill()` (003) + `fill_rect()` (004)** — see §5 |
| `mel-compositing` | layer tree, scenes | empty placeholder |
| `mel-scheduling` | frame pipeline, tickers | empty placeholder |
| `mel-rendering` | RenderObject, layout | empty placeholder |
| `mel-widgets` | Widget/Element trees | empty placeholder |
| `mel-platform` | iOS app shell | **implemented (002–008)** — see §4 |

Inter-crate dependency edges are added only when a feature first needs them.
Exactly one exists: `mel-platform` → `mel-painting` as a **dev-dependency**
(examples are app-level code above every layer — ADR 003); the libraries
themselves still have zero edges, and `cargo tree -i skia-safe` shows Skia
confined to `mel-painting`.

### `mel-foundation` geometry (010)

[`src/geometry.rs`](../crates/mel-foundation/src/geometry.rs) defines `Offset`
and `Size`, re-exported at the crate root. Both are `Copy` values with public
`f64` fields, const constructors, `ZERO`/`INFINITE`, exact `PartialEq` and no
dependencies or unsafe code. Public API tests live in
[`tests/geometry.rs`](../crates/mel-foundation/tests/geometry.rs).

```rust
use mel_foundation::{Offset, Size};
let origin = Offset::new(12.0, 20.0);
let size = Size::new(100.0, 80.0);
assert_eq!(size.center(origin), Offset::new(62.0, 60.0));
assert!(size.contains(size.center(origin) - origin));
let growth: Offset = Size::new(108.0, 84.0) - size;
assert_eq!(size + growth, Size::new(108.0, 84.0));
```

Offset has vector arithmetic, direction/distance, scale/translate. Size has
extent constructors, nine origin-relative anchors, half-open containment,
aspect ratio, flipping and side magnitudes. Both support scalar arithmetic,
Euclidean `%`, floating `div_trunc`, component predicates `all_lt/le/gt/ge`
and non-null `lerp` with extrapolation. No `PartialOrd`: componentwise strict
comparison does not fit Rust's ordering laws. `Offset & Size` now constructs a
Rect (011).

Negative extents, infinities and NaN are retained. `is_finite` requires both
components finite; `is_infinite` recognizes either sign. NaN ordered comparisons
fail, numeric results propagate NaN, and empty or NaN sizes contain no points.
`is_empty` means a nonpositive extent, not merely a non-finite one. Zero-height
aspect ratios have explicit Flutter-style rules; NaN stays NaN. Equal lerp
components are preserved even if infinite. Full API/numeric differences are
recorded in [ADR 010](../DECISIONS/010-offset-size.md).

Run `cargo run -p mel-foundation --example 010_offset_size` on the host.
Platform carriers remain platform-owned: importing foundation there would
invert the documented layers. At an app boundary, explicitly construct an
Offset from `PointerPosition`'s `x/y` if needed; no new dependency edge is added.

### `mel-foundation` rectangles (011)

[`src/rect.rs`](../crates/mel-foundation/src/rect.rs) adds `Rect`, re-exported
at the crate root. It is a dependency-free `Copy` value with public `f64`
`left/top/right/bottom` fields, `ZERO`, `Default` and exact `PartialEq`.
Const constructors: `from_ltrb`, `from_ltwh`, `from_center` and
`from_origin_size` (also spelled `Offset & Size`).

```rust
use mel_foundation::{Offset, Rect, Size};
let bounds = Offset::new(12.0, 20.0) & Size::new(100.0, 80.0);
let clip = Rect::from_center(Offset::new(100.0, 80.0), 40.0, 40.0);
assert_eq!(bounds.intersect(clip), Rect::from_ltrb(80.0, 60.0, 112.0, 100.0));
assert!(bounds.overlaps(clip));
assert!(!bounds.contains(bounds.bottom_right()));
```

Accessors provide width/height/size and top-left/center/bottom-right.
`shift`/`translate`, `inflate`/`deflate`, `intersect`, bounding `union`,
half-open `contains` and positive-area `overlaps` return values without
normalizing the edges. Disjoint intersections and excessive deflation retain
inverted results. Empty inputs still contribute their edges to union; `ZERO`
is not an identity. Unlike Flutter's degenerate overlap cases, empty/inverted
inputs never overlap.

NaN propagates through edge extrema and prevents containment/overlap.
`is_empty` compares opposing edges, so NaN alone does not mean empty and equal
infinite edges are empty. `has_nan`, `is_finite` and `is_infinite` classify the
stored edges; the latter recognizes either infinity sign. Arithmetic follows
IEEE-754, including undefined infinite centers. See [ADR 011](../DECISIONS/011-rect.md)
and the public contract tests in [`tests/rect.rs`](../crates/mel-foundation/tests/rect.rs).
Run `cargo run -p mel-foundation --example 011_rect` on the host.

### `mel-foundation` rounded rectangles (012)

[`src/rrect.rs`](../crates/mel-foundation/src/rrect.rs) adds `Radius` and
`RRect`, exported at the crate root. Both are dependency-free `Copy` values
with public fields, `ZERO`, `Default` and exact `PartialEq`. Radius has `x/y`
components with const `circular`/`elliptical` constructors. RRect composes
`bounds: Rect` with `top_left/top_right/bottom_right/bottom_left: Radius`.
Its const constructors accept either one uniform radius or four clockwise
corners starting at top-left.

```rust
use mel_foundation::{Offset, Radius, Rect, RRect};
let rounded = RRect::from_rect_radius(
    Rect::from_ltwh(10.0, 20.0, 100.0, 80.0),
    Radius::elliptical(100.0, 80.0),
);
assert_eq!(rounded.scale_radii().unwrap().top_left, Radius::elliptical(50.0, 40.0));
assert!(rounded.contains(Offset::new(60.0, 60.0)));
assert!(!rounded.contains(rounded.outer_rect().top_left()));
let inner: Rect = rounded.safe_inner_rect().unwrap();
assert!(inner.contains(Offset::new(60.0, 60.0)));
```

`outer_rect` returns the original bounds. `scale_radii` uses one factor at
most one to fit every adjacent pair to its side; it returns a copy, with no
cache or mutation. `contains` fits radii before testing half-open bounds and
every applicable corner ellipse (including overlapping diagonal regions).
Arc boundaries are included; outer right/bottom edges remain excluded. Either
zero component makes that corner square. `safe_inner_rect` insets each side
by its maximum fitted radius component times `1 - 1/sqrt(2)`.

Constructors retain all inputs. Rounded queries require finite bounds and
derived extents, nonnegative extents, and finite nonnegative radii. Fitting
and inner bounds return `Option`, with `None` outside this domain; containment
returns false. Empty but noninverted bounds can be fitted, yet contain no
points. Adjacent-radius sum overflow is handled; normal `f64` rounding and
underflow still apply. See [ADR 012](../DECISIONS/012-radius-rrect.md) for
intentional differences from Flutter and the limited API scope.

[`tests/rrect.rs`](../crates/mel-foundation/tests/rrect.rs) covers the public
contract and checks sampled containment against horizontal ellipse slices.
Run `cargo run -p mel-foundation --example 012_radius_rrect` on the host;
no painting or iOS UI is involved.

### `mel-foundation` colors (013)

[`src/color.rs`](../crates/mel-foundation/src/color.rs) adds `Color`, exported
at the crate root: a dependency-free `Copy` value with private `u32` ARGB
storage, `Default` (transparent black), exact `Eq` and `Hash`. RGB is
unpremultiplied sRGB; alpha is linear. Packed integer order is `0xAARRGGBB`,
independent of host byte order and the platform's raster format.

```rust
use mel_foundation::Color;
let honey = Color::from_argb32(0xffff_c107);
let faded = honey.with_opacity(0.5).unwrap();
assert_eq!(faded.to_argb32(), 0x80ff_c107);
assert_eq!(faded.opacity(), 128.0 / 255.0);
assert_eq!(Color::lerp(Color::RED, Color::BLUE, 0.5),
           Some(Color::from_argb(255, 128, 0, 128)));
```

Const APIs cover `from_argb32`, `from_argb(u8, u8, u8, u8)`, `to_argb32`,
`alpha/red/green/blue` getters and `with_alpha`. All bits are preserved,
including RGB at zero alpha. `from_rgbo`/`with_opacity` accept finite values
in `[0, 1]`, rounding `opacity * 255` to a byte (half ties upward); invalid
inputs return `None`. Opacity replacement preserves RGB and does not multiply
existing alpha. `opacity` returns quantized alpha divided by 255.

`Color::lerp(a, b, t)` returns `None` for non-finite `t`. Finite factors
interpolate/extrapolate each byte as `a + (b - a) * t`, clamp to `[0, 255]`
and round half upward. Extreme finite factors saturate overflowing channels;
equal channels stay equal. Interpolation is in encoded sRGB with straight
alpha, not linear-light or premultiplied space. Use `color.with_alpha(0)`
as an endpoint to fade while retaining RGB. `TRANSPARENT` means black.

The basic palette is `TRANSPARENT`, `BLACK`, `WHITE`, `RED`, `GREEN`, `BLUE`,
`YELLOW`, `CYAN`, `MAGENTA`; `GREEN` is full-intensity RGB, not a Material
swatch. Color spaces, higher precision and painting integration are deferred.
See [ADR 013](../DECISIONS/013-color.md) and public contract tests in
[`tests/color.rs`](../crates/mel-foundation/tests/color.rs), including an
integer-rational interpolation oracle for every byte pair. Run
`cargo run -p mel-foundation --example 013_color` on the host; no iOS UI needed.

## 4. `mel-platform` — the app shell (features 002–008)

Public API — this is everything layers above can see:

```rust
pub trait App {                                // the platform↔app seam (ADR 005)
    fn draw(&mut self, frame: &mut Frame);     // once per granted frame request
    fn on_frame_timing(&mut self, timing: FrameTiming) {} // after CPU submission
    fn on_pointer_event(&mut self, event: PointerEvent) {} // changed touch (007)
    fn on_lifecycle_changed(&mut self, state: AppLifecycleState) {} // transition (008)
    fn on_memory_warning(&mut self) {} // release disposable app resources (008)
}

// iOS-only (#[cfg(target_os = "ios")]):
mel_platform::run(app: impl App + 'static) -> !
mel_platform::request_frame()                  // schedule exactly one draw at next vsync;
                                               // idempotent until granted (coalesces);
                                               // deferred while inactive/background;
                                               // main-thread only

pub enum AppLifecycleState { Active, Inactive, Background } // drawing only in Active

pub struct PointerId(pub u64);                 // stable per contact, never reused
pub enum PointerPhase { Down, Move, Up, Cancel }
pub struct PointerPosition { pub x: f64, pub y: f64 } // logical viewport points
pub struct PointerEvent {
    pub id: PointerId,
    pub phase: PointerPhase,
    pub position: PointerPosition,
    pub timestamp: std::time::Duration,        // monotonic sample time
}

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
- [`src/pointer.rs`](../crates/mel-platform/src/pointer.rs) — native-free
  event carriers and a private, host-tested tracker. New Down allocates a
  monotonically increasing ID; Move preserves it; Up/Cancel releases it.
  Duplicate Down and orphan events are ignored. Positions are full-viewport
  logical points, including safe areas, with no clamping or scale conversion.
  The tracker retains each last sample so interruption can drain all remaining
  contacts into Cancel events at their last positions, with monotonic times.
  `PointerPosition` remains a platform-owned carrier; apps can now construct an
  Offset explicitly (ADR 010), without platform depending upward on foundation.
- [`src/lifecycle.rs`](../crates/mel-platform/src/lifecycle.rs) — native-free
  `AppLifecycleState` and private host-tested `FrameLoop`: pending requests
  are independent of eligibility to draw. Initial Inactive blocks ticks;
  activation requests a fresh frame, duplicate transitions do nothing, and
  requests made while suspended stay pending. Grants clear pending work before
  app callbacks, preserving re-request and coalescing semantics.
- [`src/ios.rs`](../crates/mel-platform/src/ios.rs) — the only file that
  speaks Objective-C. `define_class!` registers two Rust-implemented classes:
  `MelAppDelegate` (creates the `UIWindow`, installs the root VC, keeps the
  window alive in its ivars) and `MelViewController`, which owns the frame
  loop (ADR 005, mirroring Flutter's `VsyncWaiterIOS`): a `CADisplayLink`
  created **paused** in `viewDidLoad` (main run loop, common modes) is the
  only path to the screen. Events _request_ frames — `viewDidLayoutSubviews`
  (first display, rotation, safe-area changes) memoizes the metrics snapshot
  and requests on change. The root view enables multiple touches; all four
  touch callbacks translate each changed UITouch into `App::on_pointer_event`.
  Native object addresses are private identity keys only. `locationInView`
  provides logical positions and UITouch's timestamp provides monotonic
  sample time. The app may call `request_frame()`. The tick (`onDisplayLink:`)
  repauses and checks frame eligibility (including stale queued ticks)
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
  even when their CPU work fits the refresh interval. Idle or inactive = link
  paused. Application delegate hooks translate activation/resignation/background/
  foreground into Mel states; suspension pauses before cancelling contacts and
  notifying App. No tracker/frame-loop borrow spans app callbacks. Memory
  warnings call `App::on_memory_warning` independently of lifecycle.
- **Lifecycle model:** classic `UIApplicationDelegate`/`UIWindow`, _not_
  scene-based (`UIScene`) — retained by ADRs 008/009 after a successful device
  launch; debt 002-1 was retained at the Phase 0 gate for future SDK compatibility.
  The `#[expect(deprecated)]` on `UIScreen::mainScreen` / `UIWindow::initWithFrame`
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
cargo run -p mel-foundation --example 010_offset_size
cargo run -p mel-foundation --example 011_rect
cargo run -p mel-foundation --example 012_radius_rrect
cargo run -p mel-foundation --example 013_color
cargo test -p mel-platform --example 006_frame_timing
python3 -B -m unittest discover -s scripts -p 'test_*.py'
bash -n scripts/run-sim.sh
```

iOS (the shell code only compiles for iOS targets):

```sh
cargo clippy --target aarch64-apple-ios-sim -p mel-platform --all-targets -- -D warnings
cargo build --target aarch64-apple-ios -p mel-platform --example 008_app_lifecycle
```

First builds are slow: `skia-safe` downloads prebuilt Skia binaries per
target (host, `-sim`, device) and compiles its bindings once each.

Run on the Simulator: `scripts/run-sim.sh <example>` — builds for
`aarch64-apple-ios-sim`, assembles `target/sim/<example>.app` (generated
`Info.plist`; `UILaunchScreen = {}` is what makes it full-screen without a
storyboard), boots an iPhone Simulator if none is booted, `simctl install` +
`launch --console-pty` (app stdout streams to the terminal — the only
"logging" channel until Phase 11). Apple Silicon only.

Run on an iPhone (iOS 17+, paired, Developer Mode enabled, unlocked):

```sh
export DEVICE_ID='<identifier from xcrun devicectl list devices>'
export PROVISIONING_PROFILE='/absolute/path/to/development.mobileprovision'
make run-device                         # defaults to 006_frame_timing
make run-device EXAMPLE=008_app_lifecycle
```

[`scripts/run-device.py`](../scripts/run-device.py) uses Python's standard
library plus Xcode tools. It validates profile expiry, device UDID, App ID,
team and certificate/private-key availability before building; signs the
bundle under `target/device/` with concrete minimal entitlements; verifies,
installs and launches with the console attached. `SIGNING_IDENTITY` (SHA-1)
resolves multiple matching identities; `BUNDLE_ID` supports explicit profiles.
Signing assets are provided by the developer/Xcode, never checked in. See the
[setup and troubleshooting guide](../examples/README.md#feature-009-run-on-a-physical-iphone).
The runner masks device/signing identifiers in command echoes and errors.
Native diagnostics may contain metadata; the generated `.app` embeds a profile
with provisioned device IDs. Both remain private development artifacts, and
signing-file types/local environment files are ignored by Git.
Ctrl-C stops the device app. The Simulator console keeps its existing behavior.

Both runners default to debug; `BUILD_MODE=release` selects release. Compare
the same mode and scene on both targets. Example 006 now logs viewport/scale
at animation start. ADR 009 records the hardware/Simulator sampling method
and results, which measure CPU submission rather than display completion.

A thin [Makefile](../Makefile) wraps `make run` (Simulator, defaults to 008),
`make run-device` (iPhone, defaults to 006), and `make check` (host/Simulator
Rust checks, motion tests, signing tests and shell syntax). Both run targets
accept `EXAMPLE=NNN_slug`.

## 7. Decisions in force (details in `DECISIONS/`)

- **ADR 001** — one Cargo workspace, one crate per layer; layering enforced
  by Cargo (no cycles, edges reviewed); toolchain pinned; CI disabled until
  it pays for itself.
- **ADR 002** — pure-Rust UIKit shell via `objc2`; no Xcode project, no
  winit; classic lifecycle (debt 002-1); Simulator flow is a shell script;
  examples attach to `mel-platform`. ADR 009 adds a Python device runner
  without an Xcode host. Revisit xtask at 135 if tooling growth warrants it.
- **ADR 003** — CPU raster, no Metal yet (re-evaluated and retained by
  ADR 008): app draws via `mel_painting::fill` into a shell-owned BGRA
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
  `request_frame()` unpauses while Active (008), the tick repauses before drawing (so requests
  coalesce and `draw` can re-request), and nothing presents synchronously
  from an event callback. `run()` takes the `App` trait — the platform↔app
  seam that 006 (timestamps), 007 (pointer events) and 008 (lifecycle)
  extend; the original `App::on_touch` stub was replaced by 007.
- **ADR 006** — immutable `FrameTime` in each `Frame`, with monotonic display
  timestamps and a per-frame interval; optional `on_frame_timing` reports
  CPU submission duration and deadline overrun. No unconditional engine
  logging. The demo samples logs and uses elapsed time for animation.
- **ADR 007** — `App::on_pointer_event` delivers Mel-owned phase, ID,
  logical position, and timestamp per changed contact. A main-thread private
  tracker translates native identities; no UIKit type crosses the API.
  Up/Cancel releases a contact independently; later Down gets a fresh ID.
  No hit testing, gestures, pressure, or predicted/coalesced samples yet.
- **ADR 008** — Active/Inactive/Background plus a separate memory-warning
  callback. Active-only rendering, suspension-safe pending requests, a fresh
  frame on activation and cancellation of interrupted contacts. Retain CPU
  raster based on current minimal-scene evidence, supported by 009 hardware
  timing; painting API design (020) revisits Metal. Classic lifecycle remains for this
  single-window shell; 002-1 and 008-1 capture the limits.
- **ADR 009** — Keep the Rust executable and use `security`/`codesign` plus
  `devicectl` for local iPhone deployment. Explicit device/profile, validated
  signing inputs, minimal entitlements, no Xcode host project. Reuse example
  006; hardware timing supports retaining CPU raster for this minimal scene.
  Classic lifecycle launched on the tested iOS device; the Phase 0 gate
  retained both debts, with their next triggers recorded in DEBT.md.
- **ADR 010** — distinct dependency-free Offset/Size value types with typed
  arithmetic, named component comparisons and an explicit IEEE-754 policy.
  No platform-carrier migration; pure-logic host demo. Rect construction
  deferred at 010 is implemented by 011.
- **ADR 011** — edge-based Rect with unnormalized empty/inverted bounds,
  half-open containment, bounding union and positive-area overlap. Overlap
  rejects Flutter's empty/inverted/NaN false positives; extrema retain NaN.
  Named origin/size construction and `Offset & Size`; pure-logic host demo.
- **ADR 012** — Radius plus RRect composition with four named corners;
  proportional radius fitting, all-corner elliptical containment and fitted
  safe inner bounds. Invalid rounded geometry returns `None`/false without
  altering stored inputs. Pure-logic host demo; painting remains in Phase 2.
- **ADR 013** — packed ARGB Color with straight sRGB bytes, exact equality
  and hashing; checked opacity replacement and channel interpolation with
  half-up quantization. Small channel-extrema palette; color spaces deferred.
  No painting/platform migration; pure-logic host demo.
- `unsafe` only in the platform/painting layers, each use justified
  (CONTEXT.md §5).

## 8. What does NOT exist yet — don't assume it does

- No GPU/Metal rendering: still CPU raster (ADR 003), one buffer alloc +
  two copies per presented frame.
- No hit testing, pointer target dispatch/capture (Phase 5), or gestures
  (Phase 8); touch events currently go directly to the App.
- No automatic provisioning/profile renewal or App Store export; no UIScene
  support yet. The device flow consumes an existing development profile.
- No Mel `Canvas`/`Paint` types — `fill()`/`fill_rect()` are the entire
  painting API until Phase 2 and do not consume foundation's Color yet;
  four of the seven crates are still empty. Color supports only 8-bit sRGB.
- No logging/debug tooling beyond `println!` through the attached console.

## 9. Where the next features land

- **After 013 review:** 014 adds Alignment & EdgeInsets to `mel-foundation`,
  including Size/Rect resolution and interpolation, with an ADR on RTL timing.

## 10. Working rules recap (full text: CONTEXT.md)

- One feature per pass: Study → Design (ADR) → Build → Demo → stop for
  human review. Never mark `[x]` before review passes.
- Never implement ahead of the roadmap; split or stub + file debt instead.
- **AI agents never commit** — leave everything in the working tree.
