//! The browser's search, walked through its states on a pack's own screen: a word typed after the
//! search key, what it found left on show with a panel and the option overlay called up over it,
//! and a word typed after a press on the pack's own search text and left on show from there.
//!
//! The first test runs wherever the tests do, on the default pack, which is in the repository and
//! has a search text: a press on that text starts the browser's search, the list is narrowed as the
//! word is typed, and the list the confirmed word leaves takes the option panel, the option overlay
//! and Escape like any list; the search key then types into the same text. It is drawn small,
//! because it holds the browser to what it does rather than to how it looks.
//!
//! The other two save frames for a person to look at and pass without drawing anything unless they
//! are asked to: the default pack with [`CAPTURE_DIR_ENV`] set, and the pack [`SKIN_PACK_ENV`]
//! names. Where a pack's search text is drawn is read out of its document, so any pack that has one
//! is walked through it and a pack that has none is walked through the rest.

use rbms_skin::dst::SkinRect;
use rbms_skin::loader::LoadedSkin;
use rbms_skin::property::generated::STRING_SEARCHWORD;
use winit::event::MouseButton;

use super::*;
use crate::pointer::PointerInput;
use crate::select_tree::SelectFolder;
use crate::stage::select::tests::entry;

/// The size the default pack's browser is drawn at where only what it does is held: 16 to 9 like
/// the pack and a third of its size.
const HELD_SIZE: (u32, u32) = (640, 360);

/// The charts the held browser lists, and the word typed into its search text with the one of them
/// it finds.
const HELD_SONGS: [&str; 5] = ["Aurora", "Borealis", "Cascade", "Drift", "Ember"];
const HELD_WORD: [(KeyCode, &str); 2] = [(KeyCode::KeyB, "b"), (KeyCode::KeyO, "o")];
const HELD_FOUND: &str = "Borealis";

/// The moment of the browser's scene the walk starts at, and how much later each state is drawn
/// than the keys that made it were read, so the timers a skin starts for a list that changed have
/// run before the frame that is kept.
const SEARCH_FROM_MS: i64 = 3_000;
const SEARCH_SETTLE_MS: i64 = 1_000;

/// The names the states are saved under, after the prefix of the pack.
const TYPED: &str = "search-typing";
const ON_SHOW: &str = "search-on-show";
const ON_SHOW_PANEL: &str = "search-on-show-panel1";
const ON_SHOW_OPTIONS: &str = "search-on-show-options";
const LEFT: &str = "search-left";
const TEXT_TYPED: &str = "search-skin-text-typing";
const TEXT_ON_SHOW: &str = "search-skin-text-on-show";

const LEFT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: true };

/// Where the search text of a browser document is drawn, in the document's own upward coordinates,
/// or `None` for a document that has none: a text that names the search word and is editable, or
/// has nowhere else to write, at the place its destination ends up.
fn search_text_of(skin: &LoadedSkin) -> Option<SkinRect> {
    let text = skin.def.text.iter().find(|text| text.reference == STRING_SEARCHWORD && (text.editable || text.event.is_none()))?;
    skin.destinations.iter().find(|named| named.id == text.id).and_then(|named| named.track.frames.last()).map(|frame| frame.rect)
}

/// The middle of `rect` of a document authored at `size`, as the cursor position the window reports.
fn cursor_over(rect: SkinRect, size: (u32, u32)) -> (f32, f32) {
    let (across, up) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    (across / size.0 as f32 * UI_SIZE.0 as f32, (size.1 as f32 - up) / size.1 as f32 * UI_SIZE.1 as f32)
}

/// One whole frame of the browser at `at_ms`: its update, its draw, and the frame's end, which is
/// what hands a skin's writes on to the browser.
fn whole_frame(app: &mut App, at_ms: i64, pixels: &mut HeadlessCanvas, deadline: Instant) -> Result<(), String> {
    browser_frame_at(app, at_ms, pixels, deadline)?;
    app.shared.finish_skin_frame(&mut Canvas::Headless(pixels));
    Ok(())
}

/// A key going down and coming back up, through the application's own route.
fn stroke(app: &mut App, key: KeyInput<'_>) {
    let code = key.code;
    press_key(app, key);
    press_key(app, key_up(code));
}

/// The titles of the rows the browser lists.
fn listed(app: &App) -> Vec<String> {
    let songs = app.shared.library.songs();
    app.shared
        .select_items
        .iter()
        .filter_map(|item| match item {
            crate::SelectItem::Song(index) => songs.get(*index).map(|entry| entry.title.clone()),
            crate::SelectItem::Folder { .. } | crate::SelectItem::Missing(_) | crate::SelectItem::Course(_) => None,
        })
        .collect()
}

/// Press the pack's search text, type `word` and confirm it, a whole frame after each step. Answers
/// whether the text took the press; `between` is run once the word is typed and before it is
/// confirmed. The press starts the browser's own search, which the pack's text draws the word of:
/// no text of the skin takes the keyboard for itself.
fn search_in_the_packs_text(
    app: &mut App,
    pixels: &mut HeadlessCanvas,
    at_ms: i64,
    word: &[(KeyCode, &'static str)],
    deadline: Instant,
    mut between: impl FnMut(&mut App, &mut HeadlessCanvas) -> Result<(), String>,
) -> Result<bool, String> {
    let authored = authored_size(app, SKIN_TYPE_MUSIC_SELECT).ok_or_else(|| NO_DOCUMENT.to_owned())?;
    let Some(text) = app.shared.skins.document(SKIN_TYPE_MUSIC_SELECT).and_then(search_text_of) else {
        return Ok(false);
    };
    if !app.shared.skin_pointer(cursor_over(text, authored), LEFT_PRESS) {
        return Ok(false);
    }
    whole_frame(app, at_ms, pixels, deadline)?;
    whole_frame(app, at_ms, pixels, deadline)?;
    if !app.shared.searching {
        return Err("the press on the pack's search text started no typing".to_owned());
    }
    for (code, letter) in word {
        stroke(app, typed_key(*code, letter));
    }
    whole_frame(app, at_ms, pixels, deadline)?;
    if app.shared.skin_text_is_focused() || !app.shared.searching {
        return Err("the word typed after a press on the pack's search text is not the browser's".to_owned());
    }
    between(app, pixels)?;
    stroke(app, key_down(KeyCode::Enter));
    whole_frame(app, at_ms, pixels, deadline)?;
    whole_frame(app, at_ms, pixels, deadline)?;
    Ok(true)
}

/// The default pack's own search text is the search box: a press on it starts the search, the list
/// is narrowed as the word is typed, and the list the confirmed word leaves is a list -- START calls
/// the first option panel up over it, F1 the option overlay, and Escape leaves the search. The
/// search key types into the same text, with the keys that are no text left to the list.
#[test]
fn a_word_typed_into_the_default_packs_search_text_is_on_show_and_the_list_stays_a_list() {
    let mut app = first_run_app("search-held");
    app.shared.library = Library::from_songs(HELD_SONGS.iter().map(|title| entry(title, "artist", "7")).collect());
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.config.library.sort = SortMode::Title;
    app.shared.config.library.preview = false;
    app.shared.audio_failed = true;
    app.shared.rebuild_select_items();
    app.stage = browser();
    let mut pixels = HeadlessCanvas::new(HELD_SIZE.0, HELD_SIZE.1);
    wait_for_scene_document(&mut app, SKIN_TYPE_MUSIC_SELECT, &mut Canvas::Headless(&mut pixels)).unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    whole_frame(&mut app, SEARCH_FROM_MS, &mut pixels, deadline).unwrap_or_else(|reason| panic!("not drawn: {reason}"));

    assert!(
        app.shared.skin_text_writing(SKIN_TYPE_MUSIC_SELECT, STRING_SEARCHWORD).is_some(),
        "the default pack has no search text for the browser to type into"
    );
    let mut narrowed = Vec::new();
    let typed = search_in_the_packs_text(&mut app, &mut pixels, SEARCH_FROM_MS, &HELD_WORD, deadline, |app, _| {
        narrowed = listed(app);
        Ok(())
    });
    assert_eq!(typed, Ok(true), "the default pack's search text was not typed into");
    assert_eq!(narrowed, [HELD_FOUND], "the list was not narrowed while the word was typed");
    assert!(!app.shared.searching, "Enter did not end the typing");
    assert_eq!(listed(&app), [HELD_FOUND], "the word is not on show");

    app.shared.note_key(&key_down(BROWSER_START_KEY));
    whole_frame(&mut app, SEARCH_FROM_MS, &mut pixels, deadline).unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(app.shared.skin_timers.is_on(timer_id::PANEL1_ON), "START called no option panel up over a list a search left on show");
    app.shared.note_key(&key_up(BROWSER_START_KEY));
    whole_frame(&mut app, SEARCH_FROM_MS, &mut pixels, deadline).unwrap_or_else(|reason| panic!("not drawn: {reason}"));

    stroke(&mut app, key_down(KeyCode::F1));
    assert!(app.shared.options.is_open(), "F1 opened no option overlay over a list a search left on show");
    stroke(&mut app, key_down(KeyCode::Escape));
    assert!(!app.shared.options.is_open());
    assert_eq!(listed(&app), [HELD_FOUND], "closing the option overlay left the search");

    stroke(&mut app, key_down(KeyCode::Escape));
    whole_frame(&mut app, SEARCH_FROM_MS, &mut pixels, deadline).unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(app.shared.search.is_empty(), "Escape did not leave the search");
    assert_eq!(listed(&app), HELD_SONGS, "leaving the search did not bring the whole list back");

    stroke(&mut app, typed_key(KeyCode::Slash, "/"));
    for (code, letter) in HELD_WORD {
        stroke(&mut app, typed_key(code, letter));
    }
    whole_frame(&mut app, SEARCH_FROM_MS, &mut pixels, deadline).unwrap_or_else(|reason| panic!("not drawn: {reason}"));
    assert!(app.shared.searching && !app.shared.skin_text_is_focused(), "the search key did not start the browser's search");
    assert_eq!(listed(&app), [HELD_FOUND], "the word typed after the search key did not narrow the list");
    stroke(&mut app, key_down(KeyCode::F1));
    assert!(app.shared.options.is_open() && !app.shared.searching, "F1 did not end the typing and open the option overlay");
    assert_eq!(listed(&app), [HELD_FOUND], "what was typed is not on show under the option overlay");
    assert_eq!(app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT), None);
}

/// Walk the browser `app` draws through its search and save a frame of each state under `prefix`.
/// Answers the names saved, or why the browser stopped drawing or a state was not reached.
fn capture_search(app: &mut App, prefix: &str) -> Result<Vec<String>, String> {
    let size = authored_size(app, SKIN_TYPE_MUSIC_SELECT).ok_or_else(|| NO_DOCUMENT.to_owned())?;
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.rebuild_select_items();
    let focus =
        app.shared.library.songs().iter().position(|entry| entry.title == BROWSER_FOCUS_TITLE).ok_or("the scan did not find the chart that names pictures")?;
    app.shared.sel = focus.checked_sub(1).ok_or("the chart that names pictures is the first of the list")?;
    let mut saved = capture_browser_view(app, &mut pixels, size, &format!("{prefix}-search-base"), SelectState::new())?;
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    let mut at_ms = SEARCH_FROM_MS;
    let mut shoot = |app: &mut App, pixels: &mut HeadlessCanvas, at_ms: &mut i64, state: &str| -> Result<(), String> {
        whole_frame(app, *at_ms, pixels, deadline)?;
        while !browser_is_at_rest(app) {
            whole_frame(app, *at_ms, pixels, deadline)?;
        }
        *at_ms += SEARCH_SETTLE_MS;
        browser_frame_at(app, *at_ms, pixels, deadline)?;
        draw_app_overlays(app, &mut Canvas::Headless(pixels));
        let name = format!("{prefix}-{state}");
        save(&name, CAPTURE_EXTENSION, size, pixels.rgba());
        saved.push(name);
        Ok(())
    };

    stroke(app, typed_key(KeyCode::Slash, "/"));
    for (code, letter) in OVERLAY_QUERY {
        stroke(app, typed_key(code, letter));
    }
    if !app.shared.searching {
        return Err("the search key opened no search box".to_owned());
    }
    shoot(app, &mut pixels, &mut at_ms, TYPED)?;

    stroke(app, key_down(KeyCode::Enter));
    if app.shared.searching || app.shared.search.is_empty() {
        return Err("Enter did not leave what the word found on show".to_owned());
    }
    shoot(app, &mut pixels, &mut at_ms, ON_SHOW)?;

    app.shared.note_key(&key_down(BROWSER_START_KEY));
    shoot(app, &mut pixels, &mut at_ms, ON_SHOW_PANEL)?;
    app.shared.note_key(&key_up(BROWSER_START_KEY));
    whole_frame(app, at_ms, &mut pixels, deadline)?;
    at_ms += SEARCH_SETTLE_MS;

    stroke(app, key_down(KeyCode::F1));
    if !app.shared.options.is_open() {
        return Err("F1 opened no option overlay over the list a search left on show".to_owned());
    }
    shoot(app, &mut pixels, &mut at_ms, ON_SHOW_OPTIONS)?;
    stroke(app, key_down(KeyCode::Escape));

    stroke(app, key_down(KeyCode::Escape));
    if !app.shared.search.is_empty() {
        return Err("Escape did not leave the search".to_owned());
    }
    shoot(app, &mut pixels, &mut at_ms, LEFT)?;

    let mut typed_shot = Ok(());
    let typed = search_in_the_packs_text(app, &mut pixels, at_ms, &OVERLAY_QUERY, deadline, |app, pixels| {
        typed_shot = shoot(app, pixels, &mut at_ms, TEXT_TYPED);
        Ok(())
    })?;
    typed_shot?;
    if !typed {
        println!("{prefix}: the pack's browser has no search text to type into");
        return Ok(saved);
    }
    if app.shared.searching || app.shared.search.is_empty() {
        return Err("the word confirmed in the pack's search text is not on show".to_owned());
    }
    shoot(app, &mut pixels, &mut at_ms, TEXT_ON_SHOW)?;
    Ok(saved)
}

/// The search of a first run's browser, which the default pack draws, saved state by state at the
/// size the pack was authored at.
///
/// Opt-in, because walking the browser at that size is seconds of drawing: without
/// [`CAPTURE_DIR_ENV`] this passes without drawing anything.
#[test]
fn the_search_of_a_first_run_is_captured_being_typed_and_on_show() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let mut app = with_browse_library(first_run_app("first-run-search"), "first-run-search").unwrap_or_else(|reason| panic!("first run: {reason}"));
    let saved = capture_search(&mut app, "first-run").unwrap_or_else(|reason| panic!("first run: {reason}"));
    println!("first run: captured {}", saved.join(", "));
    assert_eq!(app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT), None);
}

/// The search of the browser of a pack somebody else wrote, saved state by state.
///
/// Opt-in like the other captures of such a pack: without [`SKIN_PACK_ENV`] this passes without
/// drawing anything. With it, the pack's browser has to draw in every state of the search, and
/// neither the pack's folder nor the sample folder may be changed by it.
#[test]
fn the_search_of_a_skin_pack_named_by_the_environment_is_captured_being_typed_and_on_show() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join(BROWSER_SAMPLE_FOLDER);
    let before = (files_under(&pack), files_under(&samples));

    let outcome = browse_app(&pack, "pack-search").and_then(|mut app| capture_search(&mut app, "pack"));
    if let Some(saved) = drawn_or_passed_over("select search", outcome) {
        println!("select search: captured {}", saved.join(", "));
    }

    assert_eq!((files_under(&pack), files_under(&samples)), before, "capturing the pack's search changed a folder it only reads");
}
