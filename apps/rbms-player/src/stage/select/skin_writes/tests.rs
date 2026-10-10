use rbms_library::Library;
use rbms_skin::property::generated::{BUTTON_LNMODE, RATE_MASTERVOLUME, RATE_MUSICSELECT_POSITION, STRING_SEARCHWORD};

use super::super::tests::{app, entry};
use super::super::*;
use crate::skin_host::{Cluster, ClusterRequest};

fn browsing(titles: &[&str]) -> crate::App {
    let mut app = app();
    app.shared.library = Library::from_songs(titles.iter().map(|title| entry(title, "a", "5")).collect());
    app.shared.select_view = SelectView::AllSongs;
    app.shared.config.library.sort = SortMode::Title;
    app.shared.rebuild_select_items();
    app
}

fn titles(app: &crate::App) -> Vec<String> {
    app.shared
        .select_items
        .iter()
        .filter_map(|item| match item {
            SelectItem::Song(si) => Some(app.shared.library.songs()[*si].title.clone()),
            SelectItem::Folder { .. } => None,
        })
        .collect()
}

fn write_position(app: &mut crate::App, share: f32) {
    app.shared.skin_requests().push(Cluster::Select, ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, value: share });
}

fn write_word(app: &mut crate::App, word: &str) {
    app.shared.skin_requests().push(Cluster::Select, ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: word.to_string() });
}

#[test]
fn the_scrollbar_puts_the_cursor_on_the_row_its_share_of_the_list_falls_on() {
    let mut app = browsing(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]);
    let mut state = SelectState::new();
    state.record_modal = Some(0);

    write_position(&mut app, 0.5);
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.sel, 5);
    assert!(state.record_modal.is_none(), "moving the cursor closes the record modal, as a click on a row does");

    write_position(&mut app, 0.999);
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.sel, 9);

    for outside in [1.0, -0.5, f32::NAN] {
        write_position(&mut app, outside);
        state.carry_out_skin_writes(&mut app.shared);
        assert_eq!(app.shared.sel, 9, "a share of {outside} left the cursor where it was");
    }

    write_position(&mut app, 0.0);
    write_position(&mut app, 0.25);
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.sel, 2, "the writes are made in the order they came, so the last one stands");
    assert!(app.shared.skin_requests().is_empty(), "every write was taken");
}

#[test]
fn an_empty_list_and_an_empty_course_tab_have_no_row_to_put_the_cursor_on() {
    let mut app = browsing(&[]);
    let mut state = SelectState::new();
    write_position(&mut app, 0.5);
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.sel, 0);

    state.tab = SelectTab::Courses;
    write_position(&mut app, 0.5);
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.sel, 0, "a course tab with no courses took the write");
}

#[test]
fn a_write_the_browser_does_not_carry_out_is_left_where_it_is() {
    let mut app = browsing(&["a", "b"]);
    let mut state = SelectState::new();
    let event = ClusterRequest::Event { id: BUTTON_LNMODE, arg1: 1, arg2: 0 };
    let volume = ClusterRequest::WriteRate { id: RATE_MASTERVOLUME, value: 0.5 };
    app.shared.skin_requests().push(Cluster::Select, event.clone());
    app.shared.skin_requests().push(Cluster::Options, volume.clone());
    write_position(&mut app, 0.5);

    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.skin_requests().take(Cluster::Select), [event], "an event is for whoever answers it");
    assert_eq!(app.shared.skin_requests().take(Cluster::Options), [volume], "another cluster's request is not the browser's");
}

#[test]
fn a_word_confirmed_in_the_skins_search_box_runs_the_browsers_own_search() {
    let mut app = browsing(&["alpha", "beta", "bravo"]);
    let mut state = SelectState::new();
    app.shared.select_view = SelectView::Root;
    app.shared.rebuild_select_items();
    let root_rows = app.shared.select_items.len();

    write_word(&mut app, "br");
    state.carry_out_skin_writes(&mut app.shared);
    assert!(app.shared.searching, "the search box of the browser is open on the word");
    assert_eq!(app.shared.search, "br");
    assert!(app.shared.select_view == SelectView::AllSongs, "a search spans the whole library");
    assert_eq!(titles(&app), vec!["bravo".to_string()]);

    write_word(&mut app, "al");
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(titles(&app), vec!["alpha".to_string()], "a second word replaces the first");

    write_word(&mut app, "   ");
    write_word(&mut app, "");
    state.carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.search, "al", "a word with nothing in it searches for nothing (MusicSelector.search)");

    state.tab = SelectTab::Courses;
    write_word(&mut app, "be");
    state.carry_out_skin_writes(&mut app.shared);
    assert!(state.tab == SelectTab::Songs, "the search is over songs, so the song tab is brought up");
    assert_eq!(titles(&app), vec!["beta".to_string()]);

    let now = Instant::now();
    state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, super::super::tests::press(KeyCode::Escape));
    assert!(!app.shared.searching, "the box closes the way a typed search closes");
    assert!(app.shared.select_view == SelectView::Root, "and the browser is back in the folder the first search was made from");
    assert_eq!(app.shared.select_items.len(), root_rows);
}

#[test]
fn the_browsers_frame_carries_out_what_the_skin_wrote() {
    let mut app = browsing(&["a", "b", "c", "d"]);
    let mut state = SelectState::new();
    write_position(&mut app, 0.75);
    let now = Instant::now();
    state.update(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert_eq!(app.shared.sel, 3);
}

/// The folder the pack test saves what it drew in when a capture folder was asked for.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// How far into its scene the pack's browser is drawn: past the opening, so the layout has settled.
const PACK_SCENE: Duration = Duration::from_secs(5);

/// Longest the pack's browser is waited for.
const PACK_LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// The search box of a published browser skin, in the skin's own upward coordinates at the size it
/// was authored at: ModernChic puts its search text at 50, 895 and gives it 418 by 30.
const PACK_SEARCH_CENTER: (f32, f32) = (200.0, 910.0);

/// How many songs the pack's browser is given, and the middle of its scrollbar's travel: ModernChic
/// hangs the lamp from y = 873 and lets it fall 600, so halfway down is 573, at the lamp's x.
const PACK_SONGS: usize = 40;
const PACK_SCROLLBAR_MIDDLE: (f32, f32) = (1875.0, 573.0);

/// Saves a frame under the capture folder, when one was asked for.
fn save_frame(name: &str, pixels: &crate::stage::HeadlessCanvas, size: (u32, u32)) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    let path = std::path::Path::new(&folder).join(format!("{name}.png"));
    image::save_buffer(path, pixels.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

/// A browser skin somebody else wrote, named by the environment, has its search box typed into on the
/// real screen: pressed where the skin drew it, typed into, composed into, and drawn at each step.
/// It passes without doing anything when no pack is named.
#[test]
fn the_search_box_of_a_skin_pack_named_by_the_environment_is_typed_into() {
    use crate::stage::capture::{app_in, settings_of};
    use crate::stage::render_tests::render_on;
    use crate::stage::{Canvas, HeadlessCanvas, Stage};
    use rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT;
    use winit::event::{Ime, MouseButton};

    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(crate::skin_select::SKIN_PACK_ENV)) else {
        return;
    };
    let settings = settings_of("typing-pack");
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(1));
    app.shared.skins.rescan(&settings, &app.shared.config);
    let header = app.shared.skins.header_of(&app.shared.config, SKIN_TYPE_MUSIC_SELECT);
    let Some(header) = header else {
        println!("the pack has no browser document");
        return;
    };
    let size = (u32::try_from(header.width).unwrap_or(1920), u32::try_from(header.height).unwrap_or(1080));
    let titles: Vec<String> = (0..PACK_SONGS).map(|at| format!("song {at:02}")).collect();
    app.shared.library = Library::from_songs(titles.iter().map(|title| entry(title, "artist", "7")).collect());
    app.shared.select_view = SelectView::AllSongs;
    app.shared.config.library.sort = SortMode::Title;
    app.shared.rebuild_select_items();
    let browser = || Stage::Select(Box::new(SelectState::new()));
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    render_on(&mut app, browser(), &mut Canvas::Headless(&mut pixels));
    while !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        if let Some(reason) = app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT) {
            println!("the pack's browser document is not drawn: {reason}");
            return;
        }
        assert!(Instant::now() < deadline, "the pack's browser document was not read in time");
        render_on(&mut app, browser(), &mut Canvas::Headless(&mut pixels));
    }
    app.shared.age_skin_scene(PACK_SCENE);
    let frame = |app: &mut crate::App, pixels: &mut HeadlessCanvas| {
        let mut canvas = Canvas::Headless(pixels);
        render_on(app, browser(), &mut canvas);
        app.shared.finish_skin_frame(&mut canvas);
    };
    frame(&mut app, &mut pixels);
    let search_box = |pixels: &HeadlessCanvas| -> Vec<crate::Color> {
        let (left, top) = (50, size.1 - 895 - 30);
        (top..top + 30).flat_map(|y| (left..left + 418).map(move |x| (x, y))).map(|(x, y)| pixels.pixel_at(x, y)).collect()
    };
    save_frame("typing-select-idle", &pixels, size);
    let idle = search_box(&pixels);

    let ui = crate::stage::canvas::UI_SIZE;
    let at = (PACK_SEARCH_CENTER.0 / size.0 as f32 * ui.0 as f32, (size.1 as f32 - PACK_SEARCH_CENTER.1) / size.1 as f32 * ui.1 as f32);
    let press = crate::pointer::PointerInput::Button { button: MouseButton::Left, pressed: true };
    assert!(app.shared.skin_pointer(at, press), "the skin's search box did not take the press");
    frame(&mut app, &mut pixels);
    frame(&mut app, &mut pixels);
    assert!(app.shared.skin_text_is_focused(), "the press did not start any typing");
    save_frame("typing-select-focused", &pixels, size);
    let focused = search_box(&pixels);
    assert_ne!(focused, idle, "an empty search box being typed into shows its caret");

    for (code, text) in [(KeyCode::KeyA, "a"), (KeyCode::KeyB, "b"), (KeyCode::KeyC, "c")] {
        assert!(app.shared.skin_text_key(&crate::stage::KeyInput { code, pressed: true, released: false, text: Some(text) }));
    }
    app.shared.skin_text_ime(&Ime::Preedit("한".to_string(), Some((0, "한".len()))));
    frame(&mut app, &mut pixels);
    save_frame("typing-select-composing", &pixels, size);
    let composing = search_box(&pixels);
    assert_ne!(composing, focused);

    app.shared.skin_text_ime(&Ime::Preedit(String::new(), None));
    app.shared.skin_text_ime(&Ime::Commit("한".to_string()));
    frame(&mut app, &mut pixels);
    save_frame("typing-select-typed", &pixels, size);
    assert_ne!(search_box(&pixels), composing, "the committed syllable is no longer underlined");

    assert!(app.shared.skin_text_key(&crate::stage::KeyInput { code: KeyCode::Enter, pressed: true, released: false, text: None }));
    frame(&mut app, &mut pixels);
    assert_eq!(
        app.shared.skin_requests().take(Cluster::Select),
        [ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: "abc한".to_string() }],
        "Enter writes what was typed to the browser"
    );
    assert_eq!(search_box(&pixels), idle, "the search word reads empty again once the typing is over");

    let ui_point = |(x, y): (f32, f32)| (x / size.0 as f32 * ui.0 as f32, (size.1 as f32 - y) / size.1 as f32 * ui.1 as f32);
    assert!(app.shared.skin_pointer(ui_point(PACK_SCROLLBAR_MIDDLE), press), "the skin's scrollbar did not take the press");
    frame(&mut app, &mut pixels);
    save_frame("scrollbar-select-middle", &pixels, size);
    let writes =
        app.shared.skin_requests().take_if(Cluster::Select, |request| matches!(request, ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, .. }));
    let [ClusterRequest::WriteRate { value, .. }] = writes.as_slice() else {
        panic!("the scrollbar wrote {writes:?}");
    };
    assert!((value - 0.5).abs() < 0.05, "a press halfway down the travel wrote {value}");
    write_position(&mut app, *value);
    SelectState::new().carry_out_skin_writes(&mut app.shared);
    assert_eq!(app.shared.sel, PACK_SONGS / 2, "the cursor went to the row halfway down the list");
}
