//! What the browser keeps for its skin beside the list, on a browser that is really run: the chart
//! under the cursor as the chart cluster reads it, the records and the replay slot of that chart,
//! and the course under the cursor.
//!
//! The skin here is three squares, each drawn while a condition a published skin asks about holds,
//! over the sheet the tests next door draw their wheel from.

use rbms_course::{Course, CourseChart, CourseConstraint};
use rbms_library::songdb::{CONTENT_BGA, FEATURE_LONG_NOTE, FEATURE_STOP_SEQUENCE};
use rbms_library::{ChartDetail, Library};
use rbms_skin::property::generated::{NUMBER_MAINBPM, NUMBER_MAXBPM, NUMBER_MINBPM, NUMBER_TOTALNOTES, STRING_GENRE, STRING_SUBARTIST};
use rbms_skin::property::generated::{OPTION_7KEYSONG, OPTION_BGA, OPTION_BPMCHANGE, OPTION_BPMSTOP, OPTION_LN, OPTION_SELECT_REPLAYDATA, OPTION_TEXT};
use rbms_store::ScoreBook;
use rbms_store::scoredb::{ScoreDb, migrate_score_book};

use super::list::{ChartFact, chart_under_cursor};
use super::skinned_tests::{Browser, CELL, DOCUMENT_SIZE, KEY_SEVEN, KEY_SIX, SHORT_STEP, starts_a_chart};
use super::tests::{entry, record};
use super::*;
use crate::skin_host::ClusterState;
use crate::skin_host::select::REPLAY_SLOTS;

/// Where the fixture draws each square, as its left edge in the skin's own coordinates, and the box
/// they share: one while the chart under the cursor is a seven-key chart, one while the bar there is
/// a folder, and one while the first replay slot is the one selected.
const SQUARE_LEFTS: [u32; 3] = [20, 120, 220];
const SQUARE_BOTTOM: u32 = 20;
const SQUARE_SIDE: u32 = 40;
const FIRST_SLOT: usize = 2;

/// The option that says the bar under the cursor opens (`OPTION_FOLDERBAR`).
const OPTION_FOLDERBAR: i32 = 1;

/// The red of the sheet's first cell, which the squares are drawn from, and the least of it that
/// counts as the square being there.
const SQUARE_RED_FLOOR: u8 = 20;

/// The times of the fixture skin's header, in milliseconds.
const NO_WAIT_MS: u64 = 0;
const SCENE_MS: u64 = 3000;

/// The clear and the score of the records written here.
const RECORD_CLEAR: u8 = 5;
const RECORD_EX: u32 = 500;

fn lent_document() -> String {
    let squares = SQUARE_LEFTS
        .iter()
        .zip([OPTION_7KEYSONG, OPTION_FOLDERBAR, OPTION_SELECT_REPLAYDATA])
        .map(|(x, option)| format!(r#"{{"id":"lamp","op":[{option}],"dst":[{{"x":{x},"y":{SQUARE_BOTTOM},"w":{SQUARE_SIDE},"h":{SQUARE_SIDE}}}]}}"#))
        .collect::<Vec<_>>();
    format!(
        r#"{{
            "type": {SKIN_TYPE_MUSIC_SELECT}, "name": "lent", "w": {}, "h": {},
            "input": {NO_WAIT_MS}, "scene": {SCENE_MS}, "fadeout": {NO_WAIT_MS},
            "source": [{{ "id": "sheet", "path": "sheet.png" }}],
            "image": [{{ "id": "lamp", "src": "sheet", "x": 0, "y": 0, "w": {CELL}, "h": {CELL} }}],
            "destination": [{}]
        }}"#,
        DOCUMENT_SIZE.0,
        DOCUMENT_SIZE.1,
        squares.join(","),
    )
}

/// A browser over three charts that the fixture of the three squares draws.
fn browser(tag: &str) -> Browser {
    Browser::skinned_by(tag, ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec(), lent_document())
}

/// Which of the three squares the last frame drew.
fn squares(browser: &Browser) -> [bool; 3] {
    SQUARE_LEFTS.map(|left| {
        let across = (left + SQUARE_SIDE / 2) * CW / DOCUMENT_SIZE.0;
        let down = (DOCUMENT_SIZE.1 - SQUARE_BOTTOM - SQUARE_SIDE / 2) * CH / DOCUMENT_SIZE.1;
        browser.pixels.pixel_at(across, down).r > SQUARE_RED_FLOOR
    })
}

/// A record of the chart `md5` played at `played_at`, with the replay `file` saved of it.
fn played(md5: &str, played_at: i64, file: Option<&str>) -> rbms_store::ScoreRecord {
    rbms_store::ScoreRecord { replay_file: file.map(str::to_owned), ..record(md5, RECORD_CLEAR, RECORD_EX, 0, played_at) }
}

/// Put `records` in the score book, as a finished run does.
fn record_plays(browser: &mut Browser, records: Vec<rbms_store::ScoreRecord>) {
    browser.app.shared.scores = ScoreBook::from_records(records);
    browser.frame_after(SHORT_STEP);
}

/// The chart cluster is lent the chart under the cursor, so what a skin asks about the chart is
/// answered on the browser as it is on the decide screen; a bar that opens leaves the slot empty,
/// and its options read as they do with no chart.
#[test]
fn a_skins_frame_is_lent_the_chart_under_the_cursor() {
    let mut browser = browser("lent-chart");
    assert_eq!(squares(&browser), [true, false, false], "a seven-key chart with no replay saved");

    browser.app.shared.select_view = SelectView::Root;
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame_after(SHORT_STEP);
    assert_eq!(squares(&browser), [false, true, false], "a folder is no seven-key chart");
}

/// What the song database holds about a chart is what the chart cluster answers from, as the
/// reference answers from its `SongData`, and what measuring the notes found is laid over it.
#[test]
fn the_chart_under_the_cursor_reads_what_the_song_database_holds() {
    let mut chart = entry("alpha", "composer", "7");
    chart.genre = "GENRE".to_owned();
    chart.maker = "maker".to_owned();
    let fact = ChartFact {
        features: u32::try_from(FEATURE_LONG_NOTE | FEATURE_STOP_SEQUENCE).unwrap_or_default(),
        content: CONTENT_BGA,
        subartist: "obj: somebody".to_owned(),
        min_bpm: 90,
        max_bpm: 180,
        length_ms: 134_000,
        notes: 1624,
        ..ChartFact::default()
    };

    let meta = chart_under_cursor(&chart, Some(&fact), None);
    let state = ChartState::Chart(&meta);
    assert_eq!([NUMBER_MINBPM, NUMBER_MAXBPM, NUMBER_TOTALNOTES].map(|id| state.integer(id)), [Some(90), Some(180), Some(1624)]);
    assert_eq!(state.integer(NUMBER_MAINBPM), None, "the tempo most notes are played at was guessed for a chart that changes tempo");
    assert_eq!(
        [OPTION_BPMCHANGE, OPTION_BGA, OPTION_LN, OPTION_BPMSTOP, OPTION_TEXT].map(|id| state.boolean(id)),
        [Some(true), Some(true), Some(true), Some(true), Some(false)]
    );
    assert_eq!(state.text(STRING_GENRE).as_deref(), Some("GENRE"), "the genre is the chart's own, without its maker");
    assert_eq!(state.text(STRING_SUBARTIST).as_deref(), Some("obj: somebody"));

    let unread = chart_under_cursor(&chart, None, None);
    let state = ChartState::Chart(&unread);
    assert_eq!(state.integer(NUMBER_MAXBPM), None, "a tempo nobody measured was made up");
    assert_eq!(state.boolean(OPTION_7KEYSONG), Some(true), "the library's entry says what kind of play it is");

    let detail = ChartDetail {
        notes: 800,
        long_notes: 0,
        duration_us: 60_000_000,
        bpm_min: 150.0,
        bpm_max: 150.0,
        density: Vec::new(),
        peak_density: 0.0,
        avg_density: 0.0,
        end_density: 0.0,
    };
    let measured = chart_under_cursor(&chart, None, Some(&detail));
    let state = ChartState::Chart(&measured);
    assert_eq!([NUMBER_MAXBPM, NUMBER_TOTALNOTES].map(|id| state.integer(id)), [Some(150), Some(800)]);
    assert_eq!(state.integer(NUMBER_MAINBPM), Some(150), "a chart with one tempo is played at that tempo");
    assert_eq!(state.boolean(OPTION_BPMCHANGE), Some(false));
}

/// The replay slot goes to the lowest slot that holds a replay whenever the bar under the cursor
/// changes, and to none on a bar with no replay (`RESET_REPLAY`); a skin asking which slot is
/// selected is told none then, where a slot that is always the first lit its marker on every bar.
#[test]
fn the_replay_slot_follows_the_bar_under_the_cursor() {
    let mut browser = browser("lent-slot");
    assert_eq!(browser.state.selected_replay, None, "a chart with no replay has a slot selected");
    assert!(!squares(&browser)[FIRST_SLOT]);

    record_plays(&mut browser, vec![played("md5-alpha", 1, Some("one")), played("md5-alpha", 2, Some("two")), played("md5-beta", 3, None)]);
    assert_eq!(browser.state.selected_replay, Some(0), "the lowest slot that holds a replay");
    assert!(squares(&browser)[FIRST_SLOT]);

    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1);
    assert_eq!(browser.state.selected_replay, None, "beta was played and no replay was saved of it");
    assert!(!squares(&browser)[FIRST_SLOT]);

    assert_eq!(browser.arrow(KeyCode::ArrowUp), 0);
    assert_eq!(browser.state.selected_replay, Some(0));
}

/// The key for the next replay moves the slot round the slots that hold a replay and no further,
/// and does nothing with no other slot to move to (`NEXT_REPLAY`).
#[test]
fn the_next_replay_key_goes_round_the_slots_that_hold_one() {
    let mut browser = browser("lent-next");
    record_plays(&mut browser, (1..=3).map(|at| played("md5-alpha", at, Some("replay"))).collect());
    assert_eq!(browser.state.selected_replay, Some(0));

    let mut seen = Vec::new();
    for _ in 0..REPLAY_SLOTS {
        browser.tap(KEY_SIX);
        seen.push(browser.state.selected_replay);
    }
    assert_eq!(seen, [Some(1), Some(2), Some(0), Some(1)], "three replays are three slots, and the fourth is passed over");

    browser.state.selected_replay = Some(0);
    browser.key(KeyCode::Digit4);
    assert_eq!(browser.state.selected_replay, Some(1), "the number key is the same command");

    let mut browser = self::browser("lent-next-one");
    record_plays(&mut browser, vec![played("md5-alpha", 1, Some("only"))]);
    browser.tap(KEY_SIX);
    assert_eq!(browser.state.selected_replay, Some(0), "with one replay there is nowhere to move to");

    let mut browser = self::browser("lent-next-none");
    browser.tap(KEY_SIX);
    assert_eq!(browser.state.selected_replay, None, "a chart with no replay was given a slot");
}

/// Key seven plays the replay in the slot that is selected, and plays the chart when none is
/// (`REPLAY`). The replay here names a file that is not there, so taking that way leaves the browser
/// where it is, where playing the chart would have started it.
#[test]
fn key_seven_plays_the_selected_replay_when_there_is_one() {
    let mut browser = browser("lent-replay-key");
    record_plays(&mut browser, vec![played("md5-alpha", 1, Some("not-there.rbmsreplay"))]);
    browser.hold(KEY_SEVEN);
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "key seven played the chart with a replay selected");

    let mut browser = self::browser("lent-replay-key-none");
    browser.hold(KEY_SEVEN);
    assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)), "with no replay selected key seven plays the chart");
}

/// The records of the chart under the cursor are read from the score database when the cursor
/// comes to the chart or a run is recorded, and are what the frames in between are lent.
#[test]
fn the_records_of_the_chart_under_the_cursor_are_read_when_it_changes() {
    let mut browser = browser("lent-records");
    assert!(browser.state.focused_records.best().is_none());

    let book = ScoreBook::from_records(vec![played("md5-beta", 1, Some("one"))]);
    let mut database = ScoreDb::open_in_memory().expect("a database in memory opens");
    database.migrate().expect("the layout is created");
    migrate_score_book(&mut database, &book).expect("the record is stored");
    browser.app.shared.scoredb = Some(database);
    browser.app.shared.scores = book;
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.focused_records.best().is_none(), "alpha has no record");
    assert_eq!(browser.state.focused_records.replays_stored(), 0);

    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1);
    let best = browser.state.focused_records.best().expect("beta's best was not read when the cursor came to it");
    assert_eq!((best.clear, best.md5.as_str()), (RECORD_CLEAR, "md5-beta"));
    assert_eq!(browser.state.focused_records.replays_stored(), 1);

    browser.app.shared.scoredb = None;
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.focused_records.best().is_some(), "the database was asked again with nothing changed");

    assert_eq!(browser.arrow(KeyCode::ArrowDown), 2);
    assert!(browser.state.focused_records.best().is_none(), "gamma was lent beta's score");
}

/// The course under the cursor is lent with what it plays under and the title of each of its
/// stages: the library's own for a chart the library holds, and the course's, marked, for one it
/// does not.
#[test]
fn the_course_under_the_cursor_is_lent_with_its_stages() {
    let mut browser = browser("lent-course");
    let stage = |md5: &str, title: &str| CourseChart { md5: md5.to_owned(), sha256: String::new(), title: title.to_owned() };
    let mut courses = vec![
        Course {
            name: "HELD".to_owned(),
            charts: vec![stage("md5-alpha", "first"), stage("md5-gamma", "second")],
            constraints: vec![CourseConstraint::Mirror],
            ..Course::default()
        },
        Course { name: "PARTLY".to_owned(), charts: vec![stage("md5-beta", "opener"), stage(&"0".repeat(32), "absent")], ..Course::default() },
    ];
    courses.retain_mut(Course::validate);
    let library: &Library = &browser.app.shared.library;
    browser.state = SelectState::on_courses(courses, library);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);

    let (_, _, held) = browser.state.course_shown.as_ref().expect("the course under the cursor was not lent");
    assert_eq!(held.stages, ["alpha", "gamma"], "a stage the library holds reads the library's title");
    assert_eq!(held.constraints, [CourseConstraint::Mirror]);

    assert_eq!(browser.arrow(KeyCode::ArrowDown), 0, "the course list has a cursor of its own");
    let (_, row, partly) = browser.state.course_shown.as_ref().expect("the course under the cursor was not lent");
    assert_eq!(*row, 1);
    assert_eq!(partly.stages, ["beta", "(no song) absent"]);

    browser.state.tab = SelectTab::Songs;
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.course_shown.is_none(), "a course was lent with the song list up");
}
