//! The player's own folders as the browser shows them: one row at the top of the library, a row per
//! folder under it, and the charts of a folder under that, the ones the library has and then the
//! ones it does not, dimmed, the way a level of a table lists them; fetched again with the tables,
//! and left the way any folder is left.
//!
//! The score server is the test's own ([`FolderServer`]); nothing asks the network. The frames are
//! saved for a person to open when the capture variable names a folder, beside the table manager
//! that accounts for the folders and the first run's library with the table every player is given.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

use rbms_config::SortMode;
use rbms_library::{Library, SongEntry};
use winit::keyboard::KeyCode;

use super::list::missing_chart_title;
use super::skinned_tests::Browser;
use super::tests::entry;
use super::*;
use crate::ir_session::AccountSession;
use crate::stage::{HeadlessCanvas, StageHandler, TablesState};
use crate::tablesrc::{TableStore, read_and_match_all, seed_default_tables};
use crate::user_folders::tests::{
    FolderServer, MISSING, MISSING_TOO, OWNED_A, OWNED_B, PLAYER, PRACTICE, SERVER, TOKEN, TRAINING_URL, listed, summary, two_folders,
};

/// Where the frames below are saved for a person to open, when this names a folder.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// How long a test waits for the folders to come back.
const FETCH_WAIT: Duration = Duration::from_secs(10);

/// The charts of the library: two that the folders list, and one they do not.
fn songs() -> Vec<SongEntry> {
    let named = |title: &str, md5: &str| SongEntry { md5: md5.to_owned(), ..entry(title, "an artist", "12") };
    vec![named("alba", OWNED_A), named("alder", OWNED_B), entry("alpha", "an artist", "5")]
}

/// A browser at the top of the library, signed in to the test's score server.
fn signed_in(tag: &str, server: Arc<FolderServer>) -> Browser {
    let mut browser = Browser::over(tag, songs(), false);
    let shared = &mut browser.app.shared;
    shared.config.network.server_url = Some(SERVER.into());
    shared.server = server;
    shared.session = AccountSession::restored(Some(TOKEN.into()), Some(PLAYER.into()));
    crate::user_folders::follow_account(shared);
    shared.open_for_tests([]);
    shared.sel = 0;
    shared.rebuild_select_items();
    browser.frame();
    browser
}

/// Run the browser's frames until the folders are not being fetched any more.
fn land(browser: &mut Browser) {
    let deadline = Instant::now() + FETCH_WAIT;
    while browser.app.shared.user_folders.is_fetching() {
        assert!(matches!(browser.frame(), Transition::Stay), "the browser left for another screen while the folders were out");
        assert!(Instant::now() < deadline, "the folders never came back");
        std::thread::yield_now();
    }
    browser.frame();
}

/// The rows of the browser as it stands: a chart by its title, one the library does not have by
/// what it is called after `missing`, a folder by its label.
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
            SelectItem::Missing(missing) => format!("missing {}", browser.app.shared.missing_chart(missing).map(missing_chart_title).unwrap_or_default()),
            SelectItem::Course(course) => browser.app.shared.table_course(course).map(|course| course.name.clone()).unwrap_or_default(),
        })
        .collect()
}

/// Put the cursor on the row labelled `label` and enter it.
fn enter(browser: &mut Browser, label: &str) {
    let row = rows(browser).iter().position(|row| row == label).unwrap_or_else(|| panic!("no row {label} in {:?}", rows(browser)));
    browser.app.shared.sel = row;
    browser.key(KeyCode::Enter);
    browser.frame();
}

/// Save `rgba`, `size` big, as `<name>.png` in the capture folder, when one is named.
fn save(name: &str, size: (u32, u32), rgba: &[u8]) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    let folder = std::path::PathBuf::from(folder);
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    image::save_buffer(folder.join(format!("{name}.png")), rgba, size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

/// Save the frame the browser last drew.
fn capture(browser: &Browser, name: &str) {
    save(name, (CW, CH), browser.pixels.rgba());
}

/// F6 fetches the player's folders with the tables, and they land on the browser that is up as one
/// row at the top of the library. Entering it lists the folders, entering a folder lists the charts
/// of it the library has, and going back walks the same way out.
#[test]
fn the_reload_key_fetches_the_folders_and_the_browser_walks_into_and_out_of_them() {
    let mut browser = signed_in("user-folders-walk", two_folders());
    assert_eq!(rows(&browser), vec!["ALL SONGS (3)"], "no folders before they are fetched");

    browser.key(KeyCode::F6);
    land(&mut browser);
    assert_eq!(rows(&browser), vec!["ALL SONGS (3)", "MY FOLDERS (3)"]);
    capture(&browser, "folders-root");

    enter(&mut browser, "MY FOLDERS (3)");
    assert!(browser.app.shared.opened() == [SelectFolder::UserFolders]);
    assert_eq!(rows(&browser), vec!["PRACTICE 12 (2/3)", "LN (1)"]);
    capture(&browser, "folders-list");

    enter(&mut browser, "PRACTICE 12 (2/3)");
    assert!(browser.app.shared.opened() == [SelectFolder::UserFolders, browser.app.shared.user_folder(0)]);
    assert_eq!(rows(&browser), vec!["alba".to_owned(), "alder".to_owned(), format!("missing {MISSING}")], "the chart the library does not have comes last");
    capture(&browser, "folders-inside");

    browser.key(KeyCode::Escape);
    browser.frame();
    assert!(browser.app.shared.opened() == [SelectFolder::UserFolders], "going back from a folder is the list of folders");
    browser.key(KeyCode::Escape);
    browser.frame();
    assert!(browser.app.shared.is_at_root());
}

/// A browser signed in, its folders fetched, looking into a folder that lists the two charts the
/// library has and two it does not, one before them and one between them, in that order.
fn in_a_folder_missing_two(tag: &str) -> Browser {
    let server = FolderServer::serving(vec![summary(PRACTICE, "training", 1, 4)], &[]);
    server.set_items(
        PRACTICE,
        vec![listed(MISSING, "zeta", TRAINING_URL), listed(OWNED_A, "alba", ""), listed(MISSING_TOO, "albatross", ""), listed(OWNED_B, "alder", "")],
    );
    let mut browser = signed_in(tag, server);
    browser.key(KeyCode::F6);
    land(&mut browser);
    browser.app.shared.config.library.sort = SortMode::Default;
    enter(&mut browser, "MY FOLDERS (2)");
    enter(&mut browser, "TRAINING (2/4)");
    browser
}

/// Inside a folder the charts the library does not have follow the ones it has, in the folder's
/// order, each a row that knows the folder and its place in it; the folder's own row counts only
/// the ones the library has.
#[test]
fn a_folder_lists_the_charts_the_library_does_not_have_after_the_ones_it_has() {
    let browser = in_a_folder_missing_two("user-folders-missing");
    assert_eq!(rows(&browser), ["alba", "alder", "missing zeta", "missing albatross"]);
    assert!(matches!(
        browser.app.shared.select_items[2..],
        [SelectItem::Missing(MissingRef::Folder { folder: 0, entry: 0 }), SelectItem::Missing(MissingRef::Folder { folder: 0, entry: 1 })]
    ));
    assert_eq!(browser.app.shared.missing_chart(&MissingRef::Folder { folder: 0, entry: 0 }).map(|chart| chart.url_diff.as_str()), Some(TRAINING_URL));
    capture(&browser, "folders-missing");
}

/// The title ordering puts a chart the library does not have among the ones it has by its title,
/// the way a level of a table does.
#[test]
fn the_title_ordering_puts_a_missing_chart_of_a_folder_among_the_others_by_its_title() {
    let mut browser = in_a_folder_missing_two("user-folders-missing-title");
    browser.app.shared.config.library.sort = SortMode::Title;
    browser.app.shared.rebuild_select_items();
    assert_eq!(rows(&browser), ["alba", "missing albatross", "alder", "missing zeta"]);
}

/// A folder lists no chart the library does not have when the settings hide them, or while a search
/// is on, which looks through the library alone.
#[test]
fn hiding_missing_charts_or_searching_takes_a_folders_missing_charts_away() {
    let mut browser = in_a_folder_missing_two("user-folders-missing-hidden");
    browser.app.shared.config.library.show_missing_charts = false;
    browser.app.shared.rebuild_select_items();
    assert_eq!(rows(&browser), ["alba", "alder"], "the settings hide them");

    browser.app.shared.config.library.show_missing_charts = true;
    browser.app.shared.search = "al".to_owned();
    browser.app.shared.rebuild_select_items();
    assert_eq!(rows(&browser), ["alba", "alder"], "a search finds them");
}

/// The first run of a player who has no song folder yet: the table every player is given is a
/// folder of the library from the copy the binary carries, without the network, beside the
/// player's own folders. The table manager accounts for both.
#[test]
fn a_first_run_shows_the_given_table_beside_the_players_folders() {
    let mut browser = signed_in("user-folders-first-run", two_folders());
    let settings = browser.app.shared.settings_path.clone();
    let mut config = browser.app.shared.config.clone();
    assert!(seed_default_tables(&mut config, &settings));
    browser.app.shared.config = config;
    let matched =
        read_and_match_all(&TableStore::beside(&settings), &browser.app.shared.config.library.tables, &Library::from_songs(songs()), &AtomicUsize::new(0));
    browser.app.shared.set_tables(matched);
    assert!(crate::user_folders::start_fetch(&mut browser.app.shared, false), "the folders are fetched as a run that starts signed in fetches them");
    land(&mut browser);
    browser.app.shared.open_for_tests([]);
    browser.app.shared.rebuild_select_items();
    browser.frame();
    assert_eq!(rows(&browser), vec!["ALL SONGS (3)", "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868} (0)", "MY FOLDERS (3)"]);
    capture(&browser, "first-run-root");

    enter(&mut browser, "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868} (0)");
    let (courses, levels) = rows(&browser).split_last().map(|(courses, levels)| (courses.clone(), levels.to_vec())).expect("the table has rows");
    assert_eq!(levels.len(), 26, "every level of the table is a folder");
    assert!(levels.iter().all(|row| row.starts_with('\u{2605}')), "a level is not titled with the symbol the table names: {levels:?}");
    assert_eq!(courses, "COURSE (44)", "the table's folder of courses does not hold the bundled GENOSIDE 2018 courses");
    capture(&browser, "first-run-table");

    let mut tables = TablesState::new();
    let mut pixels = HeadlessCanvas::new(CW, CH);
    for _ in 0..browser.app.shared.config.library.tables.len() + 2 {
        tables.handle_key(
            &mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 },
            KeyInput { code: KeyCode::ArrowDown, pressed: true, released: false, text: None },
        );
    }
    tables.draw(&mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }, &mut Canvas::Headless(&mut pixels));
    save("folders-tables", (CW, CH), pixels.rgba());
    TableStore::beside(&settings).forget(rbms_config::DEFAULT_TABLES[0].location);
}
