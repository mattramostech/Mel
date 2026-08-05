# Mel (Skia) — Feature-by-Feature Roadmap

The single source of truth for project status. Work strictly top-to-bottom
within a phase; phases are gated (see [CONTEXT.md](CONTEXT.md) §3).

Legend: `[ ]` not started · `[~]` in progress · `[x]` done (reviewed).
Every feature gets an ADR in `DECISIONS/NNN-slug.md` and, when visible,
an example in `examples/NNN_slug.rs`.

---

## Phase 0 — Platform Shell (iOS app, surface, loop)

Goal: an iOS app we fully control, with a Skia surface we can draw into,
driven by a display-link frame loop. This is Mel's "embedder + engine
bootstrap". Target: iOS Simulator first, real device as its own feature.

- [x] **001 — Workspace, crate layout & iOS toolchain** — Cargo workspace, decide
  crate-per-layer vs. single crate with modules; install `aarch64-apple-ios` +
  `aarch64-apple-ios-sim` targets; CI basics (`fmt`, `clippy`, `test` on host mac).
  *Done when:* an empty skeleton cross-compiles for both iOS targets and the ADR records the layout decision.
- [~] **002 — App shell decision & blank app** — THE embedding ADR: pure-Rust
  shell (winit on iOS / raw `objc2`+UIKit bindings) vs. thin Xcode host app
  linking Mel as a `staticlib` via FFI. Weigh: build/run loop speed, lifecycle
  access, long-term multi-platform story.
  *Done when:* a blank full-screen app launches on the Simulator via a documented one-command flow; ADR records the decision.
- [ ] **003 — Skia surface on screen** — get a `skia-safe` surface presenting
  into the app's layer (CPU raster into a `CALayer`, or Metal-backed
  `CAMetalLayer` if 002's shell makes that the path of least resistance —
  record which and why).
  *Done when:* the Simulator shows a solid color drawn via a Skia canvas.
- [ ] **004 — Scale factor, safe areas & rotation** — contentScaleFactor
  (logical vs. physical pixels — Flutter's devicePixelRatio), expose safe-area
  insets, survive rotation/size changes with surface recreation.
  *Done when:* a 100px square measures 100 logical px on a 3× device; rotating never crashes/stretches; insets are queryable.
- [ ] **005 — Redraw-on-demand loop** — CADisplayLink-driven, but paint only
  when a frame was requested (pause the link when idle); clean separation
  `Platform events → App callback → Present`.
  *Done when:* CPU sits near 0% when idle; a tap triggers exactly one repaint.
- [ ] **006 — Frame timing** — frame callback with the display-link timestamp
  (Flutter's `onBeginFrame`), measure and log frame duration vs. target.
  *Done when:* example animates a moving rectangle smoothly using timestamps only.
- [ ] **007 — Touch input plumbing** — receive UITouch began/moved/ended/
  cancelled; convert to Mel's own pointer event types with stable pointer IDs
  for multi-touch (UIKit types must not leak upward).
  *Done when:* example logs Mel-typed pointer events for two simultaneous fingers; no UIKit/winit types in the public API.
- [ ] **008 — App lifecycle & Metal decision point** — foreground/background/
  memory-warning hooks routed into Mel; if 003 started CPU-raster, evaluate
  switching to a Metal-backed Skia surface now.
  *Done when:* backgrounding pauses the display link and drawing resumes cleanly; ADR records the GPU decision.
- [ ] **009 — Run on a real device** — codesigning, deploy flow, and a quick
  performance sanity check (frame times on hardware vs. Simulator).
  *Done when:* example 006 runs on a physical iPhone via a documented flow.

## Phase 1 — Foundation (geometry & basic types)

Goal: Flutter's `dart:ui`/foundation value types, in pure Rust, fully unit-tested.
No Skia dependency in this layer.

- [ ] **010 — Offset & Size** — with the operators Flutter has (`+`, `-`, `*`,
  `contains`, `center`, aspect ratio, `Size::infinite`, etc.).
  *Done when:* unit tests cover the operator table and edge cases (zero, infinite, NaN policy decided).
- [ ] **011 — Rect** — `from_ltwh`/`from_ltrb`/`from_center`, intersection, union,
  inflate/deflate, contains, overlaps.
  *Done when:* tests pass, including empty/inverted rect semantics (ADR: match Flutter or fix it?).
- [ ] **012 — Radius & RRect** — circular/elliptical radii, corner-specific,
  `contains`, inner/outer rect.
  *Done when:* tests pass; example draws nothing yet (painting comes in Phase 2).
- [ ] **013 — Color** — ARGB storage, opacity helpers, `lerp`, predefined palette,
  color spaces deferred (ADR).
  *Done when:* tests cover lerp/opacity rounding behavior.
- [ ] **014 — Alignment & EdgeInsets** — resolve against Size/Rect, `lerp`,
  directional variants deferred or included (ADR on RTL strategy timing).
  *Done when:* `Alignment::center.inscribe(size, rect)` behaves like Flutter's.
- [ ] **015 — Matrix4 / Transform** — decide: own 4×4 type vs. `glam`; translate/
  scale/rotate helpers, point mapping, invert.
  *Done when:* tests verify point mapping matches expected values; ADR records the dependency decision.
- [ ] **016 — Duration/time conventions & IDs** — frame timestamps, monotonic
  time source abstraction (testable clock), unique ID generation for future trees.
  *Done when:* a fake clock can drive deterministic tests.

## Phase 2 — Painting (Canvas API over Skia)

Goal: Mel's own painting API — the only layer that touches `skia-safe`.
Each feature = one drawing capability, demoed visually.

- [ ] **020 — Paint** — color, style (fill/stroke), stroke width/cap/join,
  anti-alias, blend mode enum (subset).
  *Done when:* ADR maps Mel Paint fields → Skia Paint; example draws filled + stroked rects.
- [ ] **021 — Canvas core & rects** — `draw_rect`, `draw_rrect`, `draw_circle`,
  `draw_oval`, `draw_line`, `clear`.
  *Done when:* example renders a composition of all primitives.
- [ ] **022 — Canvas transforms** — save/restore stack, translate/scale/rotate/
  `transform(Matrix4)`.
  *Done when:* example draws a rotated nested scene; save/restore depth is asserted in tests.
- [ ] **023 — Clipping** — `clip_rect`, `clip_rrect`, `clip_path` (path arrives
  in 024 — stub or reorder consciously).
  *Done when:* example shows content clipped by a rounded rect.
- [ ] **024 — Path** — move/line/quad/cubic/arc, close, fill types, `contains`,
  bounds; builder-style API design (ADR: mutable builder vs. fluent).
  *Done when:* example draws a heart/star; `contains` unit-tested.
- [ ] **025 — Gradients & shaders** — linear/radial/sweep gradient on Paint.
  *Done when:* example shows all three gradient types.
- [ ] **026 — Picture recording** — record a `Picture` (display list) and replay
  it — the primitive that layers/raster caching will build on.
  *Done when:* example records once, replays at 3 offsets; ADR covers Skia `PictureRecorder` wrapping.
- [ ] **027 — Shadows & blur** — `draw_shadow` (elevation-style), blur mask/image
  filter basics.
  *Done when:* example shows an elevated card look.
- [ ] **028 — Opacity & save_layer** — `save_layer` with alpha, why it's expensive
  (document it — this knowledge shapes later compositing decisions).
  *Done when:* example shows group opacity vs. per-primitive opacity difference.
- [ ] **029 — Images** — load PNG/JPEG to a Mel `Image`, `draw_image`,
  `draw_image_rect`, filter quality.
  *Done when:* example draws a scaled photo with both nearest & linear sampling.

## Phase 3 — Compositing & Frame Scheduling

Goal: the layer tree and the frame pipeline — Flutter's `SceneBuilder` +
`SchedulerBinding` territory.

- [ ] **030 — Layer tree core** — `Layer` model (ADR: trait objects vs. enum),
  `ContainerLayer`, `PictureLayer`; paint the tree to a canvas.
  *Done when:* example composes two picture layers under a container.
- [ ] **031 — TransformLayer & OffsetLayer** — positioned subtrees.
  *Done when:* example animates a subtree offset without re-recording its picture.
- [ ] **032 — ClipRect/ClipRRect/ClipPath layers** — subtree clipping.
  *Done when:* example clips an animated subtree.
- [ ] **033 — OpacityLayer** — group opacity in the tree (uses 028).
  *Done when:* example fades a subtree in/out.
- [ ] **034 — Scene & compositor** — `SceneBuilder`-style API producing an
  immutable `Scene` handed to the rasterizer; separates "describe" from "draw".
  *Done when:* the platform shell renders only via Scene; direct canvas access removed from examples going forward.
- [ ] **035 — Frame pipeline phases** — explicit `build → layout → paint →
  composite` pipeline object with dirty flags (empty phases for now; the skeleton
  that Phases 4–6 fill in).
  *Done when:* logging shows phases running once per requested frame, skipped when clean.
- [ ] **036 — Ticker & frame callbacks** — register transient (animation) and
  persistent (rendering) frame callbacks; tickers driven by frame timestamps.
  *Done when:* two independent tickers animate two layers; stopping one stops its repaints.

## Phase 4 — Rendering: Layout Protocol

Goal: `RenderObject`/`RenderBox` — Flutter's layout heart. The hardest Rust
design work in the project (tree ownership & mutation). Take it slow.

- [ ] **040 — RenderObject tree design** — THE ownership ADR: arena + ids vs.
  `Rc<RefCell>` vs. slotmap; parent/child links; tree mutation API.
  *Done when:* a bare tree can be built, walked, and mutated in tests; ADR is thorough (this one gets extra review time).
- [ ] **041 — BoxConstraints** — min/max width/height, `constrain`, `loosen`,
  `tighten`, `enforce`, normalized/degenerate handling.
  *Done when:* unit tests mirror Flutter's documented constraint behaviors.
- [ ] **042 — Layout protocol** — `layout(constraints)` walk, `size` setting
  rules ("constraints go down, sizes go up"), parent-uses-size, relayout boundaries.
  *Done when:* a two-node tree lays out; violating the protocol (reading size before layout) fails loudly.
- [ ] **043 — Paint protocol & offsets** — paint walk producing layer tree /
  canvas calls with child offsets (`ParentData` with offset).
  *Done when:* pipeline from 035 runs layout+paint and shows two boxes positioned by a parent.
- [ ] **044 — markNeedsLayout/markNeedsPaint** — dirty propagation up to
  boundaries, re-layout/re-paint only dirty subtrees.
  *Done when:* test proves an unrelated subtree does not relayout; logs show minimal work.
- [ ] **045 — RenderColoredBox & RenderConstrainedBox** — first two real render
  objects (Flutter's `ColoredBox` / `SizedBox` semantics).
  *Done when:* example shows a fixed-size colored box centered by hand-set offset.
- [ ] **046 — RenderPadding** — insets, child positioning, size accumulation.
  *Done when:* example + tests with asymmetric insets, zero child, infinite constraints.
- [ ] **047 — RenderAlign / RenderCenter** — alignment within loose constraints,
  widthFactor/heightFactor.
  *Done when:* example shows all 9 alignments in a grid of parents.
- [ ] **048 — RenderDecoratedBox** — BoxDecoration (color, border, border radius,
  box shadow, gradient) painted before/after child.
  *Done when:* example reproduces a Material-like card via decoration only.
- [ ] **049 — RenderFlex (Row/Column) part 1: unflexed** — main/cross axis,
  mainAxisAlignment, crossAxisAlignment, fixed children only.
  *Done when:* example shows rows/columns with the 6 main-axis alignments; layout unit tests.
- [ ] **050 — RenderFlex part 2: Flexible/Expanded** — flex factors, fit
  (tight/loose), overflow detection & debug painting of overflow.
  *Done when:* tests cover flex distribution math; example shows expanded children and a deliberate overflow warning.
- [ ] **051 — RenderStack & Positioned** — non-positioned sizing, positioned
  children (l/t/r/b/width/height combinations), stack fit.
  *Done when:* example overlays a badge on a box; tests cover positioning combos.
- [ ] **052 — RenderOpacity / RenderTransform / RenderClip\*** — render objects
  that push compositing layers (bridges Phase 3 & 4); "needs compositing" tracking.
  *Done when:* example rotates + fades a subtree; layer tree dump shows expected layers.
- [ ] **053 — Intrinsic dimensions** — min/max intrinsic width/height protocol
  (needed later by IntrinsicWidth/Height, tables, flex baselines).
  *Done when:* intrinsics implemented for boxes built so far, with tests; ADR notes the cost model.
- [ ] **054 — Baselines** — text baseline plumbing in the box protocol (dummy
  values until Phase 7 text lands).
  *Done when:* protocol exists and flex `baseline` cross-alignment compiles against it.
- [ ] **055 — Debug tree dumps** — `dump_render_tree()` with sizes, constraints,
  offsets, dirty flags (Flutter's `debugDumpRenderTree` — a DevEx cornerstone).
  *Done when:* every example can print a readable tree via a debug trigger (e.g. three-finger tap).

## Phase 5 — Hit Testing & Pointer Events

Goal: from OS pointer event to the right render object.

- [ ] **060 — HitTest protocol** — `hit_test(position)` walk on render boxes,
  hit test entries/result, position transform through offsets & transforms.
  *Done when:* tests hit-test through padding/align/transform correctly.
- [ ] **061 — Pointer event dispatch** — route touch events (007) through hit
  test to targets; down/move/up/cancel lifecycle, pointer capture (down target
  receives subsequent moves even after the finger leaves its bounds).
  *Done when:* example highlights the box under the finger and drags with capture.
- [ ] **062 — RenderPointerListener** — a render object exposing pointer
  callbacks (Flutter's `Listener`).
  *Done when:* example: tap a box to change its color, no widget layer needed.
- [ ] **063 — Multi-touch dispatch** — independent hit test + capture per
  pointer ID; touch cancel on system interruption routed correctly.
  *Done when:* example drags two boxes with two fingers simultaneously.

## Phase 6 — Widgets: Element Tree & State

Goal: the declarative layer — `Widget`/`Element`/`State`, rebuilds, reconciliation.
Second major ownership ADR of the project.

- [ ] **070 — Widget & Element core design** — the ADR: widget as cheap immutable
  description (ADR: `Rc<dyn Widget>`? enum? generics?), element tree owning state,
  mount/unmount lifecycle.
  *Done when:* a hand-built widget tree inflates into an element tree in tests.
- [ ] **071 — RenderObjectWidget bridge** — leaf/single-child render object
  widgets: widgets create & update render objects (`ColoredBox`, `SizedBox`,
  `Padding`, `Align` as widgets).
  *Done when:* example builds Phase-4 scenes declaratively; render tree matches the hand-built one.
- [ ] **072 — Multi-child widgets** — `Row`, `Column`, `Stack` as widgets;
  children list handling.
  *Done when:* example: previous flex/stack demos rewritten declaratively.
- [ ] **073 — StatelessWidget & build()** — composition: user widgets that expand
  into subtrees; `BuildContext` design (what is it in Rust? — ADR).
  *Done when:* example defines a custom `Card`-like stateless widget used 3×.
- [ ] **074 — Rebuild & diff (same widget type)** — rebuild an element with a new
  widget config, update render objects in place (no re-mount).
  *Done when:* test proves render objects are reused when only properties change.
- [ ] **075 — Reconciliation (child lists)** — `update_children`: insert/remove/
  reorder, `can_update` rules (type + key).
  *Done when:* table-driven tests for list diffs (append, prepend, remove middle, swap).
- [ ] **076 — Keys** — local keys in reconciliation; global-key question deferred
  or decided (ADR).
  *Done when:* swap-two-stateful-children test preserves state with keys, resets without.
- [ ] **077 — StatefulWidget & set_state** — `State` objects owned by elements,
  lifecycle (`init_state`, `did_update_widget`, `dispose`), `set_state` marking
  dirty + build scope (ADR: how does `set_state` work without `self` aliasing pain?).
  *Done when:* example: counter that increments on click (uses 062). The "hello world" milestone. 🎉
- [ ] **078 — BuildOwner & build scheduling** — dirty element list, build phase
  integrated into pipeline (035), build order (depth), reentrancy guards.
  *Done when:* test: parent+child both dirty → child builds once, not twice.
- [ ] **079 — InheritedWidget** — O(1) ancestor data lookup + dependency
  registration + notification on change.
  *Done when:* example: theme color provided at root, consumed twice, both update on change.
- [ ] **080 — Element tree dump & widget inspector (text)** — `dump_widget_tree()`
  with widget names, keys, dirty state.
  *Done when:* every example can dump widgets + render tree together.

## Phase 7 — Text

Goal: real text rendering. Big decision phase: Skia's paragraph module vs.
cosmic-text vs. hand-rolled shaping.

- [ ] **090 — Text stack decision** — evaluate skia-safe `textlayout` (paragraph),
  cosmic-text, rustybuzz+own layout. Criteria: shaping quality, API fit, future
  Impeller portability.
  *Done when:* ADR with a comparison table and a spike drawing "Hello, Mel!" in each finalist.
- [ ] **091 — Font loading & FontFamily** — bundle a default font, load from
  bytes/system, font collection abstraction.
  *Done when:* example renders text in two families.
- [ ] **092 — TextStyle** — size, weight, style, color, letter/word spacing,
  height; mapping to the chosen backend.
  *Done when:* example shows a style specimen sheet.
- [ ] **093 — Paragraph layout** — measure & layout constrained text: line
  breaking, max lines, ellipsis, text align.
  *Done when:* tests assert line counts/sizes for known inputs; example shows wrapping at a draggable width.
- [ ] **094 — RenderParagraph & Text widget** — text in the layout protocol
  (intrinsics + baselines from 053/054 become real).
  *Done when:* example: text inside Row/Column/Padding lays out correctly; baseline alignment works.
- [ ] **095 — RichText / InlineSpan** — styled spans within one paragraph.
  *Done when:* example renders a sentence with mixed styles.
- [ ] **096 — Text selection foundations (deferred OK)** — glyph position ↔
  offset mapping APIs only (caret math), no UI yet.
  *Done when:* tests: position→offset→position round trips.

## Phase 8 — Gestures

Goal: from raw pointers (Phase 5) to semantic gestures with disambiguation.

- [ ] **100 — Gesture arena** — the disambiguation protocol: members, accept/
  reject, sweep, hold.
  *Done when:* arena unit tests cover resolve-on-sweep and eager winner cases.
- [ ] **101 — TapRecognizer** — down/up within slop & timeout, tap cancel.
  *Done when:* example: reliable tap vs. drag-away-no-tap behavior.
- [ ] **102 — Drag/Pan recognizers** — touch slop, drag start/update/end with
  velocity tracking (velocity estimator is its own mini-feature — keep it in).
  *Done when:* example: draggable box with a velocity readout; fling velocity sane.
- [ ] **103 — Tap vs. drag competition** — both recognizers on one region resolve
  correctly through the arena.
  *Done when:* example: tap toggles color, drag moves — no misfires.
- [ ] **104 — GestureDetector widget** — the widget-layer wrapper wiring
  recognizers to element lifecycle.
  *Done when:* counter example (077) rewritten with `GestureDetector::on_tap`.
- [ ] **105 — Long press & double tap** — timing-based recognizers atop the arena.
  *Done when:* example distinguishes tap/double-tap/long-press on one target.

## Phase 9 — Animation

Goal: Flutter's animation stack, which mostly builds itself once tickers exist.

- [ ] **110 — AnimationController** — forward/reverse/stop/repeat, bounds,
  status callbacks; driven by tickers (036); vsync muting when not visible (defer OK).
  *Done when:* tests with fake clock verify timing; example animates opacity 0→1.
- [ ] **111 — Curves & CurvedAnimation** — the standard easing curve set,
  curve application, flipped/interval curves.
  *Done when:* example draws curve graphs (dogfooding: rendered with Mel itself).
- [ ] **112 — Tween & Animatable chains** — `Tween<T>` for f64/Color/Offset/Rect/
  EdgeInsets, `chain`, `animate`.
  *Done when:* tests for each lerp; example moves + recolors a box on one controller.
- [ ] **113 — AnimatedBuilder / ListenableBuilder** — rebuild-on-tick widgets;
  the Listenable/ChangeNotifier design for Rust (ADR — used by scrolling too).
  *Done when:* example animates without manual set_state plumbing.
- [ ] **114 — Implicit animations** — `AnimatedContainer`-style: detect prop
  change, tween from old to new (`ImplicitlyAnimatedWidget` machinery).
  *Done when:* example: click toggles size/color/radius, animates smoothly.
- [ ] **115 — Hero-less transitions kit** — FadeTransition, SlideTransition,
  ScaleTransition, RotationTransition (transform/opacity render objects reused).
  *Done when:* example: staggered entrance animation of a list of cards.

## Phase 10 — Scrolling

Goal: viewports and physics — the feature that stress-tests everything before it.

- [ ] **120 — ScrollPosition & physics** — offset, min/max extent, clamping vs.
  bouncing physics, ballistic simulation (friction/spring simulations).
  *Done when:* simulation unit tests (given v₀ → positions over time); no UI yet.
- [ ] **121 — RenderViewport (box-based)** — a viewport that offsets & clips one
  oversized child (ListView-less scrolling: `SingleChildScrollView` semantics).
  *Done when:* example scrolls a tall Column with drag + fling using 102 + 120.
- [ ] **122 — Scrollbar & scroll indicators** — painted scrollbar reflecting
  position (no interaction yet; thumb drag optional follow-up).
  *Done when:* example shows a fading scrollbar during scroll.
- [ ] **123 — Sliver protocol (decision + core)** — ADR: adopt slivers or a
  simpler virtualization model? If slivers: SliverConstraints/Geometry, viewport
  as sliver host. (May split into 123a/b/c during design.)
  *Done when:* ADR done; chosen model's core types compile with tests.
- [ ] **124 — Lazy list (builder)** — virtualized children: build/layout only
  what's visible + cache extent; element/render object recycling correctness.
  *Done when:* example scrolls 100k items at interactive speed; tree dump shows ~visible-only children.
- [ ] **125 — ScrollController & notifications** — programmatic `animate_to`/
  `jump_to`, position listeners, scroll notifications bubbling.
  *Done when:* example: "scroll to top" button + position-driven header effect.

## Phase 11 — Developer Experience & Tooling

Goal: make Mel pleasant to build *with* — and to debug. (Items here can be
interleaved earlier when they'd pay off immediately; record reordering in ADRs.)

- [ ] **130 — Debug paint overlay** — toggle: layout bounds, baselines, pointer
  hit regions, repaint rainbow (repaint flashing catches over-painting).
  *Done when:* any example can toggle overlays via the debug trigger (055).
- [ ] **131 — Performance overlay** — frame time graph (build/layout/paint/
  raster split), jank highlighting — rendered with Mel itself.
  *Done when:* overlay visibly spikes when an example artificially stalls a phase.
- [ ] **132 — Panic & error boundaries** — a build/layout error in one subtree
  renders an error box instead of killing the app (Flutter's red box).
  *Done when:* example with a deliberately panicking widget shows the error box and keeps running.
- [ ] **133 — Golden tests harness** — render scenes headless to PNG, compare
  against goldens with a diff report.
  *Done when:* CI runs goldens for a chosen set of examples; intentional change workflow documented.
- [ ] **134 — Widget test harness** — pump frames with fake clock, find widgets,
  synthesize taps/drags (`WidgetTester` equivalent).
  *Done when:* counter app has a widget test: tap → text changes, no window needed.
- [ ] **135 — Hot restart** — state-preserving reload is out of scope; fast
  restart loop (watch, rebuild, redeploy to Simulator in a few seconds) tuned
  instead — iOS build+deploy latency makes this feature matter more, not less.
  *Done when:* watch-based rebuild-and-relaunch flow documented + build-time optimizations measured in ADR.

## Phase 12 — Consolidation Showcase

Goal: prove the stack composes — and produce the first "real" Mel app.

- [ ] **140 — Basic controls kit** — Button, Checkbox, Switch, Slider,
  TextField-less form controls built only from public Mel APIs (each control may
  become its own sub-feature during design).
  *Done when:* controls gallery example; every control built without touching internals.
- [ ] **141 — Theming** — InheritedWidget-based theme (colors, text styles,
  shape), light/dark toggle at runtime.
  *Done when:* gallery switches themes live.
- [ ] **142 — Navigation (minimal)** — page stack, push/pop with transitions
  (115), back handling.
  *Done when:* two-page app with animated transitions.
- [ ] **143 — Showcase app** — a small real app (e.g. task list: scrolling,
  gestures, animations, theming, navigation, state).
  *Done when:* the app is pleasant to use; a full retrospective ADR reviews the entire architecture.

---

## Later / Explicitly Deferred (revisit after Phase 12)

Text editing & IME (on-screen keyboard) · hardware keyboard & focus system ·
mouse/hover support & cursors (desktop) · accessibility/semantics tree ·
RTL/directionality · desktop, Android & web platforms · multi-window ·
Impeller-style backend in `mel-impeller` (the painting-layer abstraction from
Phase 2 is the door) · hot reload · plugin/FFI story · async image loading &
cache · custom shaders · platform views (UIKit view embedding).

---

## Progress Snapshot

| Phase | Features | Done |
| --- | --- | --- |
| 0 — Platform Shell (iOS) | 9 | 1 |
| 1 — Foundation | 7 | 0 |
| 2 — Painting | 10 | 0 |
| 3 — Compositing & Scheduling | 7 | 0 |
| 4 — Layout | 16 | 0 |
| 5 — Hit Testing | 4 | 0 |
| 6 — Widgets & State | 11 | 0 |
| 7 — Text | 7 | 0 |
| 8 — Gestures | 6 | 0 |
| 9 — Animation | 6 | 0 |
| 10 — Scrolling | 6 | 0 |
| 11 — DevEx & Tooling | 6 | 0 |
| 12 — Showcase | 4 | 0 |
| **Total** | **99** | **1** |
