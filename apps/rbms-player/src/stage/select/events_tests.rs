//! The events a skin's browser runs, id by id: the setting each one steps and which way its
//! argument steps it, where the buttons that leave the browser go, and the ids that do nothing.
//!
//! An event is run on the browser directly here, which needs no skin; the last tests put one in the
//! queue of a browser a skin is drawing, the way a button of the skin does.

use rbms_chart::shuffle::NoteOption;
use rbms_config::{Config, FixHiSpeed, JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, LaneOption, ScoreTarget, SettingTab};
use rbms_judge::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeAutoShift, GaugeKind};
use rbms_judge::ln::LnMode;
use rbms_library::Library;
use rbms_skin::property::generated::*;
use rbms_store::ScoreBook;

use super::events::{SelectEvent, SettingEvent, event_of, replay_in_slot, step_setting};
use super::skinned_tests::{Browser, SHORT_STEP, starts_a_chart};
use super::tests::{app, entry, record};
use super::*;
use crate::App;
use crate::skin_host::{Cluster, ClusterRequest};

/// The argument a left click runs an event with, and the one a right click does.
const NEXT: i32 = 1;
const PREVIOUS: i32 = -1;

/// The argument an event run with none has, which is the next like any argument that is not below
/// zero.
const NO_ARGUMENT: i32 = 0;

/// The events the reference numbers without a constant.
const EVENT_DIFFICULTY: i32 = 10;
const EVENT_HISPEED: i32 = 57;
const EVENT_DURATION: i32 = 59;
const EVENT_BGA_EXPAND: i32 = 73;
const EVENT_JUDGE_TIMING_AUTO: i32 = 75;
const EVENT_UPDATE_FOLDER: i32 = 211;
const EVENT_OPEN_WITH_EXPLORER: i32 = 212;
const EVENT_OPEN_DOWNLOAD_SITE: i32 = 213;
const EVENT_SONGBAR_SORT: i32 = 312;
const EVENT_GUIDE_SE: i32 = 343;
const EVENT_CHART_REPLICATION_MODE: i32 = 344;
const EVENT_CONSTANT: i32 = 400;

/// An id far from every event there is.
const EVENT_NOBODY_HAS: i32 = 2999;

/// The row of the fixture list a chart is on and the row the folder is on.
const CHART_ROW: usize = 1;
const FOLDER_ROW: usize = 0;

/// A browser on a list of one folder and two charts, with the cursor on the first chart.
fn browsing() -> (App, SelectState) {
    let mut app = app();
    app.shared.library = Library::from_songs(vec![entry("alpha", "a", "5"), entry("beta", "a", "5")]);
    app.shared.select_items = vec![SelectItem::Folder { label: "folder".into(), target: SelectView::AllSongs }, SelectItem::Song(0), SelectItem::Song(1)];
    app.shared.sel = CHART_ROW;
    app.shared.config.play.autoplay = false;
    (app, SelectState::new())
}

/// The clear and the score of the records written here, which nothing asked of them reads.
const RECORD_CLEAR: u8 = 5;
const RECORD_EX: u32 = 500;

/// A record of the chart `md5` played at `played_at`, with the replay `file` saved of it.
fn played(md5: &str, played_at: i64, file: Option<&str>) -> rbms_store::ScoreRecord {
    rbms_store::ScoreRecord { replay_file: file.map(str::to_owned), ..record(md5, RECORD_CLEAR, RECORD_EX, 0, played_at) }
}

/// A browser the events of the settings are run on over and over, each time on settings of the
/// caller's.
struct Settings {
    app: App,
    state: SelectState,
}

impl Settings {
    fn new() -> Settings {
        let (app, state) = browsing();
        Settings { app, state }
    }

    /// The settings after the event `id` was run once with `arg1` on `config`.
    fn after(&mut self, config: &Config, id: i32, arg1: i32) -> Config {
        self.app.shared.config = config.clone();
        assert!(matches!(self.state.run_event(&mut self.app.shared, id, arg1), Transition::Stay), "event {id} left the browser");
        self.app.shared.config.clone()
    }

    /// The settings after the event `id` was run once with `arg1` on the settings as they ship.
    fn stepped(&mut self, id: i32, arg1: i32) -> Config {
        self.after(&Config::default(), id, arg1)
    }
}

/// Every event of a setting that goes round a list: where the next and the one before take it from
/// what it ships at, read as the number of the choice in the list.
#[test]
fn an_event_steps_its_setting_to_the_next_and_to_the_one_before_round_the_ends() {
    let mut settings = Settings::new();
    fn at<T: PartialEq + Copy>(all: &[T], value: T) -> usize {
        all.iter().position(|entry| *entry == value).expect("the value is one of the list")
    }
    let gauges = rbms_config::GAUGE_CYCLE;
    let bottoms = rbms_judge::gauge::BOTTOM_SHIFTABLE_GAUGES;
    type Read = fn(&Config) -> usize;
    let rows: [(i32, &str, usize, Read); 8] = [
        (BUTTON_GAUGE_1P, "gauge1p", gauges.len(), |config| at(&rbms_config::GAUGE_CYCLE, config.play.gauge)),
        (BUTTON_DPOPTION, "optiondp", LaneOption::ALL.len(), |config| at(&LaneOption::ALL, config.play.lane_option)),
        (BUTTON_HSFIX, "hsfix", FixHiSpeed::ALL.len(), |config| at(&FixHiSpeed::ALL, config.play.fix_hispeed)),
        (BUTTON_TARGET, "target", ScoreTarget::ALL.len(), |config| at(&ScoreTarget::ALL, config.judge.target)),
        (BUTTON_GAUGEAUTOSHIFT, "gaugeautoshift", GaugeAutoShift::ALL.len(), |config| at(&GaugeAutoShift::ALL, config.judge.gauge_auto_shift)),
        (BUTTON_BOTTOMSIFTABLEFGAUGE, "bottomshiftablegauge", bottoms.len(), |config| {
            at(&rbms_judge::gauge::BOTTOM_SHIFTABLE_GAUGES, config.judge.bottom_shiftable_gauge)
        }),
        (BUTTON_LNMODE, "lnmode", LnMode::ALL.len(), |config| at(&LnMode::ALL, config.judge.ln_mode)),
        (BUTTON_RANDOM_1P, "option1p", NoteOption::ALL.len(), |config| {
            usize::try_from(crate::skin_host::options::random_option_index(config.play.random)).unwrap_or(usize::MAX)
        }),
    ];
    for (id, name, count, read) in rows {
        let from = read(&Config::default());
        assert_eq!(read(&settings.stepped(id, NEXT)), (from + 1) % count, "{name} with 1");
        assert_eq!(read(&settings.stepped(id, NO_ARGUMENT)), (from + 1) % count, "{name} with no argument");
        assert_eq!(read(&settings.stepped(id, PREVIOUS)), (from + count - 1) % count, "{name} with -1");

        let mut config = Config::default();
        for _ in 0..count {
            config = settings.after(&config, id, NEXT);
        }
        assert_eq!(read(&config), from, "{name} did not come round to where it started");
    }
}

/// The gauge by name, so the numbers above are known to be the reference's order: the gauge after
/// normal is hard, the one before it easy, and the list goes round from hazard to assist easy.
#[test]
fn the_gauge_event_goes_round_the_six_gauges_in_the_references_order() {
    let mut settings = Settings::new();
    assert_eq!(settings.stepped(BUTTON_GAUGE_1P, NEXT).play.gauge, GaugeKind::Hard);
    assert_eq!(settings.stepped(BUTTON_GAUGE_1P, PREVIOUS).play.gauge, GaugeKind::Easy);
    let mut hazard = Config::default();
    hazard.play.gauge = GaugeKind::Hazard;
    assert_eq!(settings.after(&hazard, BUTTON_GAUGE_1P, NEXT).play.gauge, GaugeKind::AssistEasy);
}

/// The random option goes round in the order the reference numbers its options, which is not the
/// order of the settings row: after random comes R-random. Both sides' events step the one option.
#[test]
fn the_random_events_step_the_one_option_in_the_references_order() {
    let mut settings = Settings::new();
    let mut random = Config::default();
    random.play.random = NoteOption::Random;
    for id in [BUTTON_RANDOM_1P, BUTTON_RANDOM_2P] {
        assert_eq!(settings.stepped(id, NEXT).play.random, NoteOption::Mirror, "event {id}");
        assert_eq!(settings.stepped(id, PREVIOUS).play.random, NoteOption::AllScratch, "event {id}");
        assert_eq!(settings.after(&random, id, NEXT).play.random, NoteOption::RRandom, "event {id}");
        assert_eq!(settings.after(&random, id, PREVIOUS).play.random, NoteOption::Mirror, "event {id}");
    }
}

/// The switches: each event flips its own setting whichever way it is run.
#[test]
fn an_event_of_a_switch_flips_it_either_way() {
    let mut settings = Settings::new();
    type Read = fn(&Config) -> bool;
    let rows: [(i32, &str, Read); 8] = [
        (BUTTON_BGA, "bga", |config| config.display.bga),
        (EVENT_JUDGE_TIMING_AUTO, "notesdisplaytimingautoadjust", |config| config.judge.auto_offset),
        (BUTTON_LANECOVER, "lanecover", |config| config.play.enable_cover),
        (BUTTON_LIFT, "lift", |config| config.play.enable_lift),
        (BUTTON_HIDDEN, "hidden", |config| config.play.enable_hidden),
        (EVENT_GUIDE_SE, "guidese", |config| config.audio.guide_se),
        (BUTTON_SCROLLMODE, "scrollmode", |config| config.play.constant_speed),
        (BUTTON_LONGNOTEMODE, "longnotemode", |config| config.play.legacy_note),
    ];
    for (id, name, read) in rows {
        let shipped = read(&Config::default());
        for arg1 in [NEXT, PREVIOUS] {
            let flipped = settings.stepped(id, arg1);
            assert_eq!(read(&flipped), !shipped, "{name} with {arg1}");
            assert_eq!(read(&settings.after(&flipped, id, arg1)), shipped, "{name} with {arg1}, twice");
        }
    }
}

/// The scroll speed moves by its own step and stops at the ends of its range, and the judge timing
/// by a millisecond and stops at the ends of its.
#[test]
fn the_scroll_speed_and_the_judge_timing_step_and_stop_at_their_ends() {
    let mut settings = Settings::new();
    let shipped = Config::default();
    assert_eq!(settings.stepped(EVENT_HISPEED, NEXT).play.hispeed, shipped.play.hispeed + shipped.play.hispeed_step);
    assert_eq!(settings.stepped(EVENT_HISPEED, PREVIOUS).play.hispeed, shipped.play.hispeed - shipped.play.hispeed_step);

    assert_eq!(settings.stepped(BUTTON_JUDGE_TIMING, NEXT).judge.offset_ms, 1);
    assert_eq!(settings.stepped(BUTTON_JUDGE_TIMING, NO_ARGUMENT).judge.offset_ms, 1);
    assert_eq!(settings.stepped(BUTTON_JUDGE_TIMING, PREVIOUS).judge.offset_ms, -1);
    for (end, arg1) in [(JUDGE_OFFSET_MAX_MS, NEXT), (JUDGE_OFFSET_MIN_MS, PREVIOUS)] {
        let mut config = Config::default();
        config.judge.offset_ms = end;
        assert_eq!(settings.after(&config, BUTTON_JUDGE_TIMING, arg1).judge.offset_ms, end, "the judge timing left its range");
        assert!(!step_setting(&mut config, SettingEvent::JudgeTiming, arg1), "a judge timing at its end was said to have moved");
    }
}

/// The judge algorithm goes round the three the reference lists, and the fourth this player has is
/// not a place the event steps from.
#[test]
fn the_judge_algorithm_goes_round_the_three_the_reference_lists() {
    let mut settings = Settings::new();
    assert_eq!(settings.stepped(BUTTON_JUDGEALGORITHM, NEXT).judge.judge_algorithm, JudgeAlgorithm::Duration);
    assert_eq!(settings.stepped(BUTTON_JUDGEALGORITHM, PREVIOUS).judge.judge_algorithm, JudgeAlgorithm::Lowest);
    let mut config = Config::default();
    config.judge.judge_algorithm = JudgeAlgorithm::Lowest;
    assert_eq!(settings.after(&config, BUTTON_JUDGEALGORITHM, NEXT).judge.judge_algorithm, JudgeAlgorithm::Combo);
    config.judge.judge_algorithm = JudgeAlgorithm::Score;
    for arg1 in [NEXT, PREVIOUS] {
        assert_eq!(settings.after(&config, BUTTON_JUDGEALGORITHM, arg1).judge.judge_algorithm, JudgeAlgorithm::Score);
    }
}

/// The order of the list and the mode it is held to are stepped by their events, either way.
#[test]
fn the_sort_and_mode_events_step_the_list_either_way() {
    let mut settings = Settings::new();
    let shipped = Config::default().library.sort;
    for id in [BUTTON_SORT, EVENT_SONGBAR_SORT] {
        assert_eq!(settings.stepped(id, NEXT).library.sort, shipped.next(), "event {id}");
        assert_eq!(settings.stepped(id, PREVIOUS).library.sort, shipped.prev(), "event {id}");
    }

    let (mut app, mut state) = browsing();
    let mode = |app: &App, state: &SelectState| state.filter.filter(&app.shared.config).mode;
    state.run_event(&mut app.shared, BUTTON_MODE, NEXT);
    assert_eq!(mode(&app, &state), Some(Mode::BEAT_7K), "the mode after every mode");
    state.run_event(&mut app.shared, BUTTON_MODE, PREVIOUS);
    state.run_event(&mut app.shared, BUTTON_MODE, PREVIOUS);
    assert_eq!(mode(&app, &state), Some(Mode::BEAT_10K), "the mode before every mode, round the end");
}

/// A setting an event changed is the value the settings screen and the option overlay show, since
/// all three read the one field; and it is written out.
#[test]
fn a_setting_an_event_changed_is_the_settings_screens_and_is_written_out() {
    let (mut app, mut state) = browsing();
    state.run_event(&mut app.shared, BUTTON_GAUGE_1P, NEXT);
    assert_eq!(rbms_config::display_value(&app.shared.config, rbms_config::SettingId::Gauge), "HARD");

    rbms_config::adjust(&mut app.shared.config, rbms_config::SettingId::Gauge, NEXT);
    state.run_event(&mut app.shared, BUTTON_GAUGE_1P, PREVIOUS);
    assert_eq!(app.shared.config.play.gauge, GaugeKind::Hard, "the event stepped from something other than the settings row");

    state.settle_option_changes(&mut app.shared);
    let written = rbms_config::load(&app.shared.settings_path).expect("the settings are readable").config;
    assert_eq!(written.play.gauge, GaugeKind::Hard, "the gauge the event chose was not written out");
}

/// The settings of the play under the cursor have nowhere to go on a course that cannot be played,
/// and the events that step them do nothing there; the other settings still move.
#[test]
fn the_play_settings_stand_still_on_a_course_that_cannot_be_played() {
    let absent = rbms_course::CourseChart { md5: "0".repeat(32), sha256: String::new(), title: "absent".to_owned() };
    let course = rbms_course::Course { name: "missing".to_owned(), charts: vec![absent.clone(), absent], ..rbms_course::Course::default() };
    let (mut app, _) = browsing();
    let mut state = SelectState::on_courses(vec![course], &app.shared.library);
    let before = app.shared.config.clone();
    for id in [BUTTON_HSFIX, EVENT_HISPEED, BUTTON_LANECOVER, BUTTON_LIFT, BUTTON_HIDDEN, BUTTON_JUDGEALGORITHM] {
        state.run_event(&mut app.shared, id, NEXT);
        assert!(app.shared.config == before, "event {id} stepped a play setting on a course with no charts");
    }
    state.run_event(&mut app.shared, BUTTON_GAUGE_1P, NEXT);
    assert_eq!(app.shared.config.play.gauge, GaugeKind::Hard);
}

/// The ids the reference gives a meaning this player has nothing for, the ones it hands to the
/// machine's own programs, the assist ids that are no event in the reference either, and an id
/// nobody has: none of them changes a setting, moves the list, stars a chart or leaves the browser.
#[test]
fn an_event_with_nothing_behind_it_does_nothing() {
    let nothing_behind = [
        EVENT_DIFFICULTY,
        BUTTON_READTEXT,
        BUTTON_GAUGE_2P,
        EVENT_DURATION,
        EVENT_BGA_EXPAND,
        BUTTON_RIVAL,
        BUTTON_FAVORITTE_SONG,
        BUTTON_OPEN_IR_WEBSITE,
        EVENT_UPDATE_FOLDER,
        EVENT_OPEN_WITH_EXPLORER,
        EVENT_OPEN_DOWNLOAD_SITE,
        BUTTON_ASSIST_EXJUDGE,
        BUTTON_ASSIST_CONSTANT,
        BUTTON_ASSIST_JUDGEAREA,
        BUTTON_ASSIST_LEGACY,
        BUTTON_ASSIST_MARKNOTE,
        BUTTON_ASSIST_BPMGUIDE,
        BUTTON_ASSIST_NOMINE,
        BUTTON_AUTOSAVEREPLAY_1,
        BUTTON_AUTOSAVEREPLAY_2,
        BUTTON_AUTOSAVEREPLAY_3,
        BUTTON_AUTOSAVEREPLAY_4,
        BUTTON_HISPEEDAUTOADJUST,
        EVENT_CHART_REPLICATION_MODE,
        BUTTON_EXTRANOTE,
        BUTTON_MINEMODE,
        BUTTON_SEVENTONINE_PATTERN,
        BUTTON_SEVENTONINE_TYPE,
        EVENT_CONSTANT,
        EVENT_NOBODY_HAS,
    ];
    let (mut app, mut state) = browsing();
    let before = app.shared.config.clone();
    for id in nothing_behind {
        assert_eq!(event_of(id), None, "event {id}");
        for arg1 in [NEXT, PREVIOUS] {
            assert!(matches!(state.run_event(&mut app.shared, id, arg1), Transition::Stay), "event {id} left the browser");
        }
        assert!(app.shared.config == before, "event {id} changed a setting");
        assert_eq!(app.shared.sel, CHART_ROW, "event {id} moved the list");
        assert!(!app.shared.favorites.contains("md5-alpha"), "event {id} starred the chart");
        assert!(!app.shared.practice_requested && !state.options_dirty);
    }
}

/// The key configuration button opens the key configuration and the skin configuration button the
/// skin tab of the settings, each over the browser.
#[test]
fn the_configuration_buttons_open_the_screens_this_player_keeps_them_on() {
    let (mut app, mut state) = browsing();
    assert!(matches!(state.run_event(&mut app.shared, BUTTON_KEYCONFIG, NO_ARGUMENT), Transition::Open(Stage::KeyConfig(_))));
    match state.run_event(&mut app.shared, BUTTON_SKINSELECT, NO_ARGUMENT) {
        Transition::Open(Stage::Settings(settings)) => assert_eq!(settings.current_tab(), SettingTab::Skin),
        _ => panic!("the skin configuration button did not open the settings"),
    }
}

/// Play starts the chart under the cursor and practice opens the practice screen on it; neither
/// does anything on a folder.
#[test]
fn the_play_and_practice_buttons_start_the_chart_and_leave_a_folder_alone() {
    let (mut app, mut state) = browsing();
    assert!(starts_a_chart(&state.run_event(&mut app.shared, BUTTON_PLAY, NO_ARGUMENT)));
    assert!(!app.shared.practice_requested && !app.shared.run_plays_itself());

    let (mut app, mut state) = browsing();
    assert!(starts_a_chart(&state.run_event(&mut app.shared, BUTTON_PRACTICE, NO_ARGUMENT)));
    assert!(app.shared.practice_requested, "the chart was started rather than practised");

    for id in [BUTTON_PLAY, BUTTON_PRACTICE, BUTTON_AUTOPLAY] {
        let (mut app, mut state) = browsing();
        app.shared.sel = FOLDER_ROW;
        let view = app.shared.select_view;
        assert!(matches!(state.run_event(&mut app.shared, id, NO_ARGUMENT), Transition::Stay), "event {id} started something from a folder");
        assert!(app.shared.select_view == view, "event {id} opened the folder");
        assert!(!app.shared.practice_requested && !app.shared.run_plays_itself());
    }
}

/// Autoplay starts the chart as a run that plays itself and leaves the autoplay setting alone, so
/// the settings written out while the run is on -- which the result and an escape from the play
/// both do -- carry the player's own choice. The next run is the player's own once the browser is
/// arrived back at. A player who has the setting on keeps it on.
#[test]
fn the_autoplay_button_plays_the_chart_by_itself_once() {
    let (mut app, mut state) = browsing();
    assert!(starts_a_chart(&state.run_event(&mut app.shared, BUTTON_AUTOPLAY, NO_ARGUMENT)));
    assert!(app.shared.run_plays_itself(), "the chart was started for the player to play");
    assert!(!app.shared.config.play.autoplay, "the autoplay setting was switched on for the run");

    app.shared.save_settings();
    let written = rbms_config::load(&app.shared.settings_path).expect("the settings are readable").config;
    assert!(!written.play.autoplay, "settings written out during the run say every run plays itself");

    let now = Instant::now();
    state.on_enter(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert!(!app.shared.run_plays_itself(), "the run after it would play itself too");

    let (mut app, mut state) = browsing();
    app.shared.config.play.autoplay = true;
    assert!(starts_a_chart(&state.run_event(&mut app.shared, BUTTON_AUTOPLAY, NO_ARGUMENT)));
    state.on_enter(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert!(app.shared.config.play.autoplay && app.shared.run_plays_itself(), "a player's own autoplay setting was switched off");
}

/// An event that starts nothing leaves no run marked: a folder is not played through.
#[test]
fn an_autoplay_that_starts_nothing_marks_no_run() {
    let (mut app, mut state) = browsing();
    app.shared.select_view = SelectView::Root;
    app.shared.sel = 0;
    app.shared.rebuild_select_items();
    assert!(matches!(state.run_event(&mut app.shared, BUTTON_AUTOPLAY, NO_ARGUMENT), Transition::Stay));
    assert!(!app.shared.run_plays_itself());
}

/// The four replay slots are the replays saved of the chart, the newest first; a record with no
/// replay takes no slot.
#[test]
fn the_replay_slots_are_the_newest_replays_of_the_chart() {
    let with_replay = |played_at: i64, file: Option<&str>| played("md5-alpha", played_at, file);
    let other = played("md5-beta", 9, Some("other"));
    let scores = ScoreBook::from_records(vec![
        with_replay(1, Some("first")),
        with_replay(2, None),
        with_replay(3, Some("third")),
        other,
        with_replay(4, Some("fourth")),
        with_replay(5, Some("fifth")),
        with_replay(6, Some("sixth")),
    ]);
    let slots = [0, 1, 2, 3, 4].map(|slot| replay_in_slot(&scores, "md5-alpha", slot));
    assert_eq!(slots, [Some("sixth"), Some("fifth"), Some("fourth"), Some("third"), Some("first")].map(|file| file.map(str::to_owned)));
    assert_eq!(replay_in_slot(&scores, "md5-beta", 1), None);
    assert_eq!(replay_in_slot(&scores, "md5-nobody", 0), None);
    assert_eq!([BUTTON_REPLAY, BUTTON_REPLAY2, BUTTON_REPLAY3, BUTTON_REPLAY4].map(event_of), [0, 1, 2, 3].map(|slot| Some(SelectEvent::Replay(slot))));
}

/// A replay button with nothing in its slot does nothing, on a chart nobody has a replay of and on
/// a folder; and a replay that cannot be read leaves the browser where it is.
#[test]
fn a_replay_button_with_no_replay_to_play_does_nothing() {
    for id in [BUTTON_REPLAY, BUTTON_REPLAY2, BUTTON_REPLAY3, BUTTON_REPLAY4] {
        let (mut app, mut state) = browsing();
        assert!(matches!(state.run_event(&mut app.shared, id, NO_ARGUMENT), Transition::Stay), "event {id} on a chart with no replay");
        app.shared.sel = FOLDER_ROW;
        assert!(matches!(state.run_event(&mut app.shared, id, NO_ARGUMENT), Transition::Stay), "event {id} on a folder");
        assert!(app.shared.replay.is_none());
    }

    let (mut app, mut state) = browsing();
    let unreadable = played("md5-alpha", 1, Some("not-there.rbmsreplay"));
    app.shared.scores = ScoreBook::from_records(vec![unreadable]);
    assert!(matches!(state.run_event(&mut app.shared, BUTTON_REPLAY, NO_ARGUMENT), Transition::Stay));
    assert!(matches!(state.run_event(&mut app.shared, BUTTON_REPLAY2, NO_ARGUMENT), Transition::Stay));
    assert!(app.shared.replay.is_none());
}

/// The favourite button stars the chart under the cursor and unstars it, as the browser's own key
/// does, and leaves a folder alone.
#[test]
fn the_favourite_button_stars_the_chart_under_the_cursor() {
    let (mut app, mut state) = browsing();
    state.run_event(&mut app.shared, BUTTON_FAVORITTE_CHART, NEXT);
    assert!(app.shared.favorites.contains("md5-alpha"));
    state.run_event(&mut app.shared, BUTTON_FAVORITTE_CHART, PREVIOUS);
    assert!(!app.shared.favorites.contains("md5-alpha"));

    let generation = app.shared.select_gen;
    app.shared.sel = FOLDER_ROW;
    state.run_event(&mut app.shared, BUTTON_FAVORITTE_CHART, NEXT);
    assert_eq!(app.shared.select_gen, generation, "a folder was starred");
}

/// Every id the table gives a meaning is routed to the browser or to the settings by the host, so
/// a button of the skin that runs one reaches the browser.
#[test]
fn every_event_the_browser_runs_is_one_the_host_routes_to_it() {
    for id in 0..EVENT_NOBODY_HAS {
        if event_of(id).is_none() {
            continue;
        }
        let routed = crate::skin_host::event_clusters(id).any(|cluster| matches!(cluster, Cluster::Select | Cluster::Options));
        assert!(routed, "event {id} is run by the browser and never reaches it");
    }
}

/// What a button of the skin asked for is carried out on the frame that follows, from the
/// browser's own queue and from the settings', and a setting it changed is written out.
#[test]
fn the_events_a_skin_asked_for_are_carried_out_on_the_frame_that_follows() {
    let mut browser = Browser::three_songs("events-queue");
    let shipped_sort = browser.app.shared.config.library.sort;
    browser.app.shared.skin_requests().push(Cluster::Options, ClusterRequest::Event { id: BUTTON_GAUGE_1P, arg1: PREVIOUS, arg2: 0 });
    browser.app.shared.skin_requests().push(Cluster::Select, ClusterRequest::Event { id: BUTTON_SORT, arg1: NEXT, arg2: 0 });
    browser.app.shared.skin_requests().push(Cluster::Options, ClusterRequest::Event { id: BUTTON_RANDOM_1P, arg1: NEXT, arg2: 0 });
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Stay));

    let config = &browser.app.shared.config;
    assert_eq!((config.play.gauge, config.play.random, config.library.sort), (GaugeKind::Easy, NoteOption::Mirror, shipped_sort.next()));
    let written = rbms_config::load(&browser.app.shared.settings_path).expect("the settings are readable").config;
    assert_eq!((written.play.gauge, written.play.random), (GaugeKind::Easy, NoteOption::Mirror), "what the buttons chose was not written out");
    assert_eq!(browser.app.shared.skin_requests().len(), 0, "an event was left in the queue");
}

/// A button that leaves the browser does so on the frame that follows, and what was asked for
/// after it is not carried out.
#[test]
fn a_button_that_leaves_the_browser_ends_the_frames_events() {
    let mut browser = Browser::three_songs("events-leave");
    browser.app.shared.skin_requests().push(Cluster::Select, ClusterRequest::Event { id: BUTTON_KEYCONFIG, arg1: NO_ARGUMENT, arg2: 0 });
    browser.app.shared.skin_requests().push(Cluster::Options, ClusterRequest::Event { id: BUTTON_GAUGE_1P, arg1: NEXT, arg2: 0 });
    assert!(matches!(browser.frame_after(SHORT_STEP), Transition::Open(Stage::KeyConfig(_))));
    assert_eq!(browser.app.shared.config.play.gauge, GaugeKind::Normal);
}

/// With the record modal up an event closes it and does nothing else, as a press on a bar does.
#[test]
fn an_event_closes_the_record_modal_instead_of_running() {
    let mut browser = Browser::three_songs("events-modal");
    browser.state.record_modal = Some(0);
    browser.app.shared.skin_requests().push(Cluster::Options, ClusterRequest::Event { id: BUTTON_GAUGE_1P, arg1: NEXT, arg2: 0 });
    browser.frame_after(SHORT_STEP);
    assert_eq!(browser.state.record_modal, None);
    assert_eq!(browser.app.shared.config.play.gauge, GaugeKind::Normal);
}

/// The two keys the reference's browser opens its configuration screens with do what the buttons
/// do, on a browser a skin draws.
#[test]
fn the_keys_for_the_configuration_screens_open_them_on_a_skins_browser() {
    let mut browser = Browser::three_songs("events-keys");
    assert!(matches!(browser.key(KeyCode::Digit6), Transition::Open(Stage::KeyConfig(_))));
    match browser.key(KeyCode::F12) {
        Transition::Open(Stage::Settings(settings)) => assert_eq!(settings.current_tab(), SettingTab::Skin),
        _ => panic!("F12 did not open the settings"),
    }
}

/// On a browser no skin draws neither key is anything, and no event is taken from the queue.
#[test]
fn a_browser_no_skin_draws_runs_no_event() {
    let (mut app, mut state) = browsing();
    let now = Instant::now();
    for code in [KeyCode::Digit6, KeyCode::F12] {
        let asked = state.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, super::tests::press(code));
        assert!(matches!(asked, Transition::Stay), "{code:?} left a browser no skin draws");
    }
    app.shared.skin_requests().push(Cluster::Options, ClusterRequest::Event { id: BUTTON_GAUGE_1P, arg1: NEXT, arg2: 0 });
    state.update(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 });
    assert_eq!(app.shared.config.play.gauge, GaugeKind::Normal);
}
