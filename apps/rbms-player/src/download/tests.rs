//! The download job and the scan after it: a chart got from a table comes back as a bar of the
//! library with the cursor on it, an archive without it or without any chart says so, only one job
//! runs at a time, a scan that a full scan overtook is run again, a job that ends during a run waits
//! for the song list, and a download folder the settings name is brought in.
//!
//! Nothing here asks the network: the archive is served from memory by the fetch the tests hand
//! the job ([`serving`], [`held`]), and every folder is a temporary one of the test's own.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use rbms_config::{Config, SortMode, TableSource};
use rbms_download::{Answer, DownloadError, DownloadLimits, download_with};
use rbms_play::{PlaySession, SessionOptions};

use super::*;
use crate::notify::Level;
use crate::stage::{FrameCtx, PlayState, SelectState, Stage, StageHandler};
use crate::{App, LaunchOptions, SelectItem};

/// What a download fails with when the test has not handed the job a fetch of its own.
pub(crate) const NO_NETWORK_IN_TESTS: &str = "a test reached for the network";

/// Keeps one test's folders clear of another's when the suite runs them in one process.
static SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// The pieces of a zip archive that stores its files as they are.
const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_HEADER_SIGNATURE: u32 = 0x0201_4b50;
const END_OF_CENTRAL_DIRECTORY_SIGNATURE: u32 = 0x0605_4b50;
const ZIP_VERSION: u16 = 20;
const STORED: u16 = 0;
const FIRST_DOS_DATE: u16 = (1 << 5) | 1;
const CRC32_POLYNOMIAL: u32 = 0xEDB8_8320;

/// What the table calls itself and the level its charts are on.
const TABLE: &str = "STELLA";
const LEVEL: &str = "3";

/// The chart the tests get: its title, its link and the file it is.
const ZIP_TITLE: &str = "Zip Chart";
const ZIP_URL: &str = "https://charts.test/zip-chart.zip";
const ZIP_CHART: &[u8] = b"#TITLE Zip Chart\n#ARTIST Zip Artist\n#BPM 120\n#PLAYLEVEL 3\n#WAV01 kick.wav\n#00111:01\n";
const SOUND: &[u8] = b"RIFF....WAVEfmt ";

/// Another chart the table lists, never got, and a chart some other archive holds.
const OTHER_MD5: &str = "0123456789abcdef0123456789abcdef";
const OTHER_URL: &str = "https://charts.test/other.zip";
const STRANGER_CHART: &[u8] = b"#TITLE Stranger\n#BPM 150\n#PLAYLEVEL 9\n#00111:01\n";

/// A chart in a song folder the scans of the downloads are not to read.
const ELSEWHERE_CHART: &[u8] = b"#TITLE Elsewhere\n#BPM 130\n#PLAYLEVEL 1\n#00111:01\n";

/// How many frames a run is played for in the test that ends a job during one.
const PLAY_FRAMES: usize = 3;

/// The CRC-32 a zip archive checks its files by.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..u8::BITS {
            crc = if crc & 1 == 1 { (crc >> 1) ^ CRC32_POLYNOMIAL } else { crc >> 1 };
        }
    }
    !crc
}

fn put16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// A zip archive holding `files`, each stored as it is.
pub(crate) fn stored_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, body) in files {
        let offset = u32::try_from(out.len()).expect("a fixture archive is small");
        let size = u32::try_from(body.len()).expect("a fixture file is small");
        let name_len = u16::try_from(name.len()).expect("a fixture name is short");
        let crc = crc32(body);
        put32(&mut out, LOCAL_HEADER_SIGNATURE);
        for value in [ZIP_VERSION, 0, STORED, 0, FIRST_DOS_DATE] {
            put16(&mut out, value);
        }
        for value in [crc, size, size] {
            put32(&mut out, value);
        }
        put16(&mut out, name_len);
        put16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(body);

        put32(&mut central, CENTRAL_HEADER_SIGNATURE);
        for value in [ZIP_VERSION, ZIP_VERSION, 0, STORED, 0, FIRST_DOS_DATE] {
            put16(&mut central, value);
        }
        for value in [crc, size, size] {
            put32(&mut central, value);
        }
        for value in [name_len, 0, 0, 0, 0] {
            put16(&mut central, value);
        }
        put32(&mut central, 0);
        put32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let central_offset = u32::try_from(out.len()).expect("a fixture archive is small");
    let central_size = u32::try_from(central.len()).expect("a fixture archive is small");
    let entries = u16::try_from(files.len()).expect("a fixture archive has few files");
    out.extend_from_slice(&central);
    put32(&mut out, END_OF_CENTRAL_DIRECTORY_SIGNATURE);
    for value in [0, 0, entries, entries] {
        put16(&mut out, value);
    }
    put32(&mut out, central_size);
    put32(&mut out, central_offset);
    put16(&mut out, 0);
    out
}

/// A fetch that answers `url` with `archive` from memory, and refuses any other address.
pub(crate) fn serving(url: &str, archive: Vec<u8>) -> Fetch {
    let url = url.to_owned();
    Arc::new(move |asked, file, progress, cancel| {
        if asked != url {
            return Err(DownloadError::Io(std::io::Error::other(format!("the test serves {url}, not {asked}"))));
        }
        download_with(asked, file, &DownloadLimits::default(), progress, cancel, &mut |address| answer(address, &archive))
    })
}

/// [`serving`], with the answer held back until what this hands over is dropped.
pub(crate) fn held(url: &str, archive: Vec<u8>) -> (Fetch, Sender<()>) {
    let (open, shut) = std::sync::mpsc::channel::<()>();
    let shut = Mutex::new(shut);
    let url = url.to_owned();
    let fetch: Fetch = Arc::new(move |asked, file, progress, cancel| {
        if asked != url {
            return Err(DownloadError::Io(std::io::Error::other(format!("the test serves {url}, not {asked}"))));
        }
        download_with(asked, file, &DownloadLimits::default(), progress, cancel, &mut |address| {
            let _ = shut.lock().unwrap_or_else(PoisonError::into_inner).recv();
            answer(address, &archive)
        })
    });
    (fetch, open)
}

fn answer(address: &str, archive: &[u8]) -> Result<Answer, DownloadError> {
    Ok(Answer { url: address.to_owned(), content_length: u64::try_from(archive.len()).ok(), body: Box::new(Cursor::new(archive.to_vec())) })
}

/// Have every job started from here on fetch with `fetch`.
pub(crate) fn use_fetch(shared: &mut AppShared, fetch: Fetch) {
    shared.downloads.fetch = fetch;
}

/// Wait for the download job to end, so that the next frame of the browser takes it.
pub(crate) fn wait_for_the_job(shared: &mut AppShared) {
    shared.downloads.job.as_mut().expect("a download job is running").landing.wait();
}

/// Wait for the scan after a download to end, so that the next frame of the browser takes it.
fn wait_for_the_scan(shared: &mut AppShared) {
    shared.downloads.refresh.as_mut().expect("a scan of the downloads is running").landing.wait();
}

/// One entry of the table, on its one level.
fn entry(md5: &str, title: &str, url_diff: &str) -> String {
    format!(r#"{{"md5":"{md5}","level":"{LEVEL}","title":"{title}","artist":"","url":"","url_diff":"{url_diff}"}}"#)
}

/// The md5 the scan reads a chart file by.
fn md5_of(chart: &[u8]) -> String {
    rbms_parser::parse(chart).md5
}

/// A player of a folder of its own, with the table file listing `entries` registered, the
/// download folder chosen and the browser open on the table's level. Its library is the song
/// database's, which holds nothing yet, and a song folder beside the download folder holds a chart
/// no scan has read.
struct Player {
    app: App,
    state: SelectState,
    root: PathBuf,
}

impl Player {
    fn new(tag: &str, entries: &[String], fetch: Fetch) -> Player {
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("rbms-download-job-{tag}-{}-{sequence}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("settings")).expect("the fixture folder is writable");
        fs::create_dir_all(root.join("songs")).expect("the song folder is writable");
        fs::write(root.join("songs").join("elsewhere.bms"), ELSEWHERE_CHART).expect("the song folder's chart is written");
        let table = root.join("table.json");
        fs::write(&table, format!("[{}]", entries.join(","))).expect("the table file is written");

        let mut config = Config::default();
        config.library.preview = false;
        rbms_render::font::use_embedded_fonts_only();
        let mut app = App::new(String::new(), config, LaunchOptions::default(), root.join("settings").join("settings.ron"));
        let shared = &mut app.shared;
        shared.audio_failed = true;
        shared.config.library.folders = vec![root.join("songs").to_string_lossy().into_owned()];
        let downloads = root.join("downloads").to_string_lossy().into_owned();
        shared.downloads.scanned_with(Some(&downloads));
        shared.config.library.download_folder = Some(downloads);
        shared.config.library.sort = SortMode::Default;
        shared.config.library.tables = vec![TableSource { name: TABLE.to_owned(), location: table.to_string_lossy().into_owned(), group: String::new() }];
        let matched = read_and_match_all(&TableStore::beside(&shared.settings_path), &shared.config.library.tables, &shared.library, &AtomicUsize::new(0));
        shared.set_tables(matched);
        shared.open_for_tests([shared.table_folder(0), shared.level_folder(0, 0)]);
        shared.sel = 0;
        shared.rebuild_select_items();
        use_fetch(shared, fetch);
        Player { app, state: SelectState::new(), root }
    }

    fn shared(&mut self) -> &mut AppShared {
        &mut self.app.shared
    }

    fn downloads(&self) -> PathBuf {
        self.root.join("downloads")
    }

    /// One frame of the browser.
    fn frame(&mut self) {
        let mut ctx = FrameCtx { shared: &mut self.app.shared, now: Instant::now(), dt: 0.0 };
        let _ = self.state.update(&mut ctx);
    }

    /// Put the cursor on the chart `md5` the library does not hold, and answer its table entry.
    fn on_missing(&mut self, md5: &str) -> TableEntry {
        let shared = &mut self.app.shared;
        let (row, entry) = shared
            .select_items
            .iter()
            .enumerate()
            .find_map(|(row, item)| match item {
                SelectItem::Missing(missing) => shared.missing_chart(missing).filter(|chart| chart.md5 == md5).map(|chart| (row, chart.clone())),
                _ => None,
            })
            .expect("the level lists the chart as missing");
        shared.sel = row;
        entry
    }

    /// What the bar under the cursor is: the md5 of the chart there, and whether the library holds it.
    fn under_the_cursor(&self) -> (String, bool) {
        let shared = &self.app.shared;
        match shared.select_items.get(shared.sel) {
            Some(SelectItem::Song(index)) => (shared.library.songs()[*index].md5.clone(), true),
            Some(SelectItem::Missing(missing)) => (shared.missing_chart(missing).map(|chart| chart.md5.clone()).unwrap_or_default(), false),
            _ => panic!("the cursor is on no chart"),
        }
    }

    fn titles(&self) -> Vec<String> {
        let mut titles: Vec<String> = self.app.shared.library.songs().iter().map(|song| song.title.clone()).collect();
        titles.sort();
        titles
    }

    /// Start getting the chart under the cursor, take in the job when it ends and the scan after it.
    fn get_and_scan(&mut self, entry: &TableEntry) {
        start_download(self.shared(), entry).expect("the download starts");
        wait_for_the_job(self.shared());
        self.frame();
        wait_for_the_scan(self.shared());
        self.frame();
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn said() -> Vec<(Level, String)> {
    let mut said = Vec::new();
    crate::notify::drain(&mut said);
    said
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(folder).map(|entries| entries.flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort();
    names
}

#[test]
fn a_finished_download_scans_only_its_folder_and_brings_the_bar_back() {
    crate::notify::exclusive(|| {
        let md5 = md5_of(ZIP_CHART);
        let archive = stored_zip(&[("zip-chart/chart.bms", ZIP_CHART), ("zip-chart/kick.wav", SOUND)]);
        let mut player = Player::new("found", &[entry(OTHER_MD5, "Other Chart", OTHER_URL), entry(&md5, ZIP_TITLE, ZIP_URL)], serving(ZIP_URL, archive));
        let wanted = player.on_missing(&md5);
        assert_eq!(player.app.shared.sel, 1, "the chart is the second missing one");

        start_download(player.shared(), &wanted).expect("the download starts");
        wait_for_the_job(player.shared());
        player.frame();
        let scanning = status(&player.app.shared).expect("the job goes on into the scan");
        assert_eq!((scanning.phase, scanning.md5.as_str()), (Phase::Scanning, md5.as_str()));
        assert_eq!(player.under_the_cursor(), (md5.clone(), false), "the library was replaced before the scan of the download came back");
        wait_for_the_scan(player.shared());
        player.frame();

        assert_eq!(player.under_the_cursor(), (md5.clone(), true), "the bar under the cursor is not the chart got");
        assert_eq!(player.app.shared.select_items.len(), 2, "the other chart is still listed as missing");
        assert_eq!(player.titles(), [ZIP_TITLE], "the scan read more than the folder the chart was unpacked into");
        assert_eq!(player.app.shared.library_generation, 1);
        let done = status(&player.app.shared).expect("how the job ended is kept");
        assert_eq!(done.phase, Phase::Done { found: true, charts: 1 });
        assert_eq!(said(), [(Level::Info, format!("downloaded: {ZIP_TITLE} (1 chart(s))"))]);

        let folder = format!("{ZIP_TITLE} [{}]", &md5[..8]);
        assert_eq!(names_in(&player.downloads()), std::slice::from_ref(&folder), "the job left something beside the chart's folder");
        assert_eq!(names_in(&player.downloads().join(&folder).join("zip-chart")), ["chart.bms", "kick.wav"]);
    });
}

#[test]
fn an_archive_without_this_chart_says_so_and_keeps_the_bar_missing() {
    crate::notify::exclusive(|| {
        let archive = stored_zip(&[("stranger.bms", STRANGER_CHART)]);
        let mut player = Player::new("not-found", &[entry(OTHER_MD5, ZIP_TITLE, ZIP_URL)], serving(ZIP_URL, archive));
        let wanted = player.on_missing(OTHER_MD5);
        player.get_and_scan(&wanted);

        assert_eq!(player.under_the_cursor(), (OTHER_MD5.to_owned(), false), "a chart the archive did not hold became a bar of the library");
        assert_eq!(player.titles(), ["Stranger"], "the chart the archive held is not in the library");
        assert_eq!(status(&player.app.shared).map(|job| job.phase), Some(Phase::Done { found: false, charts: 1 }));
        assert_eq!(said(), [(Level::Warn, "the archive had 1 chart(s) but not this one".to_owned())]);
        assert_eq!(names_in(&player.downloads()), [format!("{ZIP_TITLE} [{}]", &OTHER_MD5[..8])], "the folder of what the archive held was not kept");
    });
}

#[test]
fn an_archive_without_any_chart_leaves_nothing() {
    crate::notify::exclusive(|| {
        let archive = stored_zip(&[("readme.txt", b"no chart here")]);
        let mut player = Player::new("no-chart", &[entry(OTHER_MD5, ZIP_TITLE, ZIP_URL)], serving(ZIP_URL, archive));
        let wanted = player.on_missing(OTHER_MD5);
        start_download(player.shared(), &wanted).expect("the download starts");
        wait_for_the_job(player.shared());
        player.frame();

        assert!(player.app.shared.downloads.refresh.is_none(), "an archive with nothing to play was scanned");
        assert_eq!(status(&player.app.shared).map(|job| job.phase), Some(Phase::Failed(DownloadError::NoChart.to_string())));
        assert_eq!(said(), [(Level::Warn, format!("download failed: {}", DownloadError::NoChart))]);
        assert!(names_in(&player.downloads()).is_empty(), "an archive with no chart left {:?}", names_in(&player.downloads()));
    });
}

#[test]
fn a_second_download_is_refused_while_one_runs() {
    crate::notify::exclusive(|| {
        let md5 = md5_of(ZIP_CHART);
        let (fetch, gate) = held(ZIP_URL, stored_zip(&[("chart.bms", ZIP_CHART)]));
        let mut player = Player::new("one-at-a-time", &[entry(OTHER_MD5, "Other Chart", OTHER_URL), entry(&md5, ZIP_TITLE, ZIP_URL)], fetch);
        let other = player.on_missing(OTHER_MD5);
        let wanted = player.on_missing(&md5);

        start_download(player.shared(), &wanted).expect("the first download starts");
        assert_eq!(start_download(player.shared(), &other), Err(ANOTHER_DOWNLOAD_IS_RUNNING), "a second chart was got beside the first");
        assert_eq!(start_download(player.shared(), &wanted), Err(ANOTHER_DOWNLOAD_IS_RUNNING), "the same chart was got twice at once");
        player.frame();
        let running = status(&player.app.shared).expect("the job is running");
        assert_eq!((running.phase.clone(), running.md5.as_str()), (Phase::Downloading, md5.as_str()));
        assert!(running.can_cancel());

        cancel_download(player.shared());
        drop(gate);
        wait_for_the_job(player.shared());
        player.frame();
        assert_eq!(status(&player.app.shared).map(|job| job.phase), Some(Phase::Cancelled));
        assert_eq!(said(), [(Level::Info, "download cancelled".to_owned())]);
        assert!(names_in(&player.downloads()).is_empty(), "a cancelled download left {:?}", names_in(&player.downloads()));
        assert!(player.app.shared.downloads.refresh.is_none(), "a cancelled download was scanned");

        start_download(player.shared(), &other).expect("another chart can be got once the first has ended");
    });
}

#[test]
fn a_refresh_whose_library_changed_meanwhile_is_run_again() {
    let md5 = md5_of(ZIP_CHART);
    let mut player = Player::new("overtaken", &[entry(&md5, ZIP_TITLE, ZIP_URL)], serving(ZIP_URL, stored_zip(&[("chart.bms", ZIP_CHART)])));
    let wanted = player.on_missing(&md5);
    start_download(player.shared(), &wanted).expect("the download starts");
    wait_for_the_job(player.shared());
    player.frame();

    player.app.shared.library_generation += 1;
    wait_for_the_scan(player.shared());
    player.frame();
    assert_eq!(player.under_the_cursor(), (md5.clone(), false), "a scan a full scan overtook was taken in");
    assert!(player.app.shared.downloads.refresh.is_some(), "the overtaken scan was not run again");
    assert_eq!(status(&player.app.shared).map(|job| job.phase), Some(Phase::Scanning));

    wait_for_the_scan(player.shared());
    player.frame();
    assert_eq!(player.under_the_cursor(), (md5, true), "the scan run again was not taken in");
    assert_eq!(status(&player.app.shared).map(|job| job.phase), Some(Phase::Done { found: true, charts: 1 }));
}

#[test]
fn a_download_that_ends_during_play_is_taken_on_the_browser() {
    let md5 = md5_of(ZIP_CHART);
    let mut player = Player::new("during-play", &[entry(&md5, ZIP_TITLE, ZIP_URL)], serving(ZIP_URL, stored_zip(&[("chart.bms", ZIP_CHART)])));
    let wanted = player.on_missing(&md5);
    start_download(player.shared(), &wanted).expect("the download starts");
    wait_for_the_job(player.shared());

    let source = rbms_parser::parse(STRANGER_CHART);
    let model = rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, "stranger.bms"));
    player.app.stage =
        Stage::Play(Box::new(PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, String::new())));
    for _ in 0..PLAY_FRAMES {
        let mut ctx = FrameCtx { shared: &mut player.app.shared, now: Instant::now(), dt: 0.0 };
        let _ = player.app.stage.update(&mut ctx);
    }
    assert!(player.app.shared.downloads.job.is_some(), "the play screen took the job that ended");
    assert!(player.app.shared.downloads.refresh.is_none(), "the play screen started the scan of the download");
    assert!(player.app.shared.library.is_empty(), "the library was replaced during a run");

    player.frame();
    wait_for_the_scan(player.shared());
    player.frame();
    assert_eq!(player.under_the_cursor(), (md5, true), "the browser did not take in the download that ended during the run");
}

#[test]
fn a_download_folder_chosen_in_the_settings_is_scanned_on_the_browser() {
    let mut player = Player::new("folder-changed", &[entry(OTHER_MD5, "Other Chart", OTHER_URL)], serving(ZIP_URL, Vec::new()));
    let chosen = player.root.join("chosen");
    fs::create_dir_all(&chosen).expect("the chosen folder is writable");
    fs::write(chosen.join("stranger.bms"), STRANGER_CHART).expect("the chart in the chosen folder is written");
    player.frame();
    assert!(player.app.shared.downloads.refresh.is_none(), "the download folder the library was read with was scanned again");

    player.app.shared.config.library.download_folder = Some(chosen.to_string_lossy().into_owned());
    player.frame();
    let refresh = player.app.shared.downloads.refresh.as_ref().expect("the folder the settings chose is scanned");
    assert_eq!(refresh.roots, [chosen.to_string_lossy().into_owned()]);
    assert!(status(&player.app.shared).is_none(), "scanning a folder is no download");
    wait_for_the_scan(player.shared());
    player.frame();
    assert_eq!(player.titles(), ["Stranger"], "the chart in the chosen folder is not in the library");
}

/// A chart got while a scan of a newly chosen download folder ran is waiting for that scan when a
/// full scan overtakes it. The scan run again carries the chart with it, so the job ends with the
/// chart in the library and another can be started: it is not left waiting for a scan that never
/// comes. Which frame the overtaken scan lands in does not matter.
#[test]
fn a_chart_waiting_for_an_overtaken_scan_is_carried_into_the_scan_run_again() {
    crate::notify::exclusive(|| {
        let md5 = md5_of(ZIP_CHART);
        let mut player = Player::new("carried", &[entry(&md5, ZIP_TITLE, ZIP_URL)], serving(ZIP_URL, stored_zip(&[("chart.bms", ZIP_CHART)])));
        let wanted = player.on_missing(&md5);
        let chosen = player.root.join("chosen");
        fs::create_dir_all(&chosen).expect("the chosen folder is writable");
        fs::write(chosen.join("stranger.bms"), STRANGER_CHART).expect("the chart in the chosen folder is written");
        player.app.shared.config.library.download_folder = Some(chosen.to_string_lossy().into_owned());
        player.frame();
        assert!(player.app.shared.downloads.refresh.as_ref().is_some_and(|refresh| refresh.awaited.is_none()), "the chosen folder is not being scanned");

        start_download(player.shared(), &wanted).expect("a download starts while a folder is scanned");
        wait_for_the_job(player.shared());
        player.app.shared.library_generation += 1;
        player.frame();
        while player.app.shared.downloads.refresh.is_some() {
            wait_for_the_scan(player.shared());
            player.frame();
        }

        assert_eq!(status(&player.app.shared).map(|job| job.phase), Some(Phase::Done { found: true, charts: 1 }), "the job was left waiting for a scan");
        assert!(!player.app.shared.downloads.is_busy(), "another download would be refused");
        assert_eq!(player.under_the_cursor(), (md5, true), "the chart got is not a bar of the library");
        assert_eq!(said(), [(Level::Info, format!("downloaded: {ZIP_TITLE} (1 chart(s))"))]);
    });
}

/// The address of the table in the test where it arrives while a scan of the downloads runs, and
/// what the table calls itself before and after.
const RACING_TABLE: &str = "https://tables.test/download-race/";
const BEFORE: &str = "BEFORE";
const AFTER: &str = "AFTER";

/// A table that arrives while a scan of the downloads runs is matched against the copy the scan
/// read before it arrived. Its fetch is held until the scan has read that copy, then let land
/// before the scan is taken, the order one frame of the browser takes the two in. The scan does not
/// put the copy it read back in place of the table that arrived: it is run again, and reads the
/// copy the fetch kept.
#[test]
fn a_table_that_arrives_while_the_downloads_are_scanned_is_not_put_back_as_it_was() {
    crate::notify::exclusive(|| {
        let tables = crate::tablesrc::test_outside::FakeTables::default();
        let _installed = tables.install();
        let mut player = Player::new("table-race", &[], serving(ZIP_URL, Vec::new()));
        let body = format!("[{}]", entry(&md5_of(STRANGER_CHART), "Stranger", ""));
        let source = TableSource { name: String::new(), location: RACING_TABLE.to_owned(), group: String::new() };
        let shared = player.shared();
        TableStore::beside(&shared.settings_path).keep_for_tests(RACING_TABLE, BEFORE, body.as_bytes(), Some(crate::tablesrc::unix_now()));
        tables.serve(RACING_TABLE, AFTER, body.as_bytes());
        shared.config.library.tables = vec![source.clone()];
        let matched = read_and_match_all(&TableStore::beside(&shared.settings_path), &shared.config.library.tables, &shared.library, &AtomicUsize::new(0));
        shared.set_tables(matched);
        assert_eq!(shared.table_names, [BEFORE]);

        let open = tables.hold();
        let store = TableStore::beside(&shared.settings_path);
        assert_eq!(shared.table_fetches.start(store, vec![source], crate::tablesrc::FetchWhy::Stale), 1);
        let chosen = player.root.join("chosen");
        fs::create_dir_all(&chosen).expect("the chosen folder is writable");
        fs::write(chosen.join("stranger.bms"), STRANGER_CHART).expect("the chart in the chosen folder is written");
        player.app.shared.config.library.download_folder = Some(chosen.to_string_lossy().into_owned());
        player.frame();
        wait_for_the_scan(player.shared());
        drop(open);
        while !crate::tablesrc::apply_finished_fetches(player.shared()) {
            std::thread::yield_now();
        }
        assert_eq!(player.app.shared.table_names, [AFTER], "the table that arrived is not in the list");

        player.frame();
        assert_eq!(player.app.shared.table_names, [AFTER], "the scan put back the copy it read before the table arrived");
        assert!(player.app.shared.downloads.refresh.is_some(), "the scan that read an older copy was not run again");
        wait_for_the_scan(player.shared());
        player.frame();
        assert_eq!(player.app.shared.table_names, [AFTER], "the scan run again did not read the copy the fetch kept");
        assert_eq!(player.titles(), ["Stranger"], "the scan run again was not taken in");
        crate::tablesrc::forget_table_status(RACING_TABLE);
        said();
    });
}
