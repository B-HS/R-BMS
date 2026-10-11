//! The default pack as it is carried and as it is installed: the table is the folder it was built
//! from, a first run writes all of it, a later run writes nothing, and a run after an update
//! replaces what an earlier build wrote without touching what the player changed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use sha2::{Digest, Sha256};

use super::*;

/// The pack's folder in the repository, from this package's own.
const PACK_SOURCE: &str = "../../assets/skins/rbms-default";

/// The folder of the pack that holds what its images and sounds are generated with, which no player
/// is given.
const TOOLS_FOLDER: &str = "tools";

/// The extension of the pack's skin documents.
const DOCUMENT_EXTENSION: &str = "luaskin";

/// The font the renderer carries too, as the pack names it.
const SHARED_FONT: &str = "font/Inter-Regular.ttf";

/// The most the binary may carry of the pack before shipping it as a folder has to be considered.
const EMBEDDED_BUDGET_BYTES: usize = 10 * 1024 * 1024;

/// The skin documents the pack ships: the browser, the decide scene, play in each of five key
/// modes, and the two result screens.
const SHIPPED_DOCUMENTS: usize = 9;

fn temp_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("rbms-default-pack-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create the fixture directory");
    home
}

/// One shipped file made up by a test, with the digest the build script would have given it.
fn shipped(name: &'static str, bytes: &'static [u8]) -> EmbeddedFile {
    EmbeddedFile { name, digest: hex(&Sha256::digest(bytes)).leak(), bytes }
}

/// Every file under `root`, by its path below it with `/` between its parts, with its bytes.
fn files_under(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the folder lists").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path.strip_prefix(root).expect("the file is below the folder");
            let name: Vec<String> = relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
            found.insert(name.join("/"), std::fs::read(&path).expect("the file reads"));
        }
    }
    found
}

/// When every file under `root` was last written, by its path.
fn stamps_under(root: &Path) -> BTreeMap<PathBuf, (SystemTime, u64)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the folder lists").flatten() {
            let path = entry.path();
            let metadata = entry.metadata().expect("the entry has metadata");
            if metadata.is_dir() {
                pending.push(path.clone());
            }
            found.insert(path, (metadata.modified().expect("the file system keeps a write time"), metadata.len()));
        }
    }
    found
}

/// The table carried in the binary is the pack's folder: every document and every file of its four
/// shipped folders, byte for byte, in the byte order of their names, and nothing of the scripts the
/// pack is generated with. A file added to the pack and not shipped, or shipped and since removed,
/// fails here whatever the build script was last run against.
#[test]
fn the_embedded_table_is_the_packs_folder_without_its_tools() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(PACK_SOURCE);
    let on_disk: BTreeMap<String, Vec<u8>> = files_under(&source)
        .into_iter()
        .filter(|(name, _)| !name.starts_with(&format!("{TOOLS_FOLDER}/")) && !name.rsplit('/').next().is_some_and(|file| file.starts_with('.')))
        .collect();
    let names: Vec<&str> = DEFAULT_PACK_FILES.iter().map(|file| file.name).collect();
    assert_eq!(names, on_disk.keys().map(String::as_str).collect::<Vec<_>>(), "the table and the folder name different files, or in a different order");
    assert!(names.is_sorted(), "the table is not in the byte order of its names");
    for file in DEFAULT_PACK_FILES {
        assert!(file.bytes == on_disk[file.name].as_slice(), "{} is not the file in the folder", file.name);
        assert_eq!(file.digest, hex(&Sha256::digest(file.bytes)), "{} carries a digest of something else", file.name);
        assert!(is_pack_relative(file.name) && is_digest(file.digest), "{} would not survive its own record", file.name);
    }
    let documents = names.iter().filter(|name| !name.contains('/')).count();
    assert_eq!(documents, SHIPPED_DOCUMENTS, "the pack ships a different number of documents");
    assert!(names.iter().filter(|name| !name.contains('/')).all(|name| name.ends_with(DOCUMENT_EXTENSION)), "something other than a document is at the top");
    assert!(!names.contains(&MANIFEST_FILE), "the pack ships a file under the name its record is kept under");
}

/// The font the pack names is the one the renderer already carries, not a second copy of it: the
/// table points at the renderer's bytes. And the whole table stays far inside what a binary may
/// carry.
#[test]
fn the_pack_carries_the_renderers_font_once_and_stays_inside_its_budget() {
    let font = DEFAULT_PACK_FILES.iter().find(|file| file.name == SHARED_FONT).expect("the pack ships its font");
    assert!(std::ptr::eq(font.bytes, rbms_render::font::BUNDLED_FONT.as_slice()), "the pack carries a copy of the font the renderer already has");

    let carried: usize = DEFAULT_PACK_FILES.iter().map(|file| file.bytes.len()).sum();
    let added = carried - font.bytes.len();
    println!("default pack: {} files, {carried} bytes carried, {added} of them only for the pack", DEFAULT_PACK_FILES.len());
    assert!(carried <= EMBEDDED_BUDGET_BYTES, "the pack is {carried} bytes, past the {EMBEDDED_BUDGET_BYTES} a binary carries");
}

/// The pack ships one sound for every cue the player raises, under the name the cue is looked up
/// by, and nothing else in its sound folder.
#[test]
fn the_pack_ships_one_sound_for_each_cue_the_player_raises() {
    let prefix = format!("{DEFAULT_SOUND_DIRECTORY}/");
    let sounds: Vec<&str> = DEFAULT_PACK_FILES.iter().filter_map(|file| file.name.strip_prefix(&prefix)).collect();
    assert_eq!(sounds.len(), crate::syssound::SYSTEM_SOUND_COUNT, "the shipped set and the set the player raises are different sizes");
    for sound in crate::syssound::SystemSound::ALL {
        let shipped = sounds.iter().any(|name| Path::new(name).file_stem().and_then(|stem| stem.to_str()) == Some(sound.file_stem()));
        assert!(shipped, "no sound is shipped for {sound:?}");
    }
}

/// A first run on an empty configuration folder writes the whole pack below the default skin
/// folder, with a record of what it wrote, and writes nothing else: no settings file, nothing left
/// under the name a file is written with.
#[test]
fn a_first_run_installs_every_shipped_file_below_the_default_skin_folder() {
    let home = temp_home("first-run");
    let settings = home.join(".config/rbms/settings.ron");
    assert_eq!(installed_default_pack(&settings), None, "a pack nothing was installed into was named");
    assert_eq!(default_sound_folder(&settings), None, "a folder nothing was installed into was named as the sound set");

    let began = Instant::now();
    let first = install_default_pack(&settings);
    println!("default pack: first install took {:?}", began.elapsed());
    assert_eq!(first, PackInstall { written: DEFAULT_PACK_FILES.len(), ..PackInstall::default() });

    let pack = home.join(".config/rbms").join(DEFAULT_SKIN_FOLDER).join(DEFAULT_SKIN_DIRECTORY);
    assert_eq!(installed_default_pack(&settings), Some(pack.clone()));
    assert_eq!(default_sound_folder(&settings), Some(pack.join(DEFAULT_SOUND_DIRECTORY)));
    let mut installed = files_under(&pack);
    let record = installed.remove(MANIFEST_FILE).expect("the record of what was installed is kept");
    assert_eq!(installed.len(), DEFAULT_PACK_FILES.len(), "the install wrote something that is not a shipped file");
    for file in DEFAULT_PACK_FILES {
        assert!(installed.get(file.name).is_some_and(|bytes| bytes.as_slice() == file.bytes), "{} is not the file that was shipped", file.name);
    }
    let recorded = parse_manifest(&String::from_utf8(record).expect("the record is text"));
    assert_eq!(recorded.len(), DEFAULT_PACK_FILES.len());
    assert!(DEFAULT_PACK_FILES.iter().all(|file| recorded.get(file.name).map(String::as_str) == Some(file.digest)), "the record names another build's files");
    assert!(!settings.exists(), "installing the pack wrote a settings file");
    let _ = std::fs::remove_dir_all(&home);
}

/// A run that finds the pack it ships already installed writes nothing: no file, not the record,
/// not a folder. Every file is as it was last written.
#[test]
fn a_later_run_of_the_same_build_writes_nothing() {
    let home = temp_home("rerun");
    let settings = home.join("settings.ron");
    assert!(install_default_pack(&settings).is_complete());
    let before = stamps_under(&home);

    let began = Instant::now();
    let again = install_default_pack(&settings);
    println!("default pack: a run with nothing to install took {:?}", began.elapsed());
    assert_eq!(again, PackInstall::default(), "a run with nothing to do did something");
    assert_eq!(stamps_under(&home), before, "a run with nothing to do wrote to the folder");
    let _ = std::fs::remove_dir_all(&home);
}

/// A later run repairs the pack without writing over anything: a file the player replaced stays
/// replaced, one they deleted comes back, and everything else under the skin folder -- their own
/// documents, a folder an earlier build installed -- is left exactly as it was.
#[test]
fn a_later_run_repairs_the_pack_without_writing_over_the_players_files() {
    let home = temp_home("repair");
    let settings = home.join(".config/rbms/settings.ron");
    let skin_folder = home.join(".config/rbms").join(DEFAULT_SKIN_FOLDER);
    let user_document = skin_folder.join("mine/select.json5");
    let earlier_install = skin_folder.join("earlier-bundle/theme.ron");
    for (path, text) in [(&user_document, "user document"), (&earlier_install, "earlier install")] {
        std::fs::create_dir_all(path.parent().expect("the fixture has a folder")).expect("create the fixture folder");
        std::fs::write(path, text).expect("write the fixture");
    }
    assert!(install_default_pack(&settings).is_complete());

    let pack = installed_default_pack(&settings).expect("the pack was installed");
    let replaced = pack.join(DEFAULT_PACK_FILES[0].name);
    let deleted = pack.join(DEFAULT_PACK_FILES[1].name);
    let added = pack.join("lua/mine.lua");
    std::fs::write(&replaced, "the player's own file").expect("replace a shipped file");
    std::fs::remove_file(&deleted).expect("delete a shipped file");
    std::fs::write(&added, "the player's own module").expect("add a file to the pack");

    let repaired = install_default_pack(&settings);
    assert_eq!(repaired, PackInstall { written: 1, ..PackInstall::default() }, "only the deleted file was to be written");
    assert_eq!(std::fs::read_to_string(&replaced).expect("read the replaced file"), "the player's own file", "a replaced file was written over");
    assert!(std::fs::read(&deleted).expect("read the repaired file") == DEFAULT_PACK_FILES[1].bytes, "a deleted file did not come back");
    assert_eq!(std::fs::read_to_string(&added).expect("read the added file"), "the player's own module");
    assert_eq!(std::fs::read_to_string(&user_document).expect("read the user document"), "user document");
    assert_eq!(std::fs::read_to_string(&earlier_install).expect("read the earlier install"), "earlier install");
    let _ = std::fs::remove_dir_all(&home);
}

/// A build that ships a file differently replaces the one an earlier build wrote, as long as it is
/// still the one that build wrote. A file the player changed in between is kept, on that run and on
/// every run after it; a file the earlier build wrote and this one no longer ships is taken away;
/// and a file the player put under a name neither build ships is not touched.
#[test]
fn an_update_replaces_what_an_earlier_build_wrote_and_keeps_what_the_player_changed() {
    let home = temp_home("update");
    let pack = home.join("pack");
    let earlier = [
        shipped("select.luaskin", b"select, first"),
        shipped("lua/kept.lua", b"module, first"),
        shipped("lua/same.lua", b"module that does not change"),
        shipped("lua/retired.lua", b"module the next build drops"),
        shipped("lua/retired-edited.lua", b"module the next build drops, edited by the player"),
    ];
    assert_eq!(install_pack(&pack, &earlier), PackInstall { written: earlier.len(), ..PackInstall::default() });
    std::fs::write(pack.join("lua/kept.lua"), "the player's edit").expect("edit an installed file");
    std::fs::write(pack.join("lua/retired-edited.lua"), "the player's edit").expect("edit an installed file");
    std::fs::write(pack.join("lua/own.lua"), "the player's own").expect("add a file");

    let later = [
        shipped("select.luaskin", b"select, second"),
        shipped("lua/kept.lua", b"module, second"),
        shipped("lua/same.lua", b"module that does not change"),
        shipped("lua/new.lua", b"module the earlier build did not have"),
    ];
    let update = install_pack(&pack, &later);
    assert_eq!(update, PackInstall { written: 2, removed: 1, kept: 1, failed: 0 });
    let installed = files_under(&pack);
    let text = |name: &str| installed.get(name).map(|bytes| String::from_utf8_lossy(bytes).into_owned());
    assert_eq!(text("select.luaskin").as_deref(), Some("select, second"), "a file nobody touched was not brought up to date");
    assert_eq!(text("lua/kept.lua").as_deref(), Some("the player's edit"), "a file the player edited was written over");
    assert_eq!(text("lua/new.lua").as_deref(), Some("module the earlier build did not have"));
    assert_eq!(text("lua/retired.lua"), None, "a file this build does not ship, and nobody touched, was left behind");
    assert_eq!(text("lua/retired-edited.lua").as_deref(), Some("the player's edit"), "a file the player edited was removed");
    assert_eq!(text("lua/own.lua").as_deref(), Some("the player's own"));
    assert!(installed.keys().all(|name| !name.ends_with(PARTIAL_SUFFIX)), "a half-written file was left behind: {:?}", installed.keys());

    let settled = install_pack(&pack, &later);
    assert_eq!(settled, PackInstall { kept: 1, ..PackInstall::default() }, "the edited file is kept on every run and nothing else is touched");
    assert_eq!(std::fs::read_to_string(pack.join("lua/kept.lua")).expect("read the edited file"), "the player's edit");

    std::fs::write(pack.join("lua/kept.lua"), "module, first").expect("put the earlier build's file back");
    assert_eq!(
        install_pack(&pack, &later),
        PackInstall { written: 1, ..PackInstall::default() },
        "a file put back as it was installed is the installer's again"
    );
    assert_eq!(std::fs::read_to_string(pack.join("lua/kept.lua")).expect("read the updated file"), "module, second");
    let _ = std::fs::remove_dir_all(&home);
}

/// Files an earlier build installed before any record was kept -- the sound set was -- are taken
/// for this build's when they are the ones it ships, so the first update after that still reaches
/// them; and a file that was there and is not this build's is the player's.
#[test]
fn files_installed_before_a_record_was_kept_are_recognised_by_what_they_hold() {
    let home = temp_home("unrecorded");
    let pack = home.join("pack");
    std::fs::create_dir_all(pack.join("sound")).expect("create the sound folder");
    std::fs::write(pack.join("sound/clear.wav"), b"clear, first").expect("write the earlier build's sound");
    std::fs::write(pack.join("sound/fail.wav"), b"the player's own").expect("write the player's sound");

    let first = [shipped("sound/clear.wav", b"clear, first"), shipped("sound/fail.wav", b"fail, first")];
    assert_eq!(install_pack(&pack, &first), PackInstall { kept: 1, ..PackInstall::default() });
    let second = [shipped("sound/clear.wav", b"clear, second"), shipped("sound/fail.wav", b"fail, second")];
    assert_eq!(install_pack(&pack, &second), PackInstall { written: 1, kept: 1, ..PackInstall::default() });
    assert_eq!(std::fs::read(pack.join("sound/clear.wav")).expect("read the updated sound"), b"clear, second");
    assert_eq!(std::fs::read(pack.join("sound/fail.wav")).expect("read the player's sound"), b"the player's own");
    let _ = std::fs::remove_dir_all(&home);
}

/// The record is a file the player can edit. A line that names somewhere outside the pack, or
/// carries something that is not a digest, is not acted on: nothing outside the folder is removed
/// whatever the record says was installed there.
#[test]
fn a_record_that_names_a_file_outside_the_pack_removes_nothing() {
    let home = temp_home("record");
    let pack = home.join("pack");
    let outside = home.join("outside.txt");
    std::fs::write(&outside, b"not the installer's").expect("write a file outside the pack");
    let digest = hex(&Sha256::digest(b"not the installer's"));
    std::fs::create_dir_all(&pack).expect("create the pack folder");
    let lines = [
        MANIFEST_HEADER.to_owned(),
        format!("{digest} ../outside.txt"),
        format!("{digest} {}", outside.display()),
        format!("{digest} lua/../../outside.txt"),
        format!("{digest} lua\\..\\..\\outside.txt"),
        "not-a-digest lua/x.lua".to_owned(),
        digest.clone(),
    ];
    std::fs::write(pack.join(MANIFEST_FILE), lines.join("\n")).expect("write the record");
    assert!(parse_manifest(&lines.join("\n")).is_empty(), "a line that should not be believed was");

    assert_eq!(install_pack(&pack, &[shipped("select.luaskin", b"select")]), PackInstall { written: 1, ..PackInstall::default() });
    assert_eq!(std::fs::read(&outside).expect("read the file outside the pack"), b"not the installer's", "the record reached outside the pack");

    assert!(parse_manifest("another format 9\nabc x").is_empty(), "a record of another format was read");
    let oversized = format!("{MANIFEST_HEADER}\n{}", "x".repeat(MANIFEST_MAX_BYTES as usize));
    std::fs::write(pack.join(MANIFEST_FILE), oversized).expect("write an oversized record");
    assert_eq!(read_manifest(&pack.join(MANIFEST_FILE)), None, "a record longer than any this build writes was read");
    let _ = std::fs::remove_dir_all(&home);
}

/// A skin folder that cannot be made leaves nothing installed and nothing named, takes nothing out
/// of the way, and does not stop the run; the next run installs the pack once the folder can be
/// made.
#[test]
fn installation_fails_quietly_and_is_retried_when_the_skin_folder_cannot_be_made() {
    let home = temp_home("blocked");
    let settings = home.join("settings.ron");
    let blocked = home.join(DEFAULT_SKIN_FOLDER);
    std::fs::write(&blocked, "not a folder").expect("write the blocking file");

    let failed = install_default_pack(&settings);
    assert_eq!(failed, PackInstall { failed: DEFAULT_PACK_FILES.len(), ..PackInstall::default() });
    assert!(!failed.is_complete());
    assert_eq!(installed_default_pack(&settings), None, "a pack that could not be installed was named anyway");
    assert_eq!(default_sound_folder(&settings), None, "a set that could not be installed was named anyway");
    assert_eq!(std::fs::read_to_string(&blocked).expect("read the blocking file"), "not a folder", "the install removed what was in its way");

    std::fs::remove_file(&blocked).expect("remove the blocking file");
    assert!(install_default_pack(&settings).is_complete());
    assert!(installed_default_pack(&settings).is_some());
    let _ = std::fs::remove_dir_all(&home);
}

/// A configuration folder the player may read and not write: nothing is installed, nothing panics,
/// and the run is told how much could not be written. Skipped where the folder turns out to be
/// writable all the same, which is what an account that may write anywhere finds.
#[cfg(unix)]
#[test]
fn a_read_only_configuration_folder_leaves_the_pack_out_without_stopping_the_run() {
    use std::os::unix::fs::PermissionsExt;

    const READ_ONLY: u32 = 0o555;
    const WRITABLE: u32 = 0o755;

    let home = temp_home("read-only");
    let settings = home.join("settings.ron");
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(READ_ONLY)).expect("the fixture folder is the test's own");
    let writable_anyway = std::fs::write(home.join("probe"), b"").is_ok();
    if !writable_anyway {
        let failed = install_default_pack(&settings);
        assert_eq!(failed, PackInstall { failed: DEFAULT_PACK_FILES.len(), ..PackInstall::default() });
        assert_eq!(installed_default_pack(&settings), None);
    }
    std::fs::set_permissions(&home, std::fs::Permissions::from_mode(WRITABLE)).expect("the fixture folder is the test's own");
    let _ = std::fs::remove_dir_all(&home);
}

/// Something at a shipped file's path that is not a regular file is the player's: a folder there
/// costs that one file and is not removed, and the rest of the pack is installed around it.
#[test]
fn a_folder_where_a_file_belongs_costs_that_file_and_no_other() {
    let home = temp_home("in-the-way");
    let pack = home.join("pack");
    std::fs::create_dir_all(pack.join("select.luaskin")).expect("create the folder in the way");
    let files = [shipped("select.luaskin", b"select"), shipped("decide.luaskin", b"decide")];

    assert_eq!(install_pack(&pack, &files), PackInstall { written: 1, failed: 1, ..PackInstall::default() });
    assert!(pack.join("select.luaskin").is_dir(), "what was in the way was removed");
    assert_eq!(std::fs::read(pack.join("decide.luaskin")).expect("read the installed file"), b"decide");
    let _ = std::fs::remove_dir_all(&home);
}

/// A build that only changes how a file's name is cased ships a different name for what a file
/// system that ignores case holds as the same file. The old name is taken away before the new one
/// is settled, so the run that brings the update leaves both of this build's files in place under
/// the names it ships them with, and the run after it has nothing left to do.
#[test]
fn an_update_that_only_recases_a_name_leaves_the_file_in_place() {
    let home = temp_home("recase");
    let pack = home.join("pack");
    let earlier = [shipped("lua/Old.lua", b"module, first"), shipped("lua/UI.lua", b"module that does not change")];
    assert_eq!(install_pack(&pack, &earlier), PackInstall { written: earlier.len(), ..PackInstall::default() });

    let later = [shipped("lua/old.lua", b"module, second"), shipped("lua/ui.lua", b"module that does not change")];
    assert_eq!(install_pack(&pack, &later), PackInstall { written: 2, removed: 2, ..PackInstall::default() });
    let installed = files_under(&pack);
    assert_eq!(installed.keys().map(String::as_str).collect::<Vec<_>>(), [MANIFEST_FILE, "lua/old.lua", "lua/ui.lua"]);
    assert_eq!(installed["lua/old.lua"], b"module, second");
    assert_eq!(installed["lua/ui.lua"], b"module that does not change");
    assert_eq!(install_pack(&pack, &later), PackInstall::default(), "the run after the update still had something to do");
    let _ = std::fs::remove_dir_all(&home);
}

/// A folder of the pack that is a link to somewhere else is the player's arrangement: nothing is
/// written through it and nothing is removed through it, whatever the record says was installed
/// there. A file that is already there on the other side is taken for the player's own, one that is
/// not is reported as not written, and the files that are reached through the pack's own folders
/// are installed as ever.
#[cfg(unix)]
#[test]
fn a_linked_folder_inside_the_pack_is_neither_written_nor_removed_through() {
    let home = temp_home("linked-folder");
    let pack = home.join("pack");
    let outside = home.join("outside");
    std::fs::create_dir_all(&pack).expect("create the pack folder");
    std::fs::create_dir_all(&outside).expect("create the folder outside the pack");
    std::fs::write(outside.join("victim.lua"), b"not the installer's").expect("write a file outside the pack");
    std::fs::write(outside.join("theirs.lua"), b"the player's own").expect("write the player's file");
    std::os::unix::fs::symlink(&outside, pack.join("lua")).expect("link a folder of the pack to the outside");
    let record = BTreeMap::from([("lua/victim.lua".to_owned(), hex(&Sha256::digest(b"not the installer's")))]);
    std::fs::write(pack.join(MANIFEST_FILE), manifest_text(&record)).expect("write the record");

    let files = [shipped("select.luaskin", b"select"), shipped("lua/ours.lua", b"module"), shipped("lua/theirs.lua", b"module the player has their own of")];
    assert_eq!(install_pack(&pack, &files), PackInstall { written: 1, kept: 1, failed: 1, ..PackInstall::default() });
    let beyond: Vec<String> = files_under(&outside).into_keys().collect();
    assert_eq!(beyond, ["theirs.lua", "victim.lua"], "the install reached through a link out of the pack");
    assert_eq!(std::fs::read(outside.join("theirs.lua")).expect("read the player's file"), b"the player's own");
    assert_eq!(std::fs::read(pack.join("select.luaskin")).expect("read the installed file"), b"select");
    let _ = std::fs::remove_dir_all(&home);
}

/// A run that was cut short leaves a file under the name it was being written with. The next run
/// that has anything to write takes those away, wherever in the pack they are and whichever run
/// left them, and leaves alone a file that only looks like one.
#[test]
fn what_a_run_that_was_cut_short_left_half_written_is_taken_away() {
    let home = temp_home("cut-short");
    let pack = home.join("pack");
    let files = [shipped("select.luaskin", b"select"), shipped("lua/a.lua", b"module")];
    let left_behind =
        [format!("lua/a.lua.99999.{PARTIAL_SUFFIX}"), format!("select.luaskin.7.{PARTIAL_SUFFIX}"), format!("{MANIFEST_FILE}.123.{PARTIAL_SUFFIX}")];
    let not_the_installers = [format!("lua/a.lua.{PARTIAL_SUFFIX}"), format!("lua/mine.lua.99999.{PARTIAL_SUFFIX}"), format!("lua/a.lua.pid.{PARTIAL_SUFFIX}")];
    std::fs::create_dir_all(pack.join("lua")).expect("create the pack folder");
    for name in left_behind.iter().chain(&not_the_installers) {
        std::fs::write(pack.join(name), b"half a file").expect("write what a run left behind");
    }

    assert_eq!(install_pack(&pack, &files), PackInstall { written: files.len(), ..PackInstall::default() });
    let mut expected: Vec<&str> = not_the_installers.iter().map(String::as_str).chain([MANIFEST_FILE, "lua/a.lua", "select.luaskin"]).collect();
    expected.sort_unstable();
    assert_eq!(files_under(&pack).keys().map(String::as_str).collect::<Vec<_>>(), expected);
    let _ = std::fs::remove_dir_all(&home);
}
