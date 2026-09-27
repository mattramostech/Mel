# 012 — Radius & RRect

## Flutter reference

Flutter's [Radius](https://api.flutter.dev/flutter/dart-ui/Radius-class.html)
stores the horizontal and vertical radii of an ellipse. RRect combines four
such corners with axis-aligned bounds for future drawing, clipping and hit
testing. [Outer bounds](https://api.flutter.dev/flutter/dart-ui/RRect/outerRect.html)
retain the rectangle without rounding. These are geometry values, independent
of painting; neither Dart inheritance nor cached mutable objects is needed.

[Radius fitting](https://api.flutter.dev/flutter/dart-ui/RRect/scaleRadii.html)
uses one common scale so adjacent radii fit each side, preserving proportions.
[Containment](https://api.flutter.dev/flutter/dart-ui/RRect/contains.html)
first checks the half-open bounds, then an ellipse in a corner region.
Reading that implementation shows that its first-matching-corner branch can
miss an overlapping diagonal corner: in a 100-square with only top-left and
bottom-right radii of 100,
point (80, 80) passes the first ellipse but lies outside the second.
Mel checks every applicable corner.

[Safe inner bounds](https://api.flutter.dev/flutter/dart-ui/RRect/safeInnerRect.html)
inset each side by its largest radius component times `1 - 1/sqrt(2)`.
Mel computes these from fitted radii so the result describes the same shape
as containment, including oversized inputs.

## Decision

Calling code designed before implementation:

```rust
use mel_foundation::{Offset, Radius, Rect, RRect};

let bounds = Rect::from_ltwh(10.0, 20.0, 100.0, 80.0);
let rounded = RRect::from_rect_corners(
    bounds,
    Radius::circular(20.0),        // top-left
    Radius::elliptical(30.0, 10.0), // top-right
    Radius::ZERO,                 // bottom-right
    Radius::circular(8.0),         // bottom-left
);
assert_eq!(rounded.outer_rect(), bounds);
assert!(rounded.contains(Offset::new(60.0, 60.0)));
assert!(!rounded.contains(bounds.top_left()));
let inner = rounded.safe_inner_rect().unwrap();
assert!(inner.contains(bounds.center()));
```

Two dependency-free `Copy` structs with public `f64`-based fields, exact
`PartialEq`, `Default`, `Debug`, `ZERO` and const constructors. Radius stores
`x/y`, with `circular` and `elliptical` constructors. RRect composes
`bounds: Rect` and four named Radius fields. `from_rect_radius` sets all corners;
`from_rect_corners` takes them clockwise from top-left. Values are owned by
the caller; operations return copies, without caches or interior mutation.

Constructors retain all inputs, following 010/011. Geometric queries have an
explicit validity boundary: finite bounds and finite derived extents,
nonnegative extents, and finite nonnegative radius components. `scale_radii`
and `safe_inner_rect` return `None` outside this domain; `contains` returns
false. Invalid geometry is never silently clamped. Zero extents are allowed
for fitting/inner bounds but contain no points. Inverted bounds remain visible
through `outer_rect`. Signed zero is valid. Floating arithmetic can still round
or underflow; overflowing adjacent-radius sums are handled without collapsing
finite radii to zero. Unbounded rounded shapes are explicitly unsupported.

`scale_radii` computes one factor at most one from all four adjacent pairs.
`contains` fits a temporary value, uses Rect's half-open boundary and rejects
points outside any applicable corner ellipse. Arc boundaries are included;
right/bottom outer edges remain excluded. A zero radius component leaves that
corner square (no division by zero). `safe_inner_rect` uses the fitted radii
and the full-precision inset factor; it is a conservative inner rectangle,
not a maximum-area search. No approximate equality is added.

## Alternatives considered

- Reject invalid values in constructors: conflicts with the existing geometry
  carriers and prevents inspection of malformed inputs. Checked queries make
  the usable domain explicit without panics or debug-only validation.
- Clamp negative radii or pretend infinities describe square corners: hides
  invalid geometry. Return `None`/false consistently instead.
- Independently cap corners: changes their proportions and may still violate
  a different side. Use Flutter's common-factor rule.
- Cache normalized radii: introduces mutable state into small value types.
- Copy the first-corner containment branch: accepts points outside large
  diagonal corners; checking all four has fixed, small cost.
- Implement all Radius arithmetic, RRect transforms, interpolation and shape
  classification now: no current consumer requires them. Keep 012 to geometry
  construction, fitting, containment and bounds.

## Consequences

Foundation gains rounded bounds without Skia or platform changes. Later
painting must decide its own validation boundary using this explicit domain.
The host demo draws nothing; rendering waits for Phase 2. Existing debts
002-1/008-1 are unaffected; no new debt is proposed.

## Review outcome

Pending human review. Implementation self-check: Correctness ✅ ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No new debt proposed.

Validation on 2026-09-26:

- `make check` passed: formatting, host and platform Simulator all-targets
  clippy with warnings denied, 58 workspace tests (13 new rounded-geometry
  contract tests), three geometry doctests, two motion tests, 12 signing tests
  and shell syntax.
- New tests cover const construction, all corner positions, circular and
  elliptical boundaries, half-open edges, zero-component corners, all four
  radius-fitting constraints, overflowing radius sums, overlapping diagonal
  regions, invalid values in every radius/edge, empty/inverted bounds and
  fitted inner bounds. An independent horizontal-slice oracle checks 129,600
  containment samples across 108 rounded rectangles; interior samples also
  verify the safe inner bounds.
- `cargo clippy --target aarch64-apple-ios-sim -p mel-foundation --all-targets
  -- -D warnings` and `cargo build --locked --target aarch64-apple-ios
  -p mel-foundation --lib` passed. Foundation remains dependency-free.
- `cargo run -p mel-foundation --example 012_radius_rrect` passed its
  assertions; examples 010/011 also ran successfully. No iOS UI launch or
  physical-device testing is needed for this pure-logic feature.
- Privacy review covered the full working diff, empty staged diff, all 62
  tracked working files and four nonignored new files. Gitleaks 8.30.1 used
  the repository config, `--redact=100` and `--ignore-gitleaks-allow` on staging
  and the complete 66-file working snapshot: no findings. Separate metadata
  and filename checks found only generic Simulator/SDK references, runtime
  identifier expressions, placeholders and synthetic signing fixtures. The
  snapshot contains no binary files or symlinks, and ignore probes cover
  private build/signing/log artifacts. No privacy cleanup was needed. Raw
  audit/test logs stayed outside the working tree. History, remote refs,
  CI/release artifacts and ignored local outputs were not audited.
