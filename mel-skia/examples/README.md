# Examples

Numbered runnable demos, one per visible feature (`NNN_slug.rs`), never
deleted — the living history of the project and its regression checks
(CONTEXT.md §2.4). Files live here but are `[[example]]` targets of
`mel-platform` (ADR 002), so cargo can build them.

Launch any example on the iOS Simulator with the one-command flow:

```sh
scripts/run-sim.sh 002_blank_app
```

The script builds for `aarch64-apple-ios-sim`, assembles a `.app` bundle
under `target/sim/`, boots a Simulator if none is running, installs, and
launches with the app's console attached (Ctrl-C detaches; the app keeps
running). See [ADR 002](../DECISIONS/002-app-shell-blank-app.md).

| # | Example | Shows |
| --- | --- | --- |
| 002 | `002_blank_app` | Blank white full-screen app from the pure-Rust UIKit shell |
| 003 | `003_skia_surface` | Full-screen honey color rasterized by a Skia canvas (CPU raster) |
| 004 | `004_scale_rotation` | 100-logical-pt square at the safe-area corner; survives rotation (Cmd+←/→) |
| 005 | `005_redraw_on_demand` | Tap cycles the background via `request_frame()` — exactly one repaint per tap, ~0% CPU when idle |
| 006 | `006_frame_timing` | Rectangle moves from display-link timestamps; sampled CPU cost vs. refresh interval and deadline; tap pauses/resumes |
| 007 | `007_touch_input` | Logs Mel pointer phases/IDs/positions/time; a colored square follows each active contact |

Feature 006: `make run EXAMPLE=006_frame_timing` starts a four-second horizontal cycle inside the safe
area. Check that the motion remains smooth after rotation; changing refresh
rate or dropping frames must not change its period. Tap to pause (at most one
pending frame finishes), wait for console silence, then tap to restart.
The console samples CPU submission cost once per timestamp-second. A positive
`deadline overrun` means submission missed the target, even if CPU cost is
less than the interval. These debug/Simulator measurements include CPU raster
and image copies, but do not measure actual on-screen presentation.

Run the demo's deterministic motion tests on the host with:
`cargo test -p mel-platform --example 006_frame_timing`.

Feature 007: `make run` (or `scripts/run-sim.sh 007_touch_input`) opens a
honey background. Touch and drag to show a 60-logical-point colored square;
release to remove it. Hold Option while dragging to simulate two fingers
in Simulator. The console must show distinct Down IDs and `active=2`, then
Move events preserving each ID, and Up removing both (`active=0`). A later
contact must get a fresh ID. Rotate and repeat: squares should stay under
the contacts, with positions measured from the full viewport's top-left.
Cancel also removes a contact; the host tracker tests cover cancellation of
one contact while another continues. No input means no new frame requests.
The shell reports input only; these squares are not gesture recognizers or
hit-test targets. Examples 005/006 now respond only to PointerPhase::Down.
