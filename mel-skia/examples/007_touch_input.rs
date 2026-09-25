//! 007 — Touch input plumbing. Each active contact paints a 60-point square
//! and logs Mel events with stable IDs. Try a two-finger pinch in Simulator
//! (hold Option while dragging). Releasing removes both squares; a later
//! gesture receives new IDs. No gestures or hit testing are implemented.
//!
//! Run it with: `scripts/run-sim.sh 007_touch_input`

#[cfg(target_os = "ios")]
fn main() -> ! {
    use std::collections::BTreeMap;

    use mel_platform::{App, Frame, PointerEvent, PointerId, PointerPhase, PointerPosition};

    const COLORS: [[f32; 4]; 4] = [
        [0.306, 0.204, 0.180, 1.0],
        [0.1, 0.4, 0.8, 1.0],
        [0.8, 0.1, 0.35, 1.0],
        [0.1, 0.5, 0.25, 1.0],
    ];

    #[derive(Default)]
    struct Demo {
        contacts: BTreeMap<PointerId, PointerPosition>,
    }

    impl App for Demo {
        fn draw(&mut self, frame: &mut Frame) {
            let (width, height) = (frame.physical_width(), frame.physical_height());
            let scale = frame.scale_factor();
            mel_painting::fill(frame.pixels_mut(), width, height, [1.0, 0.757, 0.027, 1.0]);
            for (id, position) in &self.contacts {
                let rect = [position.x - 30.0, position.y - 30.0, 60.0, 60.0]
                    .map(|value| (value * scale) as f32);
                mel_painting::fill_rect(
                    frame.pixels_mut(),
                    width,
                    height,
                    rect,
                    COLORS[((id.0 - 1) % COLORS.len() as u64) as usize],
                );
            }
        }

        fn on_pointer_event(&mut self, event: PointerEvent) {
            match event.phase {
                PointerPhase::Down | PointerPhase::Move => {
                    self.contacts.insert(event.id, event.position);
                }
                PointerPhase::Up | PointerPhase::Cancel => {
                    self.contacts.remove(&event.id);
                }
            }
            println!(
                "pointer #{} {:?} at ({:.1}, {:.1}) pt | {:.3}s | active={}",
                event.id.0,
                event.phase,
                event.position.x,
                event.position.y,
                event.timestamp.as_secs_f64(),
                self.contacts.len(),
            );
            mel_platform::request_frame();
        }
    }

    println!("Touch/drag to inspect pointer events; hold Option for two contacts in Simulator.");
    mel_platform::run(Demo::default())
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("007_touch_input is an iOS demo; launch it with: scripts/run-sim.sh 007_touch_input");
}
