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

_Last updated: 2026-08-05, after feature 002._

---

## 1. Status at a glance

- **Done (reviewed):** 001 (workspace & toolchain), 002 (app shell & blank app).
- **Next up:** 003 — Skia surface on screen.
- **Open debt:** 002-1 (classic lifecycle vs `UIScene` — see [DEBT.md](../DEBT.md)).

## 2. What runs today

```sh
scripts/run-sim.sh 002_blank_app
```

launches a blank white full-screen app on the iOS Simulator. It is a pure-Rust
UIKit app (no Xcode project, no Swift, no winit): Rust calls the Objective-C
runtime directly via `objc2`. **Skia is not used anywhere yet** — the white
screen is a plain `UIView.backgroundColor` composited by UIKit/Core Animation.

## 3. Repository map (real vs. placeholder)

```text
mel-skia/
├── doc_context/          CONTEXT.md (process) · ROADMAP.md (status) · this file
├── DECISIONS/            001-workspace-crate-layout.md · 002-app-shell-blank-app.md
├── DEBT.md               one open item (002-1)
├── examples/             002_blank_app.rs + README (how examples work)
├── scripts/run-sim.sh    the one-command Simulator flow (ADR 002)
├── rust-toolchain.toml   stable · edition 2024 · both iOS targets pinned
├── .github/workflows/    CI exists but is disabled/commented (ADR 001 amendment)
└── crates/               one crate per layer (ADR 001); only mel-platform has code
```

| Crate | Layer (CONTEXT.md §5) | State |
| --- | --- | --- |
| `mel-foundation` | geometry, color, ids | empty placeholder |
| `mel-painting` | Canvas/Paint over Skia — the only crate ever allowed `skia-safe` | empty placeholder |
| `mel-compositing` | layer tree, scenes | empty placeholder |
| `mel-scheduling` | frame pipeline, tickers | empty placeholder |
| `mel-rendering` | RenderObject, layout | empty placeholder |
| `mel-widgets` | Widget/Element trees | empty placeholder |
| `mel-platform` | iOS app shell | **implemented (002)** — see §4 |

Inter-crate dependency edges are added only when a feature first needs them
(none exist yet); Cargo then enforces the one-directional layering.

## 4. `mel-platform` — the app shell (feature 002)

Public API — this is everything layers above can see:

```rust
mel_platform::run() -> !   // iOS-only (#[cfg(target_os = "ios")])
```

- [`src/lib.rs`](../crates/mel-platform/src/lib.rs) — the API above; no UIKit
  type leaks upward (CONTEXT.md §5).
- [`src/ios.rs`](../crates/mel-platform/src/ios.rs) — the only file that
  speaks Objective-C. `define_class!` registers `MelAppDelegate`
  (an `NSObject` + `UIApplicationDelegate` implemented in Rust); `run()`
  calls `UIApplicationMain`, which never returns; the launch callback
  creates a screen-sized `UIWindow` + empty root `UIViewController` (white
  background) and stores the window in the delegate's ivars (UIKit does not
  retain the key window).
- **Lifecycle model:** classic `UIApplicationDelegate`/`UIWindow`, *not*
  scene-based (`UIScene`) — debt 002-1, re-evaluated at 008. The
  `#[expect(deprecated)]` on `UIScreen::mainScreen` / `UIWindow::initWithFrame`
  is deliberate: those are scenes-era deprecations that don't apply to a
  scene-less app. Don't "fix" them independently of 002-1.
- **Dependencies** ([Cargo.toml](../crates/mel-platform/Cargo.toml)):
  `objc2`, `objc2-foundation`, `objc2-ui-kit` — iOS-target-only
  (`[target.'cfg(target_os = "ios")'.dependencies]`), with minimal explicit
  feature lists. Touching a new UIKit class usually means adding its feature
  flag there.

**Examples wiring (ADR 002):** demo files live at `examples/NNN_slug.rs`
(repo convention) and are declared as `[[example]]` targets of `mel-platform`
with an explicit `path`. Each example guards `main` with
`#[cfg(target_os = "ios")]` plus a host-side stub so workspace-wide
clippy/test stay green on the Mac.

## 5. Build, check & run flows

Host (must be clean before every review — CONTEXT.md §6):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

iOS (the shell code only compiles for iOS targets):

```sh
cargo clippy --target aarch64-apple-ios-sim -p mel-platform --all-targets -- -D warnings
cargo build --target aarch64-apple-ios -p mel-platform --example 002_blank_app
```

Run on the Simulator: `scripts/run-sim.sh <example>` — builds (debug) for
`aarch64-apple-ios-sim`, assembles `target/sim/<example>.app` (generated
`Info.plist`; `UILaunchScreen = {}` is what makes it full-screen without a
storyboard), boots an iPhone Simulator if none is booted, `simctl install` +
`launch --console-pty` (app stdout streams to the terminal — the only
"logging" channel until Phase 11). Apple Silicon only.

## 6. Decisions in force (details in `DECISIONS/`)

- **ADR 001** — one Cargo workspace, one crate per layer; layering enforced
  by Cargo (no cycles, edges reviewed); toolchain pinned; CI disabled until
  it pays for itself.
- **ADR 002** — pure-Rust UIKit shell via `objc2`; no Xcode project, no
  winit; classic lifecycle (debt 002-1); one-command flow is a shell script
  (revisit xtask at 009/135); examples attach to `mel-platform`. Feature 009
  (real device) may reopen the Xcode-host question for codesigning.
- `unsafe` only in the platform/painting layers, each use justified
  (CONTEXT.md §5).

## 7. What does NOT exist yet — don't assume it does

- No `skia-safe` dependency anywhere; nothing draws except UIKit itself.
- No display link / frame loop (005–006), no touch input (007), no
  lifecycle/background hooks (008), no real-device or codesigning flow (009).
- Six of the seven crates are empty; there are no unit tests yet (nothing
  with logic to test).
- No logging/debug tooling beyond `println!` through the attached console.

## 8. Where the next features land

- **003 — Skia surface on screen:** `skia-safe` enters the workspace,
  confined to `mel-painting`. The presentation side (CPU raster into a
  `CALayer` vs. `CAMetalLayer`) extends `mel-platform/src/ios.rs` — likely a
  custom `UIView`/layer under the root view controller the shell already
  creates. First inter-crate dependency edges appear here. Example
  `003_*.rs` + ADR record which surface path and why.
- **004 (scale/safe areas/rotation), 005 (display link), 007 (touches),
  008 (lifecycle + GPU decision):** all live in `mel-platform`'s `ios`
  module, translating UIKit concepts into Mel types (which will start
  populating `mel-foundation` from Phase 1).

## 9. Working rules recap (full text: CONTEXT.md)

- One feature per pass: Study → Design (ADR) → Build → Demo → stop for
  human review. Never mark `[x]` before review passes.
- Never implement ahead of the roadmap; split or stub + file debt instead.
- **AI agents never commit** — leave everything in the working tree.
