//! What a run played by hand was, note by note: the trace it keeps with its record and sends once
//! the score server holds its score ([`rbms_ir::PlayTrace`]).
//!
//! Everything in it is read off the run as it ended, but for the scroll settings, which the player
//! moves while the run goes: those are watched once a frame ([`SpeedLog`]), so the trace says what
//! the run started on, what it ended on and the changes between, however the change was made (a
//! key, the wheel, the turntable, the offset adjusting itself). The changes, and the chart's tempo
//! points, are capped ([`rbms_ir::TRACE_MAX_SPEED_CHANGES`], [`rbms_ir::TRACE_MAX_TEMPO_POINTS`]):
//! past the cap the first and the last are kept, and the trace is sent all the same.
//!
//! The frame thread only copies out what the trace is made of ([`TraceSource::of`]); the save
//! thread lays it out, writes its body and keeps it beside the run's row
//! ([`crate::save_worker::SaveWorker::keep_trace`]).
//!
//! The notes are the chart the run judged: laid out by its lane option, with the lanes SCRATCH AUTO
//! played left out. Each carries the state the run left on it and the timing of the judgment that
//! consumed it, so the states add up to the judgments the run is scored by ([`PlaySummary`]), the
//! notes a run never reached aside: those stay at state 0 ([`TraceSource`]). A nine-key run does not
//! add up: a pop'n BAD leaves its note to be judged again, so its state is the later judgment's, or
//! 0.
//!
//! [`PlaySummary`]: rbms_play::PlaySummary

use std::collections::VecDeque;

use rbms_config::Config;
use rbms_ir::{
    ChartId, PlayTrace, RandomOption, TRACE_FORMAT, TRACE_FORMAT_VERSION, TRACE_KIND_CHARGE_END, TRACE_KIND_LONG_END, TRACE_KIND_LONG_HEAD, TRACE_KIND_NORMAL,
    TRACE_MAX_SPEED_CHANGES, TRACE_MAX_TEMPO_POINTS, TraceEmptyPoors, TraceGauge, TraceGaugeBounds, TraceNotes, TraceSpeed, TraceSpeedChange, TraceSpeeds,
    TraceTempo,
};
use rbms_judge::gauge::GaugeIndex;
use rbms_judge::{Judge, NoteMark};
use rbms_model::Model;
use rbms_play::{GAUGE_LOG_INTERVAL_MS, PlayRecord, PlaySession};

use crate::AppShared;
use crate::skin_host::play::{covered_duration_ms, duration_green};

/// The scroll rate a travel time is read at for [`TraceSpeed::green_ms`]: the chart's own, before any
/// `#SCROLL` change, as the score's `options.green_number` is.
const CHART_OWN_SCROLL: f64 = 1.0;

/// The share of the lane a travel time over the whole lane leaves covered.
const NO_COVER: f32 = 0.0;

/// A list capped at a count keeps its first entries and its last ones, in this many parts of one
/// size: the start of the run and its end.
const CAPPED_LIST_PARTS: usize = 2;

/// How many of a run's first scroll changes a trace keeps, and how many of its last ones.
const SPEED_CHANGES_FROM_THE_START: usize = TRACE_MAX_SPEED_CHANGES / CAPPED_LIST_PARTS;
const SPEED_CHANGES_TO_THE_END: usize = TRACE_MAX_SPEED_CHANGES - SPEED_CHANGES_FROM_THE_START;

/// How many of a chart's first tempo points a trace keeps, and how many of its last ones.
const TEMPO_POINTS_FROM_THE_START: usize = TRACE_MAX_TEMPO_POINTS / CAPPED_LIST_PARTS;
const TEMPO_POINTS_TO_THE_END: usize = TRACE_MAX_TEMPO_POINTS - TEMPO_POINTS_FROM_THE_START;

/// What a run's scroll settings did while it was played: the first value seen, and every change
/// seen after it, up to [`TRACE_MAX_SPEED_CHANGES`]: the first ones and the latest ones, so a run that
/// moves its settings every frame holds no more than that.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SpeedLog {
    start: Option<TraceSpeed>,
    /// The settings at the last look, and the song time of that look in milliseconds.
    last: Option<(TraceSpeed, i64)>,
    /// The first changes seen, up to [`SPEED_CHANGES_FROM_THE_START`].
    first: Vec<TraceSpeedChange>,
    /// The latest changes seen after those, up to [`SPEED_CHANGES_TO_THE_END`]: the oldest goes as a
    /// newer one comes.
    latest: VecDeque<TraceSpeedChange>,
}

impl SpeedLog {
    /// Take in the settings as they stand at `t_ms` of the song: the start, the first time, and
    /// otherwise a change for each field that moved since the last look.
    pub(crate) fn observe(&mut self, speed: TraceSpeed, t_ms: i64) {
        match self.last {
            None => self.start = Some(speed),
            Some((last, _)) if last != speed => {
                for change in last.changes_to(&speed, t_ms) {
                    self.push(change);
                }
            }
            Some(_) => {}
        }
        self.last = Some((speed, t_ms));
    }

    fn push(&mut self, change: TraceSpeedChange) {
        if self.first.len() < SPEED_CHANGES_FROM_THE_START {
            self.first.push(change);
            return;
        }
        if self.latest.len() == SPEED_CHANGES_TO_THE_END {
            self.latest.pop_front();
        }
        self.latest.push_back(change);
    }

    /// The settings the run started on, ended on (`end`, as they stand when the run is kept, which
    /// is counted as seen at the last look) and the changes between. A run never looked at started
    /// on what it ended on.
    pub(crate) fn finished(mut self, end: TraceSpeed) -> TraceSpeeds {
        self.observe(end, self.last.map_or(0, |(_, t_ms)| t_ms));
        let SpeedLog { start, first, latest, .. } = self;
        TraceSpeeds { start: start.unwrap_or(end), end, changes: first.into_iter().chain(latest).collect() }
    }
}

/// The scroll settings of a run on a chart that opens at `init_bpm`, as they stand right now
/// ([`speed_of`]).
pub(crate) fn speed_now(shared: &AppShared, init_bpm: f64) -> TraceSpeed {
    speed_of(&shared.config, shared.effective_cover(), shared.effective_lift(), shared.effective_hidden(), init_bpm)
}

/// The scroll settings `config` holds for a chart that opens at `init_bpm`, with the shades as they
/// are shown (`lane_cover`, `lift` and `hidden`, nought while one is off): the travel time the score
/// names as its GREEN number, and the GREEN number a skin shows at that tempo, worked out as the skin
/// works it out (`NUMBER_DURATION_GREEN`: the whole lane's travel time in single-precision hi-speed,
/// the cover taken off and rounded, three fifths of that cut to a whole number).
pub(crate) fn speed_of(config: &Config, lane_cover: f32, lift: f32, hidden: f32, init_bpm: f64) -> TraceSpeed {
    let play = &config.play;
    let travel_ms = crate::green_number_for(play.constant_speed, init_bpm, play.hispeed, CHART_OWN_SCROLL, lane_cover);
    let shown_hispeed = f64::from(play.hispeed as f32);
    let lane_travel_ms = crate::green_number_for(play.constant_speed, init_bpm, shown_hispeed, CHART_OWN_SCROLL, NO_COVER);
    TraceSpeed {
        hispeed: play.hispeed,
        constant: play.constant_speed,
        lane_cover,
        lift,
        hidden,
        offset_ms: config.judge.offset_ms,
        auto_offset: config.judge.auto_offset,
        green_ms: travel_ms.round() as i32,
        green: duration_green(covered_duration_ms(lane_travel_ms, lane_cover)),
    }
}

/// What a trace says of the run besides what the session holds: how it is named on the wire, and
/// what its scroll settings did and stand at as it is kept.
pub(crate) struct TraceRun<'a> {
    pub(crate) mode: &'a str,
    pub(crate) lntype: i32,
    pub(crate) random: RandomOption,
    pub(crate) speed: &'a SpeedLog,
    pub(crate) speed_end: TraceSpeed,
}

/// What the trace of a run is made of, taken off the run as it is kept: the least the frame thread
/// copies out of the session, so that laying the notes out, finishing the scroll log and writing the
/// body happen on the save thread ([`TraceSource::body`]).
///
/// Every judged object of the chart is in it, those the run never reached included. A run the
/// gauge emptied ends where it failed, so every object after that moment is one nothing consumed:
/// its state is 0 and its timing 0, as on a run left part-way, and it is not counted a POOR here.
/// The score counts those notes as POORs let pass; a server that checks a trace against its score
/// compares the objects the run judged and leaves the ones at state 0 out.
#[derive(Debug)]
pub(crate) struct TraceSource {
    chart: ChartId,
    mode: String,
    lntype: i32,
    random: RandomOption,
    auto_lanes: Vec<bool>,
    marks: Vec<NoteMark>,
    empty_poors: TraceEmptyPoors,
    gauge: TraceGauge,
    gauge_bounds: Vec<TraceGaugeBounds>,
    failed_at_ms: Option<i64>,
    play_time_ms: i64,
    speed: SpeedLog,
    speed_end: TraceSpeed,
    tempo: Vec<TraceTempo>,
}

impl TraceSource {
    /// What the trace of the run `session` played, whose record is `record`, is made of.
    pub(crate) fn of(session: &PlaySession, record: &PlayRecord, run: TraceRun<'_>) -> TraceSource {
        let model = session.model();
        let judge = session.judge();
        let empty_poors = judge.judged().iter().filter(|judged| judged.judge == Judge::Miss);
        let (time_us, lane) = empty_poors.map(|judged| (judged.note_us - judged.delta_us, judged.lane as u32)).unzip();
        TraceSource {
            chart: ChartId { md5: model.md5.clone(), sha256: model.sha256.clone() },
            mode: run.mode.to_string(),
            lntype: run.lntype,
            random: run.random,
            auto_lanes: if session.has_auto_played_notes() { session.auto_lanes().to_vec() } else { Vec::new() },
            marks: judge.note_marks().collect(),
            empty_poors: TraceEmptyPoors { time_us, lane },
            gauge: TraceGauge {
                interval_ms: GAUGE_LOG_INTERVAL_MS,
                selected: record.finished_gauge.index() as u32,
                series: GaugeIndex::ALL.iter().map(|index| record.gauge_log.of(*index).to_vec()).collect(),
            },
            gauge_bounds: record.gauge_bounds.iter().map(|bounds| TraceGaugeBounds { min: bounds.min, max: bounds.max, border: bounds.border }).collect(),
            failed_at_ms: record.failed_at_ms,
            play_time_ms: record.play_time_ms,
            speed: run.speed.clone(),
            speed_end: run.speed_end,
            tempo: tempo_of(model),
        }
    }

    /// The trace, with no score linked to it yet.
    pub(crate) fn trace(self) -> PlayTrace {
        PlayTrace {
            format: TRACE_FORMAT.to_string(),
            version: TRACE_FORMAT_VERSION,
            score_id: None,
            chart: self.chart,
            mode: self.mode,
            lntype: self.lntype,
            random: self.random,
            auto_lanes: self.auto_lanes,
            notes: notes_of(&self.marks),
            empty_poors: self.empty_poors,
            gauge: self.gauge,
            gauge_bounds: self.gauge_bounds,
            failed_at_ms: self.failed_at_ms,
            play_time_ms: self.play_time_ms,
            speed: self.speed.finished(self.speed_end),
            tempo: self.tempo,
        }
    }

    /// The trace as the JSON it is kept and sent as, or `None` when it does not serialise.
    pub(crate) fn body(self) -> Option<String> {
        self.trace().to_json()
    }
}

/// The columns of every judged object, ordered by chart time and, within one time, by lane.
///
/// The engine gives a long note's marks as its head's and then its end's, one after the other in
/// its lane, so a head is told from a normal note by the end that follows it.
fn notes_of(marks: &[NoteMark]) -> TraceNotes {
    let mut rows: Vec<(i64, u32, u8, u8, i64)> = marks
        .iter()
        .enumerate()
        .map(|(at, mark)| {
            let kind = match (mark.long_end, mark.charge) {
                (true, true) => TRACE_KIND_CHARGE_END,
                (true, false) => TRACE_KIND_LONG_END,
                (false, _) if marks.get(at + 1).is_some_and(|next| next.long_end && next.lane == mark.lane) => TRACE_KIND_LONG_HEAD,
                (false, _) => TRACE_KIND_NORMAL,
            };
            (mark.time_us, mark.lane as u32, kind, mark.state, mark.judged_us)
        })
        .collect();
    rows.sort_by_key(|(time_us, lane, ..)| (*time_us, *lane));
    let mut notes = TraceNotes::default();
    for (time_us, lane, kind, state, err_us) in rows {
        notes.time_us.push(time_us);
        notes.lane.push(lane);
        notes.kind.push(kind);
        notes.state.push(state);
        notes.err_us.push(err_us);
    }
    notes
}

/// Where the chart's tempo, scroll rate or a stop changes: its opening tempo, then every line that
/// changes one of them or stops, up to [`TRACE_MAX_TEMPO_POINTS`]: the first and the last ones.
fn tempo_of(model: &Model) -> Vec<TraceTempo> {
    let mut points = vec![TraceTempo { t_us: 0, bpm: model.init_bpm, stop_us: 0, scroll: CHART_OWN_SCROLL }];
    for line in &model.timelines {
        let moved = points.last().is_some_and(|last| last.bpm != line.bpm || last.scroll != line.scroll);
        if moved || line.stop_us > 0 {
            points.push(TraceTempo { t_us: line.time_us, bpm: line.bpm, stop_us: line.stop_us, scroll: line.scroll });
        }
    }
    if points.len() > TRACE_MAX_TEMPO_POINTS {
        points.drain(TEMPO_POINTS_FROM_THE_START..points.len() - TEMPO_POINTS_TO_THE_END);
    }
    points
}

#[cfg(test)]
mod tests;
