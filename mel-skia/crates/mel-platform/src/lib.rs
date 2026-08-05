//! Platform layer: iOS app shell, display link, raster surface, touch input.
//!
//! The shell is a pure-Rust UIKit app via `objc2` (ADR 002). UIKit types
//! never leak upward (CONTEXT.md §5): layers above see only Mel's own types.

#[cfg(target_os = "ios")]
mod ios;

/// Hands the calling thread to UIKit and runs the app. Never returns.
///
/// Must be called from `main`, on the main thread. For now (feature 002) the
/// app is a blank full-screen window; later features grow this into the
/// drawing/input/lifecycle entry point.
#[cfg(target_os = "ios")]
pub fn run() -> ! {
    ios::run()
}
