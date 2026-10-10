//! The options cluster answers a result screen with the choices the run was played with.

use rbms_chart::shuffle::NoteOption;
use rbms_config::{Config, FixHiSpeed, JUDGE_RATE_DEFAULT_PERCENT, LaneOption, ScoreTarget};
use rbms_judge::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeAutoShift, GaugeIndex, GaugeKind};
use rbms_judge::ln::LnMode;
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};

use super::*;
use crate::skin_host::result::snapshot::{FinishedRun, ResultInput, ResultSnapshot};
use crate::skin_host::{INDEX_CLEAR, ResultScene, STRING_TARGET_NAME, STRING_TARGET_NEXT_FIRST, STRING_TARGET_PREVIOUS_FIRST};

fn snapshot(options: PlayedOptions, updates_score: bool) -> ResultSnapshot {
    ResultSnapshot::of(ResultInput { options, favorite_chart: Some(true), updates_score, ..ResultInput::default() })
}

/// The finished run on the screen, with the graph on the hard gauge.
fn on_screen(snapshot: &ResultSnapshot) -> OptionsState<'_> {
    OptionsState::of_result(FinishedRun::new(snapshot, ResultScene { gauge_type: GaugeIndex::Hard.index(), ..ResultScene::default() }))
}

#[test]
fn the_random_option_is_numbered_as_the_reference_numbers_its_options() {
    let order = [
        NoteOption::Off,
        NoteOption::Mirror,
        NoteOption::Random,
        NoteOption::RRandom,
        NoteOption::SRandom,
        NoteOption::Rotate,
        NoteOption::HRandom,
        NoteOption::AllScratch,
    ];

    assert_eq!(order.map(random_option_index), [0, 1, 2, 3, 4, 5, 6, 7]);
    assert!(NoteOption::ALL.iter().all(|option| order.contains(option)), "every option the player has is numbered");
}

#[test]
fn the_choices_of_the_run_are_what_a_result_screen_shows() {
    let played = PlayedOptions {
        random: 2,
        random_2p: 5,
        double_option: 1,
        custom_judge: true,
        constant_scroll: false,
        judge_area: true,
        legacy_long_note: false,
        mark_note: true,
        bpm_guide: false,
        no_mine: true,
        long_note_mode: 1,
    };
    let snapshot = snapshot(played, true);
    let state = on_screen(&snapshot);

    assert_eq!((state.image_index(BUTTON_RANDOM_1P), state.image_index(BUTTON_RANDOM_2P), state.image_index(BUTTON_DPOPTION)), (Some(2), Some(5), Some(1)));
    assert_eq!(state.image_index(BUTTON_GAUGE_1P), Some(3), "the gauge the screen has the graph on, numbered as the reference numbers gauges");
    let assists = (BUTTON_ASSIST_EXJUDGE..=BUTTON_ASSIST_NOMINE).map(|id| state.image_index(id)).collect::<Vec<_>>();
    assert_eq!(assists, [Some(1), Some(0), Some(1), Some(0), Some(1), Some(0), Some(1)]);
    assert_eq!(state.image_index(BUTTON_LNMODE), Some(1));
}

#[test]
fn the_options_of_the_target_and_a_favourite_nobody_knows_are_absent() {
    let snapshot = snapshot(PlayedOptions::default(), true);
    let state = on_screen(&snapshot);

    for id in INDEX_TARGET_OPTION_FIRST..=INDEX_TARGET_OPTION_LAST {
        assert_eq!(state.image_index(id), Some(IMAGE_INDEX_ABSENT), "image index {id}");
    }
    assert_eq!(state.image_index(BUTTON_FAVORITTE_CHART), Some(1), "a chart the player has starred");
    assert_eq!(state.image_index(BUTTON_FAVORITTE_SONG), Some(IMAGE_INDEX_ABSENT), "and a song the player cannot star");
}

#[test]
fn a_run_that_counts_towards_the_bests_enables_saving_and_a_run_that_does_not_disables_it() {
    let counting = snapshot(PlayedOptions::default(), true);
    let state = on_screen(&counting);
    assert_eq!(
        (state.boolean(OPTION_DISABLE_SAVE_SCORE), state.boolean(OPTION_ENABLE_SAVE_SCORE), state.boolean(OPTION_NO_SAVE_CLEAR)),
        (Some(false), Some(true), Some(true))
    );

    let practice = snapshot(PlayedOptions::default(), false);
    let state = on_screen(&practice);
    assert_eq!(
        (state.boolean(OPTION_DISABLE_SAVE_SCORE), state.boolean(OPTION_ENABLE_SAVE_SCORE), state.boolean(OPTION_NO_SAVE_CLEAR)),
        (Some(true), Some(false), Some(false)),
        "the clear option reads the same as the enable option, as it does in the reference"
    );
}

#[test]
fn the_settings_of_the_configuration_screens_are_not_answered_on_a_result_screen() {
    let snapshot = snapshot(PlayedOptions::default(), true);
    let state = on_screen(&snapshot);

    for id in [BUTTON_HSFIX, BUTTON_BGA, BUTTON_GAUGEAUTOSHIFT, BUTTON_LANECOVER, BUTTON_JUDGEALGORITHM, BUTTON_EXTRANOTE] {
        assert_eq!(state.image_index(id), None, "image index {id}");
    }
}

#[test]
fn without_a_snapshot_nothing_is_answered() {
    let state = OptionsState::default();

    assert_eq!(state.image_index(BUTTON_RANDOM_1P), None);
    assert_eq!(state.boolean(OPTION_ENABLE_SAVE_SCORE), None);
}

fn settings(config: &Config) -> OptionsState<'static> {
    OptionsState::of_settings(SettingsView::of_config(config))
}

#[test]
fn the_default_settings_are_what_the_browser_shows_in_the_reference_s_numbering() {
    let state = settings(&Config::default());

    assert_eq!(state.image_index(BUTTON_GAUGE_1P), Some(2), "the normal gauge is the third of the reference's");
    assert_eq!((state.image_index(BUTTON_RANDOM_1P), state.image_index(BUTTON_RANDOM_2P), state.image_index(BUTTON_DPOPTION)), (Some(0), Some(0), Some(0)));
    assert_eq!(state.image_index(BUTTON_HSFIX), Some(3), "the tempo most notes are played at is the fourth choice");
    assert_eq!(state.image_index(BUTTON_BGA), Some(0));
    assert_eq!(
        (state.image_index(INDEX_TIMING_AUTO_ADJUST), state.image_index(BUTTON_GAUGEAUTOSHIFT), state.image_index(BUTTON_BOTTOMSIFTABLEFGAUGE)),
        (Some(0), Some(0), Some(0))
    );
    assert_eq!(
        (state.image_index(BUTTON_ASSIST_EXJUDGE), state.image_index(BUTTON_ASSIST_CONSTANT), state.image_index(BUTTON_ASSIST_LEGACY)),
        (Some(0), Some(0), Some(0))
    );
    assert_eq!(state.image_index(BUTTON_LNMODE), Some(0));
    assert_eq!((state.image_index(BUTTON_LANECOVER), state.image_index(BUTTON_LIFT), state.image_index(BUTTON_HIDDEN)), (Some(1), Some(0), Some(0)));
    assert_eq!(state.image_index(BUTTON_JUDGEALGORITHM), Some(0));
}

#[test]
fn a_changed_setting_is_the_number_the_reference_gives_the_choice() {
    let mut config = Config::default();
    config.play.gauge = GaugeKind::ExHard;
    config.play.random = NoteOption::HRandom;
    config.play.lane_option = LaneOption::Battle;
    config.play.fix_hispeed = FixHiSpeed::MinBpm;
    config.play.constant_speed = true;
    config.play.legacy_note = true;
    config.play.enable_cover = false;
    config.play.enable_lift = true;
    config.play.enable_hidden = true;
    config.display.bga = false;
    config.judge.auto_offset = true;
    config.judge.gauge_auto_shift = GaugeAutoShift::SelectToUnder;
    config.judge.bottom_shiftable_gauge = GaugeKind::Normal;
    config.judge.judge_algorithm = JudgeAlgorithm::Lowest;
    config.judge.ln_mode = LnMode::HellChargeNote;
    config.judge.judge_rate_key[0] = JUDGE_RATE_DEFAULT_PERCENT + 1;
    config.audio.guide_se = true;
    let state = settings(&config);

    let read = |ids: &[i32]| ids.iter().map(|id| state.image_index(*id)).collect::<Vec<_>>();
    assert_eq!(
        read(&[BUTTON_GAUGE_1P, BUTTON_RANDOM_1P, BUTTON_RANDOM_2P, BUTTON_DPOPTION, BUTTON_HSFIX, BUTTON_BGA]),
        [Some(4), Some(6), Some(6), Some(2), Some(4), Some(2)]
    );
    assert_eq!(
        read(&[INDEX_TIMING_AUTO_ADJUST, BUTTON_GAUGEAUTOSHIFT, BUTTON_BOTTOMSIFTABLEFGAUGE, BUTTON_JUDGEALGORITHM]),
        [Some(1), Some(4), Some(2), Some(2)]
    );
    assert_eq!(
        read(&[BUTTON_ASSIST_EXJUDGE, BUTTON_ASSIST_CONSTANT, BUTTON_ASSIST_LEGACY, BUTTON_LNMODE]),
        [Some(1), Some(1), Some(1), Some(2)],
        "a widened judge window is the custom judge"
    );
    assert_eq!(read(&[BUTTON_SCROLLMODE, BUTTON_LONGNOTEMODE]), [Some(1), Some(1)]);
    assert_eq!(read(&[BUTTON_LANECOVER, BUTTON_LIFT, BUTTON_HIDDEN]), [Some(0), Some(1), Some(1)]);
    assert_eq!(read(&[INDEX_GUIDE_SOUND]), [Some(1)]);
    assert_eq!(settings(&Config::default()).image_index(INDEX_GUIDE_SOUND), Some(0), "the guide sound ships switched off");
}

#[test]
fn what_this_player_has_no_setting_for_reads_absent_and_not_off() {
    let state = settings(&Config::default());
    let no_setting = [
        BUTTON_ASSIST_JUDGEAREA,
        BUTTON_ASSIST_MARKNOTE,
        BUTTON_ASSIST_BPMGUIDE,
        BUTTON_ASSIST_NOMINE,
        BUTTON_AUTOSAVEREPLAY_1,
        BUTTON_AUTOSAVEREPLAY_4,
        BUTTON_HISPEEDAUTOADJUST,
        BUTTON_EXTRANOTE,
        BUTTON_MINEMODE,
        BUTTON_SEVENTONINE_PATTERN,
        BUTTON_SEVENTONINE_TYPE,
        INDEX_CONSTANT,
        INDEX_TARGET_OPTION_FIRST,
        INDEX_TARGET_OPTION_LAST,
    ];

    for id in no_setting {
        assert_eq!(state.image_index(id), Some(IMAGE_INDEX_ABSENT), "image index {id}");
    }
    let mut config = Config::default();
    config.judge.judge_algorithm = JudgeAlgorithm::Score;
    assert_eq!(settings(&config).image_index(BUTTON_JUDGEALGORITHM), Some(IMAGE_INDEX_ABSENT), "the reference's list has no place for the fourth algorithm");
}

#[test]
fn the_settings_leave_the_ids_of_the_browser_and_of_the_bars_to_others() {
    let state = settings(&Config::default());

    for id in [BUTTON_MODE, BUTTON_SORT, BUTTON_FAVORITTE_CHART, BUTTON_FAVORITTE_SONG, INDEX_CLEAR] {
        assert_eq!(state.image_index(id), None, "image index {id}");
    }
    assert_eq!(state.boolean(OPTION_ENABLE_SAVE_SCORE), None);
}

#[test]
fn the_scroll_speed_is_read_three_ways_and_only_on_a_chart() {
    let mut config = Config::default();
    config.play.hispeed = 2.5;
    config.judge.offset_ms = -12;
    let view = SettingsView::of_config(&config);
    let state = OptionsState::of_settings(view);

    assert_eq!((state.integer(NUMBER_HISPEED_LR2), state.integer(NUMBER_HISPEED), state.integer(NUMBER_HISPEED_AFTERDOT)), (Some(250), Some(2), Some(50)));
    assert_eq!(state.float(FLOAT_HISPEED), Some(2.5));
    assert_eq!(state.integer(NUMBER_JUDGETIMING), Some(-12));

    let folder = OptionsState::of_settings(view.without_chart());
    assert_eq!(
        (folder.integer(NUMBER_HISPEED_LR2), folder.integer(NUMBER_HISPEED), folder.integer(NUMBER_HISPEED_AFTERDOT)),
        (Some(INTEGER_ABSENT), Some(INTEGER_ABSENT), Some(INTEGER_ABSENT))
    );
    assert_eq!(folder.float(FLOAT_HISPEED), Some(FLOAT_ABSENT));
    assert_eq!(folder.image_index(BUTTON_HSFIX), Some(IMAGE_INDEX_ABSENT), "a folder has no tempo to pin");
    assert_eq!(folder.integer(NUMBER_JUDGETIMING), Some(-12), "the judge timing is the player's, wherever the cursor is");
    assert_eq!((state.integer(NUMBER_DURATION), state.integer(NUMBER_DURATION_GREEN)), (Some(INTEGER_ABSENT), Some(INTEGER_ABSENT)));
}

#[test]
fn the_target_and_the_ones_around_it_in_the_list_are_named_with_the_list_wrapping_round() {
    let name = |state: &OptionsState<'_>, id: i32| state.text(id).map(|text| text.into_owned());
    let mut config = Config::default();
    config.judge.target = ScoreTarget::RateAaa;
    let state = settings(&config);

    assert_eq!(name(&state, STRING_TARGET_NAME), Some("RATE AAA".to_string()));
    assert_eq!(name(&state, STRING_TARGET_NEXT_FIRST), Some("RATE AAA+".to_string()), "one on");
    assert_eq!(name(&state, STRING_TARGET_NEXT_FIRST + 9), Some("RATE A+".to_string()), "ten on, past the end of the list");
    assert_eq!(name(&state, STRING_TARGET_PREVIOUS_FIRST + 9), Some("RATE AAA-".to_string()), "one back");
    assert_eq!(name(&state, STRING_TARGET_PREVIOUS_FIRST), Some("IR BEST".to_string()), "ten back, past the start of the list");

    config.judge.target = ScoreTarget::Max;
    let first = settings(&config);
    assert_eq!(name(&first, STRING_TARGET_PREVIOUS_FIRST + 9), Some("RIVAL".to_string()), "one back from the first is the last");
    assert_eq!(name(&first, STRING_TARGET_NEXT_FIRST), Some("RATE A-".to_string()));
    assert_eq!(first.text(STRING_TARGET_NEXT_FIRST + 10), None, "the run of names ends after ten");
    assert_eq!(first.text(STRING_TARGET_PREVIOUS_FIRST - 1), None);
}
