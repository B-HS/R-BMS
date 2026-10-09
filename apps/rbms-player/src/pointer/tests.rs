use std::time::Instant;

use winit::dpi::PhysicalPosition;

use super::*;
use crate::stage::{Canvas, KeyInput, SelectState, Stage, StageId};
use crate::{App, Config, Hot, KeyCode, LaunchOptions, Rect, SelectItem, SelectView};

const LEFT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: true };
const LEFT_RELEASE: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: false };
const RIGHT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Right, pressed: true };

const SPOT: (f32, f32) = (10.0, 10.0);

fn app() -> App {
    let dir = std::env::temp_dir().join(format!("rbms-pointer-tests-{}-{:?}", std::process::id(), std::thread::current().id()));
    App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron"))
}

/// A screen that writes down every mouse event it is handed, with where it was handed it.
#[derive(Default)]
struct Recorder {
    seen: Vec<(PointerInput, Option<(f32, f32)>)>,
}

impl StageHandler for Recorder {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    fn draw(&mut self, _ctx: &mut FrameCtx<'_>, _canvas: &mut Canvas<'_>) {}

    fn handle_key(&mut self, _ctx: &mut FrameCtx<'_>, _key: KeyInput<'_>) -> Transition {
        Transition::Stay
    }

    fn handle_mouse(&mut self, _ctx: &mut FrameCtx<'_>, at: (f32, f32), button: MouseButton, pressed: bool) -> Transition {
        self.seen.push((PointerInput::Button { button, pressed }, Some(at)));
        Transition::Stay
    }

    fn handle_mouse_drag(&mut self, _ctx: &mut FrameCtx<'_>, at: (f32, f32)) -> Transition {
        self.seen.push((PointerInput::Drag, Some(at)));
        Transition::Stay
    }

    fn handle_scroll(&mut self, _ctx: &mut FrameCtx<'_>, lines: f32) -> Transition {
        self.seen.push((PointerInput::Scroll { lines }, None));
        Transition::Stay
    }
}

/// A screen that implements only what every screen has to, which is what most of them are as far as
/// the mouse goes.
struct Bare;

impl StageHandler for Bare {
    fn update(&mut self, _ctx: &mut FrameCtx<'_>) -> Transition {
        Transition::Stay
    }

    fn draw(&mut self, _ctx: &mut FrameCtx<'_>, _canvas: &mut Canvas<'_>) {}

    fn handle_key(&mut self, _ctx: &mut FrameCtx<'_>, _key: KeyInput<'_>) -> Transition {
        Transition::Stay
    }
}

fn route(app: &mut App, handler: &mut dyn StageHandler, at: (f32, f32), input: PointerInput) -> Transition {
    route_pointer(handler, &mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, at, input)
}

/// The wheel is reported in the direction the content should move, and a screen is told the
/// direction the wheel was rolled: towards the player is positive, which is on to the next entry.
#[test]
fn a_wheel_rolled_towards_the_player_is_positive_lines() {
    assert_eq!(PointerInput::from_wheel(MouseScrollDelta::LineDelta(0.0, -1.0)), PointerInput::Scroll { lines: 1.0 });
    assert_eq!(PointerInput::from_wheel(MouseScrollDelta::LineDelta(0.0, 3.0)), PointerInput::Scroll { lines: -3.0 });
    assert_eq!(PointerInput::from_wheel(MouseScrollDelta::LineDelta(5.0, 0.0)), PointerInput::Scroll { lines: 0.0 }, "sideways travel is not a scroll");
}

#[test]
fn touchpad_pixels_are_converted_to_lines_at_a_fixed_rate() {
    let travel = PhysicalPosition::new(0.0, -2.0 * f64::from(PIXELS_PER_SCROLL_LINE));
    assert_eq!(PointerInput::from_wheel(MouseScrollDelta::PixelDelta(travel)), PointerInput::Scroll { lines: 2.0 });
}

#[test]
fn only_the_three_main_buttons_are_told_to_a_screen() {
    for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
        assert!(is_tracked(button), "{button:?} is left out");
    }
    for button in [MouseButton::Back, MouseButton::Forward, MouseButton::Other(7)] {
        assert!(!is_tracked(button), "{button:?} reaches a screen");
    }
}

/// A move is a drag from the first press until the last button is let go, whichever order they
/// come up in, and a press seen twice is still released by one release.
#[test]
fn a_cursor_move_is_a_drag_only_while_a_button_is_held() {
    let mut held = HeldButtons::default();
    assert_eq!(held.on_move(), None, "nothing is held yet");

    assert_eq!(held.on_button(MouseButton::Left, true), LEFT_PRESS);
    assert_eq!(held.on_move(), Some(PointerInput::Drag));

    held.on_button(MouseButton::Right, true);
    assert_eq!(held.on_button(MouseButton::Left, false), LEFT_RELEASE);
    assert_eq!(held.on_move(), Some(PointerInput::Drag), "the right button is still down");

    held.on_button(MouseButton::Right, false);
    assert_eq!(held.on_move(), None, "every button is up again");

    held.on_button(MouseButton::Middle, true);
    held.on_button(MouseButton::Middle, true);
    held.on_button(MouseButton::Middle, false);
    assert_eq!(held.on_move(), None, "a press reported twice is not held twice");
}

/// A window that loses focus never sees the release, so the buttons it thought were down must not
/// keep every later move a drag.
#[test]
fn losing_focus_lets_go_of_every_button() {
    let mut held = HeldButtons::default();
    held.on_button(MouseButton::Left, true);
    held.on_button(MouseButton::Middle, true);
    held.clear();
    assert_eq!(held.on_move(), None);
}

/// A release, a drag and a wheel turn each reach the screen that is up, and the press that always
/// did still does, at the position it happened.
#[test]
fn every_kind_of_mouse_event_reaches_the_screen() {
    let mut app = app();
    let mut screen = Recorder::default();
    for (at, input) in [
        (SPOT, LEFT_PRESS),
        ((20.0, 30.0), PointerInput::Drag),
        ((20.0, 30.0), LEFT_RELEASE),
        (SPOT, RIGHT_PRESS),
        (SPOT, PointerInput::Scroll { lines: -2.0 }),
    ] {
        route(&mut app, &mut screen, at, input);
    }
    assert_eq!(
        screen.seen,
        vec![
            (LEFT_PRESS, Some(SPOT)),
            (PointerInput::Drag, Some((20.0, 30.0))),
            (LEFT_RELEASE, Some((20.0, 30.0))),
            (RIGHT_PRESS, Some(SPOT)),
            (PointerInput::Scroll { lines: -2.0 }, None),
        ]
    );
}

/// An open option panel takes the mouse the way it takes the keys, and gives it back when it closes.
#[test]
fn an_open_option_panel_takes_every_kind_of_mouse_event() {
    let mut app = app();
    let mut screen = Recorder::default();
    let pin = KeyInput { code: KeyCode::F1, pressed: true, released: false, text: None };
    let mut ctx = FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 };
    assert!(crate::app_options::options_key(&mut ctx, StageId::Select, false, &pin), "the pin key did not open the panel");
    for input in [LEFT_PRESS, PointerInput::Drag, LEFT_RELEASE, PointerInput::Scroll { lines: 1.0 }] {
        assert!(matches!(route(&mut app, &mut screen, SPOT, input), Transition::Stay));
    }
    assert!(screen.seen.is_empty(), "the panel let {} events through", screen.seen.len());

    crate::app_options::close(&mut app.shared);
    route(&mut app, &mut screen, SPOT, LEFT_PRESS);
    assert_eq!(screen.seen.len(), 1, "the screen hears the mouse again once the panel is gone");
}

#[test]
fn a_screen_that_does_not_handle_the_mouse_stays_where_it_is() {
    let mut app = app();
    for input in [LEFT_PRESS, PointerInput::Drag, LEFT_RELEASE, RIGHT_PRESS, PointerInput::Scroll { lines: 1.0 }] {
        assert!(matches!(route(&mut app, &mut Bare, SPOT, input), Transition::Stay), "{input:?} moved a screen that ignores the mouse");
    }
}

/// The browser acts on the left button going down and on nothing else the mouse can do, so the
/// click that always worked is the one click that still does.
#[test]
fn the_browser_still_acts_on_a_left_press_and_on_no_other_mouse_event() {
    let mut app = app();
    app.stage = Stage::Select(Box::new(SelectState::new()));
    app.shared.select_items = vec![
        SelectItem::Folder { label: "one".into(), target: SelectView::AllSongs },
        SelectItem::Folder { label: "two".into(), target: SelectView::AllSongs },
    ];
    app.shared.sel = 0;
    app.shared.hot.push((Rect::new(0.0, 0.0, 100.0, 100.0), Hot::SelectRow(1)));
    let click = |app: &mut App, input| {
        app.stage.handle_pointer(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, SPOT, input);
        app.shared.sel
    };

    assert_eq!(click(&mut app, RIGHT_PRESS), 0, "a right press clicked a row");
    assert_eq!(click(&mut app, PointerInput::Button { button: MouseButton::Middle, pressed: true }), 0, "a middle press clicked a row");
    assert_eq!(click(&mut app, LEFT_RELEASE), 0, "letting go clicked a row");
    assert_eq!(click(&mut app, PointerInput::Drag), 0, "a drag clicked a row");
    assert_eq!(click(&mut app, PointerInput::Scroll { lines: 1.0 }), 0, "the wheel clicked a row");
    assert_eq!(click(&mut app, LEFT_PRESS), 1, "a left press has to click the row");
}
