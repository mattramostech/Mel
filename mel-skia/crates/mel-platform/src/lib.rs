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
/// for. The app draws into it, the shell presents it. Since feature 005
/// frames exist only on demand: one [`App::draw`] call per granted
/// [`request_frame`], with metrics read fresh at the moment the frame is
/// granted.
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

/// The platform↔app seam: the shell calls in, the app draws and reacts.
/// Implement it once per program and hand it to [`run`].
///
/// Frames are pull-based (ADR 005): the shell never draws on its own clock.
/// Something — the shell on a metrics change, the app via [`request_frame`]
/// — requests a frame, and at the next display refresh the shell calls
/// [`draw`](Self::draw) exactly once and presents the result. No pending
/// request, no callbacks, no CPU.
pub trait App {
    /// Draw one frame into `frame.pixels_mut()`; whatever is left there
    /// goes on screen when this returns. Called once per granted frame
    /// request, with the viewport metrics current at that instant. Calling
    /// [`request_frame`] from inside is allowed and re-arms the loop —
    /// that is how continuous animation works (timestamps arrive in 006).
    fn draw(&mut self, frame: &mut Frame);

    /// A finger touched the screen (UIKit's touch-began). Deliberately
    /// information-free — no position, no pointer ID: it exists so an app
    /// can answer "the user poked the screen" with [`request_frame`].
    /// Feature 007 replaces it with real Mel-typed pointer events.
    fn on_touch(&mut self) {}
}

/// Hands the calling thread to UIKit and runs the app. Never returns.
///
/// Must be called from `main`, on the main thread. The shell requests the
/// first frame itself once the viewport has a size, and again whenever the
/// metrics change — launch, rotation, safe-area moves. Everything else
/// repaints only when `app` asks (see [`request_frame`]). No `Send` bound:
/// the app is parked and always invoked on this same (main) thread.
#[cfg(target_os = "ios")]
pub fn run(app: impl App + 'static) -> ! {
    ios::run(Box::new(app))
}

/// Schedules one [`App::draw`] at the next display refresh.
///
/// Idempotent until granted: any number of requests before the next vsync
/// coalesce into a single draw (Flutter's `scheduleFrame` semantics).
/// Callable from any `App` callback, including `draw` itself (continuous
/// rendering). Main thread only — panics elsewhere. Before the shell is up
/// it is a no-op: the launch layout pass always requests the first frame.
#[cfg(target_os = "ios")]
pub fn request_frame() {
    ios::request_frame()
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
