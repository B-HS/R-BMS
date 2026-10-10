//! The chart as a skin's note field is shown it under CONSTANT and LEGACY NOTE.
//!
//! Two of this player's settings change what is drawn of a chart and nothing of how it is judged.
//! CONSTANT scrolls the field at one speed whatever the chart's tempo does, and LEGACY NOTE draws
//! every long note as the plain note at its head. The built-in field takes both as flags. A skin's
//! note field is the reference's, which has neither: it draws the timelines it is handed, each at
//! the section the chart gave it.
//!
//! So the screen hands that field timelines made over, the way the reference's own chart options of
//! the same names make a chart over before it is played (`ScrollSpeedModifier` and
//! `LongNoteModifier`, each in its `REMOVE` mode): every timeline at a section that follows from its
//! time alone, with no stop and one tempo, and every long note's head as a plain note with its end
//! gone. The run itself keeps the chart as it was written, and so does everything else the skin
//! reads: the tempo figures, the graphs and the judgements.
//!
//! One thing is this player's own and not the reference's. The reference pins a chart made over
//! this way to the tempo the chart opens at. This player's CONSTANT is pinned to no tempo of the
//! chart: a note takes [`CONSTANT_GREEN_BASE_MS`] over the hi-speed to cross the field, on a skin's
//! field as on the built-in one.

use rbms_chart::scroll::CONSTANT_GREEN_BASE_MS;
use rbms_model::{Note, NoteKind, TimeLine};

use crate::skin_host::play::shown::LANE_TRAVEL_BASE_MS;

#[cfg(test)]
mod tests;

/// The tempo a field scrolls at under CONSTANT: the one at which a note crosses the field in
/// [`CONSTANT_GREEN_BASE_MS`] at a hi-speed of one.
const CONSTANT_BPM: f64 = LANE_TRAVEL_BASE_MS / CONSTANT_GREEN_BASE_MS;

/// The scroll rate of a field under CONSTANT: the chart's own rates do not apply.
const CONSTANT_SCROLL: f64 = 1.0;

/// How many microseconds one measure of a field under CONSTANT lasts, which is what turns a time
/// into the section the note field measures its rows by.
const CONSTANT_MEASURE_US: f64 = CONSTANT_GREEN_BASE_MS * super::MICROS_PER_MILLI as f64;

/// A chart's timelines made over for the note field of a skin, and the two settings they were made
/// over for.
#[derive(Debug)]
pub(super) struct FieldChart {
    constant: bool,
    legacy_note: bool,
    timelines: Vec<TimeLine>,
}

impl FieldChart {
    /// The timelines of `chart` as the note field is shown them with the two settings as given, or
    /// `None` with both off, when the field is shown the chart itself.
    pub(super) fn of(chart: &[TimeLine], constant: bool, legacy_note: bool) -> Option<FieldChart> {
        (constant || legacy_note).then(|| FieldChart {
            constant,
            legacy_note,
            timelines: chart.iter().map(|line| shown_line(line, constant, legacy_note)).collect(),
        })
    }

    /// The CONSTANT and LEGACY NOTE settings these timelines were made over for.
    pub(super) fn settings(&self) -> (bool, bool) {
        (self.constant, self.legacy_note)
    }

    pub(super) fn timelines(&self) -> &[TimeLine] {
        &self.timelines
    }

    /// The tempo and the scroll rate the field scrolls at from first to last, when CONSTANT is what
    /// it was made over for. A field made over for LEGACY NOTE alone scrolls as its chart does.
    pub(super) fn tempo(&self) -> Option<(f64, f64)> {
        self.constant.then_some((CONSTANT_BPM, CONSTANT_SCROLL))
    }
}

/// One timeline as the note field is shown it. The sounds a timeline plays by itself and the
/// pictures it puts up are no part of the field and are left behind.
fn shown_line(line: &TimeLine, constant: bool, legacy_note: bool) -> TimeLine {
    TimeLine {
        time_us: line.time_us,
        section: if constant { line.time_us as f64 / CONSTANT_MEASURE_US } else { line.section },
        notes: if legacy_note { line.notes.iter().map(|note| note.as_ref().and_then(plain_note)).collect() } else { line.notes.clone() },
        hidden: line.hidden.clone(),
        bgnotes: Vec::new(),
        section_line: line.section_line,
        bpm: if constant { CONSTANT_BPM } else { line.bpm },
        stop_us: if constant { 0 } else { line.stop_us },
        scroll: if constant { CONSTANT_SCROLL } else { line.scroll },
        bga: line.bga,
        layer: line.layer,
    }
}

/// A note as LEGACY NOTE draws it: the head of a long note as a plain note, its end not at all, and
/// every other note as it is.
fn plain_note(note: &Note) -> Option<Note> {
    match note.kind {
        NoteKind::LongEnd { .. } => None,
        NoteKind::LongStart { .. } => Some(Note { kind: NoteKind::Normal, ..note.clone() }),
        NoteKind::Normal | NoteKind::Mine { .. } => Some(note.clone()),
    }
}
