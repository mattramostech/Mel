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
