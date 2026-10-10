//! What a run has done so far, kept in the shape a skin's play screen reads it in.
//!
//! The judge engine keeps a run as totals: how many of each judgement, the combo, the gauge. A skin
//! asks for more than that. Its note field asks what became of each note it draws
//! ([`NoteStates`]); its judge object asks what the last judgement in each of its regions was, and
//! what the combo stood at then; and its timers are switched by each judgement as it lands, in the
//! lane it landed in.
//!
//! The engine keeps both sides of that. Every judgement it counts is kept in the order it was
//! counted, with its lane and the combo it left ([`rbms_judge::JudgedNote`]), whoever gave it: an
//! input, a note that went by unhit, a lane the engine plays for the player, a replay it
//! reproduces. And every note keeps the state its judgement left it in. So a [`RunTrace`] keeps how
//! far into the engine's judgements it has read, and [`RunTrace::take_judgements`] reports the ones
//! counted since it was last asked, exactly as they came. It also keeps a copy of the state of every
//! note, which is what the note field asks for and what the two graphs of a run in progress are
//! counted from, at the cost of walking the chart's notes once a frame.

use rbms_judge::{Judge, JudgeEngine, LaneLongNote};
use rbms_render::skin_render::frame::{JudgeFrame, JudgeHit, LaneLong, NOTE_UNJUDGED, NoteStates};

use crate::skin_host::play::{Judgement as ShownJudgement, PLAYER_SIDES};
use crate::skin_host::play_timers::judge_region;

/// Microseconds in a millisecond, which is what a judgement's timing is cut to for the skin.
const MICROS_PER_MILLI: i64 = 1_000;

/// One judgement the run took since the trace was last asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Seen {
    /// The chart lane it was given in.
    pub(crate) lane: usize,
    pub(crate) judge: Judge,
    /// How early the input was, in microseconds; a late one is negative.
    pub(crate) fast_us: i64,
    /// The combo the run stood at once it was counted, carried across the stages of a course.
    pub(crate) combo: u32,
}

/// One note as the trace keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kept {
    /// When the note is reached, in microseconds of chart time.
    time_us: i64,
    /// The reference's `Note.getState()`.
    state: u8,
    /// Whether the judgement the note took came late, which is what the early and late graph tells
    /// apart.
    late: bool,
}

/// The states of one lane's notes, heads and long note ends apart, each in chart order.
#[derive(Debug, Default, Clone)]
struct LaneStates {
    heads: Vec<Kept>,
    ends: Vec<Kept>,
}

/// Puts one note in its place in `list`, answering what was kept there before.
fn keep(list: &mut Vec<Kept>, index: usize, note: Kept) -> Option<Kept> {
    match list.get_mut(index) {
        Some(kept) => Some(std::mem::replace(kept, note)),
        None => {
            list.push(note);
            None
        }
    }
}

/// How many classes the judgement graph sorts a second's notes into, and how many the early and
/// late graph does (`SkinNoteDistributionGraph`, types 1 and 2).
const JUDGE_CLASSES: usize = rbms_play::JUDGE_SECOND_KINDS;
const TIMING_CLASSES: usize = rbms_play::TIMING_SECOND_KINDS;

/// The last state the early and late graph does not split: a perfect great has no side.
const LAST_UNSPLIT_STATE: u8 = 1;

/// How far past its early class a judgement's late class is in that graph.
const LATE_CLASS_OFFSET: usize = 4;

/// Microseconds in a second, which is the row a note is counted in.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// How the notes of each second of the chart stand, for the two judgement graphs that plot a run as
/// it goes: a note is counted in the second its own time falls in, under the judgement it took, and
/// under "not judged yet" until it takes one. Kept in step with the notes as they change, so a frame
/// costs nothing to hand it over.
#[derive(Debug, Default, Clone)]
struct JudgedSeconds {
    by_judge: Vec<[u32; JUDGE_CLASSES]>,
    by_timing: Vec<[u32; TIMING_CLASSES]>,
}

impl JudgedSeconds {
    /// Makes sure there is a row for every second up to `seconds`.
    fn reach(&mut self, seconds: usize) {
        if self.by_judge.len() < seconds {
            self.by_judge.resize(seconds, [0; JUDGE_CLASSES]);
            self.by_timing.resize(seconds, [0; TIMING_CLASSES]);
        }
    }

    /// Counts `note` in its second, or takes it out again.
    fn count(&mut self, note: Kept, counted: bool) {
        let Ok(second) = usize::try_from(note.time_us / MICROS_PER_SECOND) else {
            return;
        };
        self.reach(second + 1);
        let state = usize::from(note.state);
        let timing = if note.state <= LAST_UNSPLIT_STATE || !note.late { state } else { state + LATE_CLASS_OFFSET };
        let classes = [self.by_judge[second].get_mut(state), self.by_timing[second].get_mut(timing)];
        for class in classes.into_iter().flatten() {
            *class = if counted { class.saturating_add(1) } else { class.saturating_sub(1) };
        }
    }
}

/// The run so far as a skin reads it. See the module's own notes.
#[derive(Debug, Default, Clone)]
pub(crate) struct RunTrace {
    lanes: Vec<LaneStates>,
    /// How many of the engine's judgements the trace has read.
    judged: usize,
    /// Whether the trace has seen the run at all. The first look only takes the run in.
    seen: bool,
    /// Whether a judgement of this run has left the combo at nothing.
    broke: bool,
    /// When each lane's key beam last went on, as the engine stamps it, to tell a new press from
    /// one that is still down.
    beam_on: Vec<i64>,
    /// What each judge region last reported, for the judge object.
    regions: JudgeFrame,
    /// The last judgement of each side, for the options and the numbers that show it.
    sides: [Option<ShownJudgement>; PLAYER_SIDES],
    /// How the notes of each second of the chart stand.
    seconds: JudgedSeconds,
}

impl RunTrace {
    /// A trace that has seen nothing.
    pub(crate) fn new() -> RunTrace {
        RunTrace::default()
    }

    /// Forgets what the run did, for a run that has been put somewhere else in its chart: the next
    /// look takes the run in afresh and reports nothing, and no region shows a judgement.
    pub(crate) fn reset(&mut self) {
        let seconds = self.seconds.by_judge.len();
        *self = RunTrace::new();
        self.reach(seconds);
    }

    /// The judgements the run took since this was last asked, in the order the engine counted them.
    ///
    /// The first look at a run reports nothing: whatever the run did before the trace was looking
    /// is taken in and not told again. An engine that has counted fewer judgements than the trace
    /// has read is another run of the chart -- a replay put somewhere else in it -- and is taken in
    /// the same way.
    pub(crate) fn take_judgements(&mut self, engine: &JudgeEngine) -> Vec<Seen> {
        self.take_states(engine);
        let judged = engine.judged();
        let report = std::mem::replace(&mut self.seen, true) && self.judged <= judged.len();
        let unread = if report { &judged[self.judged..] } else { judged };
        self.broke = (report && self.broke) || unread.iter().any(|judged| judged.combo == 0);
        self.judged = judged.len();
        if !report {
            return Vec::new();
        }
        unread.iter().map(|judged| Seen { lane: judged.lane, judge: judged.judge, fast_us: judged.delta_us, combo: judged.combo }).collect()
    }

    /// The combo of this chart alone, for a run that stands at `combo` and began its chart with
    /// `carried` already counted (`JudgeManager.getCombo` beside `getCourseCombo`): the two only
    /// differ in a later stage of a course, and only until the combo first breaks there.
    pub(crate) fn stage_combo(&self, combo: u32, carried: u32) -> u32 {
        if self.broke { combo } else { combo.saturating_sub(carried) }
    }

    /// Takes in the state of every note as the engine has it now, and keeps the tables of the
    /// chart's seconds in step with the ones that changed.
    fn take_states(&mut self, engine: &JudgeEngine) {
        let mut lane_now = usize::MAX;
        let (mut head, mut end) = (0, 0);
        for mark in engine.note_marks() {
            if mark.lane != lane_now {
                self.close_lane(lane_now, head, end);
                lane_now = mark.lane;
                (head, end) = (0, 0);
            }
            if self.lanes.len() <= mark.lane {
                self.lanes.resize_with(mark.lane + 1, LaneStates::default);
            }
            let lane = &mut self.lanes[mark.lane];
            let (list, index) = if mark.long_end { (&mut lane.ends, &mut end) } else { (&mut lane.heads, &mut head) };
            let note = Kept { time_us: mark.time_us, state: mark.state, late: mark.play_time_us / MICROS_PER_MILLI < 0 };
            let before = keep(list, *index, note);
            *index += 1;
            let counted = mark.charge || !mark.long_end;
            if before == Some(note) || !counted {
                continue;
            }
            if let Some(before) = before {
                self.seconds.count(before, false);
            }
            self.seconds.count(note, true);
        }
        self.close_lane(lane_now, head, end);
    }

    /// Drops whatever a lane kept past the notes it has now, for a chart that changed under the
    /// trace.
    fn close_lane(&mut self, lane: usize, heads: usize, ends: usize) {
        if let Some(lane) = self.lanes.get_mut(lane) {
            lane.heads.truncate(heads);
            lane.ends.truncate(ends);
        }
    }

    /// The lanes whose key went down since this was last asked, read off when the engine last lit
    /// each lane's beam (`KeyInputProccessor.inputKeyOn`).
    pub(crate) fn take_presses(&mut self, beam_on: &[i64]) -> Vec<usize> {
        if self.beam_on.len() != beam_on.len() {
            self.beam_on = vec![i64::MIN; beam_on.len()];
        }
        let pressed =
            beam_on.iter().zip(&self.beam_on).enumerate().filter(|(_, (now, before))| **now != i64::MIN && now != before).map(|(lane, _)| lane).collect();
        self.beam_on.copy_from_slice(beam_on);
        pressed
    }

    /// Notes a judgement as the last one of its region, as the judge manager does
    /// (`JudgeManager.java:677-690`): the judgement and the combo it left for the judge object, and
    /// the judgement and its timing for the options and the numbers. A skin with no judge object
    /// has no region to note it in.
    pub(crate) fn note_judgement(&mut self, seen: Seen, lane_count: usize, regions: usize, now_us: i64) {
        let Some(region) = judge_region(seen.lane, lane_count, regions) else {
            return;
        };
        let judgement = seen.judge as usize;
        self.regions = self.regions.with_region(region, JudgeHit { judgement, combo: i32::try_from(seen.combo).unwrap_or(i32::MAX), at_us: now_us });
        if let Some(side) = self.sides.get_mut(region) {
            *side = Some(ShownJudgement { judge: judgement, timing_ms: seen.fast_us / MICROS_PER_MILLI });
        }
    }

    /// Makes sure the tables of the chart's seconds have a row for every second of a chart that
    /// runs for `seconds`, notes in it or not.
    pub(crate) fn reach(&mut self, seconds: usize) {
        self.seconds.reach(seconds);
    }

    /// How many notes of each second of the chart took each judgement, the first class being the
    /// notes not judged yet, and the same with the judgements that came late counted apart.
    pub(crate) fn seconds(&self) -> (&[[u32; JUDGE_CLASSES]], &[[u32; TIMING_CLASSES]]) {
        (&self.seconds.by_judge, &self.seconds.by_timing)
    }

    /// What each judge region last reported.
    pub(crate) fn regions(&self) -> JudgeFrame {
        self.regions
    }

    /// The last judgement of each side.
    pub(crate) fn sides(&self) -> [Option<ShownJudgement>; PLAYER_SIDES] {
        self.sides
    }

    /// The state of the note whose head is in `lane` at `time_us`, or "not judged" for a note the
    /// trace does not know.
    fn head_state(&self, lane: usize, time_us: i64) -> u8 {
        self.lanes.get(lane).map_or(NOTE_UNJUDGED, |lane| state_at(&lane.heads, time_us))
    }

    /// The state of the long note end in `lane` at `time_us`, on the same terms.
    fn end_state(&self, lane: usize, time_us: i64) -> u8 {
        self.lanes.get(lane).map_or(NOTE_UNJUDGED, |lane| state_at(&lane.ends, time_us))
    }

    /// Whether anything has judged the note whose head is in `lane` at `time_us`.
    pub(crate) fn is_judged(&self, lane: usize, time_us: i64) -> bool {
        self.head_state(lane, time_us) != NOTE_UNJUDGED
    }
}

/// The state kept for the note at `time_us` in a list in chart order.
fn state_at(list: &[Kept], time_us: i64) -> u8 {
    list.binary_search_by_key(&time_us, |note| note.time_us).map_or(NOTE_UNJUDGED, |index| list[index].state)
}

impl NoteStates for RunTrace {
    fn state(&self, lane: usize, time_us: i64) -> u8 {
        self.head_state(lane, time_us)
    }
}

/// One long note of a chart, as the play screen keeps it for the lane it is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LongSpan {
    /// When its head and its end are reached, in microseconds of chart time.
    pub(crate) head_us: i64,
    pub(crate) end_us: i64,
}

/// The last state of a long note's end that still says the end was taken: a good
/// (`getPair().getState() <= 3`).
const LAST_TAKEN_END_STATE: u8 = 3;

/// How one lane stands with the long notes it has in hand, as the engine and the screen know it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LaneHand {
    /// The long notes the engine says the lane has in hand.
    pub(crate) held: LaneLongNote,
    /// Whether any of the lane's keys is down.
    pub(crate) down: bool,
    /// Whether the game plays the chart by itself.
    pub(crate) autoplay: bool,
}

/// The long note a lane has in hand, as the note field and the lane's timers read it.
///
/// Which note the lane holds and which hell charge note is going by are the engine's own account
/// (`JudgeManager.getProcessingLongNote`, `getPassingLongNote`), so a note taken ahead of its head
/// is held from the press that took it. Whether the note going by is gaining is worked out as the
/// reference works it out each frame (`JudgeManager.java:288-292`): the lane is down, or the note's
/// end has already been taken with a good or better, or the game is playing the chart by itself.
pub(crate) fn lane_long(spans: &[LongSpan], trace: &RunTrace, lane: usize, hand: LaneHand) -> LaneLong {
    let end_taken = |head_us: i64| {
        let end_us = spans.binary_search_by_key(&head_us, |span| span.head_us).ok().map(|index| spans[index].end_us);
        end_us.is_some_and(|end_us| (NOTE_UNJUDGED + 1..=LAST_TAKEN_END_STATE).contains(&trace.end_state(lane, end_us)))
    };
    let increasing = hand.held.passing.is_some_and(|head_us| hand.down || hand.autoplay || end_taken(head_us));
    LaneLong { processing: hand.held.processing, passing: hand.held.passing, increasing }
}

#[cfg(test)]
mod tests;
