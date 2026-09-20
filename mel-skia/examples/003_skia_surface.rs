//! 003 — Skia surface on screen.
//!
//! The first pixels Mel draws itself: the app's `draw` rasterizes a solid
//! honey color through a Skia canvas (`mel_painting::fill`), and the shell
//! presents the buffer as the root layer's contents (CPU raster — ADR 003).
//!
//! Run it with: `scripts/run-sim.sh 003_skia_surface`

#[cfg(target_os = "ios")]
fn main() -> ! {
    struct HoneyApp;
    impl mel_platform::App for HoneyApp {
        fn draw(&mut self, frame: &mut mel_platform::Frame) {
            let (width, height) = (frame.physical_width(), frame.physical_height());
            // Honey (#FFC107). If the whole screen turns this color, the
            // pixels travelled Skia canvas → BGRA buffer → CGImage → CALayer.
            mel_painting::fill(frame.pixels_mut(), width, height, [1.0, 0.757, 0.027, 1.0]);
        }
    }
    mel_platform::run(HoneyApp)
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!(
        "003_skia_surface is an iOS demo; launch it with: scripts/run-sim.sh 003_skia_surface"
    );
}
