//! The player's own folders as the browser shows them: one row at the top of the library, a row per
//! folder under it, and the charts of a folder the library has under that; fetched again with the
//! tables, and left the way any folder is left.
//!
//! The score server is the test's own ([`FolderServer`]); nothing asks the network. The frames are
//! saved for a person to open when the capture variable names a folder, beside the table manager
//! that accounts for the folders and the first run's library with the table every player is given.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

use rbms_library::{Library, SongEntry};
use winit::keyboard::KeyCode;

use super::skinned_tests::Browser;
use super::tests::entry;
use super::*;
use crate::ir_session::AccountSession;
use crate::stage::{HeadlessCanvas, StageHandler, TablesState};
use crate::tablesrc::{TableStore, read_and_match_all, seed_default_tables};
use crate::user_folders::tests::{FolderServer, OWNED_A, OWNED_B, PLAYER, SERVER, TOKEN, two_folders};

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
    shared.select_view = SelectView::Root;
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
    assert!(browser.app.shared.select_view == SelectView::UserFolders);
    assert_eq!(rows(&browser), vec!["PRACTICE 12 (2/3)", "LN (1)"]);
    capture(&browser, "folders-list");

    enter(&mut browser, "PRACTICE 12 (2/3)");
    assert!(browser.app.shared.select_view == SelectView::UserFolder(0));
    assert_eq!(rows(&browser), vec!["alba", "alder"], "the chart the library does not have is no row");
    capture(&browser, "folders-inside");

    browser.key(KeyCode::Escape);
    browser.frame();
    assert!(browser.app.shared.select_view == SelectView::UserFolders, "going back from a folder is the list of folders");
    browser.key(KeyCode::Escape);
    browser.frame();
    assert!(browser.app.shared.select_view == SelectView::Root);
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
    let (names, levels) =
        read_and_match_all(&TableStore::beside(&settings), &browser.app.shared.config.library.tables, &Library::from_songs(songs()), &AtomicUsize::new(0));
    browser.app.shared.table_names = names;
    browser.app.shared.table_levels = levels;
    assert!(crate::user_folders::start_fetch(&mut browser.app.shared, false), "the folders are fetched as a run that starts signed in fetches them");
    land(&mut browser);
    browser.app.shared.select_view = SelectView::Root;
    browser.app.shared.rebuild_select_items();
    browser.frame();
    assert_eq!(rows(&browser), vec!["ALL SONGS (3)", "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868} (0)", "MY FOLDERS (3)"]);
    capture(&browser, "first-run-root");

    enter(&mut browser, "\u{767a}\u{72c2}BMS\u{96e3}\u{6613}\u{5ea6}\u{8868} (0)");
    assert_eq!(rows(&browser).len(), 26, "every level of the table is a folder");
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
