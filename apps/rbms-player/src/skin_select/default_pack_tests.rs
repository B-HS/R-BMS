//! The pack a run draws when the player named none: the default pack every run installs beside the
//! settings file. Which pack is in effect, what the PACK FOLDER row says about it, which screens it
//! has no document for, where its skins write and what is kept of the choices made in them.

use std::path::{Path, PathBuf};

use rbms_config::Config;
use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_PLAY_24KEYS_DOUBLE, SKIN_TYPE_RESULT,
    SKIN_TYPE_SKIN_SELECT,
};
use rbms_skin::property::DefaultState;

use super::*;
use crate::assets::default_pack::{DEFAULT_SKIN_DIRECTORY, DEFAULT_SOUND_DIRECTORY};
use crate::assets::{SKIN_DATA_FOLDER, default_sound_folder, install_default_pack, skin_pack_identifier};

/// The screens the default pack has a document for, and the file each is drawn with.
const DEFAULT_DOCUMENTS: [(i32, &str); 9] = [
    (SKIN_TYPE_MUSIC_SELECT, "select.luaskin"),
    (SKIN_TYPE_DECIDE, "decide.luaskin"),
    (SKIN_TYPE_PLAY_5KEYS, "play5.luaskin"),
    (SKIN_TYPE_PLAY_7KEYS, "play7.luaskin"),
    (SKIN_TYPE_PLAY_9KEYS, "play9.luaskin"),
    (SKIN_TYPE_PLAY_10KEYS, "play10.luaskin"),
    (SKIN_TYPE_PLAY_14KEYS, "play14.luaskin"),
    (SKIN_TYPE_RESULT, "result.luaskin"),
    (SKIN_TYPE_COURSE_RESULT, "courseresult.luaskin"),
];

/// The screens the default pack has no document for, which stay with their built-in layouts: the
/// three keyboard modes, the key configuration screen and the skin configuration screen.
const BUILT_IN_SCREENS: [i32; 5] =
    [SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_PLAY_24KEYS_DOUBLE, SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_SKIN_SELECT];

/// A document for the decide screen that is only data, for a pack the player names.
const PLAIN_DECIDE: &str = r#"{ "type": 6, "name": "Plain", "w": 1280, "h": 720, "destination": [] }"#;

/// A seed the reads below are pinned with.
const READ_SEED: u64 = 7;

/// A sound the default pack's browser plays, as the skin names it.
const SKIN_SOUND: &str = "sound/o-open.wav";

/// A settings file in a folder of the test's own, with nothing beside it.
fn settings_of(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("rbms-default-select-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the fixture folder is writable");
    home.join("settings.ron")
}

/// A settings file with the default pack installed beside it.
fn installed(tag: &str) -> PathBuf {
    let settings = settings_of(tag);
    assert!(install_default_pack(&settings).is_complete(), "the default pack did not install");
    settings
}

/// The file name of the document `screen` is to be drawn with.
fn document_name(skins: &SkinLibrary, config: &Config, screen: i32) -> Option<String> {
    let path = skins.document_path(config, screen)?;
    Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned())
}

/// A pack of the player's own holding one decide document, beside `settings`.
fn players_pack(settings: &Path) -> PathBuf {
    let pack = settings.parent().expect("the settings file has a folder").join("mine");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join("decide.json"), PLAIN_DECIDE).expect("the document is written");
    pack
}

/// The Lua skin pack the skin crate's own tests are written against, which has a play skin and no
/// browser.
fn mini_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/rbms-skin/tests/fixtures/luaskin/mini")
        .canonicalize()
        .expect("the pack fixture is in the repository")
}

/// A player who named no pack is given the installed default pack: every screen it has a document
/// for is drawn with that document, and the PACK FOLDER row says the default pack is drawn and
/// where it is. Until the pack is installed there is no pack, and the row says so.
#[test]
fn with_no_pack_named_the_installed_default_pack_draws_every_screen_it_has_a_document_for() {
    let settings = settings_of("none-named");
    let config = Config::default();
    let bare = SkinLibrary::new(&settings, &config);
    assert_eq!(bare.pack_value(&config), NONE_VALUE, "a pack that is not installed was shown");
    assert!(DEFAULT_DOCUMENTS.iter().all(|(screen, _)| bare.document_path(&config, *screen).is_none()), "a screen had a document with no pack installed");
    assert_eq!(pack_in_effect(None, &config, &settings), None);

    assert!(install_default_pack(&settings).is_complete());
    let skins = SkinLibrary::new(&settings, &config);
    for (screen, file) in DEFAULT_DOCUMENTS {
        assert_eq!(document_name(&skins, &config, screen).as_deref(), Some(file), "screen {screen} is not drawn with the default pack's document");
    }
    let pack = settings.with_file_name(DEFAULT_SKIN_FOLDER).join(DEFAULT_SKIN_DIRECTORY);
    assert_eq!(skins.pack_value(&config), format!("{DEFAULT_VALUE} ({})", pack.display()), "the row does not say the default pack is in effect");
    assert_eq!(pack_in_effect(None, &config, &settings), Some(pack));
    assert!(!skins.pack_moved(&config), "a library that was just made is already behind");
    assert_eq!(config.skin.pack_folder(), None, "the default pack was written into the settings");
}

/// The default pack has no document for the keyboard modes or for the two configuration screens,
/// so those are drawn by their built-in layouts and nothing is asked to be read for them.
#[test]
fn the_screens_the_default_pack_has_no_document_for_stay_built_in() {
    let settings = installed("built-in");
    let config = Config::default();
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan(&settings, &config);
    for screen in BUILT_IN_SCREENS {
        assert_eq!(skins.document_path(&config, screen), None, "screen {screen} was given a document");
        assert!(!skins.needs_reload_for(&config, screen), "screen {screen} asks for a document it does not have");
        skins.request_for(&config, screen);
        assert!(!skins.is_waiting(screen) && skins.document(screen).is_none() && skins.take_failure(screen).is_none());
    }
    assert!(skins.take_unreadable().is_empty(), "a document of the default pack could not be read");
}

/// A pack the run was started with outranks the one the settings name, which outranks the default
/// pack. And a pack the player chose is drawn alone: a screen it has no document for is left to its
/// built-in layout, not filled in from the default pack.
#[test]
fn the_default_pack_gives_way_to_a_named_pack_and_does_not_fill_in_for_it() {
    let settings = installed("priority");
    let mine = players_pack(&settings);
    let mut config = Config::default();
    config.skin.pack = Some(mine.to_string_lossy().into_owned());

    let configured = SkinLibrary::new(&settings, &config);
    assert_eq!(document_name(&configured, &config, SKIN_TYPE_DECIDE).as_deref(), Some("decide.json"), "the configured pack is not drawn");
    assert_eq!(configured.document_path(&config, SKIN_TYPE_MUSIC_SELECT), None, "the default pack filled in for the configured one");
    assert_eq!(configured.document_path(&config, SKIN_TYPE_PLAY_9KEYS), None, "the default pack filled in for the configured one");
    assert_eq!(configured.pack_value(&config), mine.to_string_lossy());
    assert_eq!(pack_in_effect(None, &config, &settings), Some(mine.clone()));

    let forced = SkinLibrary::new(&settings, &config).forcing_pack(Some(mini_pack()), &config);
    assert_eq!(document_name(&forced, &config, SKIN_TYPE_PLAY_7KEYS).as_deref(), Some("nonote.luaskin"), "the pack named for the run is not drawn");
    assert_eq!(forced.document_path(&config, SKIN_TYPE_DECIDE), None, "the configured pack still draws a screen");
    assert_eq!(forced.document_path(&config, SKIN_TYPE_MUSIC_SELECT), None, "the default pack filled in for the pack named for the run");
    assert!(forced.pack_value(&config).ends_with(&format!("({SKIN_PACK_ENV})")));
    assert_eq!(pack_in_effect(Some(&mini_pack()), &config, &settings), Some(mini_pack()));

    let unnamed = Config::default();
    let over_default = SkinLibrary::new(&settings, &unnamed).forcing_pack(Some(mini_pack()), &unnamed);
    assert_eq!(over_default.document_path(&unnamed, SKIN_TYPE_MUSIC_SELECT), None, "the pack named for the run did not outrank the default pack");
}

/// Emptying the PACK FOLDER row goes back to the default pack, on the next walk of the folders and
/// whether the row was emptied or left blank.
#[test]
fn a_pack_folder_row_that_is_emptied_goes_back_to_the_default_pack() {
    let settings = installed("emptied");
    let mine = players_pack(&settings);
    let mut config = Config::default();
    config.skin.pack = Some(mine.to_string_lossy().into_owned());
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan(&settings, &config);
    assert_eq!(skins.document_path(&config, SKIN_TYPE_MUSIC_SELECT), None);

    for emptied in [None, Some("  ".to_owned())] {
        config.skin.pack = Some(mine.to_string_lossy().into_owned());
        skins.rescan(&settings, &config);
        config.skin.pack = emptied;
        assert!(skins.pack_moved(&config), "emptying the row changed nothing");
        skins.rescan(&settings, &config);
        assert!(!skins.pack_moved(&config));
        assert_eq!(document_name(&skins, &config, SKIN_TYPE_MUSIC_SELECT).as_deref(), Some("select.luaskin"), "the default pack did not come back");
        assert_eq!(skins.document_path(&config, SKIN_TYPE_DECIDE).map(Path::new).and_then(Path::extension), Some("luaskin".as_ref()));
        assert!(skins.pack_value(&config).starts_with(DEFAULT_VALUE), "the row does not say the default pack is back");
    }
}

/// The default pack keeps one identifier, and so one folder its skins write into, from run to run:
/// installing again changes neither, the folder is beside the settings file and not in the pack,
/// and the pack's documents read their files from the pack's own folder.
#[test]
fn the_default_pack_keeps_one_identifier_and_one_overlay_from_run_to_run() {
    let settings = installed("identifier");
    let config = Config::default();
    let skins = SkinLibrary::new(&settings, &config);
    let pack = pack_in_effect(None, &config, &settings).expect("the default pack is installed");
    let document = PathBuf::from(skins.document_path(&config, SKIN_TYPE_MUSIC_SELECT).expect("the default pack has a browser"));

    let identifier = skin_pack_identifier(&pack);
    assert!(identifier.starts_with(&format!("{DEFAULT_SKIN_DIRECTORY}-")), "the identifier does not name the pack: {identifier}");
    let overlay = skin_overlay_folder(&settings, &document);
    assert_eq!(overlay, settings.with_file_name(SKIN_DATA_FOLDER).join(&identifier), "the pack's skins do not write beside the settings file");
    assert!(!overlay.starts_with(&pack) && !overlay.exists());
    assert_eq!(plain_path(skins.reading_root(&document)), plain_path(&pack), "a document of the default pack reads from somewhere other than the pack");

    assert!(install_default_pack(&settings).is_complete());
    let again = SkinLibrary::new(&settings, &config);
    let document_again = PathBuf::from(again.document_path(&config, SKIN_TYPE_MUSIC_SELECT).expect("the default pack has a browser"));
    assert_eq!(skin_pack_identifier(&pack), identifier, "one pack was given two identifiers");
    assert_eq!(document_again, document, "the document a run stores choices under moved between two runs");
    assert_eq!(skin_overlay_folder(&settings, &document_again), overlay);
}

/// What the player chooses on a row of a default pack document reaches the settings file and is
/// read back by the next run: the row shows it, and the document is read with it.
#[test]
fn a_choice_made_in_a_default_pack_document_is_saved_and_restored() {
    let settings = installed("choices");
    let mut config = Config { skin: rbms_config::SkinOptions { screen: SKIN_TYPE_DECIDE, ..Default::default() }, ..Config::default() };
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan_afresh(&settings, &config);
    let rows = skins.rows(&config);
    let row = *rows.iter().find(|row| matches!(row, SkinRow::Property(_))).expect("the default decide document declares an option row");
    let (label, before) = skins.line(&config, row);

    assert!(skins.step(&mut config, row, 1), "the row did not move");
    let (_, chosen) = skins.line(&config, row);
    assert_ne!(chosen, before, "stepping the row left it where it was");
    rbms_config::save(&config, &settings).expect("the settings are saved");

    let restored = rbms_config::load(&settings).expect("the settings are read back").config;
    let mut next_run = SkinLibrary::new(&settings, &restored);
    next_run.rescan(&settings, &restored);
    assert_eq!(next_run.line(&restored, row), (label, chosen), "the choice did not survive to the next run");
    let path = next_run.document_path(&restored, SKIN_TYPE_DECIDE).expect("the default pack has a decide document").to_owned();
    assert_eq!(restored.skin.user_config(&path).properties.len(), 1, "the choice is not handed to the document");

    next_run.reload_for(&restored, SKIN_TYPE_DECIDE, SkinRead { host: &DefaultState, seed: Some(READ_SEED) });
    assert_eq!(next_run.failure(SKIN_TYPE_DECIDE), None);
    assert!(next_run.document(SKIN_TYPE_DECIDE).is_some(), "the installed document was not read with the stored choice");
}

/// The default pack's browser says it knows this player's own ids, which is what the option panels'
/// reassigned keys are read by. No other document of the pack says it: only the browser reads the
/// mark.
#[test]
fn the_default_select_skin_names_the_rbms_extensions() {
    let settings = installed("extensions");
    let config = Config::default();
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan(&settings, &config);
    for (screen, names_them) in [(SKIN_TYPE_MUSIC_SELECT, true), (SKIN_TYPE_PLAY_7KEYS, false), (SKIN_TYPE_DECIDE, false)] {
        skins.reload_for(&config, screen, SkinRead { host: &DefaultState, seed: Some(READ_SEED) });
        assert_eq!(skins.failure(screen), None, "screen {screen}");
        let skin = skins.document(screen).unwrap_or_else(|| panic!("screen {screen} was not read"));
        assert_eq!(skin.def.knows_rbms_extensions(), names_them, "screen {screen}");
    }
}

/// The options of the seven-key play document that pick colours, each with the item it is put on:
/// the slate theme, the sky accent and the white and yellow notes.
const COLOUR_OPTIONS: [(&str, i32); 3] = [("Theme", 921), ("Accent", 923), ("Note Colors", 932)];

/// The colours those options draw the parts of some colour rows in, by the row's name: the slate
/// panel, the sky accent, the yellow of the black keys, which their long notes follow, and the red of
/// the turntable.
const COLOURS_DRAWN: [(&str, SkinColor); 5] = [
    ("White Key Lane", SkinColor::rgba(20, 25, 34, u8::MAX)),
    ("Judge Line", SkinColor::rgba(76, 201, 240, u8::MAX)),
    ("Black Key Note", SkinColor::rgba(242, 179, 61, u8::MAX)),
    ("Black Key LN", SkinColor::rgba(242, 179, 61, u8::MAX)),
    ("Scratch Note", SkinColor::rgba(240, 86, 76, u8::MAX)),
];

/// What the alpha of the black keys' notes is put on, about half of all of it.
const HALF_ALPHA: u8 = 128;

/// A colour row of a default pack document nobody has set shows, and starts its edits from, the
/// colour the document draws its part in with the options it is read with, not the default its
/// header declares: putting the alpha of the yellow black-key notes at half stores a half-clear
/// yellow, not a half-clear cyan.
#[test]
fn an_unset_colour_row_of_the_default_pack_starts_from_the_colour_its_options_draw() {
    let settings = installed("drawn-colours");
    let mut config = Config { skin: rbms_config::SkinOptions { screen: SKIN_TYPE_PLAY_7KEYS, ..Default::default() }, ..Config::default() };
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan(&settings, &config);
    let path = skins.document_path(&config, SKIN_TYPE_PLAY_7KEYS).expect("the default pack has a seven-key document").to_owned();
    for (name, op) in COLOUR_OPTIONS {
        config.skin.customise(&path).properties.insert(name.to_owned(), op);
    }
    skins.reload_for(&config, SKIN_TYPE_PLAY_7KEYS, SkinRead { host: &DefaultState, seed: Some(READ_SEED) });
    assert_eq!(skins.failure(SKIN_TYPE_PLAY_7KEYS), None);

    let header = skins.header_of(&config, SKIN_TYPE_PLAY_7KEYS).expect("the document has a header").clone();
    let at = |name: &str| header.colors.iter().position(|row| row.name == name).unwrap_or_else(|| panic!("the document has no colour row {name:?}"));
    for (name, drawn) in COLOURS_DRAWN {
        assert_eq!(skins.color_for(&config, SKIN_TYPE_PLAY_7KEYS, at(name)), Some((drawn, false)), "the row {name:?} does not show what is drawn");
    }

    let black_notes = at("Black Key Note");
    assert!(skins.set_number_for(&mut config, SKIN_TYPE_PLAY_7KEYS, SkinRow::ColorChannel(black_notes, ColorChannel::A), i64::from(HALF_ALPHA)));
    let id = header.colors[black_notes].id;
    let yellow = COLOURS_DRAWN[2].1;
    assert_eq!(config.skin.customisation(&path).and_then(|entry| entry.colors.get(&id).copied()), Some(SkinColor { a: HALF_ALPHA, ..yellow }));
}

/// The sound a default pack skin plays by its own path and the cue the player raises are the same
/// installed file: the skin's `sound` folder is the system sound folder of a player who configured
/// none.
#[test]
fn a_default_pack_skin_and_the_system_sounds_read_the_same_installed_folder() {
    let settings = installed("sounds");
    let config = Config::default();
    let pack = pack_in_effect(None, &config, &settings).expect("the default pack is installed");
    let system = default_sound_folder(&settings).expect("the system sound set is installed");
    assert_eq!(system, pack.join(DEFAULT_SOUND_DIRECTORY));
    let played = pack.join(SKIN_SOUND);
    assert!(played.is_file(), "the sound the browser plays is not installed");
    assert_eq!(played.parent(), Some(system.as_path()), "the skin plays a file outside the system sound folder");
}
