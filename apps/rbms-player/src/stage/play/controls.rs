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
//! | START and the turntable | the cover the player is on, a thousandth of the lane at a time |
//! | START pressed twice within half a second | the lane cover on or off |
//! | SELECT and a key | the travel time a pinned speed holds, a millisecond a press |
//! | SELECT and the turntable | the same, a millisecond every 50 ms |
//! | START and SELECT together | which of the lift and the hidden cover the turntable moves |
//! | START, SELECT and the turntable | the lift, a thousandth of the lane at a time |
//! | the wheel | the cover the player is on |
//!
//! Beside them stand the controls a key or a controller element can be given outright, which the
//! reference has no keys for ([`crate::keyconfig::ControlAction`]): each shade on or off, the travel
//! time SELECT with a key moves ([`PlayState::green_control`]), the judge offset, the two rows
//! that keep a green number and let the offset adjust itself, and the modifier held for the finer
//! step of a cover or of a held green number ([`crate::AppShared::fine_pressed`]). Each of the others
//! does what the row or the held combination of the same name does. The two keys of the judge
//! offset repeat while they are held ([`PlayState::repeat_held_keys`]).
//!
//! # The turntable with START
//!
//! This is where the player sets a shade to the figure they want, so it moves by the smallest
//! step a skin can show: one of the white number, a thousandth of the lane ([`TURN_STEP`]), on the
//! steps the PLAY menu's rows stand on. A key of the turntable moves one step on the press and,
//! held, repeats after a short wait at a pace that quickens ([`repeat::SHADE_PACE`]); an analogue
//! turntable moves a step a tick ([`crate::AppShared::take_turntable_ticks`]). The turntable's
//! first direction lowers the lane cover and the lift, as in the reference.
//!
//! With START alone it is the cover the player is on, by the reference's rule below. With SELECT
//! held as well it is the lift, whatever is switched on: the reference has no combination for the
//! lift alone, and reaches it only by switching the lane cover off. A shade that is switched off
//! moves as one that is on does and stays off, which is what the reference does with its lane cover.
//!
//! The green number does not move while the lane cover is moved this way, and the scroll speed
//! does: on a run that holds its number or pins it to a tempo by the rule those runs always had,
//! and on a run that does neither by holding the number it had when the turntable first moved the
//! cover ([`TurnPin`]). The reference lets the number of that last run go; here the combination is
//! the one the player asked to set a cover by without losing the number. A speed that would leave
//! its range stands at the end of it, and comes back as the cover does -- under the same hold of
//! START or a later one, for as long as nothing but the turntable has moved the speed or the cover
//! since. The lift and the hidden cover are no part of the number.
//!
//! The reference moves a shade by `lanecovermarginlow` every 50 ms from the first frame
//! (`changeCoverValue`), with no wait after the press, so a press of ordinary length is two steps.
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
//! The reference's coarser step for a long hold (`lanecovermarginhigh`) is never reached there: the
//! turntable's two keys share one "held since" stamp, and each key that is up clears it on its turn
//! in the loop. What a long hold does here is this player's own ([`repeat`]).
//!
//! The held controls run only on a play screen a skin draws: the built-in screen's keys are what
//! they were. The repeats of the judge offset's keys run on either, as the keys themselves do.

use std::time::{Duration, Instant};

use rbms_config::{
    GREEN_NUMBER_FINE_STEP, GREEN_NUMBER_STEP, HISPEED_MAX, HISPEED_MIN, LANE_SHADE_MAX, LANE_SHADE_MIN, PlayOptions, SettingId, duration_stepped,
    duration_stepped_by_green, move_number_steps,
};

use super::PlayState;
use crate::app_input::{ControlContext, FixedSpeed};
use crate::keyconfig::{KEY_INDEX_COUNT, SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX};
use crate::{AppShared, ControlAction};
use repeat::{HeldRepeat, SHADE_PACE, STEADY_PACE};

/// How soon a second press of START has to follow the first to switch the lane cover
/// (`ControlInputProcessor.java:165`).
const START_DOUBLE_TAP: Duration = Duration::from_millis(500);

/// How often a turntable held with SELECT moves the travel time (`ControlInputProcessor.java:286`).
const HELD_STEP_EVERY: Duration = Duration::from_millis(50);

/// How far one step of the turntable with START moves a shade, in the steps the rows of the shades
/// stand on: one, which is one of the white number and a thousandth of the lane.
const TURN_STEP: i64 = 1;

/// The whole lane, which a lane cover that hides all of it leaves nothing of.
const WHOLE_LANE: f64 = 1.0;

/// How many keys the judge offset has: one up, one down.
const OFFSET_KEY_COUNT: usize = 2;

/// The two keys of the judge offset, which repeat while they are held.
const OFFSET_KEYS: [ControlAction; OFFSET_KEY_COUNT] = [ControlAction::OffsetUp, ControlAction::OffsetDown];

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

/// Which shade the turntable moves with START held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turned {
    /// The cover the player is on, with START alone.
    Standing,
    /// The lift, with SELECT held as well.
    Lift,
}

/// The green number a run with no number of its own keeps while the turntable moves its lane cover:
/// the scroll speed the run would have with the lane uncovered, which is the same thing said
/// without a tempo. The speed under any cover is that speed times what the cover leaves open.
///
/// It is kept from one hold of START to the next. A speed that stopped at the end of its range, or
/// under a lane that is all cover, is no longer the number the player chose, and a pin taken
/// afresh from it at the next hold would keep that instead.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TurnPin {
    uncovered: f64,
    /// The scroll speed and the share of the lane under the lane cover the pin last left the run
    /// at. When either is no longer what it was, something else moved it and chose a new number,
    /// and the pin is taken afresh.
    left_at: f64,
    left_under: f32,
}

/// What the reference's control reader keeps between frames, and what this player's own held
/// controls keep beside it.
#[derive(Debug, Clone)]
pub(super) struct HeldControls {
    /// Whether each key was down on the last frame START or SELECT was read with it (`hschanged`).
    changed: [bool; KEY_INDEX_COUNT],
    start_was: bool,
    select_was: bool,
    both_were: bool,
    /// Whether START itself was down on the last frame, with SELECT or without.
    start_down_was: bool,
    /// When START last went down on its own, for the second press that switches the lane cover.
    start_tapped: Option<Instant>,
    /// Whether the lift is the one the turntable moves when the lift and the hidden cover are both
    /// on (`isChangeLift`).
    change_lift: bool,
    /// When a turntable held with SELECT last moved the travel time (`lanecovertiming`).
    stepped: Option<Instant>,
    /// The turntable's two keys as START holds them, forward and backward.
    turn_forward: HeldRepeat,
    turn_backward: HeldRepeat,
    /// Which shade the turntable was last read for, so a key still down when SELECT joins START or
    /// leaves it starts over on the other shade.
    turned: Option<Turned>,
    /// The green number the turntable is holding for a run that has none of its own, for as long
    /// as nothing else moves the speed or the lane cover.
    turn_pin: Option<TurnPin>,
    /// The two keys of the judge offset as they are held, in the order of [`OFFSET_KEYS`].
    offset_keys: [HeldRepeat; OFFSET_KEY_COUNT],
}

impl Default for HeldControls {
    fn default() -> HeldControls {
        HeldControls {
            changed: [true; KEY_INDEX_COUNT],
            start_was: false,
            select_was: false,
            both_were: false,
            start_down_was: false,
            start_tapped: None,
            change_lift: true,
            stepped: None,
            turn_forward: HeldRepeat::default(),
            turn_backward: HeldRepeat::default(),
            turned: None,
            turn_pin: None,
            offset_keys: [HeldRepeat::default(); OFFSET_KEY_COUNT],
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

/// The shade the turntable and the wheel move with nothing said of which: the lane cover while it
/// is on or while the lift and the hidden cover are both off, else the lift while it is on and the
/// hidden cover is off or the lift is the one START with SELECT last chose, else the hidden cover
/// (`ControlInputProcessor.setCoverValue`).
fn shade_stood_on(play: &PlayOptions, change_lift: bool) -> SettingId {
    if play.enable_cover || (!play.enable_lift && !play.enable_hidden) {
        SettingId::LaneCover
    } else if play.enable_lift && (!play.enable_hidden || change_lift) {
        SettingId::Lift
    } else {
        SettingId::Hidden
    }
}

impl PlayState {
    /// One frame of the held controls, with what the controller's turntable turned since the last.
    pub(super) fn held_controls(&mut self, shared: &mut AppShared, now: Instant) {
        let turned = shared.take_turntable_ticks();
        self.read_held(shared, now, turned);
    }

    /// One frame of the held controls: the reference's, and the turntable with START. `turned` is
    /// how many ticks an analogue turntable turned since the last frame, forward counted up, or
    /// `None` when the turntable is keys. Nothing moves on a run that locks the speed, as nothing
    /// of the bound keys does either (`BMSPlayer.java:419-423`).
    ///
    /// The turntable is read from the second frame START is down, as the reference reads its keys,
    /// so what an analogue one turned before START went down is dropped with the frame it is taken
    /// on.
    ///
    /// A press of START is START itself going down. The reference counts SELECT coming up beside a
    /// held START as one too, so SELECT tapped under START can switch the lane cover; here that is
    /// the way to the lift and back, and switches nothing.
    fn read_held(&mut self, shared: &mut AppShared, now: Instant, turned: Option<i32>) {
        if shared.course_locks_speed() {
            return;
        }
        let (start, select) = (shared.start_pressed(), shared.select_pressed());
        let by_hand = !shared.run_plays_itself() && shared.replay.is_none();
        if start && !select {
            if by_hand && self.controls.start_was {
                self.read_keys(shared, now, true);
                self.turn_shade(shared, now, turned, Turned::Standing);
            } else if by_hand {
                self.controls.changed = [true; KEY_INDEX_COUNT];
            }
            if !self.controls.start_down_was {
                self.start_tapped(shared, now);
            }
        }
        self.controls.start_was = start && !select;
        self.controls.start_down_was = start;
        if select && !start {
            if by_hand && self.controls.select_was {
                self.read_keys(shared, now, false);
            } else if by_hand {
                self.controls.changed = [true; KEY_INDEX_COUNT];
            }
        }
        self.controls.select_was = select && !start;
        if start && select {
            if !self.controls.both_were {
                self.controls.change_lift = !self.controls.change_lift;
            } else if by_hand {
                self.turn_shade(shared, now, turned, Turned::Lift);
            }
        }
        self.controls.both_were = start && select;
        if !start {
            self.let_go_of_the_turntable();
        }
    }

    /// START came up: the turntable's keys start over the next time it is held. The green number
    /// the turntable held for a run with none of its own stays held ([`TurnPin`]).
    fn let_go_of_the_turntable(&mut self) {
        self.controls.turn_forward.reset();
        self.controls.turn_backward.reset();
        self.controls.turned = None;
    }

    /// One frame of the turntable with START held: the steps its keys are due, a step a tick of an
    /// analogue one, and the shade moved by what they come to. Beside an analogue turntable only
    /// the keyboard's keys are read as keys: the lane the controller holds while the disc turns is
    /// the same turn, already counted in ticks.
    fn turn_shade(&mut self, shared: &mut AppShared, now: Instant, turned: Option<i32>, shade: Turned) {
        if self.controls.turned != Some(shade) {
            self.controls.turn_forward.reset();
            self.controls.turn_backward.reset();
            self.controls.turned = Some(shade);
        }
        let down = |index: usize| if turned.is_some() { shared.keyboard_index_pressed(index) } else { shared.key_index_pressed(index) };
        let (forward, backward) = (down(SCRATCH_FORWARD_INDEX), down(SCRATCH_BACKWARD_INDEX));
        let forward = self.controls.turn_forward.steps(now, forward, &SHADE_PACE);
        let backward = self.controls.turn_backward.steps(now, backward, &SHADE_PACE);
        let steps = (i64::from(forward) - i64::from(backward) + i64::from(turned.unwrap_or_default())) * TURN_STEP;
        if steps == 0 {
            return;
        }
        let id = match shade {
            Turned::Standing => shade_stood_on(&shared.config.play, self.controls.change_lift),
            Turned::Lift => SettingId::Lift,
        };
        self.shade_moved_us = Some(shared.skin_now_us());
        match id {
            SettingId::LaneCover => self.turn_lane_cover(shared, steps),
            SettingId::Lift => {
                move_number_steps(&mut shared.config, SettingId::Lift, -steps);
                shared.rebuild_skin();
            }
            _ => {
                move_number_steps(&mut shared.config, SettingId::Hidden, -steps);
            }
        }
    }

    /// Move the lane cover `steps` of its row with the turntable, and the scroll speed with it so
    /// the green number stays: by the number the run holds or is pinned to when it has one
    /// ([`PlayState::follow_cover`]), and otherwise by the one it had when the turntable first moved
    /// the cover ([`PlayState::turn_pin`]).
    ///
    /// What the run holds is settled before the cover moves, as it is for every other way of moving
    /// it ([`PlayState::move_cover`]).
    fn turn_lane_cover(&mut self, shared: &mut AppShared, steps: i64) {
        let pinned = self.held_tempo(shared).is_some() || self.run_speed(shared).fixed.is_some();
        if pinned {
            move_number_steps(&mut shared.config, SettingId::LaneCover, steps);
            self.follow_cover(shared);
            return;
        }
        let pin = self.turn_pin(shared);
        move_number_steps(&mut shared.config, SettingId::LaneCover, steps);
        let Some(pin) = pin else {
            return;
        };
        let under = shared.effective_cover();
        let speed = (pin.uncovered * (WHOLE_LANE - f64::from(under))).clamp(HISPEED_MIN, HISPEED_MAX);
        shared.config.play.hispeed = speed;
        self.controls.turn_pin = Some(TurnPin { left_at: speed, left_under: under, ..pin });
    }

    /// The green number the turntable holds for a run with none of its own, taken from the speed
    /// and the cover as they stand the first time it is asked for, and again when something else
    /// has moved the speed or the cover since. A lane that is wholly covered has no number to
    /// take, and `None` is the answer.
    fn turn_pin(&mut self, shared: &AppShared) -> Option<TurnPin> {
        let (speed, under) = (shared.config.play.hispeed, shared.effective_cover());
        if let Some(pin) = self.controls.turn_pin.filter(|pin| pin.left_at == speed && pin.left_under == under) {
            return Some(pin);
        }
        let open = WHOLE_LANE - f64::from(under);
        let pin = (open > 0.0).then(|| TurnPin { uncovered: speed / open, left_at: speed, left_under: under });
        self.controls.turn_pin = pin;
        pin
    }

    /// The repeats of the keys that repeat while they are held, once a frame: the two keys of the
    /// judge offset. The press itself is acted on where it arrives, as every control's is; what is
    /// handed out here is what the hold earns after it, at a pace that does not quicken
    /// ([`repeat::STEADY_PACE`]) -- the whole range of the offset is a few hundred steps. Each
    /// repeat does what a press does, so a run that plays itself is as deaf to it.
    pub(super) fn repeat_held_keys(&mut self, shared: &mut AppShared, now: Instant) {
        for (index, action) in OFFSET_KEYS.into_iter().enumerate() {
            let down = shared.control_repeating(action);
            let due = self.controls.offset_keys[index].repeats(now, down, &STEADY_PACE);
            for _ in 0..due {
                self.in_play_control(shared, action);
            }
        }
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
    /// (`processStart`, `processSelect`). The turntable with START is read apart
    /// ([`PlayState::turn_shade`]).
    fn read_keys(&mut self, shared: &mut AppShared, now: Instant, with_start: bool) {
        for index in 0..KEY_INDEX_COUNT {
            let down = shared.key_index_pressed(index);
            let pressed = down && !self.controls.changed[index];
            match (key_bind(index), with_start) {
                (KeyBind::Down, true) if pressed => self.in_play_control(shared, ControlAction::HiSpeedDown),
                (KeyBind::Up, true) if pressed => self.in_play_control(shared, ControlAction::HiSpeedUp),
                (KeyBind::Down, false) if pressed => self.move_duration(shared, -DURATION_STEP_MS),
                (KeyBind::Up, false) if pressed => self.move_duration(shared, DURATION_STEP_MS),
                (KeyBind::TurnUp, false) if down && self.controls.step_is_due(now) => self.move_duration(shared, DURATION_STEP_MS),
                (KeyBind::TurnDown, false) if down && self.controls.step_is_due(now) => self.move_duration(shared, -DURATION_STEP_MS),
                _ => {}
            }
            self.controls.changed[index] = down;
        }
    }

    /// Move the cover the player is on by `by` of the field (`setCoverValue`), which is what the
    /// wheel does. A skin shows the figures of the shades for a moment after, as it does after a
    /// key of a shade: the wheel moves one with nothing held.
    ///
    /// What the run's speed is held to is settled before the cover moves, so the travel time a
    /// pinned run keeps is the one it had under the cover as it was, not under the one it is given
    /// here.
    pub(super) fn move_cover(&mut self, shared: &mut AppShared, by: f32) {
        self.run_speed(shared);
        self.shade_moved_us = Some(shared.skin_now_us());
        let id = shade_stood_on(&shared.config.play, self.controls.change_lift);
        let play = &mut shared.config.play;
        match id {
            SettingId::LaneCover => {
                play.cover = moved(play.cover, by);
                self.follow_cover(shared);
            }
            SettingId::Lift => {
                play.lift = moved(play.lift, -by);
                shared.rebuild_skin();
            }
            _ => play.hidden = moved(play.hidden, -by),
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
mod capture_tests;
#[cfg(test)]
mod key_tests;
mod repeat;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod turn_tests;
