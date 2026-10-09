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

fn add_ir_profile(app: &mut App, name: &str, enabled: bool) {
    app.shared.config.network.ir_profiles.push(rbms_config::IrProfile {
        name: name.to_string(),
        base_url: format!("http://{name}.invalid"),
        token: None,
        enabled,
    });
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

#[test]
fn the_ranking_panel_shows_and_switches_enabled_ir_profiles() {
    let mut app = app();
    add_ir_profile(&mut app, "FIRST", true);
    add_ir_profile(&mut app, "DISABLED", false);
    add_ir_profile(&mut app, "THIRD", true);
    app.shared.rebuild_server();
    let mut state = SelectState::new();

    key(&mut app, &mut state, press(KeyCode::KeyI));
    assert_eq!(state.ranking_lines(&app.shared)[0].label, "IR 1/2: FIRST");
    assert_eq!(state.ranking_sel, 1, "the profile label is not replay-selectable");

    key(&mut app, &mut state, press(KeyCode::ArrowUp));
    assert_eq!(state.ranking_sel, 1, "moving up does not select the profile label");

    key(&mut app, &mut state, press(KeyCode::KeyE));
    assert_eq!(state.ranking_lines(&app.shared)[0].label, "IR 2/2: THIRD");
    assert_eq!(state.ranking_sel, 1, "switching profiles keeps a result line selected");

    key(&mut app, &mut state, press(KeyCode::KeyE));
    assert_eq!(state.ranking_lines(&app.shared)[0].label, "IR 1/2: FIRST");

    key(&mut app, &mut state, press(KeyCode::KeyQ));
    assert_eq!(state.ranking_lines(&app.shared)[0].label, "IR 2/2: THIRD");
}

#[test]
fn the_ranking_panel_keeps_the_offline_view_without_a_profile() {
    let mut app = app();
    let mut state = SelectState::new();

    key(&mut app, &mut state, press(KeyCode::KeyI));
    let before = state.ranking_lines(&app.shared);
    key(&mut app, &mut state, press(KeyCode::KeyE));

    assert_eq!(state.ranking_lines(&app.shared), before);
}

#[test]
fn the_ranking_panel_keeps_its_single_profile_layout() {
    let mut app = app();
    add_ir_profile(&mut app, "ONLY", true);
    app.shared.rebuild_server();
    let mut state = SelectState::new();

    key(&mut app, &mut state, press(KeyCode::KeyI));
    let before = state.ranking_lines(&app.shared);
    key(&mut app, &mut state, press(KeyCode::KeyE));

    assert_eq!(before[0].label, "SELECT A CHART");
    assert_eq!(state.ranking_lines(&app.shared), before);
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

/// Seven-key lanes the pad tests press: the white keys are lanes 0 2 4 6, the black ones 1 3 5, and
/// the turntable is lane 7.
const WHITE_LANES: [usize; 4] = [0, 2, 4, 6];
const BLACK_CLOSE_LANES: [usize; 2] = [1, 3];
const NEXT_REPLAY_LANE: usize = 5;
const TURNTABLE_LANE: usize = 7;

fn lane_event(lane: usize, dir: ScratchDir, press: bool) -> PadEvent {
    PadEvent::Lane { lane, dir, press }
}

fn pad(app: &mut App, state: &mut SelectState, event: PadEvent, now: Instant) -> Transition {
    state.handle_pad(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, event)
}

fn tap(app: &mut App, state: &mut SelectState, lane: usize) -> Transition {
    pad(app, state, lane_event(lane, ScratchDir::Forward, true), Instant::now())
}

fn turn(app: &mut App, state: &mut SelectState, dir: ScratchDir, now: Instant) {
    pad(app, state, lane_event(TURNTABLE_LANE, dir, true), now);
}

fn three_songs() -> App {
    browsing(vec![entry("alpha", "a", "5"), entry("beta", "a", "5"), entry("gamma", "a", "5")])
}

/// Enough rows that a repeating turntable never reaches an end inside a test.
const LONG_LIST_ROWS: usize = 40;

fn long_list() -> App {
    browsing((0..LONG_LIST_ROWS).map(|i| entry(&format!("song{i:02}"), "a", "5")).collect())
}

/// The turntable is the list's up and down: forward goes to the next row, backward to the previous,
/// and neither runs off an end.
#[test]
fn a_turn_of_the_turntable_moves_the_list_and_stops_at_its_ends() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    let now = Instant::now();
    assert_eq!(app.shared.sel, 0);

    turn(&mut app, &mut state, ScratchDir::Backward, now);
    assert_eq!(app.shared.sel, 0, "backward at the top stays at the top");
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    assert_eq!(app.shared.sel, 2);
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    assert_eq!(app.shared.sel, 2, "forward at the bottom stays at the bottom");
    turn(&mut app, &mut state, ScratchDir::Backward, now);
    assert_eq!(app.shared.sel, 1);
}

/// A white key starts the focused chart, the same row Enter would.
#[test]
fn a_white_key_opens_the_focused_chart() {
    for lane in WHITE_LANES {
        let mut app = three_songs();
        let mut state = SelectState::new();
        app.shared.sel = 1;
        let transition = tap(&mut app, &mut state, lane);
        assert!(matches!(transition, Transition::Open(Stage::Loading(_))), "white key on lane {lane} did not start the chart");
    }
}

/// On a folder the same key enters it.
#[test]
fn a_white_key_on_a_folder_enters_it() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();

    assert!(matches!(tap(&mut app, &mut state, WHITE_LANES[0]), Transition::Stay));
    assert!(app.shared.select_view == SelectView::AllSongs, "the folder was not entered");
    assert_eq!(app.shared.select_items.len(), 3);
}

/// A black key goes up a folder, and never quits: at the root there is no folder to leave.
#[test]
fn a_black_key_goes_up_a_folder_and_does_nothing_at_the_root() {
    for lane in BLACK_CLOSE_LANES {
        let mut app = three_songs();
        let mut state = SelectState::new();
        assert!(matches!(tap(&mut app, &mut state, lane), Transition::Stay));
        assert!(app.shared.select_view == SelectView::Root, "black key on lane {lane} did not leave the folder");
        assert!(matches!(tap(&mut app, &mut state, lane), Transition::Stay), "at the root it must not quit");
        assert!(app.shared.select_view == SelectView::Root);
    }
}

/// The key between them is the replay slot, which the browser does not have yet: it does nothing.
#[test]
fn the_next_replay_key_does_nothing_yet() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    app.shared.sel = 1;
    assert!(matches!(tap(&mut app, &mut state, NEXT_REPLAY_LANE), Transition::Stay));
    assert_eq!(app.shared.sel, 1);
    assert!(app.shared.select_view == SelectView::AllSongs);
}

/// The second side of a double-play layout does what the first does, turntable included.
#[test]
fn both_sides_of_a_double_play_layout_drive_the_list() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    app.shared.mode = Mode::BEAT_14K;
    let now = Instant::now();
    pad(&mut app, &mut state, lane_event(7, ScratchDir::Forward, true), now);
    pad(&mut app, &mut state, lane_event(15, ScratchDir::Forward, true), now);
    assert_eq!(app.shared.sel, 2, "each turntable moved the list one row");
    let second_side_white = 8;
    assert!(matches!(tap(&mut app, &mut state, second_side_white), Transition::Open(Stage::Loading(_))));
}

/// A release, a control and a lane that has no key index are not moves.
#[test]
fn only_a_lane_going_down_with_an_index_moves_the_browser() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    let now = Instant::now();
    pad(&mut app, &mut state, lane_event(TURNTABLE_LANE, ScratchDir::Forward, false), now);
    pad(&mut app, &mut state, PadEvent::Control(ControlAction::HiSpeedUp), now);
    pad(&mut app, &mut state, PadEvent::Control(ControlAction::Start), now);
    assert_eq!(app.shared.sel, 0);

    app.shared.mode = Mode::KEYBOARD_24K;
    assert!(matches!(tap(&mut app, &mut state, 10), Transition::Stay), "lane 10 of the 24-key layout has no index");
    assert_eq!(app.shared.sel, 0);
}

/// With START or SELECT held the lane keys belong to the option panels, so the list stays put.
#[test]
fn the_list_does_not_move_while_start_or_select_is_held() {
    for (hold, name) in [(KeyCode::KeyA, "START"), (KeyCode::KeyW, "SELECT")] {
        let mut app = three_songs();
        let mut state = SelectState::new();
        let now = Instant::now();
        app.shared.note_key(&press(hold));
        turn(&mut app, &mut state, ScratchDir::Forward, now);
        assert!(matches!(tap(&mut app, &mut state, WHITE_LANES[0]), Transition::Stay), "{name} held, a white key opened the row");
        assert_eq!(app.shared.sel, 0, "{name} held, the turntable moved the list");

        app.shared.note_key(&release(hold));
        turn(&mut app, &mut state, ScratchDir::Forward, now);
        assert_eq!(app.shared.sel, 1, "{name} let go, the turntable did not move the list again");
    }
}

/// Whatever is open over the list keeps the controller away from it, like the keyboard.
#[test]
fn an_open_panel_keeps_the_controller_off_the_list() {
    let now = Instant::now();

    let mut app = three_songs();
    let mut state = SelectState::new();
    state.ranking_open = true;
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    assert_eq!(app.shared.sel, 0, "the ranking panel was open");

    let mut app = three_songs();
    let mut state = SelectState::new();
    state.record_modal = Some(0);
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    assert_eq!(app.shared.sel, 0, "the record modal was open");

    let mut app = three_songs();
    let mut state = SelectState::new();
    app.shared.searching = true;
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    assert_eq!(app.shared.sel, 0, "the search box was open");

    let mut app = three_songs();
    let mut state = SelectState::new();
    let mut ctx = FrameCtx { shared: &mut app.shared, now, dt: 0.0 };
    assert!(crate::app_options::options_key(&mut ctx, StageId::Select, false, &press(KeyCode::F1)), "F1 did not open the option overlay");
    turn(&mut app, &mut state, ScratchDir::Forward, now);
    assert_eq!(app.shared.sel, 0, "the option overlay was open");
}

/// A controller key counts as a key press for the quit confirmation: the second Esc must not quit
/// after the player has moved on.
#[test]
fn a_controller_key_cancels_an_armed_quit() {
    let mut app = app();
    let mut state = SelectState::new();
    key(&mut app, &mut state, press(KeyCode::Escape));
    assert!(state.esc_quit_at.is_some(), "the first Esc did not arm the quit");
    turn(&mut app, &mut state, ScratchDir::Forward, Instant::now());
    assert!(state.esc_quit_at.is_none(), "a controller key left the quit armed");
}

/// A turntable direction that stays down keeps moving the list: nothing before the delay, then one
/// row per interval, and it stops the moment the key is up. The key state is the keyboard's here
/// because a test has no controller; the browser reads both the same way.
#[test]
fn a_held_turntable_direction_repeats_after_a_delay() {
    let mut app = long_list();
    let mut state = SelectState::new();
    let start = Instant::now();

    turn(&mut app, &mut state, ScratchDir::Forward, start);
    assert_eq!(app.shared.sel, 1, "the press itself moves one row");
    app.shared.note_key(&press(KeyCode::ShiftLeft));
    let at = |offset: Duration, app: &mut App, state: &mut SelectState| {
        state.update(&mut FrameCtx { shared: &mut app.shared, now: start + offset, dt: 0.0 });
        app.shared.sel
    };

    assert_eq!(at(SCRATCH_REPEAT_DELAY - Duration::from_millis(1), &mut app, &mut state), 1, "before the delay nothing repeats");
    assert_eq!(at(SCRATCH_REPEAT_DELAY, &mut app, &mut state), 2, "the delay is over, one more row");
    assert_eq!(at(SCRATCH_REPEAT_DELAY + SCRATCH_REPEAT_INTERVAL - Duration::from_millis(1), &mut app, &mut state), 2, "then one per interval");
    assert_eq!(at(SCRATCH_REPEAT_DELAY + SCRATCH_REPEAT_INTERVAL, &mut app, &mut state), 3);
    assert_eq!(at(SCRATCH_REPEAT_DELAY + SCRATCH_REPEAT_INTERVAL * 2, &mut app, &mut state), 4);

    app.shared.note_key(&release(KeyCode::ShiftLeft));
    assert_eq!(at(SCRATCH_REPEAT_DELAY + SCRATCH_REPEAT_INTERVAL * 10, &mut app, &mut state), 4, "once it is up nothing repeats");
    assert!(state.scratch_hold.is_none(), "and the hold is forgotten");
}

/// Backward repeats the other way.
#[test]
fn a_held_backward_turn_repeats_upwards() {
    let mut app = long_list();
    app.shared.sel = LONG_LIST_ROWS / 2;
    app.shared.keyconfig.set_scratch_reverse(Mode::BEAT_7K, TURNTABLE_LANE, KeyCode::ControlLeft);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(Mode::BEAT_7K);
    let mut state = SelectState::new();
    let start = Instant::now();

    turn(&mut app, &mut state, ScratchDir::Backward, start);
    assert_eq!(app.shared.sel, LONG_LIST_ROWS / 2 - 1);
    app.shared.note_key(&press(KeyCode::ControlLeft));
    state.update(&mut FrameCtx { shared: &mut app.shared, now: start + SCRATCH_REPEAT_DELAY, dt: 0.0 });
    assert_eq!(app.shared.sel, LONG_LIST_ROWS / 2 - 2);
}

/// A hold whose key is already up when the delay ends (a release the pad layer never reported, or
/// one that landed while another screen was up) is dropped without moving the list.
#[test]
fn a_hold_whose_key_is_no_longer_down_is_dropped_without_a_move() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    let start = Instant::now();
    turn(&mut app, &mut state, ScratchDir::Forward, start);
    assert_eq!(app.shared.sel, 1);
    state.update(&mut FrameCtx { shared: &mut app.shared, now: start + SCRATCH_REPEAT_DELAY * 4, dt: 0.0 });
    assert_eq!(app.shared.sel, 1);
    assert!(state.scratch_hold.is_none());
}

/// The course tab's list is the one the turntable moves while that tab is up, and it wraps.
#[test]
fn the_turntable_moves_the_course_list_on_the_course_tab() {
    let mut app = three_songs();
    let mut state = SelectState::new();
    state.tab = SelectTab::Courses;
    turn(&mut app, &mut state, ScratchDir::Forward, Instant::now());
    assert_eq!(app.shared.sel, 0, "the song list stayed where it was");
    assert!(matches!(tap(&mut app, &mut state, BLACK_CLOSE_LANES[0]), Transition::Stay));
    assert!(app.shared.select_view == SelectView::AllSongs, "no folder is left from the course tab");
}
