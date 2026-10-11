//! The search of the browser, run on a browser that is really drawn: what a search word being typed
//! takes of the keyboard and what it leaves alone, and that a list a search left on show is driven
//! like any other list.
//!
//! A search is put on show here without any typing, by a word written to the browser whole, which
//! is what a writer of a skin does. The list it leaves is then held, operation by operation, to the
//! same list reached with no search at all
//! ([`OPERATIONS`]): every title of the fixture holds the word that is searched for, so the two
//! browsers list the same rows and anything one does that the other does not is the search's doing.

use rbms_library::SongEntry;
use rbms_render::theme;
use rbms_skin::property::generated::{
    BUTTON_AUTOPLAY, BUTTON_FAVORITTE_CHART, BUTTON_LNMODE, BUTTON_MODE, BUTTON_PRACTICE, BUTTON_SORT, STRING_DIRECTORY, STRING_SEARCHWORD,
};
use winit::event::{Ime, MouseButton};
use winit::keyboard::KeyCode;

use super::overlay::{SEARCH_H, STRIP_BOTTOM, STRIP_X, TAB_MARK_H};
use super::skinned_tests::{
    Browser, COMPILE_WAIT, DOCUMENT_SIZE, KEY_ONE, SELECT, SHORT_STEP, SLOT_BELOW, START, TURNTABLE_FORWARD, slot_middle, wheel_document,
};
use super::tests::{entry, press, release};
use super::*;
use crate::keyconfig::Bindable;
use crate::pointer::PointerInput;
use crate::skin_host::{Cluster, ClusterRequest};
use crate::stage::capture::skin_folder_of;

/// The charts every browser here lists, in title order, and the word all of their titles hold.
const TITLES: [&str; 4] = ["alba", "alder", "alpha", "altair"];
const WORD: &str = "al";

/// The md5 the fixture gives the first of them.
const FIRST_MD5: &str = "md5-alba";

/// The lane of the controller key that is key one, and of its turntable.
const PAD_KEY_ONE: usize = 0;
const PAD_TURNTABLE: usize = 7;

/// The key that calls the detail panel up by itself.
const DETAIL_KEY: KeyCode = KeyCode::Digit5;

/// How far inside the corner of the application's search box a pixel is read, clear of its outline.
const INSIDE: u32 = 4;

/// The argument a button of a skin runs its event with.
const NEXT: i32 = 1;

/// The font file of the fixture that draws the path of the open folders, and where and how large
/// that text is, in the fixture skin's own coordinates.
const FACE_FILE: &str = "face.ttf";
const PATH_TEXT: (u32, u32, u32, u32) = (4, 160, 90, 12);

fn songs() -> Vec<SongEntry> {
    TITLES.map(|title| entry(title, "a", "5")).to_vec()
}

/// A browser the fixture skin draws over the charts above, or one nothing but itself draws.
fn browser(tag: &str, skinned: bool) -> Browser {
    if skinned {
        return Browser::skinned(tag, songs());
    }
    let mut browser = Browser::over(tag, songs(), false);
    browser.frame();
    browser.frame();
    browser
}

/// A key going down that types `text`, as the window reports a key with a character on it.
fn typed(code: KeyCode, text: &'static str) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: Some(text) }
}

/// One key handed to the browser the way the application hands it over: the held keys, then the
/// option overlay, then the browser itself.
fn route(browser: &mut Browser, key: KeyInput<'_>) -> Transition {
    browser.app.stage = Stage::Select(Box::new(std::mem::take(&mut browser.state)));
    let asked = browser.app.stage.handle_key(&mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }, key);
    if let Stage::Select(state) = std::mem::replace(&mut browser.app.stage, Stage::Select(Box::new(SelectState::new()))) {
        browser.state = *state;
    }
    asked
}

/// A key pressed and let go again through that route, with the character it types if it types one.
fn stroke(browser: &mut Browser, code: KeyCode, text: Option<&'static str>) -> Transition {
    let asked = route(browser, KeyInput { code, pressed: true, released: false, text });
    route(browser, release(code));
    asked
}

/// The search box opened with its key and `word` typed into it a letter at a time.
fn type_a_search(browser: &mut Browser, word: &'static str) {
    stroke(browser, KeyCode::Slash, Some("/"));
    for index in 0..word.len() {
        route(browser, typed(KeyCode::KeyL, &word[index..=index]));
    }
    browser.frame_after(SHORT_STEP);
}

/// `word` written to the browser by the skin, whole, and the frame that carries the write out.
fn confirm_in_the_skins_box(browser: &mut Browser, word: &str) {
    browser.app.shared.skin_requests().push(Cluster::Select, ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: word.to_owned() });
    browser.frame_after(SHORT_STEP);
}

/// A button of the skin pressed: the event it runs, and the frame that carries it out.
fn skin_button(browser: &mut Browser, cluster: Cluster, id: i32) -> Transition {
    browser.app.shared.skin_requests().push(cluster, ClusterRequest::Event { id, arg1: NEXT, arg2: 0 });
    browser.frame_after(SHORT_STEP)
}

fn titles(browser: &Browser) -> Vec<String> {
    let songs = browser.app.shared.library.songs();
    browser
        .app
        .shared
        .select_items
        .iter()
        .map(|item| match item {
            SelectItem::Song(index) => songs[*index].title.clone(),
            SelectItem::Folder { label, .. } => label.clone(),
        })
        .collect()
}

/// Where a key or a press sent the browser.
fn went(asked: &Transition) -> &'static str {
    match asked {
        Transition::Stay => "stays",
        Transition::Open(Stage::Loading(_)) => "starts the chart",
        Transition::Open(Stage::Settings(_)) => "opens the settings",
        Transition::Open(Stage::Folders(_)) => "opens the folders",
        Transition::Open(Stage::Tables(_)) => "opens the tables",
        Transition::Open(_) => "opens a screen",
        Transition::To(_) => "goes to a screen",
        Transition::Back => "goes back",
        Transition::Quit => "quits",
    }
}

/// Everything an operation can leave behind on the browser, as one line.
fn left_behind(browser: &Browser, asked: &Transition) -> String {
    let shared = &browser.app.shared;
    let state = &browser.state;
    let filter = state.filter.filter(&shared.config);
    format!(
        "{}; row {} of {}; order {:?}; mode {}; long notes {:?}; panel {}; random {:?}; filter panel {}; ranking {}; records {}; option overlay {}; guide \
         {}; starred {}; practice {}; plays itself {}; typing {}",
        went(asked),
        shared.sel,
        shared.select_items.len(),
        shared.config.library.sort,
        filter.mode.map_or("all", |mode| mode.name),
        shared.config.judge.ln_mode,
        state.panel.number(),
        shared.config.play.random,
        state.filter.is_open(),
        state.ranking_open,
        state.record_modal.is_some(),
        shared.options.is_open(),
        state.guide_open,
        shared.favorites.contains(FIRST_MD5),
        shared.practice_requested,
        shared.run_plays_itself(),
        shared.searching,
    )
}

/// The keys of a panel held for the frame that puts it up.
fn hold_for_a_frame(browser: &mut Browser, keys: &[KeyCode]) -> Transition {
    for key in keys {
        browser.hold(*key);
    }
    browser.frame_after(SHORT_STEP)
}

/// One key of a controller going down, and the frame that reads it.
fn pad_press(browser: &mut Browser, lane: usize) -> Transition {
    match browser.pad(lane) {
        Transition::Stay => browser.frame_after(SHORT_STEP),
        asked => asked,
    }
}

/// One thing a player does on a list, by the name the table of differences prints it under.
type Operation = (&'static str, fn(&mut Browser) -> Transition);

/// Everything a list is driven with that is the same on a list a search left as on any other: the
/// option panels and a key of one, the reference's number keys, the browser's own keys, the option
/// overlay, the keys of the key table, the mouse, a controller, and the buttons of a skin.
const OPERATIONS: [Operation; 34] = [
    ("START held: panel 1", |browser| hold_for_a_frame(browser, &[START])),
    ("SELECT held: panel 2", |browser| hold_for_a_frame(browser, &[SELECT])),
    ("START and SELECT held: panel 3", |browser| hold_for_a_frame(browser, &[START, SELECT])),
    ("5 held: panel 3", |browser| hold_for_a_frame(browser, &[DETAIL_KEY])),
    ("START held and key one: random option", |browser| {
        hold_for_a_frame(browser, &[START]);
        browser.tap(KEY_ONE)
    }),
    ("1: mode filter", |browser| stroke(browser, KeyCode::Digit1, Some("1"))),
    ("2: order", |browser| stroke(browser, KeyCode::Digit2, Some("2"))),
    ("3: long note mode", |browser| stroke(browser, KeyCode::Digit3, Some("3"))),
    ("4: replay slot", |browser| stroke(browser, KeyCode::Digit4, Some("4"))),
    ("F1: option overlay", |browser| stroke(browser, KeyCode::F1, None)),
    ("left shift held: option overlay", |browser| route(browser, press(KeyCode::ShiftLeft))),
    ("F2: filter panel", |browser| stroke(browser, KeyCode::F2, None)),
    ("F3: order", |browser| stroke(browser, KeyCode::F3, None)),
    ("F4: practice", |browser| stroke(browser, KeyCode::F4, None)),
    ("F9: favourite", |browser| stroke(browser, KeyCode::F9, None)),
    ("TAB: settings", |browser| stroke(browser, KeyCode::Tab, Some("\t"))),
    ("O: folders", |browser| stroke(browser, KeyCode::KeyO, Some("o"))),
    ("T: tables", |browser| stroke(browser, KeyCode::KeyT, Some("t"))),
    ("R: records", |browser| stroke(browser, KeyCode::KeyR, Some("r"))),
    ("I: ranking", |browser| stroke(browser, KeyCode::KeyI, Some("i"))),
    ("F: favourite", |browser| stroke(browser, KeyCode::KeyF, Some("f"))),
    ("H: key guide", |browser| stroke(browser, KeyCode::KeyH, Some("h"))),
    ("down arrow: next bar", |browser| {
        browser.arrow(KeyCode::ArrowDown);
        Transition::Stay
    }),
    ("ENTER: play", |browser| stroke(browser, KeyCode::Enter, Some("\r"))),
    ("key one: play", |browser| hold_for_a_frame(browser, &[KEY_ONE])),
    ("turntable: next bar", |browser| hold_for_a_frame(browser, &[TURNTABLE_FORWARD])),
    ("mouse wheel: next bar", |browser| browser.scroll(1.0)),
    ("press on a bar: play", |browser| {
        browser.pointer(slot_middle(SLOT_BELOW), PointerInput::Button { button: MouseButton::Left, pressed: true });
        browser.frame_after(SHORT_STEP)
    }),
    ("controller key one: play", |browser| pad_press(browser, PAD_KEY_ONE)),
    ("controller turntable: next bar", |browser| pad_press(browser, PAD_TURNTABLE)),
    ("skin button: mode filter", |browser| skin_button(browser, Cluster::Select, BUTTON_MODE)),
    ("skin button: order", |browser| skin_button(browser, Cluster::Select, BUTTON_SORT)),
    ("skin button: long note mode", |browser| skin_button(browser, Cluster::Select, BUTTON_LNMODE)),
    ("skin button: autoplay", |browser| skin_button(browser, Cluster::Select, BUTTON_AUTOPLAY)),
];

/// The buttons of a skin that are held to the same thing in a test of their own, because the list
/// above is as long as one line of differences can usefully be.
const MORE_SKIN_BUTTONS: [(&str, i32); 2] = [("practice", BUTTON_PRACTICE), ("favourite", BUTTON_FAVORITTE_CHART)];

/// A list a search left on show is the list it would be with no search, to every key, every press
/// of the mouse, a controller and the buttons of a skin: each operation leaves on it exactly what
/// it leaves on a list no search made. The differences are printed as a table, so a failure says
/// which operations a search breaks and how.
#[test]
fn a_list_a_search_left_on_show_is_driven_like_any_other_list() {
    let mut differences = Vec::new();
    for (index, (name, operation)) in OPERATIONS.iter().enumerate() {
        let mut plain = browser(&format!("search-plain-{index}"), true);
        let mut searched = browser(&format!("search-shown-{index}"), true);
        confirm_in_the_skins_box(&mut searched, WORD);
        assert_eq!(titles(&searched), titles(&plain), "the search for {WORD:?} did not leave the fixture's whole list");

        let on_plain = operation(&mut plain);
        let on_searched = operation(&mut searched);
        let (expected, got) = (left_behind(&plain, &on_plain), left_behind(&searched, &on_searched));
        if expected != got {
            differences.push(format!("{name}\n    plain list:  {expected}\n    search list: {got}"));
        }
    }
    assert!(differences.is_empty(), "{} of {} operations differ on a search list:\n{}", differences.len(), OPERATIONS.len(), differences.join("\n"));
}

/// The two buttons of a skin the table above has no room for.
#[test]
fn the_other_buttons_of_a_skin_work_on_a_list_a_search_left_on_show() {
    for (name, id) in MORE_SKIN_BUTTONS {
        let mut plain = browser(&format!("search-button-plain-{name}"), true);
        let mut searched = browser(&format!("search-button-shown-{name}"), true);
        confirm_in_the_skins_box(&mut searched, WORD);
        let on_plain = skin_button(&mut plain, Cluster::Select, id);
        let on_searched = skin_button(&mut searched, Cluster::Select, id);
        assert_eq!(left_behind(&searched, &on_searched), left_behind(&plain, &on_plain), "the {name} button of a skin");
    }
}

/// The same holds for a browser no skin draws, for the keys it has.
#[test]
fn a_built_in_list_a_search_left_on_show_takes_its_own_keys() {
    let keys: [(KeyCode, Option<&'static str>); 9] = [
        (KeyCode::F2, None),
        (KeyCode::F3, None),
        (KeyCode::F4, None),
        (KeyCode::Tab, Some("\t")),
        (KeyCode::KeyO, Some("o")),
        (KeyCode::KeyT, Some("t")),
        (KeyCode::KeyI, Some("i")),
        (KeyCode::KeyF, Some("f")),
        (KeyCode::Enter, Some("\r")),
    ];
    for (index, (code, text)) in keys.into_iter().enumerate() {
        let mut plain = browser(&format!("search-built-in-plain-{index}"), false);
        let mut searched = browser(&format!("search-built-in-shown-{index}"), false);
        confirm_in_the_skins_box(&mut searched, WORD);
        let on_plain = stroke(&mut plain, code, text);
        let on_searched = stroke(&mut searched, code, text);
        assert_eq!(left_behind(&searched, &on_searched), left_behind(&plain, &on_plain), "{code:?}");
    }

    let mut searched = browser("search-built-in-pad", false);
    confirm_in_the_skins_box(&mut searched, WORD);
    searched.pad(PAD_TURNTABLE);
    assert_eq!(searched.sel(), 1, "the turntable of a controller did not move a list a search left on show");
}

/// Enter ends the typing and leaves what was found on show; it does not start the chart the cursor
/// happens to be on. The Enter after it does.
#[test]
fn enter_confirms_the_word_and_the_enter_after_it_plays() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-enter-{skinned}"), skinned);
        type_a_search(&mut browser, "alp");
        assert!(browser.app.shared.searching, "the search box is being typed into");

        let confirmed = stroke(&mut browser, KeyCode::Enter, Some("\r"));
        assert_eq!(went(&confirmed), "stays", "Enter started a chart out of a search box that was being typed into");
        assert!(!browser.app.shared.searching, "Enter left the search box open");
        assert_eq!(browser.app.shared.search, "alp", "confirming the word threw it away");
        assert_eq!(titles(&browser), ["alpha"], "the list is no longer what the word found");

        assert_eq!(went(&stroke(&mut browser, KeyCode::Enter, Some("\r"))), "starts the chart", "the list on show does not play its chart");
    }
}

/// A word with nothing in it is no search: confirming it puts the browser back where it was.
#[test]
fn confirming_an_empty_word_is_the_same_as_cancelling() {
    let mut browser = browser("search-empty", true);
    browser.app.shared.sel = 2;
    type_a_search(&mut browser, "");
    stroke(&mut browser, KeyCode::Enter, Some("\r"));
    assert!(!browser.app.shared.searching);
    assert_eq!((titles(&browser).len(), browser.sel()), (TITLES.len(), 2), "the browser is not back on the row the search was opened on");
    assert!(matches!(stroke(&mut browser, KeyCode::Escape, None), Transition::Stay));
    assert!(browser.app.shared.select_view == SelectView::Root, "Escape after an empty search did not go up a folder as it does on any list");
}

/// Escape on a list a search left on show leaves the search the way leaving a folder does: back to
/// the folder and the row the search was made from, with the word gone. It never arms the quit.
#[test]
fn escape_leaves_a_search_on_show_for_where_it_was_made_from() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-escape-{skinned}"), skinned);
        browser.app.shared.select_view = SelectView::Root;
        browser.app.shared.sel = 0;
        browser.app.shared.rebuild_select_items();
        browser.frame_after(SHORT_STEP);
        let root = titles(&browser);

        confirm_in_the_skins_box(&mut browser, "alp");
        assert_eq!(titles(&browser), ["alpha"]);
        assert!(matches!(stroke(&mut browser, KeyCode::Escape, None), Transition::Stay));
        assert!(browser.app.shared.select_view == SelectView::Root, "the browser stayed in the list the search ran over");
        assert_eq!(titles(&browser), root);
        assert!(browser.app.shared.search.is_empty(), "the word outlived the search");
        assert!(!browser.state.esc_quit_armed(), "leaving a search armed the quit");
    }
}

/// While a word is typed every key with a character on it is the word's: the letters the lanes,
/// START and SELECT are on, the digits the reference's browser gives a meaning, and the space.
/// None of them does what it does on the list.
#[test]
fn a_key_with_a_character_on_it_goes_into_the_word_and_nowhere_else() {
    let mut browser = browser("search-letters", true);
    let before = (browser.app.shared.config.library.sort, browser.app.shared.config.judge.ln_mode);
    stroke(&mut browser, KeyCode::Slash, Some("/"));
    let letters: [(KeyCode, &'static str); 11] = [
        (KeyCode::KeyA, "a"),
        (KeyCode::KeyW, "w"),
        (KeyCode::KeyZ, "z"),
        (KeyCode::KeyF, "f"),
        (KeyCode::KeyO, "o"),
        (KeyCode::KeyH, "h"),
        (KeyCode::Digit1, "1"),
        (KeyCode::Digit3, "3"),
        (KeyCode::Digit5, "5"),
        (KeyCode::Digit0, "0"),
        (KeyCode::Space, " "),
    ];
    for (code, text) in letters {
        route(&mut browser, typed(code, text));
        assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay), "{code:?} left the browser while a word was typed");
        assert_eq!(browser.state.panel.number(), 0, "{code:?} called a panel up while a word was typed");
        route(&mut browser, release(code));
    }
    assert_eq!(browser.app.shared.search, "awzfoh1350 ", "a key with a character on it did not reach the word");
    assert!(browser.app.shared.searching);
    assert_eq!((browser.app.shared.config.library.sort, browser.app.shared.config.judge.ln_mode), before, "a digit stepped a setting");
    assert!(browser.state.filter.filter(&browser.app.shared.config).mode.is_none(), "1 stepped the mode filter");
    assert!(!browser.state.guide_open && !browser.app.shared.favorites.contains(FIRST_MD5));

    route(&mut browser, press(KeyCode::Backspace));
    route(&mut browser, press(KeyCode::ShiftLeft));
    assert_eq!(browser.app.shared.search, "awzfoh1350", "Backspace is the word's");
    assert!(!browser.app.shared.options.is_open(), "the shift key of a capital letter opened the option overlay");
    assert!(browser.app.shared.searching, "a shift key ended the typing");
}

/// A key that opens something over the list or instead of it ends the typing, with what was typed
/// left on show, and then does what it does on any list.
#[test]
fn a_key_that_opens_something_ends_the_typing_and_opens_it() {
    let mut overlay = browser("search-typing-f1", true);
    type_a_search(&mut overlay, "alp");
    stroke(&mut overlay, KeyCode::F1, None);
    assert!(overlay.app.shared.options.is_open(), "F1 did not open the option overlay over a search being typed");
    assert!(!overlay.app.shared.searching, "the search box stayed open under the option overlay");
    assert_eq!((overlay.app.shared.search.as_str(), titles(&overlay)), ("alp", vec!["alpha".to_owned()]), "what was typed is not on show");

    let mut filter = browser("search-typing-f2", true);
    type_a_search(&mut filter, "alp");
    stroke(&mut filter, KeyCode::F2, None);
    assert!(filter.state.filter.is_open(), "F2 did not open the filter panel over a search being typed");
    assert!(!filter.app.shared.searching);

    let mut settings = browser("search-typing-tab", true);
    type_a_search(&mut settings, "alp");
    assert_eq!(went(&stroke(&mut settings, KeyCode::Tab, Some("\t"))), "opens the settings", "TAB did not open the settings from a search being typed");
    assert!(!settings.app.shared.searching);
    assert_eq!(titles(&settings), ["alpha"], "the list the settings are closed back onto is not what the word found");

    let mut practice = browser("search-typing-f4", true);
    type_a_search(&mut practice, "alp");
    assert_eq!(went(&stroke(&mut practice, KeyCode::F4, None)), "starts the chart");
    assert!(practice.app.shared.practice_requested && !practice.app.shared.searching);
}

/// What moves or reorders the list does so under a word being typed, and the typing goes on: the
/// two arrows, the wheel of the mouse, the sort key, and a button of the skin.
#[test]
fn the_list_is_moved_and_ordered_under_a_word_being_typed() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-typing-moves-{skinned}"), skinned);
        type_a_search(&mut browser, "al");
        assert_eq!(browser.arrow(KeyCode::ArrowDown), 1, "the down arrow did not move the list under the search box");
        assert_eq!(browser.arrow(KeyCode::ArrowUp), 0, "the up arrow did not move the list under the search box");

        let order = browser.app.shared.config.library.sort;
        stroke(&mut browser, KeyCode::F3, None);
        assert_eq!(browser.app.shared.config.library.sort, order.next(), "F3 did not step the order under the search box");
        assert!(browser.app.shared.searching, "moving the list ended the typing");
        assert_eq!(browser.app.shared.search, "al");
    }

    let mut browser = browser("search-typing-wheel", true);
    type_a_search(&mut browser, "al");
    browser.scroll(1.0);
    assert_eq!(browser.sel(), 1, "the wheel of the mouse did not move the list under the search box");
    let long_notes = browser.app.shared.config.judge.ln_mode;
    skin_button(&mut browser, Cluster::Select, BUTTON_LNMODE);
    assert!(browser.app.shared.config.judge.ln_mode != long_notes, "a button of the skin did nothing under the search box");
    assert!(browser.app.shared.searching);
}

/// A controller is not a keyboard: a key of one ends the typing, with what was typed left on show,
/// and does what it does on any list.
#[test]
fn a_key_of_a_controller_ends_the_typing_and_does_what_it_does() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-typing-pad-{skinned}"), skinned);
        type_a_search(&mut browser, "al");
        pad_press(&mut browser, PAD_TURNTABLE);
        assert!(!browser.app.shared.searching, "the search box stayed open under a controller");
        assert_eq!(browser.sel(), 1, "the turntable of a controller did not move the list");
        assert_eq!(browser.app.shared.search, "al", "what was typed is not on show");
    }
}

/// Escape throws the word away and puts the browser back where the search was opened, as it always
/// has.
#[test]
fn escape_cancels_a_word_being_typed() {
    let mut browser = browser("search-typing-escape", true);
    browser.app.shared.sel = 3;
    type_a_search(&mut browser, "alp");
    stroke(&mut browser, KeyCode::Escape, None);
    assert!(!browser.app.shared.searching && browser.app.shared.search.is_empty());
    assert_eq!((titles(&browser).len(), browser.sel()), (TITLES.len(), 3));
}

/// A word the skin wrote to the browser whole is on show at once: no search box is opened for it,
/// and the browser does not take every key for it.
#[test]
fn a_word_confirmed_in_the_skins_search_text_opens_no_second_search_box() {
    let mut browser = browser("search-one-box", true);
    browser.frame_after(SHORT_STEP);
    let inside = (STRIP_X as u32 + INSIDE, STRIP_BOTTOM as u32 - SEARCH_H as u32 + INSIDE);
    let before = browser.pixels.pixel_at(inside.0, inside.1);

    confirm_in_the_skins_box(&mut browser, "alp");
    browser.frame_after(SHORT_STEP);
    assert_eq!(titles(&browser), ["alpha"]);
    assert!(!browser.app.shared.searching, "the application's search box was opened for a word the skin's box confirmed");
    assert_eq!(browser.pixels.pixel_at(inside.0, inside.1), before, "the application's search box was drawn beside the skin's");
    let ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
    assert!(!browser.state.holds_keys(&ctx), "a list a search left on show takes every key for itself");

    let mark = (STRIP_X as u32 + INSIDE, STRIP_BOTTOM as u32 - TAB_MARK_H as u32 + INSIDE);
    assert_eq!(browser.pixels.pixel_at(mark.0, mark.1), theme().button_active, "nothing on the screen says the list is what a search found");
    stroke(&mut browser, KeyCode::Escape, None);
    browser.frame_after(SHORT_STEP);
    assert_ne!(browser.pixels.pixel_at(mark.0, mark.1), theme().button_active, "the mark of the search outlived the search");
}

/// Pressing the search key over a search that is on show takes the word up again to be changed,
/// rather than starting from nothing, and cancelling still goes back to where the first search was
/// made from.
#[test]
fn the_search_key_takes_a_word_on_show_up_again() {
    let mut browser = browser("search-again", true);
    browser.app.shared.select_view = SelectView::Root;
    browser.app.shared.rebuild_select_items();
    browser.frame_after(SHORT_STEP);
    confirm_in_the_skins_box(&mut browser, "al");

    stroke(&mut browser, KeyCode::Slash, Some("/"));
    assert!(browser.app.shared.searching);
    assert_eq!(browser.app.shared.search, "al", "the word on show was thrown away");
    route(&mut browser, typed(KeyCode::KeyP, "p"));
    assert_eq!(titles(&browser), ["alpha"]);
    stroke(&mut browser, KeyCode::Escape, None);
    assert!(browser.app.shared.select_view == SelectView::Root, "cancelling the second word did not go back to where the first search was made from");
}

/// Leaving the browser for another screen ends the typing, and a screen that put the browser back
/// at the top of the library took the search with it.
#[test]
fn leaving_the_browser_ends_the_typing_and_a_reset_list_drops_the_search() {
    let mut browser = browser("search-leave", true);
    type_a_search(&mut browser, "alp");
    let mut ctx = FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 };
    browser.state.on_exit(&mut ctx);
    assert!(!browser.app.shared.searching, "the search box was left open under another screen");
    assert_eq!(browser.app.shared.search, "alp");

    browser.come_back();
    browser.frame_after(SHORT_STEP);
    assert_eq!(titles(&browser), ["alpha"], "coming back from a screen that left the list alone lost the search");

    browser.app.shared.select_view = SelectView::Root;
    browser.app.shared.rebuild_select_items();
    browser.come_back();
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.search.is_empty(), "a word outlived the list it was searched over");
    assert!(matches!(stroke(&mut browser, KeyCode::Escape, None), Transition::Stay));
    assert!(browser.app.shared.select_view == SelectView::Root && browser.state.esc_quit_armed(), "Escape at the top went looking for a search to leave");
}

/// The fixture skin with a text that draws the path of the open folders added to it, drawing a
/// browser over the charts above.
fn browser_that_draws_the_open_folders(tag: &str) -> Browser {
    browser_that_draws_a_text(tag, STRING_DIRECTORY)
}

/// The fixture skin with a text of the string `reference` added to it, drawing a browser over the
/// charts above.
fn browser_that_draws_a_text(tag: &str, reference: i32) -> Browser {
    let (x, y, w, h) = PATH_TEXT;
    let texts = format!(
        r#""font": [{{ "id": "face", "path": "{FACE_FILE}" }}], "text": [{{ "id": "path", "font": "face", "size": {h}, "ref": {reference} }}], "imageset":"#
    );
    let places = format!(r#""destination": [{{ "id": "wheel" }}, {{ "id": "path", "dst": [{{ "x": {x}, "y": {y}, "w": {w}, "h": {h} }}] }}]"#);
    let document = wheel_document().replace(r#""imageset":"#, &texts).replace(r#""destination": [{ "id": "wheel" }]"#, &places);
    assert!(document.contains(r#""id": "path", "dst""#), "the fixture's document is not laid out the way this one is made from it");
    let mut browser = Browser::drawn_by(tag, songs(), Some(document));
    let face = skin_folder_of(&browser.app.shared.settings_path).join(FACE_FILE);
    std::fs::write(face, include_bytes!("../../../../../assets/fonts/Inter-Regular.ttf")).expect("the font is written");
    let deadline = Instant::now() + COMPILE_WAIT;
    while !browser.app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        browser.frame();
        assert!(Instant::now() < deadline, "the fixture skin never compiled: {:?}", browser.app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT));
    }
    browser.frame();
    browser
}

/// A skin that draws the path of the open folders names the list on show itself, so the
/// application's mark of a search is not put over it. The search is on all the same.
#[test]
fn the_mark_of_a_search_is_left_out_over_a_skin_that_draws_the_open_folders() {
    let mut browser = browser_that_draws_the_open_folders("search-no-mark");
    browser.frame_after(SHORT_STEP);
    let mark = (STRIP_X as u32 + INSIDE, STRIP_BOTTOM as u32 - TAB_MARK_H as u32 + INSIDE);
    let before = browser.pixels.pixel_at(mark.0, mark.1);

    confirm_in_the_skins_box(&mut browser, "alp");
    browser.frame_after(SHORT_STEP);
    assert_eq!(titles(&browser), ["alpha"]);
    assert!(browser.state.search_is_on_show(&browser.app.shared));
    assert_eq!(browser.pixels.pixel_at(mark.0, mark.1), before, "the application marked a search over a skin that names the list on show");
}

/// The fixture is what the comparisons above take it for: the word finds every chart of it.
#[test]
fn the_fixture_lists_every_chart_for_the_word() {
    let mut browser = browser("search-fixture", true);
    confirm_in_the_skins_box(&mut browser, WORD);
    assert_eq!(titles(&browser), TITLES);
}

/// The pixels the fixture's added text is drawn over, of the frame drawn last.
fn text_pixels(browser: &Browser) -> Vec<Color> {
    let (x, y, w, h) = PATH_TEXT;
    let scale = CW / DOCUMENT_SIZE.0;
    let top = (DOCUMENT_SIZE.1 - y - h) * scale;
    (top..top + h * scale)
        .flat_map(|row| (x * scale..(x + w) * scale).map(move |column| (column, row)))
        .map(|(column, row)| browser.pixels.pixel_at(column, row))
        .collect()
}

/// The cursor, in the space the window's events arrive in, at the middle of the fixture's added text.
fn text_middle() -> (f32, f32) {
    let (x, y, w, h) = PATH_TEXT;
    let across = (x + w / 2) as f32 / DOCUMENT_SIZE.0 as f32;
    let up = (y + h / 2) as f32 / DOCUMENT_SIZE.1 as f32;
    (across * CW as f32, (1.0 - up) * CH as f32)
}

/// The report this was written for: on a skin with a search text of its own the search key opened
/// the application's search box as well, over the skin, and left the skin's text empty. The skin's
/// text is the search box: what is typed is drawn there, and no box of the application's is.
#[test]
fn the_search_key_types_into_the_skins_own_search_text_and_opens_no_second_box() {
    let mut browser = browser_that_draws_a_text("search-key-skin-text", STRING_SEARCHWORD);
    browser.frame_after(SHORT_STEP);
    let inside = (STRIP_X as u32 + INSIDE, STRIP_BOTTOM as u32 - SEARCH_H as u32 + INSIDE);
    let (no_box, empty) = (browser.pixels.pixel_at(inside.0, inside.1), text_pixels(&browser));

    type_a_search(&mut browser, "alp");
    assert!(browser.app.shared.searching, "the search key opened no search");
    assert_eq!(titles(&browser), ["alpha"], "the list is not narrowed as the word is typed");
    assert_eq!(browser.pixels.pixel_at(inside.0, inside.1), no_box, "the application's search box was drawn over a skin that has a search text");
    let typed = text_pixels(&browser);
    assert_ne!(typed, empty, "what is typed is not drawn in the skin's search text");

    stroke(&mut browser, KeyCode::Enter, None);
    browser.frame_after(SHORT_STEP);
    assert!(browser.state.search_is_on_show(&browser.app.shared));
    assert_eq!(text_pixels(&browser), empty, "the skin's search text still shows a word being typed once the typing is over");
}

/// A press on the skin's search text starts the same search the search key does, held to the same
/// rules: the list is narrowed with every letter, the keys that are no text go on to the list, and a
/// key that opens something ends the typing first.
#[test]
fn a_press_on_the_skins_search_text_starts_the_search_the_search_key_does() {
    let mut browser = browser_that_draws_a_text("search-press-skin-text", STRING_SEARCHWORD);
    browser.frame_after(SHORT_STEP);
    browser.pointer(text_middle(), PointerInput::Button { button: MouseButton::Left, pressed: true });
    browser.frame_after(SHORT_STEP);
    assert!(browser.app.shared.searching, "the press on the skin's search text started no search");

    route(&mut browser, typed(KeyCode::KeyA, "a"));
    route(&mut browser, typed(KeyCode::KeyL, "l"));
    browser.frame_after(SHORT_STEP);
    assert_eq!(
        (browser.app.shared.search.as_str(), titles(&browser).len()),
        ("al", TITLES.len()),
        "a letter typed into the skin's search text did not reach the list"
    );
    assert_eq!(browser.arrow(KeyCode::ArrowDown), 1, "the down arrow did not move the list under the skin's search text");
    route(&mut browser, typed(KeyCode::KeyP, "p"));
    assert_eq!(titles(&browser), ["alpha"]);

    stroke(&mut browser, KeyCode::F1, None);
    assert!(browser.app.shared.options.is_open(), "F1 did not open the option overlay over the skin's search text being typed into");
    assert!(!browser.app.shared.searching, "the typing went on under the option overlay");
    assert_eq!(titles(&browser), ["alpha"], "what was typed is not on show");
}

/// A skin that draws the path of the open folders says a search is on show the way the reference
/// does, with the folder it files a search under.
#[test]
fn the_path_of_the_open_folders_names_a_search_that_is_on_show() {
    let mut browser = browser("search-path", true);
    let path = |browser: &Browser| browser.app.shared.select_shown(None, &crate::skin_host::select::BrowserLent::default()).directory;
    assert_eq!(path(&browser), ["ALL SONGS"]);
    confirm_in_the_skins_box(&mut browser, " alp ");
    assert_eq!(path(&browser), ["Search : 'alp'"], "nothing a skin can draw says the list is what a search found");
    stroke(&mut browser, KeyCode::Escape, None);
    assert_eq!(path(&browser), ["ALL SONGS"]);
}

/// One report of the window's input method handed to the browser the way the application hands it
/// over, with whether the application would have asked the window for such reports at all.
fn compose(browser: &mut Browser, ime: Ime) -> bool {
    browser.app.stage = Stage::Select(Box::new(std::mem::take(&mut browser.state)));
    let wanted = browser.app.stage.takes_composed_text(&browser.app.shared);
    browser.app.stage.handle_ime(&mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }, &ime);
    if let Stage::Select(state) = std::mem::replace(&mut browser.app.stage, Stage::Select(Box::new(SelectState::new()))) {
        browser.state = *state;
    }
    wanted
}

/// The search word takes what an input method composes, which is how a title in Hangul or kana is
/// searched for at all: what is being composed is shown and is not yet the word, what is committed
/// is typed and narrows the list, and nothing is taken while no word is being typed.
#[test]
fn an_input_method_composes_into_the_word_being_typed() {
    const SYLLABLE: &str = "\u{c54c}";
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-ime-{skinned}"), skinned);
        assert!(!compose(&mut browser, Ime::Commit("alp".to_owned())), "the window was asked for composed text with no word being typed");
        assert!(browser.app.shared.search.is_empty() && !browser.state.search_is_on(&browser.app.shared), "text was taken with no word being typed");

        stroke(&mut browser, KeyCode::Slash, Some("/"));
        assert!(
            compose(&mut browser, Ime::Preedit(SYLLABLE.to_owned(), Some((0, SYLLABLE.len())))),
            "the window is not asked for composed text while a word is typed"
        );
        assert!(browser.app.shared.search.is_empty(), "a syllable still being composed narrowed the list");
        assert_eq!(titles(&browser).len(), TITLES.len());
        assert!(browser.state.search_display().starts_with(SYLLABLE), "what is being composed is not shown in the search box");

        compose(&mut browser, Ime::Preedit(String::new(), None));
        compose(&mut browser, Ime::Commit("alp".to_owned()));
        assert_eq!(
            (browser.app.shared.search.as_str(), titles(&browser)),
            ("alp", vec!["alpha".to_owned()]),
            "what the input method committed did not narrow the list"
        );

        compose(&mut browser, Ime::Preedit(SYLLABLE.to_owned(), None));
        stroke(&mut browser, KeyCode::Enter, None);
        assert_eq!(browser.app.shared.search, "alp", "a syllable never committed became part of the word");
        assert!(browser.state.search_is_on_show(&browser.app.shared));
    }
}

/// On a skin with a search text what an input method is composing is drawn in that text, like the
/// rest of the word.
#[test]
fn what_an_input_method_composes_is_drawn_in_the_skins_search_text() {
    let mut browser = browser_that_draws_a_text("search-ime-skin-text", STRING_SEARCHWORD);
    stroke(&mut browser, KeyCode::Slash, Some("/"));
    browser.frame_after(SHORT_STEP);
    let caret_only = text_pixels(&browser);
    compose(&mut browser, Ime::Preedit("al".to_owned(), Some((0, 2))));
    browser.frame_after(SHORT_STEP);
    assert_ne!(text_pixels(&browser), caret_only, "what is being composed is not drawn in the skin's search text");
    assert!(browser.app.shared.search.is_empty());
}

/// One controller event on the browser.
fn pad_event(browser: &mut Browser, event: PadEvent) -> Transition {
    browser.state.handle_pad(&mut FrameCtx { shared: &mut browser.app.shared, now: browser.now, dt: 0.0 }, event)
}

/// Give `shortcut` the key `code` in the browser's key configuration.
fn move_shortcut(browser: &mut Browser, shortcut: Shortcut, code: KeyCode) {
    let mode = browser.app.shared.mode;
    browser.app.shared.keyconfig.bind(mode, Bindable::Shortcut(shortcut), Some(code));
}

/// The rule that a key which is text is the word's holds for a shortcut the player moved: one put on
/// a letter types that letter while a word is typed and does what it does once the typing is over,
/// and the key it shipped on is nothing any more.
#[test]
fn a_shortcut_moved_onto_a_letter_is_a_letter_of_the_word_while_one_is_typed() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-moved-letter-{skinned}"), skinned);
        move_shortcut(&mut browser, Shortcut::Sort, KeyCode::KeyG);
        move_shortcut(&mut browser, Shortcut::Folders, KeyCode::KeyP);
        type_a_search(&mut browser, "a");
        let order = browser.app.shared.config.library.sort;

        assert!(matches!(stroke(&mut browser, KeyCode::KeyG, Some("g")), Transition::Stay), "skinned {skinned}");
        assert!(matches!(stroke(&mut browser, KeyCode::KeyP, Some("p")), Transition::Stay), "skinned {skinned}: a letter left the browser");
        assert!(browser.app.shared.searching, "skinned {skinned}: a letter ended the typing");
        assert_eq!(browser.app.shared.search, "agp", "skinned {skinned}: the letters did not reach the word");
        assert_eq!(browser.app.shared.config.library.sort, order, "skinned {skinned}: a letter of the word reordered the list");

        stroke(&mut browser, KeyCode::F3, None);
        assert_eq!(browser.app.shared.config.library.sort, order, "skinned {skinned}: the key the order shipped on still steps it");
        assert!(browser.app.shared.searching);

        stroke(&mut browser, KeyCode::Escape, None);
        stroke(&mut browser, KeyCode::KeyG, Some("g"));
        assert_ne!(browser.app.shared.config.library.sort, order, "skinned {skinned}: the moved key does not step the order on a list");
        assert!(matches!(stroke(&mut browser, KeyCode::KeyP, Some("p")), Transition::Open(Stage::Folders(_))), "skinned {skinned}");
    }
}

/// A shortcut moved onto a key that is not text is read under the box by the rule its job falls
/// under: one that reorders the list does so and the typing goes on, and one that opens something
/// ends the typing first and then opens it.
#[test]
fn a_shortcut_moved_onto_a_key_that_is_not_text_is_read_under_the_box() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-moved-other-{skinned}"), skinned);
        move_shortcut(&mut browser, Shortcut::Sort, KeyCode::F7);
        move_shortcut(&mut browser, Shortcut::Filter, KeyCode::F6);
        move_shortcut(&mut browser, Shortcut::Folders, KeyCode::Insert);
        type_a_search(&mut browser, WORD);
        let order = browser.app.shared.config.library.sort;

        stroke(&mut browser, KeyCode::F7, None);
        assert_ne!(browser.app.shared.config.library.sort, order, "skinned {skinned}: the moved order key does nothing under the box");
        assert!(browser.app.shared.searching, "skinned {skinned}: reordering ended the typing");

        stroke(&mut browser, KeyCode::F2, None);
        assert!(browser.app.shared.searching && !browser.state.filter.is_open(), "skinned {skinned}: the key the filter shipped on still opens it");
        stroke(&mut browser, KeyCode::F6, None);
        assert!(!browser.app.shared.searching, "skinned {skinned}: the filter opened over a box still being typed into");
        assert!(browser.state.filter.is_open(), "skinned {skinned}: the moved filter key did not open the panel");
        assert_eq!(browser.app.shared.search, WORD, "skinned {skinned}: what was typed was not left on show");
        stroke(&mut browser, KeyCode::F6, None);
        assert!(!browser.state.filter.is_open(), "skinned {skinned}: the panel did not close on the key that opened it");

        stroke(&mut browser, KeyCode::Slash, Some("/"));
        assert!(browser.app.shared.searching);
        assert!(matches!(stroke(&mut browser, KeyCode::Insert, None), Transition::Open(Stage::Folders(_))), "skinned {skinned}");
        assert!(!browser.app.shared.searching, "skinned {skinned}: the browser was left with the box still open");
    }
}

/// A controller button bound to a shortcut does what the key does, ends the typing of a word first
/// as any key of a controller does, and is nothing while a panel that takes every key is up.
#[test]
fn a_controller_button_bound_to_a_shortcut_does_what_its_key_does() {
    for skinned in [true, false] {
        let mut browser = browser(&format!("search-pad-shortcut-{skinned}"), skinned);
        let order = browser.app.shared.config.library.sort;
        assert!(matches!(pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Sort)), Transition::Stay));
        assert_ne!(browser.app.shared.config.library.sort, order, "skinned {skinned}");
        assert!(matches!(pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Folders)), Transition::Open(Stage::Folders(_))), "skinned {skinned}");
        assert!(matches!(pad_event(&mut browser, PadEvent::Shortcut(Shortcut::KeyConfig)), Transition::Open(Stage::KeyConfig(_))), "skinned {skinned}");
        assert!(
            matches!(pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Retry)), Transition::Stay),
            "skinned {skinned}: a result key acted on the browser"
        );

        type_a_search(&mut browser, WORD);
        assert!(matches!(pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Tables)), Transition::Open(Stage::Tables(_))), "skinned {skinned}");
        assert!(!browser.app.shared.searching, "skinned {skinned}: a controller button left the box open");

        pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Filter));
        assert!(browser.state.filter.is_open(), "skinned {skinned}");
        assert!(
            matches!(pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Folders)), Transition::Stay),
            "skinned {skinned}: a button reached the list under the panel"
        );
        pad_event(&mut browser, PadEvent::Shortcut(Shortcut::Filter));
        assert!(!browser.state.filter.is_open(), "skinned {skinned}: the panel did not close on its own button");
    }
}
