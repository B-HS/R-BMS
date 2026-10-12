//! A pack that is adjusted, exported, and read back: what the exported folder holds, that a run
//! which names it as its pack draws what the run that exported it drew, how the pack's own
//! adjustments stand against the player's, what is refused and what is left behind when an export
//! fails, and how one row's value is put onto the other screens that declare the row.
//!
//! Every pack here is written by the test into a folder of its own. With [`EXPORT_DIR_ENV`] and
//! [`SKIN_PACK_ENV`] both naming folders, the pack the environment names is exported into the other
//! folder and what that came to is printed: how a pack too large for a fixture is tried by hand.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rbms_config::Config;

use super::adjustments::{ADJUSTMENTS_FILE, ADJUSTMENTS_MAX_BYTES, ADJUSTMENTS_MAX_LINES, ADJUSTMENTS_VERSION};
use super::export::{EXPORT_SUFFIX, ExportError, ExportJob, ExportLimits, ExportProgress, ExportRequest, Exported, INCOMPLETE_SUFFIX, export_pack};
use super::fixtures::{
    ADJUSTABLE_LEFT_OUT as LEFT_OUT, ADJUSTABLE_PACK_FILES as PACK_FILES, ADJUSTABLE_SELECT as SELECT_DOCUMENT, write_adjustable_pack as write_pack,
    write_fixture_file as write_file,
};
use super::*;
use crate::assets::default_pack::{DEFAULT_PACK_FILES, MANIFEST_FILE};
use crate::assets::install_default_pack;

/// Environment variable naming a folder outside the repository that the pack [`SKIN_PACK_ENV`]
/// names is exported into. Nothing is exported without both.
const EXPORT_DIR_ENV: &str = "RBMS_SKIN_EXPORT_DIR";

/// The `SkinType` ids of the screens the fixture pack has a document for.
const SELECT: i32 = 5;
const DECIDE: i32 = 6;
const RESULT: i32 = 7;

/// The rows of the browser's document, as the library addresses them.
const THEME: SkinRow = SkinRow::Property(0);
const COVER: SkinRow = SkinRow::Property(1);
const BACKGROUND: SkinRow = SkinRow::File(0);
const JUDGE_X: SkinRow = SkinRow::Offset(0, OffsetAxis::X);
const JUDGE_Y: SkinRow = SkinRow::Offset(0, OffsetAxis::Y);
const PANEL: SkinRow = SkinRow::Offset(1, OffsetAxis::A);

/// The rows a test steps and then looks for on the other side of an export.
const ADJUSTED_ROWS: [SkinRow; 6] = [THEME, COVER, BACKGROUND, JUDGE_X, JUDGE_Y, PANEL];

/// How far the judgement is moved down and how opaque the panel is left, in the steps of each row.
const JUDGE_Y_STEPS: i64 = 12;
const PANEL_PERCENT: i64 = 60;

/// A folder of a test's own, emptied.
fn home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("rbms-skin-export-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the fixture folder is writable");
    home.canonicalize().expect("the fixture folder was just made")
}

/// Every file under `root`, as paths relative to it with `/` between their parts.
fn files_under(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the folder is listable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"));
            }
        }
    }
    found
}

/// What is directly in `folder`, by name.
fn entries_of(folder: &Path) -> BTreeSet<String> {
    std::fs::read_dir(folder).expect("the folder is listable").flatten().map(|entry| entry.file_name().to_string_lossy().into_owned()).collect()
}

/// A run: a settings file of its own, the configuration, and the library over them.
struct Run {
    settings: PathBuf,
    config: Config,
    skins: SkinLibrary,
}

impl Run {
    /// A run with a fresh configuration that names `pack` as its pack, with the folders walked and
    /// the menu on the browser.
    fn naming(home: &Path, name: &str, pack: &Path) -> Run {
        let settings = home.join(name).join("settings.ron");
        std::fs::create_dir_all(settings.parent().expect("the settings file has a folder")).expect("the settings folder is writable");
        let mut config = Config::default();
        config.skin.pack = Some(pack.to_string_lossy().into_owned());
        config.skin.screen = SELECT;
        let mut skins = SkinLibrary::new(&settings, &config);
        skins.rescan(&settings, &config);
        Run { settings, config, skins }
    }

    fn line(&self, screen: i32, row: SkinRow) -> (String, String) {
        self.skins.line_for(&self.config, screen, row)
    }

    fn value(&self, screen: i32, row: SkinRow) -> String {
        self.line(screen, row).1
    }

    fn step(&mut self, row: SkinRow) {
        assert!(self.skins.step_for(&mut self.config, SELECT, row, 1), "{row:?} did not move");
    }

    /// Move the browser's rows off what the author shipped: both options, the file slot, both axes
    /// of the offset that moves, and the opacity.
    fn adjust(&mut self) {
        for row in [THEME, COVER, BACKGROUND, JUDGE_X] {
            self.step(row);
        }
        assert!(self.skins.set_number_for(&mut self.config, SELECT, JUDGE_Y, JUDGE_Y_STEPS));
        assert!(self.skins.set_number_for(&mut self.config, SELECT, PANEL, PANEL_PERCENT));
    }

    /// What every adjusted row of the browser's document reads.
    fn adjusted_lines(&self) -> Vec<(String, String)> {
        ADJUSTED_ROWS.iter().map(|row| self.line(SELECT, *row)).collect()
    }

    fn request(&self, destination: &Path) -> ExportRequest {
        self.skins.export_request(&self.config, destination).expect("there is a pack to export").expect("the adjustments are written out")
    }

    /// Export the pack into `destination`, here and now.
    fn export(&self, destination: &Path) -> Result<Exported, ExportError> {
        export_pack(&self.request(destination), &ExportProgress::default())
    }
}

/// A folder to export into, beside the pack.
fn outbox(home: &Path) -> PathBuf {
    let outbox = home.join("outbox");
    std::fs::create_dir_all(&outbox).expect("the outbox is writable");
    outbox
}

/// An export is one pack folder: every file of the pack as it was, the adjustments file, and
/// nothing of what is this machine's. The adjustments name everything by name and by its place in
/// the pack, and nothing by where the pack happened to be.
#[test]
fn an_export_holds_every_file_of_the_pack_and_the_adjustments_by_name() {
    let home = home("whole");
    let pack = write_pack(&home);
    let mut run = Run::naming(&home, "first", &pack);
    run.adjust();

    let exported = run.export(&outbox(&home)).expect("the pack is exported");
    assert_eq!(exported.folder, outbox(&home).join(format!("pack{EXPORT_SUFFIX}")));
    let mut expected: BTreeSet<String> = files_under(&pack).into_iter().filter(|name| !LEFT_OUT.contains(&name.as_str())).collect();
    expected.insert(ADJUSTMENTS_FILE.to_owned());
    assert_eq!(files_under(&exported.folder), expected, "the export is not the pack's files and the adjustments");
    assert_eq!(exported.files, expected.len());
    assert_eq!(exported.links_skipped, 0);
    for (name, text) in PACK_FILES {
        assert_eq!(std::fs::read_to_string(exported.folder.join(name)).expect("the copy reads"), text, "{name} was not copied whole");
    }
    assert_eq!(std::fs::read_to_string(exported.folder.join("select.json")).expect("the copy reads"), SELECT_DOCUMENT);

    let text = std::fs::read_to_string(exported.folder.join(ADJUSTMENTS_FILE)).expect("the adjustments were written");
    let read = adjustments::parse(&text).expect("the adjustments this build wrote are ones it reads");
    assert_eq!(read.version, ADJUSTMENTS_VERSION);
    assert_eq!(read.documents.keys().collect::<Vec<_>>(), vec!["select.json"], "a document nothing was chosen in was written of");
    let select = &read.documents["select.json"];
    assert_eq!(select.options.get("Theme").map(String::as_str), Some("Light"));
    assert_eq!(select.options.get("Cover").map(String::as_str), Some("Off"));
    assert_eq!(select.files.get("Background"), Some(&adjustments::ChosenFile::File("bg/night.png".to_owned())));
    let judge = select.offsets.get("Judge").expect("the offset is written under its name");
    assert_eq!((judge.x, judge.y), (1.0, JUDGE_Y_STEPS as f32));
    assert!(select.offsets.contains_key("Panel Opacity"));
    for machine_own in [home.to_string_lossy().into_owned(), "901".to_owned(), "902".to_owned()] {
        assert!(!text.contains(&machine_own), "the adjustments carry {machine_own}, which is this machine's or this version's");
    }
    assert!(files_under(&pack).contains(MANIFEST_FILE) && !pack.join(ADJUSTMENTS_FILE).exists(), "the pack itself was changed by exporting it");
}

/// A run that never saw the first one, with a fresh configuration that names the exported folder as
/// its pack, shows every row where the first run left it, hands a loader the same choices, and has
/// nothing of them in its settings.
#[test]
fn a_fresh_run_that_names_the_export_draws_what_was_exported() {
    let home = home("import");
    let pack = write_pack(&home);
    let mut first = Run::naming(&home, "first", &pack);
    let shipped = first.adjusted_lines();
    first.adjust();
    let adjusted = first.adjusted_lines();
    assert_ne!(adjusted, shipped);
    assert_eq!(first.value(SELECT, PANEL), format!("{PANEL_PERCENT}%"));
    let exported = first.export(&outbox(&home)).expect("the pack is exported");

    let mut second = Run::naming(&home, "second", &exported.folder);
    assert_eq!(second.adjusted_lines(), adjusted, "the rows of a run that names the export are not where they were exported");
    assert!(second.config.skin.custom.is_empty(), "what the pack gives was written into the player's own choices");

    second.skins.request_for(&second.config, SELECT);
    let loaded = second.skins.document(SELECT).expect("the exported document reads");
    let option = |name: &str| loaded.selected_options.iter().find(|(row, _)| row == name).map(|(_, op)| *op);
    assert_eq!((option("Theme"), option("Cover")), (Some(902), Some(904)), "a loader was not handed the exported options");
    assert!(loaded.filemap().values().any(|file| file == "night.png"), "a loader was not handed the exported file: {:?}", loaded.filemap());
    let drawn_with = second.skins.customisation_for(&second.config, SELECT).expect("the pack gives the document something");
    let first_drawn_with = first.skins.customisation_for(&first.config, SELECT).expect("the first run chose something");
    assert_eq!(drawn_with.offsets, first_drawn_with.offsets, "a frame of the second run reads other offsets than the first run's");
    assert_eq!(drawn_with.offsets.get(&41).map(|offset| opacity_percent(offset.a)), Some(PANEL_PERCENT as i32));

    rbms_config::save(&second.config, &second.settings).expect("the settings are written");
    let written = std::fs::read_to_string(&second.settings).expect("the settings were written");
    assert!(!written.contains("night") && !written.contains("Light") && !written.contains("902"), "the settings file holds what the pack gave");

    let again = second.export(&outbox(&home)).expect("an export can be exported again");
    assert_eq!(
        std::fs::read_to_string(again.folder.join(ADJUSTMENTS_FILE)).ok(),
        std::fs::read_to_string(exported.folder.join(ADJUSTMENTS_FILE)).ok(),
        "exporting an export lost what it was exported with"
    );
}

/// What the player chooses stands over what the pack gives, row by row, and is what the settings
/// hold. Dropping the choice -- one option's, or the whole document's -- comes back to what the
/// pack gave, which is also where the key that puts an offset back puts it.
#[test]
fn the_players_choice_stands_over_the_packs_and_reset_comes_back_to_the_packs() {
    let home = home("precedence");
    let pack = write_pack(&home);
    let mut first = Run::naming(&home, "first", &pack);
    first.adjust();
    let exported = first.export(&outbox(&home)).expect("the pack is exported");
    let mut run = Run::naming(&home, "second", &exported.folder);
    let given = run.adjusted_lines();
    let document = run.skins.document_path(&run.config, SELECT).expect("the pack draws the browser").to_owned();

    run.step(THEME);
    assert_eq!(run.value(SELECT, THEME), "DARK", "the player's step did not move the option off what the pack gave");
    assert_eq!(run.value(SELECT, COVER), "OFF", "a row the player did not touch left what the pack gave");
    let stored = run.config.skin.customisation(&document).expect("the player's choice is stored");
    assert_eq!(stored.properties.len(), 1, "what the pack gave was stored as the player's own: {stored:?}");
    assert!(stored.filepaths.is_empty() && stored.offsets.is_empty());

    assert!(run.skins.set_number_for(&mut run.config, SELECT, JUDGE_X, 7));
    assert_eq!(run.value(SELECT, JUDGE_X), "+7");
    assert_eq!(run.value(SELECT, JUDGE_Y), format!("+{JUDGE_Y_STEPS}"), "moving one axis of an offset lost where the pack had the other");
    let shipped = run.skins.number_for(&run.config, SELECT, JUDGE_X).expect("the row holds a number").shipped;
    assert!(run.skins.set_number_for(&mut run.config, SELECT, JUDGE_X, shipped));
    assert_eq!(run.value(SELECT, JUDGE_X), "+1", "an offset is not put back to where the pack had it");

    assert!(run.skins.reset_property_for(&mut run.config, SELECT, 0));
    assert_eq!(run.value(SELECT, THEME), "LIGHT", "an option whose choice was dropped is not where the pack put it");

    run.step(COVER);
    run.step(BACKGROUND);
    assert_ne!(run.adjusted_lines(), given);
    assert!(run.skins.forget(&mut run.config), "there was a choice to drop");
    assert_eq!(run.adjusted_lines(), given, "RESET did not come back to what the pack gave");
    assert!(run.config.skin.custom.is_empty());
    assert!(!run.skins.forget(&mut run.config), "what the pack gives was dropped as if it were the player's");
}

/// A configuration changed behind the library's back is settled the next time a frame is prepared,
/// which is when the stored offsets are next read.
#[test]
fn choices_made_behind_the_librarys_back_are_settled_before_the_next_frame() {
    let home = home("settle");
    let pack = write_pack(&home);
    let mut first = Run::naming(&home, "first", &pack);
    first.adjust();
    let exported = first.export(&outbox(&home)).expect("the pack is exported");
    let mut run = Run::naming(&home, "second", &exported.folder);
    let document = run.skins.document_path(&run.config, SELECT).expect("the pack draws the browser").to_owned();

    run.config.skin.customise(&document).properties.insert("Cover".to_owned(), 903);
    run.skins.drop_moved(&run.config);
    assert_eq!(run.value(SELECT, COVER), "ON");
    assert_eq!(run.value(SELECT, THEME), "LIGHT", "settling the player's choice lost what the pack gives the other rows");
    let drawn_with = run.skins.customisation_for(&run.config, SELECT).expect("the document is drawn with something");
    assert_eq!(drawn_with.offsets.get(&40).map(|offset| offset.y), Some(JUDGE_Y_STEPS as f32));
}

/// A pack with adjustments that name what its documents do not declare, written into `home`.
fn pack_with_adjustments(home: &Path, text: &str) -> PathBuf {
    let pack = write_pack(home);
    write_file(&pack.join(ADJUSTMENTS_FILE), text);
    pack
}

/// A line that names something the document does not declare is passed over, and the lines beside
/// it are taken: an unknown document, option, item, file slot, file and offset.
#[test]
fn what_the_adjustments_name_and_a_document_does_not_declare_is_passed_over() {
    let home = home("unknown");
    let text = r#"(
        version: 1,
        documents: {
            "nowhere.json": (options: { "Theme": "Light" }),
            "select.json": (
                options: { "Theme": "Light", "Cover": "Sideways", "Glow": "On" },
                files: { "Background": File("bg/dusk.png"), "Wallpaper": File("bg/night.png") },
                offsets: { "Judge": (y: 5), "Nothing": (x: 9), "Panel Opacity": (a: 100000) },
            ),
            "decide.json": (files: { "Background": Random }, offsets: { "Panel Opacity": (a: -51) }),
        },
    )"#;
    let run = Run::naming(&home, "run", &pack_with_adjustments(&home, text));
    assert_eq!(run.value(SELECT, THEME), "LIGHT");
    assert_eq!(run.value(SELECT, COVER), "ON", "an item the option does not have was taken");
    assert_eq!(run.value(SELECT, BACKGROUND), "day.png", "a file the slot does not offer was taken");
    assert_eq!((run.value(SELECT, JUDGE_X), run.value(SELECT, JUDGE_Y)), ("+0".to_owned(), "+5".to_owned()));
    let select = run.skins.customisation_for(&run.config, SELECT).expect("the pack gives the browser something");
    assert_eq!(select.properties.len(), 1);
    assert!(select.filepaths.is_empty());
    assert_eq!(select.offsets.keys().copied().collect::<Vec<_>>(), vec![40, 41], "an offset no document declares was taken");
    assert_eq!(select.offsets[&41].a, OFFSET_ALPHA_LIMIT, "a figure past what its axis may be nudged was taken as written");
    assert_eq!(run.value(DECIDE, SkinRow::Offset(0, OffsetAxis::A)), "80%");
    assert!(run.skins.customisation_for(&run.config, RESULT).is_none(), "a document the adjustments say nothing of was given something");
}

/// A file that cannot be used gives nothing, leaves the pack drawing as its author shipped it, and
/// is said once: text that is not the layout, a layout newer than this build's, a file past the
/// size that is read, and one past the lines.
#[test]
fn adjustments_that_cannot_be_used_give_nothing_and_are_said_once() {
    let long = format!("(version: 1, documents: {{}})\n{}", "\n".repeat(ADJUSTMENTS_MAX_LINES));
    let large = format!("(version: 1, documents: {{}}){}", " ".repeat(ADJUSTMENTS_MAX_BYTES as usize));
    let newer = format!("(version: {}, documents: {{ \"select.json\": (options: {{ \"Theme\": \"Light\" }}) }})", ADJUSTMENTS_VERSION + 1);
    let broken = [("garbled", "(version: 1, documents: [not a map"), ("newer", newer.as_str()), ("large", large.as_str()), ("long", long.as_str())];
    for (tag, text) in broken {
        let home = home(&format!("broken-{tag}"));
        let mut run = Run::naming(&home, "run", &pack_with_adjustments(&home, text));
        assert_eq!(run.value(SELECT, THEME), "DARK", "{tag}: adjustments that cannot be used were used");
        assert!(run.skins.customisation_for(&run.config, SELECT).is_none(), "{tag}");
        let said = run.skins.take_unreadable();
        assert_eq!(said.len(), 1, "{tag}: {said:?}");
        assert_eq!(said[0].0.file_name().and_then(|name| name.to_str()), Some(ADJUSTMENTS_FILE), "{tag}");
        assert!(!said[0].1.is_empty() && !said[0].1.contains('\n'), "{tag}: {:?}", said[0].1);
        run.skins.rescan(&run.settings, &run.config);
        assert!(run.skins.take_unreadable().is_empty(), "{tag}: a file nobody changed was said twice");
        assert!(run.export(&outbox(&home)).is_ok(), "{tag}: a pack with adjustments that cannot be used cannot be exported");
    }
}

/// An export never writes into a folder that is there: a second export, and one made where a folder
/// of the name already is, go under the next free name and leave what was there as it was.
#[test]
fn an_export_takes_a_free_name_and_leaves_what_is_there_alone() {
    let home = home("names");
    let pack = write_pack(&home);
    let run = Run::naming(&home, "run", &pack);
    let outbox = outbox(&home);
    let taken = outbox.join(format!("pack{EXPORT_SUFFIX}"));
    write_file(&taken.join("mine.txt"), "the player's own");
    let unfinished = outbox.join(format!("pack{EXPORT_SUFFIX}-2{INCOMPLETE_SUFFIX}"));
    write_file(&unfinished.join("half.txt"), "another export's");

    let first = run.export(&outbox).expect("the pack is exported");
    let second = run.export(&outbox).expect("the pack is exported again");
    assert_eq!(first.folder, outbox.join(format!("pack{EXPORT_SUFFIX}-3")), "a name that is taken, or being written under, was not passed over");
    assert_eq!(second.folder, outbox.join(format!("pack{EXPORT_SUFFIX}-4")));
    assert_eq!(files_under(&taken), BTreeSet::from(["mine.txt".to_owned()]), "a folder that was there was written into");
    assert_eq!(files_under(&unfinished), BTreeSet::from(["half.txt".to_owned()]));
    assert_eq!(files_under(&first.folder), files_under(&second.folder));
    assert!(
        entries_of(&outbox).iter().filter(|name| name.ends_with(INCOMPLETE_SUFFIX)).count() == 1,
        "an export left a folder under the name it is written with"
    );
}

/// The pack's own folder, and any folder inside it, is refused as the place to export into, and
/// nothing is made there. A folder that is not there is refused too.
#[test]
fn an_export_into_the_pack_itself_is_refused() {
    let home = home("inside");
    let pack = write_pack(&home);
    let run = Run::naming(&home, "run", &pack);
    let before = files_under(&pack);
    for inside in [pack.clone(), pack.join("bg"), pack.join("lua").join("parts")] {
        assert!(matches!(run.export(&inside), Err(ExportError::InsidePack)), "an export into {} was not refused", inside.display());
    }
    assert!(matches!(run.export(&home.join("not-there")), Err(ExportError::NoDestination(_))));
    assert_eq!(files_under(&pack), before, "a refused export wrote into the pack");
    assert_eq!(entries_of(&home), BTreeSet::from(["pack".to_owned(), "run".to_owned()]), "a refused export left something behind");

    let beside = run.export(&home).expect("the folder the pack is in can be exported into");
    assert_eq!(beside.folder, home.join(format!("pack{EXPORT_SUFFIX}")));
}

/// A pack past what one export writes is refused before anything is written, by its size and by how
/// many files it holds, and so is an export that was called off.
#[test]
fn a_pack_past_the_ceiling_and_an_export_called_off_write_nothing() {
    let home = home("ceiling");
    let pack = write_pack(&home);
    let run = Run::naming(&home, "run", &pack);
    let outbox = outbox(&home);

    let small = ExportRequest { limits: ExportLimits { bytes: 16, ..ExportLimits::default() }, ..run.request(&outbox) };
    let refused = export_pack(&small, &ExportProgress::default());
    assert!(matches!(refused, Err(ExportError::TooLarge { limit: 16, .. })), "{refused:?}");
    let few = ExportRequest { limits: ExportLimits { files: 3, ..ExportLimits::default() }, ..run.request(&outbox) };
    let refused = export_pack(&few, &ExportProgress::default());
    assert!(matches!(refused, Err(ExportError::TooManyFiles { limit: 3 })), "{refused:?}");

    let progress = ExportProgress::default();
    progress.call_off();
    assert!(matches!(export_pack(&run.request(&outbox), &progress), Err(ExportError::CalledOff)));
    assert!(entries_of(&outbox).is_empty(), "an export that was refused or called off left something behind: {:?}", entries_of(&outbox));
    for said in [ExportError::TooLarge { bytes: 1, limit: 16 }.to_string(), ExportError::InsidePack.to_string(), ExportError::CalledOff.to_string()] {
        assert!(!said.is_empty() && !said.contains('\n'));
    }
}

/// An export that fails part of the way takes away what it wrote: nothing is left under the final
/// name or under the name a folder is written with.
#[cfg(unix)]
#[test]
fn an_export_that_fails_part_of_the_way_leaves_nothing_behind() {
    use std::os::unix::fs::PermissionsExt;
    let home = home("fails");
    let pack = write_pack(&home);
    let run = Run::naming(&home, "run", &pack);
    let outbox = outbox(&home);
    let locked = pack.join("sound/open.wav");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("the fixture file can be locked");
    if std::fs::File::open(&locked).is_ok() {
        return;
    }
    let failed = run.export(&outbox);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).expect("the fixture file can be unlocked");
    assert!(matches!(&failed, Err(ExportError::Io { path, .. }) if path.ends_with("sound/open.wav")), "{failed:?}");
    assert!(entries_of(&outbox).is_empty(), "a failed export left {:?}", entries_of(&outbox));
    assert!(run.export(&outbox).is_ok(), "the name a failed export had claimed was not given back");
}

/// A link inside the pack is never followed out of it: one that leads to a file of the pack is
/// exported as that file, and one that leads out of the pack, or to a folder, is left out and
/// counted. An adjustments file that is a link is not read.
#[cfg(unix)]
#[test]
fn links_are_not_followed_out_of_the_pack() {
    use std::os::unix::fs::symlink;
    let home = home("links");
    let pack = write_pack(&home);
    write_file(&home.join("secret.txt"), "not the pack's");
    write_file(&home.join("elsewhere/inner.txt"), "not the pack's either");
    symlink(home.join("secret.txt"), pack.join("bg/secret.png")).expect("a link can be made");
    symlink(home.join("elsewhere"), pack.join("elsewhere")).expect("a link can be made");
    symlink(pack.join("lua"), pack.join("lua-again")).expect("a link can be made");
    symlink(pack.join("bg/day.png"), pack.join("bg/noon.png")).expect("a link can be made");
    symlink(home.join("gone.txt"), pack.join("dangling.txt")).expect("a link can be made");
    write_file(&home.join("adjustments.ron"), r#"(version: 1, documents: { "select.json": (options: { "Theme": "Light" }) })"#);
    symlink(home.join("adjustments.ron"), pack.join(ADJUSTMENTS_FILE)).expect("a link can be made");

    let mut run = Run::naming(&home, "run", &pack);
    assert_eq!(run.value(SELECT, THEME), "DARK", "adjustments behind a link were read");
    assert_eq!(run.skins.take_unreadable().len(), 1);

    let exported = run.export(&outbox(&home)).expect("the pack is exported");
    let files = files_under(&exported.folder);
    assert_eq!(exported.links_skipped, 4, "a link out of the pack, to a folder, or to nothing was not counted");
    assert!(files.contains("bg/noon.png"), "a link to a file of the pack was left out");
    assert_eq!(std::fs::read_to_string(exported.folder.join("bg/noon.png")).ok().as_deref(), Some("day"));
    assert!(std::fs::symlink_metadata(exported.folder.join("bg/noon.png")).is_ok_and(|found| found.file_type().is_file()), "a link was exported as a link");
    for outside in ["bg/secret.png", "elsewhere/inner.txt", "lua-again/parts/util.lua", "dangling.txt"] {
        assert!(!files.contains(outside), "{outside} was exported through a link");
    }
    let written = std::fs::read_to_string(exported.folder.join(ADJUSTMENTS_FILE)).expect("the export has adjustments of its own");
    assert!(!written.contains("Light"), "the adjustments behind the link were copied");
}

/// The default pack as it is installed is exported like any other, without the installer's record,
/// and the export draws every screen the installed pack draws.
#[test]
fn the_installed_default_pack_is_exported_without_the_installers_record() {
    let home = home("default");
    let settings = home.join("settings.ron");
    assert!(install_default_pack(&settings).is_complete(), "the default pack did not install");
    let mut config = Config::default();
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan(&settings, &config);
    assert!(skins.can_export(), "the default pack is not a pack to export");
    let drawn: Vec<i32> = skins.pack_documents.keys().copied().collect();
    assert!(!drawn.is_empty());
    let play = *drawn.first().expect("the default pack draws a screen");
    let row = skins.rows_for(&config, play).into_iter().find(|row| matches!(row, SkinRow::Offset(_, _))).expect("a play skin has an offset");
    assert!(skins.step_for(&mut config, play, row, 1));
    let moved = skins.line_for(&config, play, row);

    let request = skins.export_request(&config, &outbox(&home)).expect("there is a pack to export").expect("the adjustments are written out");
    let exported = export_pack(&request, &ExportProgress::default()).expect("the default pack is exported");
    let files = files_under(&exported.folder);
    assert!(!files.contains(MANIFEST_FILE), "the installer's record was exported");
    assert_eq!(files.len(), DEFAULT_PACK_FILES.len() + 1, "the export is not every shipped file and the adjustments");
    assert_eq!(exported.folder.file_name().and_then(|name| name.to_str()), Some(format!("rbms-default{EXPORT_SUFFIX}").as_str()));

    let mut second = Run::naming(&home, "second", &exported.folder);
    assert_eq!(second.skins.pack_documents.keys().copied().collect::<Vec<_>>(), drawn, "the export does not draw the screens the pack drew");
    assert_eq!(second.skins.line_for(&second.config, play, row), moved, "the export does not carry the choice made in the default pack");
    assert!(second.skins.take_unreadable().is_empty(), "a document of the export could not be read");
}

/// The two rows every document of the shipped pack declares, the colours it is drawn in, and the
/// item each is stepped to from the one it ships on.
const SHIPPED_COLOUR_ROWS: [(&str, &str); 2] = [("THEME", "SLATE"), ("ACCENT", "SKY")];

/// The colours of the shipped pack are set for all of it from any one screen: its theme and its
/// accent, stepped on one screen, go onto every other screen the pack draws with one ask each, and
/// an export carries them, so a fresh run that names the export reads every screen in the colours
/// that were chosen with nothing of them in its own settings.
#[test]
fn the_colours_of_the_shipped_pack_go_onto_every_screen_and_into_an_export() {
    let home = home("shipped-colours");
    let settings = home.join("settings.ron");
    assert!(install_default_pack(&settings).is_complete(), "the default pack did not install");
    let mut config = Config::default();
    let mut skins = SkinLibrary::new(&settings, &config);
    skins.rescan(&settings, &config);
    let drawn: Vec<i32> = skins.pack_documents.keys().copied().collect();
    let from = *drawn.first().expect("the default pack draws a screen");
    let row_named = |skins: &SkinLibrary, config: &Config, screen: i32, name: &str| {
        let found = skins.rows_for(config, screen).into_iter().find(|row| skins.line_for(config, screen, *row).0 == name);
        found.unwrap_or_else(|| panic!("screen {screen} of the default pack has no row {name}"))
    };

    for (name, chosen) in SHIPPED_COLOUR_ROWS {
        let row = row_named(&skins, &config, from, name);
        assert_eq!(skins.peers_for(&config, from, row).len(), drawn.len() - 1, "{name} is not a row of every screen of the pack");
        assert!(skins.step_for(&mut config, from, row, 1));
        assert_eq!(skins.line_for(&config, from, row).1, chosen, "{name} does not step to {chosen}");
        let applied = skins.apply_to_peers(&mut config, from, row).expect("the other screens declare the row");
        assert_eq!((applied.applied, applied.skipped), (drawn.len() - 1, 0), "{name}");
    }

    let request = skins.export_request(&config, &outbox(&home)).expect("there is a pack to export").expect("the adjustments are written out");
    let exported = export_pack(&request, &ExportProgress::default()).expect("the default pack is exported");
    let second = Run::naming(&home, "second", &exported.folder);
    assert!(second.config.skin.custom.is_empty(), "what the pack gives was written into the player's own choices");
    for screen in drawn {
        for (name, chosen) in SHIPPED_COLOUR_ROWS {
            let row = row_named(&second.skins, &second.config, screen, name);
            assert_eq!(second.value(screen, row), chosen, "screen {screen} of the export does not read {name} as it was exported");
        }
        let given = second.skins.customisation_for(&second.config, screen).expect("the export gives every screen its colours");
        assert_eq!(given.properties.len(), SHIPPED_COLOUR_ROWS.len(), "screen {screen} is handed something other than the two rows that were set");
    }
}

/// A row other screens declare under the same name says how many do, and its value is put onto them
/// when asked and not before: an option onto the item of the same name, whatever number each
/// document gives it, and an offset along the axis the row is of. A screen whose option has no item
/// of that name is passed over and counted.
#[test]
fn a_rows_value_is_put_onto_the_other_screens_that_declare_it() {
    let home = home("peers");
    let pack = write_pack(&home);
    let mut run = Run::naming(&home, "run", &pack);
    assert_eq!(run.skins.peers_for(&run.config, SELECT, THEME), vec![DECIDE, RESULT]);
    assert_eq!(run.skins.peers_for(&run.config, SELECT, COVER), Vec::<i32>::new(), "an option of this screen alone has peers");
    assert_eq!(run.skins.peers_for(&run.config, SELECT, BACKGROUND), Vec::<i32>::new(), "a file slot has peers");
    assert_eq!(run.skins.peers_for(&run.config, SELECT, JUDGE_X), vec![RESULT]);
    assert_eq!(run.skins.peers_for(&run.config, SELECT, JUDGE_Y), Vec::<i32>::new(), "an offset that cannot be moved along the row's axis is a peer");
    assert_eq!(run.skins.peers_for(&run.config, SELECT, PANEL), vec![DECIDE]);
    assert_eq!(run.skins.apply_to_peers(&mut run.config, SELECT, COVER), None);

    run.step(THEME);
    assert_eq!(run.value(DECIDE, THEME), "DARK", "stepping one screen's option moved another's");
    let applied = run.skins.apply_to_peers(&mut run.config, SELECT, THEME).expect("other screens declare the option");
    assert_eq!((applied.applied, applied.skipped), (1, 1));
    assert_eq!(applied.message(), "THEME: LIGHT applied to 1 other screen (1 skipped: no item of that name)");
    assert_eq!(run.value(DECIDE, THEME), "LIGHT");
    assert_eq!(run.value(RESULT, THEME), "DARK", "a screen with no item of that name was changed");
    let decide = run.skins.document_path(&run.config, DECIDE).expect("the pack draws the decide screen").to_owned();
    assert_eq!(
        run.config.skin.customisation(&decide).and_then(|stored| stored.properties.get("Theme")),
        Some(&912),
        "the item was stored by this screen's number"
    );
    assert!(run.skins.needs_reload_for(&run.config, DECIDE), "a screen whose option moved is not read again");

    assert!(run.skins.set_number_for(&mut run.config, SELECT, PANEL, PANEL_PERCENT));
    assert!(run.skins.set_number_for(&mut run.config, SELECT, JUDGE_X, -4));
    let panel = run.skins.apply_to_peers(&mut run.config, SELECT, PANEL).expect("another screen declares the opacity");
    assert_eq!((panel.applied, panel.skipped), (1, 0));
    assert_eq!(panel.message(), format!("PANEL OPACITY: {PANEL_PERCENT}% applied to 1 other screen"));
    assert_eq!(run.value(DECIDE, SkinRow::Offset(0, OffsetAxis::A)), format!("{PANEL_PERCENT}%"));
    assert!(run.skins.apply_to_peers(&mut run.config, SELECT, JUDGE_X).is_some());
    assert_eq!(run.value(RESULT, SkinRow::Offset(0, OffsetAxis::X)), "-4");

    let exported = run.export(&outbox(&home)).expect("the pack is exported");
    let second = Run::naming(&home, "second", &exported.folder);
    assert_eq!(second.value(DECIDE, THEME), "LIGHT", "a value put onto another screen was not exported with it");
    assert_eq!(second.value(RESULT, SkinRow::Offset(0, OffsetAxis::X)), "-4");
}

/// A settings file written before packs had adjustments reads as it did, and what it holds for a
/// document stands over whatever the pack now gives that document.
#[test]
fn a_settings_file_from_before_adjustments_reads_as_it_did() {
    let home = home("settings");
    let pack = write_pack(&home);
    let mut first = Run::naming(&home, "first", &pack);
    first.adjust();
    let exported = first.export(&outbox(&home)).expect("the pack is exported");

    let mut run = Run::naming(&home, "second", &exported.folder);
    let document = run.skins.document_path(&run.config, SELECT).expect("the pack draws the browser").to_owned();
    let old = format!(
        "(schema_version: {}, skin: (pack: Some({:?}), screen: 5, custom: {{ {:?}: (properties: {{ \"Cover\": 903 }}, offsets: {{ 41: (x: 0.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, a: -25.0) }}) }}))",
        rbms_config::CURRENT_SCHEMA_VERSION,
        exported.folder.to_string_lossy(),
        document
    );
    std::fs::write(&run.settings, old).expect("the old settings file is written");
    run.config = rbms_config::load(&run.settings).expect("a settings file in the layout it always had reads").config;
    assert_eq!(run.config.skin.customisation(&document).map(|stored| stored.properties.len()), Some(1));
    run.skins = SkinLibrary::new(&run.settings, &run.config);
    run.skins.rescan(&run.settings, &run.config);
    assert_eq!(run.value(SELECT, COVER), "ON", "what the settings hold did not stand over what the pack gives");
    assert_eq!(run.value(SELECT, PANEL), "90%");
    assert_eq!(run.value(SELECT, THEME), "LIGHT", "a row the settings say nothing of is not where the pack put it");

    let before = run.config.skin.clone();
    rbms_config::save(&run.config, &run.settings).expect("the settings are written");
    let read_back = rbms_config::load(&run.settings).expect("the settings read back").config;
    assert_eq!(read_back.skin, before, "the skin group did not survive the file");
}

/// An export that runs beside the frame loop comes to what one made here and now comes to, and says
/// how much it wrote.
#[test]
fn an_export_run_beside_the_frame_loop_writes_the_same_folder() {
    let home = home("job");
    let pack = write_pack(&home);
    let mut run = Run::naming(&home, "run", &pack);
    run.adjust();
    let here = run.export(&outbox(&home)).expect("the pack is exported");
    let job = ExportJob::begin(run.request(&outbox(&home)));
    let beside = job.wait().expect("the pack is exported beside the frame loop");
    assert_ne!(beside.folder, here.folder);
    assert_eq!(files_under(&beside.folder), files_under(&here.folder));
    assert_eq!((beside.files, beside.bytes), (here.files, here.bytes));
}

/// With [`SKIN_PACK_ENV`] and [`EXPORT_DIR_ENV`] both set, export the pack the environment names
/// into the folder it names, and print what that came to. The pack is read and never written.
#[test]
fn the_pack_the_environment_names_is_exported_where_the_environment_says() {
    let (Some(pack), Some(folder)) = (pack_from_environment(std::env::var_os(SKIN_PACK_ENV)), std::env::var_os(EXPORT_DIR_ENV).map(PathBuf::from)) else {
        return;
    };
    std::fs::create_dir_all(&folder).expect("the folder to export into can be made");
    let home = home("environment");
    let began = std::time::Instant::now();
    let run = Run::naming(&home, "run", &pack);
    let listed = began.elapsed();
    let exported = run.export(&folder).expect("the pack the environment names is exported");
    println!(
        "exported {} files, {} bytes, {} links left out, to {} in {:?} (the pack's headers took {:?})",
        exported.files,
        exported.bytes,
        exported.links_skipped,
        exported.folder.display(),
        began.elapsed() - listed,
        listed
    );
    let second = Run::naming(&home, "second", &exported.folder);
    assert_eq!(
        second.skins.pack_documents.keys().collect::<Vec<_>>(),
        run.skins.pack_documents.keys().collect::<Vec<_>>(),
        "the export does not draw the screens the pack draws"
    );
}
