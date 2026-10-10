//! Property clusters C and D: the judgement on show and the state of the lanes, on the play screen.
//!
//! Where the values come from: the last judgement of each side and how early or late it was, the
//! lane cover, lift and hidden values and the scroll speed, the tempo and the time the run has been
//! going, and the kind and the phase of the run. They are the figures `BMSPlayer`, `JudgeManager`
//! and `LaneRenderer` keep, handed over once a frame as a [`PlayShown`]
//! ([`PlayState::of`], [`ScreenHost::show_play`](super::ScreenHost::show_play)).
//!
//! What these clusters answer: options 32-33, 40-43, 82, 84, 241, 261, 361, 270-273, 400, 1080,
//! 1240, 1242-1243, 1262-1263 and 1362-1363; numbers 10, 12, 14, 160-164, 310-313, 314-316, 525-527
//! and 1312-1327; floats 310; rates 4-6 and 101; image indices 40, 42-43, 54-55 and 301-308.
//! The loading options (80-81) and the loading progress are cluster M's, the score and the gauge
//! ranges are cluster B's ([`ScoreState::of_play`](super::score::ScoreState::of_play)). The
//! offsets the lift, the cover and the hidden band move objects by belong to the timer driver
//! ([`play_timers`](super::play_timers)).
//!
//! A value is read as `IntegerPropertyFactory`, `BooleanPropertyFactory`, `FloatPropertyFactory` and
//! `LaneRenderer` compute it, in the number types they compute it in. That includes what looks odd
//! there and is kept: the lane cover figure reads the share the cover is set to whether or not it is
//! switched on, the durations of the "lane cover on" ids use that same share, a judgement between
//! zero and a whole millisecond is neither early nor late, and only the first of the six judgements
//! has an option of its own (the reference gives GREAT and below none).
//!
//! The practice state option (1080) is always off. The reference's play screen has a state in which
//! a practice range is chosen; this player chooses one on a screen of its own, so a play screen
//! here is never in that state.
//!
//! The images that show the last judgement of one key (500-519, 1510-1699) have no source here: the
//! judge engine keeps no judgement per key. They read as absent.
//!
//! A cluster that has no play frame answers nothing, so the ids fall through to whatever else can.
//! (The reference answers zero for the durations on a screen other than the play screen; here a
//! screen that has no play frame has no answer.)

use rbms_skin::property::generated::*;

use super::ClusterState;

pub mod shown;

pub use shown::{Judgement, LaneSettings, PLAYER_SIDES, PlayClock, PlayKind, PlayLive, PlayPhase, PlayShown, Scroll, travel_region_ms};

/// The gauges that are groove gauges and the first that is a hard gauge, by the number the
/// reference gives them (`BooleanPropertyFactory.gauge_groove`, `gauge_hard`).
const LAST_GROOVE_GAUGE: i32 = 2;
const FIRST_HARD_GAUGE: i32 = 3;

/// The gauges whose option is the EX one: the assist and easy gauges, the EX hard and hazard gauges
/// and the two EX course gauges (`BooleanPropertyFactory.gauge_ex`).
const EX_GAUGES: [i32; 6] = [0, 1, 4, 5, 7, 8];

/// What the scroll speed is multiplied by to read it as the reference's LR2 style number does, and
/// the modulus of the digits after the dot.
const HISPEED_SCALE: f32 = 100.0;
const HISPEED_DIGITS_MODULUS: i32 = 100;

/// What a lane figure (cover, lift, hidden) is multiplied by to read it as a whole number.
const LANE_FIGURE_SCALE: f32 = 1000.0;
const LANE_FIGURE_SCALE_WIDE: f64 = 1000.0;

/// The size of a minute and a second in the units the clock figures are cut from.
const MILLIS_PER_MINUTE: i32 = 60_000;
const MILLIS_PER_SECOND: i32 = 1_000;
const SECONDS_PER_MINUTE: i32 = 60;

/// How long past its end the time left still counts, in milliseconds
/// (`IntegerPropertyFactory.timeleft_minute`).
const TIME_LEFT_GRACE_MS: i32 = 1_000;

/// The green number is three fifths of the duration (`IntegerPropertyFactory.duration_green`), and
/// the same share of the whole travel time when asked by tempo (`createDurationLanecoverProperty`).
const GREEN_NUMERATOR: i32 = 3;
const GREEN_DENOMINATOR: i32 = 5;
const GREEN_SHARE: f64 = 0.6;

/// How the ids from [`NUMBER_DURATION_LANECOVER_ON`] to [`NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF`]
/// are laid out: four tempos, each with the cover counted or not and the green number or not.
const DURATION_IDS_PER_TEMPO: i32 = 4;
const DURATION_IDS_PER_COVER_CHOICE: i32 = 2;
const DURATION_NOW_BPM: i32 = 0;
const DURATION_MAIN_BPM: i32 = 1;
const DURATION_MIN_BPM: i32 = 2;
const DURATION_MAX_BPM: i32 = 3;

/// The first judgement after the best one, which the options of the side's judgement tell apart
/// from it.
const FIRST_NON_PERFECT_JUDGE: usize = 1;

/// The sides a judgement is kept for: the first player's, the second's and the third's.
const SIDE_1P: usize = 0;
const SIDE_2P: usize = 1;
const SIDE_3P: usize = 2;

/// What the play screen's clusters read from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlayState<'a> {
    /// The play screen's frame, when there is one.
    pub shown: Option<&'a PlayShown>,
}

impl<'a> PlayState<'a> {
    /// The cluster over one play frame.
    pub fn of(shown: &'a PlayShown) -> PlayState<'a> {
        PlayState { shown: Some(shown) }
    }
}

/// What `Math.round` adds before it takes the floor.
const ROUND_HALF: f64 = 0.5;

/// `Math.round` cut to an `int` the way Java does: rounded half up to a `long`, then narrowed.
fn java_round_to_int(value: f64) -> i32 {
    (value + ROUND_HALF).floor() as i64 as i32
}

impl PlayShown {
    /// How long a note takes to cross the lane with the cover over it, in milliseconds
    /// (`LaneRenderer.currentduration`).
    fn current_duration(&self) -> i32 {
        let lanes = &self.live.lanes;
        let cover = if lanes.lane_cover_on { lanes.lane_cover } else { 0.0 };
        java_round_to_int(self.live.scroll.region_ms * f64::from(1.0_f32 - cover))
    }

    /// The duration at one of the four tempos, with the cover counted or not and the green number or
    /// not, as the id says (`createDurationLanecoverProperty`).
    fn duration_at(&self, id: i32) -> i32 {
        let place = id - NUMBER_DURATION_LANECOVER_ON;
        let green = place % DURATION_IDS_PER_COVER_CHOICE == 1;
        let covered = place % DURATION_IDS_PER_TEMPO < DURATION_IDS_PER_COVER_CHOICE;
        let scroll = &self.live.scroll;
        let bpm = match place / DURATION_IDS_PER_TEMPO {
            DURATION_NOW_BPM => scroll.now_bpm,
            DURATION_MAIN_BPM => scroll.main_bpm,
            DURATION_MIN_BPM => scroll.min_bpm,
            DURATION_MAX_BPM => scroll.max_bpm,
            _ => 0.0,
        };
        let lanes = &self.live.lanes;
        let cover_share = if covered { f64::from(1.0_f32 - lanes.lane_cover) } else { 1.0 };
        let green_share = if green { GREEN_SHARE } else { 1.0 };
        java_round_to_int(shown::LANE_TRAVEL_BASE_MS / bpm / f64::from(lanes.hispeed) * cover_share * green_share)
    }

    /// The share of the lane the cover takes, which the lane cover slider shows
    /// (`FloatPropertyFactory.createLanecover`): none while the cover is off, and the part of it
    /// that is not under the lift while the lift is on.
    fn lane_cover_share(&self) -> f32 {
        let lanes = &self.live.lanes;
        if !lanes.lane_cover_on {
            return 0.0;
        }
        if lanes.lift_on { lanes.lane_cover * (1.0 - lanes.lift) } else { lanes.lane_cover }
    }

    /// How far through the chart's playing time the run is (`FloatPropertyFactory.createMusicProgress`):
    /// nothing before the play timer is on, then the share of the time it has been on, held at one.
    fn music_progress(&self) -> f32 {
        let Some(elapsed) = self.live.clock.elapsed_ms() else {
            return 0.0;
        };
        let share = elapsed as f32 / self.play_time_ms as f32;
        if share.is_nan() { share } else { share.min(1.0) }
    }

    /// The milliseconds the play timer has been on, cut to the `int` the clock figures are cut from.
    fn played_ms(&self) -> i32 {
        self.live.clock.elapsed_ms().unwrap_or_default() as i32
    }

    /// The milliseconds of playing time left, counted a second past its end
    /// (`IntegerPropertyFactory.timeleft_minute`), never below zero.
    fn left_ms(&self) -> i32 {
        self.play_time_ms.wrapping_sub(self.played_ms()).wrapping_add(TIME_LEFT_GRACE_MS).max(0)
    }

    /// The last judgement given on one side.
    fn judgement(&self, side: usize) -> Option<Judgement> {
        self.live.judgements.get(side).copied().flatten()
    }

    /// Whether the last judgement on one side was the first of the six (`getNowJudge(player) == 1`).
    fn is_perfect_on(&self, side: usize) -> bool {
        self.judgement(side).is_some_and(|judgement| judgement.judge < FIRST_NON_PERFECT_JUDGE)
    }

    /// Whether the last judgement on one side was a later one that came early or late
    /// (`getNowJudge(player) > 1 && getRecentJudgeTiming(player) > 0` or `< 0`).
    fn is_off_beat_on(&self, side: usize, early: bool) -> bool {
        self.judgement(side)
            .is_some_and(|judgement| judgement.judge >= FIRST_NON_PERFECT_JUDGE && if early { judgement.timing_ms > 0 } else { judgement.timing_ms < 0 })
    }
}

impl ClusterState for PlayState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let shown = self.shown?;
        let live = &shown.live;
        let gauge = shown.gauge.kind;
        Some(match id {
            OPTION_AUTOPLAYON => live.kind == PlayKind::Autoplay,
            OPTION_AUTOPLAYOFF => live.kind != PlayKind::Autoplay,
            OPTION_REPLAY_OFF => matches!(live.kind, PlayKind::Play | PlayKind::Practice),
            OPTION_REPLAY_PLAYING => live.kind == PlayKind::Replay,
            OPTION_STATE_PRACTICE => false,
            OPTION_BGAOFF => !live.bga_on,
            OPTION_BGAON => live.bga_on,
            OPTION_GAUGE_GROOVE => gauge <= LAST_GROOVE_GAUGE,
            OPTION_GAUGE_HARD => gauge >= FIRST_HARD_GAUGE,
            OPTION_GAUGE_EX => EX_GAUGES.contains(&gauge),
            OPTION_1P_BORDER_OR_MORE => shown.qualified,
            OPTION_1P_PERFECT => shown.is_perfect_on(SIDE_1P),
            OPTION_2P_PERFECT => shown.is_perfect_on(SIDE_2P),
            OPTION_3P_PERFECT => shown.is_perfect_on(SIDE_3P),
            OPTION_1P_EARLY => shown.is_off_beat_on(SIDE_1P, true),
            OPTION_1P_LATE => shown.is_off_beat_on(SIDE_1P, false),
            OPTION_2P_EARLY => shown.is_off_beat_on(SIDE_2P, true),
            OPTION_2P_LATE => shown.is_off_beat_on(SIDE_2P, false),
            OPTION_3P_EARLY => shown.is_off_beat_on(SIDE_3P, true),
            OPTION_3P_LATE => shown.is_off_beat_on(SIDE_3P, false),
            OPTION_LANECOVER1_CHANGING => live.cover_keys_held,
            OPTION_LANECOVER1_ON => live.lanes.lane_cover_on,
            OPTION_LIFT1_ON => live.lanes.lift_on,
            OPTION_HIDDEN1_ON => live.lanes.hidden_on,
            OPTION_CONSTANT => live.lanes.constant_on,
            _ => return None,
        })
    }

    fn integer(&self, id: i32) -> Option<i32> {
        let shown = self.shown?;
        let live = &shown.live;
        let lanes = &live.lanes;
        let side_timing = |side: usize| shown.judgement(side).map_or(0, |judgement| judgement.timing_ms as i32);
        Some(match id {
            NUMBER_JUDGETIMING => live.judge_timing_ms,
            NUMBER_HISPEED_LR2 => (lanes.hispeed * HISPEED_SCALE) as i32,
            NUMBER_HISPEED => lanes.hispeed as i32,
            NUMBER_HISPEED_AFTERDOT => (lanes.hispeed * HISPEED_SCALE) as i32 % HISPEED_DIGITS_MODULUS,
            NUMBER_DURATION => shown.current_duration(),
            NUMBER_DURATION_GREEN => shown.current_duration().wrapping_mul(GREEN_NUMERATOR) / GREEN_DENOMINATOR,
            NUMBER_DURATION_LANECOVER_ON..=NUMBER_MAXBPM_DURATION_GREEN_LANECOVER_OFF => shown.duration_at(id),
            NUMBER_LANECOVER1 => (lanes.lane_cover * LANE_FIGURE_SCALE) as i32,
            NUMBER_LIFT1 => (lanes.lift * LANE_FIGURE_SCALE) as i32,
            NUMBER_HIDDEN1 => (lanes.hidden * LANE_FIGURE_SCALE) as i32,
            NUMBER_LANECOVER2 => ((1.0 - f64::from(lanes.lift)) * f64::from(lanes.lane_cover) * LANE_FIGURE_SCALE_WIDE) as i32,
            NUMBER_NOWBPM => live.scroll.now_bpm as i32,
            NUMBER_PLAYTIME_MINUTE => shown.played_ms() / MILLIS_PER_MINUTE,
            NUMBER_PLAYTIME_SECOND => shown.played_ms() / MILLIS_PER_SECOND % SECONDS_PER_MINUTE,
            NUMBER_TIMELEFT_MINUTE => shown.left_ms() / MILLIS_PER_MINUTE,
            NUMBER_TIMELEFT_SECOND => shown.left_ms() / MILLIS_PER_SECOND % SECONDS_PER_MINUTE,
            VALUE_JUDGE_1P_DURATION => side_timing(SIDE_1P),
            VALUE_JUDGE_2P_DURATION => side_timing(SIDE_2P),
            VALUE_JUDGE_3P_DURATION => side_timing(SIDE_3P),
            _ => return None,
        })
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        let shown = self.shown?;
        let live = &shown.live;
        let played = &live.played;
        Some(match id {
            BUTTON_GAUGE_1P => shown.gauge.kind,
            BUTTON_RANDOM_1P => played.random,
            BUTTON_RANDOM_2P => played.random_2p,
            BUTTON_DPOPTION => played.double_option,
            BUTTON_HSFIX => live.lanes.fix_hispeed,
            BUTTON_ASSIST_EXJUDGE => i32::from(played.custom_judge),
            BUTTON_ASSIST_CONSTANT => i32::from(played.constant_scroll),
            BUTTON_ASSIST_JUDGEAREA => i32::from(played.judge_area),
            BUTTON_ASSIST_LEGACY => i32::from(played.legacy_long_note),
            BUTTON_ASSIST_MARKNOTE => i32::from(played.mark_note),
            BUTTON_ASSIST_BPMGUIDE => i32::from(played.bpm_guide),
            BUTTON_ASSIST_NOMINE => i32::from(played.no_mine),
            BUTTON_LNMODE => live.long_note_mode,
            _ => return None,
        })
    }

    fn rate(&self, id: i32) -> Option<f32> {
        let shown = self.shown?;
        Some(match id {
            RATE_LANECOVER | RATE_LANECOVER2 => shown.lane_cover_share(),
            RATE_MUSIC_PROGRESS | RATE_MUSIC_PROGRESS_BAR => shown.music_progress(),
            _ => return None,
        })
    }

    fn float(&self, id: i32) -> Option<f32> {
        let shown = self.shown?;
        (id == FLOAT_HISPEED).then_some(shown.live.lanes.hispeed)
    }
}

#[cfg(test)]
mod tests;
