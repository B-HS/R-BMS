//! The keyboard's lane keys as a run is handed them: debounced key by key the way LR2 debounces
//! every key and button it reads (`inputinterval`, OpenLR2 `En_input.cpp:276-292`), measured
//! between the instants the window reported each change rather than between polls.
//!
//! A press is handed over the instant it comes: nothing a window does ever makes a press late. A
//! release is handed over the instant it comes too, unless the key went down less than the window
//! ago. One that comes inside the window is handed over the instant the window ends if the key
//! still reads up then, and not at all if it went back down before: LR2 holds such a key down
//! (state 2) until `inputinterval` has passed since its press. So a switch that chatters as it
//! closes is one press, a tap shorter than the window is let go of where the window ends, and a
//! key let go of and pressed again in a quick run of the same lane is judged where each press came.
//!
//! After a release has been handed over, a press that comes within one of LR2's polls of it is the
//! switch bouncing as it opens, and is never handed over: LR2 reads a key down on the poll after
//! the one that saw it let go as held, not as a new press (state 3 to 2), and its play loop polls
//! no more often than every [`LR2_POLL_GAP`]. The key is the run's to press again once it has been
//! let go of. A window of 0 hands over every change, that bounce included.
//!
//! A release kept out is handed over late, by whoever looks next ([`KeyGates::take_opened`]): the
//! next key, the next controller event or the next frame, each of which asks first for every
//! window that has ended before its own instant. It is handed over at the instant its window ended,
//! so the run judges it there whoever noticed.

use std::time::{Duration, Instant};

use winit::keyboard::KeyCode;

/// The shortest time between two of LR2's reads of its keys during play: `ProcGame` returns at once
/// when called sooner (`POLLGAP`, OpenLR2 `Scene04_Play.cpp:1229-1232`). A press this soon after a
/// release is the same poll's bounce.
const LR2_POLL_GAP: Duration = Duration::from_millis(2);

/// One key: whether the run was handed it down, when it was last handed a press, when the key was
/// last let go of (handed over, or reported while the run already held it up), and the state the
/// window last reported.
#[derive(Clone, Copy, Debug, Default)]
struct KeyGate {
    handed: bool,
    pressed_at: Option<Instant>,
    released_at: Option<Instant>,
    reported: bool,
}

impl KeyGate {
    /// The instant a release of the press last handed over may be handed over from.
    fn release_opens_at(&self, window: Duration) -> Option<Instant> {
        self.pressed_at.map(|pressed| pressed + window)
    }
}

/// Every lane key the window has reported while a run was up.
#[derive(Debug, Default)]
pub(crate) struct KeyGates {
    keys: Vec<(KeyCode, KeyGate)>,
}

impl KeyGates {
    fn gate(&mut self, code: KeyCode) -> &mut KeyGate {
        let at = match self.keys.iter().position(|(seen, _)| *seen == code) {
            Some(at) => at,
            None => {
                self.keys.push((code, KeyGate::default()));
                self.keys.len() - 1
            }
        };
        &mut self.keys[at].1
    }

    /// The window reported `code` down (`pressed`) or up at `at`: whether the run is handed that
    /// now. A release the window keeps out is remembered, for [`KeyGates::take_opened`], which the
    /// caller asks first up to `at`.
    pub(crate) fn offer(&mut self, code: KeyCode, pressed: bool, at: Instant, debounce_ms: u32) -> bool {
        let window = Duration::from_millis(u64::from(debounce_ms));
        let bounce = window.min(LR2_POLL_GAP);
        let gate = self.gate(code);
        gate.reported = pressed;
        match (pressed, gate.handed) {
            (true, true) => false,
            (true, false) => {
                if gate.released_at.is_some_and(|released| at < released + bounce) {
                    return false;
                }
                gate.handed = true;
                gate.pressed_at = Some(at);
                true
            }
            (false, false) => {
                gate.released_at = Some(at);
                false
            }
            (false, true) => {
                if gate.release_opens_at(window).is_some_and(|opens| at < opens) {
                    return false;
                }
                gate.handed = false;
                gate.released_at = Some(at);
                true
            }
        }
    }

    /// Every release a window kept out and has ended for by `until`, each with the key and the
    /// instant its window ended, earliest first. A key pressed again before its window ended has
    /// nothing to hand over.
    pub(crate) fn take_opened(&mut self, until: Instant, debounce_ms: u32) -> Vec<(KeyCode, Instant)> {
        let window = Duration::from_millis(u64::from(debounce_ms));
        let mut opened: Vec<(KeyCode, Instant)> = self
            .keys
            .iter()
            .filter(|(_, gate)| gate.handed && !gate.reported)
            .filter_map(|(code, gate)| gate.release_opens_at(window).filter(|opens| *opens <= until).map(|opens| (*code, opens)))
            .collect();
        opened.sort_by_key(|(_, opens)| *opens);
        for (code, opens) in &opened {
            let gate = self.gate(*code);
            gate.handed = false;
            gate.released_at = Some(*opens);
        }
        opened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window of these tests, the one a fresh install keeps.
    const WINDOW_MS: u32 = rbms_config::DEFAULT_KEY_DEBOUNCE_MS;

    const KEY: KeyCode = KeyCode::KeyZ;
    const OTHER: KeyCode = KeyCode::KeyS;

    fn ms(count: u64) -> Duration {
        Duration::from_millis(count)
    }

    fn window() -> Duration {
        ms(u64::from(WINDOW_MS))
    }

    #[test]
    fn changes_far_apart_are_handed_over_the_instant_they_come() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        assert!(gates.offer(KEY, true, t0, WINDOW_MS), "a press with nothing before it");
        assert!(gates.offer(KEY, false, t0 + ms(100), WINDOW_MS), "a release long after it");
        assert!(gates.offer(KEY, true, t0 + ms(200), WINDOW_MS), "and a press long after that");
    }

    /// The point of LR2's debounce over the reference's: a press is never late. A lane hit again
    /// and again -- thirty-second notes at 200 BPM, 37.5 ms apart, each held 30 ms -- is handed
    /// every press where it came, though each comes less than the window after the release before.
    #[test]
    fn a_press_is_never_late_however_soon_it_follows_a_release() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        let gap = Duration::from_micros(37_500);
        let held = ms(30);
        for hit in 0..8 {
            let pressed = t0 + gap * hit;
            assert!(gates.take_opened(pressed, WINDOW_MS).is_empty(), "nothing was kept out");
            assert!(gates.offer(KEY, true, pressed, WINDOW_MS), "press {hit} is handed over where it came");
            assert!(gates.offer(KEY, false, pressed + held, WINDOW_MS), "and its release, past the window, where it came");
        }
    }

    #[test]
    fn a_bounce_inside_the_window_after_a_press_is_never_handed_over() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        assert!(gates.offer(KEY, true, t0, WINDOW_MS));
        assert!(!gates.offer(KEY, false, t0 + ms(2), WINDOW_MS), "the switch bounced open");
        assert!(!gates.offer(KEY, true, t0 + ms(3), WINDOW_MS), "and closed again");
        assert!(gates.take_opened(t0 + ms(100), WINDOW_MS).is_empty(), "the key reads as it was handed when the window ends");
    }

    #[test]
    fn a_release_kept_out_is_handed_over_the_instant_the_window_ends() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        assert!(gates.offer(KEY, true, t0, WINDOW_MS));
        assert!(!gates.offer(KEY, false, t0 + ms(5), WINDOW_MS), "a tap shorter than the window");
        assert!(gates.take_opened(t0 + window() - ms(1), WINDOW_MS).is_empty(), "nothing before the window ends");
        assert_eq!(gates.take_opened(t0 + window() + ms(30), WINDOW_MS), [(KEY, t0 + window())], "at the window's end, however late it is noticed");
        assert!(gates.take_opened(t0 + window() + ms(60), WINDOW_MS).is_empty(), "and only once");
        assert!(gates.offer(KEY, true, t0 + window() + ms(31), WINDOW_MS), "a press after it is handed over where it came");
    }

    /// LR2 reads a key down on the poll right after the one that saw it let go as held, not as a
    /// new press: the switch bouncing as it opens. The run keeps the key let go of, and the key is
    /// the run's to press again once it has been let go of once more.
    #[test]
    fn a_press_within_one_poll_of_a_release_is_the_switch_bouncing_open() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        let released = t0 + ms(100);
        assert!(gates.offer(KEY, true, t0, WINDOW_MS));
        assert!(gates.offer(KEY, false, released, WINDOW_MS));
        assert!(!gates.offer(KEY, true, released + LR2_POLL_GAP - ms(1), WINDOW_MS), "the bounce is not a press");
        assert!(gates.take_opened(released + ms(100), WINDOW_MS).is_empty(), "nor does it become one later");
        assert!(!gates.offer(KEY, false, released + ms(101), WINDOW_MS), "letting go of it hands nothing over");
        assert!(gates.offer(KEY, true, released + ms(101) + LR2_POLL_GAP, WINDOW_MS), "a press one poll after that is a press");
    }

    #[test]
    fn releases_kept_out_are_handed_over_earliest_first() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        assert!(gates.offer(OTHER, true, t0 + ms(4), WINDOW_MS));
        assert!(gates.offer(KEY, true, t0, WINDOW_MS));
        assert!(!gates.offer(OTHER, false, t0 + ms(6), WINDOW_MS));
        assert!(!gates.offer(KEY, false, t0 + ms(5), WINDOW_MS));
        assert_eq!(gates.take_opened(t0 + ms(100), WINDOW_MS), [(KEY, t0 + window()), (OTHER, t0 + ms(4) + window())]);
    }

    #[test]
    fn no_window_hands_over_every_change() {
        let mut gates = KeyGates::default();
        let t0 = Instant::now();
        assert!(gates.offer(KEY, true, t0, 0));
        assert!(gates.offer(KEY, false, t0, 0), "a release at the press's own instant");
        assert!(gates.offer(KEY, true, t0 + Duration::from_micros(1), 0), "and a press right after the release");
    }

    #[test]
    fn a_release_of_a_key_never_pressed_is_nothing_to_hand_over() {
        let mut gates = KeyGates::default();
        assert!(!gates.offer(KEY, false, Instant::now(), WINDOW_MS), "a key held down before the run was up");
    }
}
