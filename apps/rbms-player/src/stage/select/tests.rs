//! The browser's own behaviour: what its keys do to the list, how the search box gets back to the
//! folder it was opened from, and what the filter panel takes out of the rows.
//!
//! What the browser *paints* is pinned next door in `render_tests_select.rs`; what it does to the
//! list is here, because the list is the thing every one of these keys is really editing.
#![allow(clippy::wildcard_imports)]

use rbms_library::Library;

use super::*;
pub(crate) use crate::stage::select::list::tests::{entry, record};
use crate::{App, Config, LaunchOptions};

/// An app on a temp settings directory, so a test that stars a chart writes somewhere disposable.
pub(crate) fn app() -> App {
    let dir = std::env::temp_dir().join(format!("rbms-select-stage-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
    let path = dir.join("settings.ron");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp dir");
    App::new(String::new(), Config::default(), LaunchOptions::default(), path)
}

pub(crate) fn press(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: None }
}

fn typed(text: &str) -> KeyInput<'_> {
    KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) }
}

pub(crate) fn release(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: true, text: None }
}

/// An app whose library holds these charts, browsing the flat song list.
fn browsing(songs: Vec<rbms_library::SongEntry>) -> App {
    let mut app = app();
    app.shared.library = Library::from_songs(songs);
    app.shared.select_view = SelectView::AllSongs;
    app.shared.config.library.sort = SortMode::Title;
    app.shared.rebuild_select_items();
    app
}

fn titles(app: &App) -> Vec<String> {
    app.shared
        .select_items
        .iter()
        .map(|item| match item {
            SelectItem::Song(si) => app.shared.library.songs()[*si].title.clone(),
            SelectItem::Folder { label, .. } => label.clone(),
        })
        .collect()
}

fn key(app: &mut App, state: &mut SelectState, key: KeyInput<'_>) -> Transition {
    let now = Instant::now();
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, key)
}

fn frame(app: &mut App, state: &mut SelectState) {
    let now = Instant::now();
    state.update(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
}

/// The sort key steps forward on its own and back with a shift held, so a list overshot by one
/// press is one press away again.
#[test]
fn the_sort_key_steps_forward_and_shift_steps_it_back() {
    let mut app = app();
    let mut state = SelectState::new();
    let start = app.shared.config.library.sort;
    key(&mut app, &mut state, press(KeyCode::F3));
    let stepped = app.shared.config.library.sort;
    assert_ne!(stepped, start, "the sort key did not move the order");

    key(&mut app, &mut state, press(KeyCode::ShiftLeft));
    key(&mut app, &mut state, press(KeyCode::F3));
    assert_eq!(app.shared.config.library.sort, start, "shift did not turn the sort key around");

    key(&mut app, &mut state, release(KeyCode::ShiftLeft));
    key(&mut app, &mut state, press(KeyCode::F3));
    assert_eq!(app.shared.config.library.sort, stepped, "letting shift go left the sort key turned around");
}

/// The SORT row on the settings screen and the sort key edit the same field, so an ordering chosen
/// on either reaches the list. Reading the browser's order off a second copy of the value left the
/// settings row doing nothing at all until the next launch.
#[test]
fn the_settings_row_and_the_sort_key_order_the_same_list() {
    let mut app = browsing(vec![entry("gamma", "z", "5"), entry("alpha", "m", "5"), entry("beta", "a", "5")]);
    let mut state = SelectState::new();
    frame(&mut app, &mut state);
    assert_eq!(titles(&app), vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]);

    rbms_config::adjust(&mut app.shared.config, rbms_config::SettingId::Sort, 1);
    while app.shared.config.library.sort != SortMode::Artist {
        rbms_config::adjust(&mut app.shared.config, rbms_config::SettingId::Sort, 1);
    }
    frame(&mut app, &mut state);
    assert_eq!(titles(&app), vec!["beta".to_string(), "alpha".to_string(), "gamma".to_string()], "the settings row did not reorder the list");

    key(&mut app, &mut state, press(KeyCode::F3));
    assert_eq!(app.shared.config.library.sort, SortMode::Artist.next(), "the sort key stepped from something other than the settings row");
}

/// An ordering chosen with the sort key is written out, so it is still there on the next launch —
/// the settings row it shares a field with has always been saved.
#[test]
fn the_sort_key_writes_the_order_it_chose_out() {
    let mut app = browsing(vec![entry("alpha", "a", "5")]);
    let mut state = SelectState::new();
    key(&mut app, &mut state, press(KeyCode::F3));
    let chosen = app.shared.config.library.sort;
    let reloaded = rbms_config::load(&app.shared.settings_path).expect("the settings the sort key wrote out are readable");
    assert_eq!(reloaded.config.library.sort, chosen, "the order the sort key chose was not written out");
}

/// The favourite key writes the set out at once, so a starred chart survives a crash the way a
/// saved score does. A folder row has no chart behind it and is left alone.
#[test]
fn the_favourite_key_stars_the_focused_chart_and_leaves_a_folder_alone() {
    let mut app = app();
    let mut state = SelectState::new();
    app.shared.select_items = vec![SelectItem::Folder { label: "ALL SONGS".into(), target: SelectView::AllSongs }];
    app.shared.sel = 0;
    key(&mut app, &mut state, press(KeyCode::KeyF));
    assert!(!app.shared.favorites_path.exists(), "a folder row wrote a favourites file");

    let mut app = browsing(vec![entry("song", "a", "5")]);
    let mut state = SelectState::new();
    key(&mut app, &mut state, press(KeyCode::KeyF));
    assert!(app.shared.favorites.contains("md5-song"), "the focused chart was not starred");
    assert!(app.shared.favorites_path.exists(), "starring a chart did not write the file");
    key(&mut app, &mut state, press(KeyCode::KeyF));
    assert!(!app.shared.favorites.contains("md5-song"), "a second press did not unstar it");
}

/// A search is a detour: closing the box has to put the browser back in the folder and on the row
/// it was opened from, rather than stranding it in the flat song list the search ran over.
#[test]
fn closing_the_search_box_goes_back_to_the_folder_it_was_opened_from() {
    let mut app = browsing(vec![entry("alpha", "a", "5"), entry("beta", "a", "5"), entry("gamma", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();
    let root_rows = titles(&app);

    key(&mut app, &mut state, press(KeyCode::Slash));
    assert!(app.shared.searching);
    assert!(app.shared.select_view == SelectView::AllSongs, "a search has to span the whole library");

    key(&mut app, &mut state, typed("bet"));
    assert_eq!(titles(&app), vec!["beta".to_string()], "the query did not filter the list");

    key(&mut app, &mut state, press(KeyCode::Escape));
    assert!(!app.shared.searching);
    assert!(app.shared.select_view == SelectView::Root, "the browser stayed in the list the search ran over");
    assert_eq!(titles(&app), root_rows);
    assert!(app.shared.search.is_empty(), "the query outlived the search box");
}

/// Backspacing the query has to widen the list again, and the row the search was opened on is
/// still the row it comes back to.
#[test]
fn the_search_box_edits_the_query_and_keeps_the_row_it_was_opened_on() {
    let mut app = browsing(vec![entry("alpha", "a", "5"), entry("beta", "a", "5"), entry("bravo", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.sel = 2;

    key(&mut app, &mut state, press(KeyCode::Slash));
    key(&mut app, &mut state, typed("br"));
    assert_eq!(titles(&app), vec!["bravo".to_string()]);
    key(&mut app, &mut state, press(KeyCode::Backspace));
    assert_eq!(titles(&app), vec!["beta".to_string(), "bravo".to_string()], "backspace did not widen the query");

    key(&mut app, &mut state, press(KeyCode::Escape));
    assert_eq!(app.shared.sel, 2, "the row the search was opened on was not restored");
}

/// The filter panel takes every key while it is up, so the list under it cannot move and Enter
/// cannot start the chart behind it.
#[test]
fn the_filter_panel_takes_the_keys_while_it_is_open() {
    let mut app = browsing(vec![entry("alpha", "a", "5"), entry("beta", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.sel = 0;

    key(&mut app, &mut state, press(KeyCode::F2));
    let moved = key(&mut app, &mut state, press(KeyCode::ArrowDown));
    assert!(matches!(moved, Transition::Stay));
    assert_eq!(app.shared.sel, 0, "the panel let the list cursor move under it");
    let started = key(&mut app, &mut state, press(KeyCode::Enter));
    assert!(matches!(started, Transition::Stay), "Enter started a chart from behind the panel");
    let after = key(&mut app, &mut state, press(KeyCode::ArrowDown));
    assert!(matches!(after, Transition::Stay));
    assert_eq!(app.shared.sel, 1, "Enter did not close the panel, so the list stayed frozen");
}

/// A level bound set in the panel has to actually take the charts outside it out of the list.
#[test]
fn a_level_bound_set_in_the_panel_filters_the_list() {
    let mut app = browsing(vec![entry("easy", "a", "3"), entry("hard", "a", "11"), entry("mid", "a", "7")]);
    let mut state = SelectState::new();
    assert_eq!(titles(&app).len(), 3);

    key(&mut app, &mut state, press(KeyCode::F2));
    for _ in 0..8 {
        key(&mut app, &mut state, press(KeyCode::ArrowRight));
    }
    assert_eq!(titles(&app), vec!["hard".to_string()], "the level bound did not filter the list");

    key(&mut app, &mut state, press(KeyCode::Backspace));
    assert_eq!(titles(&app).len(), 3, "resetting the filter did not bring the list back");
}

/// The favourites axis is remembered between runs, so a settings file that has it on has to hide
/// the unstarred charts from the very first list the browser builds.
#[test]
fn the_favourites_filter_hides_everything_that_is_not_starred() {
    let mut app = browsing(vec![entry("starred", "a", "5"), entry("plain", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.favorites.toggle("md5-starred");
    app.shared.config.library.favorite_only = true;

    frame(&mut app, &mut state);
    assert_eq!(titles(&app), vec!["starred".to_string()], "the favourites filter did not reach the list");

    app.shared.config.library.favorite_only = false;
    frame(&mut app, &mut state);
    assert_eq!(titles(&app).len(), 2, "turning the filter off did not bring the list back");
}

/// Unstarring the focused chart while only the starred ones are shown has to take it out of the
/// list, or the row would stay under the cursor claiming to be a favourite.
#[test]
fn unstarring_a_chart_removes_it_from_a_favourites_only_list() {
    let mut app = browsing(vec![entry("starred", "a", "5"), entry("plain", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.favorites.toggle("md5-starred");
    app.shared.config.library.favorite_only = true;
    frame(&mut app, &mut state);
    assert_eq!(titles(&app), vec!["starred".to_string()]);

    key(&mut app, &mut state, press(KeyCode::KeyF));
    frame(&mut app, &mut state);
    assert!(titles(&app).is_empty(), "the unstarred chart stayed in a favourites-only list");
}

/// Another screen rebuilds the list without the browser's filter — a finished scan, a chart coming
/// back from play — so the browser has to notice and put its filter back on.
#[test]
fn a_rebuild_asked_for_by_another_screen_gets_the_filter_re_applied() {
    let mut app = browsing(vec![entry("starred", "a", "5"), entry("plain", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.favorites.toggle("md5-starred");
    app.shared.config.library.favorite_only = true;
    frame(&mut app, &mut state);

    app.shared.rebuild_select_items();
    assert_eq!(titles(&app).len(), 1, "the settings-backed axis is applied by the shared rebuild too");

    app.shared.config.library.favorite_only = false;
    app.shared.rebuild_select_items();
    let mut state2 = SelectState::new();
    key(&mut app, &mut state2, press(KeyCode::F2));
    for _ in 0..3 {
        key(&mut app, &mut state2, press(KeyCode::ArrowRight));
    }
    assert_eq!(titles(&app), vec!["plain".to_string(), "starred".to_string()], "a level bound of 3 keeps neither chart out");

    app.shared.rebuild_select_items();
    frame(&mut app, &mut state2);
    assert_eq!(titles(&app).len(), 2, "the panel's filter was not re-applied after an outside rebuild");
}

/// The record orderings read the score book, so the key that changes them has to change the list.
#[test]
fn the_clear_ordering_puts_the_worst_lamp_first() {
    let mut app = browsing(vec![entry("cleared", "a", "5"), entry("failed", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.scores.push(record("md5-cleared", 5, 800, 4, 2_000));
    app.shared.scores.push(record("md5-failed", 1, 200, 40, 1_000));
    app.shared.scores.rebuild_index();
    app.shared.config.library.sort = SortMode::Clear;
    state.rebuild(&mut app.shared);
    assert_eq!(titles(&app), vec!["failed".to_string(), "cleared".to_string()]);
}

#[test]
fn the_file_preview_clip_sits_at_the_base_of_the_preview_namespace() {
    assert_eq!(PREVIEW_ID, IdNamespace::PREVIEW.base);
    assert!(IdNamespace::PREVIEW.contains(PREVIEW_ID));
    assert!(!IdNamespace::PLAY.contains(PREVIEW_ID));
}

#[test]
fn preview_keysounds_land_inside_the_preview_namespace() {
    for wav in [0, 1, 1_295, IdNamespace::PREVIEW.len - 1] {
        let id = SelectState::preview_sample_id(wav);
        assert!(IdNamespace::PREVIEW.contains(id), "wav {wav} escaped the preview namespace");
        assert!(!IdNamespace::PLAY.contains(id));
        assert_eq!(id, IdNamespace::PREVIEW.base + wav);
    }
}

#[test]
fn an_out_of_range_wav_index_is_clamped_into_the_preview_namespace() {
    let clamped = SelectState::preview_sample_id(u32::MAX);
    assert!(IdNamespace::PREVIEW.contains(clamped));
    assert_eq!(clamped, IdNamespace::PREVIEW.base + IdNamespace::PREVIEW.len - 1);
}

#[test]
fn a_chart_keysound_and_the_preview_copy_of_it_never_share_an_id() {
    for wav in [1u32, 36, 1_295] {
        assert_ne!(wav, SelectState::preview_sample_id(wav));
        assert!(IdNamespace::PLAY.contains(wav));
    }
}

/// A folder row says how many charts are reachable inside it, so a filter that empties one has to
/// show as a zero rather than as the whole folder still being there.
#[test]
fn a_folder_row_counts_only_the_charts_the_filter_leaves() {
    let mut app = browsing(vec![entry("starred", "a", "5"), entry("plain", "a", "5")]);
    let mut state = SelectState::new();
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();
    assert_eq!(titles(&app), vec!["ALL SONGS (2)".to_string()]);

    app.shared.favorites.toggle("md5-starred");
    app.shared.config.library.favorite_only = true;
    frame(&mut app, &mut state);
    assert_eq!(titles(&app), vec!["ALL SONGS (1)".to_string()], "the folder row counted charts the filter takes out");
}
