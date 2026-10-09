//! The SKIN tab of the settings screen, drawn headless.
//!
//! It is the one tab whose rows the settings table does not fix: a chosen document declares its
//! own, so what is asserted here is that the document's rows reach the screen, are clickable, and
//! redraw when they move.

use super::tests::press;
use super::*;
use crate::settings_view::visible_rows;
use crate::skin_select::{OffsetAxis, SkinRow, fixtures};
use crate::stage::render_tests::app;
use crate::stage::{Canvas, HeadlessCanvas};

/// A settings screen on the SKIN tab of an app whose skin folder holds the fixture documents,
/// already entered so the folder has been walked.
///
/// Each test names its own folder: the documents are written fresh every time, and the tests
/// run beside one another.
fn skin_screen(app: &mut App, tag: &str) -> SettingsState {
    let root = std::env::temp_dir().join(format!("rbms-skin-tab-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the fixture folder is writable");
    app.shared.settings_path = root.join("settings.ron");
    let folder = fixtures::write(&app.shared.settings_path);
    app.shared.config.skin.folder = Some(folder.to_string_lossy().into_owned());
    app.shared.config.skin.screen = fixtures::MUSIC_SELECT;
    let mut state = SettingsState::on_tab(SettingTab::Skin);
    state.on_enter(&mut ctx(app));
    state
}

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: std::time::Instant::now(), dt: 0.0 }
}

/// Draw the screen onto a headless canvas and answer the pixels, with the app's hot regions
/// rebuilt for the frame.
fn draw(app: &mut App, state: &mut SettingsState) -> HeadlessCanvas {
    let mut pixels = HeadlessCanvas::new(CW, CH);
    let mut canvas = Canvas::Headless(&mut pixels);
    app.shared.hot.clear();
    state.draw(&mut ctx(app), &mut canvas);
    pixels
}

/// Put the cursor on one row of the open tab.
fn focus(state: &mut SettingsState, row: SettingRow) {
    state.sel = state.rows.iter().position(|entry| *entry == row).unwrap_or_else(|| panic!("{row:?} is not on the open tab"));
}

/// The labels of the rows the screen is showing.
fn labels(state: &SettingsState) -> Vec<String> {
    state.lines.iter().map(|(label, _)| label.clone()).collect()
}

/// The value the screen is showing for one row.
fn value_of(state: &SettingsState, row: SettingRow) -> String {
    let at = state.rows.iter().position(|entry| *entry == row).unwrap_or_else(|| panic!("{row:?} is not on the open tab"));
    state.lines[at].1.clone()
}

/// Until a document is chosen the SKIN tab is only the five rows the table declares; choosing
/// one adds the rows that document declares, between the document row and the actions.
#[test]
fn choosing_a_document_adds_the_rows_it_declares_between_the_row_and_the_actions() {
    let mut app = app();
    let mut state = skin_screen(&mut app, "rows");
    assert_eq!(
        state.rows,
        vec![
            SettingRow::Fixed(SettingId::SkinScreen),
            SettingRow::Fixed(SettingId::SkinDocument),
            SettingRow::Fixed(SettingId::SkinInfo),
            SettingRow::Fixed(SettingId::SkinReload),
            SettingRow::Fixed(SettingId::SkinReset),
        ]
    );

    focus(&mut state, SettingRow::Fixed(SettingId::SkinDocument));
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);

    let shown = labels(&state);
    assert!(shown.contains(&fixtures::BROWSER_PROPERTY_LABEL.to_string()), "the document's own rows are not on the screen: {shown:?}");
    let property = shown.iter().position(|label| label == fixtures::BROWSER_PROPERTY_LABEL).expect("the property row is listed");
    let reload = shown.iter().position(|label| label == "RELOAD").expect("the reload row is listed");
    let document = shown.iter().position(|label| label == "SKIN").expect("the document row is listed");
    assert!(document < property && property < reload, "the document's rows are not between the document row and the actions");
    assert_eq!(value_of(&state, SettingRow::Fixed(SettingId::SkinInfo)), "1920X1080 - JSON - 0 WARNINGS - dj", "the document was not read");
}

/// The rows a document declares are drawn, are clickable, and change what is on screen when
/// they move — which is what a row wired to the wrong document, or to nothing, would not do.
#[test]
fn the_skin_tabs_own_rows_are_drawn_and_redraw_when_one_moves() {
    let mut app = app();
    let mut state = skin_screen(&mut app, "draw");
    let bare = draw(&mut app, &mut state);
    assert!(bare.painted_pixels() > 0, "the SKIN tab drew nothing");

    focus(&mut state, SettingRow::Fixed(SettingId::SkinDocument));
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);

    let chosen = draw(&mut app, &mut state);
    assert_ne!(bare.signature(), chosen.signature(), "choosing a document did not change the screen");
    assert_eq!(app.shared.hot.len(), state.rows.len().min(visible_rows()) + SettingTab::ALL.len(), "every drawn row and every tab is clickable");

    let property = SettingRow::Skin(
        state
            .rows
            .iter()
            .find_map(|row| match row {
                SettingRow::Skin(skin) => Some(*skin),
                SettingRow::Fixed(_) => None,
            })
            .expect("the document declares a row"),
    );
    focus(&mut state, property);
    let before = value_of(&state, property);
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);
    assert_ne!(value_of(&state, property), before, "stepping the document's own row changed nothing");
    assert_ne!(chosen.pixel_checksum(), draw(&mut app, &mut state).pixel_checksum(), "the moved row was not redrawn");
}

/// Everything the SKIN tab edits goes into the settings file and comes back out of it, so the next
/// launch draws the screen with the document and the choices the player left it on.
#[test]
fn what_the_tab_edits_survives_the_settings_file() {
    let mut app = app();
    let mut state = skin_screen(&mut app, "persist");
    focus(&mut state, SettingRow::Fixed(SettingId::SkinDocument));
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);

    for row in [SettingRow::Skin(SkinRow::Property(0)), SettingRow::Skin(SkinRow::File(0)), SettingRow::Skin(SkinRow::Offset(0, OffsetAxis::Y))] {
        focus(&mut state, row);
        state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    }
    state.refresh(&app.shared);
    let shown: Vec<(String, String)> = state.lines.clone();

    state.row_key(&mut ctx(&mut app), &press(KeyCode::Escape));
    let restored = rbms_config::load(&app.shared.settings_path).expect("the file the screen wrote reads back").config;
    assert_eq!(restored.skin, app.shared.config.skin, "the skin group did not survive the file");

    let mut next = crate::stage::render_tests::app();
    next.shared.settings_path = app.shared.settings_path.clone();
    next.shared.config = restored;
    let mut reopened = SettingsState::on_tab(SettingTab::Skin);
    reopened.on_enter(&mut ctx(&mut next));
    assert_eq!(reopened.lines, shown, "the reopened screen does not show what the last one left");
}

/// An offset row is the one row kind zeroed rather than cycled, and the key that does it is the
/// same one on the number row and the keypad.
#[test]
fn the_zero_key_puts_an_offset_row_back_to_the_authors_placement() {
    let mut app = app();
    let mut state = skin_screen(&mut app, "zero");
    focus(&mut state, SettingRow::Fixed(SettingId::SkinDocument));
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);

    let offset =
        *state.rows.iter().find(|row| matches!(row, SettingRow::Skin(crate::skin_select::SkinRow::Offset(_, _)))).expect("the document declares an offset row");
    focus(&mut state, offset);
    for _ in 0..3 {
        state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    }
    state.refresh(&app.shared);
    assert_eq!(value_of(&state, offset), "+3");

    state.row_key(&mut ctx(&mut app), &press(KeyCode::Digit0));
    state.refresh(&app.shared);
    assert_eq!(value_of(&state, offset), "+0");
}

/// RESET drops the choices made in the document and writes the settings file, so the next
/// launch starts on what the author shipped rather than on what was reset away.
#[test]
fn reset_drops_the_choices_and_the_file_no_longer_holds_them() {
    let mut app = app();
    let mut state = skin_screen(&mut app, "reset");
    focus(&mut state, SettingRow::Fixed(SettingId::SkinDocument));
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);

    let property = *state.rows.iter().find(|row| matches!(row, SettingRow::Skin(_))).expect("the document declares a row");
    focus(&mut state, property);
    state.row_key(&mut ctx(&mut app), &press(KeyCode::ArrowRight));
    state.refresh(&app.shared);
    let moved = value_of(&state, property);
    assert!(!app.shared.config.skin.custom.is_empty(), "the choice was never stored");

    focus(&mut state, SettingRow::Fixed(SettingId::SkinReset));
    state.row_key(&mut ctx(&mut app), &press(KeyCode::Enter));
    state.refresh(&app.shared);
    assert!(app.shared.config.skin.custom.is_empty(), "the choices survived the reset");
    assert_ne!(value_of(&state, property), moved, "the row still shows what was reset away");

    let written = std::fs::read_to_string(&app.shared.settings_path).expect("the settings file was written");
    assert!(written.contains("skin:"), "the settings file does not hold the skin group");
}
