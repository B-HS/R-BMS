//! The ranking mode: which keys open it in each layout, how a ranking key is told held from
//! pressed, what the keys of the list do while it is open, and its skin values, settings row and
//! fetch while it is closed.
//!
//! The score server is a scripted double ([`RankingDouble`]), so nothing here reaches a network.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rbms_config::SettingId;
use rbms_model::translations::{Catalog, catalog_problems};
use rbms_model::{Language, Mode};

use super::keys::{KeyLayout, SelectKey};
use super::ranking_mode::{RANKING_AUTO_FETCH_AFTER, RankingKeyAct, RankingKeyFrame, RankingKeys, is_ranking_key, keyboard_counts, ranking_keys_of, your_line};
use super::skinned_tests::{Browser, KEY_FOUR, KEY_ONE, KEY_SIX, KEY_TWO, SELECT, SHORT_STEP, START, TURNTABLE_FORWARD, starts_a_chart};
use super::tests::entry;
use super::*;
use crate::ir_ext::MultiIr;
use crate::ir_ranking::tests::{RankingDouble, record};
use crate::ir_ranking::{RANKING_PAGE_LIMIT, RankingState, build_board};
use crate::ir_ranking_view::panel_lines;
use crate::keyconfig::KEY_INDEX_COUNT;
use crate::skin_host::ClusterState;
use crate::skin_host::extension::{RBMS_EVENT_RANKING_MODE, RBMS_OPTION_RANKING_MODE_ON};
use crate::skin_host::select::BrowserLent;
use crate::stage::settings::catalog::{RowDescriptionText, description};

/// A ranking key held as long as the mode takes to open, and a little longer.
const HELD_LONG: Duration = Duration::from_millis(SCROLL_DURATION_LOW_MS as u64 + 1);

/// A ranking key held a little shorter than the mode takes to open.
const HELD_SHORT: Duration = Duration::from_millis(SCROLL_DURATION_LOW_MS as u64 - 1);

/// The score server the browsers here read, which nothing ever resolves.
const SERVER_URL: &str = "http://ranking.invalid";

/// The chart the cursor of [`Browser::three_songs`] starts on.
const FIRST_CHART: &str = "md5-alpha";

/// The key indices of the keys two, four, six and eight, and of the first turntable direction.
const KEY_TWO_INDEX: usize = 1;
const KEY_FOUR_INDEX: usize = 3;
const KEY_SIX_INDEX: usize = 5;
const KEY_EIGHT_INDEX: usize = 7;

/// The controller lane of key two and of key six in the seven-key layout.
const KEY_TWO_LANE: usize = 1;
const KEY_SIX_LANE: usize = 5;

/// The key a test presses as nine buttons' button eight.
const BUTTON_EIGHT: KeyCode = KeyCode::KeyG;

/// The keys that are down, of the nine.
fn down(indices: &[usize]) -> [bool; KEY_INDEX_COUNT] {
    std::array::from_fn(|index| indices.contains(&index))
}

/// A frame on which the keys are free, the mode may open and key two is a ranking key that goes up a
/// folder and key six one that does nothing on the list.
fn free_frame(open: bool) -> RankingKeyFrame {
    RankingKeyFrame { free: true, can_open: true, open, ranking: down(&[KEY_TWO_INDEX, KEY_SIX_INDEX]), waits: down(&[KEY_TWO_INDEX]) }
}

/// One frame of `keys` with `held` down at `now`: what the ranking keys come to.
fn step(keys: &mut RankingKeys, held: &[usize], frame: &RankingKeyFrame, now: Instant) -> RankingKeyAct {
    let fresh = keys.read(down(held));
    keys.act(fresh, frame, now)
}

/// The browser's score server is a double answering `server`, as the primary and only one.
fn serve(browser: &mut Browser, server: RankingDouble) {
    let shared = &mut browser.app.shared;
    shared.config.network.server_url = Some(SERVER_URL.to_owned());
    shared.multi_ir = MultiIr::from_network(&shared.config.network);
    shared.profile_servers = vec![Arc::new(server)];
}

/// Three charts with a score server and, for the first, a board of `rows` players among whom the
/// local player stands at `you_at` when it says so, already fetched.
fn ranked(tag: &str, rows: u32, you_at: Option<usize>) -> Browser {
    let mut browser = Browser::three_songs(tag);
    serve(&mut browser, RankingDouble::new(Vec::new()));
    let mut records: Vec<_> = (0..rows).map(|n| record(&format!("p{n}"), &format!("Player {n}"), 2000 - n, None)).collect();
    if let Some(at) = you_at {
        records[at] = record("me", "Me", 1500, None);
    }
    let you = you_at.map(|_| record("me", "Me", 1500, None));
    browser.app.shared.ranking_cache.insert(FIRST_CHART.to_owned(), RankingState::Ready(Box::new(build_board(&records, you.as_ref(), &[]))));
    browser.frame_after(SHORT_STEP);
    browser
}

/// Hold `code` long enough for the mode to open, and let it go.
fn hold_long(browser: &mut Browser, code: KeyCode) {
    browser.hold(code);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(HELD_LONG);
    browser.let_go(code);
    browser.frame_after(SHORT_STEP);
}

#[test]
fn the_ranking_keys_are_the_blue_keys_and_two_free_buttons_of_the_nine() {
    assert_eq!(ranking_keys_of(Mode::BEAT_7K), [KEY_TWO_INDEX, KEY_FOUR_INDEX, KEY_SIX_INDEX]);
    assert_eq!(ranking_keys_of(Mode::BEAT_14K), [KEY_TWO_INDEX, KEY_FOUR_INDEX, KEY_SIX_INDEX], "either side of a double layout");
    assert_eq!(ranking_keys_of(Mode::BEAT_5K), [KEY_TWO_INDEX, KEY_FOUR_INDEX]);
    assert_eq!(ranking_keys_of(Mode::BEAT_10K), [KEY_TWO_INDEX, KEY_FOUR_INDEX]);
    assert_eq!(ranking_keys_of(Mode::POPN_9K), [KEY_TWO_INDEX, KEY_EIGHT_INDEX]);
    assert!(ranking_keys_of(Mode::KEYBOARD_24K).is_empty());
    for index in 0..KEY_INDEX_COUNT {
        assert_eq!(is_ranking_key(Mode::BEAT_7K, KeyLayout::Beat, index), ranking_keys_of(Mode::BEAT_7K).contains(&index), "key index {index}");
    }
}

/// Buttons two and eight of the nine-button tables have no role on the list, the reason they are the
/// nine buttons' ranking keys. The built-in browser reads the nine buttons by the seven-key table,
/// whose button eight moves the list, so there only button two opens the mode.
#[test]
fn the_nine_buttons_open_the_mode_with_the_buttons_the_list_does_not_use() {
    let list_roles = [
        SelectKey::Play,
        SelectKey::Auto,
        SelectKey::Replay,
        SelectKey::Up,
        SelectKey::Down,
        SelectKey::FolderOpen,
        SelectKey::FolderClose,
        SelectKey::Practice,
    ];
    for layout in [KeyLayout::Popn, KeyLayout::ReferencePopn] {
        for index in [KEY_TWO_INDEX, KEY_EIGHT_INDEX] {
            assert!(!list_roles.iter().any(|role| layout.carries(index, *role)), "{layout:?} gives button {index} a role on the list");
            assert!(is_ranking_key(Mode::POPN_9K, layout, index), "{layout:?} {index}");
        }
    }
    assert!(is_ranking_key(Mode::POPN_9K, KeyLayout::Beat, KEY_TWO_INDEX));
    assert!(!is_ranking_key(Mode::POPN_9K, KeyLayout::Beat, KEY_EIGHT_INDEX), "button eight moves the built-in list");
}

#[test]
fn the_keyboard_leaves_key_six_of_the_seven_key_modes_to_the_favourite_shortcut() {
    assert!(!keyboard_counts(Mode::BEAT_7K, KEY_SIX_INDEX));
    assert!(!keyboard_counts(Mode::BEAT_14K, KEY_SIX_INDEX));
    assert!(keyboard_counts(Mode::BEAT_7K, KEY_TWO_INDEX));
    assert!(keyboard_counts(Mode::POPN_9K, KEY_SIX_INDEX), "button six of the nine moves the list and is no ranking key");
    assert!(keyboard_counts(Mode::BEAT_5K, KEY_SIX_INDEX));
}

/// A ranking key that goes up a folder waits: let go sooner than the long slide it goes up the folder
/// then, held as long it opens the mode and does nothing when it is let go.
#[test]
fn a_ranking_key_held_long_opens_the_mode_and_let_go_sooner_does_what_it_always_did() {
    let start = Instant::now();
    let mut keys = RankingKeys::default();
    let frame = free_frame(false);
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &frame, start), RankingKeyAct::Wait(KEY_TWO_INDEX));
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &frame, start + HELD_SHORT), RankingKeyAct::Nothing);
    assert_eq!(step(&mut keys, &[], &frame, start + HELD_SHORT), RankingKeyAct::Let(KEY_TWO_INDEX));

    let mut keys = RankingKeys::default();
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &frame, start), RankingKeyAct::Wait(KEY_TWO_INDEX));
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &frame, start + super::SCRATCH_REPEAT_DELAY), RankingKeyAct::Open);
    let open = free_frame(true);
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &open, start + HELD_LONG), RankingKeyAct::Nothing, "the key that opened it is still held");
    assert_eq!(step(&mut keys, &[], &open, start + HELD_LONG), RankingKeyAct::Nothing, "letting it go does nothing");
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &open, start + HELD_LONG), RankingKeyAct::Close, "a fresh press closes it");
    assert_eq!(step(&mut keys, &[], &free_frame(false), start + HELD_LONG), RankingKeyAct::Nothing, "and its release opens nothing");
}

/// A tap shorter than a frame is a short press: it waits for the frame that reads it and is let go
/// on the next.
#[test]
fn a_tap_between_two_frames_is_a_short_press() {
    let start = Instant::now();
    let mut keys = RankingKeys::default();
    keys.note_press(KEY_TWO_INDEX);
    assert_eq!(step(&mut keys, &[], &free_frame(false), start), RankingKeyAct::Wait(KEY_TWO_INDEX));
    assert_eq!(step(&mut keys, &[], &free_frame(false), start), RankingKeyAct::Let(KEY_TWO_INDEX));
}

/// A ranking key that does nothing on the list opens the mode the moment it goes down, and a key
/// that is no ranking key, a bar the mode cannot open on and keys that are something else's open
/// nothing.
#[test]
fn a_ranking_key_with_no_role_on_the_list_opens_at_once_and_nothing_else_does() {
    let now = Instant::now();
    assert_eq!(step(&mut RankingKeys::default(), &[KEY_SIX_INDEX], &free_frame(false), now), RankingKeyAct::Open);
    assert_eq!(step(&mut RankingKeys::default(), &[KEY_FOUR_INDEX], &free_frame(false), now), RankingKeyAct::Nothing);
    let folder = RankingKeyFrame { can_open: false, ..free_frame(false) };
    assert_eq!(step(&mut RankingKeys::default(), &[KEY_TWO_INDEX], &folder, now), RankingKeyAct::Nothing, "on a folder the key is the list's at once");
    let off = RankingKeyFrame { ranking: [false; KEY_INDEX_COUNT], ..free_frame(false) };
    assert_eq!(step(&mut RankingKeys::default(), &[KEY_SIX_INDEX], &off, now), RankingKeyAct::Nothing, "with the row off");
}

/// A key that went down while something else had the keys is held and no press once they are free
/// again, however long it stays down; once let go it presses as any other.
#[test]
fn a_key_pressed_while_the_keys_were_elsewhere_opens_nothing_when_they_come_back() {
    let start = Instant::now();
    let mut keys = RankingKeys::default();
    let taken = RankingKeyFrame { free: false, ..free_frame(false) };
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &taken, start), RankingKeyAct::Nothing);
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &free_frame(false), start + HELD_LONG), RankingKeyAct::Nothing);
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &free_frame(false), start + HELD_LONG * 2), RankingKeyAct::Nothing);
    assert_eq!(step(&mut keys, &[], &free_frame(false), start + HELD_LONG * 2), RankingKeyAct::Nothing, "its release goes up no folder");
    assert_eq!(step(&mut keys, &[KEY_TWO_INDEX], &free_frame(false), start + HELD_LONG * 2), RankingKeyAct::Wait(KEY_TWO_INDEX));
}

/// The line the local player's row is on: their row of the leaderboard when the board holds it,
/// else the line that names them after it, which follows the line saying there are no scores.
#[test]
fn your_line_is_your_row_or_the_line_that_names_you() {
    let records: Vec<_> = (0..4).map(|n| record(&format!("p{n}"), &format!("Player {n}"), 2000 - n, None)).collect();
    let me = record("p2", "Me", 1998, None);
    let among = build_board(&records, Some(&me), &[]);
    assert_eq!(your_line(&among), 2);
    let below = build_board(&records, Some(&record("me", "Me", 1, Some(400))), &[]);
    assert_eq!(your_line(&below), records.len());
    let empty = build_board(&[], None, &[]);
    assert_eq!(your_line(&empty), 1);
    for board in [among, below, empty] {
        let lines = panel_lines(Some(&RankingState::Ready(Box::new(board.clone()))));
        let line = &lines[your_line(&board)];
        let expected = board.rows.iter().find(|row| row.is_you).map_or("YOU", |row| row.player.as_str());
        assert_eq!(line.label, expected, "the panel draws another line where the local player's should be");
    }
}

/// On a chart with a score server, key two let go before the long slide goes up the folder when it
/// is let go rather than when it went down, and the mode stays shut.
#[test]
fn a_short_press_of_key_two_goes_up_the_folder_when_it_is_let_go() {
    let mut browser = ranked("ranking-short", 3, None);
    browser.hold(KEY_TWO);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.opened() == [SelectFolder::AllSongs], "the folder closed when the key went down");
    browser.frame_after(HELD_SHORT - SHORT_STEP);
    browser.let_go(KEY_TWO);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.is_at_root(), "the folder did not close when the key was let go");
    assert!(!browser.state.ranking_open);
}

/// Key two held as long as the long slide opens the mode without going up the folder, and a short
/// press of it closes the mode again without going up the folder either.
#[test]
fn key_two_held_opens_the_mode_and_a_press_closes_it() {
    let mut browser = ranked("ranking-long", 3, None);
    hold_long(&mut browser, KEY_TWO);
    assert!(browser.state.ranking_open, "the mode did not open");
    assert!(browser.app.shared.opened() == [SelectFolder::AllSongs], "the folder closed under the mode");
    let lent = BrowserLent { ranking_mode: browser.state.ranking_open, ..BrowserLent::default() };
    let shown = browser.app.shared.select_shown(None, &lent);
    assert_eq!(crate::skin_host::select::SelectState::of(&shown).boolean(RBMS_OPTION_RANKING_MODE_ON), Some(true));

    browser.tap(KEY_FOUR);
    assert!(!browser.state.ranking_open, "a press of a ranking key did not close the mode");
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.opened() == [SelectFolder::AllSongs], "the press that closed the mode went up the folder too");
    let shown = browser.app.shared.select_shown(None, &BrowserLent::default());
    assert_eq!(crate::skin_host::select::SelectState::of(&shown).boolean(RBMS_OPTION_RANKING_MODE_ON), Some(false));
}

/// With the row off, with no score server, and on a folder, key two goes up the folder the moment it
/// goes down, as it always has, and holding it opens nothing.
#[test]
fn key_two_goes_up_at_once_where_the_mode_cannot_open() {
    for case in ["row-off", "no-server"] {
        let mut browser = ranked(&format!("ranking-at-once-{case}"), 3, None);
        if case == "row-off" {
            browser.app.shared.config.library.ranking_keys = false;
        } else {
            browser.app.shared.config.network.server_url = None;
            browser.app.shared.multi_ir = MultiIr::from_network(&browser.app.shared.config.network);
        }
        browser.hold(KEY_TWO);
        browser.frame_after(SHORT_STEP);
        assert!(browser.app.shared.is_at_root(), "{case}: the key waited");
        browser.frame_after(HELD_LONG);
        assert!(!browser.state.ranking_open, "{case}: the mode opened");
    }
    let mut browser = ranked("ranking-at-once-folder", 3, None);
    browser.app.shared.table_names = vec!["TABLE".to_owned()];
    browser.app.shared.table_levels = vec![vec![("1".to_owned(), vec![0, 1])]];
    browser.app.shared.open_for_tests([]);
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame_after(SHORT_STEP);
    let order = browser.app.shared.config.library.sort;
    browser.hold(KEY_TWO);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.app.shared.config.library.sort, order.next(), "on a folder at the top the key moves the order on at once");
}

/// A ranking key pressed while START called an option panel up is the panel's: once the panel has
/// gone it opens nothing however long it stays down, and goes up no folder when it is let go.
#[test]
fn a_key_held_through_an_option_panel_opens_nothing() {
    let mut browser = ranked("ranking-panel", 3, None);
    browser.hold(START);
    browser.hold(KEY_TWO);
    browser.frame_after(SHORT_STEP);
    browser.let_go(START);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(HELD_LONG);
    assert!(!browser.state.ranking_open);
    browser.let_go(KEY_TWO);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.opened() == [SelectFolder::AllSongs]);
}

/// Key six of the keyboard is the favourite shortcut's and opens nothing; key six of a controller has
/// no role on the list and opens the mode at once.
#[test]
fn key_six_opens_the_mode_from_a_controller_and_not_from_the_keyboard() {
    let mut browser = ranked("ranking-six", 3, None);
    browser.hold(KEY_SIX);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(HELD_LONG);
    assert!(!browser.state.ranking_open, "key six of the keyboard opened the mode");
    browser.let_go(KEY_SIX);
    browser.frame_after(SHORT_STEP);

    browser.pad(KEY_SIX_LANE);
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.ranking_open, "key six of a controller did not open the mode");
}

/// The nine buttons open the mode with button two or eight the moment it goes down, and close it the
/// same way.
#[test]
fn nine_buttons_open_and_close_the_mode_with_buttons_two_and_eight() {
    let mut browser = ranked("ranking-nine", 3, None);
    browser.app.shared.mode = Mode::POPN_9K;
    browser.app.shared.active_keys = browser.app.shared.keyconfig.lane_keys(Mode::POPN_9K);
    browser.frame_after(SHORT_STEP);
    browser.tap(KEY_TWO);
    assert!(browser.state.ranking_open, "button two did not open the mode");
    browser.tap(BUTTON_EIGHT);
    assert!(!browser.state.ranking_open, "button eight did not close the mode");
    assert_eq!(browser.sel(), 0, "a button moved the list");
}

/// In the mode the turntable moves the selection and repeats while held, the wheel of the mouse moves
/// it a row a notch, and a white key plays the replay of the row selected, here a row with none.
#[test]
fn the_keys_of_the_list_drive_the_open_mode() {
    let mut browser = ranked("ranking-drive", 5, None);
    hold_long(&mut browser, KEY_TWO);
    assert_eq!(browser.state.ranking_sel, 0);

    browser.hold(TURNTABLE_FORWARD);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.ranking_sel, 1, "the turntable did not select the row below");
    browser.frame_after(HELD_SHORT);
    assert_eq!(browser.state.ranking_sel, 1, "it repeated before the long slide was over");
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.ranking_sel, 2, "it did not repeat");
    browser.let_go(TURNTABLE_FORWARD);
    browser.frame_after(HELD_LONG);
    assert_eq!(browser.state.ranking_sel, 2, "it moved once let go");
    assert_eq!(browser.sel(), 0, "the list moved under the mode");

    browser.scroll(1.0);
    assert_eq!(browser.state.ranking_sel, 3);
    browser.scroll(-1.0);
    assert_eq!(browser.state.ranking_sel, 2);

    browser.hold(KEY_ONE);
    assert!(!starts_a_chart(&browser.frame_after(SHORT_STEP)), "a white key started the chart under the mode");
    assert_eq!(browser.app.shared.net_status, "no replay for that row");
    assert!(browser.state.ranking_open);
}

/// In the mode SELECT goes to the local player's row and START asks the server for the page after
/// the ones the board holds.
#[test]
fn select_goes_to_your_row_and_start_asks_for_the_next_page() {
    let mut browser = ranked("ranking-buttons", 5, Some(3));
    hold_long(&mut browser, KEY_TWO);
    browser.hold(SELECT);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.ranking_sel, 3, "SELECT did not go to the local player's row");
    browser.let_go(SELECT);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.ranking_rx.is_none(), "the board holds every page there is");

    let mut browser = ranked("ranking-next-page", RANKING_PAGE_LIMIT, None);
    hold_long(&mut browser, KEY_TWO);
    browser.hold(START);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.ranking_rx.is_some(), "START did not ask for the next page");
    assert!(browser.state.ranking_open, "START closed the mode");
}

/// The skin's event opens the mode and closes it, whatever the RANKING KEYS row says.
#[test]
fn the_skins_event_opens_and_closes_the_mode() {
    let mut browser = ranked("ranking-event", 3, None);
    browser.app.shared.config.library.ranking_keys = false;
    browser.state.run_event(&mut browser.app.shared, RBMS_EVENT_RANKING_MODE, 0);
    assert!(browser.state.ranking_open);
    browser.state.run_event(&mut browser.app.shared, RBMS_EVENT_RANKING_MODE, 0);
    assert!(!browser.state.ranking_open);
}

/// With the panel closed the ranking of the chart the cursor rests on is fetched once it has rested
/// there long enough, and not before.
#[test]
fn the_ranking_of_a_chart_rested_on_is_fetched_with_the_panel_closed() {
    let mut browser = Browser::three_songs("ranking-auto-fetch");
    serve(&mut browser, RankingDouble::new(vec![record("p0", "Player 0", 2000, None)]));
    browser.frame_after(SHORT_STEP);
    browser.frame_after(RANKING_AUTO_FETCH_AFTER - SHORT_STEP * 2);
    assert!(browser.app.shared.ranking_requested.is_none(), "the ranking was fetched before the cursor had rested");
    browser.frame_after(SHORT_STEP * 2);
    assert_eq!(browser.app.shared.ranking_requested.as_deref(), Some(FIRST_CHART));
    assert!(!browser.state.ranking_open);
}

/// The built-in browser reads the lanes of a controller: key two on a chart goes up the folder once
/// it is let go, and key six opens the mode.
#[test]
fn the_built_in_browser_reads_the_ranking_keys_of_a_controller() {
    let mut browser = Browser::over("ranking-built-in", ["alpha", "beta"].map(|title| entry(title, "a", "5")).to_vec(), false);
    serve(&mut browser, RankingDouble::new(Vec::new()));
    browser.app.shared.ranking_cache.insert(FIRST_CHART.to_owned(), RankingState::Ready(Box::new(build_board(&[], None, &[]))));
    browser.frame_after(SHORT_STEP);
    browser.pad(KEY_TWO_LANE);
    assert!(browser.app.shared.opened() == [SelectFolder::AllSongs], "key two went up the folder when it went down");
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.is_at_root(), "key two did not go up the folder once let go");

    let mut browser = Browser::over("ranking-built-in-six", ["alpha", "beta"].map(|title| entry(title, "a", "5")).to_vec(), false);
    serve(&mut browser, RankingDouble::new(Vec::new()));
    browser.frame_after(SHORT_STEP);
    browser.pad(KEY_SIX_LANE);
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.ranking_open, "key six did not open the mode");
}

/// The row's sentences are written in every language and are the ones the panel shows in each.
#[test]
fn the_ranking_keys_row_is_described_in_every_language() {
    assert!(catalog_problems(RowDescriptionText::ALL).is_empty(), "{:?}", catalog_problems(RowDescriptionText::ALL));
    for language in Language::ALL {
        rbms_render::text::set_language(language);
        assert_eq!(description(SettingId::RankingKeys), RowDescriptionText::RankingKeys.get(language), "{language:?}");
    }
    rbms_render::text::set_language(Language::English);
}
