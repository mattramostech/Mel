# 011 — Rect

## Flutter reference

Flutter's [Rect](https://api.flutter.dev/flutter/dart-ui/Rect-class.html)
stores four axis-aligned edges. It supplies the bounds needed for later layout,
clipping and hit testing, independently of a renderer. Named constructors
derive edges from extents or a center; `Offset & Size` constructs bounds at an
origin. Edges are retained even when width or height is negative.

[Intersection](https://api.flutter.dev/flutter/dart-ui/Rect/intersect.html)
uses the inner edges, preserving inverted results for disjoint inputs.
[Expansion](https://api.flutter.dev/flutter/dart-ui/Rect/expandToInclude.html)
uses the outer edges, including the coordinates of empty inputs.
[Containment](https://api.flutter.dev/flutter/dart-ui/Rect/contains.html)
includes top/left and excludes bottom/right. These value semantics matter;
Dart object identity, nullable APIs and debug-only checks do not carry over.

The [overlap implementation](https://api.flutter.dev/flutter/dart-ui/Rect/overlaps.html)
only rejects separated edges. An empty or inverted rectangle inside a larger
one can therefore report overlap; NaN comparisons can also fall through to
true. Mel deliberately requires positive intersection extents instead.

## Decision

Calling code designed before implementation:

```rust
use mel_foundation::{Offset, Rect, Size};

let bounds = Offset::new(12.0, 20.0) & Size::new(100.0, 80.0);
let clip = Rect::from_center(Offset::new(100.0, 80.0), 40.0, 40.0);
assert_eq!(bounds.intersect(clip), Rect::from_ltrb(80.0, 60.0, 112.0, 100.0));
assert!(bounds.contains(bounds.center()));
assert!(!bounds.overlaps(Rect::from_ltwh(20.0, 30.0, 0.0, 10.0)));
```

One dependency-free `Copy` struct with public `f64` fields
(`left/top/right/bottom`), const constructors, `ZERO`, `Default`, `Debug` and
exact `PartialEq`. Ownership and numeric policy follow ADR 010: callers own
values, operations return new values, and every `f64` is accepted.
`from_origin_size` is the named equivalent of `Offset & Size`; `from_ltrb`,
`from_ltwh` and `from_center` do not normalize or reject inverted bounds.

Width/height/size and top-left/center/bottom-right accessors connect Rect with
010. `shift`/`translate` move all edges; `inflate`/`deflate` move opposite edges
apart/together and may cross them. `union` is the bounding expansion operation
(Flutter's `expandToInclude`), not an exact union of two sets. No duplicate
alias, implicit conversion or generic geometry abstraction is needed.

**Empty/inverted policy:** `is_empty` is `left >= right || top >= bottom`,
matching Flutter. Intersections retain zero or negative extents rather than
collapsing to `ZERO`. Union uses edge extrema even for empty inputs, so
`ZERO` is not an identity and inverted inputs are not implicitly normalized.
Containment remains half-open. Overlap requires strictly positive intersection
extents: touching edges/corners and empty/inverted inputs never overlap.
This last rule is an intentional correction of Flutter's degenerate cases.

**Non-finite policy:** extrema explicitly propagate NaN on each affected edge,
rather than Rust's `min/max` discarding it. NaN bounds or points never contain
or overlap; NaN alone does not imply emptiness. `has_nan` distinguishes that
case. `is_finite` checks all edges; `is_infinite` recognizes either sign, as in
010. Infinite bounds use the same comparisons (including the half-open edge
rule); derived arithmetic follows IEEE-754, including undefined centers or
extents becoming NaN. Equal infinite edges are empty even if subtraction
would produce NaN. No sentinel "largest" rectangle is introduced.

## Alternatives considered

- Normalize every constructor or clamp intersections: loses inverted edge
  information and silently turns malformed bounds into usable geometry.
- Copy Flutter's overlap behavior verbatim: contradicts nonzero-area overlap
  for empty/inverted inputs and accepts invalid NaN geometry.
- Skip empty operands in union: changes Flutter's bounds expansion and makes
  a point rectangle useless for expanding an accumulator.
- Add the rest of Flutter's Rect conveniences now: circle/point constructors,
  all edge anchors and interpolation can be added when a feature needs them.

## Consequences

The foundation layer gains bounds without adding a dependency or changing
painting/platform. Rasterization and layout remain responsible for validating
their own inputs. A host example demonstrates the public contract; an iOS UI
demo is unnecessary for pure geometry. Existing debts 002-1/008-1 are unaffected.

## Review outcome

Pending human review. Implementation self-check: Correctness ✅ ·
Architecture ✅ · API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No new debt proposed.

Validation on 2026-09-26:

- `make check` passed: formatting, host and platform Simulator all-targets
  clippy with warnings denied, 45 workspace tests (11 new Rect contract tests),
  two geometry doctests, two motion tests, 12 signing tests and shell syntax.
- Rect tests cover const construction and `Offset & Size`, accessors,
  translation, inflation/deflation, touching/disjoint intersections, bounding
  union with empty/inverted inputs, signed zero, infinities and NaN on every
  edge. A sampled-point oracle checks all 6,561 pairs of rectangles whose
  edges are in `{-1, 0, 1}`, including degenerate orderings.
- `cargo clippy --target aarch64-apple-ios-sim -p mel-foundation --all-targets
  -- -D warnings` and `cargo build --locked --target aarch64-apple-ios
  -p mel-foundation --lib` passed. Foundation still has no dependencies.
- `cargo run -p mel-foundation --example 011_rect` passed its assertions;
  example 010 also ran successfully. This pure-logic feature needs no iOS UI
  launch or physical-device testing.
- Privacy review covered the working diff, empty staged diff, all 58 tracked
  working files and four nonignored new files. Gitleaks 8.30.1 used the repository
  config, `--redact=100` and `--ignore-gitleaks-allow` on staging and a complete
  working snapshot: no findings. Separate metadata/filename checks found only
  synthetic signing fixtures, generic Simulator references and the public
  repository URL. No binary files or symlinks were present in the snapshot;
  ignore coverage includes private build/signing/log artifacts. No cleanup was
  needed. Audit/test logs stayed outside the working tree. History, remote
  refs, CI/release artifacts and ignored local outputs were not audited.
