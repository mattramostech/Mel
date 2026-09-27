# 013 — Color

## Flutter reference

Flutter's [Color](https://api.flutter.dev/flutter/dart-ui/Color-class.html)
gives drawing and styling a renderer-independent color value. Current Flutter
stores floating-point components and a color space; its integer constructors
accept sRGB bytes. [ARGB export](https://api.flutter.dev/flutter/dart-ui/Color/toARGB32.html)
packs alpha in the high byte, then red, green and blue, rounding normalized
channels to bytes. Mel's roadmap deliberately starts with that smaller,
8-bit representation and defers color-space support.

[Opacity replacement](https://api.flutter.dev/flutter/dart-ui/Color/withOpacity.html)
rounds `255 * opacity` and preserves RGB; Flutter now deprecates it in favor
of floating-point replacement to avoid quantization. [Interpolation](https://api.flutter.dev/flutter/dart-ui/Color/lerp.html)
works on individual channels, permits extrapolation and clamps the result.
Nullable endpoints fade the other color's alpha. The useful subset here is
value semantics and channel interpolation; nullable dispatch, inheritance,
debug-only validation and wide-gamut conversions need not cross into Rust.

## Decision

Calling code designed before implementation:

```rust
use mel_foundation::Color;

let honey = Color::from_argb32(0xffff_c107);
let faded = honey.with_opacity(0.5).unwrap();
assert_eq!(faded.to_argb32(), 0x80ff_c107);
assert_eq!(faded.opacity(), 128.0 / 255.0);
let midpoint = Color::lerp(Color::RED, Color::BLUE, 0.5).unwrap();
assert_eq!(midpoint, Color::from_argb(255, 128, 0, 128));
assert!(honey.with_opacity(f64::NAN).is_none());
```

One dependency-free `Copy` struct with a private `u32` in `0xAARRGGBB`
order. Channels describe straight (unpremultiplied), sRGB-encoded RGB plus
linear alpha, independently of host byte order or any raster buffer format.
`Clone`, `Debug`, `Default`, exact `PartialEq`/`Eq` and `Hash` are derived.
The default is transparent black. Callers own values; helpers return copies,
with no allocation, shared mutation, dependencies or unsafe code.

`from_argb32`/`to_argb32` preserve all 32 bits, including RGB at zero alpha.
`from_argb` takes four `u8` channels, preventing out-of-range integer inputs
instead of wrapping. These constructors, channel getters and `with_alpha`
are const. `from_rgbo` and `with_opacity` share one checked conversion:
finite opacity in `[0, 1]` becomes `round(opacity * 255)`; exact half-byte
ties round upward. Signed zero is accepted. Negative/out-of-range values,
infinities and NaN return `None` in every build. `opacity` reads back the
quantized alpha divided by 255; setting opacity replaces alpha, not multiplies it.

`Color::lerp(a, b, t)` requires finite `t` and returns `Option<Color>`.
For each byte, evaluate `a + (b - a) * t`, clamp to `[0, 255]`, then round
to nearest with half-byte ties upward. Finite extrapolation is allowed,
including extreme factors whose intermediate products overflow: infinities
saturate and equal channels stay equal. Byte differences are evaluated in
`f64`, avoiding unsigned underflow. Non-finite `t` always returns `None`,
even for identical colors. No nullable endpoints: callers can fade explicitly
with `color.with_alpha(0)`. Transparent black is a distinct endpoint and can
darken RGB when interpolated. This is neither premultiplied-alpha nor
linear-light/perceptual interpolation. Quantization occurs on every call;
it is intentionally not the precision of current Flutter's float storage.

The associated constants `TRANSPARENT`, `BLACK`, `WHITE`, `RED`, `GREEN`,
`BLUE`, `YELLOW`, `CYAN` and `MAGENTA` form a small channel-extrema palette.
`GREEN` is full-intensity `0xff00ff00`; these are RGB colors, not Material
swatches or a theme. No duplicate `Colors` type or palette dependency.

## Alternatives considered

- Float channels plus color-space tags now: exceeds 013's ARGB scope. Defer
  wide gamut, HDR, conversion and linear-light interpolation until requested
  by a concrete feature; do not introduce an unused enum.
- Clamp/wrap invalid opacity or panic: would hide input mistakes or make
  validity depend on build mode. Checked floating inputs follow 012's explicit
  domain boundary; every integer color itself is valid.
- Nullable lerp and implicit integer conversions: unnecessary API surface.
  Named packing operations make channel order visible; fading uses an explicit
  alpha-zero copy of the endpoint.
- Migrate bootstrap painting and old demos: 013 needs only foundation logic.
  Paint/Canvas integration belongs to Phase 2, with premultiplication handled
  at the renderer boundary. Compositing, parsing and luminance are also deferred.

## Consequences

Foundation gains exact, hashable colors without Skia or platform changes.
A host demo prints packed colors and checks opacity/interpolation behavior;
no drawing or device launch is needed. The 8-bit precision and sRGB-only domain
are deliberate feature limits, not a claim of full modern Flutter parity.
Existing debts 002-1/008-1 are unaffected; no new debt is proposed.

## Review outcome

Pending human review. Implementation self-check: Correctness ✅ ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No new debt proposed.

Validation on 2026-09-26:

- `make check` passed: formatting, host and platform Simulator all-targets
  clippy with warnings denied, 69 workspace tests (11 new Color contract
  tests), four foundation doctests, two motion tests, 12 signing tests and
  shell syntax.
- Color tests cover packing/channel order, hidden RGB equality/hashing,
  palette values, opacity replacement and half-byte thresholds, all 256 alpha
  round trips, quantization error and invalid inputs. An independent integer
  oracle checks all 65,536 byte pairs at 11 interpolation/extrapolation
  factors (720,896 color samples); separate cases cover transparent endpoints,
  extreme finite factors, unchanged channels and NaN/infinities.
- `cargo test -p mel-foundation --release --test color` also passed, verifying
  the same invalid-input and rounding contract with optimizations enabled.
- `cargo clippy --target aarch64-apple-ios-sim -p mel-foundation --all-targets
  -- -D warnings` and `cargo build --locked --target aarch64-apple-ios
  -p mel-foundation --lib` passed. Foundation remains dependency-free.
- `cargo run -p mel-foundation --example 013_color` passed all assertions:
  half-opacity honey, red/blue interpolation and extrapolation, explicit
  RGB-preserving fade versus transparent black, and invalid-input rejection.
  No iOS UI launch or physical-device testing is needed for this logic feature.
- Privacy review covered the full working diff, empty staged diff, all 66
  tracked working files and four nonignored new files. Gitleaks 8.30.1 used
  the repository config, `--redact=100` and `--ignore-gitleaks-allow` on staging
  and the complete 70-file working snapshot: no findings. Separate metadata
  and filename review found only generic Simulator/SDK references, placeholders
  and synthetic signing fixtures. No binary files or symlinks were present;
  all 12 private artifact ignore probes passed. No cleanup was needed. Raw
  audit/test logs stayed outside the working tree. History, remote refs,
  CI/release artifacts and ignored local outputs were not audited.
