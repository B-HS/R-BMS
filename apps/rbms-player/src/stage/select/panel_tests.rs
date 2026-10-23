//! The option panels of a browser a skin draws, on a browser that is really run: which panel the
//! held keys call up and the timers that go with it, what a skin's own conditions make of it, and
//! what each key of the reference's table does to the settings while a panel is up.
//!
//! The skin here is three squares, each drawn while one of the panels is up, over the sheet the
//! tests next door draw their wheel from.

use std::time::Duration;

use rbms_chart::shuffle::NoteOption;
use rbms_config::{Config, FixHiSpeed, LaneOption, ScoreTarget};
use rbms_judge::gauge::{GaugeAutoShift, GaugeKind};
use rbms_skin::model::RBMS_EXTENSIONS_PANEL_KEYS;
use rbms_skin::timer::{TimerId, timer_id};

use super::skinned_tests::{
    Browser, CELL, DOCUMENT_SIZE, KEY_FIVE, KEY_FOUR, KEY_ONE, KEY_SEVEN, KEY_SIX, KEY_THREE, KEY_TWO, SELECT, SHORT_STEP, START, TURNTABLE_BACKWARD,
    TURNTABLE_FORWARD,
};
use super::tests::{entry, press};
use super::*;
use crate::stage::StageId;

/// The key that calls up the third panel by itself.
const DETAIL_KEY: KeyCode = KeyCode::Digit5;

/// Where the fixture draws the square of each panel, as the left edge of each in the skin's own
/// coordinates, and the box they share.
const SQUARE_LEFTS: [u32; 3] = [20, 120, 220];
const SQUARE_BOTTOM: u32 = 20;
const SQUARE_SIDE: u32 = 40;

/// The red of the sheet's first cell, which the squares are drawn from, and the least of it that
/// counts as the square being there.
const SQUARE_RED_FLOOR: u8 = 20;

/// The times of the fixture skin's header, in milliseconds: it takes input at once and fades for no
/// time, and its length is one the browser never reads.
const NO_WAIT_MS: u64 = 0;
const SCENE_MS: u64 = 3000;

/// The options a skin asks about the three panels.
const PANEL_OPTIONS: [u32; 3] = [21, 22, 23];

/// The pair of timers of each panel, the one of its arrival first.
const PANEL_TIMERS: [(TimerId, TimerId); 3] =
    [(timer_id::PANEL1_ON, timer_id::PANEL1_OFF), (timer_id::PANEL2_ON, timer_id::PANEL2_OFF), (timer_id::PANEL3_ON, timer_id::PANEL3_OFF)];

/// How far a scene is aged between two looks at a timer, so one that started over reads later.
const TIMER_STEP: Duration = Duration::from_millis(100);

/// Longer than a held target key waits before it first repeats.
const PAST_THE_FIRST_REPEAT: Duration = Duration::from_millis(400);

/// The fixture document of the three squares, saying it knows this player's own ids when
/// `extensions` has the value to say so with.
fn panel_document_naming(extensions: Option<i32>) -> String {
    let squares = SQUARE_LEFTS
        .iter()
        .zip(PANEL_OPTIONS)
        .map(|(x, option)| format!(r#"{{"id":"lamp","op":[{option}],"dst":[{{"x":{x},"y":{SQUARE_BOTTOM},"w":{SQUARE_SIDE},"h":{SQUARE_SIDE}}}]}}"#))
        .collect::<Vec<_>>();
    let extensions = extensions.map_or_else(String::new, |value| format!(r#""rbms_extensions": {value},"#));
    format!(
        r#"{{
            "type": {SKIN_TYPE_MUSIC_SELECT}, "name": "panels", "w": {}, "h": {}, {extensions}
            "input": {NO_WAIT_MS}, "scene": {SCENE_MS}, "fadeout": {NO_WAIT_MS},
            "source": [{{ "id": "sheet", "path": "sheet.png" }}],
            "image": [{{ "id": "lamp", "src": "sheet", "x": 0, "y": 0, "w": {CELL}, "h": {CELL} }}],
            "destination": [{}]
        }}"#,
        DOCUMENT_SIZE.0,
        DOCUMENT_SIZE.1,
        squares.join(","),
    )
}

/// A browser over three charts that the fixture of the three squares draws, which says it knows
/// this player's own ids.
fn browser(tag: &str) -> Browser {
    browser_drawn_by(tag, panel_document_naming(Some(RBMS_EXTENSIONS_PANEL_KEYS)))
}

/// A browser over three charts that the fixture of the three squares draws, which says nothing of
/// this player's own ids, as a document written for the reference does.
fn reference_browser(tag: &str) -> Browser {
    browser_drawn_by(tag, panel_document_naming(None))
}

fn browser_drawn_by(tag: &str, document: String) -> Browser {
    Browser::skinned_by(tag, ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec(), document)
}

/// Which of the three squares the last frame drew.
fn squares(browser: &Browser) -> [bool; 3] {
    SQUARE_LEFTS.map(|left| {
        let across = (left + SQUARE_SIDE / 2) * CW / DOCUMENT_SIZE.0;
        let down = (DOCUMENT_SIZE.1 - SQUARE_BOTTOM - SQUARE_SIDE / 2) * CH / DOCUMENT_SIZE.1;
        browser.pixels.pixel_at(across, down).r > SQUARE_RED_FLOOR
    })
}

/// Whether the timers of the panel numbered `panel` stand as they do with it up: its arrival on and
/// its departure off.
fn stands_up(browser: &Browser, panel: usize) -> bool {
    let (on, off) = PANEL_TIMERS[panel - 1];
    browser.timer(on).is_some() && browser.timer(off).is_none()
}

/// Whether they stand as they do once it has gone: its departure on and its arrival off.
fn has_gone(browser: &Browser, panel: usize) -> bool {
    let (on, off) = PANEL_TIMERS[panel - 1];
    browser.timer(on).is_none() && browser.timer(off).is_some()
}

/// A browser with `keys` held for one frame.
fn holding(tag: &str, keys: &[KeyCode]) -> Browser {
    holding_in(browser(tag), keys)
}

/// `browser` with `keys` held for one frame.
fn holding_in(mut browser: Browser, keys: &[KeyCode]) -> Browser {
    for key in keys {
        browser.hold(*key);
    }
    browser.frame_after(SHORT_STEP);
    browser
}

/// START alone is the first panel, SELECT alone the second, both the third, and so is the detail
/// key with neither. A panel is up for as long as its keys are held: it comes with its arrival
/// timer and the option a skin asks about it, and goes with its departure timer.
#[test]
fn the_held_keys_put_a_panel_up_and_letting_go_takes_it_down() {
    let cases: [(&[KeyCode], usize, &str); 4] = [(&[START], 1, "start"), (&[SELECT], 2, "select"), (&[START, SELECT], 3, "both"), (&[DETAIL_KEY], 3, "detail")];
    for (keys, panel, name) in cases {
        let mut browser = browser(&format!("panel-up-{name}"));
        assert_eq!(squares(&browser), [false; 3], "{name}: a panel was up before any key");
        for timers in PANEL_TIMERS {
            assert_eq!((browser.timer(timers.0), browser.timer(timers.1)), (None, None), "{name}: a panel timer was on before any key");
        }

        for key in keys {
            browser.hold(*key);
        }
        browser.frame_after(SHORT_STEP);
        assert_eq!(usize::from(browser.state.panel.number()), panel, "{name}");
        assert!(stands_up(&browser, panel), "{name}: the timers of panel {panel}");
        let mut lit = [false; 3];
        lit[panel - 1] = true;
        assert_eq!(squares(&browser), lit, "{name}: what the skin drew");

        for key in keys {
            browser.let_go(*key);
        }
        browser.frame_after(SHORT_STEP);
        assert_eq!(browser.state.panel.number(), 0, "{name}: the panel outlived its keys");
        assert!(has_gone(&browser, panel), "{name}: the timers of panel {panel} once it had gone");
        assert_eq!(squares(&browser), [false; 3], "{name}: the skin still drew a panel");
    }
}

/// Going from one panel straight to another starts the departure of the first and the arrival of
/// the second on the one frame, and leaves the third alone.
#[test]
fn going_from_one_panel_to_another_ends_the_first_and_begins_the_second() {
    let mut browser = holding("panel-change", &[START]);
    browser.hold(SELECT);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 3);
    assert!(has_gone(&browser, 1) && stands_up(&browser, 3));
    assert_eq!((browser.timer(timer_id::PANEL2_ON), browser.timer(timer_id::PANEL2_OFF)), (None, None));

    browser.let_go(START);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 2);
    assert!(has_gone(&browser, 3) && stands_up(&browser, 2) && has_gone(&browser, 1));

    browser.hold(START);
    browser.let_go(SELECT);
    browser.frame_after(SHORT_STEP);
    assert!(has_gone(&browser, 2), "the detail panel is not passed through on the way");
    browser.frame_after(SHORT_STEP);
    assert!(stands_up(&browser, 1), "a panel that comes back switches its departure off again");
}

/// The timers are switched when the panel changes and at no other time: a panel that stays up
/// keeps the moment it came up at, and one that stays down the moment it went.
#[test]
fn a_panel_that_stays_as_it_is_keeps_its_timers_where_they_were() {
    let mut browser = holding("panel-steady", &[START]);
    let came = browser.timer(timer_id::PANEL1_ON);
    browser.frame_after(TIMER_STEP);
    browser.frame_after(TIMER_STEP);
    assert_eq!(browser.timer(timer_id::PANEL1_ON), came, "the arrival started over while the panel stayed up");

    browser.let_go(START);
    browser.frame_after(TIMER_STEP);
    let went = browser.timer(timer_id::PANEL1_OFF);
    assert!(went.is_some());
    browser.frame_after(TIMER_STEP);
    assert_eq!(browser.timer(timer_id::PANEL1_OFF), went, "the departure started over while the panel stayed down");
}

/// A panel held when the browser is arrived back at is up on its first frame, and the keys held
/// then are not presses on it.
#[test]
fn a_panel_held_on_arrival_is_up_and_takes_no_press_from_what_was_held() {
    let mut browser = browser("panel-arrival");
    browser.hold(START);
    browser.hold(KEY_ONE);
    browser.come_back();
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 1);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.app.shared.config.play.random, NoteOption::Off, "a key held on arrival stepped the random option");
}

/// While the keys are something else's -- here the browser's own search box -- no panel is up,
/// whatever is held.
#[test]
fn a_panel_goes_down_while_the_keys_are_something_elses() {
    let mut browser = holding("panel-elsewhere", &[SELECT]);
    assert_eq!(browser.state.panel.number(), 2);
    browser.app.shared.searching = true;
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 0);
    assert!(has_gone(&browser, 2));

    browser.app.shared.searching = false;
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 2, "SELECT is still held");
}

/// The option overlay is the application's own panel and has nothing to do with a skin's: opening
/// and closing it starts no panel timer and puts no panel up.
#[test]
fn the_option_overlay_starts_no_panel_of_the_skin() {
    let mut browser = browser("panel-overlay");
    let now = browser.now;
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now, dt: 0.0 };
    assert!(crate::app_options::options_key(&mut ctx, StageId::Select, false, &press(KeyCode::F1)));
    browser.frame_after(SHORT_STEP);
    assert_eq!(squares(&browser), [false; 3], "the overlay opened a panel of the skin");

    crate::app_options::close(&mut browser.app.shared);
    browser.frame_after(SHORT_STEP);
    for (on, off) in PANEL_TIMERS {
        assert_eq!((browser.timer(on), browser.timer(off)), (None, None));
    }
}

/// One tap of `key` with `panel_keys` held, and the settings it left.
fn tapped(tag: &str, panel_keys: &[KeyCode], key: KeyCode) -> Config {
    tapped_in(browser(tag), tag, panel_keys, key)
}

/// One tap of `key` on `browser` with `panel_keys` held, and the settings it left.
fn tapped_in(browser: Browser, tag: &str, panel_keys: &[KeyCode], key: KeyCode) -> Config {
    let mut browser = holding_in(browser, panel_keys);
    assert!(matches!(browser.tap(key), Transition::Stay), "{tag}: a key of a panel left the browser");
    assert_eq!(browser.sel(), 0, "{tag}: a key of a panel moved the list");
    browser.app.shared.config.clone()
}

/// Whether `key`, held with `panel_keys` for longer than any repeat waits, leaves every setting as
/// it was.
fn changes_nothing(tag: &str, panel_keys: &[KeyCode], key: KeyCode) -> bool {
    changes_nothing_in(browser(tag), panel_keys, key)
}

/// Whether `key`, held on `browser` with `panel_keys` for longer than any repeat waits, leaves every
/// setting as it was.
fn changes_nothing_in(browser: Browser, panel_keys: &[KeyCode], key: KeyCode) -> bool {
    let mut browser = holding_in(browser, panel_keys);
    let before = browser.app.shared.config.clone();
    browser.hold(key);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(PAST_THE_FIRST_REPEAT);
    browser.let_go(key);
    browser.frame_after(SHORT_STEP);
    browser.app.shared.config == before
}

/// The first panel's rows of the key table: keys one and two step the random option forward and
/// back, three and four the gauge forward and back, five and six the fixed tempo forward and back,
/// and seven the random option forward. The double option is on no key: no run applies it.
#[test]
fn the_keys_of_the_first_panel_step_the_play_options() {
    let random = |key: KeyCode, name: &str| tapped(&format!("panel1-{name}"), &[START], key).play.random;
    assert_eq!(random(KEY_ONE, "one"), NoteOption::Mirror, "OPTION1_DOWN is the next option");
    assert_eq!(random(KEY_TWO, "two"), NoteOption::AllScratch, "OPTION1_UP is the one before, round the end");
    assert_eq!(random(KEY_SEVEN, "seven"), NoteOption::Mirror, "OPTION2_DOWN");

    let defaults = Config::default();
    assert_eq!(defaults.play.gauge, GaugeKind::Normal);
    assert_eq!(tapped("panel1-three", &[START], KEY_THREE).play.gauge, GaugeKind::Hard, "GAUGE_DOWN is the next gauge");
    let four = tapped("panel1-four", &[START], KEY_FOUR);
    assert_eq!(four.play.gauge, GaugeKind::Easy, "GAUGE_UP is the gauge before");
    assert_eq!(four.play.lane_option, LaneOption::Off, "key four no longer steps the double option");
    assert_eq!(defaults.play.fix_hispeed, FixHiSpeed::MainBpm);
    assert_eq!(tapped("panel1-five", &[START], KEY_FIVE).play.fix_hispeed, FixHiSpeed::MinBpm, "HSFIX_DOWN");
    let six = tapped("panel1-six", &[START], KEY_SIX);
    assert_eq!(six.play.fix_hispeed, FixHiSpeed::MaxBpm, "HSFIX_UP is the fixed tempo before");
    assert_eq!(six.play.random, NoteOption::Off, "key six no longer steps the random option");
}

/// The second panel's rows: key two is the constant speed and key four the legacy notes, key one the
/// custom judge window, which is no switch here and changes nothing. Key three switches SCRATCH
/// AUTO, five KEEP GREEN NUMBER, six the lane cover, seven the lift, and the turntable the HIDDEN+
/// cover, each the other way from how it ships.
#[test]
fn the_keys_of_the_second_panel_flip_the_assists_there_are() {
    assert!(tapped("panel2-two", &[SELECT], KEY_TWO).play.constant_speed, "CONSTANT");
    assert!(tapped("panel2-four", &[SELECT], KEY_FOUR).play.legacy_note, "LEGACYNOTE");
    assert!(changes_nothing("panel2-one", &[SELECT], KEY_ONE), "key one of the second panel changed a setting");

    let shipped = Config::default().play;
    assert!(!shipped.scratch_auto && !shipped.keep_green_number && shipped.enable_cover && !shipped.enable_lift && !shipped.enable_hidden);
    assert!(tapped("panel2-three", &[SELECT], KEY_THREE).play.scratch_auto, "SCRATCH AUTO");
    assert!(tapped("panel2-five", &[SELECT], KEY_FIVE).play.keep_green_number, "KEEP GREEN NUMBER");
    assert!(!tapped("panel2-six", &[SELECT], KEY_SIX).play.enable_cover, "LANE COVER ON");
    assert!(tapped("panel2-seven", &[SELECT], KEY_SEVEN).play.enable_lift, "LIFT ON");
    for (key, name) in [(TURNTABLE_FORWARD, "forward"), (TURNTABLE_BACKWARD, "backward")] {
        assert!(tapped(&format!("panel2-turntable-{name}"), &[SELECT], key).play.enable_hidden, "HIDDEN+ ON with the turntable {name}");
    }
}

/// The turntable of the third panel steps the judge algorithm round the three the reference lists:
/// forwards to the one before, backwards to the one after, as the first panel's steps the target.
#[test]
fn the_turntable_of_the_third_panel_steps_the_judge_algorithm() {
    use rbms_judge::JudgeAlgorithm;
    let both: &[KeyCode] = &[START, SELECT];
    assert_eq!(Config::default().judge.judge_algorithm, JudgeAlgorithm::Combo);
    assert_eq!(tapped("panel3-turntable-forward", both, TURNTABLE_FORWARD).judge.judge_algorithm, JudgeAlgorithm::Lowest, "the one before, round the end");
    assert_eq!(tapped("panel3-turntable-backward", both, TURNTABLE_BACKWARD).judge.judge_algorithm, JudgeAlgorithm::Duration, "the one after");
    assert_eq!(tapped("panel3-detail-turntable", &[DETAIL_KEY], TURNTABLE_BACKWARD).judge.judge_algorithm, JudgeAlgorithm::Duration);

    let mut browser = holding("panel3-turntable-held", both);
    browser.hold(TURNTABLE_BACKWARD);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(PAST_THE_FIRST_REPEAT);
    assert_eq!(browser.app.shared.config.judge.judge_algorithm, JudgeAlgorithm::Duration, "a held turntable steps it once");
    assert_eq!(browser.sel(), 0, "the turntable moved the list under the panel");
}

/// A turntable direction that was already down when a panel came up, or when the panel gave way to
/// another, is the list's scrolling carried on and not a press on the panel: it neither switches
/// HIDDEN+ nor steps the judge algorithm. A turn made once the panel is up still does.
#[test]
fn a_turntable_direction_held_before_the_panel_came_up_is_not_a_press_on_it() {
    use rbms_judge::JudgeAlgorithm;
    let hidden = |browser: &Browser| browser.app.shared.config.play.enable_hidden;
    let algorithm = |browser: &Browser| browser.app.shared.config.judge.judge_algorithm;

    let mut browser = browser("panel-turntable-held-before");
    browser.hold(TURNTABLE_FORWARD);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);

    browser.hold(SELECT);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 2);
    assert!(!hidden(&browser), "a turntable held before the second panel came up switched HIDDEN+");

    browser.hold(START);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 3);
    assert_eq!(algorithm(&browser), JudgeAlgorithm::Combo, "a turntable held before the third panel came up stepped the judge algorithm");

    browser.let_go(START);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.panel.number(), 2);
    assert!(!hidden(&browser), "a turntable held through the change back to the second panel switched HIDDEN+");

    browser.let_go(TURNTABLE_FORWARD);
    browser.frame_after(SHORT_STEP);
    browser.hold(TURNTABLE_FORWARD);
    browser.frame_after(SHORT_STEP);
    assert!(hidden(&browser), "a turn made on the panel was not a press");
}

/// With the first panel up the key the option overlay is held open by, which ships on the turntable,
/// is the panel's: the overlay stays shut and the turntable steps the target. With no panel up the
/// same key opens the overlay as it always has.
#[test]
fn the_turntable_steps_the_target_rather_than_open_the_option_overlay_while_a_panel_is_up() {
    assert_eq!(crate::app_options::HOLD_KEY, TURNTABLE_FORWARD, "the overlay's hold key no longer ships on the turntable");
    let first = ScoreTarget::ALL[0];
    let mut browser = holding("panel1-turntable-not-overlay", &[START]);
    browser.app.shared.config.judge.target = first;
    browser.hold(TURNTABLE_FORWARD);
    let now = browser.now;
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now, dt: 0.0 };
    assert!(!crate::app_options::options_key(&mut ctx, StageId::Select, false, &press(TURNTABLE_FORWARD)), "the overlay took the turntable");
    assert!(!browser.app.shared.options.is_open(), "the overlay opened over the panel");
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.app.shared.config.judge.target, ScoreTarget::ALL[ScoreTarget::ALL.len() - 1], "the turntable did not step the target");

    for held in [SELECT, DETAIL_KEY] {
        let mut browser = holding("panel-turntable-not-overlay", &[held]);
        browser.hold(TURNTABLE_FORWARD);
        let now = browser.now;
        let mut ctx = FrameCtx { shared: &mut browser.app.shared, now, dt: 0.0 };
        assert!(!crate::app_options::options_key(&mut ctx, StageId::Select, false, &press(TURNTABLE_FORWARD)), "{held:?}");
    }

    let mut unpaneled = self::browser("panel-none-overlay");
    unpaneled.hold(TURNTABLE_FORWARD);
    let now = unpaneled.now;
    let mut ctx = FrameCtx { shared: &mut unpaneled.app.shared, now, dt: 0.0 };
    assert!(crate::app_options::options_key(&mut ctx, StageId::Select, false, &press(TURNTABLE_FORWARD)), "with no panel the key opens the overlay");
    assert!(unpaneled.app.shared.options.is_open());
}

/// The third panel's rows: key one is the background animation, two the gauge auto shift, three the
/// automatic judge timing, and five and seven the judge timing down and up a millisecond. Keys four
/// and six are the note display time, which there is none of while no green number is held.
#[test]
fn the_keys_of_the_third_panel_step_the_detail_options() {
    let both: &[KeyCode] = &[START, SELECT];
    assert!(!tapped("panel3-one", both, KEY_ONE).display.bga, "BGA_DOWN");
    assert_eq!(tapped("panel3-two", both, KEY_TWO).judge.gauge_auto_shift, GaugeAutoShift::Continue, "GAUGEAUTOSHIFT_DOWN");
    assert!(tapped("panel3-three", both, KEY_THREE).judge.auto_offset, "NOTESDISPLAYTIMING_AUTOADJUST");
    assert_eq!(tapped("panel3-five", both, KEY_FIVE).judge.offset_ms, -1, "NOTESDISPLAYTIMING_DOWN");
    assert_eq!(tapped("panel3-seven", both, KEY_SEVEN).judge.offset_ms, 1, "NOTESDISPLAYTIMING_UP");
    for (key, name) in [(KEY_FOUR, "four"), (KEY_SIX, "six")] {
        assert!(changes_nothing(&format!("panel3-{name}"), both, key), "key {name} of the third panel changed a setting");
    }
    assert_eq!(tapped("panel3-detail-key", &[DETAIL_KEY], KEY_FIVE).judge.offset_ms, -1, "the detail key's panel is the same panel");
}

/// To a skin that names none of this player's ids the keys of the panels are the reference's. On the
/// first panel key four steps the double option and key six the second side's random option back,
/// where they are the gauge and the fixed tempo before for a skin that knows the ids. On the second
/// panel keys three, five, six and seven are the judge area, the processed note mark, the BPM guide
/// and no mines, which this player has no setting for and which the panel takes and does nothing
/// with, and the turntable has no role on the second panel or on the third.
#[test]
fn on_a_skin_that_names_no_extensions_the_panels_keys_are_the_references() {
    let reference = |name: &str| reference_browser(&format!("panel-reference-{name}"));
    let tapped_on_reference = |name: &str, panel_keys: &[KeyCode], key: KeyCode| tapped_in(reference(name), name, panel_keys, key);

    let shipped = Config::default();
    let four = tapped_on_reference("start-four", &[START], KEY_FOUR);
    assert_eq!(four.play.lane_option, LaneOption::Flip, "OPTIONDP_DOWN is the next double option");
    assert_eq!(four.play.gauge, shipped.play.gauge, "key four stepped the gauge");
    let six = tapped_on_reference("start-six", &[START], KEY_SIX);
    assert_eq!(six.play.random, NoteOption::AllScratch, "OPTION2_UP is the random option before, round the end");
    assert_eq!(six.play.fix_hispeed, shipped.play.fix_hispeed, "key six stepped the fixed tempo");
    assert_eq!(tapped_on_reference("start-seven", &[START], KEY_SEVEN).play.random, NoteOption::Mirror, "OPTION2_DOWN");
    assert_eq!(tapped_on_reference("start-three", &[START], KEY_THREE).play.gauge, GaugeKind::Hard, "GAUGE_DOWN is where it always was");

    assert!(tapped_on_reference("select-two", &[SELECT], KEY_TWO).play.constant_speed, "CONSTANT is where it always was");
    assert!(tapped_on_reference("select-four", &[SELECT], KEY_FOUR).play.legacy_note, "LEGACYNOTE is where it always was");
    for (key, name) in [(KEY_ONE, "one"), (KEY_THREE, "three"), (KEY_FIVE, "five"), (KEY_SIX, "six"), (KEY_SEVEN, "seven")] {
        assert!(changes_nothing_in(reference(&format!("select-{name}")), &[SELECT], key), "key {name} of the second panel changed a setting");
    }
    for (key, name) in [(TURNTABLE_FORWARD, "forward"), (TURNTABLE_BACKWARD, "backward")] {
        assert!(
            changes_nothing_in(reference(&format!("select-turntable-{name}")), &[SELECT], key),
            "the turntable {name} on the second panel changed a setting"
        );
        assert!(
            changes_nothing_in(reference(&format!("both-turntable-{name}")), &[START, SELECT], key),
            "the turntable {name} on the third panel changed a setting"
        );
    }

    let both: &[KeyCode] = &[START, SELECT];
    assert_eq!(tapped_on_reference("both-five", both, KEY_FIVE).judge.offset_ms, -1, "NOTESDISPLAYTIMING_DOWN is where it always was");
}

/// With a green number held, keys four and six of the third panel move the travel time it is held
/// at, down and up a millisecond (`DURATION_DOWN`, `DURATION_UP`), and a key that stays down
/// repeats, which no other key of a panel does.
#[test]
fn the_third_panel_moves_the_held_travel_time_while_its_keys_are_held() {
    let shipped = rbms_config::DEFAULT_DURATION_MS;
    let held_with = |tag: &str| {
        let mut browser = holding(tag, &[START, SELECT]);
        browser.app.shared.config.play.keep_green_number = true;
        browser
    };
    let duration = |browser: &Browser| browser.app.shared.config.play.duration_ms;

    let mut browser = held_with("panel3-duration-up");
    browser.hold(KEY_SIX);
    browser.frame_after(SHORT_STEP);
    assert_eq!(duration(&browser), shipped + 1, "DURATION_UP moves it at once");
    browser.frame_after(SHORT_STEP);
    assert_eq!(duration(&browser), shipped + 1, "a held key waits before it repeats");
    browser.frame_after(PAST_THE_FIRST_REPEAT);
    assert_eq!(duration(&browser), shipped + 2, "and then repeats");
    browser.let_go(KEY_SIX);
    browser.frame_after(SHORT_STEP);
    assert_eq!(duration(&browser), shipped + 2);

    let mut browser = held_with("panel3-duration-down");
    browser.hold(KEY_FOUR);
    browser.frame_after(SHORT_STEP);
    assert_eq!(duration(&browser), shipped - 1, "DURATION_DOWN");
    assert_eq!(browser.sel(), 0, "a key of the panel moved the list");
    assert!((browser.app.shared.config.play.hispeed - Config::default().play.hispeed).abs() < f64::EPSILON, "the speed is worked out when a chart starts");
}

/// With the cursor off a chart the panel shows no travel time, so the keys that move it leave it
/// where it is -- a number nobody can see is not moved -- and with the cursor on a chart they move it
/// again.
#[test]
fn the_third_panel_leaves_the_travel_time_alone_while_the_cursor_is_off_a_chart() {
    let shipped = rbms_config::DEFAULT_DURATION_MS;
    let mut browser = holding("panel3-duration-folder", &[START, SELECT]);
    browser.app.shared.config.play.keep_green_number = true;
    browser.app.shared.select_view = SelectView::Root;
    browser.app.shared.sel = 0;
    browser.app.shared.rebuild_select_items();
    browser.frame_after(SHORT_STEP);
    assert!(matches!(browser.app.shared.select_items.first(), Some(SelectItem::Folder { .. })), "the cursor is not on a folder");

    for key in [KEY_SIX, KEY_FOUR] {
        browser.hold(key);
        browser.frame_after(SHORT_STEP);
        browser.frame_after(PAST_THE_FIRST_REPEAT);
        browser.let_go(key);
        browser.frame_after(SHORT_STEP);
        assert_eq!(browser.app.shared.config.play.duration_ms, shipped, "{key:?} moved a travel time the panel does not show");
    }

    browser.app.shared.select_view = SelectView::AllSongs;
    browser.app.shared.rebuild_select_items();
    browser.frame_after(SHORT_STEP);
    browser.hold(KEY_SIX);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.app.shared.config.play.duration_ms, shipped + 1, "on a chart the key moves it again");
}

/// The browser lends the skin which keys of its table the bound mode has no lane on: none in the
/// seven-key mode the fixture starts in, and the two of keys six and seven once it is a five-key one.
#[test]
fn the_browser_lends_the_keys_the_bound_mode_has_no_lane_on() {
    use crate::keyconfig::KEY_INDEX_COUNT;
    use crate::skin_host::select::BrowserLent;
    let mut browser = browser("panel-keys-out-of-reach");
    let lent = BrowserLent::default();
    assert_eq!(browser.app.shared.select_shown(None, &lent).unreachable_keys, [false; KEY_INDEX_COUNT]);

    browser.app.shared.mode = Mode::BEAT_5K;
    let out_of_reach = browser.app.shared.select_shown(None, &lent).unreachable_keys;
    assert_eq!(out_of_reach.iter().enumerate().filter(|(_, out)| **out).map(|(index, _)| index).collect::<Vec<_>>(), [5, 6]);
}

/// A key held on a panel is one press: the option moves once however long it is held.
#[test]
fn a_key_held_on_a_panel_steps_its_option_once() {
    let mut browser = holding("panel-held-key", &[START]);
    browser.hold(KEY_THREE);
    browser.frame_after(SHORT_STEP);
    browser.frame_after(PAST_THE_FIRST_REPEAT);
    browser.frame_after(PAST_THE_FIRST_REPEAT);
    assert_eq!(browser.app.shared.config.play.gauge, GaugeKind::Hard);
}

/// With the first panel up the turntable, the mouse wheel and the keyboard's own arrows scroll the
/// target and leave the list alone. Towards the top of the list is the target before.
#[test]
fn the_first_panel_scrolls_the_target_with_the_turntable_the_wheel_and_the_arrows() {
    let first = ScoreTarget::ALL[0];
    let last = ScoreTarget::ALL[ScoreTarget::ALL.len() - 1];
    let second = ScoreTarget::ALL[1];
    let target_of = |browser: &Browser| browser.app.shared.config.judge.target;

    let mut browser = holding("panel1-target-turntable", &[START]);
    browser.app.shared.config.judge.target = first;
    browser.hold(TURNTABLE_FORWARD);
    browser.frame_after(SHORT_STEP);
    assert_eq!(target_of(&browser), last, "TARGET_UP is the target before, round the end");
    browser.frame_after(SHORT_STEP);
    assert_eq!(target_of(&browser), last, "a held key waits before it repeats");
    browser.frame_after(PAST_THE_FIRST_REPEAT);
    assert_eq!(target_of(&browser), ScoreTarget::ALL[ScoreTarget::ALL.len() - 2], "and then repeats");
    assert_eq!(browser.sel(), 0, "the turntable moved the list under the panel");

    let mut browser = holding("panel1-target-wheel", &[START]);
    browser.app.shared.config.judge.target = first;
    browser.scroll(-2.0);
    assert_eq!(target_of(&browser), ScoreTarget::ALL[2], "two notches towards the bottom of the list");
    browser.scroll(1.0);
    assert_eq!(target_of(&browser), second);
    assert_eq!(browser.sel(), 0, "the wheel moved the list under the panel");

    let mut browser = holding("panel1-target-arrows", &[START]);
    browser.app.shared.config.judge.target = first;
    browser.hold(KeyCode::ArrowUp);
    assert!(matches!(browser.key(KeyCode::ArrowUp), Transition::Stay));
    browser.frame_after(SHORT_STEP);
    assert_eq!(target_of(&browser), second, "the up arrow is TARGET_DOWN, the target after");
    assert_eq!(browser.sel(), 0, "an arrow moved the list under the panel");
}

/// With a panel up the keys that move through the list and start what is under the cursor do
/// nothing to the list, and with none up they do what they always did.
#[test]
fn the_list_keys_of_the_keyboard_are_the_panels_while_one_is_up() {
    let mut browser = holding("panel-list-keys", &[SELECT]);
    for code in [KeyCode::ArrowDown, KeyCode::ArrowUp, KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Enter, KeyCode::NumpadEnter] {
        assert!(matches!(browser.key(code), Transition::Stay), "{code:?} left the browser with a panel up");
        assert_eq!(browser.sel(), 0, "{code:?} moved the list with a panel up");
        assert!(browser.app.shared.select_view == SelectView::AllSongs, "{code:?} left the folder with a panel up");
    }

    browser.let_go(SELECT);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1, "the down arrow is the list's again once the panel is down");
}

/// A key bound to a lane is the panel's alone while a panel is called up: it steps the panel's
/// option and does not also do what the same key does on the list. The default layout puts key six
/// on `F`, which stars the chart under the cursor, and with the panel covering the list that would
/// happen unseen. The panel counts as called up from the moment its keys are held, before the frame
/// that draws it. With no panel the key is the list's again.
#[test]
fn a_lane_key_does_not_reach_the_lists_shortcuts_while_a_panel_is_called_up() {
    let starred = |browser: &Browser| browser.app.shared.favorites.contains("md5-alpha");
    for held in [START, SELECT, DETAIL_KEY] {
        let mut browser = holding("panel-lane-keys", &[held]);
        assert!(matches!(browser.key(KEY_SIX), Transition::Stay));
        assert!(!starred(&browser), "key six starred the chart under the panel {held:?} calls up");
    }

    let mut browser = browser("panel-lane-keys-held");
    browser.hold(START);
    browser.key(KEY_SIX);
    assert!(!starred(&browser), "the panel's keys were held and the frame that draws it had not come yet");

    browser.let_go(START);
    browser.frame_after(SHORT_STEP);
    browser.key(KEY_SIX);
    assert!(starred(&browser), "with no panel the key is the list's own again");
}

/// What a panel's keys changed is written out when the panel goes down, and not at every step of a
/// key held on it.
#[test]
fn the_settings_a_panel_changed_are_written_out_when_it_goes_down() {
    let mut browser = holding("panel-save", &[START]);
    browser.hold(KEY_THREE);
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.app.shared.config.play.gauge, GaugeKind::Hard);
    let written = |browser: &Browser| rbms_config::load(&browser.app.shared.settings_path).expect("the settings are readable").config.play.gauge;
    assert_eq!(written(&browser), GaugeKind::Normal, "the settings were written with the panel still up");

    browser.let_go(KEY_THREE);
    browser.let_go(START);
    browser.frame_after(SHORT_STEP);
    assert_eq!(written(&browser), GaugeKind::Hard, "the gauge the panel chose was not written out");
}

/// A browser no skin draws has no panels: START and SELECT put none up and switch no timer.
#[test]
fn a_browser_no_skin_draws_has_no_panels() {
    let mut app = super::tests::app();
    let mut state = SelectState::new();
    app.shared.note_key(&press(START));
    app.shared.note_key(&press(KEY_ONE));
    let now = Instant::now();
    state.update(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert_eq!(state.panel.number(), 0);
    for (on, off) in PANEL_TIMERS {
        assert!(!app.shared.skin_timers.is_on(on) && !app.shared.skin_timers.is_on(off));
    }
    assert_eq!(app.shared.config.play.random, NoteOption::Off);
}
