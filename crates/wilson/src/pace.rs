// SPDX-License-Identifier: GPL-3.0-or-later
//! Frame pacing gate for the winit event loop.
//!
//! The loop advances the engine on `RedrawRequested`. But the OS also emits `RedrawRequested`
//! on its own — when the window is shown, resized, or its scale factor changes — and there is
//! typically a **burst** of these right after the window is created (and again when it goes
//! fullscreen). If we advanced the engine on *every* redraw, those spurious redraws would race
//! past our `WaitUntil` timer and play the first frames far too fast. The most visible symptom:
//! the intro screen (`INTRO.SCR`, emitted with a ~4 s hold = 250 ticks × 16 ms) would flash by
//! in well under a second.
//!
//! [`FramePacer`] gates the advance on a deadline, so a frame is held for its full delay no
//! matter how many redraws arrive in between. On a non-due redraw the loop re-presents the last
//! frame (so resizes still repaint) without stepping the animation.

use std::time::{Duration, Instant};

/// Gates engine advance: a frame shown at `frame_start` is held until `frame_start + delay`,
/// regardless of how many redraw requests arrive in the meantime.
#[derive(Debug, Default)]
pub struct FramePacer {
    next_due: Option<Instant>,
}

impl FramePacer {
    /// A fresh pacer — the first frame is always due.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the engine may advance now: always for the very first frame, then only once the
    /// current frame's hold has elapsed.
    pub fn due(&self, now: Instant) -> bool {
        self.next_due.is_none_or(|due| now >= due)
    }

    /// Record that the frame shown at `frame_start` holds for `delay`; returns its deadline
    /// (for the loop's `ControlFlow::WaitUntil`).
    pub fn schedule(&mut self, frame_start: Instant, delay: Duration) -> Instant {
        let due = frame_start + delay;
        self.next_due = Some(due);
        due
    }

    /// The current frame's deadline, if one is scheduled (used to re-arm `WaitUntil` on a
    /// non-due redraw without moving the deadline).
    pub fn deadline(&self) -> Option<Instant> {
        self.next_due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_a_frame_for_its_full_delay_despite_extra_redraws() {
        // Regression (intro flashed by in <1 s): the intro is emitted with a ~4 s hold
        // (250 ticks × 16 ms). Spurious OS redraws (window show/resize/scale at startup) must
        // NOT advance the engine before the hold elapses — only the deadline may.
        let mut p = FramePacer::new();
        let t0 = Instant::now();
        assert!(p.due(t0), "the very first frame always advances");
        p.schedule(t0, Duration::from_millis(4000)); // the intro hold

        assert!(
            !p.due(t0 + Duration::from_millis(50)),
            "a redraw 50 ms into the hold must NOT advance the engine"
        );
        assert!(
            !p.due(t0 + Duration::from_millis(3_999)),
            "still holding at 3.999 s"
        );
        assert!(
            p.due(t0 + Duration::from_millis(4_000)),
            "the next frame advances once the hold elapses"
        );
        assert!(
            p.due(t0 + Duration::from_millis(10_000)),
            "and stays due after the deadline"
        );
    }

    #[test]
    fn deadline_tracks_the_last_schedule() {
        let mut p = FramePacer::new();
        assert_eq!(p.deadline(), None, "no deadline before the first frame");
        let t0 = Instant::now();
        let due = p.schedule(t0, Duration::from_millis(16));
        assert_eq!(due, t0 + Duration::from_millis(16));
        assert_eq!(p.deadline(), Some(due));
    }
}
