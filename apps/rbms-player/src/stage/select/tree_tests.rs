//! The folders of the browser as a stack (`BarManager.dir`): every way out of a folder puts the
//! cursor back on the bar the folder was opened from, at any depth, and whatever changes under the
//! browser -- a table arriving, a search, a run, the table manager -- leaves it in the folders it
//! was in. The root holds a group of tables under one row, and a level of a table is titled with the
//! symbol the table names.
//!
//! A table that lists courses holds them in a folder after its levels, and the 発狂BMS難易度表 every
//! player is given holds the GENOSIDE 2018 courses the build carries there as well.
//!
//! The tables are put in the browser by hand, as a scan leaves them, unless a test is about a table
//! arriving; nothing asks the network. The frames of a group and of a level inside it are saved for
//! a person to open when the capture variable names a folder.

use rbms_config::TableSource;
use rbms_render::skin_render::frame::BarKind;
use rbms_skin::property::generated::BUTTON_PRACTICE;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::list::ChartFacts;
use super::search_tests::stroke;
use super::skinned_tests::{Browser, KEY_THREE, KEY_TWO, SLOT_ABOVE, SLOT_BELOW};
use super::tests::{entry, press};
use super::*;
use crate::select_tree::{BarIdentity, ListPlace, SelectFolder, TableRef, show_the_list_again};
use crate::stage::{FrameCtx, StageHandler, TablesState};
use crate::tablesrc::TableExtras;
use crate::tablesrc::test_outside::FakeTables;
use crate::user_folders::ShownFolder;
use crate::{CH, CW, CourseRef};

/// Where the frames below are saved for a person to open, when this names a folder: the variable
/// the capture harness reads.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The charts every browser here lists, in title order.
const TITLES: [&str; 6] = ["alba", "alder", "alpha", "altair", "amber", "angel"];

/// The group the tests file tables under.
const GROUP: &str = "STELLA";

/// A difficulty table as the browser holds it after a scan: its row of the list, its symbol, and
/// the owned charts of each of its levels.
struct Listed {
    name: &'static str,
    group: &'static str,
    symbol: Option<&'static str>,
    levels: &'static [(&'static str, &'static [usize])],
}

/// A table of its own, with no symbol, of the levels `1` to `3`.
const PLAIN: Listed = Listed { name: "PLAIN", group: "", symbol: None, levels: &[("1", &[0]), ("2", &[1]), ("3", &[2, 3])] };

/// Where the table at `at` of the browser `tag` is registered.
fn address(tag: &str, at: usize) -> String {
    format!("https://tables.test/tree/{tag}/{at}/")
}

/// A browser over the charts above holding `tables`, at the top of the library; drawn by the
/// fixture skin when `skinned`, and by nothing but itself otherwise.
fn browser_with(tag: &str, skinned: bool, tables: &[Listed]) -> Browser {
    let songs = TITLES.map(|title| entry(title, "a", "5")).to_vec();
    let mut browser = if skinned { Browser::skinned(tag, songs) } else { Browser::over(tag, songs, false) };
    let shared = &mut browser.app.shared;
    shared.config.library.tables =
        tables.iter().enumerate().map(|(at, table)| TableSource { name: table.name.into(), location: address(tag, at), group: table.group.into() }).collect();
    shared.table_names = tables.iter().map(|table| table.name.to_owned()).collect();
    shared.table_levels = tables.iter().map(|table| table.levels.iter().map(|(level, charts)| ((*level).to_owned(), charts.to_vec())).collect()).collect();
    shared.table_extras = tables.iter().map(|table| TableExtras { symbol: table.symbol.map(str::to_owned), ..TableExtras::default() }).collect();
    shared.open_for_tests([]);
    shared.sel = 0;
    shared.rebuild_select_items();
    browser.frame();
    browser
}

/// The rows of the browser as it stands: a chart by its title, a folder by its label.
fn rows(browser: &Browser) -> Vec<String> {
    let songs = browser.app.shared.library.songs();
    browser
        .app
        .shared
        .select_items
        .iter()
        .map(|item| match item {
            SelectItem::Song(index) => songs[*index].title.clone(),
            SelectItem::Folder { label, .. } => label.clone(),
            SelectItem::Missing(missing) => browser.app.shared.missing_chart(missing).map(|chart| chart.title.clone()).unwrap_or_default(),
            SelectItem::Course(course) => browser.app.shared.table_course(course).map(|course| course.name.clone()).unwrap_or_default(),
        })
        .collect()
}

/// Put the cursor on `row` and enter what is there.
fn enter(browser: &mut Browser, row: usize) -> Transition {
    browser.app.shared.sel = row;
    let asked = browser.key(KeyCode::Enter);
    browser.frame();
    asked
}

/// Leave the folder open innermost with Esc.
fn escape(browser: &mut Browser) {
    browser.key(KeyCode::Escape);
    browser.frame();
}

/// What the bar on `row` is.
fn identity_at(browser: &Browser, row: usize) -> Option<BarIdentity> {
    browser.app.shared.select_items.get(row).and_then(|item| browser.app.shared.identity_of(item))
}

/// The browser over the group of two tables and one of its levels: `STELLA > S1 > st3`, with the
/// cursor on the second chart of the level.
fn inside_the_group(tag: &str, skinned: bool) -> Browser {
    let tables = [
        PLAIN,
        Listed { name: "S1", group: GROUP, symbol: Some("st"), levels: &[("0", &[4]), ("3", &[0, 1, 2])] },
        Listed { name: "S2", group: GROUP, symbol: Some("sl"), levels: &[("1", &[5])] },
    ];
    let mut browser = browser_with(tag, skinned, &tables);
    enter(&mut browser, 2);
    enter(&mut browser, 0);
    enter(&mut browser, 1);
    browser.app.shared.sel = 1;
    browser.frame();
    browser
}

/// Save the frame the browser last drew as `<name>.png` in the capture folder, when one is named.
fn capture(browser: &Browser, name: &str) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    let folder = std::path::PathBuf::from(folder);
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    let file = folder.join(format!("{name}.png"));
    image::save_buffer(file, browser.pixels.rgba(), CW, CH, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

#[test]
fn leaving_a_level_puts_the_cursor_on_that_level() {
    let mut browser = browser_with("tree-leave-level", false, &[PLAIN]);
    enter(&mut browser, 1);
    assert_eq!(rows(&browser), ["LV 1 (1)", "LV 2 (1)", "LV 3 (2)"]);
    enter(&mut browser, 2);
    assert_eq!(rows(&browser), ["alpha", "altair"]);

    escape(&mut browser);
    let table = browser.app.shared.table_folder(0);
    assert!(browser.app.shared.opened() == [table], "the browser did not go back to the table");
    assert_eq!(browser.sel(), 2, "the cursor went back to the first level");
    let level = SelectFolder::TableLevel(TableRef::of(&browser.app.shared, 0), "3".to_owned());
    assert_eq!(identity_at(&browser, 2), Some(BarIdentity::Opens(level)));
}

#[test]
fn leaving_a_table_puts_the_cursor_on_that_table_at_the_root() {
    let mut browser = browser_with("tree-leave-table", false, &[PLAIN, Listed { name: "SECOND", ..PLAIN }]);
    assert_eq!(rows(&browser), ["ALL SONGS (6)", "PLAIN (4)", "SECOND (4)"]);
    enter(&mut browser, 2);
    escape(&mut browser);
    assert!(browser.app.shared.is_at_root());
    assert_eq!(browser.sel(), 2, "the cursor went back to the first row of the root");
}

#[test]
fn leaving_my_folders_and_a_folder_of_them_puts_the_cursor_on_each() {
    let mut browser = browser_with("tree-leave-my-folders", false, &[PLAIN]);
    let folder =
        |id: &str, charts: Vec<usize>| ShownFolder { id: id.to_owned(), name: id.to_ascii_uppercase(), listed: charts.len(), owned: charts.len(), charts };
    browser.app.shared.user_folders.shown = vec![folder("one", vec![0]), folder("two", vec![1, 2])];
    browser.app.shared.rebuild_select_items();
    browser.frame();
    assert_eq!(rows(&browser), ["ALL SONGS (6)", "PLAIN (4)", "MY FOLDERS (3)"]);

    enter(&mut browser, 2);
    enter(&mut browser, 1);
    assert_eq!(rows(&browser), ["alder", "alpha"]);
    escape(&mut browser);
    assert!(browser.app.shared.opened() == [SelectFolder::UserFolders]);
    assert_eq!(browser.sel(), 1, "the cursor is not on the folder that was left");
    escape(&mut browser);
    assert!(browser.app.shared.is_at_root());
    assert_eq!(browser.sel(), 2, "the cursor is not on MY FOLDERS");
}

/// One way out of a folder: what it is called, whether it needs a skin to draw the browser, and
/// what it does.
struct WayOut {
    name: &'static str,
    skinned: bool,
    leave: fn(&mut Browser),
}

/// Esc, the left arrow, a black key of the controller, the skin's FOLDER_CLOSE key and a press of
/// the skin's other button all leave a folder the same way.
#[test]
fn every_way_out_of_a_folder_puts_the_cursor_on_the_same_bar() {
    let ways: [WayOut; 5] = [
        WayOut { name: "escape", skinned: false, leave: escape },
        WayOut {
            name: "left",
            skinned: false,
            leave: |browser| {
                browser.key(KeyCode::ArrowLeft);
                browser.frame();
            },
        },
        WayOut {
            name: "black-key",
            skinned: false,
            leave: |browser| {
                browser.pad(1);
                browser.frame();
            },
        },
        WayOut {
            name: "folder-close",
            skinned: true,
            leave: |browser| {
                browser.tap(KEY_TWO);
            },
        },
        WayOut {
            name: "close-bar",
            skinned: true,
            leave: |browser| {
                browser.click(SLOT_ABOVE, MouseButton::Right);
            },
        },
    ];
    for WayOut { name, skinned: is_skinned, leave } in ways {
        let mut browser = browser_with(&format!("tree-way-out-{name}"), is_skinned, &[PLAIN]);
        enter(&mut browser, 1);
        enter(&mut browser, 2);
        leave(&mut browser);
        let table = browser.app.shared.table_folder(0);
        assert!(browser.app.shared.opened() == [table], "{name}: the browser did not go back to the table");
        assert_eq!(browser.sel(), 2, "{name}: the cursor is not on the level that was left");
    }
}

/// A folder opened by a press on a bar off the cursor comes back to the bar the cursor was on, and
/// to the first bar when that bar is gone by then.
#[test]
fn a_folder_whose_bar_is_gone_when_left_puts_the_cursor_first() {
    let mut browser = browser_with("tree-bar-gone", true, &[PLAIN]);
    enter(&mut browser, 1);
    assert!(matches!(browser.click(SLOT_BELOW, MouseButton::Left), Transition::Stay));
    let shared = &mut browser.app.shared;
    assert!(shared.opened() == [shared.table_folder(0), shared.level_folder(0, 1)], "the bar below the cursor was not the one opened");

    let place = ListPlace::of(shared);
    shared.table_levels[0].remove(0);
    show_the_list_again(shared, &place);
    assert!(shared.opened() == [shared.table_folder(0), shared.level_folder(0, 0)], "the level open was lost with the one before it");
    browser.frame();
    escape(&mut browser);
    assert_eq!(rows(&browser), ["LV 2 (1)", "LV 3 (2)"]);
    assert_eq!(browser.sel(), 0, "the cursor was not put on the first bar");
}

#[test]
fn a_group_holds_its_tables_where_its_first_table_is() {
    let tables = [
        Listed { name: "A", ..PLAIN },
        Listed { name: "S1", group: GROUP, ..PLAIN },
        Listed { name: "B", ..PLAIN },
        Listed { name: "S2", group: GROUP, levels: &[("1", &[5])], ..PLAIN },
    ];
    let mut browser = browser_with("tree-group", false, &tables);
    assert_eq!(rows(&browser), ["ALL SONGS (6)", "A (4)", "STELLA (5)", "B (4)"]);
    let bars = browser.app.shared.select_bars(&ChartFacts::default(), None, false);
    assert_eq!(bars.iter().map(|bar| bar.kind).collect::<Vec<_>>(), [BarKind::Folder, BarKind::Table, BarKind::Command, BarKind::Table]);
    capture(&browser, "folders-group");

    enter(&mut browser, 2);
    assert!(browser.app.shared.opened() == [SelectFolder::TableGroup(GROUP.to_owned())]);
    assert_eq!(rows(&browser), ["S1 (4)", "S2 (1)"]);
    enter(&mut browser, 1);
    escape(&mut browser);
    assert_eq!(browser.sel(), 1, "the cursor is not on the table that was left");
    escape(&mut browser);
    assert_eq!(browser.sel(), 2, "the cursor is not on the group that was left");
}

#[test]
fn a_group_none_of_whose_tables_has_levels_is_no_row() {
    let tables =
        [Listed { name: "A", ..PLAIN }, Listed { name: "S1", group: GROUP, levels: &[], ..PLAIN }, Listed { name: "S2", group: GROUP, levels: &[], ..PLAIN }];
    let browser = browser_with("tree-empty-group", false, &tables);
    assert_eq!(rows(&browser), ["ALL SONGS (6)", "A (4)"]);
}

#[test]
fn a_level_is_titled_with_the_symbol_its_table_names() {
    let mut browser = browser_with("tree-symbol", false, &[Listed { name: "ST", symbol: Some("st"), ..PLAIN }, PLAIN]);
    enter(&mut browser, 1);
    assert_eq!(rows(&browser), ["st1 (1)", "st2 (1)", "st3 (2)"]);
    escape(&mut browser);
    enter(&mut browser, 2);
    assert_eq!(rows(&browser), ["LV 1 (1)", "LV 2 (1)", "LV 3 (2)"], "a table that names no symbol has its levels titled as before");
}

#[test]
fn the_directory_string_is_the_titles_of_the_open_folders() {
    let mut browser = inside_the_group("tree-directory", false);
    assert_eq!(browser.app.shared.open_titles(), ["STELLA", "S1", "st3"]);
    let shown = browser.app.shared.select_shown(None, &crate::skin_host::select::BrowserLent::default());
    assert_eq!(shown.directory_path(), "STELLA > S1 > st3 > ");
    capture(&browser, "folders-nested-level");
    escape(&mut browser);
    escape(&mut browser);
    escape(&mut browser);
    let shown = browser.app.shared.select_shown(None, &crate::skin_host::select::BrowserLent::default());
    assert_eq!(shown.directory_path(), "", "the top of the library is no folder");
}

#[test]
fn a_chart_played_inside_a_nested_level_comes_back_to_it() {
    let mut browser = inside_the_group("tree-played", false);
    let before = (browser.app.shared.opened(), browser.sel());
    assert!(matches!(enter(&mut browser, 1), Transition::Open(_)), "the chart did not start");
    assert!(matches!(browser.app.shared.leave_play(), Transition::Back));
    browser.come_back();
    browser.frame();
    assert_eq!((browser.app.shared.opened(), browser.sel()), before);
    assert_eq!(rows(&browser)[browser.sel()], "alder");
}

#[test]
fn a_search_made_in_a_nested_level_is_left_for_that_level() {
    let mut browser = inside_the_group("tree-search", false);
    let before = (browser.app.shared.opened(), browser.sel());
    stroke(&mut browser, KeyCode::Slash, Some("/"));
    stroke(&mut browser, KeyCode::KeyA, Some("a"));
    browser.frame();
    assert!(browser.app.shared.opened() == [SelectFolder::AllSongs], "a search has to span the whole library");
    stroke(&mut browser, KeyCode::Escape, None);
    browser.frame();
    assert_eq!((browser.app.shared.opened(), browser.sel()), before);
}

#[test]
fn a_table_arriving_keeps_a_nested_path_and_its_cursor() {
    let tables = FakeTables::default();
    let _installed = tables.install();
    let mut browser = inside_the_group("tree-arrival", false);
    let before = (browser.app.shared.open_titles(), identity_at(&browser, browser.sel()));
    let arriving = address("tree-arrival", 9);
    tables.serve(&arriving, "Arrived", br#"[{"md5":"md5-alba","level":"1"}]"#);
    crate::tablesrc::register_table(&mut browser.app.shared, TableSource { name: String::new(), location: arriving.clone(), group: GROUP.to_owned() });
    while !browser.app.shared.table_fetches.is_idle() {
        assert!(matches!(browser.frame(), Transition::Stay));
        std::thread::yield_now();
    }
    browser.frame();
    assert_eq!(browser.app.shared.table_names.last().map(String::as_str), Some("ARRIVED"), "the table did not arrive");
    assert_eq!((browser.app.shared.open_titles(), identity_at(&browser, browser.sel())), before);
    crate::tablesrc::forget_table(&browser.app.shared, &arriving);
}

#[test]
fn a_table_removed_from_the_list_cuts_the_path_back_to_what_is_there() {
    for (inside, removed, kept) in [(1, 0, None), (1, 1, Some("PLAIN")), (2, 0, Some("SECOND"))] {
        let tag = format!("tree-removed-{inside}-{removed}");
        let mut browser = browser_with(&tag, false, &[PLAIN, Listed { name: "SECOND", ..PLAIN }]);
        enter(&mut browser, inside);
        enter(&mut browser, 2);
        let mut tables = TablesState::new();
        let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
        if removed > 0 {
            tables.handle_key(&mut ctx, press(KeyCode::ArrowDown));
        }
        tables.handle_key(&mut ctx, press(KeyCode::KeyD));
        assert!(matches!(tables.handle_key(&mut ctx, press(KeyCode::Escape)), Transition::Back));
        browser.come_back();
        browser.frame();
        match kept {
            None => assert!(browser.app.shared.is_at_root(), "the browser is still inside the table that was removed"),
            Some(name) => {
                assert_eq!(browser.app.shared.open_titles(), [name, "LV 3"], "the browser left the table that was not removed");
                assert_eq!(rows(&browser), ["alpha", "altair"]);
            }
        }
    }
}

#[test]
fn two_tables_of_the_same_name_are_told_apart_by_their_address() {
    let mut browser = browser_with("tree-same-name", false, &[PLAIN, Listed { levels: &[("9", &[5])], ..PLAIN }]);
    assert_eq!(rows(&browser), ["ALL SONGS (6)", "PLAIN (4)", "PLAIN (1)"]);
    enter(&mut browser, 2);
    assert_eq!(rows(&browser), ["LV 9 (1)"]);
    escape(&mut browser);
    assert_eq!(browser.sel(), 2, "the cursor went to the first table of the name");
}

/// Where the 発狂BMS難易度表 every player is given is registered, whose folder of courses holds the
/// bundled GENOSIDE 2018 courses.
const INSANE_LOCATION: &str = "https://bms.hyuns.uk/tables/insane1";

/// A stage of the courses below that the library does not hold.
const ABSENT_MD5: &str = "00000000000000000000000000000000";

/// How many GENOSIDE 2018 courses the build carries: single play, then double play.
const BUNDLED_COURSES: usize = 44;

/// The md5s of the first bundled course, quoted for a table header.
fn first_bundled_md5s() -> String {
    let first = &rbms_course::genoside_2018().courses().next().expect("a bundled course").course;
    first.charts.iter().map(|chart| format!("\"{}\"", chart.md5)).collect::<Vec<_>>().join(",")
}

/// The courses of a header `course` field written as one group: a course of two owned charts, one
/// with a stage the library lacks, an entry that is no course, a course of one owned chart, and the
/// first bundled course under a name of the table's own.
fn listed_course_group() -> String {
    format!(
        r#"[{{"name":"DAN A","md5":["md5-alba","md5-alder"]}},{{"name":"DAN B","md5":["md5-alpha","{ABSENT_MD5}"]}},{{"name":"broken"}},{{"name":"DAN C","md5":["md5-altair"]}},{{"name":"MY SHODAN","md5":[{}]}}]"#,
        first_bundled_md5s()
    )
}

/// Give the table at `at` of the browser the courses its header lists as `course`, read the way a
/// table that arrives is read, and rebuild the list where it stands.
fn list_courses(browser: &mut Browser, at: usize, course: &str) {
    let header = rbms_table::parse_header(format!(r#"{{"name":"COURSED","course":{course}}}"#).as_bytes()).expect("the header reads");
    let table = rbms_table::DifficultyTable::from_parts(Some(header), Vec::new());
    let index = rbms_table::Md5Index::new(browser.app.shared.library.md5s());
    let courses = crate::tablesrc::MatchedTable::of(String::new(), &table, &index).extras.courses;
    let shared = &mut browser.app.shared;
    let place = ListPlace::of(shared);
    shared.table_extras[at].courses = courses;
    show_the_list_again(shared, &place);
    browser.frame();
}

/// A browser over two tables of their own, `PLAIN` and `SECOND`, at the top of the library. The
/// header of the first lists [`listed_course_group`] in the shape the Stella tables write, a list
/// of groups.
fn browser_with_courses(tag: &str) -> Browser {
    browser_with_courses_drawn_by(tag, false)
}

#[test]
fn a_table_that_lists_courses_has_a_course_folder_after_its_levels() {
    let mut browser = browser_with_courses("tree-courses-folder");
    enter(&mut browser, 1);
    assert_eq!(rows(&browser), ["LV 1 (1)", "LV 2 (1)", "LV 3 (2)", "COURSE (4)"], "the entry that is no course was counted");
    enter(&mut browser, 3);
    let table = TableRef::of(&browser.app.shared, 0);
    assert!(browser.app.shared.opened() == [SelectFolder::Table(table.clone()), SelectFolder::TableCourses(table)]);
    assert_eq!(browser.app.shared.open_titles(), ["PLAIN", "COURSE"]);
    assert_eq!(
        rows(&browser),
        ["DAN A", "DAN B", "DAN C", "GENOSIDE 2018 段位認定 初段"],
        "the courses are not in the header's order, or the bundled course was not taken for its copy"
    );

    escape(&mut browser);
    escape(&mut browser);
    enter(&mut browser, 2);
    assert_eq!(rows(&browser), ["LV 1 (1)", "LV 2 (1)", "LV 3 (2)"], "a table that lists no course has a folder of courses");
}

#[test]
fn a_course_whose_charts_are_all_owned_starts_and_one_missing_a_chart_says_so() {
    let mut browser = browser_with_courses("tree-courses-start");
    enter(&mut browser, 1);
    enter(&mut browser, 3);

    assert!(matches!(enter(&mut browser, 1), Transition::Stay), "a course with a stage the library lacks started");
    assert!(browser.app.shared.course_run.is_none());

    assert!(!matches!(enter(&mut browser, 0), Transition::Stay), "a course whose every chart is owned did not start");
    let run = browser.app.shared.course_run.as_ref().expect("the course is being played");
    assert_eq!(run.course.name, "DAN A");
    assert_eq!(run.index, 0, "the course did not start on its first stage");
}

/// A browser over the tables [`browser_with_courses`] holds, drawn by the fixture skin when `skinned`.
fn browser_with_courses_drawn_by(tag: &str, skinned: bool) -> Browser {
    let mut browser = browser_with(tag, skinned, &[PLAIN, Listed { name: "SECOND", ..PLAIN }]);
    list_courses(&mut browser, 0, &format!("[{}]", listed_course_group()));
    browser
}

/// A course has no practice screen, so the key that opens it on a chart starts the course as it is
/// on a bar of the folder of the courses a table lists, as the reference does
/// (`MusicSelector.render`: a practice that reaches a course bar is turned into play): by the
/// browser's own key, and by the key of the reference's table on a skin's browser. A course with a
/// stage the library lacks is said so and does not start.
#[test]
fn the_practice_key_starts_a_course_of_the_course_folder_of_a_table_as_the_reference_does() {
    for skinned in [false, true] {
        let mut browser = browser_with_courses_drawn_by(&format!("tree-courses-practice-{skinned}"), skinned);
        enter(&mut browser, 1);
        enter(&mut browser, 3);
        let practise = |browser: &mut Browser| if skinned { browser.tap(KEY_THREE) } else { browser.key(KeyCode::F4) };

        browser.app.shared.sel = 1;
        browser.frame();
        assert!(matches!(practise(&mut browser), Transition::Stay), "skinned {skinned}: a course with a stage the library lacks started");
        assert!(browser.app.shared.course_run.is_none() && !browser.app.shared.practice_requested, "skinned {skinned}");

        browser.app.shared.sel = 0;
        browser.frame();
        assert!(!matches!(practise(&mut browser), Transition::Stay), "skinned {skinned}: the practice key did not start a course whose every chart is owned");
        let run = browser.app.shared.course_run.as_ref().expect("the course is being played");
        assert_eq!((run.course.name.as_str(), run.index), ("DAN A", 0), "skinned {skinned}");
        assert!(!browser.app.shared.practice_requested, "skinned {skinned}: the course was sent to the practice screen");
    }
}

/// The practice button of a skin starts the course under the cursor the same way.
#[test]
fn the_practice_button_of_a_skin_starts_a_course_of_the_course_folder_of_a_table() {
    let mut browser = browser_with_courses_drawn_by("tree-courses-practice-button", true);
    enter(&mut browser, 1);
    enter(&mut browser, 3);
    browser.app.shared.sel = 0;
    browser.frame();
    let asked = browser.state.run_event(&mut browser.app.shared, BUTTON_PRACTICE, 0);
    assert!(!matches!(asked, Transition::Stay), "the practice button did not start the course");
    assert_eq!(browser.app.shared.course_run.as_ref().map(|run| run.course.name.as_str()), Some("DAN A"));
    assert!(!browser.app.shared.practice_requested, "the course was sent to the practice screen");
}

/// On the course tab of the browser no skin draws the practice key starts the course under the
/// cursor, and does not practise the chart the song list has under its own cursor behind it.
#[test]
fn the_practice_key_starts_the_course_under_the_cursor_of_the_course_tab() {
    let mut browser = browser_with("tree-courses-practice-tab", false, &[PLAIN]);
    let mut own = rbms_course::Course { name: "MINE".to_owned(), ..rbms_course::Course::default() };
    own.charts = vec![rbms_course::CourseChart { md5: "md5-alba".to_owned(), sha256: String::new(), title: String::new() }];
    assert!(own.validate());
    browser.state = SelectState::on_courses(vec![own], &browser.app.shared.library);
    browser.app.shared.open_for_tests([SelectFolder::AllSongs]);
    browser.app.shared.sel = 0;
    browser.frame();
    assert!(!matches!(browser.key(KeyCode::F4), Transition::Stay), "the practice key did not start the course");
    assert_eq!(browser.app.shared.course_run.as_ref().map(|run| run.course.name.as_str()), Some("MINE"));
    assert!(!browser.app.shared.practice_requested, "the chart behind the course tab was practised");
}

#[test]
fn leaving_the_course_folder_puts_the_cursor_on_it() {
    let mut browser = browser_with_courses("tree-courses-leave");
    enter(&mut browser, 1);
    enter(&mut browser, 3);
    browser.app.shared.sel = 2;
    let on_dan_c = identity_at(&browser, 2);
    let dan_c = browser.app.shared.table_course(&CourseRef { table: 0, course: 2 }).expect("the third course").hash();
    assert_eq!(on_dan_c, Some(BarIdentity::Course(dan_c)), "a course bar is not known by its id");

    let group = listed_course_group();
    list_courses(&mut browser, 0, &format!(r#"[[{{"name":"DAN 0","md5":["md5-angel"]}}],{group}]"#));
    assert_eq!(identity_at(&browser, browser.sel()), on_dan_c, "the cursor left the course it was on when a course was put before it");
    assert_eq!(browser.sel(), 3);

    escape(&mut browser);
    let folder = SelectFolder::TableCourses(TableRef::of(&browser.app.shared, 0));
    assert_eq!(browser.sel(), 3, "the cursor is not on the folder of courses that was left");
    assert_eq!(identity_at(&browser, 3), Some(BarIdentity::Opens(folder)));
}

#[test]
fn a_table_course_folder_bar_is_a_table_bar_and_its_courses_are_course_bars() {
    let mut browser = browser_with_courses("tree-courses-bars");
    enter(&mut browser, 1);
    let kinds = |browser: &Browser| browser.app.shared.select_bars(&ChartFacts::default(), None, true).iter().map(|bar| bar.kind).collect::<Vec<_>>();
    assert_eq!(kinds(&browser), [BarKind::Table; 4]);
    enter(&mut browser, 3);
    assert_eq!(
        kinds(&browser),
        [BarKind::Course { complete: true }, BarKind::Course { complete: false }, BarKind::Course { complete: true }, BarKind::Course { complete: false }]
    );
    let bars = browser.app.shared.select_bars(&ChartFacts::default(), None, false);
    assert_eq!(bars[0].course.as_ref().map(|course| course.stages), Some(2), "the course bar does not count its stages");
}

#[test]
fn the_insane_table_course_folder_lists_the_bundled_genoside_courses() {
    let mut browser = browser_with("tree-courses-insane", false, &[PLAIN]);
    browser.app.shared.config.library.tables[0].location = INSANE_LOCATION.to_owned();
    list_courses(&mut browser, 0, &format!(r#"[{{"name":"OWN","md5":["md5-alba"]}},{{"name":"MY SHODAN","md5":[{}]}}]"#, first_bundled_md5s()));
    enter(&mut browser, 1);
    assert_eq!(rows(&browser).last().map(String::as_str), Some("COURSE (45)"), "the bundled course the header copies was listed twice");
    enter(&mut browser, 3);
    let titles = rows(&browser);
    assert_eq!(titles.len(), BUNDLED_COURSES + 1);
    assert_eq!(titles[0], "OWN", "the header's own course is not before the bundled ones");
    assert_eq!(titles[1], "GENOSIDE 2018 段位認定 初段");
    assert_eq!(titles[BUNDLED_COURSES / 2 + 1], "GENOSIDE 2018 DP段位認定 初段", "double play does not follow single play");
    let first = rbms_course::genoside_2018().courses().next().expect("a bundled course");
    assert_eq!(identity_at(&browser, 1), Some(BarIdentity::Course(first.course_hash.clone())), "the bundled course is not known by the id the data states");

    let mut other = browser_with("tree-courses-not-insane", false, &[PLAIN]);
    enter(&mut other, 1);
    assert_eq!(rows(&other), ["LV 1 (1)", "LV 2 (1)", "LV 3 (2)"], "a table other than the 発狂BMS難易度表 was given the bundled courses");
}

/// A fresh folder for the course files of the test `tag`.
fn course_folder(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms-tree-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the course folder can be made");
    dir
}

/// The course tab lists the bundled courses whatever the course folder holds, so the list is read
/// again every time the browser is arrived on: a course file put in the folder since is listed and
/// one taken out is not, with the cursor kept on the course it was on.
#[test]
fn the_course_files_are_read_again_each_time_the_browser_is_arrived_on() {
    let dir = course_folder("course-files");
    let library = Library::from_songs(TITLES.map(|title| entry(title, "a", "5")).to_vec());
    let mut state = SelectState::new();
    state.reload_courses(&dir, &library);
    assert_eq!(state.courses.len(), BUNDLED_COURSES);
    state.courses.focus(2);
    let focused = state.courses.focused().map(|entry| entry.course.hash());

    let mut own = rbms_course::Course { name: "MINE".to_owned(), ..rbms_course::Course::default() };
    own.charts = vec![rbms_course::CourseChart { md5: "md5-alba".to_owned(), sha256: String::new(), title: String::new() }];
    assert!(own.validate());
    rbms_course::save(&dir, &own).expect("the course file is written");
    state.reload_courses(&dir, &library);
    assert_eq!(state.courses.len(), BUNDLED_COURSES + 1, "a course file put in the folder was not listed");
    assert!(state.courses.entries().last().is_some_and(|entry| entry.course.name == "MINE" && entry.is_playable()));
    assert_eq!(state.courses.focused().map(|entry| entry.course.hash()), focused, "the cursor left the course it was on");

    std::fs::remove_dir_all(&dir).expect("the course folder is removed");
    state.reload_courses(&dir, &library);
    assert_eq!(state.courses.len(), BUNDLED_COURSES, "a course file taken out of the folder is still listed");
}
