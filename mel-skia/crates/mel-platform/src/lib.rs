//! Platform layer: iOS app shell, display link, raster surface, touch input.
//!
//! The shell is a pure-Rust UIKit app via `objc2` (ADR 002). UIKit types
//! never leak upward (CONTEXT.md §5): layers above see only Mel's own types.

#[cfg(target_os = "ios")]
mod ios;

/// The screen regions covered by system UI (notch, home indicator, status
/// bar), in **logical points** — the safe area is where content is fully
/// visible. A plain data carrier owned by the platform layer; Phase 1's
/// `EdgeInsets` (feature 014) is the real geometry type and will absorb it
/// at this boundary (ADR 004).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SafeAreaInsets {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// The pixel buffer for one frame plus the viewport metrics it was sized
/// for. The app draws into it, the shell presents it. Since feature 004 the
/// draw callback runs once per *metrics change* — launch, rotation,
/// safe-area change; the redraw-on-demand loop is feature 005.
///
/// Two coordinate spaces are in play (ADR 004), so every dimension names
/// its unit:
///
/// - **physical pixels** — the buffer's grid, what `pixels()` is sized in;
/// - **logical points** — what apps reason in; one point spans
///   [`scale_factor`](Self::scale_factor) physical pixels (Flutter's
///   `devicePixelRatio`; 3.0 on modern iPhones).
///
/// Buffer contract (the painting layer documents the same one on `fill`):
/// 32-bit **BGRA**, premultiplied alpha, tightly packed rows
/// (`stride == physical_width * 4`).
pub struct Frame {
    width: u32,
    height: u32,
    scale: f64,
    safe_area: SafeAreaInsets,
    pixels: Vec<u8>,
}

impl Frame {
    /// Buffer width in physical pixels.
    pub fn physical_width(&self) -> u32 {
        self.width
    }

    /// Buffer height in physical pixels.
    pub fn physical_height(&self) -> u32 {
        self.height
    }

    /// Physical pixels per logical point (Flutter's `devicePixelRatio`).
    pub fn scale_factor(&self) -> f64 {
        self.scale
    }

    /// Viewport width in logical points.
    pub fn logical_width(&self) -> f64 {
        f64::from(self.width) / self.scale
    }

    /// Viewport height in logical points.
    pub fn logical_height(&self) -> f64 {
        f64::from(self.height) / self.scale
    }

    /// Safe-area insets in logical points, for this frame's orientation.
    pub fn safe_area(&self) -> SafeAreaInsets {
        self.safe_area
    }

    /// The whole buffer: `physical_width * physical_height * 4` bytes,
    /// initially all zero (transparent — an untouched frame presents as the
    /// window background).
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

/// Hands the calling thread to UIKit and runs the app. Never returns.
///
/// Must be called from `main`, on the main thread. `draw` runs with a fresh
/// [`Frame`] every time the viewport metrics change — at launch, on
/// rotation, when the safe area moves — and whatever it left there goes on
/// screen. No `Send` bound: the callback is parked and always invoked on
/// this same (main) thread.
#[cfg(target_os = "ios")]
pub fn run(draw: impl FnMut(&mut Frame) + 'static) -> ! {
    ios::run(Box::new(draw))
}

#[cfg(test)]
mod tests {
    use super::{Frame, SafeAreaInsets};

    /// Logical size is derived, not stored: physical ÷ scale. 3× mirrors the
    /// devices the Simulator flow targets.
    #[test]
    fn logical_size_is_physical_over_scale() {
        let frame = Frame {
            width: 300,
            height: 750,
            scale: 3.0,
            safe_area: SafeAreaInsets {
                left: 0.0,
                top: 59.0,
                right: 0.0,
                bottom: 34.0,
            },
            pixels: Vec::new(),
        };
        assert_eq!(frame.physical_width(), 300);
        assert_eq!(frame.physical_height(), 750);
        assert_eq!(frame.logical_width(), 100.0);
        assert_eq!(frame.logical_height(), 250.0);
        assert_eq!(frame.scale_factor(), 3.0);
        assert_eq!(
            frame.safe_area(),
            SafeAreaInsets {
                left: 0.0,
                top: 59.0,
                right: 0.0,
                bottom: 34.0
            }
        );
    }
}
