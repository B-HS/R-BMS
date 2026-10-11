//! What a screen does when its skin is not coming: the built-in layout draws it, and the player is
//! told once which skin failed and what is drawn instead -- whether the skin's file does not parse,
//! its body raises, runs past its budget, or is no longer where the settings say. And the screens
//! the default pack has no document for, which were never going to be drawn by a skin.

use std::path::PathBuf;

use rbms_model::Mode;
use rbms_skin::loader::{SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_PLAY_24KEYS_DOUBLE, mode_skin_type};

use super::*;
use crate::skin_select::LAUNCH_LAYOUT_FLAG;
use crate::stage::{Canvas, FrameCtx, HeadlessCanvas, KeyConfigState as KeyConfigStage, Stage};
use crate::toast::TOAST_TEXT_CHARS;

/// The file a pack's decide-screen skin is written to.
const DECIDE_SKIN_FILE: &str = "decide.luaskin";

/// Frames a screen is drawn for: far more than the two a Lua skin needs to be read and taken in, so
/// a failure said once a frame would show.
const FRAMES: usize = 12;

/// A Lua skin that is not Lua, and another that is not Lua either and is a different length, so its
/// file is seen to have been written again.
const UNPARSABLE: &str = "return {{{";
const UNPARSABLE_AGAIN: &str = "return {{{{{{";

/// A layout for the built-in note field a run is started with. It is never read here: only a run
/// that reaches the play screen reads it.
const FIELD_LAYOUT_FILE: &str = "field.ron";

/// What the loader says of a file that is not Lua.
const PARSE_REASON: &str = "unexpected symbol";

/// A Lua skin for the decide screen whose header answers and whose body never ends.
const ENDLESS: &str = "if skin_config then\n    while true do end\nend\nreturn { type = 6, name = 'Endless', w = 1280, h = 720 }\n";

/// What the loader says of a skin it cut off.
const BUDGET_REASON: &str = "exceeded its evaluation budget";

/// A document the settings name for the decide screen that is not on disk.
const GONE_FILE: &str = "gone.json";

/// The longest file name a document of the default pack has, which a message on screen has to have
/// room for.
const LONGEST_DEFAULT_DOCUMENT: &str = "/home/player/.config/rbms/skin/rbms-default/courseresult.luaskin";

/// A settings folder of the test's own, and a pack folder in it.
fn home_of(tag: &str) -> (PathBuf, PathBuf) {
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-fallback-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let pack = home.join("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    (home, pack)
}

/// An app whose skin pack holds one decide-screen skin.
fn app_with_decide_skin(tag: &str, source: &str) -> (crate::App, PathBuf) {
    let (home, pack) = home_of(tag);
    std::fs::write(pack.join(DECIDE_SKIN_FILE), source).expect("the skin is written");
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    (crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron")), pack)
}

/// Draw the decide screen the way its stage does, `FRAMES` times over, and answer whether any frame
/// was drawn by a document.
fn draw_decide(app: &mut crate::App, pixels: &mut HeadlessCanvas) -> bool {
    (0..FRAMES).fold(false, |drawn, _| {
        let mut canvas = Canvas::Headless(pixels);
        app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
        let chart = ChartMeta::default();
        app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() }) || drawn
    })
}

/// Every message said so far that is about a skin that did not load.
fn failures_said() -> Vec<String> {
    let mut said = Vec::new();
    crate::notify::drain(&mut said);
    said.into_iter().map(|(_, message)| message).filter(|message| message.contains(SKIN_FAILURE_NOTE)).collect()
}

/// The one message says which skin failed and what is drawn instead before it says why, so the
/// part of it a line on screen has room for is the part the player needs.
#[test]
fn the_failure_message_names_the_skin_and_the_fallback_before_the_reason() {
    let message = skin_failure_message(Some("/skins/pack/decide.luaskin"), SKIN_FAILURE_NOTE, "/skins/pack/decide.luaskin: unexpected symbol");
    assert_eq!(message, "skin decide.luaskin did not load, drawing the built-in screen: /skins/pack/decide.luaskin: unexpected symbol");
    assert!(skin_failure_message(None, SKIN_FAILURE_NOTE, "why").starts_with(&format!("skin {SKIN_FAILURE_UNNAMED}{SKIN_FAILURE_NOTE}")));

    let long = skin_failure_message(Some(LONGEST_DEFAULT_DOCUMENT), SKIN_FAILURE_NOTE, &"reason ".repeat(TOAST_TEXT_CHARS));
    let on_screen: String = long.chars().take(TOAST_TEXT_CHARS).collect();
    assert!(on_screen.contains("courseresult.luaskin") && on_screen.contains(SKIN_FAILURE_NOTE.trim_end()), "{on_screen:?}");
}

/// A Lua skin of the pack whose file does not parse cannot say which screen it draws, so no screen
/// is ever drawn with it. It is still reported, once, by the first screen drawn after the pack was
/// read; walking the folders again says nothing more until the file is written again.
#[test]
fn a_lua_skin_that_does_not_parse_is_reported_once_and_again_only_when_its_file_changes() {
    crate::notify::exclusive(|| {
        let (mut app, pack) = app_with_decide_skin("unparsable", UNPARSABLE);
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

        assert!(!draw_decide(&mut app, &mut pixels), "a skin that does not parse drew a frame");
        assert!(!app.shared.has_skin_document(SKIN_TYPE_DECIDE), "the screen stands aside for a skin that is not coming");
        let said = failures_said();
        assert_eq!(said.len(), 1, "the failure was not said exactly once: {said:?}");
        assert!(said[0].starts_with(&format!("skin {DECIDE_SKIN_FILE}{SKIN_FAILURE_NOTE}")) && !said[0].contains('\n'), "{:?}", said[0]);

        app.shared.rescan_skins();
        app.shared.rescan_skins_afresh();
        draw_decide(&mut app, &mut pixels);
        assert_eq!(failures_said(), Vec::<String>::new(), "a file nobody changed was reported again");

        std::fs::write(pack.join(DECIDE_SKIN_FILE), UNPARSABLE_AGAIN).expect("the skin is written again");
        app.shared.rescan_skins();
        draw_decide(&mut app, &mut pixels);
        assert_eq!(failures_said().len(), 1, "a skin that was written again and is still broken was not reported");
    });
}

/// A Lua skin of the pack that does not parse and that the player chose for its screen by hand is
/// reported once, by the screen it was chosen for, and not a second time as a skin of the pack that
/// no screen is drawn with.
#[test]
fn a_lua_skin_that_does_not_parse_and_was_chosen_by_hand_is_reported_once() {
    crate::notify::exclusive(|| {
        let (home, pack) = home_of("unparsable-by-hand");
        let document = pack.join(DECIDE_SKIN_FILE);
        std::fs::write(&document, UNPARSABLE).expect("the skin is written");
        let mut config = crate::Config::default();
        config.skin.pack = Some(pack.to_string_lossy().into_owned());
        config.skin.select(SKIN_TYPE_DECIDE, Some(document.to_string_lossy().into_owned()));
        let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

        assert!(!draw_decide(&mut app, &mut pixels), "a skin that does not parse drew a frame");
        let said = failures_said();
        assert_eq!(said.len(), 1, "the failure was not said exactly once: {said:?}");
        assert!(said[0].starts_with(&format!("skin {DECIDE_SKIN_FILE}{SKIN_FAILURE_NOTE}")), "{:?}", said[0]);
        assert!(said[0].contains(PARSE_REASON), "the one message does not say that the file does not parse: {:?}", said[0]);

        app.shared.rescan_skins();
        draw_decide(&mut app, &mut pixels);
        assert_eq!(failures_said(), Vec::<String>::new(), "a file nobody changed was reported again");
    });
}

/// A Lua skin whose body never ends is cut off by its budget, which is a failure like any other:
/// the built-in screen draws, and the one message says the skin ran past its budget.
#[test]
fn a_lua_skin_that_runs_past_its_budget_is_reported_once_and_leaves_the_built_in_screen_drawing() {
    crate::notify::exclusive(|| {
        let (mut app, _pack) = app_with_decide_skin("endless", ENDLESS);
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

        assert!(!draw_decide(&mut app, &mut pixels), "a skin that was cut off drew a frame");
        assert!(!app.shared.has_skin_document(SKIN_TYPE_DECIDE));
        let said = failures_said();
        assert_eq!(said.len(), 1, "the failure was not said exactly once: {said:?}");
        assert!(said[0].starts_with(&format!("skin {DECIDE_SKIN_FILE}{SKIN_FAILURE_NOTE}")) && said[0].contains(BUDGET_REASON), "{:?}", said[0]);

        app.shared.begin_skin_scene();
        draw_decide(&mut app, &mut pixels);
        assert_eq!(failures_said(), Vec::<String>::new(), "a new scene ran a skin that had already been cut off");
    });
}

/// A document the settings name for a screen that is no longer on disk: the built-in screen draws,
/// and the one message names the file that is gone.
#[test]
fn a_document_that_is_no_longer_there_is_reported_once_and_leaves_the_built_in_screen_drawing() {
    crate::notify::exclusive(|| {
        let (home, pack) = home_of("gone");
        let mut config = crate::Config::default();
        config.skin.folder = Some(pack.to_string_lossy().into_owned());
        config.skin.select(SKIN_TYPE_DECIDE, Some(pack.join(GONE_FILE).to_string_lossy().into_owned()));
        let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

        assert!(!draw_decide(&mut app, &mut pixels), "a document that is not there drew a frame");
        assert!(!app.shared.has_skin_document(SKIN_TYPE_DECIDE));
        assert!(app.shared.skin_failure(SKIN_TYPE_DECIDE).is_some());
        let said = failures_said();
        assert_eq!(said.len(), 1, "the failure was not said exactly once: {said:?}");
        assert!(said[0].starts_with(&format!("skin {GONE_FILE}{SKIN_FAILURE_NOTE}")), "{:?}", said[0]);
    });
}

/// A run started with a layout for the built-in note field (`--skin`) is played on that field in
/// every mode, though the default pack has a document for five of them: the layout the player named
/// for the run is not set aside by a pack they did not name. The screens that are not play screens
/// keep their documents, and the play documents come back once the layout is let go of.
#[test]
fn a_run_started_with_a_note_field_layout_is_played_on_the_built_in_field() {
    let (home, _pack) = home_of("launch-layout");
    let settings = home.join("settings.ron");
    assert!(crate::assets::install_default_pack(&settings).is_complete(), "the default pack did not install");
    let launch = crate::LaunchOptions { skin_path: Some(home.join(FIELD_LAYOUT_FILE).to_string_lossy().into_owned()), ..crate::LaunchOptions::default() };
    let mut app = crate::App::new(String::new(), crate::Config::default(), launch, settings);

    for mode in Mode::ALL {
        assert!(!app.shared.has_play_scene_for(*mode), "{} is played on a skin over the layout the run was started with", mode.name);
        let screen = mode_skin_type(*mode).expect("every mode the player offers has a play screen");
        assert!(!app.shared.skin_document_is_enabled(screen), "{} has a document to wait for", mode.name);
    }
    for screen in [SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_DECIDE, SKIN_TYPE_RESULT, SKIN_TYPE_COURSE_RESULT] {
        assert!(app.shared.skin_document_is_enabled(screen), "screen {screen} lost its document to a layout for the note field");
    }
    assert!(
        app.shared.skins.document_value(&play_tab(&app.shared.config)).contains(LAUNCH_LAYOUT_FLAG),
        "the SKIN row does not say why the built-in field draws"
    );

    app.shared.drop_launch_layout();
    for mode in Mode::ALL {
        assert!(app.shared.has_play_scene_for(*mode), "{} is not played on the default pack's skin once the layout is let go of", mode.name);
    }
}

/// The configuration with the skin menu on the seven-key play screen.
fn play_tab(config: &crate::Config) -> crate::Config {
    let mut config = config.clone();
    config.skin.screen = rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
    config
}

/// With the default pack installed and no pack named, the five key modes the pack has a document
/// for are played on a skin, and the keyboard modes and the key configuration screen are not: no
/// document is asked for, nothing is reported, and the key configuration screen is the frame an
/// app with no pack at all draws.
#[test]
fn the_keyboard_modes_and_the_key_configuration_screen_stay_built_in_under_the_default_pack() {
    crate::notify::exclusive(|| {
        let (home, _pack) = home_of("default-pack");
        let settings = home.join("settings.ron");
        assert!(crate::assets::install_default_pack(&settings).is_complete(), "the default pack did not install");
        let mut app = crate::App::new(String::new(), crate::Config::default(), crate::LaunchOptions::default(), settings);
        let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

        for mode in Mode::ALL {
            assert!(app.shared.has_play_scene_for(*mode), "{} is not played on the default pack's skin", mode.name);
        }
        assert_eq!(mode_skin_type(Mode::KEYBOARD_24K), Some(SKIN_TYPE_PLAY_24KEYS));
        assert!(!app.shared.has_play_scene_for(Mode::KEYBOARD_24K), "the keyboard mode is played on a skin the default pack does not have");
        for screen in [SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_PLAY_24KEYS_DOUBLE, SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_KEY_CONFIG] {
            for _ in 0..FRAMES {
                app.shared.prepare_skin(&mut Canvas::Headless(&mut pixels), screen);
            }
            assert!(!app.shared.has_skin_document(screen) && !app.shared.skin_is_loading(screen), "screen {screen} waits for a document");
            assert_eq!(app.shared.skin_failure(screen), None);
        }

        let key_config = |app: &mut crate::App| {
            let mut pixels = HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
            app.stage = Stage::KeyConfig(KeyConfigStage::new());
            for _ in 0..FRAMES {
                app.shared.hot.clear();
                let mut canvas = Canvas::Headless(&mut pixels);
                app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, &mut canvas);
                app.shared.finish_skin_frame(&mut canvas);
            }
            pixels
        };
        let under_default_pack = key_config(&mut app);
        let (bare_home, _) = home_of("no-pack");
        let mut bare = crate::App::new(String::new(), crate::Config::default(), crate::LaunchOptions::default(), bare_home.join("settings.ron"));
        assert!(under_default_pack.rgba() == key_config(&mut bare).rgba(), "the key configuration screen is not the built-in one under the default pack");
        assert_eq!(failures_said(), Vec::<String>::new(), "a screen with no document reported a failure");
    });
}
