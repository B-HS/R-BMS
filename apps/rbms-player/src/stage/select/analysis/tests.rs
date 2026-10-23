//! The browser's chart analysis: what a parse of the chart under the cursor gives the chart cluster
//! and the graphs, which request is answered when the cursor moves on, and where the radar comes
//! from. Every test waits for the worker on its channel, never on a clock.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use rbms_library::songdb::{SongDb, SongRow, mode_id};

use super::*;
use crate::stage::select::list::tests::entry;

/// Keeps one test's charts clear of another's when the suite runs them in one process.
static FIXTURE_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// A seven-key chart: two notes on the first key, one on the sixth and one on the turntable.
const SEVEN_KEY_CHART: &str = "#PLAYER 1\n#TITLE Seven\n#BPM 150\n#00111:0101\n#00118:01\n#00116:01\n";

/// A five-key chart with a single note on the first key.
const FIVE_KEY_CHART: &str = "#PLAYER 1\n#TITLE Five\n#BPM 150\n#00111:01\n";

/// A nine-key chart, which has neither lane counts nor a radar.
const NINE_KEY_CHART: &str = "#PLAYER 1\n#TITLE Nine\n#BPM 150\n#00111:01\n";

/// The lanes of a seven-key chart, and the first key's and the turntable's among them.
const SEVEN_KEY_LANES: usize = 8;
const FIRST_KEY: usize = 0;
const SIXTH_KEY: usize = 5;
const TURNTABLE: usize = 7;

/// A folder of chart files of one test's own, removed with it.
struct Charts {
    root: PathBuf,
}

impl Charts {
    fn new(tag: &str) -> Charts {
        let seq = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("rbms-select-analysis-{tag}-{}-{seq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create the chart folder");
        Charts { root }
    }

    /// The library entry of a chart written into the folder as `name`.
    fn write(&self, name: &str, body: &str, mode: Mode) -> SongEntry {
        let path = self.root.join(name);
        std::fs::write(&path, body).expect("write the chart");
        SongEntry { path, mode, ..entry(name, "Artist", "1") }
    }

    /// The library entry of a chart the folder does not hold.
    fn missing(&self, name: &str) -> SongEntry {
        SongEntry { path: self.root.join(name), ..entry(name, "Artist", "1") }
    }
}

impl Drop for Charts {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A song database listing `entries`.
fn listing(entries: &[&SongEntry]) -> SongDb {
    let mut db = SongDb::open_in_memory().expect("open the in-memory database");
    db.migrate().expect("apply the schema");
    let rows: Vec<SongRow> = entries
        .iter()
        .map(|entry| SongRow { path: rbms_library::scan::normalize(&entry.path), title: entry.title.clone(), mode: mode_id(entry.mode), ..SongRow::default() })
        .collect();
    db.upsert_batch(&rows).expect("store the charts");
    db
}

/// The analysis of `entry`, at place 0 of the library, once the worker has answered.
fn settled_on(db: Option<&SongDb>, entry: &SongEntry) -> ChartAnalysis {
    let mut analysis = ChartAnalysis::default();
    analysis.follow_focus(db, Some((0, entry)));
    analysis.settle(db);
    analysis
}

#[test]
fn a_rested_chart_is_counted_lane_by_lane_and_by_kind_and_graphed() {
    let charts = Charts::new("rested");
    let seven = charts.write("seven.bms", SEVEN_KEY_CHART, Mode::BEAT_7K);
    let analysis = settled_on(None, &seven);

    let chart = analysis.describe(ChartMeta::of_entry(&seven));
    let lanes = chart.lane_notes.expect("a seven-key chart is counted lane by lane");
    assert_eq!((0..SEVEN_KEY_LANES).filter_map(|lane| lanes.on(lane)).sum::<i32>(), 4);
    assert_eq!((lanes.on(FIRST_KEY), lanes.on(SIXTH_KEY), lanes.on(TURNTABLE)), (Some(2), Some(1), Some(1)));
    let counts = chart.note_counts.expect("the kinds are counted");
    assert_eq!((counts.normal, counts.scratch), (3, 1));
    assert_eq!(chart.main_bpm, Some(150.0));
    let radar = chart.notes_radar.expect("a seven-key chart has a radar");
    assert!(radar.iter().all(|value| (0.0..=rbms_chart::RADAR_DISPLAY_MAX).contains(value)), "{radar:?}");
    assert_eq!(analysis.notes_radar(), Some(radar), "the picture and the numbers show the same radar");

    let series = analysis.series();
    assert!(series.notes.is_some_and(|notes| !notes.kinds.is_empty()), "the judgement graph has the chart's seconds");
    assert!(series.bpm.is_some(), "and the tempo graph its changes");
}

#[test]
fn only_the_newest_request_is_answered() {
    let charts = Charts::new("newest");
    let seven = charts.write("seven.bms", SEVEN_KEY_CHART, Mode::BEAT_7K);
    let five = charts.write("five.bms", FIVE_KEY_CHART, Mode::BEAT_5K);
    let mut analysis = ChartAnalysis::default();

    analysis.follow_focus(None, Some((0, &seven)));
    analysis.follow_focus(None, Some((1, &five)));
    analysis.settle(None);

    let lanes = analysis.describe(ChartMeta::of_entry(&five)).lane_notes.expect("the five-key chart was parsed");
    assert_eq!((lanes.on(FIRST_KEY), lanes.on(TURNTABLE)), (Some(1), None), "five keys and a turntable are six lanes");
}

#[test]
fn a_cursor_that_has_not_rested_shows_nothing_of_the_chart_it_left() {
    let charts = Charts::new("moving");
    let seven = charts.write("seven.bms", SEVEN_KEY_CHART, Mode::BEAT_7K);
    let mut analysis = settled_on(None, &seven);

    analysis.follow_focus(None, None);
    let chart = analysis.describe(ChartMeta::of_entry(&seven));
    assert_eq!((chart.lane_notes, chart.notes_radar, chart.note_counts), (None, None, None));
    assert!(analysis.series().notes.is_none());

    analysis.follow_focus(None, Some((0, &seven)));
    assert!(analysis.describe(ChartMeta::of_entry(&seven)).lane_notes.is_some(), "coming back to it shows it again without a new parse");
}

#[test]
fn the_radar_the_database_holds_is_shown_for_a_chart_that_cannot_be_read() {
    let charts = Charts::new("stored");
    let gone = charts.missing("gone.bms");
    let db = listing(&[&gone]);
    let stored = RadarRow { version: rbms_chart::RADAR_VERSION, raw: Some(rbms_chart::RadarRaw { notes: 10.0, ..rbms_chart::RadarRaw::default() }) };
    crate::library::keep_radar(&db, &gone.path, &stored);

    let mut analysis = ChartAnalysis::default();
    analysis.follow_focus(Some(&db), Some((0, &gone)));
    let expected = stored.raw.map(|raw| radar_display(&raw, &gone.mode));
    assert_eq!(analysis.notes_radar(), expected, "the stored radar is there before the parse");

    analysis.settle(Some(&db));
    let chart = analysis.describe(ChartMeta::of_entry(&gone));
    assert_eq!(chart.notes_radar, expected, "and after a parse that found no chart");
    assert_eq!(chart.lane_notes, None);
    assert!(analysis.series().notes.is_none());
}

#[test]
fn a_radar_the_database_did_not_hold_is_measured_and_kept() {
    let charts = Charts::new("kept");
    let seven = charts.write("seven.bms", SEVEN_KEY_CHART, Mode::BEAT_7K);
    let db = listing(&[&seven]);
    assert_eq!(crate::library::stored_radar(&db, &seven.path), None);

    let analysis = settled_on(Some(&db), &seven);
    let kept = crate::library::stored_radar(&db, &seven.path).expect("the measured radar is kept");
    assert_eq!(kept.raw.map(|raw| radar_display(&raw, &seven.mode)), analysis.notes_radar());
}

#[test]
fn a_nine_key_chart_has_no_lane_counts_and_no_radar() {
    let charts = Charts::new("nine");
    let nine = charts.write("nine.pms", NINE_KEY_CHART, Mode::POPN_9K);
    let analysis = settled_on(None, &nine);

    let chart = analysis.describe(ChartMeta::of_entry(&nine));
    assert_eq!((chart.lane_notes, chart.notes_radar), (None, None));
    assert!(chart.note_counts.is_some(), "its notes are still counted by kind");
    assert!(analysis.series().notes.is_some_and(|notes| notes.popn), "and graphed in the pop'n colours");
}
