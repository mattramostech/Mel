use std::time::Duration;

/// Vsync times from one monotonic, unspecified platform epoch (ADR 006).
/// These are neither wall-clock time nor time since app launch. Subtract
/// timestamps to animate; do not compare them with `Instant`/`SystemTime`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTime {
    /// Time of the display refresh associated with this callback.
    pub timestamp: Duration,
    /// Expected next display refresh, the submission deadline.
    pub target_timestamp: Duration,
}

impl FrameTime {
    /// This frame's refresh interval, not time since the last requested
    /// frame. Recomputed per tick to support variable refresh rates.
    pub fn interval(self) -> Duration {
        self.target_timestamp.saturating_sub(self.timestamp)
    }

    #[cfg(any(target_os = "ios", test))]
    pub(crate) fn from_seconds(timestamp: f64, target_timestamp: f64) -> Option<Self> {
        let timestamp = Duration::try_from_secs_f64(timestamp).ok()?;
        let target_timestamp = Duration::try_from_secs_f64(target_timestamp).ok()?;
        (target_timestamp > timestamp).then_some(Self {
            timestamp,
            target_timestamp,
        })
    }
}

/// CPU submission measurements, delivered after the shell assigns the
/// rendered image to the layer. Not a GPU or on-screen presentation timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTiming {
    pub frame_time: FrameTime,
    /// Tick entry through layer submission, including viewport reads,
    /// allocation, drawing and image copies; excludes the timing callback.
    pub cpu_duration: Duration,
    /// How far submission finished past the target refresh (zero if on
    /// time). Includes callback lateness, even if CPU work fits the interval.
    pub deadline_overrun: Duration,
}

impl FrameTiming {
    #[cfg(any(target_os = "ios", test))]
    pub(crate) fn from_seconds(frame_time: FrameTime, start: f64, end: f64) -> Option<Self> {
        let start = Duration::try_from_secs_f64(start).ok()?;
        let end = Duration::try_from_secs_f64(end).ok()?;
        Some(Self {
            frame_time,
            cpu_duration: end.checked_sub(start)?,
            deadline_overrun: end.saturating_sub(frame_time.target_timestamp),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_follows_refresh_rate_even_after_idle() {
        for (start, interval) in [(10.0, 1.0 / 60.0), (200.0, 1.0 / 120.0)] {
            let time = FrameTime::from_seconds(start, start + interval).unwrap();
            assert!((time.interval().as_secs_f64() - interval).abs() < 1e-9);
            assert_eq!(time.timestamp, Duration::from_secs_f64(start));
        }
    }

    #[test]
    fn invalid_platform_times_are_rejected() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, f64::MAX] {
            assert!(FrameTime::from_seconds(invalid, 2.0).is_none());
            assert!(FrameTime::from_seconds(1.0, invalid).is_none());
        }
        assert!(FrameTime::from_seconds(2.0, 1.0).is_none());
        assert!(FrameTime::from_seconds(1.0, 1.0).is_none());
        assert!(FrameTime::from_seconds(0.0, 0.016).is_some());
    }

    #[test]
    fn submission_cost_and_deadline_are_distinct() {
        let time = FrameTime::from_seconds(1.0, 1.016).unwrap();
        let on_time = FrameTiming::from_seconds(time, 1.002, 1.010).unwrap();
        assert_eq!(on_time.cpu_duration, Duration::from_millis(8));
        assert_eq!(on_time.deadline_overrun, Duration::ZERO);
        let at_deadline = FrameTiming::from_seconds(time, 1.002, 1.016).unwrap();
        assert_eq!(at_deadline.deadline_overrun, Duration::ZERO);
        // A late callback needs only 4 ms of work but misses the deadline.
        let late = FrameTiming::from_seconds(time, 1.014, 1.018).unwrap();
        assert_eq!(late.cpu_duration, Duration::from_millis(4));
        assert_eq!(late.deadline_overrun, Duration::from_millis(2));
        let slow = FrameTiming::from_seconds(time, 1.002, 1.022).unwrap();
        assert!(slow.cpu_duration > time.interval());
        assert_eq!(slow.deadline_overrun, Duration::from_millis(6));
    }

    #[test]
    fn invalid_measurement_samples_are_rejected() {
        let time = FrameTime::from_seconds(1.0, 1.016).unwrap();
        assert!(FrameTiming::from_seconds(time, 2.0, 1.0).is_none());
        assert!(FrameTiming::from_seconds(time, f64::NAN, 1.0).is_none());
        assert!(FrameTiming::from_seconds(time, 1.0, f64::INFINITY).is_none());
    }
}
