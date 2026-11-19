//! The captures: what a run is captured as and why one is not, the file kept once per layout, and a
//! file that is missing or broken.

use super::*;

/// The chart the captures are of, and another.
const MD5: &str = "0123456789abcdef0123456789abcdef";
const OTHER_MD5: &str = "fedcba9876543210fedcba9876543210";

/// The seed RANDOM lays seven keys out as `3517246` from (the seed LAYOUT PICK finds for it).
const SEED_3517246: u64 = 4761;

/// The moments two captures are kept at.
const EARLIER_MS: i64 = 1_700_000_000_000;
const LATER_MS: i64 = 1_700_000_060_000;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms-captured-layouts-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp dir");
    dir
}

/// A run of the chart on `option` from `seed`, on seven keys, played by hand.
fn run(option: NoteOption, seed: u64) -> CapturedRun<'static> {
    CapturedRun { md5: MD5, title: "Title", subtitle: "[ANOTHER]", mode: Mode::BEAT_7K, option, seed, ex_score: 1234, lamp: "HARD" }
}

fn captured(option: NoteOption, seed: u64) -> CapturedLayout {
    run(option, seed).capture(RunKind::default()).expect("the run is captured")
}

/// A capture is what a run is asked to borrow -- the chart, the arrangement as a replay file names
/// it and the seed -- with the lane order of each side written as LAYOUT PICK writes it, worked
/// out the way the run laid its lanes out.
#[test]
fn a_capture_writes_the_lane_order_the_run_was_laid_out_in() {
    let random = captured(NoteOption::Random, SEED_3517246);
    assert_eq!((random.md5.as_str(), random.random_label.as_str(), random.seed), (MD5, "RANDOM", SEED_3517246));
    assert_eq!(random.layout, ["3517246"]);
    assert_eq!((random.title.as_str(), random.mode.as_str(), random.ex_score, random.lamp.as_str()), ("Title [ANOTHER]", "BEAT_7K", 1234, "HARD"));
    assert_eq!(random.rows(Mode::BEAT_7K), Some(vec![vec![3, 5, 1, 7, 2, 4, 6]]));
    assert_eq!(captured(NoteOption::Mirror, 1).layout, ["7654321"]);
    for option in [NoteOption::RRandom, NoteOption::Rotate] {
        let capture = captured(option, SEED_3517246);
        let perm = lane_permutation(option, Mode::BEAT_7K, SEED_3517246);
        let rows = layout_of_permutation(Mode::BEAT_7K, &perm).expect("a lane order");
        assert_eq!(capture.layout, rows.iter().map(|row| layout_row_text(row)).collect::<Vec<_>>(), "{option:?}");
    }
    let double = CapturedRun { mode: Mode::BEAT_14K, ..run(NoteOption::Random, SEED_3517246) }.capture(RunKind::default()).expect("a double run");
    assert_eq!(double.layout.len(), 2, "a row a side");
    assert_eq!(double.layout_text(), double.layout.join(" / "));
    assert_eq!(double.rows(Mode::BEAT_14K).map(|rows| rows.len()), Some(2));
}

/// A run that lays its notes out one by one, played on OFF, played by AUTOPLAY, watched as a replay,
/// a course stage or a practice run, or of a mode LAYOUT PICK cannot lay out, is not captured, each
/// for a reason of its own.
#[test]
fn a_run_that_is_no_lane_order_of_the_players_is_not_captured() {
    let refused = |run: CapturedRun<'_>, kind: RunKind| run.capture(kind).err();
    for option in [NoteOption::SRandom, NoteOption::HRandom, NoteOption::AllScratch] {
        assert_eq!(refused(run(option, 1), RunKind::default()), Some(CaptureRefusal::NoteByNote(option.label().to_string())), "{option:?}");
    }
    assert_eq!(refused(run(NoteOption::Off, 1), RunKind::default()), Some(CaptureRefusal::Off));
    let random = || run(NoteOption::Random, SEED_3517246);
    assert_eq!(refused(random(), RunKind { plays_itself: true, ..RunKind::default() }), Some(CaptureRefusal::Autoplay));
    assert_eq!(refused(random(), RunKind { replay: true, ..RunKind::default() }), Some(CaptureRefusal::Replay));
    assert_eq!(refused(random(), RunKind { course_or_practice: true, ..RunKind::default() }), Some(CaptureRefusal::Course));
    let keyboard = CapturedRun { mode: Mode::KEYBOARD_24K, ..random() };
    assert_eq!(refused(keyboard, RunKind::default()), Some(CaptureRefusal::Mode(Mode::KEYBOARD_24K.name.to_string())));
}

/// Each capture is kept once by its chart, arrangement and seed, newest first; the file is written
/// at once and reads back the same.
#[test]
fn a_capture_is_kept_once_newest_first_and_survives_its_file() {
    let dir = temp_dir("kept");
    let path = captured_layouts_path(&dir.join("settings.ron"));
    assert_eq!(path, dir.join(CAPTURED_LAYOUTS_FILE));
    let mut kept = CapturedLayouts::load(&path);
    assert!(kept.all().is_empty(), "a fresh install has captures");
    assert_eq!(kept.keep(captured(NoteOption::Random, SEED_3517246), EARLIER_MS), Kept::Added);
    assert_eq!(kept.keep(captured(NoteOption::Mirror, 1), LATER_MS), Kept::Added);
    let again = CapturedLayout { ex_score: 1, md5: MD5.to_ascii_uppercase(), random_label: "random".to_string(), ..captured(NoteOption::Random, SEED_3517246) };
    assert_eq!(kept.keep(again, LATER_MS), Kept::AlreadyKept, "the same layout of the chart was kept twice");
    let other_chart = CapturedLayout { md5: OTHER_MD5.to_string(), ..captured(NoteOption::Random, SEED_3517246) };
    assert_eq!(kept.keep(other_chart, LATER_MS), Kept::Added, "another chart's layout is its own");
    let labels: Vec<(&str, i64)> = kept.all().iter().map(|capture| (capture.random_label.as_str(), capture.captured_at)).collect();
    assert_eq!(labels, [("RANDOM", LATER_MS), ("MIRROR", LATER_MS), ("RANDOM", EARLIER_MS)]);
    assert_eq!(kept.all()[2].ex_score, 1234, "the capture kept first was written over");
    assert_eq!(kept.of_chart(&MD5.to_ascii_uppercase()).len(), 2);
    let back = CapturedLayouts::load(&path);
    assert_eq!(back, kept);
    assert_eq!(CapturedLayouts::beside(&dir.join("settings.ron")), kept);
}

/// The seeds R-RANDOM shifts seven keys by the same lane from, and a seed it shifts them otherwise from.
const RRANDOM_SEEDS_ALIKE: [u64; 2] = [1, 3];
const RRANDOM_SEED_OTHERWISE: u64 = 2;

/// A second capture of a layout kept already is that layout captured again, whatever seed its run
/// was laid out from: MIRROR and ROTATE lay a chart out alike from every seed, and R-RANDOM lays it
/// out in one of a few shifts. A seed that lays the chart out otherwise is a capture of its own.
#[test]
fn a_layout_kept_already_is_kept_once_whatever_seed_laid_it_out() {
    let [alike, also_alike] = RRANDOM_SEEDS_ALIKE;
    assert_eq!(captured(NoteOption::RRandom, alike).layout, captured(NoteOption::RRandom, also_alike).layout, "the seeds shift the lanes alike");
    assert_ne!(captured(NoteOption::RRandom, alike).layout, captured(NoteOption::RRandom, RRANDOM_SEED_OTHERWISE).layout);
    let dir = temp_dir("alike");
    let mut kept = CapturedLayouts::load(&dir.join(CAPTURED_LAYOUTS_FILE));
    assert_eq!(kept.keep(captured(NoteOption::Mirror, 1), EARLIER_MS), Kept::Added);
    assert_eq!(kept.keep(captured(NoteOption::Mirror, 2), LATER_MS), Kept::AlreadyKept, "MIRROR from another seed");
    assert_eq!(kept.keep(captured(NoteOption::Rotate, 1), EARLIER_MS), Kept::Added);
    assert_eq!(kept.keep(captured(NoteOption::Rotate, 2), LATER_MS), Kept::AlreadyKept, "ROTATE from another seed");
    assert_eq!(kept.keep(captured(NoteOption::RRandom, alike), EARLIER_MS), Kept::Added);
    assert_eq!(kept.keep(captured(NoteOption::RRandom, also_alike), LATER_MS), Kept::AlreadyKept, "R-RANDOM shifted alike");
    assert_eq!(kept.keep(captured(NoteOption::RRandom, RRANDOM_SEED_OTHERWISE), LATER_MS), Kept::Added, "R-RANDOM shifted otherwise");
    let unwritten = CapturedLayout { layout: Vec::new(), ..captured(NoteOption::Mirror, 3) };
    assert_eq!(kept.keep(unwritten, LATER_MS), Kept::Added, "a hand-written capture with no lane order is told apart by its seed");
    assert!(kept.remove(&captured(NoteOption::Mirror, 2)), "the layout kept is not the one removed");
    assert_eq!(kept.all().len(), 4);
}

/// A capture removed is gone from the file at once; one that is not kept removes nothing.
#[test]
fn a_capture_is_removed_from_its_file() {
    let dir = temp_dir("removed");
    let path = dir.join(CAPTURED_LAYOUTS_FILE);
    let mut kept = CapturedLayouts::load(&path);
    kept.keep(captured(NoteOption::Random, SEED_3517246), EARLIER_MS);
    kept.keep(captured(NoteOption::Mirror, 1), LATER_MS);
    assert!(kept.remove(&captured(NoteOption::Random, SEED_3517246)));
    assert!(!kept.remove(&captured(NoteOption::Random, SEED_3517246)), "a capture was removed twice");
    let back = CapturedLayouts::load(&path);
    assert_eq!(back.all().iter().map(|capture| capture.random_label.as_str()).collect::<Vec<_>>(), ["MIRROR"]);
}

/// A file that is not there yet is no captures, and so is one that cannot be read -- a hand-edited
/// or cut-off file never keeps a screen from opening -- and the next capture writes it over.
#[test]
fn a_file_that_is_missing_or_broken_is_no_captures() {
    let dir = temp_dir("broken");
    let path = dir.join(CAPTURED_LAYOUTS_FILE);
    assert!(CapturedLayouts::load(&path).all().is_empty());
    std::fs::write(&path, b"(layouts: [(md5: \"cut off").expect("a file is written");
    let mut kept = crate::notify::exclusive(|| {
        let kept = CapturedLayouts::load(&path);
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        assert!(said.iter().any(|(_, line)| line.starts_with("captured layouts not loaded")), "{said:?}");
        kept
    });
    assert!(kept.all().is_empty());
    kept.keep(captured(NoteOption::Random, SEED_3517246), EARLIER_MS);
    assert_eq!(CapturedLayouts::load(&path).all().len(), 1, "the broken file was not written over");
    std::fs::write(&path, b"(layouts: [(md5: \"aa\", seed: 7)])").expect("a file is written");
    let sparse = CapturedLayouts::load(&path);
    assert_eq!(sparse.all().first().map(|capture| (capture.md5.as_str(), capture.seed, capture.random_label.as_str())), Some(("aa", 7, "")));
}

/// Bytes that are no text at all are a file that cannot be read, and are said to be, as a file cut
/// off is; only a file that is not there yet is no captures without a word.
#[test]
fn a_file_that_is_no_text_is_said_not_to_be_read() {
    let dir = temp_dir("not-text");
    let path = dir.join(CAPTURED_LAYOUTS_FILE);
    std::fs::write(&path, [0xFF, 0xFE, 0x00, 0x80]).expect("a file is written");
    let said = crate::notify::exclusive(|| {
        let kept = CapturedLayouts::load(&path);
        assert!(kept.all().is_empty());
        let _ = CapturedLayouts::load(&dir.join("missing.ron"));
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        said
    });
    let lines: Vec<&str> = said.iter().map(|(_, line)| line.as_str()).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("captured layouts not loaded"), "{lines:?}");
}
