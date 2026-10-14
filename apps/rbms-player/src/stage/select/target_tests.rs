//! What the browser says of the TARGET row on a chart, on the default skin pack: the target the
//! player chose, and -- where the chart holds no score for it -- the target that stands in.
//!
//! With `RBMS_SKIN_CAPTURE_DIR` set the frame is drawn at the size the pack was authored at and
//! saved there: `target-select-local-best.png` for a chart the player has a record of, and
//! `target-select-no-record.png` for one they have played only with SCRATCH AUTO on.

use rbms_config::ScoreTarget;

use super::keys::Panel;
use super::recorded_tests::{browse_to_the_recorded_chart, browsing_app, frame_size, play_a_run, save_frame, shown};
use super::*;
use crate::app_result::record_tests::{HAND_OFFSETS_US, played_by_hand};
use crate::stage::HeadlessCanvas;
use crate::stage::capture::scene_frame_at;

/// How long after the browser settled the play options panel is drawn at: past its rise.
const PANEL_UP_MS: i64 = 4_000;

/// Call the play options panel up over the browser and draw it once it has risen.
fn draw_the_play_options_panel(app: &mut App, pixels: &mut HeadlessCanvas) {
    let Stage::Select(state) = &mut app.stage else {
        panic!("the browser is not up");
    };
    state.show_panel(&mut app.shared, Panel::Play);
    scene_frame_at(app, PANEL_UP_MS, &mut Canvas::Headless(pixels));
    scene_frame_at(app, PANEL_UP_MS, &mut Canvas::Headless(pixels));
}

/// What the skin's TARGET text reads on the chart under the cursor.
fn target_text(app: &mut App) -> String {
    shown(app).settings.target_text().into_owned()
}

/// A chart the player has a record of is paced against it, so the browser names LOCAL BEST alone.
#[test]
fn the_browser_names_the_local_best_on_a_chart_that_has_one() {
    let mut app = browsing_app("target-record");
    app.shared.config.judge.target = ScoreTarget::LocalBest;
    play_a_run(&mut app, played_by_hand(&HAND_OFFSETS_US, false));
    let (width, height) = frame_size();
    let mut pixels = HeadlessCanvas::new(width, height);
    browse_to_the_recorded_chart(&mut app, &mut pixels);
    assert_eq!(target_text(&mut app), "LOCAL BEST");
    draw_the_play_options_panel(&mut app, &mut pixels);
    save_frame("target-select-local-best", &pixels);
}

/// A chart played only with SCRATCH AUTO on has no local best, so the browser says what a run of it
/// is paced against instead -- the fallback the player named -- and says it was LOCAL BEST that was
/// chosen. Choosing another fallback changes the text with the next frame.
#[test]
fn the_browser_names_the_fallback_on_a_chart_that_has_no_local_best() {
    let mut app = browsing_app("target-no-record");
    app.shared.config.judge.target = ScoreTarget::LocalBest;
    app.shared.config.play.scratch_auto = true;
    play_a_run(&mut app, played_by_hand(&HAND_OFFSETS_US, true));
    let (width, height) = frame_size();
    let mut pixels = HeadlessCanvas::new(width, height);
    browse_to_the_recorded_chart(&mut app, &mut pixels);
    assert_eq!(target_text(&mut app), "LOCAL BEST (NO RECORD: AAA)");
    draw_the_play_options_panel(&mut app, &mut pixels);
    save_frame("target-select-no-record", &pixels);

    app.shared.config.judge.target_fallback = ScoreTarget::RankNext;
    assert_eq!(target_text(&mut app), "LOCAL BEST (NO RECORD: RANK NEXT)");
    app.shared.config.judge.target = ScoreTarget::RateA;
    assert_eq!(target_text(&mut app), "RATE A", "a target that needs no score is named alone");
}
