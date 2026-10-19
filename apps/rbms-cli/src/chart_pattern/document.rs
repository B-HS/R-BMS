//! The pattern document of one chart: its header facts and its note layout as the chart states it,
//! before any play option has moved a note.
//!
//! The chart is read the way the player and the library scan read it: bytes through
//! `rbms_parser::parse` (`rbms_parser::bmson::parse` for a bmson), `detect_mode`, `to_model`. Only
//! the chart file's own bytes are read, so no keysound and no picture is opened; the `#WAVxx` and
//! `#BMPxx` names stay names on the model.

use std::path::Path;

use rbms_chart::{contains_undefined_long_note, count_playable_notes, detect_mode, to_model};
use rbms_library::songdb::PARSER_VERSION;
use rbms_model::{LnKind, Micros, Mode, Model, NoteKind};
use serde::Serialize;

/// The `format` tag of a pattern document.
pub const FORMAT: &str = "rbms-chart-pattern";

/// The generation of the document's shape. A reader refuses a version it does not know.
pub const VERSION: u32 = 1;

/// Scroll speed a timeline has when the chart never changed it.
const DEFAULT_SCROLL: f64 = 1.0;

const KIND_NORMAL: u8 = 0;
const KIND_LONG_HEAD: u8 = 1;
const KIND_LONG_END: u8 = 2;
const KIND_MINE: u8 = 3;

/// What reading one chart came to.
pub enum Extracted {
    /// The chart is of another mode than the one asked for; only its md5 is kept, so a table's
    /// coverage can tell a chart in another mode from one that is not there.
    OtherMode { md5: String },
    /// The chart's pattern document, serialised.
    Pattern { md5: String, document: Vec<u8> },
}

/// The long-note flavour a chart states, as the document spells it.
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum DeclaredLn {
    Ln,
    Cn,
    Hcn,
}

/// The note columns, parallel arrays of one length, ordered by time and then by lane.
#[derive(Serialize)]
struct Notes {
    time_us: Vec<Micros>,
    lane: Vec<usize>,
    kind: Vec<u8>,
    section: Vec<f64>,
}

/// A point where the tempo, the scroll speed or a stop is stated.
#[derive(Serialize)]
struct TempoPoint {
    t_us: Micros,
    bpm: f64,
    stop_us: Micros,
    scroll: f64,
}

#[derive(Serialize)]
struct Document<'a> {
    format: &'static str,
    version: u32,
    parser_version: u32,
    md5: &'a str,
    sha256: &'a str,
    mode: &'static str,
    declared_ln: Option<DeclaredLn>,
    has_undefined_ln: bool,
    uses_control_flow: bool,
    init_bpm: f64,
    total: f64,
    rank: i32,
    play_level: Option<i64>,
    playable_notes: usize,
    notes: Notes,
    tempo: Vec<TempoPoint>,
}

/// Reads the chart in `bytes`, which came from `path`, into its pattern document when it is of
/// `mode`.
pub fn extract(bytes: &[u8], path: &Path, mode: Mode) -> Result<Extracted, String> {
    if rbms_library::is_bmson(path) {
        let chart = rbms_parser::bmson::parse(bytes).map_err(|e| format!("bmson parse error: {e}"))?;
        if chart.mode() != mode {
            return Ok(Extracted::OtherMode { md5: chart.md5 });
        }
        return pattern_of(&chart.to_model(), false);
    }
    let source = rbms_parser::parse(bytes);
    if detect_mode(&source, &path.to_string_lossy()) != mode {
        return Ok(Extracted::OtherMode { md5: source.md5 });
    }
    pattern_of(&to_model(&source, mode), rbms_library::scan::uses_control_flow(bytes))
}

fn pattern_of(model: &Model, uses_control_flow: bool) -> Result<Extracted, String> {
    let document = Document {
        format: FORMAT,
        version: VERSION,
        parser_version: PARSER_VERSION,
        md5: &model.md5,
        sha256: &model.sha256,
        mode: model.mode.name,
        declared_ln: declared_ln(model),
        has_undefined_ln: contains_undefined_long_note(model),
        uses_control_flow,
        init_bpm: model.init_bpm,
        total: model.meta.total,
        rank: model.meta.rank,
        play_level: model.meta.play_level.trim().parse().ok(),
        playable_notes: count_playable_notes(model),
        notes: notes_of(model),
        tempo: tempo_of(model),
    };
    let bytes = serde_json::to_vec(&document).map_err(|e| format!("pattern not serialised: {e}"))?;
    Ok(Extracted::Pattern { md5: model.md5.clone(), document: bytes })
}

fn kind_code(kind: &NoteKind) -> u8 {
    match kind {
        NoteKind::Normal => KIND_NORMAL,
        NoteKind::LongStart { .. } => KIND_LONG_HEAD,
        NoteKind::LongEnd { .. } => KIND_LONG_END,
        NoteKind::Mine { .. } => KIND_MINE,
    }
}

/// Every note the player is shown: hidden notes are not read, and neither are the background
/// sounds or the notes layered under another.
fn notes_of(model: &Model) -> Notes {
    let lanes = model.mode.key;
    let mut placed: Vec<(Micros, usize, u8, f64)> = model
        .timelines
        .iter()
        .flat_map(|timeline| {
            timeline
                .notes
                .iter()
                .take(lanes)
                .enumerate()
                .filter_map(|(lane, note)| note.as_ref().map(|note| (note.time_us, lane, kind_code(&note.kind), note.section)))
        })
        .collect();
    placed.sort_by_key(|(time_us, lane, ..)| (*time_us, *lane));
    Notes {
        time_us: placed.iter().map(|note| note.0).collect(),
        lane: placed.iter().map(|note| note.1).collect(),
        kind: placed.iter().map(|note| note.2).collect(),
        section: placed.iter().map(|note| note.3).collect(),
    }
}

/// The long-note flavour the chart's long notes carry, which is what the chart stated: a flavour
/// the chart left unstated stays undefined on the model for the player's setting to decide, and
/// reads as `None` here. `None` as well when the chart has no long note.
fn declared_ln(model: &Model) -> Option<DeclaredLn> {
    let lanes = model.mode.key;
    model.timelines.iter().flat_map(|timeline| timeline.notes.iter().take(lanes).flatten()).find_map(|note| match note.kind {
        NoteKind::LongStart { ln } | NoteKind::LongEnd { ln } => match ln {
            LnKind::Ln => Some(DeclaredLn::Ln),
            LnKind::Cn => Some(DeclaredLn::Cn),
            LnKind::Hcn => Some(DeclaredLn::Hcn),
            LnKind::Undefined => None,
        },
        NoteKind::Normal | NoteKind::Mine { .. } => None,
    })
}

/// Where the tempo or the scroll speed differs from the timeline before it, and every stop. A chart
/// that never changes either has none: its tempo is `init_bpm` and its scroll speed is one. A stop
/// is a pause at its point and not a state, so a stop's point stands on its own.
fn tempo_of(model: &Model) -> Vec<TempoPoint> {
    let mut points = Vec::new();
    let (mut bpm, mut scroll) = (model.init_bpm, DEFAULT_SCROLL);
    for timeline in &model.timelines {
        if timeline.bpm != bpm || timeline.scroll != scroll || timeline.stop_us != 0 {
            points.push(TempoPoint { t_us: timeline.time_us, bpm: timeline.bpm, stop_us: timeline.stop_us, scroll: timeline.scroll });
        }
        (bpm, scroll) = (timeline.bpm, timeline.scroll);
    }
    points
}
