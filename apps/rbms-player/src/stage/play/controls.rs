//! The reference's in-play controls: what START and SELECT do while they are held on the play
//! screen (`ControlInputProcessor.input`).
//!
//! The keys bound to the speed and the covers do what they always did
//! ([`PlayState::in_play_control`]). These are the combinations the reference adds to them, read
//! from what is held once a frame the way it reads them:
//!
//! | Held | Does |
//! | --- | --- |
//! | START and a key | the scroll speed down (keys 1, 3, 5, 7) or up (keys 2, 4, 6), once a press |
//! | START and the turntable | the cover the player is on, a fine step every 50 ms |
//! | START pressed twice within half a second | the lane cover on or off |
//! | SELECT and a key | the travel time a pinned speed holds, a millisecond a press |
//! | SELECT and the turntable | the same, a millisecond every 50 ms |
//! | START and SELECT together | which of the lift and the hidden cover the turntable moves |
//! | the wheel | the cover the player is on |
//!
//! Beside them stand the controls a key or a controller element can be given outright, which the
//! reference has no keys for ([`crate::keyconfig::ControlAction`]): each shade on or off, the travel
//! time SELECT with a key moves ([`PlayState::green_control`]), the judge offset, the two rows
//! that keep a green number and let the offset adjust itself, and the modifier held for the finer
//! step of a cover or of a held green number ([`crate::AppShared::fine_pressed`]). Each of the others
//! does what the row or the held combination of the same name does.
//!
//! A run that holds its green number from chart to chart (the KEEP GREEN NUMBER row) reads the same
//! keys and does one thing differently ([`PlayState::held_green_control`]): its speed is never set
//! by hand, so the speed keys move the number that is held instead, and whatever moves the lane
//! cover moves the speed under it. SELECT with a key moves the same number a millisecond of travel
//! at a time. What is held is the configuration's own value, so it is what the next chart starts
//! from.
//!
//! Which cover "the cover the player is on" is follows the reference's rule
//! (`ControlInputProcessor.setCoverValue`): the lane cover while it is on or while the lift and the
//! hidden cover are both off, else the lift while it is on and the hidden cover is off or the lift is
//! the one START with SELECT last chose, else the hidden cover. The lift and the hidden cover move
//! the other way round from the lane cover.
//!
//! The reference's oddities are kept. The turntable's two keys share one "held since" stamp, and
//! each key that is up clears it on its turn in the loop, so with one key held the stamp is new
//! every frame and the coarser step the reference has for a long hold is never reached.
//!
//! These run only on a play screen a skin draws: the built-in screen's keys are what they were.

use std::time::{Duration, Instant};

use rbms_config::{GREEN_NUMBER_FINE_STEP, GREEN_NUMBER_STEP, LANE_SHADE_MAX, LANE_SHADE_MIN, LANE_SHADE_STEP, duration_stepped, duration_stepped_by_green};

use super::PlayState;
use crate::app_input::{ControlContext, FixedSpeed};
use crate::keyconfig::{KEY_INDEX_COUNT, SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX};
use crate::{AppShared, ControlAction};

/// How soon a second press of START has to follow the first to switch the lane cover
/// (`ControlInputProcessor.java:165`).
const START_DOUBLE_TAP: Duration = Duration::from_millis(500);

/// How often a held turntable moves a cover or the travel time (`ControlInputProcessor.java:261`,
/// `:286`).
const HELD_STEP_EVERY: Duration = Duration::from_millis(50);

/// How long a turntable has to be held before it moves a cover by the coarser step
/// (`PlayConfig.lanecoverswitchduration`).
const COARSE_STEP_AFTER: Duration = Duration::from_millis(500);

/// How far one notch of the wheel moves a cover (`ControlInputProcessor.java:152`).
const WHEEL_STEP: f32 = 0.005;

/// How far one press moves the travel time a pinned speed holds, in milliseconds
/// (`ControlInputProcessor.java:106`, `:111`).
const DURATION_STEP_MS: f64 = 1.0;

/// Which way one press of a speed key moves a held green number: a faster scroll is a smaller
/// number, and a slower one a larger.
const FASTER: i32 = -1;
const SLOWER: i32 = 1;

/// What a held key does with START or SELECT, by key index (`ControlInputProcessor`'s `keybinds`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyBind {
    /// The lesser of a pair: slower, or a shorter travel time.
    Down,
    /// The greater of a pair.
    Up,
    /// The turntable's first key.
    TurnUp,
    /// The turntable's second key.
    TurnDown,
}

/// The bind of the key with index `index`: the keys alternate from the first, and the two indices a
/// turntable has are its two directions. The reference's tables for five, seven and nine keys are
/// all this one, cut to length.
fn key_bind(index: usize) -> KeyBind {
    match index {
        SCRATCH_FORWARD_INDEX => KeyBind::TurnUp,
        SCRATCH_BACKWARD_INDEX => KeyBind::TurnDown,
        index if index % 2 == 0 => KeyBind::Down,
        _ => KeyBind::Up,
    }
}

/// What the reference's control reader keeps between frames.
#[derive(Debug, Clone)]
pub(super) struct HeldControls {
    /// Whether each key was down on the last frame START or SELECT was read with it (`hschanged`).
    changed: [bool; KEY_INDEX_COUNT],
    start_was: bool,
    select_was: bool,
    both_were: bool,
    /// When START last went down on its own, for the second press that switches the lane cover.
    start_tapped: Option<Instant>,
    /// Whether the lift is the one the turntable moves when the lift and the hidden cover are both
    /// on (`isChangeLift`).
    change_lift: bool,
    /// When a held turntable last moved anything (`lanecovertiming`).
    stepped: Option<Instant>,
    /// Since when a turntable has been held (`laneCoverStartTiming`).
    held_since: Option<Instant>,
}

impl Default for HeldControls {
    fn default() -> HeldControls {
        HeldControls {
            changed: [true; KEY_INDEX_COUNT],
            start_was: false,
            select_was: false,
            both_were: false,
            start_tapped: None,
            change_lift: true,
            stepped: None,
            held_since: None,
        }
    }
}

impl HeldControls {
    /// Whether a held turntable is due another step, noting the step when it is.
    fn step_is_due(&mut self, now: Instant) -> bool {
        let due = self.stepped.is_none_or(|last| now.duration_since(last) > HELD_STEP_EVERY);
        if due {
            self.stepped = Some(now);
        }
        due
    }
}

/// A cover's share of the field after a move, held to the range a cover can take.
fn moved(share: f32, by: f32) -> f32 {
    (share + by).clamp(LANE_SHADE_MIN, LANE_SHADE_MAX)
}

impl PlayState {
    /// One frame of the reference's held controls. Nothing moves on a run that locks the speed, as
    /// nothing of the bound keys does either (`BMSPlayer.java:419-423`).
    pub(super) fn held_controls(&mut self, shared: &mut AppShared, now: Instant) {
        if shared.course_locks_speed() {
            return;
        }
        let (start, select) = (shared.start_pressed(), shared.select_pressed());
        let by_hand = !shared.run_plays_itself() && shared.replay.is_none();
        if start && !select {
            if by_hand && self.controls.start_was {
                self.read_keys(shared, now, true);
            } else if by_hand {
                self.controls.changed = [true; KEY_INDEX_COUNT];
            }
            if !self.controls.start_was {
                self.start_tapped(shared, now);
            }
        }
        self.controls.start_was = start && !select;
        if select && !start {
            if by_hand && self.controls.select_was {
                self.read_keys(shared, now, false);
            } else if by_hand {
                self.controls.changed = [true; KEY_INDEX_COUNT];
            }
        }
        self.controls.select_was = select && !start;
        if start && select && !self.controls.both_were {
            self.controls.change_lift = !self.controls.change_lift;
        }
        self.controls.both_were = start && select;
    }

    /// START going down on its own: the second press within [`START_DOUBLE_TAP`] of the first
    /// switches the lane cover, and is not itself the first of another pair.
    fn start_tapped(&mut self, shared: &mut AppShared, now: Instant) {
        if self.controls.start_tapped.is_some_and(|first| now.duration_since(first) < START_DOUBLE_TAP) {
            self.run_speed(shared);
            shared.config.play.enable_cover = !shared.config.play.enable_cover;
            self.controls.start_tapped = None;
            self.follow_cover(shared);
            return;
        }
        self.controls.start_tapped = Some(now);
    }

    /// Every key as START (`with_start`) or SELECT reads it, in index order
    /// (`processStart`, `processSelect`).
    fn read_keys(&mut self, shared: &mut AppShared, now: Instant, with_start: bool) {
        for index in 0..KEY_INDEX_COUNT {
            let down = shared.key_index_pressed(index);
            let pressed = down && !self.controls.changed[index];
            match (key_bind(index), with_start) {
                (KeyBind::Down, true) if pressed => self.in_play_control(shared, ControlAction::HiSpeedDown),
                (KeyBind::Up, true) if pressed => self.in_play_control(shared, ControlAction::HiSpeedUp),
                (KeyBind::Down, false) if pressed => self.move_duration(shared, -DURATION_STEP_MS),
                (KeyBind::Up, false) if pressed => self.move_duration(shared, DURATION_STEP_MS),
                (KeyBind::TurnUp, true) => self.turn_cover(shared, now, down, true),
                (KeyBind::TurnDown, true) => self.turn_cover(shared, now, down, false),
                (KeyBind::TurnUp, false) if down && self.controls.step_is_due(now) => self.move_duration(shared, DURATION_STEP_MS),
                (KeyBind::TurnDown, false) if down && self.controls.step_is_due(now) => self.move_duration(shared, -DURATION_STEP_MS),
                _ => {}
            }
            self.controls.changed[index] = down;
        }
    }

    /// One of the turntable's keys with START (`changeCoverValue`, for a key that is not analogue):
    /// held, it moves the cover a step every [`HELD_STEP_EVERY`], and up, it forgets since when the
    /// turntable was held.
    fn turn_cover(&mut self, shared: &mut AppShared, now: Instant, down: bool, up: bool) {
        if !down {
            self.controls.held_since = None;
            return;
        }
        let since = *self.controls.held_since.get_or_insert(now);
        if !self.controls.step_is_due(now) {
            return;
        }
        let step = if now.duration_since(since) > COARSE_STEP_AFTER { LANE_SHADE_STEP } else { shared.config.play.lanecover_step_fine };
        self.move_cover(shared, if up { step } else { -step });
    }

    /// Move the cover the player is on by `by` of the field (`setCoverValue`).
    ///
    /// What the run's speed is held to is settled before the cover moves, so the travel time a
    /// pinned run keeps is the one it had under the cover as it was, not under the one it is given
    /// here.
    pub(super) fn move_cover(&mut self, shared: &mut AppShared, by: f32) {
        self.run_speed(shared);
        let play = &mut shared.config.play;
        if play.enable_cover || (!play.enable_lift && !play.enable_hidden) {
            play.cover = moved(play.cover, by);
            self.follow_cover(shared);
        } else if play.enable_lift && (!play.enable_hidden || self.controls.change_lift) {
            play.lift = moved(play.lift, -by);
            shared.rebuild_skin();
        } else {
            play.hidden = moved(play.hidden, -by);
        }
    }

    /// The wheel turning by `lines` (`ControlInputProcessor.java:151-154`).
    pub(super) fn wheel_cover(&mut self, shared: &mut AppShared, lines: f32) {
        if !shared.course_locks_speed() {
            self.move_cover(shared, -lines * WHEEL_STEP);
        }
    }

    /// Keep a pinned travel time where it was after the lane cover moved, by moving the speed
    /// (`LaneRenderer.java:212-215`). A run that holds its green number from chart to chart does the
    /// same from the number the configuration holds.
    fn follow_cover(&mut self, shared: &mut AppShared) {
        if self.hold_green_number(shared) {
            return;
        }
        if let Some(fixed) = self.run_speed(shared).fixed {
            shared.retarget_hispeed(fixed);
        }
    }

    /// Move the travel time a pinned speed holds by `by_ms`, and the speed with it
    /// (`LaneRenderer.setDuration`). A run that pins its speed to no tempo has no travel time to
    /// hold, and nothing moves. On a run that holds its green number the travel time is the
    /// configuration's own, as it is in the reference (`PlayConfig.setDuration`), and stays moved
    /// for the charts after this one.
    fn move_duration(&mut self, shared: &mut AppShared, by_ms: f64) {
        if self.held_tempo(shared).is_some() {
            let play = &mut shared.config.play;
            play.duration_ms = duration_stepped(play.duration_ms, by_ms as i32);
            self.hold_green_number(shared);
            return;
        }
        let speed = self.run_speed(shared);
        let Some(fixed) = speed.fixed else {
            return;
        };
        let held = FixedSpeed { green: fixed.green + by_ms, ..fixed };
        if held.green <= 0.0 {
            return;
        }
        shared.retarget_hispeed(held);
        self.speed = Some(super::RunSpeed { fixed: Some(held), ..speed });
    }

    /// One of the two controls that move the green number a millisecond of travel at a time, and
    /// whether `action` was one of them. They do what SELECT with a key does
    /// ([`PlayState::move_duration`]) from a key of their own: on a run that holds its green number
    /// they move the number that is held, on a run pinned to a tempo the travel time it is pinned
    /// to, and on a run with a free speed there is no travel time to move. A course that locks the
    /// speed locks them too.
    pub(super) fn green_control(&mut self, shared: &mut AppShared, action: ControlAction) -> bool {
        let by_ms = match action {
            ControlAction::GreenUp => DURATION_STEP_MS,
            ControlAction::GreenDown => -DURATION_STEP_MS,
            _ => return false,
        };
        if !shared.course_locks_speed() {
            self.move_duration(shared, by_ms);
        }
        true
    }

    /// One in-play control on a run that holds its green number, and whether this run is one.
    ///
    /// The speed of such a run is worked out, not set, so the two speed controls move the number
    /// that is held: a faster scroll is a smaller green number and a slower one a larger, by
    /// [`GREEN_NUMBER_STEP`] a press and by [`GREEN_NUMBER_FINE_STEP`] with the fine modifier down.
    /// The reference moves the speed alone here (`LaneRenderer.changeHispeed`) and leaves the travel
    /// time it stores where it was, so its next cover move or its next chart puts the speed back;
    /// here the press is the player choosing a new number, and it is kept. Every other control does
    /// what it always does, and the speed is then put where the number wants it under the lane as
    /// the control left it.
    pub(super) fn held_green_control(&mut self, shared: &mut AppShared, action: ControlAction) -> bool {
        if self.held_tempo(shared).is_none() {
            return false;
        }
        match action {
            ControlAction::HiSpeedUp => self.step_held_green(shared, FASTER),
            ControlAction::HiSpeedDown => self.step_held_green(shared, SLOWER),
            _ => {
                let ctx = ControlContext { fine: shared.fine_pressed(), base_hispeed: shared.config.play.hispeed, fixed: None };
                shared.apply_control(action, &ctx);
            }
        }
        self.hold_green_number(shared);
        true
    }

    /// Move the held green number one step: down for a faster scroll, up for a slower one.
    fn step_held_green(&mut self, shared: &mut AppShared, direction: i32) {
        let step = if shared.fine_pressed() { GREEN_NUMBER_FINE_STEP } else { GREEN_NUMBER_STEP };
        let play = &mut shared.config.play;
        play.duration_ms = duration_stepped_by_green(play.duration_ms, direction * step);
    }
}

#[cfg(test)]
mod key_tests;
#[cfg(test)]
mod tests;
