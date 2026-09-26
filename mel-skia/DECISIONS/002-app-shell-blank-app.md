# 002 — App shell decision & blank app

## Flutter reference

Flutter's iOS story is a **native host app embedding an engine library**. The
tool generates a real Xcode project (`Runner`) whose app delegate subclasses
`FlutterAppDelegate`; the screen is a `FlutterViewController` (a plain
`UIViewController` subclass) owning a `FlutterView` backed by a
`CAMetalLayer`. The engine itself ships as `Flutter.framework`. The embedder
(`shell/platform/darwin/ios`, Objective-C++) is the translation layer:

- **Lifecycle**: `UIApplicationDelegate` callbacks (`didFinishLaunching`,
  `willResignActive`, `didEnterBackground`, memory warnings) are forwarded
  into the engine as lifecycle messages.
- **Vsync**: `VsyncWaiterIOS` wraps a `CADisplayLink`.
- **Input**: `FlutterViewController` overrides `touchesBegan/Moved/Ended/
  Cancelled`, packs them into engine `PointerData` packets.

What's essential: *some* code must live on the UIKit side of the fence and
translate UIKit concepts (delegate callbacks, display link, touches, insets)
into engine concepts — that translation layer is exactly what `mel-platform`
is. What's incidental: that Flutter's translation layer is written in
Objective-C inside an Xcode host project. That is an artifact of Flutter
being a *product* that must slot into the App Store/Xcode/add-to-app
ecosystem and host a Dart runtime. Mel has no second language runtime — the
"engine" and the "embedder" can be one Rust program.

## Decision

**Pure-Rust shell, talking to UIKit directly through `objc2` +
`objc2-ui-kit`. No Xcode project, no winit.** `main()` is Rust: it calls
`UIApplicationMain` with a Rust-defined `MelAppDelegate` class
(`define_class!`), which on `didFinishLaunchingWithOptions` creates a
full-screen `UIWindow` with a plain root `UIViewController` — the blank app.

- `mel-platform` exposes exactly one symbol for now: `run() -> !`
  (iOS-only, `#[cfg(target_os = "ios")]`). The UIKit interop lives in a
  private `ios` module; no `objc2`/UIKit type appears in the public API
  (CONTEXT.md §5 — the platform layer never leaks upward).
- The `objc2*` dependencies are declared under
  `[target.'cfg(target_os = "ios")'.dependencies]`, so host-side
  `cargo test`/`clippy` never build them.
- **Lifecycle model**: classic `UIApplicationDelegate` + `UIWindow`, *not*
  the scene-based (`UIScene`) lifecycle. Scenes buy multi-window on
  iPad/visionOS, which is explicitly deferred ("multi-window" in ROADMAP's
  Later list), and cost an extra delegate + manifest now. Apple still fully
  supports the classic path (Xcode 16.3 / iOS 18.4), but is nudging toward
  scenes — debt item filed to re-evaluate at 008 (lifecycle feature).
- **App bundle & one-command flow**: an iOS app bundle is just a directory
  (`Name.app/` containing the binary + `Info.plist`); the Simulator requires
  no codesigning. `scripts/run-sim.sh <example>` does: `cargo build
  --target aarch64-apple-ios-sim --example <example>` → assemble
  `target/sim/<example>.app` (generated `Info.plist`, `UILaunchScreen = {}`
  for edge-to-edge full screen without a storyboard) → boot a Simulator if
  none is booted → `simctl install` + `simctl launch --console-pty` (stdout
  of the app streams into the terminal — our print-debugging channel until
  real tooling lands in Phase 11).
- **Examples attachment** (left open by ADR 001): demo files stay at the
  repo-convention location `examples/NNN_slug.rs` and are declared as
  `[[example]]` targets of `mel-platform` with an explicit `path`. Each
  example guards `main` with `#[cfg(target_os = "ios")]` (plus a stub `main`
  that explains itself on other targets) so host-side `clippy --all-targets`
  stays green.

## Alternatives considered

- **winit** — cross-platform event loop, iOS backend exists. Rejected:
  winit's job is to *hide* platform differences, while Phase 0's job is to
  *learn and own* them — features 004–008 need precise control of
  `contentScaleFactor`, safe-area insets, `CADisplayLink` pause/resume,
  memory warnings, and raw `UITouch` streams, exactly the things a
  lowest-common-denominator API abstracts away or drips through extension
  traits. winit would also dictate the shape of our frame loop (its
  callback-driven `ApplicationHandler` model) before we've designed
  scheduling (Phase 3). Notably winit's iOS backend is itself built on
  `objc2` — we'd be paying for a middleman to use the same bricks. Desktop
  ports later may still choose winit *inside their own platform shell*;
  nothing above the platform layer would notice.
- **Thin Xcode host app + `staticlib` FFI (Flutter's model)** — canonical
  Apple flow; would make codesigning/device deploy (009) and any future
  add-to-app story trivial. Rejected for now: it puts `xcodebuild` and a
  `.xcodeproj` into every build loop (slower, noisier diffs), splits the
  embedder's logic across Swift/ObjC + a hand-maintained FFI boundary —
  directly against "understanding over velocity" since the interesting
  lifecycle code would live outside Rust. If 009 (real device) turns out to
  demand an Xcode wrapper for signing/deploy, we can add a *shell-only*
  host that links `mel-platform` without rewriting anything — the Rust API
  boundary (`run()`) already exists.
- **Scene-based lifecycle from day one** — rejected above; see debt item.
- **`cargo-xtask` instead of a shell script** for the run flow — more
  Rust-y and could grow watch/deploy modes, but it's a second binary and
  ~10× the code for zero present benefit on a macOS-only, Simulator-only
  flow. Re-evaluate at 009 (device deploy) and 135 (fast restart loop).

## Consequences

- Phase-0 features (003–008) all land as Rust code in `mel-platform`'s
  `ios` module: the display link, Metal/raster layer, touch handlers and
  lifecycle hooks each have an obvious home with full UIKit access.
- `mel-platform` is the second crate (after `mel-painting`'s Skia grant)
  allowed `unsafe`: every ObjC message send is an FFI call. Kept inside the
  private `ios` module, each `unsafe` block justified (CONTEXT.md §5).
- The Simulator flow needs no signing; **real-device deploy (009) will need
  a signing story** we have deliberately not built — that feature may
  reopen the Xcode-host alternative, and this ADR says that's fine.
- No `.xcodeproj` exists, so Xcode-native debugging (Instruments, view
  hierarchy inspector) isn't one click away; `lldb` still attaches to the
  Simulator process fine, and Instruments can target it by PID.
- The classic-lifecycle choice is a known bet against Apple's direction —
  revisited at 008 (debt item 002-1).

## Review outcome

Device-flow follow-up: [ADR 009](009-real-device.md) keeps the Rust executable
and adds development signing/deployment through Xcode CLI tools. No Xcode host
or FFI boundary was needed; profile creation/renewal remains a setup step.

Reviewed and approved 2026-08-05. Correctness ✅ · Architecture ✅ ·
API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
Debt filed: **002-1** (classic `UIApplicationDelegate` lifecycle instead of
`UIScene`; re-evaluate at 008 — Apple has announced scene adoption will
eventually become mandatory, and Flutter itself migrated in 2025).
