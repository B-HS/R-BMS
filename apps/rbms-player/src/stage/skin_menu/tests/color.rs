//! The colour rows of the skin menu: how a skin's colours and the row that shows their channels are
//! listed, how a colour is stepped, typed and given back with the keys of the menu, and that the
//! preview is drawn in a colour on the frame after its row moved.
//!
//! The menu is opened over the tinted play document of the library's colour fixtures, a seven-key
//! play skin of white parts, so a part tinted by a colour comes out as exactly that colour.

use rbms_skin::dst::SkinColor;
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

use super::*;
use crate::number_row::{EDIT_HINT_HEX, REFUSAL_SHOWN_FOR};
use crate::skin_select::color_fixtures::{NOTE_COLOR_ROW, ON_THE_PANEL, PANEL_COLOR, PANEL_COLOR_ROW, TINTED_AUTHORED, write_tinted_play};
use crate::skin_select::{color_channels_label, skin_root};

/// How many channel rows the tinted document's two colours come to.
const CHANNEL_ROWS: usize = 8;

/// An app whose skin folder holds the tinted play documents, with the seven-key one chosen for its
/// screen, which the menu is configuring.
fn app_with_tinted_play(tag: &str) -> App {
    let mut app = bare_app(tag);
    let folder = skin_root(&app.shared.settings_path, &app.shared.config);
    let (seven, _) = write_tinted_play(&folder);
    app.shared.config.skin.screen = SKIN_TYPE_PLAY_7KEYS;
    app.shared.config.skin.select(SKIN_TYPE_PLAY_7KEYS, Some(seven.to_string_lossy().into_owned()));
    app
}

impl Menu {
    /// The row of the list whose label is exactly `label`.
    fn listed_as(&self, label: &str) -> SettingRow {
        let at = self.state.lines.iter().position(|line| line.label == label).unwrap_or_else(|| panic!("no row is called {label} among {:?}", self.labels()));
        self.state.rows[at]
    }

    /// The colour stored for one id of the document being configured.
    fn stored_color(&self, id: i32) -> Option<SkinColor> {
        let config = &self.app.shared.config;
        let path = self.app.shared.skins.document_path(config, config.skin.screen)?;
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
        pixel(&self.pixels, (across, down))
    }
}

/// A colour as the canvas shows it, opaque.
fn opaque(color: SkinColor) -> Color {
    Color::rgb(color.r, color.g, color.b)
}

/// A skin's colours are rows under the names it gave them, each reading DEFAULT while nobody set
/// it, and their channels are behind one row that says how many there are. Enter shows them under
/// it and hides them again, with the cursor kept on it; what each kind of row is for is said.
#[test]
fn the_colours_of_a_skin_are_listed_with_a_row_that_shows_their_channels() {
    let mut menu = Menu::over(app_with_tinted_play("color-list"));
    let labels = menu.labels();
    let first = labels.iter().position(|label| label == PANEL_COLOR_ROW).expect("the panel's colour is listed");
    assert_eq!(&labels[first..first + 3], [PANEL_COLOR_ROW, NOTE_COLOR_ROW, color_channels_label()]);
    assert_eq!((menu.value_of(menu.listed_as(PANEL_COLOR_ROW)).as_str(), menu.value_of(menu.listed_as(NOTE_COLOR_ROW)).as_str()), ("DEFAULT", "DEFAULT"));
    let fold = SettingRow::Skin(SkinRow::ColorChannels);
    assert_eq!(menu.value_of(fold), format!("HIDDEN ({CHANNEL_ROWS})"));
    assert!(!labels.iter().any(|label| label.ends_with(" R")), "a channel is on show with its row closed");
    assert_eq!(row_help(&menu.app.shared, menu.listed_as(PANEL_COLOR_ROW)), SkinMenuText::ColorHelp.get(language()));
    assert_eq!(row_help(&menu.app.shared, fold), SkinMenuText::ChannelFoldHelp.get(language()));
    for help in [SkinMenuText::ColorHelp.get(language()), SkinMenuText::ChannelHelp.get(language()), SkinMenuText::ChannelFoldHelp.get(language())] {
        assert_eq!(fit_text(help, INFO_SCALE, INFO_RECT.w - INFO_PAD * 2.0), help, "what is said of a colour row is cut short");
    }

    menu.focus(fold);
    menu.key(KeyCode::Enter);
    assert_eq!(menu.value_of(fold), format!("SHOWN ({CHANNEL_ROWS})"));
    assert_eq!(menu.state.focused(), Some(fold), "the cursor left the row it showed the channels with");
    let under = menu.at(fold) + 1;
    let shown: Vec<String> = menu.state.lines[under..under + CHANNEL_ROWS].iter().map(|line| line.label.clone()).collect();
    let expected: Vec<String> =
        [PANEL_COLOR_ROW, NOTE_COLOR_ROW].iter().flat_map(|name| ["R", "G", "B", "A"].map(|channel| format!("{name} {channel}"))).collect();
    assert_eq!(shown, expected);
    assert_eq!(menu.value_of(menu.listed_as(&format!("{PANEL_COLOR_ROW} B"))), "64", "a channel does not read the skin's default");
    assert_eq!(row_help(&menu.app.shared, menu.listed_as(&format!("{PANEL_COLOR_ROW} B"))), SkinMenuText::ChannelHelp.get(language()));
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of(fold), format!("HIDDEN ({CHANNEL_ROWS})"), "an arrow did not hide the channels");
    assert!(menu.app.shared.config.skin.custom.is_empty(), "showing the channels stored something");
}

/// The arrows go round the preset colours, Enter opens the row to have a colour typed in hex, what
/// is no colour is refused and says so, and the zero key gives the colour back to the skin. The
/// document is read again when the colour comes to be set and when it is given back, and not when
/// a colour that is set changes.
#[test]
fn a_colour_row_is_stepped_typed_and_given_back_with_the_keys_of_the_menu() {
    let mut menu = Menu::over(app_with_tinted_play("color-keys"));
    let screen = SKIN_TYPE_PLAY_7KEYS;
    let row = menu.listed_as(PANEL_COLOR_ROW);
    menu.focus(row);
    menu.frame();
    let read = menu.app.shared.skins.build_of(screen);

    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(row), "#FFFFFFFF");
    let set = menu.app.shared.skins.build_of(screen);
    assert_ne!(set, read, "a colour that came to be set did not have the document read again");
    menu.key(KeyCode::ArrowRight);
    assert_eq!(menu.value_of(row), "#C0C0C0FF");
    menu.key(KeyCode::ArrowLeft);
    assert_eq!(menu.value_of(row), "#FFFFFFFF");

    menu.key(KeyCode::Enter);
    assert_eq!(menu.state.numbers.edit_hint(), Some(EDIT_HINT_HEX));
    assert_eq!(menu.state.numbers.edit_of(row).map(|edit| edit.text().to_owned()), Some("#FFFFFFFF".to_owned()), "the row does not open on its colour");
    menu.frame();
    menu.type_text("#ff4080");
    menu.key(KeyCode::Enter);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_color(PANEL_COLOR)), ("#FF4080FF", Some(SkinColor::rgba(0xFF, 0x40, 0x80, u8::MAX))));
    assert_eq!(menu.app.shared.skins.build_of(screen), set, "a colour that was set had the document read again");

    let now = std::time::Instant::now();
    menu.key(KeyCode::Enter);
    menu.type_text("12z");
    menu.key_at(KeyCode::Enter, now);
    assert_eq!(menu.value_of(row), "#FF4080FF", "what is no colour was stored");
    assert!(menu.state.numbers.is_refused(row, now) && !menu.state.numbers.is_refused(row, now + REFUSAL_SHOWN_FOR));
    menu.frame_at(now);

    menu.key(KeyCode::Digit0);
    assert_eq!((menu.value_of(row).as_str(), menu.stored_color(PANEL_COLOR)), ("DEFAULT", None), "the zero key did not give the colour back");
    assert_ne!(menu.app.shared.skins.build_of(screen), set, "giving a colour back did not have the document read again");
    assert!(menu.app.shared.config.skin.custom.is_empty(), "a colour given back left something stored");
}

/// The preview is drawn in a colour on the frame after its row moved: once the colour is set, a
/// change to it is on the next frame with the document neither read nor drawn dark again, and the
/// part is in its own colour again once the colour is given back.
#[test]
fn the_preview_is_drawn_in_a_colour_on_the_frame_after_its_row_moved() {
    let mut menu = Menu::over(app_with_tinted_play("color-preview"));
    let screen = SKIN_TYPE_PLAY_7KEYS;
    assert_eq!(menu.until_said(), PreviewStatus::Drawn, "{:?}", menu.app.shared.skin_failure(screen));
    let white = Color::rgb(u8::MAX, u8::MAX, u8::MAX);
    assert_eq!(menu.previewed_at(ON_THE_PANEL), white, "the panel is not where the skin put it, in its own colour");

    let row = menu.listed_as(PANEL_COLOR_ROW);
    menu.focus(row);
    menu.key(KeyCode::Enter);
    menu.type_text("204080");
    menu.key(KeyCode::Enter);
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    assert_eq!(menu.previewed_at(ON_THE_PANEL), opaque(SkinColor::rgba(0x20, 0x40, 0x80, u8::MAX)), "the panel is not in the colour set for it");
    let build = menu.app.shared.skins.build_of(screen);

    menu.key(KeyCode::ArrowRight);
    menu.frame();
    assert_eq!(*menu.state.preview.status(), PreviewStatus::Drawn, "a change to a colour that is set cost a frame of the preview");
    assert_eq!(menu.previewed_at(ON_THE_PANEL), white, "the panel is not in the next colour on the frame after its row moved");
    menu.key(KeyCode::ArrowRight);
    menu.frame();
    assert_eq!(menu.previewed_at(ON_THE_PANEL), Color::rgb(0xC0, 0xC0, 0xC0));
    assert_eq!(menu.app.shared.skins.build_of(screen), build, "a change to a colour that is set had the document read again");

    menu.key(KeyCode::Digit0);
    assert_eq!(menu.until_said(), PreviewStatus::Drawn);
    assert_eq!(menu.previewed_at(ON_THE_PANEL), white, "the panel did not come back in its own colour");
    menu.save("skin-menu-color-default");
}
