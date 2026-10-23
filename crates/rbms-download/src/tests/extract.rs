use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use super::{ArchiveBuilder, CHART, Fixture, SOUND, files_under, not_cancelled};
use crate::extract::{Plan, open_archive, plan_entry};
use crate::{
    DownloadError, EXTRACT_MAX_ENTRIES, EXTRACT_MAX_ENTRY_BYTES, EXTRACT_MAX_TOTAL_BYTES, ExtractLimits, ExtractReport, extract_zip, is_kept_extension,
};

const SMALL_ENTRY_BYTES: u64 = 1000;
const SMALL_TOTAL_BYTES: u64 = 1500;
const BOMB_BYTES: usize = 8 * 1024 * 1024;
const BIG_FILE_BYTES: usize = 300 * 1024;

struct Unpacked {
    fixture: Fixture,
    result: Result<ExtractReport, DownloadError>,
    into: PathBuf,
}

impl Unpacked {
    fn files(&self) -> Vec<String> {
        files_under(&self.into)
    }

    fn report(&self) -> ExtractReport {
        *self.result.as_ref().expect("the archive unpacks")
    }
}

fn unpack_with(tag: &str, archive: Vec<u8>, limits: &ExtractLimits) -> Unpacked {
    let fixture = Fixture::new(tag);
    let path = fixture.write("pack.zip", &archive);
    let into = fixture.path("out").join("pack");
    let result = extract_zip(&path, &into, limits, &not_cancelled());
    Unpacked { fixture, result, into }
}

fn unpack(tag: &str, archive: Vec<u8>) -> Unpacked {
    unpack_with(tag, archive, &ExtractLimits::default())
}

fn small_limits() -> ExtractLimits {
    ExtractLimits { max_entries: EXTRACT_MAX_ENTRIES, max_entry_bytes: SMALL_ENTRY_BYTES, max_total_bytes: SMALL_TOTAL_BYTES }
}

#[test]
fn the_default_limits_are_the_ones_the_player_uses() {
    let limits = ExtractLimits::default();
    assert_eq!((limits.max_entries, limits.max_entry_bytes, limits.max_total_bytes), (EXTRACT_MAX_ENTRIES, EXTRACT_MAX_ENTRY_BYTES, EXTRACT_MAX_TOTAL_BYTES));
    assert_eq!((EXTRACT_MAX_ENTRIES, EXTRACT_MAX_ENTRY_BYTES, EXTRACT_MAX_TOTAL_BYTES), (10_000, 1 << 30, 2 << 30));
}

#[test]
fn a_pack_is_unpacked_with_its_contents() {
    let big: Vec<u8> = (0..BIG_FILE_BYTES).map(|index| (index * 7 % 251) as u8).collect();
    let archive = ArchiveBuilder::new()
        .directory("song/")
        .stored("song/a.bms", CHART)
        .deflated("song/b.bme", CHART)
        .deflated("song/sounds/kick.wav", &big)
        .stored("song/readme.txt", b"hello")
        .stored("song/empty.ogg", b"")
        .build();
    let unpacked = unpack("pack", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 5, skipped: 0, charts: 2 });
    assert_eq!(unpacked.files(), ["song/a.bms", "song/b.bme", "song/empty.ogg", "song/readme.txt", "song/sounds/kick.wav"]);
    assert_eq!(fs::read(unpacked.into.join("song/sounds/kick.wav")).expect("the sound is there"), big);
    assert_eq!(fs::read(unpacked.into.join("song/a.bms")).expect("the chart is there"), CHART);
    assert_eq!(fs::read(unpacked.into.join("song/b.bme")).expect("the chart is there"), CHART);
}

#[test]
fn extensions_are_read_without_regard_to_case_and_a_back_slash_is_a_separator() {
    let archive = ArchiveBuilder::new().stored("A.BMS", CHART).stored("Sub\\Inner\\B.WaV", SOUND).stored("c.PNG", b"png").build();
    let unpacked = unpack("case", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 3, skipped: 0, charts: 1 });
    assert_eq!(unpacked.files(), ["A.BMS", "Sub/Inner/B.WaV", "c.PNG"]);
}

#[test]
fn entries_that_climb_out_are_refused() {
    let archive = ArchiveBuilder::new()
        .stored("ok.bms", CHART)
        .stored("../up.bms", CHART)
        .stored("/abs.bms", CHART)
        .stored("C:x.bms", CHART)
        .stored("a\\..\\..\\b.bms", CHART)
        .stored("dir/../../c.bms", CHART)
        .build();
    let unpacked = unpack("climb", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 1, skipped: 5, charts: 1 });
    assert_eq!(unpacked.files(), ["ok.bms"]);
    assert_eq!(files_under(&unpacked.fixture.root), ["out/pack/ok.bms", "pack.zip"]);
}

#[test]
fn names_that_not_every_platform_takes_are_skipped() {
    let archive = ArchiveBuilder::new()
        .stored("ok.bms", CHART)
        .stored("a//b.bms", CHART)
        .stored("dir/./c.bms", CHART)
        .stored("space /d.bms", CHART)
        .stored("dot./e.bms", CHART)
        .stored("star*.wav", SOUND)
        .stored("what?.wav", SOUND)
        .stored("pipe|.wav", SOUND)
        .stored("quote\".wav", SOUND)
        .stored("angle<.wav", SOUND)
        .stored("stream.bms:evil", CHART)
        .stored("tab\t.wav", SOUND)
        .build();
    let unpacked = unpack("platform-names", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 1, skipped: 11, charts: 1 });
    assert_eq!(unpacked.files(), ["ok.bms"]);
}

#[test]
fn windows_device_names_are_refused() {
    let archive = ArchiveBuilder::new()
        .stored("ok.bms", CHART)
        .stored("CON.bms", CHART)
        .stored("nul.wav", SOUND)
        .stored("dir/Com1.txt", b"x")
        .stored("LPT9.ogg", SOUND)
        .stored("aux.ogg", SOUND)
        .stored("PRN", b"x")
        .stored("COM10.bms", CHART)
        .stored("console.bms", CHART)
        .build();
    let unpacked = unpack("devices", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 3, skipped: 6, charts: 3 });
    assert_eq!(unpacked.files(), ["COM10.bms", "console.bms", "ok.bms"]);
}

#[test]
fn link_entries_are_skipped() {
    let archive = ArchiveBuilder::new().stored("ok.bms", CHART).symlink("link.wav", "/etc/passwd").symlink("dir/other.bms", "../../outside").build();
    let unpacked = unpack("links", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 1, skipped: 2, charts: 1 });
    assert_eq!(unpacked.files(), ["ok.bms"]);
}

#[test]
fn shift_jis_names_are_read_as_shift_jis() {
    let shift_jis = |text: &str| encoding_rs::SHIFT_JIS.encode(text).0.into_owned();
    let names = [shift_jis("\u{65e5}\u{672c}\u{8a9e}.bms"), shift_jis("\u{97f3}\u{697d}/\u{4e2d}.wav"), shift_jis("\u{8868}\u{80fd}.bms")];
    assert!(names.iter().all(|name| std::str::from_utf8(name).is_err()));
    assert!(names[2].contains(&b'\\'), "a trail byte of 0x5C is what a split on the raw bytes would break on");
    let mut builder = ArchiveBuilder::new();
    for (index, raw) in names.iter().enumerate() {
        let placeholder = char::from(b'A' + index as u8).to_string().repeat(raw.len());
        builder = builder.stored(&placeholder, CHART).name_bytes(&placeholder, raw);
    }
    let unpacked = unpack("sjis", builder.build());
    assert_eq!(unpacked.report().written, 3, "{:?}", unpacked.result);
    assert_eq!(unpacked.files(), ["\u{65e5}\u{672c}\u{8a9e}.bms", "\u{8868}\u{80fd}.bms", "\u{97f3}\u{697d}/\u{4e2d}.wav"]);
}

#[test]
fn utf8_names_are_kept() {
    let archive =
        ArchiveBuilder::new().stored("\u{66f2}\u{540d}/\u{30c1}\u{30e3}\u{30fc}\u{30c8}.bms", CHART).stored("\u{66f2}\u{540d}/\u{97f3}.wav", SOUND).build();
    let unpacked = unpack("utf8", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 2, skipped: 0, charts: 1 });
    assert_eq!(unpacked.files(), ["\u{66f2}\u{540d}/\u{30c1}\u{30e3}\u{30fc}\u{30c8}.bms", "\u{66f2}\u{540d}/\u{97f3}.wav"]);
}

#[test]
fn a_name_flagged_as_utf8_is_never_taken_for_shift_jis() {
    let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("\u{66f2}.bms");
    let flagged = plan_entry(&sjis, true, false, false);
    let unflagged = plan_entry(&sjis, false, false, false);
    assert!(matches!(&flagged, Plan::Write { relative, .. } if relative.to_string_lossy().contains('\u{fffd}')), "{flagged:?}");
    assert_eq!(unflagged, Plan::Write { relative: PathBuf::from("\u{66f2}.bms"), is_chart: true });
}

#[test]
fn files_of_other_kinds_are_skipped_and_counted() {
    let archive = ArchiveBuilder::new()
        .stored("a.bms", CHART)
        .stored("kick.wav", SOUND)
        .stored("bg.png", b"png")
        .stored("movie.mp4", b"mp4")
        .stored("readme.txt", b"txt")
        .stored("run.exe", b"MZ")
        .stored("setup.bat", b"echo")
        .stored("script.sh", b"#!/bin/sh")
        .stored("lib.dll", b"MZ")
        .stored("readme", b"no extension")
        .stored(".hidden", b"hidden")
        .stored("Thumbs.db", b"db")
        .build();
    let unpacked = unpack("kinds", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 5, skipped: 7, charts: 1 });
    assert_eq!(unpacked.files(), ["a.bms", "bg.png", "kick.wav", "movie.mp4", "readme.txt"]);
}

#[test]
fn every_kind_a_pack_is_made_of_is_kept() {
    let names = [
        "a.bms", "a.bme", "a.bml", "a.pms", "a.bmson", "a.wav", "a.ogg", "a.mp3", "a.flac", "a.bmp", "a.png", "a.jpg", "a.jpeg", "a.gif", "a.mpg", "a.mpeg",
        "a.avi", "a.mp4", "a.wmv", "a.m4v", "a.webm", "a.mkv", "a.txt",
    ];
    let archive = names.iter().fold(ArchiveBuilder::new(), |builder, name| builder.stored(name, b"x")).build();
    let unpacked = unpack("all-kinds", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: names.len(), skipped: 0, charts: 5 });
}

#[test]
fn the_kinds_that_are_kept_are_told_without_regard_to_case() {
    for extension in ["bms", "BME", "Pms", "bmson", "wav", "OGG", "png", "mp4", "txt"] {
        assert!(is_kept_extension(extension), "{extension}");
    }
    for extension in ["", "exe", "m4a", "wma", "ogv", "flv", "tga", "webp", "bms.exe", "zip"] {
        assert!(!is_kept_extension(extension), "{extension}");
    }
}

#[test]
fn an_entry_past_the_cap_stops_the_unpacking_and_removes_it() {
    let over = vec![b'a'; SMALL_ENTRY_BYTES as usize + 1];
    let archive = ArchiveBuilder::new().stored("first.bms", CHART).deflated("big.wav", &over).stored("last.bms", CHART).build();
    let unpacked = unpack_with("entry-cap", archive, &small_limits());
    assert!(matches!(&unpacked.result, Err(DownloadError::EntryTooLarge { name, limit: SMALL_ENTRY_BYTES }) if name == "big.wav"), "{:?}", unpacked.result);
    assert!(!unpacked.into.exists());
}

#[test]
fn an_archive_that_unpacks_past_the_cap_is_stopped_and_removed() {
    let part = vec![b'a'; 600];
    let archive = ArchiveBuilder::new().deflated("1.bms", &part).deflated("2.wav", &part).deflated("3.wav", &part).build();
    let unpacked = unpack_with("total-cap", archive, &small_limits());
    assert!(matches!(&unpacked.result, Err(DownloadError::UnpackedTooLarge { limit: SMALL_TOTAL_BYTES })), "{:?}", unpacked.result);
    assert!(!unpacked.into.exists());
}

#[test]
fn sizes_that_land_exactly_on_the_caps_are_allowed() {
    let one_thousand = vec![b'a'; SMALL_ENTRY_BYTES as usize];
    let five_hundred = vec![b'a'; 500];
    let archive = ArchiveBuilder::new().deflated("1.bms", &one_thousand).deflated("2.wav", &five_hundred).stored("3.wav", b"").build();
    let unpacked = unpack_with("exact-caps", archive, &small_limits());
    assert_eq!(unpacked.report(), ExtractReport { written: 3, skipped: 0, charts: 1 });
}

#[test]
fn a_small_archive_that_unpacks_huge_is_stopped_early() {
    let zeros = vec![0u8; BOMB_BYTES];
    let archive = ArchiveBuilder::new().stored("a.bms", CHART).deflated("bomb.wav", &zeros).build();
    assert!(archive.len() < BOMB_BYTES / 100, "the archive is the small thing it is meant to be");
    let limits = ExtractLimits { max_entry_bytes: 1 << 20, ..ExtractLimits::default() };
    let unpacked = unpack_with("bomb", archive, &limits);
    assert!(matches!(&unpacked.result, Err(DownloadError::EntryTooLarge { limit, .. }) if *limit == 1 << 20), "{:?}", unpacked.result);
    assert!(!unpacked.into.exists());
}

#[test]
fn too_many_entries_are_refused() {
    let archive = ArchiveBuilder::new().stored("1.bms", CHART).stored("2.bms", CHART).stored("3.bms", CHART).build();
    let limits = ExtractLimits { max_entries: 2, ..ExtractLimits::default() };
    let unpacked = unpack_with("too-many", archive.clone(), &limits);
    assert!(matches!(&unpacked.result, Err(DownloadError::TooManyEntries { limit: 2 })), "{:?}", unpacked.result);
    assert!(!unpacked.into.exists());
    let exactly = unpack_with("exactly-many", archive, &ExtractLimits { max_entries: 3, ..ExtractLimits::default() });
    assert_eq!(exactly.report().written, 3);
}

#[test]
fn an_archive_with_no_chart_is_discarded() {
    for (tag, archive) in [
        ("no-chart-sounds", ArchiveBuilder::new().stored("a.wav", SOUND).stored("b.png", b"png").build()),
        ("no-chart-other", ArchiveBuilder::new().stored("run.exe", b"MZ").build()),
        ("no-chart-folders", ArchiveBuilder::new().directory("empty/").build()),
    ] {
        let unpacked = unpack(tag, archive);
        assert!(matches!(&unpacked.result, Err(DownloadError::NoChart)), "{tag}: {:?}", unpacked.result);
        assert!(!unpacked.into.exists(), "{tag}");
    }
}

#[test]
fn a_central_directory_past_its_budget_is_refused() {
    let fixture = Fixture::new("index-budget");
    let archive = (0..50).fold(ArchiveBuilder::new(), |builder, index| builder.stored(&format!("chart-{index:02}.bms"), CHART)).build();
    let path = fixture.write("pack.zip", &archive);
    let refused = open_archive(&path, 64).map(|opened| opened.len());
    assert!(matches!(refused, Err(DownloadError::IndexTooLarge { limit: 64 })), "{refused:?}");
    let allowed = open_archive(&path, archive.len() as u64 * 4).map(|opened| opened.len());
    assert!(matches!(allowed, Ok(50)), "{allowed:?}");
}

#[test]
fn a_cancelled_unpacking_leaves_nothing() {
    let fixture = Fixture::new("extract-cancel");
    let path = fixture.write("pack.zip", &ArchiveBuilder::new().stored("a.bms", CHART).build());
    let into = fixture.path("out");
    let result = extract_zip(&path, &into, &ExtractLimits::default(), &AtomicBool::new(true));
    assert!(matches!(result, Err(DownloadError::Cancelled)), "{result:?}");
    assert!(!into.exists());
}

#[test]
fn a_folder_that_has_something_in_it_is_refused_and_left_alone() {
    let fixture = Fixture::new("extract-not-empty");
    let path = fixture.write("pack.zip", &ArchiveBuilder::new().stored("a.bms", CHART).build());
    let into = fixture.path("out");
    fs::create_dir_all(&into).expect("make the folder");
    fs::write(into.join("mine.txt"), b"mine").expect("write into it");
    let result = extract_zip(&path, &into, &ExtractLimits::default(), &not_cancelled());
    assert!(matches!(result, Err(DownloadError::StagingNotEmpty(_))), "{result:?}");
    assert_eq!(fs::read(into.join("mine.txt")).expect("the file is as it was"), b"mine");
    assert_eq!(files_under(&into), ["mine.txt"]);
}

#[test]
fn a_folder_that_exists_and_is_empty_is_unpacked_into() {
    let fixture = Fixture::new("extract-empty");
    let path = fixture.write("pack.zip", &ArchiveBuilder::new().stored("a.bms", CHART).build());
    let into = fixture.path("out");
    fs::create_dir_all(&into).expect("make the folder");
    extract_zip(&path, &into, &ExtractLimits::default(), &not_cancelled()).expect("the archive unpacks");
    assert_eq!(files_under(&into), ["a.bms"]);
}

#[test]
fn a_file_that_is_not_a_zip_is_an_archive_error_and_makes_nothing() {
    let fixture = Fixture::new("extract-garbage");
    let path = fixture.write("pack.zip", b"<html>this is a web page</html>");
    let into = fixture.path("out");
    let result = extract_zip(&path, &into, &ExtractLimits::default(), &not_cancelled());
    assert!(matches!(result, Err(DownloadError::Archive(_))), "{result:?}");
    assert!(!into.exists());
    let missing = extract_zip(&fixture.path("missing.zip"), &into, &ExtractLimits::default(), &not_cancelled());
    assert!(matches!(missing, Err(DownloadError::Io(_))), "{missing:?}");
}

#[test]
fn a_name_that_is_a_file_and_a_folder_is_skipped_not_overwritten() {
    let archive = ArchiveBuilder::new()
        .stored("a.bms", CHART)
        .stored("x.wav", b"file first")
        .stored("x.wav/inner.bms", CHART)
        .stored("d.wav/inner.bms", CHART)
        .stored("d.wav", b"file last")
        .build();
    let unpacked = unpack("file-and-folder", archive);
    assert_eq!(unpacked.report(), ExtractReport { written: 3, skipped: 2, charts: 2 });
    assert_eq!(fs::read(unpacked.into.join("x.wav")).expect("the file is as it was"), b"file first");
    assert!(unpacked.into.join("d.wav").is_dir());
}
