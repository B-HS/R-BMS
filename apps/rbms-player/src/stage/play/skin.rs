//! What a play screen keeps for the skin that draws it, and how a frame of the run reaches it.
//!
//! A skin's play screen is fed from three sides. Its timers are switched by a
//! [`PlayTimerDriver`], which has to be told what happened: which keys are down, which lane's key
//! just went down, every judgement, and which lanes hold a long note. Its note field and its judge
//! object are handed data no property id can carry ([`rbms_render::FrameData`]): the chart under
//! the play head with what became of each note, the gauge, and the last judgement of each region.
//! And everything it reads by id comes from the host's clusters, which are lent one
//! [`PlayShown`](crate::skin_host::play::PlayShown) a frame.
//!
//! [`SkinFeed`] is what the screen keeps between frames to do that: the driver, the trace of what
//! the run has done so far ([`RunTrace`]), the chart's long notes lane by lane, and the chart as the
//! chart cluster and the loading graphs read it. A run no skin draws keeps none of it.

use rbms_judge::Judge;
use rbms_judge::matcher::ScratchDir;
use rbms_model::{Model, NoteKind};
use rbms_play::PlaySession;
use rbms_render::RecentHits;
use rbms_render::skin_render::frame::LaneLong;
use rbms_skin::loader::LoadedSkin;
use rbms_skin::timer::TimerState;

use super::trace::{LaneHand, LongSpan, RunTrace, Seen, lane_long};
use crate::skin_host::overview::ChartOverview;
use crate::skin_host::play_timers::{HcnPass, Judgement, LaneKeys, PlaySetup, PlayTimerDriver, Reached, SceneEvent, judge_regions};
use crate::skin_host::score::RunScore;
use crate::stage::result::judge_area;

/// How many of the run's latest judgements are kept for the strips a skin plots its timing on, which
/// is how many the reference keeps (`JudgeManager.recentJudges`).
const RECENT_HITS_KEPT: usize = 100;

/// The first judgement the reference leaves out of that record: a note that went by unhit.
const FIRST_UNRECORDED_JUDGE: Judge = Judge::Poor;

/// Microseconds in a millisecond, which is what a judgement's timing is kept in for those strips.
const MICROS_PER_MILLI: i64 = 1_000;

/// Milliseconds in a second: a judgement graph has a row for every second of the chart.
const MILLIS_PER_SECOND: i64 = 1_000;

/// The windows a run's key notes are judged in, as [`judge_area`] answers them.
type JudgeArea = [[i32; 2]; rbms_render::skin_render::graphs::TIMING_JUDGE_AREAS];

/// Every long note of `model`, per lane, in the order they are played.
///
/// The chart states a long note as a head in one timeline and an end in a later one; this pairs
/// them once so a frame can look a note's end up by its head instead of walking the chart.
fn long_spans_of(model: &Model) -> Vec<Vec<LongSpan>> {
    let lanes = model.timelines.first().map_or(0, |timeline| timeline.notes.len());
    let mut spans: Vec<Vec<LongSpan>> = vec![Vec::new(); lanes];
    let mut open: Vec<Option<i64>> = vec![None; lanes];
    for timeline in &model.timelines {
        for (lane, head) in open.iter_mut().enumerate() {
            let Some(Some(note)) = timeline.notes.get(lane) else {
                continue;
            };
            match note.kind {
                NoteKind::LongStart { .. } => *head = Some(timeline.time_us),
                NoteKind::LongEnd { .. } => {
                    if let Some(head_us) = head.take() {
                        spans[lane].push(LongSpan { head_us, end_us: timeline.time_us });
                    }
                }
                NoteKind::Normal | NoteKind::Mine { .. } => {}
            }
        }
    }
    spans
}

/// What the screen has to say about the run for one judgement to be reported: the scores the run
/// is held against, and the scene time the judgement is stamped with.
#[derive(Debug, Clone, Copy)]
pub(super) struct Standing {
    pub(super) now_us: i64,
    /// The best EX score the player had on the chart, and the target's.
    pub(super) best_score: u32,
    pub(super) target_score: u32,
    /// The combo the run began its chart with: what a course carried over from the stage before.
    pub(super) carried_combo: u32,
}

/// What a play screen keeps for the skin that draws it. See the module's own notes.
pub(super) struct SkinFeed {
    driver: PlayTimerDriver,
    /// Whether the game plays the chart by itself.
    autoplay: bool,
    /// Whether the driver was set up from the skin, which is read a frame or more after the screen
    /// is entered.
    has_skin: bool,
    /// How many judge regions the skin splits the lanes into.
    judge_regions: usize,
    trace: RunTrace,
    /// The chart's long notes, lane by lane.
    spans: Vec<Vec<LongSpan>>,
    /// The long note each lane has in hand this frame.
    longs: Vec<LaneLong>,
    /// Which way each lane was last pressed, which is the way a turntable that is down is turning.
    last_dir: Vec<ScratchDir>,
    /// The chart as the chart cluster and the loading graphs read it.
    overview: ChartOverview,
    /// The run's latest judgements better than a poor as `(how early in milliseconds, judgement)`,
    /// oldest first, and how many of them the run has recorded in all, for the strips a skin plots
    /// its timing on.
    recent: Vec<(i64, u8)>,
    recorded: usize,
    /// The windows the run's key notes are judged in, which those strips are ruled with.
    judge_area: JudgeArea,
}

impl SkinFeed {
    /// The feed of one run, set up without a skin: the skin is taken in when it has been read
    /// ([`SkinFeed::adopt_skin`]).
    pub(super) fn new(session: &PlaySession, autoplay: bool) -> SkinFeed {
        let model = session.model();
        let spans = long_spans_of(model);
        let mut trace = RunTrace::new();
        trace.reach(usize::try_from(rbms_play::last_event_time_ms(model) / MILLIS_PER_SECOND + 1).unwrap_or_default());
        SkinFeed {
            driver: PlayTimerDriver::new(PlaySetup::of(model, None, autoplay)),
            autoplay,
            has_skin: false,
            judge_regions: 0,
            trace,
            longs: vec![LaneLong::default(); spans.len()],
            last_dir: vec![ScratchDir::Forward; spans.len()],
            spans,
            overview: ChartOverview::of_model(model),
            recent: Vec::with_capacity(RECENT_HITS_KEPT),
            recorded: 0,
            judge_area: judge_area(session),
        }
    }

    /// The run's latest judgements, for the timing and the hit error visualisers.
    pub(super) fn recent_hits(&self) -> RecentHits<'_> {
        RecentHits::new(&self.recent).with_recorded(self.recorded).with_judge_area(self.judge_area)
    }

    /// Keep one judgement for the strips a skin plots the run's timing on. The reference records a
    /// judgement better than a poor, whoever gave it, and nothing else (`JudgeManager.java:654-658`),
    /// so a note that went by and a press that took none do not push a hit out of the record. Early
    /// is positive.
    fn keep_hit(&mut self, seen: Seen) {
        if seen.judge as u8 >= FIRST_UNRECORDED_JUDGE as u8 {
            return;
        }
        if self.recent.len() == RECENT_HITS_KEPT {
            self.recent.remove(0);
        }
        self.recent.push((seen.fast_us / MICROS_PER_MILLI, seen.judge as u8));
        self.recorded += 1;
    }

    /// Set the driver up from the skin, the first time the skin is there to set it up from: what a
    /// skin asks of its timers -- the judgement that fires a bomb, how many judge regions it has --
    /// is not known before it is read. Nothing has been judged by then: the skin is read while the
    /// screen loads.
    pub(super) fn adopt_skin(&mut self, session: &PlaySession, skin: Option<&LoadedSkin>, autoplay: bool) {
        let Some(skin) = skin.filter(|_| !self.has_skin) else {
            return;
        };
        self.has_skin = true;
        self.autoplay = autoplay;
        self.judge_regions = judge_regions(&skin.def);
        self.driver = PlayTimerDriver::new(PlaySetup::of(session.model(), Some(skin), autoplay));
    }

    pub(super) fn driver(&self) -> &PlayTimerDriver {
        &self.driver
    }

    pub(super) fn driver_mut(&mut self) -> &mut PlayTimerDriver {
        &mut self.driver
    }

    pub(super) fn trace(&self) -> &RunTrace {
        &self.trace
    }

    pub(super) fn longs(&self) -> &[LaneLong] {
        &self.longs
    }

    pub(super) fn overview(&self) -> &ChartOverview {
        &self.overview
    }

    /// Note which way a lane was pressed.
    pub(super) fn note_direction(&mut self, lane: usize, dir: ScratchDir) {
        if let Some(last) = self.last_dir.get_mut(lane) {
            *last = dir;
        }
    }

    /// Forget what the run did, for a run that was put somewhere else in its chart: every lane's
    /// timers go off, and the judgements on show go with them.
    pub(super) fn seeked(&mut self, timers: &mut TimerState, now_us: i64, chart_us: i64) {
        self.driver.apply(timers, now_us, SceneEvent::Seeked { chart_us });
        self.trace.reset();
        self.recent.clear();
        self.recorded = 0;
    }

    /// The keys of every lane as the timer driver reads them: a lane that is down is down by the
    /// key it was last pressed with.
    fn lane_keys(&self, down: &[bool]) -> Vec<LaneKeys> {
        down.iter()
            .enumerate()
            .map(|(lane, down)| if *down { LaneKeys::down(self.last_dir.get(lane).copied().unwrap_or_default()) } else { LaneKeys::UP })
            .collect()
    }

    /// Report the keys that went down and the judgements that landed since this was last asked, each
    /// with the combo it left. Answers whether one of them left the chart's own combo at nothing.
    ///
    /// The full combo is the chart's own: every note of it gone by on a combo as long, whatever a
    /// course carried in from the stage before (`BMSPlayer.java:1028-1029`).
    pub(super) fn pump(&mut self, timers: &mut TimerState, session: &PlaySession, standing: Standing) -> bool {
        let now_us = standing.now_us;
        for lane in self.trace.take_presses(session.beam_on()) {
            self.driver.apply(timers, now_us, SceneEvent::Pressed { lane });
        }
        let judge = session.judge();
        let seen = self.trace.take_judgements(judge);
        if seen.is_empty() {
            return false;
        }
        let run = RunScore::of_session(session, standing.best_score, standing.target_score);
        let stage_combo = self.trace.stage_combo(judge.combo, standing.carried_combo);
        let reached = Reached::of(&run.standing, judge.total_judged(), judge.total_notes(), stage_combo);
        let lane_count = self.driver.lanes().len();
        let mut broke = false;
        for seen in seen {
            self.driver.apply(timers, now_us, SceneEvent::Judged(Judgement { lane: seen.lane, judge: seen.judge, fast_us: seen.fast_us, reached }));
            self.trace.note_judgement(seen, lane_count, self.judge_regions, now_us);
            self.keep_hit(seen);
            broke |= self.trace.stage_combo(seen.combo, standing.carried_combo) == 0;
        }
        broke
    }

    /// One frame of the run as the skin's timers see it: the keys that are down, what landed since
    /// the last frame, and the long note each lane has in hand. Answers what [`SkinFeed::pump`]
    /// does.
    ///
    /// A lane's hold timer follows the note it holds or a hell charge note that is gaining
    /// (`JudgeManager.java:632`). Its two hell charge timers are both off while no such note is
    /// going by, and while the one going by has not been judged yet (`JudgeManager.java:311-315`).
    pub(super) fn frame(&mut self, timers: &mut TimerState, session: &PlaySession, down: &[bool], standing: Standing) -> bool {
        let now_us = standing.now_us;
        let keys = self.lane_keys(down);
        self.driver.apply(timers, now_us, SceneEvent::Input(&keys));
        let broke = self.pump(timers, session, standing);
        let judge = session.judge();
        for lane in 0..self.longs.len() {
            let hand = LaneHand { held: judge.long_note(lane), down: down.get(lane).copied().unwrap_or_default(), autoplay: self.autoplay };
            let long = lane_long(&self.spans[lane], &self.trace, lane, hand);
            self.longs[lane] = long;
            let gaining = long.passing.is_some() && long.increasing;
            self.driver.apply(timers, now_us, SceneEvent::LongNote { lane, held: long.processing.is_some() || gaining });
            let judged = long.passing.is_some_and(|head_us| self.trace.is_judged(lane, head_us));
            let pass = match (judged, long.increasing) {
                (false, _) => HcnPass::None,
                (true, true) => HcnPass::Active,
                (true, false) => HcnPass::Damage,
            };
            self.driver.apply(timers, now_us, SceneEvent::Hcn { lane, pass });
        }
        broke
    }
}
