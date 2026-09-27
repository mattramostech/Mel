# Debt

Accepted ⚠️ items from feature reviews, each tagged with the feature ID that
introduced it. Revisited at every phase boundary (CONTEXT.md §3).

- **002-1** — App shell uses the classic `UIApplicationDelegate`/`UIWindow`
  lifecycle, not scene-based (`UIScene`). Apple is nudging toward scenes;
  multi-window stays deferred. Re-evaluated at **008**: retain the tested
  single-window delegate for now. **009** built with the tested Xcode toolchain
  and launched on a physical iPhone using that lifecycle. Retain for the tested
  toolchain. **Phase 0 gate (2026-09-26):** consciously retained after rerunning
  all Simulator demos; migrate before adopting a toolchain
  that requires scenes. See [DECISIONS/002-app-shell-blank-app.md](DECISIONS/002-app-shell-blank-app.md).

- **008-1** — Retain CPU raster and per-frame
  allocation/image copy. **009** measured the 006 debug demo on a physical iPhone:
  30 post-initial samples, 3.07 ms median CPU submission at 16.68 ms intervals,
  with no sampled overruns. Retain for the current minimal scene; this does not
  establish complex-scene performance or full-frame/display latency.
  **Phase 0 gate (2026-09-26):** consciously retained; review again before
  **020** defines the painting API. A GPU path must
  preserve painting ownership of Skia. See [ADR 008](DECISIONS/008-app-lifecycle.md)
  and [ADR 009](DECISIONS/009-real-device.md).
