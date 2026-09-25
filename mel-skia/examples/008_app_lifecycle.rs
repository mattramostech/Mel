//! 008 — Background/foreground, interruptions and memory warnings.
//! Run: `scripts/run-sim.sh 008_app_lifecycle` (or `make run`).
//! Go Home and reopen: frame counts stop in background, motion restarts.
//! A memory warning changes the background color; see README for LLDB steps.

#[cfg(target_os = "ios")]
fn main() -> ! {
    use mel_platform::{App, AppLifecycleState, Frame, FrameTiming, PointerEvent};
    use std::time::Duration;

    #[derive(Default)]
    struct Demo {
        origin: Option<Duration>,
        last_log: Option<Duration>,
        frames: u64,
        warnings: u64,
    }

    impl App for Demo {
        fn draw(&mut self, frame: &mut Frame) {
            self.frames += 1;
            let time = frame.time().timestamp;
            let origin = *self.origin.get_or_insert(time);
            let phase = time.saturating_sub(origin).as_secs_f64() % 4.0 / 4.0;
            let progress = (1.0 - (phase * std::f64::consts::TAU).cos()) * 0.5;
            let insets = frame.safe_area();
            let width = (frame.logical_width() - insets.left - insets.right).max(0.0);
            let height = (frame.logical_height() - insets.top - insets.bottom).max(0.0);
            let side = 80.0_f64.min(width).min(height);
            let rect = [
                insets.left + (width - side) * progress,
                insets.top + (height - side) * 0.5,
                side,
                side,
            ]
            .map(|value| (value * frame.scale_factor()) as f32);
            let (width, height) = (frame.physical_width(), frame.physical_height());
            let color = if self.warnings.is_multiple_of(2) {
                [1.0, 0.757, 0.027, 1.0]
            } else {
                [0.4, 0.8, 0.65, 1.0]
            };
            mel_painting::fill(frame.pixels_mut(), width, height, color);
            mel_painting::fill_rect(
                frame.pixels_mut(),
                width,
                height,
                rect,
                [0.306, 0.204, 0.180, 1.0],
            );
            mel_platform::request_frame();
        }

        fn on_lifecycle_changed(&mut self, state: AppLifecycleState) {
            println!(
                "lifecycle {state:?} | frames={} | warnings={}",
                self.frames, self.warnings
            );
            if state == AppLifecycleState::Active {
                self.origin = None;
                self.last_log = None;
            }
            // Deliberately request in every state: suspension must keep
            // the link paused even when an app callback asks for work.
            mel_platform::request_frame();
        }

        fn on_memory_warning(&mut self) {
            self.warnings += 1;
            println!("memory warning {} | frames={}", self.warnings, self.frames);
            // No app cache exists in this demo; a real app releases its
            // disposable resources here. Color shows callback delivery.
            mel_platform::request_frame();
        }

        fn on_pointer_event(&mut self, event: PointerEvent) {
            println!("pointer {} {:?}", event.id.0, event.phase);
        }

        fn on_frame_timing(&mut self, timing: FrameTiming) {
            let timestamp = timing.frame_time.timestamp;
            if self
                .last_log
                .is_none_or(|last| timestamp.saturating_sub(last) >= Duration::from_secs(1))
            {
                self.last_log = Some(timestamp);
                println!(
                    "frames={} | CPU {:.2} ms / interval {:.2} ms | overrun {:.2} ms",
                    self.frames,
                    timing.cpu_duration.as_secs_f64() * 1000.0,
                    timing.frame_time.interval().as_secs_f64() * 1000.0,
                    timing.deadline_overrun.as_secs_f64() * 1000.0
                );
            }
        }
    }

    mel_platform::run(Demo::default())
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!(
        "008_app_lifecycle is an iOS demo; launch it with: scripts/run-sim.sh 008_app_lifecycle"
    );
}
