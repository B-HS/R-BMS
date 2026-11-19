//! What a run shows of how it was started is the run's own: the arrangement it was laid out with,
//! the gauge it is on, the folder it was picked in and the assists it is played with, whatever the
//! settings say beside it.

use std::collections::HashMap;

use rbms_chart::to_model;
use rbms_judge::gauge::{GaugeAutoShift, GaugeIndex};
use rbms_model::Mode;
use rbms_parser::parse;
use rbms_play::{JudgeSetup, NullSink, SessionClock, SessionOptions};
use rbms_store::SCORE_LN_MODE_FROM_CHART;

use super::*;
use crate::borrowed_layout::tests::{REPLAY_SEED, a_library_of_one_chart, next_run, request};
use crate::select_tree::{OpenedFolder, SelectFolder};
use crate::skin_host::play::{GAUGE_NAMES, PlayLive, PlayShown};
use crate::stage::PlayState;
use crate::stage::play::skin_preview::PlaySample;

/// The folders a test opens in the browser, outermost first, and what a run picked in the last of
/// them names as where it was started.
const OPEN_FOLDERS: [&str; 3] = ["TABLES", "発狂BMS", "★8"];
const ORIGIN: &str = "発狂BMS / ★8";

/// A chart of sixty-four notes on one key, four rows of sixteen from two seconds in: enough unhit
/// notes to empty a HARD gauge.
const MANY_NOTES: &[u8] =
    b"#BPM 120\r\n#RANK 3\r\n#WAV01 a.wav\r\n#00111:01010101010101010101010101010101\r\n#00211:01010101010101010101010101010101\r\n#00311:01010101010101010101010101010101\r\n#00411:01010101010101010101010101010101\r\n";

/// A double play chart with a note on a key of each side.
const DOUBLE: &[u8] = b"#PLAYER 3\r\n#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n#00121:01\r\n";

/// Long after every note of [`MANY_NOTES`] has gone by unhit.
const AFTER_MANY_NOTES_US: i64 = 20_000_000;

/// The gauge a course stage starts on in a test.
const COURSE_GAUGE_START: f32 = 100.0;

/// What the next frame of `state` tells the skin of its run, settled as the screen settles it.
fn shown_of(state: &mut PlayState, shared: &AppShared) -> PlayShown {
    state.ensure_setup(shared);
    let live = PlayLive { setup: state.setup.clone().unwrap_or_default(), ..PlayLive::default() };
    PlayShown::of(&state.session, &live)
}

/// The name of the gauge `shown` is on, as the skin is told it.
fn gauge_name(shown: &PlayShown) -> &'static str {
    GAUGE_NAMES[usize::try_from(shown.gauge.kind).expect("a gauge number")]
}

/// What the sample run the menus draw for the play screen `screen` tells the skin of its setup.
fn sample_setup(shared: &AppShared, screen: i32) -> RunSetup {
    PlaySample::new(screen, &shared.config).expect("a sample is written for the screen").run_setup(shared)
}

/// Opens [`OPEN_FOLDERS`] in the browser of `shared`.
fn open_the_folders(shared: &mut AppShared) {
    shared.select_path = OPEN_FOLDERS.map(|title| OpenedFolder::plain(SelectFolder::TableGroup(title.to_string()))).to_vec();
}

#[test]
fn a_run_on_a_borrowed_layout_shows_the_borrowed_arrangement_and_not_the_players_own() {
    let mut shared = a_library_of_one_chart("setup-borrowed");
    shared.config.play.random = NoteOption::Mirror;
    shared.borrow_layout(request(NoteOption::RRandom.label(), REPLAY_SEED), 0).expect("the layout is borrowed");
    let mut state = next_run(&mut shared);

    let shown = shown_of(&mut state, &shared);
    assert_eq!(shown.live.setup.arrangements, ["R-RANDOM", ""], "a single play chart has no second side");
    assert_eq!(shared.settings_to_save().play.random, NoteOption::Mirror, "the player's own arrangement is kept to be saved");
}

#[test]
fn the_setup_is_settled_once_and_a_setting_moved_after_the_start_does_not_move_it() {
    let mut shared = a_library_of_one_chart("setup-settled");
    shared.config.play.random = NoteOption::Random;
    let mut state = next_run(&mut shared);
    state.ensure_setup(&shared);

    shared.config.play.random = NoteOption::Off;
    shared.config.play.constant_speed = true;
    let shown = shown_of(&mut state, &shared);
    assert_eq!((shown.live.setup.arrangements[0], &*shown.live.setup.assists), ("RANDOM", ""));
}

#[test]
fn a_practice_slice_shows_the_arrangement_it_was_laid_out_with() {
    let mut shared = a_library_of_one_chart("setup-practice");
    shared.config.play.random = NoteOption::Off;
    let mut state = next_run(&mut shared);
    let practice = crate::practice::PracticeSession {
        start_us: 0,
        end_us: AFTER_MANY_NOTES_US,
        gauge: GaugeIndex::Normal,
        gauge_set: Default::default(),
        start_gauge: COURSE_GAUGE_START,
        judge_rate_percent: rbms_config::JUDGE_RATE_DEFAULT_PERCENT,
        freq_percent: rbms_config::PLAYBACK_RATE_UNCHANGED_PERCENT,
        total: None,
        option: NoteOption::HRandom,
        gauge_locked: true,
    };
    state.set_practice(practice);

    assert_eq!(shown_of(&mut state, &shared).live.setup.arrangements[0], "H-RANDOM");
}

#[test]
fn an_arrangement_of_off_is_named_too() {
    let mut shared = a_library_of_one_chart("setup-off");
    shared.config.play.random = NoteOption::Off;
    let mut state = next_run(&mut shared);

    assert_eq!(shown_of(&mut state, &shared).live.setup.arrangements[0], "OFF");
}

#[test]
fn a_double_play_chart_names_the_arrangement_of_each_side() {
    let shared = a_library_of_one_chart("setup-double");
    let session = PlaySession::new(to_model(&parse(DOUBLE), Mode::BEAT_14K), SessionOptions::default());

    let setup = run_setup(&shared, &session, NoteOption::Random, false);
    assert_eq!(setup.arrangements, ["RANDOM", "RANDOM"], "one arrangement lays out both sides, each from a seed of its own");
}

#[test]
fn a_run_picked_in_a_folder_names_its_last_two_titles_and_one_from_nowhere_names_none() {
    let mut shared = a_library_of_one_chart("setup-origin");
    let mut state = next_run(&mut shared);
    assert_eq!(&*shown_of(&mut state, &shared).live.setup.origin, "", "the top of the browser is no folder");

    open_the_folders(&mut shared);
    let mut state = next_run(&mut shared);
    assert_eq!(&*shown_of(&mut state, &shared).live.setup.origin, ORIGIN);

    shared.select_path.truncate(1);
    let mut state = next_run(&mut shared);
    assert_eq!(&*shown_of(&mut state, &shared).live.setup.origin, OPEN_FOLDERS[0], "one folder open is named alone");
}

#[test]
fn a_course_stage_and_a_watched_replay_name_no_folder_whatever_the_browser_has_open() {
    let mut shared = a_library_of_one_chart("setup-elsewhere");
    open_the_folders(&mut shared);
    let session = || PlaySession::new(to_model(&parse(DOUBLE), Mode::BEAT_14K), SessionOptions::default());

    shared.course_run = Some(rbms_course::CourseRun::new(rbms_course::Course::default(), COURSE_GAUGE_START));
    assert_eq!(&*run_setup(&shared, &session(), NoteOption::Off, false).origin, "", "a course stage");
    shared.course_run = None;

    shared.replay = Some(rbms_store::Replay {
        chart_path: String::new(),
        md5: String::new(),
        mode: String::new(),
        random: NoteOption::Off.label().to_string(),
        seed: REPLAY_SEED,
        offset_ms: 0,
        scratch_auto: false,
        gauge: String::new(),
        judge: Default::default(),
        events: Vec::new(),
    });
    assert_eq!(&*run_setup(&shared, &session(), NoteOption::Off, false).origin, "", "a replay being watched");
}

#[test]
fn a_course_stage_is_on_its_course_gauge_and_a_shifted_gauge_is_named_as_it_shifts() {
    let model = || to_model(&parse(MANY_NOTES), Mode::BEAT_7K);
    let course = PlaySession::new(model(), SessionOptions { initial_gauge: Some((GaugeIndex::ExClass, COURSE_GAUGE_START)), ..SessionOptions::default() });
    assert_eq!(gauge_name(&PlayShown::of(&course, &PlayLive::default())), "EX CLASS");

    let mut shifting = PlaySession::new(model(), SessionOptions { gauge: rbms_judge::GaugeKind::Hard, ..SessionOptions::default() });
    shifting.set_judge_setup(JudgeSetup { gauge_auto_shift: GaugeAutoShift::SurvivalToGroove, ..shifting.judge_setup() });
    assert_eq!(gauge_name(&PlayShown::of(&shifting, &PlayLive::default())), "HARD", "the run starts on the gauge it was set to");
    shifting.tick(SessionClock::at(AFTER_MANY_NOTES_US), &mut NullSink);
    assert_eq!(gauge_name(&PlayShown::of(&shifting, &PlayLive::default())), "NORMAL", "an emptied HARD gauge has dropped the run to NORMAL");
}

#[test]
fn the_assists_are_the_ones_on_named_in_order_and_the_ln_mode_only_where_it_decides() {
    let mut shared = a_library_of_one_chart("setup-assists");
    let model = || to_model(&parse(MANY_NOTES), Mode::BEAT_7K);
    let plain = PlaySession::new(model(), SessionOptions::default());
    assert_eq!(&*run_setup(&shared, &plain, NoteOption::Off, true).assists, "", "nothing is on");

    let auto_lanes = crate::app_play::scratch_auto_lanes(Mode::BEAT_7K, true);
    let mut assisted = PlaySession::new(model(), SessionOptions { auto_lanes, ..SessionOptions::default() });
    let wide = [rbms_config::JUDGE_RATE_DEFAULT_PERCENT * 2; rbms_config::JUDGE_WIDTH_TIER_COUNT];
    assisted.set_judge_setup(JudgeSetup { ln_mode: LnMode::HellChargeNote, judge_rate_key: wide, ..assisted.judge_setup() });
    shared.config.play.constant_speed = true;
    shared.config.play.legacy_note = true;
    let named = |decides| run_setup(&shared, &assisted, NoteOption::Off, decides).assists;

    assert_eq!(&*named(true), "SCRATCH AUTO · HCN · EX JUDGE · CONSTANT · LEGACY NOTE");
    assert_eq!(&*named(false), "SCRATCH AUTO · EX JUDGE · CONSTANT · LEGACY NOTE", "a chart that states its long notes is not changed by the LN MODE");
}

#[test]
fn a_run_that_left_its_long_notes_to_the_ln_mode_is_known_by_the_key_its_record_is_kept_under() {
    let mut shared = a_library_of_one_chart("setup-ln-key");
    let mut state = next_run(&mut shared);
    assert_eq!(state.ln_mode_key, SCORE_LN_MODE_FROM_CHART);
    state.session.set_judge_setup(JudgeSetup { ln_mode: LnMode::ChargeNote, ..state.session.judge_setup() });
    assert_eq!(&*shown_of(&mut state, &shared).live.setup.assists, "", "the chart decides its own long notes");

    let mut decided =
        PlayState::new(PlaySession::new(to_model(&parse(MANY_NOTES), Mode::BEAT_7K), SessionOptions::default()), HashMap::new(), 0, "CN".to_string());
    decided.session.set_judge_setup(JudgeSetup { ln_mode: LnMode::ChargeNote, ..decided.session.judge_setup() });
    assert_eq!(&*shown_of(&mut decided, &shared).live.setup.assists, "CN");
}

/// The sample run the skin menu and the play options draw is played with none of the player's
/// assists, and is shown with the ones the settings hold, as a run started now would be: the
/// option figures beside it are the settings' too.
#[test]
fn the_sample_run_of_the_menus_names_the_arrangement_and_the_assists_the_settings_hold() {
    let mut shared = a_library_of_one_chart("setup-sample");
    shared.config.play.random = NoteOption::Mirror;
    shared.config.play.scratch_auto = true;
    shared.config.judge.ln_mode = LnMode::ChargeNote;
    shared.config.judge.judge_rate_key = [rbms_config::JUDGE_RATE_DEFAULT_PERCENT * 2; rbms_config::JUDGE_WIDTH_TIER_COUNT];
    let seven = sample_setup(&shared, rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS);
    assert_eq!((seven.arrangements, &*seven.assists), (["MIRROR", ""], "SCRATCH AUTO · CN · EX JUDGE"));
    let fourteen = sample_setup(&shared, rbms_skin::loader::SKIN_TYPE_PLAY_14KEYS);
    assert_eq!(fourteen.arrangements, ["MIRROR", "MIRROR"], "a double play sample names both sides");
    let nine = sample_setup(&shared, rbms_skin::loader::SKIN_TYPE_PLAY_9KEYS);
    assert_eq!(&*nine.assists, "CN · EX JUDGE", "SCRATCH AUTO plays nothing of a mode with no turntable");

    shared.config.play.scratch_auto = false;
    shared.config.judge = rbms_config::JudgeOptions::default();
    shared.config.play.constant_speed = true;
    assert_eq!(&*sample_setup(&shared, rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS).assists, "CONSTANT", "the line follows the settings as they are moved");
}
