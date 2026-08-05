# 001 — Workspace, crate layout & iOS toolchain

## Flutter reference

Flutter splits its stack across two repos and two languages: the **engine**
(C++: Skia/Impeller rasterization, platform embedders, `dart:ui` bindings) and
the **framework** (Dart: a single package, `packages/flutter`, layered as
libraries — `foundation`, `painting`, `scheduler`, `rendering`, `widgets` —
where each library only imports the ones below it). The layering is enforced
by convention and code review, not by the toolchain: nothing in Dart stops
`painting` from importing `widgets`.

Essential to keep: the strict one-directional layering itself, and the split
between "wraps the rasterizer" (engine/`dart:ui`) and "pure framework logic".
Incidental: the two-language/two-repo split (an artifact of Dart-over-C++),
and single-package packaging (an artifact of Dart's library system).

## Decision

**One Cargo workspace at `mel-skia/`, one crate per layer** under `crates/`:

```text
mel-foundation   → (nothing)
mel-painting     → foundation            (only crate allowed to touch skia-safe)
mel-compositing  → painting, foundation
mel-scheduling   → compositing and below
mel-rendering    → scheduling and below
mel-widgets      → rendering and below
mel-platform     → bottom of the app stack; the shell that hosts everything
                   (its exact edges are settled by the 002 embedding ADR)
```

Crates start **empty and with zero dependency edges**. An edge is added to a
crate's `[dependencies]` only when a feature first needs it (all path deps are
pre-declared once in `[workspace.dependencies]`, so adding an edge is one
line). Cargo rejects dependency cycles, so the moment edges exist, the
one-directional rule from CONTEXT.md §5 is enforced by the compiler instead of
by review. `cargo tree --invert skia-safe` will later give a mechanical check
that Skia stays inside `mel-painting`.

Toolchain is pinned via `rust-toolchain.toml`: stable channel, edition 2024,
with `rustfmt`, `clippy`, and the `aarch64-apple-ios` +
`aarch64-apple-ios-sim` targets — so a fresh clone (and CI) gets the right
toolchain automatically. CI (GitHub Actions, macOS runner) runs `cargo fmt
--check`, `clippy -D warnings`, `cargo test` on the host, and `cargo build`
for both iOS targets.

## Alternatives considered

- **Single crate with modules** — less manifest boilerplate and easier
  cross-layer refactors, but layering stays convention-only (any module can
  `use` any other), exactly the weakness of Flutter's approach we can fix for
  free. Rejected: compiler-enforced boundaries are worth seven small
  `Cargo.toml`s, and CONTEXT.md §5 already names the crates.
- **Fewer crates now, split later** (e.g. just `mel-platform` +
  `mel-foundation` for Phases 0–1) — deferred work with no real savings; an
  empty crate costs ~10 lines. Creating the full skeleton also records the
  target architecture in code, not just prose.
- **Nightly toolchain** — nothing in the roadmap needs nightly features;
  stable keeps the contributor story simple.

## Consequences

- Layer violations become compile errors (cycles) or one-line-diff review
  items (a new `[dependencies]` edge is impossible to miss in a PR).
- Skia stays quarantined in `mel-painting` mechanically, keeping the
  `mel-impeller` door open.
- Cost accepted: per-crate manifests, and moving code across layers later
  means moving files between crates rather than between modules.
- `examples/` (numbered demos) will attach to a host crate once feature 002
  decides the app-shell shape; nothing to attach them to yet.

## Review outcome

Reviewed and approved 2026-08-05. Correctness ✅ · Architecture ✅ ·
API ergonomics ✅ · Understandability ✅ · Scope discipline ✅.
No debt items filed.

**Amendment (2026-08-05, post-review):** the CI workflow was disabled (kept
commented out in `.github/workflows/ci.yml`). With an empty skeleton and a
single contributor, running CI on every push adds no signal — all checks are
already run locally before each review. Re-enable by uncommenting when it
starts paying for itself (skia-safe build breakage in 003+, or a second
contributor).
