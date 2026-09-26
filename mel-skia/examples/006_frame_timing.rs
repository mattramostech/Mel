//! 006 — A moving rectangle driven only by display-link timestamps.
//! Run: `scripts/run-sim.sh 006_frame_timing` (or `make run`).
//! Tap to pause/resume. Resuming starts a new four-second cycle.
//! Logs one timing sample per second; CPU cost includes image submission,
//! not the eventual Core Animation composition or on-screen presentation.

#[cfg(any(target_os = "ios", test))]
fn progress(elapsed: std::time::Duration) -> f64 {
    let phase = elapsed.as_secs_f64().rem_euclid(4.0) / 4.0;
    (1.0 - (phase * std::f64::consts::TAU).cos()) * 0.5
}

#[cfg(target_os = "ios")]
fn main() -> ! {
    use mel_platform::{App, Frame, FrameTiming};
    use std::time::Duration;

    struct Demo {
        origin: Option<Duration>,
        elapsed: Duration,
        last_log: Option<Duration>,
        running: bool,
    }

    impl App for Demo {
        fn draw(&mut self, frame: &mut Frame) {
            if self.origin.is_none() {
                println!(
                    "viewport {}×{} physical | {}×{} logical | scale {}",
                    frame.physical_width(),
                    frame.physical_height(),
                    frame.logical_width(),
                    frame.logical_height(),
                    frame.scale_factor(),
                );
            }
            let time = frame.time();
            let origin = *self.origin.get_or_insert(time.timestamp);
            if self.running {
                self.elapsed = time.timestamp.saturating_sub(origin);
            }
            // Metrics changes can redraw while paused; keep the animation
            // position frozen while adapting its travel range to rotation.
            let position = progress(self.elapsed);
            let insets = frame.safe_area();
            let available_width = (frame.logical_width() - insets.left - insets.right).max(0.0);
            let available_height = (frame.logical_height() - insets.top - insets.bottom).max(0.0);
            let side = 80.0_f64.min(available_width).min(available_height);
            let x = insets.left + (available_width - side) * position;
            let y = insets.top + (available_height - side) * 0.5;
            let scale = frame.scale_factor();
            let rect = [x, y, side, side].map(|value| (value * scale) as f32);
            let (width, height) = (frame.physical_width(), frame.physical_height());
            mel_painting::fill(frame.pixels_mut(), width, height, [1.0, 0.757, 0.027, 1.0]);
            mel_painting::fill_rect(
                frame.pixels_mut(),
                width,
                height,
                rect,
                [0.306, 0.204, 0.180, 1.0],
            );
            if self.running {
                mel_platform::request_frame();
            }
        }

        fn on_frame_timing(&mut self, timing: FrameTiming) {
            let time = timing.frame_time;
            if self
                .last_log
                .is_none_or(|last| time.timestamp.saturating_sub(last) >= Duration::from_secs(1))
            {
                self.last_log = Some(time.timestamp);
                println!(
                    "vsync {:.3}s → {:.3}s | CPU {:.2} ms / interval {:.2} ms | deadline overrun {:.2} ms",
                    time.timestamp.as_secs_f64(),
                    time.target_timestamp.as_secs_f64(),
                    timing.cpu_duration.as_secs_f64() * 1000.0,
                    time.interval().as_secs_f64() * 1000.0,
                    timing.deadline_overrun.as_secs_f64() * 1000.0,
                );
            }
        }

        fn on_pointer_event(&mut self, event: mel_platform::PointerEvent) {
            if event.phase != mel_platform::PointerPhase::Down {
                return;
            }
            self.running = !self.running;
            println!(
                "animation {}",
                if self.running { "resumed" } else { "paused" }
            );
            if self.running {
                self.origin = None;
                self.last_log = None;
                mel_platform::request_frame();
            }
            // On pause, the already-pending frame may finish once. It does
            // not re-request: the link then stays idle, preserving ADR 005.
        }
    }

    mel_platform::run(Demo {
        origin: None,
        elapsed: Duration::ZERO,
        last_log: None,
        running: true,
    })
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!(
        "006_frame_timing is an iOS demo; launch it with: scripts/run-sim.sh 006_frame_timing"
    );
}

#[cfg(test)]
mod tests {
    use super::progress;
    use std::time::Duration;

    #[test]
    fn motion_reaches_both_edges_and_repeats_after_four_seconds() {
        for (seconds, expected) in [
            (0, 0.0),
            (1, 0.5),
            (2, 1.0),
            (3, 0.5),
            (4, 0.0),
            (4002, 1.0),
        ] {
            assert!((progress(Duration::from_secs(seconds)) - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn skipped_frames_do_not_slow_animation() {
        let dense: Vec<_> = (0..=240)
            .map(|tick| progress(Duration::from_secs_f64(f64::from(tick) / 120.0)))
            .collect();
        // A 30 Hz display, or a 120 Hz display skipping three of every four
        // frames, reaches exactly the same positions at the same times.
        for tick in 0..=60 {
            let sparse = progress(Duration::from_secs_f64(f64::from(tick) / 30.0));
            assert_eq!(sparse, dense[tick as usize * 4]);
            assert!((0.0..=1.0).contains(&sparse));
        }
    }
}
