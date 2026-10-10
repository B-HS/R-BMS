//! What a parsed chart says about itself before a note of it is played.
//!
//! The reference keeps the model of the chart that was picked in its player resource, and every
//! screen that follows reads from it: the decide screen's texts and numbers from the `SongData`
//! made of it (`PlayerResource.setBMSFile`), its judgement graph and its tempo graph from the model
//! itself (`SkinNoteDistributionGraph.updateGraph`, `SkinBPMGraph.updateGraph(BMSModel)`). A
//! `SongData` made that way has no `SongInformation` behind it, so the densities and the main tempo
//! a browser reads from one have no value here.
//!
//! The note counts by kind are the exception. A chart picked in the browser keeps the browser's own
//! `SongData` through the screens that follow, `SongInformation` and all
//! (`PlayerResource.setBMSFile` keeps a `songdata` it was handed), and a play skin shows those four
//! counts while the chart loads. They are counted from the model here, as `SongInformation` counts
//! them from the same model.
//!
//! [`ChartOverview`] is that reading, taken once from the model as it was parsed and before any
//! lane option moved a note: an owned snapshot the screens lend to the chart cluster
//! ([`ChartOverview::meta`]) and to the graphs ([`ChartOverview::series`]).

use rbms_model::{LnKind, Mode, Model, NoteKind, TimeLine};
use rbms_render::skin_render::graphs::NOTE_KINDS;
use rbms_render::{BpmTimeline, FrameSeries, NoteDistribution};
use rbms_skin::timer::MICROS_PER_MILLI;

use super::chart::{BpmRange, ChartContents, ChartMeta, NoteCounts};

/// Microseconds in the second the judgement graph counts a chart's notes by.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// How far into a lane group's three classes each kind of note sits: the end of a long note, its
/// body, and a plain note (`SkinNoteDistributionGraph.updateData`).
const CLASS_LONG_END: usize = 0;
const CLASS_LONG_BODY: usize = 1;
const CLASS_NOTE: usize = 2;

/// Where the classes of the keys begin; the turntable's begin at zero.
const KEY_CLASSES: usize = 3;

/// The class every mine is counted in, whichever lane it is on.
const CLASS_MINE: usize = 6;

/// Everything a screen shows about a chart that has been parsed and not yet played.
#[derive(Debug, Clone, Default)]
pub struct ChartOverview {
    pub title: String,
    pub subtitle: String,
    pub genre: String,
    pub artist: String,
    pub subartist: String,
    pub md5: String,
    pub sha256: String,
    /// The `#PLAYLEVEL` as a number; 0 when it does not parse.
    pub level: i32,
    /// The `#DIFFICULTY` slot.
    pub difficulty: i32,
    pub mode: Option<Mode>,
    /// The chart's `#RANK`.
    pub judge: i32,
    /// The chart's `#TOTAL`, or nothing above zero when the chart states none.
    pub total: f64,
    /// How many notes there are to judge.
    pub notes: usize,
    /// The same notes by kind: plain and long, on the keys and on the turntable.
    pub note_counts: NoteCounts,
    /// When the last thing in the chart happens, in milliseconds (`BMSModel.getLastTime`).
    pub length_ms: i32,
    /// The slowest and the fastest tempo the chart states, the opening one included.
    pub min_bpm: f64,
    pub max_bpm: f64,
    /// The tempo the most notes are played at, or zero for a chart with no note to count.
    pub main_bpm: f64,
    pub has_long_note: bool,
    pub has_bga: bool,
    pub has_stop: bool,
    /// The file the chart names as its stage image, as the chart wrote it.
    pub stagefile: String,
    /// How many notes of each kind each second of the chart holds.
    pub kinds: Vec<[u32; NOTE_KINDS]>,
    /// Every change of speed as `(speed, time in milliseconds)`, opening speed first.
    pub speeds: Vec<(f64, f64)>,
}

impl ChartOverview {
    /// Reads `model` as the reference's screens do between a chart being picked and its first note.
    pub fn of_model(model: &Model) -> ChartOverview {
        let meta = &model.meta;
        let last_us = last_time_us(model);
        let (speeds, main_bpm) = speed_changes(model);
        let tempos = || std::iter::once(model.init_bpm).chain(model.timelines.iter().map(|line| line.bpm));
        let notes = || model.timelines.iter().flat_map(|line| line.notes.iter().flatten());
        ChartOverview {
            title: meta.title.clone(),
            subtitle: meta.subtitle.clone(),
            genre: meta.genre.clone(),
            artist: meta.artist.clone(),
            subartist: meta.subartist.clone(),
            md5: model.md5.clone(),
            sha256: model.sha256.clone(),
            level: meta.play_level.trim().parse().unwrap_or_default(),
            difficulty: meta.difficulty,
            mode: Some(model.mode),
            judge: meta.rank,
            total: meta.total,
            notes: rbms_chart::count_playable_notes(model),
            note_counts: note_counts(model),
            length_ms: millis_of(last_us),
            min_bpm: tempos().fold(f64::INFINITY, f64::min),
            max_bpm: tempos().fold(f64::NEG_INFINITY, f64::max),
            main_bpm,
            has_long_note: notes().any(|note| matches!(note.kind, NoteKind::LongStart { .. })),
            has_bga: model.bgamap.iter().any(|name| !name.trim().is_empty()),
            has_stop: model.timelines.iter().any(|line| line.stop_us > 0),
            stagefile: meta.stagefile.clone(),
            kinds: note_kinds(model, last_us),
            speeds,
        }
    }

    /// The chart as the chart cluster reads it. `stagefile_loaded` says whether the stage image is
    /// on hand, which is what the reference's stage image options ask.
    ///
    /// The back image and the banner are never on hand here, so their options read as they do for a
    /// chart that has neither.
    pub fn meta(&self, stagefile_loaded: bool) -> ChartMeta<'_> {
        ChartMeta {
            title: &self.title,
            subtitle: &self.subtitle,
            genre: &self.genre,
            artist: &self.artist,
            subartist: &self.subartist,
            md5: &self.md5,
            sha256: &self.sha256,
            level: self.level,
            difficulty: self.difficulty,
            mode: self.mode,
            judge: Some(self.judge),
            length_ms: Some(self.length_ms),
            notes: i32::try_from(self.notes).ok(),
            note_counts: Some(self.note_counts),
            bpm: Some(BpmRange { min: self.min_bpm as i32, max: self.max_bpm as i32 }),
            total: Some(self.total).filter(|total| *total > 0.0),
            contents: ChartContents {
                bga: Some(self.has_bga),
                long_note: Some(self.has_long_note),
                bpm_stop: Some(self.has_stop),
                stagefile: Some(stagefile_loaded),
                banner: Some(false),
                backbmp: Some(false),
                ..ChartContents::default()
            },
            ..ChartMeta::default()
        }
    }

    /// The two series a chart's graphs plot: its notes by kind for the judgement graph, and its
    /// changes of speed for the tempo graph.
    pub fn series(&self) -> FrameSeries<'_> {
        FrameSeries {
            notes: Some(NoteDistribution { kinds: &self.kinds, popn: self.mode == Some(Mode::POPN_9K), ..NoteDistribution::default() }),
            bpm: Some(BpmTimeline::of_chart(&self.speeds, self.main_bpm, self.min_bpm, self.max_bpm, Some(self.length_ms))),
            ..FrameSeries::default()
        }
    }
}

/// A time on the chart in the whole milliseconds the reference keeps it in.
fn millis_of(time_us: i64) -> i32 {
    i32::try_from(time_us / MICROS_PER_MILLI).unwrap_or(i32::MAX)
}

/// Which second of the chart a time falls in.
fn second_of(time_us: i64) -> usize {
    usize::try_from(time_us / MICROS_PER_SECOND).unwrap_or_default()
}

/// Whether anything at all happens on a line: a note, a hidden note, a background sound or a change
/// of picture.
fn has_content(line: &TimeLine) -> bool {
    line.notes.iter().chain(&line.hidden).any(Option::is_some) || !line.bgnotes.is_empty() || line.bga >= 0 || line.layer >= 0
}

/// When the last line with anything on it is reached (`BMSModel.getLastMilliTime`).
fn last_time_us(model: &Model) -> i64 {
    model.timelines.iter().rev().find(|line| has_content(line)).map_or(0, |line| line.time_us)
}

/// Whether a note is one the player is judged on: every plain note and every long note's head, and
/// a long note's end when it is judged apart from its head. A mine is not.
fn is_judged(kind: &NoteKind) -> bool {
    match kind {
        NoteKind::Mine { .. } => false,
        NoteKind::LongEnd { ln } => matches!(ln, LnKind::Cn | LnKind::Hcn),
        NoteKind::Normal | NoteKind::LongStart { .. } => true,
    }
}

/// How many notes a line gives the player to judge (`TimeLine.getTotalNotes`).
fn judged_notes(line: &TimeLine) -> usize {
    line.notes.iter().flatten().filter(|note| is_judged(&note.kind)).count()
}

/// How many notes of each kind a chart gives the player to judge, as `SongInformation` counts them
/// (`BMSModelUtils.getTotalNotes` asked for the plain and the long notes of the keys and of the
/// turntable): the four add up to the chart's notes.
fn note_counts(model: &Model) -> NoteCounts {
    let mut counts = NoteCounts::default();
    for line in &model.timelines {
        for (lane, note) in line.notes.iter().enumerate().take(model.mode.key) {
            let Some(note) = note.as_ref().filter(|note| is_judged(&note.kind)) else {
                continue;
            };
            let count = match (model.mode.is_scratch(lane), &note.kind) {
                (false, NoteKind::Normal) => &mut counts.normal,
                (false, _) => &mut counts.long,
                (true, NoteKind::Normal) => &mut counts.scratch,
                (true, _) => &mut counts.long_scratch,
            };
            *count += 1;
        }
    }
    counts
}

/// How many notes of each kind each second of the chart holds, up to the second of its last line
/// (`SkinNoteDistributionGraph.updateData`, the note-kind graph).
///
/// A long note counts as a body in every second from its start to its end. One whose end is not
/// judged apart from it trades the body of its last second for an end.
fn note_kinds(model: &Model, last_us: i64) -> Vec<[u32; NOTE_KINDS]> {
    let mut seconds = vec![[0u32; NOTE_KINDS]; second_of(last_us) + 1];
    let mut started: Vec<Option<usize>> = vec![None; model.mode.key];
    for line in &model.timelines {
        let second = second_of(line.time_us);
        if second >= seconds.len() {
            break;
        }
        for (lane, note) in line.notes.iter().enumerate().take(model.mode.key) {
            let Some(note) = note else {
                continue;
            };
            let classes = if model.mode.is_scratch(lane) { 0 } else { KEY_CLASSES };
            match note.kind {
                NoteKind::Normal => seconds[second][classes + CLASS_NOTE] += 1,
                NoteKind::Mine { .. } => seconds[second][CLASS_MINE] += 1,
                NoteKind::LongStart { .. } => started[lane] = Some(second),
                NoteKind::LongEnd { ln } => {
                    let Some(start) = started[lane].take() else {
                        continue;
                    };
                    for spanned in &mut seconds[start..=second] {
                        spanned[classes + CLASS_LONG_BODY] += 1;
                    }
                    if matches!(ln, LnKind::Ln | LnKind::Undefined) {
                        seconds[second][classes + CLASS_LONG_END] += 1;
                        seconds[second][classes + CLASS_LONG_BODY] -= 1;
                    }
                }
            }
        }
    }
    seconds
}

/// Every change of speed as `(speed, time in milliseconds)`, and the tempo the most notes are played
/// at (`SkinBPMGraph.updateGraph(BMSModel)`).
///
/// A speed is the tempo times the scroll rate, and zero on a line the chart stops at. The list
/// opens with the chart's first tempo at time zero and closes on its last line. The main tempo of a
/// chart with no note to count is zero, which is what leaves its graph empty.
fn speed_changes(model: &Model) -> (Vec<(f64, f64)>, f64) {
    let mut speed = model.init_bpm;
    let mut changes = vec![(speed, 0.0)];
    let mut notes_by_tempo: Vec<(f64, usize)> = Vec::new();
    for line in &model.timelines {
        match notes_by_tempo.iter_mut().find(|(tempo, _)| *tempo == line.bpm) {
            Some((_, count)) => *count += judged_notes(line),
            None => notes_by_tempo.push((line.bpm, judged_notes(line))),
        }
        let next = if line.stop_us > 0 { 0.0 } else { line.bpm * line.scroll };
        if speed != next {
            speed = next;
            changes.push((speed, f64::from(millis_of(line.time_us))));
        }
    }
    if let Some(last) = model.timelines.last() {
        let end = f64::from(millis_of(last.time_us));
        if changes.last().is_some_and(|(_, time)| *time != end) {
            changes.push((speed, end));
        }
    }
    let mut main = (0.0, 0);
    for (tempo, count) in notes_by_tempo {
        if count > main.1 {
            main = (tempo, count);
        }
    }
    (changes, main.0)
}

#[cfg(test)]
mod tests;
