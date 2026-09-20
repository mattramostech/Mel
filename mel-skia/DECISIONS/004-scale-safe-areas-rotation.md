# 004 — Scale factor, safe areas & rotation

## Flutter reference

Flutter routes all three concerns through one struct: `ViewportMetrics`.
The iOS embedder's `FlutterViewController` overrides `viewDidLayoutSubviews`
and calls `updateViewportMetrics`, which packs:

- **physical size** — `view.bounds.size` multiplied by the screen scale,
- **`devicePixelRatio`** — `[UIScreen mainScreen].scale` (2× or 3× on modern
  iPhones): how many physical pixels one logical point spans,
- **padding** — `view.safeAreaInsets` (notch, home indicator, status bar),
  also converted to physical pixels,

and sends the packet to the engine, which resizes its raster surface to the
physical size. Rotation needs no special code path: it is just another
layout pass — UIKit changes the view's bounds, `viewDidLayoutSubviews`
fires, new metrics flow, the surface is recreated at the new size. The
framework's `MediaQuery` divides the physical padding back by
`devicePixelRatio` so app code reasons in logical points.

Essential: one metrics snapshot per layout pass; the surface lives in
physical pixels while apps think in logical points, bridged by a single
scalar; `viewDidLayoutSubviews` as the sole trigger (it covers launch,
rotation, and inset changes uniformly). Incidental: Flutter reporting
padding in *physical* pixels — an artifact of the engine↔framework wire
protocol; the framework immediately converts back to logical, which is the
unit apps actually use.

## Decision

**`viewDidLayoutSubviews` on a Mel-owned `UIViewController` subclass becomes
the single render trigger; `Frame` carries the viewport metrics; the pixel
buffer moves to physical pixels.**

- **`MelViewController`** (second `define_class!` in `ios.rs`, no ivars) is
  installed as the window's root view controller. Its
  `viewDidLayoutSubviews` override reads the view's bounds, the scale from
  `window.screen.scale`, and `safeAreaInsets`, and re-renders.
  `didFinishLaunching` no longer draws: the first layout pass and every
  rotation/inset change go through the exact same path — mirroring
  Flutter's design.
- **`Frame` grows the logical-vs-physical distinction.** The buffer is now
  sized in physical pixels (`round(bounds × scale)`); the API says so
  explicitly: `physical_width()`/`physical_height()` (buffer dimensions),
  `scale_factor()` (Flutter's `devicePixelRatio`), `logical_width()`/
  `logical_height()` (derived: physical ÷ scale), and `safe_area()`. The
  003-era `width()`/`height()` names are gone — with two coordinate spaces
  in play, every dimension names its unit.
- **`SafeAreaInsets`** is a new plain public type in `mel-platform`
  (`left/top/right/bottom: f64`), in **logical points** — the unit apps
  reason in; we diverge from Flutter's physical-pixel wire format because
  Mel has no separate framework layer to convert back. It is deliberately a
  dumb data carrier: Phase 1's `EdgeInsets` (feature 014) is the real
  geometry type and will absorb or replace it at the platform boundary —
  scheduled, not debt.
- **`run` takes `FnMut`** instead of `FnOnce`: the draw callback now runs
  once per *metrics change* (launch, rotation, safe-area change). It stays
  parked in the main-thread `thread_local`; a second slot memoizes the last
  metrics so redundant layout passes (UIKit fires `viewDidLayoutSubviews`
  liberally) don't re-raster identical frames. The redraw-on-*demand* loop
  — repainting without a size change — remains feature 005.
- **Presentation maps pixels 1:1**: the `CGImage` is physical-pixel-sized
  and the layer gets `contentsScale = scale`, so Core Animation composites
  without resampling (with the default `resize` gravity, an exact-size
  image is presented identity). A zero-sized layout pass (possible
  mid-transition) is skipped instead of wrapping an empty Skia surface.
- **`mel-painting` gains `fill_rect`** — a bootstrap sibling of 003's
  `fill`, same buffer contract, filling only an `[x, y, w, h]` sub-rectangle
  (anti-aliasing off: the demo *measures* the square's edges in a
  screenshot). Needed because 004's "Done when" requires drawing a square
  and the painting layer is the only place allowed to rasterize. Phase 2
  (021 `draw_rect`) replaces both bootstraps.
- **`scripts/run-sim.sh`** declares `UISupportedInterfaceOrientations`
  (portrait + both landscapes) in the generated `Info.plist`, making the
  rotation contract explicit rather than relying on UIKit's
  "all-but-upside-down" default.

## Alternatives considered

- **`viewWillTransitionToSize:withTransitionCoordinator:`** — announces the
  *target* size before the rotation animation, which matters for animating
  engines. Rejected as the trigger: it fires only for rotation (not for
  the initial layout or inset-only changes), and safe-area insets for the
  new orientation aren't available yet when it fires. `viewDidLayoutSubviews`
  covers all three cases with one code path — and is what Flutter uses.
- **Custom `UIView` subclass overriding `layoutSubviews`** — the same
  signal one level lower. A view controller override needs no new view
  class (the plain root view keeps working) and is where Flutter hooks;
  revisit if a dedicated view class appears for other reasons (008's Metal
  layer may want `layerClass`).
- **Insets in physical pixels (Flutter's wire format)** — rejected:
  Flutter itself converts back to logical before any app code sees them;
  without a framework layer in between, exposing physical insets would just
  push the division into every caller.
- **Keep `width()`/`height()` (logical) and add `pixel_width()` etc.** —
  rejected: after 004 every consumer of the buffer needs *physical*
  dimensions (that's what `fill` wraps), so the short names would be the
  wrong default. Explicit `physical_*`/`logical_*` makes call sites
  self-documenting; examples 002/003 adapt (they are regression checks,
  not frozen call sites — 003 set the precedent).
- **`view.contentScaleFactor` as the scale source** — what Apple's docs
  suggest ("defaults to the scale factor associated with the screen").
  Rejected *empirically*: it mirrors the layer's `contentsScale`, which
  UIKit only raises to the screen scale for views it draws (`drawRect:`);
  for a plain layer-backed view it stays 1.0 — the demo printed `@ 1×` on a
  3× Simulator. `window.screen.scale` (non-deprecated, per-window correct)
  is what we present with; Flutter reads the same value via its view's
  screen. `traitCollection.displayScale` would also work; the window route
  needs no extra feature flags and the window is already in hand as the
  attachment check.
- **KVO on the layer/view bounds** — observer plumbing and ordering
  hazards for a signal UIKit already delivers as a method override.

## Consequences

- The metrics snapshot (`Frame`) is the seed of Flutter's
  `ViewportMetrics`; 005/006 will extend the same callback with timing
  rather than invent a new channel, and Phase 1's geometry types will
  replace the raw `f64`s.
- Every metrics change reallocates the buffer and copies it twice into the
  `CGImage` (unchanged from 003). At rotation frequency this is irrelevant;
  the frame loop (005) and GPU decision (008) own the presentation-cost
  question.
- During the rotation *animation*, Core Animation stretches the old
  contents until the new frame is set — the end state is always crisp and
  correctly sized. Smooth-through-rotation rendering needs the frame loop
  (005) and likely GPU presentation (008); out of scope here.
- The draw callback is borrowed from the `thread_local` while it runs; a
  future platform API that re-enters the shell from inside `draw` would
  re-borrow and panic. Nothing today can do that; 005's loop design must
  keep it in mind.
- `layer.contentsScale` plus exact-size images means nothing resamples —
  the "100 logical px is 300 physical px at 3×" contract is testable from
  a Simulator screenshot (screenshots are physical pixels).

## Review outcome

Reviewed and approved 2026-08-06. Correctness ✅ · Architecture ✅ ·
API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No debt items filed — the known limitations (stretched contents during the
rotation *animation*, per-present buffer copies, manual logical→physical
conversion in app code until canvas transforms exist) are scheduled roadmap
features (005, 008, 022), recorded in this ADR's Decision/Consequences.
Verified: the 100-logical-pt square measured exactly 300×300 physical px on
the 3× Simulator (screenshot pixel measurement), insets reported 0/59/0/34,
and the `@ 1×` `contentScaleFactor` pitfall is documented under
Alternatives.
