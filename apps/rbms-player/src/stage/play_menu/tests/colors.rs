//! The colours of the previewed play skin on the play menu: that they are kept in a fold of their
//! own and their channels in another, that a colour is stepped, typed and given back here as the
//! skin menu does it and stored in the same place, and that the preview is drawn in a colour on the
//! frame after its row moved.
//!
//! The menu is opened over the tinted play document of the library's colour fixtures, a seven-key
//! play skin of white parts, so a part tinted by a colour comes out as exactly that colour.

use rbms_render::Color;
use rbms_skin::dst::SkinColor;

use super::*;
use crate::skin_select::color_fixtures::{NOTE_COLOR_ROW, ON_THE_PANEL, PANEL_COLOR, PANEL_COLOR_ROW, TINTED_AUTHORED, write_tinted_play};
use crate::skin_select::{SkinRowKind, skin_root};
use crate::stage::skin_menu::skin_row_help;

/// How many channel rows the tinted document's two colours come to.
const CHANNEL_ROWS: usize = 8;

/// An app whose skin folder holds the tinted play documents, the seven-key one chosen for its
/// screen. The skin menu is left configuring the song browser, so a row read through the screen it
/// configures is not read by accident.
fn app_with_tinted_play(tag: &str) -> App {
    let mut app = bare_app(tag);
    let folder = skin_root(&app.shared.settings_path, &app.shared.config);
    let (seven, _) = write_tinted_play(&folder);
    app.shared.config.skin.screen = SKIN_TYPE_MUSIC_SELECT;
    app.shared.config.skin.select(PREVIEW_MODES[0].0, Some(seven.to_string_lossy().into_owned()));
    app
}

impl Menu {
    /// The row of the list whose label is exactly `label`.
    fn listed_as(&self, label: &str) -> Row {
        let found = self.state.rows.iter().copied().find(|row| self.line(*row).label == label);
        found.unwrap_or_else(|| panic!("no row is called {label} among {:?}", self.state.rows.iter().map(|row| self.line(*row).label).collect::<Vec<_>>()))
    }

    /// Put the cursor on a fold and step it once, which shows its rows or hides them.
    fn open_fold(&mut self, fold: Fold) {
        self.focus(Row::Fold(fold));
        self.key(KeyCode::Enter);
    }

    /// The colour stored under `id` for the document of the previewed mode.
    fn stored_color(&self, id: i32) -> Option<SkinColor> {
        let config = &self.app.shared.config;
        let path = self.app.shared.skins.document_path(config, self.state.target().screen)?;
        config.skin.customisation(path)?.colors.get(&id).copied()
    }

    /// Type `text` into the row being typed into, a character at a time.
    fn type_text(&mut self, text: &'static str) {
        for at in 0..text.len() {
            self.typed(&text[at..at + 1]);
        }
    }

    /// The pixel of the frame drawn last that shows a point of the previewed document, given in the
    /// document's own pixels and measured up from its foot.
    fn previewed_at(&self, at: (f32, f32)) -> Color {
        let across = PREVIEW_RECT.x + at.0 * PREVIEW_RECT.w / TINTED_AUTHORED.0;
        let down = PREVIEW_RECT.y + (TINTED_AUTHORED.1 - at.1) * PREVIEW_RECT.h / TINTED_AUTHORED.1;
        self.pixels.pixel_at(across as u32, down as u32)
    }
}

/// The colours of the play skin are one closed fold at the head of the skin's group and their
/// channels another under it, each saying how many rows it holds. Opening the colour fold lists a
/// row for each colour, said as the skin menu says one, and opening the other a number row for each
/// channel.
#[test]
fn the_colours_of_the_play_skin_are_a_fold_and_their_channels_another() {
    let mut menu = Menu::over(app_with_tinted_play("color-folds"));
    let skin_group = menu.at(Row::Group(group_skin()));
    assert_eq!(&menu.state.rows[skin_group + 1..skin_group + 3], [Row::Fold(Fold::Color), Row::Fold(Fold::ColorChannels)]);
    let folds = menu.line(Row::Fold(Fold::Color));
    assert_eq!((folds.label.as_str(), folds.value.as_str()), ("COLOR", "HIDDEN (2)"));
    assert_eq!(menu.line(Row::Fold(Fold::ColorChannels)).value, format!("HIDDEN ({CHANNEL_ROWS})"));
    assert!(!menu.state.rows.iter().any(|row| matches!(row, Row::Skin(SkinRow::Color(_) | SkinRow::ColorChannel(_, _)))), "a colour is on show");

    menu.open_fold(Fold::Color);
    let panel = menu.listed_as(PANEL_COLOR_ROW);
    assert_eq!(menu.at(panel), menu.at(Row::Fold(Fold::Color)) + 1);
    assert_eq!((menu.line(panel).value.as_str(), menu.line(menu.listed_as(NOTE_COLOR_ROW)).value.as_str()), ("DEFAULT", "DEFAULT"));
    assert_eq!(menu.state.help_of(&menu.app.shared, panel), skin_row_help(SkinRowKind::Color));

    menu.open_fold(Fold::ColorChannels);
    let red = menu.listed_as(&format!("{PANEL_COLOR_ROW} R"));
    assert_eq!(menu.at(red), menu.at(Row::Fold(Fold::ColorChannels)) + 1);
    assert_eq!(menu.line(Row::Fold(Fold::ColorChannels)).value, format!("SHOWN ({CHANNEL_ROWS})"));
    assert_eq!(menu.steps_of(red), 0x20, "a channel does not stand on the skin's default");
    menu.focus(red);
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.stored_color(PANEL_COLOR).map(|color| color.r), Some(0x21), "a channel row did not move its channel");
    assert_eq!(menu.line(panel).value, "#213040FF");
    menu.key(KeyCode::Digit0);
    assert_eq!(menu.steps_of(red), 0x20, "the zero key did not put the channel back where the colour ships");
}

/// The arrows, Enter and the zero key do on a colour row what they do on the skin menu's, and the
/// colour is stored where the skin menu stores it. The preview is drawn in the colour on the frame
/// after a colour that is set moved, with the document read again only as the colour came to be set
/// and as it was given back.
#[test]
fn a_colour_of_the_play_skin_is_set_here_and_drawn_on_the_frame_after() {
    let mut menu = Menu::over(app_with_tinted_play("color-preview"));
    let screen = PREVIEW_MODES[0].0;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let white = Color::rgb(u8::MAX, u8::MAX, u8::MAX);
    assert_eq!(menu.previewed_at(ON_THE_PANEL), white, "the panel is not where the skin put it, in its own colour");
    menu.open_fold(Fold::Color);
    let row = menu.listed_as(PANEL_COLOR_ROW);
    menu.focus(row);
    let read = menu.app.shared.skins.build_of(screen);

    menu.key(KeyCode::Enter);
    menu.type_text("#204080");
    menu.key(KeyCode::Enter);
    assert_eq!((menu.line(row).value.as_str(), menu.stored_color(PANEL_COLOR)), ("#204080FF", Some(SkinColor::rgba(0x20, 0x40, 0x80, u8::MAX))));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    assert_ne!(menu.app.shared.skins.build_of(screen), read, "a colour that came to be set did not have the document read again");
    assert_eq!(menu.previewed_at(ON_THE_PANEL), Color::rgb(0x20, 0x40, 0x80), "the panel is not in the colour set for it");
    let set = menu.app.shared.skins.build_of(screen);

    menu.key(KeyCode::ArrowRight);
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a change to a colour that is set cost a frame of the preview");
    assert_eq!(menu.previewed_at(ON_THE_PANEL), white, "the panel is not in the next colour on the frame after its row moved");
    assert_eq!(menu.app.shared.skins.build_of(screen), set, "a change to a colour that is set had the document read again");

    let config = &menu.app.shared.config;
    let as_the_skin_menu_reads_it = Config { skin: rbms_config::SkinOptions { screen, ..config.skin.clone() }, ..config.clone() };
    assert_eq!(menu.app.shared.skins.line(&as_the_skin_menu_reads_it, SkinRow::Color(0)).1, "#FFFFFFFF", "the skin menu reads the colour from elsewhere");

    menu.key(KeyCode::Digit0);
    assert_eq!((menu.line(row).value.as_str(), menu.stored_color(PANEL_COLOR)), ("DEFAULT", None));
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    assert_ne!(menu.app.shared.skins.build_of(screen), set, "giving the colour back did not have the document read again");
    assert_eq!(menu.previewed_at(ON_THE_PANEL), white, "the panel did not come back in its own colour");
}
