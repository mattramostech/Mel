# Mel (Skia) — Project Context & Working Process

Mel is a gradual, feature-by-feature rewrite of Flutter in Rust, using Skia as the
rasterization backend. This document defines **how** the project is built and
**how each stage is evaluated**. The **what** and the **order** live in
[ROADMAP.md](ROADMAP.md).

---

## 1. Mission & Non-Goals

**Mission:** Rebuild Flutter's architecture (engine → rendering → widgets) in
idiomatic Rust, one small reviewable feature at a time, so that every design,
architecture, and developer-experience decision can be made deliberately and
understood completely.

**Guiding principle:** *Understanding over velocity.* A feature is only "done"
when the person following along can explain why it exists, how it works, and
what trade-offs were made.

**Non-goals (for now):**
- Binary compatibility or API-identical parity with Flutter/Dart.
- Supporting every platform from day one (start with **iOS**; desktop/Android/web later).
- Hot reload, plugins, or the Dart ecosystem.
- Performance records. Correctness and clarity first; optimize when measured.

Flutter is the **reference architecture**, not a spec. When Rust idioms
(ownership, no inheritance, no GC) conflict with Flutter's OOP design, we stop
and make an explicit, recorded decision instead of forcing a translation.

---

## 2. The Feature Loop

Every feature in the roadmap goes through the same five steps. Never skip a
step, never batch multiple features into one pass.

```
┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
│ 1. Study │ → │ 2. Design│ → │ 3. Build │ → │ 4. Demo  │ → │ 5. Review│
└──────────┘   └──────────┘   └──────────┘   └──────────┘   └──────────┘
```

### 2.1 Study
Read how Flutter implements the feature (engine C++ and/or framework Dart).
Write a short summary (a few paragraphs in the feature's ADR, see §4) of:
- What problem the feature solves.
- How Flutter models it (key classes, data flow).
- What is essential vs. incidental (Dart-isms we should not copy).

### 2.2 Design
Decide how it maps to Rust **before writing code**:
- Ownership model: who owns what, where mutation happens.
- Trait vs. enum vs. struct composition (Flutter inheritance rarely maps 1:1).
- Public API sketch: write the *calling code* first (how a user of the crate
  would use it), then design the types to serve it.
- If there is a real fork in the road, list the options and pick one in the ADR.

### 2.3 Build
Implement the **smallest version that satisfies the feature's "Done when"**
criteria from the roadmap. Rules:
- One feature = one branch = one PR-sized diff (target: reviewable in ≤ 30 min,
  roughly ≤ ~500 lines of non-test code; split the feature if it grows past that).
- No speculative generality. Don't add parameters, traits, or hooks for future
  features — the roadmap tells us what's coming; we refactor when we get there.
- Unit tests for logic (geometry math, layout algorithms, tree diffing).
  Rendering features get a runnable example instead of (or in addition to) tests.

### 2.4 Demo
Every feature that has any visible or behavioral effect gets a runnable example
in `examples/` (e.g. `021_canvas_rects.rs`), launched on the iOS Simulator via
the one-command flow decided in feature 002 (pure-logic features may also run
as host-side `cargo run --example`/tests on the Mac). Examples are numbered by
feature ID and never deleted — they are the living history of the project and
double as regression checks ("does example 007 still render correctly?").

### 2.5 Review
The stage gate. See §3. Only after review passes does the roadmap checkbox get
ticked and the next feature begin.

---

## 3. Stage Evaluation — the Review Rubric

Each feature is evaluated against five axes. Score each axis ✅ (good),
⚠️ (acceptable, note the debt), or ❌ (rework before proceeding).

| Axis | Question to answer |
|---|---|
| **Correctness** | Does it behave the way Flutter's semantics say it should (for the subset we implement)? Are edge cases (zero sizes, infinite constraints, empty children) handled or explicitly rejected? |
| **Architecture** | Does it respect layer boundaries (§5)? Does anything in a lower layer know about a higher layer? Is state ownership clear and single-writer? |
| **API ergonomics (DevEx)** | Is the calling code pleasant? Would a Rust developer find it idiomatic, and would a Flutter developer find it familiar? Read the example's `main.rs` out loud — does it feel good? |
| **Understandability** | Can the reviewer explain the implementation without help? Are the tricky parts commented with *why*, not *what*? |
| **Scope discipline** | Did the diff stay inside the feature? No drive-by refactors, no future-proofing? |

Any ❌ blocks progression. ⚠️s are allowed but must be written down in
`DEBT.md` with the feature ID — they are revisited at every phase boundary.

### Phase gates
At the end of each roadmap **phase** (not just each feature), do a wider review:
1. Re-run every example from the phase (and spot-check earlier ones).
2. Read `DEBT.md`; pay down or consciously re-defer each item.
3. Write a short phase retrospective in the phase's section of `DECISIONS/`:
   what we'd design differently, what surprised us.
4. Only then open the next phase.

---

## 4. Decision Records (ADRs)

Every feature gets a markdown file in `DECISIONS/`, named
`NNN-feature-slug.md` (matching the roadmap ID). Template:

```markdown
# NNN — Feature Name

## Flutter reference
How Flutter does it, key classes, links to source.

## Decision
What we built and the ownership/type design chosen.

## Alternatives considered
Option A / Option B, and why we rejected them.

## Consequences
What this locks in, what it makes easier/harder later.

## Review outcome
Rubric scores + any debt items filed.
```

Small features may have a short ADR (five lines is fine). The point is that
**no design decision is ever implicit**. When a later feature forces us to
revisit a decision, we update the old ADR with a `## Superseded by NNN` note
instead of rewriting history.

---

## 5. Architecture Ground Rules

Mirroring Flutter's layering, strictly one-directional (each layer may depend
only on layers below it):

```
widgets      (Widget/Element trees, state, build)          ← Phase 6+
rendering    (RenderObject, layout, paint, hit test)       ← Phase 4–5
scheduling   (frame pipeline, vsync, tickers)              ← Phase 3
compositing  (layer tree, scene building)                  ← Phase 3
painting     (Canvas, Paint, Path — wraps Skia)            ← Phase 2
foundation   (geometry, color, keys, diagnostics)          ← Phase 1
platform     (iOS app shell, display link, Metal/raster    ← Phase 0
              surface, touch input — UIKit/winit, per ADR)
```

Rules:
- **Skia never leaks upward.** Only the painting layer touches `skia-safe`
  types directly; everything above uses Mel's own `Canvas`/`Paint`/`Path`
  wrappers. This keeps the door open for a future Impeller-style backend
  (`mel-impeller` exists for exactly that reason).
- **The platform layer never leaks upward** either: widgets never see UIKit/
  winit types, only Mel's own pointer/keyboard event types. This is what makes
  desktop/Android ports later a Phase-0-only job.
- Crate layout follows the layers: one cargo workspace, one crate per layer
  (`mel-foundation`, `mel-painting`, …) *or* one crate with strict modules —
  this itself is an early ADR (feature 001).
- `unsafe` only inside the painting/platform layers, always with a comment
  justifying it.

---

## 6. Repository Conventions

```
mel-skia/
├── CONTEXT.md          ← this file
├── ROADMAP.md          ← feature schedule; the single source of truth for status
├── DEBT.md             ← accepted ⚠️ items, each tagged with a feature ID
├── DECISIONS/          ← one ADR per feature
├── examples/           ← NNN_slug.rs runnable demos, never deleted
└── (crates/ or src/)   ← decided in feature 001
```

- **Status tracking** lives only in ROADMAP.md checkboxes: `[ ]` not started,
  `[~]` in progress, `[x]` done (reviewed). Never mark `[x]` before review.
- Commit messages: `feat(NNN): short description`, `adr(NNN): ...`,
  `debt(NNN): ...`. One feature per commit series. These conventions describe
  how *humans* write commits — they are not an instruction to commit
  automatically (see §7).
- Rust edition 2021+, `cargo fmt` + `cargo clippy -- -D warnings` clean before
  every review.
- All docs, code, comments, and ADRs in **English**.

---

## 7. How to Resume Work (for humans and AI assistants)

At the start of any session:
1. Read ROADMAP.md — find the first `[~]` (in progress) or `[ ]` feature.
2. Read the ADRs of the current phase to reload context on recent decisions.
3. Check `DEBT.md` for anything relevant to the feature at hand.
4. Follow the Feature Loop (§2). Do **one** feature, then stop for review.

Never implement ahead of the roadmap. If a feature turns out to need something
scheduled later, either (a) split off the minimal prerequisite as a new
sub-feature inserted into the roadmap, or (b) stub it and file a debt item —
and record the choice in the ADR.

**Rule for AI agents: never commit.** Do not run `git commit`, `git merge`,
`git push`, or anything else that writes to git history — not even when the
work seems finished, reviewed, or approved. Leave all changes in the working
tree and stop; a human reviews the diff and decides what, when, and how to
commit. The commit-message conventions in §6 exist for the human who commits,
not as permission to commit. Creating a feature branch to work on is fine;
recording history is not.
