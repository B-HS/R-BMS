use rbms_model::{LnKind, Mode, Model, ModelMeta, Note, NoteKind, TimeLine};
use rbms_skin::dst::DrawStateSource;
use rbms_skin::property::SkinHost;
use rbms_skin::property::generated::{
    NUMBER_MAXBPM, NUMBER_MINBPM, NUMBER_PLAYLEVEL, NUMBER_TOTALNOTE_BSS, NUMBER_TOTALNOTE_LN, NUMBER_TOTALNOTE_NORMAL, NUMBER_TOTALNOTE_SCRATCH,
    NUMBER_TOTALNOTES, OPTION_DIFFICULTY0, OPTION_DIFFICULTY4, OPTION_NO_STAGEFILE, OPTION_STAGEFILE, STRING_ARTIST, STRING_TITLE,
};
use rbms_skin::timer::TimerState;

use super::*;
use crate::skin_host::ScreenHost;
use crate::skin_host::chart::ChartState;

/// The mode the fixtures are written in: seven keys on lanes 0 to 6 and the turntable on lane 7.
const MODE: Mode = Mode::BEAT_7K;
const KEY_LANE: usize = 0;
const OTHER_KEY_LANE: usize = 1;
const SCRATCH_LANE: usize = 7;

/// The tempo the fixtures open on.
const OPENING_BPM: f64 = 120.0;

const SECOND_US: i64 = MICROS_PER_SECOND;

/// A line `seconds` into the chart at `bpm`, with nothing on it.
fn line(seconds: i64, bpm: f64) -> TimeLine {
    TimeLine::empty(MODE.key, seconds * SECOND_US, 0.0, bpm)
}

/// The same line with one note of `kind` on `lane`.
fn with_note(mut line: TimeLine, lane: usize, kind: NoteKind) -> TimeLine {
    line.notes[lane] = Some(Note { kind, ..Note::normal(1, line.time_us, line.section) });
    line
}

fn model(timelines: Vec<TimeLine>) -> Model {
    Model {
        mode: MODE,
        meta: ModelMeta::default(),
        wavmap: Vec::new(),
        bgamap: Vec::new(),
        init_bpm: OPENING_BPM,
        timelines,
        md5: String::new(),
        sha256: String::new(),
    }
}

/// The classes of one second of the note-kind graph, in its own order: the turntable's long note
/// end, body and plain note, the keys' three, then mines.
fn kinds(scratch: [u32; 3], keys: [u32; 3], mines: u32) -> [u32; NOTE_KINDS] {
    [scratch[0], scratch[1], scratch[2], keys[0], keys[1], keys[2], mines]
}

/// A plain note is counted in the second it falls in, on the turntable's classes or the keys', and
/// a mine in a class of its own. The rows run to the second of the chart's last line and no further.
#[test]
fn plain_notes_and_mines_are_counted_by_second_and_by_lane_group() {
    let chart = model(vec![
        with_note(with_note(line(0, OPENING_BPM), KEY_LANE, NoteKind::Normal), SCRATCH_LANE, NoteKind::Normal),
        with_note(line(0, OPENING_BPM), OTHER_KEY_LANE, NoteKind::Normal),
        with_note(line(2, OPENING_BPM), KEY_LANE, NoteKind::Mine { damage: 1.0 }),
        line(9, OPENING_BPM),
    ]);
    let overview = ChartOverview::of_model(&chart);

    assert_eq!(overview.kinds, vec![kinds([0, 0, 1], [0, 0, 2], 0), kinds([0; 3], [0; 3], 0), kinds([0; 3], [0; 3], 1)]);
    assert_eq!(overview.length_ms, 2000, "a line with nothing on it is not where the chart ends");
    assert_eq!(overview.notes, 3, "a mine is not a note to judge");
}

/// A long note is a body in every second it spans. One whose end is judged with it gives up the
/// body of its last second for an end; a charge note, whose end is a note of its own, keeps it.
#[test]
fn a_long_note_is_a_body_through_every_second_it_spans() {
    let spanning = |ln: LnKind| {
        let chart = model(vec![
            with_note(line(1, OPENING_BPM), KEY_LANE, NoteKind::LongStart { ln }),
            with_note(with_note(line(3, OPENING_BPM), KEY_LANE, NoteKind::LongEnd { ln }), SCRATCH_LANE, NoteKind::LongStart { ln }),
            with_note(line(4, OPENING_BPM), SCRATCH_LANE, NoteKind::LongEnd { ln }),
        ]);
        ChartOverview::of_model(&chart)
    };

    let plain = spanning(LnKind::Ln);
    assert_eq!(
        plain.kinds,
        vec![kinds([0; 3], [0; 3], 0), kinds([0; 3], [0, 1, 0], 0), kinds([0; 3], [0, 1, 0], 0), kinds([0, 1, 0], [1, 0, 0], 0), kinds([1, 0, 0], [0; 3], 0)]
    );
    assert_eq!(plain.notes, 2);
    assert!(plain.has_long_note);

    let charge = spanning(LnKind::Cn);
    assert_eq!(charge.kinds[3], kinds([0, 1, 0], [0, 1, 0], 0), "a charge note's last second lost its body");
    assert_eq!(charge.kinds[4], kinds([0, 1, 0], [0; 3], 0));
    assert_eq!(charge.notes, 4, "the end of a charge note is judged on its own");
}

/// The tempo line opens on the chart's first tempo at time zero, adds a point wherever the speed
/// changes -- a new tempo, a stop, a scroll rate -- and closes on the chart's last line.
#[test]
fn the_speed_changes_follow_tempo_stops_and_scroll() {
    let mut stopped = line(3, 240.0);
    stopped.stop_us = SECOND_US;
    let mut slowed = line(4, 240.0);
    slowed.scroll = 0.5;
    let chart = model(vec![
        with_note(line(0, OPENING_BPM), KEY_LANE, NoteKind::Normal),
        with_note(with_note(line(2, 240.0), KEY_LANE, NoteKind::Normal), OTHER_KEY_LANE, NoteKind::Normal),
        stopped,
        slowed,
        with_note(line(6, 240.0), KEY_LANE, NoteKind::Normal),
        with_note(line(8, 240.0), KEY_LANE, NoteKind::Normal),
    ]);
    let overview = ChartOverview::of_model(&chart);

    assert_eq!(overview.speeds, vec![(120.0, 0.0), (240.0, 2000.0), (0.0, 3000.0), (120.0, 4000.0), (240.0, 6000.0), (240.0, 8000.0)]);
    assert_eq!(overview.main_bpm, 240.0, "the tempo most notes are played at");
    assert_eq!((overview.min_bpm, overview.max_bpm), (120.0, 240.0));
    assert!(overview.has_stop);
    assert_eq!(overview.length_ms, 8000);
}

/// A chart with no note to count has no main tempo, which is what leaves the reference's tempo graph
/// empty for it.
#[test]
fn a_chart_with_no_notes_has_no_main_tempo() {
    let overview = ChartOverview::of_model(&model(vec![line(0, OPENING_BPM)]));
    assert_eq!(overview.main_bpm, 0.0);
    assert_eq!(overview.speeds, vec![(120.0, 0.0)]);
    assert_eq!(overview.kinds.len(), 1);
}

/// The chart cluster answers from the overview what the reference's decide screen reads from its
/// `SongData`: the texts, the level, the difficulty slot, the note count, the tempo range and whether
/// the stage image is on hand.
#[test]
fn the_chart_cluster_reads_the_overview_as_a_decide_screen_reads_its_chart() {
    let mut chart = model(vec![with_note(line(0, OPENING_BPM), KEY_LANE, NoteKind::Normal), with_note(line(1, 180.0), KEY_LANE, NoteKind::Normal)]);
    chart.meta =
        ModelMeta { title: "Picked".into(), artist: "Composer".into(), play_level: " 12 ".into(), difficulty: 4, total: 300.0, ..ModelMeta::default() };
    let overview = ChartOverview::of_model(&chart);

    let timers = TimerState::new();
    let read = |stagefile_loaded: bool, id: i32| {
        let meta = overview.meta(stagefile_loaded);
        let mut host = ScreenHost::new(0, &timers);
        host.chart = ChartState::Chart(&meta);
        (host.boolean(id), host.integer(id), host.text(id).into_owned())
    };

    assert_eq!(read(false, STRING_TITLE).2, "Picked");
    assert_eq!(read(false, STRING_ARTIST).2, "Composer");
    assert_eq!(read(false, NUMBER_PLAYLEVEL).1, 12);
    assert_eq!(read(false, NUMBER_TOTALNOTES).1, 2);
    assert_eq!((read(false, NUMBER_MINBPM).1, read(false, NUMBER_MAXBPM).1), (120, 180));
    assert_eq!(read(false, OPTION_DIFFICULTY4).0, Some(true));
    assert_eq!(read(false, OPTION_DIFFICULTY0).0, Some(false));
    assert_eq!((read(false, OPTION_STAGEFILE).0, read(false, OPTION_NO_STAGEFILE).0), (Some(false), Some(true)));
    assert_eq!((read(true, OPTION_STAGEFILE).0, read(true, OPTION_NO_STAGEFILE).0), (Some(true), Some(false)));
}

/// The series handed to the graphs are the overview's own rows and changes, with the chart's
/// length and tempos beside them.
#[test]
fn the_graphs_are_handed_the_overviews_rows_and_changes() {
    let chart = model(vec![with_note(line(0, OPENING_BPM), KEY_LANE, NoteKind::Normal), with_note(line(5, 90.0), SCRATCH_LANE, NoteKind::Normal)]);
    let overview = ChartOverview::of_model(&chart);
    let series = overview.series();

    let notes = series.notes.expect("the judgement graph has its series");
    assert_eq!(notes.kinds, overview.kinds.as_slice());
    assert!(!notes.popn);
    assert!(notes.playing.is_none(), "a chart that is not being played has no cursor");

    let tempo = series.bpm.expect("the tempo graph has its series");
    assert_eq!(tempo.changes, overview.speeds.as_slice());
    assert_eq!((tempo.min_bpm, tempo.max_bpm, tempo.length_ms), (90.0, 120.0, Some(5000)));
    assert!(series.gauge_history.is_none() && series.timing.is_none(), "a chart that was not played has no run to plot");
}

/// The notes are counted by kind as the reference's chart information counts them: plain and long,
/// on the keys and on the turntable. A long note is one count, and two when its end is judged apart
/// from its head; a mine is none. The four add up to the chart's notes, and the chart cluster
/// answers them from the overview.
#[test]
fn the_notes_are_counted_by_kind_and_add_up_to_the_charts_notes() {
    let chart = model(vec![
        with_note(with_note(line(0, OPENING_BPM), KEY_LANE, NoteKind::Normal), SCRATCH_LANE, NoteKind::Normal),
        with_note(with_note(line(1, OPENING_BPM), KEY_LANE, NoteKind::LongStart { ln: LnKind::Ln }), OTHER_KEY_LANE, NoteKind::LongStart { ln: LnKind::Cn }),
        with_note(with_note(line(2, OPENING_BPM), KEY_LANE, NoteKind::LongEnd { ln: LnKind::Ln }), OTHER_KEY_LANE, NoteKind::LongEnd { ln: LnKind::Cn }),
        with_note(with_note(line(3, OPENING_BPM), SCRATCH_LANE, NoteKind::LongStart { ln: LnKind::Ln }), KEY_LANE, NoteKind::Mine { damage: 1.0 }),
        with_note(with_note(line(4, OPENING_BPM), SCRATCH_LANE, NoteKind::LongEnd { ln: LnKind::Ln }), OTHER_KEY_LANE, NoteKind::Normal),
    ]);
    let overview = ChartOverview::of_model(&chart);
    assert_eq!(overview.note_counts, NoteCounts { normal: 2, long: 3, scratch: 1, long_scratch: 1 });
    let counts = overview.note_counts;
    assert_eq!(usize::try_from(counts.normal + counts.long + counts.scratch + counts.long_scratch), Ok(overview.notes), "the kinds do not add up to the notes");

    let timers = TimerState::new();
    let meta = overview.meta(false);
    let mut host = ScreenHost::new(0, &timers);
    host.chart = ChartState::Chart(&meta);
    let read = [NUMBER_TOTALNOTE_NORMAL, NUMBER_TOTALNOTE_LN, NUMBER_TOTALNOTE_SCRATCH, NUMBER_TOTALNOTE_BSS].map(|id| host.integer(id));
    assert_eq!(read, [2, 3, 1, 1]);
}
