//! Golden signature tests for the result screen, in both palettes it is drawn with.

mod golden_harness;

use golden_harness::{check, golden_canvas, signature_of};
use rbms_render::result::{ResultExtras, TargetView};
use rbms_render::{Color, ResultPalette, ResultView, SkinConfig, render_result, render_result_with_palette};

const GOLDEN_RESULT: u64 = 0x6a90_2207_198b_e349;
const GOLDEN_RESULT_SKIN: u64 = 0xdbfb_eda9_56a6_9aa8;
const GOLDEN_RESULT_PACED: u64 = 0x5782_d81c_02bb_d1a9;

fn result_view() -> ResultView {
    ResultView {
        title: "GOLDEN RESULT".into(),
        mode_label: "7K",
        counts: [712, 64, 21, 8, 5, 2],
        ex_score: 1488,
        max_score: 1624,
        max_combo: 612,
        total_notes: 812,
        fast: [30, 0],
        slow: [40, 0],
        gauge: 78.0,
        clear_label: "HARD CLEAR",
        clear_color: Color::WHITE,
        prev_best_ex: Some(1440),
        prev_ex: Some(1502),
        show_graph: true,
        show_result_graphs: true,
        gauge_series: vec![100.0, 96.0, 88.0, 91.0, 84.0, 79.0, 83.0, 78.0],
        timing_hist: vec![1, 2, 6, 18, 44, 91, 44, 17, 5, 2, 1].into_boxed_slice(),
        judge_dist: [712, 64, 21, 8, 5, 2],
    }
}

/// What the player's own result screen carries besides the run: the target it was paced against and
/// the offer to run it again.
fn result_extras() -> ResultExtras {
    ResultExtras { target: Some(TargetView { name: "RANK AAA".into(), ex: 1444 }), run_again: true }
}

#[test]
fn result_screen_matches_its_golden_signature() {
    let mut canvas = golden_canvas();
    render_result(&mut canvas, &result_view());
    check(&canvas, "result", "GOLDEN_RESULT", GOLDEN_RESULT);
}

/// The player draws the result screen through `render_result_with_palette` with a palette built from
/// the active skin, whose judge colours and labels differ from the built-in ones. This golden pins
/// the pixels a user actually sees; `result_screen_matches_its_golden_signature` pins the library
/// default.
#[test]
fn result_screen_with_the_default_skin_palette_matches_its_golden_signature() {
    let palette = ResultPalette::from_skin(&SkinConfig::default());
    let mut canvas = golden_canvas();
    render_result_with_palette(&mut canvas, &result_view(), &palette, &ResultExtras::default());
    check(&canvas, "result (skin palette)", "GOLDEN_RESULT_SKIN", GOLDEN_RESULT_SKIN);
}

/// The screen a finished interactive run actually lands on: the same report, plus the TARGET line
/// and the run-again keys in the hint.
#[test]
fn result_screen_with_a_target_matches_its_golden_signature() {
    let palette = ResultPalette::from_skin(&SkinConfig::default());
    let mut canvas = golden_canvas();
    render_result_with_palette(&mut canvas, &result_view(), &palette, &result_extras());
    check(&canvas, "result (paced)", "GOLDEN_RESULT_PACED", GOLDEN_RESULT_PACED);
}

/// The three result goldens must not collide: the built-in palette paints PGREAT hot pink and labels
/// it "PGREAT", the default skin paints it green and labels it "PERFECT", and the paced screen adds
/// a line the other two do not have.
#[test]
fn the_three_result_goldens_differ() {
    assert_ne!(GOLDEN_RESULT, GOLDEN_RESULT_SKIN, "the skin palette repaints and relabels the judge rows");
    assert_ne!(GOLDEN_RESULT_SKIN, GOLDEN_RESULT_PACED, "the TARGET line and the run-again hint are not drawn");
}

/// The harness only earns its keep if it fails on the regressions it claims to catch, so these pin
/// its sensitivity: a one-digit score change and a swapped clear label must each move the signature.
#[test]
fn the_signature_detects_a_single_changed_score_digit() {
    let base = signature_of(|c| render_result(c, &result_view()));
    let bumped = signature_of(|c| render_result(c, &ResultView { ex_score: 1489, ..result_view() }));
    assert_ne!(base, bumped, "an EX score of 1489 instead of 1488 changes the signature");
}

#[test]
fn the_signature_detects_a_swapped_clear_label() {
    let base = signature_of(|c| render_result(c, &result_view()));
    let failed = signature_of(|c| render_result(c, &ResultView { clear_label: "FAILED", ..result_view() }));
    assert_ne!(base, failed, "relabelling HARD CLEAR to FAILED changes the signature");
}

/// The early/late totals are stored per lane kind but shown as one number, so a run whose inputs are
/// split between the key lanes and the scratch draws exactly as one that has them all on the keys.
#[test]
fn the_early_and_late_totals_read_as_one_number_however_they_are_split() {
    let together = signature_of(|c| render_result(c, &result_view()));
    let split = signature_of(|c| render_result(c, &ResultView { fast: [20, 10], slow: [25, 15], ..result_view() }));
    assert_eq!(together, split, "the screen shows the total, so where the inputs came from cannot move a pixel");
}

/// The graphs are what the measurements are for, so a run measured differently draws differently —
/// and a run with the graphs switched off draws neither them nor anything behind them.
#[test]
fn the_measurements_the_graphs_are_drawn_from_move_the_signature() {
    let base = signature_of(|c| render_result(c, &result_view()));
    let flat = signature_of(|c| render_result(c, &ResultView { gauge_series: vec![50.0; 8], ..result_view() }));
    assert_ne!(base, flat, "the gauge trend is not drawn");
    let off = signature_of(|c| render_result(c, &ResultView { show_result_graphs: false, ..result_view() }));
    assert_ne!(base, off, "RESULT GRAPHS off still draws the panels");
    let off_unmeasured = signature_of(|c| {
        let view = ResultView { show_result_graphs: false, gauge_series: Vec::new(), timing_hist: Box::new([]), judge_dist: [0; 6], ..result_view() };
        render_result(c, &view);
    });
    assert_eq!(off, off_unmeasured, "with the graphs off the measurements behind them cannot move a pixel");
}
