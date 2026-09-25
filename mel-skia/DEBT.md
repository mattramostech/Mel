# Debt

Accepted ⚠️ items from feature reviews, each tagged with the feature ID that
introduced it. Revisited at every phase boundary (CONTEXT.md §3).

- **002-1** — App shell uses the classic `UIApplicationDelegate`/`UIWindow`
  lifecycle, not scene-based (`UIScene`). Apple is nudging toward scenes;
  multi-window stays deferred. Re-evaluated at **008**: retain the tested
  single-window delegate for now. Revisit at **009** for device/SDK
  compatibility and at the Phase 0 gate; migrate before adopting a toolchain
  that requires scenes. See [DECISIONS/002-app-shell-blank-app.md](DECISIONS/002-app-shell-blank-app.md).

- **008-1 (proposed, pending review)** — Retain CPU raster and per-frame
  allocation/image copy. Simulator measurements are not hardware evidence.
  Re-evaluate Metal with **009** hardware timings and before **020** defines
  the painting API. A GPU path must preserve painting ownership of Skia.
  See [ADR 008](DECISIONS/008-app-lifecycle.md).
