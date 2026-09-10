//! Headless render snapshots of the result screen.

use rbms_render::result::{ResultExtras, TargetView};

use crate::stage::Stage;
use crate::stage::render_tests::{app, render, result_state};

#[test]
fn the_result_screen_paints_its_summary() {
    let mut app = app();
    let pixels = render(&mut app, Stage::Result(result_state()));
    assert!(pixels.painted_pixels() > 0, "the result screen drew nothing");
    assert!(pixels.background().is_none(), "the result screen clears the background image slot");
}

/// The result screen is the one that runs after a chart, so it must not still be showing the
/// chart's own background.
#[test]
fn drawing_the_result_screen_twice_gives_the_same_frame() {
    let mut app = app();
    let first = render(&mut app, Stage::Result(result_state())).pixel_checksum();
    let second = render(&mut app, Stage::Result(result_state())).pixel_checksum();
    assert_eq!(first, second, "the result screen is not deterministic");
}

/// The gauge trend, the timing spread and the judge split are drawn from what the run measured, so
/// filling those in has to change the frame.
#[test]
fn the_measurements_the_graphs_are_drawn_from_reach_the_screen() {
    let mut app = app();
    let bare = render(&mut app, Stage::Result(result_state())).pixel_checksum();
    let mut state = result_state();
    state.set_measurements(vec![10.0, 40.0, 80.0], vec![1, 4, 9, 4, 1].into_boxed_slice(), [3, 2, 1, 0, 0, 0]);
    let measured = render(&mut app, Stage::Result(state)).pixel_checksum();
    assert_ne!(bare, measured, "the graphs ignore the measurements behind them");
}

/// Each measurement feeds its own panel, so moving one of them alone still moves the screen — which
/// is what a panel wired to the wrong field would not do.
#[test]
fn each_graph_answers_to_its_own_measurement() {
    let mut app = app();
    let paint = |app: &mut crate::App, gauge: Vec<f32>, timing: Vec<u32>, judge: [u32; 6]| {
        let mut state = result_state();
        state.set_measurements(gauge, timing.into_boxed_slice(), judge);
        render(app, Stage::Result(state)).pixel_checksum()
    };
    let base = paint(&mut app, vec![10.0, 40.0, 80.0], vec![1, 4, 9, 4, 1], [3, 2, 1, 0, 0, 0]);
    assert_ne!(base, paint(&mut app, vec![90.0, 20.0, 5.0], vec![1, 4, 9, 4, 1], [3, 2, 1, 0, 0, 0]), "the gauge panel");
    assert_ne!(base, paint(&mut app, vec![10.0, 40.0, 80.0], vec![9, 1, 1, 1, 1], [3, 2, 1, 0, 0, 0]), "the timing panel");
    assert_ne!(base, paint(&mut app, vec![10.0, 40.0, 80.0], vec![1, 4, 9, 4, 1], [1, 1, 1, 1, 1, 1]), "the judge panel");
}

/// A run nothing was measured for still has to draw a whole screen: the panels are there with their
/// note in them, and nothing panics on the empty series.
#[test]
fn a_run_with_no_measurements_still_paints_a_whole_screen() {
    let mut app = app();
    let mut state = result_state();
    state.set_measurements(Vec::new(), Vec::new().into_boxed_slice(), [0; 6]);
    let pixels = render(&mut app, Stage::Result(state));
    assert!(pixels.painted_pixels() > 0, "an unmeasured run drew nothing");
}

/// The target the run was paced against and the offer to run it again are what the player's own
/// result screen carries over the bare one, so they have to reach the frame.
#[test]
fn what_surrounds_the_run_reaches_the_frame() {
    let mut app = app();
    let bare = render(&mut app, Stage::Result(result_state())).pixel_checksum();
    let extras = ResultExtras { target: Some(TargetView { name: "RANK AAA".into(), ex: 11 }), run_again: true };
    let paced = render(&mut app, Stage::Result(result_state().paced_by(extras))).pixel_checksum();
    assert_ne!(bare, paced, "neither the TARGET line nor the run-again hint is drawn");
}
