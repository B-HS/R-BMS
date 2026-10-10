//! The play scene as a bare state machine, with a clock the test counts: each state, the time that
//! ends it, and what the timers read when it does.

use rbms_model::Mode;
use rbms_skin::timer::{MICROS_PER_MILLI, TimerState, timer_id};

use super::*;
use crate::skin_host::play_timers::{CHART_PREVIEW, PlaySetup, PlayTimerDriver};

/// The times the scene tests run on.
const TIMES: PlayTimes = PlayTimes { input_ms: 500, loadend_ms: 2000, playstart_ms: 1000, close_ms: 1500, finishmargin_ms: 800, fadeout_ms: 600 };

/// When the first note of the fixture chart is reached, in microseconds.
const FIRST_NOTE_US: i64 = 2_000_000;

/// One microsecond short of the next whole millisecond.
const ALMOST_A_MILLI_US: i64 = MICROS_PER_MILLI - 1;

/// A frame's length, for the tests that walk a scene a frame at a time.
const FRAME_US: i64 = 16_000;

fn at_ms(millis: i64) -> i64 {
    millis * MICROS_PER_MILLI
}

/// A driver for a seven-key chart whose first note is at [`FIRST_NOTE_US`], and timers with nothing
/// on.
fn stage() -> (PlayTimerDriver, TimerState) {
    let setup = PlaySetup { first_note_us: FIRST_NOTE_US, ..PlaySetup::new(Mode::BEAT_7K) };
    (PlayTimerDriver::new(setup), TimerState::new())
}

/// The facts of a frame at `now_us` on which nothing is happening: the files are in, nothing is
/// held, and the chart is as far along as `chart_us`.
fn facts(now_us: i64, chart_us: i64) -> SceneFacts {
    SceneFacts {
        now_us,
        times: TIMES,
        loaded: true,
        start: false,
        select: false,
        start_offset_us: 0,
        playing: PlayingFrame { chart_us, bpm: 120.0, play_speed: 100, gauge_max: false, past_notes: 0 },
        failed: false,
        end: RunEnd::default(),
        notes_done: false,
        may_retry: true,
    }
}

/// A scene walked to the frame its chart starts playing on, and the scene time of that frame.
fn playing() -> (PlayScene, PlayTimerDriver, TimerState, i64) {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    let ready_at = at_ms(TIMES.loadend_ms) + 1;
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(ready_at, 0)), SceneStep::Ready);
    let started_at = ready_at + at_ms(TIMES.playstart_ms + 1);
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(started_at, 0)), SceneStep::Started);
    (scene, driver, timers, started_at)
}

/// A screen with no skin has no times at all, and a skin's own are read off it.
#[test]
fn a_screen_with_no_skin_has_no_times() {
    assert_eq!(PlayTimes::of_skin(None), PlayTimes::default());
}

/// Loading ends only when all three of its conditions hold: the files, the skin's `loadend`, and
/// the second START and SELECT have to have been up for.
#[test]
fn loading_waits_for_the_files_and_for_the_skins_load_time() {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    let after_loadend = at_ms(TIMES.loadend_ms) + 1;

    let still_decoding = SceneFacts { loaded: false, ..facts(after_loadend, 0) };
    assert_eq!(scene.step(&mut driver, &mut timers, &still_decoding), SceneStep::Stay, "a chart whose files are not in became ready");
    assert_eq!(scene.phase(), PlayPhase::Preload);

    let mut scene = PlayScene::loading();
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(at_ms(TIMES.loadend_ms), 0)), SceneStep::Stay, "the load time itself is not past it");
    assert!(!timers.is_on(timer_id::READY));

    assert_eq!(scene.step(&mut driver, &mut timers, &facts(after_loadend, 0)), SceneStep::Ready);
    assert_eq!(scene.phase(), PlayPhase::Ready);
    assert_eq!(timers.value_us(timer_id::READY), after_loadend, "the ready timer was not switched on as the chart became ready");
}

/// The second START and SELECT have to have been up for is counted from the scene's own beginning
/// when neither was touched, so no chart is ready sooner than that -- whatever its skin says.
#[test]
fn no_chart_is_ready_within_the_first_second_of_its_scene() {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    let no_times = |now_us| SceneFacts { times: PlayTimes::default(), ..facts(now_us, 0) };

    assert_eq!(scene.step(&mut driver, &mut timers, &no_times(START_RELEASE_WAIT_US)), SceneStep::Stay, "a second is not longer than a second");
    assert_eq!(scene.step(&mut driver, &mut timers, &no_times(START_RELEASE_WAIT_US + 1)), SceneStep::Ready);
}

/// Holding START or SELECT while the chart loads previews the chart: timer 141 goes on a second
/// ahead of the first note, goes off when the key comes up, and the chart is not ready until the
/// key has been up for a second.
#[test]
fn holding_start_while_loading_previews_the_chart_and_holds_the_run_back() {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    let pressed_at = at_ms(TIMES.loadend_ms + 500);
    let held = |now_us| SceneFacts { start: true, ..facts(now_us, 0) };

    assert_eq!(scene.step(&mut driver, &mut timers, &held(pressed_at)), SceneStep::Stay, "a chart became ready with START held");
    assert_eq!(timers.value_us(CHART_PREVIEW), pressed_at - FIRST_NOTE_US + 1_000_000, "the preview does not start a second ahead of the first note");

    let released_at = pressed_at + at_ms(3000);
    assert_eq!(scene.step(&mut driver, &mut timers, &held(released_at - FRAME_US)), SceneStep::Stay);
    assert_eq!(timers.value_us(CHART_PREVIEW), pressed_at - FIRST_NOTE_US + 1_000_000, "a preview that is running was started again");

    assert_eq!(scene.step(&mut driver, &mut timers, &facts(released_at, 0)), SceneStep::Stay);
    assert!(!timers.is_on(CHART_PREVIEW), "the preview outlived the key that held it");

    let a_second_on = released_at - FRAME_US + START_RELEASE_WAIT_US;
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(a_second_on, 0)), SceneStep::Stay, "the key has not been up for longer than a second");
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(a_second_on + 1, 0)), SceneStep::Ready);
}

/// SELECT previews the chart as START does, and a chart that becomes ready ends its preview.
#[test]
fn select_previews_too_and_a_chart_that_becomes_ready_ends_its_preview() {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    let pressed_at = at_ms(100);
    scene.step(&mut driver, &mut timers, &SceneFacts { select: true, ..facts(pressed_at, 0) });
    assert!(timers.is_on(CHART_PREVIEW), "SELECT did not start the preview");

    timers.set_on(CHART_PREVIEW, pressed_at);
    let mut ready = PlayScene::loading();
    assert_eq!(ready.step(&mut driver, &mut timers, &facts(at_ms(TIMES.loadend_ms) + START_RELEASE_WAIT_US, 0)), SceneStep::Ready);
    assert!(!timers.is_on(CHART_PREVIEW), "the preview's timer was left on under a chart that is ready");
}

/// `STARTINPUT` goes on once the skin's `input` time has passed, whatever state the scene is in.
#[test]
fn the_input_timer_goes_on_once_the_input_time_has_passed() {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    scene.step(&mut driver, &mut timers, &facts(at_ms(TIMES.input_ms), 0));
    assert!(!timers.is_on(timer_id::STARTINPUT), "the input time itself is not past it");
    scene.step(&mut driver, &mut timers, &facts(at_ms(TIMES.input_ms) + 1, 0));
    assert_eq!(timers.value_us(timer_id::STARTINPUT), at_ms(TIMES.input_ms) + 1);
}

/// A chart stands ready for longer than the skin's `playstart`, in whole milliseconds, and the play
/// timer goes on with the chart: that frame is where the song clock starts.
#[test]
fn a_ready_chart_starts_playing_once_its_ready_timer_has_run_past_playstart() {
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::loading();
    let ready_at = at_ms(TIMES.loadend_ms) + 1;
    scene.step(&mut driver, &mut timers, &facts(ready_at, 0));

    let not_yet = ready_at + at_ms(TIMES.playstart_ms) + ALMOST_A_MILLI_US;
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(not_yet, 0)), SceneStep::Stay, "the start time itself is not past it");
    assert!(!timers.is_on(timer_id::PLAY), "the play timer is on before the chart plays");
    assert!(!scene.is_playing());

    let started_at = ready_at + at_ms(TIMES.playstart_ms + 1);
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(started_at, 0)), SceneStep::Started);
    assert!(scene.is_playing());
    assert_eq!(timers.value_us(timer_id::PLAY), started_at, "the play timer does not start with the chart");
    assert_eq!(timers.value_us(timer_id::RHYTHM), started_at);
    assert_eq!(timers.value_us(timer_id::READY), ready_at, "the ready timer was switched off or on again");
}

/// While the chart plays, the play timer is kept where the chart's own time puts it, so what a skin
/// reads off it is the place in the chart.
#[test]
fn the_play_timer_reads_the_place_in_the_chart() {
    let (mut scene, mut driver, mut timers, started_at) = playing();
    let now_us = started_at + at_ms(700);
    let chart_us = at_ms(650);
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(now_us, chart_us)), SceneStep::Stay);
    assert_eq!(elapsed_ms(&timers, timer_id::PLAY, now_us), 650);
}

/// A practice slice has nothing to load: its scene begins ready, and its play timer starts as far
/// into the chart as the slice does.
#[test]
fn a_practice_slice_begins_ready_and_its_play_timer_starts_at_the_slice() {
    const SLICE_START_US: i64 = 30_000_000;
    let (mut driver, mut timers) = stage();
    let mut scene = PlayScene::ready();
    let slice = |now_us| SceneFacts { start_offset_us: SLICE_START_US, ..facts(now_us, SLICE_START_US) };

    assert_eq!(scene.step(&mut driver, &mut timers, &slice(FRAME_US)), SceneStep::Ready, "the slice was not ready on its first frame");
    assert_eq!(timers.value_us(timer_id::READY), FRAME_US);
    assert_eq!(scene.step(&mut driver, &mut timers, &slice(FRAME_US + at_ms(TIMES.playstart_ms))), SceneStep::Stay);

    let started_at = FRAME_US + at_ms(TIMES.playstart_ms + 1);
    assert_eq!(scene.step(&mut driver, &mut timers, &slice(started_at)), SceneStep::Started);
    assert_eq!(elapsed_ms(&timers, timer_id::PLAY, started_at), SLICE_START_US / MICROS_PER_MILLI, "the play timer does not read the slice's own start");
    assert_eq!(elapsed_ms(&timers, timer_id::RHYTHM, started_at), SLICE_START_US / MICROS_PER_MILLI);
}

/// A run whose gauge empties fails: the failure's timer goes on, and the run is left for the result
/// once that timer has run for longer than the skin's `close`.
#[test]
fn a_failed_run_closes_for_as_long_as_the_skin_says_and_then_leaves_for_the_result() {
    let (mut scene, mut driver, mut timers, started_at) = playing();
    let failed_at = started_at + at_ms(3000);
    let no_retry = |now_us| SceneFacts { may_retry: false, ..facts(now_us, at_ms(3000)) };

    assert_eq!(scene.step(&mut driver, &mut timers, &SceneFacts { failed: true, ..no_retry(failed_at) }), SceneStep::Failed);
    assert_eq!(scene.phase(), PlayPhase::Failed);
    assert!(scene.is_closing());
    assert_eq!(timers.value_us(timer_id::FAILED), failed_at);
    assert!(!timers.is_on(timer_id::FADEOUT), "a failure does not fade out: the skin closes it on its own timer");

    assert_eq!(
        scene.step(&mut driver, &mut timers, &no_retry(failed_at + at_ms(TIMES.close_ms) + ALMOST_A_MILLI_US)),
        SceneStep::Stay,
        "the close time is not past it"
    );
    assert_eq!(scene.step(&mut driver, &mut timers, &no_retry(failed_at + at_ms(TIMES.close_ms + 1))), SceneStep::Leave(Leave::Result));
    assert_eq!(timers.value_us(timer_id::FAILED), failed_at, "the failure's timer was started again while it closed");
}

/// A failed run played by hand starts again at once on START or on SELECT, but not on both, and
/// not at all where a retry is not allowed.
#[test]
fn start_or_select_starts_a_failed_run_again_without_waiting_for_it_to_close() {
    let failed = || {
        let (mut scene, mut driver, mut timers, started_at) = playing();
        let failed_at = started_at + at_ms(3000);
        scene.step(&mut driver, &mut timers, &SceneFacts { failed: true, ..facts(failed_at, at_ms(3000)) });
        (scene, driver, timers, failed_at + FRAME_US)
    };

    let (mut scene, mut driver, mut timers, now_us) = failed();
    let with_start = SceneFacts { start: true, ..facts(now_us, 0) };
    assert_eq!(scene.step(&mut driver, &mut timers, &with_start), SceneStep::Leave(Leave::Retry { same_layout: false }), "START lays the chart out afresh");

    let (mut scene, mut driver, mut timers, now_us) = failed();
    let with_select = SceneFacts { select: true, ..facts(now_us, 0) };
    assert_eq!(scene.step(&mut driver, &mut timers, &with_select), SceneStep::Leave(Leave::Retry { same_layout: true }), "SELECT keeps the layout");

    let (mut scene, mut driver, mut timers, now_us) = failed();
    let with_both = SceneFacts { start: true, select: true, ..facts(now_us, 0) };
    assert_eq!(scene.step(&mut driver, &mut timers, &with_both), SceneStep::Stay, "START with SELECT is not a retry");

    let (mut scene, mut driver, mut timers, now_us) = failed();
    let not_allowed = SceneFacts { start: true, may_retry: false, ..facts(now_us, 0) };
    assert_eq!(scene.step(&mut driver, &mut timers, &not_allowed), SceneStep::Stay, "a run that may not be retried was");
}

/// A chart that runs out is finished: the music-end timer goes on, the fade begins once it has run
/// for longer than the skin's `finishmargin`, and the run leaves once the fade has run for longer
/// than `fadeout`.
#[test]
fn a_finished_run_waits_its_margin_fades_and_then_leaves_for_the_result() {
    let (mut scene, mut driver, mut timers, started_at) = playing();
    let ended_at = started_at + at_ms(8001);
    let over = SceneFacts { end: RunEnd { ended: true, last_note_passed: true }, ..facts(ended_at, at_ms(8001)) };

    assert_eq!(scene.step(&mut driver, &mut timers, &over), SceneStep::Finished { failed_too: false });
    assert_eq!(scene.phase(), PlayPhase::Finished);
    assert_eq!(timers.value_us(timer_id::MUSIC_END), ended_at);
    assert!(!timers.is_on(timer_id::FADEOUT), "the fade began with no margin");

    let margin_over = ended_at + at_ms(TIMES.finishmargin_ms + 1);
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(ended_at + at_ms(TIMES.finishmargin_ms) + ALMOST_A_MILLI_US, 0)), SceneStep::Stay);
    assert!(!timers.is_on(timer_id::FADEOUT), "the margin itself is not past it");
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(margin_over, 0)), SceneStep::Stay);
    assert_eq!(timers.value_us(timer_id::FADEOUT), margin_over);

    assert_eq!(
        scene.step(&mut driver, &mut timers, &facts(margin_over + at_ms(TIMES.fadeout_ms) + ALMOST_A_MILLI_US, 0)),
        SceneStep::Stay,
        "the fade time is not past it"
    );
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(margin_over + at_ms(TIMES.fadeout_ms + 1), 0)), SceneStep::Leave(Leave::Result));
    assert_eq!(timers.value_us(timer_id::FADEOUT), margin_over, "the fade was started again while it ran");
}

/// Once the last note has gone by its timer goes on, and stays on from the first frame it did.
#[test]
fn the_end_of_notes_timer_goes_on_once_the_last_note_has_gone_by() {
    let (mut scene, mut driver, mut timers, started_at) = playing();
    let passed = |now_us| SceneFacts { end: RunEnd { ended: false, last_note_passed: true }, ..facts(now_us, 0) };
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(started_at + FRAME_US, 0)), SceneStep::Stay);
    assert!(!timers.is_on(timer_id::ENDOFNOTE_1P));

    let passed_at = started_at + at_ms(3100);
    assert_eq!(scene.step(&mut driver, &mut timers, &passed(passed_at)), SceneStep::Stay);
    scene.step(&mut driver, &mut timers, &passed(passed_at + FRAME_US));
    assert_eq!(timers.value_us(timer_id::ENDOFNOTE_1P), passed_at);
    assert!(scene.is_playing(), "the last note going by is not the end of the run");
}

/// A run that fails on the frame its chart runs out is finished, not failed, with both timers on:
/// the reference assigns the finished state after the failed one.
#[test]
fn a_run_that_fails_and_runs_out_on_one_frame_is_finished_with_both_timers_on() {
    let (mut scene, mut driver, mut timers, started_at) = playing();
    let now_us = started_at + at_ms(8001);
    let both = SceneFacts { failed: true, end: RunEnd { ended: true, last_note_passed: true }, ..facts(now_us, at_ms(8001)) };
    assert_eq!(scene.step(&mut driver, &mut timers, &both), SceneStep::Finished { failed_too: true });
    assert_eq!(scene.phase(), PlayPhase::Finished);
    assert!(timers.is_on(timer_id::FAILED) && timers.is_on(timer_id::MUSIC_END));
}

/// A run with nothing left to hit ends when START or SELECT is pressed, and fades at once rather
/// than after the margin a chart that ran out waits for.
#[test]
fn a_run_with_nothing_left_to_hit_ends_on_start_or_select_and_fades_at_once() {
    let (mut scene, mut driver, mut timers, started_at) = playing();
    let now_us = started_at + at_ms(3500);
    let still_notes = SceneFacts { start: true, ..facts(now_us, at_ms(3500)) };
    assert_eq!(scene.step(&mut driver, &mut timers, &still_notes), SceneStep::Stay, "START ended a run that still has notes");

    let done = SceneFacts { select: true, notes_done: true, ..facts(now_us + FRAME_US, at_ms(3500)) };
    assert_eq!(scene.step(&mut driver, &mut timers, &done), SceneStep::Finished { failed_too: false });
    assert_eq!(timers.value_us(timer_id::FADEOUT), now_us + FRAME_US);
    assert!(!timers.is_on(timer_id::MUSIC_END), "a run the player ended has no music-end margin to wait");

    let leaves_at = now_us + FRAME_US + at_ms(TIMES.fadeout_ms + 1);
    assert_eq!(scene.step(&mut driver, &mut timers, &facts(leaves_at, 0)), SceneStep::Leave(Leave::Result));
}

/// Ending a run by hand is for a chart that is playing or already finished: one that is loading or
/// ready is left another way, and one that failed is not turned into a finished one.
#[test]
fn only_a_chart_that_is_playing_or_finished_can_be_ended_by_hand() {
    let (mut driver, mut timers) = stage();
    let mut loading = PlayScene::loading();
    assert!(!loading.stop_finished(&mut driver, &mut timers, FRAME_US));
    assert_eq!(loading.phase(), PlayPhase::Preload);
    assert!(!timers.is_on(timer_id::FADEOUT));

    let (mut scene, mut driver, mut timers, started_at) = playing();
    assert!(scene.stop_finished(&mut driver, &mut timers, started_at + FRAME_US), "ending a chart that is playing is what ended it");
    assert_eq!(scene.phase(), PlayPhase::Finished);
    assert!(!scene.stop_finished(&mut driver, &mut timers, started_at + FRAME_US * 2), "a run that is over was ended a second time");
    assert_eq!(timers.value_us(timer_id::FADEOUT), started_at + FRAME_US, "the fade was started again");

    let (mut scene, mut driver, mut timers, started_at) = playing();
    scene.step(&mut driver, &mut timers, &SceneFacts { failed: true, may_retry: false, ..facts(started_at + FRAME_US, 0) });
    assert!(!scene.stop_finished(&mut driver, &mut timers, started_at + FRAME_US * 2));
    assert_eq!(scene.phase(), PlayPhase::Failed);
}

/// A run is over once it has played for longer than its playing time, and its last note has gone by
/// the reference's five seconds before that.
#[test]
fn a_run_ends_past_its_playing_time_and_its_last_note_goes_by_five_seconds_earlier() {
    const PLAY_TIME_MS: i64 = 8_000;
    assert_eq!(RunEnd::of(PLAY_TIME_MS, 0), RunEnd::default());
    assert_eq!(RunEnd::of(PLAY_TIME_MS, PLAY_TIME_MS - TIME_MARGIN_MS), RunEnd::default(), "the last note's own moment is not past it");
    assert_eq!(RunEnd::of(PLAY_TIME_MS, PLAY_TIME_MS - TIME_MARGIN_MS + 1), RunEnd { ended: false, last_note_passed: true });
    assert_eq!(RunEnd::of(PLAY_TIME_MS, PLAY_TIME_MS), RunEnd { ended: false, last_note_passed: true }, "the playing time itself is not past it");
    assert_eq!(RunEnd::of(PLAY_TIME_MS, PLAY_TIME_MS + 1), RunEnd { ended: true, last_note_passed: true });
}
