//! Unit tests for the loading cluster: the progress reads the same in each id space, and the two
//! loading options belong to the player screen alone.

use rbms_skin::property::generated::*;

use super::{LoadingScreen, LoadingState};
use crate::skin_host::ClusterState;

fn on(screen: LoadingScreen, progress: f32) -> LoadingState {
    LoadingState { screen, progress }
}

#[test]
fn the_progress_is_the_same_share_as_a_float_a_rate_and_a_whole_percent() {
    let state = on(LoadingScreen::Preload, 0.456);

    assert_eq!(state.float(FLOAT_LOADING_PROGRESS), Some(0.456));
    assert_eq!(state.rate(RATE_LOAD_PROGRESS), Some(0.456));
    assert_eq!(state.integer(NUMBER_LOADING_PROGRESS), Some(45), "the percent is cut, not rounded");
}

#[test]
fn the_progress_runs_from_nothing_to_everything() {
    assert_eq!(on(LoadingScreen::Preload, 0.0).integer(NUMBER_LOADING_PROGRESS), Some(0));
    assert_eq!(on(LoadingScreen::Started, 1.0).integer(NUMBER_LOADING_PROGRESS), Some(100));
    assert_eq!(on(LoadingScreen::Started, 1.0).rate(RATE_LOAD_PROGRESS), Some(1.0));
}

#[test]
fn the_progress_is_read_on_any_screen_that_connects_it() {
    for screen in [LoadingScreen::Elsewhere, LoadingScreen::Preload, LoadingScreen::Started] {
        assert_eq!(on(screen, 0.5).integer(NUMBER_LOADING_PROGRESS), Some(50), "{screen:?}");
    }
}

#[test]
fn the_player_screen_is_loading_until_it_has_started_and_exactly_one_option_says_so() {
    let preload = on(LoadingScreen::Preload, 0.3);
    assert_eq!((preload.boolean(OPTION_NOW_LOADING), preload.boolean(OPTION_LOADED)), (Some(true), Some(false)));

    let started = on(LoadingScreen::Started, 1.0);
    assert_eq!((started.boolean(OPTION_NOW_LOADING), started.boolean(OPTION_LOADED)), (Some(false), Some(true)));
}

#[test]
fn on_any_other_screen_both_loading_options_are_off() {
    let state = on(LoadingScreen::Elsewhere, 1.0);

    assert_eq!((state.boolean(OPTION_NOW_LOADING), state.boolean(OPTION_LOADED)), (Some(false), Some(false)));
}

#[test]
fn a_cluster_with_nothing_connected_knows_nothing() {
    let state = LoadingState::default();

    assert_eq!(state.screen, LoadingScreen::Unconnected);
    assert_eq!(state.boolean(OPTION_NOW_LOADING), None);
    assert_eq!(state.boolean(OPTION_LOADED), None);
    assert_eq!(state.integer(NUMBER_LOADING_PROGRESS), None);
    assert_eq!(state.rate(RATE_LOAD_PROGRESS), None);
    assert_eq!(state.float(FLOAT_LOADING_PROGRESS), None);
}

#[test]
fn ids_of_other_clusters_are_not_answered() {
    let state = on(LoadingScreen::Preload, 0.5);

    assert_eq!(state.integer(NUMBER_PLAYLEVEL), None);
    assert_eq!(state.boolean(OPTION_DIFFICULTY0), None);
    assert_eq!(state.rate(RATE_MUSICSELECT_POSITION), None);
    assert_eq!(state.float(FLOAT_CHART_TOTALGAUGE), None);
    assert_eq!(state.text(STRING_TITLE), None);
}
