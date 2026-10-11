//! The notes the settings screen writes beside the rows a skin does not read, drawn headless with
//! the skin pack this repository ships.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, every tab that carries a note is also saved there as a
//! PNG a person can open, which is how the wording and the room around it are looked at.

use std::path::{Path, PathBuf};

use rbms_config::{BUILT_IN_ONLY_NOTE, SKIN_OPTION_NOTE};

use super::*;
use crate::skin_select::SkinLibrary;
use crate::stage::render_tests::app;
use crate::stage::{Canvas, HeadlessCanvas};

/// Environment variable naming the folder captures are saved in, the one the screen capture harness
/// reads. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// Where the captures of this file go below that folder.
const CAPTURE_SUBFOLDER: &str = "settings";

/// The skin pack this repository ships, relative to the player's manifest.
const DEFAULT_PACK: &str = "../../assets/skins/rbms-default";

/// Every row that carries a note while the shipped pack draws the screens, with the tab it is on and
/// what is said beside it.
const NOTED_ROWS: [(SettingTab, SettingId, &str); 6] = [
    (SettingTab::Play, SettingId::FiveKeyLayout, BUILT_IN_ONLY_NOTE),
    (SettingTab::Display, SettingId::Skin, BUILT_IN_ONLY_NOTE),
    (SettingTab::Display, SettingId::ResultGraphs, BUILT_IN_ONLY_NOTE),
    (SettingTab::Display, SettingId::WhiteNumber, BUILT_IN_ONLY_NOTE),
    (SettingTab::Display, SettingId::JudgeTextY, BUILT_IN_ONLY_NOTE),
    (SettingTab::Input, SettingId::ScratchSide, SKIN_OPTION_NOTE),
];

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
}

/// An app whose settings live in a folder of this test's own, with no skin chosen and none
/// installed beside the settings file.
fn bare_app(tag: &str) -> App {
    let mut app = app();
    let root = std::env::temp_dir().join(format!("rbms-settings-notes-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    app.shared.settings_path = root.join("settings.ron");
    app.shared.config.skin = rbms_config::SkinOptions::default();
    app.shared.skins = SkinLibrary::new(&app.shared.settings_path, &app.shared.config);
    app
}

/// The same app with the pack this repository ships named on the PACK FOLDER row.
fn app_with_shipped_pack(tag: &str) -> App {
    let mut app = bare_app(tag);
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK);
    assert!(pack.is_dir(), "the shipped skin pack is not at {}", pack.display());
    app.shared.set_skin_pack(Some(pack.to_string_lossy().into_owned()));
    app
}

/// A settings screen entered on `tab`, with the cursor on `focus` when one is named.
fn screen_on(app: &mut App, tab: SettingTab, focus: Option<SettingId>) -> SettingsState {
    let mut state = SettingsState::on_tab(tab);
    state.on_enter(&mut ctx(app));
    if let Some(id) = focus {
        state.sel = row_at(&state, id);
    }
    state
}

fn row_at(state: &SettingsState, id: SettingId) -> usize {
    state.rows.iter().position(|row| *row == SettingRow::Fixed(id)).unwrap_or_else(|| panic!("{id:?} is not on the open tab"))
}

fn draw(app: &mut App, state: &mut SettingsState) -> HeadlessCanvas {
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let mut canvas = Canvas::Headless(&mut pixels);
    app.shared.hot.clear();
    state.draw(&mut ctx(app), &mut canvas);
    pixels
}

/// Save a drawn screen as `<capture folder>/settings/<name>.png`, when a capture folder was named.
fn save(name: &str, pixels: &HeadlessCanvas) -> Option<PathBuf> {
    let folder = PathBuf::from(std::env::var_os(CAPTURE_DIR_ENV)?).join(CAPTURE_SUBFOLDER);
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    let path = folder.join(format!("{name}.png"));
    image::save_buffer(&path, pixels.rgba(), CW, CH, image::ExtendedColorType::Rgba8).expect("the capture is written");
    Some(path)
}

#[test]
fn the_shipped_pack_annotates_every_row_its_screens_do_not_read() {
    let mut app = app_with_shipped_pack("annotates");
    for (tab, id, note) in NOTED_ROWS {
        let state = screen_on(&mut app, tab, None);
        assert_eq!(state.lines.len(), state.rows.len(), "{tab:?} has a row with no line");
        assert_eq!(state.lines[row_at(&state, id)].note, Some(note), "{id:?}");
    }
    for tab in SettingTab::ALL {
        let state = screen_on(&mut app, tab, None);
        let noted: Vec<SettingRow> = state.rows.iter().zip(&state.lines).filter(|(_, line)| line.note.is_some()).map(|(row, _)| *row).collect();
        let expected: Vec<SettingRow> = NOTED_ROWS.iter().filter(|(on, _, _)| *on == tab).map(|(_, id, _)| SettingRow::Fixed(*id)).collect();
        assert_eq!(noted, expected, "{tab:?}");
    }
}

#[test]
fn a_configuration_with_no_skin_shows_every_row_bare() {
    let mut app = bare_app("bare");
    for tab in SettingTab::ALL {
        let state = screen_on(&mut app, tab, None);
        assert!(state.lines.iter().all(|line| line.note.is_none()), "{tab:?} says something beside a row with no skin drawing anything");
    }
}

/// The notes follow the pack as it is named and taken away again on the SKIN tab, without the
/// settings screen being left: the rows are rebuilt the next time a tab is opened.
#[test]
fn the_notes_come_and_go_with_the_pack() {
    let mut app = app_with_shipped_pack("follows");
    let mut noted = screen_on(&mut app, SettingTab::Display, None);
    let with_pack = draw(&mut app, &mut noted).pixel_checksum();

    app.shared.set_skin_pack(None);
    let mut bare = screen_on(&mut app, SettingTab::Display, None);
    assert!(bare.lines.iter().all(|line| line.note.is_none()), "a note outlived the pack that raised it");
    assert_ne!(draw(&mut app, &mut bare).pixel_checksum(), with_pack, "the notes change nothing about what is drawn");
}

/// An annotated row is still a row: Left and Right move it exactly as they do when it is bare.
#[test]
fn an_annotated_row_still_steps() {
    let mut app = app_with_shipped_pack("steps");
    let mut state = screen_on(&mut app, SettingTab::Input, Some(SettingId::ScratchSide));
    let before = app.shared.config.play.scratch_left;
    let row = state.focused().expect("the cursor is on a row");
    state.step(&mut ctx(&mut app), row, ROW_STEP_FORWARD);
    assert_ne!(app.shared.config.play.scratch_left, before, "the annotated row did not move");
    state.ensure_rows(&app.shared);
    assert_eq!(state.lines[row_at(&state, SettingId::ScratchSide)].note, Some(SKIN_OPTION_NOTE), "the note was lost when the row moved");
}

/// Draw every tab that carries a note, with the cursor off the noted rows and on each one in turn,
/// so both the dimmed and the focused form of a row are on a capture.
#[test]
fn the_annotated_tabs_draw_and_can_be_captured() {
    let mut app = app_with_shipped_pack("capture");
    for (tab, id, _) in NOTED_ROWS {
        let mut focused = screen_on(&mut app, tab, Some(id));
        let pixels = draw(&mut app, &mut focused);
        assert!(pixels.painted_pixels() > 0, "{tab:?} drew nothing");
        save(&format!("{}-{}-focused", tab.label().to_ascii_lowercase(), descriptor_slug(id)), &pixels);
    }
    for tab in [SettingTab::Play, SettingTab::Display, SettingTab::Input, SettingTab::Skin] {
        let mut resting = screen_on(&mut app, tab, None);
        let pixels = draw(&mut app, &mut resting);
        assert!(pixels.painted_pixels() > 0, "{tab:?} drew nothing");
        save(&format!("{}-top", tab.label().to_ascii_lowercase()), &pixels);
    }
    let mut bare_app = bare_app("capture-bare");
    let mut bare = screen_on(&mut bare_app, SettingTab::Display, None);
    save("display-top-no-skin", &draw(&mut bare_app, &mut bare));
}

/// A row's label as part of a file name.
fn descriptor_slug(id: SettingId) -> String {
    rbms_config::descriptor(id).label.to_ascii_lowercase().replace(' ', "-")
}
