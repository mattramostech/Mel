# 003 — Skia surface on screen

## Flutter reference

Flutter's mainline iOS path is GPU-composited: `FlutterView` overrides
`layerClass` to return `CAMetalLayer`, and the rasterizer (Skia then, Impeller
now) records GPU commands and presents drawables through Metal. But Flutter
also shipped a **software fallback** (`IOSSurfaceSoftware`, used on the
pre-Metal Simulator and old devices) whose pipeline is exactly three steps:
raster the frame with Skia's CPU backend into a bitmap, wrap the bitmap's
bytes in a `CGImage`, and hand that image to Core Animation as a layer's
`contents`. UIKit then composites it like any other layer.

Essential: the engine draws into *a surface it owns* and hands finished
pixels to the OS compositor; the app's layer is the meeting point. Also
essential is Flutter's layering: the rasterizer (Skia) knows nothing about
UIKit, and the embedder knows nothing about Skia — they meet through a
neutral "here are the frame's pixels" contract. Incidental: Metal itself
(an optimization we're explicitly allowed to defer to 008), and Flutter's
`layerClass` override (an artifact of having a dedicated view subclass).

## Decision

**CPU raster presented through `CALayer.contents` — no Metal yet.** The
roadmap schedules the GPU decision at 008; starting with software rendering
keeps 003 at "prove pixels flow", with Skia's raster backend (no `skia-safe`
GPU features, no `CAMetalLayer`, no command queues).

The pixel pipeline, and who owns each step:

```text
mel_painting::fill()             mel-platform (ios.rs)              UIKit
Skia canvas → BGRA buffer   →    CGImage (CFData copy)      →   root view's
   (the only Skia user)          layer.contents = image          compositor
```

- **`mel-painting` gets its first real API**: `fill(pixels, width, height,
  color)` — creates a Skia raster surface *wrapping the caller's buffer*
  (`surfaces::wrap_pixels`) and clears it through a Skia canvas. A deliberate
  bootstrap: Phase 2 (020+) replaces it with real `Canvas`/`Paint` wrappers.
  No Skia type appears in the signature (CONTEXT.md §5). Unit tests pin the
  buffer contract: **BGRA8888, premultiplied alpha, tightly packed rows**.
- **`mel-platform` grows the other half of the contract**: `run()` becomes
  `run(draw: impl FnOnce(&mut Frame) + 'static)`. `Frame` is a Mel-owned
  type (logical width/height + byte buffer) — no UIKit or Skia types cross
  the API. At launch the shell allocates the buffer, invokes `draw`, wraps
  the bytes in a `CGImage` (`CFData` → `CGDataProvider` → `CGImageCreate`,
  device RGB, `PremultipliedFirst | Order32Little` = BGRA little-endian) and
  assigns it to the root view's `layer.contents`. The callback is parked in
  a `thread_local` (not a `static`): `run()` asserts main-thread and UIKit
  delivers `didFinishLaunching` on that same thread, so no `Send` bound.
- **The layers meet only in the example**: `003_skia_surface.rs` passes a
  closure that calls `mel_painting::fill` on the frame the shell provides.
  The byte-format contract is documented on both sides and enforced by
  `mel-painting`'s unit tests; the crates stay decoupled.
- **First inter-crate edge**: `mel-platform` → `mel-painting` as a
  **dev-dependency only** (examples are app-level code sitting above every
  layer; the library itself gains no edge, so the CONTEXT.md §5 layering is
  untouched — `cargo tree -i skia-safe` still shows only `mel-painting`).
- Example 002 was adapted to the new signature (`run(|_frame| {})`) — the
  API evolved and examples are regression checks, not frozen call sites.

Deliberately deferred, per the roadmap (not debt — these have numbers):
scale factor (buffer is logical-points-sized, so on a 3× screen the solid
color is upscaled — invisible for 003, fixed in **004**); redraw/resize (the
frame is drawn **once at launch**; loop in **005**); GPU/Metal (**008**).

## Alternatives considered

- **Metal-backed `CAMetalLayer` now** (`skia-safe` `metal` feature) — the
  performant path and where 008 may land. Rejected for 003: it drags in a
  device/queue/drawable lifecycle, `layerClass` plumbing, and skia-safe's
  GPU context API before we've drawn a single pixel. The roadmap explicitly
  makes GPU a *later, informed* decision (008) once lifecycle and frame
  timing exist to evaluate it against.
- **Custom `UIView` subclass with `drawRect:`** (Flutter's software path
  drew via the `CGContext` there) — needs a second `define_class!` and buys
  nothing over `layer.contents` for a full-screen static image; UIKit ends
  up compositing a layer either way. Revisit only if partial-invalidation
  drawing ever matters.
- **Skia-owned surface + `read_pixels` copy-out** instead of
  `wrap_pixels` on the platform buffer — an extra copy and an extra API to
  hold; wrapping the caller's buffer is what `SkSurface` is designed for.
- **A shared pixel-format type between the crates** — would need a home
  (`mel-foundation`? a new contract crate?) and a design pass, for a
  contract that is today two integers and a byte order. Documented + tested
  convention now; Phase 1/2 types will absorb it.

## Consequences

- The painting layer's "only crate that touches `skia-safe`" rule is now
  mechanical: `cargo tree -i skia-safe` → `mel-painting` (+ dev-edge from
  `mel-platform` for examples).
- `run(draw)`'s shape — app hands the shell a callback, shell owns the
  buffer and the presentation — is the seed of the real frame pipeline;
  005/006 will evolve the callback (timestamps, repeated frames) rather
  than invent a new channel.
- Presenting copies the buffer twice (Skia writes it, `CFData::from_bytes`
  copies for the provider). Fine for one launch frame; the frame loop (005)
  and GPU decision (008) must revisit presentation cost before animating.
- `skia-safe` is a heavyweight dependency (prebuilt binaries per target;
  first build downloads them for host, `-sim`, and device targets). ADR 001's
  amendment names skia-safe breakage as a trigger to re-enable CI.
- Color is untagged device RGB end-to-end; color management stays deferred
  until feature 013's ADR.

## Review outcome

GPU decision follow-up: [ADR 008](008-app-lifecycle.md) retains CPU raster
after evaluating Metal, with proposed debt 008-1 for hardware timing (009)
and painting API design (020).

Reviewed and approved 2026-08-05. Correctness ✅ · Architecture ✅ ·
API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No debt items filed — the known limitations (logical-points buffer, single
launch-time frame, CPU raster, per-present copy) are scheduled roadmap
features (004, 005, 008), recorded in this ADR's Decision/Consequences.
