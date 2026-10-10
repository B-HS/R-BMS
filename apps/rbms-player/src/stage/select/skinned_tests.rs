//! The browser a skin draws, driven the way the reference's is: its key table row by row, the
//! repeat of a held key, the wheel and the buttons of the mouse, the timers a skin is animated on,
//! and what the browser does while its skin is on its way.
//!
//! The skin here is written by the tests: a wheel of three bars and nothing else, with an input
//! time. The built-in browser's own behaviour is pinned next door in `tests.rs`, which runs with no
//! skin and so says what must not change.
use std::time::{Duration, Instant};

use rbms_library::{Library, SongEntry};
use rbms_render::skin_render::frame::{SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS};
use rbms_skin::property::generated::RATE_MUSICSELECT_POSITION;
use rbms_skin::timer::{TimerId, timer_id};

use super::tests::{add_ir_profile, entry, press, release};
use super::*;
use crate::ir_ranking::RankingState;
use crate::pointer::{PointerInput, route_pointer};
use crate::skin_host::{Cluster, ClusterRequest};
use crate::skin_screen::texture_tests::HandDecode;
use crate::stage::HeadlessCanvas;
use crate::stage::capture::{app_in, settings_of, skin_folder_of};
use crate::{App, Config};

/// The size the fixture skin is authored at.
pub(super) const DOCUMENT_SIZE: (u32, u32) = (320, 180);

/// The side of one cell of the fixture's sheet, in pixels, and how many bar images it holds.
pub(super) const CELL: u32 = 8;
const BAR_IMAGES: u32 = 7;

/// The three times of the fixture skin's header, in milliseconds. The browser reads the first.
const INPUT_MS: u64 = 500;
const SCENE_MS: u64 = 3000;
const FADEOUT_MS: u64 = 500;

/// Where the fixture's wheel puts its three slots, top to bottom, as the bottom edge of each in the
/// skin's own coordinates, and the box they share across.
const SLOT_BOTTOMS: [u32; 3] = [120, 80, 40];
const SLOT_X: u32 = 100;
const SLOT_W: u32 = 100;
const SLOT_H: u32 = 20;

/// The slots above and below the one under the cursor.
pub(super) const SLOT_ABOVE: usize = 0;
pub(super) const SLOT_BELOW: usize = 2;

/// Longest a test waits for the worker that reads the fixture's sheet.
const COMPILE_WAIT: Duration = Duration::from_secs(20);

/// The keys the default seven-key layout puts on each key index, and the key the tests bind to the
/// turntable's other direction.
pub(super) const KEY_ONE: KeyCode = KeyCode::KeyZ;
pub(super) const KEY_TWO: KeyCode = KeyCode::KeyS;
pub(super) const KEY_THREE: KeyCode = KeyCode::KeyX;
pub(super) const KEY_FOUR: KeyCode = KeyCode::KeyD;
pub(super) const KEY_FIVE: KeyCode = KeyCode::KeyC;
pub(super) const KEY_SIX: KeyCode = KeyCode::KeyF;
pub(super) const KEY_SEVEN: KeyCode = KeyCode::KeyV;
pub(super) const TURNTABLE_FORWARD: KeyCode = KeyCode::ShiftLeft;
const TURNTABLE_BACKWARD: KeyCode = KeyCode::ControlLeft;
const TURNTABLE_LANE: usize = 7;
const KEY_ONE_LANE: usize = 0;

/// The keys START and SELECT ship on.
pub(super) const START: KeyCode = KeyCode::KeyA;
pub(super) const SELECT: KeyCode = KeyCode::KeyW;

/// A step of the frame clock no repeat is due within.
pub(super) const SHORT_STEP: Duration = Duration::from_millis(10);

/// A step of the frame clock just past the slide a held key's first move starts, after which the
/// wheel is at rest again.
pub(super) const SLIDE_REST: Duration = Duration::from_millis(SCROLL_DURATION_LOW_MS as u64 + 1);

/// How far a scene is aged between two looks at a timer, so one that started over reads later.
const TIMER_STEP: Duration = Duration::from_millis(100);

/// A stretch far longer than any time in the fixture's header.
const LONG_AFTER: Duration = Duration::from_secs(3600);

/// A scene clock reading a scene that has only just begun is under.
const JUST_BEGUN_US: i64 = 2_000_000;

/// The timers the reference has a name for on its browser and never switches.
const NEVER_SWITCHED: [TimerId; 5] =
    [timer_id::SONGBAR_MOVE, timer_id::SONGBAR_MOVE_UP, timer_id::SONGBAR_MOVE_DOWN, timer_id::SONGBAR_STOP, timer_id::FADEOUT];

pub(super) fn wheel_document() -> String {
    let slots = SLOT_BOTTOMS.map(|y| format!(r#"{{"id":"bars","dst":[{{"x":{SLOT_X},"y":{y},"w":{SLOT_W},"h":{SLOT_H}}}]}}"#)).join(",");
    let images =
        (0..BAR_IMAGES).map(|index| format!(r#"{{"id":"bar-{index}","src":"sheet","x":{},"y":0,"w":{CELL},"h":{CELL}}}"#, index * CELL)).collect::<Vec<_>>();
    let set = (0..BAR_IMAGES).map(|index| format!(r#""bar-{index}""#)).collect::<Vec<_>>();
    format!(
        r#"{{
            "type": {SKIN_TYPE_MUSIC_SELECT}, "name": "keys", "w": {}, "h": {},
            "input": {INPUT_MS}, "scene": {SCENE_MS}, "fadeout": {FADEOUT_MS},
            "source": [{{ "id": "sheet", "path": "sheet.png" }}],
            "image": [{}],
            "imageset": [{{ "id": "bars", "images": [{}] }}],
            "songlist": {{ "id": "wheel", "center": 1, "clickable": [0, 1, 2], "liston": [{slots}], "listoff": [{slots}] }},
            "destination": [{{ "id": "wheel" }}]
        }}"#,
        DOCUMENT_SIZE.0,
        DOCUMENT_SIZE.1,
        images.join(","),
        set.join(","),
    )
}

/// The cursor, in the space the window's events arrive in, at the middle of the wheel's slot `slot`.
pub(super) fn slot_middle(slot: usize) -> (f32, f32) {
    let across = (SLOT_X + SLOT_W / 2) as f32 / DOCUMENT_SIZE.0 as f32;
    let up = (SLOT_BOTTOMS[slot] + SLOT_H / 2) as f32 / DOCUMENT_SIZE.1 as f32;
    (across * CW as f32, (1.0 - up) * CH as f32)
}

/// A browser with its app and the canvas it is drawn on, and the frame clock the tests move.
pub(super) struct Browser {
    pub(super) app: App,
    pub(super) state: SelectState,
    pub(super) pixels: HeadlessCanvas,
    pub(super) now: Instant,
}

impl Browser {
    /// A browser over `songs` in title order, drawn by the fixture skin when `skinned` and by
    /// nothing but itself otherwise. Its skin has not been read yet.
    pub(super) fn over(tag: &str, songs: Vec<SongEntry>, skinned: bool) -> Browser {
        Browser::drawn_by(tag, songs, skinned.then(wheel_document))
    }

    /// A browser over `songs` in title order, drawn by `document` over the fixture's sheet, or by
    /// nothing but itself when there is none. Its skin has not been read yet.
    fn drawn_by(tag: &str, songs: Vec<SongEntry>, document: Option<String>) -> Browser {
        let settings = settings_of(tag);
        let mut config = Config::default();
        config.library.preview = false;
        if let Some(text) = document {
            let skin = skin_folder_of(&settings);
            let sheet =
                image::RgbaImage::from_fn(BAR_IMAGES * CELL, CELL, |x, _| image::Rgba([u8::try_from(40 + 30 * (x / CELL)).unwrap_or(u8::MAX), 0, 0, u8::MAX]));
            sheet.save(skin.join("sheet.png")).expect("the sheet is written");
            let document = skin.join("keys.json");
            std::fs::write(&document, text).expect("the document is written");
            config.skin.select(SKIN_TYPE_MUSIC_SELECT, Some(document.to_string_lossy().into_owned()));
        }
        let mut app = app_in(settings, config);
        app.shared.audio_failed = true;
        app.shared.library = Library::from_songs(songs);
        app.shared.select_view = SelectView::AllSongs;
        app.shared.config.library.sort = SortMode::Title;
        app.shared.rebuild_select_items();
        Browser { app, state: SelectState::new(), pixels: HeadlessCanvas::new(CW, CH), now: Instant::now() }
    }

    /// A browser the fixture skin is drawing, on the frame after its keys were first read.
    pub(super) fn skinned(tag: &str, songs: Vec<SongEntry>) -> Browser {
        Browser::skinned_by(tag, songs, wheel_document())
    }

    /// A browser `document` is drawing, on the frame after its keys were first read.
    pub(super) fn skinned_by(tag: &str, songs: Vec<SongEntry>, document: String) -> Browser {
        let mut browser = Browser::drawn_by(tag, songs, Some(document));
        let deadline = Instant::now() + COMPILE_WAIT;
        while !browser.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
            browser.frame();
            assert!(Instant::now() < deadline, "the fixture skin never compiled: {:?}", browser.app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT));
        }
        browser.frame();
        browser
    }

    /// Three charts in the flat list, with the cursor on the first.
    pub(super) fn three_songs(tag: &str) -> Browser {
        Browser::skinned(tag, ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec())
    }

    /// The root of a library of three charts with one difficulty table over two of them: the row of
    /// every chart, and the row of the table.
    pub(super) fn at_the_root(tag: &str) -> Browser {
        let mut browser = Browser::three_songs(tag);
        browser.app.shared.table_names = vec!["TABLE".to_owned()];
        browser.app.shared.table_levels = vec![vec![("1".to_owned(), vec![0, 1])]];
        browser.go_to(SelectView::Root);
        browser
    }

    fn go_to(&mut self, view: SelectView) {
        self.app.shared.select_view = view;
        self.app.shared.sel = 0;
        self.app.shared.rebuild_select_items();
        self.frame();
    }

    /// One frame at the frame clock's present: the browser's update and then, if it stays, its draw.
    pub(super) fn frame(&mut self) -> Transition {
        let now = self.now;
        let asked = self.state.update(&mut FrameCtx { shared: &mut self.app.shared, now, dt: 0.0 });
        if matches!(asked, Transition::Stay) {
            self.app.shared.hot.clear();
            self.state.draw(&mut FrameCtx { shared: &mut self.app.shared, now, dt: 0.0 }, &mut Canvas::Headless(&mut self.pixels));
        }
        asked
    }

    /// Move the frame clock on by `step` and run a frame there.
    pub(super) fn frame_after(&mut self, step: Duration) -> Transition {
        self.now += step;
        self.frame()
    }

    /// Put a key down, or let it go, the way the window's events do before any screen sees them.
    pub(super) fn hold(&mut self, code: KeyCode) {
        self.app.shared.note_key(&press(code));
    }

    pub(super) fn let_go(&mut self, code: KeyCode) {
        self.app.shared.note_key(&release(code));
    }

    /// A key held for the frame that follows and let go after it.
    pub(super) fn tap(&mut self, code: KeyCode) -> Transition {
        self.hold(code);
        let asked = self.frame_after(SHORT_STEP);
        self.let_go(code);
        match asked {
            Transition::Stay => self.frame_after(SHORT_STEP),
            asked => asked,
        }
    }

    /// One key going down, handed to the browser as its own keys are.
    pub(super) fn key(&mut self, code: KeyCode) -> Transition {
        let mut ctx = FrameCtx { shared: &mut self.app.shared, now: self.now, dt: 0.0 };
        self.state.handle_key(&mut ctx, press(code))
    }

    /// One press of an arrow that moves the cursor, let go before a frame saw it held: the press,
    /// the frame that reads it, and the wheel left to finish the slide the press started, so that
    /// the press after it does not land during a slide and go by. Answers where the cursor is.
    pub(super) fn arrow(&mut self, code: KeyCode) -> usize {
        self.key(code);
        self.frame_after(SHORT_STEP);
        self.frame_after(SLIDE_REST);
        self.sel()
    }

    /// One key of a controller going down on the lane `lane`.
    fn pad(&mut self, lane: usize) -> Transition {
        let mut ctx = FrameCtx { shared: &mut self.app.shared, now: self.now, dt: 0.0 };
        self.state.handle_pad(&mut ctx, PadEvent::Lane { lane, dir: ScratchDir::Forward, press: true })
    }

    /// The browser being arrived back on from a screen that was opened over it.
    pub(super) fn come_back(&mut self) {
        let mut ctx = FrameCtx { shared: &mut self.app.shared, now: self.now, dt: 0.0 };
        self.state.on_enter(&mut ctx);
    }

    /// One mouse event at `at`, routed as the window's are: to the skin first, then to the browser.
    pub(super) fn pointer(&mut self, at: (f32, f32), input: PointerInput) {
        let mut ctx = FrameCtx { shared: &mut self.app.shared, now: self.now, dt: 0.0 };
        route_pointer(&mut self.state, &mut ctx, at, input);
    }

    fn click(&mut self, slot: usize, button: MouseButton) -> Transition {
        self.pointer(slot_middle(slot), PointerInput::Button { button, pressed: true });
        self.frame_after(SHORT_STEP)
    }

    pub(super) fn scroll(&mut self, lines: f32) -> Transition {
        self.pointer(slot_middle(SLOT_ABOVE), PointerInput::Scroll { lines });
        self.frame_after(SHORT_STEP)
    }

    pub(super) fn sel(&self) -> usize {
        self.app.shared.sel
    }

    pub(super) fn timer(&self, id: TimerId) -> Option<i64> {
        let timers = &self.app.shared.skin_timers;
        timers.is_on(id).then(|| timers.value_us(id))
    }

    fn bind_the_turntables_other_direction(&mut self) {
        self.app.shared.keyconfig.set_scratch_reverse(Mode::BEAT_7K, TURNTABLE_LANE, TURNTABLE_BACKWARD);
        self.app.shared.active_reverse_keys = self.app.shared.keyconfig.scratch_reverse_keys(Mode::BEAT_7K);
    }
}

pub(super) fn starts_a_chart(asked: &Transition) -> bool {
    matches!(asked, Transition::Open(Stage::Loading(_)))
}

/// Key one plays the chart under the cursor and key seven, with no replay chosen, plays it too
/// (`PLAY`, `REPLAY`).
#[test]
fn key_one_and_key_seven_play_the_chart_under_the_cursor() {
    for (code, name) in [(KEY_ONE, "one"), (KEY_SEVEN, "seven")] {
        let mut browser = Browser::three_songs(&format!("keys-play-{name}"));
        browser.hold(code);
        assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)), "key {name} did not start the chart");
        assert!(!browser.app.shared.practice_requested);
    }
}

/// Key three opens the practice screen on the chart, as the browser's own practice key does
/// (`PRACTICE`).
#[test]
fn key_three_practises_the_chart_under_the_cursor() {
    let mut browser = Browser::three_songs("keys-practice");
    browser.hold(KEY_THREE);
    assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)));
    assert!(browser.app.shared.practice_requested, "the chart was started rather than practised");
}

/// Key five plays the chart by itself: the run is one that plays itself without the autoplay
/// setting being touched, and the next run is the player's own once the browser is arrived back at
/// (`AUTO`).
#[test]
fn key_five_plays_the_chart_by_itself_once() {
    let mut browser = Browser::three_songs("keys-auto");
    browser.app.shared.config.play.autoplay = false;
    browser.hold(KEY_FIVE);
    assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)), "key five did not start the chart");
    assert!(browser.app.shared.run_plays_itself(), "the chart was started for the player to play");
    assert!(!browser.app.shared.config.play.autoplay, "the autoplay setting was switched on for the run");
    assert!(!browser.app.shared.practice_requested);

    browser.let_go(KEY_FIVE);
    browser.come_back();
    assert!(!browser.app.shared.run_plays_itself(), "the run after it would play itself too");
}

/// Key six steps through replay slots the browser does not keep a choice of: the press is taken and
/// nothing happens (`NEXT_REPLAY`).
#[test]
fn key_six_is_taken_and_starts_nothing() {
    let mut browser = Browser::three_songs("keys-next-replay");
    assert!(matches!(browser.tap(KEY_SIX), Transition::Stay));
    assert_eq!(browser.sel(), 0);
    assert!(browser.app.shared.select_view == SelectView::AllSongs);
}

/// On a folder every white key opens it (`FOLDER_OPEN`), and a key held through the opening does
/// not go on to start the chart it lands on: a held key is one press.
#[test]
fn a_white_key_opens_the_folder_under_the_cursor_once() {
    for (code, name) in [(KEY_ONE, "one"), (KEY_THREE, "three"), (KEY_FIVE, "five"), (KEY_SEVEN, "seven")] {
        let mut browser = Browser::at_the_root(&format!("keys-open-{name}"));
        browser.hold(code);
        assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay));
        assert!(browser.app.shared.select_view == SelectView::AllSongs, "key {name} did not open the folder");
        assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "key {name}, still held, started the chart inside");
        assert!(!browser.app.shared.practice_requested);
    }
}

/// Keys two and four go up a folder, and at the top of the list move the order on instead
/// (`FOLDER_CLOSE`, `BarManager.close`).
#[test]
fn a_black_key_closes_the_folder_and_at_the_top_steps_the_order() {
    for (code, name) in [(KEY_TWO, "two"), (KEY_FOUR, "four")] {
        let mut browser = Browser::three_songs(&format!("keys-close-{name}"));
        assert!(matches!(browser.tap(code), Transition::Stay));
        assert!(browser.app.shared.select_view == SelectView::Root, "key {name} did not leave the folder");

        let order = browser.app.shared.config.library.sort;
        assert!(matches!(browser.tap(code), Transition::Stay), "at the top of the list a key that closes must not quit");
        assert!(browser.app.shared.select_view == SelectView::Root);
        assert_eq!(browser.app.shared.config.library.sort, order.next(), "key {name} at the top did not move the order on");
    }
}

/// The turntable moves the cursor: forward to the next bar, backward to the one before, round the
/// ends of the list (`UP`, `DOWN`, `BarManager.move`).
#[test]
fn the_turntable_moves_the_cursor_round_the_list() {
    let mut browser = Browser::three_songs("keys-turn");
    browser.bind_the_turntables_other_direction();
    browser.tap(TURNTABLE_BACKWARD);
    assert_eq!(browser.sel(), 2, "backward from the first bar is the last");

    browser.frame_after(Duration::from_millis(u64::from(SCROLL_DURATION_LOW_MS.unsigned_abs()) + 1));
    browser.tap(TURNTABLE_FORWARD);
    assert_eq!(browser.sel(), 0, "forward from the last bar is the first");
}

/// A turntable direction that stays down moves one bar, waits out the long slide, and then moves a
/// bar for every short one, until it is let go (`BarRenderer.input`, 300 ms and then 50 ms).
#[test]
fn a_held_turntable_repeats_after_the_long_slide_and_then_every_short_one() {
    let low = Duration::from_millis(u64::from(SCROLL_DURATION_LOW_MS.unsigned_abs()));
    let high = Duration::from_millis(u64::from(SCROLL_DURATION_HIGH_MS.unsigned_abs()));
    let one = Duration::from_millis(1);
    let mut browser = Browser::skinned("keys-repeat", (0..40).map(|index| entry(&format!("song{index:02}"), "a", "5")).collect());

    browser.hold(TURNTABLE_FORWARD);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1, "the press itself moves one bar");
    browser.frame_after(low);
    assert_eq!(browser.sel(), 1, "nothing repeats until the long slide is over");
    browser.frame_after(one);
    assert_eq!(browser.sel(), 2, "the long slide is over");
    browser.frame_after(high);
    assert_eq!(browser.sel(), 2, "then one bar for every short slide");
    browser.frame_after(one);
    assert_eq!(browser.sel(), 3);
    browser.frame_after(high + one);
    assert_eq!(browser.sel(), 4);

    browser.let_go(TURNTABLE_FORWARD);
    browser.frame_after(high + one);
    browser.frame_after(low);
    assert_eq!(browser.sel(), 4, "once it is up nothing repeats");
}

/// With START or SELECT held the keys belong to an option panel: the list stands still, and a key
/// pressed for the panel is not a press to the list once the panel has closed.
#[test]
fn start_or_select_held_keeps_the_keys_off_the_list() {
    for (panel, name) in [(START, "start"), (SELECT, "select")] {
        let mut browser = Browser::three_songs(&format!("keys-panel-{name}"));
        browser.hold(panel);
        browser.hold(TURNTABLE_FORWARD);
        browser.hold(KEY_ONE);
        assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "{name} held, key one started the chart");
        assert_eq!(browser.sel(), 0, "{name} held, the turntable moved the list");

        browser.let_go(TURNTABLE_FORWARD);
        browser.let_go(panel);
        assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "{name} let go, the key the panel took started the chart");

        browser.let_go(KEY_ONE);
        browser.frame_after(SHORT_STEP);
        browser.hold(KEY_ONE);
        assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)), "{name} let go, a fresh press of key one did nothing");
    }
}

/// The detail panel reads key four as held rather than as pressed, so a press of it made with the
/// panel up is still a press to the list when the panel closes: the folder closes. The reference
/// does the same.
#[test]
fn the_detail_panel_leaves_the_press_of_a_key_it_reads_as_held() {
    let mut browser = Browser::three_songs("keys-detail");
    browser.hold(START);
    browser.hold(SELECT);
    browser.hold(KEY_FOUR);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.select_view == SelectView::AllSongs, "the folder closed with the panel up");

    browser.let_go(START);
    browser.let_go(SELECT);
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.select_view == SelectView::Root);
}

/// A layout with no turntable reads the nine-button table: button five plays, button one plays the
/// chart by itself, and buttons six and four move the cursor.
#[test]
fn nine_buttons_read_their_own_table() {
    let mut browser = Browser::three_songs("keys-nine");
    browser.app.shared.mode = Mode::POPN_9K;
    browser.app.shared.active_keys = browser.app.shared.keyconfig.lane_keys(Mode::POPN_9K);
    browser.frame_after(SHORT_STEP);

    browser.tap(KEY_SIX);
    assert_eq!(browser.sel(), 1, "button six is the next bar");
    browser.frame_after(Duration::from_millis(u64::from(SCROLL_DURATION_LOW_MS.unsigned_abs()) + 1));
    browser.tap(KEY_FOUR);
    assert_eq!(browser.sel(), 0, "button four is the bar before");
    browser.app.shared.config.play.autoplay = false;
    browser.hold(KEY_FIVE);
    assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)), "button five did not play");
    assert!(!browser.app.shared.run_plays_itself(), "button five is play here, not autoplay");
    browser.let_go(KEY_FIVE);
    browser.come_back();
    browser.frame_after(SHORT_STEP);
    browser.hold(KEY_ONE);
    assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)), "button one did not start the chart");
    assert!(browser.app.shared.run_plays_itself(), "button one is autoplay here, not play");
}

/// A key of a controller is read off its state like a key of the keyboard, and a tap too short for
/// a frame to see is still a press.
#[test]
fn a_tap_of_a_controller_key_between_two_frames_is_a_press() {
    let mut browser = Browser::three_songs("keys-pad");
    assert!(matches!(browser.pad(TURNTABLE_LANE), Transition::Stay));
    assert_eq!(browser.sel(), 0, "the list waits for the frame, as the reference's does");
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1);

    browser.pad(KEY_ONE_LANE);
    assert!(starts_a_chart(&browser.frame_after(SHORT_STEP)));
}

/// What is held when the browser starts listening, or gets its keys back from something drawn over
/// it, is held and not pressed.
#[test]
fn a_key_held_while_the_keys_were_elsewhere_is_not_a_press() {
    let mut browser = Browser::three_songs("keys-elsewhere");
    browser.state.ranking_open = true;
    browser.hold(KEY_ONE);
    browser.hold(TURNTABLE_FORWARD);
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay));
    assert_eq!(browser.sel(), 0, "the list moved under the ranking panel");

    browser.let_go(TURNTABLE_FORWARD);
    browser.state.ranking_open = false;
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "a key held under the panel started the chart when it closed");

    browser.come_back();
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "a key held across a return to the browser started the chart");
}

/// The wheel of the mouse moves the cursor a bar a notch, either way, and what is left of a notch
/// waits for the rest of it (`BarRenderer.input`).
#[test]
fn the_mouse_wheel_moves_the_cursor() {
    let mut browser = Browser::skinned("wheel", (0..40).map(|index| entry(&format!("song{index:02}"), "a", "5")).collect());
    browser.scroll(1.0);
    assert_eq!(browser.sel(), 1);
    browser.scroll(-1.0);
    assert_eq!(browser.sel(), 0);
    browser.scroll(0.5);
    assert_eq!(browser.sel(), 0, "half a notch is not a bar");
    browser.scroll(0.5);
    assert_eq!(browser.sel(), 1);

    browser.state.ranking_open = true;
    browser.scroll(1.0);
    browser.state.ranking_open = false;
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1, "the wheel turned under the ranking panel moved the list");
}

/// The left button on a bar that is not a folder starts play of the bar under the cursor, whichever
/// bar it landed on (`MusicSelector.select`, then `MusicSelector.render`).
#[test]
fn a_left_press_on_a_chart_bar_plays_the_chart_under_the_cursor() {
    let mut browser = Browser::three_songs("click-play");
    let asked = browser.click(SLOT_BELOW, MouseButton::Left);
    assert!(starts_a_chart(&asked), "the press did not start a chart");
    assert_eq!(browser.sel(), 0, "the cursor went to the bar that was pressed");
}

/// The left button on a folder opens that folder, wherever the cursor is.
#[test]
fn a_left_press_on_a_folder_bar_opens_that_folder() {
    let mut browser = Browser::at_the_root("click-open");
    assert_eq!(browser.app.shared.select_items.len(), 2, "the root lists every chart and the table");
    assert!(matches!(browser.click(SLOT_BELOW, MouseButton::Left), Transition::Stay));
    assert!(browser.app.shared.select_view == SelectView::TableLevels(0), "the folder below the cursor was not the one opened");
}

/// Any other button on a bar closes the folder that is open, and at the top of the list moves the
/// order on (`BarManager.close`).
#[test]
fn another_button_on_a_bar_closes_the_folder() {
    let mut browser = Browser::three_songs("click-close");
    assert!(matches!(browser.click(SLOT_ABOVE, MouseButton::Right), Transition::Stay));
    assert!(browser.app.shared.select_view == SelectView::Root);

    let order = browser.app.shared.config.library.sort;
    browser.click(SLOT_ABOVE, MouseButton::Right);
    assert!(browser.app.shared.select_view == SelectView::Root);
    assert_eq!(browser.app.shared.config.library.sort, order.next());
}

/// The reference takes input from a scene's first frame: the input time of a skin's header only
/// lights a timer on the browser. And the browser never fades out.
#[test]
fn the_input_timer_goes_on_after_the_input_time_and_holds_no_input_back() {
    let mut browser = Browser::three_songs("timer-input");
    browser.app.shared.start_skin_scene_clock();
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.timer(timer_id::STARTINPUT), None, "the input timer was on before the input time");
    browser.tap(TURNTABLE_FORWARD);
    assert_eq!(browser.sel(), 1, "the browser held input back for its input timer, which the reference does not");

    browser.app.shared.age_skin_scene(Duration::from_millis(INPUT_MS + 1));
    browser.frame_after(SHORT_STEP);
    assert!(browser.timer(timer_id::STARTINPUT).is_some(), "the input time has passed");

    browser.app.shared.age_skin_scene(LONG_AFTER);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.timer(timer_id::FADEOUT), None, "the browser faded out on its skin's scene time");
}

/// The bar timer is on from the scene's first frame of input and starts over whenever the bar under
/// the cursor is another one: the cursor moved, the list was built again, the scrollbar was
/// dragged, the long note mode was stepped. The timers the reference never switches stay off.
#[test]
fn the_bar_timer_starts_over_when_the_bar_under_the_cursor_changes() {
    let mut browser = Browser::three_songs("timer-bar");
    let mut last = browser.timer(timer_id::SONGBAR_CHANGE).expect("the bar timer is on from the first frame of input");
    browser.app.shared.age_skin_scene(TIMER_STEP);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.timer(timer_id::SONGBAR_CHANGE), Some(last), "the bar timer started over with nothing changed");

    let mut restarted_by = |browser: &mut Browser, what: &str| {
        browser.app.shared.age_skin_scene(TIMER_STEP);
        browser.frame_after(SHORT_STEP);
        let now = browser.timer(timer_id::SONGBAR_CHANGE).expect("the bar timer stays on");
        assert!(now > last, "{what} did not start the bar timer over");
        last = now;
    };
    browser.key(KeyCode::ArrowDown);
    restarted_by(&mut browser, "an arrow key");
    browser.scroll(1.0);
    restarted_by(&mut browser, "the mouse wheel");
    browser.key(KeyCode::Digit2);
    restarted_by(&mut browser, "a new order");
    browser.key(KeyCode::Digit3);
    restarted_by(&mut browser, "a new long note mode");
    let share = browser.sel() as f32 / browser.app.shared.select_items.len() as f32;
    browser.app.shared.skin_requests().push(Cluster::Select, ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, value: share });
    let before = browser.sel();
    restarted_by(&mut browser, "a drag of the scrollbar that left the cursor where it was");
    assert_eq!(browser.sel(), before);
    browser.go_to(SelectView::Root);
    restarted_by(&mut browser, "another folder");

    for timer in NEVER_SWITCHED {
        assert_eq!(browser.timer(timer), None, "a timer the reference never switches is on");
    }
}

/// The three ranking timers follow the ranking of the chart under the cursor: one while it is being
/// fetched, one once it is in, one when it could not be had (`MusicSelector.render`).
#[test]
fn the_ranking_timers_follow_the_ranking_of_the_chart_under_the_cursor() {
    let mut browser = Browser::three_songs("timer-ranking");
    let on = |browser: &Browser| [timer_id::IR_CONNECT_BEGIN, timer_id::IR_CONNECT_SUCCESS, timer_id::IR_CONNECT_FAIL].map(|id| browser.timer(id).is_some());
    browser.app.shared.ranking_cache.insert("md5-alpha".to_owned(), RankingState::Loading);
    browser.frame_after(SHORT_STEP);
    assert_eq!(on(&browser), [false, false, false], "with no score server there is no ranking to follow");

    add_ir_profile(&mut browser.app, "SERVER", true);
    browser.app.shared.rebuild_server();
    browser.app.shared.ranking_cache.insert("md5-alpha".to_owned(), RankingState::Loading);
    browser.frame_after(SHORT_STEP);
    assert_eq!(on(&browser), [true, false, false]);

    browser.app.shared.ranking_cache.insert("md5-alpha".to_owned(), RankingState::Failed("offline".to_owned()));
    browser.frame_after(SHORT_STEP);
    assert_eq!(on(&browser), [false, false, true]);

    browser.key(KeyCode::ArrowDown);
    browser.frame_after(SHORT_STEP);
    assert_eq!(on(&browser), [false, false, false], "nothing has been asked about the chart the cursor moved to");
}

/// The two arrows that move the cursor are read once a frame beside the turntable, as the
/// reference reads them: a press moves one bar and slides the wheel, a press that lands during the
/// slide goes by, and a press let go before any frame saw it held is still a press
/// (`BarRenderer.input`, `ControlKeys.DOWN` and `UP`).
#[test]
fn an_arrow_slides_the_wheel_as_the_turntable_does() {
    let mut browser = Browser::three_songs("arrow-slide");
    browser.key(KeyCode::ArrowDown);
    assert_eq!(browser.sel(), 0, "an arrow waits for the frame that reads the keys");
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1, "the down arrow is the next bar");
    assert_ne!(browser.state.scroller, BarScroller::default(), "the arrow did not slide the wheel");

    browser.key(KeyCode::ArrowDown);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1, "a press that landed during the slide moved the list");
    browser.frame_after(SLIDE_REST);
    assert_eq!(browser.arrow(KeyCode::ArrowUp), 0, "the up arrow is the bar before");
    assert_eq!(browser.arrow(KeyCode::ArrowUp), 2, "and goes round the top of the list");
}

/// An arrow held down moves one bar, waits, and then repeats, on the two times a held turntable
/// does.
#[test]
fn a_held_arrow_repeats_like_a_held_turntable() {
    let mut browser = Browser::skinned("arrow-hold", ["a", "b", "c", "d", "e", "f"].map(|title| entry(title, "a", "5")).to_vec());
    browser.hold(KeyCode::ArrowDown);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1, "the first move is at once");
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.sel(), 1, "nothing repeats before the first wait is over");
    browser.frame_after(SLIDE_REST);
    assert_eq!(browser.sel(), 2, "the first repeat comes after the long wait");
    browser.frame_after(Duration::from_millis(SCROLL_DURATION_HIGH_MS as u64 + 1));
    assert_eq!(browser.sel(), 3, "and the ones after it after the short one");

    browser.let_go(KeyCode::ArrowDown);
    browser.frame_after(SLIDE_REST);
    assert_eq!(browser.sel(), 3, "the list went on moving with the arrow let go");
}

/// The keys the browser has always had do on a skin's browser what they do on the built-in one.
#[test]
fn the_browsers_own_keys_work_as_they_always_have() {
    let mut browser = Browser::three_songs("own-keys");
    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1, "the down arrow moves a bar");

    assert!(matches!(browser.key(KeyCode::Tab), Transition::Open(Stage::Settings(_))));
    assert!(matches!(browser.key(KeyCode::KeyO), Transition::Open(Stage::Folders(_))));
    assert!(matches!(browser.key(KeyCode::KeyT), Transition::Open(Stage::Tables(_))));
    let order = browser.app.shared.config.library.sort;
    browser.key(KeyCode::F3);
    assert_eq!(browser.app.shared.config.library.sort, order.next());
    browser.key(KeyCode::F2);
    assert!(browser.state.filter.is_open());
    browser.key(KeyCode::F2);
    browser.key(KeyCode::KeyI);
    assert!(browser.state.ranking_open);
    browser.key(KeyCode::KeyI);
    browser.key(KeyCode::KeyF);
    assert!(browser.app.shared.focused_md5().is_some_and(|md5| browser.app.shared.favorites.contains(&md5)), "F did not star the chart");
    browser.key(KeyCode::Slash);
    assert!(browser.app.shared.searching);
    browser.key(KeyCode::Escape);
    assert!(!browser.app.shared.searching && browser.app.shared.select_view == SelectView::AllSongs);

    assert!(starts_a_chart(&browser.key(KeyCode::Enter)));
    assert!(starts_a_chart(&browser.key(KeyCode::ArrowRight)));
    assert!(starts_a_chart(&browser.key(KeyCode::F4)) && browser.app.shared.practice_requested);

    assert!(matches!(browser.key(KeyCode::Escape), Transition::Stay));
    assert!(browser.app.shared.select_view == SelectView::Root, "Esc goes up a folder");
    assert!(matches!(browser.key(KeyCode::Escape), Transition::Stay), "the first Esc at the top only arms the quit");
    assert!(matches!(browser.key(KeyCode::Escape), Transition::Quit));
}

/// The keys the reference's browser has and this one had no use for: the mode filter, the order,
/// the long note mode, the search and the favourite.
#[test]
fn the_reference_keys_step_what_the_browser_already_has() {
    let mut browser = Browser::three_songs("reference-keys");
    let mode = |browser: &Browser| browser.state.filter.filter(&browser.app.shared.config).mode;
    let expected = [Some(Mode::BEAT_7K), Some(Mode::BEAT_14K), Some(Mode::POPN_9K), Some(Mode::BEAT_5K), Some(Mode::BEAT_10K), None];
    for step in expected {
        browser.key(KeyCode::Digit1);
        assert!(mode(&browser) == step, "the mode filter did not go round in the reference's order");
    }

    let order = browser.app.shared.config.library.sort;
    browser.key(KeyCode::Digit2);
    assert_eq!(browser.app.shared.config.library.sort, order.next());

    let long_notes = browser.app.shared.config.judge.ln_mode;
    browser.key(KeyCode::Digit3);
    assert!(browser.app.shared.config.judge.ln_mode != long_notes, "3 did not step the long note mode");

    browser.key(KeyCode::F9);
    assert!(browser.app.shared.focused_md5().is_some_and(|md5| browser.app.shared.favorites.contains(&md5)), "F9 did not star the chart");
    browser.key(KeyCode::Digit0);
    assert!(browser.app.shared.searching, "0 did not open the search");
}

/// While its skin is on its way the browser is black and takes no input; a skin given up on leaves
/// the built-in browser to draw and to take input as it always has.
#[test]
fn a_browser_waiting_for_its_skin_is_black_and_deaf_until_the_skin_is_given_up_on() {
    let songs: Vec<SongEntry> = ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec();
    let mut browser = Browser::over("waiting", songs.clone(), true);
    let _decode = HandDecode::begin(&mut browser.app.shared, SKIN_TYPE_MUSIC_SELECT);
    browser.frame();
    browser.frame();
    let samples = [(CW / 2, CH / 2), (CW / 4, CH / 4), (CW - 1, CH - 1), (0, 0)];
    for (x, y) in samples {
        assert_eq!(browser.pixels.pixel_at(x, y), Color::BLACK, "something was drawn at {x},{y} while the skin was on its way");
    }

    assert!(matches!(browser.key(KeyCode::ArrowDown), Transition::Stay));
    assert!(matches!(browser.key(KeyCode::Enter), Transition::Stay), "a chart was started on a black screen");
    assert!(matches!(browser.key(KeyCode::Tab), Transition::Stay), "the settings were opened from a black screen");
    browser.pad(TURNTABLE_LANE);
    browser.hold(KEY_ONE);
    browser.scroll(1.0);
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay));
    assert_eq!(browser.sel(), 0, "the list moved on a black screen");
    assert_eq!(browser.timer(timer_id::SONGBAR_CHANGE), None, "a timer went on before the scene had a skin");
    browser.let_go(KEY_ONE);

    browser.app.shared.outlast_skin_wait();
    browser.frame_after(SHORT_STEP);
    let mut plain = Browser::over("waiting-plain", songs, false);
    plain.frame();
    assert_eq!(browser.pixels.pixel_checksum(), plain.pixels.pixel_checksum(), "a skin given up on did not leave the built-in browser on screen");
    browser.key(KeyCode::ArrowDown);
    assert_eq!(browser.sel(), 1, "the built-in browser took no key");
}

/// The frame a skin can first be drawn on after the browser has waited for it is the first moment
/// of its scene: every timer off and the clock at zero. Frames after that carry the scene on.
#[test]
fn the_frame_a_waited_for_skin_arrives_on_begins_its_scene() {
    let mut browser = Browser::over("arrives", vec![entry("alpha", "a", "5")], true);
    browser.app.shared.age_skin_scene(LONG_AFTER);
    browser.app.shared.skin_timers.set_on(timer_id::PLAY, 0);
    let decode = HandDecode::begin(&mut browser.app.shared, SKIN_TYPE_MUSIC_SELECT);
    browser.frame();
    browser.frame();
    assert!(browser.timer(timer_id::PLAY).is_some(), "the scene began before its skin was in");

    drop(decode);
    browser.frame();
    assert!(browser.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT));
    assert_eq!(browser.timer(timer_id::PLAY), None, "a timer of the scene before was left on");
    assert!(browser.app.shared.skin_now_us() < JUST_BEGUN_US, "the scene did not begin on the frame its skin arrived");

    browser.app.shared.skin_timers.set_on(timer_id::PLAY, 0);
    browser.app.shared.age_skin_scene(LONG_AFTER);
    browser.frame_after(SHORT_STEP);
    assert!(browser.timer(timer_id::PLAY).is_some(), "a later frame began the scene again");
    assert!(browser.app.shared.skin_now_us() >= JUST_BEGUN_US);
}

/// A browser that comes up with its skin already there -- one a screen opened over it was closed
/// back onto -- carries the scene on rather than beginning it again.
#[test]
fn a_browser_that_comes_up_with_its_skin_there_carries_the_scene_on() {
    let mut browser = Browser::three_songs("carries-on");
    browser.app.shared.skin_timers.set_on(timer_id::PLAY, 0);
    browser.app.shared.age_skin_scene(LONG_AFTER);
    browser.state = SelectState::new();
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    assert!(browser.timer(timer_id::PLAY).is_some(), "the scene was begun again under a skin that never left");
    assert!(browser.app.shared.skin_now_us() >= JUST_BEGUN_US);
}

/// With no skin the browser is the one it always was: no timer of a skin's is touched, the wheel
/// and the reference's own keys do nothing, and a lane key of the keyboard is not a key of the list.
#[test]
fn a_browser_no_skin_draws_is_untouched() {
    let mut browser = Browser::over("plain", ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec(), false);
    browser.frame();
    browser.frame();
    browser.app.shared.age_skin_scene(LONG_AFTER);
    browser.hold(KEY_ONE);
    browser.hold(KEY_TWO);
    browser.scroll(3.0);
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "a lane key of the keyboard started a chart on the built-in browser");
    assert_eq!(browser.sel(), 0, "the wheel moved the built-in list");
    assert!(browser.app.shared.select_view == SelectView::AllSongs);
    for timer in [timer_id::STARTINPUT, timer_id::SONGBAR_CHANGE, timer_id::IR_CONNECT_BEGIN] {
        assert_eq!(browser.timer(timer), None, "a skin's timer went on with no skin");
    }

    for code in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit0, KeyCode::F9] {
        let before = (browser.app.shared.config.library.sort, browser.app.shared.config.judge.ln_mode, browser.app.shared.searching);
        browser.key(code);
        assert!(before == (browser.app.shared.config.library.sort, browser.app.shared.config.judge.ln_mode, browser.app.shared.searching));
        assert!(browser.state.filter.filter(&browser.app.shared.config).mode.is_none());
    }
    browser.key(KeyCode::ArrowDown);
    browser.key(KeyCode::ArrowDown);
    browser.key(KeyCode::ArrowDown);
    assert_eq!(browser.sel(), 2, "the built-in list stops at its end");
}

/// The environment variables that name a skin pack to draw with and a folder to save frames in.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The size a published pack's browser is captured at, the seed its skins are read with, and how
/// long its files are waited for.
const PACK_CANVAS: (u32, u32) = (1920, 1080);
const PACK_SEED: u64 = 1;
const PACK_WAIT: Duration = Duration::from_secs(180);

/// How far into its scene the pack's browser is captured, once nothing is still arriving.
const PACK_SETTLED: Duration = Duration::from_secs(3);

/// Save the canvas as `<name>.png` in the capture folder, when one was asked for.
fn save_frame(pixels: &HeadlessCanvas, name: &str) {
    if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
        let folder = std::path::Path::new(&folder);
        std::fs::create_dir_all(folder).expect("the capture folder can be created");
        let (width, height) = pixels.size();
        image::save_buffer(folder.join(format!("{name}.png")), pixels.rgba(), width, height, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}

/// The browser of a pack somebody else wrote: black while the pack is read, and once it is in, the
/// bar timer is on without the cursor ever having moved, so what the skin shows of the chart under
/// the cursor is there from the start.
///
/// Opt-in: without [`SKIN_PACK_ENV`] this passes without drawing anything.
#[test]
fn the_browser_of_a_skin_pack_named_by_the_environment_waits_in_the_dark_and_opens_on_its_chart() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    let settings = settings_of("skinned-pack");
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    config.library.preview = false;
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.shared.audio_failed = true;
    app.shared.library =
        Library::from_songs(["Aurora", "Borealis", "Cascade", "Drift", "Ember", "Flux", "Glacier"].map(|title| entry(title, "Fixture", "7")).to_vec());
    app.shared.select_view = SelectView::AllSongs;
    app.shared.config.library.sort = SortMode::Title;
    app.shared.rebuild_select_items();
    app.shared.sel = 3;
    let mut browser = Browser { app, state: SelectState::new(), pixels: HeadlessCanvas::new(PACK_CANVAS.0, PACK_CANVAS.1), now: Instant::now() };
    browser.app.shared.begin_skin_scene();

    browser.frame();
    browser.frame();
    assert!(!browser.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT), "the pack was in before a frame could be drawn without it");
    let step = usize::try_from(PACK_CANVAS.0 / 16).unwrap_or(1);
    let dark = (0..PACK_CANVAS.1).step_by(step).all(|y| (0..PACK_CANVAS.0).step_by(step).all(|x| browser.pixels.pixel_at(x, y) == Color::BLACK));
    assert!(dark, "something was drawn while the pack was on its way");
    save_frame(&browser.pixels, "select-waiting");

    let deadline = Instant::now() + PACK_WAIT;
    while !browser.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        browser.frame();
        assert!(Instant::now() < deadline, "the pack's browser never compiled: {:?}", browser.app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT));
    }
    assert!(browser.app.shared.skin_now_us() < JUST_BEGUN_US, "the scene did not begin on the frame the pack arrived");
    browser.frame();
    assert!(browser.timer(timer_id::SONGBAR_CHANGE).is_some(), "the bar timer is off on a browser nobody has moved");
    save_frame(&browser.pixels, "select-opening");

    browser.app.shared.age_skin_scene(PACK_SETTLED);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    save_frame(&browser.pixels, "select-unmoved-3000ms");
    assert_eq!(browser.sel(), 3, "the cursor moved by itself");
}
