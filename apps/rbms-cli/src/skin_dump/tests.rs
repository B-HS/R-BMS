use std::path::{Path, PathBuf};
use std::time::Duration;

use rbms_skin::dst::DrawStateSource;
use rbms_skin::loader::{
    SKIN_TYPE_COURSE_RESULT, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG, SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_5KEYS, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_PLAY_9KEYS,
    SKIN_TYPE_PLAY_10KEYS, SKIN_TYPE_PLAY_14KEYS, SKIN_TYPE_PLAY_24KEYS, SKIN_TYPE_PLAY_24KEYS_BATTLE, SKIN_TYPE_RESULT, SKIN_TYPE_SKIN_SELECT,
};
use rbms_skin::property::{MapHost, SkinHost, StaticScreen};

use super::args::{DEFAULT_FRAMES, DEFAULT_SEED, MAX_FRAMES, parse};
use super::frames::percentile;
use super::probe::smallest;
use super::report::{DocumentReport, HeaderReport};
use super::scenario::{SCENARIO_KEYS, Scenario, Screen};
use super::{DumpArgs, documents, dump, gate, pack, render, temporary_overlay};

const SCRATCH_FRAMES: usize = 20;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

fn mini_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/rbms-skin/tests/fixtures/luaskin/mini")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-cli-skin-dump-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch folder should be creatable");
        Self(path)
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::write(&path, text).expect("the scratch file should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dump_args(target: &Path) -> DumpArgs {
    DumpArgs {
        target: target.to_path_buf(),
        scenario: None,
        overlay: None,
        frames: DEFAULT_FRAMES,
        seed: DEFAULT_SEED,
        json: false,
        probe_budget: false,
        strict: false,
        allowed_failures: Vec::new(),
    }
}

fn document<'a>(documents: &'a [DocumentReport], file: &str) -> &'a DocumentReport {
    documents.iter().find(|document| document.file == file).unwrap_or_else(|| panic!("{file} should have been dumped"))
}

#[test]
fn a_folder_alone_is_enough_and_the_options_default_to_a_static_dump() {
    let parsed = parse(&args(&["/skins/pack"])).expect("a folder alone is enough");
    assert_eq!(parsed.target.to_str(), Some("/skins/pack"));
    assert_eq!((parsed.frames, parsed.seed), (DEFAULT_FRAMES, DEFAULT_SEED));
    assert!(parsed.scenario.is_none() && parsed.overlay.is_none() && !parsed.json && !parsed.probe_budget);
    assert!(!parsed.strict && parsed.allowed_failures.is_empty(), "a dump is only read unless it is asked to be a gate");
}

#[test]
fn every_option_is_read_wherever_it_stands() {
    let parsed = parse(&args(&["--frames", "1000", "/p", "--scenario", "s.json", "--overlay", "/ov", "--seed", "9", "--json", "--probe-budget"])).unwrap();
    assert_eq!(parsed.frames, 1000);
    assert_eq!(parsed.seed, 9);
    assert_eq!(parsed.scenario.as_deref(), Some(Path::new("s.json")));
    assert_eq!(parsed.overlay.as_deref(), Some(Path::new("/ov")));
    assert_eq!(parsed.target, Path::new("/p"));
    assert!(parsed.json && parsed.probe_budget);
}

#[test]
fn a_missing_target_value_or_unknown_option_is_refused() {
    assert!(parse(&args(&[])).is_err(), "the target is required");
    assert!(parse(&args(&["/p", "--frames"])).is_err(), "--frames needs a value");
    assert!(parse(&args(&["/p", "--frames", "many"])).is_err(), "--frames needs a number");
    assert!(parse(&args(&["/p", "--seed", "-1"])).is_err(), "--seed needs an unsigned number");
    assert!(parse(&args(&["/p", "--nope"])).is_err(), "unknown options are rejected");
    assert!(parse(&args(&["/p", "/q"])).is_err(), "only one target is accepted");
    assert!(parse(&args(&["/p", "--allow-failure"])).is_err(), "--allow-failure needs a document");
}

#[test]
fn more_frames_than_can_be_kept_are_refused_rather_than_asked_of_the_allocator() {
    let most = MAX_FRAMES.to_string();
    assert_eq!(parse(&args(&["/p", "--frames", &most])).map(|parsed| parsed.frames), Ok(MAX_FRAMES));
    let over = (MAX_FRAMES + 1).to_string();
    let error = parse(&args(&["/p", "--frames", &over])).expect_err("one frame over the most is refused");
    assert!(error.contains(&most), "the error does not say how many are taken: {error}");
    let widest = usize::MAX.to_string();
    assert!(parse(&args(&["/p", "--frames", &widest])).is_err(), "the widest count there is was accepted");
}

#[test]
fn a_gate_is_asked_for_by_name_or_by_naming_what_may_fail() {
    let strict = parse(&args(&["/p", "--strict"])).unwrap();
    assert!(strict.strict && strict.allowed_failures.is_empty());
    let allowing = parse(&args(&["--allow-failure", "a.luaskin", "/p", "--allow-failure", "b.luaskin"])).unwrap();
    assert!(allowing.strict, "naming a document that may fail did not ask for a gate");
    assert_eq!(allowing.allowed_failures, ["a.luaskin", "b.luaskin"]);
}

#[test]
fn a_skin_type_picks_the_screen_whose_scenario_it_is_dumped_with() {
    for play in [
        SKIN_TYPE_PLAY_7KEYS,
        SKIN_TYPE_PLAY_5KEYS,
        SKIN_TYPE_PLAY_14KEYS,
        SKIN_TYPE_PLAY_10KEYS,
        SKIN_TYPE_PLAY_9KEYS,
        SKIN_TYPE_PLAY_24KEYS,
        SKIN_TYPE_PLAY_24KEYS_BATTLE,
    ] {
        assert_eq!(Screen::of(play), Screen::Play, "type {play}");
    }
    assert_eq!(Screen::of(SKIN_TYPE_MUSIC_SELECT), Screen::Select);
    assert_eq!(Screen::of(SKIN_TYPE_DECIDE), Screen::Decide);
    assert_eq!(Screen::of(SKIN_TYPE_RESULT), Screen::Result);
    assert_eq!(Screen::of(SKIN_TYPE_COURSE_RESULT), Screen::CourseResult);
    assert_eq!(Screen::of(SKIN_TYPE_KEY_CONFIG), Screen::Other);
    assert_eq!(Screen::of(SKIN_TYPE_SKIN_SELECT), Screen::Other);
}

#[test]
fn every_built_in_scenario_turns_exactly_the_first_difficulty_on() {
    for screen in [Screen::Play, Screen::Select, Screen::Decide, Screen::Result, Screen::CourseResult, Screen::Other] {
        let host = Scenario::builtin(screen).expect("the built-in scenario should parse").host().expect("and describe a host");
        assert_eq!(host.boolean(150), Some(true), "{screen:?}");
        assert!((151..=155).all(|id| host.boolean(id) == Some(false)), "{screen:?}: the other five difficulties are off");
        assert!(!host.text(10).is_empty() && host.integer(96) > 0, "{screen:?}: the chart's title and level are filled in");
        assert_eq!(host.boolean(171), Some(true), "{screen:?}: the chart has a BGA");
    }
}

#[test]
fn a_screens_scenario_is_laid_over_the_base_and_not_instead_of_it() {
    let course = Scenario::builtin(Screen::CourseResult).unwrap().host().unwrap();
    assert_eq!(course.static_screen, Some(StaticScreen::Result));
    assert!(!course.text(150).is_empty(), "a course result names its stages");
    assert!(!course.text(10).is_empty(), "and keeps the base's chart title");

    let result = Scenario::builtin(Screen::Result).unwrap().host().unwrap();
    assert_eq!(result.static_screen, Some(StaticScreen::Result));
    assert!(result.text(150).is_empty(), "a single result has no course stage titles");
    assert_eq!(result.integer(370), 5, "and a clear type, which the result screen reads while it builds");

    let select = Scenario::builtin(Screen::Select).unwrap().host().unwrap();
    assert_eq!(select.static_screen, Some(StaticScreen::Select));
    let play = Scenario::builtin(Screen::Play).unwrap().host().unwrap();
    assert_eq!(play.static_screen, Some(StaticScreen::Other));
    assert_eq!(play.image_index(55), 3, "the high-speed fix a play skin's cover slider reads");
}

#[test]
fn a_scenario_file_is_read_whole_and_a_broken_one_names_itself() {
    let scratch = Scratch::new("scenario");
    let good = scratch.write("good.json", r#"{ "integers": { "71": 33 }, "texts": { "10": "Given" } }"#);
    let host = Scenario::from_file(&good).expect("a scenario in MapHost's shape").host().unwrap();
    assert_eq!((host.integer(71), host.text(10).into_owned()), (33, "Given".to_owned()));
    assert_eq!(host.boolean(150), None, "a given scenario is not laid over the built-in ones");

    let bad = scratch.write("bad.json", "{ not json");
    let error = Scenario::from_file(&bad).expect_err("a broken file is refused");
    assert!(error.contains("bad.json"), "{error}");
    let wrong = scratch.write("wrong.json", r#"{ "integers": { "x": 1 } }"#);
    assert!(Scenario::from_file(&wrong).is_err(), "an id that is not a number is refused");
    assert!(Scenario::from_file(&scratch.0.join("absent.json")).is_err());
}

#[test]
fn a_scenario_of_the_wrong_shape_or_with_a_key_no_host_reads_is_refused() {
    let scratch = Scratch::new("scenario-shape");
    let misspelt = scratch.write("misspelt.json", r#"{ "options": { "150": true }, "integers": { "71": 1 }, "nope": 3 }"#);
    let error = Scenario::from_file(&misspelt).expect_err("a key no host reads is refused");
    assert!(error.contains("options") && error.contains("nope"), "the error does not name the keys: {error}");
    assert!(error.contains("booleans"), "the error does not say what the keys are: {error}");
    assert!(!error.contains("integers, nope") && !error.contains("options, integers"), "a known key was reported as unknown: {error}");

    for (file, text) in [("list.json", "[]"), ("number.json", "3"), ("text.json", r#""booleans""#), ("null.json", "null")] {
        let error = Scenario::from_file(&scratch.write(file, text)).expect_err("a scenario that is not an object is refused");
        assert!(error.contains(file) && error.contains("JSON object"), "{error}");
    }
    assert!(Scenario::from_file(&scratch.write("empty.json", "{}")).is_ok(), "an empty scenario is every table empty");
}

#[test]
fn every_scenario_key_is_one_the_host_reads() {
    for key in SCENARIO_KEYS {
        let document = serde_json::json!({ key: true });
        let read: Result<MapHost, _> = serde_json::from_value(document);
        assert!(read.is_err(), "{key} took a value none of the host's tables holds, so the host does not read it");
    }
}

#[test]
fn the_ninety_ninth_percentile_is_the_nearest_rank() {
    let ascending: Vec<Duration> = (1..=100).map(Duration::from_millis).collect();
    assert_eq!(percentile(&ascending, (99, 100)), Duration::from_millis(99));
    assert_eq!(percentile(&ascending, (100, 100)), Duration::from_millis(100));
    assert_eq!(percentile(&[Duration::from_millis(7)], (99, 100)), Duration::from_millis(7));
    assert_eq!(percentile(&[], (99, 100)), Duration::ZERO, "no frames, no time");
}

#[test]
fn the_smallest_allowance_is_found_within_a_twentieth() {
    const NEED: u64 = 123_456;
    let mut asked = Vec::new();
    let found = smallest(0, 500_000_000, |allowance| {
        asked.push(allowance);
        allowance >= NEED
    })
    .expect("the ceiling passes");
    assert!((NEED..=NEED + NEED / 20 + 1).contains(&found), "{found}");
    assert!(asked.len() < 40, "a bisection, not a scan: {} asks", asked.len());

    assert_eq!(smallest(0, 10, |_| false), None, "nothing passes");
    assert_eq!(smallest(5, 10, |_| true), Some(5), "the floor passes");
}

#[test]
fn a_pack_folder_gives_its_skin_documents_and_a_file_gives_itself() {
    let (root, found) = documents(&mini_root()).expect("the fixture folder");
    let names: Vec<_> = found.iter().filter_map(|path| path.file_name()).map(|name| name.to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["config.luaskin", "course.luaskin", "nonote.luaskin", "play.luaskin"]);

    let (alone_root, alone) = documents(&root.join("play.luaskin")).expect("one document");
    assert_eq!((alone_root, alone.len()), (root, 1));

    let scratch = Scratch::new("empty");
    scratch.write("notes.txt", "not a skin");
    assert!(documents(&scratch.0).expect_err("no skin in it").contains("no .luaskin"));
    assert!(documents(&scratch.0.join("absent")).is_err());
}

#[test]
fn the_fixture_pack_is_dumped_header_body_frames_and_the_pack_is_left_alone() {
    let overlay = Scratch::new("overlay");
    let report = dump(&DumpArgs { overlay: Some(overlay.0.clone()), frames: SCRATCH_FRAMES, ..dump_args(&mini_root()) }).expect("the fixture dumps");
    assert_eq!(report.documents.len(), 4);
    assert!(report.pack.unchanged(), "the pack changed: {:?}", report.pack.changed);
    assert!(report.pack.files_before > 0 && report.pack.files_before == report.pack.files_after);
    assert!(overlay.0.join("log/loaded.txt").is_file(), "the skin's write went to the overlay");

    let play = document(&report.documents, "play.luaskin");
    let header = play.header.as_ref().expect("the header pass reads");
    assert_eq!((header.skin_type, header.width, header.height, header.parser.as_str()), (0, 1920, 1080, "lua"));
    assert_eq!((header.options.len(), HeaderReport::defaults(&header.options)), (2, 1), "two option rows, one of them with a default");
    assert_eq!((header.file_slots.len(), HeaderReport::defaults(&header.file_slots)), (2, 2));
    assert_eq!((header.offsets.len(), header.automatic_offsets), (5, 4), "one declared and the four every play skin gains");
    assert!(play.scenario.contains("play"), "{}", play.scenario);

    let body = play.body.as_ref().expect("the body loads");
    let count = |kind: &str| body.objects.iter().find(|entry| entry.kind == kind).map_or(0, |entry| entry.count);
    assert_eq!((count("image"), count("value"), count("note")), (3, 1, 1));
    assert!(body.destinations >= body.destinations_assembled && body.destinations_assembled > 0);
    let kinds = |kind: &str| body.function_kinds.iter().find(|entry| entry.kind == kind).map_or(0, |entry| entry.count);
    assert_eq!((body.functions, kinds("boolean"), kinds("integer"), kinds("timer")), (3, 1, 1, 1));
    assert_eq!(body.diagnostics.swallowed.len(), 1, "the broken part the skin wrapped in pcall");
    assert!(body.diagnostics.swallowed[0].text.contains("broken.lua"), "{}", body.diagnostics.swallowed[0].text);
    assert!(body.diagnostics.function_failures.is_empty());
    assert!(body.lua_memory_bytes.is_some_and(|bytes| bytes > 0));
    for file in body.files.sources.iter().chain(&body.files.fonts) {
        assert_eq!(file.exists, mini_root().join(&file.path).is_file(), "{} is reported as it is on disk", file.path);
    }

    let frames = play.frames.as_ref().expect("frames were asked for");
    assert_eq!((frames.frames, frames.calls_per_frame, frames.frames_over_budget), (SCRATCH_FRAMES, 3, 0));
    assert!(frames.function_failures.is_empty() && frames.swallowed_during_frames == 0);
    assert!(frames.mean_micros > 0.0 && frames.mean_micros <= frames.max_micros);
    assert!(frames.median_micros <= frames.p99_micros && frames.p99_micros <= frames.max_micros);
    assert_eq!(frames.slowest_calls.len(), 3, "every function ranks when there are fewer than five");

    for other in ["config.luaskin", "course.luaskin", "nonote.luaskin"] {
        assert!(document(&report.documents, other).body.is_some(), "{other} loads too");
    }
    assert!(document(&report.documents, "course.luaskin").scenario.contains("course_result"));
}

#[test]
fn a_document_alone_is_dumped_against_its_folder_and_a_given_scenario_replaces_the_built_in_one() {
    let scratch = Scratch::new("given");
    let given = scratch.write("given.json", r#"{ "integers": { "71": 5 } }"#);
    let report = dump(&DumpArgs { scenario: Some(given.clone()), ..dump_args(&mini_root().join("play.luaskin")) }).expect("one document dumps");
    assert_eq!(report.documents.len(), 1);
    assert_eq!(report.documents[0].scenario, given.display().to_string());
    assert!(report.documents[0].frames.is_none(), "no frames unless asked for");
    assert!(report.pack.unchanged());
}

#[test]
fn a_skin_that_cannot_load_is_reported_with_its_error_and_the_rest_still_dump() {
    let scratch = Scratch::new("broken");
    scratch.write("broken.luaskin", "return { type = 5, w = 1280, h = 720,");
    scratch.write("empty.luaskin", "return {}");
    let report = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&scratch.0) }).expect("a broken skin does not stop the dump");
    let broken = document(&report.documents, "broken.luaskin");
    assert!(broken.body.is_none() && broken.frames.is_none());
    assert!(broken.header_error.is_some() && broken.load_error.is_some());
    let empty = document(&report.documents, "empty.luaskin");
    assert!(empty.header_error.is_some(), "a program that returns no header is not a skin");

    let text = render::text(&report);
    assert!(text.contains("FAILED"), "{text}");
    assert!(text.contains("broken.luaskin") && text.contains("summary"), "{text}");
}

#[test]
fn a_strict_dump_fails_on_what_was_not_allowed_to_fail_and_on_what_was_and_did_not() {
    let scratch = Scratch::new("strict");
    scratch.write("broken.luaskin", "return { type = 5, w = 1280, h = 720,");
    scratch.write("plain.json", r#"{ "type": 6, "name": "Plain", "w": 1280, "h": 720 }"#);
    let report = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&scratch.0) }).expect("the folder dumps");
    let allowing = |names: &[&str]| gate::findings(&report, &args(names));

    let unallowed = allowing(&[]);
    assert_eq!(unallowed.len(), 1, "{unallowed:?}");
    assert!(unallowed[0].contains("broken.luaskin") && unallowed[0].contains("did not load"), "{unallowed:?}");

    assert!(allowing(&["broken.luaskin"]).is_empty(), "the one failure there is was allowed: {:?}", allowing(&["broken.luaskin"]));

    let wrong = allowing(&["broken.luaskin", "plain.json"]);
    assert_eq!(wrong.len(), 1, "{wrong:?}");
    assert!(wrong[0].contains("plain.json") && wrong[0].contains("loaded"), "{wrong:?}");

    let absent = allowing(&["broken.luaskin", "absent.luaskin"]);
    assert_eq!(absent.len(), 1, "{absent:?}");
    assert!(absent[0].contains("absent.luaskin"), "{absent:?}");
}

#[test]
fn a_strict_dump_fails_on_what_a_skin_caught_and_on_a_function_that_fails() {
    let caught = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&mini_root().join("play.luaskin")) }).expect("the fixture dumps");
    let findings = gate::findings(&caught, &[]);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("play.luaskin") && findings[0].contains("pcall"), "{findings:?}");

    let scratch = Scratch::new("strict-failing");
    scratch.write(
        "fail.luaskin",
        "local skin = { type = 5, name = 'fail', w = 1280, h = 720 } \
         if skin_config then \
         skin.image = { { id = 'a', src = 0, x = 0, y = 0, w = 1, h = 1 } } \
         skin.destination = { { id = 'a', draw = function() error('boom') end, dst = { { x = 0, y = 0, w = 1, h = 1 } } } } \
         end \
         return skin",
    );
    let failing = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&scratch.0) }).expect("the skin dumps");
    let findings = gate::findings(&failing, &[]);
    assert!(findings.iter().any(|finding| finding.contains("fail.luaskin") && finding.contains("during the frames")), "{findings:?}");
}

#[test]
fn a_file_rewritten_at_the_same_length_is_a_change_to_the_pack() {
    const LATER: Duration = Duration::from_secs(90);
    let scratch = Scratch::new("pack-stamp");
    let file = scratch.write("part.lua", "return 1");
    let before = pack::list(&scratch.0);
    assert!(pack::compare(&scratch.0, &before).unchanged(), "a folder nobody touched reads as changed");

    std::fs::write(&file, "return 2").expect("the scratch file should be writable");
    let written = std::fs::File::options().write(true).open(&file).expect("the scratch file opens");
    let stamped = written.metadata().and_then(|metadata| metadata.modified()).expect("the scratch file has a modification time") + LATER;
    written.set_modified(stamped).expect("the scratch file's time can be set");
    drop(written);

    let check = pack::compare(&scratch.0, &before);
    assert_eq!(check.changed, ["part.lua"], "a rewrite that kept the length went unnoticed");
    assert_eq!((check.files_before, check.files_after), (1, 1));
}

#[test]
fn no_two_dumps_of_one_process_share_a_temporary_overlay() {
    let (first, second) = (temporary_overlay(), temporary_overlay());
    assert_ne!(first, second);
    assert!(first.starts_with(std::env::temp_dir()) && second.starts_with(std::env::temp_dir()));
}

#[test]
fn a_function_that_fails_every_frame_is_counted_every_frame() {
    let scratch = Scratch::new("failing");
    scratch.write(
        "fail.luaskin",
        "local skin = { type = 5, name = 'fail', w = 1280, h = 720 } \
         if skin_config then \
         skin.source = { { id = 0, path = 'a.png' } } \
         skin.image = { { id = 'a', src = 0, x = 0, y = 0, w = 1, h = 1 } } \
         skin.destination = { { id = 'a', draw = function() error('boom') end, dst = { { x = 0, y = 0, w = 1, h = 1 } } } } \
         end \
         return skin",
    );
    let report = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&scratch.0) }).expect("the skin dumps");
    let fail = document(&report.documents, "fail.luaskin");
    let frames = fail.frames.as_ref().expect("the frames ran");
    assert_eq!(frames.calls_per_frame, 1);
    assert_eq!(frames.function_failures.len(), 1);
    assert_eq!(frames.function_failures[0].count, u64::try_from(SCRATCH_FRAMES).unwrap());
    assert!(frames.function_failures[0].text.contains("boom"), "{}", frames.function_failures[0].text);
    let body = fail.body.as_ref().unwrap();
    assert_eq!(body.files.sources.iter().filter(|file| !file.exists).count(), 1, "the source the skin names is not there");
}

#[test]
fn a_json_document_is_dumped_like_a_lua_one_and_other_files_are_not_documents() {
    let scratch = Scratch::new("json");
    scratch.write("plain.json", r#"{ "type": 6, "name": "Plain", "w": 1280, "h": 720 }"#);
    scratch.write("readme.md", "not a skin");
    let report = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&scratch.0) }).expect("the folder dumps");
    assert_eq!(report.documents.len(), 1, "only the .json is a document");
    let plain = &report.documents[0];
    assert_eq!(plain.header.as_ref().map(|header| header.parser.as_str()), Some("json"));
    assert!(plain.scenario.contains("decide"), "{}", plain.scenario);
    assert!(plain.body.is_some() && plain.frames.is_some(), "{:?}", plain.load_error);
}

#[test]
fn the_budget_probe_finds_a_light_skin_far_under_the_defaults() {
    let report = dump(&DumpArgs { probe_budget: true, ..dump_args(&mini_root().join("play.luaskin")) }).expect("the fixture dumps");
    let probe = report.documents[0].probe.as_ref().expect("the probe was asked for");
    assert!(probe.load_instructions.is_some_and(|needed| needed < probe.load_instructions_limit / 100), "{probe:?}");
    assert!(probe.call_instructions.is_some_and(|needed| needed < probe.call_instructions_limit / 100), "{probe:?}");
}

#[test]
fn the_report_is_text_for_people_and_json_for_machines() {
    let report = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&mini_root().join("play.luaskin")) }).expect("the fixture dumps");
    let text = render::text(&report);
    for expected in [
        "== play.luaskin ==",
        "type 0 (play 7 keys)",
        "timings     scene ",
        "functions   3 registered",
        "frames      20 frames x 3 calls",
        "summary",
        "unchanged",
    ] {
        assert!(text.contains(expected), "{expected:?} is missing from\n{text}");
    }

    let json = serde_json::to_value(&report).expect("the report serialises");
    let document = &json["documents"][0];
    assert_eq!(document["file"], "play.luaskin");
    assert_eq!(document["header"]["skin_type"], 0);
    assert_eq!(document["body"]["functions"], 3);
    for timing in ["scene", "input", "fadeout", "loadend", "playstart", "close"] {
        assert!(document["body"]["timings"][timing].is_number(), "the header's {timing} is not in the report");
    }
    assert_eq!(document["frames"]["calls_per_frame"], 3);
    assert_eq!(json["pack"]["changed"].as_array().map(Vec::len), Some(0));
}

#[test]
fn an_external_skin_pack_loads_every_screen_but_the_one_its_author_left_broken() {
    const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";
    const KNOWN_BROKEN_ENTRY: &str = "keyconfig.luaskin";
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let report = dump(&DumpArgs { frames: SCRATCH_FRAMES, ..dump_args(&pack) }).expect("the pack dumps");
    assert!(report.pack.unchanged(), "the dump changed the pack: {:?}", report.pack.changed);
    for document in &report.documents {
        let Some(body) = &document.body else {
            assert_eq!(document.file, KNOWN_BROKEN_ENTRY, "a screen that should load did not: {:?}", document.load_error);
            continue;
        };
        assert!(body.diagnostics.swallowed.is_empty(), "{}: a part failed inside the skin's own pcall: {:?}", document.file, body.diagnostics.swallowed);
        assert!(body.diagnostics.function_failures.is_empty(), "{}: {:?}", document.file, body.diagnostics.function_failures);
        let frames = document.frames.as_ref().expect("frames were asked for");
        assert!(frames.function_failures.is_empty(), "{}: {:?}", document.file, frames.function_failures);
        assert_eq!(frames.frames_over_budget, 0, "{}: the budget cut a call off", document.file);
    }
}
