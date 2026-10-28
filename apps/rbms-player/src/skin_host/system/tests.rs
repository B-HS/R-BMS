//! Unit tests for the system cluster: the clock is read as it was handed over, every total is
//! summed as the reference sums its player data, and a source the frame did not bring is not
//! answered.

use std::borrow::Cow;
use std::path::Path;

use rbms_model::Language;
use rbms_skin::lua::{LocalTime, SkinLua, SkinLuaConfig};
use rbms_skin::property::VolumeBus;
use rbms_skin::property::generated::*;
use rbms_skin::timer::TimerState;
use rbms_store::{SCORE_LN_MODE_FROM_CHART, ScoreRecord};

use super::{CourseStage, PlayerTotals, SystemState, Volumes};
use crate::skin_host::extension::{LANGUAGE_OPTIONS, RBMS_OPTION_LANGUAGE_FIRST, RBMS_OPTION_LANGUAGE_LAST};
use crate::skin_host::{ClusterState, ScreenHost};

/// The lamp ids a run can end on that matter to the totals: failed, and the first and last clear.
const LAMP_FAILED: u8 = 1;
const LAMP_ASSIST_EASY: u8 = 2;
const LAMP_MAX: u8 = 10;

/// The default skin pack, from this package's folder.
const DEFAULT_PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// A date and a time of day whose six fields all differ.
const SAMPLE_CLOCK: LocalTime = LocalTime { year: 2023, month: 11, day: 14, hour: 22, minute: 13, second: 20 };

fn record(clear: u8, counts: [u32; 6]) -> ScoreRecord {
    ScoreRecord {
        md5: "md5".to_owned(),
        title: "title".to_owned(),
        mode: "BEAT_7K".to_owned(),
        clear,
        ex_score: 0,
        max_ex: 0,
        counts,
        empty_poor: 0,
        max_combo: 0,
        total_notes: 0,
        gauge: "NORMAL".to_owned(),
        gauge_value: 0.0,
        random: "OFF".to_owned(),
        played_at: 0,
        replay_file: None,
        rule_version: 0,
        ln_mode: SCORE_LN_MODE_FROM_CHART.to_owned(),
        assisted: false,
        assist: 0,
        log_id: None,
    }
}

fn history() -> Vec<ScoreRecord> {
    vec![record(LAMP_FAILED, [10, 5, 3, 2, 1, 4]), record(LAMP_ASSIST_EASY, [100, 20, 7, 3, 2, 1]), record(LAMP_MAX, [200, 0, 0, 0, 0, 0])]
}

#[test]
fn the_clock_numbers_read_the_date_and_the_time_of_day() {
    let state = SystemState { clock: Some(SAMPLE_CLOCK), ..SystemState::default() };

    assert_eq!(
        [NUMBER_TIME_YEAR, NUMBER_TIME_MONTH, NUMBER_TIME_DAY, NUMBER_TIME_HOUR, NUMBER_TIME_MINUTE, NUMBER_TIME_SECOND].map(|id| state.integer(id)),
        [Some(2023), Some(11), Some(14), Some(22), Some(13), Some(20)]
    );
}

#[test]
fn the_uptime_numbers_split_the_milliseconds_into_hours_minutes_and_seconds() {
    let uptime_ms = (26 * 3600 + 7 * 60 + 9) * 1000 + 999;
    let state = SystemState { uptime_ms: Some(uptime_ms), ..SystemState::default() };

    assert_eq!(state.integer(NUMBER_OPERATING_TIME_HOUR), Some(26), "the hours are not wrapped at a day");
    assert_eq!(state.integer(NUMBER_OPERATING_TIME_MINUTE), Some(7));
    assert_eq!(state.integer(NUMBER_OPERATING_TIME_SECOND), Some(9));
}

#[test]
fn the_frame_rate_is_a_whole_number() {
    assert_eq!(SystemState { fps: Some(59.6), ..SystemState::default() }.integer(NUMBER_CURRENT_FPS), Some(60));
    assert_eq!(SystemState { fps: Some(143.4), ..SystemState::default() }.integer(NUMBER_CURRENT_FPS), Some(143));
}

#[test]
fn the_totals_are_summed_from_the_recorded_runs() {
    let records = history();
    let totals = PlayerTotals::of_records(&records);

    assert_eq!(totals, PlayerTotals { plays: 3, clears: 2, judgements: [310, 25, 10, 5, 3, 5] });
    assert_eq!(PlayerTotals::of_records(&[]), PlayerTotals::default());
}

#[test]
fn the_total_numbers_read_the_player_data_as_the_reference_does() {
    let records = history();
    let state = SystemState { history: Some(&records), ..SystemState::default() };

    assert_eq!(state.integer(NUMBER_TOTALPLAYCOUNT), Some(3));
    assert_eq!(state.integer(NUMBER_TOTALCLEARCOUNT), Some(2), "a clear is a lamp above failed");
    assert_eq!(state.integer(NUMBER_TOTALFAILCOUNT), Some(1), "the fails are the plays less the clears");
    assert_eq!(
        [NUMBER_TOTALPERFECT, NUMBER_TOTALGREAT, NUMBER_TOTALGOOD, NUMBER_TOTALBAD, NUMBER_TOTALPOOR].map(|id| state.integer(id)),
        [Some(310), Some(25), Some(10), Some(5), Some(3)]
    );
    assert_eq!(state.integer(NUMBER_TOTALPLAYNOTES), Some(350), "poor and miss are not notes played");
}

#[test]
fn an_empty_history_is_a_player_who_has_played_nothing_and_no_history_is_not_known() {
    let none: [ScoreRecord; 0] = [];
    let played_nothing = SystemState { history: Some(&none), ..SystemState::default() };
    assert_eq!(played_nothing.integer(NUMBER_TOTALPLAYCOUNT), Some(0));
    assert_eq!(played_nothing.integer(NUMBER_TOTALPLAYNOTES), Some(0));

    let unknown = SystemState::default();
    for id in [NUMBER_TOTALPLAYCOUNT, NUMBER_TOTALCLEARCOUNT, NUMBER_TOTALFAILCOUNT, NUMBER_TOTALPERFECT, NUMBER_TOTALPOOR, NUMBER_TOTALPLAYNOTES] {
        assert_eq!(unknown.integer(id), None, "number {id}");
    }
}

#[test]
fn the_total_play_time_is_not_answered_because_the_scores_do_not_keep_it() {
    let records = history();
    let state = SystemState { history: Some(&records), ..SystemState::default() };

    for id in [NUMBER_TOTALPLAYTIME_HOUR, NUMBER_TOTALPLAYTIME_MINUTE, NUMBER_TOTALPLAYTIME_SECOND] {
        assert_eq!(state.integer(id), None, "number {id}");
    }
}

#[test]
fn the_volumes_are_read_as_a_share_a_percent_and_a_bus() {
    let volumes = Volumes { master: 0.75, system: 0.5, key: 0.25, background: 1.0 };
    let state = SystemState { volumes: Some(volumes), ..SystemState::default() };

    assert_eq!(state.volume(VolumeBus::System), Some(0.5), "main_state.volume_sys is still the system volume");
    assert_eq!(state.volume(VolumeBus::Key), Some(0.25));
    assert_eq!(state.volume(VolumeBus::Background), Some(1.0));
    assert_eq!(
        [NUMBER_MASTER_VOLUME, NUMBER_KEY_VOLUME, NUMBER_BGM_VOLUME].map(|id| state.integer(id)),
        [Some(75), Some(25), Some(100)],
        "number 57 shows the master volume, not the system volume"
    );
    assert_eq!(
        [RATE_MASTERVOLUME, RATE_KEYVOLUME, RATE_BGMVOLUME].map(|id| state.rate(id)),
        [Some(0.75), Some(0.25), Some(1.0)],
        "rate 17 is the master volume, not the system volume"
    );

    let unknown = SystemState::default();
    assert_eq!(unknown.volume(VolumeBus::System), None);
    assert_eq!(unknown.integer(NUMBER_MASTER_VOLUME), None);
    assert_eq!(unknown.rate(RATE_MASTERVOLUME), None);
}

#[test]
fn a_volume_percent_is_cut_and_not_rounded() {
    let state = SystemState { volumes: Some(Volumes { master: 0.999, system: 0.0, key: 0.0, background: 0.0 }), ..SystemState::default() };

    assert_eq!(state.integer(NUMBER_MASTER_VOLUME), Some(99));
}

#[test]
fn the_texts_are_the_player_and_the_version() {
    let state = SystemState { player_name: Some("guest"), version: Some("R-BMS 0.1.0"), ..SystemState::default() };

    assert_eq!(state.text(STRING_PLAYER), Some(Cow::Borrowed("guest")));
    assert_eq!(state.text(STRING_VERSION), Some(Cow::Borrowed("R-BMS 0.1.0")));
    assert_eq!(SystemState::default().text(STRING_PLAYER), None);
    assert_eq!(state.text(STRING_TITLE), None);
}

#[test]
fn a_single_chart_is_not_a_course_and_every_course_option_says_so() {
    let state = SystemState::default();

    assert_eq!(state.boolean(OPTION_MODE_COURSE), Some(false));
    for stage in OPTION_COURSE_STAGE1..=OPTION_COURSE_STAGE4 {
        assert_eq!(state.boolean(stage), Some(false), "stage option {stage}");
    }
    assert_eq!(state.boolean(OPTION_COURSE_STAGE_FINAL), Some(false));
}

#[test]
fn the_stage_options_follow_the_stage_of_the_course_being_played() {
    let stage_options = [OPTION_COURSE_STAGE1, OPTION_COURSE_STAGE2, OPTION_COURSE_STAGE3, OPTION_COURSE_STAGE4, OPTION_COURSE_STAGE_FINAL];
    for (index, on) in [(0, OPTION_COURSE_STAGE1), (1, OPTION_COURSE_STAGE2), (2, OPTION_COURSE_STAGE3), (3, OPTION_COURSE_STAGE_FINAL)] {
        let state = SystemState { course: Some(CourseStage { index, count: 4 }), ..SystemState::default() };

        assert_eq!(state.boolean(OPTION_MODE_COURSE), Some(true));
        assert_eq!(stage_options.into_iter().filter(|id| state.boolean(*id) == Some(true)).collect::<Vec<_>>(), [on], "stage {index} of 4");
    }
}

#[test]
fn a_course_of_one_stage_is_all_final() {
    let state = SystemState { course: Some(CourseStage { index: 0, count: 1 }), ..SystemState::default() };

    assert_eq!(state.boolean(OPTION_COURSE_STAGE1), Some(false));
    assert_eq!(state.boolean(OPTION_COURSE_STAGE_FINAL), Some(true));
}

#[test]
fn ids_of_other_clusters_are_not_answered() {
    let records = history();
    let state = SystemState {
        clock: Some(SAMPLE_CLOCK),
        uptime_ms: Some(1),
        fps: Some(60.0),
        player_name: Some("guest"),
        version: Some("v"),
        history: Some(&records),
        volumes: Some(Volumes { master: 1.0, system: 1.0, key: 1.0, background: 1.0 }),
        course: Some(CourseStage { index: 0, count: 2 }),
        language: Some(Language::Korean),
    };

    assert_eq!(state.integer(NUMBER_PLAYLEVEL), None);
    assert_eq!(state.integer(NUMBER_SCORE), None);
    assert_eq!(state.boolean(OPTION_DIFFICULTY0), None);
    assert_eq!(state.boolean(OPTION_NOW_LOADING), None);
    assert_eq!(state.text(STRING_TITLE), None);
    assert_eq!(state.rate(RATE_LOAD_PROGRESS), None);
    assert_eq!(state.key_pressed(0), None, "nothing fills in the keys held yet");
}

#[test]
fn exactly_one_language_option_is_on_and_it_is_the_one_of_the_resolved_language() {
    for language in Language::ALL {
        let state = SystemState { language: Some(language), ..SystemState::default() };
        let on: Vec<i32> = (RBMS_OPTION_LANGUAGE_FIRST..=RBMS_OPTION_LANGUAGE_LAST).filter(|id| state.boolean(*id) == Some(true)).collect();

        let id = i32::try_from(language.id()).expect("a language id fits");
        assert_eq!(on, [RBMS_OPTION_LANGUAGE_FIRST + id], "{language:?}");
    }
}

#[test]
fn every_option_of_the_language_run_answers_and_the_ones_kept_for_later_languages_are_off() {
    let state = SystemState { language: Some(Language::Japanese), ..SystemState::default() };

    assert_eq!(RBMS_OPTION_LANGUAGE_LAST - RBMS_OPTION_LANGUAGE_FIRST + 1, LANGUAGE_OPTIONS);
    for id in RBMS_OPTION_LANGUAGE_FIRST..=RBMS_OPTION_LANGUAGE_LAST {
        assert!(state.boolean(id).is_some(), "option {id} is in the run and is answered");
    }
    let written = i32::try_from(Language::ALL.len()).expect("the languages fit");
    let kept_for_later = (RBMS_OPTION_LANGUAGE_FIRST + written)..=RBMS_OPTION_LANGUAGE_LAST;
    assert!(kept_for_later.map(|id| state.boolean(id)).all(|answer| answer == Some(false)));
}

#[test]
fn a_frame_that_brought_no_language_leaves_the_language_options_unanswered() {
    let state = SystemState::default();

    for id in RBMS_OPTION_LANGUAGE_FIRST..=RBMS_OPTION_LANGUAGE_LAST {
        assert_eq!(state.boolean(id), None, "option {id}");
    }
}

#[test]
fn the_ids_beside_the_language_run_are_not_the_languages() {
    let state = SystemState { language: Some(Language::English), ..SystemState::default() };

    assert_eq!(state.boolean(RBMS_OPTION_LANGUAGE_FIRST - 1), None);
    assert_eq!(state.boolean(RBMS_OPTION_LANGUAGE_LAST + 1), None);
    assert_eq!(state.integer(RBMS_OPTION_LANGUAGE_FIRST), None, "the options are a space of their own");
}

#[test]
fn a_text_of_the_default_skin_is_read_in_the_language_the_host_answers_for_the_frame() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER);
    let runtime = SkinLua::new(SkinLuaConfig::new(&pack)).expect("the pack's interpreter builds");
    let timers = TimerState::new();
    let build = "hello = require('lua.i18n').translator({ { hello = { en = 'Hello', ko = 'Annyeong', ja = 'Konnichiwa' } } })('hello')";
    runtime.with_host(&ScreenHost::new(0, &timers), || runtime.lua().load(build).exec()).expect("the host binds").expect("the text is built");

    let mut heard = Vec::new();
    for language in [Language::English, Language::Korean, Language::Japanese, Language::Korean] {
        let mut host = ScreenHost::new(0, &timers);
        host.system = SystemState { language: Some(language), ..SystemState::default() };
        heard.push(runtime.with_host(&host, || runtime.lua().load("return hello()").eval::<String>()).expect("the host binds").expect("the text is read"));
    }
    assert_eq!(heard, ["Hello", "Annyeong", "Konnichiwa", "Annyeong"]);

    let silent = ScreenHost::new(0, &timers);
    let unsaid = runtime.with_host(&silent, || runtime.lua().load("return hello()").eval::<String>()).expect("the host binds").expect("the text is read");
    assert_eq!(unsaid, "Hello", "a frame that brought no language is read as English");
}
