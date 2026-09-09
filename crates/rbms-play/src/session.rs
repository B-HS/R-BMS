//! One run of a chart, driven headlessly.
//!
//! [`PlaySession`] owns everything that only exists while a chart is being played: the judge
//! [`Player`], the replay being reproduced, the inputs being recorded, the auto-calibration
//! accumulator, the replay-analysis clock, the recent judgement errors and the BGA timeline. Sound
//! leaves through the [`SoundSink`] trait, so this crate stays free of any audio backend and the
//! whole run can be reproduced against [`NullSink`] with no device present.

use rbms_judge::{ClearType, GaugeKind, JudgeEngine, JudgeResult};
use rbms_model::Model;
use rbms_store::{Replay, ReplayEvent};

use crate::{PlayEvent, PlaySource, Player};

/// Neutral per-voice parameters for a chart keysound: reference level, centred, unpitched. The
/// balance between keysounds and accompaniment belongs on the output buses, not on the voice.
pub const KEYSOUND_GAIN: f32 = 1.0;
/// Stereo position of a chart keysound.
pub const KEYSOUND_PAN: f32 = 0.0;
/// Playback rate of a chart keysound.
pub const KEYSOUND_PITCH: f32 = 1.0;

/// How long the run keeps going past the last note before the result screen is due.
const PLAY_TAIL_US: i64 = 2_000_000;

/// How many judged inputs the analysis overlay keeps.
const TIMING_MARK_CAPACITY: usize = 16;

/// Slowest and fastest analysis playback rates, and the step the controls move by.
pub const ANALYSIS_RATE_MIN: f64 = 0.25;
/// Fastest analysis playback rate.
pub const ANALYSIS_RATE_MAX: f64 = 4.0;
/// How much one rate step changes the analysis playback rate.
pub const ANALYSIS_RATE_STEP: f64 = 0.25;
/// How far one analysis seek moves the virtual clock.
pub const ANALYSIS_SEEK_STEP_US: i64 = 2_000_000;

/// Worst judgement an input may earn and still count towards auto-calibration (GOOD), and the
/// widest timing error accepted, so a wild miss cannot drag the mean.
const CALIBRATION_WORST_JUDGE: usize = 2;
/// Widest timing error an input may have and still count towards auto-calibration.
const CALIBRATION_MAX_ABS_DELTA_US: i64 = 150_000;
/// How many accepted inputs auto-calibration needs before it will propose an offset.
const CALIBRATION_MIN_SAMPLES: u32 = 20;

/// The BGA frame shown before the chart's first BGA event.
const BGA_FRAME_NONE: i32 = -1;

/// When a keysound should sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundTime {
    /// As soon as the device can, without waiting for the scheduling lookahead. An input's keysound
    /// is wanted at the instant the key went down, not at the time of the note it hit.
    Immediate,
    /// Booked at this position on the session clock (µs), which the host rebases onto its own axis.
    Scheduled(i64),
}

/// One keysound the session asks the host to play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundRequest {
    /// Keysound id within the chart's own bank.
    pub wav: u32,
    /// Which output bus the sound belongs to.
    pub source: PlaySource,
    pub gain: f32,
    pub pan: f32,
    pub pitch: f32,
    pub at: SoundTime,
}

/// Where a session's keysounds go. The host implements this over its audio engine, mapping
/// [`PlaySource`] onto its own buses and [`SoundTime`] onto its own clock.
pub trait SoundSink {
    /// Play one keysound.
    fn play(&mut self, sound: SoundRequest);
    /// Silence everything this session has started.
    fn stop_all(&mut self);
}

/// A sink that drops every sound: headless tests, offline analysis, and the muted scrubbing of
/// replay analysis all run against it.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullSink;

impl SoundSink for NullSink {
    fn play(&mut self, _sound: SoundRequest) {}

    fn stop_all(&mut self) {}
}

/// The two time axes one play frame drives: what the listener is hearing right now, and how far
/// ahead sounds are being booked. Judgement only ever reads the audible axis, so the scheduler's
/// lookahead cannot move a judgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionClock {
    pub audible_us: i64,
    pub scheduled_us: i64,
}

impl SessionClock {
    /// Both axes at one instant, for the virtual clocks (offline analysis, tests, autoplay
    /// simulation) that have no device lead to book against.
    pub fn at(song_us: i64) -> Self {
        SessionClock { audible_us: song_us, scheduled_us: song_us }
    }
}

/// How one run is set up. The note shuffle is applied to the model before the session is built, so
/// an analysis seek rebuilds the judge state against the very same lane layout.
#[derive(Debug, Clone)]
pub struct SessionOptions {
    /// Play the chart from itself instead of from input.
    pub autoplay: bool,
    pub gauge: GaugeKind,
    /// How far the judgement of an input is shifted from the instant it arrived.
    pub judge_offset_us: i64,
    /// JUDGE WIDTH as a percentage of the chart's own windows.
    pub judge_rate_percent: i32,
    /// Lanes played from the chart even in an interactive run, empty when there are none.
    pub auto_lanes: Vec<bool>,
    /// The note-shuffle seed this run was laid out with, reported with the score.
    pub seed: u64,
    /// Show the replay-analysis overlay and accept its playback controls.
    pub analysis: bool,
    /// Accumulate the timing error of accurate hits so an offset can be proposed afterwards.
    pub auto_calibration: bool,
    /// The replay to reproduce instead of taking input.
    pub replay: Option<Replay>,
}

impl Default for SessionOptions {
    fn default() -> Self {
        SessionOptions {
            autoplay: false,
            gauge: GaugeKind::Normal,
            judge_offset_us: 0,
            judge_rate_percent: 100,
            auto_lanes: Vec::new(),
            seed: 0,
            analysis: false,
            auto_calibration: false,
            replay: None,
        }
    }
}

/// The part of [`SessionOptions`] an analysis seek has to rebuild the judge state from.
#[derive(Debug, Clone)]
struct Setup {
    autoplay: bool,
    gauge: GaugeKind,
    judge_rate_percent: i32,
    auto_lanes: Vec<bool>,
    seed: u64,
}

/// The replay being reproduced and how far into it the run has got.
#[derive(Debug)]
struct ReplayTrack {
    events: Vec<ReplayEvent>,
    cursor: usize,
}

impl ReplayTrack {
    /// The next recorded input whose raw time has been reached, consuming it.
    fn next_due(&mut self, song_us: i64) -> Option<ReplayEvent> {
        let event = *self.events.get(self.cursor)?;
        if event.t > song_us {
            return None;
        }
        self.cursor += 1;
        Some(event)
    }
}

/// Running mean of the timing error of accurate hits, which the result screen turns into a new
/// judge offset.
#[derive(Debug, Default, Clone, Copy)]
struct Calibration {
    sum_us: i64,
    count: u32,
}

impl Calibration {
    /// Accumulate one judged input, ignoring anything too inaccurate to calibrate from.
    fn record(&mut self, result: &JudgeResult) {
        if (result.judge as usize) > CALIBRATION_WORST_JUDGE || result.delta_us.abs() > CALIBRATION_MAX_ABS_DELTA_US {
            return;
        }
        self.sum_us += result.delta_us;
        self.count += 1;
    }

    /// The mean timing error, once enough inputs have been accepted to trust it.
    fn mean_us(&self) -> Option<i64> {
        if self.count < CALIBRATION_MIN_SAMPLES { None } else { Some(self.sum_us / self.count as i64) }
    }
}

/// Replay-analysis playback: a virtual clock the player can pause, slow down and scrub with.
#[derive(Debug, Clone, Copy)]
struct AnalysisState {
    enabled: bool,
    manual: bool,
    paused: bool,
    rate: f64,
    position_us: i64,
}

impl AnalysisState {
    fn new(enabled: bool) -> Self {
        AnalysisState { enabled, manual: false, paused: false, rate: 1.0, position_us: 0 }
    }
}

/// One judged input's timing error, newest last, for the analysis overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimingMark {
    pub lane: usize,
    pub delta_us: i64,
    pub judge: u8,
}

/// A bounded ring of the most recent judged inputs.
#[derive(Debug, Default)]
struct TimingMarks {
    marks: Vec<TimingMark>,
}

impl TimingMarks {
    fn push(&mut self, mark: TimingMark) {
        if self.marks.len() == TIMING_MARK_CAPACITY {
            self.marks.remove(0);
        }
        self.marks.push(mark);
    }

    fn clear(&mut self) {
        self.marks.clear();
    }

    fn recent(&self, count: usize) -> &[TimingMark] {
        &self.marks[self.marks.len().saturating_sub(count)..]
    }
}

/// The chart's BGA changes and the frame currently showing.
#[derive(Debug, Default)]
struct BgaTimeline {
    events: Vec<(i64, i32)>,
    cursor: usize,
    current: i32,
}

impl BgaTimeline {
    fn from_model(model: &Model) -> Self {
        let mut events: Vec<(i64, i32)> = model.timelines.iter().filter(|tl| tl.bga >= 0).map(|tl| (tl.time_us, tl.bga)).collect();
        events.sort_by_key(|e| e.0);
        BgaTimeline { events, cursor: 0, current: BGA_FRAME_NONE }
    }

    fn advance(&mut self, song_us: i64) {
        while self.cursor < self.events.len() && self.events[self.cursor].0 <= song_us {
            self.current = self.events[self.cursor].1;
            self.cursor += 1;
        }
    }
}

/// Everything the result screen reads off the judge engine, as plain values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaySummary {
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub max_ex_score: u32,
    pub max_combo: u32,
    pub total_notes: u32,
    /// Notes resolved so far; empty POOR consumes none, so this can trail `total_notes`.
    pub total_judged: u32,
    pub fast: u32,
    pub slow: u32,
    /// Per-judge early/late split, indexed the same way as `counts`.
    pub early: [u32; 6],
    pub late: [u32; 6],
    /// Mean signed timing error over the run's actual hits.
    pub avg_judge_us: i64,
    pub empty_poor: u32,
    pub gauge_value: f32,
    pub clear_lamp: ClearType,
    /// Bad + poor + miss, the reference implementation's minimum bad-poor count.
    pub min_bp: u32,
}

/// One run of a chart: the judge state, the replay being reproduced or recorded, and the
/// analysis clock. Sound is emitted through a [`SoundSink`], so a whole run can be reproduced with
/// no audio device present.
pub struct PlaySession {
    player: Player,
    setup: Setup,
    judge_offset_us: i64,
    auto_calibration: bool,
    replay: Option<ReplayTrack>,
    recording: Vec<ReplayEvent>,
    calibration: Calibration,
    analysis: AnalysisState,
    marks: TimingMarks,
    bga: BgaTimeline,
}

impl PlaySession {
    /// Start a run over an already laid-out `model` (the note shuffle is the caller's, so a seek
    /// can rebuild against the same lanes).
    pub fn new(model: Model, options: SessionOptions) -> Self {
        let bga = BgaTimeline::from_model(&model);
        let setup = Setup {
            autoplay: options.autoplay,
            gauge: options.gauge,
            judge_rate_percent: options.judge_rate_percent,
            auto_lanes: options.auto_lanes,
            seed: options.seed,
        };
        let player = build_player(model, &setup);
        PlaySession {
            player,
            setup,
            judge_offset_us: options.judge_offset_us,
            auto_calibration: options.auto_calibration,
            replay: options.replay.map(|replay| ReplayTrack { events: replay.events, cursor: 0 }),
            recording: Vec::new(),
            calibration: Calibration::default(),
            analysis: AnalysisState::new(options.analysis),
            marks: TimingMarks::default(),
            bga,
        }
    }

    /// Advance one frame: reproduce any replay input the audible axis has reached, book the sounds
    /// that fall inside the schedule axis, judge up to the audible axis, then move the BGA on.
    pub fn tick(&mut self, clock: SessionClock, sink: &mut dyn SoundSink) {
        self.feed_replay(clock.audible_us, sink);
        self.player.update_schedule(clock.scheduled_us, |event| sink.play(scheduled_sound(event)));
        self.player.update_judge(clock.audible_us);
        self.bga.advance(clock.audible_us);
    }

    /// Take one lane press at the instant it arrived, recording it for the replay and judging it at
    /// the configured offset. The keysound sounds now, not at the note's time.
    pub fn press(&mut self, lane: usize, raw_us: i64, sink: &mut dyn SoundSink) -> Option<JudgeResult> {
        self.recording.push(ReplayEvent { t: raw_us, lane, press: true });
        let judge_us = raw_us + self.judge_offset_us;
        let result = self.player.press(lane, judge_us, |event| sink.play(immediate_sound(event)));
        if let (true, Some(result)) = (self.auto_calibration, result.as_ref()) {
            self.calibration.record(result);
        }
        result
    }

    /// Take one lane release at the instant it arrived, recording it for the replay.
    pub fn release(&mut self, lane: usize, raw_us: i64) -> Option<JudgeResult> {
        self.recording.push(ReplayEvent { t: raw_us, lane, press: false });
        self.player.release(lane, raw_us + self.judge_offset_us)
    }

    /// Rebuild the judge state at an arbitrary song time by re-running the recorded inputs from the
    /// start, so scrubbing backwards is exactly as correct as scrubbing forwards. Only a replay run
    /// can be scrubbed.
    pub fn seek(&mut self, target_us: i64) {
        if self.replay.is_none() {
            return;
        }
        let target_us = target_us.max(0);
        let offset_us = self.judge_offset_us;
        let mut player = build_player(self.player.model().clone(), &self.setup);
        let mut cursor = 0;
        if let Some(track) = self.replay.as_ref() {
            for event in &track.events {
                if event.t > target_us {
                    break;
                }
                if event.press {
                    player.press(event.lane, event.t + offset_us, |_| {});
                } else {
                    player.release(event.lane, event.t + offset_us);
                }
                cursor += 1;
            }
        }
        player.update(target_us, |_| {});
        self.player = player;
        if let Some(track) = self.replay.as_mut() {
            track.cursor = cursor;
        }
        self.analysis.manual = true;
        self.analysis.position_us = target_us;
        self.marks.clear();
    }

    /// Whether the run is over and the result is due: every note has gone past and the tail silence
    /// has elapsed. An analysis run never ends on its own — the player is scrubbing it.
    pub fn is_finished(&self, song_us: i64) -> bool {
        !self.analysis.enabled && self.player.judge().total_notes() > 0 && song_us > self.player.last_time_us() + PLAY_TAIL_US
    }

    /// Whether there is nothing left to hit, so leaving the field should show the result rather
    /// than discard the run.
    pub fn all_notes_resolved(&self) -> bool {
        let judge = self.player.judge();
        judge.total_notes() > 0 && judge.total_judged() >= judge.total_notes()
    }

    /// Everything the result screen needs from this run, as plain values.
    pub fn summary(&self) -> PlaySummary {
        let judge = self.player.judge();
        let counts = judge.counts;
        PlaySummary {
            counts,
            ex_score: judge.ex_score,
            max_ex_score: judge.total_notes() * 2,
            max_combo: judge.max_combo,
            total_notes: judge.total_notes(),
            total_judged: judge.total_judged(),
            fast: judge.fast,
            slow: judge.slow,
            early: judge.early,
            late: judge.late,
            avg_judge_us: judge.avg_judge_us(),
            empty_poor: judge.empty_poor,
            gauge_value: judge.gauge.value(),
            clear_lamp: judge.clear_lamp(),
            min_bp: counts[3] + counts[4] + counts[5],
        }
    }

    /// Live judge state, for the play HUD.
    pub fn judge(&self) -> &JudgeEngine {
        self.player.judge()
    }

    /// The chart being played, laid out as this run sees it.
    pub fn model(&self) -> &Model {
        self.player.model()
    }

    /// Per-lane key-beam press timestamps.
    pub fn beam_on(&self) -> &[i64] {
        self.player.beam_on()
    }

    /// Per-lane key-beam release timestamps.
    pub fn beam_off(&self) -> &[i64] {
        self.player.beam_off()
    }

    /// Per-lane key-bomb state.
    pub fn bomb(&self) -> &[(i64, u8)] {
        self.player.bomb()
    }

    /// Song time of the chart's last timeline row.
    pub fn last_time_us(&self) -> i64 {
        self.player.last_time_us()
    }

    /// The note-shuffle seed this run was laid out with.
    pub fn seed(&self) -> u64 {
        self.setup.seed
    }

    /// Whether this run is reproducing a replay rather than taking input.
    pub fn is_replay(&self) -> bool {
        self.replay.is_some()
    }

    /// The inputs recorded so far, in arrival order.
    pub fn recorded_events(&self) -> &[ReplayEvent] {
        &self.recording
    }

    /// Shift the judgement of later inputs, following a live change of the judge offset.
    pub fn set_judge_offset_us(&mut self, offset_us: i64) {
        self.judge_offset_us = offset_us;
    }

    /// Follow a live change of the auto-calibration setting.
    pub fn set_auto_calibration(&mut self, enabled: bool) {
        self.auto_calibration = enabled;
    }

    /// The mean timing error auto-calibration proposes, once enough accurate hits have landed.
    pub fn calibration_mean_us(&self) -> Option<i64> {
        self.calibration.mean_us()
    }

    /// How many hits auto-calibration has accepted.
    pub fn calibration_samples(&self) -> u32 {
        self.calibration.count
    }

    /// Whether the replay-analysis overlay and its playback controls are active.
    pub fn analysis_enabled(&self) -> bool {
        self.analysis.enabled
    }

    /// Follow a live change of the REPLAY ANALYSIS setting.
    pub fn set_analysis_enabled(&mut self, enabled: bool) {
        self.analysis.enabled = enabled;
    }

    /// Whether playback has been taken over by the analysis controls, in which case the run is on
    /// the virtual clock and its keysounds are muted.
    pub fn analysis_manual(&self) -> bool {
        self.analysis.enabled && self.analysis.manual
    }

    pub fn analysis_paused(&self) -> bool {
        self.analysis.paused
    }

    pub fn analysis_rate(&self) -> f64 {
        self.analysis.rate
    }

    /// Where the virtual analysis clock currently stands.
    pub fn analysis_position_us(&self) -> i64 {
        self.analysis.position_us
    }

    /// Keep the virtual clock on the live position, so a first manual control resumes from what is
    /// being heard rather than from the start.
    pub fn sync_analysis_position(&mut self, song_us: i64) {
        if self.analysis.enabled {
            self.analysis.position_us = song_us;
        }
    }

    /// Move the virtual clock on by one frame at the current rate, and report where it now stands.
    pub fn advance_analysis(&mut self, frame_us: i64) -> i64 {
        if !self.analysis.paused {
            self.analysis.position_us = (self.analysis.position_us + (frame_us as f64 * self.analysis.rate) as i64).max(0);
        }
        self.analysis.position_us
    }

    /// Pause or resume analysis playback, taking manual control of it.
    pub fn toggle_analysis_pause(&mut self) {
        self.analysis.manual = true;
        self.analysis.paused = !self.analysis.paused;
    }

    /// Step the analysis playback rate, taking manual control of it.
    pub fn adjust_analysis_rate(&mut self, steps: i32) {
        self.analysis.manual = true;
        let moved = self.analysis.rate + steps as f64 * ANALYSIS_RATE_STEP;
        self.analysis.rate = moved.clamp(ANALYSIS_RATE_MIN, ANALYSIS_RATE_MAX);
    }

    /// The most recent judged inputs, oldest first.
    pub fn recent_marks(&self, count: usize) -> &[TimingMark] {
        self.marks.recent(count)
    }

    /// The BGA frame currently showing, or a negative id before the chart's first BGA event.
    pub fn bga_frame(&self) -> i32 {
        self.bga.current
    }

    /// Reproduce every recorded input whose raw time the audible axis has reached, judging each at
    /// the configured offset so the original run comes out identical.
    fn feed_replay(&mut self, audible_us: i64, sink: &mut dyn SoundSink) {
        let offset_us = self.judge_offset_us;
        while let Some(event) = self.replay.as_mut().and_then(|track| track.next_due(audible_us)) {
            let result = if event.press {
                self.player.press(event.lane, event.t + offset_us, |emitted| sink.play(immediate_sound(emitted)))
            } else {
                self.player.release(event.lane, event.t + offset_us)
            };
            if let Some(result) = result {
                self.marks.push(TimingMark { lane: result.lane, delta_us: result.delta_us, judge: result.judge as u8 });
            }
        }
    }
}

/// A judge player set up the way this run was configured, used both to start the run and to rebuild
/// it at a seek target.
fn build_player(model: Model, setup: &Setup) -> Player {
    let mut player = Player::new(model, setup.autoplay);
    player.set_gauge(setup.gauge);
    player.set_judge_rate(setup.judge_rate_percent);
    if !setup.auto_lanes.is_empty() {
        player.set_auto_lanes(setup.auto_lanes.clone());
    }
    player
}

/// A keysound booked ahead on the song clock: autoplay accompaniment and autoplay note sounds.
fn scheduled_sound(event: PlayEvent) -> SoundRequest {
    SoundRequest {
        wav: event.wav.max(0) as u32,
        source: event.source,
        gain: KEYSOUND_GAIN,
        pan: KEYSOUND_PAN,
        pitch: KEYSOUND_PITCH,
        at: SoundTime::Scheduled(event.at_us),
    }
}

/// A keysound wanted at once: an input's own sound, which must not wait for the scheduling
/// lookahead and must not move with the judge offset.
fn immediate_sound(event: PlayEvent) -> SoundRequest {
    SoundRequest { wav: event.wav.max(0) as u32, source: event.source, gain: KEYSOUND_GAIN, pan: KEYSOUND_PAN, pitch: KEYSOUND_PITCH, at: SoundTime::Immediate }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_chart::to_model;
    use rbms_model::Mode;
    use rbms_parser::parse;

    /// A sink that keeps every request, so a test can assert what a run asked to be played.
    #[derive(Debug, Default)]
    struct RecordingSink {
        sounds: Vec<SoundRequest>,
        stopped: u32,
    }

    impl SoundSink for RecordingSink {
        fn play(&mut self, sound: SoundRequest) {
            self.sounds.push(sound);
        }

        fn stop_all(&mut self) {
            self.stopped += 1;
        }
    }

    fn model(bms: &[u8]) -> Model {
        to_model(&parse(bms), Mode::BEAT_7K)
    }

    fn one_note() -> Model {
        model(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n")
    }

    fn replay_of(events: Vec<ReplayEvent>) -> Replay {
        Replay {
            chart_path: String::new(),
            md5: String::new(),
            mode: Mode::BEAT_7K.name.to_string(),
            random: String::new(),
            seed: 0,
            offset_ms: 0,
            scratch_auto: false,
            gauge: String::new(),
            events,
        }
    }

    #[test]
    fn a_press_is_judged_at_the_offset_while_its_keysound_stays_immediate() {
        for offset_ms in [-200_i64, -30, 0, 30, 200] {
            let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: offset_ms * 1000, ..Default::default() });
            let mut sink = RecordingSink::default();
            session.press(0, 1_000_000, &mut sink);
            assert_eq!(sink.sounds.len(), 1, "offset {offset_ms}ms still sounds the keysound once");
            assert_eq!(sink.sounds[0].at, SoundTime::Immediate, "offset {offset_ms}ms must not move the sound");
            assert_eq!(sink.sounds[0].source, PlaySource::Key);
        }
    }

    #[test]
    fn the_offset_moves_which_note_a_press_lands_on() {
        let hit_at = |offset_us: i64| {
            let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: offset_us, ..Default::default() });
            session.press(0, 2_000_000, &mut NullSink).map(|result| result.delta_us)
        };
        assert_eq!(hit_at(0), Some(0), "an exact press on the note has no timing error");
        assert_eq!(hit_at(30_000), Some(-30_000), "a +30ms offset judges the same press 30ms late");
        assert_eq!(hit_at(-45_000), Some(45_000), "a -45ms offset judges it 45ms early");
    }

    #[test]
    fn an_input_is_recorded_at_the_instant_it_arrived_not_at_the_offset_judgement() {
        let mut session = PlaySession::new(one_note(), SessionOptions { judge_offset_us: 30_000, ..Default::default() });
        session.press(0, 2_000_000, &mut NullSink);
        session.release(0, 2_050_000);
        assert_eq!(session.recorded_events().len(), 2);
        assert_eq!(session.recorded_events()[0].t, 2_000_000, "the raw input time is what a replay reproduces");
        assert!(session.recorded_events()[0].press);
        assert_eq!(session.recorded_events()[1].t, 2_050_000);
        assert!(!session.recorded_events()[1].press);
    }

    #[test]
    fn autoplay_books_its_sounds_on_the_schedule_axis_only() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        session.tick(SessionClock { audible_us: 0, scheduled_us: 3_000_000 }, &mut sink);
        assert!(!sink.sounds.is_empty(), "the whole booking horizon is scheduled at once");
        assert!(
            sink.sounds.iter().all(|sound| matches!(sound.at, SoundTime::Scheduled(_))),
            "an autoplay sound is booked ahead, never collapsed onto the current callback"
        );
        assert_eq!(session.judge().total_judged(), 0, "nothing inside the booking horizon may be judged yet");
    }

    #[test]
    fn judging_follows_the_audible_axis_however_far_the_scheduler_has_run_ahead() {
        let mut ahead = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut level = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        for step in 0..40 {
            let audible_us = step * 100_000;
            ahead.tick(SessionClock { audible_us, scheduled_us: audible_us + 500_000 }, &mut NullSink);
            level.tick(SessionClock::at(audible_us), &mut NullSink);
        }
        assert_eq!(ahead.summary(), level.summary(), "the scheduler's lookahead must not shift a judgement");
    }

    #[test]
    fn accompaniment_and_note_sounds_are_tagged_with_their_own_source() {
        let m = model(b"#BPM 120\r\n#WAV01 a.wav\r\n#WAV02 b.wav\r\n#00101:02\r\n#00111:01\r\n");
        let mut session = PlaySession::new(m, SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        session.tick(SessionClock::at(3_000_000), &mut sink);
        assert!(sink.sounds.iter().any(|sound| sound.source == PlaySource::Bgm), "the BGM channel is tagged as accompaniment");
        assert!(sink.sounds.iter().any(|sound| sound.source == PlaySource::Key), "an autoplayed note is tagged as a keysound");
    }

    #[test]
    fn every_emitted_sound_carries_the_neutral_voice_parameters() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        session.tick(SessionClock::at(3_000_000), &mut sink);
        session.press(0, 3_000_000, &mut sink);
        assert!(!sink.sounds.is_empty());
        for sound in &sink.sounds {
            assert_eq!(sound.gain, KEYSOUND_GAIN);
            assert_eq!(sound.pan, KEYSOUND_PAN);
            assert_eq!(sound.pitch, KEYSOUND_PITCH);
        }
    }

    #[test]
    fn a_null_sink_changes_nothing_about_the_judgement() {
        let mut heard = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut silent = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let mut sink = RecordingSink::default();
        heard.tick(SessionClock::at(3_000_000), &mut sink);
        silent.tick(SessionClock::at(3_000_000), &mut NullSink);
        assert!(!sink.sounds.is_empty(), "the recording sink did hear the run");
        assert_eq!(heard.summary(), silent.summary());
    }

    #[test]
    fn auto_calibration_only_counts_accurate_hits_and_waits_for_enough_of_them() {
        let mut session = PlaySession::new(one_note(), SessionOptions { auto_calibration: true, ..Default::default() });
        session.press(0, 2_000_000, &mut NullSink);
        assert_eq!(session.calibration_samples(), 1, "an accurate hit is accepted");
        assert_eq!(session.calibration_mean_us(), None, "one sample is not enough to propose an offset");
    }

    #[test]
    fn auto_calibration_ignores_inputs_while_it_is_switched_off() {
        let mut session = PlaySession::new(one_note(), SessionOptions { auto_calibration: false, ..Default::default() });
        session.press(0, 2_000_000, &mut NullSink);
        assert_eq!(session.calibration_samples(), 0);
        session.set_auto_calibration(true);
        session.release(0, 2_010_000);
        session.press(0, 2_000_000, &mut NullSink);
        assert_eq!(session.calibration_samples(), 0, "the note was already consumed, so there is nothing to accept");
    }

    #[test]
    fn a_wildly_late_input_never_calibrates_the_offset() {
        let mut calibration = Calibration::default();
        for _ in 0..CALIBRATION_MIN_SAMPLES {
            calibration.record(&JudgeResult { judge: rbms_judge::Judge::PerfectGreat, lane: 0, note_index: 0, fast: true, delta_us: 4_000 });
        }
        assert_eq!(calibration.mean_us(), Some(4_000));
        calibration.record(&JudgeResult { judge: rbms_judge::Judge::Poor, lane: 0, note_index: 0, fast: false, delta_us: -500_000 });
        calibration.record(&JudgeResult { judge: rbms_judge::Judge::Good, lane: 0, note_index: 0, fast: false, delta_us: CALIBRATION_MAX_ABS_DELTA_US + 1 });
        assert_eq!(calibration.mean_us(), Some(4_000), "neither a POOR nor an out-of-band hit moved the mean");
    }

    #[test]
    fn the_analysis_rate_steps_between_its_limits() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        assert_eq!(session.analysis_rate(), 1.0);
        for _ in 0..40 {
            session.adjust_analysis_rate(1);
        }
        assert_eq!(session.analysis_rate(), ANALYSIS_RATE_MAX);
        for _ in 0..80 {
            session.adjust_analysis_rate(-1);
        }
        assert_eq!(session.analysis_rate(), ANALYSIS_RATE_MIN);
        assert!(session.analysis_manual(), "touching a playback control takes manual control");
    }

    #[test]
    fn a_paused_analysis_clock_stands_still() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        session.sync_analysis_position(1_000_000);
        assert_eq!(session.advance_analysis(16_000), 1_016_000);
        session.toggle_analysis_pause();
        assert_eq!(session.advance_analysis(16_000), 1_016_000, "a paused clock ignores the frame");
        session.toggle_analysis_pause();
        assert_eq!(session.advance_analysis(16_000), 1_032_000);
    }

    #[test]
    fn the_analysis_clock_scales_with_the_playback_rate_and_never_goes_negative() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        session.adjust_analysis_rate(-2);
        assert_eq!(session.analysis_rate(), 0.5);
        assert_eq!(session.advance_analysis(100_000), 50_000);
        assert_eq!(session.advance_analysis(-1_000_000), 0, "scrubbing back past the start stops at zero");
    }

    #[test]
    fn the_virtual_clock_only_follows_the_live_one_while_analysis_is_on() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: false, ..Default::default() });
        session.sync_analysis_position(5_000_000);
        assert_eq!(session.analysis_position_us(), 0);
        session.set_analysis_enabled(true);
        session.sync_analysis_position(5_000_000);
        assert_eq!(session.analysis_position_us(), 5_000_000);
    }

    #[test]
    fn the_overlay_keeps_only_the_most_recent_judged_inputs() {
        let mut marks = TimingMarks::default();
        for i in 0..(TIMING_MARK_CAPACITY as i64 + 5) {
            marks.push(TimingMark { lane: 0, delta_us: i, judge: 0 });
        }
        assert_eq!(marks.marks.len(), TIMING_MARK_CAPACITY);
        let recent = marks.recent(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent.last().map(|mark| mark.delta_us), Some(TIMING_MARK_CAPACITY as i64 + 4), "newest last");
        assert_eq!(marks.recent(1_000).len(), TIMING_MARK_CAPACITY, "asking for more than there are is not an error");
    }

    #[test]
    fn the_bga_frame_holds_until_the_next_event_is_reached() {
        let m = model(b"#BPM 120\r\n#BMP01 a.png\r\n#BMP02 b.png\r\n#00104:01\r\n#00204:02\r\n");
        let mut session = PlaySession::new(m, SessionOptions { autoplay: true, ..Default::default() });
        assert_eq!(session.bga_frame(), BGA_FRAME_NONE, "nothing is shown before the first event");
        session.tick(SessionClock::at(1_999_999), &mut NullSink);
        assert_eq!(session.bga_frame(), BGA_FRAME_NONE, "the first event has not been reached yet");
        session.tick(SessionClock::at(2_000_000), &mut NullSink);
        assert_eq!(session.bga_frame(), 1, "the first frame is shown from the moment it is due");
        session.tick(SessionClock::at(3_000_000), &mut NullSink);
        assert_eq!(session.bga_frame(), 1, "it holds until the next event");
        session.tick(SessionClock::at(5_000_000), &mut NullSink);
        assert_eq!(session.bga_frame(), 2);
    }

    #[test]
    fn a_run_finishes_only_after_the_tail_silence() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        let last = session.last_time_us();
        session.tick(SessionClock::at(last + PLAY_TAIL_US), &mut NullSink);
        assert!(!session.is_finished(last + PLAY_TAIL_US), "the tail is not over yet");
        assert!(session.is_finished(last + PLAY_TAIL_US + 1));
    }

    #[test]
    fn an_analysis_run_never_finishes_on_its_own() {
        let events = vec![ReplayEvent { t: 2_000_000, lane: 0, press: true }];
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, replay: Some(replay_of(events)), ..Default::default() });
        let past_the_end = session.last_time_us() + PLAY_TAIL_US * 4;
        session.tick(SessionClock::at(past_the_end), &mut NullSink);
        assert!(!session.is_finished(past_the_end), "the player is scrubbing, so the result screen must wait");
    }

    #[test]
    fn a_resolved_chart_reports_that_nothing_is_left_to_hit() {
        let mut session = PlaySession::new(one_note(), SessionOptions { autoplay: true, ..Default::default() });
        assert!(!session.all_notes_resolved(), "nothing has been judged yet");
        session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
        assert!(session.all_notes_resolved());
    }

    #[test]
    fn only_a_replay_run_can_be_scrubbed() {
        let mut session = PlaySession::new(one_note(), SessionOptions { analysis: true, ..Default::default() });
        session.seek(1_000_000);
        assert_eq!(session.analysis_position_us(), 0, "a live run has no recorded input to rebuild from");
        assert!(!session.analysis_manual());
    }

    #[test]
    fn a_session_reports_whether_it_is_reproducing_a_replay() {
        let live = PlaySession::new(one_note(), SessionOptions::default());
        assert!(!live.is_replay());
        let replayed = PlaySession::new(one_note(), SessionOptions { replay: Some(replay_of(Vec::new())), ..Default::default() });
        assert!(replayed.is_replay());
    }

    #[test]
    fn the_summary_reports_the_run_as_the_result_screen_reads_it() {
        let m = model(b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101\r\n");
        let notes = rbms_chart::count_playable_notes(&m);
        let mut session = PlaySession::new(m, SessionOptions { autoplay: true, ..Default::default() });
        session.tick(SessionClock::at(session.last_time_us() + PLAY_TAIL_US), &mut NullSink);
        let summary = session.summary();
        assert_eq!(summary.total_notes, notes as u32);
        assert_eq!(summary.counts[0], notes as u32, "autoplay hits everything perfectly");
        assert_eq!(summary.ex_score, summary.max_ex_score);
        assert_eq!(summary.max_ex_score, summary.total_notes * 2);
        assert_eq!(summary.max_combo, summary.total_notes);
        assert_eq!(summary.min_bp, 0);
        assert_eq!(summary.total_judged, summary.total_notes);
        for judge in 0..summary.counts.len() {
            assert_eq!(summary.early[judge] + summary.late[judge], summary.counts[judge], "the early/late split covers every judgement");
        }
    }

    #[test]
    fn the_seed_the_run_was_laid_out_with_is_reported_back() {
        let session = PlaySession::new(one_note(), SessionOptions { seed: 0x5EED, ..Default::default() });
        assert_eq!(session.seed(), 0x5EED);
    }
}
