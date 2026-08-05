//! 002 — Blank app.
//!
//! The whole demo is one call: hand the process over to the app shell. A
//! blank white full-screen app on the iOS Simulator proves the pure-Rust
//! UIKit shell (ADR 002) boots, creates its window, and runs an event loop.
//!
//! Run it with: `scripts/run-sim.sh 002_blank_app`

#[cfg(target_os = "ios")]
fn main() -> ! {
    // Draw nothing: an untouched frame presents as fully transparent, so the
    // shell's white window background is all there is — the blank app.
    // (`run` grew its draw-callback parameter in feature 003.)
    mel_platform::run(|_frame| {})
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("002_blank_app is an iOS demo; launch it with: scripts/run-sim.sh 002_blank_app");
}
