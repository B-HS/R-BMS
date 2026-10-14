//! The run the skin preview shows on a play screen: a sample chart played by the engine itself, fed
//! to a skin the way the play screen feeds its own run.
//!
//! It lives under the play screen because it is made of that screen's parts -- the scene that walks
//! a run through its states on the skin's own times, the trace of what became of each note, the
//! timer driver's feed, and the chart as the note field is shown it -- none of which a module
//! outside this one can reach. Nothing here is the play screen itself: no sound is scheduled, no key
//! is read, no score is kept, and the run starts over when it ends.
//!
//! # How it ends
//!
//! One of two ways, which the run is made for ([`SampleEnding`]). A run shown for its skin closes as
//! a run does -- the last note, the margin after it, the fade -- and its scene is over, so the
//! preview begins the scene again and every state a skin draws comes round. A run shown for the play
//! options never closes: a moment after its last note a run of the chart is begun afresh under the
//! scene as it stands, so notes keep coming down and nothing is read again.
//!
//! Such a run is begun afresh the same way when the caller names another tempo
//! ([`PlaySample::show_tempo`]) and when the GAUGE row names another gauge, which a run is on from
//! its first note. Those two carry on from the same share of the way through the chart: the new run
//! is played by the same hand up to there at once, so the notes on show, the combo and the score are
//! the ones a run on the new terms would have reached.
//!
//! # The chart
//!
//! The chart is written here, for the mode the previewed screen draws ([`sample_model`]): a little
//! over twenty seconds of plain notes, a long note every few measures, the turntable where the mode
//! has one, and one change of tempo. It names no sound, no picture and nobody's song.
//!
//! A caller that previews the play options rather than a skin asks for the same notes at one tempo
//! of its choosing instead ([`SampleTempo::Steady`], [`PlaySample::at_tempo`]): with one tempo from
//! the first measure to the last, the tempo a held green number is held against is that tempo
//! whichever of a chart's tempos the player has named, and what is on show can be read against one
//! number.
//!
//! # Who plays it
//!
//! A hand that is written down beforehand ([`sample_session`]): the run is handed its presses as a
//! replay is, each a set distance early or late of its note, so the judgements a skin shows are not
//! all the best one and its early and late marks have something to say. Every press lands inside the
//! window of a good, so no gauge the player has chosen empties and the run always reaches its end.
//!
//! # The clock
//!
//! The chart stands where the play timer says it does, on the time the preview counts for itself.
//! The song clock, which is the sound system's, is never read.

use rbms_config::{Config, HeldTravel, PlayOptions, held_speed};
use rbms_judge::GaugeKind;
use rbms_judge::gauge::GaugeIndex;
use rbms_model::{Mode, Model, NoteKind};
use rbms_play::{NullSink, PlaySession, SessionClock, SessionOptions};
use rbms_render::skin_render::frame::{BgaExpand, GaugeScale, LaneNotes, first_lane_rect, lane_offsets};
use rbms_render::skin_render::graphs::{NoteDistribution, PlayCursor};
use rbms_render::{BgaFrame, FrameData, FrameSeries, GaugeFrame};
use rbms_skin::loader::skin_type_mode;
use rbms_skin::timer::{TimerId, TimerState, timer_id};
use rbms_store::{Replay, ReplayEvent, ReplayJudge};

use super::field::FieldChart;
use super::scene::{PlayScene, PlayTimes, RunEnd, SceneFacts, SceneStep, TIME_MARGIN_MS};
use super::skin::{SkinFeed, Standing};
use super::{BpmStats, LOADED, MICROS_PER_MILLI, whole_ms};
use crate::AppShared;
use crate::skin_host::options::SettingsView;
use crate::skin_host::play::{LaneSettings, PlayClock, PlayKind, PlayLive, PlayPhase, PlayShown, Scroll, travel_region_ms};
use crate::skin_host::play_timers::{FieldOffsets, PlayingFrame, SceneEvent};
use crate::skin_preview::SampleStep;
use crate::skin_screen::PlayDraw;
use crate::skin_screen::preview::{PreviewDrawn, PreviewFrame, PreviewScene};
use crate::stage::Canvas;
use crate::stage::result::judge_area;
use crate::target::{self, TargetContext};

/// What the sample chart says of itself. All of it is invented.
const SAMPLE_TITLE: &str = "SAMPLE TRACK 01";
const SAMPLE_ARTIST: &str = "PREVIEW ARTIST";
const SAMPLE_GENRE: &str = "PREVIEW";

/// The tempo the sample chart opens at and the one it changes to, and the measure it changes on.
const OPENING_BPM: u32 = 150;
const SECOND_BPM: u32 = 180;
const TEMPO_CHANGE_MEASURE: u32 = 8;

/// The channel a chart states a change of tempo on, as a whole tempo in two hexadecimal digits.
const TEMPO_CHANNEL: &str = "03";

/// The slowest and the fastest tempo a sample chart of one tempo is written at. A tempo outside
/// them is held to the nearer: a chart slower than the first would take minutes to cross the lane
/// once, and the second is the largest a chart file states in three digits.
const STEADY_BPM_MIN: u32 = 30;
const STEADY_BPM_MAX: u32 = 999;

/// The tempo a sample chart is written at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SampleTempo {
    /// The tempo the skin menu's sample has always had: it opens at one tempo and changes to
    /// another part of the way through, so a skin's tempo figures have something to show.
    Shipped,
    /// One tempo from the first measure to the last, in beats a minute.
    Steady(u32),
}

impl SampleTempo {
    /// The tempo the chart opens at, and the one it changes to when it changes.
    fn stated(self) -> (u32, Option<u32>) {
        match self {
            SampleTempo::Shipped => (OPENING_BPM, Some(SECOND_BPM)),
            SampleTempo::Steady(bpm) => (bpm.clamp(STEADY_BPM_MIN, STEADY_BPM_MAX), None),
        }
    }
}

/// What a sample run does once its chart is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SampleEnding {
    /// The run closes as a run does and its scene is over with it.
    Closes,
    /// A run of the chart is begun afresh under the scene as it stands, and the scene goes on.
    BeginsAgain,
}

/// How long after its last note a run that begins again goes on first, in milliseconds: long enough
/// for what the last note set off to be seen, and well short of the margin a run closes after.
const BEGIN_AGAIN_AFTER_MS: i64 = 1_000;

/// The timers a run leaves on that say what it came to: its last note gone by, its full combo, the
/// ranks and the scores it reached, and the judgement and the combo last on show. A run begun at the
/// top of its chart has come to none of it yet.
const RUN_OUTCOME_TIMERS: [TimerId; 13] = [
    timer_id::ENDOFNOTE_1P,
    timer_id::FULLCOMBO_1P,
    timer_id::SCORE_A,
    timer_id::SCORE_AA,
    timer_id::SCORE_AAA,
    timer_id::SCORE_BEST,
    timer_id::SCORE_TARGET,
    timer_id::JUDGE_1P,
    timer_id::JUDGE_2P,
    timer_id::JUDGE_3P,
    timer_id::COMBO_1P,
    timer_id::COMBO_2P,
    timer_id::COMBO_3P,
];

/// The tempo the scroll of a sample chart that holds `bpm` throughout is worked out at under
/// `play`: its own, and the one tempo the field scrolls at while CONSTANT is on
/// ([`super::field::CONSTANT_BPM`]). It is the tempo [`BpmStats::held`] answers for such a chart
/// while a green number is held, and the one the note field travels at either way.
pub(crate) fn steady_scroll_bpm(play: &PlayOptions, bpm: u32) -> f64 {
    if play.constant_speed { super::field::CONSTANT_BPM } else { f64::from(SampleTempo::Steady(bpm).stated().0) }
}

/// The measures of the sample chart that hold notes: the first is left empty, so nothing is on the
/// judgement line the moment the chart starts.
const FIRST_MEASURE: u32 = 1;
const LAST_MEASURE: u32 = 14;

/// One measure in this many holds a long note, and which of them it is.
const LONG_NOTE_EVERY: u32 = 4;
const LONG_NOTE_MEASURE: u32 = 2;

/// A long note of the sample chart: its head on the first quarter of the measure and its end on
/// the last.
const LONG_NOTE_ROW: &str = "01000001";

/// What a lane plays in one measure, on a grid of eighths: a rest, one note, or two notes half a
/// measure apart. The rows are ordered so that lanes side by side, which play rows side by side,
/// make a stair of notes rather than a chord.
const LANE_ROWS: [&str; 8] =
    ["", "0100000000000000", "0001000000010000", "0000010000000000", "0000000100000001", "0000000001000000", "0100000001000000", "0000000000000100"];

/// How far the row a lane plays moves on from one measure to the next. The lane beside it plays the
/// row after its own.
const ROW_STEP_PER_MEASURE: usize = 3;

/// What the turntable plays on every other measure, beginning with the first that holds notes, and
/// on every fourth.
const SCRATCH_OTHER_ROW: &str = "01";
const SCRATCH_OTHER_EVERY: u32 = 2;
const SCRATCH_FOURTH_ROW: &str = "0001";
const SCRATCH_FOURTH_EVERY: u32 = 4;

/// The level, the difficulty slot, the judge rank and the total the sample chart states. The total
/// is a low one, so a gauge that fills does so over most of the chart rather than in its first
/// measures.
const SAMPLE_HEADER: &str = "#PLAYLEVEL 7\n#DIFFICULTY 3\n#RANK 2\n#TOTAL 110\n#LNTYPE 1\n";

/// The channels one side of a mode writes its notes on: its keys in lane order, its turntable when
/// it has one, and the channel the long notes of one of its keys are written on.
struct SampleSide {
    keys: &'static [&'static str],
    scratch: Option<&'static str>,
    long: &'static str,
    /// Which of `keys` the long notes are on. Its plain notes are left out of a measure with one.
    long_key: usize,
}

const FIVE_KEYS_FIRST: SampleSide = SampleSide { keys: &["11", "12", "13", "14", "15"], scratch: Some("16"), long: "53", long_key: 2 };
const FIVE_KEYS_SECOND: SampleSide = SampleSide { keys: &["21", "22", "23", "24", "25"], scratch: Some("26"), long: "63", long_key: 2 };
const SEVEN_KEYS_FIRST: SampleSide = SampleSide { keys: &["11", "12", "13", "14", "15", "18", "19"], scratch: Some("16"), long: "54", long_key: 3 };
const SEVEN_KEYS_SECOND: SampleSide = SampleSide { keys: &["21", "22", "23", "24", "25", "28", "29"], scratch: Some("26"), long: "64", long_key: 3 };

/// The nine buttons of a chart with no turntable: five on the first side's channels and four on the
/// second's, which is how such a chart is written.
const NINE_KEYS: SampleSide = SampleSide { keys: &["11", "12", "13", "14", "15", "22", "23", "24", "25"], scratch: None, long: "53", long_key: 2 };

/// The sides a sample chart of each mode is written on.
const SAMPLE_SIDES: [(Mode, &[SampleSide]); 5] = [
    (Mode::BEAT_5K, &[FIVE_KEYS_FIRST]),
    (Mode::BEAT_7K, &[SEVEN_KEYS_FIRST]),
    (Mode::POPN_9K, &[NINE_KEYS]),
    (Mode::BEAT_10K, &[FIVE_KEYS_FIRST, FIVE_KEYS_SECOND]),
    (Mode::BEAT_14K, &[SEVEN_KEYS_FIRST, SEVEN_KEYS_SECOND]),
];

/// The sample chart as a chart file would state it, on `sides` and at `tempo`.
fn sample_chart(sides: &[SampleSide], tempo: SampleTempo) -> String {
    let (opening_bpm, second_bpm) = tempo.stated();
    let mut chart = format!("#PLAYER 1\n#GENRE {SAMPLE_GENRE}\n#TITLE {SAMPLE_TITLE}\n#ARTIST {SAMPLE_ARTIST}\n#BPM {opening_bpm}\n{SAMPLE_HEADER}");
    if let Some(second_bpm) = second_bpm {
        chart.push_str(&format!("#{TEMPO_CHANGE_MEASURE:03}{TEMPO_CHANNEL}:{second_bpm:02X}\n"));
    }
    for measure in FIRST_MEASURE..=LAST_MEASURE {
        let has_long_note = measure % LONG_NOTE_EVERY == LONG_NOTE_MEASURE;
        for side in sides {
            for (index, channel) in side.keys.iter().enumerate() {
                if has_long_note && index == side.long_key {
                    chart.push_str(&format!("#{measure:03}{}:{LONG_NOTE_ROW}\n", side.long));
                    continue;
                }
                let row = LANE_ROWS[(measure as usize * ROW_STEP_PER_MEASURE + index) % LANE_ROWS.len()];
                if !row.is_empty() {
                    chart.push_str(&format!("#{measure:03}{channel}:{row}\n"));
                }
            }
            let scratch_row = match (measure % SCRATCH_OTHER_EVERY == FIRST_MEASURE % SCRATCH_OTHER_EVERY, measure.is_multiple_of(SCRATCH_FOURTH_EVERY)) {
                (true, _) => Some(SCRATCH_OTHER_ROW),
                (false, true) => Some(SCRATCH_FOURTH_ROW),
                (false, false) => None,
            };
            if let (Some(channel), Some(row)) = (side.scratch, scratch_row) {
                chart.push_str(&format!("#{measure:03}{channel}:{row}\n"));
            }
        }
    }
    chart
}

/// The sample chart for a run of `mode`, or `None` for a mode no sample is written for.
pub(crate) fn sample_model(mode: Mode) -> Option<Model> {
    sample_model_at(mode, SampleTempo::Shipped)
}

/// The sample chart for a run of `mode` written at `tempo`, or `None` for a mode no sample is
/// written for.
fn sample_model_at(mode: Mode, tempo: SampleTempo) -> Option<Model> {
    let (_, sides) = SAMPLE_SIDES.iter().find(|(written, _)| *written == mode)?;
    let source = rbms_parser::parse_with(sample_chart(sides, tempo).as_bytes(), Default::default());
    Some(rbms_chart::to_model(&source, mode))
}

/// Which judgement the hand that plays the sample aims a note at.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Aim {
    Best,
    Great,
    Good,
}

/// One note in this many is aimed at a great, and one in this many at a good. The second is taken
/// first, so a note that is both is a good.
const GREAT_EVERY: usize = 5;
const GOOD_EVERY: usize = 23;

/// One press in this many is early; the rest are late.
const EARLY_EVERY: usize = 2;

/// How many different distances inside the best window the hand lands at, and how much of the
/// window the furthest of them is.
const BEST_SPREAD_STEPS: usize = 4;
const BEST_SPREAD_SHARE: f64 = 0.6;

/// How far between the two windows either side of it a great or a good is aimed: halfway.
const BETWEEN_WINDOWS: f64 = 0.5;

/// How long the hand holds a plain note down, in microseconds.
const TAP_US: i64 = 80_000;

/// The judgements the hand's aims are measured against, as [`judge_area`] numbers them.
const AREA_BEST: usize = 0;
const AREA_GREAT: usize = 1;
const AREA_GOOD: usize = 2;

/// What a play timer started this frame reads: the chart at its beginning.
const CHART_START_US: i64 = 0;

/// The scroll rate of a chart before its first timeline: its own, unchanged.
const UNSCALED_SCROLL: f64 = 1.0;

/// The play speed of a run nobody is slowing down, in percent.
const FULL_SPEED_PERCENT: i32 = 100;

/// How far from its note the hand's `count`-th press lands, in microseconds, positive when late.
///
/// `area` is the window of each judgement in milliseconds, late bound then early
/// ([`judge_area`]). Every other press is early. A great or a good whose window is no wider than
/// the one inside it is aimed at the best instead.
fn press_distance_us(area: &[[i32; 2]], count: usize) -> i64 {
    let aim = if count % GOOD_EVERY == GOOD_EVERY - 1 {
        Aim::Good
    } else if count % GREAT_EVERY == GREAT_EVERY - 1 {
        Aim::Great
    } else {
        Aim::Best
    };
    let early = count.is_multiple_of(EARLY_EVERY);
    let bound_us = |judgement: usize| {
        let [late, early_bound] = area.get(judgement).copied().unwrap_or_default();
        f64::from(if early { early_bound } else { -late }) * MICROS_PER_MILLI as f64
    };
    let best = bound_us(AREA_BEST);
    let between =
        |inner: usize, outer: usize| (bound_us(outer) > bound_us(inner)).then(|| bound_us(inner) + (bound_us(outer) - bound_us(inner)) * BETWEEN_WINDOWS);
    let inside_best = best * BEST_SPREAD_SHARE * (count % BEST_SPREAD_STEPS) as f64 / BEST_SPREAD_STEPS as f64;
    let distance = match aim {
        Aim::Best => inside_best,
        Aim::Great => between(AREA_BEST, AREA_GREAT).unwrap_or(inside_best),
        Aim::Good => between(AREA_GREAT, AREA_GOOD).unwrap_or(inside_best),
    };
    if early { -(distance as i64) } else { distance as i64 }
}

/// Every press and release of the hand that plays `model`, oldest first.
///
/// A plain note is pressed near its time and let go a moment later. A long note is pressed near its
/// head and let go on its end. A mine is left alone.
fn sample_inputs(model: &Model, area: &[[i32; 2]]) -> Vec<ReplayEvent> {
    let mut events = Vec::new();
    let mut count = 0;
    for line in &model.timelines {
        for (lane, note) in line.notes.iter().enumerate() {
            let Some(note) = note else {
                continue;
            };
            let input = |t: i64, press: bool| ReplayEvent { t, lane, press, backward: false };
            match note.kind {
                NoteKind::Normal => {
                    let pressed_us = line.time_us + press_distance_us(area, count);
                    events.push(input(pressed_us, true));
                    events.push(input(pressed_us + TAP_US, false));
                    count += 1;
                }
                NoteKind::LongStart { .. } => {
                    events.push(input(line.time_us + press_distance_us(area, count), true));
                    count += 1;
                }
                NoteKind::LongEnd { .. } => events.push(input(line.time_us, false)),
                NoteKind::Mine { .. } => {}
            }
        }
    }
    events.sort_by_key(|event| event.t);
    events
}

/// What a stage of a course carries into the one after it: the gauge it ended on and the combo it
/// ended with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Carried {
    pub(crate) gauge: f32,
    pub(crate) combo: u32,
}

/// A run of `model` on the gauge the player has chosen, with the hand that plays it written down:
/// nothing has been played yet. `carried` makes it a stage of a course, begun on the course's own
/// gauge.
fn sample_session(model: Model, config: &Config, carried: Option<Carried>) -> PlaySession {
    let area = judge_area(&PlaySession::new(model.clone(), SessionOptions::default()));
    let replay = Replay {
        chart_path: String::new(),
        md5: model.md5.clone(),
        mode: model.mode.name.to_owned(),
        random: String::new(),
        seed: 0,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: ReplayJudge::default(),
        events: sample_inputs(&model, &area),
    };
    let options = SessionOptions {
        gauge: config.play.gauge,
        replay: Some(replay),
        initial_gauge: carried.map(|carried| (GaugeIndex::Class, carried.gauge)),
        initial_combo: carried.map_or(0, |carried| carried.combo),
        ..SessionOptions::default()
    };
    PlaySession::new(model, options)
}

/// The step the engine is run at when a sample is played through at once, and how far past the
/// chart's own playing time it is run so the last gauge sample is taken.
const PLAYED_THROUGH_STEP_US: i64 = 10_000;
const PLAYED_THROUGH_MARGIN_US: i64 = 1_000_000;

/// Play `session` from where it stands up to `chart_us` at once, a step of the engine at a time.
fn play_through(session: &mut PlaySession, chart_us: i64) {
    for frame in 0..=chart_us / PLAYED_THROUGH_STEP_US {
        session.tick(SessionClock::at(frame * PLAYED_THROUGH_STEP_US), &mut NullSink);
    }
}

/// The sample chart of `mode` played from its first moment to past its end, for the screens that
/// show a run that is over. `None` for a mode no sample is written for.
pub(crate) fn played_sample(mode: Mode, config: &Config, carried: Option<Carried>) -> Option<PlaySession> {
    let mut session = sample_session(sample_model(mode)?, config, carried);
    let run_us = session.play_time_ms() * MICROS_PER_MILLI + PLAYED_THROUGH_MARGIN_US;
    play_through(&mut session, run_us);
    Some(session)
}

/// Where a run that takes the place of another begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunStart {
    /// At the top of its chart.
    Top,
    /// As far through its chart as the run it takes the place of had come.
    Carried,
}

/// A sample run on a play screen: the run, the scene it is walked through, and what the skin that
/// draws it is fed from.
pub(crate) struct PlaySample {
    /// The play screen being previewed.
    screen: i32,
    session: PlaySession,
    scene: PlayScene,
    feed: Box<SkinFeed>,
    bpm: BpmStats,
    /// The chart as the note field is shown it while CONSTANT or LEGACY NOTE is on.
    field: Option<FieldChart>,
    /// Where the chart stood on the last frame the run moved on.
    chart_us: i64,
    /// What the run does once its chart is over.
    ending: SampleEnding,
    /// The tempo the chart of the run on show is written at, and the one its caller last named.
    tempo: SampleTempo,
    asked: SampleTempo,
    /// The gauge the run on show was begun on.
    gauge: GaugeKind,
}

impl PlaySample {
    /// A run of the sample chart the skin menu shows for the play screen `screen`, at its loading
    /// state. `None` when the screen is no play screen a sample is written for.
    #[cfg(test)]
    pub(crate) fn new(screen: i32, config: &Config) -> Option<PlaySample> {
        PlaySample::at_tempo(screen, config, SampleTempo::Shipped)
    }

    /// A run of the sample chart for the play screen `screen` written at `tempo`, at its loading
    /// state. `None` when the screen is no play screen a sample is written for.
    ///
    /// The notes, the hand that plays them and everything the run is drawn from are the same at any
    /// tempo; a chart at a slower tempo takes longer to play through. The run closes when its chart
    /// is over ([`SampleEnding::Closes`]) unless it is told otherwise ([`PlaySample::ending`]).
    pub(crate) fn at_tempo(screen: i32, config: &Config, tempo: SampleTempo) -> Option<PlaySample> {
        let session = sample_session(sample_model_at(skin_type_mode(screen)?, tempo)?, config, None);
        let feed = Box::new(SkinFeed::new(&session, false));
        let bpm = BpmStats::of(session.model());
        Some(PlaySample {
            screen,
            session,
            scene: PlayScene::loading(),
            feed,
            bpm,
            field: None,
            chart_us: CHART_START_US,
            ending: SampleEnding::Closes,
            tempo,
            asked: tempo,
            gauge: config.play.gauge,
        })
    }

    /// The same run, ending as `ending` says.
    pub(crate) fn ending(self, ending: SampleEnding) -> PlaySample {
        PlaySample { ending, ..self }
    }

    /// Have the run go on at `tempo`: the step that follows begins a run of the chart written at
    /// it, as far through as this one has come, under the scene as it stands
    /// ([`PlaySample::step`]). `false` for a run that closes, which is at one tempo for the whole of
    /// its scene and takes no other.
    pub(crate) fn show_tempo(&mut self, tempo: SampleTempo) -> bool {
        if self.ending == SampleEnding::Closes {
            return false;
        }
        self.asked = tempo;
        true
    }

    /// The scroll speed the sample is shown at, which is the one a run of this chart would start
    /// at: the one the settings hold, or, while a green number is held from chart to chart, the one
    /// that holds it on this chart under the lane cover as it stands.
    fn shown_hispeed(&self, config: &Config) -> f64 {
        let play = &config.play;
        let held = self.bpm.held(play).and_then(|bpm| held_speed(&HeldTravel::of(play, bpm)));
        held.map_or(play.hispeed, |held| held.hispeed)
    }

    /// The tempo and the scroll in force at `chart_us`: those of the last timeline it has reached,
    /// and the chart's opening tempo before the first.
    fn tempo_at(&self, chart_us: i64) -> (f64, f64) {
        let timelines = &self.session.model().timelines;
        let reached = timelines.partition_point(|timeline| timeline.time_us <= chart_us);
        reached.checked_sub(1).map_or((self.session.model().init_bpm, UNSCALED_SCROLL), |last| (timelines[last].bpm, timelines[last].scroll))
    }

    /// Where the chart stands for a frame drawn at `now_us`: what the play timer has run for, cut to
    /// whole milliseconds the way the play screen cuts it, and the chart's beginning before the
    /// timer is on; and `lead_us` ahead of either, which is the player's judge offset. The field of
    /// a run is drawn that far ahead of the song and the hand that plays this one presses where it
    /// always does, so the JUDGE OFFSET row shows here as it does on a run: the notes reach the
    /// line that much before they are taken.
    fn drawn_chart_us(timers: &TimerState, now_us: i64, lead_us: i64) -> i64 {
        let stands_at_us = if timers.is_on(timer_id::PLAY) { whole_ms(now_us - timers.value_us(timer_id::PLAY)) * MICROS_PER_MILLI } else { CHART_START_US };
        stands_at_us + lead_us
    }

    /// Who the run is told to be played by, and the score it is paced against.
    ///
    /// A run shown for the play options follows the two rows that say so: AUTOPLAY makes it a run
    /// the game plays itself, and TARGET is settled on the sample chart as a run of it would settle
    /// it, with no record and no rival to measure against. A run shown for its skin is one played
    /// by hand with nothing to pace against, whatever those rows hold.
    fn told(&self, config: &Config) -> (PlayKind, u32, String) {
        if self.ending == SampleEnding::Closes {
            return (PlayKind::Play, 0, String::new());
        }
        let kind = if config.play.autoplay { PlayKind::Autoplay } else { PlayKind::Play };
        let context = TargetContext {
            total_notes: self.session.judge().total_notes(),
            custom_rate_percent: config.judge.target_rate_percent,
            ..TargetContext::default()
        };
        let settled = target::resolve(config.judge.target, config.judge.target_fallback, &context);
        let legend = settled.legend();
        (kind, settled.ex, legend)
    }

    /// The scores the run is held against at `now_us`, as the skin's timers read them: no best of
    /// the player's, and the target the run is paced against.
    fn held_against(&self, config: &Config, now_us: i64) -> Standing {
        Standing { now_us, best_score: 0, target_score: self.told(config).1, carried_combo: 0 }
    }

    /// Which lanes the hand has down, in lane order.
    fn lanes_down(&self) -> Vec<bool> {
        self.session.beam_on().iter().zip(self.session.beam_off()).map(|(on, off)| on > off).collect()
    }

    /// Where a run is to take the place of the one on show on this frame, for a run that begins
    /// again: at the top once the chart's last note is a moment behind, and where this one stands
    /// when another tempo was asked for or `play` names another gauge. `None` while the run on show
    /// goes on, which is always for a run that closes.
    fn due_run(&self, play: &PlayOptions) -> Option<RunStart> {
        if self.ending == SampleEnding::Closes {
            return None;
        }
        let last_note_ms = self.session.play_time_ms() - TIME_MARGIN_MS;
        if whole_ms(self.chart_us) > last_note_ms + BEGIN_AGAIN_AFTER_MS {
            return Some(RunStart::Top);
        }
        (self.asked != self.tempo || play.gauge != self.gauge).then_some(RunStart::Carried)
    }

    /// Begin a run of the chart in place of the one on show, written at the tempo asked for and on
    /// the gauge the settings name, with the scene left as it is: no state of it is gone back to and
    /// nothing of the skin is read again.
    ///
    /// A run that takes over part of the way through is played up to there at once by the hand
    /// that plays it, so its notes, its combo, its score and its gauge are the ones it would have
    /// had. What it did on the way is taken in without a word: the feed's first look at a run tells
    /// nothing, and the driver it is told to is not judging yet. Then the lanes' timers are put out
    /// and the bar lines behind the run counted, as for a run moved about in its chart, and the play
    /// timer is started where the run stands. A run begun at the top has the timers that say what a
    /// run came to put out as well.
    ///
    /// A scene that is not playing yet has none of that to do: the run waits at the top of its
    /// chart for the scene to start it.
    fn begin_run(&mut self, shared: &mut AppShared, now_us: i64, ready: bool, start: RunStart) {
        let Some(model) = skin_type_mode(self.screen).and_then(|mode| sample_model_at(mode, self.asked)) else {
            return;
        };
        let (was_ms, is_ms) = (rbms_play::last_note_time_ms(self.session.model()), rbms_play::last_note_time_ms(&model));
        let from_us = match start {
            RunStart::Carried if self.scene.is_playing() && was_ms > 0 => (self.chart_us as f64 * is_ms as f64 / was_ms as f64) as i64,
            RunStart::Carried | RunStart::Top => CHART_START_US,
        };
        self.session = sample_session(model, &shared.config, None);
        play_through(&mut self.session, from_us);
        *self.feed = SkinFeed::new(&self.session, false);
        self.bpm = BpmStats::of(self.session.model());
        self.field = None;
        self.chart_us = from_us;
        self.tempo = self.asked;
        self.gauge = shared.config.play.gauge;
        if !self.scene.is_playing() {
            return;
        }
        let skin = shared.skins.document(self.screen).filter(|_| ready);
        self.feed.adopt_skin(&self.session, skin, false);
        let standing = self.held_against(&shared.config, now_us);
        let timers = &mut shared.skin_timers;
        self.feed.driver_mut().apply(timers, now_us, SceneEvent::Seeked { chart_us: from_us });
        let down = self.lanes_down();
        self.feed.frame(timers, &self.session, &down, standing);
        self.feed.driver_mut().apply(timers, now_us, SceneEvent::Started { start_offset_us: from_us });
        if from_us == CHART_START_US {
            RUN_OUTCOME_TIMERS.into_iter().for_each(|timer| timers.off(timer));
        }
    }

    /// Move the run on to `now_us` on the preview's own time. `ready` is whether the skin that draws
    /// it can be drawn.
    ///
    /// The scene is the play screen's own: it loads for as long as the skin says, stands ready,
    /// plays, and closes on the fade after the last note. While it plays, the chart stands where the
    /// play timer has run to and the engine is handed that time. A run that has left its loading
    /// state stands still while its skin is away, which is while the skin is being read again.
    ///
    /// What the skin asks of its timers is taken in from the first read of it the run is drawn with,
    /// as the play screen takes it in, and a read that is not yet the one to be drawn is not asked.
    ///
    /// A run that begins again ([`SampleEnding::BeginsAgain`]) never tells its scene that the chart
    /// ran out: before the scene is asked, a run that is due takes the place of the one on show
    /// ([`PlaySample::due_run`]), so the scene only ever sees a chart that is playing.
    pub(crate) fn step(&mut self, shared: &mut AppShared, now_us: i64, ready: bool) -> SampleStep {
        if !ready && self.scene.phase() != PlayPhase::Preload {
            return SampleStep::Running;
        }
        if let Some(start) = self.due_run(&shared.config.play) {
            self.begin_run(shared, now_us, ready, start);
        }
        let skin = shared.skins.document(self.screen).filter(|_| ready);
        self.feed.adopt_skin(&self.session, skin, false);
        let times = PlayTimes::of_skin(skin);
        if self.scene.is_playing() && shared.skin_timers.is_on(timer_id::PLAY) {
            self.chart_us = (now_us - shared.skin_timers.value_us(timer_id::PLAY)).max(CHART_START_US);
            self.session.tick(SessionClock::at(self.chart_us), &mut NullSink);
        }
        let (bpm, _) = self.tempo_at(self.chart_us);
        let judge = self.session.judge();
        let facts = SceneFacts {
            now_us,
            times,
            loaded: ready,
            start: false,
            select: false,
            start_offset_us: CHART_START_US,
            playing: PlayingFrame {
                chart_us: self.chart_us,
                bpm,
                play_speed: FULL_SPEED_PERCENT,
                gauge_max: judge.gauge.selected().is_max(),
                past_notes: judge.total_judged(),
            },
            failed: self.session.is_failed(),
            end: RunEnd::of(self.session.play_time_ms(), whole_ms(self.chart_us)),
            notes_done: self.session.all_notes_resolved(),
            may_retry: false,
        };
        let step = self.scene.step(self.feed.driver_mut(), &mut shared.skin_timers, &facts);
        let down = self.lanes_down();
        let standing = self.held_against(&shared.config, now_us);
        self.feed.frame(&mut shared.skin_timers, &self.session, &down, standing);
        if matches!(step, SceneStep::Leave(_)) { SampleStep::Over } else { SampleStep::Running }
    }

    /// Make sure the chart the note field is shown is the one CONSTANT and LEGACY NOTE ask for as
    /// they stand.
    fn ensure_field(&mut self, config: &Config) {
        let (constant, legacy_note) = (config.play.constant_speed, config.play.legacy_note);
        let wanted = (constant || legacy_note).then_some((constant, legacy_note));
        if self.field.as_ref().map(FieldChart::settings) != wanted {
            self.field = FieldChart::of(&self.session.model().timelines, constant, legacy_note);
        }
    }

    /// The gauge the run is on, with the limits of every gauge it could be on.
    fn gauge_frame(&self) -> GaugeFrame {
        let gauges = &self.session.judge().gauge;
        let scales = GaugeIndex::ALL.map(|index| {
            let gauge = gauges.gauge_at(index);
            GaugeScale::new(gauge.min(), gauge.max(), gauge.border())
        });
        GaugeFrame::playing(gauges.selected_index().index(), gauges.value(), scales)
    }

    /// Draw one frame of the run into `at.rect`.
    ///
    /// Everything the document reads comes from the run as it stands, in the shapes the play screen
    /// hands its own: the property clusters from a [`PlayShown`], the note field from the chart and
    /// the trace of what became of each note, the judge object from the last judgement of each
    /// region, and the offsets the lift and the covers move objects by from the document's own
    /// first lane. The covers and the lift are the player's own settings, and the scroll speed is
    /// the one a run of this chart would start at ([`PlaySample::shown_hispeed`]). The chart has no
    /// background, so the skin's is the black of a chart without one.
    ///
    /// CONSTANT reaches the skin the way it reaches it from the play screen: the note field is
    /// handed the chart made over for it ([`FieldChart`]), the travel time is worked out at the
    /// field's one tempo, and the option is told as the assist it is ([`SettingsView::of_config`]).
    /// The lane settings' own flag of that name is the reference's lane renderer option, which this
    /// player has not, and stays off here as it does there.
    ///
    /// A run shown for its skin is shown as one played by hand, which is what the screen being
    /// previewed is for. A run shown for the play options is told as the AUTOPLAY and TARGET rows
    /// have it ([`PlaySample::told`]); the hand that plays it is the written one either way.
    pub(crate) fn draw(&mut self, shared: &AppShared, canvas: &mut Canvas<'_>, at: PreviewFrame) -> PreviewDrawn {
        self.ensure_field(&shared.config);
        let play = &shared.config.play;
        let settings = SettingsView::of_config(&shared.config);
        let chart_us = PlaySample::drawn_chart_us(&shared.skin_timers, at.now_us, shared.offset_us());
        let (bpm, scroll) = self.tempo_at(chart_us);
        let travel = self.field.as_ref().and_then(FieldChart::tempo).unwrap_or((bpm, scroll));
        let hispeed = self.shown_hispeed(&shared.config) as f32;
        let (kind, target_score, paced_by) = self.told(&shared.config);
        let live = PlayLive {
            phase: self.scene.phase(),
            kind,
            best_score: 0,
            target_score,
            judgements: self.feed.trace().sides(),
            lanes: LaneSettings {
                hispeed,
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
                now_bpm: bpm,
                main_bpm: self.bpm.main,
                min_bpm: self.bpm.min,
                max_bpm: self.bpm.max,
                region_ms: travel_region_ms(travel.0, hispeed, travel.1),
            },
            clock: PlayClock::of(&shared.skin_timers, at.now_us),
            judge_timing_ms: shared.config.judge.offset_ms,
            judge_timing_auto: shared.config.judge.auto_offset,
            bga_on: shared.config.display.bga,
            cover_keys_held: false,
            long_note_mode: settings.played.long_note_mode,
            played: settings.played,
            load_progress: LOADED,
        };
        let shown = PlayShown::of(&self.session, &live);
        let (lift, lanecover, hidden) =
            (play.enable_lift.then_some(play.lift), play.enable_cover.then_some(play.cover), play.enable_hidden.then_some(play.hidden));
        let field = shared.skins.document(self.screen).and_then(first_lane_rect).map(|first_lane| lane_offsets(first_lane, lift, lanecover, hidden));
        let offsets = self.feed.driver().offsets(FieldOffsets {
            lift: field.map(|field| field.lift),
            lane_cover: field.map(|field| field.lanecover),
            hidden_cover: field.map(|field| field.hidden),
        });
        let model = self.session.model();
        let shown_chart = self.field.as_ref().map_or(model.timelines.as_slice(), FieldChart::timelines);
        let opening_bpm = self.field.as_ref().and_then(FieldChart::tempo).map_or(model.init_bpm, |(bpm, _)| bpm);
        let notes = LaneNotes {
            hispeed,
            lift,
            lanecover,
            hidden,
            ln_mode: self.session.ln_mode().resolve(),
            states: self.feed.trace(),
            longs: self.feed.longs(),
            ..LaneNotes::new(shown_chart, chart_us, opening_bpm)
        };
        let overview = self.feed.overview();
        let chart = overview.meta(false);
        let chart_series = overview.series();
        let (judgements, early_late) = self.feed.trace().seconds();
        let distribution = NoteDistribution {
            judged: &self.session.judge().counts,
            judgements,
            early_late,
            playing: Some(PlayCursor::default()),
            ..chart_series.notes.unwrap_or_default()
        };
        let data = FrameData {
            notes: Some(&notes),
            gauge: Some(self.gauge_frame()),
            series: FrameSeries { recent_hits: Some(self.feed.recent_hits()), notes: Some(distribution), ..chart_series },
            bga: BgaFrame::blank(BgaExpand::default()),
            judge: self.feed.trace().regions(),
            ..FrameData::default()
        };
        shared.draw_skin_preview(canvas, at, PreviewScene::Play(PlayDraw { chart: &chart, shown: &shown, offsets: &offsets, paced_by: &paced_by, data }))
    }

    /// Where the run stands, for the tests that check it moves: its state, where the chart is in
    /// milliseconds, and the combo, the score and the gauge it has reached.
    #[cfg(test)]
    pub(crate) fn standing(&self) -> (PlayPhase, i64, u32, u32, f32) {
        let judge = self.session.judge();
        (self.scene.phase(), whole_ms(self.chart_us), judge.combo, judge.ex_score, judge.gauge.value())
    }

    /// The run on show, for the tests that read its chart, its gauge and its tally.
    #[cfg(test)]
    pub(crate) fn session(&self) -> &PlaySession {
        &self.session
    }
}

#[cfg(test)]
mod tests;
