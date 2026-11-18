//! What the play screen lends the host for one frame.
//!
//! [`PlayShown::of`] reads a live [`PlaySession`] and the few things only the screen knows
//! ([`PlayLive`]) and makes plain values of them: the run so far held against the best and the
//! target, the gauge being played, the last judgement of each side, the lane figures, the clock and
//! how many notes the last second of the chart brought ([`notes_in_the_last_second`]). It is a pure
//! function of its two arguments. It reads no timer table, no configuration and no screen, and it
//! allocates nothing (the texts of [`RunSetup`] are shared, not copied), so the screen can make one
//! every frame and lend it to the host
//! ([`ScreenHost::show_play`](crate::skin_host::ScreenHost::show_play)).
//!
//! Everything in [`PlayLive`] is a figure the reference keeps in `BMSPlayer`, `LaneRenderer` or the
//! player's configuration and that the judge engine does not hold. Where the reference's figure is a
//! different number type from the one the screen has (the lane cover is a `float`, the tempo a
//! `double`), the field has the reference's type, so the skin reads the same truncation. The one
//! exception is [`RunSetup`], which the reference has no figure for at all.

use std::sync::Arc;

use rbms_model::{NoteKind, TimeLine};
use rbms_play::PlaySession;
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerState, timer_id};

use crate::skin_host::loading::{LoadingScreen, LoadingState};
use crate::skin_host::options::PlayedOptions;
use crate::skin_host::score::standing::PacedScores;
use crate::skin_host::score::{GaugeReading, RunScore};

/// How many sides a judgement is kept for: the first and second player's and the third's
/// (`JudgeManager.judgenow`, whose length is the number of judge areas the skin declares).
pub const PLAYER_SIDES: usize = 3;

/// The time a note takes to cross the lane at one beat per minute, in milliseconds
/// (`LaneRenderer.java:322`: `240000 / bpm / hispeed`).
pub const LANE_TRAVEL_BASE_MS: f64 = 240_000.0;

/// Where the player screen is in its life (`BMSPlayer.STATE_*`).
///
/// The reference's two practice states are not among them: this player sets a practice slice up on
/// a screen of its own, so its play screen is never choosing a practice range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayPhase {
    /// Loading the chart and its sounds (`STATE_PRELOAD`).
    #[default]
    Preload,
    /// The wait before the first note (`STATE_READY`).
    Ready,
    /// The chart is playing (`STATE_PLAY`).
    Play,
    /// The gauge has emptied and the run is closing (`STATE_FAILED`).
    Failed,
    /// Every note has gone by and the run is closing (`STATE_FINISHED`).
    Finished,
}

/// What kind of run this is (`BMSPlayerMode.Mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayKind {
    #[default]
    Play,
    Practice,
    Autoplay,
    Replay,
}

/// The last judgement given on one side (`JudgeManager.judgenow` and `judgefast`).
///
/// The reference keeps it until the next judgement on that side replaces it, so a side that has
/// been judged at least once always has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Judgement {
    /// Which judgement, as the reference numbers them: 0 PGREAT, 1 GREAT, 2 GOOD, 3 BAD, 4 POOR
    /// (a note that went by unhit) and 5 MISS (a press that took no note).
    pub judge: usize,
    /// How far ahead of the note the input came, in whole milliseconds cut towards zero
    /// (`mfast / 1000`). Early is positive and late is negative.
    pub timing_ms: i64,
}

/// The lane covers and the scroll speed as the player has them set (`PlayConfig`, read through
/// `LaneRenderer`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaneSettings {
    /// The scroll speed (`LaneRenderer.getHispeed`).
    pub hispeed: f32,
    /// How much of the lane the cover, the lift and the hidden band take, each as the share it is
    /// set to whether or not it is switched on (`getLanecover`, `getLiftRegion`, `getHiddenCover`).
    pub lane_cover: f32,
    pub lift: f32,
    pub hidden: f32,
    /// Whether each of the three is switched on.
    pub lane_cover_on: bool,
    pub lift_on: bool,
    pub hidden_on: bool,
    /// Whether the reference's lane renderer option of that name is on
    /// (`PlayConfig.isEnableConstant`): notes further ahead than a fixed time are not drawn. This
    /// player has no such option, so a play screen leaves it off. Its own CONSTANT setting is the
    /// reference's scroll assist, which the assist button reports
    /// ([`PlayedOptions::constant_scroll`]).
    pub constant_on: bool,
    /// Which tempo the green number is pinned to, as the reference numbers it (`PlayConfig.getFixhispeed`).
    pub fix_hispeed: i32,
}

/// The tempo the lanes are scrolling at, and the figures the lane renderer works the scroll out of.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Scroll {
    /// The tempo at the play head (`LaneRenderer.getNowBPM`).
    pub now_bpm: f64,
    /// The tempo most of the chart's notes are played at, the slowest and the fastest
    /// (`getMainBPM`, `getMinBPM`, `getMaxBPM`).
    pub main_bpm: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    /// How long a note takes to cross the whole lane at this moment with no cover over it, in
    /// milliseconds (`LaneRenderer.region`). See [`travel_region_ms`].
    pub region_ms: f64,
}

/// How long a note takes to cross the whole lane with no cover over it, for the tempo and the
/// scroll under the play head (`LaneRenderer.java:322`). A chart that scrolls backwards or not at all
/// gives none.
pub fn travel_region_ms(bpm: f64, hispeed: f32, scroll: f64) -> f64 {
    if scroll > 0.0 { LANE_TRAVEL_BASE_MS / bpm / f64::from(hispeed) / scroll } else { 0.0 }
}

/// The scene clock as far as the play figures need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayClock {
    /// The frame's time in microseconds.
    pub now_us: i64,
    /// When the play timer (41) switched on, or [`TIMER_OFF`].
    pub started_us: i64,
}

impl PlayClock {
    /// The clock of a frame at `now_us` over the scene's timers.
    pub fn of(timers: &TimerState, now_us: i64) -> PlayClock {
        PlayClock { now_us, started_us: timers.value_us(timer_id::PLAY) }
    }

    /// A clock with the play timer off.
    pub fn stopped(now_us: i64) -> PlayClock {
        PlayClock { now_us, started_us: TIMER_OFF }
    }

    /// How long the play timer has been on, in whole milliseconds (`TimerManager.getNowTime(id)`),
    /// or `None` while it is off.
    pub fn elapsed_ms(&self) -> Option<i64> {
        (self.started_us != TIMER_OFF).then(|| (self.now_us - self.started_us) / MICROS_PER_MILLI)
    }
}

impl Default for PlayClock {
    fn default() -> PlayClock {
        PlayClock::stopped(0)
    }
}

/// How many sides a run's lanes are arranged for: the first player's and, on a double play chart,
/// the second's.
pub const ARRANGED_SIDES: usize = 2;

/// What a run was started with and from, which no frame of it changes: settled once when the run
/// starts, from the run itself and not from the settings, which a run can be laid out against (a
/// layout borrowed from a replay, a replay being watched, a practice slice).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RunSetup {
    /// The folder of the song browser the run was started from, as its last two titles joined by
    /// `" / "` (a difficulty table's name and the level): empty for a run started anywhere else.
    pub origin: Arc<str>,
    /// The arrangement each side's lanes were laid out with, by the name the option panel gives it
    /// (`OFF`, `MIRROR`, `RANDOM` and the rest); empty for a side the chart has not.
    pub arrangements: [&'static str; ARRANGED_SIDES],
    /// The assists the run is played with, by the names the option panel gives them and joined by
    /// `" · "`: only the ones that are on, and empty when none is.
    pub assists: Arc<str>,
}

/// How far back from the judge line [`notes_in_the_last_second`] counts, in microseconds of the
/// chart.
pub const NOTES_PER_SECOND_WINDOW_US: i64 = 1_000_000;

/// How many notes reached the judge line in the second of the chart up to `chart_ms`, the play head:
/// the notes a player hits, a plain note or the head of a long note, timed after `chart_ms` less a
/// second and at or before `chart_ms`. A mine, a note the chart hides and the end of a long note are
/// not counted. Nothing has reached the line before the chart plays (`None`).
pub fn notes_in_the_last_second(timelines: &[TimeLine], chart_ms: Option<i64>) -> i32 {
    let Some(now_us) = chart_ms.map(|ms| ms.saturating_mul(MICROS_PER_MILLI)) else {
        return 0;
    };
    let first = timelines.partition_point(|timeline| timeline.time_us <= now_us.saturating_sub(NOTES_PER_SECOND_WINDOW_US));
    let last = timelines.partition_point(|timeline| timeline.time_us <= now_us);
    let hit = |kind: &NoteKind| matches!(kind, NoteKind::Normal | NoteKind::LongStart { .. });
    let count =
        timelines.get(first..last).unwrap_or_default().iter().flat_map(|timeline| timeline.notes.iter().flatten()).filter(|note| hit(&note.kind)).count();
    i32::try_from(count).unwrap_or(i32::MAX)
}

/// What only the play screen knows, for [`PlayShown::of`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayLive {
    pub phase: PlayPhase,
    pub kind: PlayKind,
    /// The best EX score the player had on the chart before this run, and the target's.
    pub best_score: u32,
    pub target_score: u32,
    /// Where those two stand by now, for one whose run is known note by note.
    pub paced_now: PacedScores,
    /// The last judgement of each side, `None` for one that has not been judged yet. The third side
    /// is a third player's, which a run here never has.
    ///
    /// The screen keeps these as the judgements come. The judge engine holds only the last one and
    /// whether it was early, with no side and no time, so a judgement it gives by itself -- a note
    /// that went by unhit, an auto-played lane -- has to be taken from what it reports after a tick.
    pub judgements: [Option<Judgement>; PLAYER_SIDES],
    pub lanes: LaneSettings,
    pub scroll: Scroll,
    pub clock: PlayClock,
    /// The judge timing offset in milliseconds (`PlayerConfig.getJudgetiming`).
    pub judge_timing_ms: i32,
    /// Whether the judge timing adjusts itself as inputs land
    /// (`PlayerConfig.isNotesDisplayTimingAutoAdjust`).
    pub judge_timing_auto: bool,
    /// Whether the background animation is on (`BMSResource.isBGAOn`).
    pub bga_on: bool,
    /// Whether START or SELECT is held, which is how the lane cover is being moved
    /// (`InputProcessor.startPressed() || isSelectPressed()`), or a control of this player's own
    /// has just moved a shade with neither held.
    pub cover_keys_held: bool,
    /// The long note mode the chart is played in, as the reference numbers it (0 long note, 1 charge
    /// note, 2 hell charge note).
    pub long_note_mode: i32,
    /// The random and double options this run was laid out with, and the assists it was played with.
    pub played: PlayedOptions,
    /// How much of the chart and its sounds has loaded, from nothing (0) to everything (1).
    pub load_progress: f32,
    /// The rate the run is played at, in percent of the chart's own speed, when that is not the
    /// chart's own speed: its PLAYBACK RATE, or a practice slice's FREQUENCY. The tempos and travel
    /// times of [`PlayLive::scroll`] are already the ones the player hears at it.
    pub changed_playback_percent: Option<i32>,
    /// Where the run was started from and what it was laid out and played with.
    pub setup: RunSetup,
}

/// The play screen's frame, as the host reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayShown {
    pub live: PlayLive,
    /// The run so far.
    pub run: RunScore,
    /// The gauge being played.
    pub gauge: GaugeReading,
    /// Whether that gauge is at or over its border (`Gauge.isQualified`).
    pub qualified: bool,
    /// How long the reference keeps the chart in its playing state, in milliseconds
    /// (`BMSPlayer.getPlaytime`).
    pub play_time_ms: i32,
    /// How many notes reached the judge line in the last second of the chart, by the play timer
    /// ([`notes_in_the_last_second`]).
    pub notes_per_second: i32,
}

impl PlayShown {
    /// The frame of `session` with what the screen adds to it.
    pub fn of(session: &PlaySession, live: &PlayLive) -> PlayShown {
        let judge = session.judge();
        PlayShown {
            live: live.clone(),
            run: RunScore::of_session(session, live.best_score, live.target_score, live.paced_now),
            gauge: GaugeReading::of_engine(judge),
            qualified: judge.gauge.selected().is_cleared(),
            play_time_ms: i32::try_from(session.play_time_ms()).unwrap_or(i32::MAX),
            notes_per_second: notes_in_the_last_second(&session.model().timelines, live.clock.elapsed_ms()),
        }
    }

    /// The load the screen is in, for the loading options and the progress figures.
    pub fn loading(&self) -> LoadingState {
        let screen = if self.live.phase == PlayPhase::Preload { LoadingScreen::Preload } else { LoadingScreen::Started };
        LoadingState { screen, progress: self.live.load_progress }
    }
}

#[cfg(test)]
mod tests;
