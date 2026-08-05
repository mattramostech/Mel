//! Platform layer: iOS app shell, display link, raster surface, touch input.
//!
//! The shell is a pure-Rust UIKit app via `objc2` (ADR 002). UIKit types
//! never leak upward (CONTEXT.md §5): layers above see only Mel's own types.

#[cfg(target_os = "ios")]
mod ios;

/// The pixel buffer for one frame: the app draws into it, the shell presents
/// it (feature 003: exactly once, at launch — the frame loop is feature 005).
///
/// Buffer contract (the painting layer documents the same one on `fill`):
/// 32-bit **BGRA**, premultiplied alpha, tightly packed rows
/// (`stride == width * 4`). Dimensions are the screen's *logical* size in
/// points — mapping to physical pixels (`contentScaleFactor`) is feature 004.
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The whole buffer: `width * height * 4` bytes, initially all zero
    /// (transparent — an untouched frame presents as the window background).
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

/// Hands the calling thread to UIKit and runs the app. Never returns.
///
/// Must be called from `main`, on the main thread. Once the window exists,
/// `draw` runs with the [`Frame`] to fill, and whatever it left there goes on
/// screen. No `Send` bound: the callback is parked and later invoked on this
/// same (main) thread.
#[cfg(target_os = "ios")]
pub fn run(draw: impl FnOnce(&mut Frame) + 'static) -> ! {
    ios::run(Box::new(draw))
}
