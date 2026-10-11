//! The offsets of the previewed play skin on the play menu: that the ones it declares to move a
//! part are listed with its options and the rest in two folds at the end of the list, how a fold
//! is opened and closed, that an opacity and a nudge are read and stepped here as the skin menu
//! reads and steps them and stored in the same place, and that the preview is drawn with one on
//! the frame after its row moved.
//!
//! The menu is opened over the fading play document of the skin menu's fixtures, a seven-key play
//! skin made of flat parts that each name an offset of their own, so a part of a frame can be told
//! by its colour.

use rbms_render::Color;

use super::*;
use crate::skin_select::fixtures::{
    FADING_GLOW, FADING_GROUND_OPACITY, FADING_GROUND_ROW, FADING_JUDGE_LINE_ROW, FADING_JUDGE_PART, FADING_LINE_ROW, FADING_NOTES_ROW, FADING_OPACITIES,
    FADING_PANEL, FADING_PANEL_OPACITY, FADING_PANEL_ROW, JUDGE_OFFSET, write_fading_play,
};
use crate::skin_select::{OffsetAxis, SkinRowKind, opacity_alpha, skin_root};
use crate::stage::skin_menu::{automatic_offset_help, skin_row_help};

/// The size the fading play document is authored at.
const AUTHORED: (f32, f32) = (1280.0, 720.0);

/// A point of the fading document's panel that none of its other parts covers, in the document's
/// own pixels, measured up from its foot.
const ON_THE_PANEL: (f32, f32) = (500.0, 100.0);

/// The alpha a part is left with at half of its opacity.
const HALF_ALPHA: u32 = 127;

/// How many rows the offsets every play skin has come to, which is what the fold of the offsets
/// that move holds.
const PLACE_ROWS: usize = 15;

/// The rows of the offsets the fading document declares itself that move what names them, as they
/// are listed: two of the mark's and two of the glow's.
const DECLARED_PLACE_ROWS: [&str; 4] = ["MARK PLACE X", "MARK PLACE Y", "GLOW Y", "GLOW ALPHA"];

/// How far the judgement's stand-in is raised to see which way a positive Y goes, in pixels of the
/// document, which a preview draws smaller.
const RAISED: i32 = 60;

/// An app whose skin folder holds the fading play document, chosen for the seven-key play screen.
/// The skin menu is left configuring the song browser, so a row read through the screen it
/// configures is not read by accident.
fn app_with_fading_play(tag: &str) -> App {
    let mut app = bare_app(tag);
    let folder = skin_root(&app.shared.settings_path, &app.shared.config);
    let path = write_fading_play(&folder);
    app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    app.shared.config.skin.select(PREVIEW_MODES[0].0, Some(path.to_string_lossy().into_owned()));
    app
}

impl Menu {
    /// The row of the list whose label is exactly `label`.
    fn row_called(&self, label: &str) -> Row {
        let found = self.state.rows.iter().copied().find(|row| self.line(*row).label == label);
        found.unwrap_or_else(|| panic!("no row is called {label} among {:?}", self.state.rows.iter().map(|row| self.line(*row).label).collect::<Vec<_>>()))
    }

    /// Put the cursor on a fold and step it once, which shows its rows or hides them.
    fn toggle(&mut self, fold: Fold) {
        self.focus(Row::Fold(fold));
        self.key(KeyCode::Enter);
    }

    /// The offset stored under `id` for the document of the previewed mode.
    fn stored(&self, id: i32) -> Option<rbms_skin::dst::SkinOffset> {
        let config = &self.app.shared.config;
        let path = self.app.shared.skins.document_path(config, self.state.target().screen)?;
        config.skin.customisation(path)?.offsets.get(&id).copied()
    }

    /// What the skin menu would read for the row of the previewed play skin called `label`, were it
    /// configuring that screen.
    fn as_the_skin_menu_reads(&self, label: &str) -> String {
        let screen = self.state.target().screen;
        let config = Config { skin: rbms_config::SkinOptions { screen, ..self.app.shared.config.skin.clone() }, ..self.app.shared.config.clone() };
        let skins = &self.app.shared.skins;
        let row = skins.rows(&config).into_iter().find(|row| skins.line(&config, *row).0 == label).unwrap_or_else(|| panic!("the skin menu has no {label}"));
        skins.line(&config, row).1
    }

    /// The pixel of the frame drawn last that shows a point of the previewed document, given in the
    /// document's own pixels and measured up from its foot.
    fn previewed(&self, at: (f32, f32)) -> Color {
        let across = PREVIEW_RECT.x + at.0 * PREVIEW_RECT.w / AUTHORED.0;
        let down = PREVIEW_RECT.y + (AUTHORED.1 - at.1) * PREVIEW_RECT.h / AUTHORED.1;
        self.pixels.pixel_at(across as u32, down as u32)
    }
}

/// The colour a part of the fading play document is tinted, opaque.
fn tint_of(color: (u8, u8, u8)) -> Color {
    Color::rgb(color.0, color.1, color.2)
}

/// What a part tinted `color` comes to over black at `alpha`, by the canvas's own arithmetic.
fn over_black(color: (u8, u8, u8), alpha: u32) -> Color {
    let dim = |channel: u8| (u32::from(channel) * alpha / u32::from(u8::MAX)) as u8;
    Color::rgb(dim(color.0), dim(color.1), dim(color.2))
}

/// The offsets the play skin declares to move a part of its own are rows at the end of the lane
/// group, where its options are, on show as the menu opens. The rest are the last group of the
/// list: a fold of its opacities and a fold of the offsets every play skin has, each one row while
/// it is closed, which is how the menu opens. A fold says how many rows it holds. A skin that
/// declares no offset adds nothing.
#[test]
fn the_offsets_of_the_play_skin_are_rows_of_the_lane_group_and_two_closed_folds_at_the_end_of_the_list() {
    let menu = Menu::over(app_with_fading_play("folds"));
    let tail = [Row::Group(GROUP_SKIN), Row::Fold(Fold::Opacity), Row::Fold(Fold::Place)];
    let own: Vec<Row> = menu.state.rows.iter().copied().filter(|row| matches!(row, Row::Skin(_))).collect();
    let labels: Vec<String> = own.iter().map(|row| menu.line(*row).label).collect();
    assert_eq!(labels, DECLARED_PLACE_ROWS, "the offsets the skin declares to move a part are not rows of the closed list");
    let first = menu.at(own[0]);
    assert_eq!(menu.state.rows[first - 1], Row::Setting(*LANE_ROWS.last().expect("the lane group has rows")), "they do not follow the lane rows");
    assert!(matches!(menu.state.rows[first + own.len()], Row::Group(_)), "they are not the end of the lane group");
    let without_own: Vec<Row> = menu.state.rows.iter().copied().filter(|row| !own.contains(row)).collect();
    assert_eq!(&without_own[..without_own.len() - tail.len()], rows_without_skin_options(), "the rows above the skin's are not the menu's own");
    assert_eq!(&menu.state.rows[menu.state.rows.len() - tail.len()..], tail);
    assert_eq!(menu.line(Row::Group(GROUP_SKIN)).look, RowLook::Heading);
    let opacity = menu.line(Row::Fold(Fold::Opacity));
    assert_eq!((opacity.label.as_str(), opacity.value.as_str(), opacity.look), ("OPACITY", "HIDDEN (5)", RowLook::Plain));
    assert_eq!(FADING_OPACITIES, 5);
    let place = menu.line(Row::Fold(Fold::Place));
    assert_eq!((place.label.as_str(), place.value), ("POSITION AND SIZE", format!("HIDDEN ({PLACE_ROWS})")));
    assert_eq!(menu.state.focused(), Some(Row::PreviewBpm), "the cursor does not open where it always did");

    let bare = Menu::over(bare_app("folds-none"));
    assert_eq!(bare.state.rows, rows_without_skin_options(), "a menu with no play skin lists a group for its offsets");
}

/// A fold is opened by stepping it either way, by Enter and by a press of either button, and closed
/// the same way. Its rows are listed right under it, the cursor stays on the fold, and what is
/// under another fold is not touched. Opening one stores nothing and tells the preview nothing.
#[test]
fn a_fold_shows_and_hides_its_rows_and_keeps_the_cursor() {
    let mut menu = Menu::over(app_with_fading_play("fold-toggle"));
    let before = menu.app.shared.config.clone();
    let closed = menu.state.rows.len();
    menu.toggle(Fold::Opacity);
    assert_eq!(menu.state.focused(), Some(Row::Fold(Fold::Opacity)), "the cursor left the fold it opened");
    assert_eq!(menu.line(Row::Fold(Fold::Opacity)).value, "SHOWN (5)");
    assert_eq!(menu.state.rows.len(), closed + FADING_OPACITIES);
    let at = menu.at(Row::Fold(Fold::Opacity));
    let labels: Vec<String> = menu.state.rows[at + 1..at + 1 + FADING_OPACITIES].iter().map(|row| menu.line(*row).label).collect();
    assert_eq!(labels, [FADING_GROUND_ROW, FADING_LINE_ROW, FADING_JUDGE_LINE_ROW, FADING_NOTES_ROW, FADING_PANEL_ROW]);
    assert_eq!(menu.state.rows[at + 1 + FADING_OPACITIES], Row::Fold(Fold::Place), "the other fold does not follow the rows of this one");
    assert!(menu.state.top <= menu.state.sel && menu.state.sel < menu.state.top + VISIBLE_ROWS, "the cursor is off the screen");
    assert!(at + FADING_OPACITIES < menu.state.top + VISIBLE_ROWS, "the rows the fold showed are below the foot of the screen");

    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.state.rows.len(), closed, "an arrow did not close the fold");
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.state.rows.len(), closed + FADING_OPACITIES, "an arrow did not open it again");
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.state.rows.len(), closed + FADING_OPACITIES, "the zero key closed a fold");

    menu.toggle(Fold::Place);
    assert_eq!(menu.state.rows.len(), closed + FADING_OPACITIES + PLACE_ROWS);
    assert_eq!(menu.line(Row::Fold(Fold::Place)).value, format!("SHOWN ({PLACE_ROWS})"));
    let place = menu.at(Row::Fold(Fold::Place));
    assert!(menu.state.top <= place && place + PLACE_ROWS < menu.state.top + VISIBLE_ROWS, "the fold and the rows it showed are not all on the screen");
    menu.frame();
    let on_fold = menu.middle_of_hot(Hot::PlayMenuRow(menu.at(Row::Fold(Fold::Place))));
    menu.click(on_fold, MouseButton::Right);
    assert_eq!(menu.state.rows.len(), closed + FADING_OPACITIES, "a press of the right button did not close the fold");
    assert_eq!(menu.state.focused(), Some(Row::Fold(Fold::Place)));
    assert_eq!(menu.app.shared.config, before, "opening and closing a fold stored something");

    menu.key(KeyCode::Tab);
    assert_eq!(menu.state.focused(), Some(Row::Fold(Fold::Place)).filter(|row| menu.state.rows.contains(row)).or(menu.state.focused()));
    assert!(!menu.state.rows.contains(&Row::Fold(Fold::Opacity)), "a mode with no play skin lists the folds of another's");
}

/// An opacity is read and stepped here as the skin menu reads and steps it -- five percent a step,
/// a quarter of the way with Shift, back to a hundred with the zero key -- and is stored for the
/// play skin's document as the alpha offset the skin menu reads, with the screen the skin menu
/// configures left alone and nothing read again.
#[test]
fn an_opacity_is_stepped_here_and_read_by_the_skin_menu_from_the_same_place() {
    let mut menu = Menu::over(app_with_fading_play("opacity"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let build = menu.app.shared.skins.build_of(screen);
    menu.toggle(Fold::Opacity);
    let row = menu.row_called(FADING_GROUND_ROW);
    assert_eq!(menu.line(row).value, "100%");
    assert_eq!(menu.state.help_of(&menu.app.shared, row), skin_row_help(SkinRowKind::Opacity), "an opacity is not said as the skin menu says it");

    menu.focus(row);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.line(row).value, "105% (+13)", "an opacity cannot be added to");
    menu.key(KeyCode::ArrowLeft);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.line(row).value, "95%");
    menu.shifted(KeyCode::ArrowLeft);
    assert_eq!(menu.line(row).value, "70%", "Shift is not a quarter of the way");
    assert_eq!(menu.stored(FADING_GROUND_OPACITY).map(|offset| offset.a), Some(opacity_alpha(70)));
    assert_eq!(menu.as_the_skin_menu_reads(FADING_GROUND_ROW), "70%", "the skin menu reads the opacity from somewhere else");
    assert_eq!(menu.app.shared.config.skin.screen, SKIN_TYPE_MUSIC_SELECT, "the screen the skin menu configures moved");

    for expected in ["45%", "20%", "0%", "0%"] {
        menu.shifted(KeyCode::ArrowLeft);
        assert_eq!(menu.line(row).value, expected);
    }
    assert_eq!(menu.stored(FADING_GROUND_OPACITY).map(|offset| offset.a), Some(-255.0));
    menu.key(KeyCode::Enter);
    assert_eq!(menu.line(row).value, "5%", "Enter is not one step forwards");
    menu.frame();
    let on_row = menu.middle_of_hot(Hot::PlayMenuRow(menu.at(row)));
    menu.click(on_row, MouseButton::Left);
    menu.click(on_row, MouseButton::Right);
    menu.click(on_row, MouseButton::Right);
    assert_eq!(menu.line(row).value, "0%", "a press of the right button did not step the opacity back");

    menu.key(KeyCode::Digit0);
    assert_eq!(
        (menu.line(row).value.as_str(), menu.stored(FADING_GROUND_OPACITY).map(|offset| offset.a)),
        ("100%", Some(0.0)),
        "the zero key did not put it back"
    );
    assert_eq!(menu.as_the_skin_menu_reads(FADING_GROUND_ROW), "100%");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an opacity had the document read again");
    assert!(!menu.app.shared.skins.needs_reload_for(&menu.app.shared.config, screen));
}

/// What the skin menu stored is what this menu reads: an opacity and a nudge written for the play
/// skin's document before the menu was opened are on its rows as it opens.
#[test]
fn what_the_skin_menu_stored_is_read_here() {
    let mut app = app_with_fading_play("stored");
    let screen = PREVIEW_MODES[0].0;
    let path = app.shared.config.skin.document(screen).expect("the fading document is chosen").to_owned();
    let offsets = &mut app.shared.config.skin.customise(&path).offsets;
    offsets.insert(FADING_PANEL_OPACITY, rbms_skin::dst::SkinOffset { a: opacity_alpha(35), ..Default::default() });
    offsets.insert(JUDGE_OFFSET, rbms_skin::dst::SkinOffset { y: 24.0, a: -16.0, ..Default::default() });
    offsets.insert(FADING_GLOW, rbms_skin::dst::SkinOffset { y: -5.0, a: -90.0, ..Default::default() });
    let mut menu = Menu::over(app);
    assert_eq!(
        (menu.line(menu.row_called("GLOW Y")).value.as_str(), menu.line(menu.row_called("GLOW ALPHA")).value.as_str()),
        ("-5", "-90"),
        "with no fold open"
    );
    menu.toggle(Fold::Opacity);
    menu.toggle(Fold::Place);
    assert_eq!(menu.line(menu.row_called(FADING_PANEL_ROW)).value, "35%");
    assert_eq!(menu.line(menu.row_called("JUDGE OFFSET Y")).value, "+24");
    assert_eq!(menu.line(menu.row_called("JUDGE OFFSET ALPHA")).value, "-16", "the alpha of an offset that also moves is not read as a number");
    assert_eq!((menu.line(menu.row_called("GLOW Y")).value.as_str(), menu.line(menu.row_called("GLOW ALPHA")).value.as_str()), ("-5", "-90"));
    for label in [FADING_PANEL_ROW, "JUDGE OFFSET Y", "JUDGE OFFSET ALPHA", "GLOW ALPHA"] {
        assert_eq!(menu.line(menu.row_called(label)).value, menu.as_the_skin_menu_reads(label), "{label}");
    }
}

/// An offset that moves is stepped one pixel a press and ten with Shift held, put back by the zero
/// key, and said as the skin menu says it. The judgement offset every play skin has raises what
/// names it when its Y is positive: the preview shows it higher on the frame after the row moved.
#[test]
fn a_nudge_is_stepped_here_and_a_positive_y_raises_what_names_it() {
    let mut menu = Menu::over(app_with_fading_play("nudge"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let build = menu.app.shared.skins.build_of(screen);
    let ((x, y, w, h), color) = FADING_JUDGE_PART;
    let middle = ((x + w / 2) as f32, (y + h / 2) as f32);
    let raised = (middle.0, middle.1 + RAISED as f32);
    let lowered = (middle.0, middle.1 - RAISED as f32);
    assert_eq!(
        (menu.previewed(middle), menu.previewed(raised)),
        (tint_of(color), tint_of(FADING_PANEL.1)),
        "the judgement's stand-in is not where the skin put it"
    );

    menu.toggle(Fold::Place);
    let row = menu.row_called("JUDGE OFFSET Y");
    assert_eq!(menu.state.help_of(&menu.app.shared, row), automatic_offset_help(OffsetAxis::Y), "an offset the game adds is not said as one");
    let own = menu.row_called("MARK PLACE Y");
    assert_eq!(
        menu.state.help_of(&menu.app.shared, own),
        skin_row_help(SkinRowKind::Offset(OffsetAxis::Y)),
        "an offset of the skin is not said as the skin menu says it"
    );
    menu.focus(row);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.line(row).value, "+1");
    for _ in 0..RAISED / SHIFT_STEPS {
        menu.shifted(KeyCode::ArrowRight);
    }
    menu.key(KeyCode::ArrowLeft);
    assert_eq!((menu.line(row).value, menu.stored(JUDGE_OFFSET).map(|offset| offset.y)), (format!("+{RAISED}"), Some(RAISED as f32)));
    assert_eq!(menu.as_the_skin_menu_reads("JUDGE OFFSET Y"), format!("+{RAISED}"));

    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a nudge cost a frame of the preview");
    assert_eq!(menu.previewed(raised), tint_of(color), "a positive Y did not raise what names the judgement offset");
    assert_eq!(menu.previewed(middle), tint_of(FADING_PANEL.1), "it was also left where it had been");
    assert_eq!(menu.previewed(lowered), tint_of(FADING_PANEL.1));

    for _ in 0..2 * RAISED / SHIFT_STEPS {
        menu.shifted(KeyCode::ArrowLeft);
    }
    menu.frame();
    assert_eq!(menu.line(row).value, format!("-{RAISED}"));
    assert_eq!(menu.previewed(lowered), tint_of(color), "a negative Y did not lower it");

    menu.key(KeyCode::Digit0);
    menu.frame();
    assert_eq!((menu.line(row).value.as_str(), menu.previewed(middle)), ("+0", tint_of(color)), "the zero key did not put the nudge back");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "a nudge had the document read again");
}

/// The preview is drawn with an opacity on the frame after its row moved, with the document
/// neither read nor compiled again.
#[test]
fn the_preview_is_drawn_with_an_opacity_on_the_frame_after_its_row_moved() {
    let mut menu = Menu::over(app_with_fading_play("opacity-preview"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let build = menu.app.shared.skins.build_of(screen);
    assert_eq!(menu.previewed(ON_THE_PANEL), tint_of(FADING_PANEL.1), "the panel is not where the skin put it");

    menu.toggle(Fold::Opacity);
    menu.focus(menu.row_called(FADING_PANEL_ROW));
    menu.shifted(KeyCode::ArrowLeft);
    menu.shifted(KeyCode::ArrowLeft);
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a step of an opacity cost a frame of the preview");
    assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA), "the panel is not at half on the frame after its row moved");

    menu.shifted(KeyCode::ArrowLeft);
    menu.shifted(KeyCode::ArrowLeft);
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), Color::rgb(0, 0, 0), "a panel with none of its opacity left is drawn");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an opacity had the document read again");
}

/// What the menu writes with a fold open fits where it is written: the name and the value of every
/// row of a fold in its row, and what is said of each on one line under the preview.
#[test]
fn what_the_rows_of_a_fold_write_fits_where_it_is_written() {
    rbms_render::font::use_embedded_fonts_only();
    let mut menu = Menu::over(app_with_fading_play("fold-fits"));
    menu.toggle(Fold::Opacity);
    menu.toggle(Fold::Place);
    let room = row_room();
    for row in menu.state.rows.clone() {
        let line = menu.line(row);
        if line.look == RowLook::Heading {
            continue;
        }
        let value = fitted_value(&line.value);
        assert_eq!(value, line.value, "the value of {} is cut short", line.label);
        let label_room = room - text_width(&value, VALUE_SCALE) - LABEL_GAP;
        assert_eq!(fit_text(&line.label, LABEL_SCALE, label_room), line.label, "the name of {} is cut short", line.label);
        let said = menu.state.help_of(&menu.app.shared, row);
        assert_eq!(fit_text(said, INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0), said, "what is said of {} is cut short", line.label);
    }
    assert_eq!(fit_text(HINT, HINT_SCALE, CW as f32 - MARGIN * 2.0), HINT, "the line of keys is cut short");
}

/// The folds drawn for a person to look at: both closed, as the menu opens; the opacities open
/// with the lane background and its lines taken away and the panel at half; and the offsets that
/// move open with the judgement raised.
#[test]
fn the_folds_of_a_play_skins_offsets_can_be_captured() {
    let mut menu = Menu::over(app_with_fading_play("fold-capture"));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    menu.focus(Row::Fold(Fold::Opacity));
    menu.capture_at("play-menu-folds-closed", RUN_SECONDS);

    menu.toggle(Fold::Opacity);
    for (label, wide_steps) in [(FADING_GROUND_ROW, 4), (FADING_LINE_ROW, 4), (FADING_PANEL_ROW, 2)] {
        menu.focus(menu.row_called(label));
        for _ in 0..wide_steps {
            menu.shifted(KeyCode::ArrowLeft);
        }
    }
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA));
    menu.save("play-menu-opacity-faded");

    menu.toggle(Fold::Opacity);
    menu.toggle(Fold::Place);
    menu.focus(menu.row_called("JUDGE OFFSET Y"));
    for _ in 0..RAISED / SHIFT_STEPS {
        menu.shifted(KeyCode::ArrowRight);
    }
    menu.frame();
    menu.save("play-menu-judge-raised");
}

/// Every opacity the shipped pack's seven-key play skin declares is taken down to nothing and put
/// back from this menu, with the preview drawn on every frame between and the document never read
/// again. A pack that declares no opacity has no fold for them, and there is then nothing to step.
#[test]
fn every_opacity_of_the_shipped_play_skin_is_taken_to_nothing_and_put_back() {
    let mut menu = Menu::over(app_with_shipped_pack("shipped-opacity"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    if !menu.state.rows.contains(&Row::Fold(Fold::Opacity)) {
        return;
    }
    let before = menu.app.shared.config.skin.clone();
    let build = menu.app.shared.skins.build_of(screen);
    menu.toggle(Fold::Opacity);
    let opacities: Vec<Row> = folded(&menu.app.shared, screen, Fold::Opacity).into_iter().map(Row::Skin).collect();
    assert_eq!(menu.line(Row::Fold(Fold::Opacity)).value, format!("SHOWN ({})", opacities.len()));
    menu.capture_at("play-menu-shipped-opacity-all", RUN_SECONDS);

    for row in &opacities {
        let label = menu.line(*row).label;
        assert_eq!(menu.line(*row).value, "100%", "{label} does not open at a hundred percent");
        menu.focus(*row);
        for _ in 0..4 {
            menu.shifted(KeyCode::ArrowLeft);
        }
        assert_eq!(menu.line(*row).value, "0%", "{label} was not taken to nothing by four wide steps");
        menu.frame();
        assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "taking {label} to nothing cost a frame of the preview");
    }
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an opacity had the document read again");
    menu.save("play-menu-shipped-opacity-none");

    for row in &opacities {
        menu.focus(*row);
        menu.key(KeyCode::Digit0);
        assert_eq!(menu.line(*row).value, "100%");
    }
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn);
    let document = menu.app.shared.skins.document_path(&menu.app.shared.config, screen).expect("the pack has a seven-key play skin").to_owned();
    let stored = menu.app.shared.config.skin.customisation(&document).map(|entry| entry.offsets.clone()).unwrap_or_default();
    assert!(stored.values().all(|offset| *offset == rbms_skin::dst::SkinOffset::default()), "an opacity was left moved: {stored:?}");
    assert_eq!(menu.app.shared.config.skin.selected, before.selected);
}

/// Where the shipped pack's seven-key play skin writes its judgement and whether it writes it are
/// rows of the lane group, on the list as the menu opens with no fold opened: the offset it declares
/// for that right after its options. The offset is stepped a pixel a press and ten with Shift held,
/// stored for the skin's document under the offset's own id with nothing read again, and the
/// preview is drawn on the frame after; the option that hides the judgement has its document read
/// again and drawn. A pack that declares neither has nothing to step here.
#[test]
fn the_judgement_of_the_shipped_play_skin_is_moved_and_hidden_from_the_lane_rows() {
    /// The rows of the shipped pack's play skin this is about, and how far the judgement is raised.
    const POSITION_ROW: &str = "JUDGE POSITION Y";
    const DISPLAY_OPTION: &str = "JUDGE DISPLAY";
    const RAISED_BY: i32 = 100;
    let mut menu = Menu::over(app_with_shipped_pack("shipped-judge"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let Some(position) = menu.state.rows.iter().copied().find(|row| menu.line(*row).label == POSITION_ROW) else {
        return;
    };
    let display = menu.skin_option(DISPLAY_OPTION);
    assert!(menu.at(display) < menu.at(position) && menu.at(position) < menu.at(Row::Group("JUDGE")), "the judgement's rows are not the end of the lane group");
    assert_eq!((menu.line(position).value.as_str(), menu.line(display).value.as_str()), ("+0", "SHOW"));
    assert_eq!(menu.state.help_of(&menu.app.shared, position), skin_row_help(SkinRowKind::Offset(OffsetAxis::Y)));

    let build = menu.app.shared.skins.build_of(screen);
    menu.focus(position);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.line(position).value, "+1");
    menu.key(KeyCode::ArrowLeft);
    for _ in 0..RAISED_BY / SHIFT_STEPS {
        menu.shifted(KeyCode::ArrowRight);
    }
    assert_eq!(menu.line(position).value, format!("+{RAISED_BY}"));
    assert_eq!(menu.as_the_skin_menu_reads(POSITION_ROW), format!("+{RAISED_BY}"), "the skin menu reads the position from somewhere else");
    let Row::Skin(SkinRow::Offset(at, _)) = position else {
        panic!("{POSITION_ROW} is listed as {position:?}");
    };
    let id = menu.app.shared.skins.header_of(&menu.app.shared.config, screen).map(|header| header.offsets[at].id).expect("the play skin has its header");
    assert_eq!(menu.stored(id).map(|offset| offset.y), Some(RAISED_BY as f32));
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "moving the judgement cost a frame of the preview");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "moving the judgement had the document read again");
    menu.capture_at("play-menu-shipped-judge-raised", RUN_SECONDS);

    menu.focus(display);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.line(display).value, "HIDE");
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "the preview did not come back with the judgement hidden");
    menu.capture_at("play-menu-shipped-judge-hidden", RUN_SECONDS);

    menu.key(KeyCode::Digit0);
    menu.focus(position);
    menu.key(KeyCode::Digit0);
    assert_eq!((menu.line(position).value.as_str(), menu.line(display).value.as_str()), ("+0", "SHOW"), "the zero key did not put the judgement back");
}
