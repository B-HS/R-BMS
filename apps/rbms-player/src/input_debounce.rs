//! LR2's debounce of one key or button (`inputinterval`, OpenLR2 `En_input.cpp:276-292`), measured
//! between the instants each change was reported rather than between polls. The keyboard's lane
//! keys ([`crate::stage`]'s `key_gate`) and the controller's buttons and two-key axes
//! ([`crate::gamepad`]) both go through it, as LR2 puts its keyboard, pad and MIDI through the same
//! `inputinterval`.
//!
//! A press is handed over the instant it comes: nothing a window does ever makes a press late. A
//! release is handed over the instant it comes too, unless the key went down less than the window
//! ago. One that comes inside the window is handed over the instant the window ends if the key
//! still reads up then ([`Lr2Debounce::release_opened_by`]), and not at all if it went back down
//! before: LR2 holds such a key down (state 2) until `inputinterval` has passed since its press. So
//! a switch that chatters as it closes is one press, a tap shorter than the window is let go of
//! where the window ends, and a key let go of and pressed again in a quick run of the same lane is
//! judged where each press came.
//!
//! After a release has been handed over, a press that comes within one of LR2's polls of it is the
//! switch bouncing as it opens, and is never handed over: LR2 reads a key down on the poll after the
//! one that saw it let go as held, not as a new press (state 3 to 2), and its play loop polls no
//! more often than every [`LR2_POLL_GAP`]. The key is the run's to press again once it has been let
//! go of. A window of 0 hands over every change, that bounce included.

use std::time::{Duration, Instant};

/// The shortest time between two of LR2's reads of its keys during play: `ProcGame` returns at once
/// when called sooner (`POLLGAP`, OpenLR2 `Scene04_Play.cpp:1229-1232`). A press this soon after a
/// release is the same poll's bounce.
pub(crate) const LR2_POLL_GAP: Duration = Duration::from_millis(2);

/// A clock a change is reported on: the window's [`Instant`], or the controller layer's own clock
/// in microseconds (`gamepad::PadState::now_us`).
pub(crate) trait ReportedAt: Copy + Ord {
    /// The instant `span` after this one.
    fn after(self, span: Duration) -> Self;
}

impl ReportedAt for Instant {
    fn after(self, span: Duration) -> Self {
        self + span
    }
}

impl ReportedAt for i64 {
    fn after(self, span: Duration) -> Self {
        i64::try_from(span.as_micros()).map_or(i64::MAX, |span_us| self.saturating_add(span_us))
    }
}

/// One key or button: whether it was handed over down, when it was last handed a press, when it
/// was last let go of (handed over, or reported while it was already handed over up), and the state
/// it last reported.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lr2Debounce<T> {
    handed: bool,
    pressed_at: Option<T>,
    released_at: Option<T>,
    reported: bool,
}

impl<T> Default for Lr2Debounce<T> {
    fn default() -> Self {
        Lr2Debounce { handed: false, pressed_at: None, released_at: None, reported: false }
    }
}

impl<T: ReportedAt> Lr2Debounce<T> {
    /// Whether it was last handed over down.
    pub(crate) fn handed(&self) -> bool {
        self.handed
    }

    /// The key reported down (`pressed`) or up at `at`: whether that is handed over now. A report of
    /// the state it last reported is no change and hands nothing over. A release the window keeps
    /// out is remembered, for [`Lr2Debounce::release_opened_by`], which the caller asks first up to
    /// `at`.
    pub(crate) fn offer(&mut self, pressed: bool, at: T, window: Duration) -> bool {
        if pressed == self.reported {
            return false;
        }
        self.reported = pressed;
        match (pressed, self.handed) {
            (true, true) => false,
            (true, false) => {
                if self.released_at.is_some_and(|released| at < released.after(window.min(LR2_POLL_GAP))) {
                    return false;
                }
                self.handed = true;
                self.pressed_at = Some(at);
                true
            }
            (false, false) => {
                self.released_at = Some(at);
                false
            }
            (false, true) => {
                if self.release_opens_at(window).is_some_and(|opens| at < opens) {
                    return false;
                }
                self.handed = false;
                self.released_at = Some(at);
                true
            }
        }
    }

    /// The instant the window ended for a release it kept out, if that is by `until`. Nothing for a
    /// key pressed again before its window ended, or with no release kept out.
    pub(crate) fn release_opened_by(&self, until: T, window: Duration) -> Option<T> {
        if !self.handed || self.reported {
            return None;
        }
        self.release_opens_at(window).filter(|opens| *opens <= until)
    }

    /// Hand a change over at `at` as it is, whatever the window says, and report whether it was
    /// one: the release [`Lr2Debounce::release_opened_by`] found, at the instant its window ended,
    /// or a reading of a controller's turntable, which keeps a window of its own
    /// ([`Lr2Debounce::last_handed_at`]).
    pub(crate) fn hand_over(&mut self, pressed: bool, at: T) -> bool {
        self.reported = pressed;
        if pressed == self.handed {
            return false;
        }
        self.handed = pressed;
        if pressed {
            self.pressed_at = Some(at);
        } else {
            self.released_at = Some(at);
        }
        true
    }

    /// The instant the state it was last handed over in began: its press while it is down, its
    /// release while it is up. `None` before anything was handed over.
    pub(crate) fn last_handed_at(&self) -> Option<T> {
        if self.handed { self.pressed_at } else { self.released_at }
    }

    /// The instant a release of the press last handed over may be handed over from.
    fn release_opens_at(&self, window: Duration) -> Option<T> {
        self.pressed_at.map(|pressed| pressed.after(window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Duration = Duration::from_millis(16);
    const MS: i64 = 1_000;

    #[test]
    fn a_report_of_the_state_already_reported_is_no_change() {
        let mut gate = Lr2Debounce::<i64>::default();
        assert!(gate.offer(true, 0, WINDOW));
        assert!(gate.offer(false, 100 * MS, WINDOW));
        assert!(!gate.offer(true, 101 * MS, WINDOW), "the bounce as the switch opens");
        assert!(!gate.offer(true, 150 * MS, WINDOW), "the same press reported again is still not a press");
        assert!(!gate.handed(), "the key stays let go of");
    }

    #[test]
    fn the_controller_clock_adds_the_window_in_microseconds() {
        assert_eq!(5_i64.after(WINDOW), 5 + 16 * MS);
        assert_eq!(i64::MAX.after(WINDOW), i64::MAX, "the far end of the clock does not wrap");
    }
}
