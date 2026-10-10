//! The application's own panels over a browser a skin draws: that each is drawn and each is the first
//! to be offered the keys and the mouse, that the key guide lists keys that do what it says and
//! reads the ones that can be rebound, and that a browser no skin draws is not touched by any of it.
//!
//! The skin is the one the browser's other tests write: a wheel of three bars and nothing else, so
//! what is drawn over it is told from what the skin drew by where the wheel is not.

use rbms_render::{Color, Rect, Renderer, theme};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use super::SelectState;
use super::guide::{BROWSER_SHORTCUTS, GUIDE_KEY, browser_rows, key_rows, panel_rows, shortcut_label};
use super::overlay::{HINT_H, HINT_Y, SEARCH_H, STRIP_BOTTOM, STRIP_X, TAB_MARK_H};
use super::skinned_tests::{Browser, SLOT_ABOVE, SLOT_BELOW, slot_middle, starts_a_chart, wheel_document};
use super::tests::{app, entry, press, record};
use crate::app_options::{HOLD_KEY, TOGGLE_KEY, options_key};
use crate::ir_ranking_view::panel_action;
use crate::keyconfig::{ControlAction, KeyConfig, default_keys_for_mode};
use crate::pointer::PointerInput;
use crate::stage::select::filter::filter_panel_rect;
use crate::stage::select::overlay::ranking_panel_rect;
use crate::stage::{Canvas, FrameCtx, HeadlessCanvas, Stage, StageHandler, StageId, Transition};
use crate::{App, AppShared, CW, Hot, Mode, SKIN_TYPE_MUSIC_SELECT, SelectTab, SelectView, SortMode};

/// How far inside the corner of a box a pixel that is to be looked at is taken from, so that its
/// outline is not what is read.
const INSIDE: u32 = 4;

/// The connection dot's side, and the stretch of the debug panel's top that is looked at.
const DOT_SIDE: f32 = 10.0;
const SAMPLE_PANEL: (f32, f32) = (200.0, 40.0);

/// Half the connection dot's side: from its corner to its middle.
const DOT_HALF: u32 = 5;

/// How far down from the top of the hint's card a pixel above its first line is.
const ABOVE_THE_TITLE: u32 = 6;

const LEFT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: true };

/// The three charts of the fixture browser, as the title order lists them, and the md5 the first of
/// them carries.
const FIRST_CHART_MD5: &str = "md5-alpha";

/// How far to the right of the ranking panel a press is still over the fixture's wheel, which spans
/// x 400 to 800 of the screen the application is laid out for.
const PAST_THE_PANEL: f32 = 52.0;

/// A press of the left button at `at`, run through the frame that follows it.
fn click_at(browser: &mut Browser, at: (f32, f32)) -> Transition {
    browser.pointer(at, LEFT_PRESS);
    browser.frame_after(super::skinned_tests::SHORT_STEP)
}

/// The presses on a bar of the skin's wheel that are waiting for the next frame, which is what a
/// press the skin was offered leaves behind.
fn bar_presses(browser: &mut Browser) -> usize {
    browser.app.shared.take_skin_bar_presses(SKIN_TYPE_MUSIC_SELECT).len()
}

/// Whether the frame drawn last registered a region of this kind among the clickable ones.
fn has_region(browser: &Browser, wanted: Hot) -> bool {
    browser.app.shared.hot.iter().any(|(_, hot)| *hot == wanted)
}

/// How many pixels of a rectangle of the drawn frame carry anything.
fn painted_in(pixels: &HeadlessCanvas, area: Rect) -> usize {
    let (width, height) = pixels.size();
    let (x0, y0) = (area.x.max(0.0) as u32, area.y.max(0.0) as u32);
    let (x1, y1) = (((area.x + area.w) as u32).min(width), ((area.y + area.h) as u32).min(height));
    (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|(x, y)| pixels.pixel_at(*x, *y).a != 0).count()
}

/// The fixture's document with its wheel moved to the left edge of the screen, where none of the
/// panels that cover the middle of it reach: a press on its bars is a press on the skin and on
/// nothing the application drew over it.
fn corner_wheel_document() -> String {
    wheel_document().replace(r#""x":100"#, r#""x":0"#)
}

/// A browser drawn by [`corner_wheel_document`], and where the middle of its lowest bar is.
fn browser_with_the_wheel_in_the_corner(tag: &str) -> (Browser, (f32, f32)) {
    let songs = ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec();
    let browser = Browser::skinned_by(tag, songs, corner_wheel_document());
    (browser, (CORNER_WHEEL_X, slot_middle(SLOT_BELOW).1))
}

/// The middle of the corner wheel's bars across the screen, which no panel reaches.
const CORNER_WHEEL_X: f32 = 100.0;

/// How many pixels of a rectangle differ between `before`, the frame as it was drawn, and the frame
/// as it stands now.
fn changed_in(before: &[u8], pixels: &HeadlessCanvas, area: Rect) -> usize {
    let (width, height) = pixels.size();
    let (x0, y0) = (area.x.max(0.0) as u32, area.y.max(0.0) as u32);
    let (x1, y1) = (((area.x + area.w) as u32).min(width), ((area.y + area.h) as u32).min(height));
    let bytes_per_pixel = rbms_render::BYTES_PER_PIXEL as u32;
    (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let at = ((*y * width + *x) * bytes_per_pixel) as usize;
            before[at..at + bytes_per_pixel as usize] != pixels.rgba()[at..at + bytes_per_pixel as usize]
        })
        .count()
}

/// The browser's fixture with a record on its first chart, so the record modal has one to show.
fn with_a_record(tag: &str) -> Browser {
    let mut browser = Browser::three_songs(tag);
    browser.app.shared.scores = rbms_store::ScoreBook::from_records(vec![record(FIRST_CHART_MD5, 5, 800, 2, 1_000)]);
    browser.frame();
    browser
}

/// The guide goes up on its key and takes every key while it is up: the list stands still and a
/// chart is not started. Its own key, Escape and Enter put it away, and Escape is not the first of
/// the two that quit.
#[test]
fn the_guide_comes_up_on_its_key_takes_every_key_and_goes_on_its_key_escape_or_enter() {
    let mut browser = Browser::three_songs("overlay-guide-keys");
    assert!(!browser.state.guide_open);
    assert!(matches!(browser.key(GUIDE_KEY), Transition::Stay));
    assert!(browser.state.guide_open, "the guide key did not put it up");

    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert!(matches!(browser.key(KeyCode::ArrowDown), Transition::Stay));
    assert_eq!(browser.sel(), 0, "the list moved under the guide");
    assert!(browser.state.guide_open, "an arrow key put the guide away");
    assert!(browser.state.holds_keys(&FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }), "the application's own overlays are not told");

    for away in [KeyCode::Escape, GUIDE_KEY, KeyCode::Enter] {
        browser.state.guide_open = true;
        let asked = browser.key(away);
        assert!(matches!(asked, Transition::Stay), "{away:?} did something more than put the guide away");
        assert!(!browser.state.guide_open, "{away:?} left the guide up");
        assert!(browser.state.esc_quit_at.is_none(), "{away:?} armed the quit confirmation");
    }
}

/// The guide is over the whole screen: a press on the skin beside it puts it away and goes no
/// further, and once it is gone the same press reaches the skin again.
#[test]
fn a_press_on_the_skin_beside_the_guide_only_puts_the_guide_away() {
    let (mut browser, bar) = browser_with_the_wheel_in_the_corner("overlay-guide-mouse");
    browser.key(GUIDE_KEY);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert!(has_region(&browser, Hot::GuideClose), "the guide did not register the screen as its own");

    let asked = click_at(&mut browser, bar);
    assert!(!browser.state.guide_open, "the press did not put the guide away");
    assert!(!starts_a_chart(&asked), "the press went through the guide and started a chart");
    assert_eq!(bar_presses(&mut browser), 0, "the skin was offered the press");

    assert!(starts_a_chart(&click_at(&mut browser, bar)), "the skin does not take presses once the guide is gone");
}

/// A press on the guide's own panel is the panel's: it does not put it away, and the skin under the
/// panel is not offered it.
#[test]
fn a_press_on_the_guide_itself_leaves_it_up() {
    let mut browser = Browser::three_songs("overlay-guide-panel");
    browser.key(GUIDE_KEY);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    let asked = click_at(&mut browser, slot_middle(SLOT_BELOW));
    assert!(browser.state.guide_open, "a press on the guide put it away");
    assert!(!starts_a_chart(&asked));
    assert_eq!(bar_presses(&mut browser), 0, "the skin was offered a press on the guide");
}

/// The record modal is over the whole screen too: the press that lands anywhere puts it away, the
/// skin beside it included.
#[test]
fn a_press_on_the_skin_beside_the_record_modal_only_puts_the_modal_away() {
    let (mut browser, bar) = browser_with_the_wheel_in_the_corner("overlay-modal-mouse");
    browser.app.shared.scores = rbms_store::ScoreBook::from_records(vec![record(FIRST_CHART_MD5, 5, 800, 2, 1_000)]);
    browser.frame();
    browser.key(KeyCode::KeyR);
    assert_eq!(browser.state.record_modal, Some(0), "the record did not open");
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert!(has_region(&browser, Hot::ModalClose), "the modal registered nothing to close it by");

    let asked = click_at(&mut browser, bar);
    assert!(browser.state.record_modal.is_none(), "the press did not put the modal away");
    assert!(!starts_a_chart(&asked), "the press went through the modal and started a chart");
    assert_eq!(bar_presses(&mut browser), 0, "the skin was offered the press");
    assert!(starts_a_chart(&click_at(&mut browser, bar)), "the skin does not take presses once the modal is gone");
}

/// The ranking panel takes a press that lands on it. It is not a modal, as the built-in browser's is
/// not: a press beside it is the skin's.
#[test]
fn the_ranking_panel_takes_the_presses_on_it_and_leaves_the_rest_to_the_skin() {
    let mut browser = Browser::three_songs("overlay-ranking-mouse");
    browser.key(KeyCode::KeyI);
    assert!(browser.state.ranking_open);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert!(has_region(&browser, Hot::OverlayPanel));

    let panel = ranking_panel_rect();
    let on_it = slot_middle(SLOT_ABOVE);
    assert!(on_it.0 < panel.x + panel.w, "the fixture's wheel is not under the panel: the test is aimed wrong");
    let asked = click_at(&mut browser, on_it);
    assert!(!starts_a_chart(&asked), "a press on the panel went through it and started a chart");
    assert_eq!(bar_presses(&mut browser), 0, "the skin was offered a press on the panel");

    let beside = (panel.x + panel.w + PAST_THE_PANEL, slot_middle(SLOT_BELOW).1);
    assert!(starts_a_chart(&click_at(&mut browser, beside)), "a press beside the panel was not the skin's");
}

/// The filter panel takes a press on it, and a press below it is the skin's.
#[test]
fn the_filter_panel_takes_the_presses_on_it_and_leaves_the_rest_to_the_skin() {
    let mut browser = Browser::three_songs("overlay-filter-mouse");
    browser.key(KeyCode::F2);
    assert!(browser.state.filter.is_open());
    browser.frame_after(super::skinned_tests::SHORT_STEP);

    let panel = filter_panel_rect();
    let on_it = slot_middle(SLOT_ABOVE);
    assert!(on_it.0 < panel.x + panel.w && on_it.1 < panel.y + panel.h, "the fixture's wheel is not under the panel: the test is aimed wrong");
    let asked = click_at(&mut browser, on_it);
    assert!(!starts_a_chart(&asked), "a press on the panel went through it and started a chart");
    assert_eq!(bar_presses(&mut browser), 0, "the skin was offered a press on the panel");

    let below = slot_middle(SLOT_BELOW);
    assert!(below.1 > panel.y + panel.h, "the lowest slot is not below the panel: the test is aimed wrong");
    assert!(starts_a_chart(&click_at(&mut browser, below)), "a press below the panel was not the skin's");
}

/// The option overlay (F1) already takes the whole mouse, over a skin as over the built-in browser.
#[test]
fn the_option_overlay_takes_the_keys_and_the_mouse_before_the_skin() {
    let mut browser = Browser::three_songs("overlay-options");
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
    assert!(options_key(&mut ctx, StageId::Select, false, &press(TOGGLE_KEY)));
    assert!(browser.app.shared.options.is_open());
    assert!(!starts_a_chart(&click_at(&mut browser, slot_middle(SLOT_BELOW))), "a press went through the option overlay");
    assert_eq!(bar_presses(&mut browser), 0, "the skin was offered a press over the option overlay");
}

/// A list with nothing in it has no bar on the skin's wheel and a hint on a card over it, in words
/// that point at keys: a browser a skin draws has no button to point at.
#[test]
fn an_empty_list_draws_no_bar_and_a_hint_that_names_keys() {
    let mut browser = Browser::skinned_by("overlay-empty", Vec::new(), wheel_document());
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    let background = browser.pixels.pixel_at(2, 2);
    for slot in [SLOT_ABOVE, SLOT_BELOW] {
        let (x, y) = slot_middle(slot);
        assert_eq!(browser.pixels.pixel_at(x as u32, y as u32), background, "the wheel drew a bar on a list with nothing in it");
    }
    let card = browser.pixels.pixel_at(CW / 2, HINT_Y as u32 + ABOVE_THE_TITLE);
    assert_ne!(card, background, "the hint is not on screen");
    assert!(painted_in(&browser.pixels, Rect::new(0.0, HINT_Y, CW as f32, HINT_H)) > 0);

    let (title, body) = browser.state.cached_scene.as_ref().and_then(|scene| scene.empty_hint).expect("a list with nothing in it carries a hint");
    assert_eq!(title, "WELCOME TO rbms");
    assert!(body.contains("press O"), "{body}");
    assert!(body.contains(shortcut_label(GUIDE_KEY)), "the hint does not say which key lists the rest: {body}");
    assert!(!body.contains("FOLDERS"), "the hint points at a button the skin does not have: {body}");

    let mut stocked = Browser::three_songs("overlay-stocked");
    stocked.frame_after(super::skinned_tests::SHORT_STEP);
    let (x, y) = slot_middle(SLOT_BELOW);
    assert_ne!(stocked.pixels.pixel_at(x as u32, y as u32), background, "the fixture's wheel draws nothing with charts in it: the test says nothing");
}

/// The built-in browser's first-run hint keeps pointing at its button.
#[test]
fn the_built_in_browser_keeps_the_hint_that_points_at_its_button() {
    let app = app();
    let (title, body) = SelectState::new().build_scene(&app.shared).empty_hint.expect("a list with nothing in it carries a hint");
    assert_eq!(title, "WELCOME TO rbms");
    assert!(body.contains("click FOLDERS below"), "{body}");
}

/// A search is drawn as a box over a skin whose own search box does not show what is typed, and goes
/// when the search does.
#[test]
fn the_search_box_is_drawn_while_a_search_is_open() {
    let mut browser = Browser::three_songs("overlay-search");
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    let inside = (STRIP_X as u32 + INSIDE, STRIP_BOTTOM as u32 - SEARCH_H as u32 + INSIDE);
    let before = browser.pixels.pixel_at(inside.0, inside.1);

    browser.key(KeyCode::Slash);
    assert!(browser.app.shared.searching);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    let during = browser.pixels.pixel_at(inside.0, inside.1);
    assert_ne!(during, before, "no search box was drawn");
    assert_eq!(during, theme().button_active, "the box is not the browser's own colour");

    browser.key(KeyCode::Escape);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert_eq!(browser.pixels.pixel_at(inside.0, inside.1), before, "the search box outlived the search");
}

/// The course tab is marked over the skin, and the mark goes with the tab.
#[test]
fn the_course_tab_is_marked() {
    let mut browser = Browser::three_songs("overlay-courses");
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    let inside = (STRIP_X as u32 + INSIDE, STRIP_BOTTOM as u32 - TAB_MARK_H as u32 + INSIDE);
    let before = browser.pixels.pixel_at(inside.0, inside.1);

    browser.state.tab = SelectTab::Courses;
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert_eq!(browser.pixels.pixel_at(inside.0, inside.1), theme().button_active, "the course tab is not marked");

    browser.state.tab = SelectTab::Songs;
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert_eq!(browser.pixels.pixel_at(inside.0, inside.1), before, "the mark outlived the tab");
}

/// A browser no skin draws gets none of it: its keys and its clickable regions are the ones it has
/// always had.
#[test]
fn a_browser_no_skin_draws_has_no_guide_and_no_overlay_regions() {
    let mut browser = Browser::over("overlay-built-in", ["alpha", "beta", "gamma"].map(|title| entry(title, "a", "5")).to_vec(), false);
    browser.frame();
    browser.key(GUIDE_KEY);
    assert!(!browser.state.guide_open, "the guide came up on the built-in browser");
    let overlay_regions = |browser: &Browser| browser.app.shared.hot.iter().any(|(_, hot)| matches!(hot, Hot::OverlayPanel | Hot::GuideClose));

    browser.key(KeyCode::F2);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert!(browser.state.filter.is_open(), "the built-in filter panel did not open");
    assert!(!overlay_regions(&browser), "a region only a skin's overlays register was registered on the built-in browser");
    browser.key(KeyCode::F2);

    browser.key(KeyCode::KeyI);
    browser.frame_after(super::skinned_tests::SHORT_STEP);
    assert!(browser.state.ranking_open, "the built-in ranking panel did not open");
    assert!(has_region(&browser, Hot::RankingRow(0)), "the built-in ranking panel registered no rows");
    assert!(!overlay_regions(&browser), "a region only a skin's overlays register was registered on the built-in browser");
}

/// Put a browser back to where a fresh one starts, so one skin can be read once and serve every key
/// of a test: the browser's own state, the list in its folder with the cursor on its first chart, no
/// search, no option overlay, no chart starred, the order the fixture lists in, and no key held.
fn reset(browser: &mut Browser) {
    browser.state = SelectState::new();
    let shared = &mut browser.app.shared;
    crate::app_options::close(shared);
    shared.searching = false;
    shared.search.clear();
    shared.practice_requested = false;
    shared.config.library.sort = SortMode::Title;
    shared.select_view = SelectView::AllSongs;
    shared.sel = 0;
    if shared.favorites.contains(FIRST_CHART_MD5) {
        shared.favorites.toggle(FIRST_CHART_MD5);
    }
    shared.keyconfig.held.clear();
    shared.rebuild_select_items();
    browser.frame();
}

/// Every key the guide lists does what it says on a browser a skin draws, and the guide leaves out
/// none of the browser's own: a key put in the table without a case here fails.
#[test]
fn every_key_the_guide_lists_does_what_it_says() {
    let mut browser = with_a_record("overlay-listed");
    for shortcut in &BROWSER_SHORTCUTS {
        for code in shortcut.keys {
            reset(&mut browser);
            match *code {
                KeyCode::ArrowDown => assert_eq!(browser.arrow(*code), 1, "{code:?}"),
                KeyCode::ArrowUp => assert_eq!(browser.arrow(*code), 2, "{code:?} goes round the top of the list"),
                KeyCode::Enter => assert!(starts_a_chart(&browser.key(*code)), "{code:?}"),
                KeyCode::Escape => {
                    browser.app.shared.select_view = SelectView::TableLevels(0);
                    browser.key(*code);
                    assert!(browser.app.shared.select_view == SelectView::Root, "{code:?} did not go back a folder");
                }
                KeyCode::KeyO => assert!(matches!(browser.key(*code), Transition::Open(Stage::Folders(_))), "{code:?}"),
                KeyCode::KeyT => assert!(matches!(browser.key(*code), Transition::Open(Stage::Tables(_))), "{code:?}"),
                KeyCode::KeyR => {
                    browser.key(*code);
                    assert!(browser.state.record_modal.is_some(), "{code:?}");
                }
                KeyCode::Slash => {
                    browser.key(*code);
                    assert!(browser.app.shared.searching, "{code:?}");
                }
                KeyCode::F2 => {
                    browser.key(*code);
                    assert!(browser.state.filter.is_open(), "{code:?}");
                }
                KeyCode::F3 => {
                    let before = browser.app.shared.config.library.sort;
                    browser.key(*code);
                    assert_ne!(browser.app.shared.config.library.sort, before, "{code:?}");
                    browser.key(KeyCode::ShiftLeft);
                    browser.key(*code);
                    assert_eq!(browser.app.shared.config.library.sort, before, "shift does not turn {code:?} round");
                }
                KeyCode::F4 => {
                    assert!(starts_a_chart(&browser.key(*code)), "{code:?}");
                    assert!(browser.app.shared.practice_requested, "{code:?} did not ask for the practice panel");
                }
                KeyCode::Tab => {
                    assert!(matches!(browser.key(*code), Transition::Open(Stage::Settings(_))), "{code:?}");
                    browser.key(KeyCode::ShiftLeft);
                    browser.key(*code);
                    assert!(browser.state.tab == SelectTab::Courses, "shift does not make {code:?} the course list");
                }
                KeyCode::KeyI => {
                    browser.key(*code);
                    assert!(browser.state.ranking_open, "{code:?}");
                }
                KeyCode::KeyF => {
                    browser.key(*code);
                    assert!(browser.app.shared.favorites.contains(FIRST_CHART_MD5), "{code:?}");
                }
                code if code == GUIDE_KEY => {
                    browser.key(code);
                    assert!(browser.state.guide_open, "{code:?}");
                }
                code if code == TOGGLE_KEY || code == HOLD_KEY => {
                    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
                    assert!(options_key(&mut ctx, StageId::Select, false, &press(code)), "{code:?}");
                    assert!(browser.app.shared.options.is_open(), "{code:?}");
                }
                other => panic!("the guide lists {other:?}, which this test does not know how to press"),
            }
        }
    }
}

/// The key that puts the guide up is nothing else's: not a lane or a control the player has by
/// default in any mode, not a key of the reference's table the skinned browser reads, not a key of
/// the ranking panel, and no two entries of the guide name the same key.
#[test]
fn the_guide_key_is_not_a_key_anything_else_takes() {
    for mode in Mode::ALL.iter().copied() {
        assert!(default_keys_for_mode(mode).iter().all(|(code, _)| *code != GUIDE_KEY), "{} plays a lane on the guide key", mode.name);
    }
    let controls = KeyConfig::default();
    for action in ControlAction::ALL {
        assert_ne!(controls.control_key(action), Some(GUIDE_KEY), "{} is on the guide key", action.label());
    }
    let reference_keys = [KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit5, KeyCode::Digit6, KeyCode::F9, KeyCode::F12];
    assert!(!reference_keys.contains(&GUIDE_KEY), "the skinned browser reads the guide key as one of the reference's");
    assert!(panel_action(GUIDE_KEY).is_none(), "the ranking panel takes the guide key");

    let listed: Vec<KeyCode> = BROWSER_SHORTCUTS.iter().flat_map(|shortcut| shortcut.keys.iter().copied()).collect();
    for (at, code) in listed.iter().enumerate() {
        assert!(!listed[at + 1..].contains(code), "{code:?} is listed twice");
    }
}

/// START, SELECT and the keys of the reference's table are read from the key configuration, so a key
/// that was rebound is the key the guide names, and a START a lane has taken is not named at all.
#[test]
fn the_guide_names_the_keys_as_they_are_bound() {
    let mut browser = Browser::three_songs("overlay-guide-bound");
    let shared = &mut browser.app.shared;
    let panels = panel_rows(shared);
    assert_eq!(
        panels.iter().map(|row| row.keys.as_str()).collect::<Vec<_>>(),
        ["A", "W", "A + W / 5"],
        "START, SELECT and both are not on the keys they ship on"
    );

    shared.keyconfig.set_control(ControlAction::Start, KeyCode::KeyQ);
    assert_eq!(panel_rows(shared)[0].keys, "Q", "a rebound START is not the key the guide names");
    assert_eq!(panel_rows(shared)[2].keys, "Q + W / 5");

    shared.keyconfig.set_lane(Mode::BEAT_7K, 3, KeyCode::KeyQ);
    shared.active_keys = shared.keyconfig.lane_keys(Mode::BEAT_7K);
    assert_eq!(panel_rows(shared)[0].keys, "-", "a START a lane has taken is still named");

    let by_role = |shared: &AppShared| key_rows(shared).into_iter().map(|row| (row.does, row.keys)).collect::<Vec<_>>();
    let seven = by_role(shared);
    let keys_of = |rows: &[(String, String)], does: &str| rows.iter().find(|(label, _)| label == does).map(|(_, keys)| keys.clone());
    assert_eq!(keys_of(&seven, "PLAY THE CHART").as_deref(), Some("Z"));
    assert_eq!(keys_of(&seven, "PRACTICE").as_deref(), Some("X"));
    assert_eq!(keys_of(&seven, "PLAY IT BY ITSELF").as_deref(), Some("C"));
    assert_eq!(keys_of(&seven, "OPEN A FOLDER").as_deref(), Some("Z X C V"));
    assert_eq!(keys_of(&seven, "NEXT BAR").as_deref(), Some("LSHIFT"));
    assert_eq!(keys_of(&seven, "PREVIOUS BAR").as_deref(), Some("-"), "no key spins the turntable the other way until one is bound");

    shared.keyconfig.set_scratch_reverse(Mode::BEAT_7K, 7, KeyCode::ControlLeft);
    shared.active_reverse_keys = shared.keyconfig.scratch_reverse_keys(Mode::BEAT_7K);
    assert_eq!(keys_of(&by_role(shared), "PREVIOUS BAR").as_deref(), Some("LCTRL"));

    shared.mode = Mode::POPN_9K;
    shared.active_keys = shared.keyconfig.lane_keys(Mode::POPN_9K);
    shared.active_reverse_keys = Vec::new();
    let nine = by_role(shared);
    assert_eq!(keys_of(&nine, "PLAY THE CHART").as_deref(), Some("C"), "nine buttons read their own table");
    assert_eq!(keys_of(&nine, "PRACTICE").as_deref(), Some("V"));
    assert_eq!(keys_of(&nine, "PLAY IT BY ITSELF").as_deref(), Some("Z"));
    assert_eq!(keys_of(&nine, "NEXT BAR").as_deref(), Some("F"));
    assert_eq!(keys_of(&nine, "PREVIOUS BAR").as_deref(), Some("D"));
}

/// Every line of the guide has the keys and what they do.
#[test]
fn every_line_of_the_guide_has_keys_and_words() {
    let browser = Browser::three_songs("overlay-guide-lines");
    let rows = browser_rows().into_iter().chain(panel_rows(&browser.app.shared)).chain(key_rows(&browser.app.shared));
    for row in rows {
        assert!(!row.keys.is_empty() && !row.does.is_empty(), "{row:?}");
    }
}

/// Draw the application's overlays over the frame the browser has just drawn, and answer the frame
/// as it was before them.
fn overlays_over(browser: &mut Browser) -> Vec<u8> {
    browser.app.stage = Stage::Select(Box::new(SelectState::new()));
    browser.frame();
    let before = browser.pixels.rgba().to_vec();
    let mut canvas = Canvas::Headless(&mut browser.pixels);
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
    App::draw_overlays(&browser.app.stage, &mut ctx, &mut canvas);
    before
}

/// Over a browser a skin draws the connection dot and the debug panel stand where the skin has the
/// least of its own, and over the built-in one they stand where they always have.
#[test]
fn the_dot_and_the_debug_panel_move_off_a_skins_frame_and_only_there() {
    let panel_area = |origin: (f32, f32)| Rect::new(origin.0, origin.1, SAMPLE_PANEL.0, SAMPLE_PANEL.1);
    let dot_area = |origin: (f32, f32)| Rect::new(origin.0, origin.1, DOT_SIDE, DOT_SIDE);
    for skinned in [true, false] {
        let mut browser = if skinned {
            Browser::three_songs("overlay-placement-skinned")
        } else {
            Browser::over("overlay-placement", ["alpha"].map(|title| entry(title, "a", "5")).to_vec(), false)
        };
        let (dot_here, dot_there, panel_here, panel_there) = if skinned {
            (crate::SKIN_DOT_ORIGIN, crate::BUILT_IN_DOT_ORIGIN, crate::SKIN_DEBUG_ORIGIN, crate::BUILT_IN_DEBUG_ORIGIN)
        } else {
            (crate::BUILT_IN_DOT_ORIGIN, crate::SKIN_DOT_ORIGIN, crate::BUILT_IN_DEBUG_ORIGIN, crate::SKIN_DEBUG_ORIGIN)
        };

        browser.app.shared.config.network.server_url = Some("http://placement.invalid".to_owned());
        let before = overlays_over(&mut browser);
        let centre = (dot_here.0 as u32 + DOT_HALF, dot_here.1 as u32 + DOT_HALF);
        assert_eq!(browser.pixels.pixel_at(centre.0, centre.1), Color::RED, "skinned {skinned}: the dot is not where it belongs");
        assert_eq!(changed_in(&before, &browser.pixels, dot_area(dot_there)), 0, "skinned {skinned}: the dot is where it does not belong");

        browser.app.shared.config.network.server_url = None;
        browser.app.shared.config.display.debug = true;
        let before = overlays_over(&mut browser);
        assert!(changed_in(&before, &browser.pixels, panel_area(panel_here)) > 0, "skinned {skinned}: the debug panel is not where it belongs");
        if skinned {
            assert_eq!(changed_in(&before, &browser.pixels, panel_area(panel_there)), 0, "the debug panel is over the skin's top frame");
        }
    }
}
