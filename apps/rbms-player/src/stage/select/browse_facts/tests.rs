//! What the browser works out beside the list for its skin: which charts are the other charts of a
//! song, how the open folder's charts are cleared and the rows of the player's own plays, and the
//! events that move the cursor between the charts of a song.

use std::path::PathBuf;

use rbms_library::SongEntry;
use rbms_model::Mode;
use rbms_render::dj_rank;
use rbms_skin::lua::local_time;
use rbms_store::scoredb::{JudgeCounts, PlayLog, ScoreDb, migrate_score_book};
use rbms_store::{REFERENCE_RECORD_RULE_VERSION, ScoreBook, ScoreRecord};

use super::super::events::{SelectEvent, event_of};
use super::super::skinned_tests::{Browser, SHORT_STEP};
use super::super::tests::{entry, record};
use super::*;
use crate::select_tree::SelectFolder;
use crate::skin_host::extension::RBMS_EVENT_SIBLING_CHART_FIRST;
use crate::skin_host::select::browse::clock_text;
use crate::skin_host::{Cluster, ClusterRequest};

/// A chart of the song `title` in the folder `folder`, of difficulty slot `difficulty`.
fn chart(folder: &str, title: &str, difficulty: i32, mode: Mode) -> SongEntry {
    SongEntry {
        path: PathBuf::from(format!("{folder}/{title}-{difficulty}-{}.bms", mode.name)),
        difficulty,
        mode,
        md5: format!("md5-{folder}-{title}-{difficulty}-{}", mode.name),
        level: (difficulty * 3).to_string(),
        ..entry(title, "artist", "1")
    }
}

/// The rows of a list that holds the library's charts in order.
fn rows_of(songs: &[SongEntry]) -> Vec<SelectItem> {
    (0..songs.len()).map(SelectItem::Song).collect()
}

#[test]
fn the_charts_of_a_song_are_the_ones_of_its_folder_title_and_mode_by_difficulty() {
    let songs = [
        chart("a", "Song", 1, Mode::BEAT_7K),
        chart("a", "Song", 3, Mode::BEAT_7K),
        chart("a", "Song", 4, Mode::BEAT_7K),
        chart("a", "Another Song", 3, Mode::BEAT_7K),
        chart("b", "Song", 3, Mode::BEAT_7K),
        chart("a", "Song", 4, Mode::BEAT_14K),
    ];
    let items = rows_of(&songs);

    assert_eq!(sibling_rows(&items, &songs, 1), [Some(0), None, Some(1), Some(2), None]);
    assert_eq!(sibling_rows(&items, &songs, 0), sibling_rows(&items, &songs, 2), "every chart of the song sees the same slots");
    assert_eq!(sibling_rows(&items, &songs, 5), [None, None, None, Some(5), None], "a double chart is a song of its own");
    assert_eq!(sibling_rows(&items, &songs, 4), [None, None, Some(4), None, None], "so is the same title in another folder");
}

#[test]
fn the_chart_under_the_cursor_keeps_its_slot_and_the_first_of_two_others_takes_one_they_both_claim() {
    let songs = [chart("a", "Song", 1, Mode::BEAT_7K), chart("a", "Song", 4, Mode::BEAT_7K), chart("a", "Song", 4, Mode::BEAT_7K)];
    let items = rows_of(&songs);

    assert_eq!(sibling_rows(&items, &songs, 0)[3], Some(1), "from another chart the first of the two has the slot");
    assert_eq!(sibling_rows(&items, &songs, 1)[3], Some(1));
    assert_eq!(sibling_rows(&items, &songs, 2)[3], Some(2), "the chart under the cursor has its own slot to itself");
}

#[test]
fn a_chart_the_list_does_not_hold_is_in_no_slot() {
    let songs = [chart("a", "Song", 1, Mode::BEAT_7K), chart("a", "Song", 3, Mode::BEAT_7K), chart("a", "Song", 4, Mode::BEAT_7K)];
    let items = vec![SelectItem::Song(0), SelectItem::Song(2)];

    assert_eq!(sibling_rows(&items, &songs, 0), [Some(0), None, None, Some(1), None]);
}

#[test]
fn a_difficulty_that_is_no_slot_names_no_slot_and_the_chart_still_sees_the_others() {
    let songs = [
        chart("a", "Song", 0, Mode::BEAT_7K),
        chart("a", "Song", 2, Mode::BEAT_7K),
        chart("a", "Song", 6, Mode::BEAT_7K),
        chart("a", "Song", -1, Mode::BEAT_7K),
    ];
    let items = rows_of(&songs);

    assert_eq!(sibling_rows(&items, &songs, 0), [None, Some(1), None, None, None], "slot 0 and slots past the fifth are nobody's");
    assert_eq!(sibling_rows(&items, &songs, 2), [None, Some(1), None, None, None]);
}

#[test]
fn a_row_that_is_no_chart_has_no_other_charts() {
    let songs = [chart("a", "Song", 1, Mode::BEAT_7K)];
    let items = vec![SelectItem::Folder { label: "folder".to_string(), target: SelectFolder::AllSongs }, SelectItem::Song(0)];

    assert_eq!(sibling_rows(&items, &songs, 0), [None; SIBLING_SLOTS]);
    assert_eq!(sibling_rows(&items, &songs, 1), [Some(1), None, None, None, None]);
    assert_eq!(sibling_rows(&items, &songs, 7), [None; SIBLING_SLOTS], "a row past the list is none either");
    assert_eq!(sibling_rows(&[SelectItem::Song(9)], &songs, 0), [None; SIBLING_SLOTS], "nor is a chart the library does not hold");
}

#[test]
fn only_the_charts_on_disk_are_counted_by_the_lamp_the_player_holds_on_them() {
    let song = |lamp: Option<u8>, kind: BarKind| SelectBar { lamp, ..SelectBar::new(kind, "") };
    let bars = [
        song(None, BarKind::Song { exists: true }),
        song(Some(5), BarKind::Song { exists: true }),
        song(Some(5), BarKind::Song { exists: true }),
        song(Some(10), BarKind::Song { exists: true }),
        song(Some(3), BarKind::Song { exists: false }),
        song(Some(4), BarKind::Folder),
        song(Some(2), BarKind::Course { complete: true }),
        song(Some(99), BarKind::Song { exists: true }),
    ];

    assert_eq!(lamp_counts(&bars), [1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1]);
    assert_eq!(lamp_counts(&[]), [0; LAMP_KINDS]);
}

/// A play of a chart of 100 notes with `perfect` PGREATs, dated `date`.
fn play(date: i64, perfect: u32, clear: u8, miss: u32) -> PlayLog {
    PlayLog {
        judge: JudgeCounts::from_early_late([perfect, 0, 0, 0, 0, 0], [0; 6]),
        notes: 100,
        clear,
        minbp: miss,
        date,
        rule_version: REFERENCE_RECORD_RULE_VERSION,
        ..PlayLog::default()
    }
}

/// The best of a chart of 100 notes with `perfect` PGREATs, whose lamp is `clear`.
fn best(perfect: u32, clear: u8, miss: i64) -> BestScore {
    BestScore { judge: JudgeCounts::from_early_late([perfect, 0, 0, 0, 0, 0], [0; 6]), notes: 100, clear, minbp: miss, date: 9_999, ..BestScore::default() }
}

fn clock(date: i64) -> String {
    local_time(date).map(clock_text).expect("the machine has a date for it")
}

#[test]
fn the_best_and_the_five_latest_plays_make_the_rows_and_the_best_is_dated_by_the_play_that_first_scored_it() {
    let plays =
        [play(700, 50, 5, 9), play(600, 60, 5, 8), play(500, 90, 4, 3), play(400, 45, 1, 20), play(300, 90, 5, 2), play(200, 30, 1, 30), play(100, 20, 1, 40)];
    let rows = record_rows(&plays, Some(&best(90, 5, 2)));

    let held: Vec<_> = rows.iter().map(|row| row.as_ref().map(|row| row.ex_score)).collect();
    assert_eq!(held, [Some(180), Some(100), Some(120), Some(180), Some(90), Some(180)], "the best, then the five latest plays newest first");
    let dates: Vec<_> = rows.iter().map(|row| row.as_ref().map(|row| row.date.clone())).collect();
    assert_eq!(dates, [300, 700, 600, 500, 400, 300].map(|date| Some(clock(date))), "the best is dated by the first play that scored it");
    let best_row = rows[0].as_ref().expect("the best has a row");
    assert_eq!((best_row.lamp, best_row.miss_count), (5, 2), "the lamp and the bad-poor count are the chart's best");
    assert_eq!(best_row.dj_level, i32::try_from(dj_rank(180, 200)).expect("a level fits"));
    let latest = rows[1].as_ref().expect("the newest play has a row");
    assert_eq!((latest.lamp, latest.miss_count), (5, 9), "a later row is the play it names");
}

#[test]
fn a_chart_played_fewer_times_than_there_are_rows_leaves_the_rows_after_them_empty() {
    let rows = record_rows(&[play(200, 40, 1, 12), play(100, 20, 1, 30)], Some(&best(40, 1, 12)));

    assert_eq!(rows.iter().map(Option::is_some).collect::<Vec<_>>(), [true, true, true, false, false, false]);
    assert!(record_rows(&[], None).iter().all(Option::is_none), "a chart never played has no row at all");
}

#[test]
fn a_chart_with_no_play_that_may_set_a_best_has_no_best_row_and_still_lists_its_plays() {
    let assisted_only = BestScore { minbp: rbms_store::scoredb::UNSET_MINIMUM, ..best(0, 2, 0) };
    let rows = record_rows(&[play(100, 30, 2, 8)], Some(&assisted_only));

    assert!(rows[0].is_none(), "a best that was never set is not shown");
    assert_eq!(rows[1].as_ref().map(|row| row.ex_score), Some(60));
}

#[test]
fn the_date_of_the_best_is_the_first_unassisted_play_that_scored_it_and_none_when_that_play_is_not_in_the_log() {
    let assisted = |date: i64| PlayLog { assist: 2, ..play(date, 90, 5, 2) };
    let rows = record_rows(&[assisted(300), play(200, 90, 5, 2), play(100, 90, 5, 2)], Some(&best(90, 5, 2)));
    assert_eq!(rows[0].as_ref().map(|row| row.date.clone()), Some(clock(100)), "the first to score it, not the latest");

    let only_assisted = record_rows(&[assisted(300)], Some(&best(90, 5, 2)));
    assert_eq!(only_assisted[0].as_ref().map(|row| row.date.as_str()), Some(""), "an assisted play set no best, so none dates it");

    let older = record_rows(&[play(200, 40, 1, 12)], Some(&best(90, 5, 2)));
    assert_eq!(older[0].as_ref().map(|row| row.date.as_str()), Some(""), "the play that scored the best is beyond the log that was read");
}

/// The titles of the fixture's charts, three of one song and a fourth of another.
const SONG: &str = "Song";

/// The md5 of the seven-key chart of `title` in difficulty slot `difficulty` that [`browsing`] makes.
fn md5(title: &str, difficulty: i32) -> String {
    format!("md5-pack-{title}-{difficulty}-{}", Mode::BEAT_7K.name)
}

/// A browser a skin draws over three charts of one song and one of another, all in one folder.
fn browsing(tag: &str) -> Browser {
    let songs = vec![
        chart("pack", SONG, 1, Mode::BEAT_7K),
        chart("pack", SONG, 3, Mode::BEAT_7K),
        chart("pack", SONG, 4, Mode::BEAT_7K),
        chart("pack", "Zeta", 3, Mode::BEAT_7K),
    ];
    Browser::skinned(tag, songs)
}

/// The row of the list the chart named `md5` is on.
fn row_of(browser: &Browser, md5: &str) -> usize {
    let songs = browser.app.shared.library.songs();
    browser
        .app
        .shared
        .select_items
        .iter()
        .position(|item| matches!(item, SelectItem::Song(index) if songs.get(*index).is_some_and(|song| song.md5 == md5)))
        .expect("the chart is listed")
}

fn run_event(browser: &mut Browser, slot: i32) {
    browser.app.shared.skin_requests().push(Cluster::Select, ClusterRequest::Event { id: RBMS_EVENT_SIBLING_CHART_FIRST + slot, arg1: 0, arg2: 0 });
    browser.frame_after(SHORT_STEP);
}

#[test]
fn the_other_charts_of_the_song_under_the_cursor_are_kept_for_the_skin_and_follow_the_cursor() {
    let mut browser = browsing("browse-siblings");
    let hyper = row_of(&browser, &md5(SONG, 3));
    browser.app.shared.sel = hyper;
    browser.frame_after(SHORT_STEP);

    let levels = browser.state.browse.facts.siblings.map(|chart| chart.map(|chart| chart.level));
    assert_eq!(levels, [Some(3), None, Some(9), Some(12), None], "the level of each chart in the slot of its difficulty");

    browser.app.shared.sel = row_of(&browser, &md5("Zeta", 3));
    browser.frame_after(SHORT_STEP);
    let levels = browser.state.browse.facts.siblings.map(|chart| chart.map(|chart| chart.level));
    assert_eq!(levels, [None, None, Some(9), None, None], "another song has charts of its own");
}

#[test]
fn the_five_events_after_the_first_are_the_slots_and_the_ids_either_side_of_them_are_not() {
    let slots: Vec<_> = (0..SIBLING_SLOTS).map(|slot| event_of(RBMS_EVENT_SIBLING_CHART_FIRST + i32::try_from(slot).expect("a slot fits"))).collect();

    assert_eq!(slots, (0..SIBLING_SLOTS).map(|slot| Some(SelectEvent::SiblingChart(slot))).collect::<Vec<_>>());
    assert_eq!(event_of(RBMS_EVENT_SIBLING_CHART_FIRST - 1), None);
    assert_eq!(event_of(RBMS_EVENT_SIBLING_CHART_FIRST + i32::try_from(SIBLING_SLOTS).expect("the slots fit")), None);
}

#[test]
fn an_event_of_a_slot_moves_the_cursor_to_the_chart_in_it_and_an_empty_slot_or_the_chart_itself_moves_nothing() {
    let mut browser = browsing("browse-sibling-events");
    let beginner = row_of(&browser, &md5(SONG, 1));
    let another = row_of(&browser, &md5(SONG, 4));
    browser.app.shared.sel = beginner;
    browser.frame_after(SHORT_STEP);

    run_event(&mut browser, 3);
    assert_eq!(browser.app.shared.sel, another, "the fourth slot is ANOTHER");
    run_event(&mut browser, 1);
    assert_eq!(browser.app.shared.sel, another, "an empty slot leaves the cursor");
    run_event(&mut browser, 3);
    assert_eq!(browser.app.shared.sel, another, "so does the chart the cursor is on");
    run_event(&mut browser, 0);
    assert_eq!(browser.app.shared.sel, beginner, "the first slot is BEGINNER");
    assert_eq!(browser.app.shared.skin_requests().len(), 0, "an event was left in the queue");
}

#[test]
fn an_event_of_a_slot_does_nothing_on_the_course_tab_or_on_a_bar_that_is_no_chart() {
    let mut browser = browsing("browse-sibling-events-elsewhere");
    let beginner = row_of(&browser, &md5(SONG, 1));
    browser.app.shared.sel = beginner;
    browser.frame_after(SHORT_STEP);

    browser.state.tab = SelectTab::Courses;
    run_event(&mut browser, 3);
    assert_eq!(browser.app.shared.sel, beginner, "the course tab has no song to take the cursor round");

    browser.state.tab = SelectTab::Songs;
    browser.app.shared.open_for_tests([]);
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame_after(SHORT_STEP);
    assert!(matches!(browser.app.shared.select_items.first(), Some(SelectItem::Folder { .. })), "the root of the library lists folders");
    run_event(&mut browser, 3);
    assert_eq!(browser.app.shared.sel, 0, "a folder has no charts of a song");
}

#[test]
fn the_lamps_of_the_open_folder_are_counted_again_when_a_run_is_recorded() {
    let mut browser = browsing("browse-lamps");
    assert_eq!(browser.state.browse.facts.folder_lamps, [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], "four charts and no play");

    let book = ScoreBook::from_records(vec![
        record(&md5(SONG, 3), 5, 100, 0, 1_000),
        record(&md5(SONG, 4), 5, 100, 0, 2_000),
        record(&md5("Zeta", 3), 10, 100, 0, 3_000),
    ]);
    browser.app.shared.scores = book;
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.browse.facts.folder_lamps, [1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1]);
}

#[test]
fn the_plays_of_the_chart_under_the_cursor_are_read_from_the_score_database_when_the_cursor_comes_to_it() {
    let mut browser = browsing("browse-records");
    let hyper = md5(SONG, 3);
    let played =
        |played_at: i64, perfect: u32| ScoreRecord { counts: [perfect, 0, 0, 0, 0, 0], total_notes: 100, max_ex: 200, ..record(&hyper, 5, 0, 0, played_at) };
    let book = ScoreBook::from_records(vec![played(1_700_000_100_000, 80), played(1_700_000_200_000, 95), played(1_700_000_300_000, 60)]);
    let mut database = ScoreDb::open_in_memory().expect("a database in memory opens");
    database.migrate().expect("the layout is created");
    migrate_score_book(&mut database, &book).expect("the plays are stored");
    browser.app.shared.scoredb = Some(database);
    browser.app.shared.scores = book;

    browser.app.shared.sel = row_of(&browser, &md5("Zeta", 3));
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.browse.facts.records.iter().all(Option::is_none), "a chart never played has no row");

    browser.app.shared.sel = row_of(&browser, &hyper);
    browser.frame_after(SHORT_STEP);
    let rows = &browser.state.browse.facts.records;
    let held: Vec<_> = rows.iter().map(|row| row.as_ref().map(|row| row.ex_score)).collect();
    assert_eq!(held, [Some(190), Some(120), Some(190), Some(160), None, None], "the best, then the plays newest first");
    assert_eq!(rows[1].as_ref().map(|row| row.date.clone()), Some(clock(1_700_000_300)));
    assert_eq!(rows[0].as_ref().map(|row| row.date.clone()), Some(clock(1_700_000_200)), "the best is dated by the play that made it");

    browser.app.shared.scoredb = None;
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.browse.facts.records[0].is_some(), "the database was asked again with nothing changed");
}
