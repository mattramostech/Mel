//! Native-free touch events (ADR 007).

use std::time::Duration;

/// One contact's identity, stable from Down through Up/Cancel. The shell
/// never reuses an ID, even if the OS reuses a native touch object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointerId(pub u64);

/// The lifetime of a touch contact; Cancel ends it without a successful Up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
    Cancel,
}

/// Logical points from the full viewport's top-left (x right, y down).
/// Safe-area insets are not subtracted; positions are not clamped. This
/// bootstrap carrier will be absorbed by foundation's Offset (010).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerPosition {
    pub x: f64,
    pub y: f64,
}

/// One changed contact, delivered on the main thread independently of
/// drawing. There is no ordering guarantee between simultaneous contacts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub id: PointerId,
    pub phase: PointerPhase,
    pub position: PointerPosition,
    /// Monotonic platform time of the sample (on iOS, seconds since system
    /// startup). Subtract samples for elapsed time; not wall time/Instant.
    pub timestamp: Duration,
}

#[cfg(any(target_os = "ios", test))]
#[derive(Default)]
pub(crate) struct PointerTracker {
    active: std::collections::HashMap<usize, PointerId>,
    last_id: u64,
}

#[cfg(any(target_os = "ios", test))]
impl PointerTracker {
    /// `native` is an identity key, never an address to dereference. Only
    /// Down creates contacts; duplicate Down and orphan events are ignored.
    pub(crate) fn event(
        &mut self,
        native: usize,
        phase: PointerPhase,
        position: PointerPosition,
        timestamp: Duration,
    ) -> Option<PointerEvent> {
        let id = match phase {
            PointerPhase::Down => {
                if self.active.contains_key(&native) {
                    return None;
                }
                self.last_id = self.last_id.checked_add(1).expect("pointer IDs exhausted");
                let id = PointerId(self.last_id);
                self.active.insert(native, id);
                id
            }
            PointerPhase::Move => *self.active.get(&native)?,
            PointerPhase::Up | PointerPhase::Cancel => self.active.remove(&native)?,
        };
        Some(PointerEvent {
            id,
            phase,
            position,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(tracker: &mut PointerTracker, native: usize, phase: PointerPhase) -> PointerEvent {
        tracker
            .event(native, phase, PointerPosition::default(), Duration::ZERO)
            .unwrap()
    }

    #[test]
    fn simultaneous_contacts_keep_ids_when_delivery_order_changes() {
        let mut tracker = PointerTracker::default();
        let a = sample(&mut tracker, 100, PointerPhase::Down).id;
        let b = sample(&mut tracker, 200, PointerPhase::Down).id;
        assert_ne!(a, b);
        assert_eq!(sample(&mut tracker, 200, PointerPhase::Move).id, b);
        assert_eq!(sample(&mut tracker, 100, PointerPhase::Move).id, a);
        assert_eq!(sample(&mut tracker, 100, PointerPhase::Up).id, a);
        assert_eq!(sample(&mut tracker, 200, PointerPhase::Move).id, b);
        assert_eq!(sample(&mut tracker, 200, PointerPhase::Up).id, b);
        assert!(tracker.active.is_empty());
    }

    #[test]
    fn cancel_and_native_address_reuse_do_not_affect_other_contacts() {
        let mut tracker = PointerTracker::default();
        let a = sample(&mut tracker, 100, PointerPhase::Down).id;
        let b = sample(&mut tracker, 200, PointerPhase::Down).id;
        assert_eq!(sample(&mut tracker, 100, PointerPhase::Cancel).id, a);
        let c = sample(&mut tracker, 100, PointerPhase::Down).id;
        assert!(c > a && c > b);
        assert_eq!(sample(&mut tracker, 200, PointerPhase::Move).id, b);
        assert_eq!(sample(&mut tracker, 100, PointerPhase::Up).id, c);
        let d = sample(&mut tracker, 100, PointerPhase::Down).id;
        assert!(d > c);
    }

    #[test]
    fn duplicate_down_and_orphan_events_are_ignored() {
        let mut tracker = PointerTracker::default();
        for phase in [PointerPhase::Move, PointerPhase::Up, PointerPhase::Cancel] {
            assert!(
                tracker
                    .event(100, phase, PointerPosition::default(), Duration::ZERO)
                    .is_none()
            );
        }
        let id = sample(&mut tracker, 100, PointerPhase::Down).id;
        assert!(
            tracker
                .event(
                    100,
                    PointerPhase::Down,
                    PointerPosition::default(),
                    Duration::ZERO
                )
                .is_none()
        );
        assert_eq!(sample(&mut tracker, 100, PointerPhase::Cancel).id, id);
        assert!(
            tracker
                .event(
                    100,
                    PointerPhase::Up,
                    PointerPosition::default(),
                    Duration::ZERO
                )
                .is_none()
        );
        assert!(tracker.active.is_empty());
    }

    #[test]
    fn events_preserve_sample_coordinates_phase_and_time() {
        let mut tracker = PointerTracker::default();
        let id = sample(&mut tracker, 100, PointerPhase::Down).id;
        for phase in [PointerPhase::Move, PointerPhase::Up] {
            let position = PointerPosition { x: -3.5, y: 900.25 };
            let timestamp = Duration::from_secs_f64(1234.125);
            assert_eq!(
                tracker.event(100, phase, position, timestamp),
                Some(PointerEvent {
                    id,
                    phase,
                    position,
                    timestamp,
                })
            );
        }
    }
}
