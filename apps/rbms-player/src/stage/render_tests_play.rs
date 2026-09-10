//! Headless render snapshots of the play screen.

use crate::stage::render_tests::{app, play_state, render};
use crate::stage::{PlayState, Stage};
use rbms_config::FixHiSpeed;
use rbms_play::{PlaySession, SessionOptions};
use rbms_store::SCORE_LN_MODE_FROM_CHART;

/// A chart whose lane 0 holds one long note in the opening measure, so its head and body are both
/// on screen at the start of the run. The seventh-key note is there so the chart is detected as the
/// seven-key layout the browser's skin is resolved for.
const LONG_NOTE_CHART: &str = "#PLAYER 1\n#TITLE legacy\n#BPM 120\n#LNTYPE 1\n#WAV01 a.wav\n#00018:01\n#00051:0001000000000001\n";

/// A run on [`LONG_NOTE_CHART`] at its opening frame, where the long note is still ahead.
fn long_note_state() -> PlayState {
    let src = rbms_parser::parse_with(LONG_NOTE_CHART.as_bytes(), Default::default());
    let mode = rbms_chart::detect_mode(&src, "legacy.bms");
    let model = rbms_chart::to_model(&src, mode);
    PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

#[test]
fn the_play_screen_paints_its_field() {
    let mut app = app();
    let pixels = render(&mut app, Stage::Play(Box::new(play_state())));
    assert!(pixels.painted_pixels() > 0, "the play screen drew nothing");
    assert!(pixels.quad_count() > 0, "the play screen drew no quads");
    assert!(app.shared.hot.is_empty(), "a running chart has nothing to click");
}

/// The lane cover is drawn over the field, so raising it has to change what is painted — a cover
/// wired to nothing would leave the frame alone.
#[test]
fn the_lane_cover_reaches_the_painted_frame() {
    let mut app = app();
    let uncovered = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.play.cover = 0.5;
    let covered = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(uncovered, covered, "the lane cover is not drawn");
}

/// The cover has a switch of its own, so a height the player has set but switched off must not
/// reach the screen.
#[test]
fn a_lane_cover_that_is_switched_off_does_not_reach_the_painted_frame() {
    let mut app = app();
    app.shared.config.play.cover = 0.5;
    app.shared.config.play.enable_cover = false;
    let off = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.play.enable_cover = true;
    let on = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(off, on, "the cover switch does not reach the screen");
}

/// HID+ is the other end of the field from the cover, so it has to move the frame on its own and
/// differently from a cover of the same height.
#[test]
fn the_hidden_band_reaches_the_painted_frame_and_is_not_the_cover() {
    let mut app = app();
    let bare = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.play.enable_hidden = true;
    app.shared.config.play.hidden = 0.3;
    let hidden = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(bare, hidden, "the hidden band is not drawn");

    app.shared.config.play.enable_hidden = false;
    app.shared.config.play.enable_cover = true;
    app.shared.config.play.cover = 0.3;
    let covered = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(hidden, covered, "the two shades are drawn at the same end of the field");
}

/// The pacemaker line names the target the TARGET row settled on, so switching that row has to
/// change what the HUD says while the run is going.
#[test]
fn the_pacemaker_names_the_target_the_settings_ask_for() {
    let mut app = app();
    app.shared.config.judge.target = rbms_config::ScoreTarget::Max;
    let against_max = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.judge.target = rbms_config::ScoreTarget::RateA;
    let against_rate = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(against_max, against_rate, "the pacemaker line does not reach the screen");
}

/// The green number on the HUD follows the scroll speed, so changing it has to show.
#[test]
fn the_scroll_speed_reaches_the_painted_frame() {
    let mut app = app();
    let slow = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.play.hispeed *= 2.0;
    let fast = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(slow, fast, "the scroll speed does not reach the screen");
}

/// The white number is drawn next to the green one, so switching the row on with a cover up has to
/// change the frame, and switching it on with no cover must not.
#[test]
fn the_white_number_is_drawn_only_when_a_cover_is_hiding_something() {
    let mut app = app();
    app.shared.config.play.enable_cover = true;
    app.shared.config.play.cover = 0.4;
    let hidden_row = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.display.show_white_number = true;
    let shown = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(hidden_row, shown, "the white number does not reach the screen");

    app.shared.config.play.cover = 0.0;
    let no_cover_off = {
        app.shared.config.display.show_white_number = false;
        render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum()
    };
    app.shared.config.display.show_white_number = true;
    let no_cover_on = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_eq!(no_cover_off, no_cover_on, "a run with no cover has no white number to draw");
}

/// Moving the judgment label is a settings row, so it has to reach the frame the screen paints.
#[test]
fn the_judge_text_position_reaches_the_painted_frame() {
    let mut app = app();
    let mut state = play_state();
    state.session.tick(rbms_play::SessionClock::at(3_000_000), &mut rbms_play::NullSink);
    assert!(state.session.judge().last_judge.is_some(), "the fixture run has a judgement to draw");
    let from_skin = render(&mut app, Stage::Play(Box::new(state))).pixel_checksum();

    let mut state = play_state();
    state.session.tick(rbms_play::SessionClock::at(3_000_000), &mut rbms_play::NullSink);
    app.shared.config.display.judge_text_y = 0.7;
    let moved = render(&mut app, Stage::Play(Box::new(state))).pixel_checksum();
    assert_ne!(from_skin, moved, "the judge text position does not reach the screen");
}

/// LEGACY NOTE draws a long note as the plain note at its head, so a chart with one has to paint
/// fewer pixels with the row on.
#[test]
fn legacy_notes_reach_the_painted_frame() {
    let mut app = app();
    let full = render(&mut app, Stage::Play(Box::new(long_note_state())));
    let full_quads = full.quad_count();
    app.shared.config.play.legacy_note = true;
    let legacy = render(&mut app, Stage::Play(Box::new(long_note_state())));
    assert_ne!(full.pixel_checksum(), legacy.pixel_checksum(), "the legacy note row does not reach the screen");
    assert!(legacy.quad_count() < full_quads, "the body is what stops being drawn: {} quads against {full_quads}", legacy.quad_count());
}

/// Pinning the green number to a tempo changes the number the HUD shows, because it stops following
/// the timeline under the play head.
#[test]
fn pinning_the_green_number_changes_what_the_hud_reads() {
    let mut app = app();
    app.shared.config.play.fix_hispeed = FixHiSpeed::Off;
    let floating = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    app.shared.config.play.fix_hispeed = FixHiSpeed::MaxBpm;
    app.shared.config.play.hispeed = 8.0;
    let pinned = render(&mut app, Stage::Play(Box::new(play_state()))).pixel_checksum();
    assert_ne!(floating, pinned, "the pinned tempo does not reach the screen");
}
