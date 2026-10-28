//! The default pack's browser over charts whose texts and numbers are longer than the boxes of its
//! info window: a title, a subtitle, an artist and a genre far past their widths, bars with long
//! titles beside the cursor, and a chart of over six thousand notes whose best has five digits in
//! its EX SCORE and MISS COUNT and a five-digit EX that its AAA target asks for.
//!
//! Like the other captures of the default pack this saves frames for a person to look at and passes
//! without drawing anything unless [`CAPTURE_DIR_ENV`] is set. The frames are drawn with the bundled
//! face only, so a script it lacks is drawn as empty boxes; the texts here are Latin.

use rbms_library::scan::ScanRequest;

use super::*;
use crate::select_tree::SelectFolder;

/// The default pack, from this crate's folder.
const LONG_PACK: &str = "../../assets/skins/rbms-default";

/// The name the long browser is saved under, with the milliseconds [`BROWSER_SHOTS_MS`] adds.
const LONG_BROWSER: &str = "long-text";

/// The texts of the chart the cursor lands on, each far past the box it is written in.
const LONG_TITLE: &str = "The Quick Brown Fox Jumps Over The Lazy Dog And Keeps Running Through The Night";
const LONG_SUBTITLE: &str = "[SP ANOTHER] Extended Remix Version For The Long Winded";
const LONG_ARTIST: &str = "A Very Long Artist Name Feat. Another Very Long Guest Artist Name And Friends";
const LONG_GENRE: &str = "Progressive Hardcore Breakbeat Trance Fusion Of Everything";

/// The charts of the library: the title, the subtitle, `#DIFFICULTY`, `#PLAYLEVEL` and the file of
/// each. The two of [`LONG_TITLE`] are one chart's siblings, so the difficulty tabs hold two levels.
const LONG_CHARTS: [(&str, &str, u32, u32, &str); 5] = [
    ("Alpha", "", 2, 5, "alpha.bms"),
    ("Bravo Charlie Delta Echo Foxtrot Golf Hotel India Juliet Kilo Lima Mike", "", 3, 8, "bravo.bms"),
    (LONG_TITLE, LONG_SUBTITLE, 4, 12, "long-another.bms"),
    (LONG_TITLE, "[SP HYPER] Extended Remix Version", 3, 9, "long-hyper.bms"),
    ("Zulu Yankee Xray Whiskey Victor Uniform Tango Sierra Romeo Quebec Papa", "", 5, 14, "zulu.bms"),
];

/// How many measures the long chart has, how many objects each lane of a measure holds, and the
/// lanes that carry them: the seven keys.
const LONG_MEASURES: u32 = 56;
const OBJECTS_PER_LANE: usize = 16;
const LONG_LANES: [&str; 7] = ["11", "12", "13", "14", "15", "18", "19"];

/// What the best of the long chart is: its clear lamp (EX HARD), its EX, and its misses. The EX
/// and the misses have five digits.
const LONG_BEST_CLEAR: u8 = 7;
const LONG_BEST_EX: u32 = 12_000;
const LONG_BEST_BREAKS: u32 = 12_345;

fn long_chart_text(title: &str, subtitle: &str, difficulty: u32, level: u32) -> String {
    let notes: String =
        (1..=LONG_MEASURES).flat_map(|measure| LONG_LANES.iter().map(move |lane| format!("#{measure:03}{lane}:{}\n", "01".repeat(OBJECTS_PER_LANE)))).collect();
    format!(
        "#PLAYER 1\n#GENRE {LONG_GENRE}\n#TITLE {title}\n#SUBTITLE {subtitle}\n#ARTIST {LONG_ARTIST}\n#BPM 150\n#PLAYLEVEL {level}\n#DIFFICULTY {difficulty}\n#RANK 2\n#TOTAL 300\n{notes}"
    )
}

/// An app drawing with `pack` whose library is the charts above, scanned into a song database of
/// the test's own, with a best on the long chart.
fn long_text_app(pack: &Path, tag: &str) -> Result<App, String> {
    let mut app = pack_app(pack, tag);
    let folder = settings_of(&format!("{tag}-charts")).with_file_name("charts");
    std::fs::create_dir_all(&folder).map_err(|error| format!("the chart folder was not made: {error}"))?;
    for (title, subtitle, difficulty, level, file) in LONG_CHARTS {
        std::fs::write(folder.join(file), long_chart_text(title, subtitle, difficulty, level)).map_err(|error| format!("{file} was not written: {error}"))?;
    }
    let database = folder.with_file_name("songs.db");
    let report = crate::library::scan_now(&database, &ScanRequest::new(vec![folder.to_string_lossy().into_owned()], true));
    if let Some(failure) = report.failure {
        return Err(format!("the library was not scanned: {failure}"));
    }
    app.shared.library = report.library;
    app.shared.song_db = crate::library::open_song_db(&database);
    let best =
        app.shared.library.songs().iter().find(|entry| entry.title == LONG_TITLE && entry.subtitle == LONG_SUBTITLE).map(|entry| rbms_store::ScoreRecord {
            mode: entry.mode.name.to_owned(),
            counts: [LONG_BEST_EX / EX_PER_PGREAT, LONG_BEST_EX % EX_PER_PGREAT, 0, LONG_BEST_BREAKS, 0, 0],
            ..record(&entry.md5, LONG_BEST_CLEAR, LONG_BEST_EX, LONG_BEST_BREAKS, BROWSER_RECORD_PLAYED_AT)
        });
    app.shared.scores = rbms_store::ScoreBook::from_records(best.into_iter().collect());
    let mut scores = ScoreDb::open_in_memory().map_err(|error| format!("the score database did not open: {error}"))?;
    scores.migrate().map_err(|error| format!("the score database was not laid out: {error}"))?;
    migrate_score_book(&mut scores, &app.shared.scores).map_err(|error| format!("the record was not stored: {error}"))?;
    app.shared.scoredb = Some(scores);
    app.shared.config.library.sort = SortMode::Title;
    app.shared.config.library.preview = false;
    app.shared.audio_failed = true;
    Ok(app)
}

/// The browser over the long library with the cursor on [`LONG_TITLE`].
fn capture_long_browser(pack: &Path) -> Result<Vec<String>, String> {
    let mut app = long_text_app(pack, "long-browser")?;
    let size = authored_size(&app, SKIN_TYPE_MUSIC_SELECT).ok_or_else(|| NO_DOCUMENT.to_owned())?;
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.rebuild_select_items();
    let songs = app.shared.library.songs();
    let long = app
        .shared
        .select_items
        .iter()
        .position(|item| matches!(item, crate::SelectItem::Song(index) if songs.get(*index).is_some_and(|entry| entry.title == LONG_TITLE && entry.subtitle == LONG_SUBTITLE)))
        .ok_or("the scan did not find the chart with the long texts")?;
    app.shared.sel = long.checked_sub(1).ok_or("the chart with the long texts is the first of the list")?;
    capture_browser_view(&mut app, &mut pixels, size, LONG_BROWSER, SelectState::new())
}

/// Captures the default pack's browser on a chart whose texts and numbers are longer than the boxes
/// of its info window.
///
/// Opt-in: without [`CAPTURE_DIR_ENV`] this passes without drawing anything. With it, the screen
/// has to draw, and the pack's folder has to be left as it was.
#[test]
fn the_default_packs_info_window_is_captured_with_texts_and_numbers_longer_than_its_boxes() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(LONG_PACK);
    let before = files_under(&pack);
    let saved = capture_long_browser(&pack).unwrap_or_else(|reason| panic!("long texts: not drawn: {reason}"));
    println!("long texts: captured {}", saved.join(", "));
    assert_eq!(files_under(&pack), before, "capturing the default pack changed its folder");
}
