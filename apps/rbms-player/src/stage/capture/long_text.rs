//! The default pack's browser over charts whose texts and numbers are longer than the boxes of its
//! info window: a title, a subtitle, an artist and a genre far past their widths, bars with long
//! titles beside the cursor, and a chart of over six thousand notes whose best has five digits in
//! its EX SCORE, MISS COUNT and MAX COMBO and a five-digit EX that its AAA target asks for.
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

/// What every play of the long chart is: its clear lamp (EX HARD), its EX, its misses and its combo.
/// The EX, the misses and the combo have five digits.
const LONG_BEST_CLEAR: u8 = 7;
const LONG_BEST_EX: u32 = 12_000;
const LONG_BEST_BREAKS: u32 = 12_345;
const LONG_BEST_COMBO: u32 = 11_111;

/// How many times the long chart was played, all of them cleared: the least that gives PLAY COUNT
/// and CLEAR COUNT five digits, the most a count of the pair of the info window shows whole. The
/// plays are a second apart.
const LONG_PLAYS: u32 = 10_000;
const MILLIS_PER_PLAY: i64 = 1_000;

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
    let plays: Vec<rbms_store::ScoreRecord> = app
        .shared
        .library
        .songs()
        .iter()
        .find(|entry| entry.title == LONG_TITLE && entry.subtitle == LONG_SUBTITLE)
        .map(|entry| {
            (0..LONG_PLAYS)
                .map(|play| rbms_store::ScoreRecord {
                    mode: entry.mode.name.to_owned(),
                    counts: [LONG_BEST_EX / EX_PER_PGREAT, LONG_BEST_EX % EX_PER_PGREAT, 0, LONG_BEST_BREAKS, 0, 0],
                    max_combo: LONG_BEST_COMBO,
                    ..record(&entry.md5, LONG_BEST_CLEAR, LONG_BEST_EX, LONG_BEST_BREAKS, BROWSER_RECORD_PLAYED_AT + i64::from(play) * MILLIS_PER_PLAY)
                })
                .collect()
        })
        .unwrap_or_default();
    app.shared.scores = rbms_store::ScoreBook::from_records(plays);
    let mut scores = ScoreDb::open_in_memory().map_err(|error| format!("the score database did not open: {error}"))?;
    scores.migrate().map_err(|error| format!("the score database was not laid out: {error}"))?;
    migrate_score_book(&mut scores, &app.shared.scores).map_err(|error| format!("the record was not stored: {error}"))?;
    app.shared.scoredb = Some(scores);
    app.shared.config.library.sort = SortMode::Title;
    app.shared.config.library.preview = false;
    app.shared.audio_failed = true;
    Ok(app)
}

/// The browser over the long library with the cursor on [`LONG_TITLE`], drawn at each of
/// [`BROWSER_SHOTS_MS`]: the names the frames were saved under, and the canvas the last of them is
/// left on.
fn long_browser_drawn(pack: &Path) -> Result<(Vec<String>, HeadlessCanvas), String> {
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
    let saved = capture_browser_view(&mut app, &mut pixels, size, LONG_BROWSER, SelectState::new())?;
    Ok((saved, pixels))
}

/// The browser over the long library with the cursor on [`LONG_TITLE`], saved.
fn capture_long_browser(pack: &Path) -> Result<Vec<String>, String> {
    long_browser_drawn(pack).map(|(saved, _)| saved)
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

/// What of the drawn frame the check below reads, in the pixels of the 1920 by 1080 document: the
/// columns of the head where a song's four lines are written and the rows that hold them, and the
/// cell of the MISS column of the best record of MY RECORDS with the rows of that record.
const HEAD_TEXT_COLUMNS: Range<u32> = 224..1096;
const HEAD_TEXT_ROWS: Range<u32> = 112..280;
const BEST_MISS_COLUMNS: Range<u32> = 1046..1098;
const BEST_RECORD_ROWS: Range<u32> = 618..656;

/// The three label and value pairs of MY SCORE that count: the columns and the rows each is drawn
/// in, and how many letters its label has.
struct CountPair {
    label: &'static str,
    columns: Range<u32>,
    rows: Range<u32>,
    letters: usize,
}

const COUNT_PAIRS: [CountPair; 3] = [
    CountPair { label: "MAX COMBO", columns: 352..552, rows: 764..790, letters: 8 },
    CountPair { label: "PLAY COUNT", columns: 560..752, rows: 764..790, letters: 9 },
    CountPair { label: "CLEAR COUNT", columns: 560..752, rows: 788..814, letters: 10 },
];

/// How many lines the head of a chart writes, how many digits the misses of the best are, how many
/// digits the combo and the counts of the best have, and how many pixels apart the last letter and
/// the first digit are to be read as two words.
const HEAD_LINES: usize = 4;
const BEST_MISS_DIGITS: usize = 5;
const COUNT_VALUE_GLYPHS: usize = 5;
const LABEL_TO_VALUE_GAP: u32 = 3;

/// The brightest channel a pixel has to reach to be taken as part of a glyph: any trace of one for
/// the rows between two lines, and a body of the glyph for the columns between two digits.
pub(super) const TRACE_OF_INK: u8 = 16;
pub(super) const BODY_OF_INK: u8 = 64;

/// The runs of rows of `rows` of `pixels` that carry ink in `columns` -- of columns when `by_column`
/// -- each as the range it covers.
pub(super) fn ink_runs(pixels: &HeadlessCanvas, columns: Range<u32>, rows: Range<u32>, by_column: bool, ink: u8) -> Vec<Range<u32>> {
    let carries = |x: u32, y: u32| {
        let color = pixels.pixel_at(x, y);
        color.r.max(color.g).max(color.b) >= ink
    };
    let scanned = if by_column { columns.clone() } else { rows.clone() };
    let mut runs: Vec<Range<u32>> = Vec::new();
    for at in scanned {
        let inked = if by_column { rows.clone().any(|y| carries(at, y)) } else { columns.clone().any(|x| carries(x, at)) };
        match runs.last_mut() {
            Some(run) if inked && run.end == at => run.end += 1,
            _ if inked => runs.push(at..at + 1),
            _ => {}
        }
    }
    runs
}

/// The four lines of a chart's head leave blank pixel rows between them even where a letter that
/// hangs below its line stands over one that rises above the next -- a title ending in `g`, `y` or
/// `p` over a subtitle that opens with `[` -- the misses of a best that has five digits are drawn
/// with all five, and the labels MAX COMBO, PLAY COUNT and CLEAR COUNT are clear of the five digits
/// beside them.
///
/// Runs wherever the tests do: it draws the default pack's browser and keeps nothing.
#[test]
fn the_default_packs_info_window_keeps_its_lines_apart_and_its_five_digit_numbers_whole_and_clear() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(LONG_PACK);
    let (_, pixels) = long_browser_drawn(&pack).unwrap_or_else(|reason| panic!("long texts: not drawn: {reason}"));

    let lines = ink_runs(&pixels, HEAD_TEXT_COLUMNS, HEAD_TEXT_ROWS, false, TRACE_OF_INK);
    assert_eq!(lines.len(), HEAD_LINES, "the head's lines run into one another: the rows that carry ink are {lines:?}");

    let digits = ink_runs(&pixels, BEST_MISS_COLUMNS, BEST_RECORD_ROWS, true, BODY_OF_INK);
    assert_eq!(digits.len(), BEST_MISS_DIGITS, "the misses of the best are not all drawn: the columns that carry ink are {digits:?}");

    for CountPair { label, columns, rows, letters } in COUNT_PAIRS {
        let pair = ink_runs(&pixels, columns, rows, true, BODY_OF_INK);
        assert_eq!(pair.len(), letters + COUNT_VALUE_GLYPHS, "{label} and its five digits are not all drawn apart: the columns that carry ink are {pair:?}");
        let apart = pair[letters].start - pair[letters - 1].end;
        assert!(apart >= LABEL_TO_VALUE_GAP, "{label} touches the five digits beside it, {apart} px from them: the columns that carry ink are {pair:?}");
    }
}
