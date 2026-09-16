use std::sync::atomic::AtomicU32;
use std::time::{Duration, Instant};

use rbms_library::songdb::mode_id;

use super::*;

/// Keeps one test's fixture tree clear of another's when the suite runs them in one process.
static FIXTURE_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// How long a test waits for a background scan before calling it hung.
const SCAN_TIMEOUT: Duration = Duration::from_secs(30);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        let seq = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("rbms-player-library-{tag}-{}-{seq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create the fixture root");
        Fixture { root }
    }

    fn write(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::write(&path, body).expect("write the fixture chart");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn sample_chart() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/preview-demo/preview-demo.bms")
}

fn db() -> SongDb {
    let mut db = SongDb::open_in_memory().expect("open the in-memory database");
    db.migrate().expect("apply the schema");
    db
}

fn row(path: &str, title: &str) -> SongRow {
    SongRow {
        path: path.to_string(),
        title: title.to_string(),
        artist: "artist".to_string(),
        genre: "genre".to_string(),
        maker: "maker".to_string(),
        level: "9".to_string(),
        difficulty: 2,
        mode: mode_id(Mode::BEAT_7K),
        judge: 3,
        total: 260.0,
        init_bpm: 150.0,
        min_bpm: 100,
        max_bpm: 200,
        notes: 900,
        long_notes: 7,
        md5: "AABBCC".to_string(),
        stagefile: "stage.png".to_string(),
        banner: "banner.png".to_string(),
        preview: "preview.ogg".to_string(),
        ..SongRow::default()
    }
}

#[test]
fn a_stored_row_becomes_the_entry_the_browser_lists() {
    let stored = row("/songs/pack/a.bms", "A Chart");
    let entry = entry_of(&stored);
    assert_eq!(entry.path, PathBuf::from("/songs/pack/a.bms"));
    assert_eq!(entry.title, "A Chart");
    assert_eq!(entry.artist, "artist");
    assert_eq!(entry.genre, "genre");
    assert_eq!(entry.maker, "maker");
    assert_eq!(entry.level, "9");
    assert_eq!(entry.difficulty, 2);
    assert_eq!(entry.rank, 3, "the stored judge column is the chart's #RANK");
    assert_eq!(entry.total, 260.0);
    assert_eq!(entry.init_bpm, 150.0);
    assert_eq!(entry.mode, Mode::BEAT_7K);
    assert_eq!(entry.md5, "AABBCC");
    assert_eq!(entry.stagefile, "stage.png");
    assert_eq!(entry.banner, "banner.png");
    assert_eq!(entry.preview, "preview.ogg");
}

#[test]
fn a_row_stored_under_a_mode_this_build_does_not_know_still_lists() {
    let mut stored = row("/songs/pack/a.bms", "A Chart");
    stored.mode = 99;
    assert_eq!(entry_of(&stored).mode, FALLBACK_MODE, "an unreadable mode id falls back rather than dropping the chart");
}

#[test]
fn the_library_is_title_ordered_and_indexed_by_md5() {
    let mut first = row("/songs/pack/c.bms", "cherry");
    first.md5 = "AAAA".to_string();
    let mut second = row("/songs/pack/a.bms", "Apple");
    second.md5 = "aaaa".to_string();
    let mut third = row("/songs/pack/b.bms", "banana");
    third.md5 = "bbbb".to_string();

    let library = library_of(vec![first, second, third]);
    let titles: Vec<&str> = library.songs().iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, ["Apple", "banana", "cherry"]);
    assert_eq!(library.indices_for_md5("AaAa").len(), 2, "both spellings of one md5 index together");
    assert_eq!(library.indices_for_md5("bbbb"), [1]);
}

#[test]
fn an_empty_database_reads_as_an_empty_library() {
    assert!(stored_library(&db()).is_empty());
}

#[test]
fn every_stored_chart_comes_back_in_the_library() {
    let mut db = db();
    db.upsert_batch(&[row("/songs/pack/a.bms", "A"), row("/songs/pack/b.bms", "B")]).expect("store the charts");
    let library = stored_library(&db);
    assert_eq!(library.len(), 2);
    assert_eq!(library.songs()[0].title, "A");
}

#[test]
fn a_cached_detail_is_served_without_reading_the_chart_again() {
    let mut db = db();
    let missing = "/nowhere/gone.bms";
    let mut stored = row(missing, "Gone");
    stored.notes = 1234;
    stored.long_notes = 56;
    db.upsert_batch(std::slice::from_ref(&stored)).expect("store the chart");
    db.put_detail(missing, &DetailRow { duration_us: 90_000_000, peak_density: 21.0, avg_density: 12.5, end_density: 8.0, density: vec![1, 2, 3] })
        .expect("cache the detail");

    let detail = chart_detail(&db, Path::new(missing), Mode::BEAT_7K).expect("the cached detail is served");
    assert_eq!(detail.notes, 1234);
    assert_eq!(detail.long_notes, 56);
    assert_eq!(detail.duration_us, 90_000_000);
    assert_eq!(detail.bpm_min, 100.0);
    assert_eq!(detail.bpm_max, 200.0);
    assert_eq!(detail.density, vec![1, 2, 3]);
    assert_eq!(detail.peak_density, 21.0);
    assert_eq!(detail.avg_density, 12.5);
    assert_eq!(detail.end_density, 8.0);
}

#[test]
fn a_detail_that_was_never_cached_is_computed_once_and_kept() {
    let chart = sample_chart();
    let key = normalize(&chart);
    let mut db = db();
    db.upsert_batch(&[row(&key, "Preview Demo")]).expect("store the chart");
    assert!(db.detail(&key).expect("read the detail").is_none(), "nothing is cached yet");

    let computed = chart_detail(&db, &chart, Mode::BEAT_7K).expect("the chart is integrated on the spot");
    assert!(computed.duration_us > 0);
    let cached = db.detail(&key).expect("read the detail").expect("the computed detail was cached");
    assert_eq!(cached.duration_us, computed.duration_us);
    assert_eq!(cached.density, computed.density);
}

#[test]
fn a_chart_that_is_neither_stored_nor_readable_has_no_detail() {
    assert!(chart_detail(&db(), Path::new("/nowhere/gone.bms"), Mode::BEAT_7K).is_none());
}

#[test]
fn a_background_scan_reports_the_library_it_stored() {
    let fixture = Fixture::new("scan");
    fixture.write("song.bms", "#TITLE Scanned\n#ARTIST tester\n#PLAYLEVEL 7\n#BPM 130\n#WAV01 kick.wav\n#00111:01010101\n");
    let db_path = fixture.root.join(SONGDB_FILE);

    let scan = spawn_scan(db_path.clone(), vec![fixture.root.to_string_lossy().into_owned()], false);
    let report = wait_for(&scan);
    assert!(report.failure.is_none(), "the scan ran: {:?}", report.failure);
    assert!(!report.cancelled);
    assert_eq!(report.upserted, 1);
    assert_eq!(report.removed, 0);
    assert_eq!(report.library.len(), 1);
    assert_eq!(report.library.songs()[0].title, "Scanned");
    assert!(db_path.exists(), "the scan created the database it was pointed at");

    let counts = scan.counts();
    assert_eq!(counts.found, 1);
    assert_eq!(counts.parsed, 1);
}

#[test]
fn a_scan_of_a_database_that_cannot_be_opened_reports_the_failure() {
    let fixture = Fixture::new("unopenable");
    let db_path = fixture.root.join("not-a-directory.bms").join(SONGDB_FILE);
    fixture.write("not-a-directory.bms", "#TITLE in the way\n");

    let scan = spawn_scan(db_path, vec![fixture.root.to_string_lossy().into_owned()], false);
    let report = wait_for(&scan);
    assert!(report.failure.is_some(), "a database that cannot be created is reported rather than swallowed");
    assert!(report.library.is_empty());
}

#[test]
fn a_scan_cancelled_before_it_starts_reports_as_cancelled_and_stores_nothing() {
    let fixture = Fixture::new("cancelled");
    fixture.write("song.bms", "#TITLE Never scanned\n#BPM 120\n#00111:0101\n");
    let db_path = fixture.root.join(SONGDB_FILE);

    let request = ScanRequest::new(vec![fixture.root.to_string_lossy().into_owned()], false);
    request.cancel.store(true, Ordering::Relaxed);
    let report = scan_now(&db_path, &request);
    assert!(report.cancelled);
    assert!(report.failure.is_none(), "cancelling is not a failure: {:?}", report.failure);
    assert!(report.library.is_empty());
}

#[test]
fn the_song_database_is_named_next_to_the_other_config_files() {
    assert_eq!(SONGDB_FILE, "songdb.sqlite");
}

/// Wait for a scan to report, failing the test rather than hanging forever if it never does.
fn wait_for(scan: &LibraryScan) -> ScanReport {
    let deadline = Instant::now() + SCAN_TIMEOUT;
    loop {
        if let Some(report) = scan.poll() {
            return report;
        }
        assert!(Instant::now() < deadline, "the background scan did not report within {SCAN_TIMEOUT:?}");
        std::thread::sleep(Duration::from_millis(5));
    }
}
