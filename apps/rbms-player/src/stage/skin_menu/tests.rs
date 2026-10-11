//! The skin menu as a screen: what it lists and how the list is moved through, that every row
//! changes the stored choice the settings screen's row of the same name changed, how the screen
//! being configured is stepped, what the mouse and the wheel do, and how the menu is left.
//!
//! The preview on the right is another module's. What is held here about it is where it goes, that
//! the menu fills its place when it says it drew nothing, and that a press or a turn of the wheel
//! over it changes nothing. A frame of a skin is never asserted on.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, the frames these tests draw are also saved there as PNGs
//! a person can open. With [`SKIN_PACK_ENV`] naming a pack as well, the menu is drawn over that pack.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rbms_skin::loader::{SKIN_TYPE_PLAY_24KEYS, is_supported_skin_type};

use super::*;
use crate::pointer::PointerInput;
use crate::skin_preview::PreviewAbsence;
use crate::skin_select::{OffsetAxis, SKIN_PACK_ENV, SkinLibrary, fixtures, pack_from_environment};
use crate::stage::render_tests::{FRAME_DT, app};
use crate::stage::{HeadlessCanvas, StageId};
use crate::{App, CH};

/// Environment variable naming the folder captures are saved in, the one the screen capture harness
/// reads. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The skin pack this repository ships, relative to the player's manifest.
const DEFAULT_PACK: &str = "../../assets/skins/rbms-default";

/// The size the menu is laid out for, and a window half as large again.
const UI: (u32, u32) = (CW, CH);
const LARGE: (u32, u32) = (1920, 1080);

/// How many options the long document declares, which with the fixed rows is more than the list
/// shows at once.
const LONG_OPTIONS: usize = 30;

/// The first option id the long document gives its items, which are numbered up from it in twos.
const LONG_FIRST_OP: usize = 900;

/// How many frames a test waits for its preview to say something other than that the document is on
/// its way, and how long it pauses between two of them.
const SETTLE_FRAMES: usize = 600;
const SETTLE_PAUSE: Duration = Duration::from_millis(10);

/// The room a tab's name is kept from the tab's own edges.
const KIND_NAME_ROOM: f32 = 8.0;

/// How tall a line of what is said under the preview is drawn, which is the room its last line needs
/// above the panel's foot.
const INFO_LINE_H: f32 = 14.0;

fn press(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: None }
}

fn release(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: true, text: None }
}

/// A folder picker nobody will ever answer, in place of the native one.
fn unanswered_picker() -> DialogHandle {
    crate::dialog::handle_for_tests().1
}

/// An app whose settings live in a folder of this test's own, with no skin chosen and none
/// installed beside the settings file.
fn bare_app(tag: &str) -> App {
    let mut app = app();
    let root = std::env::temp_dir().join(format!("rbms-skin-menu-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the fixture folder is writable");
    app.shared.settings_path = root.join("settings.ron");
    app.shared.config.skin = rbms_config::SkinOptions::default();
    app.shared.skins = SkinLibrary::new(&app.shared.settings_path, &app.shared.config);
    app
}

/// The same app with a pack named as its pack folder.
fn app_with_pack(tag: &str, pack: &Path) -> App {
    let mut app = bare_app(tag);
    assert!(pack.is_dir(), "the skin pack is not at {}", pack.display());
    app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    app
}

fn shipped_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK)
}

/// An app whose skin folder holds the fixture documents, configuring the song browser, which is the
/// screen the fixture with a row of every kind draws.
fn app_with_fixtures(tag: &str) -> App {
    let mut app = bare_app(tag);
    let folder = fixtures::write(&app.shared.settings_path);
    app.shared.config.skin.folder = Some(folder.to_string_lossy().into_owned());
    app.shared.config.skin.screen = fixtures::MUSIC_SELECT;
    app
}

/// A document for the song browser that declares more options than the list shows at once, and an
/// offset the player may nudge along two axes.
fn long_document() -> String {
    let options: Vec<String> = (0..LONG_OPTIONS)
        .map(|at| {
            let op = LONG_FIRST_OP + at * 2;
            format!(
                r#"{{ "category": "Group", "name": "Option {:02}", "item": [{{ "name": "First", "op": {op} }}, {{ "name": "Second", "op": {} }}], "def": "First" }}"#,
                at + 1,
                op + 1
            )
        })
        .collect();
    format!(
        r#"{{ "type": 5, "name": "Long", "author": "dj", "w": 1280, "h": 720, "property": [{}],
            "offset": [{{ "category": "Group", "name": "Judge", "id": 12, "x": true, "y": true }}], "destination": [] }}"#,
        options.join(",")
    )
}

/// An app whose song browser is drawn with [`long_document`], chosen by hand.
fn app_with_long_document(tag: &str) -> App {
    let mut app = bare_app(tag);
    let folder = app.shared.settings_path.parent().expect("the settings file has a folder").join("skin");
    std::fs::create_dir_all(&folder).expect("the skin folder is writable");
    let document = folder.join("long.json");
    std::fs::write(&document, long_document()).expect("the document is written");
    app.shared.config.skin.screen = fixtures::MUSIC_SELECT;
    app.shared.config.skin.select(fixtures::MUSIC_SELECT, Some(document.to_string_lossy().into_owned()));
    app
}

/// The menu over an app, entered, with the one canvas all of its frames are drawn on: a compiled
/// skin holds textures of the target it was compiled for, so a preview drawn over several frames has
/// to be drawn on the same one.
struct Menu {
    app: App,
    state: SkinMenuState,
    pixels: HeadlessCanvas,
    size: (u32, u32),
}

impl Menu {
    fn over(app: App) -> Menu {
        Menu::sized(app, UI)
    }

    fn sized(mut app: App, size: (u32, u32)) -> Menu {
        let mut state = SkinMenuState::new();
        state.open_pack_picker = unanswered_picker;
        state.on_enter(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
        Menu { app, state, pixels: HeadlessCanvas::new(size.0, size.1), size }
    }

    /// One frame, as the application makes one: the screen is brought up to date and drawn, and the
    /// skin frame is finished.
    fn frame(&mut self) -> &HeadlessCanvas {
        self.app.shared.hot.clear();
        let now = std::time::Instant::now();
        self.state.update(&mut FrameCtx { shared: &mut self.app.shared, now, dt: FRAME_DT });
        let mut canvas = Canvas::Headless(&mut self.pixels);
        self.state.draw(&mut FrameCtx { shared: &mut self.app.shared, now, dt: FRAME_DT }, &mut canvas);
        self.app.shared.finish_skin_frame(&mut canvas);
        &self.pixels
    }

    fn key(&mut self, code: KeyCode) -> Transition {
        self.state.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, press(code))
    }

    fn key_up(&mut self, code: KeyCode) -> Transition {
        self.state.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, release(code))
    }

    /// A press of the left button at a point of the screen, judged against the frame drawn last.
    fn click(&mut self, at: (f32, f32)) -> Transition {
        self.state.handle_mouse(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, at, MouseButton::Left, true)
    }

    /// A turn of the wheel with the pointer at a point of the screen.
    fn wheel(&mut self, at: (f32, f32), lines: f32) {
        self.app.shared.cursor = at;
        self.state.handle_scroll(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, lines);
    }

    fn at(&self, row: SettingRow) -> usize {
        self.state.rows.iter().position(|entry| *entry == row).unwrap_or_else(|| panic!("{row:?} is not in the list"))
    }

    /// Put the cursor on one row of the list.
    fn focus(&mut self, row: SettingRow) {
        let at = self.at(row);
        self.state.focus(at);
    }

    fn focus_fixed(&mut self, id: SettingId) {
        self.focus(SettingRow::Fixed(id));
    }

    /// The value the list shows for one row.
    fn value_of(&self, row: SettingRow) -> String {
        self.state.lines[self.at(row)].value.clone()
    }

    fn value_of_fixed(&self, id: SettingId) -> String {
        self.value_of(SettingRow::Fixed(id))
    }

    fn labels(&self) -> Vec<String> {
        self.state.lines.iter().map(|line| line.label.clone()).collect()
    }

    /// Choose the first document on offer for the screen being configured, the way the row does.
    fn choose_first_document(&mut self) {
        self.focus_fixed(SettingId::SkinDocument);
        self.key(KeyCode::ArrowRight);
    }

    /// The middle of what the frame drawn last made clickable for `hot`.
    fn middle_of_hot(&self, hot: Hot) -> (f32, f32) {
        let (rect, _) = self.app.shared.hot.iter().find(|(_, entry)| *entry == hot).unwrap_or_else(|| panic!("{hot:?} is not on the screen"));
        middle(*rect)
    }

    /// The rows the frame drawn last made clickable, top to bottom.
    fn rows_on_show(&self) -> Vec<usize> {
        self.app
            .shared
            .hot
            .iter()
            .filter_map(|(_, hot)| match hot {
                Hot::SkinMenuRow(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    /// Save the frame drawn last as `<capture folder>/<name>.png`, when a capture folder was named.
    /// A frame larger than the size the menu is laid out for says so in its name.
    fn save(&self, name: &str) -> Option<PathBuf> {
        let folder = PathBuf::from(std::env::var_os(CAPTURE_DIR_ENV)?);
        std::fs::create_dir_all(&folder).expect("the capture folder can be created");
        let suffix = if self.size == UI { String::new() } else { format!("-{}x{}", self.size.0, self.size.1) };
        let path = folder.join(format!("{name}{suffix}.png"));
        image::save_buffer(&path, self.pixels.rgba(), self.size.0, self.size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
        Some(path)
    }
}

fn capturing() -> bool {
    std::env::var_os(CAPTURE_DIR_ENV).is_some()
}

fn middle(rect: Rect) -> (f32, f32) {
    (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

/// The pixel at a point of the screen the menu is laid out for.
fn pixel(pixels: &HeadlessCanvas, at: (f32, f32)) -> Color {
    pixels.pixel_at(at.0 as u32, at.1 as u32)
}

/// The fixed rows of the list, in the order the settings table lists them.
const FIXED_ROWS: [SettingId; 6] =
    [SettingId::SkinPack, SettingId::SkinScreen, SettingId::SkinDocument, SettingId::SkinInfo, SettingId::SkinReload, SettingId::SkinReset];

#[test]
fn the_preview_is_the_shape_of_a_screen_and_everything_is_on_the_screen() {
    assert!((PREVIEW_RECT.w * CH as f32 - PREVIEW_RECT.h * CW as f32).abs() < f32::EPSILON, "the preview is not the shape of the screen");
    assert_eq!((PREVIEW_RECT.x, PREVIEW_RECT.y, PREVIEW_RECT.w, PREVIEW_RECT.h), (424.0, 84.0, 832.0, 468.0));
    assert_eq!((LIST_RECT.x, LIST_RECT.y, LIST_RECT.w, LIST_RECT.h), (24.0, 84.0, 384.0, 592.0));
    assert_eq!((INFO_RECT.x, INFO_RECT.y, INFO_RECT.w, INFO_RECT.h), (424.0, 564.0, 832.0, 112.0));
    for rect in [PREVIEW_RECT, LIST_RECT, INFO_RECT] {
        assert!(rect.x >= MARGIN && rect.y >= COLUMNS_TOP && rect.w > 0.0 && rect.h > 0.0, "{rect:?}");
        assert!(rect.x + rect.w <= CW as f32 - MARGIN && rect.y + rect.h <= HINT_Y - HINT_GAP, "{rect:?} runs off the screen");
    }
    const { assert!(LIST_RECT.x + LIST_RECT.w <= PREVIEW_RECT.x - COLUMN_GAP, "the list runs under the preview") };
    const { assert!(PREVIEW_RECT.y + PREVIEW_RECT.h <= INFO_RECT.y, "the preview runs over what is said under it") };
}

/// The strip of screens is over the preview and no wider than it, every name fits its tab, and the
/// screens on it are exactly the ones this build draws with a skin.
#[test]
fn the_strip_offers_every_screen_a_skin_draws_and_fits_over_the_preview() {
    rbms_render::font::use_embedded_fonts_only();
    let last = kind_rect(SCREEN_KINDS.len() - 1);
    assert!(
        (last.x + last.w - (PREVIEW_RECT.x + PREVIEW_RECT.w)).abs() < 0.01,
        "the strip is not as wide as the preview: its last tab ends at {}",
        last.x + last.w
    );
    assert!(kind_rect(0).x >= PREVIEW_RECT.x && KIND_STRIP_Y + KIND_TAB_H <= PREVIEW_RECT.y - PREVIEW_BORDER, "the strip runs into the preview");
    for (_, name) in SCREEN_KINDS {
        let width = text_width(name, KIND_SCALE);
        assert!(width + KIND_NAME_ROOM * 2.0 <= KIND_TAB_W, "{name} is {width} wide on a tab of {KIND_TAB_W}");
    }

    let mut offered: Vec<i32> = SCREEN_KINDS.iter().map(|(screen, _)| *screen).collect();
    offered.sort_unstable();
    let drawn: Vec<i32> = (0..).take(rbms_config::SKIN_SCREEN_LABELS.len()).filter(|screen| is_supported_skin_type(*screen)).collect();
    assert_eq!(offered, drawn, "the strip and the screens this build draws with a skin are not the same");
}

/// The rows on show, the cursor's place under them and the line of keys all fit where they are put,
/// and every line the preview can ask the menu to write fits the preview.
#[test]
fn the_rows_the_counter_the_hint_and_the_status_fit_where_they_are_written() {
    rbms_render::font::use_embedded_fonts_only();
    let last = row_rect(VISIBLE_ROWS - 1);
    assert!(last.y + last.h <= LIST_RECT.y + LIST_RECT.h - COUNTER_RISE, "the last row on show runs under the cursor's place in the list");
    assert!(last.x + last.w <= LIST_RECT.x + LIST_RECT.w - SCROLL_BAR_ROOM, "the rows run under the scroll bar");
    let lowest_line = INFO_ROW_Y + (1 + INFO_VALUE_LINES) as f32 * INFO_LINE_PITCH;
    assert!(lowest_line + INFO_LINE_H <= INFO_RECT.h, "what is said of a row with a value of {INFO_VALUE_LINES} lines runs off the panel it is said on");
    assert!(INFO_LOADED_Y + INFO_LOADED_LINES as f32 * INFO_LINE_PITCH <= INFO_ROW_Y, "what loaded runs into the row under it");
    let hint = text_width(HINT, HINT_SCALE);
    assert!(hint <= CW as f32 - MARGIN * 2.0, "the line of keys is {hint} wide");
    for help in [PROPERTY_HELP, FILE_HELP, OFFSET_PLACE_HELP, OFFSET_ANGLE_HELP, OFFSET_ALPHA_HELP, PICKER_NOTE] {
        assert!(text_width(help, INFO_SCALE) <= INFO_RECT.w - INFO_PAD * 2.0, "{help}");
    }
    for id in FIXED_ROWS {
        let help = descriptor(id).help;
        assert_eq!(fit_text(help, INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0), help, "what is said of {id:?} is cut short");
    }
    let statuses = [
        PreviewStatus::Loading,
        PreviewStatus::Absent(PreviewAbsence::NoDocument),
        PreviewStatus::Absent(PreviewAbsence::UnsupportedScreen),
        PreviewStatus::Absent(PreviewAbsence::NoPreview),
        PreviewStatus::PackTrouble(String::new()),
        PreviewStatus::Failed(String::new()),
    ];
    for status in statuses {
        let message = status.message().expect("a status that is not a drawn frame has a line");
        assert!(text_width(message, STATUS_SCALE) <= PREVIEW_RECT.w - STATUS_PAD * 2.0, "{message}");
    }
}

/// What is written out in full under the preview is broken over the lines it is given: at a space
/// where there is one, anywhere in a path that has none, and cut short only past the last line.
#[test]
fn a_long_value_is_broken_over_its_lines_and_cut_short_only_past_the_last() {
    rbms_render::font::use_embedded_fonts_only();
    let width = INFO_RECT.w - INFO_PAD * 2.0;
    assert_eq!(wrapped("BUILT-IN SCREEN", INFO_SCALE, width, INFO_LOADED_LINES), vec!["BUILT-IN SCREEN".to_string()]);
    assert!(wrapped("   ", INFO_SCALE, width, INFO_LOADED_LINES).is_empty());

    let path = format!("/skins/{}/pack", "a-folder-with-a-long-name/".repeat(8));
    let lines = wrapped(&path, INFO_SCALE, width, INFO_VALUE_LINES);
    assert_eq!(lines.len(), INFO_VALUE_LINES, "a path wider than the panel was not broken");
    assert_eq!(lines.concat(), path, "a path that fits its lines lost something on the way");
    assert!(lines.iter().all(|line| text_width(line, INFO_SCALE) <= width + 1.0), "a line is wider than the panel");

    let sentence = "the skin did not load because a file it names is not there ".repeat(6);
    let broken = wrapped(&sentence, INFO_SCALE, width, INFO_LOADED_LINES);
    assert_eq!(broken.len(), INFO_LOADED_LINES);
    assert!(!broken[0].ends_with(' ') && !broken[1].starts_with(' '), "a line keeps the space it was broken at");
    assert!(sentence.starts_with(&format!("{} ", broken[0])), "the first line was not broken at a space: {}", broken[0]);
    assert_ne!(broken.concat().len(), sentence.trim().len(), "the fixture is short enough to fit, so nothing was cut");
    assert!(text_width(&broken[1], INFO_SCALE) <= width + 1.0, "the last line runs off the panel");

    assert_eq!(wrapped("no room at all", INFO_SCALE, 1.0, INFO_VALUE_LINES).len(), 1, "a width nothing fits in did not end the breaking");
}

/// A Lua skin for the decide screen whose header answers and whose body gives up, and what it says
/// as it does.
const GIVES_UP: &str =
    "local skin = { type = 6, name = 'Gives up', w = 1280, h = 720 }\nif skin_config then\n    error('the sample skin gives up')\nend\nreturn skin\n";
const GIVES_UP_REASON: &str = "the sample skin gives up";

/// A Lua skin for the decide screen that draws one image over the whole screen, for a scene short
/// enough to end and begin again while a test watches.
const SHORT_DECIDE: &str = r#"
local skin = { type = 6, name = "Short", w = 1280, h = 720, scene = 1000, fadeout = 200 }
if skin_config then
    skin.source = { { id = 0, path = "panel.png" } }
    skin.image = { { id = "panel", src = 0, x = 0, y = 0, w = 8, h = 8 } }
    skin.destination = { { id = "panel", dst = { { x = 0, y = 0, w = 1280, h = 720 } } } }
end
return skin
"#;

/// The edge of the one image [`SHORT_DECIDE`] draws from.
const PANEL_EDGE: u32 = 8;

/// How many frames a scene of [`SHORT_DECIDE`] is watched for, which is several of its lengths.
const WATCHED_FRAMES: usize = 400;

/// An app whose pack is one Lua skin for the decide screen, with the image it draws from, and whose
/// menu is configuring that screen.
fn app_with_decide_skin(tag: &str, source: &str) -> App {
    let mut app = bare_app(tag);
    let pack = app.shared.settings_path.parent().expect("the settings file has a folder").join("decide-pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join("decide.luaskin"), source).expect("the skin is written");
    image::RgbaImage::from_pixel(PANEL_EDGE, PANEL_EDGE, image::Rgba([u8::MAX; 4])).save(pack.join("panel.png")).expect("the sheet is written");
    app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    app.shared.config.skin.screen = SKIN_TYPE_DECIDE;
    app
}

impl Menu {
    /// Draw until the preview says something other than that its document is on its way, and answer
    /// what it said.
    fn until_said(&mut self) -> PreviewStatus {
        for _ in 0..SETTLE_FRAMES {
            self.frame();
            if *self.state.preview.status() != PreviewStatus::Loading {
                break;
            }
            std::thread::sleep(SETTLE_PAUSE);
        }
        self.state.preview.status().clone()
    }
}

/// What a Lua skin says as it fails is a line of why and then a traceback. Only the line is written:
/// under the preview on the lines the LOADED value is given, and in the preview's place.
#[test]
fn a_reason_that_comes_with_a_traceback_is_written_on_the_lines_it_is_given() {
    rbms_render::font::use_embedded_fonts_only();
    let width = INFO_RECT.w - INFO_PAD * 2.0;
    let said = "decide.luaskin:3: the sample skin gives up\nstack traceback:\n\t[C]: in ?\r\n\t[C]: in function 'error'\n\tdecide.luaskin:3: in main chunk";
    let lines = wrapped(said, INFO_SCALE, width, INFO_LOADED_LINES);
    assert!(!lines.is_empty() && lines.len() <= INFO_LOADED_LINES, "{lines:?}");
    assert!(lines.iter().all(|line| !line.chars().any(char::is_control)), "a line carries a break of its own: {lines:?}");

    let mut menu = Menu::over(app_with_decide_skin("traceback", GIVES_UP));
    let status = menu.until_said();
    let PreviewStatus::Failed(reason) = &status else {
        panic!("a skin that gives up was answered with {status:?}");
    };
    assert!(reason.contains(GIVES_UP_REASON) && !reason.chars().any(char::is_control), "{reason:?}");
    menu.frame();
    let loaded = menu.value_of_fixed(SettingId::SkinInfo);
    assert!(loaded.contains(GIVES_UP_REASON), "the LOADED row does not say why: {loaded:?}");
    assert!(!loaded.chars().any(char::is_control), "the LOADED row carries the traceback: {loaded:?}");
    menu.save("skin-menu-failed");
}

/// What is said of an offset row names the unit of the axis the row moves: an angle and an opacity
/// are not counted in pixels.
#[test]
fn what_is_said_of_an_offset_row_goes_with_its_axis() {
    rbms_render::font::use_embedded_fonts_only();
    let help = |axis| row_help(SettingRow::Skin(SkinRow::Offset(0, axis)));
    for axis in [OffsetAxis::R, OffsetAxis::A] {
        assert!(!help(axis).to_ascii_lowercase().contains("pixel"), "{axis:?} is said to be in pixels: {}", help(axis));
    }
    assert!(help(OffsetAxis::R).contains("degrees"), "{}", help(OffsetAxis::R));
    assert!(help(OffsetAxis::A).contains("255"), "{}", help(OffsetAxis::A));
    for axis in [OffsetAxis::X, OffsetAxis::Y, OffsetAxis::W, OffsetAxis::H] {
        assert!(help(axis).contains("percent"), "an offset of the whole screen is a share of it, and {axis:?} says nothing of that: {}", help(axis));
    }
    for axis in OffsetAxis::ALL {
        assert_eq!(fit_text(help(axis), INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0), help(axis), "what is said of {axis:?} is cut short");
    }
}

/// The first wait for a document is said in the preview's place. A scene that ends and begins again
/// is not: the frames its skin is read again on leave the rectangle dark, as the fade left it.
#[test]
fn a_scene_that_begins_again_leaves_its_place_dark() {
    let corner = (PREVIEW_RECT.x + 1.0, PREVIEW_RECT.y + 1.0);
    let mut menu = Menu::over(app_with_decide_skin("again", SHORT_DECIDE));
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Loading);
    assert_eq!(pixel(&menu.pixels, corner), theme().panel, "the first wait for a document is not said on a panel");
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(SKIN_TYPE_DECIDE));

    let mut waits = 0;
    for _ in 0..WATCHED_FRAMES {
        menu.frame();
        if *menu.state.preview.status() == PreviewStatus::Loading {
            waits += 1;
            assert_eq!(pixel(&menu.pixels, corner), Color::BLACK, "a scene that began again flashed a panel in the preview's place");
        }
    }
    assert!(waits > 0, "the scene never began again in {WATCHED_FRAMES} frames, so nothing was held");
}

#[test]
fn a_menu_with_no_skin_lists_the_six_rows_and_says_so_where_the_preview_would_be() {
    let mut app = bare_app("bare");
    app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    let mut menu = Menu::over(app);
    assert_eq!(menu.state.rows, FIXED_ROWS.map(SettingRow::Fixed).to_vec());
    menu.frame();
    assert!(menu.pixels.painted_pixels() > 0 && menu.pixels.quad_count() > 0);
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Absent(PreviewAbsence::NoDocument));
    let corner = (PREVIEW_RECT.x + 1.0, PREVIEW_RECT.y + 1.0);
    assert_eq!(pixel(&menu.pixels, corner), theme().panel, "the status is not on a panel like the list's");
    assert_eq!(pixel(&menu.pixels, (LIST_RECT.x + 1.0, LIST_RECT.y + LIST_RECT.h - 2.0)), theme().panel, "the list is not on a panel");
    assert_eq!(menu.rows_on_show(), (0..FIXED_ROWS.len()).collect::<Vec<_>>());
    menu.save("skin-menu-empty");
}

/// The list is the rows the settings screen's SKIN tab had: the six the settings table declares, and
/// between the document row and the actions the rows the chosen document declares.
#[test]
fn the_list_is_the_settings_tables_skin_rows_with_the_documents_own_between_them() {
    let mut menu = Menu::over(app_with_fixtures("rows"));
    assert_eq!(menu.state.rows, menu.app.shared.settings_rows(SettingTab::Skin));
    assert_eq!(menu.state.rows, FIXED_ROWS.map(SettingRow::Fixed).to_vec(), "a document nobody chose has rows in the list");

    menu.choose_first_document();
    assert_eq!(menu.state.rows, menu.app.shared.settings_rows(SettingTab::Skin));
    let shown = menu.labels();
    let property = shown.iter().position(|label| label == fixtures::BROWSER_PROPERTY_LABEL).expect("the document's own rows are in the list");
    let document = menu.at(SettingRow::Fixed(SettingId::SkinDocument));
    let reload = menu.at(SettingRow::Fixed(SettingId::SkinReload));
    assert!(document < property && property < reload, "the document's rows are not between the document row and the actions");
    assert_eq!(menu.state.sel, document, "the cursor left the row that was stepped");
    assert_eq!(menu.value_of_fixed(SettingId::SkinInfo), "1920X1080 - JSON - 0 WARNINGS - dj", "the document was not read");
    let lines: Vec<SettingsLine> = menu.state.rows.iter().map(|row| menu.app.shared.settings_scene_line(*row)).collect();
    assert_eq!(menu.state.lines, lines, "the list does not show what the rows hold");
}

/// The rows a document declares are drawn, are clickable, and change what is on screen when they
/// move -- which is what a row wired to the wrong document, or to nothing, would not do.
#[test]
fn the_documents_rows_are_drawn_and_redraw_when_one_moves() {
    let mut menu = Menu::over(app_with_fixtures("draw"));
    let bare = menu.frame().pixel_checksum();
    menu.choose_first_document();
    let chosen = menu.frame().pixel_checksum();
    assert_ne!(bare, chosen, "choosing a document did not change the screen");
    assert_eq!(menu.app.shared.hot.len(), menu.state.rows.len().min(VISIBLE_ROWS) + SCREEN_KINDS.len(), "every drawn row and every tab is clickable");

    let property = SettingRow::Skin(SkinRow::Property(0));
    menu.focus(property);
    let focused = menu.frame().pixel_checksum();
    let before = menu.value_of(property);
    menu.key(KeyCode::ArrowRight);
    assert_ne!(menu.value_of(property), before, "stepping the document's own row changed nothing");
    assert_ne!(focused, menu.frame().pixel_checksum(), "the moved row was not redrawn");
    menu.save("skin-menu-fixture");
}

/// A step of an option, a file slot or the document row has the library read the document again,
/// which is what the preview draws from; a nudge of an offset is read live and reads nothing again.
#[test]
fn an_option_a_file_and_the_document_are_read_again_and_an_offset_is_not() {
    let mut menu = Menu::over(app_with_fixtures("reload"));
    menu.choose_first_document();
    let screen = fixtures::MUSIC_SELECT;
    let mut build = menu.app.shared.skins.build_of(screen).expect("the chosen document was read");

    for row in [SettingRow::Skin(SkinRow::Property(0)), SettingRow::Skin(SkinRow::File(0))] {
        menu.focus(row);
        menu.key(KeyCode::ArrowRight);
        let read = menu.app.shared.skins.build_of(screen).expect("the document is still read");
        assert_ne!(read, build, "{row:?} moved and the document was not read again");
        assert!(!menu.app.shared.skin_reload_pending(), "{row:?} left a read waiting to be asked for");
        build = read;
    }

    let offset = SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::Y));
    menu.focus(offset);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(offset), "+1");
    assert_eq!(menu.app.shared.skins.build_of(screen), Some(build), "a nudge of an offset read the document again");

    menu.focus_fixed(SettingId::SkinDocument);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "DEFAULT", "the document row does not step back to the built-in screen");
    assert_eq!(menu.app.shared.skins.build_of(screen), None, "a document nothing draws is still held");
}

/// Left, Right and Enter step the row under the cursor, Enter the way Right does, and none of them
/// leaves the menu, whichever row the cursor is on.
#[test]
fn left_right_and_enter_step_the_row_under_the_cursor_and_stay_on_the_menu() {
    let mut menu = Menu::over(app_with_fixtures("steps"));
    menu.choose_first_document();
    let property = SettingRow::Skin(SkinRow::Property(0));
    menu.focus(property);
    let first = menu.value_of(property);
    assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay));
    let second = menu.value_of(property);
    assert_ne!(second, first, "Enter did not step the row");
    assert!(matches!(menu.key(KeyCode::ArrowLeft), Transition::Stay));
    assert_eq!(menu.value_of(property), first, "Left did not step the row back");
    assert!(matches!(menu.key(KeyCode::NumpadEnter), Transition::Stay));
    assert_eq!(menu.value_of(property), second, "the keypad's Enter is not Enter");

    menu.focus_fixed(SettingId::SkinScreen);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.app.shared.config.skin.screen, fixtures::MUSIC_SELECT + 1, "the SCREEN row does not step through every screen a document can declare");
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.app.shared.config.skin.screen, fixtures::MUSIC_SELECT);

    for at in 0..menu.state.rows.len() {
        if at >= menu.state.rows.len() {
            break;
        }
        menu.state.focus(at);
        let row = menu.state.rows[at];
        assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay), "Enter on {row:?} left the menu");
        menu.frame();
    }
    let info = menu.value_of_fixed(SettingId::SkinInfo);
    menu.focus_fixed(SettingId::SkinInfo);
    let skin = menu.app.shared.config.skin.clone();
    menu.key(KeyCode::ArrowRight);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.app.shared.config.skin, skin, "the LOADED row changed a setting");
    assert_eq!(menu.value_of_fixed(SettingId::SkinInfo), info);
}

/// Everything the menu edits goes into the settings file and comes back out of it, so the next
/// launch draws the screen with the document and the choices the player left it on -- in the group
/// of the file the settings screen's SKIN tab wrote, read by a menu that was never open before.
#[test]
fn what_the_menu_edits_survives_the_settings_file() {
    let mut menu = Menu::over(app_with_fixtures("persist"));
    menu.choose_first_document();
    for row in [SettingRow::Skin(SkinRow::Property(0)), SettingRow::Skin(SkinRow::File(0)), SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::Y))] {
        menu.focus(row);
        menu.key(KeyCode::ArrowRight);
    }
    let shown = menu.state.lines.clone();

    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back), "Escape does not leave the menu");
    let written = std::fs::read_to_string(&menu.app.shared.settings_path).expect("Escape wrote the settings file");
    assert!(written.contains("skin:"), "the settings file does not hold the skin group");
    let restored = rbms_config::load(&menu.app.shared.settings_path).expect("the file the menu wrote reads back").config;
    assert_eq!(restored.skin, menu.app.shared.config.skin, "the skin group did not survive the file");

    let mut next = app();
    next.shared.settings_path = menu.app.shared.settings_path.clone();
    next.shared.config = restored;
    let reopened = Menu::over(next);
    assert_eq!(reopened.state.lines, shown, "the reopened menu does not show what the last one left");
}

/// An offset row is the one row kind zeroed rather than cycled, and the key that does it is the same
/// one on the number row and the keypad. On any other row the key does nothing.
#[test]
fn the_zero_key_puts_an_offset_row_back_to_the_authors_placement() {
    let mut menu = Menu::over(app_with_fixtures("zero"));
    menu.choose_first_document();
    let offset = *menu.state.rows.iter().find(|row| matches!(row, SettingRow::Skin(SkinRow::Offset(_, _)))).expect("the document declares an offset row");
    for zero in [KeyCode::Digit0, KeyCode::Numpad0] {
        menu.focus(offset);
        for _ in 0..3 {
            menu.key(KeyCode::ArrowRight);
        }
        assert_eq!(menu.value_of(offset), "+3");
        menu.key(zero);
        assert_eq!(menu.value_of(offset), "+0", "{zero:?}");
    }

    let property = SettingRow::Skin(SkinRow::Property(0));
    menu.focus(property);
    menu.key(KeyCode::ArrowRight);
    let moved = menu.value_of(property);
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.value_of(property), moved, "the zero key moved a row that is not an offset");
}

/// RESET drops the choices made in the document and writes the settings file, so the next launch
/// starts on what the author shipped rather than on what was reset away.
#[test]
fn reset_drops_the_choices_and_the_file_no_longer_holds_them() {
    let mut menu = Menu::over(app_with_fixtures("reset"));
    menu.choose_first_document();
    let property = SettingRow::Skin(SkinRow::Property(0));
    menu.focus(property);
    menu.key(KeyCode::ArrowRight);
    let moved = menu.value_of(property);
    assert!(!menu.app.shared.config.skin.custom.is_empty(), "the choice was never stored");

    menu.focus_fixed(SettingId::SkinReset);
    menu.key(KeyCode::ArrowLeft);
    assert!(!menu.app.shared.config.skin.custom.is_empty(), "a step to the left ran the reset");
    menu.key(KeyCode::Enter);
    assert!(menu.app.shared.config.skin.custom.is_empty(), "the choices survived the reset");
    assert_ne!(menu.value_of(property), moved, "the row still shows what was reset away");
    assert_eq!(menu.state.sel, menu.at(SettingRow::Fixed(SettingId::SkinReset)), "the cursor left the row that was run");

    let restored = rbms_config::load(&menu.app.shared.settings_path).expect("the reset wrote the settings file").config;
    assert!(restored.skin.custom.is_empty(), "the settings file still holds the choices that were reset away");
}

/// RELOAD reads every header from its file again, reads the chosen document again, and writes the
/// settings file. A step to the left runs nothing.
#[test]
fn reload_reads_the_folder_and_the_document_afresh_and_writes_the_settings_file() {
    let mut menu = Menu::over(app_with_fixtures("reload-row"));
    menu.choose_first_document();
    let screen = fixtures::MUSIC_SELECT;
    let build = menu.app.shared.skins.build_of(screen).expect("the chosen document was read");
    let headers = menu.app.shared.skins.header_reads();
    let _ = std::fs::remove_file(&menu.app.shared.settings_path);

    menu.focus_fixed(SettingId::SkinReload);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.app.shared.skins.header_reads(), headers, "a step to the left ran the reload");
    assert!(!menu.app.shared.settings_path.exists());

    menu.key(KeyCode::ArrowRight);
    assert!(menu.app.shared.skins.header_reads() > headers, "no header was read from its file again");
    assert_ne!(menu.app.shared.skins.build_of(screen), Some(build), "the document was not read again");
    assert!(menu.app.shared.settings_path.exists(), "the reload did not write the settings file");
    assert_eq!(menu.state.sel, menu.at(SettingRow::Fixed(SettingId::SkinReload)), "the cursor left the row that was run");
}

/// A folder holding one Lua skin for the song browser, which is a skin pack of one document.
fn lua_pack(app: &App) -> PathBuf {
    let pack = app.shared.settings_path.parent().expect("the settings file has a folder").join("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join("browser.luaskin"), "return { type = 5, name = 'Wheel', author = 'dj', w = 1920, h = 1080, destination = {} }")
        .expect("the skin is written");
    pack
}

/// Right, Enter and a press on the PACK FOLDER row put the folder picker up.
#[test]
fn the_pack_folder_row_puts_the_picker_up() {
    let mut menu = Menu::over(bare_app("picker-opens"));
    menu.focus_fixed(SettingId::SkinPack);
    assert!(menu.state.pack_picker.is_none());
    assert!(matches!(menu.key(KeyCode::Enter), Transition::Stay));
    assert!(menu.state.pack_picker.is_some(), "Enter on the row did not put the picker up");

    for opens in [KeyCode::ArrowRight, KeyCode::Enter] {
        menu.state.pack_picker = None;
        menu.key(opens);
        assert!(menu.state.pack_picker.is_some(), "{opens:?}");
    }
    menu.state.pack_picker = None;
    menu.frame();
    menu.click(menu.middle_of_hot(Hot::SkinMenuRow(0)));
    assert!(menu.state.pack_picker.is_some(), "a press on the row did not put the picker up");
}

/// The picker is a native window with the keyboard: the menu behind it keeps drawing and acts on
/// neither a key nor a press until it has been answered.
#[test]
fn keys_and_presses_are_ignored_while_the_picker_is_up() {
    let mut menu = Menu::over(app_with_fixtures("picker-keys"));
    menu.choose_first_document();
    menu.focus(SettingRow::Skin(SkinRow::Property(0)));
    menu.frame();
    let (tx, handle) = crate::dialog::handle_for_tests();
    menu.state.pack_picker = Some(handle);
    let skin = menu.app.shared.config.skin.clone();
    let cursor = menu.state.sel;
    for code in [KeyCode::ArrowDown, KeyCode::ArrowUp, KeyCode::ArrowRight, KeyCode::Enter, KeyCode::Digit0, KeyCode::Tab, KeyCode::PageDown, KeyCode::Escape] {
        assert!(matches!(menu.key(code), Transition::Stay), "{code:?} left the menu behind the picker");
    }
    menu.click(menu.middle_of_hot(Hot::SkinMenuKind(0)));
    menu.click(menu.middle_of_hot(Hot::SkinMenuRow(0)));
    assert_eq!(menu.app.shared.config.skin, skin, "a key or a press behind the picker changed a setting");
    assert_eq!(menu.state.sel, cursor, "a key or a press behind the picker moved the cursor");

    menu.frame();
    assert!(menu.state.pack_picker.is_some(), "the menu gave up on a picker nobody has answered");
    tx.send(None).expect("the handle is still held");
    menu.frame();
    assert!(menu.state.pack_picker.is_none(), "the menu kept waiting after the picker was dismissed");
    assert_eq!(menu.app.shared.config.skin, skin, "a dismissed picker changed a setting");
    menu.key(KeyCode::ArrowDown);
    assert_eq!(menu.state.sel, cursor + 1, "the keys did not come back with the picker gone");
}

/// The folder that comes back from the picker is the pack every screen takes its document from: it
/// is shown on the row, written to the settings file, and its Lua skin is what the SKIN row then
/// names for the screen. Stepping the row left names no pack again.
#[test]
fn the_pack_folder_row_takes_a_picked_folder_and_offers_its_lua_skins() {
    let mut menu = Menu::over(app_with_fixtures("pack"));
    let pack = lua_pack(&menu.app);
    assert_eq!(menu.value_of_fixed(SettingId::SkinPack), "(none)");
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "DEFAULT");

    let (tx, handle) = crate::dialog::handle_for_tests();
    menu.state.pack_picker = Some(handle);
    tx.send(Some(pack.clone())).expect("the handle is still held");
    menu.state.poll_pack_picker(&mut FrameCtx { shared: &mut menu.app.shared, now: std::time::Instant::now(), dt: 0.0 });
    assert!(menu.state.pack_picker.is_none());

    assert_eq!(menu.value_of_fixed(SettingId::SkinPack), pack.to_string_lossy());
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "PACK: Wheel", "the pack's Lua skin does not draw the screen it declares");
    assert_eq!(menu.value_of_fixed(SettingId::SkinInfo), "READ WHEN ITS SCREEN OPENS", "a Lua skin was read with no screen to read it against");
    let restored = rbms_config::load(&menu.app.shared.settings_path).expect("the file the menu wrote reads back").config;
    assert_eq!(restored.skin.pack_folder(), Some(pack.to_string_lossy().as_ref()), "the pack did not reach the settings file");

    menu.focus_fixed(SettingId::SkinDocument);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "Browser", "the row does not step from the pack's skin to the folder's document");
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "Wheel", "the pack's Lua skin cannot be chosen by hand");
    menu.app.shared.config.skin.select(fixtures::MUSIC_SELECT, None);

    menu.focus_fixed(SettingId::SkinPack);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of_fixed(SettingId::SkinPack), "(none)");
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "DEFAULT", "a pack that is no longer named still draws a screen");
    assert!(menu.state.pack_picker.is_none(), "a step to the left put the picker up");
    let cleared = rbms_config::load(&menu.app.shared.settings_path).expect("the file the menu wrote reads back").config;
    assert_eq!(cleared.skin.pack_folder(), None, "the settings file still names the pack");
}

/// Up and Down move the cursor a row and stop at the ends, the page keys a screenful, and the list
/// scrolls just far enough to keep the cursor on show.
#[test]
fn the_cursor_stays_on_show_through_a_list_longer_than_the_screen() {
    let mut menu = Menu::over(app_with_long_document("scroll"));
    let rows = menu.state.rows.len();
    assert!(rows > VISIBLE_ROWS * 2, "the fixture has {rows} rows, which does not scroll far enough to tell");
    menu.frame();
    assert_eq!(menu.rows_on_show(), (0..VISIBLE_ROWS).collect::<Vec<_>>());

    menu.key(KeyCode::ArrowUp);
    assert_eq!((menu.state.sel, menu.state.top), (0, 0), "Up at the top of the list moved something");
    for step in 1..rows {
        menu.key(KeyCode::ArrowDown);
        assert_eq!(menu.state.sel, step);
        assert!(menu.state.top <= menu.state.sel && menu.state.sel < menu.state.top + VISIBLE_ROWS, "the cursor went off the screen at row {step}");
        assert_eq!(menu.state.top, step.saturating_sub(VISIBLE_ROWS - 1), "the list scrolled further than the cursor needed at row {step}");
    }
    menu.key(KeyCode::ArrowDown);
    assert_eq!(menu.state.sel, rows - 1, "Down at the foot of the list moved the cursor");
    menu.frame();
    assert_eq!(menu.rows_on_show(), (rows - VISIBLE_ROWS..rows).collect::<Vec<_>>(), "the foot of the list is not what is drawn");

    menu.key(KeyCode::PageUp);
    assert_eq!(menu.state.sel, rows - 1 - VISIBLE_ROWS);
    assert_eq!(menu.state.top, menu.state.sel, "a page up left the cursor off the screen");
    menu.key(KeyCode::PageDown);
    menu.key(KeyCode::PageDown);
    assert_eq!(menu.state.sel, rows - 1, "a page down past the foot did not stop on the last row");
    for _ in 0..rows {
        menu.key(KeyCode::PageUp);
    }
    assert_eq!((menu.state.sel, menu.state.top), (0, 0));
}

/// The wheel scrolls the list while the pointer is over it, stops at both ends, and leaves the
/// cursor on its row; the key that next moves the cursor brings it back onto the screen. Over the
/// preview, or anywhere else, the wheel does nothing.
#[test]
fn the_wheel_scrolls_the_list_under_the_pointer_and_nothing_anywhere_else() {
    let mut menu = Menu::over(app_with_long_document("wheel"));
    let last = menu.state.rows.len() - VISIBLE_ROWS;
    let over_list = middle(LIST_RECT);

    for elsewhere in [middle(PREVIEW_RECT), middle(INFO_RECT), (MARGIN, TITLE_Y), middle(kind_rect(0))] {
        menu.wheel(elsewhere, 3.0);
        assert_eq!(menu.state.top, 0, "the wheel scrolled the list with the pointer at {elsewhere:?}");
    }

    menu.wheel(over_list, 3.0);
    assert_eq!((menu.state.top, menu.state.sel), (3, 0), "the wheel did not scroll the list alone");
    menu.frame();
    assert_eq!(menu.rows_on_show(), (3..3 + VISIBLE_ROWS).collect::<Vec<_>>());
    menu.wheel(over_list, -1.0);
    assert_eq!(menu.state.top, 2);
    menu.wheel(over_list, -50.0);
    assert_eq!(menu.state.top, 0, "the wheel scrolled past the top");
    menu.wheel(over_list, 1000.0);
    assert_eq!(menu.state.top, last, "the wheel scrolled past the foot");
    menu.wheel(over_list, f32::NAN);
    menu.wheel(over_list, -1.0);
    assert_eq!(menu.state.top, last - 1, "a turn that is not a number broke the wheel");

    menu.wheel(over_list, -0.4);
    menu.wheel(over_list, -0.4);
    assert_eq!(menu.state.top, last - 1, "less than a line of travel moved the list");
    menu.wheel(over_list, -0.4);
    assert_eq!(menu.state.top, last - 2, "three parts of a line did not add up to one");

    menu.key(KeyCode::ArrowDown);
    assert_eq!((menu.state.sel, menu.state.top), (1, 1), "the cursor was not brought back onto the screen");
}

/// A press on a row moves the cursor there and steps the row once; a press on a tab of the strip
/// configures that screen; a press on the preview, or on nothing, changes nothing at all.
#[test]
fn a_press_steps_the_row_or_chooses_the_screen_it_lands_on_and_nothing_on_the_preview() {
    let mut menu = Menu::over(app_with_long_document("click"));
    let option = SettingRow::Skin(SkinRow::Property(3));
    let at = menu.at(option);
    menu.frame();
    assert!(menu.rows_on_show().contains(&at));
    let before = menu.value_of(option);
    assert!(matches!(menu.click(menu.middle_of_hot(Hot::SkinMenuRow(at))), Transition::Stay));
    assert_eq!(menu.state.sel, at, "the press did not move the cursor to the row");
    assert_ne!(menu.value_of(option), before, "the press did not step the row");

    menu.wheel(middle(LIST_RECT), 5.0);
    menu.frame();
    let scrolled = menu.rows_on_show()[VISIBLE_ROWS / 2];
    menu.click(menu.middle_of_hot(Hot::SkinMenuRow(scrolled)));
    assert_eq!(menu.state.sel, scrolled, "a press on a scrolled list landed on another row than the one under it");

    let skin = menu.app.shared.config.skin.clone();
    let cursor = menu.state.sel;
    for nowhere in [middle(PREVIEW_RECT), (PREVIEW_RECT.x + 1.0, PREVIEW_RECT.y + 1.0), middle(INFO_RECT), (MARGIN, TITLE_Y), (CW as f32 - 1.0, HINT_Y)] {
        assert_eq!(menu.app.shared.hit_test(nowhere), None, "{nowhere:?} is clickable");
        assert!(matches!(menu.click(nowhere), Transition::Stay));
    }
    let another_row = menu.middle_of_hot(Hot::SkinMenuRow(scrolled + 1));
    for (button, pressed) in [(MouseButton::Middle, true), (MouseButton::Left, false), (MouseButton::Right, false)] {
        menu.state.handle_mouse(&mut FrameCtx { shared: &mut menu.app.shared, now: std::time::Instant::now(), dt: 0.0 }, another_row, button, pressed);
    }
    let a_tab = menu.middle_of_hot(Hot::SkinMenuKind(0));
    menu.state.handle_mouse(&mut FrameCtx { shared: &mut menu.app.shared, now: std::time::Instant::now(), dt: 0.0 }, a_tab, MouseButton::Right, true);
    assert_eq!(menu.app.shared.config.skin, skin, "a press that landed on nothing changed a setting");
    assert_eq!(menu.state.sel, cursor, "a press that landed on nothing moved the cursor");

    for (place, (screen, name)) in SCREEN_KINDS.iter().enumerate() {
        menu.frame();
        menu.click(menu.middle_of_hot(Hot::SkinMenuKind(place)));
        assert_eq!(menu.app.shared.config.skin.screen, *screen, "a press on {name} configures another screen");
        assert_eq!(menu.state.rows, menu.app.shared.settings_rows(SettingTab::Skin), "the list is not the rows of {name}");
    }
}

/// A press of the right button on a row moves the cursor there and steps the row back, which is how
/// a step taken with the left button is taken back. A row that only runs to the right -- RESET --
/// does nothing for it.
#[test]
fn a_press_of_the_right_button_steps_the_row_back() {
    let mut menu = Menu::over(app_with_long_document("right-click"));
    let offset = SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::X));
    menu.focus(offset);
    menu.frame();
    let on_offset = menu.middle_of_hot(Hot::SkinMenuRow(menu.at(offset)));
    menu.state.focus(0);
    menu.click(on_offset);
    assert_eq!(menu.value_of(offset), "+1");
    let right = |menu: &mut Menu, at| {
        menu.state.handle_mouse(&mut FrameCtx { shared: &mut menu.app.shared, now: std::time::Instant::now(), dt: 0.0 }, at, MouseButton::Right, true)
    };
    menu.state.focus(0);
    assert!(matches!(right(&mut menu, on_offset), Transition::Stay));
    assert_eq!(menu.value_of(offset), "+0", "the right button did not take the step back");
    assert_eq!(menu.state.sel, menu.at(offset), "the right button did not move the cursor to the row");
    right(&mut menu, on_offset);
    assert_eq!(menu.value_of(offset), "-1");

    let skin = menu.app.shared.config.skin.clone();
    menu.focus_fixed(SettingId::SkinReset);
    menu.frame();
    let on_reset = menu.middle_of_hot(Hot::SkinMenuRow(menu.at(SettingRow::Fixed(SettingId::SkinReset))));
    right(&mut menu, on_reset);
    assert_eq!(menu.app.shared.config.skin, skin, "the right button ran the reset");
}

/// With Shift held an arrow moves an offset row ten steps, and stops at the row's own limit. Any
/// other row is stepped once whatever is held, and Enter is one step always.
#[test]
fn shift_moves_an_offset_ten_steps_at_a_time_and_nothing_else() {
    let mut menu = Menu::over(app_with_long_document("shift"));
    let offset = SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::Y));
    let screen = menu.app.shared.config.skin.screen;
    menu.focus(offset);
    menu.key(KeyCode::ShiftLeft);
    let build = menu.app.shared.skins.build_of(screen);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(offset), "+10");
    menu.key(KeyCode::ArrowLeft);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of(offset), "-10");
    menu.key(KeyCode::Enter);
    assert_eq!(menu.value_of(offset), "-9", "Enter took more than a step with Shift held");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an offset had the document read again");

    let option = SettingRow::Skin(SkinRow::Property(0));
    menu.focus(option);
    assert_eq!(menu.value_of(option), "FIRST");
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(option), "SECOND", "an option of two was stepped more than once with Shift held");

    menu.key_up(KeyCode::ShiftLeft);
    menu.focus(offset);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(offset), "-8", "an arrow went on moving ten steps with Shift let go");
}

/// A path too long for its row keeps its end and gives up its beginning, so the folder's own name is
/// on the row. What is said before the path is kept in front of it.
#[test]
fn a_path_too_long_for_its_row_keeps_its_end() {
    rbms_render::font::use_embedded_fonts_only();
    let room = pack_value_room(descriptor(SettingId::SkinPack).label);
    assert!(room > (row_rect(0).w - ROW_PAD * 2.0) * VALUE_MAX_SHARE, "the pack folder has no more room than any other value");
    assert_eq!(fit_path("/skins/pack", VALUE_SCALE, room), "/skins/pack", "a path that fits was cut");
    const LONG: &str = "/Users/somebody/with/a/long/way/down/to/Downloads/ModernChic";
    let wide = "\u{30b9}\u{30ad}\u{30f3}/".repeat(20);
    let application = format!("DEFAULT ({}rbms-default)", "/a/folder/of/the/application/".repeat(4));
    for (value, lead, name) in [
        (LONG.to_string(), "", "ModernChic"),
        (format!("(MISSING) {LONG}"), "(MISSING) ", "ModernChic"),
        (application, "DEFAULT (", "rbms-default)"),
        (format!("{LONG} (RBMS_SKIN_PACK)"), "", "(RBMS_SKIN_PACK)"),
        (wide.clone(), "\u{30b9}\u{30ad}\u{30f3}", "/"),
    ] {
        let fitted = fit_path(&value, VALUE_SCALE, room);
        assert!(text_width(&fitted, VALUE_SCALE) <= room + 1.0, "{fitted} is wider than its row");
        assert!(fitted.starts_with(&format!("{lead}{PATH_GAP}")), "{fitted} does not keep what is written before the path");
        assert!(fitted.ends_with(name), "{fitted} does not end with the last of {value}");
        let kept = fitted.trim_start_matches(lead).trim_start_matches(PATH_GAP);
        assert!(value.ends_with(kept), "{fitted} is not the end of {value}");
    }
    let crowded = format!("{} /skins/pack", "A LEAD LONGER THAN THE ROW ".repeat(4));
    let fitted = fit_path(&crowded, VALUE_SCALE, room);
    assert!(fitted.starts_with(PATH_GAP) && fitted.ends_with("/skins/pack"), "a lead with no room for a path after it was kept: {fitted}");
    assert_eq!(fit_path("no room at all", VALUE_SCALE, 1.0), PATH_GAP, "a width nothing fits in");
}

/// The scroll bar's thumb is drawn over a track the length of the rows on show, inside the list and
/// clear of the rows, and is told from the track and from the panel under both.
#[test]
fn the_scroll_bar_is_a_thumb_on_a_track_beside_the_rows() {
    let mut menu = Menu::over(app_with_long_document("bar"));
    menu.frame();
    let (track, thumb) = (scroll_track(), scroll_thumb(menu.state.top, menu.state.rows.len()));
    assert!(track.x >= row_rect(0).x + row_rect(0).w && track.x + track.w <= LIST_RECT.x + LIST_RECT.w, "{track:?}");
    assert!(thumb.y >= track.y && thumb.y + thumb.h <= track.y + track.h && thumb.h < track.h, "{thumb:?} on {track:?}");
    let on_thumb = pixel(&menu.pixels, middle(thumb));
    let on_track = pixel(&menu.pixels, (track.x + track.w * 0.5, track.y + track.h - 1.0));
    assert_eq!(on_thumb, theme().text_muted);
    assert_eq!(on_track, theme().panel_hi);
    assert!(on_thumb != on_track && on_track != theme().panel, "the bar does not show against what is under it");

    menu.key(KeyCode::PageDown);
    menu.key(KeyCode::PageDown);
    assert!(scroll_thumb(menu.state.top, menu.state.rows.len()).y > thumb.y, "the thumb did not move with the list");
}

/// What a status says under its heading is written on the lines it is given and no wider than the
/// preview, however long the reason a skin gave.
#[test]
fn what_a_status_says_under_its_heading_fits_the_preview() {
    rbms_render::font::use_embedded_fonts_only();
    let room = PREVIEW_RECT.w - STATUS_PAD * 2.0;
    let long = format!("play7.luaskin (body pass): lua/play/{}field.lua:120: attempt to index a nil value (field 'judge')", "deep/".repeat(40));
    let statuses = [
        PreviewStatus::Absent(PreviewAbsence::NoDocument),
        PreviewStatus::Absent(PreviewAbsence::UnsupportedScreen),
        PreviewStatus::Absent(PreviewAbsence::NoPreview),
        PreviewStatus::PackTrouble("decide.luaskin DID NOT READ (header pass): decide.luaskin:1: '}' expected near 'w' (AND 2 MORE)".to_owned()),
        PreviewStatus::Failed("decide.luaskin (body pass): decide.luaskin:3: the sample skin gives up".to_owned()),
        PreviewStatus::Failed(long),
    ];
    for status in &statuses {
        let lines = status_detail_lines(status);
        assert!(!lines.is_empty() && lines.len() <= STATUS_DETAIL_LINES, "{status:?}: {lines:?}");
        assert!(lines.iter().all(|line| text_width(line, STATUS_DETAIL_SCALE) <= room + 1.0), "{lines:?}");
    }
    assert_eq!(
        status_detail_lines(&statuses[4]).concat().replace(' ', ""),
        statuses[4].detail().unwrap_or_default().replace(' ', ""),
        "a reason that fits lost something"
    );
    assert_eq!(status_detail_lines(&statuses[5]).len(), STATUS_DETAIL_LINES);
    assert!(status_detail_lines(&PreviewStatus::Loading).is_empty() && status_detail_lines(&PreviewStatus::Drawn).is_empty());
    let lowest = PREVIEW_RECT.h * 0.5 + (STATUS_DETAIL_LINES - 1) as f32 * STATUS_DETAIL_PITCH + INFO_LINE_H;
    assert!(lowest <= PREVIEW_RECT.h - STATUS_PAD, "the last line of a reason runs off the preview");
}

/// A press reaches the menu through the application's own routing: the preview leaves nothing of a
/// skin for the pointer to be judged against, so every press is the menu's.
#[test]
fn a_press_routed_by_the_application_reaches_the_menu() {
    let mut app = app_with_long_document("routed");
    app.switch(Transition::Open(SkinMenuState::stage()));
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let now = std::time::Instant::now();
    for _ in 0..2 {
        app.shared.hot.clear();
        app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
        let mut canvas = Canvas::Headless(&mut pixels);
        app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, &mut canvas);
        app.shared.finish_skin_frame(&mut canvas);
    }
    let (rect, _) = *app.shared.hot.iter().find(|(_, hot)| *hot == Hot::SkinMenuKind(0)).expect("the strip is on the screen");
    let press = PointerInput::Button { button: MouseButton::Left, pressed: true };
    app.stage.handle_pointer(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, middle(rect), press);
    assert_eq!(app.shared.config.skin.screen, SCREEN_KINDS[0].0, "the press did not reach the strip");

    let skin = app.shared.config.skin.clone();
    app.stage.handle_pointer(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, middle(PREVIEW_RECT), press);
    assert_eq!(app.shared.config.skin, skin, "a press on the preview changed a setting");
    assert_eq!(app.stage.id(), StageId::SkinMenu);
}

#[test]
fn escape_goes_back_and_every_other_key_stays() {
    let mut menu = Menu::over(bare_app("escape"));
    for code in [KeyCode::Enter, KeyCode::ArrowDown, KeyCode::ArrowRight, KeyCode::Tab, KeyCode::F12, KeyCode::Digit0, KeyCode::PageDown, KeyCode::KeyA] {
        menu.state.pack_picker = None;
        assert!(matches!(menu.key(code), Transition::Stay), "{code:?}");
    }
    menu.state.pack_picker = None;
    assert!(matches!(menu.key_up(KeyCode::Escape), Transition::Stay), "a key coming up left the menu");
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back));
}

/// Tab steps the screen being configured along the strip and round its end, Shift+Tab the other
/// way, and from a screen the strip does not offer either lands on the strip's first.
#[test]
fn tab_steps_the_screen_along_the_strip_and_shift_steps_it_back() {
    let mut app = bare_app("tab");
    app.shared.config.skin.screen = SCREEN_KINDS[0].0;
    let mut menu = Menu::over(app);
    for (screen, name) in SCREEN_KINDS.iter().skip(1) {
        menu.key(KeyCode::Tab);
        assert_eq!(menu.app.shared.config.skin.screen, *screen, "Tab did not reach {name}");
    }
    menu.key(KeyCode::Tab);
    assert_eq!(menu.app.shared.config.skin.screen, SCREEN_KINDS[0].0, "the strip does not come back round");

    menu.key(KeyCode::ShiftLeft);
    menu.key(KeyCode::Tab);
    assert_eq!(menu.app.shared.config.skin.screen, SCREEN_KINDS[SCREEN_KINDS.len() - 1].0, "shift did not turn the step round");
    menu.key(KeyCode::Tab);
    assert_eq!(menu.app.shared.config.skin.screen, SCREEN_KINDS[SCREEN_KINDS.len() - 2].0);
    menu.key_up(KeyCode::ShiftLeft);

    for held in [false, true] {
        menu.app.shared.config.skin.screen = SKIN_TYPE_PLAY_24KEYS;
        if held {
            menu.key(KeyCode::ShiftRight);
        }
        menu.key(KeyCode::Tab);
        assert_eq!(menu.app.shared.config.skin.screen, SCREEN_KINDS[0].0, "a screen off the strip did not step onto its first tab");
        menu.key_up(KeyCode::ShiftRight);
    }
}

/// A screen the strip does not offer is reached with the SCREEN row, is named in the head, and
/// lights no tab.
#[test]
fn a_screen_off_the_strip_is_reached_with_the_row_and_lights_no_tab() {
    let mut app = bare_app("off-strip");
    app.shared.config.skin.screen = SKIN_TYPE_PLAY_24KEYS - 1;
    let mut menu = Menu::over(app);
    menu.focus_fixed(SettingId::SkinScreen);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.app.shared.config.skin.screen, SKIN_TYPE_PLAY_24KEYS);
    assert_eq!(menu.value_of_fixed(SettingId::SkinScreen), "PLAY 24KEYS");
    menu.frame();
    for (place, (_, name)) in SCREEN_KINDS.iter().enumerate() {
        let rect = kind_rect(place);
        assert_eq!(pixel(&menu.pixels, (rect.x + 1.0, rect.y + 1.0)), theme().panel, "{name} is lit");
    }
    menu.save("skin-menu-off-strip");

    menu.key(KeyCode::Tab);
    menu.frame();
    let first = kind_rect(0);
    assert_eq!(pixel(&menu.pixels, (first.x + 1.0, first.y + 1.0)), theme().button, "the screen being configured is not lit on the strip");
}

/// The menu is opened over the screen it was asked for from, and Escape goes back to that screen.
#[test]
fn the_menu_opens_over_the_browser_and_goes_back_to_it() {
    let mut app = bare_app("history");
    app.switch(Transition::Open(SkinMenuState::stage()));
    assert_eq!(app.stage.id(), StageId::SkinMenu);
    let now = std::time::Instant::now();
    let asked = app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(KeyCode::Escape));
    app.switch(asked);
    assert_eq!(app.stage.id(), StageId::Select);
    assert!(app.suspended.is_empty(), "the browser was left parked under a menu that is gone");
}

/// Entering the menu walks the skin folder, so a document dropped into it while the game was running
/// is on offer without a restart.
#[test]
fn entering_the_menu_finds_a_document_dropped_into_the_folder() {
    let mut app = bare_app("dropped");
    app.shared.config.skin.screen = fixtures::MUSIC_SELECT;
    let folder = fixtures::write(&app.shared.settings_path);
    app.shared.config.skin.folder = Some(folder.to_string_lossy().into_owned());
    let mut menu = Menu::over(app);
    menu.choose_first_document();
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "Browser", "the document in the folder was not found on the way in");
}

/// The shipped pack names a document for the screens it draws, and none for the key configuration:
/// the menu says which screen it is on and what is chosen for it, and says so where the preview
/// would be for the screen that has no document.
#[test]
fn a_menu_over_the_shipped_pack_names_the_screen_and_what_is_chosen_for_it() {
    let mut app = app_with_pack("shipped", &shipped_pack());
    app.shared.config.skin.screen = SKIN_TYPE_KEY_CONFIG;
    let mut menu = Menu::over(app);
    let keys = menu.frame().pixel_checksum();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Absent(PreviewAbsence::NoDocument), "the shipped pack has no document for the key configuration");
    assert_eq!(menu.value_of_fixed(SettingId::SkinDocument), "DEFAULT");
    menu.save("skin-menu-shipped-keys");

    menu.key(KeyCode::Tab);
    assert_eq!(menu.app.shared.config.skin.screen, SKIN_TYPE_PLAY_7KEYS);
    assert!(menu.value_of_fixed(SettingId::SkinDocument).starts_with("PACK: "), "the pack's document is not named for the seven key screen");
    assert!(menu.state.rows.len() > FIXED_ROWS.len(), "the pack's document declares no row of its own");
    assert_ne!(keys, menu.frame().pixel_checksum(), "the screen being configured moved and the menu drew the same frame");
}

/// How far into its scene a screen is captured, in seconds: a run far enough for notes to be falling
/// and figures to have moved, a list far enough to have turned twice, a scene that only opens far
/// enough to be open.
const RUN_SECONDS: f32 = 8.0;
const LIST_SECONDS: f32 = 3.5;
const DECIDE_SECONDS: f32 = 2.0;
const RESULT_SECONDS: f32 = 3.0;

/// A capture of a scene where it stands, moved on by nothing.
const WHERE_IT_STANDS: f32 = 0.0;

/// The screens the shipped pack draws, each with the name its capture is saved under and how far
/// into its scene it is captured.
const CAPTURED_SCREENS: [(i32, &str, f32); 9] = [
    (SKIN_TYPE_PLAY_7KEYS, "7k", RUN_SECONDS),
    (SKIN_TYPE_PLAY_5KEYS, "5k", RUN_SECONDS),
    (SKIN_TYPE_PLAY_14KEYS, "14k", RUN_SECONDS),
    (SKIN_TYPE_PLAY_10KEYS, "10k", RUN_SECONDS),
    (SKIN_TYPE_PLAY_9KEYS, "9k", RUN_SECONDS),
    (SKIN_TYPE_MUSIC_SELECT, "select", LIST_SECONDS),
    (SKIN_TYPE_DECIDE, "decide", DECIDE_SECONDS),
    (SKIN_TYPE_RESULT, "result", RESULT_SECONDS),
    (SKIN_TYPE_COURSE_RESULT, "course", RESULT_SECONDS),
];

/// How far a capture moves a scene on at a time without drawing it, in seconds: short enough that a
/// sample run is handed its chart a note at a time.
const CAPTURE_STEP_SECONDS: f32 = 0.05;

impl Menu {
    /// Draw the menu once its preview has said what is there and its scene stands `seconds` in, and
    /// save the frame. The scene is moved on without being drawn, so a capture costs a handful of
    /// frames however far in it is taken.
    fn capture_at(&mut self, name: &str, seconds: f32) {
        self.until_said();
        for _ in 0..(seconds / CAPTURE_STEP_SECONDS) as usize {
            self.state.update(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: CAPTURE_STEP_SECONDS });
        }
        self.until_said();
        self.save(name);
    }

    /// Configure the screen at one place of the strip, the way a press on its tab does.
    fn choose(&mut self, screen: i32) {
        let place = kind_index(screen).expect("the screen is on the strip");
        self.frame();
        self.click(self.middle_of_hot(Hot::SkinMenuKind(place)));
    }
}

/// The menu over the shipped pack, drawn for a person to look at: every screen the pack draws with
/// its scene under way, an option before and after it is stepped, an offset nudged, a screen the
/// pack has no document for, and one screen at a larger window. Draws nothing without a capture
/// folder.
#[test]
fn the_menu_over_the_shipped_pack_can_be_captured() {
    if !capturing() {
        return;
    }
    let mut menu = Menu::over(app_with_pack("capture", &shipped_pack()));
    for (screen, name, seconds) in CAPTURED_SCREENS {
        menu.choose(screen);
        menu.state.focus(0);
        menu.capture_at(&format!("skin-menu-shipped-{name}"), seconds);
        assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "{name}: {:?}", menu.app.shared.skin_failure(screen));
    }

    menu.choose(SKIN_TYPE_PLAY_7KEYS);
    let option = menu.state.rows.iter().position(|row| matches!(row, SettingRow::Skin(SkinRow::Property(_)))).expect("the play skin has an option");
    menu.state.focus(option);
    menu.capture_at("skin-menu-shipped-7k-option-before", RUN_SECONDS);
    menu.key(KeyCode::ArrowRight);
    menu.capture_at("skin-menu-shipped-7k-option-after", WHERE_IT_STANDS);
    menu.key(KeyCode::ArrowLeft);

    let whole = SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::X));
    menu.focus(whole);
    menu.key(KeyCode::ShiftLeft);
    menu.key(KeyCode::ArrowRight);
    menu.key_up(KeyCode::ShiftLeft);
    menu.capture_at("skin-menu-shipped-7k-offset", WHERE_IT_STANDS);
    menu.key(KeyCode::Digit0);

    menu.choose(SKIN_TYPE_KEY_CONFIG);
    menu.state.focus(0);
    menu.capture_at("skin-menu-shipped-keys-no-document", WHERE_IT_STANDS);
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Absent(PreviewAbsence::NoDocument));

    let mut large = Menu::sized(app_with_pack("capture-large", &shipped_pack()), LARGE);
    large.choose(SKIN_TYPE_PLAY_7KEYS);
    large.capture_at("skin-menu-shipped-7k", RUN_SECONDS);
}

/// What the menu shows where the preview would be when there is no frame of a skin to show, drawn
/// for a person to look at: a pack folder that is not there, a skin of the pack that does not
/// parse, and a skin that gives up as it is read. Each says so in the preview's place and on the
/// LOADED line under it, and nothing written runs into anything else.
#[test]
fn what_the_menu_says_in_place_of_a_frame_can_be_captured() {
    let mut gone = bare_app("capture-gone");
    let missing = gone.shared.settings_path.parent().expect("the settings file has a folder").join("a-pack-that-is-not-there");
    gone.shared.set_skin_pack(Some(missing.to_string_lossy().into_owned()));
    gone.shared.config.skin.screen = SKIN_TYPE_DECIDE;
    let mut menu = Menu::over(gone);
    assert!(matches!(menu.until_said(), PreviewStatus::PackTrouble(_)), "{:?}", menu.state.preview.status());
    assert!(menu.value_of_fixed(SettingId::SkinPack).starts_with("(MISSING)"));
    menu.frame();
    menu.save("skin-menu-pack-folder-missing");

    let mut menu = Menu::over(app_with_decide_skin("capture-unparsed", "local skin = { type = 6, name = 'Unparsed' w = 1280 \nreturn skin +"));
    let status = menu.until_said();
    assert!(matches!(&status, PreviewStatus::PackTrouble(trouble) if trouble.starts_with("decide.luaskin")), "{status:?}");
    menu.frame();
    assert!(menu.value_of_fixed(SettingId::SkinInfo).contains("decide.luaskin"), "{}", menu.value_of_fixed(SettingId::SkinInfo));
    menu.save("skin-menu-skin-unparsed");

    let mut menu = Menu::over(app_with_decide_skin("capture-gives-up", GIVES_UP));
    assert!(matches!(menu.until_said(), PreviewStatus::Failed(_)));
    menu.state.focus(0);
    menu.frame();
    menu.save("skin-menu-skin-gives-up");
}

/// The long list drawn for a person to look at: at its top, scrolled to its middle with the cursor
/// on an option, and at its foot with the cursor on an offset, at both window sizes.
#[test]
fn a_list_longer_than_the_screen_can_be_captured() {
    for size in [UI, LARGE] {
        let mut menu = Menu::sized(app_with_long_document(&format!("capture-long-{}", size.0)), size);
        menu.frame();
        menu.save("skin-menu-long-top");
        let middle_option = SettingRow::Skin(SkinRow::Property(LONG_OPTIONS / 2));
        menu.focus(middle_option);
        menu.key(KeyCode::ArrowRight);
        menu.frame();
        assert!(menu.rows_on_show().contains(&menu.at(middle_option)), "the cursor's row is not drawn");
        menu.save("skin-menu-long-scrolled");
        let offset = SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::Y));
        menu.focus(offset);
        menu.key(KeyCode::ArrowLeft);
        menu.frame();
        assert!(menu.rows_on_show().contains(&menu.at(offset)), "the cursor's row is not drawn");
        menu.save("skin-menu-long-offset");
    }
}

/// The menu over the pack [`SKIN_PACK_ENV`] names, drawn for a person to look at: a pack somebody
/// else wrote, whose documents declare far more rows than the shipped one's. Its browser and its
/// seven-key play screen with their scenes under way, the list at its middle and its foot, and one
/// screen at a larger window. Opt-in, because the pack is outside the repository: without the
/// variable and a capture folder this draws nothing.
#[test]
fn the_menu_over_the_pack_the_environment_names_can_be_captured() {
    let Some(pack) = pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    if !capturing() {
        return;
    }
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let mut menu = Menu::over(app_with_pack("capture-env", &pack));
    menu.choose(SKIN_TYPE_PLAY_7KEYS);
    menu.state.focus(0);
    menu.capture_at("skin-menu-pack-7k", RUN_SECONDS);
    menu.state.focus(menu.state.rows.len() / 2);
    menu.capture_at("skin-menu-pack-7k-middle", WHERE_IT_STANDS);
    menu.state.focus(menu.state.rows.len().saturating_sub(1));
    menu.capture_at("skin-menu-pack-7k-foot", WHERE_IT_STANDS);
    menu.choose(SKIN_TYPE_MUSIC_SELECT);
    menu.state.focus(0);
    menu.capture_at("skin-menu-pack-select", LIST_SECONDS);
    menu.choose(SKIN_TYPE_PLAY_9KEYS);
    menu.capture_at("skin-menu-pack-9k", WHERE_IT_STANDS);

    let mut large = Menu::sized(app_with_pack("capture-env-large", &pack), LARGE);
    large.choose(SKIN_TYPE_MUSIC_SELECT);
    large.capture_at("skin-menu-pack-select", LIST_SECONDS);
}
