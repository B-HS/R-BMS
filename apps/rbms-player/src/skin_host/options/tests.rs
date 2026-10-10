//! The options cluster answers a result screen with the choices the run was played with.

use rbms_chart::shuffle::NoteOption;
use rbms_judge::gauge::GaugeIndex;
use rbms_skin::property::IMAGE_INDEX_ABSENT;
use rbms_skin::property::generated::*;

use super::*;
use crate::skin_host::ResultScene;
use crate::skin_host::result::snapshot::{FinishedRun, ResultInput, ResultSnapshot};

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
