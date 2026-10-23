//! The play menu as a screen: where it is laid out, which rows it lists and what each reads, what
//! the keys and the mouse do to them, what the mode and the tempo of its preview do, that neither is
//! a setting, and how the menu is opened and left.
//!
//! The preview on the right is another module's. What is held here about it is what the menu asks it
//! to show, that it comes back after a row moved, and that the menu fills its place when it says it
//! drew nothing.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, the frames these tests draw are also saved there as PNGs
//! a person can open.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rbms_config::{
    Config, GAUGE_CYCLE, HISPEED_MAX, HeldTravel, JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, SettingId, SettingTab, cycle_values, descriptor, green_of_duration,
    held_speed, tab_rows, travel_duration_ms,
};
use rbms_render::{Rect, fit_text, text_width};
use rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT;

use super::rows::{Fold, GROUP_PREVIEW, GROUP_SKIN, declared, holds_a_number};
use super::*;
use crate::keyconfig::Shortcut;
use crate::skin_preview::{PreviewAbsence, PreviewStatus, PreviewTarget};
use crate::skin_select::{SKIN_PACK_ENV, SkinLibrary, pack_from_environment};
use crate::stage::preview_menu::{
    FIGURE_NOTE_SCALE, HINT_SCALE, INFO_PAD, INFO_RECT, INFO_SCALE, LABEL_GAP, LABEL_SCALE, MARGIN, PREVIEW_RECT, ROW_PAD, TITLE_Y, VALUE_SCALE, figure_width,
    figure_x, head_subtitle, number_label_room, number_value_rect, number_value_room, row_room, slider_rect, tab_rect,
};
use crate::stage::render_tests::{FRAME_DT, app};
use crate::stage::select::PLAY_MENU_KEY;
use crate::stage::{HeadlessCanvas, SelectState, StageId};
use crate::{App, CH, CW, gauge_name};

/// Environment variable naming the folder captures are saved in, the one the screen capture harness
/// reads. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The skin pack this repository ships, relative to the player's manifest.
const DEFAULT_PACK: &str = "../../assets/skins/rbms-default";

/// The size the menu is laid out for, and a window half as large again.
const UI: (u32, u32) = (CW, CH);
const LARGE: (u32, u32) = (1920, 1080);

/// How many frames a test waits for its preview to say something other than that the document is on
/// its way, and how long it pauses between two of them.
const SETTLE_FRAMES: usize = 600;
const SETTLE_PAUSE: Duration = Duration::from_millis(10);

/// How far into its scene a preview is captured, in seconds: far enough for the sample run to be
/// past its loading state with notes on the lanes.
const RUN_SECONDS: f32 = 8.0;

/// How far a capture moves a scene on at a time without drawing it, in seconds: short enough that a
/// sample run is handed its chart a note at a time.
const CAPTURE_STEP_SECONDS: f32 = 0.05;

/// How many steps Shift moves a row that holds a number by, as the whole number a setting is counted
/// in.
const SHIFT_STEPS: i32 = crate::stage::preview_menu::SHIFT_STEPS as i32;

/// The least room kept between what one figure of the summary ends with and where the next begins.
const FIGURE_ROOM: f32 = 8.0;

/// The option of the shipped play skin the tests step, as the list names it.
const LANE_WIDTH_OPTION: &str = "LANE WIDTH";

/// The rows of the settings table the menu lists, group by group in the order it lists them.
const SPEED_ROWS: [SettingId; 6] =
    [SettingId::KeepGreenNumber, SettingId::GreenNumber, SettingId::HiSpeed, SettingId::HiSpeedStep, SettingId::FixHiSpeed, SettingId::SpeedFix];
const LANE_ROWS: [SettingId; 7] = [
    SettingId::EnableCover,
    SettingId::LaneCover,
    SettingId::EnableLift,
    SettingId::Lift,
    SettingId::EnableHidden,
    SettingId::Hidden,
    SettingId::LaneCoverFineStep,
];
const JUDGE_ROWS: [SettingId; 2] = [SettingId::JudgeOffset, SettingId::AutoCal];
const CHART_ROWS: [SettingId; 10] = [
    SettingId::Random,
    SettingId::Gauge,
    SettingId::LegacyNote,
    SettingId::Target,
    SettingId::TargetFallback,
    SettingId::TargetRate,
    SettingId::Autoplay,
    SettingId::ScratchAuto,
    SettingId::AutoReplay,
    SettingId::PlayEscapeMode,
];

fn press(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text: None }
}

fn release(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: true, text: None }
}

/// An app whose settings live in a folder of this test's own, with no skin chosen and none
/// installed beside the settings file.
fn bare_app(tag: &str) -> App {
    let mut app = app();
    let root = std::env::temp_dir().join(format!("rbms-play-menu-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the fixture folder is writable");
    app.shared.settings_path = root.join("settings.ron");
    app.shared.config.skin = rbms_config::SkinOptions::default();
    app.shared.skins = SkinLibrary::new(&app.shared.settings_path, &app.shared.config);
    app
}

/// The same app with the shipped pack named as its pack folder.
fn app_with_shipped_pack(tag: &str) -> App {
    let mut app = bare_app(tag);
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK);
    assert!(pack.is_dir(), "the skin pack is not at {}", pack.display());
    app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    app
}

/// The menu over an app, entered, with the one canvas all of its frames are drawn on.
struct Menu {
    app: App,
    state: PlayMenuState,
    pixels: HeadlessCanvas,
    size: (u32, u32),
}

impl Menu {
    fn over(app: App) -> Menu {
        Menu::sized(app, UI)
    }

    fn sized(mut app: App, size: (u32, u32)) -> Menu {
        let mut state = PlayMenuState::new();
        state.on_enter(&mut FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 });
        Menu { app, state, pixels: HeadlessCanvas::new(size.0, size.1), size }
    }

    /// One frame, as the application makes one: the screen is brought up to date and drawn, and the
    /// skin frame is finished.
    fn frame(&mut self) -> &HeadlessCanvas {
        self.frame_at(std::time::Instant::now())
    }

    /// The same frame made at `now`, which is the time everything on it that is shown for a while
    /// is judged by, however long ago the machine passed it.
    fn frame_at(&mut self, now: std::time::Instant) -> &HeadlessCanvas {
        self.app.shared.hot.clear();
        self.state.update(&mut FrameCtx { shared: &mut self.app.shared, now, dt: FRAME_DT });
        let mut canvas = Canvas::Headless(&mut self.pixels);
        self.state.draw(&mut FrameCtx { shared: &mut self.app.shared, now, dt: FRAME_DT }, &mut canvas);
        self.app.shared.finish_skin_frame(&mut canvas);
        &self.pixels
    }

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

    /// Draw the menu once its preview has said what is there and its scene stands `seconds` in, and
    /// save the frame. The scene is moved on without being drawn, so a capture costs a handful of
    /// frames however far in it is taken.
    fn capture_at(&mut self, name: &str, seconds: f32) -> PreviewStatus {
        self.until_said();
        for _ in 0..(seconds / CAPTURE_STEP_SECONDS) as usize {
            self.state.update(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: CAPTURE_STEP_SECONDS });
        }
        let status = self.until_said();
        self.save(name);
        status
    }

    fn key(&mut self, code: KeyCode) -> Transition {
        self.key_at(code, std::time::Instant::now())
    }

    /// A key pressed at `now`.
    fn key_at(&mut self, code: KeyCode, now: std::time::Instant) -> Transition {
        self.state.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now, dt: 0.0 }, press(code))
    }

    /// A key pressed with Shift held, and Shift let go of after it.
    fn shifted(&mut self, code: KeyCode) {
        self.key(KeyCode::ShiftLeft);
        self.key(code);
        self.state.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, release(KeyCode::ShiftLeft));
    }

    /// A press of a button at a point of the screen, judged against the frame drawn last.
    fn click(&mut self, at: (f32, f32), button: MouseButton) -> Transition {
        self.state.handle_mouse(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, at, button, true)
    }

    /// A turn of the wheel with the pointer at a point of the screen.
    fn wheel(&mut self, at: (f32, f32), lines: f32) {
        self.app.shared.cursor = at;
        self.state.handle_scroll(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, lines);
    }

    /// Where one row is in the list.
    fn at(&self, row: Row) -> usize {
        self.state.rows.iter().position(|entry| *entry == row).unwrap_or_else(|| panic!("{row:?} is not in the list"))
    }

    /// Put the cursor on one row of the list.
    fn focus(&mut self, row: Row) {
        let at = self.at(row);
        self.state.focus(at, ROW_STEP_FORWARD);
        assert_eq!(self.state.focused(), Some(row));
    }

    fn focus_setting(&mut self, id: SettingId) {
        self.focus(Row::Setting(id));
    }

    /// One row as the list writes it now.
    fn line(&self, row: Row) -> Line {
        let readout = self.state.target().readout(&self.app.shared.config.play);
        self.state.line(&self.app.shared, &readout, row)
    }

    fn value(&self, id: SettingId) -> String {
        self.line(Row::Setting(id)).value
    }

    fn look(&self, id: SettingId) -> RowLook {
        self.line(Row::Setting(id)).look
    }

    /// The option of the previewed play skin whose label ends with `name`.
    fn skin_option(&self, name: &str) -> Row {
        let found = self.state.rows.iter().copied().find(|row| matches!(row, Row::Skin(_)) && self.line(*row).label.ends_with(name));
        found.unwrap_or_else(|| panic!("the list has no option of the skin called {name}"))
    }

    /// The middle of what the frame drawn last made clickable for `hot`.
    fn middle_of_hot(&self, hot: Hot) -> (f32, f32) {
        middle(self.rect_of_hot(hot))
    }

    /// Where the frame drawn last made `hot` clickable.
    fn rect_of_hot(&self, hot: Hot) -> Rect {
        let (rect, _) = self.app.shared.hot.iter().find(|(_, entry)| *entry == hot).unwrap_or_else(|| panic!("{hot:?} is not on the screen"));
        *rect
    }

    /// A point on the label of one row as the frame drawn last drew it: on the row, and on neither
    /// the slider nor the number of a row that holds a number.
    fn on_the_label_of(&self, row: Row) -> (f32, f32) {
        let rect = self.rect_of_hot(Hot::PlayMenuRow(self.at(row)));
        (rect.x + ROW_PAD, rect.y + rect.h * 0.5)
    }

    /// Where the frame drawn last drew the slider of one row that holds a number.
    fn slider_of(&self, row: Row) -> Rect {
        slider_rect(self.rect_of_hot(Hot::PlayMenuRow(self.at(row))))
    }

    /// The middle of where the frame drawn last wrote the number of one row that holds one.
    fn on_the_number_of(&self, row: Row) -> (f32, f32) {
        middle(number_value_rect(self.rect_of_hot(Hot::PlayMenuRow(self.at(row)))))
    }

    /// The pointer moved to a point of the screen with a button held.
    fn drag(&mut self, at: (f32, f32)) {
        self.state.handle_mouse_drag(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, at);
    }

    /// The left button let go of at a point of the screen.
    fn let_go(&mut self, at: (f32, f32)) {
        self.state.handle_mouse(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, at, MouseButton::Left, false);
    }

    /// One key pressed that types `text`.
    fn typed(&mut self, text: &'static str) -> Transition {
        let input = KeyInput { code: KeyCode::KeyA, pressed: true, released: false, text: Some(text) };
        self.state.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now: std::time::Instant::now(), dt: 0.0 }, input)
    }

    /// A key pressed as many times over as it is asked.
    fn keys(&mut self, code: KeyCode, presses: usize) {
        for _ in 0..presses {
            self.key(code);
        }
    }

    /// The step one row that holds a number stands on.
    fn steps_of(&self, row: Row) -> i64 {
        self.state.number_of(&self.app.shared, row).unwrap_or_else(|| panic!("{row:?} holds no number")).steps
    }

    /// The rows the frame drawn last made clickable, top to bottom.
    fn rows_on_show(&self) -> Vec<usize> {
        self.app
            .shared
            .hot
            .iter()
            .filter_map(|(_, hot)| match hot {
                Hot::PlayMenuRow(index) => Some(*index),
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

fn middle(rect: Rect) -> (f32, f32) {
    (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

/// The list the menu shows with no option of a skin in it: the tempo of the sample chart, then the
/// four groups of the settings table, each under its name, with the colour SCRATCH AUTO draws in --
/// its row and its closed fold of channels -- under the row of SCRATCH AUTO.
fn rows_without_skin_options() -> Vec<Row> {
    let groups: [(&str, &[SettingId]); 4] = [("SPEED", &SPEED_ROWS), ("LANE", &LANE_ROWS), ("JUDGE", &JUDGE_ROWS), ("CHART", &CHART_ROWS)];
    let mut rows = vec![Row::Group(GROUP_PREVIEW), Row::PreviewBpm];
    for (name, ids) in groups {
        rows.push(Row::Group(name));
        for id in ids {
            rows.push(Row::Setting(*id));
            if *id == SettingId::ScratchAuto {
                rows.extend([Row::AutoColor, Row::Fold(Fold::AutoChannels)]);
            }
        }
    }
    rows
}

/// How many entries the offsets of the play skin of `screen` add to the end of the list while both
/// of their folds are closed: the name of their group and a row for each fold the skin declares
/// something for, or nothing at all for a skin that declares no offset.
fn closed_skin_group(menu: &Menu, screen: i32) -> usize {
    let folds = Fold::ALL.into_iter().filter(|fold| !folded(&menu.app.shared, screen, *fold).is_empty()).count();
    if folds == 0 { 0 } else { folds + 1 }
}

/// The figures of a summary as one line of words: each under its name with what follows it, and
/// what is said under them after the last.
fn said(readout: &SpeedReadout, constant: bool) -> String {
    let figures = summary(readout, constant).map(|figure| [figure.name, figure.value.as_str(), figure.tag].join(" ").trim_end().to_string());
    figures.into_iter().chain(summary_note(readout).map(str::to_string)).collect::<Vec<_>>().join("   ")
}

/// The strip of modes is as wide as the preview, and the tempos are listed slowest first with the
/// one the menu opens on among them.
#[test]
fn the_strip_of_modes_is_as_wide_as_the_preview() {
    rbms_render::font::use_embedded_fonts_only();
    let count = PREVIEW_MODES.len();
    let (first, last) = (tab_rect(0, count), tab_rect(count - 1, count));
    assert!((first.x - PREVIEW_RECT.x).abs() < 0.01 && (last.x + last.w - (PREVIEW_RECT.x + PREVIEW_RECT.w)).abs() < 0.01);
    assert!(PREVIEW_BPMS.is_sorted() && OPENING_BPM < PREVIEW_BPMS.len());
}

/// The list is the tempo of the sample chart and the four groups of the settings table, each row
/// under the name of its group, in the order the design lists them.
#[test]
fn the_list_is_the_tempo_row_and_the_four_groups_under_their_names() {
    let menu = Menu::over(bare_app("rows"));
    assert_eq!(menu.state.rows, rows_without_skin_options());
    assert_eq!(menu.state.focused(), Some(Row::PreviewBpm), "the cursor does not open on the first row");
    let labels: Vec<String> = menu.state.rows.iter().map(|row| menu.line(*row).label).collect();
    assert_eq!(
        &labels[..9],
        ["PREVIEW", "PREVIEW BPM", "SPEED", "KEEP GREEN NUMBER", "GREEN NUMBER", "HI-SPEED", "HI-SPEED STEP", "HI-SPEED FIX", "SPEED FIX"]
    );
    assert_eq!(&labels[9..17], ["LANE", "LANE COVER ON", "LANE COVER", "LIFT ON", "LIFT", "HIDDEN+ ON", "HIDDEN+", "COVER FINE STEP"]);
    assert_eq!(&labels[17..20], ["JUDGE", "JUDGE OFFSET", "AUTO ADJUST"]);
    assert_eq!(
        &labels[20..],
        [
            "CHART",
            "RANDOM",
            "GAUGE",
            "LEGACY NOTE",
            "TARGET",
            "TARGET FALLBACK",
            "TARGET RATE",
            "AUTOPLAY",
            "SCRATCH AUTO",
            "SCRATCH AUTO COLOR",
            "SCRATCH AUTO CHANNELS",
            "AUTO REPLAY",
            "ESCAPE"
        ]
    );
    for row in &menu.state.rows {
        assert_eq!(menu.line(*row).look == RowLook::Heading, row.is_heading(), "{row:?}");
    }
}

/// Every row the settings table gives the play menu is listed once, a row nothing acts on yet is
/// not, and no row of another screen's is.
#[test]
fn every_row_of_the_play_menus_groups_is_listed_once_and_no_other() {
    let menu = Menu::over(bare_app("table"));
    let config = Config::default();
    let offered: Vec<SettingId> = SettingTab::PLAY_MENU.iter().flat_map(|tab| tab_rows(*tab, &config)).collect();
    let listed: Vec<SettingId> = menu
        .state
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Setting(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(listed, offered);
    assert!(!listed.contains(&SettingId::LaneOption), "a row nothing acts on is offered");
    for id in &listed {
        assert!(SettingTab::PLAY_MENU.contains(&descriptor(*id).tab), "{id:?} is a row of {:?}", descriptor(*id).tab);
    }
}

/// Every row the menu lists can be put back: a row stepped away from what it ships as comes back to
/// it, which holds the list of rows the zero key knows against the rows the table lists.
#[test]
fn every_listed_row_of_the_table_is_one_the_zero_key_puts_back() {
    let shipped = Config::default();
    for tab in SettingTab::PLAY_MENU {
        for id in tab_rows(tab, &shipped) {
            let mut config = shipped.clone();
            assert!(step_setting(&mut config, id, ROW_STEP_FORWARD, false), "{id:?} does not step");
            assert_ne!(config, shipped, "{id:?}");
            assert!(reset_setting(&mut config, id), "{id:?} is not put back");
            assert_eq!(config, shipped, "{id:?}");
            assert!(!reset_setting(&mut config, id), "{id:?} is reported moved when it was already where it ships");
        }
    }
    assert!(!reset_setting(&mut shipped.clone(), SettingId::Bga), "a row of another screen is put back from here");
}

/// With no skin installed the menu draws its own half and says in the preview's place that the mode
/// has no document.
#[test]
fn a_menu_with_no_skin_draws_its_half_and_says_what_is_missing() {
    let mut menu = Menu::over(bare_app("bare"));
    menu.frame();
    assert_eq!(menu.until_said(), PreviewStatus::Absent(PreviewAbsence::NoDocument));
    assert!(menu.pixels.painted_pixels() > 0);
    let tabs = menu.app.shared.hot.iter().filter(|(_, hot)| matches!(hot, Hot::PlayMenuMode(_))).count();
    assert_eq!(tabs, PREVIEW_MODES.len(), "the strip is not on the screen");
    for rect in [LIST_RECT, INFO_RECT] {
        assert_eq!(menu.pixels.pixel_at(rect.x as u32 + 1, rect.y as u32 + 1), theme().panel, "{rect:?} is not drawn");
    }
    assert_eq!(menu.pixels.pixel_at(PREVIEW_RECT.x as u32 + 1, PREVIEW_RECT.y as u32 + 1), theme().panel, "the status is not said on a panel");
    menu.save("play-menu-no-skin");
}

/// Over the shipped pack the preview draws the play screen of the mode on the strip, and goes on
/// drawing it when the tempo and the mode are stepped.
#[test]
fn the_shipped_pack_is_previewed_on_the_mode_and_the_tempo_on_show() {
    let mut menu = Menu::over(app_with_shipped_pack("shipped"));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(PREVIEW_MODES[0].0));
    assert_eq!(menu.capture_at("play-menu-default", RUN_SECONDS), PreviewStatus::Drawn);

    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.state.preview.target(), PreviewTarget::Play(PlayTarget { screen: PREVIEW_MODES[0].0, bpm: PREVIEW_BPMS[OPENING_BPM + 1] }));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "the preview did not come back at another tempo");

    menu.key(KeyCode::Tab);
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(PREVIEW_MODES[1].0));
    assert_eq!(menu.capture_at("play-menu-5k-180", RUN_SECONDS), PreviewStatus::Drawn);
}

/// Over the shipped pack the options its play skin declares are listed at the end of the LANE
/// group, the ones of the mode on the strip, with the offsets it declares to move a part of its
/// own after them and neither a file slot, an opacity nor an offset every play skin has among
/// them; and the cursor stays on its row of the table when the mode -- and with it the rows above
/// that row -- changes.
#[test]
fn the_options_of_the_previewed_play_skin_are_listed_under_the_lane_rows() {
    let mut menu = Menu::over(app_with_shipped_pack("options"));
    let seven_keys = PREVIEW_MODES[0].0;
    let shared = &menu.app.shared;
    let all = shared.skins.rows_for(&shared.config, seven_keys);
    assert!(!all.is_empty(), "the shipped play skin declares no option");
    let after_lane = menu.at(Row::Setting(SettingId::LaneCoverFineStep)) + 1;
    let before_judge = menu.at(Row::Group("JUDGE"));
    let listed: Vec<Row> = menu.state.rows[after_lane..before_judge].to_vec();
    let is_listed_here = |row: &SkinRow| match shared.skins.kind_for(&shared.config, seven_keys, *row) {
        Some(SkinRowKind::Property) => true,
        Some(SkinRowKind::Offset(_)) => !shared.skins.is_automatic_for(&shared.config, seven_keys, *row),
        _ => false,
    };
    assert_eq!(listed, all.iter().copied().filter(is_listed_here).map(Row::Skin).collect::<Vec<_>>());
    assert_eq!(listed, declared(shared, seven_keys).into_iter().map(Row::Skin).collect::<Vec<_>>());
    let options = listed.iter().take_while(|row| matches!(row, Row::Skin(SkinRow::Property(_)))).count();
    assert!(listed[options..].iter().all(|row| matches!(row, Row::Skin(SkinRow::Offset(_, _)))), "an option is listed after an offset: {listed:?}");
    assert_eq!(menu.state.rows.len(), rows_without_skin_options().len() + listed.len() + closed_skin_group(&menu, seven_keys));
    assert_eq!(menu.line(menu.skin_option(LANE_WIDTH_OPTION)).value, "NORMAL");

    menu.focus_setting(SettingId::JudgeOffset);
    for place in (1..PREVIEW_MODES.len()).chain([0]) {
        menu.key(KeyCode::Tab);
        let screen = PREVIEW_MODES[place].0;
        let own = declared(&menu.app.shared, screen).len();
        assert_eq!(
            menu.state.rows.len(),
            rows_without_skin_options().len() + own + closed_skin_group(&menu, screen),
            "the list is not the one of mode {place}"
        );
        assert_eq!(menu.state.focused(), Some(Row::Setting(SettingId::JudgeOffset)), "the cursor left its row when the mode moved to {place}");
        assert!(menu.state.top <= menu.state.sel && menu.state.sel < menu.state.top + VISIBLE_ROWS, "the cursor is off the screen in mode {place}");
    }
}

/// An option of the previewed play skin is stepped and put back here as the skin menu does it: the
/// choice is stored for that document, which is read again and drawn, and the screen the skin menu
/// configures is not touched.
#[test]
fn an_option_of_the_play_skin_is_stored_for_its_document_and_the_preview_comes_back() {
    let mut app = app_with_shipped_pack("skin-option");
    app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    let mut menu = Menu::over(app);
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(PREVIEW_MODES[0].0));
    let before = menu.app.shared.config.clone();
    let screen = PREVIEW_MODES[0].0;
    let document = menu.app.shared.skins.document_path(&menu.app.shared.config, screen).expect("the pack has a seven-key play skin").to_owned();
    let option = menu.skin_option(LANE_WIDTH_OPTION);
    let Row::Skin(skin_row) = option else {
        panic!("{option:?} is not an option of a skin");
    };

    menu.focus(option);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.line(option).value, "WIDE");
    let stored = menu.app.shared.config.skin.customisation(&document).expect("the choice is stored for the document");
    assert_eq!(stored.properties.len(), 1, "{:?}", stored.properties);
    assert_eq!(menu.app.shared.config.skin.screen, SKIN_TYPE_MUSIC_SELECT, "the screen the skin menu configures moved");
    assert_eq!(menu.app.shared.skins.line_for(&menu.app.shared.config, screen, skin_row).1, "WIDE");
    let as_the_skin_menu_reads_it =
        Config { skin: rbms_config::SkinOptions { screen, ..menu.app.shared.config.skin.clone() }, ..menu.app.shared.config.clone() };
    assert_eq!(menu.app.shared.skins.line(&as_the_skin_menu_reads_it, skin_row).1, "WIDE", "the skin menu reads the option from somewhere else");
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "the preview did not come back with the option moved");
    assert!(!menu.app.shared.skins.needs_reload_for(&menu.app.shared.config, screen), "the document was not read again");
    menu.capture_at("play-menu-lane-width-wide", RUN_SECONDS);

    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.line(option).value, "NORMAL", "shift stepped an option of a skin more than once");
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.line(option).value, "WIDE");
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.line(option).value, "NORMAL", "the zero key did not put the option back");
    assert_eq!(menu.app.shared.config, before, "putting the option back left something stored");
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "the preview did not come back with the option put back");
}

/// Tab steps the mode along the strip and round its end, Shift+Tab the other way, and a press on a
/// tab goes to it. None of it is stored: the screen the skin menu configures stays what it was.
#[test]
fn the_mode_is_stepped_along_the_strip_and_is_no_setting() {
    let mut app = bare_app("mode");
    app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    let mut menu = Menu::over(app);
    let before = menu.app.shared.config.clone();
    menu.frame();
    for place in (1..PREVIEW_MODES.len()).chain([0]) {
        menu.key(KeyCode::Tab);
        assert_eq!(menu.state.target().screen, PREVIEW_MODES[place].0);
    }
    menu.shifted(KeyCode::Tab);
    assert_eq!(menu.state.target().screen, PREVIEW_MODES[PREVIEW_MODES.len() - 1].0, "shift did not turn the key round");
    assert!(!menu.state.shift_held, "shift is still held after it was let go of");

    menu.frame();
    let third = menu.middle_of_hot(Hot::PlayMenuMode(2));
    menu.click(third, MouseButton::Right);
    assert_eq!(menu.state.target().screen, PREVIEW_MODES[PREVIEW_MODES.len() - 1].0, "the right button moved the strip");
    menu.click(third, MouseButton::Left);
    assert_eq!(menu.state.target().screen, PREVIEW_MODES[2].0);
    menu.frame();
    let lit = tab_rect(2, PREVIEW_MODES.len());
    assert_eq!(menu.pixels.pixel_at(lit.x as u32 + 1, lit.y as u32 + 1), theme().button, "the mode on show is not lit on the strip");
    assert_eq!(menu.app.shared.config, before, "stepping the mode changed a setting");
}

/// The arrows, Enter, the zero key and the mouse move the tempo of the sample chart round its list.
/// It is no setting either, and a press on the preview or on a heading does nothing.
#[test]
fn the_tempo_is_stepped_round_its_list_and_is_no_setting() {
    let mut menu = Menu::over(bare_app("tempo"));
    let before = menu.app.shared.config.clone();
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM]);
    assert_eq!(menu.line(Row::PreviewBpm).value, "150 BPM");
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM - 1]);
    for _ in 0..PREVIEW_BPMS.len() {
        menu.key(KeyCode::ArrowRight);
    }
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM - 1], "a whole turn of the list did not come back to where it began");
    menu.shifted(KeyCode::ArrowLeft);
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM - 2], "shift stepped the tempo more than once");
    menu.key(KeyCode::Enter);
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM - 1]);
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM], "the zero key did not go back to the tempo the menu opens on");
    menu.key(KeyCode::ArrowLeft);

    menu.frame();
    let row = menu.middle_of_hot(Hot::PlayMenuRow(menu.at(Row::PreviewBpm)));
    menu.click(row, MouseButton::Left);
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM]);
    menu.click(row, MouseButton::Right);
    assert_eq!(menu.state.target().bpm, PREVIEW_BPMS[OPENING_BPM - 1]);
    let target = menu.state.target();
    menu.click(middle(PREVIEW_RECT), MouseButton::Left);
    assert!(!menu.app.shared.hot.iter().any(|(_, hot)| *hot == Hot::PlayMenuRow(0)), "a heading takes a press");
    assert_eq!(menu.state.target(), target, "a press on the preview moved something");
    assert_eq!(menu.state.preview.target(), PreviewTarget::Play(target), "the preview was not told the tempo on show");
    assert_eq!(menu.app.shared.config, before, "stepping the tempo changed a setting");
    assert_eq!(PlayMenuState::new().target(), target_at(0, OPENING_BPM), "a menu opened afresh does not begin where every menu does");
}

/// What is written of the scroll is what the options come to at the tempo on show: a held green
/// number stays where it is from one tempo to the next, and a stored speed does not hold it.
#[test]
fn a_held_green_number_is_the_same_at_every_tempo_on_the_list() {
    let mut menu = Menu::over(bare_app("held"));
    menu.app.shared.config.play.keep_green_number = true;
    let held = menu.state.target().readout(&menu.app.shared.config.play);
    menu.key(KeyCode::ArrowRight);
    let after = menu.state.target().readout(&menu.app.shared.config.play);
    assert_eq!(held.green_number, after.green_number, "the held number moved with the tempo");
    assert!(after.hispeed < held.hispeed, "a faster chart is not scrolled slower to hold the number");

    menu.app.shared.config.play.keep_green_number = false;
    let stored = menu.state.target().readout(&menu.app.shared.config.play);
    menu.key(KeyCode::ArrowLeft);
    let slower = menu.state.target().readout(&menu.app.shared.config.play);
    assert!(slower.green_number > stored.green_number, "a slower chart at the same speed does not read a larger number");
    assert!(said(&held, false).contains("(HELD)") && !said(&stored, false).contains("(HELD)"));
}

/// The summary under the preview is what the functions of the configuration crate come to at the
/// tempo on show -- the ones the play screen works its own speed out with -- at every tempo of the
/// list, held or not, covered or not.
#[test]
fn the_summary_is_what_the_configuration_crates_functions_come_to_at_every_tempo() {
    let mut menu = Menu::over(bare_app("summary"));
    menu.app.shared.config.play.cover = 0.3;
    for (keep_green_number, enable_cover) in [(false, true), (false, false), (true, true), (true, false)] {
        menu.app.shared.config.play.keep_green_number = keep_green_number;
        menu.app.shared.config.play.enable_cover = enable_cover;
        for _ in 0..PREVIEW_BPMS.len() {
            menu.key(KeyCode::ArrowRight);
            let play = menu.app.shared.config.play.clone();
            let bpm = f64::from(menu.state.target().bpm);
            let cover = enable_cover.then_some(play.cover);
            let (hispeed, duration_ms) = if keep_green_number {
                let held = held_speed(&HeldTravel::of(&play, bpm)).expect("a tempo of the list can be held");
                (held.hispeed, held.duration_ms)
            } else {
                (play.hispeed, travel_duration_ms(bpm, play.hispeed, cover))
            };
            let readout = menu.state.target().readout(&play);
            let said = said(&readout, play.constant_speed);
            let case = format!("keep {keep_green_number} cover {enable_cover} at {bpm}: {said}");
            let held = if keep_green_number { " (HELD)" } else { "" };
            assert!(said.starts_with(&format!("BPM {bpm}   HI-SPEED {hispeed:.2}{held}   TRAVEL {duration_ms} MS   ")), "{case}");
            assert!(
                said.ends_with(&format!("GREEN {}   WHITE 300{}", green_of_duration(duration_ms), if enable_cover { "" } else { " (COVER OFF)" })),
                "{case}"
            );
            assert_eq!(summary_note(&readout), None, "{case}");
            if keep_green_number {
                assert_eq!(duration_ms, play.duration_ms, "{case}");
            }
        }
    }

    menu.app.shared.config.play.constant_speed = true;
    let constant = menu.state.target().readout(&menu.app.shared.config.play);
    assert!(said(&constant, true).starts_with("BPM 120 (CONSTANT)   HI-SPEED "), "{}", said(&constant, true));
    menu.app.shared.config.play.duration_ms = 1;
    let limited = menu.state.target().readout(&menu.app.shared.config.play);
    assert!(summary_note(&limited).is_some_and(|note| note.starts_with("HI-SPEED IS AT THE END OF ITS RANGE")), "{:?}", summary_note(&limited));
    assert!(said(&limited, true).contains(&format!("   HI-SPEED {HISPEED_MAX:.2} (LIMIT)   ")), "{}", said(&limited, true));

    let figures = summary(&limited, true);
    assert_eq!(figures[3].color, Color::GREEN, "the green number is not written in green");
    assert_eq!(figures[1].tag_color, theme().warn, "a speed at the end of its range is not marked as the note under it is");
}

/// KEEP GREEN NUMBER decides which of the two speed rows is set and which is shown: the one worked
/// out reads what it comes to at the tempo on show, is drawn as a row that cannot be changed, and no
/// key or press moves it.
#[test]
fn the_green_number_and_the_speed_change_places_with_the_switch_that_holds_one() {
    let mut menu = Menu::over(bare_app("read-only"));
    let untouched = |menu: &mut Menu, id: SettingId| {
        let before = menu.app.shared.config.clone();
        menu.focus_setting(id);
        for code in [KeyCode::ArrowLeft, KeyCode::ArrowRight, KeyCode::Enter, KeyCode::Digit0, KeyCode::Numpad0] {
            menu.key(code);
        }
        menu.shifted(KeyCode::ArrowRight);
        menu.frame();
        let row = Row::Setting(id);
        let slider = menu.slider_of(row);
        for at in [menu.on_the_label_of(row), middle(slider), menu.on_the_number_of(row)] {
            menu.click(at, MouseButton::Left);
            menu.drag((slider.x, at.1));
            menu.let_go((slider.x, at.1));
            menu.typed("7");
            menu.key(KeyCode::Enter);
            menu.click(at, MouseButton::Right);
        }
        assert_eq!(menu.state.numbers.editing(), None, "{id:?} was opened to be typed into while it is worked out from the other");
        assert_eq!(menu.app.shared.config, before, "{id:?} was moved while it is worked out from the other");
    };

    assert_eq!((menu.look(SettingId::HiSpeed), menu.look(SettingId::GreenNumber)), (RowLook::Plain, RowLook::ReadOnly));
    assert_eq!(menu.value(SettingId::HiSpeed), "2.00x");
    assert_eq!(menu.value(SettingId::GreenNumber), "480 (800 MS) AUTO", "what a speed of two comes to at 150 beats a minute");
    assert!(help(&menu.app.shared.config.play, Row::Setting(SettingId::GreenNumber), None).contains("Turn KEEP GREEN NUMBER on"));
    assert_eq!(help(&menu.app.shared.config.play, Row::Setting(SettingId::HiSpeed), None), descriptor(SettingId::HiSpeed).help);
    untouched(&mut menu, SettingId::GreenNumber);
    menu.focus_setting(SettingId::HiSpeed);
    menu.keys(KeyCode::ArrowRight, 25);
    assert_eq!(menu.value(SettingId::HiSpeed), "2.25x");
    assert_eq!(menu.value(SettingId::GreenNumber), "426 (711 MS) AUTO", "the number shown did not follow the speed");
    menu.focus(Row::PreviewBpm);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::HiSpeed), "2.25x", "a stored speed moved with the tempo");
    assert_eq!(menu.value(SettingId::GreenNumber), "355 (593 MS) AUTO", "the number shown did not follow the tempo");
    menu.key(KeyCode::ArrowLeft);

    menu.focus_setting(SettingId::KeepGreenNumber);
    menu.key(KeyCode::ArrowRight);
    assert!(menu.app.shared.config.play.keep_green_number);
    assert_eq!((menu.look(SettingId::HiSpeed), menu.look(SettingId::GreenNumber)), (RowLook::ReadOnly, RowLook::Plain));
    assert_eq!(menu.value(SettingId::GreenNumber), "300 (500 MS)");
    assert_eq!(menu.value(SettingId::HiSpeed), "3.20x AUTO", "the speed that holds 500 ms at 150 beats a minute");
    assert!(help(&menu.app.shared.config.play, Row::Setting(SettingId::HiSpeed), None).contains("Turn KEEP GREEN NUMBER off"));
    assert_eq!(help(&menu.app.shared.config.play, Row::Setting(SettingId::GreenNumber), None), descriptor(SettingId::GreenNumber).help);
    untouched(&mut menu, SettingId::HiSpeed);
    assert!((menu.app.shared.config.play.hispeed - 2.25).abs() < f64::EPSILON, "the stored speed was written while a number is held");
    menu.focus_setting(SettingId::GreenNumber);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::GreenNumber), "301 (502 MS)");
    menu.focus(Row::PreviewBpm);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::GreenNumber), "301 (502 MS)", "a held number moved with the tempo");
    assert_eq!(menu.value(SettingId::HiSpeed), "2.66x AUTO", "the speed shown did not follow the tempo");

    menu.app.shared.config.play.duration_ms = 1;
    assert_eq!(menu.value(SettingId::HiSpeed), "20.00x (LIMIT) AUTO");
}

/// Every row reads with the unit it is measured in, and the gauge with the name the result screen
/// gives it.
#[test]
fn every_row_reads_with_its_unit() {
    let mut menu = Menu::over(bare_app("units"));
    assert_eq!(menu.line(Row::PreviewBpm).value, "150 BPM");
    assert_eq!(menu.value(SettingId::HiSpeedStep), "0.25x");
    assert_eq!(menu.value(SettingId::LaneCover), "0.0% (WHITE 0)");
    menu.app.shared.config.play.cover = 0.3;
    menu.app.shared.config.play.lift = 0.15;
    menu.app.shared.config.play.hidden = 0.1;
    menu.app.shared.config.judge.offset_ms = 5;
    assert_eq!(menu.value(SettingId::LaneCover), "30.0% (WHITE 300)");
    assert_eq!(menu.value(SettingId::Lift), "15.0%");
    assert_eq!(menu.value(SettingId::Hidden), "10.0%");
    assert_eq!(menu.value(SettingId::LaneCoverFineStep), "0.0010 (WHITE 1.0)");
    assert_eq!(menu.value(SettingId::JudgeOffset), "+5 MS");
    assert_eq!(menu.value(SettingId::KeepGreenNumber), "OFF");
    assert_eq!(menu.value(SettingId::FixHiSpeed), "MAIN");
    assert_eq!(menu.value(SettingId::SpeedFix), "FLOATING");
    for gauge in GAUGE_CYCLE {
        menu.app.shared.config.play.gauge = gauge;
        assert_eq!(menu.value(SettingId::Gauge), gauge_name(gauge));
    }
}

/// A lane cover set with the keys reads as ten times its percent in the white number, on its row and
/// under the preview, at every tenth of a percent there is: a press at a time up the whole lane,
/// which is one of the white number at a time, and ten at a time with Shift. The cover is never
/// written to here but by the keys, which is how a player sets it.
#[test]
fn a_lane_cover_set_with_the_keys_reads_as_ten_times_its_percent_in_the_white_number() {
    let mut menu = Menu::over(bare_app("white"));
    menu.focus_setting(SettingId::LaneCover);
    let white_under_the_preview = |menu: &Menu| {
        let play = &menu.app.shared.config.play;
        summary(&menu.state.target().readout(play), play.constant_speed)[4].value.clone()
    };
    let reads = |white: i32| format!("{}.{}% (WHITE {white})", white / 10, white % 10);
    for white in 1..=1000 {
        menu.key(KeyCode::ArrowRight);
        assert_eq!(menu.value(SettingId::LaneCover), reads(white));
        assert_eq!(white_under_the_preview(&menu), white.to_string(), "at {white} tenths of a percent");
        assert_eq!((menu.app.shared.config.play.cover * 1000.0) as i32, white, "a skin that cuts the cover to thousandths reads it one short");
    }
    for white in (0..1000).rev() {
        menu.key(KeyCode::ArrowLeft);
        assert_eq!(menu.value(SettingId::LaneCover), reads(white), "on the way down");
    }
    for percent in 1..=100 {
        menu.shifted(KeyCode::ArrowRight);
        assert_eq!(menu.value(SettingId::LaneCover), reads(percent * 10), "ten steps at a time");
    }
}

/// Left and Right step the row under the cursor in the field of the configuration the settings
/// screen wrote it in -- a row that holds a number by the smallest change it can hold -- a row at
/// an end of its range stays there, and a switch and a list go round, which Enter steps forwards.
#[test]
fn the_arrows_and_enter_step_the_row_under_the_cursor_to_the_ends_of_its_range() {
    let mut menu = Menu::over(bare_app("steps"));
    menu.focus_setting(SettingId::LaneCover);
    menu.key(KeyCode::ArrowLeft);
    assert!(menu.app.shared.config.play.cover.abs() < f32::EPSILON, "the cover went below nothing");
    menu.key(KeyCode::ArrowRight);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::LaneCover), "0.2% (WHITE 2)", "the cover is not stepped a tenth of a percent at a time");
    for _ in 0..=1000 / SHIFT_STEPS {
        menu.shifted(KeyCode::ArrowRight);
    }
    assert_eq!(menu.value(SettingId::LaneCover), "100.0% (WHITE 1000)", "the cover does not stop at the whole lane");

    menu.focus_setting(SettingId::JudgeOffset);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.app.shared.config.judge.offset_ms, 1, "the offset is not stepped a millisecond at a time");
    menu.key(KeyCode::ArrowLeft);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value(SettingId::JudgeOffset), "-1 MS");
    let wide_steps_across = (JUDGE_OFFSET_MAX_MS - JUDGE_OFFSET_MIN_MS) / SHIFT_STEPS;
    for _ in 0..=wide_steps_across {
        menu.shifted(KeyCode::ArrowLeft);
    }
    assert_eq!(menu.app.shared.config.judge.offset_ms, JUDGE_OFFSET_MIN_MS);
    for _ in 0..=wide_steps_across {
        menu.shifted(KeyCode::ArrowRight);
    }
    assert_eq!(menu.app.shared.config.judge.offset_ms, JUDGE_OFFSET_MAX_MS);

    menu.focus_setting(SettingId::AutoCal);
    menu.key(KeyCode::Enter);
    assert!(menu.app.shared.config.judge.auto_offset);
    menu.key(KeyCode::ArrowLeft);
    assert!(!menu.app.shared.config.judge.auto_offset);

    menu.focus_setting(SettingId::Random);
    let names = cycle_values(SettingId::Random);
    for name in names.iter().skip(1).chain([&names[0]]) {
        menu.key(KeyCode::ArrowRight);
        assert_eq!(menu.value(SettingId::Random), *name);
    }
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value(SettingId::Random), names[names.len() - 1], "the list does not go round its end backwards");
}

/// Shift moves a row that holds a number ten steps at a time and every other row one. A row that
/// would be carried past its end goes as far as its end.
#[test]
fn shift_moves_a_number_ten_steps_at_a_time_and_nothing_else() {
    let mut menu = Menu::over(bare_app("shift"));
    let numbers: Vec<SettingId> = SPEED_ROWS.iter().chain(&LANE_ROWS).chain(&JUDGE_ROWS).chain(&CHART_ROWS).copied().filter(|id| holds_a_number(*id)).collect();
    assert_eq!(
        numbers,
        [
            SettingId::GreenNumber,
            SettingId::HiSpeed,
            SettingId::HiSpeedStep,
            SettingId::LaneCover,
            SettingId::Lift,
            SettingId::Hidden,
            SettingId::LaneCoverFineStep,
            SettingId::JudgeOffset,
            SettingId::TargetRate
        ]
    );

    menu.focus_setting(SettingId::HiSpeed);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::HiSpeed), "2.01x", "one press is not a hundredth, whatever HI-SPEED STEP holds");
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::HiSpeed), "2.11x");
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::HiSpeed), "2.12x", "shift was still held after it was let go of");
    menu.app.shared.config.play.hispeed = 19.95;
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::HiSpeed), "20.00x", "a wide step past the fastest speed did not go as far as there was room for");
    menu.app.shared.config.play.hispeed = 0.05;
    menu.shifted(KeyCode::ArrowLeft);
    assert_eq!(menu.value(SettingId::HiSpeed), "0.01x", "a wide step past the slowest speed did not go as far as there was room for");

    menu.focus_setting(SettingId::HiSpeedStep);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::HiSpeedStep), "0.35x");
    menu.focus_setting(SettingId::Lift);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::Lift), "1.0%");
    menu.focus_setting(SettingId::Hidden);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::Hidden), "1.0%");
    menu.focus_setting(SettingId::LaneCoverFineStep);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::LaneCoverFineStep), "0.0020 (WHITE 2.0)");
    menu.focus_setting(SettingId::JudgeOffset);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::JudgeOffset), "+10 MS");
    menu.focus_setting(SettingId::TargetRate);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::TargetRate), "95%");

    menu.focus_setting(SettingId::KeepGreenNumber);
    menu.shifted(KeyCode::ArrowRight);
    assert!(menu.app.shared.config.play.keep_green_number, "shift turned a switch over an even number of times");
    menu.focus_setting(SettingId::GreenNumber);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value(SettingId::GreenNumber), "310 (517 MS)");
    menu.focus_setting(SettingId::Gauge);
    let before = menu.app.shared.config.play.gauge;
    menu.shifted(KeyCode::ArrowRight);
    let next = GAUGE_CYCLE[(GAUGE_CYCLE.iter().position(|gauge| *gauge == before).expect("the gauge is on the list") + 1) % GAUGE_CYCLE.len()];
    assert_eq!(menu.app.shared.config.play.gauge, next, "shift stepped a list more than one name on");
}

/// The zero key puts the row under the cursor back to what a fresh configuration holds, on the top
/// row of the keyboard and on the keypad, and leaves every other row as it was.
#[test]
fn the_zero_key_puts_the_row_under_the_cursor_back_to_what_it_ships_as() {
    let mut menu = Menu::over(bare_app("zero"));
    let shipped = menu.app.shared.config.clone();
    let listed: Vec<SettingId> = SPEED_ROWS.iter().chain(&LANE_ROWS).chain(&JUDGE_ROWS).chain(&CHART_ROWS).copied().collect();
    for (at, id) in listed.into_iter().enumerate() {
        if id == SettingId::GreenNumber {
            continue;
        }
        menu.focus_setting(id);
        menu.key(KeyCode::ArrowRight);
        assert_ne!(menu.app.shared.config, shipped, "{id:?} did not move");
        menu.key(if at % 2 == 0 { KeyCode::Digit0 } else { KeyCode::Numpad0 });
        assert_eq!(menu.app.shared.config, shipped, "{id:?} was not put back, or another row was");
    }

    menu.app.shared.config.play.keep_green_number = true;
    menu.focus_setting(SettingId::GreenNumber);
    menu.shifted(KeyCode::ArrowRight);
    menu.app.shared.config.play.cover = 0.4;
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.app.shared.config.play.duration_ms, shipped.play.duration_ms);
    assert!((menu.app.shared.config.play.cover - 0.4).abs() < f32::EPSILON, "the zero key put back a row the cursor is not on");
}

/// The cursor moves over the rows and never stands on the name of a group; a page is as many
/// entries as are on show, and the list follows the cursor with the name of the group over it.
#[test]
fn the_cursor_steps_over_the_names_of_the_groups_and_stays_on_show() {
    let mut menu = Menu::over(bare_app("cursor"));
    let last = menu.state.rows.len() - 1;
    assert_eq!((menu.state.sel, menu.state.top), (1, 0));
    menu.key(KeyCode::ArrowUp);
    assert_eq!(menu.state.sel, 1, "the cursor went up onto the name of the first group");
    menu.key(KeyCode::ArrowDown);
    assert_eq!(menu.state.focused(), Some(Row::Setting(SettingId::KeepGreenNumber)), "the cursor did not step over the name of the next group");
    let mut visited = vec![menu.state.sel];
    for _ in 0..menu.state.rows.len() {
        menu.key(KeyCode::ArrowDown);
        assert!(!menu.state.rows[menu.state.sel].is_heading(), "the cursor stands on the name of a group at {}", menu.state.sel);
        assert!(menu.state.top <= menu.state.sel && menu.state.sel < menu.state.top + VISIBLE_ROWS, "the cursor is off the screen at {}", menu.state.sel);
        visited.push(menu.state.sel);
    }
    visited.dedup();
    let rows: Vec<usize> = (3..=last).filter(|at| !menu.state.rows[*at].is_heading()).collect();
    assert_eq!(visited, rows, "a row was stepped over on the way down");
    assert_eq!((menu.state.sel, menu.state.top), (last, last_top(menu.state.rows.len())));

    menu.key(KeyCode::PageUp);
    assert_eq!(menu.state.sel, last - VISIBLE_ROWS);
    menu.key(KeyCode::PageUp);
    assert_eq!((menu.state.sel, menu.state.top), (1, 0), "a page up from near the top did not land on the first row with its group's name on show");
    menu.key(KeyCode::PageDown);
    assert_eq!(menu.state.sel, 1 + VISIBLE_ROWS);
    menu.key(KeyCode::PageDown);
    assert_eq!(menu.state.sel, last);

    let judge = menu.at(Row::Group("JUDGE"));
    menu.state.focus(judge, ROW_STEP_BACK);
    assert_eq!(menu.state.sel, judge - 1, "from the name of a group the cursor did not go to the row before it when asked back");
    menu.state.focus(last, ROW_STEP_FORWARD);
    let first_of_lane = menu.at(Row::Setting(SettingId::EnableCover));
    menu.state.focus(first_of_lane, ROW_STEP_FORWARD);
    assert_eq!(menu.state.top, first_of_lane - 1, "the name of the group is not brought on show with its first row");
    menu.frame();
    assert!(menu.rows_on_show().contains(&menu.state.sel), "the cursor's row is not on show");
}

/// A press on a row moves the cursor there and steps the row once, forwards with the left button
/// and back with the right -- a row that holds a number by the smallest change it can hold, when
/// the press lands on its label, and back with the right button wherever on the row it lands. The
/// name of a group takes no press, and neither does the preview.
#[test]
fn a_press_steps_the_row_it_lands_on_forwards_or_back() {
    let mut menu = Menu::over(bare_app("click"));
    let before = menu.app.shared.config.clone();
    menu.frame();
    let headings = menu.state.rows.iter().take(VISIBLE_ROWS).filter(|row| row.is_heading()).count();
    assert_eq!(menu.rows_on_show().len(), VISIBLE_ROWS - headings, "the names of the groups on show are not the only entries that take no press");
    let speed = menu.at(Row::Setting(SettingId::HiSpeed));
    let at = menu.on_the_label_of(Row::Setting(SettingId::HiSpeed));
    assert!(matches!(menu.click(at, MouseButton::Left), Transition::Stay));
    assert_eq!((menu.state.sel, menu.value(SettingId::HiSpeed)), (speed, "2.01x".to_string()));
    menu.click(at, MouseButton::Right);
    menu.click(middle(menu.slider_of(Row::Setting(SettingId::HiSpeed))), MouseButton::Right);
    menu.click(menu.on_the_number_of(Row::Setting(SettingId::HiSpeed)), MouseButton::Right);
    assert_eq!(menu.value(SettingId::HiSpeed), "1.98x", "the right button did not step the row back, wherever on the row it landed");
    menu.click(at, MouseButton::Middle);
    menu.let_go(at);
    assert_eq!(menu.value(SettingId::HiSpeed), "1.98x", "another button, or letting one go, stepped the row");
    menu.click(at, MouseButton::Left);
    menu.click(at, MouseButton::Left);

    let cover = menu.middle_of_hot(Hot::PlayMenuRow(menu.at(Row::Setting(SettingId::EnableCover))));
    menu.click(cover, MouseButton::Left);
    assert!(!menu.app.shared.config.play.enable_cover);
    assert_eq!(menu.state.focused(), Some(Row::Setting(SettingId::EnableCover)));
    menu.click(cover, MouseButton::Right);
    assert_eq!(menu.app.shared.config, before);

    let cursor = menu.state.sel;
    for nowhere in [middle(PREVIEW_RECT), middle(INFO_RECT), (MARGIN, TITLE_Y), middle(crate::stage::preview_menu::row_rect(0))] {
        menu.click(nowhere, MouseButton::Left);
        menu.click(nowhere, MouseButton::Right);
    }
    assert_eq!((menu.state.sel, &menu.app.shared.config), (cursor, &before), "a press off the rows moved something");
}

/// The wheel scrolls the list while the pointer is over it, stops at both ends, and leaves the
/// cursor on its row; the key that next moves the cursor brings it back onto the screen. Over the
/// preview, or anywhere else, the wheel does nothing.
#[test]
fn the_wheel_scrolls_the_list_under_the_pointer_and_nothing_anywhere_else() {
    let mut menu = Menu::over(bare_app("wheel"));
    let last = last_top(menu.state.rows.len());
    assert!(last > 0, "the list fits the screen, so there is nothing to scroll");
    let over_list = middle(LIST_RECT);
    for elsewhere in [middle(PREVIEW_RECT), middle(INFO_RECT), (MARGIN, TITLE_Y), middle(tab_rect(0, PREVIEW_MODES.len()))] {
        menu.wheel(elsewhere, 3.0);
        assert_eq!(menu.state.top, 0, "the wheel scrolled the list with the pointer at {elsewhere:?}");
    }
    menu.wheel(over_list, 3.0);
    assert_eq!((menu.state.top, menu.state.sel), (3, 1), "the wheel did not scroll the list alone");
    menu.frame();
    let on_show: Vec<usize> = (3..3 + VISIBLE_ROWS).filter(|at| !menu.state.rows[*at].is_heading()).collect();
    assert_eq!(menu.rows_on_show(), on_show);
    menu.wheel(over_list, -50.0);
    assert_eq!(menu.state.top, 0, "the wheel scrolled past the top");
    menu.wheel(over_list, 1000.0);
    assert_eq!(menu.state.top, last, "the wheel scrolled past the foot");
    menu.wheel(over_list, f32::NAN);
    menu.wheel(over_list, -1.0);
    assert_eq!(menu.state.top, last - 1, "a turn that is not a number broke the wheel");
    menu.key(KeyCode::ArrowDown);
    assert!(menu.state.top <= menu.state.sel && menu.state.sel < menu.state.top + VISIBLE_ROWS, "the cursor was not brought back onto the screen");
    let before = menu.app.shared.config.clone();
    menu.wheel(over_list, 4.0);
    assert_eq!(menu.app.shared.config, before, "the wheel changed a setting");
}

/// Everything the menu writes fits where it is written: a row's label beside its widest value, what
/// is said of every row on its line, the summary on the lines the panel has for it, the line of
/// keys and what is written beside the title.
#[test]
fn what_the_menu_writes_fits_where_it_is_written() {
    let mut menu = Menu::over(app_with_shipped_pack("fits"));
    let hint = text_width(HINT, HINT_SCALE);
    assert!(hint <= CW as f32 - MARGIN * 2.0, "the line of keys is {hint} wide");

    let fits = |menu: &Menu, case: &str| {
        for row in menu.state.rows.iter().copied().filter(|row| !row.is_heading()) {
            let Line { label, value, .. } = menu.line(row);
            assert_eq!(fitted_value(&value), value, "{case}: the value of {label} is cut short");
            let needed = text_width(&label, LABEL_SCALE) + LABEL_GAP + text_width(&value, VALUE_SCALE);
            let named_by_a_skin = matches!(row, Row::Skin(_));
            assert!(named_by_a_skin || needed <= row_room(), "{case}: {label} with {value} needs {needed} of {}", row_room());
            let said = menu.state.help_of(&menu.app.shared, row);
            assert!(!said.is_empty(), "{case}: nothing is said of {label}");
            assert_eq!(fit_text(said, INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0), said, "{case}: what is said of {label} is cut short");
        }
        let readout = menu.state.target().readout(&menu.app.shared.config.play);
        let figures = summary(&readout, menu.app.shared.config.play.constant_speed);
        for (at, figure) in figures.iter().enumerate() {
            let (room, needed) = (figure_x(at + 1, figures.len()) - figure_x(at, figures.len()), figure_width(figure) + FIGURE_ROOM);
            assert!(needed <= room, "{case}: {} with {} {} needs {needed} of {room}", figure.name, figure.value, figure.tag);
        }
        if let Some(note) = summary_note(&readout) {
            assert_eq!(fit_text(note, FIGURE_NOTE_SCALE, INFO_RECT.w - INFO_PAD * 2.0), note, "{case}: what is said under the figures is cut short");
        }
        assert_eq!(head_subtitle(&menu.state.subtitle()), menu.state.subtitle(), "{case}: what is written beside the title is cut short");
    };

    for _ in 0..PREVIEW_MODES.len() {
        fits(&menu, "as shipped");
        menu.key(KeyCode::Tab);
    }
    let play = &mut menu.app.shared.config.play;
    (play.cover, play.lift, play.hidden, play.enable_cover) = (1.0, 1.0, 1.0, false);
    (play.duration_ms, play.hispeed, play.constant_speed) = (rbms_config::DURATION_MAX_MS, HISPEED_MAX, true);
    menu.app.shared.config.judge.offset_ms = JUDGE_OFFSET_MIN_MS;
    fits(&menu, "at the ends, speed stored");
    menu.app.shared.config.play.keep_green_number = true;
    fits(&menu, "at the ends, number held");
    menu.app.shared.config.play.duration_ms = 1;
    fits(&menu, "limited");

    for id in SPEED_ROWS.iter().chain(&LANE_ROWS).chain(&JUDGE_ROWS).chain(&CHART_ROWS) {
        for _ in 0..cycle_values(*id).len() {
            step_setting(&mut menu.app.shared.config, *id, ROW_STEP_FORWARD, false);
            let Line { label, value, .. } = menu.line(Row::Setting(*id));
            assert_eq!(fitted_value(&value), value, "{label}");
            assert!(text_width(&label, LABEL_SCALE) + LABEL_GAP + text_width(&value, VALUE_SCALE) <= row_room(), "{label} with {value}");
        }
    }
}

/// Escape writes the settings file and goes back, and every other key stays.
#[test]
fn escape_saves_and_goes_back_and_every_other_key_stays() {
    let mut menu = Menu::over(bare_app("escape"));
    let settings = menu.app.shared.settings_path.clone();
    assert!(!settings.exists(), "the fixture begins with a settings file");
    for code in [KeyCode::Enter, KeyCode::ArrowDown, KeyCode::ArrowRight, KeyCode::Tab, KeyCode::F8, KeyCode::Digit0, KeyCode::KeyA, KeyCode::PageDown] {
        assert!(matches!(menu.key(code), Transition::Stay), "{code:?}");
    }
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back));
    assert!(settings.exists(), "leaving the menu did not write the settings out");
}

/// What the menu changed is in the settings file once the menu is left, in the fields the settings
/// screen wrote, and reads back as it was written. The mode and the tempo of the preview are not in
/// it.
#[test]
fn what_the_menu_changed_survives_the_settings_file() {
    let mut menu = Menu::over(app_with_shipped_pack("file"));
    let rows = [
        SettingId::KeepGreenNumber,
        SettingId::GreenNumber,
        SettingId::HiSpeedStep,
        SettingId::FixHiSpeed,
        SettingId::EnableLift,
        SettingId::Lift,
        SettingId::LaneCover,
        SettingId::EnableHidden,
        SettingId::Hidden,
        SettingId::JudgeOffset,
        SettingId::AutoCal,
        SettingId::Random,
        SettingId::Gauge,
        SettingId::Target,
        SettingId::ScratchAuto,
        SettingId::PlayEscapeMode,
    ];
    for id in rows {
        menu.focus_setting(id);
        menu.shifted(KeyCode::ArrowRight);
    }
    let option = menu.skin_option(LANE_WIDTH_OPTION);
    menu.focus(option);
    menu.key(KeyCode::ArrowRight);
    menu.focus(Row::PreviewBpm);
    menu.key(KeyCode::ArrowRight);
    menu.key(KeyCode::Tab);
    let (play, judge) = (menu.app.shared.config.play.clone(), menu.app.shared.config.judge.clone());
    assert!(play.keep_green_number && play.enable_lift && play.scratch_auto && judge.auto_offset);
    assert_eq!((play.duration_ms, judge.offset_ms), (517, SHIFT_STEPS));

    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back));
    let stored = rbms_config::load(&menu.app.shared.settings_path).expect("the settings file the menu wrote is read").config;
    assert_eq!(stored.play, play);
    assert_eq!(stored.judge, judge);
    assert_eq!(stored.skin, menu.app.shared.config.skin, "the option of the skin is not in the file as it was chosen");
    assert_eq!(stored.skin.screen, Config::default().skin.screen, "the mode on the strip was stored as the screen the skin menu configures");
    let text = std::fs::read_to_string(&menu.app.shared.settings_path).expect("the settings file is text").to_ascii_lowercase();
    assert!(!text.contains("preview_bpm") && !text.contains("preview_mode"), "what the preview is shown on is in the settings file");
}

/// The key the menu ships on opens it over the browser, on a browser no skin draws, and Escape goes
/// back to the browser with nothing left parked.
#[test]
fn the_key_opens_the_menu_over_the_browser_and_escape_goes_back_to_it() {
    let mut app = bare_app("history");
    assert_eq!(app.shared.shortcut_key(Shortcut::PlayMenu), Some(PLAY_MENU_KEY));
    let now = std::time::Instant::now();
    app.stage = Stage::Select(Box::new(SelectState::new()));
    let asked = app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(PLAY_MENU_KEY));
    assert!(matches!(asked, Transition::Open(Stage::PlayMenu(_))), "the key did not open the play menu");
    app.switch(asked);
    assert_eq!(app.stage.id(), StageId::PlayMenu);
    let asked = app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(KeyCode::Escape));
    app.switch(asked);
    assert_eq!(app.stage.id(), StageId::Select);
    assert!(app.suspended.is_empty(), "the browser was left parked under a menu that is gone");
}

/// A menu opened by the application has its list from the first key on, and a press routed by the
/// application reaches it.
#[test]
fn a_menu_the_application_opened_takes_keys_and_presses() {
    let mut app = bare_app("routed");
    app.switch(Transition::Open(PlayMenuState::stage()));
    let now = std::time::Instant::now();
    for code in [KeyCode::ArrowDown, KeyCode::ArrowRight] {
        let asked = app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(code));
        app.switch(asked);
    }
    assert!(app.shared.config.play.keep_green_number, "the first row under the tempo was not stepped");
    let mut pixels = HeadlessCanvas::new(CW, CH);
    app.shared.hot.clear();
    app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, &mut Canvas::Headless(&mut pixels));
    let (rect, _) = *app.shared.hot.iter().find(|(_, hot)| *hot == Hot::PlayMenuRow(3)).expect("the row is on the screen");
    let input = crate::pointer::PointerInput::Button { button: MouseButton::Left, pressed: true };
    let asked = app.stage.handle_pointer(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, middle(rect), input);
    app.switch(asked);
    assert!(!app.shared.config.play.keep_green_number, "a press on the row did not reach the menu");
}

/// Leaving the menu lets go of what its preview compiled: the scene the menu was opened over is put
/// back, and by the end of the frame that follows the previewed screen is no longer compiled and no
/// texture of it is held.
#[test]
fn leaving_the_menu_lets_go_of_what_the_preview_held() {
    let mut app = app_with_shipped_pack("released");
    let underneath = app.shared.suspend_skin_scene();
    let mut menu = Menu::over(app);
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    let screen = menu.state.target().screen;
    assert!(menu.app.shared.has_compiled_skin(screen) && menu.app.shared.skin_screens.texture_stats().count > 0, "a drawn preview holds no texture");
    menu.state.on_exit(&mut FrameCtx { shared: &mut menu.app.shared, now: std::time::Instant::now(), dt: 0.0 });
    menu.app.shared.resume_skin_scene(underneath);
    let mut canvas = Canvas::Headless(&mut menu.pixels);
    menu.app.shared.finish_skin_frame(&mut canvas);
    assert!(!menu.app.shared.has_compiled_skin(screen), "the previewed screen outlived the menu");
    assert_eq!(menu.app.shared.skin_screens.texture_stats().count, 0, "the preview's textures outlived the menu");
    assert_eq!(menu.state.preview.target(), PreviewTarget::Play(menu.state.target()), "a released preview forgot what it shows");
}

/// A row of the settings table that moved is drawn by the preview from the frame that follows: the
/// preview is told, and goes on drawing.
#[test]
fn the_preview_goes_on_drawing_after_a_row_moved() {
    let mut menu = Menu::over(app_with_shipped_pack("moved"));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(PREVIEW_MODES[0].0));
    for id in [SettingId::LaneCover, SettingId::EnableLift, SettingId::Lift, SettingId::SpeedFix, SettingId::Gauge, SettingId::KeepGreenNumber] {
        menu.focus_setting(id);
        menu.shifted(KeyCode::ArrowRight);
        menu.frame();
        assert_eq!(menu.until_said(), PreviewStatus::Drawn, "the preview stopped drawing after {id:?} moved");
    }
}

/// The menu over the shipped pack, saved as it is looked at. Every value on show was set with the
/// keys a player sets it with: the covers before and after they are raised, a held green number at
/// the slowest, the opening and the fastest tempo of the list, a stored speed at two tempos, the
/// field at one tempo of its own, a target and a run the game plays, the list at its foot, a speed
/// at the end of its range, another mode, and a larger window.
#[test]
fn the_menu_over_the_shipped_pack_can_be_captured() {
    let mut menu = Menu::over(app_with_shipped_pack("capture"));
    let slowest = |menu: &mut Menu| {
        menu.focus(Row::PreviewBpm);
        menu.key(KeyCode::Digit0);
        (0..OPENING_BPM).for_each(|_| drop(menu.key(KeyCode::ArrowLeft)));
    };
    let fastest = |menu: &mut Menu| {
        menu.focus(Row::PreviewBpm);
        menu.key(KeyCode::Digit0);
        (OPENING_BPM + 1..PREVIEW_BPMS.len()).for_each(|_| drop(menu.key(KeyCode::ArrowRight)));
    };
    menu.focus_setting(SettingId::LaneCover);
    assert_eq!(menu.capture_at("play-menu-covers-before", RUN_SECONDS), PreviewStatus::Drawn);
    (0..30).for_each(|_| menu.shifted(KeyCode::ArrowRight));
    menu.focus_setting(SettingId::EnableLift);
    menu.key(KeyCode::ArrowRight);
    menu.focus_setting(SettingId::Lift);
    (0..15).for_each(|_| menu.shifted(KeyCode::ArrowRight));
    assert_eq!((menu.value(SettingId::LaneCover).as_str(), menu.value(SettingId::Lift).as_str()), ("30.0% (WHITE 300)", "15.0%"));
    assert_eq!(menu.capture_at("play-menu-covers-after", RUN_SECONDS), PreviewStatus::Drawn);

    menu.focus_setting(SettingId::KeepGreenNumber);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.capture_at("play-menu-held-150", RUN_SECONDS), PreviewStatus::Drawn);
    menu.focus_setting(SettingId::HiSpeed);
    assert_eq!(menu.capture_at("play-menu-read-only-cursor", RUN_SECONDS), PreviewStatus::Drawn);
    slowest(&mut menu);
    assert_eq!(menu.capture_at("play-menu-held-100", RUN_SECONDS), PreviewStatus::Drawn);
    fastest(&mut menu);
    assert_eq!(menu.capture_at("play-menu-held-200", RUN_SECONDS), PreviewStatus::Drawn);

    menu.focus_setting(SettingId::KeepGreenNumber);
    menu.key(KeyCode::ArrowRight);
    menu.focus_setting(SettingId::GreenNumber);
    assert_eq!(menu.capture_at("play-menu-stored-200", RUN_SECONDS), PreviewStatus::Drawn);
    slowest(&mut menu);
    assert_eq!(menu.capture_at("play-menu-stored-100", RUN_SECONDS), PreviewStatus::Drawn);
    menu.focus_setting(SettingId::SpeedFix);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.capture_at("play-menu-constant", RUN_SECONDS), PreviewStatus::Drawn);
    menu.key(KeyCode::ArrowRight);

    menu.focus_setting(SettingId::Gauge);
    menu.key(KeyCode::ArrowRight);
    menu.focus_setting(SettingId::Autoplay);
    menu.key(KeyCode::ArrowRight);
    menu.focus_setting(SettingId::Target);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.capture_at("play-menu-target-autoplay", RUN_SECONDS), PreviewStatus::Drawn);
    menu.focus_setting(SettingId::PlayEscapeMode);
    assert_eq!(menu.capture_at("play-menu-scrolled-end", RUN_SECONDS), PreviewStatus::Drawn);
    let lane_width = menu.skin_option(LANE_WIDTH_OPTION);
    menu.focus(lane_width);
    assert_eq!(menu.capture_at("play-menu-skin-option-cursor", RUN_SECONDS), PreviewStatus::Drawn);

    menu.focus_setting(SettingId::KeepGreenNumber);
    menu.key(KeyCode::ArrowRight);
    menu.focus_setting(SettingId::GreenNumber);
    (0..40).for_each(|_| menu.shifted(KeyCode::ArrowLeft));
    assert!(menu.state.target().readout(&menu.app.shared.config.play).limited, "the smallest green number can be held at the slowest tempo");
    menu.focus_setting(SettingId::HiSpeed);
    assert_eq!(menu.capture_at("play-menu-limited", RUN_SECONDS), PreviewStatus::Drawn);

    let fourteen_keys = PREVIEW_MODES.iter().position(|(_, name)| *name == "14K").expect("the strip has a fourteen-key tab");
    (0..fourteen_keys).for_each(|_| drop(menu.key(KeyCode::Tab)));
    menu.focus_setting(SettingId::GreenNumber);
    menu.key(KeyCode::Digit0);
    menu.capture_at("play-menu-14k", RUN_SECONDS);

    let mut large = Menu::sized(app_with_shipped_pack("capture-large"), LARGE);
    large.focus_setting(SettingId::LaneCover);
    (0..30).for_each(|_| large.shifted(KeyCode::ArrowRight));
    assert_eq!(large.capture_at("play-menu-default", RUN_SECONDS), PreviewStatus::Drawn);
    large.focus_setting(SettingId::PlayEscapeMode);
    assert_eq!(large.capture_at("play-menu-scrolled-end", RUN_SECONDS), PreviewStatus::Drawn);
    let mut bare = Menu::sized(bare_app("capture-large-bare"), LARGE);
    assert_eq!(bare.capture_at("play-menu-no-skin", 0.0), PreviewStatus::Absent(PreviewAbsence::NoDocument));
}

/// The menu over the pack an environment variable names, saved as it is looked at: a pack somebody
/// else wrote, on its seven-key play screen, as it opens and with a green number held at the slowest
/// tempo of the list under a raised cover. Opt-in, because the pack is outside the repository:
/// without the variable and a capture folder this draws nothing.
#[test]
fn the_menu_over_the_pack_the_environment_names_can_be_captured() {
    let Some(pack) = pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let mut app = bare_app("capture-env");
    app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    let mut menu = Menu::over(app);
    assert_ne!(menu.capture_at("play-menu-pack-7k", RUN_SECONDS), PreviewStatus::Loading, "the preview of the pack never said what is there");
    menu.focus_setting(SettingId::LaneCover);
    (0..3).for_each(|_| menu.shifted(KeyCode::ArrowRight));
    menu.focus_setting(SettingId::KeepGreenNumber);
    menu.key(KeyCode::ArrowRight);
    menu.focus(Row::PreviewBpm);
    (0..OPENING_BPM).for_each(|_| drop(menu.key(KeyCode::ArrowLeft)));
    assert_ne!(menu.capture_at("play-menu-pack-7k-held-100", RUN_SECONDS), PreviewStatus::Loading);
}

mod colors;
mod numbers;
mod scratch_auto_color;
mod shared;
mod skin_offsets;
mod target;
