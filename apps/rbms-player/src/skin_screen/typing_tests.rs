//! Typing into the editable texts of a skin, and the sliders a skin writes volumes through, from the
//! pointer and the keyboard to what the application ends up holding.
//!
//! A small Lua skin for the decide screen stands in for a published one: a search box that says
//! nothing but the string it reads, a text that writes through a function of the skin, and a volume
//! slider. Every step goes through the entry points the window uses.

use rbms_skin::property::generated::{RATE_MASTERVOLUME, STRING_SEARCHWORD};
use winit::keyboard::KeyCode;

use super::*;
use crate::stage::{FrameCtx, SelectState, Stage};

/// The file the skin's one image source and its font are written to, beside the skin.
const PANEL_SHEET: &str = "panel.png";
const PANEL_EDGE: u32 = 8;
const FACE_FILE: &str = "face.ttf";

/// The rate the skin's volume slider writes, which is the system volume.
const VOLUME_RATE: i32 = RATE_MASTERVOLUME;

/// The share of the volume slider's travel a press a quarter of the way along it writes.
const VOLUME_QUARTER: f32 = 0.25;

/// The key START is on a fresh install (`A`).
const START_KEY: KeyCode = KeyCode::KeyA;

/// What a word the player types in is, and the text the second box starts with.
const TYPED_WORD: &str = "ab";
const KOREAN_SYLLABLE: &str = "한";
const START_TEXT: &str = "start";

/// The skin: `search` is a text that only names the search word as its `ref`, `named` writes what is
/// typed into it to a global of the skin, and `volume` is a slider on the system volume.
const TYPING_DECIDE: &str = r#"
written = "never written"
local skin = { type = 6, name = "Typing", w = 1280, h = 720 }
if skin_config then
    skin.font = { { id = 0, path = "FACE_FILE" } }
    skin.source = { { id = 0, path = "PANEL_SHEET" } }
    skin.text = {
        { id = "search", font = 0, size = 40, ref = SEARCH_ID },
        { id = "named", font = 0, size = 40, constantText = "START_TEXT", editable = true, event = function(text) written = text end },
    }
    skin.slider = { { id = "volume", src = 0, x = 0, y = 0, w = PANEL_EDGE, h = PANEL_EDGE, angle = 1, range = 200, type = VOLUME_RATE } }
    skin.destination = {
        { id = "search", dst = { { x = 100, y = 600, w = 400, h = 40 } } },
        { id = "named", dst = { { x = 100, y = 500, w = 400, h = 40 } } },
        { id = "volume", dst = { { x = 100, y = 400, w = 20, h = 30 } } },
    }
end
return skin
"#;

const LEFT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: true };
const LEFT_RELEASE: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: false };

/// A point of the skin, in its own upward coordinates, as the cursor position the window reports.
fn cursor_at(x: f32, y: f32) -> (f32, f32) {
    (x, UI_SIZE.1 as f32 - y)
}

fn over_search() -> (f32, f32) {
    cursor_at(200.0, 620.0)
}

fn over_named() -> (f32, f32) {
    cursor_at(200.0, 520.0)
}

fn over_nothing() -> (f32, f32) {
    cursor_at(900.0, 100.0)
}

fn over_volume_quarter() -> (f32, f32) {
    cursor_at(150.0, 415.0)
}

/// The rows of the screen the search box is drawn on.
fn search_rows() -> std::ops::Range<u32> {
    UI_SIZE.1 - 640..UI_SIZE.1 - 600
}

/// An app drawing the typing skin on its decide screen, with one whole frame of it behind it.
fn app_with_typing_skin(tag: &str) -> (crate::App, crate::stage::HeadlessCanvas) {
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-typing-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let pack = home.join("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    let source = TYPING_DECIDE
        .replace("FACE_FILE", FACE_FILE)
        .replace("PANEL_SHEET", PANEL_SHEET)
        .replace("PANEL_EDGE", &PANEL_EDGE.to_string())
        .replace("SEARCH_ID", &STRING_SEARCHWORD.to_string())
        .replace("START_TEXT", START_TEXT)
        .replace("VOLUME_RATE", &VOLUME_RATE.to_string());
    std::fs::write(pack.join("decide.luaskin"), source).expect("the skin is written");
    std::fs::write(pack.join(FACE_FILE), include_bytes!("../../../../assets/fonts/Inter-Regular.ttf")).expect("the font is written");
    image::RgbaImage::from_pixel(PANEL_EDGE, PANEL_EDGE, image::Rgba([u8::MAX; 4])).save(pack.join(PANEL_SHEET)).expect("the sheet is written");
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    let began = Instant::now();
    while !app.shared.has_compiled_skin(SKIN_TYPE_DECIDE) {
        assert!(began.elapsed() < Duration::from_secs(20), "the skin never compiled: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));
        draw_one(&mut app, &mut pixels);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(app.shared.skin_warnings(SKIN_TYPE_DECIDE), &[] as &[String], "the skin did not compile whole");
    whole_frame(&mut app, &mut pixels);
    (app, pixels)
}

fn draw_one(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas) -> bool {
    let mut canvas = Canvas::Headless(pixels);
    app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
    let chart = ChartMeta::default();
    app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() })
}

/// One frame of the decide screen as the frame loop makes it: drawn, and then ended.
fn whole_frame(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas) {
    assert!(draw_one(app, pixels), "the skin did not draw its screen");
    app.shared.finish_skin_frame(&mut Canvas::Headless(pixels));
}

/// How many pixels of the search box's rows are not black.
fn lit_in_search_box(pixels: &crate::stage::HeadlessCanvas) -> usize {
    search_rows().flat_map(|y| (0..UI_SIZE.0).map(move |x| (x, y))).filter(|(x, y)| pixels.pixel_at(*x, *y) != Color::BLACK).count()
}

fn skin_text(app: &crate::App, name: &str) -> String {
    let runtime = app.shared.skins.document(SKIN_TYPE_DECIDE).and_then(LoadedSkin::runtime).expect("the skin is loaded with its interpreter");
    runtime.lua().globals().get(name).expect("the skin keeps the text in a global")
}

fn key_down(code: KeyCode, text: Option<&'static str>) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text }
}

fn key_repeat(code: KeyCode, text: Option<&'static str>) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: false, text }
}

fn key_up(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: true, text: None }
}

/// Presses the search box and runs the frame that starts the typing.
fn start_typing(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas, at: (f32, f32)) {
    assert!(app.shared.skin_pointer(at, LEFT_PRESS), "the text under the cursor did not take the press");
    assert!(!app.shared.skin_text_is_focused(), "the typing began before the frame that runs the press");
    whole_frame(app, pixels);
    assert!(app.shared.skin_text_is_focused(), "the press did not start any typing");
}

fn type_word(app: &mut crate::App) {
    for (code, text) in [(KeyCode::KeyA, "a"), (KeyCode::KeyB, "b")] {
        assert!(app.shared.skin_text_key(&key_down(code, Some(text))), "a key was not taken by the text being typed into");
    }
}

/// A press on the search box starts the typing, what is typed is drawn in the box with a caret, and
/// Enter writes it to the browser's cluster -- after the frame that follows, which is where a
/// writer of the skin runs.
#[test]
fn a_word_typed_into_the_search_box_is_drawn_and_written_to_the_browser_on_enter() {
    let (mut app, mut pixels) = app_with_typing_skin("search");
    assert_eq!(lit_in_search_box(&pixels), 0, "the search word reads empty, so the box shows nothing");

    start_typing(&mut app, &mut pixels, over_search());
    whole_frame(&mut app, &mut pixels);
    let caret_only = lit_in_search_box(&pixels);
    assert!(caret_only > 0, "an empty box being typed into shows its caret");

    type_word(&mut app);
    whole_frame(&mut app, &mut pixels);
    assert!(lit_in_search_box(&pixels) > caret_only, "the typed letters are drawn in the box");
    assert!(app.shared.skin_requests().is_empty(), "nothing is written before the typing is confirmed");

    assert!(app.shared.skin_text_key(&key_down(KeyCode::Enter, None)));
    assert!(!app.shared.skin_text_is_focused(), "Enter ends the typing");
    whole_frame(&mut app, &mut pixels);
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: TYPED_WORD.to_string() }],
        "what was typed reaches the browser"
    );
    whole_frame(&mut app, &mut pixels);
    assert_eq!(lit_in_search_box(&pixels), 0, "and the box reads empty again, as the search word does");
}

/// The input method's text is shown where it will land, underlined, and is part of the line only when
/// it commits.
#[test]
fn text_an_input_method_composes_is_shown_and_only_a_commit_is_written() {
    let (mut app, mut pixels) = app_with_typing_skin("composing");
    start_typing(&mut app, &mut pixels, over_search());
    type_word(&mut app);
    whole_frame(&mut app, &mut pixels);
    let typed = lit_in_search_box(&pixels);

    app.shared.skin_text_ime(&Ime::Preedit(KOREAN_SYLLABLE.to_string(), Some((0, KOREAN_SYLLABLE.len()))));
    whole_frame(&mut app, &mut pixels);
    assert!(lit_in_search_box(&pixels) > typed, "the syllable being composed is drawn after the letters");

    app.shared.skin_text_ime(&Ime::Preedit(String::new(), None));
    app.shared.skin_text_ime(&Ime::Commit(KOREAN_SYLLABLE.to_string()));
    assert!(app.shared.skin_text_key(&key_down(KeyCode::Enter, None)));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: format!("{TYPED_WORD}{KOREAN_SYLLABLE}") }],
        "the committed syllable is part of what was typed"
    );

    start_typing(&mut app, &mut pixels, over_search());
    type_word(&mut app);
    app.shared.skin_text_ime(&Ime::Preedit(KOREAN_SYLLABLE.to_string(), None));
    assert!(app.shared.skin_text_key(&key_down(KeyCode::Enter, None)));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: TYPED_WORD.to_string() }],
        "a syllable still being composed is not written"
    );
}

/// Escape throws the typing away, a press on the text starts it over from what the text shows, and a
/// press anywhere else confirms it.
#[test]
fn escape_cancels_a_press_on_the_text_restarts_and_a_press_outside_confirms() {
    let (mut app, mut pixels) = app_with_typing_skin("ends");
    start_typing(&mut app, &mut pixels, over_search());
    type_word(&mut app);
    assert!(app.shared.skin_text_key(&key_down(KeyCode::Escape, None)));
    assert!(!app.shared.skin_text_is_focused());
    whole_frame(&mut app, &mut pixels);
    whole_frame(&mut app, &mut pixels);
    assert!(app.shared.skin_requests().is_empty(), "Escape wrote nothing");

    start_typing(&mut app, &mut pixels, over_search());
    type_word(&mut app);
    assert!(!app.shared.skin_pointer(over_search(), LEFT_RELEASE), "letting go is nothing to the skin");
    assert!(app.shared.skin_text_is_focused(), "a button coming up does not end the typing");
    assert!(app.shared.skin_pointer(over_search(), LEFT_PRESS), "a press inside the box is the box's");
    whole_frame(&mut app, &mut pixels);
    assert!(app.shared.skin_text_is_focused(), "and keeps it typed into");
    assert!(app.shared.skin_text_key(&key_down(KeyCode::Enter, None)));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: String::new() }],
        "pressing the box again starts over from what it shows, as the reference does, so the word typed first is gone"
    );

    start_typing(&mut app, &mut pixels, over_search());
    type_word(&mut app);
    assert!(!app.shared.skin_pointer(over_nothing(), LEFT_PRESS), "a press over nothing is the screen's");
    assert!(!app.shared.skin_text_is_focused(), "but it ends the typing first");
    whole_frame(&mut app, &mut pixels);
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: TYPED_WORD.to_string() }],
        "a press outside confirms what was typed"
    );
    assert!(!app.shared.skin_text_is_focused());

    start_typing(&mut app, &mut pixels, over_search());
    assert!(app.shared.skin_pointer(over_search(), LEFT_PRESS));
    assert!(app.shared.skin_pointer(over_named(), LEFT_PRESS), "a press on another text is taken by that text");
    whole_frame(&mut app, &mut pixels);
    assert!(app.shared.skin_text_is_focused(), "the second text is typed into now");
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: String::new() }],
        "the first text, ended by the press, wrote its empty word, and the press that ended it did not start it over"
    );
}

/// A text written through a function of the skin starts from what it shows and calls the function with
/// the line, inside a frame.
#[test]
fn a_text_with_a_writer_of_the_skin_starts_from_what_it_shows_and_calls_it_on_confirm() {
    let (mut app, mut pixels) = app_with_typing_skin("function");
    start_typing(&mut app, &mut pixels, over_named());
    assert!(app.shared.skin_text_key(&key_down(KeyCode::Backspace, None)));
    assert!(app.shared.skin_text_key(&key_repeat(KeyCode::Backspace, None)), "a key held down repeats");
    assert!(app.shared.skin_text_key(&key_down(KeyCode::KeyX, Some("x"))));
    assert!(app.shared.skin_text_key(&key_down(KeyCode::Enter, None)));
    assert_eq!(skin_text(&app, "written"), "never written", "the function ran outside a frame, where its host is not bound");
    whole_frame(&mut app, &mut pixels);
    assert_eq!(skin_text(&app, "written"), format!("{}x", &START_TEXT[..START_TEXT.len() - 2]));
    assert!(app.shared.skin_requests().is_empty(), "a function writes nowhere else");
}

/// While a text is typed into, no key reaches a shortcut: not the screen's, not the option overlay's
/// and not START or SELECT. The keys come back the moment the typing ends.
#[test]
fn a_key_typed_into_a_text_reaches_no_shortcut() {
    let (mut app, mut pixels) = app_with_typing_skin("keys");
    let mut stage = Stage::Select(Box::new(SelectState::new()));
    let sort_before = app.shared.config.library.sort;
    let press = |app: &mut crate::App, stage: &mut Stage, key: KeyInput<'_>| {
        stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, key);
    };

    press(&mut app, &mut stage, key_down(KeyCode::F3, None));
    let sort_after_f3 = app.shared.config.library.sort;
    assert_ne!(sort_after_f3, sort_before, "without a text to type into, F3 moves the ordering of the list");

    press(&mut app, &mut stage, key_down(START_KEY, Some("a")));
    assert!(app.shared.start_pressed(), "the test needs a key that is START when nothing is typed");
    press(&mut app, &mut stage, key_up(START_KEY));
    assert!(!app.shared.start_pressed());

    start_typing(&mut app, &mut pixels, over_search());
    press(&mut app, &mut stage, key_down(KeyCode::F3, None));
    assert_eq!(app.shared.config.library.sort, sort_after_f3, "a shortcut of the screen was run while a text was typed into");
    press(&mut app, &mut stage, key_down(START_KEY, Some("a")));
    assert!(!app.shared.start_pressed(), "the letter that is START held the panels open while it was typed");
    press(&mut app, &mut stage, key_up(START_KEY));
    assert!(app.shared.skin_text_is_focused(), "a key coming up does not end the typing");
    press(&mut app, &mut stage, key_down(KeyCode::Enter, None));
    assert!(!app.shared.skin_text_is_focused());

    press(&mut app, &mut stage, key_down(KeyCode::F3, None));
    assert_ne!(app.shared.config.library.sort, sort_after_f3, "the shortcut came back when the typing ended");
}

/// Dragging a volume slider sets the volume, is heard at once, and is written to the settings file once
/// the button is let go rather than at every step of the drag.
#[test]
fn a_volume_slider_sets_the_volume_and_the_settings_are_written_when_the_button_is_let_go() {
    let (mut app, mut pixels) = app_with_typing_skin("volume");
    let settings = app.shared.settings_path.clone();
    assert!(!settings.exists());
    let before = app.shared.config.audio.system;
    assert_ne!(before, VOLUME_QUARTER, "the test needs a volume to move");

    app.shared.mouse_held.on_button(MouseButton::Left, true);
    assert!(app.shared.skin_pointer(over_volume_quarter(), LEFT_PRESS));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(app.shared.config.audio.system, VOLUME_QUARTER, "the system volume went to the share the slider was pressed at");
    assert!(app.shared.skin_requests().is_empty(), "the volume was carried out, not left waiting");
    assert!(!settings.exists(), "the settings were written in the middle of a drag");

    app.shared.mouse_held.on_button(MouseButton::Left, false);
    whole_frame(&mut app, &mut pixels);
    assert!(settings.exists(), "the settings were not written once the button was let go");
    let saved = rbms_config::load(&settings).expect("the settings read back").config;
    assert_eq!(saved.audio.system, VOLUME_QUARTER);
}
