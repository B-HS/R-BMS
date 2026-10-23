//! Playing the focused chart laid out as a replay of it was, from the browser: the record modal's
//! button and key, the ranking panel's key and the replay it downloads, and the controller button
//! that reaches the panel.
//!
//! The replays are written to the test's own folder or handed to the browser the way the download
//! hands them, and the score server is a scripted double that answers nothing about replays, so
//! nothing here reaches a network.

use std::sync::Arc;
use std::sync::mpsc::channel;

use rbms_chart::shuffle::NoteOption;
use rbms_ir::{ChartId, RandomOption, ReplayData};
use rbms_model::Language;
use rbms_store::{Replay, ScoreBook};
use winit::event::MouseButton;

use super::skinned_tests::{Browser, SHORT_STEP, starts_a_chart};
use super::tests::{entry, record};
use super::*;
use crate::borrowed_layout::BorrowedLayout;
use crate::format::fmt_datetime;
use crate::gamepad::PadEvent;
use crate::ir_ext::MultiIr;
use crate::ir_ranking::tests::{RankingDouble, record_with_replay};
use crate::ir_ranking::{RankingState, build_board};
use crate::keyconfig::Bindable;
use crate::pointer::{PointerInput, route_pointer};
use crate::text::settings::InstalledLanguage;

/// The chart the cursor of the fixture browser starts on, its title, and the path it is listed under.
const FIRST_CHART: &str = "md5-alpha";

/// The day, the EX and the replay file of the run of this machine the browser holds a record of.
const KEPT_AT: i64 = 1_700_000_000_000;
const KEPT_EX: u32 = 800;
const REPLAY_FILE: &str = "kept.ron";

/// The seed the replays a test borrows from were laid out with.
const REPLAY_SEED: u64 = 0x5EED;

/// The score server the panel tests read, which nothing ever resolves, and the player and the replay
/// of the row that has one.
const SERVER_URL: &str = "http://ranking.invalid";
const PLAYER: &str = "Alice";
const REPLAY_ID: &str = "rep-a";

/// The seat of the first lane, which a key bound to it is a key of the lanes.
const FIRST_LANE: usize = 0;

/// How many rows of pixels from the top of the ranking panel its head covers, above the first row.
const HEAD_H: u32 = 60;

/// The gauge the course a test refuses is opened on.
const COURSE_OPENING_GAUGE: f32 = 100.0;

/// A replay of the first chart recorded on the arrangement `random` names, from `seed`.
fn a_replay(random: &str, seed: u64) -> Replay {
    Replay {
        chart_path: String::new(),
        md5: FIRST_CHART.to_string(),
        mode: String::new(),
        random: random.to_string(),
        seed,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    }
}

/// `browser` with a record of a run of its first chart that kept the file [`REPLAY_FILE`] when
/// `keeps` says so. The file holds `replay` when there is one, and is missing when there is none.
fn keeping(mut browser: Browser, keeps: bool, replay: Option<Replay>) -> Browser {
    let shared = &mut browser.app.shared;
    shared.config.play.autoplay = false;
    if let Some(replay) = replay {
        std::fs::create_dir_all(&shared.replay_dir).expect("the replay folder is writable");
        std::fs::write(shared.replay_dir.join(REPLAY_FILE), replay.to_ron().expect("the replay writes")).expect("the replay is written");
    }
    let kept = rbms_store::ScoreRecord { replay_file: keeps.then(|| REPLAY_FILE.to_string()), ..record(FIRST_CHART, 5, KEPT_EX, 2, KEPT_AT) };
    shared.scores = ScoreBook::from_records(vec![kept]);
    browser.frame();
    browser
}

/// The fixture browser, a skin drawing it, with such a record.
fn browser_with_a_run(tag: &str, keeps: bool, replay: Option<Replay>) -> Browser {
    keeping(Browser::three_songs(tag), keeps, replay)
}

/// The key the shortcut that plays a layout is read from.
fn layout_key(browser: &Browser) -> KeyCode {
    browser.app.shared.shortcut_key(Shortcut::ReplayPlayLayout).expect("the shortcut ships on a key")
}

/// Open the record modal on the first record and draw it.
fn open_the_modal(browser: &mut Browser) {
    browser.key(KeyCode::KeyR);
    assert_eq!(browser.state.record_modal, Some(0), "the record modal did not open");
    browser.frame_after(SHORT_STEP);
}

/// Where the middle of the region of `wanted` the last frame registered is.
fn middle_of(browser: &Browser, wanted: Hot) -> Option<(f32, f32)> {
    browser.app.shared.hot.iter().find(|(_, hot)| *hot == wanted).map(|(rect, _)| (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0))
}

/// A press of the left button at `at`, handed to the browser as the window hands one.
fn click(browser: &mut Browser, at: (f32, f32)) -> Transition {
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
    route_pointer(&mut browser.state, &mut ctx, at, PointerInput::Button { button: MouseButton::Left, pressed: true })
}

/// The layout the browser's runs are borrowing, if it is one.
fn borrowed(browser: &Browser) -> Option<BorrowedLayout> {
    browser.app.shared.borrowed_layout().cloned()
}

#[test]
fn the_layout_key_of_the_record_modal_plays_the_chart_on_the_layout_of_the_replay_the_record_kept() {
    let mut browser = browser_with_a_run("layout-modal-key", true, Some(a_replay("RANDOM", REPLAY_SEED)));
    browser.key(KeyCode::KeyR);
    assert_eq!(browser.state.record_modal, Some(0));
    let asked = browser.key(layout_key(&browser));
    assert!(starts_a_chart(&asked), "the key did not start the chart");
    assert!(browser.state.record_modal.is_none(), "the modal is still up over the run");
    assert_eq!(borrowed(&browser), Some(BorrowedLayout { random: NoteOption::Random, seed: REPLAY_SEED, source: fmt_datetime(KEPT_AT) }));
    let (_, kept) = browser.app.shared.borrowed_kept_run(FIRST_CHART).expect("the run on the layout is not held against the run it was borrowed from");
    assert_eq!(kept.ex, KEPT_EX);
    assert_eq!(browser.app.shared.layout_to_load(FIRST_CHART, None), (NoteOption::Random, REPLAY_SEED), "the chart is not laid out as the replay was");
}

#[test]
fn the_layout_button_of_the_record_modal_does_what_its_key_does() {
    let mut browser = browser_with_a_run("layout-modal-button", true, Some(a_replay("MIRROR", REPLAY_SEED)));
    open_the_modal(&mut browser);
    let at = middle_of(&browser, Hot::ModalLayout).expect("the modal registered no layout button");
    let asked = click(&mut browser, at);
    assert!(starts_a_chart(&asked), "the button did not start the chart");
    assert!(browser.state.record_modal.is_none());
    assert_eq!(borrowed(&browser).map(|layout| (layout.random, layout.seed)), Some((NoteOption::Mirror, REPLAY_SEED)));
    assert!(browser.app.shared.borrowed_kept_run(FIRST_CHART).is_some());
}

#[test]
fn the_built_in_browsers_modal_has_the_layout_button_too() {
    let songs = ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec();
    let mut browser = keeping(Browser::over("layout-built-in-modal", songs, false), true, Some(a_replay("S-RANDOM", REPLAY_SEED)));
    open_the_modal(&mut browser);
    let at = middle_of(&browser, Hot::ModalLayout).expect("the built-in browser's modal registered no layout button");
    assert!(starts_a_chart(&click(&mut browser, at)), "the button did not start the chart");
    assert_eq!(borrowed(&browser).map(|layout| (layout.random, layout.seed)), Some((NoteOption::SRandom, REPLAY_SEED)));
}

#[test]
fn the_modal_reads_the_layout_key_from_the_key_configuration() {
    let mut browser = browser_with_a_run("layout-modal-rebound", true, Some(a_replay("RANDOM", REPLAY_SEED)));
    let shipped = layout_key(&browser);
    let moved_to = KeyCode::KeyM;
    let mode = browser.app.shared.mode;
    browser.app.shared.keyconfig.bind(mode, Bindable::Shortcut(Shortcut::ReplayPlayLayout), Some(moved_to));
    browser.key(KeyCode::KeyR);
    assert!(!starts_a_chart(&browser.key(shipped)), "the key it shipped on still plays the layout");
    assert!(borrowed(&browser).is_none());
    assert!(starts_a_chart(&browser.key(moved_to)), "the key it was moved to does not");
}

#[test]
fn a_record_that_kept_no_replay_has_no_layout_to_lend() {
    let mut browser = browser_with_a_run("layout-no-replay", false, None);
    open_the_modal(&mut browser);
    assert!(middle_of(&browser, Hot::ModalLayout).is_none(), "a record with no replay offered a layout");
    assert!(!starts_a_chart(&browser.key(layout_key(&browser))));
    assert!(browser.state.record_modal.is_some() && borrowed(&browser).is_none());
}

#[test]
fn a_replay_that_cannot_be_read_borrows_nothing_and_leaves_the_modal_up() {
    let mut browser = browser_with_a_run("layout-unreadable", true, None);
    open_the_modal(&mut browser);
    let asked = browser.key(layout_key(&browser));
    assert!(!starts_a_chart(&asked) && borrowed(&browser).is_none());
    assert_eq!(browser.state.record_modal, Some(0), "the modal went away over a notice");
}

#[test]
fn a_layout_that_is_refused_leaves_the_modal_up_and_the_chart_unplayed() {
    let refused = |tag: &str, replay: Replay, spoil: &dyn Fn(&mut AppShared)| {
        let mut browser = browser_with_a_run(tag, true, Some(replay));
        spoil(&mut browser.app.shared);
        open_the_modal(&mut browser);
        let asked = browser.key(layout_key(&browser));
        assert!(!starts_a_chart(&asked), "{tag}: the chart was started");
        assert_eq!(browser.state.record_modal, Some(0), "{tag}: the modal went away over a notice");
        assert!(borrowed(&browser).is_none() && browser.app.shared.borrowed_kept_run(FIRST_CHART).is_none(), "{tag}: a layout was held");
    };
    let untouched = |_: &mut AppShared| {};
    refused("layout-no-seed", a_replay("RANDOM", 0), &untouched);
    refused("layout-other-chart", Replay { md5: "0123456789abcdef".to_string(), ..a_replay("RANDOM", REPLAY_SEED) }, &untouched);
    refused("layout-unknown-arrangement", a_replay("CONVERGE", REPLAY_SEED), &untouched);
    refused("layout-in-a-course", a_replay("RANDOM", REPLAY_SEED), &|shared| {
        shared.course_run = Some(rbms_course::CourseRun::new(rbms_course::Course::default(), COURSE_OPENING_GAUGE));
    });
    refused("layout-while-autoplay-is-on", a_replay("RANDOM", REPLAY_SEED), &|shared| shared.config.play.autoplay = true);
}

/// A browser whose first chart's board holds one row that has a replay and one that has none, with
/// the ranking panel open on it and the first row selected.
fn with_the_ranking_panel_open(tag: &str) -> Browser {
    let mut browser = Browser::three_songs(tag);
    let shared = &mut browser.app.shared;
    shared.config.play.autoplay = false;
    shared.config.network.server_url = Some(SERVER_URL.to_owned());
    shared.multi_ir = MultiIr::from_network(&shared.config.network);
    shared.profile_servers = vec![Arc::new(RankingDouble::new(Vec::new()))];
    let rows = [record_with_replay("a", PLAYER, 1500, None, REPLAY_ID), crate::ir_ranking::tests::record("b", "Bob", 1400, None)];
    shared.ranking_cache.insert(FIRST_CHART.to_owned(), RankingState::Ready(Box::new(build_board(&rows, None, &[]))));
    browser.frame_after(SHORT_STEP);
    browser.key(KeyCode::KeyI);
    assert!(browser.state.ranking_open, "the ranking panel did not open");
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.ranking_sel, 0);
    browser
}

/// Hand the browser a replay as the download hands it, for the chart it was asked for.
fn deliver(browser: &mut Browser, answer: Result<ReplayData, rbms_ir::IrError>) -> Transition {
    let (sender, receiver) = channel();
    sender.send(answer).expect("the receiver is held");
    let path = browser.app.shared.library.songs()[0].path.to_string_lossy().to_string();
    browser.app.shared.replay_download_target = Some((path, FIRST_CHART.to_string()));
    browser.app.shared.replay_download_rx = Some(receiver);
    browser.frame_after(SHORT_STEP)
}

/// What a score server holds of a replay of the first chart played on `random` from `seed`.
fn downloaded(random: Option<RandomOption>, seed: Option<u64>, chart_md5: &str) -> ReplayData {
    ReplayData { random, seed, chart: Some(ChartId { md5: chart_md5.to_string(), sha256: String::new() }), ..ReplayData::default() }
}

#[test]
fn the_layout_key_of_the_ranking_panel_asks_for_the_replay_of_the_selected_row_to_play_its_layout() {
    let mut browser = with_the_ranking_panel_open("layout-panel-key");
    let asked = browser.key(layout_key(&browser));
    assert!(!starts_a_chart(&asked), "the layout started before the replay came");
    assert_eq!(browser.state.layout_download.as_deref(), Some(PLAYER));
    assert!(browser.app.shared.replay_download_rx.is_some(), "no replay was asked for");
    let target = browser.app.shared.replay_download_target.clone().expect("the chart the replay is for");
    assert_eq!(target.1, FIRST_CHART);
    assert!(browser.app.shared.net_status.contains(REPLAY_ID), "{}", browser.app.shared.net_status);
}

#[test]
fn the_panels_other_ways_to_play_a_row_still_ask_for_a_replay_to_watch() {
    let mut browser = with_the_ranking_panel_open("layout-panel-watch");
    browser.key(KeyCode::Enter);
    assert!(browser.app.shared.replay_download_rx.is_some(), "no replay was asked for");
    assert_eq!(browser.state.layout_download, None, "a replay to watch was asked for as a layout");
}

#[test]
fn a_row_with_no_replay_has_no_layout_to_ask_for() {
    let mut browser = with_the_ranking_panel_open("layout-panel-no-replay");
    browser.key(KeyCode::ArrowDown);
    assert_eq!(browser.state.ranking_sel, 1);
    browser.key(layout_key(&browser));
    assert_eq!(browser.app.shared.net_status, "no replay for that row");
    assert!(browser.app.shared.replay_download_rx.is_none() && browser.state.layout_download.is_none());
}

#[test]
fn the_status_lines_of_a_replay_are_worded_in_the_language_they_are_set_in() {
    let mut browser = with_the_ranking_panel_open("layout-panel-status-language");
    let _korean = InstalledLanguage::new(Language::Korean);
    browser.key(KeyCode::ArrowDown);
    browser.key(layout_key(&browser));
    assert_eq!(browser.app.shared.net_status, "해당 줄에는 리플레이가 없습니다");

    browser.state.layout_download = Some(PLAYER.to_string());
    deliver(&mut browser, Err(rbms_ir::IrError::Network("down".to_string())));
    assert!(browser.app.shared.net_status.starts_with("리플레이 다운로드: "), "{}", browser.app.shared.net_status);
}

#[test]
fn a_key_of_the_lanes_is_never_the_layout_key_of_the_panel() {
    let mut browser = with_the_ranking_panel_open("layout-panel-lane");
    let key = layout_key(&browser);
    browser.app.shared.active_keys.push((key, FIRST_LANE));
    browser.key(key);
    assert!(browser.app.shared.replay_download_rx.is_none() && browser.state.layout_download.is_none(), "a key that plays a lane asked for a layout");
}

#[test]
fn a_controller_button_given_the_shortcut_asks_for_the_layout_while_the_panel_is_open_and_does_nothing_otherwise() {
    let mut browser = with_the_ranking_panel_open("layout-panel-pad");
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
    let asked = browser.state.handle_pad(&mut ctx, PadEvent::Shortcut(Shortcut::ReplayPlayLayout));
    assert!(!starts_a_chart(&asked));
    assert_eq!(browser.state.layout_download.as_deref(), Some(PLAYER), "the button asked for no layout");

    let mut closed = Browser::three_songs("layout-panel-pad-closed");
    let mut ctx = FrameCtx { shared: &mut closed.app.shared, now: closed.now, dt: 0.0 };
    closed.state.handle_pad(&mut ctx, PadEvent::Shortcut(Shortcut::ReplayPlayLayout));
    assert!(closed.state.layout_download.is_none() && closed.app.shared.replay_download_rx.is_none(), "a button asked for a layout with no panel open");
}

#[test]
fn a_replay_that_comes_for_a_layout_starts_the_chart_on_it_and_closes_the_panel() {
    let mut browser = with_the_ranking_panel_open("layout-panel-arrives");
    browser.state.layout_download = Some(PLAYER.to_string());
    let asked = deliver(&mut browser, Ok(downloaded(Some(RandomOption::Spiral), Some(REPLAY_SEED), FIRST_CHART)));
    assert!(starts_a_chart(&asked), "the chart was not started");
    assert!(!browser.state.ranking_open, "the panel is still up over the run");
    assert_eq!(borrowed(&browser), Some(BorrowedLayout { random: NoteOption::Rotate, seed: REPLAY_SEED, source: PLAYER.to_string() }));
    assert!(browser.app.shared.borrowed_kept_run(FIRST_CHART).is_none(), "a replay from a server was taken for a run of this machine");
    assert_eq!(browser.state.layout_download, None);
}

#[test]
fn a_replay_that_comes_for_a_layout_and_cannot_lend_one_leaves_the_panel_up() {
    let refused = |tag: &str, data: ReplayData| {
        let mut browser = with_the_ranking_panel_open(tag);
        browser.state.layout_download = Some(PLAYER.to_string());
        let asked = deliver(&mut browser, Ok(data));
        assert!(!starts_a_chart(&asked), "{tag}: the chart was started");
        assert!(browser.state.ranking_open, "{tag}: the panel went away over a notice");
        assert!(borrowed(&browser).is_none(), "{tag}: a layout was borrowed");
        assert_eq!(browser.state.layout_download, None, "{tag}: the ask outlived its replay");
    };
    refused("layout-arrives-converge", downloaded(Some(RandomOption::Converge), Some(REPLAY_SEED), FIRST_CHART));
    refused("layout-arrives-no-seed", downloaded(Some(RandomOption::Random), None, FIRST_CHART));
    refused("layout-arrives-other-chart", downloaded(Some(RandomOption::Random), Some(REPLAY_SEED), "0123456789abcdef"));
}

#[test]
fn a_download_that_fails_leaves_the_panel_up_and_forgets_what_it_was_for() {
    let mut browser = with_the_ranking_panel_open("layout-arrives-failed");
    browser.state.layout_download = Some(PLAYER.to_string());
    let asked = deliver(&mut browser, Err(rbms_ir::IrError::Network("down".to_string())));
    assert!(!starts_a_chart(&asked) && borrowed(&browser).is_none());
    assert!(browser.state.ranking_open, "the panel went away over a failed download");
    assert_eq!(browser.state.layout_download, None);
    assert!(browser.app.shared.net_status.starts_with("replay download"), "{}", browser.app.shared.net_status);
}

#[test]
fn a_replay_that_comes_for_a_chart_the_library_no_longer_holds_borrows_nothing() {
    let mut browser = with_the_ranking_panel_open("layout-arrives-gone");
    browser.state.layout_download = Some(PLAYER.to_string());
    let (sender, receiver) = channel();
    sender.send(Ok(downloaded(Some(RandomOption::Random), Some(REPLAY_SEED), ""))).expect("the receiver is held");
    browser.app.shared.replay_download_target = Some(("gone.bms".to_string(), "md5-gone".to_string()));
    browser.app.shared.replay_download_rx = Some(receiver);
    let asked = browser.frame_after(SHORT_STEP);
    assert!(!starts_a_chart(&asked) && borrowed(&browser).is_none());
}

#[test]
fn the_ranking_panel_of_the_built_in_browser_names_the_chart_in_its_head() {
    let songs = ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec();
    let mut browser = Browser::over("layout-built-in-heading", songs, false);
    browser.frame();
    browser.key(KeyCode::KeyI);
    assert!(browser.state.ranking_open);
    browser.frame_after(SHORT_STEP);
    let panel = rbms_render::select_layout().detail_rect;
    let lines = browser.state.ranking_lines(&browser.app.shared);
    let head = |heading: Option<&str>| {
        let mut canvas = rbms_render::CpuCanvas::new(CW, CH);
        crate::ir_ranking_view::render_ranking_panel_at(&mut canvas, panel, heading, &lines, 0, true, false);
        canvas
    };
    let (named, unnamed) = (head(Some("alpha")), head(None));
    let in_the_head_that_differs = |one: &rbms_render::CpuCanvas, other: &rbms_render::CpuCanvas| {
        let mut columns = (panel.x as u32..(panel.x + panel.w) as u32)
            .filter(|x| (panel.y as u32..panel.y as u32 + HEAD_H).any(|y| one.pixel_at(*x, y) != other.pixel_at(*x, y)));
        columns.next().is_some()
    };
    assert!(in_the_head_that_differs(&named, &unnamed), "the fixture cannot tell a head with a name from one without");
    let drawn = |x: u32, y: u32| browser.pixels.pixel_at(x, y);
    let matches_named =
        (panel.x as u32..(panel.x + panel.w) as u32).all(|x| (panel.y as u32..panel.y as u32 + HEAD_H).all(|y| drawn(x, y) == named.pixel_at(x, y)));
    assert!(matches_named, "the built-in browser's panel is not the one with the chart's name in its head");
}
