//! The opacity rows of the skin menu: how an offset that only fades is listed, read and stepped,
//! that what the rows write is the offset the skin format reads, and that the preview is drawn with
//! it on the frame after a row moved.
//!
//! The menu is opened over the fading play document of the skin menu's fixtures, a seven-key play
//! skin made of flat parts that each name an offset of their own, so a part of a frame can be told
//! by its colour.

use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

use super::*;
use crate::skin_select::fixtures::{
    FADING_GLOW, FADING_GROUND_OPACITY, FADING_GROUND_ROW, FADING_JUDGE_LINE_ROW, FADING_LINE_ROW, FADING_NOTES_ROW, FADING_OPACITIES, FADING_PANEL,
    FADING_PANEL_OPACITY, FADING_PANEL_ROW, write_fading_play,
};
use crate::skin_select::{opacity_alpha, skin_root};
use crate::stage::preview_menu::VISIBLE_ROWS;

/// The size the fading play document is authored at.
const AUTHORED: (f32, f32) = (1280.0, 720.0);

/// A point of the fading document's panel that none of its other parts covers, in the document's
/// own pixels, measured up from its foot.
const ON_THE_PANEL: (f32, f32) = (500.0, 100.0);

/// The alpha a part is left with at half of its opacity.
const HALF_ALPHA: u32 = 127;

/// How far into its scene the preview is captured.
const CAPTURE_SECONDS: f32 = 6.0;

/// An app whose skin folder holds the fading play document, chosen for the seven-key play screen,
/// which the menu is configuring.
fn app_with_fading_play(tag: &str) -> App {
    let mut app = bare_app(tag);
    let folder = skin_root(&app.shared.settings_path, &app.shared.config);
    let path = write_fading_play(&folder);
    app.shared.config.skin.screen = SKIN_TYPE_PLAY_7KEYS;
    app.shared.config.skin.select(SKIN_TYPE_PLAY_7KEYS, Some(path.to_string_lossy().into_owned()));
    app
}

impl Menu {
    /// The row of the list whose label is exactly `label`.
    fn row_called(&self, label: &str) -> SettingRow {
        let at = self.state.lines.iter().position(|line| line.label == label).unwrap_or_else(|| panic!("no row is called {label} among {:?}", self.labels()));
        self.state.rows[at]
    }

    /// A key pressed with Shift held, and Shift let go of after it.
    fn shifted(&mut self, code: KeyCode) {
        self.key(KeyCode::ShiftLeft);
        self.key(code);
        self.key_up(KeyCode::ShiftLeft);
    }

    /// The alpha stored for one offset of the document being configured.
    fn stored_alpha(&self, id: i32) -> Option<f32> {
        let config = &self.app.shared.config;
        let path = self.app.shared.skins.document_path(config, config.skin.screen)?;
        config.skin.customisation(path)?.offsets.get(&id).map(|offset| offset.a)
    }

    /// The pixel of the frame drawn last that shows a point of the previewed document, given in the
    /// document's own pixels and measured up from its foot.
    fn previewed(&self, at: (f32, f32)) -> Color {
        let across = PREVIEW_RECT.x + at.0 * PREVIEW_RECT.w / AUTHORED.0;
        let down = PREVIEW_RECT.y + (AUTHORED.1 - at.1) * PREVIEW_RECT.h / AUTHORED.1;
        pixel(&self.pixels, (across, down))
    }
}

/// What a part tinted `color` comes to over black at `alpha`, by the canvas's own arithmetic.
fn over_black(color: (u8, u8, u8), alpha: u32) -> Color {
    let dim = |channel: u8| (u32::from(channel) * alpha / u32::from(u8::MAX)) as u8;
    Color::rgb(dim(color.0), dim(color.1), dim(color.2))
}

/// The opacities of a play skin are listed under the names the skin gave them, after every offset
/// that moves and before the two actions, each reading all of its opacity.
#[test]
fn the_opacities_of_a_skin_are_listed_together_under_their_own_names() {
    let menu = Menu::over(app_with_fading_play("opacity-list"));
    let labels = menu.labels();
    let opacities = [FADING_GROUND_ROW, FADING_LINE_ROW, FADING_JUDGE_LINE_ROW, FADING_NOTES_ROW, FADING_PANEL_ROW];
    let first = labels.iter().position(|label| label == FADING_GROUND_ROW).expect("the ground's opacity is listed");
    assert_eq!(&labels[first..first + FADING_OPACITIES], opacities);
    assert_eq!(&labels[first + FADING_OPACITIES..], ["RELOAD", "RESET"], "the opacities are not the last rows of the document");
    assert_eq!(labels[first - 1], "JUDGE DETAIL OFFSET ALPHA", "an offset that moves is listed among the opacities");
    for label in opacities {
        assert_eq!(menu.value_of(menu.row_called(label)), "100%", "{label}");
        assert_eq!(row_help(&menu.app.shared, menu.row_called(label)), SkinMenuText::OpacityHelp.get(language()));
    }
    assert_eq!(
        row_help(&menu.app.shared, menu.row_called("GLOW ALPHA")),
        SkinMenuText::OffsetAlphaHelp.get(language()),
        "the alpha of an offset that also moves is said as an opacity"
    );
    let alpha_help = SkinMenuText::OffsetAlphaHelp.get(language());
    assert!(alpha_help.contains("-255") && alpha_help.contains("+255"), "the range of a raw alpha is not said: {alpha_help}");
    assert_eq!(
        row_help(&menu.app.shared, menu.row_called("MARK PLACE Y")),
        SkinMenuText::OffsetPlaceHelp.get(language()),
        "an offset the skin declares is not said as the skin's"
    );
    let added = [
        ("JUDGE OFFSET Y", SkinMenuText::AutomaticPlaceHelp.get(language())),
        ("ALL OFFSET(%) X", SkinMenuText::AutomaticPlaceHelp.get(language())),
        ("JUDGE DETAIL OFFSET ALPHA", SkinMenuText::AutomaticAlphaHelp.get(language())),
    ];
    for (label, help) in added {
        assert_eq!(row_help(&menu.app.shared, menu.row_called(label)), help, "{label} is not said as an offset the game adds to every play skin");
    }
}

/// An opacity row is stepped one percent by an arrow and ten with Shift held, stops at none of it
/// and at all of it, and is put back to a hundred by the zero key. A press of the right button
/// takes a step back and one of the left on the row's label a step forwards. What is stored is the
/// alpha offset, and the document is not read again.
#[test]
fn an_opacity_row_is_stepped_by_one_and_by_ten_percent_and_put_back_by_the_zero_key() {
    let mut menu = Menu::over(app_with_fading_play("opacity-keys"));
    let screen = menu.app.shared.config.skin.screen;
    let build = menu.app.shared.skins.build_of(screen);
    let row = menu.row_called(FADING_GROUND_ROW);
    menu.focus(row);

    menu.key(KeyCode::ArrowRight);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)), ("100%", None), "an opacity went past all of it");
    menu.key(KeyCode::ArrowLeft);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)), ("99%", Some(opacity_alpha(99))), "one press is not one percent");
    menu.keys(KeyCode::ArrowLeft, 4);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)), ("95%", Some(opacity_alpha(95))));
    menu.shifted(KeyCode::ArrowLeft);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)), ("85%", Some(opacity_alpha(85))), "Shift is not ten percent");
    menu.shifted(KeyCode::ArrowRight);
    menu.shifted(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(row), "100%", "a wide step past all of the opacity did not stop at it");
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.value_of(row), "100%");

    for expected in ["90%", "80%", "70%", "60%", "50%", "40%", "30%", "20%", "10%", "0%", "0%"] {
        menu.shifted(KeyCode::ArrowLeft);
        assert_eq!(menu.value_of(row), expected);
    }
    assert_eq!(menu.stored_alpha(FADING_GROUND_OPACITY), Some(-255.0), "none of the opacity is not the whole alpha taken away");
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of(row), "0%", "an opacity went below none");
    menu.shifted(KeyCode::ArrowRight);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of(row), "9%");

    menu.key(KeyCode::Digit0);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GROUND_OPACITY)), ("100%", Some(0.0)), "the zero key did not put the opacity back");

    menu.frame();
    let on_row = menu.on_the_label_of(row);
    menu.right_click(on_row);
    menu.right_click(on_row);
    assert_eq!(menu.value_of(row), "98%", "the right button did not step the opacity back");
    menu.click(on_row);
    assert_eq!(menu.value_of(row), "99%", "the left button did not step it forwards");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an opacity had the document read again");
    assert_eq!(menu.value_of(menu.row_called(FADING_NOTES_ROW)), "100%", "one opacity moved another");
}

/// The alpha of an offset that also moves stays a number that is added, stepped one at a time and
/// ten with Shift held, as every axis of an offset is.
#[test]
fn the_alpha_of_an_offset_that_also_moves_stays_a_number() {
    let mut menu = Menu::over(app_with_fading_play("opacity-raw"));
    let row = menu.row_called("GLOW ALPHA");
    menu.focus(row);
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of(row), "-1");
    menu.shifted(KeyCode::ArrowLeft);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_alpha(FADING_GLOW)), ("-11", Some(-11.0)));
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.value_of(row), "+0");
}

/// What the rows write survives the settings file, and a menu opened on the file reads the same
/// opacities.
#[test]
fn the_opacities_survive_the_settings_file() {
    let mut menu = Menu::over(app_with_fading_play("opacity-persist"));
    for (label, wide_steps) in [(FADING_GROUND_ROW, 10), (FADING_PANEL_ROW, 5)] {
        menu.focus(menu.row_called(label));
        for _ in 0..wide_steps {
            menu.shifted(KeyCode::ArrowLeft);
        }
    }
    let shown = menu.state.lines.clone();
    assert!(matches!(menu.key(KeyCode::Escape), Transition::Back));
    let restored = rbms_config::load(&menu.app.shared.settings_path).expect("the file the menu wrote reads back").config;
    assert_eq!(restored.skin, menu.app.shared.config.skin);

    let mut next = app();
    next.shared.settings_path = menu.app.shared.settings_path.clone();
    next.shared.config = restored;
    next.shared.skins = SkinLibrary::new(&next.shared.settings_path, &next.shared.config);
    let reopened = Menu::over(next);
    assert_eq!(reopened.state.lines, shown, "the reopened menu does not show what the last one left");
    assert_eq!(
        (reopened.value_of(reopened.row_called(FADING_GROUND_ROW)).as_str(), reopened.value_of(reopened.row_called(FADING_PANEL_ROW)).as_str()),
        ("0%", "50%")
    );
}

/// The preview is drawn with an opacity on the frame after its row moved: the part that names it is
/// fainter, then gone, with the document neither read nor compiled again in between.
#[test]
fn the_preview_is_drawn_with_an_opacity_on_the_frame_after_its_row_moved() {
    let mut menu = Menu::over(app_with_fading_play("opacity-preview"));
    let screen = menu.app.shared.config.skin.screen;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let build = menu.app.shared.skins.build_of(screen);
    assert_eq!(menu.previewed(ON_THE_PANEL), Color::rgb(FADING_PANEL.1.0, FADING_PANEL.1.1, FADING_PANEL.1.2), "the panel is not where the skin put it");

    menu.focus(menu.row_called(FADING_PANEL_ROW));
    (0..5).for_each(|_| menu.shifted(KeyCode::ArrowLeft));
    assert_eq!(menu.stored_alpha(FADING_PANEL_OPACITY), Some(opacity_alpha(50)));
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a step of an opacity cost a frame of the preview");
    assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA), "the panel is not at half on the frame after its row moved");

    (0..5).for_each(|_| menu.shifted(KeyCode::ArrowLeft));
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), Color::rgb(0, 0, 0), "a panel with none of its opacity left is drawn");
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "an opacity had the document read again");

    menu.key(KeyCode::Digit0);
    menu.frame();
    assert_eq!(menu.previewed(ON_THE_PANEL), Color::rgb(FADING_PANEL.1.0, FADING_PANEL.1.1, FADING_PANEL.1.2), "the panel did not come back with its opacity");
}

/// The opacity rows drawn for a person to look at: the lane background and its lines taken away,
/// the panel at half, with the cursor on an opacity and the preview drawn with all of it.
#[test]
fn the_opacity_rows_of_a_play_skin_can_be_captured() {
    let mut menu = Menu::over(app_with_fading_play("opacity-capture"));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    menu.capture_at("skin-menu-opacity-all", CAPTURE_SECONDS);
    for (label, wide_steps) in [(FADING_GROUND_ROW, 10), (FADING_LINE_ROW, 10), (FADING_PANEL_ROW, 5)] {
        menu.focus(menu.row_called(label));
        for _ in 0..wide_steps {
            menu.shifted(KeyCode::ArrowLeft);
        }
    }
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn);
    assert!(
        menu.rows_on_show().len() == VISIBLE_ROWS && menu.rows_on_show().contains(&menu.at(menu.row_called(FADING_PANEL_ROW))),
        "the cursor's row is not drawn"
    );
    menu.save("skin-menu-opacity-faded");
    assert_eq!(menu.previewed(ON_THE_PANEL), over_black(FADING_PANEL.1, HALF_ALPHA));
}

/// Every offset row of the seven-key play skin of the pack [`SKIN_PACK_ENV`] names behaves as its
/// kind says, whatever the pack calls it: an offset the pack lets the player move along its alpha
/// alone reads in percent, a hundred with nothing stored, steps one percent down from there and no
/// further up, and is stored as the alpha offset that comes to; every other axis reads the number
/// that is added and steps by one, as it always did. Each row is put back as it was found.
///
/// A published pack names such an offset on a shade it lays over a lane and brings up with it, so
/// a percent above a hundred has to be there for it: a figure typed into the row reaches it, and
/// the arrows then step the row from where it stands. Opt-in, because the pack is outside the
/// repository: without the variable this checks nothing.
#[test]
fn the_offset_rows_of_the_pack_the_environment_names_behave_as_their_kind_says() {
    let Some(pack) = pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let mut menu = Menu::over(app_with_pack("opacity-env", &pack));
    menu.choose(SKIN_TYPE_PLAY_7KEYS);
    let screen = menu.app.shared.config.skin.screen;
    let rows: Vec<SettingRow> = menu.state.rows.iter().copied().filter(|row| matches!(row, SettingRow::Skin(SkinRow::Offset(_, _)))).collect();
    assert!(!rows.is_empty(), "the pack's seven-key play skin has no offset row: {:?}", menu.labels());
    let kind_of = |menu: &Menu, row: SettingRow| match row {
        SettingRow::Skin(skin_row) => menu.app.shared.skins.kind_for(&menu.app.shared.config, screen, skin_row),
        SettingRow::Fixed(_) => None,
    };
    let kinds: Vec<Option<SkinRowKind>> = rows.iter().map(|row| kind_of(&menu, *row)).collect();
    let first_opacity = kinds.iter().position(|kind| *kind == Some(SkinRowKind::Opacity)).unwrap_or(kinds.len());
    assert!(kinds[first_opacity..].iter().all(|kind| *kind == Some(SkinRowKind::Opacity)), "an offset that moves is listed after an opacity: {kinds:?}");

    let before = menu.app.shared.config.skin.clone();
    for (row, kind) in rows.iter().copied().zip(kinds) {
        let label = menu.state.lines[menu.at(row)].label.clone();
        menu.focus(row);
        match kind {
            Some(SkinRowKind::Opacity) => {
                assert_eq!(menu.value_of(row), "100%", "{label} does not open at a hundred percent");
                menu.key(KeyCode::ArrowRight);
                assert_eq!(menu.value_of(row), "100%", "{label}: an arrow carried an opacity past all of it");
                menu.key(KeyCode::ArrowLeft);
                assert_eq!(menu.value_of(row), "99%", "{label}");
                menu.key(KeyCode::Enter);
                for digit in ["1", "5", "0"] {
                    menu.typed(digit);
                }
                menu.key(KeyCode::Enter);
                assert_eq!(menu.value_of(row), "150% (+128)", "{label} cannot be added to");
                menu.key(KeyCode::ArrowLeft);
                assert_eq!(menu.value_of(row), "149% (+125)", "{label} is not stepped from where a typed figure put it");
                assert_eq!(row_help(&menu.app.shared, row), SkinMenuText::OpacityHelp.get(language()), "{label}");
            }
            Some(SkinRowKind::Offset(axis)) => {
                assert_eq!(menu.value_of(row), "+0", "{label} does not open at no offset");
                menu.key(KeyCode::ArrowRight);
                assert_eq!(menu.value_of(row), "+1", "{label}");
                menu.key(KeyCode::ArrowLeft);
                menu.key(KeyCode::ArrowLeft);
                assert_eq!(menu.value_of(row), "-1", "{label}");
                let SettingRow::Skin(skin_row) = row else {
                    panic!("{label} is no row of the document");
                };
                let added = menu.app.shared.skins.is_automatic_for(&menu.app.shared.config, screen, skin_row);
                let help = if added { automatic_offset_help(axis) } else { skin_row_help(SkinRowKind::Offset(axis)) };
                assert_eq!(row_help(&menu.app.shared, row), help, "{label}");
            }
            other => panic!("{label} is listed as an offset and is {other:?}"),
        }
        menu.key(KeyCode::Digit0);
        assert!(matches!(menu.value_of(row).as_str(), "100%" | "+0"), "{label} was not put back");
    }
    let document = menu.app.shared.skins.document_path(&menu.app.shared.config, screen).expect("the pack has a seven-key play skin").to_owned();
    let stored = menu.app.shared.config.skin.customisation(&document).expect("the rows that were stepped are stored");
    assert!(stored.offsets.values().all(|offset| *offset == rbms_skin::dst::SkinOffset::default()), "a row was left moved: {:?}", stored.offsets);
    assert_eq!(menu.app.shared.config.skin.selected, before.selected, "stepping an offset chose another document");

    if capturing() {
        let opacities: Vec<SettingRow> = rows.iter().copied().filter(|row| kind_of(&menu, *row) == Some(SkinRowKind::Opacity)).collect();
        if let Some(last) = opacities.last().copied() {
            menu.focus(last);
            menu.capture_at("skin-menu-pack-7k-opacity", CAPTURE_SECONDS);
        }
    }
}

/// The rows of the shipped pack's seven-key play skin a player who wants nothing on the field but
/// what is played takes down to nothing.
const SHIPPED_BARE_ROWS: [&str; 8] = [
    "LANE BACKGROUND OPACITY",
    "LANE DIVIDER OPACITY",
    "BAR LINE OPACITY",
    "PROGRESS BAR OPACITY",
    "SCORE GRAPH OPACITY",
    "SONG HEADER OPACITY",
    "SPEED DISPLAY OPACITY",
    "TIMING OPACITY",
];

/// The parts of the shipped pack's seven-key play skin are taken off its preview with the keys of
/// the menu: each of [`SHIPPED_BARE_ROWS`] opens at a hundred percent and is at none after ten
/// presses of the left arrow with Shift held, the preview is drawn on the frame after each with
/// the document never read again, and what is stored for each is the offset that takes all of an
/// alpha away. The zero key puts each back.
#[test]
fn the_parts_of_the_shipped_play_skin_are_taken_off_its_preview_with_the_keys_of_the_menu() {
    let mut menu = Menu::over(app_with_pack("opacity-shipped", &shipped_pack()));
    menu.choose(SKIN_TYPE_PLAY_7KEYS);
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS));
    let build = menu.app.shared.skins.build_of(SKIN_TYPE_PLAY_7KEYS);
    for label in SHIPPED_BARE_ROWS {
        let row = menu.row_called(label);
        assert_eq!(menu.value_of(row), "100%", "{label} does not open at a hundred percent");
        assert_eq!(row_help(&menu.app.shared, row), SkinMenuText::OpacityHelp.get(language()), "{label}");
        menu.focus(row);
        for _ in 0..10 {
            menu.shifted(KeyCode::ArrowLeft);
        }
        assert_eq!(menu.value_of(row), "0%", "{label} was not taken to nothing by ten wide steps");
        menu.frame();
        assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "taking {label} to nothing cost a frame of the preview");
    }
    assert_eq!(menu.app.shared.skins.build_of(SKIN_TYPE_PLAY_7KEYS), build, "an opacity had the document read again");
    let config = &menu.app.shared.config;
    let document = menu.app.shared.skins.document_path(config, SKIN_TYPE_PLAY_7KEYS).expect("the pack has a seven-key play skin").to_owned();
    let stored = config.skin.customisation(&document).expect("the rows that were stepped are stored").offsets.clone();
    assert_eq!(stored.len(), SHIPPED_BARE_ROWS.len(), "something else than the rows that were stepped is stored: {stored:?}");
    assert!(stored.values().all(|offset| offset.a == opacity_alpha(0)), "a part is not stored as the offset that takes all of its alpha away: {stored:?}");
    menu.capture_at("skin-menu-shipped-bare", CAPTURE_SECONDS);

    for label in SHIPPED_BARE_ROWS {
        let row = menu.row_called(label);
        menu.focus(row);
        menu.key(KeyCode::Digit0);
        assert_eq!(menu.value_of(row), "100%", "{label} was not put back by the zero key");
    }
}

mod numbers;
