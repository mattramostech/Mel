# 010 — Offset & Size

## Flutter reference

Flutter's [Offset](https://api.flutter.dev/flutter/dart-ui/Offset-class.html)
and [Size](https://api.flutter.dev/flutter/dart-ui/Size-class.html) model points,
displacements and extents before any renderer exists. Their
[geometry implementation](https://github.com/flutter/flutter/blob/master/engine/src/flutter/lib/ui/geometry.dart)
shares two doubles through `OffsetBase`. Arithmetic returns new values;
`Size - Size` produces an Offset, while adding/subtracting an Offset resizes
a Size. Containment excludes right/bottom edges. Extents may be negative or
unbounded. Those semantics matter; inheritance, dynamic return types and
nullable interpolation do not need to cross into Rust.

## Decision

Calling code designed before implementation:

```rust
use mel_foundation::{Offset, Size};

let origin = Offset::new(12.0, 20.0);
let size = Size::new(100.0, 80.0);
let center = size.center(origin);
assert!(size.contains(center - origin));
let grown = size + Offset::new(8.0, 4.0);
assert_eq!(grown - size, Offset::new(8.0, 4.0));
```

Two separate `Copy` structs with public `f64` fields (`dx/dy`, `width/height`),
const constructors, `ZERO`/`INFINITE`, `Default`, `Debug`, and `PartialEq`.
No allocation, shared ownership, dependencies or unsafe code. Mutation follows
ordinary Rust bindings; methods consume cheap copies and return new values.

| Operation | Result |
| --- | --- |
| `Offset + Offset`, `Offset - Offset`, `-Offset` | Offset |
| `Size + Offset`, `Size - Offset` | Size |
| `Size - Size` | Offset |
| Either type `* f64`, `/ f64`, `% f64` | Same type |
| `div_trunc(f64)` | Same type; floating quotient rounded toward zero |
| `all_lt/le/gt/ge(Self)` | Both components satisfy the named comparison |

Do not implement `PartialOrd`: Flutter's strict component comparisons cannot
obey Rust's ordering laws when only one component is equal. Named predicates
preserve their meaning, without cross-type comparison/inheritance. `%` uses
[Dart's Euclidean modulo](https://api.dart.dev/dart-core/double/operator_modulo.html)
(nonnegative even for negative operands), explicitly unlike scalar Rust `%`.
The `& Size -> Rect` operator waits for 011; no placeholder Rect is introduced.

`Offset` includes direction/distance, independent scale and translation.
`Size` includes square/radius/one-axis constructors, flipped extents, magnitude
extrema, all nine origin-relative anchor helpers, containment and aspect ratio.
Both support non-null `lerp` with extrapolation; callers choose zero explicitly.

**Numeric policy:** accept every `f64`, including negatives, signed zero,
infinities and NaN, without normalization. `is_finite` requires both finite;
`is_infinite` follows Rust and recognizes either sign (Flutter recognizes only
positive infinity). No `Eq`, `Hash`, or approximate equality. NaN ordered
comparisons are false, NaN bounds/points cannot contain a point, and derived numeric
operations propagate NaN. `is_empty` means either extent is nonpositive; a
NaN extent alone is not classified as empty. Extrema explicitly propagate NaN
instead of Rust's `min/max` discarding it. Aspect ratio matches Flutter for
zero heights (including negative zero), with the deliberate correction that
NaN width stays NaN instead of turning into zero.

Division by zero follows IEEE-754. `div_trunc` remains floating point, so
non-finite quotients propagate rather than taking Dart's integer-conversion
exception path. Distance uses square/sum/square-root as Flutter does, including
overflow/underflow. Lerp uses weighted components, preserves equal endpoints,
and otherwise permits IEEE non-finite results without debug-only assertions.

The host demo belongs to `mel-foundation`, with its file in shared `examples/`.
Do not migrate platform's `PointerPosition` to a foundation dependency: platform
is below foundation. App code may explicitly construct an Offset from it; a
future event boundary migration must respect that layering. No rendering or
platform API changes are needed to satisfy 010.

## Alternatives considered

- Vector crate or shared base trait: more abstraction than two value types need.
- Reject negatives/NaN/infinities in constructors: would prevent unbounded
  extents and push fallibility into every arithmetic operation.
- Lexicographic ordering: sorts vectors but changes geometric comparisons.
- Nullable lerp, assignment operators, conversions and mixed-type comparisons:
  no current callers require these additions.

## Consequences

Foundation now provides the geometry vocabulary for 011 onward, without Skia
or UIKit. Consumers must validate finiteness where their own protocol demands
it (e.g. rasterization); the value types themselves are not layout constraints.
The few intentional Rust differences above are part of the tested contract.

## Review outcome

Pending human review. Implementation self-check: Correctness ✅ ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No new debt proposed; existing 002-1/008-1 were retained at the Phase 0 gate.

Validation on 2026-09-26:

- `make check` passed: formatting, host and platform Simulator all-targets
  clippy with warnings denied, 34 workspace tests (15 new geometry contract
  tests), one geometry doctest, two motion tests, 12 signing tests and shell syntax.
- New tests cover the operator/result-type table, component comparisons with
  one equal axis and crossing axes, constructors, directions, all nine anchors,
  containment boundaries, negative/empty extents, signed zero, both infinity
  signs, NaN propagation, Euclidean modulo and interpolation/extrapolation.
- `cargo clippy --target aarch64-apple-ios-sim -p mel-foundation --all-targets
  -- -D warnings` and `cargo build --locked --target aarch64-apple-ios
  -p mel-foundation --lib` passed. Foundation remains dependency-free.
- `cargo run -p mel-foundation --example 010_offset_size` ran on the Mac and
  passed its assertions: center `(62, 60)`, growth `(8, 4)`, ratio `1.25`,
  interpolation/extrapolation samples and infinite-height containment.
- Phase 0 demos were rerun before opening this phase; see the
  [retrospective](phase-0-retrospective.md). Geometry requires no iOS UI demo.
