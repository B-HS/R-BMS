//! What the SKIN tab reads out of a skin folder: which documents are in it, which of them draws
//! each screen, and what stepping one of a document's own customisation rows stores.

use super::fixtures::*;
use super::*;
use rbms_config::{Config, SKIN_SCREEN_LABELS};

struct Fixture {
    _root: PathBuf,
    settings: PathBuf,
    config: Config,
    skins: SkinLibrary,
}

impl Fixture {
    /// A settings file with a skin folder beside it holding the fixture documents, already
    /// scanned, with the SKIN tab on the browser screen.
    fn new(tag: &str) -> Fixture {
        let root = std::env::temp_dir().join(format!("rbms-skin-select-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the fixture folder is writable");
        let settings = root.join("settings.ron");
        fixtures::write(&settings);
        let mut config = Config::default();
        config.skin.screen = MUSIC_SELECT;
        let mut skins = SkinLibrary::new(&settings, &config);
        skins.rescan(&settings, &config);
        Fixture { _root: root, settings, config, skins }
    }

    fn rows(&self) -> Vec<SkinRow> {
        self.skins.rows(&self.config)
    }

    fn line(&self, row: SkinRow) -> (String, String) {
        self.skins.line(&self.config, row)
    }

    /// Put the SKIN row on the only document that draws the open screen.
    fn choose(&mut self) {
        assert!(self.skins.cycle_document(&mut self.config, 1), "there was no document to choose");
    }
}

/// The folder is walked for documents, not for everything: a file that is not a skin is passed
/// over, and a document is listed under the screen it declares rather than the folder it is in.
#[test]
fn the_folder_is_walked_once_and_lists_a_document_under_the_screen_it_declares() {
    let mut fixture = Fixture::new("scan");
    assert_eq!(fixture.skins.documents.len(), 2, "the decoy file was read as a skin, or a document was missed");

    assert_eq!(fixture.skins.candidates(&fixture.config).len(), 1, "the browser screen offers a document that does not draw it");
    fixture.config.skin.screen = PLAY_7KEYS;
    assert_eq!(fixture.skins.candidates(&fixture.config).len(), 1);
    fixture.config.skin.screen = COURSE_RESULT;
    assert!(fixture.skins.candidates(&fixture.config).is_empty(), "a screen with no document still offered one");
}

/// The SKIN row walks the built-in screen and every document that draws this screen, in both
/// directions, and comes back to where it started.
#[test]
fn the_document_row_cycles_through_the_built_in_screen_and_back() {
    let mut fixture = Fixture::new("cycle");
    assert_eq!(fixture.skins.document_value(&fixture.config), DEFAULT_VALUE);

    fixture.choose();
    assert_eq!(fixture.skins.document_value(&fixture.config), "Browser", "the row shows the document's own name");
    assert!(fixture.skins.needs_reload(&fixture.config), "choosing a document did not ask for it to be read");

    assert!(fixture.skins.cycle_document(&mut fixture.config, 1));
    assert_eq!(fixture.skins.document_value(&fixture.config), DEFAULT_VALUE, "one document plus the built-in screen is a two-entry cycle");

    assert!(fixture.skins.cycle_document(&mut fixture.config, -1));
    assert_eq!(fixture.skins.document_value(&fixture.config), "Browser", "the row does not step backwards");
}

/// A document with no customisation of its own gets no rows, and the built-in screen gets none
/// either — there is nothing to configure until a document is chosen.
#[test]
fn only_a_chosen_document_contributes_rows() {
    let mut fixture = Fixture::new("rows-empty");
    assert!(fixture.rows().is_empty(), "the built-in screen offered customisation rows");

    fixture.config.skin.screen = PLAY_7KEYS;
    fixture.choose();
    assert!(fixture.rows().is_empty(), "a document that declares no customisation still produced rows");
}

/// One row per declared property, one per file slot, and one per axis the document allows —
/// which is two of the offset's six, not all six.
#[test]
fn the_rows_are_the_ones_the_document_declares() {
    let mut fixture = Fixture::new("rows");
    fixture.choose();
    assert_eq!(fixture.rows(), vec![SkinRow::Property(0), SkinRow::File(0), SkinRow::Offset(0, OffsetAxis::X), SkinRow::Offset(0, OffsetAxis::Y)]);

    assert_eq!(fixture.line(SkinRow::Property(0)), ("LANE > COVER".to_string(), "OFF".to_string()), "the author's own default is not shown");
    assert_eq!(fixture.line(SkinRow::File(0)), ("LANE > BACKGROUND".to_string(), "day.png".to_string()), "a file slot is labelled by its category too");
    assert_eq!(fixture.line(SkinRow::Offset(0, OffsetAxis::X)), ("LANE > JUDGE X".to_string(), "+0".to_string()));
}

/// Stepping a property row walks the document's own items and stores the option id each one
/// turns on, which is what the loader is later handed.
#[test]
fn a_property_row_walks_the_documents_items_and_is_stored_by_name() {
    let mut fixture = Fixture::new("property");
    fixture.choose();
    let path = fixture.config.skin.document(MUSIC_SELECT).expect("a document is chosen").to_string();

    assert!(fixture.skins.step(&mut fixture.config, SkinRow::Property(0), 1));
    assert_eq!(fixture.line(SkinRow::Property(0)).1, "ON");
    assert_eq!(fixture.config.skin.user_config(&path).properties.get("Cover"), Some(&901));

    assert!(fixture.skins.step(&mut fixture.config, SkinRow::Property(0), 1));
    assert_eq!(fixture.line(SkinRow::Property(0)).1, "OFF", "the two-item list did not wrap");
    assert_eq!(fixture.config.skin.user_config(&path).properties.get("Cover"), Some(&902));
}

/// A file row offers the files the document's wildcard actually matched, with the random draw
/// first, and stores the name rather than the whole path.
#[test]
fn a_file_row_offers_what_the_wildcard_matched_and_stores_the_name() {
    let mut fixture = Fixture::new("file");
    fixture.choose();
    let path = fixture.config.skin.document(MUSIC_SELECT).expect("a document is chosen").to_string();

    assert!(fixture.skins.step(&mut fixture.config, SkinRow::File(0), 1));
    assert_eq!(fixture.line(SkinRow::File(0)).1, "night.png");
    assert_eq!(fixture.config.skin.user_config(&path).filepaths.get("Background").map(String::as_str), Some("night.png"));

    assert!(fixture.skins.step(&mut fixture.config, SkinRow::File(0), 1));
    assert_eq!(fixture.line(SkinRow::File(0)).1, RANDOM_SELECTION, "the draw is one of the entries the row cycles");
}

/// An offset row moves by one step a press, stops at the limit its axis declares, and is put
/// back to the author's placement rather than cycled.
#[test]
fn an_offset_row_steps_stops_at_its_limit_and_is_reset_to_nothing() {
    let mut fixture = Fixture::new("offset");
    fixture.choose();
    fixture.skins.reload(&fixture.config);
    let row = SkinRow::Offset(0, OffsetAxis::Y);

    assert!(fixture.skins.step(&mut fixture.config, row, -1));
    assert_eq!(fixture.line(row).1, "-1");

    for _ in 0..2 {
        fixture.skins.step(&mut fixture.config, row, 1);
    }
    assert_eq!(fixture.line(row).1, "+1");
    assert!(!fixture.skins.needs_reload(&fixture.config), "a nudge asked for a reload it does not need");

    let steps = OFFSET_POSITION_LIMIT as i32 + 1;
    for _ in 0..steps {
        fixture.skins.step(&mut fixture.config, row, 1);
    }
    assert_eq!(fixture.line(row).1, format!("+{}", OFFSET_POSITION_LIMIT as i32), "the row left the range its axis declares");
    assert!(!fixture.skins.step(&mut fixture.config, row, 1), "a row at its limit still reported a move");

    assert!(fixture.skins.reset_row(&mut fixture.config, row));
    assert_eq!(fixture.line(row).1, "+0");
    assert!(!fixture.skins.reset_row(&mut fixture.config, row), "a row already at nothing still reported a move");
    assert!(!fixture.skins.reset_row(&mut fixture.config, SkinRow::Property(0)), "a property row was zeroed rather than cycled");
}

/// RESET drops the choices made in one document and leaves the ones made in another alone.
#[test]
fn reset_drops_only_the_chosen_documents_choices() {
    let mut fixture = Fixture::new("reset");
    fixture.choose();
    fixture.skins.step(&mut fixture.config, SkinRow::Property(0), 1);
    fixture.config.skin.customise("elsewhere.json").properties.insert("Kept".into(), 7);

    assert!(fixture.skins.forget(&mut fixture.config));
    assert_eq!(fixture.line(SkinRow::Property(0)).1, "OFF", "the document did not go back to what its author chose");
    assert_eq!(fixture.config.skin.user_config("elsewhere.json").properties.get("Kept"), Some(&7));
    assert!(!fixture.skins.forget(&mut fixture.config), "a document with nothing stored still reported a drop");
}

/// The LOADED row is what a player reads to find out whether their document is being drawn: the
/// built-in screen, a document that is not read yet, and the size and parser once it is.
#[test]
fn the_loaded_row_reports_the_document_that_is_actually_being_drawn() {
    let mut fixture = Fixture::new("info");
    assert_eq!(fixture.skins.info(&fixture.config), BUILT_IN_INFO);

    fixture.choose();
    assert_eq!(fixture.skins.info(&fixture.config), NOT_READ_INFO);

    fixture.skins.reload(&fixture.config);
    assert!(!fixture.skins.needs_reload(&fixture.config));
    assert_eq!(fixture.skins.info(&fixture.config), "1920X1080 - JSON - 0 WARNINGS - dj");

    fixture.config.skin.screen = PLAY_7KEYS;
    fixture.choose();
    fixture.skins.reload(&fixture.config);
    assert_eq!(fixture.skins.info(&fixture.config), "1280X720 - JSON5 - 0 WARNINGS", "the lenient dialect was read as strict JSON");
}

/// A screen keeps the document it was read for while the tab is configuring another: every screen
/// is drawn from its own document, so reading one must not drop the one before it.
#[test]
fn each_screen_keeps_its_own_document_and_its_own_read() {
    let mut fixture = Fixture::new("per-screen");
    let browser = fixture.settings.parent().expect("the settings file has a folder").join("skin/browser/browser.json");
    let play = fixture.settings.parent().expect("the settings file has a folder").join("skin/play.json5");
    fixture.config.skin.select(MUSIC_SELECT, Some(browser.to_string_lossy().into_owned()));
    fixture.config.skin.select(PLAY_7KEYS, Some(play.to_string_lossy().into_owned()));

    assert!(fixture.skins.needs_reload_for(&fixture.config, MUSIC_SELECT), "an unread screen did not ask to be read");
    fixture.skins.reload_for(&fixture.config, MUSIC_SELECT);
    fixture.skins.reload_for(&fixture.config, PLAY_7KEYS);

    assert!(fixture.skins.document(MUSIC_SELECT).is_some(), "reading the play screen dropped the browser's document");
    assert!(fixture.skins.document(PLAY_7KEYS).is_some());
    assert!(!fixture.skins.needs_reload_for(&fixture.config, MUSIC_SELECT), "a screen just read still asks to be read");

    let first = fixture.skins.build_of(MUSIC_SELECT).expect("the browser was read");
    assert_ne!(fixture.skins.build_of(PLAY_7KEYS), Some(first), "two reads were handed the same build");
    fixture.skins.reload_for(&fixture.config, MUSIC_SELECT);
    assert_ne!(fixture.skins.build_of(MUSIC_SELECT), Some(first), "reading the same screen again reused its build");
}

/// Deselecting a screen's document takes it away rather than leaving the last one drawn.
#[test]
fn choosing_the_built_in_screen_drops_the_document_that_was_read() {
    let mut fixture = Fixture::new("deselect");
    fixture.choose();
    fixture.skins.reload(&fixture.config);
    assert!(fixture.skins.document(MUSIC_SELECT).is_some(), "the chosen document was never read");

    fixture.config.skin.select(MUSIC_SELECT, None);
    assert!(fixture.skins.needs_reload_for(&fixture.config, MUSIC_SELECT), "going back to the built-in screen changed nothing");
    fixture.skins.reload_for(&fixture.config, MUSIC_SELECT);
    assert!(fixture.skins.document(MUSIC_SELECT).is_none(), "the built-in screen is drawing but a document is still loaded");
}

/// A screen this build draws nothing for is still listed, and says so rather than leaving the
/// player wondering where their skin went.
#[test]
fn a_screen_this_build_does_not_draw_says_so_rather_than_hiding() {
    let mut fixture = Fixture::new("unsupported");
    assert!(skin_screen_label_exists(COURSE_RESULT), "a screen the reference declares is missing from the row's list");
    fixture.config.skin.screen = COURSE_RESULT;
    fixture.config.skin.select(COURSE_RESULT, Some("ghost.json".into()));
    assert_eq!(fixture.skins.info(&fixture.config), UNSUPPORTED_INFO);
    fixture.skins.reload(&fixture.config);
    assert!(fixture.skins.document(COURSE_RESULT).is_none(), "an unsupported screen was read anyway");
}

/// A document that cannot be read leaves the built-in screen drawing and puts the reason on the
/// LOADED row, rather than stopping the player getting into a song.
#[test]
fn a_document_that_cannot_be_read_falls_back_and_says_why() {
    let mut fixture = Fixture::new("broken");
    let broken = fixture.settings.parent().expect("the settings file has a folder").join("skin/broken.json");
    std::fs::write(&broken, "{ \"type\": 5, ").expect("the document is written");
    fixture.config.skin.select(MUSIC_SELECT, Some(broken.to_string_lossy().into_owned()));

    fixture.skins.reload(&fixture.config);
    assert!(fixture.skins.document(MUSIC_SELECT).is_none(), "a document that will not parse was drawn anyway");
    assert_ne!(fixture.skins.info(&fixture.config), NOT_READ_INFO, "the reason never reached the row");
    assert!(fixture.skins.info(&fixture.config).contains("broken.json"), "the reason does not name the document");
}

/// A document outside the skin folder is refused: the folder is the only place a skin may be
/// read from, and a path that climbs out of it is a traversal attempt whether or not it exists.
#[test]
fn a_document_outside_the_skin_folder_is_refused() {
    let mut fixture = Fixture::new("escape");
    fixture.config.skin.select(MUSIC_SELECT, Some("../settings.ron".into()));
    fixture.skins.reload(&fixture.config);
    assert!(fixture.skins.document(MUSIC_SELECT).is_none(), "a path that climbed out of the skin folder was read");
}

fn skin_screen_label_exists(screen: i32) -> bool {
    usize::try_from(screen).is_ok_and(|at| at < SKIN_SCREEN_LABELS.len())
}
