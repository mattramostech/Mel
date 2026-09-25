//! 005 — Redraw-on-demand loop.
//!
//! A tap cycles the background color by requesting exactly one frame; when
//! nothing is requested, the display link stays paused and the app burns no
//! CPU. The console tells the whole story: every draw prints a numbered
//! line, so "one tap, one repaint" is checkable by reading the log — and
//! idleness by its silence (plus ~0% CPU for the app's process in Activity
//! Monitor). Rotation still redraws, now through the same request path
//! (004's regression check rides along as the espresso square).
//!
//! Run it with: `scripts/run-sim.sh 005_redraw_on_demand`

#[cfg(target_os = "ios")]
fn main() -> ! {
    use mel_platform::{App, Frame, PointerEvent, PointerPhase};

    /// Background palette the taps cycle through: honey, mint, sky, blossom.
    const PALETTE: [(&str, [f32; 4]); 4] = [
        ("honey", [1.0, 0.757, 0.027, 1.0]),
        ("mint", [0.647, 0.839, 0.655, 1.0]),
        ("sky", [0.506, 0.831, 0.980, 1.0]),
        ("blossom", [0.957, 0.561, 0.694, 1.0]),
    ];

    struct Demo {
        taps: usize,
        draws: usize,
    }

    impl App for Demo {
        fn draw(&mut self, frame: &mut Frame) {
            self.draws += 1;
            let (name, color) = PALETTE[self.taps % PALETTE.len()];
            println!(
                "draw #{}: {name} background at {}×{} px (taps so far: {})",
                self.draws,
                frame.physical_width(),
                frame.physical_height(),
                self.taps,
            );

            let (width, height) = (frame.physical_width(), frame.physical_height());
            mel_painting::fill(frame.pixels_mut(), width, height, color);

            // 004's regression check: a 100-logical-point espresso square
            // pinned to the safe area's top-left corner.
            let scale = frame.scale_factor();
            let insets = frame.safe_area();
            let side = (100.0 * scale) as f32;
            let (x, y) = ((insets.left * scale) as f32, (insets.top * scale) as f32);
            mel_painting::fill_rect(
                frame.pixels_mut(),
                width,
                height,
                [x, y, side, side],
                [0.306, 0.204, 0.180, 1.0],
            );
        }

        fn on_pointer_event(&mut self, event: PointerEvent) {
            if event.phase != PointerPhase::Down {
                return;
            }
            self.taps += 1;
            println!("touch #{} → request_frame()", self.taps);
            // The app decides a touch is worth a repaint — the shell never
            // does (ADR 005). Exactly one draw follows at the next vsync.
            mel_platform::request_frame();
        }
    }

    mel_platform::run(Demo { taps: 0, draws: 0 })
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!(
        "005_redraw_on_demand is an iOS demo; launch it with: scripts/run-sim.sh 005_redraw_on_demand"
    );
}
