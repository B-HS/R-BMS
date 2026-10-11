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
use super::guide::{FIXED_KEYS, GUIDE_KEY, columns, key_rows, panel_rows, shortcut_label};
use super::overlay::{HINT_H, HINT_Y, SEARCH_H, STRIP_BOTTOM, STRIP_X, TAB_MARK_H};
use super::skinned_tests::{Browser, SLOT_ABOVE, SLOT_BELOW, slot_middle, starts_a_chart, wheel_document};
use super::tests::{app, entry, press, record};
use crate::app_options::{TOGGLE_KEY, options_key};
use crate::ir_ranking_view::panel_action;
use crate::keyconfig::{ControlAction, KeyConfig, Shortcut, ShortcutGroup, ShortcutScreen, default_keys_for_mode};
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

/// A place on the corner wheel's bars across the screen that no panel reaches: the key guide is the
/// widest of them and stops short of the screen's left edge.
const CORNER_WHEEL_X: f32 = 12.0;

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

/// Press the key a shortcut of the browser is on and check that the browser did what the shortcut
/// says, with `code` being whatever key the shortcut is on in this browser.
fn check_shortcut(browser: &mut Browser, shortcut: Shortcut, code: KeyCode) {
    match shortcut {
        Shortcut::Folders => assert!(matches!(browser.key(code), Transition::Open(Stage::Folders(_))), "{shortcut:?}"),
        Shortcut::Tables => assert!(matches!(browser.key(code), Transition::Open(Stage::Tables(_))), "{shortcut:?}"),
        Shortcut::Records => {
            browser.key(code);
            assert!(browser.state.record_modal.is_some(), "{shortcut:?}");
        }
        Shortcut::Search | Shortcut::RefSearch => {
            browser.key(code);
            assert!(browser.app.shared.searching, "{shortcut:?}");
        }
        Shortcut::Filter => {
            browser.key(code);
            assert!(browser.state.filter.is_open(), "{shortcut:?}");
            browser.key(code);
            assert!(!browser.state.filter.is_open(), "{shortcut:?} did not close the panel it opened");
        }
        Shortcut::Sort => {
            let before = browser.app.shared.config.library.sort;
            browser.key(code);
            assert_ne!(browser.app.shared.config.library.sort, before, "{shortcut:?}");
            browser.key(KeyCode::ShiftRight);
            browser.key(code);
            assert_eq!(browser.app.shared.config.library.sort, before, "shift does not turn {shortcut:?} round");
        }
        Shortcut::RefSort => {
            let before = browser.app.shared.config.library.sort;
            browser.key(code);
            assert_ne!(browser.app.shared.config.library.sort, before, "{shortcut:?}");
        }
        Shortcut::Practice => {
            assert!(starts_a_chart(&browser.key(code)), "{shortcut:?}");
            assert!(browser.app.shared.practice_requested, "{shortcut:?} did not ask for the practice panel");
        }
        Shortcut::Settings => {
            assert!(matches!(browser.key(code), Transition::Open(Stage::Settings(_))), "{shortcut:?}");
            browser.key(KeyCode::ShiftRight);
            browser.key(code);
            assert!(browser.state.tab == SelectTab::Courses, "shift does not make {shortcut:?} the course list");
        }
        Shortcut::Ranking => {
            browser.key(code);
            assert!(browser.state.ranking_open, "{shortcut:?}");
            browser.key(code);
            assert!(!browser.state.ranking_open, "{shortcut:?} did not close the panel it opened");
        }
        Shortcut::Favourite | Shortcut::RefFavourite => {
            browser.key(code);
            assert!(browser.app.shared.favorites.contains(FIRST_CHART_MD5), "{shortcut:?}");
        }
        Shortcut::SkinMenu => assert!(matches!(browser.key(code), Transition::Open(Stage::SkinMenu(_))), "{shortcut:?}"),
        Shortcut::KeyConfig | Shortcut::RefKeyConfig => assert!(matches!(browser.key(code), Transition::Open(Stage::KeyConfig(_))), "{shortcut:?}"),
        Shortcut::OptionsPin | Shortcut::OptionsHold => {
            let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
            assert!(options_key(&mut ctx, StageId::Select, false, &press(code)), "{shortcut:?}");
            assert!(browser.app.shared.options.is_open(), "{shortcut:?}");
        }
        Shortcut::RefMode => {
            browser.key(code);
            assert!(browser.state.filter.filter(&browser.app.shared.config).mode.is_some(), "{shortcut:?}");
        }
        Shortcut::RefLnMode => {
            let before = browser.app.shared.config.judge.ln_mode;
            browser.key(code);
            assert_ne!(browser.app.shared.config.judge.ln_mode, before, "{shortcut:?}");
        }
        Shortcut::RefReplay => {
            browser.key(code);
        }
        Shortcut::RefDetail => {
            browser.app.shared.note_key(&press(code));
            browser.frame();
            assert_eq!(browser.state.panel.number(), DETAIL_PANEL, "{shortcut:?}");
        }
        other => panic!("{other:?} is not a shortcut of the browser"),
    }
}

/// The number of the detail panel, which is the third of the three.
const DETAIL_PANEL: u8 = 3;

/// Every key the guide lists does what it says on a browser a skin draws: the fixed keys, and every
/// shortcut of the browser on the key it ships on. A shortcut added without a case here fails.
#[test]
fn every_key_the_guide_lists_does_what_it_says() {
    let mut browser = with_a_record("overlay-listed");
    for fixed in &FIXED_KEYS {
        for code in fixed.keys {
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
                other => panic!("the guide lists {other:?}, which this test does not know how to press"),
            }
        }
    }
    for shortcut in Shortcut::ALL.into_iter().filter(|shortcut| shortcut.screen() == ShortcutScreen::Browser) {
        reset(&mut browser);
        check_shortcut(&mut browser, shortcut, shortcut.default_key());
    }
    reset(&mut browser);
    browser.key(GUIDE_KEY);
    assert!(browser.state.guide_open, "the guide key");
}

/// The key that puts the guide up is nothing else's: not a lane or a control the player has by
/// default in any mode, not the key of any shortcut as shipped, not a key of the ranking panel, and
/// no two shortcuts of the browser ship on one key.
#[test]
fn the_guide_key_is_not_a_key_anything_else_takes() {
    for mode in Mode::ALL.iter().copied() {
        assert!(default_keys_for_mode(mode).iter().all(|(code, _)| *code != GUIDE_KEY), "{} plays a lane on the guide key", mode.name);
    }
    let controls = KeyConfig::default();
    for action in ControlAction::ALL {
        assert_ne!(controls.control_key(action), Some(GUIDE_KEY), "{} is on the guide key", action.label());
    }
    assert!(panel_action(GUIDE_KEY).is_none(), "the ranking panel takes the guide key");

    let listed: Vec<KeyCode> =
        Shortcut::ALL.into_iter().filter(|shortcut| shortcut.screen() == ShortcutScreen::Browser).map(Shortcut::default_key).chain([GUIDE_KEY]).collect();
    for (at, code) in listed.iter().enumerate() {
        assert!(!listed[at + 1..].contains(code), "{code:?} is shipped on twice");
    }
}

/// A shortcut the player moved is read from the key it was moved to and no longer from the key it
/// shipped on, and the guide names the key it is on now. A shortcut left without a key is read
/// from none and the guide shows it so.
#[test]
fn a_moved_shortcut_is_read_from_its_new_key_and_the_guide_names_it() {
    let free = [KeyCode::F6, KeyCode::F7, KeyCode::F8, KeyCode::F10, KeyCode::F11, KeyCode::Insert, KeyCode::Home, KeyCode::End, KeyCode::Backquote];
    let moved: Vec<Shortcut> = Shortcut::ALL
        .into_iter()
        .filter(|shortcut| shortcut.screen() == ShortcutScreen::Browser && !matches!(shortcut, Shortcut::OptionsPin | Shortcut::OptionsHold))
        .collect();
    for (index, shortcut) in moved.into_iter().enumerate() {
        let to = free[index % free.len()];
        let mut browser = with_a_record(&format!("overlay-moved-{index}"));
        reset(&mut browser);
        let mode = browser.app.shared.mode;
        browser.app.shared.keyconfig.bind(mode, crate::keyconfig::Bindable::Shortcut(shortcut), Some(to));

        let named = columns(&browser.app.shared)
            .into_iter()
            .flatten()
            .flat_map(|section| section.rows)
            .any(|row| row.keys.split(' ').any(|key| key == shortcut_label(to)));
        assert!(named, "the guide does not name {to:?} once {shortcut:?} is on it");

        let before = (
            browser.app.shared.searching,
            browser.state.filter.is_open(),
            browser.app.shared.config.library.sort,
            browser.state.record_modal,
            browser.state.ranking_open,
            browser.app.shared.favorites.contains(FIRST_CHART_MD5),
            browser.app.shared.config.judge.ln_mode,
        );
        let asked = browser.key(shortcut.default_key());
        browser.app.shared.note_key(&super::tests::release(shortcut.default_key()));
        let after = (
            browser.app.shared.searching,
            browser.state.filter.is_open(),
            browser.app.shared.config.library.sort,
            browser.state.record_modal,
            browser.state.ranking_open,
            browser.app.shared.favorites.contains(FIRST_CHART_MD5),
            browser.app.shared.config.judge.ln_mode,
        );
        assert!(matches!(asked, Transition::Stay) && before == after, "{shortcut:?} is still read from the key it shipped on");

        check_shortcut(&mut browser, shortcut, to);
    }

    let mut browser = with_a_record("overlay-unbound");
    reset(&mut browser);
    let mode = browser.app.shared.mode;
    browser.app.shared.keyconfig.bind(mode, crate::keyconfig::Bindable::Shortcut(Shortcut::Folders), None);
    assert!(matches!(browser.key(Shortcut::Folders.default_key()), Transition::Stay), "a shortcut with no key was read from the one it shipped on");
    let screens = super::guide::screen_rows(&browser.app.shared);
    assert!(screens.iter().any(|row| row.keys == "-" && row.does == Shortcut::Folders.label()), "the guide does not show the shortcut as having no key");
}

/// The guide lists every shortcut of the browser, group by group, and the controls of a run, each
/// with the key the key configuration holds for it.
#[test]
fn the_guide_lists_every_shortcut_of_the_browser_and_the_controls_of_a_run() {
    let mut browser = Browser::three_songs("overlay-guide-whole");
    let shared = &mut browser.app.shared;
    let all: Vec<String> = columns(shared).into_iter().flatten().flat_map(|section| section.rows).map(|row| format!("{} = {}", row.keys, row.does)).collect();
    for group in [ShortcutGroup::Browser, ShortcutGroup::Screens, ShortcutGroup::SkinBrowser] {
        for shortcut in Shortcut::ALL.into_iter().filter(|shortcut| shortcut.group() == group) {
            let key = shortcut_label(shortcut.default_key());
            assert!(
                all.iter().any(|line| line.split(" = ").next().is_some_and(|keys| keys.split(' ').any(|word| word == key))),
                "{shortcut:?} ({key}) is not on the guide: {all:#?}"
            );
        }
    }
    assert!(all.contains(&"LEFT RIGHT = LANE COVER LESS / MORE".to_string()), "{all:#?}");
    assert!(all.contains(&"BACKSLASH = LANE COVER ON/OFF".to_string()), "{all:#?}");
    assert!(all.contains(&"A TWICE = LANE COVER ON/OFF".to_string()), "{all:#?}");

    shared.keyconfig.set_control(ControlAction::CoverUp, KeyCode::KeyP);
    shared.keyconfig.set_control(ControlAction::CoverToggle, KeyCode::KeyU);
    let all: Vec<String> = columns(shared).into_iter().flatten().flat_map(|section| section.rows).map(|row| format!("{} = {}", row.keys, row.does)).collect();
    assert!(all.contains(&"LEFT P = LANE COVER LESS / MORE".to_string()), "{all:#?}");
    assert!(all.contains(&"U = LANE COVER ON/OFF".to_string()), "{all:#?}");
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
    let rows = columns(&browser.app.shared).into_iter().flatten().flat_map(|section| section.rows);
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

/// Names the folder the frames of the guide are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The two sizes the guide is drawn at by itself: the one it is laid out for, and a full HD window.
const GUIDE_SIZES: [(u32, u32); 2] = [(CW, crate::CH), (1920, 1080)];

/// Save a frame as `<name>.png` in the capture folder, when one was asked for.
fn save_capture(pixels: &HeadlessCanvas, name: &str) {
    if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
        let folder = std::path::Path::new(&folder);
        std::fs::create_dir_all(folder).expect("the capture folder can be created");
        let (width, height) = pixels.size();
        image::save_buffer(folder.join(format!("{name}.png")), pixels.rgba(), width, height, image::ExtendedColorType::Rgba8).expect("the capture is written");
    }
}

/// The guide fits the screen the application is laid out for with every line it lists, as shipped
/// and with every key that can be moved put on the longest word a key has, and it draws at that
/// size and at full HD.
#[test]
fn the_guide_fits_the_screen_and_draws_at_both_sizes() {
    let mut browser = Browser::three_songs("overlay-guide-fits");
    let panel = super::guide::guide_rect(&columns(&browser.app.shared));
    assert!(panel.y >= 0.0 && panel.y + panel.h <= crate::CH as f32, "the guide is {} tall and runs off the screen", panel.h);
    assert!(panel.x >= 0.0 && panel.x + panel.w <= CW as f32);

    browser.key(GUIDE_KEY);
    browser.frame();
    save_capture(&browser.pixels, "guide-over-a-browser-1280x720");

    let mode = browser.app.shared.mode;
    browser.app.shared.keyconfig.bind(mode, crate::keyconfig::Bindable::Shortcut(Shortcut::Search), Some(KeyCode::F6));
    browser.app.shared.keyconfig.bind(mode, crate::keyconfig::Bindable::Shortcut(Shortcut::Folders), None);
    browser.app.shared.keyconfig.set_control(ControlAction::CoverUp, KeyCode::KeyP);
    browser.app.shared.keyconfig.set_control(ControlAction::LiftToggle, KeyCode::F8);
    for size in GUIDE_SIZES {
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        let mut canvas = Canvas::Headless(&mut pixels);
        canvas.clear(Color::BLACK);
        super::guide::render_guide(&mut canvas, &browser.app.shared);
        assert!(pixels.painted_pixels() > 0);
        save_capture(&pixels, &format!("guide-with-moved-keys-{}x{}", size.0, size.1));
    }
}
