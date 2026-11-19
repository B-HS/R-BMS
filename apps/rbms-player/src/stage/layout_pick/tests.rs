//! LAYOUT PICK as it is worked: the rows written by number keys and by lane keys, the two sides of a
//! double chart, the preview moved a measure at a time, and the start -- played as RANDOM with the
//! seed of the layout, or refused and told why.

use std::path::Path;
use std::time::{Duration, Instant};

use rbms_library::{Library, SongEntry};

use super::*;
use crate::App;
use crate::borrowed_layout::BorrowedLayout;
use crate::captured_layouts::{CAPTURED_LAYOUTS_FILE, CapturedLayouts, CapturedRun, RunKind};
use crate::select_tree::SelectFolder;
use crate::stage::render_tests::app;
use crate::stage::select::tests::press;
use crate::stage::{HeadlessCanvas, SelectState, Stage};

/// How long a test waits for the chart to be read or the seed to be found.
const WAIT: Duration = Duration::from_secs(30);

/// How long a test sleeps between two looks at the threads.
const POLL_EVERY: Duration = Duration::from_millis(2);

/// A seven-key chart: each key lane and the turntable once, over two measures, and a third measure.
const SEVEN_KEY_CHART: &str = "#PLAYER 1\n#BPM 150\n#WAV01 a.wav\n#00111:01000000\n#00112:00010000\n#00113:00000100\n#00114:00000001\n#00215:01000000\n#00218:00010000\n#00219:00000100\n#00216:00000001\n#00311:0101\n";

/// A fourteen-key chart: a note on each side's first key lane.
const FOURTEEN_KEY_CHART: &str = "#PLAYER 3\n#BPM 150\n#WAV01 a.wav\n#00111:01\n#00119:01\n#00121:01\n#00129:01\n";

/// A ten-key chart, which is a bmson one: a BMS file of two sides is always read as fourteen keys,
/// and only a bmson document's mode hint names ten.
const TEN_KEY_CHART: &str =
    r#"{"info":{"init_bpm":120,"resolution":240,"mode_hint":"beat-10k"},"sound_channels":[{"name":"a.ogg","notes":[{"x":1,"y":0},{"x":8,"y":240}]}]}"#;

/// The file names a BMS chart and a bmson one are written under.
const BMS_FILE: &str = "chart.bms";
const BMSON_FILE: &str = "chart.bmson";

/// An app whose library holds one chart, written to a file of its own, stated in the library as of
/// `mode` and with the hash `md5` -- or the file's own hash when that is `None`. A ten-key chart is
/// written as bmson.
fn with_chart(tag: &str, text: &str, mode: Mode, md5: Option<&str>) -> App {
    let dir = std::env::temp_dir().join(format!("rbms-layout-pick-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temp dir");
    let path = dir.join(if mode == Mode::BEAT_10K { BMSON_FILE } else { BMS_FILE });
    std::fs::write(&path, text).expect("the chart is written");
    let own = decode_chart(text.as_bytes(), &path.to_string_lossy()).expect("a chart").md5;
    let mut app = app();
    app.shared.library = Library::from_songs(vec![entry(&path, mode, md5.map_or(own, str::to_string))]);
    let captures = dir.join(CAPTURED_LAYOUTS_FILE);
    let _ = std::fs::remove_file(&captures);
    app.shared.captured_layouts = CapturedLayouts::load(&captures);
    app
}

fn entry(path: &Path, mode: Mode, md5: String) -> SongEntry {
    SongEntry {
        path: path.to_path_buf(),
        title: "a chart".to_string(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: "5".to_string(),
        difficulty: 0,
        init_bpm: 150.0,
        rank: 2,
        total: 260.0,
        mode,
        md5,
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }
}

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }
}

fn key(app: &mut App, state: &mut LayoutPickState, code: KeyCode) -> Transition {
    state.handle_key(&mut ctx(app), press(code))
}

/// Frames until the chart is read and the seed search is over.
fn settle(app: &mut App, state: &mut LayoutPickState) {
    let deadline = Instant::now() + WAIT;
    loop {
        state.update(&mut ctx(app));
        let reading = matches!(state.preview, Preview::Reading(_));
        let looking = matches!(state.seed, SeedLook::Looking { .. });
        if !reading && !looking {
            return;
        }
        assert!(Instant::now() < deadline, "the chart was not read or the seed not found in time");
        std::thread::sleep(POLL_EVERY);
    }
}

/// The key the key configuration plays `lane` of `mode` on.
fn lane_key(app: &App, mode: Mode, lane: usize) -> KeyCode {
    app.shared.keyconfig.lane_keys(mode).into_iter().find(|(_, bound)| *bound == lane).map(|(code, _)| code).expect("the lane has a key")
}

fn rows_text(state: &LayoutPickState) -> Vec<String> {
    state.layout().iter().map(|row| layout_row_text(row)).collect()
}

/// A row puts each lane typed in the next place and the rest in order after them. A lane it already
/// has, or one that is not the side's, is refused; a row typed to its end is begun again by the next
/// lane; and Backspace, Delete and 0 take back one place, every place, or type the row in order.
#[test]
fn a_row_takes_the_lanes_typed_and_fills_in_the_rest_in_order() {
    let mut row = SideRow::new(7, None);
    assert_eq!(row.row(), vec![1, 2, 3, 4, 5, 6, 7]);
    assert!(row.type_lane(3) && row.type_lane(5));
    assert_eq!((row.row(), row.typed_len()), (vec![3, 5, 1, 2, 4, 6, 7], 2));
    assert!(!row.type_lane(3), "a lane typed twice");
    assert!(!row.type_lane(8), "a lane past the side");
    assert!(!row.type_lane(0), "a lane zero");
    row.erase();
    assert_eq!(row.row(), vec![3, 1, 2, 4, 5, 6, 7]);
    row.clear();
    assert_eq!((row.row(), row.typed_len()), (vec![1, 2, 3, 4, 5, 6, 7], 0));
    for number in [7, 6, 5, 4, 3, 2, 1] {
        assert!(row.type_lane(number));
    }
    assert_eq!(row.row(), vec![7, 6, 5, 4, 3, 2, 1]);
    assert!(row.type_lane(7), "a whole row is begun again");
    assert_eq!((row.row(), row.typed_len()), (vec![7, 1, 2, 3, 4, 5, 6], 1));
    assert!(!row.type_lane(9), "a refused lane does not begin a whole row again");
    row.in_order();
    assert_eq!((row.row(), row.typed_len()), (vec![1, 2, 3, 4, 5, 6, 7], 7));
    assert_eq!(SideRow::new(7, Some(&vec![1, 1, 2, 3, 4, 5, 6])).typed_len(), 0, "a kept row that is no layout is not taken");
}

/// Number keys and the lane keys of the chart's mode write the row, and the turntable does not.
#[test]
fn number_keys_and_lane_keys_write_the_row() {
    let mut app = with_chart("keys", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    let mut state = LayoutPickState::open(&app.shared, 0);
    assert_eq!(rows_text(&state), ["1234567"]);
    key(&mut app, &mut state, KeyCode::Digit3);
    let fifth = lane_key(&app, Mode::BEAT_7K, 4);
    key(&mut app, &mut state, fifth);
    key(&mut app, &mut state, KeyCode::Numpad1);
    assert_eq!(rows_text(&state), ["3512467"]);
    let turntable = lane_key(&app, Mode::BEAT_7K, 7);
    key(&mut app, &mut state, turntable);
    assert_eq!(rows_text(&state), ["3512467"], "the turntable typed a lane");
    key(&mut app, &mut state, KeyCode::Backspace);
    assert_eq!((rows_text(&state), state.rows[0].typed_len()), (vec!["3512467".to_string()], 2), "Backspace took back the 1");
    key(&mut app, &mut state, KeyCode::Delete);
    assert_eq!(rows_text(&state), ["1234567"]);
    key(&mut app, &mut state, KeyCode::Digit9);
    assert_eq!(rows_text(&state), ["1234567"], "a lane past the side");
    assert!(state.notice.is_some(), "a refused lane is not said");
}

/// On a double chart Tab turns to the other side's row, a number goes to the side turned to, and a
/// lane key writes its own side's row as that side's lane number.
#[test]
fn tab_turns_to_the_other_side_and_a_lane_key_writes_its_own_side() {
    let mut app = with_chart("sides", FOURTEEN_KEY_CHART, Mode::BEAT_14K, None);
    let mut state = LayoutPickState::open(&app.shared, 0);
    assert_eq!(rows_text(&state), ["1234567", "1234567"], "both sides start in order");
    key(&mut app, &mut state, KeyCode::Tab);
    assert_eq!(state.side(), 1);
    key(&mut app, &mut state, KeyCode::Digit7);
    assert_eq!(rows_text(&state), ["1234567", "7123456"]);
    let first_of_left = lane_key(&app, Mode::BEAT_14K, 0);
    let third_of_right = lane_key(&app, Mode::BEAT_14K, 10);
    key(&mut app, &mut state, first_of_left);
    assert_eq!((state.side(), rows_text(&state)), (0, vec!["1234567".to_string(), "7123456".to_string()]));
    key(&mut app, &mut state, third_of_right);
    assert_eq!((state.side(), rows_text(&state)), (1, vec!["1234567".to_string(), "7312456".to_string()]));
    key(&mut app, &mut state, KeyCode::Tab);
    assert_eq!(state.side(), 0, "Tab goes round the two sides");
}

/// What was written is kept for the mode when the screen is left, and is what it opens with next.
#[test]
fn the_screen_opens_on_the_layout_last_written_for_the_mode() {
    let mut app = with_chart("kept", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    let mut state = LayoutPickState::open(&app.shared, 0);
    for code in [KeyCode::Digit3, KeyCode::Digit5] {
        key(&mut app, &mut state, code);
    }
    assert!(matches!(key(&mut app, &mut state, KeyCode::Escape), Transition::Back));
    state.on_exit(&mut ctx(&mut app));
    let again = LayoutPickState::open(&app.shared, 0);
    assert_eq!(rows_text(&again), ["3512467"]);
    assert_eq!(app.shared.picked_layouts.of(Mode::BEAT_14K), None, "another mode is not written");
}

/// The layout is played as RANDOM with the first seed that gives it, on a layout borrowed for the
/// run, and the run takes the screen's place.
#[test]
fn the_layout_is_played_as_random_with_its_seed() {
    let mut app = with_chart("start", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    app.shared.config.play.autoplay = false;
    let mut state = LayoutPickState::open(&app.shared, 0);
    for code in [KeyCode::Digit3, KeyCode::Digit5, KeyCode::Digit1, KeyCode::Digit7, KeyCode::Digit2, KeyCode::Digit4, KeyCode::Digit6] {
        key(&mut app, &mut state, code);
    }
    settle(&mut app, &mut state);
    assert_eq!(state.found_seed(), Some(4761));
    let started = key(&mut app, &mut state, KeyCode::Enter);
    assert!(matches!(started, Transition::To(Stage::Loading(_) | Stage::Decide(_))), "the run did not take the screen's place");
    let borrowed = app.shared.borrowed_layout().cloned();
    assert_eq!(borrowed.map(|BorrowedLayout { random, seed, .. }| (random, seed)), Some((NoteOption::Random, 4761)));
}

/// While AUTOPLAY is on the layout is previewed and its seed found, but it is not played.
#[test]
fn a_start_is_refused_while_autoplay_is_on_and_the_preview_still_draws() {
    let mut app = with_chart("autoplay", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    app.shared.config.play.autoplay = true;
    let mut state = LayoutPickState::open(&app.shared, 0);
    settle(&mut app, &mut state);
    assert!(state.found_seed().is_some());
    assert!(matches!(key(&mut app, &mut state, KeyCode::Enter), Transition::Stay));
    assert!(app.shared.borrowed_layout().is_none());
    assert!(matches!(state.notice, Some((Notice::Says(LayoutPickText::AutoplayOn), _))));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    state.draw(&mut ctx(&mut app), &mut Canvas::Headless(&mut pixels));
    assert!(pixels.lit_pixels() > 0, "the preview drew nothing");
}

/// A ten-key pair RANDOM never gives is found to be one and is not played.
#[test]
fn a_ten_key_pair_random_never_gives_is_refused() {
    let mut app = with_chart("ten", TEN_KEY_CHART, Mode::BEAT_10K, None);
    app.shared.config.play.autoplay = false;
    let mut state = LayoutPickState::open(&app.shared, 0);
    key(&mut app, &mut state, KeyCode::Tab);
    key(&mut app, &mut state, KeyCode::Digit5);
    assert_eq!(rows_text(&state), ["12345", "51234"]);
    settle(&mut app, &mut state);
    assert!(matches!(state.seed, SeedLook::Done { answer: Err(SeedSearchError::Unreachable), .. }));
    assert!(matches!(key(&mut app, &mut state, KeyCode::Enter), Transition::Stay));
    assert!(matches!(state.notice, Some((Notice::Says(LayoutPickText::Unreachable), _))), "{:?}", state.notice);
    key(&mut app, &mut state, KeyCode::Delete);
    settle(&mut app, &mut state);
    assert!(state.found_seed().is_some(), "both sides in order is a layout RANDOM gives");
}

/// A chart file that is no longer the one the library scanned is previewed but not played.
#[test]
fn a_chart_changed_since_the_scan_is_not_played() {
    let mut app = with_chart("changed", SEVEN_KEY_CHART, Mode::BEAT_7K, Some("0123456789abcdef0123456789abcdef"));
    app.shared.config.play.autoplay = false;
    let mut state = LayoutPickState::open(&app.shared, 0);
    settle(&mut app, &mut state);
    assert!(matches!(key(&mut app, &mut state, KeyCode::Enter), Transition::Stay));
    assert!(matches!(state.notice, Some((Notice::Says(LayoutPickText::ChartChanged), _))));
}

/// The preview moves a measure at a time on the arrows and the turntable, four on the page keys, to
/// either end on Home and End, and never past an end.
#[test]
fn the_preview_moves_a_measure_at_a_time() {
    let mut app = with_chart("measures", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    let mut state = LayoutPickState::open(&app.shared, 0);
    settle(&mut app, &mut state);
    let last = state.measures().len() - 1;
    assert!(last >= 2, "the chart has three measures: {:?}", state.measures());
    key(&mut app, &mut state, KeyCode::ArrowUp);
    assert_eq!(state.measure(), 1);
    key(&mut app, &mut state, KeyCode::ArrowDown);
    key(&mut app, &mut state, KeyCode::ArrowDown);
    assert_eq!(state.measure(), 0, "the preview went before the chart's start");
    key(&mut app, &mut state, KeyCode::PageUp);
    assert_eq!(state.measure(), last, "the page key went past the end");
    key(&mut app, &mut state, KeyCode::Home);
    assert_eq!(state.measure(), 0);
    key(&mut app, &mut state, KeyCode::End);
    assert_eq!(state.measure(), last);
    let turntable = lane_key(&app, Mode::BEAT_7K, 7);
    key(&mut app, &mut state, KeyCode::Home);
    key(&mut app, &mut state, turntable);
    assert_eq!(state.measure(), 1, "the turntable turned forwards");
    state.handle_scroll(&mut ctx(&mut app), 1.0);
    assert_eq!(state.measure(), 0, "the wheel rolled towards the player");
}

/// The browser's key opens the screen on a chart, and on a folder only says that it does not.
#[test]
fn the_key_opens_the_screen_on_a_chart_and_not_on_a_folder() {
    let mut app = with_chart("browser", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    let mut browser = SelectState::new();
    app.shared.rebuild_select_items();
    let code = Shortcut::LayoutPick.default_key();
    assert!(app.shared.focused_song_index().is_none(), "the root has a chart under the cursor");
    assert!(matches!(browser.handle_key(&mut ctx(&mut app), press(code)), Transition::Stay));
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.rebuild_select_items();
    assert_eq!(app.shared.focused_song_index(), Some(0));
    assert!(matches!(browser.handle_key(&mut ctx(&mut app), press(code)), Transition::Open(Stage::LayoutPick(_))));
}

/// A frame of the screen that was up for `dt_seconds`.
fn frame(app: &mut App, state: &mut LayoutPickState, dt_seconds: f32) {
    state.update(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: dt_seconds });
}

/// How long the frames of the clock test are up, in seconds and in the clock's microseconds.
const HALF_A_SECOND: f32 = 0.5;
const HALF_A_SECOND_US: i64 = 500_000;

/// A frame long enough to run past the end of any chart of these tests.
const PAST_THE_END: f32 = 60.0;

/// The preview plays on the screen's own clock from the moment the chart is read: it moves on by
/// the time each frame was up, stands still while stopped, moves by measures on the page keys, goes
/// back to the top on Home, keeps its place when the rows are written again, and begins the chart
/// again once it has run past its end.
#[test]
fn the_preview_plays_on_the_screens_clock_and_begins_again_at_the_end() {
    let mut app = with_chart("clock", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    let mut state = LayoutPickState::open(&app.shared, 0);
    settle(&mut app, &mut state);
    assert_eq!((state.clock().at_us(), state.clock().is_playing()), (0, true), "the preview did not open playing from the top");
    frame(&mut app, &mut state, HALF_A_SECOND);
    assert_eq!(state.clock().at_us(), HALF_A_SECOND_US);
    key(&mut app, &mut state, KeyCode::Digit3);
    assert_eq!(state.clock().at_us(), HALF_A_SECOND_US, "writing the rows moved the preview");
    key(&mut app, &mut state, KeyCode::Space);
    frame(&mut app, &mut state, HALF_A_SECOND);
    assert_eq!((state.clock().at_us(), state.clock().is_playing()), (HALF_A_SECOND_US, false), "a stopped preview moved");
    key(&mut app, &mut state, KeyCode::Space);
    frame(&mut app, &mut state, HALF_A_SECOND);
    assert_eq!(state.clock().at_us(), 2 * HALF_A_SECOND_US, "the preview did not play again");
    let last = state.measures().len() - 1;
    key(&mut app, &mut state, KeyCode::PageUp);
    assert_eq!((state.measure(), state.clock().at_us()), (last, state.measures()[last]), "the page key did not go four measures on, to the last");
    key(&mut app, &mut state, KeyCode::PageDown);
    assert_eq!((state.measure(), state.clock().at_us()), (0, 0));
    frame(&mut app, &mut state, HALF_A_SECOND);
    key(&mut app, &mut state, KeyCode::Home);
    assert_eq!(state.clock().at_us(), 0, "Home did not go back to the top");
    frame(&mut app, &mut state, PAST_THE_END);
    assert_eq!((state.clock().at_us(), state.clock().is_playing()), (0, true), "the preview did not begin the chart again");
}

/// The clock on its own: a frame of no time or of negative time leaves it, and the measures it
/// moves by are counted from the one it is in.
#[test]
fn the_preview_clock_counts_frames_and_measures() {
    let starts = [0, 1_000, 2_000, 3_000];
    let mut clock = PreviewClock::default();
    clock.advance(0.0, 10_000);
    clock.advance(-1.0, 10_000);
    assert_eq!(clock.at_us(), 0);
    clock.seek(1_500);
    assert_eq!(clock.measure(&starts), 1);
    clock.move_measures(&starts, 1);
    assert_eq!(clock.at_us(), 2_000);
    clock.move_measures(&starts, -MEASURES_PER_PAGE);
    assert_eq!(clock.at_us(), 0);
    clock.seek(1_500);
    clock.move_measures(&[], 1);
    assert_eq!(clock.at_us(), 1_500, "a chart not read yet moved the clock");
    clock.seek(-5);
    assert_eq!(clock.at_us(), 0);
    assert_eq!(clock::end_of(4_000), 4_000 + clock::END_TAIL_US);
}

/// A capture of the app's first chart laid out by `option` from `seed`, as a run of it by hand is
/// captured.
fn capture_of(app: &App, option: NoteOption, seed: u64) -> CapturedLayout {
    let entry = &app.shared.library.songs()[0];
    let run = CapturedRun { md5: &entry.md5, title: &entry.title, subtitle: "", mode: entry.mode, option, seed, ex_score: 100, lamp: "HARD" };
    run.capture(RunKind::default()).expect("the run is captured")
}

/// Keep `capture` at `at_ms`.
fn keep(app: &mut App, capture: CapturedLayout, at_ms: i64) {
    app.shared.captured_layouts.keep(capture, at_ms);
}

/// A seed MIRROR is captured with: any seed lays MIRROR out the same, and a seed of zero is never
/// one a run had.
const MIRROR_SEED: u64 = 99;

/// The seed RANDOM lays the seven-key chart out as `3517246` from.
const SEED_3517246: u64 = 4761;

/// The moments the captures of a test are kept at, the later one first in the list.
const EARLIER_MS: i64 = 1_700_000_000_000;
const LATER_MS: i64 = 1_700_000_060_000;

/// An app with the seven-key chart and two captures of it: MIRROR, and later RANDOM as `3517246`.
fn with_two_captures(tag: &str) -> App {
    let mut app = with_chart(tag, SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    app.shared.config.play.autoplay = false;
    let mirror = capture_of(&app, NoteOption::Mirror, MIRROR_SEED);
    keep(&mut app, mirror, EARLIER_MS);
    let random = capture_of(&app, NoteOption::Random, SEED_3517246);
    keep(&mut app, random, LATER_MS);
    app
}

/// The cursor on a capture of the chart writes the rows as it and lays the preview out so with no
/// seed to look for, and the layout is played on the capture's own arrangement and seed.
#[test]
fn a_captured_layout_is_shown_and_played_on_its_own_arrangement_and_seed() {
    let mut app = with_two_captures("captured");
    let mut state = LayoutPickState::open(&app.shared, 0);
    settle(&mut app, &mut state);
    key(&mut app, &mut state, KeyCode::ArrowRight);
    assert_eq!(rows_text(&state), ["3517246"], "the newest capture is not on the rows");
    assert!(matches!(state.seed, SeedLook::Captured), "a seed was looked for a captured layout");
    key(&mut app, &mut state, KeyCode::ArrowDown);
    assert_eq!(rows_text(&state), ["7654321"]);
    assert_eq!(state.status(Language::English), "PLAYED AS MIRROR, SEED 99 (CAPTURED)");
    let started = key(&mut app, &mut state, KeyCode::Enter);
    assert!(matches!(started, Transition::To(Stage::Loading(_) | Stage::Decide(_))), "the run did not take the screen's place");
    let borrowed = app.shared.borrowed_layout().cloned();
    assert_eq!(borrowed.map(|BorrowedLayout { random, seed, .. }| (random, seed)), Some((NoteOption::Mirror, MIRROR_SEED)));
}

/// A captured layout written over on the rows is the player's own pick again, looked for and played
/// as RANDOM; typed back as it was, it is the capture's again.
#[test]
fn a_captured_layout_written_over_is_played_as_the_rows_are() {
    let mut app = with_two_captures("captured-over");
    let mut state = LayoutPickState::open(&app.shared, 0);
    settle(&mut app, &mut state);
    key(&mut app, &mut state, KeyCode::ArrowRight);
    key(&mut app, &mut state, KeyCode::ArrowDown);
    key(&mut app, &mut state, KeyCode::ArrowLeft);
    key(&mut app, &mut state, KeyCode::Backspace);
    key(&mut app, &mut state, KeyCode::Backspace);
    assert_eq!(rows_text(&state), ["7654312"]);
    assert!(!matches!(state.seed, SeedLook::Captured), "the rows are no longer the capture's");
    key(&mut app, &mut state, KeyCode::Digit2);
    assert_eq!(rows_text(&state), ["7654321"], "the same lanes typed back");
    assert!(matches!(state.seed, SeedLook::Captured), "the rows are the capture's again");
    key(&mut app, &mut state, KeyCode::Digit0);
    settle(&mut app, &mut state);
    let seed = state.found_seed().expect("OFF is looked for as RANDOM again");
    assert!(matches!(key(&mut app, &mut state, KeyCode::Enter), Transition::To(_)));
    assert_eq!(app.shared.borrowed_layout().map(|layout| (layout.random, layout.seed)), Some((NoteOption::Random, seed)));
}

/// Delete pressed twice on the list removes the capture under the cursor, from the file too; a key
/// pressed between the two takes the first back.
#[test]
fn a_captured_layout_is_removed_on_a_second_delete() {
    let mut app = with_two_captures("removed");
    let mut state = LayoutPickState::open(&app.shared, 0);
    key(&mut app, &mut state, KeyCode::ArrowRight);
    key(&mut app, &mut state, KeyCode::Delete);
    assert!(state.list.is_armed() && matches!(state.notice, Some((Notice::Says(LayoutPickText::RemoveArmed), _))));
    assert_eq!(app.shared.captured_layouts.all().len(), 2, "the first press removed the capture");
    key(&mut app, &mut state, KeyCode::ArrowDown);
    assert!(!state.list.is_armed(), "another key did not take the first press back");
    key(&mut app, &mut state, KeyCode::Delete);
    key(&mut app, &mut state, KeyCode::Delete);
    assert!(matches!(state.notice, Some((Notice::Says(LayoutPickText::Removed), _))));
    let left: Vec<String> = app.shared.captured_layouts.all().iter().map(|capture| capture.random_label.clone()).collect();
    assert_eq!(left, ["RANDOM"]);
    let path = app.shared.library.songs()[0].path.with_file_name(CAPTURED_LAYOUTS_FILE);
    assert_eq!(CapturedLayouts::load(&path).all().len(), 1, "the file still holds the removed capture");
    assert_eq!(rows_text(&state), ["3517246"], "the cursor did not come to the capture that is left");
}

/// A capture of another chart, among every chart's, is played on that chart: Enter goes there with
/// the rows written as it. One whose chart the library does not have is said to be missing.
#[test]
fn a_capture_of_another_chart_goes_to_that_chart_or_says_it_is_missing() {
    let mut app = with_chart("elsewhere", SEVEN_KEY_CHART, Mode::BEAT_7K, None);
    let first = app.shared.library.songs()[0].clone();
    let second_path = first.path.with_file_name("second.bms");
    let second_text = SEVEN_KEY_CHART.replace("#BPM 150", "#BPM 151");
    std::fs::write(&second_path, &second_text).expect("the chart is written");
    let second_md5 = decode_chart(second_text.as_bytes(), &second_path.to_string_lossy()).expect("a chart").md5;
    let missing = CapturedLayout { md5: "0123456789abcdef0123456789abcdef".to_string(), ..capture_of(&app, NoteOption::Mirror, MIRROR_SEED) };
    let elsewhere = CapturedLayout { md5: second_md5.clone(), ..capture_of(&app, NoteOption::Random, SEED_3517246) };
    keep(&mut app, missing, EARLIER_MS);
    keep(&mut app, elsewhere, LATER_MS);
    app.shared.library = Library::from_songs(vec![first, entry(&second_path, Mode::BEAT_7K, second_md5.clone())]);
    let mut state = LayoutPickState::open(&app.shared, 0);
    key(&mut app, &mut state, KeyCode::ArrowRight);
    assert!(state.list.entries(&app.shared.captured_layouts, &state.md5).is_empty(), "another chart's capture is in this chart's list");
    key(&mut app, &mut state, KeyCode::Tab);
    assert_eq!(state.list.entries(&app.shared.captured_layouts, &state.md5).len(), 2);
    assert_eq!(rows_text(&state), ["1234567"], "another chart's capture was written on this chart's rows");
    assert!(matches!(key(&mut app, &mut state, KeyCode::Enter), Transition::Stay));
    assert_eq!((state.md5.as_str(), state.focus, rows_text(&state)), (second_md5.as_str(), Focus::Captured, vec!["3517246".to_string()]));
    key(&mut app, &mut state, KeyCode::ArrowDown);
    assert!(matches!(key(&mut app, &mut state, KeyCode::Enter), Transition::Stay));
    assert!(matches!(state.notice, Some((Notice::Says(LayoutPickText::CapturedChartMissing), _))), "{:?}", state.notice);
    assert_eq!(state.md5, second_md5, "a missing chart was gone to");
}

/// On a controller SELECT goes round the rows and the list, and while the list has the keys the
/// lanes work it -- the first two keys move the cursor, the third turns to every chart's captures,
/// the fourth pressed twice removes one -- rather than writing the rows.
#[test]
fn a_controller_works_the_list_with_select_and_its_lanes() {
    let mut app = with_two_captures("pad");
    app.shared.mode = Mode::BEAT_7K;
    let mut state = LayoutPickState::open(&app.shared, 0);
    let pad = |app: &mut App, state: &mut LayoutPickState, event: PadEvent| state.handle_pad(&mut ctx(app), event);
    let lane = |lane: usize| PadEvent::Lane { lane, dir: ScratchDir::Forward, press: true };
    pad(&mut app, &mut state, PadEvent::Control(ControlAction::Select));
    assert_eq!((state.focus, rows_text(&state)), (Focus::Captured, vec!["3517246".to_string()]));
    pad(&mut app, &mut state, lane(1));
    assert_eq!(rows_text(&state), ["7654321"], "the second key did not move the cursor down");
    pad(&mut app, &mut state, lane(0));
    assert_eq!(rows_text(&state), ["3517246"], "the first key did not move the cursor up");
    pad(&mut app, &mut state, lane(2));
    assert_eq!(state.list.scope, captured::Scope::All);
    pad(&mut app, &mut state, lane(3));
    pad(&mut app, &mut state, lane(3));
    assert_eq!(app.shared.captured_layouts.all().len(), 1, "the fourth key pressed twice did not remove the capture");
    pad(&mut app, &mut state, PadEvent::Control(ControlAction::Select));
    assert_eq!(state.focus, Focus::Rows);
    pad(&mut app, &mut state, lane(1));
    assert_eq!(rows_text(&state), ["2134567"], "the rows did not take the lane");
}

/// On the keyboard as on a controller, the lane key of the fourth key pressed twice removes the
/// capture under the cursor while the list has the keys, and any other key between the two presses
/// takes the first back.
#[test]
fn the_fourth_lane_key_pressed_twice_on_the_keyboard_removes_a_capture() {
    let mut app = with_two_captures("lane-remove");
    let mut state = LayoutPickState::open(&app.shared, 0);
    key(&mut app, &mut state, KeyCode::ArrowRight);
    let fourth = lane_key(&app, Mode::BEAT_7K, LIST_KEY_REMOVE);
    let first = lane_key(&app, Mode::BEAT_7K, LIST_KEY_UP);
    key(&mut app, &mut state, fourth);
    assert!(state.list.is_armed(), "the first press did not ask for a second");
    key(&mut app, &mut state, first);
    key(&mut app, &mut state, fourth);
    assert_eq!(app.shared.captured_layouts.all().len(), 2, "a key between the two presses did not take the first back");
    key(&mut app, &mut state, fourth);
    assert_eq!(app.shared.captured_layouts.all().len(), 1, "the fourth lane key pressed twice did not remove the capture");
    assert!(matches!(state.notice, Some((Notice::Says(LayoutPickText::Removed), _))), "{:?}", state.notice);
}

/// Where a test saves what it drew when asked to, and the name it saves the screen under.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";
const CAPTURE_FILE: &str = "layout-pick-captured.png";

/// How far into the seven-key chart the drawn screen is played, in seconds: into its second measure,
/// where its notes are.
const AMONG_THE_NOTES: f32 = 1.8;

/// The screen with captures in its list draws more than the same screen with none; saved as
/// `layout-pick-captured.png` when a capture folder is named.
#[test]
fn the_screen_draws_its_list_of_captured_layouts() {
    let draw = |app: &mut App| {
        let mut state = LayoutPickState::open(&app.shared, 0);
        settle(app, &mut state);
        key(app, &mut state, KeyCode::ArrowRight);
        frame(app, &mut state, AMONG_THE_NOTES);
        let mut pixels = HeadlessCanvas::new(CW, CH);
        state.draw(&mut ctx(app), &mut Canvas::Headless(&mut pixels));
        pixels
    };
    let listed = draw(&mut with_two_captures("drawn"));
    let bare = draw(&mut with_chart("drawn-bare", SEVEN_KEY_CHART, Mode::BEAT_7K, None));
    assert!(listed.lit_pixels() > bare.lit_pixels(), "the list drew nothing");
    if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
        let folder = std::path::PathBuf::from(folder);
        std::fs::create_dir_all(&folder).expect("the capture folder can be created");
        image::save_buffer(folder.join(CAPTURE_FILE), listed.rgba(), CW, CH, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}
