//! 004 — Scale factor, safe areas & rotation.
//!
//! Honey background with a 100-logical-point espresso square anchored at
//! the safe area's top-left corner. On a 3× device the square is 300
//! *physical* pixels — measure a Simulator screenshot to check (that is the
//! feature's "Done when"). Rotate with Cmd+← / Cmd+→: the shell re-renders
//! at the new metrics — the square stays square, the insets move with the
//! notch, and the console prints each new snapshot.
//!
//! Run it with: `scripts/run-sim.sh 004_scale_rotation`

#[cfg(target_os = "ios")]
fn main() -> ! {
    struct ScaleDemo;
    impl mel_platform::App for ScaleDemo {
        fn draw(&mut self, frame: &mut mel_platform::Frame) {
            let (width, height) = (frame.physical_width(), frame.physical_height());
            let scale = frame.scale_factor();
            let insets = frame.safe_area();
            println!(
                "frame: {width}×{height} px = {:.0}×{:.0} pt @ {scale}× · safe area l/t/r/b: \
                 {:.0}/{:.0}/{:.0}/{:.0} pt",
                frame.logical_width(),
                frame.logical_height(),
                insets.left,
                insets.top,
                insets.right,
                insets.bottom,
            );

            // Honey (#FFC107) background, then a 100-logical-point square —
            // drawn in the buffer's own unit, physical pixels.
            mel_painting::fill(frame.pixels_mut(), width, height, [1.0, 0.757, 0.027, 1.0]);
            let side = (100.0 * scale) as f32;
            let (x, y) = ((insets.left * scale) as f32, (insets.top * scale) as f32);
            // Espresso (#4E342E), sitting exactly at the safe area's corner.
            mel_painting::fill_rect(
                frame.pixels_mut(),
                width,
                height,
                [x, y, side, side],
                [0.306, 0.204, 0.180, 1.0],
            );
        }
    }
    mel_platform::run(ScaleDemo)
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!(
        "004_scale_rotation is an iOS demo; launch it with: scripts/run-sim.sh 004_scale_rotation"
    );
}
