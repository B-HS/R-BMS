//! The difficulty tables as the browser sees them arrive: a table that has no copy is fetched beside
//! a browser that is already up, and is a folder from the frame it lands, with the browser left
//! where it was.
//!
//! Every address here is one of the test's own ([`FakeTables`]); nothing asks the network. The
//! library is read "as a scan does" without walking a folder: the tables are read the way the
//! LOADING screen reads them and the first fetches are marked due, which is all of a scan that the
//! browser is told about. The scan itself, from the LOADING screen through to the browser, is held
//! to the same in [`crate::stage::loading`]'s own tests.

use std::sync::atomic::AtomicUsize;

use rbms_config::TableSource;
use rbms_library::Library;
use winit::keyboard::KeyCode;

use super::skinned_tests::Browser;
use super::tests::entry;
use super::*;
use crate::notify::Level;
use crate::tablesrc::test_outside::FakeTables;
use crate::tablesrc::{TableStatus, TableStore, forget_table_status, read_and_match_all, register_table, table_is_fetching, table_status};

/// The charts every browser here lists, in title order.
const TITLES: [&str; 4] = ["alba", "alder", "alpha", "altair"];

/// Where the frames below are saved for a person to open, when this names a folder: the variable
/// the capture harness reads.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// A browser the fixture skin draws over the charts above, or one nothing but itself draws.
fn browser(tag: &str, skinned: bool) -> Browser {
    let songs = TITLES.map(|title| entry(title, "a", "5")).to_vec();
    if skinned {
        return Browser::skinned(tag, songs);
    }
    let mut browser = Browser::over(tag, songs, false);
    browser.frame();
    browser.frame();
    browser
}

fn at(address: &str) -> TableSource {
    TableSource { name: String::new(), location: address.to_owned(), group: String::new() }
}

/// A table body that lists the charts titled `titles` at level 1.
fn listing(titles: &[&str]) -> Vec<u8> {
    let entries: Vec<String> = titles.iter().map(|title| format!(r#"{{"md5":"md5-{title}","level":"1"}}"#)).collect();
    format!("[{}]", entries.join(",")).into_bytes()
}

/// Read the browser's tables the way a scan of the library does, against the library as it stands,
/// and leave the browser at the top of it as a scan does.
fn read_as_a_scan_does(browser: &mut Browser) {
    let shared = &mut browser.app.shared;
    let store = TableStore::beside(&shared.settings_path);
    let matched = read_and_match_all(&store, &shared.config.library.tables, &shared.library, &AtomicUsize::new(0));
    shared.set_tables(matched);
    shared.table_fetches.first_fetches_are_due();
    shared.open_for_tests([]);
    shared.sel = 0;
    shared.rebuild_select_items();
}

/// Run the browser's frames until nothing is being fetched any more.
fn land(browser: &mut Browser) {
    while !browser.app.shared.table_fetches.is_idle() {
        assert!(matches!(browser.frame(), Transition::Stay), "the browser left for another screen while a table was out");
        std::thread::yield_now();
    }
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

/// Everything said since the last look.
fn said() -> Vec<(Level, String)> {
    let mut said = Vec::new();
    crate::notify::drain(&mut said);
    said
}

/// A full HD window, which the captures are also drawn for beside the size the browser is laid out
/// for.
const LARGE: (u32, u32) = (1920, 1080);

/// Save the frame the browser last drew as `<name>-<size>.png` in the capture folder, when one is
/// named, and the same frame drawn again at full HD beside it. The browser is left as it was: what
/// the second drawing recorded as clickable is taken away again.
fn capture(browser: &mut Browser, name: &str) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    let folder = std::path::PathBuf::from(folder);
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    let mut large = crate::stage::HeadlessCanvas::new(LARGE.0, LARGE.1);
    let recorded = browser.app.shared.hot.len();
    browser.state.draw(&mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }, &mut Canvas::Headless(&mut large));
    browser.app.shared.hot.truncate(recorded);
    for (frame, size) in [(&browser.pixels, (CW, CH)), (&large, LARGE)] {
        let file = folder.join(format!("{name}-{}x{}.png", size.0, size.1));
        image::save_buffer(file, frame.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}

/// Let go of what a test left of the tables at `addresses`: what became of them, and their copies.
fn forget(browser: &Browser, addresses: &[&str]) {
    let store = TableStore::beside(&browser.app.shared.settings_path);
    for address in addresses {
        forget_table_status(address);
        store.forget(address);
    }
}

/// A table that has no copy is not waited for: the browser is up without it, and fetches it from
/// its first frame, once. When it lands it is kept, it is a folder among the tables in the order
/// they are listed in, and one line says the tables were fetched for the first time. The browser is
/// where it was, with the cursor on the bar it was on -- whichever bar that was, and although the
/// folder that arrived took a place above it.
#[test]
fn a_table_with_no_copy_is_fetched_beside_the_browser_and_its_folder_arrives_under_a_cursor_that_stays() {
    crate::notify::exclusive(|| {
        for (skinned, cursor, cursor_after) in [(false, 0, 0), (false, 1, 2), (true, 1, 2)] {
            let tag = format!("tables-first-{skinned}-{cursor}");
            let (new, kept) = (format!("https://tables.test/browser/first/{tag}/new/"), format!("https://tables.test/browser/first/{tag}/kept/"));
            let tables = FakeTables::default();
            let _installed = tables.install();
            tables.serve(&new, "New", &listing(&["alba", "alder"]));
            let mut browser = browser(&tag, skinned);
            let store = TableStore::beside(&browser.app.shared.settings_path);
            store.keep_for_tests(&kept, "Kept", &listing(&["alpha"]), Some(crate::tablesrc::unix_now()));
            browser.app.shared.config.library.tables = vec![at(&new), at(&kept)];

            read_as_a_scan_does(&mut browser);
            assert_eq!(tables.asked(), 0, "{tag}: reading the library asked an address for its table");
            assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "KEPT (1)"], "{tag}: the table that has a copy is not there without the other");
            assert_eq!(table_status(&new), Some(TableStatus::NotFetched), "{tag}");
            browser.app.shared.sel = cursor;
            said();

            let open = tables.hold();
            assert!(matches!(browser.frame(), Transition::Stay), "{tag}");
            assert!(table_is_fetching(&browser.app.shared, &new), "{tag}: the browser's first frame started nothing");
            assert!(!table_is_fetching(&browser.app.shared, &kept), "{tag}: a table that has a copy is being fetched");
            browser.frame();
            browser.frame();
            assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "KEPT (1)"], "{tag}: the list changed before the table arrived");
            if !skinned && cursor == 1 {
                capture(&mut browser, "select-before-the-table-arrives");
            }
            drop(open);
            land(&mut browser);
            browser.frame();

            assert_eq!(tables.asked(), 1, "{tag}: the table is fetched once");
            assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "NEW (2)", "KEPT (1)"], "{tag}: the table that arrived is not a folder in its place");
            assert_eq!(browser.sel(), cursor_after, "{tag}: the cursor is not on the bar it was on");
            assert!(browser.app.shared.is_at_root(), "{tag}: the table arriving moved the browser");
            assert_eq!(store.copy_name(&new).as_deref(), Some("New"), "{tag}: the table that arrived was not kept");
            assert!(matches!(table_status(&new), Some(TableStatus::Loaded { owned: 2, .. })), "{tag}");
            let said = said();
            let count = |line: &str| said.iter().filter(|(_, message)| message == line).count();
            assert_eq!(count("fetching 1 difficulty table(s) for the first time"), 1, "{tag}: {said:?}");
            assert_eq!(count("difficulty tables fetched for the first time: 1 added, 0 failed"), 1, "{tag}: {said:?}");
            assert!(said.iter().all(|(level, _)| *level == Level::Info), "{tag}: {said:?}");
            if !skinned && cursor == 1 {
                capture(&mut browser, "select-after-the-table-arrived");
            }

            browser.frame();
            read_as_a_scan_does(&mut browser);
            land(&mut browser);
            browser.frame();
            assert_eq!(tables.asked(), 1, "{tag}: a table that has a copy now was fetched when the library was read again");
            forget(&browser, &[&new, &kept]);
        }
    });
}

/// A browser looking into a level of another table is not moved by a table arriving: the view, the
/// rows and the cursor are what they were.
#[test]
fn a_table_arriving_leaves_a_browser_that_is_inside_another_table_where_it_is() {
    let (new, kept) = ("https://tables.test/browser/inside/new/", "https://tables.test/browser/inside/kept/");
    let tables = FakeTables::default();
    let _installed = tables.install();
    tables.serve(new, "New", &listing(&["alba"]));
    let mut browser = browser("tables-inside", false);
    let store = TableStore::beside(&browser.app.shared.settings_path);
    store.keep_for_tests(kept, "Kept", &listing(&["alder", "alpha", "altair"]), Some(1));
    browser.app.shared.config.library.tables = vec![at(new), at(kept)];
    read_as_a_scan_does(&mut browser);

    let open = tables.hold();
    browser.frame();
    browser.app.shared.open_for_tests([browser.app.shared.table_folder(1), browser.app.shared.level_folder(1, 0)]);
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame();
    browser.arrow(KeyCode::ArrowDown);
    browser.arrow(KeyCode::ArrowDown);
    let (before, cursor) = (rows(&browser), browser.sel());
    assert_eq!((before.len(), cursor), (3, 2));
    drop(open);
    land(&mut browser);
    browser.frame();

    assert!(
        browser.app.shared.opened() == [browser.app.shared.table_folder(1), browser.app.shared.level_folder(1, 0)],
        "the table arriving took the browser out of the level it was in"
    );
    assert_eq!((rows(&browser), browser.sel()), (before, cursor), "the rows or the cursor moved");
    assert_eq!(browser.app.shared.table_names, vec!["NEW".to_owned(), "KEPT".to_owned()]);
    forget(&browser, &[new, kept]);
}

/// A list of charts the browser's own filter has taken rows out of is left as it is by a table
/// arriving: the rows the filter leaves, and the cursor on the chart it was on. The filter is laid
/// over the list by the browser, after the table has been put in the library.
#[test]
fn a_table_arriving_leaves_a_filtered_list_of_charts_and_its_cursor_as_they_are() {
    let address = "https://tables.test/browser/filtered/";
    let tables = FakeTables::default();
    let _installed = tables.install();
    tables.serve(address, "Filtered", &listing(&["alba", "altair"]));
    let mut songs = TITLES.map(|title| entry(title, "a", "5")).to_vec();
    songs[0].mode = rbms_model::Mode::BEAT_5K;
    let mut browser = Browser::over("tables-filtered", songs, false);
    browser.frame();
    browser.app.shared.config.library.tables = vec![at(address)];
    read_as_a_scan_does(&mut browser);

    let open = tables.hold();
    browser.frame();
    browser.app.shared.open_for_tests([SelectFolder::AllSongs]);
    browser.app.shared.sel = 0;
    while browser.state.filter.filter(&browser.app.shared.config).mode != Some(rbms_model::Mode::BEAT_7K) {
        browser.state.filter.cycle_mode(true);
    }
    browser.frame();
    browser.arrow(KeyCode::ArrowDown);
    let (before, cursor) = (rows(&browser), browser.sel());
    assert_eq!((before.clone(), cursor), (vec!["alder".to_owned(), "alpha".to_owned(), "altair".to_owned()], 1), "the filter is not on the list");
    drop(open);
    land(&mut browser);
    browser.frame();

    assert_eq!(browser.app.shared.table_names, vec!["FILTERED".to_owned()]);
    assert_eq!((rows(&browser), browser.sel()), (before, cursor), "the table arriving moved the cursor off the chart it was on, or changed the rows");
    forget(&browser, &[address]);
}

/// A table whose address does not answer is no folder, says why once -- in one line, with no second
/// line counting a fetch of which nothing arrived -- and is left alone for the rest of the run: more
/// frames do not fetch it again, and neither does reading the library again. The key that fetches
/// the tables again is the player asking, and does.
#[test]
fn a_table_whose_first_fetch_fails_says_so_once_and_is_not_fetched_again_by_itself() {
    crate::notify::exclusive(|| {
        let dead = "https://tables.test/browser/dead/";
        let tables = FakeTables::default();
        let _installed = tables.install();
        let mut browser = browser("tables-dead", false);
        browser.app.shared.config.library.tables = vec![at(dead)];
        read_as_a_scan_does(&mut browser);
        said();

        browser.frame();
        land(&mut browser);
        assert_eq!(tables.asked(), 1);
        assert_eq!(rows(&browser), vec!["ALL SONGS (4)"], "a table that was not fetched is a folder");
        let Some(TableStatus::Failed { reason }) = table_status(dead) else {
            panic!("the table manager has nothing to say of the table that failed: {:?}", table_status(dead));
        };
        let first = said();
        let failures =
            |said: &[(Level, String)]| said.iter().filter(|(_, message)| message.contains("failed")).map(|(_, message)| message.clone()).collect::<Vec<_>>();
        assert_eq!(failures(&first), vec![format!("table fetch failed ({dead}): {reason}")], "the one failure is said more than once: {first:?}");
        assert!(first.iter().filter(|(_, message)| message.contains("failed")).all(|(level, _)| *level == Level::Warn));

        for _ in 0..3 {
            browser.frame();
        }
        read_as_a_scan_does(&mut browser);
        browser.frame();
        land(&mut browser);
        assert_eq!(tables.asked(), 1, "a table whose fetch failed was fetched again in the same run without being asked for");
        assert_eq!(table_status(dead), Some(TableStatus::Failed { reason }), "reading the library again forgot why the table failed");
        let again = said();
        assert!(again.is_empty(), "the failure was reported a second time: {again:?}");

        tables.serve(dead, "Back", &listing(&["alba"]));
        assert!(matches!(browser.key(KeyCode::F6), Transition::Stay));
        land(&mut browser);
        assert_eq!(tables.asked(), 2, "the key that fetches the tables again did not fetch the one that had failed");
        assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "BACK (1)"]);
        forget(&browser, &[dead]);
    });
}

/// While a table is out for the first time, the key that fetches the tables again does not fetch
/// it a second time.
#[test]
fn the_reload_key_does_not_fetch_a_table_twice_that_is_on_its_way_for_the_first_time() {
    crate::notify::exclusive(|| {
        let address = "https://tables.test/browser/once/";
        let tables = FakeTables::default();
        let _installed = tables.install();
        tables.serve(address, "Once", &listing(&["alba"]));
        let mut browser = browser("tables-once", false);
        browser.app.shared.config.library.tables = vec![at(address)];
        read_as_a_scan_does(&mut browser);

        let open = tables.hold();
        browser.frame();
        said();
        browser.key(KeyCode::F6);
        assert!(said().iter().any(|(_, message)| message == "the tables are being fetched already"));
        drop(open);
        land(&mut browser);
        assert_eq!(tables.asked(), 1, "a table on its way was fetched a second time");
        assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "ONCE (1)"]);
        forget(&browser, &[address]);
    });
}

/// A table registered in the table manager goes on being fetched after the manager is left, and
/// lands on the browser: it is a folder there, and the browser says it was added.
#[test]
fn a_table_registered_in_the_manager_lands_on_the_browser_after_the_manager_is_left() {
    crate::notify::exclusive(|| {
        let address = "https://tables.test/browser/registered/";
        let tables = FakeTables::default();
        let _installed = tables.install();
        tables.serve(address, "Registered", &listing(&["alba", "altair"]));
        let mut browser = browser("tables-registered", false);

        let open = tables.hold();
        register_table(&mut browser.app.shared, at(address));
        browser.app.shared.open_for_tests([]);
        browser.app.shared.sel = 0;
        browser.app.shared.rebuild_select_items();
        browser.come_back();
        browser.frame();
        assert_eq!(rows(&browser), vec!["ALL SONGS (4)"]);
        assert!(table_is_fetching(&browser.app.shared, address), "leaving the manager ended the fetch");
        said();
        drop(open);
        land(&mut browser);

        assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "REGISTERED (2)"]);
        assert_eq!(tables.asked(), 1);
        assert!(said().iter().any(|(level, message)| *level == Level::Info && message == "table added: REGISTERED (2 local charts)"));
        forget(&browser, &[address]);
    });
}

/// A table that lands after the library was read again is matched against the library as it is
/// when it lands, not the one its fetch was started beside; and reading the library again while
/// the table is out does not start a second fetch of it.
#[test]
fn a_table_that_lands_after_the_library_was_read_again_is_matched_against_the_new_library() {
    let address = "https://tables.test/browser/rescanned/";
    let tables = FakeTables::default();
    let _installed = tables.install();
    tables.serve(address, "Rescanned", &listing(&["altair", "zeta"]));
    let mut browser = browser("tables-rescanned", false);
    browser.app.shared.config.library.tables = vec![at(address)];
    read_as_a_scan_does(&mut browser);

    let open = tables.hold();
    browser.frame();
    assert!(table_is_fetching(&browser.app.shared, address));
    browser.app.shared.library = Library::from_songs(["zeta", "altair", "omega"].map(|title| entry(title, "a", "5")).to_vec());
    read_as_a_scan_does(&mut browser);
    browser.frame();
    assert_eq!(rows(&browser), vec!["ALL SONGS (3)"], "the table is a folder before it has arrived");
    drop(open);
    land(&mut browser);

    assert_eq!(tables.asked(), 1, "reading the library again fetched a table that was already on its way");
    assert_eq!(browser.app.shared.table_levels, vec![vec![("1".to_owned(), vec![0, 1])]], "the table was matched against the library that was replaced");
    assert_eq!(rows(&browser), vec!["ALL SONGS (3)", "RESCANNED (2)"]);
    forget(&browser, &[address]);
}

/// One key pressed and let go of on the browser, with the character it types if it types one.
fn stroke(browser: &mut Browser, code: KeyCode, text: Option<&'static str>) -> Transition {
    let asked = browser
        .state
        .handle_key(&mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }, KeyInput { code, pressed: true, released: false, text });
    browser.state.handle_key(
        &mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 },
        KeyInput { code, pressed: false, released: true, text: None },
    );
    asked
}

/// A search is left for the folder it was made from, although a table arrived while the search was
/// on and its folder took a place above that one: the row the search goes back to is the folder, not
/// the place the folder had.
#[test]
fn a_search_made_on_a_table_folder_is_left_for_that_folder_after_another_table_arrived_above_it() {
    for skinned in [false, true] {
        let tag = format!("tables-search-{skinned}");
        let (new, kept) = (format!("https://tables.test/browser/search/{tag}/new/"), format!("https://tables.test/browser/search/{tag}/kept/"));
        let tables = FakeTables::default();
        let _installed = tables.install();
        tables.serve(&new, "New", &listing(&["alba", "alder"]));
        let mut browser = browser(&tag, skinned);
        let store = TableStore::beside(&browser.app.shared.settings_path);
        store.keep_for_tests(&kept, "Kept", &listing(&["alpha"]), Some(1));
        browser.app.shared.config.library.tables = vec![at(&new), at(&kept)];
        read_as_a_scan_does(&mut browser);
        browser.app.shared.sel = 1;

        let open = tables.hold();
        browser.frame();
        assert_eq!(rows(&browser).get(browser.sel()).map(String::as_str), Some("KEPT (1)"), "{tag}");
        stroke(&mut browser, KeyCode::Slash, Some("/"));
        stroke(&mut browser, KeyCode::KeyA, Some("a"));
        stroke(&mut browser, KeyCode::KeyL, Some("l"));
        browser.frame();
        assert!(browser.app.shared.searching, "{tag}: the search box is not open");
        let found = rows(&browser);
        drop(open);
        land(&mut browser);
        browser.frame();
        assert_eq!(rows(&browser), found, "{tag}: the table arriving changed what the word found");

        stroke(&mut browser, KeyCode::Escape, None);
        browser.frame();
        assert!(!browser.state.search_is_on(&browser.app.shared), "{tag}: Escape did not leave the search");
        assert_eq!(rows(&browser), vec!["ALL SONGS (4)", "NEW (2)", "KEPT (1)"], "{tag}");
        assert_eq!(
            rows(&browser).get(browser.sel()).map(String::as_str),
            Some("KEPT (1)"),
            "{tag}: the search was left for another folder than it was made from"
        );
        forget(&browser, &[&new, &kept]);
    }
}

/// A search made inside a level of a table is left for that level, by its name, although the table
/// was fetched again while the search was on and came back with a level before it.
#[test]
fn a_search_made_inside_a_level_is_left_for_that_level_after_its_table_came_back_with_one_before_it() {
    let address = "https://tables.test/browser/search/level/";
    let tables = FakeTables::default();
    let _installed = tables.install();
    tables.serve(address, "Levels", br#"[{"md5":"md5-alba","level":"1"},{"md5":"md5-alpha","level":"2"},{"md5":"md5-altair","level":"2"}]"#);
    let mut browser = browser("tables-search-level", false);
    let store = TableStore::beside(&browser.app.shared.settings_path);
    store.keep_for_tests(address, "Levels", br#"[{"md5":"md5-alpha","level":"2"},{"md5":"md5-altair","level":"2"}]"#, Some(1));
    browser.app.shared.config.library.tables = vec![at(address)];
    read_as_a_scan_does(&mut browser);
    browser.frame();
    browser.app.shared.open_for_tests([browser.app.shared.table_folder(0), browser.app.shared.level_folder(0, 0)]);
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame();
    browser.arrow(KeyCode::ArrowDown);
    let inside = (rows(&browser), browser.sel());
    assert_eq!(inside, (vec!["alpha".to_owned(), "altair".to_owned()], 1));

    stroke(&mut browser, KeyCode::Slash, Some("/"));
    stroke(&mut browser, KeyCode::KeyA, Some("a"));
    browser.frame();
    assert!(matches!(stroke(&mut browser, KeyCode::F6, None), Transition::Stay));
    land(&mut browser);
    browser.frame();
    assert_eq!(browser.app.shared.table_levels[0].iter().map(|(level, _)| level.as_str()).collect::<Vec<_>>(), ["1", "2"], "the table did not come back");

    stroke(&mut browser, KeyCode::Escape, None);
    browser.frame();
    assert!(
        browser.app.shared.opened() == [browser.app.shared.table_folder(0), browser.app.shared.level_folder(0, 1)],
        "the search was left for another level than it was made in"
    );
    assert_eq!((rows(&browser), browser.sel()), inside);
    forget(&browser, &[address]);
}
