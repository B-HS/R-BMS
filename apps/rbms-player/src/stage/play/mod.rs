//! The PLAY screen: the running session, the song clock that drives it, the replay-analysis
//! controls, and the result the run is turned into when it ends.
//!
//! The screen runs one of two ways, settled when it is entered.
//!
//! With no skin for the chart's mode it is what it always was: the chart was waited for on the
//! LOADING screen, the run starts the moment this screen is up, and it leaves for the result the
//! moment the run is over. Nothing about that path is touched by the other.
//!
//! With a skin it runs the reference's states ([`scene`]): it loads here, on the skin's own loading
//! state, stands ready for as long as the skin says, plays, and closes by one of the two ways the
//! skin animates -- the failure, or the fade after the last note -- before the result. The song
//! clock starts when the chart starts playing and not before. While it runs, the screen tells the
//! skin's timer driver what happens and hands the skin's note field, judge object and property
//! clusters a frame of the run ([`skin`], [`trace`]).
//!
//! What is this player's own on that path:
//!
//! - **A practice slice** has nothing to load, so it begins ready, and its play timer starts as far
//!   into the chart as the slice does.
//! - **A replay under analysis** carries the scene clock with it: paused, slowed or scrubbed, the
//!   scene's time is the play timer's start plus the place in the chart, so everything a skin
//!   animates stands still, slows and jumps with the notes. A jump switches every lane's timers off.
//! - **The chart preview**: holding START or SELECT while the chart loads scrolls the chart past,
//!   on timer 141, as the reference does it.
//! - **CONSTANT and LEGACY NOTE** change what a skin's note field is shown and nothing of the run:
//!   the field is handed the chart made over for them ([`field`]), so the same two settings move the
//!   notes of a skin's field as they move the built-in one's.
//! - **A held green number** (the KEEP GREEN NUMBER row): the run starts at the scroll speed that
//!   shows the travel time the settings hold, at this chart's tempo and under the lane cover as it
//!   stands, and is put back there whenever the cover moves ([`PlayState::hold_green_number`]). It
//!   is the reference's own rule for a speed fixed to a tempo (`LaneRenderer.resetHispeed`); what is
//!   this player's is that the row can be off, when the speed itself is what the settings hold.
//! - **The judge offset** (the JUDGE OFFSET row) is the reference's: the note field is drawn that
//!   far ahead of the song, on a skin's field and on the built-in one alike
//!   ([`PlayState::field_lead_us`]), and no input and no sound is moved by it. With the AUTO ADJUST
//!   row on it moves as each input of a run played by hand lands, and what it has moved to is the
//!   settings' own value from that moment ([`PlayState::keep_adjusted_offset`]).
//! - **Leaving** is by Escape, on the terms of the ESCAPE setting, as it always was. While a run is
//!   closing Escape skips what is left of the closing instead. A failed run played by hand starts
//!   again at once on START or SELECT, as in the reference.
#![allow(clippy::wildcard_imports)]

mod controls;
mod field;
#[cfg(test)]
mod hold_tests;
#[cfg(test)]
mod input_time_tests;
mod key_gate;
mod scene;
#[cfg(test)]
mod screen_tests;
mod skin;
pub(crate) mod skin_preview;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_capture_tests;
#[cfg(test)]
mod timing_tests;
mod trace;

use crate::app_input::{ControlContext, ControlEffect, FixedSpeed, green_for_hispeed};
use crate::app_play::schedule_position_us;
use crate::app_result::{enter_result, run_target};
use crate::keyconfig::{Shortcut, ShortcutScreen, mode_config_key};
use crate::skin_host::options::SettingsView;
use crate::skin_host::play::{LaneSettings, PlayClock, PlayKind, PlayLive, PlayPhase, PlayShown, Scroll, travel_region_ms};
use crate::skin_host::play_timers::{CHART_PREVIEW, FieldOffsets, PlayingFrame, SceneEvent};
use crate::skin_screen::PlayDraw;
use crate::stage::loading::ChartLoads;
use crate::stage::{Canvas, FrameCtx, KeyInput, StageHandler, Transition};
use crate::target::ResolvedTarget;
use crate::*;
use controls::HeldControls;
use field::FieldChart;
use rbms_config::FixHiSpeed;
use rbms_render::playfield::LaneShade;
use rbms_render::skin_render::frame::{
    BgaExpand, BgaPlayhead, BgaPreloadOrder, BgaTextures, DEFAULT_MISS_LAYER_DURATION_MS, GaugeScale, LaneNotes, first_lane_rect, lane_offsets,
};
use rbms_render::skin_render::graphs::{NoteDistribution, PlayCursor};
use rbms_render::{
    BgaFrame, FrameData, FrameSeries, GaugeFrame, HudPace, KEY_LANE_KIND, LANE_KIND_COUNT, ReferenceImages, SCRATCH_LANE_KIND, TextureId, render_hud,
};
use rbms_skin::timer::timer_id;
use scene::{Leave, PlayScene, PlayTimes, RunEnd, SceneFacts, SceneStep};
use skin::{SkinFeed, Standing};

/// Microseconds in one millisecond, which is the unit the play clock is kept in and the unit a
/// document reads the play head in.
const MICROS_PER_MILLI: i64 = 1_000;

/// How long Escape has to be held down to abandon a run under [`PlayEscape::Hold`].
const ESC_HOLD: Duration = Duration::from_millis(PLAY_ESCAPE_HOLD_MS);

/// How long a second Escape still counts as the other half of a pair under [`PlayEscape::Double`].
const ESC_DOUBLE: Duration = Duration::from_millis(PLAY_ESCAPE_DOUBLE_MS);

/// How many of the most recent judged inputs the analysis overlay shows.
const ANALYSIS_MARKS_SHOWN: usize = 14;

/// The play speed of a run nobody is driving by hand, in percent (`BMSPlayer.playspeed`).
const FULL_SPEED_PERCENT: f64 = 100.0;

/// How much of a chart is in when everything it named is.
const LOADED: f32 = 1.0;

/// What the play clock reads for a chart that has not started, as the background reads it
/// (`SkinBGA.prepare`).
const BEFORE_PLAY_MS: i64 = -1;

/// The key the chart's stage picture is registered with the renderer under.
const STAGE_TEXTURE_KEY: &str = "rbms.player.play.stagefile";

/// Height of the analysis overlay strip along the bottom of the screen.
const ANALYSIS_PANEL_H: f32 = 74.0;

/// Horizontal inset of the analysis playback bar.
const ANALYSIS_BAR_INSET: f32 = 40.0;

/// The tempos a chart is played at, so a green number can be pinned to one of them.
///
/// The four are the reference implementation's own (`LaneRenderer.java:127-144`): the tempo the
/// chart opens at, its slowest, its fastest, and the one the most notes are played at. Which of them
/// a run uses is the SPEED FIX row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BpmStats {
    start: f64,
    min: f64,
    max: f64,
    main: f64,
}

impl BpmStats {
    /// Read the four tempos off a chart.
    ///
    /// The main tempo is the one carrying the most notes. A chart that splits its notes evenly
    /// between two tempos keeps the first of them, where the reference leaves the choice to hash
    /// order; a run has to pick the same tempo every time it loads the chart, or its pinned speed
    /// would move between two launches of the same file.
    pub(crate) fn of(model: &rbms_model::Model) -> BpmStats {
        let mut stats = BpmStats { start: model.init_bpm, min: model.init_bpm, max: model.init_bpm, main: model.init_bpm };
        let mut by_bpm: Vec<(f64, u32)> = Vec::new();
        for tl in &model.timelines {
            stats.min = stats.min.min(tl.bpm);
            stats.max = stats.max.max(tl.bpm);
            let notes = tl.notes.iter().flatten().filter(|n| !matches!(n.kind, rbms_model::NoteKind::Mine { .. })).count() as u32;
            match by_bpm.iter_mut().find(|(bpm, _)| *bpm == tl.bpm) {
                Some((_, count)) => *count += notes,
                None => by_bpm.push((tl.bpm, notes)),
            }
        }
        let mut most = 0;
        for (bpm, count) in by_bpm {
            if count > most {
                most = count;
                stats.main = bpm;
            }
        }
        stats
    }

    /// The tempo the SPEED FIX row pins to, or `None` when it pins to nothing.
    pub(crate) fn target(&self, fix: FixHiSpeed) -> Option<f64> {
        match fix {
            FixHiSpeed::Off => None,
            FixHiSpeed::StartBpm => Some(self.start),
            FixHiSpeed::MinBpm => Some(self.min),
            FixHiSpeed::MaxBpm => Some(self.max),
            FixHiSpeed::MainBpm => Some(self.main),
        }
    }

    /// The tempo a green number held from chart to chart is held at on this chart, or `None` while
    /// the KEEP GREEN NUMBER row is off.
    ///
    /// It is the tempo the HI-SPEED FIX row names, and the one the most notes are played at while
    /// that row names none, so a held green number always has a tempo to be held against. Under
    /// CONSTANT the field scrolls at one tempo of its own whatever the chart's are
    /// ([`field::CONSTANT_BPM`]), and that is the tempo the number is held at, so the number on show
    /// is the one that was asked for.
    pub(crate) fn held(&self, play: &rbms_config::PlayOptions) -> Option<f64> {
        if !play.keep_green_number {
            return None;
        }
        Some(if play.constant_speed { field::CONSTANT_BPM } else { self.target(play.fix_hispeed).unwrap_or(self.main) })
    }
}

/// What this run's scroll speed is being held to.
///
/// `base` is the speed the chart started at, which is what a pinned green number steps in multiples
/// of. `fixed` is the green number being held, and the tempo it is held at, for a run whose SPEED
/// FIX row names one.
#[derive(Clone, Copy, Debug)]
struct RunSpeed {
    base: f64,
    fixed: Option<FixedSpeed>,
}

/// The EX a target would hold this far into a run: its final EX scaled by how much of the chart has
/// been judged, so what the pacemaker compares is the run against the target's pace rather than
/// against its finished total.
fn pace_ex_at(target_ex: u32, judged: u32, total: u32) -> i64 {
    if total == 0 {
        return 0;
    }
    (u64::from(target_ex) * u64::from(judged) / u64::from(total)) as i64
}

/// The run in progress: the session (judge state, replay, recording, analysis clock and BGA
/// timeline), the chart's decoded BGA images, and the LN mode the score submission reports.
pub(crate) struct PlayState {
    pub(crate) session: PlaySession,
    bga: std::collections::HashMap<i32, crate::DecodedImage>,
    /// IR `lntype` of this run (0=LN, 1=CN, 2=HCN): the chart's own `#LNMODE` when it states one,
    /// and the LN MODE the run was judged under when it does not.
    pub(crate) lntype: i32,
    /// The long-note key this run's record is compared within, so a chart forced to charge notes —
    /// twice the judged objects, twice the EX ceiling — does not overwrite the best of the same
    /// chart played as plain long notes.
    pub(crate) ln_mode_key: String,
    /// Song position this frame, taken once in `update` and reused by `draw`.
    song_us: i64,
    /// When Escape went down and is still down, for the mode that asks for it to be held.
    esc_down_at: Option<Instant>,
    /// When Escape was last pressed, for the mode that asks for two presses.
    esc_pressed_at: Option<Instant>,
    /// The four tempos of this chart, read off it once.
    bpm: BpmStats,
    /// What the scroll speed is being held to, taken from the settings the first time this run
    /// needs it. The screen is built before it can see them, so it is filled in on first use rather
    /// than in the constructor.
    speed: Option<RunSpeed>,
    /// When a control of the run last moved or switched a shade, on the clock a skin is animated
    /// against, for the figures a skin shows while a shade is being changed ([`shade_is_changing`]).
    shade_moved_us: Option<i64>,
    /// The target this run is paced against, settled the first time the HUD needs it. It is fixed
    /// for the run: the records it is settled against are the ones that stood when the run started,
    /// which is what the result screen scores it against too.
    pace_target: Option<ResolvedTarget>,
    /// The best EX score the player had on the chart when that target was settled, which is what
    /// a skin's play screen holds the run against beside the target.
    best_score: u32,
    /// The practice slice this run is, when it is one. The run ends at the slice's end time rather
    /// than at the last note, and an emptied gauge does not stop it ([`PlayState::set_practice`]).
    pub(crate) practice: Option<crate::practice::PracticeSession>,
    /// The states a skin's play screen runs through, when a skin draws this run. `None` is the run
    /// as it is without a skin: it starts the moment the screen is up and ends the moment it is
    /// over.
    scene: Option<PlayScene>,
    /// What the chart named that is still decoding, which the scene's loading state waits for.
    loads: Option<ChartLoads>,
    /// What the skin that draws this run is fed from. `None` while nothing draws it but the
    /// built-in field.
    feed: Option<Box<SkinFeed>>,
    /// What the reference's START and SELECT controls keep between frames.
    controls: HeldControls,
    /// Which of each lane's two keys the player is holding, forward and backward, whether or not
    /// the run is taking input.
    held: Vec<(bool, bool)>,
    /// Which of those presses the run was handed, so a key that went down before the chart began
    /// playing is not handed to the run as a release with no press behind it.
    sent: Vec<(bool, bool)>,
    /// The keyboard's lane keys, debounced by KEY DEBOUNCE before a lane hears of them.
    key_gates: key_gate::KeyGates,
    /// Where the chart's background stands, as a skin's `bga` object shows it: the picture, the
    /// layer over it and the miss layer, walked as the play timer passes the chart's events.
    background: BgaPlayhead,
    /// The textures those pictures are shown from.
    background_textures: BgaTextures,
    /// The chart's stage picture, shown by a skin while the chart loads.
    stagefile: Option<crate::DecodedImage>,
    /// The stage picture once the renderer has it.
    stage_texture: Option<TextureId>,
    /// Whether the screen has drawn a frame, which is the first place its skin can be asked for.
    drawn: bool,
    /// The combo the run began its chart with: what a course carried over from the stage before,
    /// and nothing anywhere else.
    carried_combo: u32,
    /// The chart as a skin's note field is shown it while CONSTANT or LEGACY NOTE is on, made over
    /// the first time a frame asks for it and again when either setting moves.
    field: Option<FieldChart>,
}

/// The frame `PlaySession::bga_frame` reports before the chart's first background event, which is
/// what a chart with no events of its own stays on.
#[cfg(test)]
pub(crate) const NO_BGA_FRAME: i32 = -1;

/// What a play document needs from the frame that the built-in field also works out: the tempo
/// under the play head, the tempo and the scroll rate its note field scrolls at, the scroll speed,
/// and the background image this frame decoded.
#[derive(Clone, Copy)]
struct DocumentFrame {
    bpm: f64,
    /// The tempo and the scroll rate the note field scrolls at: the chart's own under the play
    /// head, or the one a field under CONSTANT keeps from first to last.
    travel: (f64, f64),
    hispeed: f64,
    background: Option<TextureId>,
}

/// A whole millisecond of a time in microseconds, cut the way the reference cuts every time it
/// reads (`TimerManager.getNowTime`).
const fn whole_ms(micros: i64) -> i64 {
    micros / MICROS_PER_MILLI
}

impl PlayState {
    pub(crate) fn new(session: PlaySession, bga: std::collections::HashMap<i32, crate::DecodedImage>, lntype: i32, ln_mode_key: String) -> PlayState {
        let bpm = BpmStats::of(session.model());
        let background = BgaPlayhead::of_chart(&session.model().timelines);
        let carried_combo = session.judge().combo;
        PlayState {
            session,
            bga,
            lntype,
            ln_mode_key,
            song_us: 0,
            esc_down_at: None,
            esc_pressed_at: None,
            bpm,
            speed: None,
            shade_moved_us: None,
            pace_target: None,
            best_score: 0,
            practice: None,
            scene: None,
            loads: None,
            feed: None,
            controls: HeldControls::default(),
            held: Vec::new(),
            sent: Vec::new(),
            key_gates: key_gate::KeyGates::default(),
            background,
            background_textures: BgaTextures::default(),
            stagefile: None,
            stage_texture: None,
            drawn: false,
            carried_combo,
            field: None,
        }
    }

    /// Hand this screen the decodes of its chart that are still running. A screen a skin draws
    /// waits for them in its own loading state; they are polled from the frame it is entered on.
    pub(crate) fn wait_for(&mut self, loads: ChartLoads) {
        self.loads = (!loads.is_done()).then_some(loads);
    }

    /// The state a skin's play screen is in, or `None` for a run no scene is run over.
    #[cfg(test)]
    pub(crate) fn phase(&self) -> Option<PlayPhase> {
        self.scene.as_ref().map(PlayScene::phase)
    }

    /// Where the chart stood on the last frame of the run, in whole milliseconds.
    #[cfg(test)]
    pub(crate) fn chart_ms(&self) -> i64 {
        whole_ms(self.song_us)
    }

    #[cfg(test)]
    pub(crate) fn bga_count(&self) -> usize {
        self.bga.len()
    }

    /// Mark this run as a practice slice, which ends it on the slice's own end time, and judge it
    /// the way the slice is played: in the player's own time at the slice's FREQUENCY, and under a
    /// locked gauge, which keeps the judge going once the gauge empties rather than stopping it with
    /// the slice still running. Its background is uploaded ahead in the slice's own order, since the
    /// plan its pictures' decodes carry was made for the whole chart ([`BgaPreloadOrder::Part`]).
    /// Call it before the first frame.
    pub(crate) fn set_practice(&mut self, practice: crate::practice::PracticeSession) {
        self.session.set_playback_rate(practice.freq_percent);
        let mut judge = self.session.judge_setup();
        let shift = practice.gauge_shift(judge.gauge_auto_shift);
        if shift != judge.gauge_auto_shift {
            judge.gauge_auto_shift = shift;
            self.session.set_judge_setup(judge);
        }
        self.background_textures.set_order(BgaPreloadOrder::Part);
        self.practice = Some(practice);
    }

    fn practice_clock(&self) -> crate::practice::PracticeClock {
        self.practice.as_ref().map(crate::practice::PracticeClock::from_session).unwrap_or_else(crate::practice::PracticeClock::normal)
    }

    /// Early hits of this run, split into the keys and the turntable.
    ///
    /// The split is kept by the session rather than here because not every input comes through this
    /// screen: a replay reproduces its own and an auto-played lane is judged inside the session, and
    /// a tally kept here would report both as nothing at all.
    ///
    /// It counts inputs, so it is not the judge engine's own `fast`/`slow`: those include the notes
    /// an auto-played lane resolved, which nobody hit. The debug overlay reports both, side by side.
    pub(crate) fn fast(&self) -> [u32; LANE_KIND_COUNT] {
        PlayState::lane_kinds(self.session.instrumentation().fast())
    }

    /// Late hits of this run, split the same way as [`PlayState::fast`].
    pub(crate) fn slow(&self) -> [u32; LANE_KIND_COUNT] {
        PlayState::lane_kinds(self.session.instrumentation().slow())
    }

    /// Put the session's two columns where the screens expect to read them, naming both ends, so
    /// the two crates' orderings cannot quietly swap the keys and the turntable.
    fn lane_kinds(split: [u32; rbms_play::LANE_KIND_COUNT]) -> [u32; LANE_KIND_COUNT] {
        let mut out = [0; LANE_KIND_COUNT];
        out[KEY_LANE_KIND] = split[rbms_play::KEY_LANE_KIND];
        out[SCRATCH_LANE_KIND] = split[rbms_play::SCRATCH_LANE_KIND];
        out
    }

    /// What this run's scroll speed is being held to, filled in from the settings on first use.
    ///
    /// A run whose SPEED FIX row names a tempo is pinned to the travel time it starts with: the
    /// speed the settings hold, read at that tempo, becomes the number the run keeps
    /// (`LaneRenderer.java:155-157` takes the same reading for its own base speed).
    fn run_speed(&mut self, shared: &AppShared) -> RunSpeed {
        if let Some(speed) = self.speed {
            return speed;
        }
        let base = shared.config.play.hispeed;
        let fixed =
            self.bpm.target(shared.config.play.fix_hispeed).map(|bpm| FixedSpeed { bpm, green: green_for_hispeed(bpm, base, shared.effective_cover()) });
        let speed = RunSpeed { base, fixed };
        self.speed = Some(speed);
        speed
    }

    /// Settle the target this run is paced against, once, from the records that stand right now.
    ///
    /// It is fixed for the rest of the run: the records it is settled against are the ones the
    /// result screen scores the run against too, so the line the HUD paces to and the line the
    /// summary reports are the same one.
    fn ensure_pace_target(&mut self, shared: &AppShared) {
        if self.pace_target.is_some() {
            return;
        }
        let md5 = self.session.model().md5.clone();
        let total_notes = self.session.judge().total_notes();
        let best = shared.scores.best_ex_for_md5_in_ln_mode(&md5, &self.ln_mode_key);
        self.best_score = best.unwrap_or_default();
        self.pace_target = Some(run_target(shared, &md5, total_notes, best));
    }

    /// The tempo this run's green number is held at ([`BpmStats::held`]), or `None` for a run that
    /// holds none: the KEEP GREEN NUMBER row is off, or the course in progress locks the speed.
    fn held_tempo(&self, shared: &AppShared) -> Option<f64> {
        if shared.course_locks_speed() {
            return None;
        }
        self.bpm.held(&shared.config.play)
    }

    /// Put the scroll speed where the held green number wants it for this chart and the lane as it
    /// stands, and answer whether this run holds one.
    ///
    /// The speed of such a run is never a value of its own: it is what the travel time, the tempo
    /// and the lane cover come to ([`rbms_config::held_speed`]), worked out when the chart starts
    /// and again whenever any of them has moved. A speed that would leave its range stands at the
    /// end of it, and the figures a skin reads say what the travel time then really is.
    ///
    /// While the number is held the run has no pin of its own ([`PlayState::run_speed`]): whatever
    /// one it read before is forgotten, so a run that stops holding its number reads its pin from
    /// the speed the number left it at, and not from the one it had when something first asked.
    fn hold_green_number(&mut self, shared: &mut AppShared) -> bool {
        let Some(bpm) = self.held_tempo(shared) else {
            return false;
        };
        shared.config.play.hold_travel_at(bpm);
        self.speed = None;
        true
    }

    /// Apply one in-play control and keep a pinned green number in step with what it moved.
    ///
    /// Moving the speed itself re-reads the green number the run is now at, so the next lane-cover
    /// change holds the number the player just chose. Moving the cover holds the number and moves
    /// the speed instead, which is what keeps a covered field reading the same as an uncovered one
    /// (`LaneRenderer.java:212-215`).
    ///
    /// A run that holds its green number from chart to chart takes its controls another way
    /// ([`PlayState::held_green_control`]).
    fn in_play_control(&mut self, shared: &mut AppShared, action: ControlAction) {
        if action.moves_a_shade() && !shared.course_locks_speed() {
            self.shade_moved_us = Some(shared.skin_now_us());
        }
        if self.green_control(shared, action) || self.held_green_control(shared, action) {
            return;
        }
        let speed = self.run_speed(shared);
        let ctx = ControlContext { fine: shared.fine_pressed(), base_hispeed: speed.base, fixed: speed.fixed };
        match shared.apply_control(action, &ctx) {
            ControlEffect::Speed => {
                let refreshed =
                    speed.fixed.map(|fixed| FixedSpeed { green: green_for_hispeed(fixed.bpm, shared.config.play.hispeed, shared.effective_cover()), ..fixed });
                self.speed = Some(RunSpeed { fixed: refreshed, ..speed });
            }
            ControlEffect::Shade => {
                if let Some(fixed) = speed.fixed {
                    shared.retarget_hispeed(fixed);
                }
            }
            ControlEffect::None => {}
        }
    }

    /// Hand the running session the judge settings as they stand right now, so an offset that
    /// adjusts itself goes on from a JUDGE OFFSET changed mid-song, and an AUTO ADJUST switched
    /// mid-song takes effect on the very next input rather than on the next chart.
    fn sync_judge_settings(&mut self, shared: &AppShared) {
        self.session.set_judge_offset_us(shared.offset_us());
        self.session.set_auto_calibration(shared.config.judge.auto_offset);
    }

    /// Keep the judge offset an input of the player's just moved, while the offset adjusts itself.
    ///
    /// The reference writes each move straight into the player's configuration
    /// (`JudgeManager.java:701`), so it stands however the run ends: finished, failed or abandoned,
    /// and the note field, which reads the configuration, is drawn by it from the next frame.
    /// Here the configuration is written out on each of those ways out of the screen, and what it
    /// holds by then is what the last input left.
    fn keep_adjusted_offset(&mut self, shared: &mut AppShared) {
        shared.config.judge.offset_ms = self.session.judge_offset_ms();
    }

    /// How far ahead of the song the note field is drawn, in microseconds of the chart: the judge
    /// offset, which is the reference's `judgetiming` and is added to the time its field is drawn
    /// at and to nothing else (`LaneRenderer.java:300-301`). At +10 ms a note is on the judge line
    /// 10 ms before its sound and before the instant a press of it is on time.
    ///
    /// The offset is a length of the player's own time. A practice slice played slower or faster
    /// goes through less or more of the chart in it, which is what the reference comes to by
    /// stretching the chart instead of the clock (`BMSModelUtils.changeFrequency`).
    fn field_lead_us(&self, shared: &AppShared) -> i64 {
        let clock = self.practice_clock();
        clock.chart_time_us(shared.offset_us()) - clock.chart_time_us(0)
    }

    /// Where the chart stands for the built-in field, which is drawn off the song clock: the song
    /// position of this frame and the judge offset ahead of it.
    fn built_in_field_us(&self, shared: &AppShared) -> i64 {
        self.song_us + self.field_lead_us(shared)
    }

    /// Whether the run is over and the result is due: the song ended, or the gauge emptied under a
    /// shift mode that does not rescue it — the reference moves to `STATE_FAILED` and stops judging
    /// there (`BMSPlayer.java:653-661, 694`) rather than tallying a dead run to the end of the song.
    ///
    /// A replay being scrubbed never ends on its own, failed or not: the player is driving the
    /// clock and can scrub back out of it.
    fn run_is_over(&self, song_us: i64) -> bool {
        if let Some(practice) = &self.practice {
            return practice.is_past_end(song_us);
        }
        self.session.is_finished(song_us) || (!self.session.analysis_enabled() && self.session.is_failed())
    }

    /// Escape while a chart is still running.
    ///
    /// A run is abandoned outright, on a press held for [`ESC_HOLD`], or on a second press within
    /// [`ESC_DOUBLE`], depending on the ESCAPE setting. The two guarded modes are what decision 6
    /// asks for; the immediate one is what the key has always done and is what a fresh install
    /// holds, so the setting adds a guard rather than moving one.
    ///
    /// Every mode acts on the press, as every other screen in the app does. A release carried over
    /// from the screen before — the Escape that left it — would otherwise abandon the run the
    /// moment it started.
    fn escape_key(&mut self, ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Transition {
        match ctx.shared.config.play.play_escape {
            PlayEscape::Immediate => {
                if !key.pressed {
                    return Transition::Stay;
                }
                self.leave_run(ctx)
            }
            PlayEscape::Hold => {
                if key.released {
                    self.esc_down_at = None;
                } else if key.pressed {
                    self.esc_down_at = Some(ctx.now);
                }
                Transition::Stay
            }
            PlayEscape::Double => {
                if !key.pressed {
                    return Transition::Stay;
                }
                let paired = self.esc_pressed_at.is_some_and(|first| ctx.now.duration_since(first) <= ESC_DOUBLE);
                if paired {
                    self.esc_pressed_at = None;
                    return self.leave_run(ctx);
                }
                self.esc_pressed_at = Some(ctx.now);
                Transition::Stay
            }
        }
    }

    /// Whether Escape has now been held long enough to abandon the run. Checked every frame, since
    /// a key that is simply held down produces no further events.
    fn escape_hold_elapsed(&self, now: Instant) -> bool {
        self.esc_down_at.is_some_and(|since| now.duration_since(since) >= ESC_HOLD)
    }

    /// Abandon the run: keep whatever the in-play controls changed, and go back to the browser.
    ///
    /// A chart with nothing left to hit goes to its result screen instead, and so does a run that is
    /// already closing, failed or finished: it ends as every closing run does, with the notes it did
    /// not reach counted as the closing counted them, whichever key or hold took it out. That holds
    /// for a course stage too, which then goes on to the next stage or the course's result as a stage
    /// played to its end does.
    ///
    /// A run that has judged nothing at all was never played, and nothing is kept of it
    /// ([`Self::has_judged_nothing`], the player's decision of 2026-10-19): a single chart goes back
    /// to the browser, and a course ends where it stands, with the stages played before it kept and
    /// sent as a FAILED course ([`crate::app_result::leave_course`]).
    ///
    /// Otherwise a run the player was playing by hand is kept and sent as a FAILED run whose notes
    /// not reached were let pass ([`crate::app_result::record_left_run`]), straight on the way out:
    /// Escape is still the key that leaves at once. That is so however few notes the run judged
    /// (the player's decision of 2026-10-15): one note let pass unhit, or one empty POOR, is a
    /// judgment. A course left part-way keeps the stage it was left on that way and is kept and sent
    /// as a FAILED course with every stage it played. Nothing is kept of a run the engine played, a
    /// replay being watched or practice.
    fn leave_run(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.esc_down_at = None;
        self.esc_pressed_at = None;
        ctx.shared.play_system_sound(SystemSound::PlayStop);
        ctx.shared.save_settings();
        if self.practice.is_some() {
            return Transition::Back;
        }
        if self.has_ended() {
            return enter_result(self, ctx.shared);
        }
        if ctx.shared.course_run.is_some() {
            return crate::app_result::leave_course(self, ctx.shared);
        }
        if keeps_a_left_run(ctx.shared) && !self.has_judged_nothing() {
            crate::app_result::record_left_run(self, ctx.shared);
        }
        ctx.shared.leave_play()
    }

    /// Whether the run has nothing left to play: every note has gone by, or it is closing, failed
    /// or finished. Leaving such a run ends it on its result rather than as a run left part-way.
    pub(crate) fn has_ended(&self) -> bool {
        self.session.all_notes_resolved() || self.scene.as_ref().is_some_and(PlayScene::is_closing)
    }

    /// Whether the player has hit nothing in the run: no note, long note head or end has been
    /// judged PGREAT, GREAT, GOOD or BAD, and no lane holds a long note whose head was taken. A
    /// note that went by unhit (POOR) and a press that met no note (empty POOR) are not hits, so a
    /// run that has only those has hit nothing (the player's decision of 2026-10-19).
    pub(crate) fn has_judged_nothing(&self) -> bool {
        let judge = self.session.judge();
        let hit = &judge.counts[..rbms_judge::Judge::Poor as usize];
        hit.iter().all(|&count| count == 0) && (0..self.session.beam_on().len()).all(|lane| judge.long_note(lane).processing.is_none())
    }

    /// Handle an analysis-mode playback key (only while a replay analysis is active): pause/resume,
    /// playback rate, and one seek step either way, each on the key the key configuration gives it.
    /// Returns whether the key was an analysis control.
    fn analysis_key(&mut self, shared: &mut AppShared, code: KeyCode) -> bool {
        match shared.shortcut_for(ShortcutScreen::Replay, code) {
            Some(shortcut) => self.analysis_shortcut(shared, shortcut),
            None => {
                self.sync_judge_settings(shared);
                false
            }
        }
    }

    /// Carry out one control of a replay under analysis, from a key or from a controller. Returns
    /// whether a replay is under analysis for it to act on; a seek is also told to the skin.
    fn analysis_shortcut(&mut self, shared: &mut AppShared, shortcut: Shortcut) -> bool {
        self.sync_judge_settings(shared);
        if !self.session.analysis_enabled() {
            return false;
        }
        let play = &mut self.session;
        match shortcut {
            Shortcut::ReplayPause => play.toggle_analysis_pause(),
            Shortcut::ReplayFaster => play.adjust_analysis_rate(1),
            Shortcut::ReplaySlower => play.adjust_analysis_rate(-1),
            Shortcut::ReplayForward => play.seek(play.analysis_position_us() + ANALYSIS_SEEK_STEP_US),
            Shortcut::ReplayBack => play.seek(play.analysis_position_us() - ANALYSIS_SEEK_STEP_US),
            _ => return false,
        }
        if matches!(shortcut, Shortcut::ReplayForward | Shortcut::ReplayBack) {
            self.skin_seeked(shared);
        }
        true
    }

    /// A jump of a replay under analysis, as a skin sees it: the scene clock goes where the chart
    /// went, every lane's timers go off, and the judgements on show go with them. The background
    /// starts over as a chart that is yet to play, so the frame after passes every change up to
    /// where the run landed, the ones at the chart's very start among them.
    fn skin_seeked(&mut self, shared: &mut AppShared) {
        let chart_us = self.session.analysis_position_us();
        if self.scene.is_some() && shared.skin_timers.is_on(timer_id::PLAY) {
            shared.set_skin_scene_clock(shared.skin_timers.value_us(timer_id::PLAY) + chart_us);
        }
        let now_us = shared.skin_now_us();
        if let Some(feed) = self.feed.as_mut() {
            feed.seeked(&mut shared.skin_timers, now_us, chart_us);
        }
        self.background.reset();
        self.background.prepare(BEFORE_PLAY_MS);
    }

    /// The lane press/release path: judged against the song clock as it stood at `at`, the instant
    /// the window handed the key over, with the keysound played on the raw input time so a judge
    /// offset never moves the sound.
    ///
    /// Without an output stream the press still goes through the session, into a sink that drops
    /// it. The session is what records the replay, so skipping the call would leave a run with
    /// releases but no presses — a replay that cannot be played back.
    ///
    /// A lane key goes through KEY DEBOUNCE first ([`key_gate`]): a press is played where it came,
    /// a release inside the window after the press is kept out and played by
    /// [`PlayState::take_opened_keys`] when the window ends, and a press within one of LR2's polls of
    /// a release is the switch bouncing and is not played.
    fn lane_key(&mut self, shared: &mut AppShared, key: &KeyInput<'_>, at: Instant) {
        let Some((lane, dir)) = shared.lane_input_for(key.code) else {
            return;
        };
        if !key.pressed && !key.released {
            return;
        }
        if self.key_gates.offer(key.code, key.pressed, at, shared.config.play.key_debounce_ms) {
            self.lane_input(shared, lane, dir, key.pressed, at);
        }
    }

    /// Play every lane key release KEY DEBOUNCE kept out and whose window has ended by `until`, each
    /// at the instant its window ended. Whatever hands the run an input or a frame at `until` calls
    /// this first, so the run takes its inputs in the order of their instants.
    fn take_opened_keys(&mut self, shared: &mut AppShared, until: Instant) {
        for (code, at) in self.key_gates.take_opened(until, shared.config.play.key_debounce_ms) {
            if let Some((lane, dir)) = shared.lane_input_for(code) {
                self.lane_input(shared, lane, dir, false, at);
            }
        }
    }

    /// One lane press or release, whichever device it arrived on. A controller cannot make a
    /// [`KeyCode`], so it enters here rather than through [`PlayState::lane_key`], and the guards
    /// that decide whether a run takes input at all live here so both entrances share them.
    ///
    /// A run a scene is run over takes input only while its chart is playing. A key that went down
    /// before that is still a key that is down -- a skin lights its beam -- but the run never hears
    /// of it, and so never of its release either. Nor does it hear of a key coming up once the run
    /// is closing.
    ///
    /// `at` is when the input happened: the instant the window handed a key over, or the instant
    /// the controller layer stamped a button with. It is read on the song clock as it stood then,
    /// so the time the frame thread took to get round to the input is no part of its timing.
    fn lane_input(&mut self, shared: &mut AppShared, lane: usize, dir: ScratchDir, press: bool, at: Instant) {
        self.note_held(lane, dir, press);
        if shared.run_plays_itself() || shared.replay.is_some() {
            return;
        }
        if self.scene.is_some() && !self.takes_lane(lane, dir, press) {
            return;
        }
        let clock = self.practice_clock();
        let raw = clock.chart_time_us(shared.song_us_at(at));
        self.sync_judge_settings(shared);
        if press {
            let anchor = shared.anchor_us;
            let hit = match shared.audio.as_mut() {
                Some(audio) => self.session.press_dir(lane, dir, raw, &mut PlayAudioSink::new(audio, anchor, clock)),
                None => self.session.press_dir(lane, dir, raw, &mut NullSink),
            };
            shared.push_timing_sample(raw, clock, hit.map(|r| r.delta_us));
            if let Some(judge) = hit.filter(|r| r.counted).map(|r| r.judge) {
                shared.play_system_sound(crate::syssound::guide_for_judge(judge));
            }
        } else if let Some(judge) = self.session.release_dir(lane, dir, raw).filter(|r| r.counted).map(|r| r.judge) {
            shared.play_system_sound(crate::syssound::guide_for_judge(judge));
        }
        self.keep_adjusted_offset(shared);
        self.pump_skin(shared);
    }

    /// Note which of a lane's keys the player is holding.
    fn note_held(&mut self, lane: usize, dir: ScratchDir, press: bool) {
        if self.held.len() <= lane {
            self.held.resize(lane + 1, (false, false));
        }
        match dir {
            ScratchDir::Forward => self.held[lane].0 = press,
            ScratchDir::Backward => self.held[lane].1 = press,
        }
        if press && let Some(feed) = self.feed.as_mut() {
            feed.note_direction(lane, dir);
        }
    }

    /// Whether the run a scene is run over is handed this press or release: a press while the chart
    /// is playing, and the release of a press it was handed for as long as the chart still is.
    ///
    /// A run that is closing has stopped judging (`KeyInputProccessor.stopJudge`,
    /// `BMSPlayer.java:694, 746`), so a key that comes up then ends no long note: what the result
    /// reports and what the replay holds are the run as it stood when it closed.
    fn takes_lane(&mut self, lane: usize, dir: ScratchDir, press: bool) -> bool {
        if self.sent.len() <= lane {
            self.sent.resize(lane + 1, (false, false));
        }
        let sent = match dir {
            ScratchDir::Forward => &mut self.sent[lane].0,
            ScratchDir::Backward => &mut self.sent[lane].1,
        };
        let playing = self.scene.as_ref().is_some_and(PlayScene::is_playing);
        let taken = playing && (press || *sent);
        *sent = press && playing;
        taken
    }

    /// The scores the run is held against, as the skin's timers and numbers read them: the
    /// player's best on the chart before this run, and the target's, both as they stood when the
    /// target was settled.
    fn standing(&self, shared: &AppShared) -> Standing {
        Standing {
            now_us: shared.skin_now_us(),
            best_score: self.best_score,
            target_score: self.pace_target.as_ref().map_or(0, |target| target.ex),
            carried_combo: self.carried_combo,
        }
    }

    /// Tell the skin's timers what landed just now, for an input that was judged between two frames.
    fn pump_skin(&mut self, shared: &mut AppShared) {
        if self.feed.is_none() {
            return;
        }
        let standing = self.standing(shared);
        let broke = self.feed.as_mut().is_some_and(|feed| feed.pump(&mut shared.skin_timers, &self.session, standing));
        self.note_combo_break(broke, self.song_us);
    }

    /// Whether this run reproduces itself -- the game plays it, or a replay does -- rather than
    /// taking the player's keys.
    fn plays_back(shared: &AppShared) -> bool {
        shared.run_plays_itself() || shared.replay.is_some()
    }

    /// Whether the game plays the chart by itself, which is what lights a skin's key beams from the
    /// keys the game presses. A replay is judged as a player's keys are.
    fn is_autoplay(shared: &AppShared) -> bool {
        shared.run_plays_itself() && shared.replay.is_none()
    }

    /// Which lanes are down as a skin's timers see them: the lanes the run itself has down on one
    /// that plays back, and on a run played by hand the keys the player is holding together with
    /// the lanes the run plays for the player while the chart is playing -- a turntable the game
    /// turns holds its long notes and keeps its beam lit as a key would
    /// (`JudgeManager.auto_presstime`).
    fn lanes_down(&self, shared: &AppShared) -> Vec<bool> {
        let run = self.keys_down();
        if PlayState::plays_back(shared) {
            return run;
        }
        let playing = self.scene.as_ref().is_none_or(PlayScene::is_playing);
        let held = |lane: usize| self.held.get(lane).is_some_and(|(forward, backward)| *forward || *backward);
        run.into_iter().enumerate().map(|(lane, run_down)| held(lane) || (playing && run_down)).collect()
    }

    /// Make sure there is something to feed a skin from, set up from the skin once it has been read.
    fn ensure_feed(&mut self, shared: &AppShared) {
        let autoplay = PlayState::is_autoplay(shared);
        let feed = self.feed.get_or_insert_with(|| Box::new(SkinFeed::new(&self.session, autoplay)));
        let skin = mode_skin_type(shared.mode).and_then(|screen| shared.skins.document(screen));
        feed.adopt_skin(&self.session, skin, autoplay);
    }

    /// One frame of the run for the skin's timers: the keys, the judgements and the long notes.
    fn feed_frame(&mut self, shared: &mut AppShared, chart_us: i64) {
        let down = self.lanes_down(shared);
        let standing = self.standing(shared);
        let broke = self.feed.as_mut().is_some_and(|feed| feed.frame(&mut shared.skin_timers, &self.session, &down, standing));
        self.note_combo_break(broke, chart_us);
    }

    /// A judgement that leaves the combo at nothing puts the chart's miss layer up
    /// (`BMSPlayer.java:1021-1023`).
    fn note_combo_break(&mut self, broke: bool, chart_us: i64) {
        if broke {
            self.background.start_miss(whole_ms(chart_us), DEFAULT_MISS_LAYER_DURATION_MS);
        }
    }

    /// The tempo and the scroll in force at `chart_us`: those of the last timeline it has reached,
    /// and the chart's opening tempo before the first.
    fn tempo_at(&self, chart_us: i64) -> (f64, f64) {
        let timelines = &self.session.model().timelines;
        let reached = timelines.partition_point(|timeline| timeline.time_us <= chart_us);
        reached.checked_sub(1).map_or((self.session.model().init_bpm, 1.0), |last| (timelines[last].bpm, timelines[last].scroll))
    }

    /// How far the chart has played by the frame being drawn, in microseconds: what the play timer
    /// has run for, or the chart preview's timer while only that is on, in whole milliseconds.
    /// Before either is on the chart stands at its beginning, which for a practice slice is where
    /// the slice begins.
    ///
    /// The reference cuts the clock and the timer to milliseconds apart and takes one from the
    /// other, which is one steady step for it because its play timer stands still. The play timer
    /// here is set from the song clock every frame, so the two would be cut at places that drift
    /// against each other and the field would shake by up to a millisecond either way. What the
    /// timer has run for is cut once instead, which is the same step without the shake.
    fn played_chart_us(&self, shared: &AppShared) -> i64 {
        let timers = &shared.skin_timers;
        let now_us = shared.skin_now_us();
        let since = |timer| whole_ms(now_us - timers.value_us(timer)) * MICROS_PER_MILLI;
        if timers.is_on(timer_id::PLAY) {
            since(timer_id::PLAY)
        } else if timers.is_on(CHART_PREVIEW) {
            since(CHART_PREVIEW)
        } else {
            self.practice_clock().chart_time_us(0)
        }
    }

    /// Where the chart stands for the note field of the frame being drawn, in microseconds, as the
    /// reference's field works it out (`LaneRenderer.java:300-301`): how far the chart has played
    /// ([`PlayState::played_chart_us`]) and the judge offset ahead of it
    /// ([`PlayState::field_lead_us`]). The tempo a skin is told is the one in force there, as it is
    /// in the reference, whose field is what works its `nowbpm` out.
    ///
    /// Nothing else of the frame is ahead of the song. The background in particular changes when
    /// the play timer reaches the change (`SkinBGA.java:42-44`).
    fn drawn_chart_us(&self, shared: &AppShared) -> i64 {
        self.played_chart_us(shared) + self.field_lead_us(shared)
    }

    /// How much of the chart's files are in, from nothing (0) to everything (1).
    fn load_progress(&self) -> f32 {
        let (done, total) = self.loads.as_ref().map_or((0, 0), ChartLoads::progress);
        if total == 0 { LOADED } else { done as f32 / total as f32 }
    }

    /// What kind of run this is, as the reference names them (`BMSPlayerMode.Mode`).
    fn kind(&self, shared: &AppShared) -> PlayKind {
        if self.practice.is_some() {
            PlayKind::Practice
        } else if shared.replay.is_some() {
            PlayKind::Replay
        } else if shared.run_plays_itself() {
            PlayKind::Autoplay
        } else {
            PlayKind::Play
        }
    }

    /// What only this screen knows of the frame, for the host's play clusters.
    fn live(&self, shared: &AppShared, feed: &SkinFeed, frame: DocumentFrame) -> PlayLive {
        let play = &shared.config.play;
        let settings = SettingsView::of_config(&shared.config);
        let standing = self.standing(shared);
        PlayLive {
            phase: self.scene.as_ref().map_or(PlayPhase::Play, PlayScene::phase),
            kind: self.kind(shared),
            best_score: standing.best_score,
            target_score: standing.target_score,
            judgements: feed.trace().sides(),
            lanes: LaneSettings {
                hispeed: frame.hispeed as f32,
                lane_cover: play.cover,
                lift: play.lift,
                hidden: play.hidden,
                lane_cover_on: play.enable_cover,
                lift_on: play.enable_lift,
                hidden_on: play.enable_hidden,
                constant_on: false,
                fix_hispeed: settings.chart.map_or(0, |chart| chart.fix_hispeed),
            },
            scroll: Scroll {
                now_bpm: frame.bpm,
                main_bpm: self.bpm.main,
                min_bpm: self.bpm.min,
                max_bpm: self.bpm.max,
                region_ms: travel_region_ms(frame.travel.0, frame.hispeed as f32, frame.travel.1),
            },
            clock: PlayClock::of(&shared.skin_timers, standing.now_us),
            judge_timing_ms: shared.config.judge.offset_ms,
            judge_timing_auto: shared.config.judge.auto_offset,
            bga_on: shared.config.display.bga,
            cover_keys_held: shared.start_pressed() || shared.select_pressed() || shade_is_changing(self.shade_moved_us, standing.now_us),
            long_note_mode: settings.played.long_note_mode,
            played: settings.played,
            load_progress: self.load_progress(),
        }
    }

    /// Where the judgement graph's cursor runs: the whole chart, or the slice a practice run plays
    /// at the speed it plays it.
    fn play_cursor(&self) -> PlayCursor {
        let Some(practice) = &self.practice else {
            return PlayCursor::default();
        };
        let millis = |time_us: i64| i32::try_from(whole_ms(time_us)).ok();
        PlayCursor {
            start_ms: millis(practice.start_us),
            end_ms: millis(practice.end_us),
            speed: Some(practice.freq_percent as f32 / FULL_SPEED_PERCENT as f32),
        }
    }

    /// The gauge the run is on, with the limits of every gauge it could be on, for the gauge object.
    fn gauge_frame(&self) -> GaugeFrame {
        let gauges = &self.session.judge().gauge;
        let scales = rbms_judge::gauge::GaugeIndex::ALL.map(|index| {
            let gauge = gauges.gauge_at(index);
            GaugeScale::new(gauge.min(), gauge.max(), gauge.border())
        });
        GaugeFrame::playing(gauges.selected_index().index(), gauges.value(), scales)
    }

    /// The chart's stage picture as the renderer holds it, uploaded the first time it is asked for.
    fn stage_texture(&mut self, canvas: &mut Canvas<'_>) -> Option<TextureId> {
        if self.stage_texture.is_none() {
            self.stage_texture = self.stagefile.as_ref().map(|image| canvas.register_texture(STAGE_TEXTURE_KEY, &image.rgba, image.width, image.height));
        }
        self.stage_texture
    }

    /// What a skin's `bga` object shows this frame (`SkinBGA.prepare`): black until the chart plays,
    /// then whatever the chart's events have put up by the time the play timer has run for, with the
    /// layer over it. A player who turned backgrounds off is shown the black alone from first to
    /// last, as the reference shows a chart it was handed no background for
    /// (`BGAProcessor.java:337-340`).
    ///
    /// A run no scene is run over hands the object the one picture the screen always handed it.
    ///
    /// Until the chart plays, each frame also uploads a share of the pictures the chart will show
    /// ([`Canvas::preload_bga`]), so the frames that judge keys hand the renderer none of them.
    fn background_frame(&mut self, shared: &AppShared, canvas: &mut Canvas<'_>, single: Option<TextureId>) -> BgaFrame {
        if self.scene.is_none() {
            return BgaFrame::of(single);
        }
        if !shared.config.display.bga {
            canvas.release_bga_textures(&mut self.background_textures);
            return BgaFrame::blank(BgaExpand::default());
        }
        let playing = shared.skin_timers.is_on(timer_id::PLAY);
        if !playing {
            canvas.preload_bga(&mut self.background_textures, &self.background, &self.bga);
        }
        let played_ms = if playing { whole_ms(self.played_chart_us(shared)) } else { BEFORE_PLAY_MS };
        self.background.prepare(played_ms);
        canvas.bga_frame(&mut self.background_textures, self.background.pick(), BgaExpand::default(), &self.bga)
    }

    /// Draw the document this run's layout is selected for, and report whether it drew.
    ///
    /// The mode decides which document: a five-key chart reaches for the five-key screen, and a mode
    /// no document type covers goes straight back to the built-in field. The document itself was
    /// read and compiled at the top of the frame, because whether one exists is also what decides
    /// where the chart's background image goes.
    ///
    /// Everything the document reads comes from the run as it stands: the property clusters from a
    /// [`PlayShown`], the note field from the chart and the trace of what became of each note, the
    /// judge object from the last judgement of each region, and the offsets the lift and the covers
    /// move objects by from the document's own first lane. The chart the note field is handed is the
    /// run's own, or the one CONSTANT and LEGACY NOTE make of it ([`FieldChart`]).
    ///
    /// A run no scene is run over -- a screen that was put up without being entered -- is fed here,
    /// from the frame it is drawn on, as a chart that is playing.
    fn draw_document(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>, frame: DocumentFrame) -> bool {
        let shared = &mut *ctx.shared;
        let Some(skin_type) = mode_skin_type(shared.mode).filter(|skin_type| shared.has_skin_document(*skin_type)) else {
            return false;
        };
        self.ensure_feed(shared);
        if self.scene.is_none() {
            self.feed_unentered(shared);
        }
        self.ensure_field(shared);
        let stagefile = self.stage_texture(canvas);
        let background = self.background_frame(shared, canvas, frame.background);
        let Some(feed) = self.feed.as_deref() else {
            return false;
        };
        let live = self.live(shared, feed, frame);
        let shown = PlayShown::of(&self.session, &live);
        let play = &shared.config.play;
        let (lift, lanecover, hidden) =
            (play.enable_lift.then_some(play.lift), play.enable_cover.then_some(play.cover), play.enable_hidden.then_some(play.hidden));
        let field = shared.skins.document(skin_type).and_then(first_lane_rect).map(|first_lane| lane_offsets(first_lane, lift, lanecover, hidden));
        let offsets = feed.driver().offsets(FieldOffsets {
            lift: field.map(|field| field.lift),
            lane_cover: field.map(|field| field.lanecover),
            hidden_cover: field.map(|field| field.hidden),
        });
        let model = self.session.model();
        let shown_chart = self.field.as_ref().map_or(self.session.field_timelines(), FieldChart::timelines);
        let opening_bpm = self.field.as_ref().and_then(FieldChart::tempo).map_or(model.init_bpm, |(bpm, _)| bpm);
        let notes = LaneNotes {
            hispeed: frame.hispeed as f32,
            lift,
            lanecover,
            hidden,
            ln_mode: self.session.ln_mode().resolve(),
            states: feed.trace(),
            longs: feed.longs(),
            auto_lanes: self.session.auto_lanes(),
            ..LaneNotes::new(shown_chart, self.drawn_chart_us(shared), opening_bpm)
        };
        let overview = feed.overview();
        let chart = overview.meta(stagefile.is_some());
        let chart_series = overview.series();
        let (judgements, early_late) = feed.trace().seconds();
        let distribution = NoteDistribution {
            judged: &self.session.judge().counts,
            judgements,
            early_late,
            playing: Some(self.play_cursor()),
            ..chart_series.notes.unwrap_or_default()
        };
        let data = FrameData {
            notes: Some(&notes),
            gauge: Some(self.gauge_frame()),
            series: FrameSeries { recent_hits: Some(feed.recent_hits()), notes: Some(distribution), ..chart_series },
            images: ReferenceImages { stagefile, ..ReferenceImages::default() },
            bga: background,
            judge: feed.trace().regions(),
            ..FrameData::default()
        };
        let paced_by = self.pace_target.as_ref().map(ResolvedTarget::legend).unwrap_or_default();
        shared.draw_play_skin(canvas, skin_type, &PlayDraw { chart: &chart, shown: &shown, offsets: &offsets, paced_by: &paced_by, data })
    }

    /// Make sure the chart the note field is shown is the one CONSTANT and LEGACY NOTE ask for as
    /// they stand: made over when either is on and has moved since it last was, and the run's own
    /// with both off.
    fn ensure_field(&mut self, shared: &AppShared) {
        let (constant, legacy_note) = (shared.config.play.constant_speed, shared.config.play.legacy_note);
        let wanted = (constant || legacy_note).then_some((constant, legacy_note));
        if self.field.as_ref().map(FieldChart::settings) != wanted {
            self.field = FieldChart::of(self.session.field_timelines(), constant, legacy_note);
        }
    }

    /// One frame of a run that was never entered, for the skin's timers: the chart counts as
    /// playing from the first frame it is drawn on.
    fn feed_unentered(&mut self, shared: &mut AppShared) {
        let now_us = shared.skin_now_us();
        let chart_us = self.song_us;
        if !shared.skin_timers.is_on(timer_id::PLAY)
            && let Some(feed) = self.feed.as_mut()
        {
            feed.driver_mut().apply(&mut shared.skin_timers, now_us, SceneEvent::Started { start_offset_us: chart_us });
        }
        self.feed_frame(shared, chart_us);
    }

    /// Which lanes are being held right now, in lane order.
    ///
    /// A beam is lit from the moment a lane goes down until it comes back up, and the release stamp
    /// is what outlives it into the fade, so a lane is down exactly while its press is the later of
    /// the two.
    fn keys_down(&self) -> Vec<bool> {
        let (on, off) = (self.session.beam_on(), self.session.beam_off());
        on.iter().zip(off).map(|(on, off)| on > off).collect()
    }

    /// The replay-analysis strip over a skin's screen, which is a system overlay there: the skin
    /// draws the whole screen and the strip is laid over it in the built-in layout's own space.
    fn draw_analysis_over(&self, ctx: &FrameCtx<'_>, canvas: &mut Canvas<'_>, song: i64) {
        if self.scene.is_some() && self.session.analysis_enabled() && ctx.shared.replay.is_some() {
            self.draw_analysis(canvas, song);
        }
    }

    /// The replay-analysis overlay: a playback bar, the current rate/paused state and time, plus the
    /// recent per-note timing errors (ms early = cyan +, late = orange -).
    fn draw_analysis(&self, canvas: &mut Canvas<'_>, song: i64) {
        let play = &self.session;
        let colors = theme();
        let total = play.last_time_us().max(1);
        let prog = (song as f32 / total as f32).clamp(0.0, 1.0);
        let bx = ANALYSIS_BAR_INSET;
        let bw = CW as f32 - ANALYSIS_BAR_INSET * 2.0;
        let by = CH as f32 - 14.0;
        canvas.fill_rect(Rect::new(0.0, CH as f32 - ANALYSIS_PANEL_H, CW as f32, ANALYSIS_PANEL_H), Color { r: 0, g: 0, b: 0, a: 170 });
        canvas.fill_rect(Rect::new(bx, by, bw, 6.0), colors.divider);
        canvas.fill_rect(Rect::new(bx, by, bw * prog, 6.0), colors.good);
        canvas.fill_rect(Rect::new((bx + bw * prog - 1.5).max(bx), by - 3.0, 3.0, 12.0), Color::WHITE);
        let state = if play.analysis_paused() { "PAUSED".to_string() } else { format!("{:.2}x", play.analysis_rate()) };
        draw_text(canvas, bx, CH as f32 - 66.0, 1.4, colors.accent, &format!("ANALYSIS  {state}   {:.1} / {:.1} S", song as f32 / 1e6, total as f32 / 1e6));
        draw_text_right(canvas, bx + bw, CH as f32 - 66.0, 1.1, colors.text_muted, "SPACE PAUSE  -/+ SPEED  PGUP/PGDN SEEK  ESC EXIT");
        let mut mx = bx;
        for mark in play.recent_marks(ANALYSIS_MARKS_SHOWN) {
            let col = if mark.delta_us > 0 {
                Color::CYAN
            } else if mark.delta_us < 0 {
                Color::ORANGE
            } else {
                Color::WHITE
            };
            let txt = format!("{:+}", mark.delta_us / 1000);
            draw_text(canvas, mx, CH as f32 - 44.0, 1.3, col, &txt);
            mx += text_width(&txt, 1.3) + 12.0;
        }
    }
}

impl PlayState {
    /// Move the run on by one frame of the song clock: read the clock, hand the session what the
    /// clock has reached, and answer where the chart now stands.
    ///
    /// In manual analysis the displayed song time comes from the virtual clock (pausable,
    /// rate-scaled); otherwise it follows the real (audio) clock, and analysis mirrors it so a
    /// first manual control resumes from the live position. Manual analysis mutes keysounds.
    fn tick_run(&mut self, ctx: &mut FrameCtx<'_>) -> i64 {
        let manual = self.session.analysis_manual();
        let practice_clock = self.practice_clock();
        let (song, scheduled_us) = if manual {
            let frame_us = (ctx.dt as f64 * 1_000_000.0) as i64;
            let song = self.session.advance_analysis(frame_us);
            (song, song)
        } else {
            let (engine_song_us, lookahead_us) = ctx.shared.song_and_lookahead_us();
            let song = practice_clock.chart_time_us(engine_song_us);
            self.session.sync_analysis_position(song);
            let scheduled_engine_us = schedule_position_us(engine_song_us, lookahead_us, ctx.shared.schedule_poll_us, false);
            (song, practice_clock.chart_time_us(scheduled_engine_us))
        };
        self.song_us = song;
        ctx.shared.push_clock_sample(song);
        self.sync_judge_settings(ctx.shared);
        let clock = SessionClock { audible_us: song, scheduled_us };
        let anchor = ctx.shared.anchor_us;
        match (manual, ctx.shared.audio.as_mut()) {
            (false, Some(audio)) => self.session.tick(clock, &mut PlayAudioSink::new(audio, anchor, practice_clock)),
            _ => self.session.tick(clock, &mut NullSink),
        }
        song
    }

    /// Take in whatever the chart's decodes have finished since the last frame.
    fn poll_loads(&mut self, shared: &mut AppShared) {
        if let Some(mut loads) = self.loads.take()
            && !loads.poll(shared, &mut self.bga)
        {
            self.loads = Some(loads);
        }
    }

    /// How fast the chart is running against the scene clock, in percent: full speed, or whatever a
    /// replay under analysis is being played at, which is nought while it is paused.
    fn play_speed_percent(&self) -> i32 {
        if !self.session.analysis_manual() {
            return FULL_SPEED_PERCENT as i32;
        }
        if self.session.analysis_paused() { 0 } else { (self.session.analysis_rate() * FULL_SPEED_PERCENT) as i32 }
    }

    /// Where the run stands against the end of its playing time. A practice slice ends where the
    /// slice does, and a replay under analysis never ends by itself: the player is driving it.
    fn run_end(&self, chart_us: i64) -> RunEnd {
        if let Some(practice) = &self.practice {
            return RunEnd { ended: practice.is_past_end(chart_us), last_note_passed: self.session.all_notes_resolved() };
        }
        if self.session.analysis_enabled() {
            return RunEnd::default();
        }
        RunEnd::of(self.session.play_time_ms(), whole_ms(chart_us))
    }

    /// One frame of a run a scene is run over: the load, the song clock while the chart plays, the
    /// scene's own step, and what the step asks of the screen.
    fn update_scene(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.ensure_pace_target(ctx.shared);
        self.ensure_feed(ctx.shared);
        self.poll_loads(ctx.shared);
        self.held_controls(ctx.shared, ctx.now);
        let playing = self.scene.as_ref().is_some_and(PlayScene::is_playing);
        let chart_us = if playing { self.tick_run(ctx) } else { self.song_us };
        let shared = &mut *ctx.shared;
        if !playing {
            shared.poll_audio_clock();
        }
        if playing && self.session.analysis_manual() && shared.skin_timers.is_on(timer_id::PLAY) {
            shared.set_skin_scene_clock(shared.skin_timers.value_us(timer_id::PLAY) + chart_us);
        }
        let skin_type = mode_skin_type(shared.mode);
        let (bpm, _) = self.tempo_at(chart_us);
        let judge = self.session.judge();
        let by_hand = !PlayState::plays_back(shared);
        let facts = SceneFacts {
            now_us: shared.skin_now_us(),
            times: PlayTimes::of_skin(skin_type.and_then(|screen| shared.skins.document(screen))),
            loaded: self.loads.is_none() && self.drawn && !skin_type.is_some_and(|screen| shared.skin_is_loading(screen)),
            start: shared.start_pressed(),
            select: shared.select_pressed(),
            start_offset_us: self.practice_clock().chart_time_us(0),
            playing: PlayingFrame {
                chart_us,
                bpm,
                play_speed: self.play_speed_percent(),
                gauge_max: judge.gauge.selected().is_max(),
                past_notes: judge.total_judged(),
            },
            failed: self.practice.is_none() && !self.session.analysis_enabled() && self.session.is_failed(),
            end: self.run_end(chart_us),
            notes_done: self.session.all_notes_resolved(),
            may_retry: by_hand && self.practice.is_none() && shared.course_run.is_none(),
        };
        let (Some(scene), Some(feed)) = (self.scene.as_mut(), self.feed.as_mut()) else {
            return Transition::Stay;
        };
        let step = scene.step(feed.driver_mut(), &mut shared.skin_timers, &facts);
        self.feed_frame(shared, chart_us);
        match step {
            SceneStep::Stay => {}
            SceneStep::Ready => shared.play_system_sound(SystemSound::PlayReady),
            SceneStep::Started => {
                shared.start_play();
                self.song_us = facts.start_offset_us;
            }
            SceneStep::Failed | SceneStep::Finished { failed_too: true } => {
                shared.stop_chart_sounds(self.session.model());
                shared.play_system_sound(SystemSound::PlayStop);
            }
            SceneStep::Finished { failed_too: false } => {}
            SceneStep::Leave(Leave::Result) => return enter_result(self, shared),
            SceneStep::Leave(Leave::Retry { same_layout }) => {
                if let Some(again) = self.retry(shared, same_layout) {
                    return again;
                }
            }
        }
        if self.escape_hold_elapsed(ctx.now) {
            return self.leave_run(ctx);
        }
        Transition::Stay
    }

    /// Start the chart again at once, which is what START or SELECT does to a run that has just
    /// failed (`BMSPlayer.java:696-710`): START lays the chart out afresh on the same options, and
    /// SELECT keeps the layout the failed run was played on. `None` when there is no chart to start
    /// -- one that is not in the library -- and the failure closes as it would have.
    ///
    /// The failed run is kept and sent before the chart starts again
    /// ([`crate::app_result::record_retried_run`]), as its result screen would have kept it.
    fn retry(&mut self, shared: &mut AppShared, same_layout: bool) -> Option<Transition> {
        shared.retry_seed = same_layout.then(|| self.session.seed());
        match crate::app_result::retry(shared) {
            Transition::Stay => {
                shared.retry_seed = None;
                None
            }
            again => {
                crate::app_result::record_retried_run(self, shared);
                shared.save_settings();
                Some(again)
            }
        }
    }

    /// Escape on a run a scene is run over. A run that is closing skips what is left of its
    /// closing; a chart with nothing left to hit is finished, and fades out as one the player ended
    /// (`BMSPlayer.stopPlay`); anything else is abandoned on the terms of the ESCAPE setting.
    fn scene_escape(&mut self, ctx: &mut FrameCtx<'_>, key: &KeyInput<'_>) -> Transition {
        let closing = self.scene.as_ref().is_some_and(PlayScene::is_closing);
        if closing {
            return if key.pressed { enter_result(self, ctx.shared) } else { Transition::Stay };
        }
        let playing = self.scene.as_ref().is_some_and(PlayScene::is_playing);
        if key.pressed
            && playing
            && self.session.all_notes_resolved()
            && let (Some(scene), Some(feed)) = (self.scene.as_mut(), self.feed.as_mut())
        {
            let now_us = ctx.shared.skin_now_us();
            scene.stop_finished(feed.driver_mut(), &mut ctx.shared.skin_timers, now_us);
            return Transition::Stay;
        }
        self.escape_key(ctx, key)
    }
}

/// Whether a run left before its end can be kept: the player was playing it by hand. Only a run the
/// engine played and a replay being watched are no record. A run of their own that judged nothing
/// is not kept either ([`PlayState::has_judged_nothing`], the player's decision of 2026-10-19).
pub(crate) fn keeps_a_left_run(shared: &AppShared) -> bool {
    !shared.run_plays_itself() && shared.replay.is_none()
}

/// How long after a control of the run moved or switched a shade a skin still shows the figures of
/// the shades, in microseconds of its own clock.
const SHADE_FIGURES_SHOWN_US: i64 = 1_500_000;

/// Whether a shade moved recently enough that a skin shows the figures it shows while one is being
/// changed (`OPTION_LANECOVER1_CHANGING`).
///
/// The reference shows them while START or SELECT is held, which is the only way it has of moving
/// a shade. A key of its own moves one here with neither held, so its press stands in for the hold
/// for a moment: without it a cover that is switched off moves with nothing on the screen to say so.
///
/// The two readings are held to being that close to each other, whichever is the later: a scene
/// clock that was put back reads earlier than the move it follows.
fn shade_is_changing(moved_us: Option<i64>, now_us: i64) -> bool {
    moved_us.is_some_and(|moved_us| now_us.abs_diff(moved_us) < SHADE_FIGURES_SHOWN_US.unsigned_abs())
}

impl StageHandler for PlayState {
    /// A run a skin draws begins the reference's scene here: loading, or ready at once for a
    /// practice slice, with the skin asked for afresh, because a Lua skin builds its screen out of
    /// the chart it is read against. The stage picture a skin shows while it loads is decoded now,
    /// as the reference reads it when the chart is picked.
    ///
    /// A run that holds its green number takes the speed that holds it for this chart before
    /// anything is drawn (`LaneRenderer.java:154`), and keeps it there on every frame after
    /// ([`StageHandler::update`]).
    ///
    /// A run no skin draws starts as it always did, with the ready cue.
    fn on_enter(&mut self, ctx: &mut FrameCtx<'_>) {
        let shared = &mut *ctx.shared;
        self.hold_green_number(shared);
        let Some(skin_type) = mode_skin_type(shared.mode).filter(|_| shared.has_play_scene()) else {
            shared.play_system_sound(SystemSound::PlayReady);
            return;
        };
        shared.skins.request_for(&shared.config, skin_type);
        self.scene = Some(if self.practice.is_some() { PlayScene::ready() } else { PlayScene::loading() });
        self.song_us = self.practice_clock().chart_time_us(0);
        self.ensure_feed(shared);
        let stagefile = self.feed.as_deref().map(|feed| feed.overview().stagefile.clone()).filter(|name| !name.trim().is_empty());
        self.stagefile = stagefile.and_then(|name| Path::new(&shared.chart_path).parent().and_then(|folder| crate::decode_bga_image(folder, &name)));
    }

    fn on_exit(&mut self, ctx: &mut FrameCtx<'_>) {
        if let Some(loads) = self.loads.take() {
            loads.stop();
        }
        if self.practice.is_some() {
            ctx.shared.restore_practice_images(&mut self.bga);
        }
        if let Some(gpu) = ctx.shared.gpu.as_mut() {
            self.background_textures.release(gpu);
        }
    }

    fn update(&mut self, ctx: &mut FrameCtx<'_>) -> Transition {
        self.take_opened_keys(ctx.shared, ctx.now);
        self.hold_green_number(ctx.shared);
        self.repeat_held_keys(ctx.shared, ctx.now);
        if self.scene.is_some() {
            return self.update_scene(ctx);
        }
        let song = self.tick_run(ctx);
        if self.run_is_over(song) {
            return enter_result(self, ctx.shared);
        }
        if self.escape_hold_elapsed(ctx.now) {
            return self.leave_run(ctx);
        }
        Transition::Stay
    }

    /// One controller event: a lane goes to the same path a key does, and a control action to the
    /// same one a bound key does. Controls are only ever emitted on the way down, so there is no
    /// press gate here. The frame loop hands each event over with `ctx.now` set to the instant the
    /// controller layer stamped it with, which is what a lane is judged at.
    fn handle_pad(&mut self, ctx: &mut FrameCtx<'_>, event: crate::gamepad::PadEvent) -> Transition {
        self.take_opened_keys(ctx.shared, ctx.now);
        match event {
            crate::gamepad::PadEvent::Lane { lane, dir, press } => self.lane_input(ctx.shared, lane, dir, press, ctx.now),
            crate::gamepad::PadEvent::Control(action) => self.in_play_control(ctx.shared, action),
            crate::gamepad::PadEvent::Shortcut(shortcut) => {
                self.analysis_shortcut(ctx.shared, shortcut);
            }
        }
        Transition::Stay
    }

    /// The wheel moves the cover the player is on, on a screen a skin draws
    /// (`ControlInputProcessor.java:151-154`).
    fn handle_scroll(&mut self, ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        if self.scene.is_some() {
            self.wheel_cover(ctx.shared, lines);
        }
        Transition::Stay
    }

    /// Escape with nothing left to hit (every note resolved) goes straight to the result screen
    /// rather than discarding the run; otherwise it abandons the chart, in the way the ESCAPE
    /// setting asks for. A run a scene is run over has its own rules ([`PlayState::scene_escape`]).
    fn handle_key(&mut self, ctx: &mut FrameCtx<'_>, key: KeyInput<'_>) -> Transition {
        self.take_opened_keys(ctx.shared, ctx.now);
        if key.code == KeyCode::Escape {
            if self.scene.is_some() {
                return self.scene_escape(ctx, &key);
            }
            if key.pressed && self.session.all_notes_resolved() {
                return enter_result(self, ctx.shared);
            }
            return self.escape_key(ctx, &key);
        }
        if key.pressed && self.analysis_key(ctx.shared, key.code) {
            return Transition::Stay;
        }
        if key.pressed
            && let Some(action) = ctx.shared.control_for(key.code)
        {
            self.in_play_control(ctx.shared, action);
            return Transition::Stay;
        }
        self.lane_key(ctx.shared, &key, ctx.now);
        Transition::Stay
    }

    /// A screen that waits for its chart with no skin to draw the wait -- the skin chosen for it
    /// could not be read, which is not known until the screen is up -- shows the built-in LOADING
    /// screen for as long as it waits, and the built-in field from there on.
    ///
    /// The green number on the HUD is the note travel time (ms) for the scroll speed shown right
    /// now: fixed in CONSTANT mode (2000/hi-speed at the calibration BPM), read against the tempo
    /// the SPEED FIX row pins to when it names one, and otherwise tracking the BPM and SCROLL of the
    /// timeline segment under the play head. The white number beside it is what the rest of the
    /// travel time is spent under the lane cover.
    fn draw(&mut self, ctx: &mut FrameCtx<'_>, canvas: &mut Canvas<'_>) {
        self.ensure_pace_target(ctx.shared);
        let song = self.song_us;
        let skin_type = mode_skin_type(ctx.shared.mode);
        if let Some(skin_type) = skin_type {
            ctx.shared.prepare_skin(canvas, skin_type);
        }
        self.drawn = true;
        let has_document = skin_type.is_some_and(|skin_type| ctx.shared.has_skin_document(skin_type));
        let play = &self.session;
        let frame_image = ctx.shared.config.display.bga.then(|| self.bga.get(&play.bga_frame())).flatten();
        let mut document_background = None;
        match (has_document, ctx.shared.skin.bga, frame_image) {
            (false, Some(rect), Some(img)) => canvas.set_background(img.generation, &img.rgba, img.width, img.height, rect),
            (true, _, Some(img)) if self.scene.is_none() => {
                canvas.clear_bga();
                document_background = canvas.background_texture(img.generation, &img.rgba, img.width, img.height);
            }
            _ => canvas.clear_bga(),
        }
        let hispeed = ctx.shared.config.play.hispeed;
        let field_us = self.built_in_field_us(ctx.shared);
        let (segment_bpm, scroll) = {
            let tls = &play.model().timelines;
            let seg = tls.binary_search_by(|t| t.time_us.cmp(&field_us)).unwrap_or_else(|i| i.saturating_sub(1));
            tls.get(seg).map(|t| (t.bpm, t.scroll)).unwrap_or((play.model().init_bpm, 1.0))
        };
        if has_document {
            self.ensure_field(ctx.shared);
            let (bpm, scroll) = self.tempo_at(self.drawn_chart_us(ctx.shared));
            let travel = self.field.as_ref().and_then(FieldChart::tempo).unwrap_or((bpm, scroll));
            let document_frame = DocumentFrame { bpm, travel, hispeed, background: document_background };
            if self.draw_document(ctx, canvas, document_frame) {
                self.draw_analysis_over(ctx, canvas, song);
                return;
            }
            if self.scene.is_some() && skin_type.is_some_and(|skin_type| ctx.shared.skin_is_loading(skin_type)) {
                canvas.native().clear(Color::BLACK);
                return;
            }
        }
        if self.scene.as_ref().is_some_and(|scene| scene.phase() == PlayPhase::Preload) {
            canvas.clear_bga();
            self.loads.as_ref().unwrap_or(&ChartLoads::default()).draw(ctx.shared, canvas);
            return;
        }
        let play = &self.session;
        let constant = ctx.shared.config.play.constant_speed;
        let cover = ctx.shared.effective_cover();
        let j = play.judge();
        let pinned = self.bpm.target(ctx.shared.config.play.fix_hispeed);
        let (bpm, shown_scroll) = match pinned {
            Some(bpm) => (bpm, 1.0),
            None => (segment_bpm, scroll),
        };
        let green = green_number_for(constant, bpm, hispeed, shown_scroll, cover);
        let white = match ctx.shared.config.display.show_white_number {
            true => (green_number_for(constant, bpm, hispeed, shown_scroll, 0.0) - green).max(0.0),
            false => 0.0,
        };
        let best_ex = ctx.shared.scores.best_ex_for_md5_in_ln_mode(&play.model().md5, &self.ln_mode_key);
        let hud = HudView {
            mode_label: mode_config_key(ctx.shared.mode),
            combo: j.combo,
            last_judge: j.last_judge.map(|x| x as u8),
            last_fast: j.last_fast,
            fast: self.fast(),
            slow: self.slow(),
            counts: j.counts,
            ex_score: j.ex_score,
            gauge: j.gauge.value(),
            green_number: green,
            white_number: white,
            judge_text_y: ctx.shared.config.display.judge_text_y,
            max_ex: j.total_notes() * 2,
            best_ex,
            pace: self
                .pace_target
                .as_ref()
                .map(|target| HudPace { name: &target.name, delta: j.ex_score as i64 - pace_ex_at(target.ex, j.total_judged(), j.total_notes()) }),
        };
        let playfield = PlayfieldView {
            timelines: self.session.field_timelines(),
            microtime: field_us,
            hispeed,
            beam_on: self.session.beam_on(),
            beam_off: self.session.beam_off(),
            constant,
            legacy_note: ctx.shared.config.play.legacy_note,
            auto_lanes: self.session.auto_lanes(),
        };
        let shade = LaneShade { cover, hidden: ctx.shared.effective_hidden() };
        render_playfield_view(canvas, &ctx.shared.skin, &playfield);
        render_lane_cover(canvas, &ctx.shared.skin, shade);
        render_key_bomb(canvas, &ctx.shared.skin, self.session.bomb(), song);
        render_hud(canvas, &ctx.shared.skin, &hud);
        if self.session.analysis_enabled() {
            self.draw_analysis(canvas, song);
        }
    }

    fn debug_lines(&self, ctx: &FrameCtx<'_>) -> Vec<String> {
        let j = self.session.judge();
        vec![
            format!("TIME {:.2} / {:.2} S", self.song_us as f32 / 1e6, self.session.last_time_us() as f32 / 1e6),
            format!("NOTES {} / {}", j.total_judged(), j.total_notes()),
            format!("COMBO {}  MAX {}", j.combo, j.max_combo),
            format!("EX {}  GAUGE {:.1}%", j.ex_score, j.gauge.value()),
            format!("FAST {}  SLOW {}  EPOOR {}", j.fast, j.slow, j.empty_poor),
            format!("PLAYED FAST {:?}  SLOW {:?}", self.fast(), self.slow()),
            format!("HISPEED {:.2}  OFFSET {:+}MS", ctx.shared.config.play.hispeed, ctx.shared.config.judge.offset_ms),
        ]
    }
}
