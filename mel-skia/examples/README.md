# Examples

Numbered runnable demos, one per visible feature (`NNN_slug.rs`), never
deleted — the living history of the project and its regression checks
(CONTEXT.md §2.4). Files live here and attach to their owning crate as
`[[example]]` targets: iOS demos use `mel-platform` (ADR 002); pure geometry
uses `mel-foundation` (ADR 010).

Launch any iOS demo on the Simulator with the one-command flow:

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
| 008 | `008_app_lifecycle` | Animation pauses on interruption/background, restarts on activation; lifecycle/frame logs and memory-warning color change |
| 009 | Reuses `006_frame_timing` | Signed physical-iPhone deployment via `make run-device`; hardware vs. Simulator timing sanity check |
| 010 | `010_offset_size` (host) | Offset/Size arithmetic, origin-relative center, containment, interpolation and unbounded extents |
| 011 | `011_rect` (host) | Rect construction, clipping intersection, bounding union, half-open containment and empty/inverted overlap rules |
| 012 | `012_radius_rrect` (host) | Circular/elliptical corners, proportional fitting, containment and safe inner/outer bounds |

Feature 010 runs on the Mac without UIKit or Skia:

```sh
cargo run -p mel-foundation --example 010_offset_size
cargo test -p mel-foundation
```

The console prints a center at `(62, 60)` for a 100×80 size at `(12, 20)`,
growth as an Offset, interpolation/extrapolation samples and an unbounded
height. Assertions check containment (right/bottom excluded) and edge cases.
This pure-logic demo does not use `scripts/run-sim.sh`.

Feature 011 also runs on the host:

```sh
cargo run -p mel-foundation --example 011_rect
cargo test -p mel-foundation --test rect
```

The console shows bounds created from `Offset & Size`, their intersection
with a centered clip, and their bounding union. Assertions check excluded
right/bottom edges, a disjoint intersection that retains negative width,
inflation/deflation, and rejection of empty, inverted or NaN overlap. No
rendering or Simulator is needed. See [ADR 011](../DECISIONS/011-rect.md).

Feature 012 also runs on the host and draws nothing:

```sh
cargo run -p mel-foundation --example 012_radius_rrect
cargo test -p mel-foundation --test rrect
```

The console shows outer/inner bounds for four distinct corners and the common
scale applied to oversized elliptical radii. Assertions check half-open edges,
overlapping diagonal corner regions and invalid/empty geometry. Fitting and
safe inner bounds return `None` for negative/non-finite radii or unusable
bounds; containment returns false. See [ADR 012](../DECISIONS/012-radius-rrect.md).

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

Feature 007: `make run EXAMPLE=007_touch_input` (or `scripts/run-sim.sh 007_touch_input`) opens a
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

Feature 008: `make run` (or `scripts/run-sim.sh 008_app_lifecycle`) starts
an animated square on honey. Use Simulator Home, wait, then reopen the app:
logs show Active → Inactive → Background → Inactive → Active. Frame counts
must remain unchanged between leaving Active and returning; the demo requests
a frame in every lifecycle callback specifically to exercise this guard.
Motion restarts on activation, with current viewport dimensions. Repeat the
cycle and rotate to check recovery. Brief interruptions (e.g. Control Center)
should show Inactive → Active without needing Background.

The memory callback logs a warning count and toggles honey/green on the next
frame. This Simulator version (iOS 26.5) has no memory-warning
menu item. To exercise callback routing without real memory pressure, obtain
the app PID from `xcrun simctl launch booted dev.mel.example.008-app-lifecycle`,
then run `xcrun lldb -p <PID>`. On the main thread, while stopped in the run
loop (not inside an App callback), enter:

```text
thread select 1
expression -l objc++ -- id $mel_application = (id)[(id)objc_getClass("UIApplication") sharedApplication]
expression -l objc++ -- (void)[(id)[$mel_application delegate] applicationDidReceiveMemoryWarning:$mel_application]
process detach
quit
```

This invokes the actual delegate hook but does not simulate OS memory pressure.
In background, a warning must not wake drawing.
Run 007 and interrupt a held contact: each remaining ID gets exactly one
Cancel before the lifecycle notification; returning must leave no stuck marker.
Earlier 005 should repaint once on activation and then remain idle. CPU timing
samples are submission costs, not GPU/display latency or device benchmarks.

### Feature 009: run on a physical iPhone

Prerequisites: Xcode with `devicectl`, Python 3 (standard library only), the
`aarch64-apple-ios` Rust target, and an iPhone running iOS 17+. Pair the phone
with Xcode's Devices and Simulators window, trust the Mac, enable
[Developer Mode](https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device),
and unlock the phone before launching. USB or an already paired network
connection works. The script uses existing signing assets; it does not create
or renew certificates/profiles or require an Xcode project for Mel.

One-time signing setup: in Xcode, sign in with your Apple account and create an
Apple Development certificate (Settings → Accounts → Manage Certificates).
Use a small temporary iOS App project with automatic signing, your team, the
desired bundle ID and this iPhone as destination; build/run it once so Xcode
registers the device and creates a development profile. Alternatively, create
and download an iOS development profile in your developer account. Xcode stores
profiles in `~/Library/Developer/Xcode/UserData/Provisioning Profiles/` (older
versions used `~/Library/MobileDevice/Provisioning Profiles/`). The matching
certificate **and private key** must be installed in the login keychain.
See Apple's [device setup](https://help.apple.com/xcode/mac/current/en.lproj/dev5a825a1ca.html).

Choose a wildcard profile permitting `dev.mel.example.*`, or set `BUNDLE_ID`
to an explicit ID covered by your profile. The profile must include the selected
iPhone's **UDID**; this is different from devicectl's device identifier. The
script queries the UDID and validates membership automatically.

```sh
xcrun devicectl list devices
export DEVICE_ID='<device identifier from that list>'
export PROVISIONING_PROFILE='/absolute/path/to/profile.mobileprovision'
# Only for a profile with a different explicit App ID:
# export BUNDLE_ID='com.yourteam.mel-demo'
make run-device                         # defaults to 006_frame_timing
make run-device EXAMPLE=008_app_lifecycle
# Equivalent direct invocation:
python3 scripts/run-device.py 006_frame_timing
```

The script validates the profile, chooses its sole matching keychain identity,
builds the device binary, creates `target/device/<example>.app`, embeds the
profile, signs and verifies the bundle, then installs and launches it with the
console attached. If multiple identities match, use `security find-identity -v
-p codesigning` and set `SIGNING_IDENTITY` to the desired certificate's SHA-1
fingerprint. No private key is exported. Generated profiles/signatures remain
under ignored `target/`; do not commit signing assets or local identifiers.
The script masks device selectors and certificate fingerprints in its echoed
commands and command-failure messages. Apple tool diagnostics and app output
are passed through: inspect captured logs before sharing. The generated `.app`
contains the embedded profile (including provisioned device IDs and team
metadata), so it is a private development artifact even though Git ignores it.
Signing-file types and local `.env` files are also ignored as a safeguard.
An explicit bundle ID is reused across examples and replaces that installed
app, so use an ID dedicated to Mel demos.

**Ctrl-C stops the device app** (devicectl forwards signals). Relaunch from
its icon or rerun the command. If launch fails, check that the phone is
unlocked, Developer Mode is enabled, and any developer trust prompt is resolved
in Settings → General → VPN & Device Management. Expired/mismatched profiles
need renewal in Xcode, not changes to Mel. Free/personal-team profiles may
expire sooner; the script validates the actual expiration date. This flow is
for development installs, not App Store/TestFlight export.

For the timing sanity check, run the same scene in portrait with the **same
build mode** on both targets. Both runners default to debug; release is optional:

```sh
BUILD_MODE=release make run-device
BUILD_MODE=release make run EXAMPLE=006_frame_timing
```

Record model, OS, build mode, logged viewport/scale and refresh interval. Let
the first sample warm up, then collect at least 30 of the once-per-second CPU
and deadline samples without touching/rotating/backgrounding the app. Check the
four-second motion cycle, tap pause/resume, rotation and Home/reopen separately.
These are sampled CPU **submission** costs, not every frame, a release benchmark
or on-screen presentation latency. Simulator and phone sizes/refresh rates may
differ; do not interpret the ratio as a device speedup. See
[ADR 009](../DECISIONS/009-real-device.md) for measured results and the CPU/Metal
and classic-lifecycle decisions.
