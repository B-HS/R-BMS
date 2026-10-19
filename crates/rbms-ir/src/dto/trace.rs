use serde::{Deserialize, Serialize};

use crate::dto::{ChartId, RandomOption};

/// The `format` tag every play trace carries.
pub const TRACE_FORMAT: &str = "rbms-trace";

/// The generation of the play trace format this build writes.
pub const TRACE_FORMAT_VERSION: u32 = 1;

/// The largest play trace body a client sends, in bytes: about twice what fifty thousand notes and
/// all nine gauges of a long chart come to. A trace past it is kept on the machine and not sent; the
/// server refuses a body past the same size.
pub const TRACE_MAX_BYTES: usize = 2 * 1024 * 1024;

/// The most scroll setting changes a trace carries ([`TraceSpeeds::changes`]), about a quarter of a
/// MiB of the body. A run that moved its settings more often keeps its first and its last changes up
/// to this count, and its trace is sent all the same.
pub const TRACE_MAX_SPEED_CHANGES: usize = 4_096;

/// The most tempo points a trace carries ([`PlayTrace::tempo`]), about a quarter of a MiB of the
/// body. A chart that changes its tempo, its scroll rate or stops more often keeps its first and its
/// last points up to this count, and its trace is sent all the same.
pub const TRACE_MAX_TEMPO_POINTS: usize = 4_096;

/// `notes.kind` of a note judged once: a normal note.
pub const TRACE_KIND_NORMAL: u8 = 0;
/// `notes.kind` of the head of a long note of any flavour.
pub const TRACE_KIND_LONG_HEAD: u8 = 1;
/// `notes.kind` of the end of a plain long note, which is judged on its head and keeps state 0.
pub const TRACE_KIND_LONG_END: u8 = 2;
/// `notes.kind` of the end of a charge or hell-charge note, which is judged on its own.
pub const TRACE_KIND_CHARGE_END: u8 = 3;

/// What one run was, note by note, as the client judged it: the body of
/// `POST /charts/{hash}/traces`, sent once the server took the run's score and linked to it by
/// `score_id`.
///
/// It holds itself: the notes as they were laid out for the run (after the lane option and with the
/// lanes SCRATCH AUTO played left out), the state and timing each was judged with, every gauge's
/// history, the scroll settings the run started and ended on and every change between, and the
/// chart's tempo changes. The server checks it against the score and judges nothing again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayTrace {
    pub format: String,
    pub version: u32,
    /// The score the server stored the run as; `None` until the run's score was taken.
    pub score_id: Option<String>,
    pub chart: ChartId,
    pub mode: String,
    /// The long-note flavour the run was judged under, as the score's `options.lntype`.
    pub lntype: i32,
    /// The lane option, as the score's `options.random`.
    pub random: RandomOption,
    /// The lanes SCRATCH AUTO played, in lane order; empty when it played none.
    pub auto_lanes: Vec<bool>,
    pub notes: TraceNotes,
    pub empty_poors: TraceEmptyPoors,
    pub gauge: TraceGauge,
    /// The bounds of each of the nine gauges, in gauge index order.
    pub gauge_bounds: Vec<TraceGaugeBounds>,
    /// Song time at which an empty gauge ended the run, in milliseconds.
    pub failed_at_ms: Option<i64>,
    /// How long the chart stays in its playing state, in milliseconds.
    pub play_time_ms: i64,
    pub speed: TraceSpeeds,
    /// The chart's opening tempo, then every line that changes its tempo or scroll rate or stops,
    /// at most [`TRACE_MAX_TEMPO_POINTS`] of them: the first and the last, the ones between left out.
    pub tempo: Vec<TraceTempo>,
}

impl PlayTrace {
    /// The trace as the JSON it is kept and sent as, or `None` when it does not serialise.
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }

    /// A trace read back from [`PlayTrace::to_json`], or `None` when the text is not one.
    pub fn from_json(text: &str) -> Option<PlayTrace> {
        serde_json::from_str(text).ok()
    }
}

/// Every judged object of the chart as the run was laid out, in parallel columns, ordered by chart
/// time and, within one time, by lane.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceNotes {
    /// Chart time of the object, in microseconds.
    pub time_us: Vec<i64>,
    /// The lane the object was played on, after the lane option.
    pub lane: Vec<u32>,
    /// [`TRACE_KIND_NORMAL`], [`TRACE_KIND_LONG_HEAD`], [`TRACE_KIND_LONG_END`] or
    /// [`TRACE_KIND_CHARGE_END`].
    pub kind: Vec<u8>,
    /// 0 for an object nothing consumed (one the run never reached, as every object after the
    /// moment an empty gauge ended the run, or a plain long note's end), otherwise the judgment that
    /// consumed it plus one: 1 PGREAT to 5 POOR.
    pub state: Vec<u8>,
    /// The signed timing of the judgment that consumed the object, in microseconds, positive when
    /// the input came early; 0 for state 0.
    pub err_us: Vec<i64>,
}

/// Every empty POOR of the run, in the order they were judged.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEmptyPoors {
    /// Chart time of the press, in microseconds.
    pub time_us: Vec<i64>,
    pub lane: Vec<u32>,
}

/// The history of every gauge through the run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceGauge {
    /// Song time between two samples, in milliseconds.
    pub interval_ms: i64,
    /// The index of the gauge the run finished on.
    pub selected: u32,
    /// One history per gauge, in gauge index order, oldest sample first. A failed run is padded with
    /// zeroes from its failure to the chart's end.
    pub series: Vec<Vec<f32>>,
}

/// The values one gauge moves between.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceGaugeBounds {
    pub min: f32,
    pub max: f32,
    /// The value it has to hold at the end to clear.
    pub border: f32,
}

/// The scroll settings a run started on and ended on, and every change between.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceSpeeds {
    pub start: TraceSpeed,
    pub end: TraceSpeed,
    /// The changes in the order they were seen, at most [`TRACE_MAX_SPEED_CHANGES`] of them: the
    /// first and the last, the ones between left out, so the changes need not lead from `start` to
    /// `end` when there were more.
    pub changes: Vec<TraceSpeedChange>,
}

/// The scroll settings at one moment of a run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceSpeed {
    pub hispeed: f64,
    pub constant: bool,
    /// The share of the lane the cover hides; 0 while the cover is off.
    pub lane_cover: f32,
    /// The share of the lane the lift raises the judge line by; 0 while the lift is off.
    pub lift: f32,
    /// The share of the lane the hidden band hides; 0 while it is off.
    pub hidden: f32,
    pub offset_ms: i32,
    pub auto_offset: bool,
    /// How long a note takes to cross the lane left open under the cover at the chart's first tempo,
    /// in whole milliseconds: the score's `options.green_number` with the cover as it is shown.
    pub green_ms: i32,
    /// The GREEN number a skin shows for that travel time.
    pub green: i32,
}

/// The names [`TraceSpeedChange::field`] takes, one for each field of [`TraceSpeed`].
pub const TRACE_SPEED_FIELDS: [&str; 9] = ["hispeed", "constant", "lane_cover", "lift", "hidden", "offset_ms", "auto_offset", "green_ms", "green"];

impl TraceSpeed {
    /// Every field as a number, in [`TRACE_SPEED_FIELDS`] order: a switch is 1 when on.
    pub fn values(&self) -> [f64; TRACE_SPEED_FIELDS.len()] {
        [
            self.hispeed,
            f64::from(u8::from(self.constant)),
            f64::from(self.lane_cover),
            f64::from(self.lift),
            f64::from(self.hidden),
            f64::from(self.offset_ms),
            f64::from(u8::from(self.auto_offset)),
            f64::from(self.green_ms),
            f64::from(self.green),
        ]
    }

    /// The changes that turn `self` into `next` at `t_ms`, one for each field that differs, in
    /// [`TRACE_SPEED_FIELDS`] order.
    pub fn changes_to(&self, next: &TraceSpeed, t_ms: i64) -> Vec<TraceSpeedChange> {
        TRACE_SPEED_FIELDS
            .iter()
            .zip(self.values().into_iter().zip(next.values()))
            .filter(|(_, (was, now))| was != now)
            .map(|(field, (_, value))| TraceSpeedChange { t_ms, field: (*field).to_string(), value })
            .collect()
    }
}

/// One scroll setting that moved during a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceSpeedChange {
    /// Song time it was seen to have moved at, in milliseconds.
    pub t_ms: i64,
    /// One of [`TRACE_SPEED_FIELDS`].
    pub field: String,
    /// What it moved to; a switch is 1 when on.
    pub value: f64,
}

/// One place where the chart's tempo, its scroll rate or a stop changes, as it was judged.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TraceTempo {
    /// Chart time, in microseconds.
    pub t_us: i64,
    pub bpm: f64,
    /// How long the chart stops here, in microseconds.
    pub stop_us: i64,
    pub scroll: f64,
}

/// What `POST /charts/{hash}/traces` answers with: the id the trace is stored under, which is the
/// one already stored when the score had a trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceUploadResponse {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use crate::dto::*;
    use serde_json::json;

    fn trace() -> PlayTrace {
        PlayTrace {
            format: TRACE_FORMAT.into(),
            version: TRACE_FORMAT_VERSION,
            score_id: None,
            chart: ChartId { md5: "m".into(), sha256: "s".into() },
            mode: "BEAT_7K".into(),
            lntype: 0,
            random: RandomOption::Off,
            auto_lanes: Vec::new(),
            notes: TraceNotes { time_us: vec![1_000], lane: vec![1], kind: vec![TRACE_KIND_NORMAL], state: vec![1], err_us: vec![-3] },
            empty_poors: TraceEmptyPoors::default(),
            gauge: TraceGauge { interval_ms: 500, selected: 2, series: vec![vec![20.0, 22.5]] },
            gauge_bounds: vec![TraceGaugeBounds { min: 2.0, max: 100.0, border: 80.0 }],
            failed_at_ms: None,
            play_time_ms: 6_000,
            speed: TraceSpeeds::default(),
            tempo: vec![TraceTempo { t_us: 0, bpm: 150.0, stop_us: 0, scroll: 1.0 }],
        }
    }

    #[test]
    fn a_trace_round_trips_with_its_columns_and_its_link() {
        let mut sent = trace();
        sent.score_id = Some("sc_1".into());
        let text = serde_json::to_string(&sent).unwrap();
        assert!(text.contains(r#""format":"rbms-trace""#), "got {text}");
        assert!(text.contains(r#""score_id":"sc_1""#));
        assert!(text.contains(r#""random":"Off""#), "the lane option is written as a score's is");
        assert_eq!(PlayTrace::from_json(&text), Some(sent.clone()));
        assert_eq!(sent.to_json(), Some(text));
        assert_eq!(PlayTrace::from_json("{}"), None, "a body that is no trace reads as none");
    }

    #[test]
    fn a_speed_change_names_every_field_that_moved_and_nothing_else() {
        let was = TraceSpeed { hispeed: 2.0, lane_cover: 0.2, green_ms: 600, green: 360, ..TraceSpeed::default() };
        let now = TraceSpeed { hispeed: 2.5, auto_offset: true, ..was };
        let changes = was.changes_to(&now, 1_234);
        let named: Vec<(&str, f64)> = changes.iter().map(|change| (change.field.as_str(), change.value)).collect();
        assert_eq!(named, [("hispeed", 2.5), ("auto_offset", 1.0)]);
        assert!(changes.iter().all(|change| change.t_ms == 1_234));
        assert!(was.changes_to(&was, 0).is_empty(), "nothing moved, nothing to say");
    }

    #[test]
    fn the_upload_answer_needs_only_its_id() {
        let answer: TraceUploadResponse = serde_json::from_value(json!({"id": "tr_1", "extra": 1})).unwrap();
        assert_eq!(answer.id, "tr_1");
    }
}
