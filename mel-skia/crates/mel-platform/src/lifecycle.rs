//! Native-free lifecycle and frame eligibility (ADR 008).

/// App visibility/interaction state. Drawing is allowed only while Active.
/// This is Mel's small lifecycle model, not a UIKit type or Flutter's full enum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppLifecycleState {
    /// In the foreground and available for interaction.
    Active,
    /// Launching, entering the foreground, or temporarily interrupted.
    #[default]
    Inactive,
    /// In the background; execution may be suspended by the OS.
    Background,
}

#[cfg(any(target_os = "ios", test))]
#[derive(Default)]
pub(crate) struct FrameLoop {
    state: AppLifecycleState,
    pending: bool,
}

#[cfg(any(target_os = "ios", test))]
impl FrameLoop {
    pub(crate) fn is_active(&self) -> bool {
        self.state == AppLifecycleState::Active
    }

    pub(crate) fn wants_tick(&self) -> bool {
        self.is_active() && self.pending
    }

    pub(crate) fn request(&mut self) {
        self.pending = true;
    }

    pub(crate) fn transition(&mut self, state: AppLifecycleState) -> bool {
        if self.state == state {
            return false;
        }
        self.state = state;
        // Repaint even static apps: the system may have discarded contents,
        // or the viewport may have changed without a layout request.
        if self.is_active() {
            self.request();
        }
        true
    }

    pub(crate) fn take_frame(&mut self) -> bool {
        if !self.wants_tick() {
            return false;
        }
        self.pending = false;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use AppLifecycleState::*;

    #[test]
    fn launch_and_resume_repaint_static_apps_once() {
        let mut frames = FrameLoop::default();
        assert!(!frames.take_frame());
        assert!(frames.transition(Active));
        assert!(frames.take_frame());
        assert!(!frames.take_frame());
        assert!(!frames.transition(Active));
        assert!(!frames.wants_tick());
        for state in [Inactive, Background, Inactive] {
            assert!(frames.transition(state));
            assert!(!frames.take_frame());
        }
        frames.transition(Active);
        assert!(frames.take_frame());
        assert!(!frames.wants_tick());
    }

    #[test]
    fn suspension_blocks_stray_ticks_and_callback_requests_until_active() {
        let mut frames = FrameLoop::default();
        frames.request(); // Launch layout before activation.
        assert!(!frames.take_frame());
        frames.transition(Active);
        assert!(frames.take_frame());
        frames.request(); // Animation requests the next frame.
        for state in [Inactive, Background, Inactive] {
            frames.transition(state);
            frames.request(); // Lifecycle / memory warning callback.
            frames.request();
            assert!(!frames.wants_tick());
            assert!(!frames.take_frame());
            assert!(frames.pending); // A queued tick must not consume it.
        }
        frames.transition(Active);
        assert!(frames.take_frame());
        assert!(!frames.take_frame());
    }

    #[test]
    fn interruption_without_background_and_draw_rerequests_work() {
        let mut frames = FrameLoop::default();
        frames.transition(Active);
        for _ in 0..3 {
            assert!(frames.take_frame());
            assert!(!frames.wants_tick());
            frames.request();
            frames.request();
        }
        frames.transition(Inactive);
        assert!(!frames.take_frame());
        frames.transition(Active);
        assert!(frames.take_frame());
        assert!(!frames.wants_tick());
    }
}
