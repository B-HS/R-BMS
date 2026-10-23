use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rbms_download::{EXTRACT_MAX_ENTRIES, EXTRACT_MAX_ENTRY_BYTES, EXTRACT_MAX_TOTAL_BYTES, ExtractLimits, extract_zip};
use zip::{CompressionMethod, ZipArchive};

use super::args::{MirrorPackArgs, parse};
use super::folders::{PackLimits, Refusal, pack_entries, skipped_by_client, song_folders_under};
use super::manifest::{MANIFEST_FILE_NAME, ManifestRefusal, MirrorManifest, read_manifest};
use super::notice::scan_notices;
use super::pack::{WriteError, fingerprint, write_pack};
use super::{MirrorPackReport, PACKS_FOLDER_NAME, export, export_within, render};

const ALPHA: &[&str] = &["#TITLE Alpha", "#BPM 120", "#WAV01 a.wav", "#00111:01"];
const ALPHA_HARD: &[&str] = &["#TITLE Alpha Hard", "#BPM 120", "#WAV01 a.wav", "#00111:0101"];
const BETA: &[&str] = &["#TITLE Beta", "#BPM 130", "#WAV01 k.wav", "#00111:01"];

const SOUND: &[u8] = b"RIFF....WAVEfmt , a sound that is only bytes";
const CLIP: &[u8] = b"OggS, a clip that is only bytes";
const PICTURE: &[u8] = b"\x89PNG, a picture that is only bytes";

const NOT_IN_ANY_LIBRARY_MD5: &str = "ffffffffffffffffffffffffffffffff";

static SCRATCH_SERIAL: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let serial = SCRATCH_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("rbms-cli-mirror-pack-{}-{serial}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the scratch folder should be creatable");
        Self(path)
    }

    fn library(&self) -> PathBuf {
        self.0.join("library")
    }

    fn out(&self) -> PathBuf {
        self.0.join("out")
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.library().join(relative);
        fs::create_dir_all(path.parent().expect("a file has a folder")).expect("the library folder should be creatable");
        fs::write(&path, bytes).expect("the scratch file should be writable");
        path
    }

    fn write_chart(&self, relative: &str, lines: &[&str]) -> PathBuf {
        self.write(relative, text_of(lines).as_bytes())
    }

    fn stage_alpha(&self) {
        self.write_chart("songs/alpha/alpha.bms", ALPHA);
        self.write_chart("songs/alpha/alpha-hard.bme", ALPHA_HARD);
        self.write("songs/alpha/a.wav", SOUND);
        self.write("songs/alpha/b.ogg", CLIP);
        self.write("songs/alpha/bg.png", PICTURE);
        self.write("songs/alpha/assets/x.wav", SOUND);
    }

    fn stage_beta(&self) {
        self.write_chart("songs/beta/beta.bms", BETA);
        self.write("songs/beta/k.wav", SOUND);
    }

    fn table_of(&self, md5s: &[&str]) -> PathBuf {
        let entries: Vec<String> = md5s.iter().map(|md5| format!(r#"{{"md5":"{md5}","level":"1"}}"#)).collect();
        let path = self.0.join("data.json");
        fs::write(&path, format!("[{}]", entries.join(","))).expect("the table should be writable");
        path
    }

    fn args(&self) -> MirrorPackArgs {
        MirrorPackArgs { library: self.library(), out: self.out(), tables: Vec::new(), all: true, dry_run: false }
    }

    fn args_for(&self, md5s: &[&str]) -> MirrorPackArgs {
        MirrorPackArgs { tables: vec![self.table_of(md5s)], all: false, ..self.args() }
    }

    fn manifest_in(&self, out: &Path) -> MirrorManifest {
        read_manifest(&out.join(MANIFEST_FILE_NAME)).expect("the manifest should be readable").expect("the manifest should be there")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text_of(lines: &[&str]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn md5_of(lines: &[&str]) -> String {
    rbms_parser::parse(text_of(lines).as_bytes()).md5
}

fn args_of(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

fn run_report(args: &MirrorPackArgs) -> MirrorPackReport {
    export(args).expect("the run should come out")
}

fn pack_files(out: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(out.join(PACKS_FOLDER_NAME))
        .expect("the packs folder should be there")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn open_pack(out: &Path, key: &str) -> ZipArchive<File> {
    ZipArchive::new(File::open(out.join(key)).expect("the pack should be there")).expect("the pack should be a zip archive")
}

fn entry_names(archive: &mut ZipArchive<File>) -> Vec<String> {
    (0..archive.len()).map(|index| archive.by_index(index).expect("the entry should open").name().expect("the entry should be named").into_owned()).collect()
}

fn files_under(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("read the folder") {
            let entry = entry.expect("read the entry");
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.file_type().expect("read the entry type").is_dir() {
                walk(&entry.path(), &format!("{name}/"), found);
            } else {
                found.push(name);
            }
        }
    }
    let mut found = Vec::new();
    walk(root, "", &mut found);
    found.sort();
    found
}

fn folder_of(scratch: &Scratch, relative: &str) -> super::folders::SongFolder {
    song_folders_under(&scratch.library()).folders.into_iter().find(|folder| folder.relative == relative).expect("the song folder should be found")
}

#[test]
fn a_table_or_all_is_required() {
    assert!(parse(&args_of(&["/lib", "--out", "/o"])).is_err(), "neither a table nor --all");
    assert!(parse(&args_of(&["/lib", "--out", "/o", "--all"])).is_ok());
    assert!(parse(&args_of(&["/lib", "--out", "/o", "--table", "x.json"])).is_ok());
}

#[test]
fn the_options_may_stand_on_either_side_of_the_library() {
    let parsed = parse(&args_of(&["--table", "a.json", "/lib", "--out", "/o", "--table", "b.json", "--dry-run"])).expect("the options are enough");
    assert_eq!(parsed, MirrorPackArgs { library: "/lib".into(), out: "/o".into(), tables: vec!["a.json".into(), "b.json".into()], all: false, dry_run: true });
}

#[test]
fn a_command_line_that_cannot_be_read_is_an_error() {
    assert!(parse(&args_of(&["--out", "/o", "--all"])).is_err(), "the library is required");
    assert!(parse(&args_of(&["/lib", "--all"])).is_err(), "--out is required");
    assert!(parse(&args_of(&["/lib", "--out"])).is_err(), "--out needs a value");
    assert!(parse(&args_of(&["/lib", "--out", "/o", "--all", "--nope"])).is_err(), "an unknown option");
    assert!(parse(&args_of(&["/lib", "/other", "--out", "/o", "--all"])).is_err(), "only one library");
}

#[test]
fn the_out_folder_inside_the_library_is_refused() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let inside = scratch.library().join("sub").join("out");
    let error = export(&MirrorPackArgs { out: inside.clone(), ..scratch.args() }).expect_err("an out folder in the library is refused");
    assert!(error.contains("inside the library"), "{error}");
    assert!(!inside.exists(), "nothing is made for a refused run");
    export(&MirrorPackArgs { out: scratch.library(), ..scratch.args() }).expect_err("the library itself is no place for the packs");
}

#[test]
fn a_library_that_is_not_a_folder_is_an_error() {
    let scratch = Scratch::new();
    let error = export(&scratch.args()).expect_err("there is no library");
    assert!(error.contains("not a folder"), "{error}");
}

#[test]
fn a_song_folder_is_one_that_holds_a_chart() {
    let scratch = Scratch::new();
    scratch.write("plain/readme.txt", b"no chart here");
    scratch.write("plain/deeper/a.wav", SOUND);
    scratch.stage_alpha();
    scratch.write_chart("songs/alpha/inner/inner.bms", BETA);
    scratch.write("songs/alpha/inner/inner.wav", SOUND);

    let walked = song_folders_under(&scratch.library());
    let relatives: Vec<&str> = walked.folders.iter().map(|folder| folder.relative.as_str()).collect();
    assert_eq!(relatives, ["songs/alpha", "songs/alpha/inner"]);
    assert!(walked.failures.is_empty(), "{:?}", walked.failures);

    let alpha = pack_entries(&walked.folders[0], &PackLimits::default()).expect("alpha is packable");
    let names: Vec<&str> = alpha.entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["alpha/a.wav", "alpha/alpha-hard.bme", "alpha/alpha.bms", "alpha/assets/x.wav", "alpha/b.ogg", "alpha/bg.png"]);
    let inner = pack_entries(&walked.folders[1], &PackLimits::default()).expect("inner is packable");
    let names: Vec<&str> = inner.entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["inner/inner.bms", "inner/inner.wav"]);
}

#[test]
fn a_library_that_is_itself_a_song_folder_is_one() {
    let scratch = Scratch::new();
    scratch.write_chart("root.bms", ALPHA);
    scratch.write("root.wav", SOUND);
    let library = fs::canonicalize(scratch.library()).expect("the library should resolve");
    let walked = song_folders_under(&library);
    assert_eq!(walked.folders.len(), 1);
    assert_eq!((walked.folders[0].relative.as_str(), walked.folders[0].top.as_str()), (".", "library"));
    let gathered = pack_entries(&walked.folders[0], &PackLimits::default()).expect("the library is packable");
    let names: Vec<&str> = gathered.entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, ["library/root.bms", "library/root.wav"]);
}

#[test]
fn chart_hashes_match_the_parser() {
    let scratch = Scratch::new();
    scratch.write_chart("songs/mixed/one.bms", ALPHA);
    scratch.write_chart("songs/mixed/two.bme", BETA);
    let bmson =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/rbms-parser/tests/bmson/minimal.bmson")).expect("the bmson fixture should be there");
    scratch.write("songs/mixed/three.bmson", &bmson);

    let folder = folder_of(&scratch, "songs/mixed");
    assert_eq!(folder.charts.len(), 3);
    for chart in &folder.charts {
        let bytes = fs::read(&chart.path).expect("the chart should be readable");
        let (md5, sha256) = match chart.path.extension().and_then(|extension| extension.to_str()) {
            Some("bmson") => {
                let parsed = rbms_parser::bmson::parse(&bytes).expect("the bmson should parse");
                (parsed.md5, parsed.sha256)
            }
            _ => {
                let parsed = rbms_parser::parse(&bytes);
                (parsed.md5, parsed.sha256)
            }
        };
        assert_eq!((chart.md5.as_str(), chart.sha256.as_str()), (md5.as_str(), sha256.as_str()), "{}", chart.path.display());
    }
}

#[test]
fn only_folders_with_a_table_md5_are_packed() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let alpha_md5 = md5_of(ALPHA).to_ascii_uppercase();
    let report = run_report(&scratch.args_for(&[alpha_md5.as_str(), NOT_IN_ANY_LIBRARY_MD5]));

    assert_eq!((report.folders_found, report.folders_selected, report.written, report.reused), (2, 1, 1, 0));
    assert!(report.refused.is_empty() && report.failures.is_empty());
    assert_eq!(report.tables.len(), 1);
    assert_eq!((report.tables[0].source.as_str(), report.tables[0].md5s, report.tables[0].matched), ("data.json", 2, 1));
    assert_eq!(report.tables[0].missing, [NOT_IN_ANY_LIBRARY_MD5]);

    let manifest = scratch.manifest_in(&scratch.out());
    assert_eq!(manifest.packs.len(), 1);
    assert_eq!(manifest.packs[0].folder, "songs/alpha");
    assert_eq!(manifest.tables, report.tables);
    assert_eq!(pack_files(&scratch.out()), [format!("{}.zip", manifest.packs[0].sha256)], "one pack, and no temporary file left");
}

#[test]
fn every_chart_of_a_packed_folder_is_listed() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    run_report(&scratch.args_for(&[md5_of(ALPHA).as_str()]));

    let pack = &scratch.manifest_in(&scratch.out()).packs[0];
    let listed: Vec<(&str, &str)> = pack.charts.iter().map(|chart| (chart.path.as_str(), chart.md5.as_str())).collect();
    assert_eq!(listed, [("alpha/alpha-hard.bme", md5_of(ALPHA_HARD).as_str()), ("alpha/alpha.bms", md5_of(ALPHA).as_str())]);
    assert_eq!(pack.key, format!("packs/{}.zip", pack.sha256));
    assert_eq!(pack.top, "alpha");
}

#[test]
fn all_packs_every_song_folder() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let report = run_report(&scratch.args());
    assert_eq!((report.folders_found, report.folders_selected, report.written), (2, 2, 2));
    assert!(report.tables.is_empty());
    let folders: Vec<String> = scratch.manifest_in(&scratch.out()).packs.into_iter().map(|pack| pack.folder).collect();
    assert_eq!(folders, ["songs/alpha", "songs/beta"]);
}

#[test]
fn a_chart_in_two_folders_is_packed_from_the_first_and_counted() {
    let scratch = Scratch::new();
    scratch.write_chart("songs/a/x.bms", ALPHA);
    scratch.write_chart("songs/b/x.bms", ALPHA);
    let report = run_report(&scratch.args_for(&[md5_of(ALPHA).as_str()]));
    assert_eq!((report.folders_selected, report.duplicates, report.tables[0].matched), (1, 1, 1));
    let manifest = scratch.manifest_in(&scratch.out());
    assert_eq!(manifest.packs[0].folder, "songs/a");
    assert_eq!(manifest.duplicates.len(), 1);
    assert_eq!((manifest.duplicates[0].kept.as_str(), manifest.duplicates[0].also.as_slice()), ("songs/a", ["songs/b".to_string()].as_slice()));
}

#[test]
fn two_runs_write_the_same_bytes() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let (first, second) = (scratch.0.join("out-one"), scratch.0.join("out-two"));
    run_report(&MirrorPackArgs { out: first.clone(), ..scratch.args() });
    run_report(&MirrorPackArgs { out: second.clone(), ..scratch.args() });

    assert_eq!(fs::read(first.join(MANIFEST_FILE_NAME)).unwrap(), fs::read(second.join(MANIFEST_FILE_NAME)).unwrap(), "the manifest");
    assert_eq!(pack_files(&first), pack_files(&second), "the packs are named by their bytes");
    for name in pack_files(&first) {
        let path = Path::new(PACKS_FOLDER_NAME).join(&name);
        assert_eq!(fs::read(first.join(&path)).unwrap(), fs::read(second.join(&path)).unwrap(), "{name}");
    }
}

#[test]
fn a_second_run_reuses_an_unchanged_pack() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let first = run_report(&scratch.args());
    let manifest = fs::read(scratch.out().join(MANIFEST_FILE_NAME)).unwrap();
    let again = run_report(&scratch.args());

    assert_eq!((first.written, first.reused), (1, 0));
    assert_eq!((again.written, again.reused), (0, 1));
    assert_eq!(again.total_bytes, first.total_bytes);
    assert_eq!(fs::read(scratch.out().join(MANIFEST_FILE_NAME)).unwrap(), manifest, "the manifest is the same");
}

#[cfg(unix)]
#[test]
fn a_cached_pack_is_not_read_again() {
    use std::os::unix::fs::PermissionsExt;

    const NO_ACCESS_MODE: u32 = 0o000;
    const READABLE_MODE: u32 = 0o644;

    let scratch = Scratch::new();
    scratch.stage_alpha();
    run_report(&scratch.args());
    let sound = scratch.library().join("songs/alpha/a.wav");
    fs::set_permissions(&sound, fs::Permissions::from_mode(NO_ACCESS_MODE)).expect("the file should be lockable");
    let again = run_report(&scratch.args());
    fs::set_permissions(&sound, fs::Permissions::from_mode(READABLE_MODE)).expect("the file should be unlockable");

    assert_eq!((again.written, again.reused), (0, 1), "the folder is unchanged, so its files are not opened");
    assert!(again.refused.is_empty(), "{:?}", again.refused);
}

#[test]
fn a_changed_file_makes_a_new_key() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    run_report(&scratch.args());
    let before = scratch.manifest_in(&scratch.out()).packs[0].clone();
    scratch.write("songs/alpha/a.wav", b"a sound of another length");
    let again = run_report(&scratch.args());
    let after = scratch.manifest_in(&scratch.out()).packs[0].clone();

    assert_eq!((again.written, again.reused), (1, 0));
    assert_ne!((&before.sha256, &before.key, &before.fingerprint), (&after.sha256, &after.key, &after.fingerprint));
    assert_eq!(pack_files(&scratch.out()).len(), 2, "the earlier pack is left where it is");
}

#[test]
fn entries_are_sorted_and_carry_the_folder_name() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    run_report(&scratch.args());
    let key = scratch.manifest_in(&scratch.out()).packs[0].key.clone();
    let names = entry_names(&mut open_pack(&scratch.out(), &key));

    assert_eq!(names, ["alpha/a.wav", "alpha/alpha-hard.bme", "alpha/alpha.bms", "alpha/assets/x.wav", "alpha/b.ogg", "alpha/bg.png"]);
    assert!(names.iter().all(|name| !name.ends_with('/')), "no folder entries");
}

#[test]
fn system_files_are_left_out() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    for name in [".DS_Store", "Thumbs.db", "desktop.ini", "._a.wav", "assets/.DS_Store", "__MACOSX/._alpha.bms", "__MACOSX/alpha.bms"] {
        scratch.write(&format!("songs/alpha/{name}"), b"junk");
    }
    scratch.write("songs/ghost/._ghost.bms", b"#TITLE Ghost\n");
    scratch.write("__MACOSX/songs/._beta.bms", b"#TITLE Beta\n");

    let walked = song_folders_under(&scratch.library());
    assert_eq!(walked.folders.len(), 1, "a stand-in for a chart does not make a song folder: {:?}", walked.folders);
    run_report(&scratch.args());
    let key = scratch.manifest_in(&scratch.out()).packs[0].key.clone();
    let names = entry_names(&mut open_pack(&scratch.out(), &key));
    assert_eq!(names, ["alpha/a.wav", "alpha/alpha-hard.bme", "alpha/alpha.bms", "alpha/assets/x.wav", "alpha/b.ogg", "alpha/bg.png"]);
}

#[test]
fn compressed_media_is_stored_and_the_rest_deflated() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    run_report(&scratch.args());
    let key = scratch.manifest_in(&scratch.out()).packs[0].key.clone();
    let mut archive = open_pack(&scratch.out(), &key);
    let method = |archive: &mut ZipArchive<File>, name: &str| archive.by_name(name).expect("the entry should be there").compression();

    assert_eq!(method(&mut archive, "alpha/b.ogg"), CompressionMethod::Stored);
    assert_eq!(method(&mut archive, "alpha/bg.png"), CompressionMethod::Stored);
    assert_eq!(method(&mut archive, "alpha/a.wav"), CompressionMethod::Deflated);
    assert_eq!(method(&mut archive, "alpha/alpha.bms"), CompressionMethod::Deflated);
}

#[test]
fn the_pack_limits_are_the_ones_the_player_unpacks_to() {
    let limits = PackLimits::default();
    assert_eq!(
        (limits.max_uncompressed_bytes, limits.max_entry_bytes, limits.max_entries),
        (EXTRACT_MAX_TOTAL_BYTES, EXTRACT_MAX_ENTRY_BYTES, EXTRACT_MAX_ENTRIES)
    );
}

#[test]
fn a_folder_over_the_limit_is_refused() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let folder = folder_of(&scratch, "songs/alpha");
    let all_bytes: u64 = pack_entries(&folder, &PackLimits::default()).unwrap().entries.iter().map(|entry| entry.bytes).sum();
    let roomy = PackLimits { max_uncompressed_bytes: u64::MAX, max_entry_bytes: u64::MAX, max_entries: usize::MAX };

    assert_eq!(pack_entries(&folder, &PackLimits { max_uncompressed_bytes: all_bytes - 1, ..roomy }), Err(Refusal::TooLarge { uncompressed_bytes: all_bytes }));
    assert_eq!(pack_entries(&folder, &PackLimits { max_entries: 5, ..roomy }), Err(Refusal::TooManyEntries { entries: 6 }));
    let one_too_big = pack_entries(&folder, &PackLimits { max_entry_bytes: (SOUND.len() - 1) as u64, ..roomy });
    assert_eq!(one_too_big, Err(Refusal::EntryTooLarge { name: "alpha/a.wav".to_string(), bytes: SOUND.len() as u64 }));
    assert!(pack_entries(&folder, &PackLimits { max_uncompressed_bytes: all_bytes, ..roomy }).is_ok(), "a folder at the limit is packed");
}

#[test]
fn a_refusal_says_what_it_is_in_the_manifest() {
    let refusal = Refusal::EntryTooLarge { name: "alpha/a.wav".to_string(), bytes: 7 };
    assert_eq!((refusal.code(), refusal.detail().as_str()), ("entry_too_large", "alpha/a.wav: 7"));
    assert_eq!((Refusal::TooLarge { uncompressed_bytes: 9 }.code(), Refusal::TooLarge { uncompressed_bytes: 9 }.detail().as_str()), ("too_large", "9"));
    assert_eq!(Refusal::TooManyEntries { entries: 3 }.code(), "too_many_entries");
    assert_eq!(Refusal::NonUtf8Name { path: PathBuf::from("x") }.code(), "non_utf8_name");
    assert_eq!(Refusal::Unreadable { path: PathBuf::from("x"), reason: "gone".to_string() }.detail(), "x: gone");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_not_followed() {
    use std::os::unix::fs::symlink;

    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let outside = scratch.0.join("outside.wav");
    fs::write(&outside, b"a sound outside the library").unwrap();
    symlink(&outside, scratch.library().join("songs/alpha/link.wav")).unwrap();
    symlink(scratch.library().join("songs/beta"), scratch.library().join("songs/alpha/linked-folder")).unwrap();
    symlink(scratch.library(), scratch.library().join("songs/loop")).unwrap();

    let walked = song_folders_under(&scratch.library());
    assert_eq!(walked.folders.len(), 2, "a link is not a song folder");
    let alpha = pack_entries(&folder_of(&scratch, "songs/alpha"), &PackLimits::default()).unwrap();
    assert_eq!(alpha.symlinks, 2);
    assert!(alpha.entries.iter().all(|entry| !entry.name.contains("link")), "{:?}", alpha.entries);
    run_report(&scratch.args());
    assert_eq!(scratch.manifest_in(&scratch.out()).packs[0].symlinks, 2);
}

#[test]
fn notices_find_redistribution_words_in_shift_jis() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.write("songs/alpha/readme.txt", &encoding_rs::SHIFT_JIS.encode("再配布禁止").0);
    scratch.write("songs/alpha/english.txt", "Do not REDISTRIBUTE this".as_bytes());
    scratch.write("songs/alpha/plain.txt", b"just a greeting");
    scratch.write("songs/alpha/unicode.txt", "転載はご遠慮ください".as_bytes());

    let folder = folder_of(&scratch, "songs/alpha");
    let gathered = pack_entries(&folder, &PackLimits::default()).unwrap();
    let notices = scan_notices(&folder, &gathered.entries);
    assert_eq!(notices.text_files, ["alpha/english.txt", "alpha/plain.txt", "alpha/readme.txt", "alpha/unicode.txt"]);
    assert_eq!(notices.notice_files, ["alpha/english.txt", "alpha/readme.txt", "alpha/unicode.txt"]);
}

#[test]
fn a_text_file_cut_inside_a_character_is_still_read_as_utf8() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let mut text = "再配布".to_string();
    text.push_str(&"あ".repeat(400_000));
    scratch.write("songs/alpha/long.txt", text.as_bytes());

    let folder = folder_of(&scratch, "songs/alpha");
    let gathered = pack_entries(&folder, &PackLimits::default()).unwrap();
    assert_eq!(scan_notices(&folder, &gathered.entries).notice_files, ["alpha/long.txt"]);
}

#[test]
fn decomposed_names_are_counted() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.write("songs/alpha/\u{304B}\u{3099}.wav", SOUND);

    let folder = folder_of(&scratch, "songs/alpha");
    let gathered = pack_entries(&folder, &PackLimits::default()).unwrap();
    assert_eq!(scan_notices(&folder, &gathered.entries).nfd_names, 1);
}

#[test]
fn a_path_that_leaves_the_folder_in_a_chart_is_counted() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.write_chart(
        "songs/alpha/outside.bms",
        &["#TITLE Outside", "#WAV01 ../a.wav", "#WAV02 sub/../ok.wav", "#WAV03 sub/ok.wav", "#BMP01 /abs.bmp", "#00111:01"],
    );
    scratch.write_chart("songs/alpha/again.bms", &["#TITLE Again", "#WAV01 ../a.wav", "#00111:01"]);

    let folder = folder_of(&scratch, "songs/alpha");
    let gathered = pack_entries(&folder, &PackLimits::default()).unwrap();
    assert_eq!(scan_notices(&folder, &gathered.entries).escapes_root, 2, "the same file named twice is one");
}

#[test]
fn a_chart_that_names_only_its_own_folder_counts_nothing() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let folder = folder_of(&scratch, "songs/alpha");
    let gathered = pack_entries(&folder, &PackLimits::default()).unwrap();
    let notices = scan_notices(&folder, &gathered.entries);
    assert_eq!((notices.escapes_root, notices.nfd_names, notices.text_files.len(), notices.notice_files.len()), (0, 0, 0, 0));
}

#[test]
fn the_manifest_has_no_absolute_path() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    scratch.write("songs/alpha/readme.txt", b"hello");
    run_report(&scratch.args_for(&[md5_of(ALPHA).as_str(), NOT_IN_ANY_LIBRARY_MD5]));

    let manifest = fs::read_to_string(scratch.out().join(MANIFEST_FILE_NAME)).unwrap();
    let scratch_name = scratch.0.file_name().unwrap().to_string_lossy().into_owned();
    assert!(!manifest.contains(&scratch_name), "{manifest}");
    assert!(!manifest.contains(std::env::temp_dir().to_string_lossy().as_ref()), "{manifest}");
}

#[test]
fn a_dry_run_writes_nothing() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let report = run_report(&MirrorPackArgs { dry_run: true, ..scratch.args_for(&[md5_of(ALPHA).as_str()]) });

    assert!(!scratch.out().exists(), "not even the out folder is made");
    assert_eq!((report.folders_selected, report.written, report.reused, report.total_bytes), (1, 0, 0, 0));
    assert_eq!(report.total_entries, 6);
    assert!(report.total_uncompressed_bytes > 0);
    assert_eq!(report.tables[0].matched, 1);
}

const FEW_ENTRIES: usize = 3;

#[test]
fn a_refused_folder_is_listed_and_the_run_goes_on() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let limits = PackLimits { max_entries: FEW_ENTRIES, ..PackLimits::default() };
    let report = export_within(&scratch.args(), &limits).expect("a refused folder does not fail the run");

    assert_eq!((report.folders_selected, report.written), (2, 1));
    assert_eq!(report.refused, [("songs/alpha".to_string(), Refusal::TooManyEntries { entries: 6 })]);
    let manifest = scratch.manifest_in(&scratch.out());
    let packed: Vec<&str> = manifest.packs.iter().map(|pack| pack.folder.as_str()).collect();
    assert_eq!(packed, ["songs/beta"]);
    assert_eq!(manifest.refused, [ManifestRefusal { folder: "songs/alpha".to_string(), reason: "too_many_entries".to_string(), detail: "6".to_string() }]);
    assert_eq!(pack_files(&scratch.out()).len(), 1, "nothing is written for the refused folder");
}

#[test]
fn the_report_names_what_was_packed_refused_and_missing() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.stage_beta();
    let limits = PackLimits { max_entries: FEW_ENTRIES, ..PackLimits::default() };
    let args = scratch.args_for(&[md5_of(ALPHA).as_str(), md5_of(BETA).as_str(), NOT_IN_ANY_LIBRARY_MD5]);
    let report = export_within(&args, &limits).expect("the run should come out");

    let text = render::text(&args, &report);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.iter().any(|line| line.starts_with("packs") && line.ends_with("1 (written 1, reused 0)")), "{text}");
    assert!(lines.contains(&"  songs/alpha: too_many_entries 6"), "{text}");
    assert!(lines.contains(&format!("  {NOT_IN_ANY_LIBRARY_MD5}").as_str()), "{text}");
    assert!(lines.iter().any(|line| line.starts_with("packs size")), "{text}");

    let dry = MirrorPackArgs { dry_run: true, ..args };
    let dry_text = render::text(&dry, &export_within(&dry, &limits).expect("the dry run should come out"));
    assert!(dry_text.lines().any(|line| line.starts_with("would pack") && line.ends_with(": 1")), "{dry_text}");
    assert!(!dry_text.contains("packs size"), "{dry_text}");
}

#[test]
fn a_pack_opens_with_the_zip_reader() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    run_report(&scratch.args());
    let pack = scratch.manifest_in(&scratch.out()).packs[0].clone();
    let mut archive = open_pack(&scratch.out(), &pack.key);

    assert_eq!(archive.len(), pack.entries);
    let mut total = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().expect("the entry should be named").into_owned();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
        let original = fs::read(scratch.library().join("songs").join(&name)).unwrap();
        assert_eq!(bytes, original, "{name}");
        total += bytes.len() as u64;
    }
    assert_eq!(total, pack.uncompressed_bytes);
    assert_eq!(pack.bytes, fs::metadata(scratch.out().join(&pack.key)).unwrap().len());
}

#[test]
fn a_pack_is_fully_unpacked_by_the_client_extractor() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.write("songs/alpha/readme.txt", b"hello");
    scratch.write("songs/alpha/movie.mp4", b"a movie that is only bytes");
    scratch.write("songs/alpha/cover.bmp", b"BM, a picture that is only bytes");
    run_report(&scratch.args());
    let pack = scratch.manifest_in(&scratch.out()).packs[0].clone();

    let unpacked = scratch.0.join("unpacked");
    let report =
        extract_zip(&scratch.out().join(&pack.key), &unpacked, &ExtractLimits::default(), &AtomicBool::new(false)).expect("the player should unpack the pack");
    assert_eq!((report.written, report.skipped, report.charts), (pack.entries, 0, 2));
    assert_eq!(pack.skipped_by_client, 0);
    let expected: Vec<String> = files_under(&scratch.library().join("songs/alpha")).into_iter().map(|name| format!("alpha/{name}")).collect();
    assert_eq!(files_under(&unpacked), expected);
    for name in &expected {
        assert_eq!(fs::read(unpacked.join(name)).unwrap(), fs::read(scratch.library().join("songs").join(name)).unwrap(), "{name}");
    }
}

#[test]
fn the_kinds_the_player_leaves_out_are_counted_as_it_leaves_them_out() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    scratch.write("songs/alpha/tool.exe", b"a program");
    scratch.write("songs/alpha/more.rar", b"another pack");
    scratch.write("songs/alpha/LICENSE", b"no extension");
    let report = run_report(&scratch.args());
    let pack = scratch.manifest_in(&scratch.out()).packs[0].clone();

    let unpacked = scratch.0.join("unpacked");
    let unpacking =
        extract_zip(&scratch.out().join(&pack.key), &unpacked, &ExtractLimits::default(), &AtomicBool::new(false)).expect("the player should unpack the pack");
    assert_eq!((pack.skipped_by_client, report.skipped_by_client), (3, 3));
    assert_eq!((unpacking.skipped, unpacking.written), (pack.skipped_by_client, pack.entries - 3));
}

#[test]
fn skipped_kinds_are_judged_by_extension_in_any_case() {
    let scratch = Scratch::new();
    scratch.write_chart("songs/alpha/alpha.BMS", ALPHA);
    scratch.write("songs/alpha/A.WAV", SOUND);
    scratch.write("songs/alpha/b.m4a", SOUND);
    let gathered = pack_entries(&folder_of(&scratch, "songs/alpha"), &PackLimits::default()).unwrap();
    assert_eq!(skipped_by_client(&gathered.entries), 1);
}

#[test]
fn a_file_that_cannot_be_read_is_a_source_error_and_leaves_no_temporary_file() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let folder = folder_of(&scratch, "songs/alpha");
    let mut entries = pack_entries(&folder, &PackLimits::default()).unwrap().entries;
    entries[2].source = scratch.0.join("vanished.wav");
    let vanished = entries[2].name.clone();
    let packs = scratch.0.join("packs");

    match write_pack(&entries, &packs) {
        Err(WriteError::Source { name, .. }) => assert_eq!(name, vanished),
        other => panic!("expected a source error, got {other:?}"),
    }
    assert_eq!(fs::read_dir(&packs).unwrap().count(), 0, "the half-written archive is removed");
}

#[test]
fn the_fingerprint_follows_names_sizes_and_times() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let entries = pack_entries(&folder_of(&scratch, "songs/alpha"), &PackLimits::default()).unwrap().entries;
    let same = fingerprint(&entries);
    assert_eq!(same, fingerprint(&entries));
    let mut other_size = entries.clone();
    other_size[0].bytes += 1;
    let mut other_time = entries.clone();
    other_time[0].modified_nanos += 1;
    let mut other_name = entries.clone();
    other_name[0].name.push('x');
    for changed in [other_size, other_time, other_name] {
        assert_ne!(same, fingerprint(&changed));
    }
}

#[test]
fn the_manifest_reads_back_and_a_file_that_is_not_one_is_an_error() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    let path = scratch.out().join(MANIFEST_FILE_NAME);
    assert_eq!(read_manifest(&path), Ok(None), "no manifest yet");

    run_report(&scratch.args());
    let manifest = read_manifest(&path).unwrap().expect("the run wrote a manifest");
    assert_eq!((manifest.format.as_str(), manifest.version), ("rbms-song-mirror-manifest", 1));
    assert!(manifest.generator.starts_with("rbms-cli "), "{}", manifest.generator);

    fs::write(&path, b"not json").unwrap();
    assert!(read_manifest(&path).is_err());
    fs::write(&path, br#"{"format":"something-else","version":1,"generator":"x","tables":[],"packs":[],"refused":[],"duplicates":[]}"#).unwrap();
    assert!(read_manifest(&path).is_err());
}

#[test]
fn a_manifest_that_cannot_be_read_does_not_stop_the_run() {
    let scratch = Scratch::new();
    scratch.stage_alpha();
    fs::create_dir_all(scratch.out()).unwrap();
    fs::write(scratch.out().join(MANIFEST_FILE_NAME), b"not json").unwrap();
    let report = run_report(&scratch.args());
    assert_eq!((report.written, report.reused), (1, 0));
    assert_eq!(scratch.manifest_in(&scratch.out()).packs.len(), 1, "the manifest is written afresh");
}
