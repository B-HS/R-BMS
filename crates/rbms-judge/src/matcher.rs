use rbms_model::{LnKind, Mode, Model, NoteKind};
use serde::{Deserialize, Serialize};

use crate::Judge;
use crate::algorithm::{JudgeAlgorithm, NoteRef, NoteType, UNJUDGED_STATE};
use crate::gauge::{ClearType, GaugeAutoShift, GaugeKind, GaugeShiftOutcome, GrooveGauge, clear_lamp};
use crate::gauge_tables::GaugeSetId;
use crate::ln::{HCN_GAUGE_TICK_RATE, HCN_GAUGE_TICK_US, LaneHold, LnMode};
use crate::windows::{
    EMPTY_POOR_JUDGE_INDEX, JudgeProperty, JudgeWindowRule, JudgeWindowSet, JudgeWindows, MissCondition, UNMODIFIED_JUDGE_WIDTH_RATES, candidate_gate_across,
};

/// Per-lane note seeds as `(head_us, end_us, long-note kind)`, the shape the constructors build
/// lanes from.
type LaneSeeds = Vec<Vec<(i64, Option<i64>, Option<LnKind>)>>;

/// Any non-zero `Note.getState()`. The reference only ever tests state against zero
/// (`JudgeManager.java:396, 398, 401`), so one non-zero value stands for every judged note.
const JUDGED_STATE: u8 = UNJUDGED_STATE + 1;

/// The judgment each reference judge code names (`JudgeManager.java:400-404`): 0..3 are the timing
/// bands, 4 is 見逃し POOR and 5 is 空POOR. A code past the end of this table names no judgment and
/// cancels the press (`JudgeManager.java:412`).
const JUDGE_BY_CODE: [Judge; 6] = [Judge::PerfectGreat, Judge::Great, Judge::Good, Judge::Bad, Judge::Poor, Judge::Miss];

/// Reference judge code for a press that reached only the 空POOR band (`JudgeManager.java:401, 404`).
const EMPTY_POOR_CODE: usize = Judge::Miss as usize;

/// Reference judge code for a candidate too far out to judge at all (`JudgeManager.java:401, 412`).
const IGNORED_CODE: usize = JUDGE_BY_CODE.len();

/// Judge code at or above which a long-note release that came early is deferred instead of
/// confirmed (`JudgeManager.java:509, 540`).
const DEFERRED_RELEASE_CODE: usize = Judge::Bad as usize;

/// Highest judge code that still counts as "the end was taken cleanly", the reference's
/// `getState() > 0 && getState() <= 3` test on a hell-charge end (`JudgeManager.java:290`).
const HCN_HELD_END_CODE: usize = Judge::Good as usize;

/// Divisor the reference's `Note.getPlayTime()` applies to the microsecond value
/// `setMicroPlayTime` stored, so a hit inside one millisecond reads back as "never played"
/// (`JudgeManager.java:398, 648, 651`).
const MICROS_PER_MILLI: i64 = 1_000;

/// Playback rate, in percent, of a chart played at its own speed.
pub const UNMODIFIED_PLAYBACK_PERCENT: i32 = 100;

/// Slowest playback rate the engine takes, in percent. A rate of nothing would stand the chart still.
const MIN_PLAYBACK_PERCENT: i32 = 1;

/// How much faster or slower than its own speed a chart is played, as the engine converts between
/// the chart's clock it is driven on and the player's own time it judges in.
///
/// The reference plays a practice slice at another FREQUENCY by retiming the chart itself
/// (`BMSModelUtils.changeFrequency`, `BMSPlayer.java:554-558`) and judges the retimed chart with
/// windows of the same length (`JudgeManager` knows no frequency), so every window, margin and tick
/// is a length of the player's time. Driven on the chart's clock, the engine stretches each of them
/// onto that clock and reads each timing back off it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaybackRate {
    percent: i64,
}

impl PlaybackRate {
    const UNMODIFIED: PlaybackRate = PlaybackRate { percent: UNMODIFIED_PLAYBACK_PERCENT as i64 };

    fn new(percent: i32) -> PlaybackRate {
        PlaybackRate { percent: i64::from(percent.max(MIN_PLAYBACK_PERCENT)) }
    }

    /// How much of the chart's clock goes by in `real_us` of the player's time.
    fn chart_span_us(self, real_us: i64) -> i64 {
        scale_span_us(real_us, self.percent, i64::from(UNMODIFIED_PLAYBACK_PERCENT))
    }

    /// How much of the player's time `chart_us` of the chart's clock takes.
    fn real_span_us(self, chart_us: i64) -> i64 {
        scale_span_us(chart_us, i64::from(UNMODIFIED_PLAYBACK_PERCENT), self.percent)
    }

    /// `windows`, which are lengths of the player's time, as lengths of the chart's clock.
    fn chart_windows(self, windows: JudgeWindows) -> JudgeWindows {
        if self == PlaybackRate::UNMODIFIED {
            return windows;
        }
        let pair = |(late, early): (i64, i64)| (self.chart_span_us(late), self.chart_span_us(early));
        JudgeWindows { pg: pair(windows.pg), gr: pair(windows.gr), gd: pair(windows.gd), bd: pair(windows.bd), ms: windows.ms.map(pair) }
    }
}

fn scale_span_us(span_us: i64, numerator: i64, denominator: i64) -> i64 {
    let scaled = i128::from(span_us) * i128::from(numerator) / i128::from(denominator);
    scaled.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

fn judge_of_code(code: usize) -> Option<Judge> {
    JUDGE_BY_CODE.get(code).copied()
}

struct JNote {
    head_us: i64,
    end_us: Option<i64>,
    /// The flavour the chart stated for this long note, kept untouched so that LN MODE resolves
    /// from the chart every time it is set rather than from an already-resolved value.
    chart_ln: Option<LnKind>,
    ln: Option<LnKind>,
    judged: bool,
    holding: bool,
    /// `Note.playtime` of the head object: the signed delta of the last judgment it took.
    head_play_time_us: i64,
    /// `Note.playtime` of the paired end object, which only a charge note ever judges separately.
    end_play_time_us: i64,
    /// Judgment the paired end object took, the reference's `getPair().getState() - 1`. Read by the
    /// hell-charge tick to decide whether a released note still gains gauge.
    end_judge: Option<Judge>,
    /// `Note.getState()` of the head object: [`UNJUDGED_STATE`] until a judgment consumes it, then
    /// that judgment's code plus one (`JudgeManager.java:645`).
    head_state: u8,
    /// `Note.getState()` of the paired end object.
    end_state: u8,
}

/// One object of the chart as the reference's `Note` reads back once it has been played: what the
/// result screen's timing distribution and per-second judge graph walk the chart for
/// (`AbstractResult`/`MusicResult.java:352-363`, `SkinNoteDistributionGraph.java:300-360`).
///
/// A long note yields two marks, its head and its end, because the reference keeps them as two
/// objects with a state and a play time each. Mines are not judged objects and yield none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteMark {
    pub lane: usize,
    /// Chart time of this object: the head's for a head or a normal note, the end's for an end.
    pub time_us: i64,
    /// Whether this is the end object of a long note.
    pub long_end: bool,
    /// Whether the note this object belongs to is judged at both ends, a charge or hell-charge
    /// note. A plain long note is judged once, on its head object, and its end keeps state 0.
    pub charge: bool,
    /// `Note.getState()`: 0 while nothing has consumed the object, otherwise the judge code of the
    /// judgment that did plus one, so 1 is PGREAT and 5 is a 見逃し POOR.
    pub state: u8,
    /// `Note.getMicroPlayTime()`: the signed timing of the last judgment the object took, positive
    /// when the input came early. An empty POOR overwrites it without consuming the object.
    pub play_time_us: i64,
}

/// One note of the chart, by its lane and its place among that lane's notes in chart order: the
/// note whose head is the `index`th head [`JudgeEngine::note_marks`] gives for `lane`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteAt {
    pub lane: usize,
    pub index: usize,
}

/// One judgment as the engine counted it, in the order it was counted: what the reference hands its
/// screen from `JudgeManager.updateMicro` each time it gets as far as counting one
/// (`JudgeManager.java:652-692`).
///
/// A screen that shows a judgment where it landed needs the lane, and one that shows the combo beside
/// it needs the combo as it stood right then: two judgments counted on the same frame leave two
/// different combos behind, and the totals only ever show the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JudgedNote {
    pub lane: usize,
    pub judge: Judge,
    /// The signed timing the judgment was counted with, positive when the input came early
    /// (`mfast`).
    pub delta_us: i64,
    /// The combo once this judgment was counted, carried across the stages of a course
    /// (`JudgeManager.getCourseCombo`).
    pub combo: u32,
}

/// The long notes one lane has in hand, each named by the chart time of its head.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaneLongNote {
    /// The long note the lane is holding, from the press that took its head until the note is
    /// resolved (`LaneState.processing`).
    pub processing: Option<i64>,
    /// The hell-charge note the play head is inside, hit or not (`LaneState.passing`).
    pub passing: Option<i64>,
}

/// CN/HCN ("charge"/"hell-charge") long notes are judged twice — the head at press and the release
/// end at key-up — each a counted judgment (the reference implementation's `JudgeManager` calls `updateMicro` at both),
/// whereas a plain LN is a single judgment (the worse of head/end). This predicate gates the
/// charge-note behaviour so the verified LN/Normal paths are byte-identical to before. See
/// `docs/reference/cn-hcn-judgment.md`.
fn is_charge(ln: Option<LnKind>) -> bool {
    matches!(ln, Some(LnKind::Cn) | Some(LnKind::Hcn))
}

/// Whether this note pays out the continuous gauge ticks of a hell-charge note
/// (`JudgeManager.java:235-242`).
fn is_hell_charge(ln: Option<LnKind>) -> bool {
    matches!(ln, Some(LnKind::Hcn))
}

/// How the candidate-selection predicate sees a note.
fn note_ref(note: &JNote) -> NoteRef {
    NoteRef { time_us: note.head_us, state: if note.judged || note.holding { JUDGED_STATE } else { UNJUDGED_STATE }, is_long: note.end_us.is_some() }
}

struct Lane {
    notes: Vec<JNote>,
    cursor: usize,
    /// Scan position of the hell-charge pass detector, kept apart from `cursor` because a passing
    /// note is not yet a resolved one.
    hcn_cursor: usize,
}

/// A mine note: passing it with the lane key held costs `damage` gauge points
/// (reference implementation `JudgeManager.java:244-247`).
struct Mine {
    time_us: i64,
    damage: f64,
}

/// How the selected gauge may move as a run is judged: the auto-shift mode, the gauge the player
/// chose and the floor the selection may fall to ([`GrooveGauge::auto_shift`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GaugeShift {
    pub mode: GaugeAutoShift,
    pub configured: GaugeKind,
    pub bottom: GaugeKind,
}

/// One judgment the clock resolved. Collected while the lane is borrowed and applied once the
/// borrow ends.
struct SweepEvent {
    lane: usize,
    note: usize,
    judge: Judge,
    delta_us: i64,
    on_end: bool,
}

/// Physical direction of a scratch input. The reference remembers which of a scratch lane's two
/// keys grabbed a long note (`JudgeManager.java:435, 449` set `sckey[sc]`) so that the other one
/// ends it — a back-spin scratch. Key lanes have no second direction and always use
/// [`ScratchDir::Forward`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScratchDir {
    #[default]
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy)]
pub struct JudgeResult {
    pub judge: Judge,
    pub lane: usize,
    pub note_index: usize,
    pub fast: bool,
    pub delta_us: i64,
    /// Whether the engine counted this as a judgment. A plain long note is judged once, at its end,
    /// so the press that only takes its head into the hold reports how that head was timed without
    /// counting anything (`JudgeManager.java:426-438`): the tallies, the sound and the offset
    /// adjustment of a judgment belong to the end alone.
    pub counted: bool,
}

fn worse(a: Judge, b: Judge) -> Judge {
    if a as usize >= b as usize { a } else { b }
}

/// The marks of `note`, in `lane`: its head's, and its end's when it is a long note.
fn marks_of(lane: usize, note: &JNote) -> impl Iterator<Item = NoteMark> {
    let charge = is_charge(note.ln);
    let head = NoteMark { lane, time_us: note.head_us, long_end: false, charge, state: note.head_state, play_time_us: note.head_play_time_us };
    let end = note.end_us.map(|end_us| NoteMark { lane, time_us: end_us, long_end: true, charge, state: note.end_state, play_time_us: note.end_play_time_us });
    std::iter::once(head).chain(end)
}

/// Judged objects in `lanes`: a charge note's head and end each count, every other note counts once.
fn count_notes(lanes: &[Lane]) -> u32 {
    lanes.iter().flat_map(|l| l.notes.iter()).map(|n| if is_charge(n.ln) && n.end_us.is_some() { 2 } else { 1 }).sum()
}

/// Note objects in `lanes`, whatever they are judged as: a head, and an end for a long note. A run
/// resolves each of them at most once.
fn count_objects(lanes: &[Lane]) -> usize {
    lanes.iter().flat_map(|lane| lane.notes.iter()).map(|note| 1 + usize::from(note.end_us.is_some())).sum()
}

/// Judgments the engine makes room for, per note object, when it is built: the one that resolves the
/// object, and one 空POOR. A 空POOR leaves its note where it was (`judgeVanish[5]` is false in every
/// mode), so a run may take any number of them; one for each object is more than a run played by
/// hand makes.
const JUDGMENT_ROOM_PER_OBJECT: usize = 2;

/// Room for the judgments of a run over `lanes` ([`JUDGMENT_ROOM_PER_OBJECT`]).
fn judgment_room(lanes: &[Lane]) -> usize {
    JUDGMENT_ROOM_PER_OBJECT * count_objects(lanes)
}

/// Stateful note-matching engine with LN support: per-lane time-sorted notes, a moving
/// cursor, combo, EX score, per-judge counts and the nine groove gauges. `press` folds the lane's
/// candidates through the selected [`JudgeAlgorithm`] (LN heads start a hold); `release` finalises
/// or defers a held long note against the LN-end window; `update` confirms deferred releases, pays
/// out hell-charge ticks, sweeps unhittable notes into 見逃し POOR and applies mine damage.
///
/// Time moves on in the order things happen rather than once per display frame. `update(t)` settles
/// every time-driven judgment due at or before `t` at the very instant it falls due, one instant
/// after another, and every input first settles the clock up to its own instant: a release and a
/// re-grab, a mine and the key over it, and a hell-charge tick come out the same however the calls
/// that brought the clock there were spaced. The reference advances all of these once per frame
/// (`JudgeManager.update`), which makes a run depend on its frame rate; see
/// `docs/acknowledge/reference-divergences.md`.
///
/// Judge slots follow the reference implementation exactly: index 4 ([`Judge::Poor`]) is 見逃し POOR — a note that
/// went by unhit — and index 5 ([`Judge::Miss`]) is 空POOR, a press that reached only the MS band
/// and consumed nothing.
pub struct JudgeEngine {
    lanes: Vec<Lane>,
    holds: Vec<LaneHold>,
    mines: Vec<Vec<Mine>>,
    mine_cursor: Vec<usize>,
    held: Vec<bool>,
    scratch_lane: Vec<bool>,
    prop: JudgeProperty,
    windows: JudgeWindows,
    scratch_windows: JudgeWindows,
    /// LN/CN release window for key lanes. Derived from the chart's mode + `#RANK` (and scaled by
    /// JUDGE WIDTH) alongside `windows`, so long-note releases respect rank/width just like note
    /// heads. Defaults to `SEVENKEY_LN_END` for the mode-less constructors.
    ln_end: JudgeWindows,
    ln_scratch_end: JudgeWindows,
    gate: (i64, i64),
    longnote_margin_rate: i32,
    algorithm: JudgeAlgorithm,
    ln_mode: LnMode,
    gauge_set: GaugeSetId,
    gauge_total: f64,
    /// How fast the chart is played against its own speed ([`JudgeEngine::set_playback_rate`]).
    playback: PlaybackRate,
    /// The instant the clock has been settled up to: every time-driven judgment due at or before it
    /// has been made, and a hell-charge tick balance has run up to it. The reference's
    /// `prevmtime` (`JudgeManager.java:100`), kept per event rather than per frame.
    settled_us: i64,
    /// How the selected gauge may move, or `None` for an engine nothing shifts and nothing fails.
    gauge_shift: Option<GaugeShift>,
    /// The instant the selected gauge emptied under a shift mode that does not rescue the run. From
    /// then on the run is over and the engine judges nothing more (`BMSPlayer.java:653-661, 694`).
    failed_at_us: Option<i64>,
    /// The reference's `autoplay` (`JudgeManager.java:289-293`): the run plays itself, so a
    /// hell-charge note in hand always gains gauge although no key is down.
    autoplay: bool,
    /// The combo as the player sees it, carried across the stages of a course
    /// (`JudgeManager.coursecombo`): a course stage starts it at the streak the stage before ended on.
    pub combo: u32,
    /// The longest [`combo`](Self::combo), carried streak included (`JudgeManager.coursemaxcombo`).
    pub max_combo: u32,
    /// The combo within this chart alone (`JudgeManager.combo`), which starts at nothing on every
    /// course stage. A chart's own record and its full-combo lamp are read from it
    /// (`BMSPlayer.java:868`); outside a course it is [`combo`](Self::combo).
    pub stage_combo: u32,
    /// The longest [`stage_combo`](Self::stage_combo) (`ScoreData.combo`), the max combo a chart's
    /// record keeps.
    pub stage_max_combo: u32,
    pub counts: [u32; 6],
    pub ex_score: u32,
    pub gauge: GrooveGauge,
    pub last_judge: Option<Judge>,
    pub last_fast: bool,
    pub fast: u32,
    pub slow: u32,
    /// Per-judge early/late split (index = judge: 0=PG..4=PR, 5=MS), so `early[i] + late[i] ==
    /// counts[i]`. EARLY = pressed at or before the note (`dm >= 0`, reference implementation
    /// `JudgeManager.java:652` passes `mfast >= 0`); LATE = after.
    pub early: [u32; 6],
    pub late: [u32; 6],
    /// Empty-poor (空POOR) tally, kept as a mirror of `counts[5]` for callers that read it by name.
    pub empty_poor: u32,
    /// Running sum and count of signed hit deltas (µs) for actual hits (press/release, not sweeps),
    /// so `avg_judge_us()` yields the run's mean timing offset.
    sum_delta_us: i64,
    timing_count: u32,
    total_notes: u32,
    /// The reference implementation's `IRScoreData.passnotes`: judgments that consumed their note.
    pass_notes: u32,
    /// Every judgment counted so far, oldest first. Nothing in the engine reads it back.
    judged: Vec<JudgedNote>,
    /// Every note whose marks may have changed since the engine was built, oldest first, once for
    /// each time. Nothing in the engine reads it back.
    ///
    /// Both lists are made room for a judgment of every note object and a 空POOR for each when the
    /// engine is built ([`JUDGMENT_ROOM_PER_OBJECT`]), and [`JudgeEngine::set_ln_mode`] makes room
    /// again past the notes it names here, so a run inside that never grows them, and moves them, in
    /// the middle of play.
    touched: Vec<NoteAt>,
}

impl JudgeEngine {
    pub fn new(per_lane_times: Vec<Vec<i64>>, windows: JudgeWindows) -> Self {
        let pairs = per_lane_times.into_iter().map(|ts| ts.into_iter().map(|t| (t, None)).collect()).collect();
        Self::from_pairs(pairs, windows)
    }

    pub fn from_pairs(per_lane: Vec<Vec<(i64, Option<i64>)>>, windows: JudgeWindows) -> Self {
        let triples: LaneSeeds = per_lane.into_iter().map(|lane| lane.into_iter().map(|(h, e)| (h, e, e.map(|_| LnKind::Ln))).collect()).collect();
        Self::from_triples(triples, windows)
    }

    fn from_triples(per_lane: LaneSeeds, windows: JudgeWindows) -> Self {
        let lanes: Vec<Lane> = per_lane
            .into_iter()
            .map(|mut notes| {
                notes.sort_by_key(|n| n.0);
                Lane {
                    notes: notes
                        .into_iter()
                        .map(|(h, e, ln)| JNote {
                            head_us: h,
                            end_us: e,
                            chart_ln: ln,
                            ln,
                            judged: false,
                            holding: false,
                            head_play_time_us: 0,
                            end_play_time_us: 0,
                            end_judge: None,
                            head_state: UNJUDGED_STATE,
                            end_state: UNJUDGED_STATE,
                        })
                        .collect(),
                    cursor: 0,
                    hcn_cursor: 0,
                }
            })
            .collect();
        let total_notes = count_notes(&lanes);
        let room = judgment_room(&lanes);
        let gauge_set = GaugeSetId::default();
        let gauge_total = DEFAULT_GAUGE_TOTAL;
        let gauge = GrooveGauge::new(gauge_set, GaugeKind::Normal, gauge_total, total_notes as usize);
        let n = lanes.len();
        let prop = JudgeProperty::SEVENKEYS;
        JudgeEngine {
            lanes,
            holds: (0..n).map(|_| LaneHold::default()).collect(),
            mines: (0..n).map(|_| Vec::new()).collect(),
            mine_cursor: vec![0; n],
            held: vec![false; n],
            scratch_lane: vec![false; n],
            prop,
            windows,
            scratch_windows: prop.scratch,
            ln_end: JudgeWindows::SEVENKEY_LN_END,
            ln_scratch_end: prop.ln_scratch_end,
            gate: candidate_gate_across(&windows, &prop.scratch),
            longnote_margin_rate: DEFAULT_LONGNOTE_MARGIN_RATE,
            algorithm: JudgeAlgorithm::default(),
            ln_mode: LnMode::default(),
            gauge_set,
            gauge_total,
            playback: PlaybackRate::UNMODIFIED,
            settled_us: i64::MIN,
            gauge_shift: None,
            failed_at_us: None,
            autoplay: false,
            combo: 0,
            max_combo: 0,
            stage_combo: 0,
            stage_max_combo: 0,
            counts: [0; 6],
            ex_score: 0,
            gauge,
            last_judge: None,
            last_fast: false,
            fast: 0,
            slow: 0,
            early: [0; 6],
            late: [0; 6],
            empty_poor: 0,
            sum_delta_us: 0,
            timing_count: 0,
            total_notes,
            pass_notes: 0,
            judged: Vec::with_capacity(room),
            touched: Vec::with_capacity(room),
        }
    }

    /// Build an engine from `model`, deriving all four timing tables (note/scratch/LN end/LN
    /// scratch end) from the chart's [`Mode`] at its `#RANK`/`#DEFEXRANK` judgerank. Prefer this
    /// over [`from_model`](Self::from_model), whose `windows` argument replaces only the key-lane
    /// note table and can therefore disagree with the other three.
    pub fn from_model_for_mode(model: &Model) -> Self {
        let windows = Self::mode_window_set(model).note;
        Self::from_model(model, windows)
    }

    /// The four stock tables `model` is judged against: its mode's [`JudgeProperty`] row, built
    /// under its mode's [`JudgeWindowRule`] at the judgerank that rule resolves `#RANK`/`#DEFEXRANK`
    /// to, with no user JUDGE WIDTH applied.
    fn mode_window_set(model: &Model) -> JudgeWindowSet {
        let rule = JudgeWindowRule::for_mode(&model.mode);
        let judgerank = rule.judgerank_for(model.meta.rank, model.meta.defexrank);
        JudgeProperty::for_mode(&model.mode).window_set(rule, judgerank, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES)
    }

    /// Build an engine from `model`, overriding the key-lane note table with `windows`.
    ///
    /// The scratch, LN-end and LN-scratch-end tables are always derived from the chart's mode and
    /// judgerank, so `windows` must be the matching mode table (optionally re-scaled) or the key
    /// lanes will judge on a different curve from everything else. [`from_model_for_mode`]
    /// (Self::from_model_for_mode) derives it for you.
    pub fn from_model(model: &Model, windows: JudgeWindows) -> Self {
        let n = model.mode.key;
        let mut per_lane: LaneSeeds = vec![Vec::new(); n];
        let mut mines: Vec<Vec<Mine>> = (0..n).map(|_| Vec::new()).collect();
        for lane in 0..n {
            let mut pending_start: Option<(i64, LnKind)> = None;
            for tl in &model.timelines {
                let Some(note) = &tl.notes[lane] else {
                    continue;
                };
                match note.kind {
                    NoteKind::Mine { damage } => mines[lane].push(Mine { time_us: note.time_us, damage }),
                    NoteKind::Normal => per_lane[lane].push((note.time_us, None, None)),
                    NoteKind::LongStart { ln } => pending_start = Some((note.time_us, ln)),
                    NoteKind::LongEnd { .. } => {
                        if let Some((s, ln)) = pending_start.take() {
                            per_lane[lane].push((s, Some(note.time_us), Some(ln)));
                        }
                    }
                }
            }
            mines[lane].sort_by_key(|m| m.time_us);
        }
        let mut engine = Self::from_triples(per_lane, windows);
        engine.mines = mines;
        engine.mine_cursor = vec![0; n];
        engine.apply_mode(&model.mode, JudgeWindowRule::for_mode(&model.mode).judgerank_for(model.meta.rank, model.meta.defexrank));
        engine.windows = windows;
        engine.refresh_gate();
        engine.set_gauge(GaugeKind::Normal, model.meta.total);
        engine
    }

    /// Adopt `mode`'s [`JudgeProperty`] row, derive all four timing tables at `judgerank` under the
    /// mode's own [`JudgeWindowRule`], and take the gauge table the mode plays on.
    pub fn apply_mode(&mut self, mode: &Mode, judgerank: i32) {
        let prop = JudgeProperty::for_mode(mode);
        self.prop = prop;
        let set = prop.window_set(JudgeWindowRule::for_mode(mode), judgerank, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        self.apply_window_set(&set);
        self.scratch_lane = (0..self.lanes.len()).map(|l| mode.is_scratch(l)).collect();
        self.gauge_set = GaugeSetId::for_mode(mode);
        self.rebuild_gauge();
    }

    /// Judge against `set`: all four tables plus the candidate gate, which is the only place the
    /// scratch JUDGE WIDTH rates reach (`JudgeManager.java:185-198`). Call before play begins.
    pub fn apply_window_set(&mut self, set: &JudgeWindowSet) {
        self.windows = set.note;
        self.scratch_windows = set.scratch;
        self.ln_end = set.ln_end;
        self.ln_scratch_end = set.ln_scratch_end;
        self.gate = set.candidate_gate;
    }

    fn refresh_gate(&mut self) {
        self.gate = candidate_gate_across(&self.windows, &self.scratch_windows);
    }

    fn is_scratch(&self, lane: usize) -> bool {
        self.scratch_lane.get(lane).copied().unwrap_or(false)
    }

    /// The note table `lane` is judged against, on the chart's clock.
    fn note_window(&self, lane: usize) -> JudgeWindows {
        self.playback.chart_windows(if self.is_scratch(lane) { self.scratch_windows } else { self.windows })
    }

    /// The long-note end table `lane` is judged against, on the chart's clock.
    fn ln_window(&self, lane: usize) -> JudgeWindows {
        self.playback.chart_windows(if self.is_scratch(lane) { self.ln_scratch_end } else { self.ln_end })
    }

    /// Release grace for a held long note (`JudgeProperty.longnoteMargin` scaled by the user's
    /// long-note margin rate; the scratch table's margin is not user-scalable in the reference implementation,
    /// `JudgeManager.java:186-188`), on the chart's clock.
    fn ln_margin(&self, lane: usize) -> i64 {
        let margin_us =
            if self.is_scratch(lane) { self.prop.longscratch_margin } else { self.prop.longnote_margin * self.longnote_margin_rate.max(0) as i64 / 100 };
        self.playback.chart_span_us(margin_us)
    }

    /// The span between hell-charge gauge ticks (`JudgeManager.java:81`), on the chart's clock.
    fn hcn_tick_us(&self) -> i64 {
        self.playback.chart_span_us(HCN_GAUGE_TICK_US)
    }

    /// Rebuild the nine gauges for the current table, chart total and note count, keeping whichever
    /// slot is selected.
    fn rebuild_gauge(&mut self) {
        let selected = self.gauge.selected_index();
        self.gauge = GrooveGauge::at_index(self.gauge_set, selected, self.gauge_total, self.total_notes as usize);
    }

    pub fn set_gauge(&mut self, kind: GaugeKind, total: f64) {
        self.gauge_total = total;
        self.gauge = GrooveGauge::new(self.gauge_set, kind, total, self.total_notes as usize);
    }

    /// Play on another gauge table — the LR2 set, which no mode selects, is chosen this way
    /// (`GaugeProperty.java:117-125`).
    pub fn set_gauge_set(&mut self, set: GaugeSetId) {
        self.gauge_set = set;
        self.rebuild_gauge();
    }

    /// The gauge table the nine gauges are built from.
    pub fn gauge_set(&self) -> GaugeSetId {
        self.gauge_set
    }

    /// Replace the key-lane note timing window (e.g. after applying a user judge-width multiplier).
    /// Call before play begins.
    pub fn set_windows(&mut self, windows: JudgeWindows) {
        self.windows = windows;
        self.refresh_gate();
    }

    /// Replace the scratch-lane note timing window.
    pub fn set_scratch_windows(&mut self, windows: JudgeWindows) {
        self.scratch_windows = windows;
        self.refresh_gate();
    }

    /// Replace the key-lane LN/CN release window (kept in step with [`set_windows`](Self::set_windows)
    /// when the user changes JUDGE WIDTH, so note and LN leniency scale together).
    pub fn set_ln_end(&mut self, ln_end: JudgeWindows) {
        self.ln_end = ln_end;
    }

    /// Replace the scratch-lane long-note release window.
    pub fn set_ln_scratch_end(&mut self, ln_end: JudgeWindows) {
        self.ln_scratch_end = ln_end;
    }

    /// Candidate-selection policy for [`press`](Self::press). Defaults to
    /// [`JudgeAlgorithm::Combo`], the reference implementation's own default
    /// (`JudgeAlgorithm.java:42` lists it first); a replay recorded before the algorithm was written
    /// down names [`JudgeAlgorithm::Duration`] and is played back on that.
    pub fn set_algorithm(&mut self, algorithm: JudgeAlgorithm) {
        self.algorithm = algorithm;
    }

    /// The candidate-selection policy currently in force.
    pub fn algorithm(&self) -> JudgeAlgorithm {
        self.algorithm
    }

    /// Resolve every chart note that stated no long-note flavour to `mode`
    /// (`JudgeManager.java:262, 273, 356, 495, 615` consult `lntype` only for `TYPE_UNDEFINED`).
    /// Charge notes are judged twice, so this also re-counts the chart and rebuilds the gauges;
    /// call it before play begins.
    /// Resolving from each note's recorded chart flavour rather than from its current one keeps this
    /// idempotent: setting one mode and then another lands on the second, and setting the same mode
    /// twice changes nothing.
    pub fn set_ln_mode(&mut self, mode: LnMode) {
        self.ln_mode = mode;
        let resolved = mode.resolve();
        let flavoured = self.lanes.iter().flat_map(|lane| lane.notes.iter()).filter(|note| note.chart_ln == Some(LnKind::Undefined)).count();
        self.touched.reserve(flavoured + judgment_room(&self.lanes));
        for (lane, notes) in self.lanes.iter_mut().enumerate() {
            for (index, note) in notes.notes.iter_mut().enumerate() {
                if note.chart_ln == Some(LnKind::Undefined) {
                    note.ln = Some(resolved);
                    self.touched.push(NoteAt { lane, index });
                }
            }
        }
        self.total_notes = count_notes(&self.lanes);
        self.rebuild_gauge();
    }

    /// The long-note flavour chart notes with no stated type play as.
    pub fn ln_mode(&self) -> LnMode {
        self.ln_mode
    }

    /// User LONGNOTE MARGIN rate in percent (100 = the mode's stock margin), reference implementation
    /// `PlayerConfig.longnoteMarginRate`.
    pub fn set_longnote_margin_rate(&mut self, rate_percent: i32) {
        self.longnote_margin_rate = rate_percent;
    }

    /// Judge a chart that is played at `percent` of its own speed, a practice slice's FREQUENCY, in
    /// the player's own time while the engine is still driven on the chart's clock: every window, the
    /// long-note margin, the candidate gate and the hell-charge tick keep their length of the
    /// player's time, and every timing a judgment reports is in the player's time. A rate below 1%
    /// is taken as 1%. Call it before play begins.
    pub fn set_playback_rate(&mut self, percent: i32) {
        self.playback = PlaybackRate::new(percent);
    }

    /// The rate the chart is played at, in percent ([`set_playback_rate`](Self::set_playback_rate)).
    pub fn playback_rate(&self) -> i32 {
        self.playback.percent as i32
    }

    /// Mark the run as one that plays itself. Autoplay is driven through [`auto_press`](Self::auto_press)
    /// and [`auto_release`](Self::auto_release), which never raise a lane's key state, and this flag
    /// is what keeps its hell-charge notes gaining gauge without one (`JudgeManager.java:289-293`).
    pub fn set_autoplay(&mut self, autoplay: bool) {
        self.autoplay = autoplay;
    }

    /// Let the selected gauge move as the run is judged, and end the run the instant it empties
    /// under a mode that does not rescue it (`BMSPlayer.java:638-672`). The shift is taken after
    /// every instant that changed a gauge and at the end of every [`update`](Self::update), not once
    /// per frame, so where it lands does not depend on the frame rate either. An engine never given
    /// one never shifts and never fails.
    pub fn set_gauge_shift(&mut self, shift: GaugeShift) {
        self.gauge_shift = Some(shift);
    }

    /// Whether the selected gauge emptied under a shift mode that does not rescue the run. A failed
    /// engine takes no more input and settles no more time.
    pub fn failed(&self) -> bool {
        self.failed_at_us.is_some()
    }

    /// The instant the run failed at, on the clock the engine is driven on.
    pub fn failed_at_us(&self) -> Option<i64> {
        self.failed_at_us
    }

    /// The instant the clock has been settled up to ([`update`](Self::update)), or `i64::MIN`
    /// before anything has moved it.
    pub fn settled_us(&self) -> i64 {
        self.settled_us
    }

    pub fn total_notes(&self) -> u32 {
        self.total_notes
    }

    /// Notes resolved so far, the reference implementation's `IRScoreData.passnotes`
    /// (`JudgeManager.java:640-646` counts one per judgment that consumed its note). A judgment the
    /// mode's `judgeVanish` leaves the note alive — 空POOR everywhere, and a pop'n BAD — resolves
    /// nothing and is not counted, so this stays the "how far through the chart are we" counter even
    /// when the same note is judged twice.
    pub fn total_judged(&self) -> u32 {
        self.pass_notes
    }

    pub fn clear_lamp(&self) -> ClearType {
        clear_lamp(self.gauge.selected(), &self.counts, self.stage_combo, self.pass_notes, self.total_notes)
    }

    /// Every judged object of the chart with the state and play time the run left on it, lane by
    /// lane in chart order. See [`NoteMark`].
    pub fn note_marks(&self) -> impl Iterator<Item = NoteMark> + '_ {
        self.lanes.iter().enumerate().flat_map(|(lane, l)| l.notes.iter().flat_map(move |note| marks_of(lane, note)))
    }

    /// The marks of one note, its head's and then its end's, as [`JudgeEngine::note_marks`] gives
    /// them; nothing for a note the chart does not have.
    pub fn note_marks_at(&self, at: NoteAt) -> impl Iterator<Item = NoteMark> + '_ {
        self.lanes.get(at.lane).and_then(|lane| lane.notes.get(at.index)).into_iter().flat_map(move |note| marks_of(at.lane, note))
    }

    /// Every note whose marks may have changed since the engine was built, oldest first, named
    /// once for each time something reached it: every judgment counted or not, and every note a
    /// long note mode gave a flavour to. A screen that keeps a copy of [`JudgeEngine::note_marks`]
    /// takes in the notes past the ones it has read rather than walking the whole chart again.
    pub fn touched(&self) -> &[NoteAt] {
        &self.touched
    }

    /// Every judgment the run has counted, in the order it counted them. See [`JudgedNote`].
    pub fn judged(&self) -> &[JudgedNote] {
        &self.judged
    }

    /// The long notes `lane` has in hand right now. A lane the chart does not have has none.
    pub fn long_note(&self, lane: usize) -> LaneLongNote {
        let head_us = |index: usize| self.lanes.get(lane).and_then(|lane| lane.notes.get(index)).map(|note| note.head_us);
        self.holds
            .get(lane)
            .map_or_else(LaneLongNote::default, |hold| LaneLongNote { processing: hold.note.and_then(head_us), passing: hold.passing.and_then(head_us) })
    }

    pub fn press(&mut self, lane: usize, press_us: i64) -> Option<JudgeResult> {
        self.press_dir(lane, ScratchDir::Forward, press_us)
    }

    /// Press a lane from one physical direction. On a scratch lane holding a charge note, spinning
    /// the other way ends it — the back-spin scratch of `JudgeManager.java:358-372` — while the
    /// same way is a re-grab that cancels a deferred release (`:373-375`).
    ///
    /// The clock is settled up to `press_us` first, so whatever fell due at or before that instant
    /// has been judged by the time the press is. A failed run takes no press.
    pub fn press_dir(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        self.update(press_us);
        if self.failed() {
            return None;
        }
        let result = self.press_at(lane, dir, press_us);
        self.shift_gauge(press_us);
        result
    }

    fn press_at(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        if let Some(h) = self.held.get_mut(lane) {
            *h = true;
        }
        if lane >= self.lanes.len() {
            return None;
        }
        if self.holds[lane].note.is_some() {
            return self.press_while_holding(lane, dir, press_us);
        }
        self.press_new_note(lane, dir, press_us)
    }

    fn press_while_holding(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        let idx = self.holds[lane].note?;
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        let owner = self.holds[lane].owner;
        let ends_the_spin = charge && self.is_scratch(lane) && owner.is_some() && owner != Some(dir);
        if !ends_the_spin {
            self.holds[lane].release_us = None;
            return None;
        }
        let end_us = end_us?;
        let delta_us = end_us - press_us;
        let judge = judge_of_code(self.ln_window(lane).judge_code(delta_us))?;
        Some(self.confirm_hold(lane, idx, judge, delta_us, true))
    }

    /// The reference's two-stage candidate fold (`JudgeManager.java:385-415`). Stage one asks the
    /// algorithm whether to move on — a candidate already chosen but already judged is always
    /// replaced, which is what keeps [`JudgeAlgorithm::Lowest`] from re-judging a consumed note.
    /// Stage two turns the survivor into a judge code, where a 空POOR candidate only wins on a
    /// strictly smaller `|Δt|` and a candidate past every window cancels the press outright.
    ///
    /// `code` deliberately lives outside the loop, as the reference's `judge` does: it is the code
    /// of the last candidate that cleared both filters, not necessarily of `chosen`.
    fn select_candidate(&self, lane: usize, press_us: i64) -> (Option<usize>, usize) {
        let w = self.note_window(lane);
        let note_type = if self.is_scratch(lane) { NoteType::Scratch } else { NoteType::Note };
        let algorithm = self.algorithm;
        let miss_once = self.prop.miss_condition == MissCondition::One;
        let (gate_late, gate_early) = (self.playback.chart_span_us(self.gate.0), self.playback.chart_span_us(self.gate.1));
        let l = &self.lanes[lane];

        let mut chosen: Option<usize> = None;
        let mut code = 0usize;
        let floor_us = press_us + gate_late - self.playback.chart_span_us(REMARK_SLACK_US);
        let mut i = l.cursor;
        while i > 0 && l.notes[i - 1].head_us >= floor_us {
            i -= 1;
        }
        while i < l.notes.len() {
            let note = &l.notes[i];
            let delta_us = note.head_us - press_us;
            if delta_us >= gate_early {
                break;
            }
            if delta_us < gate_late {
                i += 1;
                continue;
            }
            let cand = note_ref(note);
            let unjudged = cand.state == UNJUDGED_STATE;
            let adopt = match chosen {
                None => true,
                Some(c) => {
                    let best = note_ref(&l.notes[c]);
                    best.state != UNJUDGED_STATE || algorithm.prefer(&best, &cand, press_us, &w, note_type)
                }
            };
            let blocked = miss_once && (!unjudged || (was_played(note.head_play_time_us) && !w.in_good_band(delta_us)));
            if adopt && !blocked {
                code = if unjudged {
                    let band = w.judge_code(delta_us);
                    if band >= EMPTY_POOR_JUDGE_INDEX { band + 1 } else { band }
                } else if w.in_ms_band(delta_us) {
                    EMPTY_POOR_CODE
                } else {
                    IGNORED_CODE
                };
                if code < IGNORED_CODE {
                    let nearer = match chosen {
                        Some(c) => (l.notes[c].head_us - press_us).abs() > delta_us.abs(),
                        None => true,
                    };
                    if code < EMPTY_POOR_JUDGE_INDEX || nearer {
                        chosen = Some(i);
                    }
                } else {
                    chosen = None;
                }
            }
            i += 1;
        }
        (chosen, code)
    }

    fn press_new_note(&mut self, lane: usize, dir: ScratchDir, press_us: i64) -> Option<JudgeResult> {
        let (chosen, code) = self.select_candidate(lane, press_us);
        let idx = chosen?;
        let judge = judge_of_code(code)?;
        let delta_us = self.lanes[lane].notes[idx].head_us - press_us;
        let vanish = self.prop.judge_vanish[judge as usize];
        let (long, charge) = {
            let note = &self.lanes[lane].notes[idx];
            (note.end_us.is_some(), is_charge(note.ln))
        };
        let mut counted = true;
        if long {
            if vanish {
                self.start_hold(lane, idx, dir, judge, delta_us);
            }
            counted = charge || !vanish;
            if counted {
                self.update_micro(lane, idx, judge, delta_us, false, vanish);
            }
        } else {
            if vanish {
                self.lanes[lane].notes[idx].judged = true;
            }
            self.update_micro(lane, idx, judge, delta_us, false, vanish);
        }
        Some(JudgeResult { judge, lane, note_index: idx, fast: delta_us >= 0, delta_us: self.playback.real_span_us(delta_us), counted })
    }

    /// The autoplay hit on the note whose head sits at `at_us` (`JudgeManager.java:251-271`). The
    /// reference does not press a key for it: it judges the note PGREAT on the spot, so the lane's
    /// key state is left alone and a mine in that lane, which damages only a key that is down
    /// (`JudgeManager.java:244`), costs nothing.
    ///
    /// A normal note is judged whatever else the lane holds. A long note is taken only by a lane
    /// holding none (`:259`): a plain one starts its hold and is judged when it ends, a charge or
    /// hell-charge note is judged at its head as well. A head that is not there, or was judged
    /// already, answers `None`. Like a press, it settles the clock up to `at_us` first.
    pub fn auto_press(&mut self, lane: usize, at_us: i64) -> Option<JudgeResult> {
        self.update(at_us);
        if self.failed() {
            return None;
        }
        let result = self.auto_press_at(lane, at_us);
        self.shift_gauge(at_us);
        result
    }

    fn auto_press_at(&mut self, lane: usize, at_us: i64) -> Option<JudgeResult> {
        let lane_notes = self.lanes.get(lane)?;
        let idx = (lane_notes.cursor..lane_notes.notes.len())
            .take_while(|&i| lane_notes.notes[i].head_us <= at_us)
            .find(|&i| lane_notes.notes[i].head_us == at_us && !lane_notes.notes[i].judged && !lane_notes.notes[i].holding)?;
        let (long, charge) = {
            let note = &self.lanes[lane].notes[idx];
            (note.end_us.is_some(), is_charge(note.ln))
        };
        let mut counted = true;
        if long {
            if self.holds[lane].note.is_some() {
                return None;
            }
            self.start_hold(lane, idx, ScratchDir::Forward, Judge::PerfectGreat, AUTOPLAY_DELTA_US);
            counted = charge;
        } else {
            self.lanes[lane].notes[idx].judged = true;
        }
        if counted {
            self.update_micro(lane, idx, Judge::PerfectGreat, AUTOPLAY_DELTA_US, false, CONSUMES_THE_NOTE);
        }
        Some(JudgeResult { judge: Judge::PerfectGreat, lane, note_index: idx, fast: true, delta_us: AUTOPLAY_DELTA_US, counted })
    }

    /// The autoplay end of the long note whose end sits at `at_us` (`JudgeManager.java:272-283`).
    /// A charge or hell-charge note is judged PGREAT on the spot, whichever way a scratch lane would
    /// have had to spin to end it by hand: the back-spin rule belongs to a key that is pressed, and
    /// autoplay presses none. A plain long note ends with the judgment its head took. A lane that
    /// holds some other note, or none, answers `None`. It settles the clock up to `at_us` first.
    pub fn auto_release(&mut self, lane: usize, at_us: i64) -> Option<JudgeResult> {
        self.update(at_us);
        if self.failed() {
            return None;
        }
        let result = self.auto_release_at(lane, at_us);
        self.shift_gauge(at_us);
        result
    }

    fn auto_release_at(&mut self, lane: usize, at_us: i64) -> Option<JudgeResult> {
        let idx = self.holds.get(lane)?.note?;
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        if end_us != Some(at_us) {
            return None;
        }
        let (judge, delta_us) = if charge {
            (Judge::PerfectGreat, AUTOPLAY_DELTA_US)
        } else {
            (self.holds[lane].start_judge.unwrap_or(Judge::PerfectGreat), self.holds[lane].start_delta_us)
        };
        Some(self.confirm_hold(lane, idx, judge, delta_us, charge))
    }

    /// Take a long note under this lane's control (`JudgeManager.java:428-437, 444-451`), which
    /// only happens when the head judgment consumed it.
    fn start_hold(&mut self, lane: usize, idx: usize, dir: ScratchDir, judge: Judge, delta_us: i64) {
        self.lanes[lane].notes[idx].holding = true;
        let scratch = self.is_scratch(lane);
        let hold = &mut self.holds[lane];
        hold.note = Some(idx);
        hold.start_judge = Some(judge);
        hold.start_delta_us = delta_us;
        hold.release_us = None;
        hold.end_judge = None;
        hold.owner = if scratch { Some(dir) } else { None };
    }

    pub fn release(&mut self, lane: usize, release_us: i64) -> Option<JudgeResult> {
        self.release_dir(lane, ScratchDir::Forward, release_us)
    }

    /// Release a lane from one physical direction. A charge note on a scratch lane only accepts a
    /// mid-spin release from the direction that grabbed it and only from outside the end window
    /// (`JudgeManager.java:501-508`); a plain long note drops the second condition (`:526-532`).
    /// A release that came early and would only be a BAD is deferred rather than confirmed
    /// (`:509-512`), so re-grabbing inside the margin rescues it. Like a press, a release settles
    /// the clock up to its own instant first, and a failed run takes none.
    pub fn release_dir(&mut self, lane: usize, dir: ScratchDir, release_us: i64) -> Option<JudgeResult> {
        self.update(release_us);
        if self.failed() {
            return None;
        }
        let result = self.release_at(lane, dir, release_us);
        self.shift_gauge(release_us);
        result
    }

    fn release_at(&mut self, lane: usize, dir: ScratchDir, release_us: i64) -> Option<JudgeResult> {
        if let Some(h) = self.held.get_mut(lane) {
            *h = false;
        }
        if lane >= self.lanes.len() {
            return None;
        }
        let idx = self.holds[lane].note?;
        let w = self.ln_window(lane);
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        let end_us = end_us?;
        let delta_us = end_us - release_us;
        let band = w.judge_code(delta_us);
        let scratch = self.is_scratch(lane);
        if charge {
            if scratch {
                if band != w.band_count() || self.holds[lane].owner != Some(dir) {
                    return None;
                }
                self.holds[lane].owner = None;
            }
            let judge = judge_of_code(band)?;
            if judge as usize >= DEFERRED_RELEASE_CODE && delta_us > 0 {
                self.holds[lane].release_us = Some(release_us);
                self.holds[lane].end_judge = Some(judge);
                return None;
            }
            return Some(self.confirm_hold(lane, idx, judge, delta_us, true));
        }
        if scratch {
            if self.holds[lane].owner != Some(dir) {
                return None;
            }
            self.holds[lane].owner = None;
        }
        let judge = worse(judge_of_code(band)?, self.holds[lane].start_judge.unwrap_or(Judge::Poor));
        let start_delta_us = self.holds[lane].start_delta_us;
        let delta_us = if start_delta_us.abs() > delta_us.abs() { start_delta_us } else { delta_us };
        if judge as usize >= DEFERRED_RELEASE_CODE && delta_us > 0 {
            self.holds[lane].release_us = Some(release_us);
            self.holds[lane].end_judge = Some(Judge::Bad);
            return None;
        }
        Some(self.confirm_hold(lane, idx, judge, delta_us, false))
    }

    /// Resolve the held note here and now, dropping the hold and its deferred-release state.
    fn confirm_hold(&mut self, lane: usize, idx: usize, judge: Judge, delta_us: i64, on_end: bool) -> JudgeResult {
        self.finish_hold(lane, idx, judge, on_end);
        self.update_micro(lane, idx, judge, delta_us, on_end, CONSUMES_THE_NOTE);
        JudgeResult { judge, lane, note_index: idx, fast: delta_us >= 0, delta_us: self.playback.real_span_us(delta_us), counted: true }
    }

    fn finish_hold(&mut self, lane: usize, idx: usize, judge: Judge, on_end: bool) {
        let note = &mut self.lanes[lane].notes[idx];
        note.judged = true;
        note.holding = false;
        if on_end {
            note.end_judge = Some(judge);
        }
        self.holds[lane].clear();
    }

    /// Settle the clock up to `now_us`: sweep notes that can no longer be hit (now past their latest
    /// BAD-late bound) into 見逃し POOR, confirm deferred and over-held long notes, pay out
    /// hell-charge gauge ticks and charge mine damage to lanes whose key is down.
    ///
    /// Each of these is made at the instant it falls due, in the order they fall due, with the
    /// state of the keys as it stood then, so one call to a far instant leaves the engine exactly
    /// where any number of calls to the instants on the way would. A clock already settled past
    /// `now_us` is left where it is, and a failed run settles nothing more.
    pub fn update(&mut self, now_us: i64) {
        while !self.failed()
            && let Some(at_us) = self.next_due_us(now_us)
        {
            self.settle_at(at_us);
        }
        if !self.failed() && now_us > self.settled_us {
            self.run_hell_charges(now_us);
            self.settled_us = now_us;
        }
        self.shift_gauge(now_us);
    }

    /// Everything that falls due at `at_us`, the next instant something does: the hell-charge
    /// balances run up to it, then the mines it reaches, the hell-charge notes it enters or leaves,
    /// and the long notes and misses it settles.
    fn settle_at(&mut self, at_us: i64) {
        self.run_hell_charges(at_us);
        self.sweep_mines(at_us);
        for lane in 0..self.lanes.len() {
            self.advance_passing(lane, at_us);
        }
        let mut events: Vec<SweepEvent> = Vec::new();
        for lane in 0..self.lanes.len() {
            self.confirm_deferred(lane, at_us, &mut events);
            self.sweep_lane(lane, at_us, &mut events);
        }
        for e in events {
            self.update_micro(e.lane, e.note, e.judge, e.delta_us, e.on_end, CONSUMES_THE_NOTE);
        }
        self.settled_us = at_us;
        self.shift_gauge(at_us);
    }

    /// The first instant, no earlier than the settled clock and no later than `limit_us`, at which
    /// something falls due: a mine, a hell-charge note's head or end, a hell-charge tick, a deferred
    /// release outlasting its margin, a plain long note held past its end, or a note going past its
    /// last chance to be hit.
    ///
    /// Something an input made due at the instant it landed — a release deferred by a margin of
    /// nothing — falls due at that same instant, after the input, as the reference confirms it in
    /// the frame that took the release. Every one of these is used up by
    /// [`settle_at`](Self::settle_at), so settling an instant twice cannot find it due again.
    fn next_due_us(&self, limit_us: i64) -> Option<i64> {
        let earliest_us = self.settled_us;
        let mut next: Option<i64> = None;
        let mut consider = |due_us: i64| {
            let due_us = due_us.max(earliest_us);
            if due_us <= limit_us && next.is_none_or(|next_us| due_us < next_us) {
                next = Some(due_us);
            }
        };
        for lane in 0..self.lanes.len() {
            if let Some(mine) = self.mines[lane].get(self.mine_cursor[lane]) {
                consider(mine.time_us);
            }
            let notes = &self.lanes[lane];
            if let Some(note) = notes.notes.get(notes.hcn_cursor) {
                match (self.holds[lane].passing == Some(notes.hcn_cursor), note.end_us) {
                    (true, Some(end_us)) => consider(end_us),
                    _ => consider(note.head_us),
                }
            }
            if let Some(direction) = self.hell_charge_direction(lane) {
                let balance_us = self.holds[lane].passing_us;
                let tick_us = self.hcn_tick_us();
                let to_tick_us = if direction { tick_us - balance_us } else { balance_us + tick_us };
                consider(self.settled_us.saturating_add(to_tick_us).saturating_add(1));
            }
            if let Some(idx) = self.holds[lane].note
                && let Some(note) = notes.notes.get(idx)
                && let Some(end_us) = note.end_us
            {
                if let Some(release_us) = self.holds[lane].release_us {
                    consider(release_us.saturating_add(self.ln_margin(lane)));
                }
                if !is_charge(note.ln) {
                    consider(end_us.saturating_add(1));
                }
            }
            let miss_bound = self.note_window(lane).bd.0;
            if let Some(note) = notes.notes.get(notes.cursor..).and_then(|ahead| ahead.iter().find(|note| !note.judged)) {
                match (note.holding, note.end_us) {
                    (false, _) => consider(note.head_us.saturating_sub(miss_bound).saturating_add(1)),
                    (true, Some(end_us)) if is_charge(note.ln) => consider(end_us.saturating_sub(miss_bound).saturating_add(1)),
                    (true, _) => {}
                }
            }
        }
        next
    }

    /// Take the auto-shift of the selected gauge at `at_us`, and fail the run there when it says so.
    fn shift_gauge(&mut self, at_us: i64) {
        if self.failed() {
            return;
        }
        let Some(shift) = self.gauge_shift else {
            return;
        };
        if self.gauge.auto_shift(shift.mode, shift.configured, shift.bottom) == GaugeShiftOutcome::Failed {
            self.failed_at_us = Some(at_us);
        }
    }

    fn sweep_mines(&mut self, now_us: i64) {
        for lane in 0..self.mines.len() {
            while self.mine_cursor[lane] < self.mines[lane].len() && self.mines[lane][self.mine_cursor[lane]].time_us <= now_us {
                let damage = self.mines[lane][self.mine_cursor[lane]].damage;
                if self.held.get(lane).copied().unwrap_or(false) {
                    self.gauge.add_value(-damage as f32);
                }
                self.mine_cursor[lane] += 1;
            }
        }
    }

    /// Whether the hell-charge note the play head is inside on `lane` is gaining gauge right now
    /// (`true`) or draining it (`false`), or `None` when it pays nothing out: the lane is inside
    /// none, or inside one nothing has judged yet (`JudgeManager.java:289-293, 300`). It gains while
    /// the run plays itself, while the lane's key is down, or once its end took a GOOD or better.
    fn hell_charge_direction(&self, lane: usize) -> Option<bool> {
        let note = self.lanes[lane].notes.get(self.holds[lane].passing?)?;
        if !(note.judged || note.holding) {
            return None;
        }
        let held = self.held.get(lane).copied().unwrap_or(false);
        Some(self.autoplay || held || matches!(note.end_judge, Some(j) if (j as usize) <= HCN_HELD_END_CODE))
    }

    /// Hell-charge gauge ticks (`JudgeManager.java:305-336`), from the settled clock up to `to_us`.
    /// A note the play head is inside pays out half a GREAT for every 200 ms it is held (or after
    /// its end already took a GOOD or better) and half a BAD for every 200 ms it is not. Counts,
    /// combo and EX score are untouched.
    ///
    /// The balance runs on the time that went by, not on frames: [`update`](Self::update) stops at
    /// every instant a tick falls due and at every instant the note or the key changes, so the
    /// ticks are the same however the time was cut up. The reference pays at most one per frame.
    fn run_hell_charges(&mut self, to_us: i64) {
        let tick_us = self.hcn_tick_us();
        for lane in 0..self.lanes.len() {
            let Some(increasing) = self.hell_charge_direction(lane) else {
                continue;
            };
            let step_us = to_us.saturating_sub(self.settled_us);
            self.holds[lane].increasing = increasing;
            if increasing {
                self.holds[lane].passing_us += step_us;
                while self.holds[lane].passing_us > tick_us {
                    self.gauge.update_with_rate(Judge::Great, HCN_GAUGE_TICK_RATE);
                    self.holds[lane].passing_us -= tick_us;
                }
            } else {
                self.holds[lane].passing_us -= step_us;
                while self.holds[lane].passing_us < -tick_us {
                    self.gauge.update_with_rate(Judge::Bad, HCN_GAUGE_TICK_RATE);
                    self.holds[lane].passing_us += tick_us;
                }
            }
        }
    }

    /// Track which hell-charge note the play head is inside (`JudgeManager.java:235-242`): the head
    /// arms it and the end disarms it, clearing the tick balance.
    fn advance_passing(&mut self, lane: usize, now_us: i64) {
        loop {
            let cursor = self.lanes[lane].hcn_cursor;
            let Some(note) = self.lanes[lane].notes.get(cursor) else {
                return;
            };
            let head_us = note.head_us;
            let end_us = note.end_us;
            let hell_charge = is_hell_charge(note.ln);
            if head_us > now_us {
                return;
            }
            let span_end_us = end_us.filter(|_| hell_charge);
            if let Some(span_end_us) = span_end_us
                && span_end_us > now_us
            {
                self.holds[lane].passing = Some(cursor);
                return;
            }
            if span_end_us.is_some() && self.holds[lane].passing == Some(cursor) {
                self.holds[lane].passing = None;
                self.holds[lane].passing_us = 0;
            }
            self.lanes[lane].hcn_cursor += 1;
        }
    }

    /// Confirm a long note whose deferred release has outlasted the margin, or a plain one whose
    /// end time has gone by while it was still held (`JudgeManager.java:559-590`).
    fn confirm_deferred(&mut self, lane: usize, now_us: i64, events: &mut Vec<SweepEvent>) {
        let Some(idx) = self.holds[lane].note else {
            return;
        };
        let margin = self.ln_margin(lane);
        let (charge, end_us) = {
            let note = &self.lanes[lane].notes[idx];
            (is_charge(note.ln), note.end_us)
        };
        let Some(end_us) = end_us else {
            return;
        };
        if let Some(release_us) = self.holds[lane].release_us
            && release_us + margin <= now_us
        {
            let judge = self.holds[lane].end_judge.unwrap_or(Judge::Poor);
            self.finish_hold(lane, idx, judge, charge);
            events.push(SweepEvent { lane, note: idx, judge, delta_us: end_us - release_us, on_end: charge });
            return;
        }
        if !charge && end_us < now_us {
            let judge = self.holds[lane].start_judge.unwrap_or(Judge::Poor);
            let delta_us = self.holds[lane].start_delta_us;
            self.finish_hold(lane, idx, judge, false);
            events.push(SweepEvent { lane, note: idx, judge, delta_us, on_end: false });
        }
    }

    /// 見逃し POOR sweep (`JudgeManager.java:592-628`): every unhit note now past its BAD-late
    /// bound, a charge note's head and end both, and a held charge note whose end went by unheld.
    fn sweep_lane(&mut self, lane: usize, now_us: i64, events: &mut Vec<SweepEvent>) {
        let miss_bound = self.note_window(lane).bd.0;
        loop {
            let cursor = self.lanes[lane].cursor;
            let Some(note) = self.lanes[lane].notes.get(cursor) else {
                return;
            };
            let (judged, holding, head_us, end_us, charge) = (note.judged, note.holding, note.head_us, note.end_us, is_charge(note.ln));
            if judged {
                self.lanes[lane].cursor += 1;
                continue;
            }
            if holding {
                let Some(end_us) = end_us else {
                    self.lanes[lane].cursor += 1;
                    continue;
                };
                if !charge || end_us - now_us >= miss_bound {
                    return;
                }
                self.finish_hold(lane, cursor, Judge::Poor, true);
                events.push(SweepEvent { lane, note: cursor, judge: Judge::Poor, delta_us: end_us - now_us, on_end: true });
                self.lanes[lane].cursor += 1;
                continue;
            }
            if head_us - now_us >= miss_bound {
                return;
            }
            let delta_us = head_us - now_us;
            self.lanes[lane].notes[cursor].judged = true;
            events.push(SweepEvent { lane, note: cursor, judge: Judge::Poor, delta_us, on_end: false });
            if charge && end_us.is_some() {
                events.push(SweepEvent { lane, note: cursor, judge: Judge::Poor, delta_us, on_end: true });
            }
            self.lanes[lane].cursor += 1;
        }
    }

    /// Count one judgment (`JudgeManager.java:639-655`). `vanish` is the reference's own
    /// `judgeVanish` argument — `judgeVanish[judge]` at a note head, and an unconditional `true`
    /// everywhere a long-note end, a deferred release or a 見逃し POOR sweep resolves a note — and
    /// only a vanishing judgment advances `passnotes`. Under [`MissCondition::One`] a 見逃し POOR on
    /// a note that already survived a judgment is consumed but not counted, which is what stops a
    /// pop'n BAD from tallying twice.
    ///
    /// `delta_us` is on the chart's clock; what the note keeps and the tallies take is the same
    /// timing in the player's time.
    fn update_micro(&mut self, lane: usize, idx: usize, judge: Judge, delta_us: i64, on_end: bool, vanish: bool) {
        let delta_us = self.playback.real_span_us(delta_us);
        self.touched.push(NoteAt { lane, index: idx });
        let note = &mut self.lanes[lane].notes[idx];
        if vanish {
            self.pass_notes += 1;
            let state = judge as u8 + 1;
            if on_end {
                note.end_state = state;
            } else {
                note.head_state = state;
            }
        }
        let played = if on_end { note.end_play_time_us } else { note.head_play_time_us };
        if self.prop.miss_condition == MissCondition::One && judge == Judge::Poor && was_played(played) {
            return;
        }
        if on_end {
            note.end_play_time_us = delta_us;
        } else {
            note.head_play_time_us = delta_us;
        }
        self.apply(judge);
        self.record_timing(judge, delta_us);
        self.judged.push(JudgedNote { lane, judge, delta_us, combo: self.combo });
    }

    fn apply(&mut self, judge: Judge) {
        let i = judge as usize;
        self.counts[i] += 1;
        match judge {
            Judge::PerfectGreat => self.ex_score += 2,
            Judge::Great => self.ex_score += 1,
            _ => {}
        }
        if self.prop.combo[i] && i < 5 {
            self.combo += 1;
            self.stage_combo += 1;
        }
        if !self.prop.combo[i] {
            self.combo = 0;
            self.stage_combo = 0;
        }
        self.gauge.update(judge);
        self.last_judge = Some(judge);
        self.max_combo = self.max_combo.max(self.combo);
        self.stage_max_combo = self.stage_max_combo.max(self.stage_combo);
        self.empty_poor = self.counts[5];
    }

    fn record_timing(&mut self, judge: Judge, dm: i64) {
        self.record_direction(judge, dm);
        if (judge as usize) < 4 {
            self.last_fast = dm >= 0;
            if dm >= 0 {
                self.fast += 1;
            } else {
                self.slow += 1;
            }
            self.sum_delta_us += dm;
            self.timing_count += 1;
        }
    }

    /// Tally a judgment as EARLY (`dm >= 0`, pressed at or before the note) or LATE, keeping
    /// `early[i] + late[i] == counts[i]` (reference implementation `score.addJudgeCount(judge, mfast >= 0, 1)`).
    fn record_direction(&mut self, judge: Judge, dm: i64) {
        let i = judge as usize;
        if dm >= 0 {
            self.early[i] += 1;
        } else {
            self.late[i] += 1;
        }
    }

    /// Mean signed hit timing (µs) over actual hits: how far early (positive) or late the run leaned.
    /// Zero when nothing was timed.
    ///
    /// Not the reference's `ScoreData.avgjudge`, which is the mean distance of every note from its
    /// input with a note never hit counted at a second (`BMSPlayer.java:911-929`); the play crate
    /// works that one out from the notes' marks (`rbms_play::PlayRecord::avg_duration_us`).
    pub fn avg_judge_us(&self) -> i64 {
        if self.timing_count == 0 { 0 } else { self.sum_delta_us / self.timing_count as i64 }
    }
}

/// Whether a stored play time reads back as non-zero through the reference's millisecond accessor
/// (`JudgeManager.java:398, 648`).
fn was_played(play_time_us: i64) -> bool {
    play_time_us / MICROS_PER_MILLI != 0
}

/// Extra slack the reference implementation subtracts when re-marking a lane's scan start each frame
/// (`JudgeManager.java:220`: `prevmtime + mjudgestart - 100000`).
const REMARK_SLACK_US: i64 = 100_000;

/// The `judgeVanish` argument of every `updateMicro` call that resolves a note outright: a
/// long-note end, a deferred release, a 見逃し POOR sweep (`JudgeManager.java:517, 567, 573, 583,
/// 596-618` all pass a literal `true`).
const CONSUMES_THE_NOTE: bool = true;

/// The timing error autoplay judges every note with: it hits each one dead on (`JudgeManager.java:256,
/// 268, 280` pass a `mfast` of 0).
const AUTOPLAY_DELTA_US: i64 = 0;

/// Stock long-note margin rate (percent) — reference implementation `PlayerConfig.longnoteMarginRate` default.
const DEFAULT_LONGNOTE_MARGIN_RATE: i32 = 100;

/// Chart total the mode-less constructors seed their gauges with until a caller supplies the real
/// one through [`JudgeEngine::set_gauge`].
const DEFAULT_GAUGE_TOTAL: f64 = 200.0;

#[cfg(test)]
mod algorithm_selection_tests {
    use super::*;
    use crate::algorithm::JudgeAlgorithm;

    const EARLY_NOTE_US: i64 = 100_000;
    const LATE_NOTE_US: i64 = 200_000;

    fn engine(algorithm: JudgeAlgorithm) -> JudgeEngine {
        let mut e = JudgeEngine::new(vec![vec![EARLY_NOTE_US, LATE_NOTE_US]], JudgeWindows::SEVENKEY_NOTE);
        e.set_algorithm(algorithm);
        e
    }

    #[test]
    fn the_default_algorithm_is_combo() {
        assert_eq!(JudgeEngine::new(vec![vec![0]], JudgeWindows::SEVENKEY_NOTE).algorithm(), JudgeAlgorithm::Combo);
    }

    #[test]
    fn apply_mode_builds_popn_under_the_pms_rule() {
        let mode = rbms_model::Mode::POPN_9K;
        let rank = JudgeWindowRule::Pms.judgerank_for_rank(2);
        let mut e = JudgeEngine::new(vec![Vec::new(); mode.key], JudgeWindows::SEVENKEY_NOTE);
        e.apply_mode(&mode, rank);
        let expected = JudgeWindowSet::for_mode(&mode, rank, UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        assert_eq!(e.windows, expected.note, "PMS fixjudge holds PG/BAD/空POOR at their tabulated width");
        assert_eq!(e.ln_end, expected.ln_end);
        assert_ne!(e.windows, JudgeProperty::PMS.note.scaled(rank), "the NORMAL rule would have scaled PG as well");
    }

    #[test]
    fn from_model_for_mode_takes_the_judgerank_of_the_modes_own_rule() {
        let mode = rbms_model::Mode::POPN_9K;
        let model = Model {
            mode,
            meta: rbms_model::ModelMeta { total: 200.0, rank: 2, ..Default::default() },
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 130.0,
            timelines: Vec::new(),
            md5: String::new(),
            sha256: String::new(),
        };
        let engine = JudgeEngine::from_model_for_mode(&model);
        let expected = JudgeWindowSet::for_mode(&mode, JudgeWindowRule::Pms.judgerank_for_rank(2), UNMODIFIED_JUDGE_WIDTH_RATES, UNMODIFIED_JUDGE_WIDTH_RATES);
        assert_eq!(engine.windows, expected.note, "the PMS rule resolves #RANK 2 to 70, not the NORMAL 75");
    }

    #[test]
    fn duration_takes_the_nearer_note_even_when_the_earlier_one_is_still_reachable() {
        let press_us = 160_000;
        assert_eq!(engine(JudgeAlgorithm::Duration).press(0, press_us).unwrap().note_index, 1);
    }

    #[test]
    fn combo_and_lowest_keep_the_earlier_note_that_duration_would_abandon() {
        let press_us = 160_000;
        for algorithm in [JudgeAlgorithm::Combo, JudgeAlgorithm::Lowest] {
            assert_eq!(engine(algorithm).press(0, press_us).unwrap().note_index, 0, "{algorithm:?}");
        }
    }

    #[test]
    fn score_abandons_a_note_that_can_no_longer_be_a_great_while_combo_keeps_it() {
        let press_us = 170_000;
        assert_eq!(engine(JudgeAlgorithm::Score).press(0, press_us).unwrap().note_index, 1);
        assert_eq!(engine(JudgeAlgorithm::Combo).press(0, press_us).unwrap().note_index, 0);
    }

    #[test]
    fn score_keeps_a_note_that_is_still_a_great() {
        let press_us = 160_000;
        assert_eq!(engine(JudgeAlgorithm::Score).press(0, press_us).unwrap().note_index, 0);
    }

    #[test]
    fn every_algorithm_still_takes_the_only_candidate() {
        for algorithm in JudgeAlgorithm::ALL {
            let mut e = JudgeEngine::new(vec![vec![EARLY_NOTE_US]], JudgeWindows::SEVENKEY_NOTE);
            e.set_algorithm(algorithm);
            let r = e.press(0, EARLY_NOTE_US).unwrap();
            assert_eq!(r.judge, Judge::PerfectGreat, "{algorithm:?}");
            assert_eq!(r.note_index, 0, "{algorithm:?}");
        }
    }

    #[test]
    fn scratch_lanes_are_judged_against_the_scratch_table() {
        let model_mode = rbms_model::Mode::BEAT_7K;
        let mut e = JudgeEngine::new(vec![Vec::new(); model_mode.key], JudgeWindows::SEVENKEY_NOTE);
        e.apply_mode(&model_mode, 100);
        assert!(e.is_scratch(model_mode.key - 1), "lane 7 is the 7K scratch");
        assert_eq!(e.note_window(model_mode.key - 1), JudgeProperty::SEVENKEYS.scratch);
    }

    #[test]
    fn a_judged_first_candidate_is_always_replaced_even_under_lowest() {
        let mut e = engine(JudgeAlgorithm::Lowest);
        assert_eq!(e.press(0, EARLY_NOTE_US).unwrap().judge, Judge::PerfectGreat, "the first note is consumed");
        let r = e.press(0, LATE_NOTE_US).unwrap();
        assert_eq!(r.note_index, 1, "JudgeManager.java:396 replaces an already-judged t1 whatever the algorithm says");
        assert_eq!(r.judge, Judge::PerfectGreat);
        assert_eq!(e.counts[5], 0, "the consumed note never becomes the chosen candidate again");
    }

    #[test]
    fn a_press_inside_the_gate_but_past_every_note_band_yields_nothing() {
        let mode = rbms_model::Mode::BEAT_7K;
        let mut e = JudgeEngine::new(vec![vec![1_000_000]; mode.key], JudgeWindows::SEVENKEY_NOTE);
        e.apply_mode(&mode, 100);
        let (gate_late, _) = e.gate;
        assert_eq!(gate_late, JudgeProperty::SEVENKEYS.scratch.bd.0, "the scratch table widens the shared gate past the key table's own bands");
        let press_us = 1_000_000 + JudgeProperty::SEVENKEYS.note.bd.0.abs() + 5_000;
        assert!(1_000_000 - press_us > gate_late, "the press is still inside the shared candidate gate");
        e.update(press_us);
        let let_pass = e.counts;
        assert!(e.press(0, press_us).is_none(), "judge code 6: inside the gate, outside every window of this lane's table");
        assert_eq!(e.counts, let_pass, "a cancelled press tallies nothing at all");
    }

    #[test]
    fn a_play_time_inside_one_millisecond_reads_back_as_never_played() {
        assert!(!was_played(0), "a note nothing has judged yet");
        assert!(!was_played(MICROS_PER_MILLI - 1), "Note.getPlayTime() truncates to milliseconds (JudgeManager.java:398, 648)");
        assert!(!was_played(-(MICROS_PER_MILLI - 1)));
        assert!(was_played(MICROS_PER_MILLI));
        assert!(was_played(-MICROS_PER_MILLI));
    }

    #[test]
    fn an_empty_poor_candidate_only_wins_on_a_strictly_smaller_delta() {
        let build = || {
            let mut e = JudgeEngine::new(vec![vec![1_000_000, 1_100_000]], JudgeWindows::SEVENKEY_NOTE);
            e.press(0, 1_000_000);
            e.press(0, 1_100_000);
            e
        };
        assert_eq!(build().press(0, 1_060_000).unwrap().note_index, 1, "JudgeManager.java:408-410 takes the nearer of two consumed notes");
        assert_eq!(build().press(0, 1_040_000).unwrap().note_index, 0, "and keeps the first one when it is no further away");
    }

    #[test]
    fn the_gate_excludes_its_own_early_bound() {
        let mut at_bound = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert!(at_bound.press(0, 500_000).is_none(), "JudgeManager.java:387 breaks on dmtime >= mjudgeend");
        let mut inside = JudgeEngine::new(vec![vec![1_000_000]], JudgeWindows::SEVENKEY_NOTE);
        assert_eq!(inside.press(0, 500_001).unwrap().judge, Judge::Miss, "one microsecond inside the gate is still an empty poor");
    }
}
