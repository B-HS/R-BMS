//! The request bodies this client sends the score server, taken from runs that are really played
//! and kept in `contract/client-requests/` at the root of the repository, where the web tests pass
//! every one of them through the server's own checks (`web/tests/dto/client-requests.test.ts`).
//!
//! A body is taken off the wire: the app is signed in to a score server on loopback, a chart is
//! judged by the engine against inputs given at the times written here, and the run goes the way
//! the play screen sends it: the result screen's submission and the replay after it, a course's
//! submission, the records sync, the settings upload. What the server receives is the file, but for
//! the few values that are the moment or the machine (the clock, the build, the platform, the
//! version), which are checked for their shape and then written as fixed values.
//!
//! Each test compares the body with the file it is committed as and fails when they differ. With
//! `RBMS_UPDATE_CONTRACT=1` it writes the file instead; the change then goes through the web tests.

mod wire;

use std::collections::BTreeSet;
use std::path::PathBuf;

use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;

use self::wire::{CaptureServer, Json, MAX_SAFE_INTEGER};
use super::*;

/// The environment variable that has the tests write the contract files rather than compare them.
const UPDATE_ENV: &str = "RBMS_UPDATE_CONTRACT";

/// What a failing comparison tells a person to do.
const UPDATE_HINT: &str = "if the change is meant, run `RBMS_UPDATE_CONTRACT=1 cargo test -p rbms-player --locked contract_requests`, then \
     `bun test tests/dto/client-requests.test.ts` in web/, and commit the files with the change";

/// Every file the contract folder holds, one per test below.
const CONTRACT_FILES: [&str; 33] = [
    "course-normal",
    "course-scratch-auto",
    "replay-long",
    "replay-random",
    "score-14k-2p",
    "score-abandoned",
    "score-autoplay",
    "score-bad",
    "score-charge-note",
    "score-empty-poor-exceeds-poor",
    "score-failed",
    "score-hell-charge-note",
    "score-long-note",
    "score-max-with-empty-poor",
    "score-normal",
    "score-popn-9k",
    "score-scratch-auto",
    "score-scratch-auto-hard",
    "score-sync-resend",
    "score-trace-normal",
    "score-widened-judge",
    "settings-blob",
    "trace-abandoned",
    "trace-bad",
    "trace-charge-note",
    "trace-empty-poor-exceeds-poor",
    "trace-failed",
    "trace-hell-charge-note",
    "trace-long-note",
    "trace-normal",
    "trace-popn-9k",
    "trace-random",
    "trace-scratch-auto",
];

/// The clock a new run's layout seed is taken from, in nanoseconds: the one `app_play`'s own test
/// of `fresh_layout_seed` uses.
const LAYOUT_CLOCK_NS: u64 = 1_791_970_074_354_123_456;

/// The bits `fresh_layout_seed` keeps of that clock (`app_play::LAYOUT_SEED_MASK`), which makes the
/// seed it gives: a value just under 2^53, as large as a new run's seed gets.
const LAYOUT_SEED_BITS: u64 = (1 << 53) - 1;

/// The seed every run here is laid out with, as `AppShared::load` lays out a run that is new.
const LAYOUT_SEED: u64 = LAYOUT_CLOCK_NS & LAYOUT_SEED_BITS;

/// The moment a body is written as having been played or saved, in milliseconds: the same instant
/// as [`LAYOUT_CLOCK_NS`].
const FIXED_MOMENT_MS: i64 = 1_791_970_074_354;

/// What a body's own client, build and platform are written as.
const FIXED_CLIENT: &str = "rbms/0.0.0";
const FIXED_BUILD_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const FIXED_PLATFORM: &str = "macos-aarch64";

/// What a trace's link to its score is written as: the id the capturing server numbers by the
/// requests it was sent, which the health checks the app makes on a timer of its own can move.
const FIXED_SCORE_ID: &str = "sc_contract";

/// The prefix every score id the capturing server answers with starts with.
const SCORE_ID_PREFIX: &str = "sc_";

/// What a build hash is: SHA-256, as lower-case hexadecimal.
const SHA256_HEX_LENGTH: usize = 64;

/// The header of the chart most runs here play: a title that is not ASCII, an artist and a level,
/// which the submission carries for a server that has not seen the chart.
const CONTRACT_HEADER: &str =
    "#PLAYER 1\n#GENRE CONTRACT\n#TITLE 契約の譜面 [ANOTHER]\n#ARTIST rbms contract\n#BPM 150\n#PLAYLEVEL 12\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n";

/// The seven keys and the turntable of the first player, in the order the stepped charts walk them.
const SINGLE_CHANNELS: [&str; 8] = ["11", "12", "13", "16", "14", "15", "18", "19"];

/// The same for both players of a fourteen-key chart.
const DOUBLE_CHANNELS: [&str; 16] = ["11", "21", "12", "22", "13", "16", "23", "26", "14", "24", "15", "25", "18", "28", "19", "29"];

/// How long the contract chart is, and how many notes a measure of it has.
const CONTRACT_MEASURES: usize = 4;
const CONTRACT_SLOTS: usize = 8;

/// The fourteen-key chart.
const DOUBLE_HEADER: &str = "#PLAYER 3\n#TITLE Contract Double\n#ARTIST rbms contract\n#BPM 150\n#PLAYLEVEL 11\n#RANK 2\n#TOTAL 400\n#WAV01 a.wav\n";
const DOUBLE_MEASURES: usize = 4;
const DOUBLE_SLOTS: usize = 16;

/// The second chart of the course.
const SECOND_STAGE_HEADER: &str = "#PLAYER 1\n#TITLE Contract Stage Two\n#ARTIST rbms contract\n#BPM 170\n#PLAYLEVEL 10\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n";
const SECOND_STAGE_MEASURES: usize = 3;
const SECOND_STAGE_SLOTS: usize = 8;

/// The long chart whose replay is the large one: a minute and a half of sixteenth notes.
const LONG_HEADER: &str = "#PLAYER 1\n#TITLE Contract Long\n#ARTIST rbms contract\n#BPM 150\n#PLAYLEVEL 12\n#RANK 2\n#TOTAL 500\n#WAV01 a.wav\n";
const LONG_MEASURES: usize = 56;
const LONG_SLOTS: usize = 16;

/// The chart the long-note runs play: plain notes and long notes the chart leaves to LN MODE
/// (`#LNTYPE 1`, no `#LNMODE`), on keys of their own, and a turntable note.
const LONG_NOTE_CHART: &[u8] =
    b"#PLAYER 1\n#TITLE Contract Long Notes\n#ARTIST rbms contract\n#BPM 150\n#PLAYLEVEL 9\n#RANK 2\n#TOTAL 300\n#LNTYPE 1\n#WAV01 a.wav\n\
#00112:00010001\n#00151:01000100\n#00211:00100010\n#00253:01000001\n#00316:01000000\n#00314:01010000\n";

/// How late a long note's head is pressed: inside GREAT, so its timing is not nought.
const LONG_HEAD_LATE_US: i64 = 30_000;

/// A key note pressed this far from its time takes a BAD on the seven-key contract chart, late and
/// early, and late on the pop'n one.
const BAD_LATE_US: i64 = 180_000;
const BAD_EARLY_US: i64 = 160_000;
const POPN_BAD_LATE_US: i64 = 150_000;

/// Key notes pressed on their time, a BAD late, a little late, and a BAD early.
const BAD_OFFSETS_US: [Option<i64>; 4] = [Some(0), Some(BAD_LATE_US), Some(20_000), Some(-BAD_EARLY_US)];

/// The same for the pop'n chart: on their time, a BAD late, a little early, and late inside GOOD.
const POPN_OFFSETS_US: [Option<i64>; 4] = [Some(0), Some(POPN_BAD_LATE_US), Some(-20_000), Some(40_000)];

/// The nine-key chart, stepping through the nine buttons.
const POPN_HEADER: &str = "#PLAYER 1\n#TITLE Contract Popn\n#ARTIST rbms contract\n#BPM 150\n#PLAYLEVEL 20\n#RANK 2\n#TOTAL 300\n#WAV01 a.wav\n";
const POPN_CHANNELS: [&str; 9] = ["11", "12", "13", "14", "15", "22", "23", "24", "25"];
const POPN_MEASURES: usize = 3;
const POPN_SLOTS: usize = 9;

/// The course the course tests play, and the trophy a clean run of it earns.
const COURSE_NAME: &str = "CONTRACT COURSE";
const COURSE_TROPHY: &str = "goldmedal";
const COURSE_TROPHY_MISS_RATE: f32 = 5.0;
const COURSE_TROPHY_SCORE_RATE: f32 = 85.0;

/// Where in the library the course's second chart is listed.
const SECOND_STAGE_PATH: &str = "/songs/contract-stage-two.bms";

/// Every key note pressed on its time.
const ON_TIME_US: [Option<i64>; 1] = [Some(0)];

/// One key note in eight let go by, the rest on their time.
const ONE_IN_EIGHT_MISSED_US: [Option<i64>; 8] = [Some(0), Some(0), Some(0), Some(0), Some(0), Some(0), Some(0), None];

/// How far ahead of each key note a run that hunts for it presses its key: inside the 空POOR band
/// and outside BAD, so the press takes no note and the note is still there to hit.
const EARLY_PRESS_US: i64 = 300_000;

/// How far into the contract chart the left run is played before Escape: into its second measure.
const LEFT_AT_US: i64 = 3_000_000;

/// How long a test waits for a sync, a settings upload or a course submission to finish.
const WORK_WAIT: Duration = Duration::from_secs(20);

/// No waiting between the requests of a records sync.
const UNPACED: crate::record_sync::SyncPacing = crate::record_sync::SyncPacing {
    score_interval: Duration::ZERO,
    replay_interval: Duration::ZERO,
    trace_interval: Duration::ZERO,
    rate_limit_wait: Duration::ZERO,
    cancel_check: Duration::from_millis(1),
};

/// A chart of `measures` measures of `slots` notes each, after `header`, whose notes step through
/// `channels` in turn.
fn stepped_chart(header: &str, measures: usize, slots: usize, channels: &[&str]) -> Vec<u8> {
    let mut text = header.to_owned();
    for measure in 1..=measures {
        let first_step = (measure - 1) * slots;
        let lines: Vec<Vec<&str>> =
            (0..channels.len()).map(|lane| (0..slots).map(|slot| if (first_step + slot) % channels.len() == lane { "01" } else { "00" }).collect()).collect();
        for (channel, line) in channels.iter().zip(&lines) {
            if line.contains(&"01") {
                text.push_str(&format!("#{measure:03}{channel}:{}\n", line.concat()));
            }
        }
    }
    text.into_bytes()
}

/// `bytes` as the engine plays it, read the way a chart file is.
fn model_of(bytes: &[u8], name: &str) -> rbms_model::Model {
    let source = rbms_parser::parse_with(bytes, Default::default());
    rbms_chart::to_model(&source, rbms_chart::detect_mode(&source, name))
}

fn contract_model() -> rbms_model::Model {
    model_of(&stepped_chart(CONTRACT_HEADER, CONTRACT_MEASURES, CONTRACT_SLOTS, &SINGLE_CHANNELS), "contract.bms")
}

/// An app signed in to `server` under the test account, through the same rebuild a login ends with.
fn signed_in_to(server: &CaptureServer, tag: &str) -> AppShared {
    let mut config = Config::default();
    config.play.autoplay = false;
    config.network.server_url = Some(server.base.clone());
    config.network.player_id = ACCOUNT_ID.to_owned();
    let mut shared = crate::App::new(String::new(), config, LaunchOptions::default(), settings_in_own_folder(&format!("contract-{tag}"))).shared;
    shared.session = AccountSession::restored(Some(ACCOUNT_TOKEN.to_owned()), Some(ACCOUNT_ID.to_owned()));
    shared.rebuild_server();
    shared.record_sync_pacing = UNPACED;
    assert!(shared.sends_to_primary_ir_server(), "the fixture is a player signed in to the server");
    shared
}

/// A run of `model` as `AppShared::load` sets one up on the app's settings: laid out by the
/// arrangement it has with [`LAYOUT_SEED`], on its gauge and judge, with SCRATCH AUTO taking the
/// turntable when it is on, and, when it is a course's stage, judged and started as the app judges
/// and starts one ([`AppShared::stage_judge_setup`]): on the table the COURSE GAUGE setting picks, on
/// the course gauge the GAUGE setting maps to, carrying every gauge and the combo the stage before
/// left.
fn loaded(shared: &AppShared, mut model: rbms_model::Model) -> (PlaySession, rbms_model::Model) {
    rbms_chart::shuffle::apply(&mut model, shared.config.play.random, LAYOUT_SEED);
    let mode = model.mode;
    let (judge_setup, course_stage) = shared.stage_judge_setup(&mode);
    let course_combo = course_stage.and(shared.course_run.as_ref()).map(|run| run.totals.combo_carry);
    let auto_lanes = if shared.config.play.scratch_auto { (0..mode.key).map(|lane| mode.is_scratch(lane)).collect() } else { Vec::new() };
    let options = SessionOptions {
        autoplay: shared.run_plays_itself(),
        gauge: shared.config.play.gauge,
        judge_offset_us: shared.offset_us(),
        auto_lanes,
        seed: LAYOUT_SEED,
        auto_calibration: shared.config.judge.auto_offset,
        initial_gauge: course_stage.map(|stage| (stage.start, stage.value)),
        initial_combo: course_combo.unwrap_or(0),
        carried_gauges: course_stage.and_then(|stage| stage.carried),
        ..SessionOptions::default()
    };
    let mut session = PlaySession::new(model.clone(), options);
    session.set_judge_setup(judge_setup);
    (session, model)
}

/// [`loaded`], played by hand to its end: every key note pressed as far from its time as
/// `offsets_us` says, the turntable on its time unless SCRATCH AUTO plays it.
fn played(shared: &AppShared, model: rbms_model::Model, offsets_us: &[Option<i64>]) -> PlaySession {
    let (session, laid_out) = loaded(shared, model);
    replay_inputs(session, &inputs_by_hand(&laid_out, offsets_us, shared.config.play.scratch_auto))
}

/// A press and release of each key note's own key [`EARLY_PRESS_US`] ahead of it: inside the 空POOR
/// band, so each is a 空POOR and leaves the note to be hit.
fn early_presses_before_every_key_note(laid_out: &rbms_model::Model) -> Vec<Input> {
    laid_out
        .timelines
        .iter()
        .flat_map(|line| line.notes.iter().enumerate().filter(|(_, note)| note.is_some()).map(move |(lane, _)| (line.time_us, lane)))
        .filter(|(_, lane)| !laid_out.mode.is_scratch(*lane))
        .flat_map(|(time_us, lane)| [(time_us - EARLY_PRESS_US, lane, true), (time_us - EARLY_PRESS_US + KEY_HELD_US, lane, false)])
        .collect()
}

/// Hand `session` to the app as a run of the chart called `name` that has just ended.
fn end_named_run(shared: &mut AppShared, session: PlaySession, name: &str) -> Transition {
    end_named_run_as(shared, session, name, FROM_THE_CHART)
}

/// How a run's long notes were judged: its `lntype` and the long-note key its record is kept under.
#[derive(Clone, Copy)]
struct LongNotes<'a> {
    lntype: i32,
    key: &'a str,
}

/// The long notes of a chart that leaves none of them to LN MODE.
const FROM_THE_CHART: LongNotes<'static> = LongNotes { lntype: 0, key: SCORE_LN_MODE_FROM_CHART };

/// [`end_named_run`] of a run whose long notes were judged as `long_notes` says.
fn end_named_run_as(shared: &mut AppShared, session: PlaySession, name: &str, long_notes: LongNotes<'_>) -> Transition {
    shared.mode = session.model().mode;
    shared.chart_path = name.to_owned();
    shared.replay = None;
    let mut ended = PlayState::new(session, std::collections::HashMap::new(), long_notes.lntype, long_notes.key.to_owned());
    enter_result(&mut ended, shared)
}

/// End `session` as the result screen does and answer the score it sent and, when one went up after
/// it, the replay.
fn sent_from_the_result_screen(shared: &mut AppShared, server: &CaptureServer, session: PlaySession) -> (wire::Request, Option<wire::Request>) {
    let entered = end_named_run(shared, session, RECORDED_CHART_NAME);
    assert!(matches!(entered, Transition::To(Stage::Result(_))), "the run did not reach the result screen");
    let outcome = shared.submit_rx.take().expect("the run was handed to the score server").recv().expect("the submission came back");
    assert!(outcome.submit.as_ref().is_ok_and(|response| response.accepted), "the server did not take the run: {:?}", outcome.submit);
    let score = server.next("POST", "/scores");
    let replay = outcome.replay.map(|uploaded| {
        assert!(uploaded.is_ok(), "the replay did not go up: {uploaded:?}");
        server.next("POST", "/charts/")
    });
    (score, replay)
}

/// End `session` as the result screen does, let the app take the server's answer as it does frame
/// by frame, and answer the score it sent and the trace a records sync of the run sent after it.
fn traced_from_the_result_screen(shared: &mut AppShared, server: &CaptureServer, session: PlaySession) -> (wire::Request, wire::Request) {
    traced_from_the_result_screen_as(shared, server, session, FROM_THE_CHART)
}

/// [`traced_from_the_result_screen`] of a run whose long notes were judged as `long_notes` says.
fn traced_from_the_result_screen_as(
    shared: &mut AppShared,
    server: &CaptureServer,
    session: PlaySession,
    long_notes: LongNotes<'_>,
) -> (wire::Request, wire::Request) {
    let md5 = session.model().md5.clone();
    let entered = end_named_run_as(shared, session, RECORDED_CHART_NAME, long_notes);
    assert!(matches!(entered, Transition::To(Stage::Result(_))), "the run did not reach the result screen");
    let deadline = std::time::Instant::now() + WORK_WAIT;
    while shared.submit_rx.is_some() {
        assert!(std::time::Instant::now() < deadline, "the submission did not come back");
        shared.poll_ir_jobs(crate::stage::StageId::Result);
        std::thread::sleep(Duration::from_millis(1));
    }
    sync_finished(shared);
    let score = server.next("POST", "/scores");
    let trace = server.next("POST", &format!("/charts/{md5}/traces"));
    (score, trace)
}

/// Hold a trace that went up to the score that went up before it: linked to the id the server
/// answered the score with, on the same chart, and as many judged objects as the score counts.
fn linked_to_its_score(trace: &wire::Request, score: &wire::Request) {
    let trace = rbms_ir::PlayTrace::from_json(&trace.body).expect("the trace body reads as a trace");
    let score = ScoreSubmission::from_json(&score.body).expect("the score body reads as a submission");
    assert!(trace.score_id.as_deref().is_some_and(|id| id.starts_with(SCORE_ID_PREFIX)), "the trace is linked to the score: {:?}", trace.score_id);
    assert_eq!(trace.chart, score.chart.id);
    let judged = trace.notes.kind.iter().filter(|kind| **kind != rbms_ir::TRACE_KIND_LONG_END).count();
    assert_eq!(judged as u32, score.total_notes);
    assert_eq!(trace.random, score.options.random);
}

/// Wait for a records sync to end.
fn sync_finished(shared: &mut AppShared) {
    let deadline = std::time::Instant::now() + WORK_WAIT;
    while shared.record_sync.is_some() {
        assert!(std::time::Instant::now() < deadline, "the records sync did not end");
        shared.poll_ir_jobs(crate::stage::StageId::Select);
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// What a body is, by the prefix of its file's name.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Score,
    Course,
    Replay,
    Settings,
    Trace,
}

impl Kind {
    fn of(name: &str) -> Kind {
        match name.split('-').next() {
            Some("score") => Kind::Score,
            Some("course") => Kind::Course,
            Some("replay") => Kind::Replay,
            Some("settings") => Kind::Settings,
            Some("trace") => Kind::Trace,
            _ => panic!("{name} names no kind of request"),
        }
    }

    /// The fields that may be `null`: the ones the Rust type leaves empty as `None`. Every other
    /// `null`, and above all one where a number should be, is a value that did not serialise (a
    /// NaN or an infinity), which the server refuses.
    fn nullable(self) -> &'static [&'static str] {
        match self {
            Kind::Score => &["replay_id", "client_build_sha256", "client_platform", "options.random_p2"],
            Kind::Course => &["trophy"],
            Kind::Replay => &["id", "chart", "score_id", "random", "random_p2", "seed", "gauge", "client_build_sha256", "event_count", "duration_us", "size"],
            Kind::Settings => &["name", "updated_at", "base_updated_at"],
            Kind::Trace => &["failed_at_ms"],
        }
    }

    /// Write the values that are the moment or the machine as fixed ones, once each is seen to be
    /// what it should be.
    fn fix_the_moment_and_the_machine(self, body: &mut Json) {
        let is_moment = |value: &Json| matches!(value, Json::Number(raw) if raw.parse::<i64>().is_ok_and(|ms| ms > 0));
        let is_build = |value: &Json| {
            value.text().is_some_and(|hash| hash.len() == SHA256_HEX_LENGTH && hash.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)))
        };
        let moment = Json::Number(FIXED_MOMENT_MS.to_string());
        match self {
            Kind::Score => {
                replace(body, "played_at", moment, is_moment);
                let client = concat!("rbms/", env!("CARGO_PKG_VERSION"));
                replace(body, "client", Json::Text(FIXED_CLIENT.to_owned()), |value| value.text() == Some(client));
                replace(body, "client_build_sha256", Json::Text(FIXED_BUILD_SHA256.to_owned()), is_build);
                let is_platform =
                    |value: &Json| value.text().is_some_and(|platform| platform.split_once('-').is_some_and(|(os, arch)| !os.is_empty() && !arch.is_empty()));
                replace(body, "client_platform", Json::Text(FIXED_PLATFORM.to_owned()), is_platform);
            }
            Kind::Course => replace(body, "played_at", moment, is_moment),
            Kind::Replay => replace(body, "client_build_sha256", Json::Text(FIXED_BUILD_SHA256.to_owned()), is_build),
            Kind::Settings => replace(body, "updated_at", moment, is_moment),
            Kind::Trace => {
                let is_score_id = |value: &Json| value.text().is_some_and(|id| id.starts_with(SCORE_ID_PREFIX));
                replace(body, "score_id", Json::Text(FIXED_SCORE_ID.to_owned()), is_score_id);
            }
        }
    }
}

/// Put `value` at `key` of `body`, which must hold either `null` there or what `holds` takes.
fn replace(body: &mut Json, key: &str, value: Json, holds: impl Fn(&Json) -> bool) {
    let slot = body.get_mut(key).unwrap_or_else(|| panic!("the body has no {key}"));
    if *slot == Json::Null {
        return;
    }
    assert!(holds(slot), "{key} is not what it should be: {slot:?}");
    *slot = value;
}

/// Hold a body to the ranges the server reads exactly: every whole number within 2^53, every other
/// number finite, and `null` only where the type has an empty value.
fn hold_to_server_ranges(name: &str, kind: Kind, body: &Json) {
    for (path, leaf) in body.leaves() {
        match leaf {
            Json::Null => assert!(kind.nullable().contains(&path.as_str()), "{name}: {path} is null, which the server reads as no number"),
            Json::Number(raw) if raw.chars().all(|c| c.is_ascii_digit() || c == '-') => {
                let whole: i128 = raw.parse().unwrap_or_else(|_| panic!("{name}: {path} = {raw} is no number"));
                assert!(whole.abs() <= MAX_SAFE_INTEGER, "{name}: {path} = {raw} is past 2^53, which a JavaScript number does not hold exactly");
            }
            Json::Number(raw) => assert!(raw.parse::<f64>().is_ok_and(f64::is_finite), "{name}: {path} = {raw} is not finite"),
            _ => {}
        }
    }
}

fn contract_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("contract").join("client-requests")
}

fn updating() -> bool {
    std::env::var(UPDATE_ENV).is_ok_and(|value| value == "1")
}

/// Compare `wire`, a body as the server received it, with the file `name` holds, or write it there
/// when [`UPDATE_ENV`] says so.
fn check_contract(name: &str, wire: &str) {
    assert!(CONTRACT_FILES.contains(&name), "{name} is not in CONTRACT_FILES");
    let kind = Kind::of(name);
    let mut body = Json::parse(wire);
    hold_to_server_ranges(name, kind, &body);
    kind.fix_the_moment_and_the_machine(&mut body);
    let rendered = body.sorted().render();
    let path = contract_folder().join(format!("{name}.json"));
    if updating() {
        std::fs::create_dir_all(contract_folder()).expect("the contract folder is writable");
        std::fs::write(&path, rendered).expect("the contract file is writable");
        return;
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} cannot be read ({e}); {UPDATE_HINT}", path.display()));
    if committed == rendered {
        return;
    }
    let (line, (was, now)) = committed
        .lines()
        .chain(std::iter::repeat(""))
        .zip(rendered.lines().chain(std::iter::repeat("")))
        .enumerate()
        .find(|(_, (was, now))| was != now)
        .expect("two texts that differ differ on a line");
    panic!("{name}.json is not what the client sends now; line {}:\n  committed: {was}\n  sent:      {now}\n{UPDATE_HINT}", line + 1);
}

/// The seed of the run the server refused on 2026-10-14 (the clock's nanoseconds) is caught here,
/// before any file is written.
#[test]
#[should_panic(expected = "past 2^53")]
fn the_range_check_refuses_a_seed_past_two_to_the_fifty_three() {
    hold_to_server_ranges("score-normal", Kind::Score, &Json::parse(&format!(r#"{{"seed":{LAYOUT_CLOCK_NS}}}"#)));
}

/// A gauge that came out NaN is sent as `null`, which the server refuses.
#[test]
#[should_panic(expected = "is null")]
fn the_range_check_refuses_a_number_that_did_not_serialise() {
    hold_to_server_ranges("score-normal", Kind::Score, &Json::parse(r#"{"gauge_value":null}"#));
}

#[test]
fn the_contract_folder_holds_every_request_and_nothing_else() {
    let listed: BTreeSet<String> = std::fs::read_dir(contract_folder())
        .map(|entries| entries.filter_map(|entry| entry.ok()?.file_name().to_str()?.strip_suffix(".json").map(str::to_owned)).collect())
        .unwrap_or_default();
    let expected: BTreeSet<String> = CONTRACT_FILES.iter().map(|name| (*name).to_owned()).collect();
    let stale: Vec<_> = listed.difference(&expected).collect();
    if updating() {
        for name in stale {
            std::fs::remove_file(contract_folder().join(format!("{name}.json"))).expect("a stale contract file is removable");
        }
        return;
    }
    assert!(stale.is_empty(), "files no test writes: {stale:?}; {UPDATE_HINT}");
    let missing: Vec<_> = expected.difference(&listed).collect();
    assert!(missing.is_empty(), "files missing: {missing:?}; {UPDATE_HINT}");
}

/// A run played to its end on RANDOM with a fresh seed, sent from the result screen, and the replay
/// that goes up after it, linked to the score the server answered with.
#[test]
fn a_finished_random_run_and_its_replay() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "random");
    shared.config.play.random = NoteOption::Random;
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    let summary = session.summary();
    assert!(summary.clear_lamp.is_cleared() && summary.ex_score > 0, "the fixture run clears: {summary:?}");
    let (score, replay) = sent_from_the_result_screen(&mut shared, &server, session);
    check_contract("score-normal", &score.body);
    let replay = replay.expect("the replay went up");
    assert_eq!(replay.path, format!("/charts/{}/replays", contract_model().md5), "the replay is not filed under the chart's MD5");
    check_contract("replay-random", &replay.body);
}

/// A run that fails on EX-HARD with most notes let go by, and its trace, which says where the gauge
/// emptied and leaves the notes after it at state 0.
#[test]
fn a_failed_run() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "failed");
    shared.config.play.gauge = GaugeKind::ExHard;
    let session = played(&shared, contract_model(), &FAILING_OFFSETS_US);
    assert!(session.summary().failed, "the fixture run fails");
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    check_contract("score-failed", &score.body);
    linked_to_its_score(&trace, &score);
    let kept = rbms_ir::PlayTrace::from_json(&trace.body).expect("the trace body reads as a trace");
    assert!(kept.failed_at_ms.is_some() && kept.notes.state.contains(&0), "the trace says where the gauge emptied: {:?}", kept.failed_at_ms);
    check_contract("trace-failed", &trace.body);
}

/// A run with SCRATCH AUTO playing the turntable.
#[test]
fn a_scratch_auto_run() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "scratch-auto");
    shared.config.play.scratch_auto = true;
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    assert!(crate::app_result::scratch_auto_played(&session), "SCRATCH AUTO took the turntable");
    let (score, _) = sent_from_the_result_screen(&mut shared, &server, session);
    check_contract("score-scratch-auto", &score.body);
}

/// A run with SCRATCH AUTO cleared on the HARD gauge. It is sent as the LIGHT ASSIST lamp the
/// reference demotes it to, with the HARD gauge beside it: the body is no different in kind from
/// any other SCRATCH AUTO clear, and the ASSIST HARD grade is what both ends derive from the two.
#[test]
fn a_scratch_auto_run_cleared_on_hard() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "scratch-auto-hard");
    shared.config.play.scratch_auto = true;
    shared.config.play.gauge = GaugeKind::Hard;
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    assert!(crate::app_result::scratch_auto_played(&session), "SCRATCH AUTO took the turntable");
    let summary = session.summary();
    assert!(!summary.failed && summary.finished_gauge == Some(GaugeKind::Hard), "the fixture run clears on HARD");
    let md5 = session.model().md5.clone();
    let (score, _) = sent_from_the_result_screen(&mut shared, &server, session);
    let kept = shared.scores.best_lamp_for_md5(&md5).expect("the run is kept");
    let light_assist = rbms_judge::clear_type_id(rbms_judge::ClearType::LightAssistEasy);
    assert_eq!(kept, rbms_store::RecordLamp { clear: light_assist, assist_grade: rbms_judge::AssistGrade::Hard.id() }, "kept as an ASSIST HARD");
    check_contract("score-scratch-auto-hard", &score.body);
}

/// A run judged with the PGREAT window widened to the most JUDGE WIDTH allows.
#[test]
fn a_widened_judge_run() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "widened");
    shared.config.judge.judge_rate_key[0] = rbms_config::JUDGE_RATE_MAX_PERCENT;
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    let (score, replay) = sent_from_the_result_screen(&mut shared, &server, session);
    assert!(replay.is_none(), "a run judged by widths the upload cannot state sent its replay");
    check_contract("score-widened-judge", &score.body);
}

/// A run of a fourteen-key chart on RANDOM.
#[test]
fn a_double_play_run() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "double");
    shared.config.play.random = NoteOption::Random;
    let model = model_of(&stepped_chart(DOUBLE_HEADER, DOUBLE_MEASURES, DOUBLE_SLOTS, &DOUBLE_CHANNELS), "double.bme");
    assert_eq!(model.mode, Mode::BEAT_14K, "the fixture is a fourteen-key chart");
    let session = played(&shared, model, &HAND_OFFSETS_US);
    let (score, _) = sent_from_the_result_screen(&mut shared, &server, session);
    check_contract("score-14k-2p", &score.body);
}

/// A run cleared with more 空POOR than POOR: each key note hunted for early, inside the 空POOR band,
/// and one in eight let go by. The server refused every such run until its count of the judgements
/// that take a note left the 空POOR out (2026-10-14).
#[test]
fn a_run_with_more_empty_poor_than_poor() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "empty-poor");
    let (session, laid_out) = loaded(&shared, contract_model());
    let mut inputs = inputs_by_hand(&laid_out, &ONE_IN_EIGHT_MISSED_US, false);
    inputs.extend(early_presses_before_every_key_note(&laid_out));
    inputs.sort_unstable();
    let session = replay_inputs(session, &inputs);
    let summary = session.summary();
    assert!(summary.counts[5] > summary.counts[4] && summary.counts[4] > 0, "the fixture has more 空POOR than POOR: {:?}", summary.counts);
    assert!(summary.clear_lamp.is_cleared() && summary.total_judged == summary.total_notes, "the fixture run is played to its end and clears: {summary:?}");
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    check_contract("score-empty-poor-exceeds-poor", &score.body);
    linked_to_its_score(&trace, &score);
    check_contract("trace-empty-poor-exceeds-poor", &trace.body);
}

/// A run with BADs early and late, and its trace.
#[test]
fn a_run_with_bads_and_its_trace() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "trace-bad");
    let session = played(&shared, contract_model(), &BAD_OFFSETS_US);
    let summary = session.summary();
    assert!(summary.early[3] > 0 && summary.late[3] > 0, "the fixture has BADs either way: {summary:?}");
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    linked_to_its_score(&trace, &score);
    check_contract("score-bad", &score.body);
    check_contract("trace-bad", &trace.body);
}

/// The inputs of a run of the long-note chart: a plain note pressed on its time and let go, a long
/// note's head pressed [`LONG_HEAD_LATE_US`] late and held to its end, let go there on its time.
fn long_note_inputs(laid_out: &rbms_model::Model) -> Vec<Input> {
    let mut inputs = Vec::new();
    for line in &laid_out.timelines {
        for (lane, note) in line.notes.iter().enumerate() {
            match note.as_ref().map(|note| &note.kind) {
                Some(rbms_model::NoteKind::Normal) => inputs.extend([(line.time_us, lane, true), (line.time_us + KEY_HELD_US, lane, false)]),
                Some(rbms_model::NoteKind::LongStart { .. }) => inputs.push((line.time_us + LONG_HEAD_LATE_US, lane, true)),
                Some(rbms_model::NoteKind::LongEnd { .. }) => inputs.push((line.time_us, lane, false)),
                Some(rbms_model::NoteKind::Mine { .. }) | None => {}
            }
        }
    }
    inputs.sort_unstable();
    inputs
}

/// The long-note chart played on LN MODE `ln_mode`, every note taken, and its trace, which tells a
/// long note's head from a plain note by the end that follows it, and the end of `end_kind`.
fn a_long_note_run(tag: &str, ln_mode: rbms_judge::ln::LnMode, end_kind: u8, files: [&str; 2]) {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, tag);
    shared.config.judge.ln_mode = ln_mode;
    let mut model = model_of(LONG_NOTE_CHART, "long-notes.bms");
    assert!(rbms_chart::contains_undefined_long_note(&model), "the chart leaves its long notes to LN MODE");
    let setup = crate::judge_setup::run_judge_setup(&shared.config, None);
    rbms_chart::resolve_long_note_flavour(&mut model, setup.ln_mode.resolve());
    let long_notes = LongNotes { lntype: crate::judge_setup::run_lntype(0, setup.ln_mode), key: rbms_config::ln_mode_token(setup.ln_mode) };
    let (session, laid_out) = loaded(&shared, model);
    let session = replay_inputs(session, &long_note_inputs(&laid_out));
    let summary = session.summary();
    assert!(summary.total_judged == summary.total_notes && summary.counts[4] == 0, "every note is taken: {summary:?}");
    let (score, trace) = traced_from_the_result_screen_as(&mut shared, &server, session, long_notes);
    linked_to_its_score(&trace, &score);
    let kept = rbms_ir::PlayTrace::from_json(&trace.body).expect("the trace body reads as a trace");
    assert!(kept.notes.kind.contains(&rbms_ir::TRACE_KIND_LONG_HEAD) && kept.notes.kind.contains(&end_kind), "the kinds: {:?}", kept.notes.kind);
    assert_eq!(kept.lntype, long_notes.lntype);
    let [score_file, trace_file] = files;
    check_contract(score_file, &score.body);
    check_contract(trace_file, &trace.body);
}

#[test]
fn a_long_note_run_and_its_trace() {
    a_long_note_run("trace-ln", rbms_judge::ln::LnMode::LongNote, rbms_ir::TRACE_KIND_LONG_END, ["score-long-note", "trace-long-note"]);
}

#[test]
fn a_charge_note_run_and_its_trace() {
    a_long_note_run("trace-cn", rbms_judge::ln::LnMode::ChargeNote, rbms_ir::TRACE_KIND_CHARGE_END, ["score-charge-note", "trace-charge-note"]);
}

#[test]
fn a_hell_charge_note_run_and_its_trace() {
    a_long_note_run("trace-hcn", rbms_judge::ln::LnMode::HellChargeNote, rbms_ir::TRACE_KIND_CHARGE_END, ["score-hell-charge-note", "trace-hell-charge-note"]);
}

/// A run of a pop'n chart with a BAD, which leaves its note to be judged again, and its trace.
#[test]
fn a_popn_run_and_its_trace() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "trace-popn");
    let model = model_of(&stepped_chart(POPN_HEADER, POPN_MEASURES, POPN_SLOTS, &POPN_CHANNELS), "popn.pms");
    assert_eq!(model.mode, Mode::POPN_9K, "the fixture is a pop'n chart");
    let session = played(&shared, model, &POPN_OFFSETS_US);
    assert!(session.summary().counts[3] > 0, "the fixture has a BAD: {:?}", session.summary());
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    let kept = rbms_ir::PlayTrace::from_json(&trace.body).expect("the trace body reads as a trace");
    assert_eq!(kept.mode, "POPN_9K");
    check_contract("score-popn-9k", &score.body);
    check_contract("trace-popn-9k", &trace.body);
}

/// A MAX run with 空POOR: every note on its time and every key note hunted for early, inside the
/// 空POOR band. A 空POOR takes no note and breaks no streak (`BMSPlayer.java:868`), so the lamp
/// stands, and the server's lamp check has to let it through.
#[test]
fn a_max_run_with_empty_poor() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "max-empty-poor");
    let (session, laid_out) = loaded(&shared, contract_model());
    let mut inputs = inputs_by_hand(&laid_out, &ON_TIME_US, false);
    inputs.extend(early_presses_before_every_key_note(&laid_out));
    inputs.sort_unstable();
    let session = replay_inputs(session, &inputs);
    let summary = session.summary();
    assert!(summary.counts[5] > 0, "the fixture has 空POOR: {:?}", summary.counts);
    assert_eq!(summary.clear_lamp, rbms_judge::ClearType::Max, "the fixture run is a MAX: {summary:?}");
    let (score, _) = sent_from_the_result_screen(&mut shared, &server, session);
    check_contract("score-max-with-empty-poor", &score.body);
}

/// A run left by Escape part-way, which its result screen sends as it sends a finished run, and its
/// trace, which a records sync of the run sends once the server has taken the score.
#[test]
fn a_run_left_mid_song() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "left");
    let (session, laid_out) = loaded(&shared, contract_model());
    let inputs = inputs_by_hand(&laid_out, &HAND_OFFSETS_US, false);
    let mut session = session;
    let mut next = 0;
    for frame in 0..=LEFT_AT_US / RUN_FRAME_US {
        let now_us = frame * RUN_FRAME_US;
        while let Some((at_us, lane, press)) = inputs.get(next).copied().filter(|(at_us, _, _)| *at_us <= now_us) {
            if press {
                session.press(lane, at_us, &mut NullSink);
            } else {
                session.release(lane, at_us);
            }
            next += 1;
        }
        session.tick(SessionClock::at(now_us), &mut NullSink);
    }
    let summary = session.summary();
    assert!(summary.total_judged > 0 && summary.total_judged < summary.total_notes, "the fixture run is left part-way: {summary:?}");
    let left = leave_by_escape(&mut shared, session);
    assert!(matches!(left, Transition::To(Stage::Result(_))), "the run did not reach the result screen");
    let deadline = std::time::Instant::now() + WORK_WAIT;
    while shared.submit_rx.is_some() {
        assert!(std::time::Instant::now() < deadline, "the submission did not come back");
        shared.poll_ir_jobs(crate::stage::StageId::Result);
        std::thread::sleep(Duration::from_millis(1));
    }
    sync_finished(&mut shared);
    let score = server.next("POST", "/scores");
    check_contract("score-abandoned", &score.body);
    let trace = server.next("POST", &format!("/charts/{}/traces", contract_model().md5));
    linked_to_its_score(&trace, &score);
    check_contract("trace-abandoned", &trace.body);
}

/// A run played to its end, sent from the result screen, and its trace, which a records sync of
/// the run sends once the server has taken the score, with the score body it is the trace of.
#[test]
fn a_finished_run_and_its_trace() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "trace-normal");
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    linked_to_its_score(&trace, &score);
    check_contract("score-trace-normal", &score.body);
    check_contract("trace-normal", &trace.body);
}

/// The same on RANDOM: the trace holds the lanes the run was laid out on.
#[test]
fn a_random_run_and_its_trace() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "trace-random");
    shared.config.play.random = NoteOption::Random;
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    linked_to_its_score(&trace, &score);
    check_contract("trace-random", &trace.body);
}

/// The same with SCRATCH AUTO playing the turntable: its notes are not in the trace.
#[test]
fn a_scratch_auto_run_and_its_trace() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "trace-scratch-auto");
    shared.config.play.scratch_auto = true;
    let session = played(&shared, contract_model(), &HAND_OFFSETS_US);
    let (score, trace) = traced_from_the_result_screen(&mut shared, &server, session);
    linked_to_its_score(&trace, &score);
    check_contract("trace-scratch-auto", &trace.body);
}

/// A run stored by a client that kept no body for it, as every run before 0.1.4 was: the records
/// sync rebuilds its submission from its row in the score database.
#[test]
fn a_run_the_records_sync_rebuilds_from_its_row() {
    let mut offline = signed_out("contract-resend-offline");
    offline.config.play.random = NoteOption::Random;
    let session = played(&offline, contract_model(), &HAND_OFFSETS_US);
    end_named_run(&mut offline, session, RECORDED_CHART_NAME);
    let db = offline.scoredb.as_ref().expect("the score database");
    let mut logs = db.history(&chart_key(&contract_model().md5, ""), HISTORY_ROWS).expect("the history reads");
    let log = logs.pop().expect("the run was stored");
    assert!(!log.sha256.is_empty() && log.seed == i64::try_from(LAYOUT_SEED).expect("the seed fits"), "the stored row is the run: {log:?}");

    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "resend");
    let old = PlayLog { id: 0, ..log };
    shared.scoredb.as_mut().expect("the score database").record_play(&old, true).expect("the row writes without a body");
    assert!(shared.start_record_sync(crate::record_sync::SyncScope::All), "a sync started");
    sync_finished(&mut shared);
    let score = server.next("POST", "/scores");
    check_contract("score-sync-resend", &score.body);
}

/// A run the engine played. The result screen holds it back (`ir_submission_block_reason`), so no
/// client sends it; this is the submission the run was built with, brought into the server's ranges
/// as the submit worker does before it sends (`clamp_to_server_bounds`), for the day one is sent.
#[test]
fn an_autoplay_run() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "autoplay");
    shared.config.play.autoplay = true;
    let (session, _) = loaded(&shared, contract_model());
    let session = replay_inputs(session, &[]);
    shared.mode = session.model().mode;
    shared.chart_path = RECORDED_CHART_NAME.to_owned();
    shared.replay = None;
    let summary = session.summary();
    let ended = PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_owned());
    let recorded = crate::app_result::record_run(&ended, &mut shared, &summary, &ended.session.record());
    assert_eq!(recorded.block_reason, Some("autoplay"), "an autoplay run is held back");
    let mut sub = recorded.sub;
    assert!(sub.options.autoplay);
    sub.clamp_to_server_bounds();
    check_contract("score-autoplay", &sub.to_json().expect("the submission serialises"));
}

/// A course of two charts played by hand and cleared, with the trophy a clean run earns.
#[test]
fn a_cleared_course() {
    check_contract("course-normal", &course_sent("course", false));
}

/// The same course with SCRATCH AUTO on for both stages.
#[test]
fn a_cleared_course_with_scratch_auto() {
    check_contract("course-scratch-auto", &course_sent("course-scratch-auto", true));
}

/// Play the contract course signed in, both stages on their time, and answer the course submission
/// the server received.
fn course_sent(tag: &str, scratch_auto: bool) -> String {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, tag);
    shared.config.play.scratch_auto = scratch_auto;
    let first = contract_model();
    let second = model_of(&stepped_chart(SECOND_STAGE_HEADER, SECOND_STAGE_MEASURES, SECOND_STAGE_SLOTS, &SINGLE_CHANNELS), "stage-two.bms");
    let stage = |model: &rbms_model::Model| rbms_course::CourseChart { md5: model.md5.clone(), sha256: model.sha256.clone(), title: model.meta.title.clone() };
    let course = rbms_course::Course {
        name: COURSE_NAME.to_owned(),
        charts: vec![stage(&first), stage(&second)],
        trophies: vec![rbms_course::TrophyRule { name: COURSE_TROPHY.to_owned(), missrate: COURSE_TROPHY_MISS_RATE, scorerate: COURSE_TROPHY_SCORE_RATE }],
        ..rbms_course::Course::default()
    };
    shared.course_run = Some(rbms_course::CourseRun::new(course, COURSE_OPENING_GAUGE));
    shared.course_facts = crate::course_ir::CourseFacts::starting(shared.config.judge.course_gauge);
    shared.library = Library::from_songs(vec![rbms_library::SongEntry {
        path: PathBuf::from(SECOND_STAGE_PATH),
        title: second.meta.title.clone(),
        subtitle: String::new(),
        artist: second.meta.artist.clone(),
        genre: String::new(),
        maker: String::new(),
        level: second.meta.play_level.clone(),
        difficulty: 0,
        init_bpm: second.init_bpm,
        rank: 2,
        total: second.meta.total,
        mode: second.mode,
        md5: second.md5.clone(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }]);

    for (model, name) in [(first, RECORDED_CHART_NAME), (second, SECOND_STAGE_PATH)] {
        let (session, laid_out) = loaded(&shared, model);
        let session = replay_inputs(session, &inputs_by_hand(&laid_out, &ON_TIME_US, scratch_auto));
        end_named_run(&mut shared, session, name);
        if let Some(sent) = shared.submit_rx.take() {
            sent.recv().expect("the stage's submission came back");
        }
    }
    assert!(shared.course_stage_reasons.iter().all(Option::is_none), "a stage held the course back: {:?}", shared.course_stage_reasons);
    server.next("POST", "/courses").body
}

/// The replay of a long chart, which is the largest body the client sends.
#[test]
fn a_long_replay() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "long");
    let model = model_of(&stepped_chart(LONG_HEADER, LONG_MEASURES, LONG_SLOTS, &SINGLE_CHANNELS), "long.bms");
    let session = played(&shared, model, &HAND_OFFSETS_US);
    let (_, replay) = sent_from_the_result_screen(&mut shared, &server, session);
    check_contract("replay-long", &replay.expect("the replay went up").body);
}

/// The settings upload of a fresh install, its first, which overwrites the account's copy.
#[test]
fn a_settings_upload() {
    let server = CaptureServer::spawn();
    let mut shared = signed_in_to(&server, "settings");
    shared.start_settings_upload();
    let put = server.next("PUT", "/players/");
    assert!(put.path.starts_with(&format!("/players/{ACCOUNT_ID}/settings/")), "the blob went to {}", put.path);
    check_contract("settings-blob", &put.body);
}
