# 009 — Run on a real device

## Flutter reference

Flutter's [code signing tool](https://github.com/flutter/flutter/blob/master/packages/flutter_tools/lib/src/ios/code_signing.dart)
selects a development identity and provisioning profile, then delegates bundle
signing to Xcode. Its [CoreDevice integration](https://github.com/flutter/flutter/blob/master/packages/flutter_tools/lib/src/ios/core_devices.dart)
installs and launches through `xcrun devicectl`, with an attached console.
The essentials are a signed device executable, a profile authorizing the app,
certificate and device, and a trusted device with Developer Mode enabled.
The generated Runner project and Dart debugger are incidental to Mel.

Apple's [entitlements reference](https://developer.apple.com/library/archive/technotes/tn2415/_index.html)
distinguishes profile permissions from the entitlements actually claimed by an
app: wildcard application identifiers must become concrete, and the App ID
prefix need not equal the team ID.

## Decision

Calling interface designed before implementation:

```sh
export DEVICE_ID='<identifier from xcrun devicectl list devices>'
export PROVISIONING_PROFILE='/absolute/path/to/development.mobileprovision'
make run-device                         # defaults to the existing 006 demo
BUILD_MODE=release make run-device      # same mode for Simulator comparisons
```

Keep the pure-Rust executable. Add a small Python standard-library script for
the device flow: read CoreDevice JSON, decode the explicit profile using
`security cms`, validate expiry/device/application/team and match a certificate
to an installed signing identity; build with Cargo; assemble an iPhone `.app`;
embed the profile; sign and verify with `codesign`; install and launch with
`devicectl --console`. Python's plist/JSON support avoids fragile shell parsing
and adds no package dependencies. No certificate, private key, profile or
device identifier belongs in source control. Xcode/the developer portal still
owns the one-time creation and renewal of signing assets.

Device and profile are explicit inputs. A single matching identity is selected
automatically; `SIGNING_IDENTITY` (SHA-1 fingerprint) resolves ambiguity.
`BUNDLE_ID` overrides the per-example default for an explicit App ID profile.
Only minimal development entitlements are claimed; capabilities present in a
profile are not copied wholesale. Bad signing inputs fail before building or
installing. Generated bundles live under ignored `target/device/`; temporary
decoded profiles and device metadata are cleaned up. The console runs without
LLDB; Rust needs no JIT. Ctrl-C is forwarded to the device app by devicectl.

Use **006_frame_timing** as 009's runnable demo, as required by the roadmap;
do not duplicate the same scene in a new Rust example. Both runners support
`BUILD_MODE=debug|release` (debug by default). Compare the same build mode,
orientation and scene, recording OS/model, viewport size and refresh interval.
The existing once-per-second CPU/deadline samples are a sanity check, not a
full-frame distribution or a measurement of display latency.

## Alternatives considered

- Add an Xcode host/staticlib boundary: signing does not require replacing the
  working Rust entry point. Revisit if capabilities or native debugging need it.
- Automatically create profiles or select an arbitrary device/team: adds account
  management and ambiguous deployment targets to a small local development tool.
- Use cargo-xtask: a new compiled tool adds overhead; stdlib plist/JSON handling
  is enough for this macOS-only workflow.
- Build a benchmark/overlay: feature 131 owns performance tooling. Reuse 006.

## Consequences

No library API, dependency or rendering backend changes. Device deployment
requires Xcode with `devicectl` and an iPhone on iOS 17+; the existing Simulator
flow remains available. This is local development signing, not App Store export.
Revisit 002-1 (classic lifecycle) and 008-1 (CPU rendering) against the observed
hardware run before the Phase 0 gate. Do not start Phase 1 until that gate.

## Review outcome

Reviewed and approved by the user on 2026-09-26. Implementation self-check:
Correctness ✅ for automated checks and native deployment (physical
touch/rotation remain manual checks) ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
Existing 002-1 and 008-1 retained with hardware evidence; no new debt proposed.

Validation on 2026-09-25:

- `make check` passed: fmt, host and Simulator all-targets clippy with warnings
  denied, 19 workspace tests, two motion tests, ten signing-contract tests and
  shell syntax. Signing tests cover distinct prefix/team IDs, wildcard/explicit
  App IDs, expired profiles, unprovisioned devices, distribution profiles,
  missing/wrong/ambiguous identities and device prerequisites.
- Device-target all-targets clippy also passed with warnings denied.
- `make run-device` built 006 for `aarch64-apple-ios`, signed and passed
  `codesign --verify --strict`, installed through devicectl on a physical iPhone
  using the locally tested iOS/Xcode pair, and streamed live frame logs.
  No Xcode project or debugger was involved.
- The Simulator flow launched the same debug scene on an iPhone Simulator.
  Visual inspection showed the brown square on honey and successful
  rotation; relaunched in portrait for the timing comparison.
- `BUILD_MODE=release` also built, signed/verified, installed and launched 006
  on the iPhone, and built/installed/launched in the Simulator. Both streamed
  viewport and timing logs; the comparison below uses matching debug builds.

Both runs used portrait, scale 3, debug builds, no debugger. The table uses the
**first 30 once-per-second samples after discarding the initial sample** on
each target; no interaction during those windows. These are not all frames.

| Target | Interval | CPU median | CPU min–max | Sampled deadline overruns |
| --- | --- | --- | --- | --- |
| Physical iPhone | 16.68 ms | 3.07 ms | 2.79–3.31 ms | 0/30 |
| iPhone Simulator | 16.67 ms | 2.64 ms | 2.17–4.05 ms | 0/30 |

Owner-specific hardware, exact device OS/toolchain versions and viewport
details are intentionally omitted from this public record. Retain any detailed
environment inventory privately; these aggregate timings are only a sanity check.

Different screen sizes, OS versions and hardware prevent treating this as a
controlled benchmark. CPU cost includes rasterization/allocation/image copies;
it excludes compositor completion. Initial samples and a separate landscape
Simulator run showed larger costs/overruns; zero in this small sample is not
proof of zero dropped frames or jank. The phone's observed cadence was about
60 Hz; no claim is made about 120 Hz behavior or energy/thermal performance.

The actual device run supports retaining CPU raster for the current minimal
scene (008-1), with a Metal re-evaluation before 020. It also demonstrates that
the classic delegate launches with this SDK/device pair (002-1); it does not
promise future SDK compatibility. Both debts were consciously retained at the
[Phase 0 gate](phase-0-retrospective.md) after user approval of 009.

Security review on 2026-09-26: no actual device identifiers, signing identities,
private keys or access-token literals were found in tracked files or the staged
change. Tests contain synthetic signing data. Mask device selectors and signing
fingerprints in the runner's command echoes/errors; add regression tests and
ignore rules for signing assets/local environment files. Native tool diagnostics
are still passed through, and the generated development bundle necessarily
embeds its provisioning profile. Those local artifacts must not be published.

Follow-up privacy audit: see [SECURITY_REVIEW.md](../../SECURITY_REVIEW.md).
The original review covered credential/device identifiers, not personal email
in commit metadata or owner-specific hardware descriptions. Working-tree
sanitization does not remove those descriptions from existing local history.
