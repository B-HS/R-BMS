//! The colour rows of a document: how they are listed and read, what stepping, typing and resetting
//! one stores, when the document has to be read again for it, and how a colour is put on the other
//! screens.

use rbms_config::Config;
use rbms_skin::dst::SkinColor;
use rbms_skin::loader::{SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS};

use super::color_fixtures::{NOTE_COLOR, NOTE_COLOR_ROW, PANEL_COLOR, PANEL_COLOR_ROW, PANEL_DEFAULT, write_tinted_play};
use super::*;

/// The panel's row and the notes' row, in the order the document declares them.
const PANEL_ROW: SkinRow = SkinRow::Color(0);
const NOTE_ROW: SkinRow = SkinRow::Color(1);

/// A colour a player could type, at a little under half of its alpha.
const TYPED: SkinColor = SkinColor::rgba(0x12, 0x34, 0x56, 0x78);

/// A library over a skin folder holding the tinted play documents, with the seven-key one chosen
/// for its screen and the five-key one for its own, read once so nothing is waiting to be read.
struct Tinted {
    _root: PathBuf,
    config: Config,
    skins: SkinLibrary,
}

impl Tinted {
    fn new(tag: &str) -> Tinted {
        let root = std::env::temp_dir().join(format!("rbms-skin-colors-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture folder is writable");
        let settings = root.join("settings.ron");
        let mut config = Config::default();
        let (seven, five) = write_tinted_play(&skin_root(&settings, &config));
        config.skin.screen = SKIN_TYPE_PLAY_7KEYS;
        config.skin.select(SKIN_TYPE_PLAY_7KEYS, Some(seven.to_string_lossy().into_owned()));
        config.skin.select(SKIN_TYPE_PLAY_5KEYS, Some(five.to_string_lossy().into_owned()));
        let mut skins = SkinLibrary::new(&settings, &config);
        skins.rescan(&settings, &config);
        let mut tinted = Tinted { _root: root, config, skins };
        tinted.read(SKIN_TYPE_PLAY_7KEYS);
        tinted.read(SKIN_TYPE_PLAY_5KEYS);
        tinted
    }

    /// Read one screen's document now, as its screen would.
    fn read(&mut self, screen: i32) {
        self.skins.reload_for(&self.config, screen, SkinRead::unhosted(None));
        assert!(!self.skins.needs_reload_for(&self.config, screen), "{:?}", self.skins.failure(screen));
    }

    fn line(&self, row: SkinRow) -> (String, String) {
        self.skins.line(&self.config, row)
    }

    /// The colour stored for one id of one screen's document.
    fn stored(&self, screen: i32, id: i32) -> Option<SkinColor> {
        let path = self.skins.document_path(&self.config, screen)?;
        self.config.skin.customisation(path)?.colors.get(&id).copied()
    }

    /// Whether the seven-key document has to be read again, and then read it.
    fn reread(&mut self) -> bool {
        let stale = self.skins.needs_reload_for(&self.config, SKIN_TYPE_PLAY_7KEYS);
        if stale {
            self.read(SKIN_TYPE_PLAY_7KEYS);
        }
        stale
    }
}

/// A document's colours are listed after its options and its file slots, each one row, and then
/// the red, green, blue and alpha of each, colour by colour, before every offset. A colour nobody
/// set reads DEFAULT, and its channels the colour the document declares as its default.
#[test]
fn the_colours_of_a_document_are_listed_one_row_each_and_then_their_channels() {
    let tinted = Tinted::new("rows");
    let rows = tinted.skins.rows(&tinted.config);
    let first = rows.iter().position(|row| *row == PANEL_ROW).expect("the panel's colour is listed");
    let channels = |at: usize| ColorChannel::ALL.map(|channel| SkinRow::ColorChannel(at, channel));
    let expected: Vec<SkinRow> = [PANEL_ROW, NOTE_ROW].into_iter().chain(channels(0)).chain(channels(1)).collect();
    assert_eq!(&rows[first..first + expected.len()], expected.as_slice());
    assert!(rows[first + expected.len()..].iter().all(|row| matches!(row, SkinRow::Offset(_, _))), "a row other than an offset follows the colours");
    assert!(rows[..first].iter().all(|row| matches!(row, SkinRow::Property(_) | SkinRow::File(_))));

    assert_eq!(tinted.skins.kind(&tinted.config, PANEL_ROW), Some(SkinRowKind::Color));
    assert_eq!(tinted.skins.kind(&tinted.config, SkinRow::ColorChannel(1, ColorChannel::A)), Some(SkinRowKind::ColorChannel(ColorChannel::A)));
    assert_eq!(tinted.skins.kind(&tinted.config, SkinRow::Color(2)), None, "a colour the document does not declare has a kind");
    assert_eq!(tinted.line(PANEL_ROW), (PANEL_COLOR_ROW.to_string(), DEFAULT_VALUE.to_string()));
    assert_eq!(tinted.line(NOTE_ROW), (NOTE_COLOR_ROW.to_string(), DEFAULT_VALUE.to_string()));
    assert_eq!(tinted.line(SkinRow::ColorChannel(0, ColorChannel::G)), (format!("{PANEL_COLOR_ROW} G"), PANEL_DEFAULT.g.to_string()));
    assert_eq!(tinted.line(SkinRow::ColorChannel(1, ColorChannel::R)).1, "255", "a colour with no default is not white");
    assert_eq!(tinted.skins.color_for(&tinted.config, SKIN_TYPE_PLAY_7KEYS, 0), Some((PANEL_DEFAULT, false)));
}

/// The arrows go round a list of colours, from the first one for a colour that is on none of it,
/// keeping the alpha the row stands at. The first step sets the colour, which has the document
/// read again once; the steps after it change a colour that is set, which is read live.
#[test]
fn the_arrows_go_round_the_preset_colours_and_keep_the_alpha() {
    let mut tinted = Tinted::new("presets");
    let screen = SKIN_TYPE_PLAY_7KEYS;
    assert!(tinted.skins.step(&mut tinted.config, PANEL_ROW, 1));
    let (r, g, b) = COLOR_PRESETS[0];
    assert_eq!(tinted.stored(screen, PANEL_COLOR), Some(SkinColor::rgba(r, g, b, PANEL_DEFAULT.a)));
    assert_eq!(tinted.line(PANEL_ROW).1, format!("{HEX_MARK}{:02X}{:02X}{:02X}FF", r, g, b));
    assert!(tinted.reread(), "a colour that came to be set did not have the document read again");

    assert!(tinted.skins.step(&mut tinted.config, PANEL_ROW, 1));
    let (r, g, b) = COLOR_PRESETS[1];
    assert_eq!(tinted.stored(screen, PANEL_COLOR), Some(SkinColor::rgba(r, g, b, u8::MAX)));
    assert!(!tinted.reread(), "a colour that was set had the document read again for a change");
    assert!(tinted.skins.step(&mut tinted.config, PANEL_ROW, -1));
    assert!(tinted.skins.step(&mut tinted.config, PANEL_ROW, -1));
    let (r, g, b) = COLOR_PRESETS[COLOR_PRESETS.len() - 1];
    assert_eq!(tinted.stored(screen, PANEL_COLOR), Some(SkinColor::rgba(r, g, b, u8::MAX)), "the list does not go round its start");

    assert!(tinted.skins.set_color_for(&mut tinted.config, screen, 0, TYPED));
    assert!(tinted.skins.step(&mut tinted.config, PANEL_ROW, -1), "a colour off the list is not stepped onto it");
    let (r, g, b) = COLOR_PRESETS[COLOR_PRESETS.len() - 1];
    assert_eq!(tinted.stored(screen, PANEL_COLOR), Some(SkinColor::rgba(r, g, b, TYPED.a)), "a step lost the alpha the colour had");
    assert!(!tinted.reread());
    assert_eq!(tinted.stored(screen, NOTE_COLOR), None, "stepping one colour set another");
}

/// A typed colour is stored as typed. Typing the colour that is already set moves nothing, and the
/// document is read again only when the colour comes to be set and when it is given back to the
/// skin -- once each -- and never for a colour that changes while it is set.
#[test]
fn the_document_is_read_again_only_when_a_colour_comes_to_be_set_or_is_given_back() {
    let mut tinted = Tinted::new("reread");
    let screen = SKIN_TYPE_PLAY_7KEYS;
    let build = tinted.skins.build_of(screen);
    assert!(!tinted.skins.reset_color_for(&mut tinted.config, screen, 0), "a colour nobody set was given back");
    assert!(!tinted.reread());

    assert!(tinted.skins.set_color_for(&mut tinted.config, screen, 0, TYPED));
    assert_eq!((tinted.stored(screen, PANEL_COLOR), tinted.line(PANEL_ROW).1), (Some(TYPED), "#12345678".to_string()));
    assert!(tinted.reread(), "setting a colour did not have the document read again");
    assert_ne!(tinted.skins.build_of(screen), build);
    let set = tinted.skins.build_of(screen);

    assert!(!tinted.skins.set_color_for(&mut tinted.config, screen, 0, TYPED), "the colour that was set moved");
    for color in [SkinColor::rgba(1, 2, 3, 4), SkinColor::WHITE, PANEL_DEFAULT] {
        assert!(tinted.skins.set_color_for(&mut tinted.config, screen, 0, color));
        assert!(!tinted.reread(), "changing a colour that is set had the document read again");
    }
    assert_eq!(tinted.skins.build_of(screen), set);
    assert_eq!(tinted.skins.color_for(&tinted.config, screen, 0), Some((PANEL_DEFAULT, true)), "a colour set to the default is not set");

    assert!(tinted.skins.reset_color_for(&mut tinted.config, screen, 0));
    assert_eq!((tinted.stored(screen, PANEL_COLOR), tinted.line(PANEL_ROW).1), (None, DEFAULT_VALUE.to_string()));
    assert!(tinted.reread(), "giving a colour back did not have the document read again");
    assert!(!tinted.reread(), "the document was read again twice");
    let path = tinted.skins.document_path(&tinted.config, screen).expect("the screen has a document").to_owned();
    assert_eq!(tinted.config.skin.customisation(&path), None, "a document given every colour back keeps an empty entry");
}

/// A channel row moves its one channel of the colour the row shows, from the document's default
/// when nobody set one, stops at 0 and at 255, and ships where the default is.
#[test]
fn a_channel_row_moves_one_channel_of_the_colour() {
    let mut tinted = Tinted::new("channels");
    let screen = SKIN_TYPE_PLAY_7KEYS;
    let red = SkinRow::ColorChannel(0, ColorChannel::R);
    let alpha = SkinRow::ColorChannel(0, ColorChannel::A);
    let value = tinted.skins.number(&tinted.config, red).expect("a channel holds a number");
    assert_eq!((value.steps, value.shipped, value.spec.min, value.spec.max), (i64::from(PANEL_DEFAULT.r), i64::from(PANEL_DEFAULT.r), 0, 255));

    assert!(tinted.skins.step(&mut tinted.config, red, 1));
    assert_eq!(tinted.stored(screen, PANEL_COLOR), Some(SkinColor { r: PANEL_DEFAULT.r + 1, ..PANEL_DEFAULT }));
    assert!(tinted.reread(), "the first channel moved did not have the document read again");
    assert!(tinted.skins.set_number(&mut tinted.config, alpha, 0));
    assert!(!tinted.reread(), "a channel of a colour that is set had the document read again");
    assert_eq!(tinted.stored(screen, PANEL_COLOR), Some(SkinColor { r: PANEL_DEFAULT.r + 1, a: 0, ..PANEL_DEFAULT }));
    assert!(!tinted.skins.set_number(&mut tinted.config, alpha, -40), "a channel went below nothing");
    assert!(tinted.skins.set_number(&mut tinted.config, red, 900));
    assert_eq!(tinted.line(red).1, "255");
    assert_eq!(tinted.skins.number(&tinted.config, red).map(|value| value.shipped), Some(i64::from(PANEL_DEFAULT.r)));
}

/// A colour is put on the other screens that declare one of the same name, from its one row or
/// any of its channels, and a colour left to the skin is left to theirs.
#[test]
fn a_colour_is_put_on_the_other_screens_that_declare_it() {
    let mut tinted = Tinted::new("peers");
    let seven = SKIN_TYPE_PLAY_7KEYS;
    assert_eq!(tinted.skins.peers_for(&tinted.config, seven, PANEL_ROW), vec![SKIN_TYPE_PLAY_5KEYS]);
    assert_eq!(tinted.skins.peers_for(&tinted.config, seven, SkinRow::ColorChannel(1, ColorChannel::B)), vec![SKIN_TYPE_PLAY_5KEYS]);

    assert!(tinted.skins.set_color_for(&mut tinted.config, seven, 0, TYPED));
    let applied = tinted.skins.apply_to_peers(&mut tinted.config, seven, SkinRow::ColorChannel(0, ColorChannel::G)).expect("the five-key screen declares it");
    assert_eq!((applied.applied, applied.skipped), (1, 0));
    assert_eq!(tinted.stored(SKIN_TYPE_PLAY_5KEYS, PANEL_COLOR), Some(TYPED));
    assert_eq!(tinted.stored(SKIN_TYPE_PLAY_5KEYS, NOTE_COLOR), None, "another colour went with it");

    assert!(tinted.skins.reset_color_for(&mut tinted.config, seven, 0));
    assert!(tinted.skins.apply_to_peers(&mut tinted.config, seven, PANEL_ROW).is_some());
    assert_eq!(tinted.stored(SKIN_TYPE_PLAY_5KEYS, PANEL_COLOR), None, "a colour given back to the skin here was kept there");
}
