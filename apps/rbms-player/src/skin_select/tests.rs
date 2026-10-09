//! What the SKIN tab reads out of a skin folder: which documents are in it, which of them draws
//! each screen, and what stepping one of a document's own customisation rows stores.

use super::fixtures::*;
use super::*;
use rbms_config::{Config, SKIN_SCREEN_LABELS};
use rbms_skin::loader::OPTION_RANDOM_VALUE;
use rbms_skin::property::MapHost;

/// The `SkinType` id of the decide screen.
const DECIDE: i32 = 6;

/// The `SkinType` id of the skin configuration screen, which the pack fixture has a document for.
const SKIN_SELECT: i32 = 9;

/// Entry files in the pack fixture.
const PACK_DOCUMENTS: u64 = 4;

/// Documents in the skin folder fixture, beside the decoy that is not one.
const FOLDER_DOCUMENTS: u64 = 2;

/// Rows a play skin with no offsets of its own shows: the automatic offsets, one row for each axis
/// the player may move -- four for the whole screen, one for the notes, five for each of the two
/// judgement offsets.
const AUTOMATIC_OFFSET_ROWS: usize = 15;

/// The number the pack fixture's value function reads, and what the host below answers for it.
const SCORE_NUMBER: (i32, i32) = (71, 21);

/// A seed the reads below are pinned with.
const READ_SEED: u64 = 7;

/// How many seeds are tried to show that the seed a read is given decides what it draws.
const SEEDS_TRIED: u64 = 32;

/// The row of the pack fixture's play skin that is stored as "pick one at random" below.
const RANDOM_ROW: &str = "Lucky";

/// A Lua skin for the decide screen whose header reads but whose body raises.
const BROKEN_DECIDE: &str = "if skin_config then\n    error('no such asset')\nend\nreturn { type = 6, name = 'Broken', w = 1280, h = 720 }\n";

/// A document for the decide screen that is only data.
const PLAIN_DECIDE: &str = r#"{ "type": 6, "name": "Plain", "w": 1280, "h": 720, "destination": [] }"#;

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

/// The Lua skin pack the skin crate's own tests are written against: two play skins, a skin
/// configuration screen, a course result screen, and the modules and images they share.
fn mini_pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/rbms-skin/tests/fixtures/luaskin/mini")
        .canonicalize()
        .expect("the pack fixture is in the repository")
}

/// Every file under `root`, as paths relative to it.
fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the folder is listable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf());
            }
        }
    }
    found
}

/// A copy of the pack fixture a test may change, in a folder of its own beside `settings`.
fn copied_pack(settings: &Path, name: &str) -> PathBuf {
    let source = mini_pack();
    let pack = settings.parent().expect("the settings file has a folder").join(name);
    for file in files_under(&source) {
        let target = pack.join(&file);
        std::fs::create_dir_all(target.parent().expect("a copied file has a folder")).expect("the copy's folder is writable");
        std::fs::copy(source.join(&file), &target).expect("the fixture file is copied");
    }
    pack.canonicalize().expect("the copy was just written")
}

/// A pack holding one document, in a folder of its own beside `settings`.
fn one_document_pack(settings: &Path, name: &str, file: &str, text: &str) -> PathBuf {
    let pack = settings.parent().expect("the settings file has a folder").join(name);
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    std::fs::write(pack.join(file), text).expect("the document is written");
    pack.canonicalize().expect("the pack was just written")
}

/// The file name of the document one screen is to be drawn with.
fn document_name(fixture: &Fixture, screen: i32) -> Option<String> {
    let path = fixture.skins.document_path(&fixture.config, screen)?;
    Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned())
}

impl Fixture {
    /// Name `pack` as the configured skin pack and walk the folders again.
    fn name_pack(&mut self, pack: &Path) {
        self.config.skin.pack = Some(pack.to_string_lossy().into_owned());
        self.skins.rescan(&self.settings, &self.config);
    }

    /// Read the Lua skin waiting for `screen` the way the screen's first two frames do.
    fn read_waiting(&mut self, screen: i32, host: &dyn SkinHost) {
        self.skins.read_waiting(&self.config, screen, SkinRead { host, seed: Some(READ_SEED) });
        self.skins.adopt(screen);
    }
}

/// A pack is one folder with a document for each screen: naming it gives every screen the document
/// that declares it, the first by file name when two do, and leaves a screen the pack has nothing
/// for to its built-in layout. The modules a Lua skin requires are not documents.
#[test]
fn a_pack_gives_each_screen_the_document_that_declares_it() {
    let mut fixture = Fixture::new("pack-map");
    assert_eq!(document_name(&fixture, PLAY_7KEYS), None, "a screen had a document before any pack was named");

    fixture.name_pack(&mini_pack());
    assert_eq!(
        document_name(&fixture, PLAY_7KEYS).as_deref(),
        Some("nonote.luaskin"),
        "two documents declare the play screen and the first by name was not taken"
    );
    assert_eq!(document_name(&fixture, SKIN_SELECT).as_deref(), Some("config.luaskin"));
    assert_eq!(document_name(&fixture, COURSE_RESULT).as_deref(), Some("course.luaskin"));
    assert_eq!(document_name(&fixture, MUSIC_SELECT), None, "the pack has no document for the browser");

    let lua: Vec<&SkinHeader> = fixture.skins.documents.iter().filter(|header| header.parser == ParserKind::Lua).collect();
    assert_eq!(
        lua.len() as u64,
        PACK_DOCUMENTS,
        "a module was read as a document, or an entry file was missed: {:?}",
        lua.iter().map(|header| &header.path).collect::<Vec<_>>()
    );

    fixture.config.skin.screen = PLAY_7KEYS;
    assert_eq!(fixture.skins.candidates(&fixture.config).len(), 3, "the SKIN row does not offer the pack's Lua skins beside the folder's document");
    assert_eq!(fixture.skins.document_value(&fixture.config), "PACK: No notes placed", "the row does not say the pack chose the document");
    assert_eq!(fixture.skins.rows(&fixture.config).len(), AUTOMATIC_OFFSET_ROWS, "a play skin's rows are its four automatic offsets, one row an axis");
}

/// A Lua skin's header is a program. It is run once per version of its file: walking the folders
/// again answers from memory, a file that changed is read again, and a reload reads everything.
#[test]
fn a_header_is_not_read_again_while_its_file_has_not_changed() {
    let mut fixture = Fixture::new("header-cache");
    let from_folder = fixture.skins.header_reads();
    assert_eq!(from_folder, FOLDER_DOCUMENTS, "the skin folder holds two documents and a decoy");

    let pack = copied_pack(&fixture.settings, "pack");
    fixture.name_pack(&pack);
    let after_pack = fixture.skins.header_reads();
    assert_eq!(after_pack, from_folder + PACK_DOCUMENTS, "the pack holds four entry files");

    fixture.skins.rescan(&fixture.settings, &fixture.config);
    fixture.skins.rescan(&fixture.settings, &fixture.config);
    assert_eq!(fixture.skins.header_reads(), after_pack, "a header whose file did not change was run again");
    assert_eq!(document_name(&fixture, COURSE_RESULT).as_deref(), Some("course.luaskin"), "the remembered headers lost a document");

    std::fs::write(pack.join("course.luaskin"), "return { type = 15, name = 'Course result, edited', w = 1280, h = 720, destination = {} }")
        .expect("the copy is writable");
    fixture.skins.rescan(&fixture.settings, &fixture.config);
    assert_eq!(fixture.skins.header_reads(), after_pack + 1, "only the file that changed is read again");
    let edited = fixture.skins.header_of(&fixture.config, COURSE_RESULT).expect("the edited document is still the pack's");
    assert_eq!(edited.name, "Course result, edited", "the header of a changed file was answered from memory");

    fixture.skins.rescan_afresh(&fixture.settings, &fixture.config);
    assert_eq!(fixture.skins.header_reads(), after_pack + 1 + FOLDER_DOCUMENTS + PACK_DOCUMENTS, "a reload reads every header from its file");
}

/// The pack a run is started with outranks the one the settings file names, for that run and
/// without being written into the settings.
#[test]
fn the_pack_named_for_this_run_outranks_the_configured_one() {
    let fixture = Fixture::new("pack-env");
    let configured = one_document_pack(&fixture.settings, "configured", "decide.json", PLAIN_DECIDE);
    let mut config = fixture.config.clone();
    config.skin.pack = Some(configured.to_string_lossy().into_owned());

    let own = SkinLibrary::new(&fixture.settings, &config);
    assert!(own.document_path(&config, DECIDE).is_some_and(|path| path.ends_with("decide.json")), "the configured pack is not drawn");
    assert_eq!(own.document_path(&config, PLAY_7KEYS), None);
    assert_eq!(own.pack_value(&config), configured.to_string_lossy());

    let forced = SkinLibrary::new(&fixture.settings, &config).forcing_pack(Some(mini_pack()), &config);
    assert!(forced.document_path(&config, PLAY_7KEYS).is_some_and(|path| path.ends_with("nonote.luaskin")), "the pack named for the run is not drawn");
    assert_eq!(forced.document_path(&config, DECIDE), None, "the configured pack still draws a screen");
    assert!(forced.pack_value(&config).ends_with(&format!("({SKIN_PACK_ENV})")), "the row does not say where the pack came from");
    assert_eq!(config.skin.pack_folder(), Some(configured.to_string_lossy().as_ref()), "the pack named for one run was written into the settings");
    assert!(!forced.pack_moved(&config));

    assert_eq!(pack_from_environment(None), None);
    assert_eq!(pack_from_environment(Some("  ".into())), None, "a blank variable named a pack");
    assert_eq!(pack_from_environment(Some("/skins/pack".into())), Some(PathBuf::from("/skins/pack")));
}

/// A document the player chose for a screen by hand is the one drawn, pack or no pack, and the
/// pack's comes back when the choice is dropped.
#[test]
fn a_document_chosen_by_hand_outranks_the_packs() {
    let mut fixture = Fixture::new("pack-manual");
    fixture.name_pack(&mini_pack());
    fixture.config.skin.screen = PLAY_7KEYS;
    let by_hand = fixture.settings.parent().expect("the settings file has a folder").join("skin/play.json5");
    fixture.config.skin.select(PLAY_7KEYS, Some(by_hand.to_string_lossy().into_owned()));

    assert_eq!(document_name(&fixture, PLAY_7KEYS).as_deref(), Some("play.json5"));
    assert_eq!(fixture.skins.document_value(&fixture.config), "Lane", "a document chosen by hand is marked as the pack's");
    fixture.skins.request_for(&fixture.config, PLAY_7KEYS);
    assert_eq!(fixture.skins.document(PLAY_7KEYS).map(|skin| skin.parser), Some(ParserKind::Json5), "the pack's document was read over the chosen one");

    fixture.config.skin.select(PLAY_7KEYS, None);
    assert_eq!(document_name(&fixture, PLAY_7KEYS).as_deref(), Some("nonote.luaskin"), "dropping the choice did not give the screen back to the pack");
    assert!(fixture.skins.needs_reload_for(&fixture.config, PLAY_7KEYS), "the screen still draws the document that is no longer chosen");
}

/// A Lua skin is not read when it is chosen: it waits for its screen, is read against the state
/// that screen brings and the seed the read is pinned with, writes into the overlay kept for its
/// pack and never into its own folder, and is read again for the next scene.
#[test]
fn a_lua_skin_waits_for_its_screen_and_is_read_against_its_state() {
    let mut fixture = Fixture::new("lua-read");
    let pack = mini_pack();
    let before = files_under(&pack);
    fixture.name_pack(&pack);
    fixture.config.skin.screen = PLAY_7KEYS;
    let play = pack.join("play.luaskin").to_string_lossy().into_owned();
    fixture.config.skin.select(PLAY_7KEYS, Some(play.clone()));
    fixture.config.skin.customise(&play).properties.insert(RANDOM_ROW.to_owned(), OPTION_RANDOM_VALUE);

    assert!(fixture.skins.needs_reload_for(&fixture.config, PLAY_7KEYS));
    fixture.skins.request_for(&fixture.config, PLAY_7KEYS);
    assert!(fixture.skins.is_waiting(PLAY_7KEYS), "a Lua skin was read before its screen had a state to read it against");
    assert!(fixture.skins.document(PLAY_7KEYS).is_none());
    assert_eq!(fixture.skins.info(&fixture.config), WAITING_INFO);
    assert!(!fixture.skins.needs_reload_for(&fixture.config, PLAY_7KEYS), "a skin that is waiting was asked for again");

    let mut host = MapHost::new();
    host.integers.insert(SCORE_NUMBER.0, SCORE_NUMBER.1);
    fixture.read_waiting(PLAY_7KEYS, &host);
    assert!(!fixture.skins.is_waiting(PLAY_7KEYS));
    let skin = fixture.skins.document(PLAY_7KEYS).expect("the skin was read on its screen's frame");
    assert_eq!(skin.parser, ParserKind::Lua);
    assert!(fixture.skins.info(&fixture.config).starts_with("1920X1080 - Lua - LUA"), "{}", fixture.skins.info(&fixture.config));
    let drawn = skin.selected_options.iter().find(|(name, _)| name == RANDOM_ROW).map(|(_, option)| *option).expect("the row is the skin's");
    assert!(!fixture.skins.needs_reload_for(&fixture.config, PLAY_7KEYS));

    let overlay = skin_overlay_folder(&fixture.settings, Path::new(&play));
    assert_eq!(std::fs::read_to_string(overlay.join("log").join("loaded.txt")).ok().as_deref(), Some("loaded"), "the skin's write did not land in its overlay");
    assert!(overlay.starts_with(fixture.settings.parent().expect("the settings file has a folder")));
    assert_eq!(files_under(&pack), before, "reading the skin changed its folder");

    fixture.skins.expire_scripted();
    assert!(fixture.skins.needs_reload_for(&fixture.config, PLAY_7KEYS), "a new scene did not ask for its Lua skin again");
    fixture.skins.request_for(&fixture.config, PLAY_7KEYS);
    fixture.read_waiting(PLAY_7KEYS, &host);
    let again = fixture.skins.document(PLAY_7KEYS).expect("the skin was read again");
    assert_eq!(again.selected_options.iter().find(|(name, _)| name == RANDOM_ROW).map(|(_, option)| *option), Some(drawn), "the same seed drew differently");

    let draws: BTreeSet<i32> = (0..SEEDS_TRIED)
        .filter_map(|seed| {
            fixture.skins.reload_for(&fixture.config, PLAY_7KEYS, SkinRead { host: &host, seed: Some(seed) });
            let skin = fixture.skins.document(PLAY_7KEYS)?;
            skin.selected_options.iter().find(|(name, _)| name == RANDOM_ROW).map(|(_, option)| *option)
        })
        .collect();
    assert!(draws.len() > 1, "the seed a read is given does not reach its draws: {draws:?}");
    assert_eq!(files_under(&pack), before, "reading the skin changed its folder");
}

/// A Lua skin whose body fails leaves its screen to the built-in layout, says why once, and is not
/// run again on the next frame or for the next scene.
#[test]
fn a_lua_skin_that_cannot_be_read_says_why_once_and_is_not_run_again() {
    let mut fixture = Fixture::new("lua-broken");
    let pack = one_document_pack(&fixture.settings, "broken", "decide.luaskin", BROKEN_DECIDE);
    fixture.name_pack(&pack);
    fixture.config.skin.screen = DECIDE;
    assert_eq!(document_name(&fixture, DECIDE).as_deref(), Some("decide.luaskin"), "a skin whose body fails still gives its header");

    fixture.skins.request_for(&fixture.config, DECIDE);
    assert_eq!(fixture.skins.take_failure(DECIDE), None, "a skin that has not been read was reported as failed");
    fixture.read_waiting(DECIDE, &DefaultState);

    assert!(fixture.skins.document(DECIDE).is_none(), "a skin that raised was drawn anyway");
    assert!(!fixture.skins.is_waiting(DECIDE));
    let reason = fixture.skins.take_failure(DECIDE).expect("the failure is reported");
    assert!(reason.contains("no such asset"), "the reason does not say what the skin raised: {reason}");
    assert!(!reason.contains('\n'), "more than the first line was reported");
    assert_eq!(fixture.skins.take_failure(DECIDE), None, "the failure was reported twice");
    assert!(fixture.skins.info(&fixture.config).contains("no such asset"), "the reason left the LOADED row");

    assert!(!fixture.skins.needs_reload_for(&fixture.config, DECIDE), "a skin that failed is read again on the next frame");
    fixture.skins.expire_scripted();
    assert!(!fixture.skins.needs_reload_for(&fixture.config, DECIDE), "a skin that failed is read again for the next scene");

    fixture.skins.reload(&fixture.config);
    assert!(fixture.skins.is_waiting(DECIDE), "asking for a reload did not ask for the skin again");
}

/// A failure belongs to the document that failed, not to the screen: naming another pack gives the
/// screen that pack's document to read, with nothing of the old failure left on the LOADED row.
#[test]
fn a_screen_whose_skin_failed_reads_the_document_of_the_pack_named_next() {
    let mut fixture = Fixture::new("pack-after-failure");
    let broken = one_document_pack(&fixture.settings, "broken", "decide.luaskin", BROKEN_DECIDE);
    let plain = one_document_pack(&fixture.settings, "plain", "decide.json", PLAIN_DECIDE);
    fixture.name_pack(&broken);
    fixture.config.skin.screen = MUSIC_SELECT;

    fixture.skins.request_for(&fixture.config, DECIDE);
    fixture.read_waiting(DECIDE, &DefaultState);
    assert!(fixture.skins.failure(DECIDE).is_some_and(|reason| reason.contains("no such asset")), "the first pack's skin did not fail");
    assert!(!fixture.skins.needs_reload_for(&fixture.config, DECIDE));

    fixture.name_pack(&plain);
    assert_eq!(document_name(&fixture, DECIDE).as_deref(), Some("decide.json"));
    assert_eq!(fixture.skins.failure(DECIDE), None, "the failure of a document the screen is no longer drawn with was kept");
    assert_eq!(fixture.skins.take_failure(DECIDE), None, "the old pack's failure is still waiting to be announced");
    assert!(fixture.skins.needs_reload_for(&fixture.config, DECIDE), "a screen that failed in one pack does not read the next pack's document");
    fixture.skins.request_for(&fixture.config, DECIDE);
    assert_eq!(fixture.skins.document(DECIDE).map(|skin| skin.def.name.as_str()), Some("Plain"), "the next pack's document was not read");
    fixture.config.skin.screen = DECIDE;
    assert!(!fixture.skins.info(&fixture.config).contains("no such asset"), "the LOADED row still shows the old pack's failure");
}

/// The same holds when the selection moves without the library being told, which is what a settings
/// file arriving from the account does: the failure of the old document does not stop the new one
/// being read, and is not what the LOADED row shows for it.
#[test]
fn a_failure_does_not_outlive_the_selection_it_was_raised_for() {
    let mut fixture = Fixture::new("selection-after-failure");
    let folder = fixture.settings.parent().expect("the settings file has a folder").join("skin");
    let broken = folder.join("broken.json");
    std::fs::write(&broken, "{ \"type\": 6, ").expect("the document is written");
    let plain = folder.join("plain.json");
    std::fs::write(&plain, PLAIN_DECIDE).expect("the document is written");
    fixture.config.skin.screen = DECIDE;

    fixture.config.skin.select(DECIDE, Some(broken.to_string_lossy().into_owned()));
    fixture.skins.reload(&fixture.config);
    assert!(fixture.skins.info(&fixture.config).contains("broken.json"));
    assert!(!fixture.skins.needs_reload_for(&fixture.config, DECIDE), "a document that failed is read again unasked");

    fixture.config.skin.select(DECIDE, Some(plain.to_string_lossy().into_owned()));
    assert!(fixture.skins.needs_reload_for(&fixture.config, DECIDE), "the old document's failure stops the new one being read");
    assert!(!fixture.skins.info(&fixture.config).contains("broken.json"), "the LOADED row shows the failure of a document that is not chosen");

    fixture.config.skin.select(DECIDE, None);
    assert_eq!(fixture.skins.info(&fixture.config), BUILT_IN_INFO, "a screen left to its built-in layout still shows a failure");
    fixture.skins.drop_moved(&fixture.config);
    assert_eq!(fixture.skins.failure(DECIDE), None);
    assert!(!fixture.skins.needs_reload_for(&fixture.config, DECIDE), "a screen with nothing read and nothing chosen asks to be read");
}

/// Taking the pack away, or naming one with nothing for a screen, lets go of what was read for every
/// screen it drew -- not only the one the tab is configuring.
#[test]
fn a_pack_that_is_taken_away_leaves_nothing_read_for_the_screens_it_drew() {
    let mut fixture = Fixture::new("pack-dropped");
    let plain = one_document_pack(&fixture.settings, "plain", "decide.json", PLAIN_DECIDE);
    fixture.name_pack(&plain);
    fixture.config.skin.screen = MUSIC_SELECT;
    fixture.skins.request_for(&fixture.config, DECIDE);
    assert!(fixture.skins.document(DECIDE).is_some(), "the pack's document was never read");

    fixture.config.skin.pack = None;
    assert!(fixture.skins.pack_moved(&fixture.config));
    fixture.skins.rescan(&fixture.settings, &fixture.config);
    assert!(fixture.skins.document(DECIDE).is_none(), "a screen nobody is configuring kept the document of a pack that is gone");
    assert_eq!(fixture.skins.build_of(DECIDE), None);
    assert!(!fixture.skins.needs_reload_for(&fixture.config, DECIDE));
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

/// The built-in screen gets no rows: there is nothing to configure until a document is chosen. A
/// play document that declares no customisation of its own still gets the four offsets every play
/// skin has -- the whole screen, the notes, the judgement and its detail -- each with the axes the
/// reference lets a player move.
#[test]
fn only_a_chosen_document_contributes_rows() {
    let mut fixture = Fixture::new("rows-empty");
    assert!(fixture.rows().is_empty(), "the built-in screen offered customisation rows");

    fixture.config.skin.screen = PLAY_7KEYS;
    fixture.choose();
    let whole = [OffsetAxis::X, OffsetAxis::Y, OffsetAxis::W, OffsetAxis::H];
    let judge = [OffsetAxis::X, OffsetAxis::Y, OffsetAxis::W, OffsetAxis::H, OffsetAxis::A];
    let expected: Vec<SkinRow> = whole
        .into_iter()
        .map(|axis| SkinRow::Offset(0, axis))
        .chain([SkinRow::Offset(1, OffsetAxis::H)])
        .chain(judge.into_iter().map(|axis| SkinRow::Offset(2, axis)))
        .chain(judge.into_iter().map(|axis| SkinRow::Offset(3, axis)))
        .collect();
    assert_eq!(fixture.rows(), expected, "a play document's rows are the automatic offsets and nothing of its own");
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
    fixture.skins.request_for(&fixture.config, MUSIC_SELECT);
    fixture.skins.request_for(&fixture.config, PLAY_7KEYS);

    assert!(fixture.skins.document(MUSIC_SELECT).is_some(), "reading the play screen dropped the browser's document");
    assert!(fixture.skins.document(PLAY_7KEYS).is_some());
    assert!(!fixture.skins.needs_reload_for(&fixture.config, MUSIC_SELECT), "a screen just read still asks to be read");

    let first = fixture.skins.build_of(MUSIC_SELECT).expect("the browser was read");
    assert_ne!(fixture.skins.build_of(PLAY_7KEYS), Some(first), "two reads were handed the same build");
    fixture.skins.request_for(&fixture.config, MUSIC_SELECT);
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
    fixture.skins.request_for(&fixture.config, MUSIC_SELECT);
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
