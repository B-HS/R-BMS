//! The folders of the browser as a stack (`BarManager.dir`): every way out of a folder puts the
//! cursor back on the bar the folder was opened from, at any depth, and whatever changes under the
//! browser -- a table arriving, a search, a run, the table manager -- leaves it in the folders it
//! was in. The root holds a group of tables under one row, and a level of a table is titled with the
//! symbol the table names.
//!
//! The tables are put in the browser by hand, as a scan leaves them, unless a test is about a table
//! arriving; nothing asks the network. The frames of a group and of a level inside it are saved for
//! a person to open when the capture variable names a folder.

use rbms_config::TableSource;
use rbms_render::skin_render::frame::BarKind;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::list::ChartFacts;
use super::search_tests::stroke;
use super::skinned_tests::{Browser, KEY_TWO, SLOT_ABOVE, SLOT_BELOW};
use super::tests::{entry, press};
use super::*;
use crate::select_tree::{BarIdentity, ListPlace, SelectFolder, TableRef, show_the_list_again};
use crate::stage::{FrameCtx, StageHandler, TablesState};
use crate::tablesrc::TableExtras;
use crate::tablesrc::test_outside::FakeTables;
use crate::user_folders::ShownFolder;
use crate::{CH, CW};

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
