# Phase 0 — Platform shell retrospective

Gate performed on 2026-09-26 after the user's approval of 009.

## Regression sweep

`make check` passed: host/Simulator clippy with warnings denied, formatting,
19 workspace tests, two motion tests, 12 signing tests and shell syntax.
Rebuilt, installed and launched every distinct Phase 0 demo (002–008) through
`scripts/run-sim.sh` on iPhone 17 Pro / iOS 26.5. Inspected direct Simulator
screenshots: white 002, honey 003/005/007, safe-area square 004, animated
squares 006/008. Logs confirmed a 1206×2622 physical viewport at scale 3,
one initial draw for idle 005, timestamps for 006 and activation for 008.
All apps terminated normally after capture; no application panic observed.
The Simulator emitted an Apple accessibility duplicate-class warning.

This sweep is a launch/render regression check, not a repeat of every manual
interaction. Prior approved ADRs retain rotation, multi-touch, lifecycle and
memory-warning evidence and their observation limits. 009 reuses 006; its
approved hardware deployment/timing evidence remains in ADR 009. No new
hardware benchmark or physical-input claim is made at this gate.

## Debt review

- **002-1:** retain classic lifecycle for the tested SDK/device pair. Foundation
  work does not change window ownership. Migrate before an SDK requiring scenes.
- **008-1:** retain CPU raster through Phase 1. The measured minimal hardware
  scene is adequate for this phase; reconsider Metal before 020's painting API.

Neither debt prevents pure value types in Phase 1. No new debt is introduced.

## Retrospective

The app-owned `App` state and explicit frame requests kept input, lifecycle and
timing additions local. Keeping UIKit and Skia on opposite sides of a tested
pixel contract worked; a Rust executable also proved sufficient for signing
and device deployment without adding an Xcode host project.

The surprises were platform details: a plain view's content scale stayed at
1×, and a timely CPU draw could still miss the display deadline. Future work
should keep units and measurement boundaries explicit. If starting again, we
would document the temporary carriers' migration boundaries earlier: platform
must not gain upward dependencies when foundation types arrive.

Proceed with 010 only. Its own review remains the next feature gate.
