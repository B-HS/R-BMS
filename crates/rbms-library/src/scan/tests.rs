use std::path::PathBuf;
use std::sync::atomic::AtomicU32;

use rbms_chart::RADAR_VERSION;

use super::*;
use crate::songdb::{SongDb, mode_from_id};

const US_PER_SECOND: i64 = 1_000_000;

/// Keeps one test's fixture tree clear of another's when the suite runs them in one process.
static FIXTURE_SEQUENCE: AtomicU32 = AtomicU32::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let seq = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("rbms-scan-{tag}-{}-{seq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create the fixture root");
        Fixture { root }
    }

    fn write(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the fixture directory");
        }
        std::fs::write(&path, body).expect("write the fixture chart");
        path
    }

    fn roots(&self) -> Vec<String> {
        vec![self.root.to_string_lossy().into_owned()]
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn db() -> SongDb {
    let mut db = SongDb::open_in_memory().expect("open the in-memory database");
    db.migrate().expect("apply the schema");
    db
}

/// A playable chart with two keys' worth of notes, sized by its title so a rewrite changes the
/// file stamp.
fn chart(title: &str) -> String {
    format!("#TITLE {title}\n#ARTIST tester\n#GENRE test\n#PLAYLEVEL 5\n#RANK 2\n#BPM 120\n#WAV01 kick.wav\n#00111:01010101\n#00113:00010100\n")
}

fn scan(db: &mut SongDb, req: &ScanRequest) -> ScanOutcomeG {
    scan_into(db, req).expect("run the scan")
}

#[test]
fn a_first_scan_parses_every_chart_and_a_second_one_parses_none() {
    let fixture = Fixture::new("incremental");
    const CHARTS: usize = 5;
    for i in 0..CHARTS {
        fixture.write(&format!("song{i}.bms"), &chart(&format!("song {i}")));
    }
    let mut db = db();

    let first = ScanRequest::new(fixture.roots(), false);
    assert_eq!(scan(&mut db, &first), ScanOutcomeG::Completed { upserted: CHARTS, removed: 0 });
    assert_eq!(first.progress.counts(), ScanCounts { found: CHARTS, parsed: CHARTS, skipped: 0, removed: 0 });
    assert_eq!(db.song_count().expect("count the charts"), CHARTS);

    let second = ScanRequest::new(fixture.roots(), false);
    assert_eq!(scan(&mut db, &second), ScanOutcomeG::Completed { upserted: 0, removed: 0 });
    assert_eq!(
        second.progress.counts(),
        ScanCounts { found: CHARTS, parsed: 0, skipped: CHARTS, removed: 0 },
        "an unchanged library is walked but not re-read"
    );
}

#[test]
fn a_rewritten_chart_is_the_only_one_read_again() {
    let fixture = Fixture::new("touch");
    const CHARTS: usize = 4;
    for i in 0..CHARTS {
        fixture.write(&format!("song{i}.bms"), &chart(&format!("song {i}")));
    }
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let touched = fixture.write("song2.bms", &chart("song 2 rewritten with a longer title"));
    let after = ScanRequest::new(fixture.roots(), false);
    assert_eq!(scan(&mut db, &after), ScanOutcomeG::Completed { upserted: 1, removed: 0 });
    assert_eq!(after.progress.counts(), ScanCounts { found: CHARTS, parsed: 1, skipped: CHARTS - 1, removed: 0 });

    let stored = db.song(&normalize(&touched)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.title, "song 2 rewritten with a longer title", "the row carries the new parse");
}

#[test]
fn a_deleted_chart_leaves_the_database_with_the_next_scan() {
    let fixture = Fixture::new("delete");
    const CHARTS: usize = 3;
    for i in 0..CHARTS {
        fixture.write(&format!("song{i}.bms"), &chart(&format!("song {i}")));
    }
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    std::fs::remove_file(fixture.root.join("song1.bms")).expect("delete a chart");
    let after = ScanRequest::new(fixture.roots(), false);
    assert_eq!(scan(&mut db, &after), ScanOutcomeG::Completed { upserted: 0, removed: 1 });
    assert_eq!(after.progress.counts(), ScanCounts { found: CHARTS - 1, parsed: 0, skipped: CHARTS - 1, removed: 1 });
    assert_eq!(db.song_count().expect("count the charts"), CHARTS - 1);
}

#[test]
fn a_full_scan_re_reads_charts_whose_files_never_moved() {
    let fixture = Fixture::new("full");
    fixture.write("song.bms", &chart("song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let full = ScanRequest::new(fixture.roots(), true);
    assert_eq!(scan(&mut db, &full), ScanOutcomeG::Completed { upserted: 1, removed: 0 });
    assert_eq!(full.progress.counts(), ScanCounts { found: 1, parsed: 1, skipped: 0, removed: 0 });
}

/// The parse generation that read every chart as Shift_JIS, so a database it filled holds Korean
/// titles as half-width katakana.
const SHIFT_JIS_ONLY_GENERATION: u32 = 1;

/// `별의 노래` in EUC-KR.
const EUC_KR_TITLE: [u8; 9] = [0xBA, 0xB0, 0xC0, 0xC7, 0x20, 0xB3, 0xEB, 0xB7, 0xA1];

#[test]
fn a_library_scanned_before_euc_kr_was_read_is_read_again_and_keeps_what_is_not_the_parse() {
    let fixture = Fixture::new("generation");
    let mut body = b"#TITLE ".to_vec();
    body.extend_from_slice(&EUC_KR_TITLE);
    body.extend_from_slice(b"\n#ARTIST tester\n#BPM 120\n#WAV01 kick.wav\n#00111:01010101\n");
    let source = fixture.root.join("song.bms");
    std::fs::write(&source, &body).expect("write the fixture chart");
    let path = normalize(&source);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let scanned = db.song(&path).expect("read the chart").expect("the chart is stored");
    let old_generation = SongRow { title: "\u{FF7A}\u{FF70}\u{FF80}\u{FF87}".to_string(), ..scanned.clone() };
    db.upsert_batch(std::slice::from_ref(&old_generation)).expect("store the title the old generation read");
    db.set_favorite(&path, 1).expect("star the chart");
    db.set_parser_version(SHIFT_JIS_ONLY_GENERATION).expect("stamp the old generation");
    let before = db.song(&path).expect("read the chart").expect("the chart is stored");
    assert_eq!(before.title, old_generation.title);

    let full = db.needs_full_rescan().expect("read the generation");
    assert!(full, "rows the Shift_JIS-only generation wrote are not trusted by this one");
    let request = ScanRequest::new(fixture.roots(), full);
    assert_eq!(scan(&mut db, &request), ScanOutcomeG::Completed { upserted: 1, removed: 0 });
    assert_eq!(request.progress.counts(), ScanCounts { found: 1, parsed: 1, skipped: 0, removed: 0 }, "the file never moved and is read all the same");

    let after = db.song(&path).expect("read the chart").expect("the chart is stored");
    assert_eq!(after.title, "별의 노래");
    assert_eq!(
        (after.md5.as_str(), after.sha256.as_str()),
        (before.md5.as_str(), before.sha256.as_str()),
        "the hashes scores are kept under come from the bytes"
    );
    assert_eq!(after.favorite, 1, "a star is the user's and outlives the rescan");
    assert_eq!(after.adddate, before.adddate, "the chart is no newer for having been read again");
    assert_eq!(db.parser_version().expect("read the generation"), Some(PARSER_VERSION));
    assert!(!db.needs_full_rescan().expect("read the generation"), "the next scan is incremental again");
}

/// The generation that counted a normal note inside a long note's body and a head nothing closes.
const UNSETTLED_LONG_NOTE_GENERATION: u32 = 2;

#[test]
fn a_library_scanned_before_long_notes_were_settled_is_read_again_and_gets_the_new_note_count() {
    let fixture = Fixture::new("long-note-generation");
    let source = fixture.write("song.bms", "#TITLE T\n#BPM 120\n#WAV01 a.wav\n#WAV02 b.wav\n#00151:01\n#00211:02\n#00251:01\n#00351:01\n");
    let path = normalize(&source);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let scanned = db.song(&path).expect("read the chart").expect("the chart is stored");
    assert_eq!((scanned.notes, scanned.long_notes), (1, 1), "the note in the body and the head nothing closes are not counted");

    let before_settling = SongRow { notes: 3, long_notes: 2, ..scanned };
    db.upsert_batch(std::slice::from_ref(&before_settling)).expect("store the counts the old generation wrote");
    db.set_parser_version(UNSETTLED_LONG_NOTE_GENERATION).expect("stamp the old generation");

    let full = db.needs_full_rescan().expect("read the generation");
    assert!(full, "rows the generation before long notes were settled wrote are not trusted by this one");
    assert_eq!(scan(&mut db, &ScanRequest::new(fixture.roots(), full)), ScanOutcomeG::Completed { upserted: 1, removed: 0 });

    let after = db.song(&path).expect("read the chart").expect("the chart is stored");
    assert_eq!((after.notes, after.long_notes), (1, 1));
}

#[test]
fn a_scan_cancelled_before_it_starts_reads_nothing_and_deletes_nothing() {
    let fixture = Fixture::new("cancel");
    for i in 0..3 {
        fixture.write(&format!("song{i}.bms"), &chart(&format!("song {i}")));
    }
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    std::fs::remove_file(fixture.root.join("song0.bms")).expect("delete a chart");

    let cancelled = ScanRequest::new(fixture.roots(), false);
    cancelled.cancel.store(true, Ordering::Relaxed);
    assert_eq!(scan(&mut db, &cancelled), ScanOutcomeG::Cancelled);
    assert_eq!(cancelled.progress.counts(), ScanCounts::default(), "a scan cancelled up front does no work at all");
    assert_eq!(db.song_count().expect("count the charts"), 3, "a cancelled scan never deletes: the walk did not get far enough to know");
}

#[test]
fn a_scan_cancelled_after_its_walk_keeps_what_it_had_already_committed() {
    let fixture = Fixture::new("cancel-mid");
    fixture.write("song.bms", &chart("song"));
    let mut db = db();

    let request = ScanRequest::new(fixture.roots(), false);
    let watcher = Arc::clone(&request.cancel);
    let progress = Arc::clone(&request.progress);
    std::thread::spawn(move || {
        while progress.found.load(Ordering::Relaxed) == 0 {
            std::thread::yield_now();
        }
        watcher.store(true, Ordering::Relaxed);
    });
    let outcome = scan(&mut db, &request);
    assert!(
        matches!(outcome, ScanOutcomeG::Cancelled | ScanOutcomeG::Completed { .. }),
        "cancelling mid-scan ends the scan one way or the other rather than hanging"
    );
    assert!(db.needs_full_rescan().expect("read the generation") || matches!(outcome, ScanOutcomeG::Completed { .. }));
}

#[test]
fn a_completed_scan_stamps_the_parse_generation_and_a_cancelled_one_does_not() {
    let fixture = Fixture::new("generation");
    fixture.write("song.bms", &chart("song"));
    let mut db = db();
    assert!(db.needs_full_rescan().expect("read the generation"));

    let cancelled = ScanRequest::new(fixture.roots(), false);
    cancelled.cancel.store(true, Ordering::Relaxed);
    scan(&mut db, &cancelled);
    assert!(db.needs_full_rescan().expect("read the generation"), "a cancelled scan leaves the upgrade rescan pending");

    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    assert!(!db.needs_full_rescan().expect("read the generation"));
}

#[test]
fn a_partial_scan_leaves_the_parser_version_alone() {
    let fixture = Fixture::new("partial-generation");
    fixture.write("downloads/song.bms", &chart("song"));
    let mut db = db();
    assert!(db.needs_full_rescan().expect("read the generation"));

    let partial = ScanRequest::partial(vec![fixture.root.join("downloads").to_string_lossy().into_owned()]);
    assert_eq!(scan(&mut db, &partial), ScanOutcomeG::Completed { upserted: 1, removed: 0 });
    assert!(db.needs_full_rescan().expect("read the generation"), "a scan of one folder took the rescan of the others as done");

    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    assert!(!db.needs_full_rescan().expect("read the generation"), "a scan of the whole library still stamps the generation");
}

#[test]
fn a_partial_scan_removes_only_under_its_folder() {
    let fixture = Fixture::new("partial-removal");
    let downloaded = fixture.write("downloads/gone.bms", &chart("downloaded"));
    let elsewhere = fixture.write("songs/gone.bms", &chart("elsewhere"));
    fixture.write("downloads/kept.bms", &chart("kept"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    std::fs::remove_file(&downloaded).expect("remove the downloaded chart");
    std::fs::remove_file(&elsewhere).expect("remove the other chart");

    let partial = ScanRequest::partial(vec![fixture.root.join("downloads").to_string_lossy().into_owned()]);
    assert_eq!(scan(&mut db, &partial), ScanOutcomeG::Completed { upserted: 0, removed: 1 });
    let mut titles: Vec<String> = db.all_songs().expect("read the rows").into_iter().map(|row| row.title).collect();
    titles.sort();
    assert_eq!(titles, ["elsewhere", "kept"], "a chart gone from outside the folder scanned was dropped, or one gone from inside it kept");
}

#[test]
fn charts_in_nested_folders_are_found_and_carry_their_own_folder() {
    let fixture = Fixture::new("nested");
    fixture.write("pack/deep/song.bms", &chart("deep song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.all_songs().expect("read the charts");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].folder, normalize(&fixture.root.join("pack/deep")));
    assert!(stored[0].path.ends_with("/pack/deep/song.bms"), "paths are stored with forward separators: {}", stored[0].path);
}

/// What a download has half fetched or half unpacked is not a chart of the library, at whatever
/// depth its work folder is, and a chart that was let in from there before is dropped.
#[test]
fn the_folder_a_download_works_in_is_never_walked_into() {
    let fixture = Fixture::new("download-work");
    fixture.write("pack/song.bms", &chart("kept"));
    fixture.write(&format!("{DOWNLOAD_WORK_FOLDER}/unpacked/half.bms"), &chart("half unpacked"));
    fixture.write(&format!("downloads/{DOWNLOAD_WORK_FOLDER}/unpacked/deep/half.bms"), &chart("half unpacked deeper"));
    fixture.write(&format!("downloads/Settled{DOWNLOAD_WORK_FOLDER}/song.bms"), &chart("named alike"));
    let mut db = db();
    let ghost = SongRow { path: normalize(&fixture.root.join(DOWNLOAD_WORK_FOLDER).join("unpacked").join("gone.bms")), ..SongRow::default() };
    db.upsert_batch(std::slice::from_ref(&ghost)).expect("store a row the walk once let in");

    let request = ScanRequest::new(fixture.roots(), false);
    scan(&mut db, &request);
    assert_eq!(request.progress.counts().found, 2, "a chart under the folder a download works in was walked into the scan");
    let mut titles: Vec<String> = db.all_songs().expect("read the charts").into_iter().map(|song| song.title).collect();
    titles.sort();
    assert_eq!(titles, ["kept", "named alike"], "the work folder's charts were stored, or a folder only named like it was skipped");
}

#[test]
fn a_scan_stores_the_stamp_it_read_the_file_at() {
    let fixture = Fixture::new("stamp");
    let path = fixture.write("song.bms", &chart("song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let meta = std::fs::metadata(&path).expect("stat the chart");
    let stamps = db.stamp_index().expect("read the stamps");
    assert_eq!(stamps.get(&normalize(&path)), Some(&(modified_secs(&meta), meta.len() as i64)));
}

#[test]
fn a_file_that_is_not_a_chart_is_not_scanned() {
    let fixture = Fixture::new("not-a-chart");
    fixture.write("song.bms", &chart("song"));
    fixture.write("readme.txt", "notes about the pack");
    fixture.write("song.wav", "not audio either");
    let mut db = db();

    let request = ScanRequest::new(fixture.roots(), false);
    scan(&mut db, &request);
    assert_eq!(request.progress.counts().found, 1, "only the chart file is walked into the scan");
}

#[test]
fn a_chart_that_cannot_be_parsed_does_not_stop_the_ones_that_can() {
    let fixture = Fixture::new("broken");
    fixture.write("good.bms", &chart("good"));
    fixture.write("empty.bms", "");
    let mut db = db();

    let request = ScanRequest::new(fixture.roots(), false);
    let outcome = scan(&mut db, &request);
    assert!(matches!(outcome, ScanOutcomeG::Completed { .. }));
    assert!(db.song_count().expect("count the charts") >= 1, "the readable chart is stored");
}

#[test]
fn a_text_file_next_to_a_chart_sets_the_text_content_bit() {
    let fixture = Fixture::new("content-text");
    fixture.write("pack/song.bms", &chart("song"));
    fixture.write("pack/liner-notes.txt", "about this song");
    fixture.write("bare/song.bms", &chart("bare song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let with_text = db.song(&normalize(&fixture.root.join("pack/song.bms"))).expect("read the chart").expect("the chart is stored");
    let without = db.song(&normalize(&fixture.root.join("bare/song.bms"))).expect("read the chart").expect("the chart is stored");
    assert_eq!(with_text.content & CONTENT_TEXT, CONTENT_TEXT);
    assert_eq!(without.content & CONTENT_TEXT, 0);
}

#[test]
fn a_chart_naming_images_and_a_preview_sets_those_content_bits() {
    let fixture = Fixture::new("content-bits");
    let body = format!("{}#BMP01 bga.png\n#PREVIEW preview.ogg\n#00104:01\n", chart("decorated"));
    let path = fixture.write("song.bms", &body);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.content & CONTENT_BGA, CONTENT_BGA);
    assert_eq!(stored.content & CONTENT_PREVIEW, CONTENT_PREVIEW);
    assert_eq!(stored.preview, "preview.ogg");
}

#[test]
fn control_flow_is_read_off_the_file_the_parser_has_already_resolved() {
    assert!(uses_control_flow(b"#TITLE x\n#RANDOM 2\n#IF 1\n"));
    assert!(uses_control_flow(b"  #setrandom 3\n"));
    assert!(uses_control_flow(b"#SWITCH 4\n"));
    assert!(uses_control_flow(b"#SETSWITCH 4\n"));
    assert!(!uses_control_flow(b"#TITLE random thoughts\n#00111:0101\n"), "the word alone is not a command");
    assert!(!uses_control_flow(b""));
}

#[test]
fn the_feature_bits_of_a_plain_chart_are_clear() {
    let fixture = Fixture::new("feature-plain");
    let path = fixture.write("song.bms", &chart("plain"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.feature, 0, "a chart with nothing but plain notes claims no features");
}

#[test]
fn a_chart_built_with_control_flow_carries_the_random_feature_bit() {
    let fixture = Fixture::new("feature-random");
    let body = format!("{}#RANDOM 1\n#IF 1\n#00115:0101\n#ENDIF\n", chart("branching"));
    let path = fixture.write("song.bms", &body);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.feature & FEATURE_RANDOM, FEATURE_RANDOM);
}

#[test]
fn a_chart_with_mines_and_stops_carries_those_feature_bits() {
    let fixture = Fixture::new("feature-mine");
    let body = format!("{}#STOP01 192\n#001D1:0100\n#00109:0100\n", chart("hazardous"));
    let path = fixture.write("song.bms", &body);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.feature & FEATURE_MINE_NOTE, FEATURE_MINE_NOTE);
    assert_eq!(stored.feature & FEATURE_STOP_SEQUENCE, FEATURE_STOP_SEQUENCE);
}

#[test]
fn the_scanned_row_carries_the_header_fields_the_browser_lists_a_song_by() {
    let fixture = Fixture::new("headers");
    let body = "#TITLE Song\n#SUBTITLE [ANOTHER]\n#ARTIST Someone\n#SUBARTIST obj:Nobody\n#GENRE Test\n#MAKER Maker\n#PLAYLEVEL 11\n#DIFFICULTY 4\n#RANK 1\n#TOTAL 300\n#BPM 145\n#STAGEFILE stage.png\n#BANNER banner.png\n#WAV01 kick.wav\n#00111:01010101\n";
    let path = fixture.write("song.bms", body);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.title, "Song");
    assert_eq!(stored.subtitle, "[ANOTHER]");
    assert_eq!(stored.artist, "Someone");
    assert_eq!(stored.subartist, "obj:Nobody");
    assert_eq!(stored.genre, "Test");
    assert_eq!(stored.maker, "Maker");
    assert_eq!(stored.level, "11");
    assert_eq!(stored.difficulty, 4);
    assert_eq!(stored.judge, 1);
    assert_eq!(stored.total, 300.0);
    assert_eq!(stored.init_bpm, 145.0);
    assert_eq!(stored.stagefile, "stage.png");
    assert_eq!(stored.banner, "banner.png");
    assert_eq!(stored.min_bpm, 145);
    assert_eq!(stored.max_bpm, 145);
    assert!(stored.notes > 0, "the note count is computed during the scan");
    assert!(!stored.md5.is_empty() && !stored.sha256.is_empty(), "both chart hashes are stored");
    assert_ne!(stored.mode, 0, "the chart is stored under a detected mode");
}

#[test]
fn a_title_less_chart_falls_back_to_its_file_name() {
    let fixture = Fixture::new("untitled");
    let path = fixture.write("untitled.bms", "#ARTIST nobody\n#BPM 120\n#00111:0101\n");
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.title, "untitled.bms");
}

#[test]
fn a_scan_caches_the_detail_panel_of_every_chart_it_parses() {
    let fixture = Fixture::new("detail");
    let path = fixture.write("song.bms", &chart("song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let detail = db.detail(&normalize(&path)).expect("read the detail").expect("the scan cached a detail");
    assert!(detail.duration_us > 0, "the chart's length is integrated during the scan");
    assert!(!detail.density.is_empty(), "the density histogram is cached with it");
}

/// The radar a direct measurement of `body` gives it, read the way the scan reads a `.bms` file.
fn measured_radar(body: &str) -> RadarRow {
    let source = rbms_parser::parse(body.as_bytes());
    let mode = detect_mode(&source, "song.bms");
    RadarRow::measure(&to_model(&source, mode), &mode)
}

#[test]
fn a_scan_stores_the_radar_of_every_chart_it_parses() {
    let fixture = Fixture::new("radar");
    let body = chart("song");
    let path = fixture.write("song.bms", &body);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.radar(&normalize(&path)).expect("read the radar").expect("the scan stored a radar");
    assert_eq!(stored, measured_radar(&body), "the stored radar is the direct measurement of the chart");
    assert!(stored.is_current());
    assert!(stored.raw.is_some_and(|raw| raw.notes > 0.0 && raw.peak > 0.0), "{stored:?}");
}

#[test]
fn the_radar_a_scan_stores_is_the_radar_the_focused_detail_computes() {
    let fixture = Fixture::new("radar-detail");
    let path = fixture.write("song.bms", &chart("song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let key = normalize(&path);
    let stored = db.song(&key).expect("read the chart").expect("the chart is stored");
    let mode = mode_from_id(stored.mode).expect("the stored mode is one of the build's");
    let computed = crate::compute_chart_detail(&path, mode).expect("the chart parses");
    assert_eq!(computed.radar, db.radar(&key).expect("read the radar"));
}

#[test]
fn a_chart_in_a_mode_without_a_radar_is_stored_with_a_row_that_says_so() {
    let fixture = Fixture::new("radar-popn");
    let path = fixture.write("pop.pms", &chart("pop"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(mode_from_id(stored.mode), Some(rbms_model::Mode::POPN_9K));
    assert_eq!(
        db.radar(&normalize(&path)).expect("read the radar"),
        Some(RadarRow { version: RADAR_VERSION, raw: None }),
        "the row is there so the cursor never measures a nine-key chart again"
    );
}

#[test]
fn a_bmson_chart_gets_a_radar_too() {
    let fixture = Fixture::new("radar-bmson");
    let path = fixture.write("song.bmson", &bmson_chart("bmson song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.radar(&normalize(&path)).expect("read the radar").expect("the scan stored a radar");
    assert!(stored.is_current());
    assert!(stored.raw.is_some_and(|raw| raw.notes > 0.0), "a bmson chart in a seven-key mode has a radar: {stored:?}");
}

#[test]
fn a_rewritten_chart_is_measured_again() {
    let fixture = Fixture::new("radar-rewrite");
    let path = fixture.write("song.bms", &chart("song"));
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    let key = normalize(&path);
    let before = db.radar(&key).expect("read the radar").expect("the scan stored a radar");

    let denser = format!("{}#00112:01010101\n#00114:01010101\n", chart("song, now with more notes"));
    fixture.write("song.bms", &denser);
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let after = db.radar(&key).expect("read the radar").expect("the rescan stored a radar");
    assert_ne!(after, before, "the rewrite changed the chart, so it changed the radar");
    assert_eq!(after, measured_radar(&denser));
}

#[test]
fn an_incremental_scan_leaves_a_stored_radar_alone_and_a_full_one_measures_it_again() {
    let fixture = Fixture::new("radar-stale");
    let body = chart("song");
    let path = fixture.write("song.bms", &body);
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    let key = normalize(&path);

    let stale = RadarRow { version: RADAR_VERSION + 1, raw: None };
    db.put_radar(&key, &stale).expect("store a radar of another generation");

    scan(&mut db, &ScanRequest::new(fixture.roots(), false));
    assert_eq!(db.radar(&key).expect("read the radar"), Some(stale), "a chart whose file did not move is not read, so its radar is not touched");

    scan(&mut db, &ScanRequest::new(fixture.roots(), true));
    assert_eq!(db.radar(&key).expect("read the radar"), Some(measured_radar(&body)), "a full scan reads every chart and measures it under this build");
}

/// A chart of a few bytes whose tempo puts its last note years away is scanned like any other, and
/// gets no histogram: one with a bin for each of its seconds would be gigabytes.
#[test]
fn a_chart_years_long_is_scanned_without_a_density_histogram() {
    let fixture = Fixture::new("years");
    let path = fixture.write("far.bms", "#BPM 0.0001\n#00111:01\n#99911:01\n");
    let mut db = db();
    scan(&mut db, &ScanRequest::new(fixture.roots(), false));

    let stored = db.song(&normalize(&path)).expect("read the chart").expect("the chart is stored");
    assert_eq!(stored.notes, 2);
    let detail = db.detail(&normalize(&path)).expect("read the detail").expect("the scan cached a detail");
    assert!(
        detail.duration_us / US_PER_SECOND > rbms_chart::MAX_TABLED_SECONDS as i64,
        "the fixture runs {} us, which is not past the bound",
        detail.duration_us
    );
    assert!(detail.density.is_empty());
    assert_eq!((detail.peak_density, detail.avg_density, detail.end_density), (0.0, 0.0, 0.0));
}

#[test]
fn worker_count_never_exceeds_the_work_or_the_ceiling() {
    assert_eq!(worker_count(1), 1);
    assert!(worker_count(1_000) <= MAX_WORKERS);
    assert!(worker_count(1_000) >= 1);
    assert_eq!(worker_count(0), 1, "an empty job list still asks for a usable worker count");
}

#[test]
fn a_normalised_path_is_absolute_and_uses_forward_separators() {
    let normalised = normalize(Path::new("relative/song.bms"));
    assert!(!normalised.contains('\\'), "separators are normalised: {normalised}");
    assert!(normalised.ends_with("relative/song.bms"));
    assert!(Path::new(&normalised).is_absolute(), "the stored path is absolute: {normalised}");
}

#[test]
fn a_root_that_does_not_exist_contributes_nothing() {
    let fixture = Fixture::new("missing-root");
    let mut db = db();
    let missing = fixture.root.join("not-here").to_string_lossy().into_owned();
    let request = ScanRequest::new(vec![missing], false);
    assert_eq!(scan(&mut db, &request), ScanOutcomeG::Completed { upserted: 0, removed: 0 });
    assert_eq!(request.progress.counts(), ScanCounts::default());
}

/// The smallest playable bmson document, mirroring `crates/rbms-parser/tests/bmson/minimal.bmson`
/// so the scanner is exercised on the same shape the parser's own tests pin.
fn bmson_chart(title: &str) -> String {
    format!(
        "{{\"version\":\"1.0.0\",\"info\":{{\"title\":\"{title}\",\"artist\":\"rbms\",\"genre\":\"test\",\"mode_hint\":\"beat-7k\",\"judge_rank\":100,\"total\":100,\"init_bpm\":120,\"level\":5,\"resolution\":240,\"eyecatch_image\":\"eye.png\",\"banner_image\":\"banner.png\",\"back_image\":\"back.png\",\"preview_music\":\"preview.ogg\"}},\"lines\":[{{\"y\":0}},{{\"y\":960}}],\"sound_channels\":[{{\"name\":\"kick.wav\",\"notes\":[{{\"x\":1,\"y\":0}},{{\"x\":2,\"y\":240}},{{\"x\":3,\"y\":480}}]}}]}}"
    )
}

#[test]
fn a_bmson_chart_is_walked_and_stored_beside_the_bms_ones() {
    let fixture = Fixture::new("bmson");
    fixture.write("song.bms", &chart("bms song"));
    fixture.write("song.bmson", &bmson_chart("bmson song"));
    let mut db = db();

    let request = ScanRequest::new(fixture.roots(), false);
    assert_eq!(scan(&mut db, &request), ScanOutcomeG::Completed { upserted: 2, removed: 0 });

    let rows = db.all_songs().expect("read the rows");
    let stored = rows.iter().find(|row| row.title == "bmson song").expect("the bmson chart is stored");
    assert_eq!(stored.notes, 3, "every bmson key note is counted");
    assert_eq!(stored.init_bpm, 120.0);
    assert_eq!(stored.stagefile, "eye.png");
    assert_eq!(stored.banner, "banner.png");
    assert_eq!(stored.preview, "preview.ogg");
    assert!(stored.length_ms > 0, "the timeline gives the chart a length");
    assert!(!stored.md5.is_empty() && !stored.sha256.is_empty(), "the file hashes are stored");
}

#[test]
fn a_bmson_chart_that_is_not_json_is_skipped_rather_than_stored_empty() {
    let fixture = Fixture::new("bmson-broken");
    fixture.write("broken.bmson", "not json at all");
    let mut db = db();

    let request = ScanRequest::new(fixture.roots(), false);
    assert_eq!(scan(&mut db, &request), ScanOutcomeG::Completed { upserted: 0, removed: 0 });
    assert_eq!(db.song_count().expect("count the charts"), 0);
}
