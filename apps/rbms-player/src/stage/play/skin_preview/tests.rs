//! The sample chart and the hand that plays it: that every mode a play screen draws has a chart,
//! and that a run of it shows a skin more than one judgement and never empties a gauge.

use rbms_config::Config;
use rbms_judge::GaugeKind;
use rbms_model::{Mode, NoteKind};

use super::*;

/// The judgements of a run's tally, as a score keeps them: the three a combo runs on, and the first
/// of the ones that break it.
const PGREAT: usize = 0;
const GREAT: usize = 1;
const GOOD: usize = 2;
const FIRST_COMBO_BREAK: usize = 3;

/// How long the sample chart is allowed to be and has to be at the least, in milliseconds: a scene
/// of about half a minute with the load before it and the fade after.
const SHORTEST_CHART_MS: i64 = 15_000;
const LONGEST_CHART_MS: i64 = 30_000;

fn kinds(model: &Model, wanted: impl Fn(&NoteKind) -> bool) -> usize {
    model.timelines.iter().flat_map(|line| line.notes.iter().flatten()).filter(|note| wanted(&note.kind)).count()
}

#[test]
fn every_mode_a_play_screen_draws_has_a_sample_chart() {
    for mode in Mode::ALL {
        let model = sample_model(*mode).unwrap_or_else(|| panic!("no sample chart is written for {}", mode.name));
        assert_eq!(model.mode, *mode);
        assert_eq!((model.meta.title.as_str(), model.meta.artist.as_str()), (SAMPLE_TITLE, SAMPLE_ARTIST));
        assert!(kinds(&model, |kind| matches!(kind, NoteKind::Normal)) > 0, "{} has no plain note", mode.name);
        assert!(kinds(&model, |kind| matches!(kind, NoteKind::LongStart { .. })) > 0, "{} has no long note", mode.name);
        assert_eq!(kinds(&model, |kind| matches!(kind, NoteKind::Mine { .. })), 0);
        let on_the_turntable = model.timelines.iter().any(|line| mode.scratch.iter().any(|lane| line.notes.get(*lane).is_some_and(Option::is_some)));
        assert_eq!(on_the_turntable, !mode.scratch.is_empty(), "{} and its turntable", mode.name);
        let every_lane_plays = (0..mode.key).all(|lane| model.timelines.iter().any(|line| line.notes.get(lane).is_some_and(Option::is_some)));
        assert!(every_lane_plays, "a lane of {} is never played", mode.name);
        let tempos: std::collections::BTreeSet<u64> = model.timelines.iter().map(|line| line.bpm.to_bits()).collect();
        assert!(tempos.len() > 1, "{} never changes its tempo", mode.name);
        assert!(model.bgamap.iter().all(|name| name.trim().is_empty()) && model.meta.stagefile.is_empty(), "{} names a picture", mode.name);
        let last_ms = rbms_play::last_note_time_ms(&model);
        assert!((SHORTEST_CHART_MS..=LONGEST_CHART_MS).contains(&last_ms), "{} runs for {last_ms} ms", mode.name);
    }
    assert!(sample_model(Mode::KEYBOARD_24K).is_none());
}

/// The sample is shown at the speed a run of it would start at: the speed the settings hold, and,
/// while a green number is held from chart to chart, the speed that holds it on the sample chart
/// under the lane cover as it stands.
#[test]
fn the_sample_scrolls_at_the_speed_a_run_of_it_would_start_at() {
    use rbms_config::travel_duration_ms;
    use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
    let mut config = Config::default();
    config.play.hispeed = 3.25;
    config.play.cover = 0.4;
    let sample = PlaySample::new(SKIN_TYPE_PLAY_7KEYS, &config).expect("seven keys have a sample");
    assert_eq!(sample.shown_hispeed(&config), 3.25, "nothing is held, so the speed is the settings' own");

    config.play.keep_green_number = true;
    let held = sample.shown_hispeed(&config);
    let tempo = sample.bpm.held(&config.play).expect("a held number has a tempo");
    assert_eq!(travel_duration_ms(tempo, held, Some(0.4)), config.play.duration_ms, "the sample is not shown at the travel time that is held");
    config.play.enable_cover = false;
    assert!(sample.shown_hispeed(&config) > held, "a lane with no cover over it scrolls faster for the same number");
    assert_eq!(travel_duration_ms(tempo, sample.shown_hispeed(&config), None), config.play.duration_ms);
}

/// A sample written at one tempo holds it from its first note to its last in every mode, with the
/// notes the shipped sample has, and is drawn at the speed the play menu writes beside it: the
/// figures of a target come from the same functions the sample's own speed does.
#[test]
fn a_steady_sample_holds_its_tempo_and_scrolls_at_the_speed_its_target_reads() {
    use crate::skin_preview::PlayTarget;
    use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
    const TEMPOS: [u32; 3] = [100, 150, 200];
    for mode in Mode::ALL {
        let shipped = sample_model(*mode).expect("the mode has a sample chart");
        for bpm in TEMPOS {
            let steady = sample_model_at(*mode, SampleTempo::Steady(bpm)).expect("the mode has a sample chart");
            assert!((steady.init_bpm - f64::from(bpm)).abs() < f64::EPSILON, "{} opens at {}", mode.name, steady.init_bpm);
            assert!(steady.timelines.iter().all(|line| (line.bpm - f64::from(bpm)).abs() < f64::EPSILON), "{} changes its tempo", mode.name);
            assert_eq!(kinds(&steady, |_| true), kinds(&shipped, |_| true), "{} at {bpm} has other notes than the shipped sample", mode.name);
        }
    }

    let mut config = Config::default();
    config.play.hispeed = 3.25;
    config.play.cover = 0.3;
    for bpm in TEMPOS {
        let target = PlayTarget { screen: SKIN_TYPE_PLAY_7KEYS, bpm };
        let sample = PlaySample::at_tempo(target.screen, &config, SampleTempo::Steady(bpm)).expect("seven keys have a sample");
        for (keep_green_number, constant_speed) in [(false, false), (true, false), (false, true), (true, true)] {
            config.play.keep_green_number = keep_green_number;
            config.play.constant_speed = constant_speed;
            let readout = target.readout(&config.play);
            assert!(
                (sample.shown_hispeed(&config) - readout.hispeed).abs() < f64::EPSILON,
                "{bpm} held {keep_green_number} constant {constant_speed}: the sample scrolls at {} and its target reads {}",
                sample.shown_hispeed(&config),
                readout.hispeed
            );
            assert_eq!(readout.held, keep_green_number);
            if keep_green_number {
                assert_eq!(sample.bpm.held(&config.play), Some(readout.bpm), "the tempo the number is held at");
                assert_eq!(readout.duration_ms, config.play.duration_ms);
            }
        }
    }
    assert!((steady_scroll_bpm(&config.play, 0) - f64::from(STEADY_BPM_MIN)).abs() > f64::EPSILON, "CONSTANT is still on, so the chart's tempo is not asked");
    config.play.constant_speed = false;
    assert!((steady_scroll_bpm(&config.play, 0) - f64::from(STEADY_BPM_MIN)).abs() < f64::EPSILON, "a tempo of nothing is not held to the slowest");
    assert!((steady_scroll_bpm(&config.play, u32::MAX) - f64::from(STEADY_BPM_MAX)).abs() < f64::EPSILON);
}

/// The run reaches its end with every note judged, on a combo that never broke, with more than one
/// judgement given and both early and late hits, whatever gauge the player has chosen.
#[test]
fn the_written_hand_plays_every_note_without_breaking_its_combo() {
    for gauge in [GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard] {
        for mode in Mode::ALL {
            let mut config = Config::default();
            config.play.gauge = gauge;
            let session = played_sample(*mode, &config, None).expect("the mode has a sample chart");
            let summary = session.summary();
            let label = format!("{} on {gauge:?}: {summary:?}", mode.name);
            assert!(summary.total_notes > 0 && summary.total_judged == summary.total_notes, "{label}");
            assert!(!summary.failed && summary.clear_lamp.is_cleared(), "{label}");
            assert!(summary.counts[FIRST_COMBO_BREAK..].iter().all(|count| *count == 0), "{label}");
            assert_eq!(summary.max_combo, summary.total_notes, "{label}");
            assert!(summary.counts[PGREAT] > summary.counts[GREAT] && summary.counts[GREAT] > 0, "{label}");
            assert!(summary.counts[GOOD] > 0 || *mode == Mode::POPN_9K, "{label}");
            assert!(summary.fast > 0 && summary.slow > 0, "{label}");
        }
    }
}

/// A stage of a course begins on the gauge and the combo it is handed.
#[test]
fn a_stage_of_a_course_begins_on_what_the_one_before_left() {
    const CARRIED: Carried = Carried { gauge: 64.0, combo: 120 };
    let session = sample_session(sample_model(Mode::BEAT_7K).expect("seven keys have a sample chart"), &Config::default(), Some(CARRIED));
    assert_eq!(session.judge().combo, CARRIED.combo);
    assert_eq!(session.judge().gauge.selected_index(), GaugeIndex::Class);
    assert!((session.judge().gauge.value() - CARRIED.gauge).abs() < f32::EPSILON);
}

/// A run that closes is at one tempo and on one gauge for the whole of its scene. A run that begins
/// again takes another tempo and another gauge where it stands, and has nothing to take up while
/// neither moved.
#[test]
fn only_a_run_that_begins_again_takes_another_tempo_or_gauge() {
    use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
    const SLOW: SampleTempo = SampleTempo::Steady(100);
    const FAST: SampleTempo = SampleTempo::Steady(200);
    let config = Config::default();
    let another_gauge = rbms_config::PlayOptions { gauge: GaugeKind::Hard, ..config.play.clone() };
    let sample = || PlaySample::at_tempo(SKIN_TYPE_PLAY_7KEYS, &config, SLOW).expect("seven keys have a sample");

    let mut closing = sample();
    assert!(!closing.show_tempo(FAST), "a run that closes took another tempo");
    assert_eq!(closing.due_run(&another_gauge), None, "a run that closes is due to begin again");

    let mut going_on = sample().ending(SampleEnding::BeginsAgain);
    assert_eq!(going_on.due_run(&config.play), None, "a run with nothing new to take up is due to begin again");
    assert_eq!(going_on.due_run(&another_gauge), Some(RunStart::Carried));
    assert!(going_on.show_tempo(SLOW) && going_on.due_run(&config.play).is_none(), "the tempo the run is at began it again");
    assert!(going_on.show_tempo(FAST));
    assert_eq!(going_on.due_run(&config.play), Some(RunStart::Carried));

    going_on.chart_us = (going_on.session.play_time_ms() - TIME_MARGIN_MS + BEGIN_AGAIN_AFTER_MS + 1) * MICROS_PER_MILLI;
    assert_eq!(going_on.due_run(&config.play), Some(RunStart::Top), "a run past its last note is not begun at the top");
    const { assert!(BEGIN_AGAIN_AFTER_MS < TIME_MARGIN_MS, "a run that begins again would be told its chart ran out first") };
}

/// A run shown for the play options is told as the AUTOPLAY and TARGET rows have it, and a run
/// shown for its skin is one played by hand with nothing to pace against whatever they hold.
#[test]
fn only_a_run_shown_for_the_play_options_follows_the_autoplay_and_target_rows() {
    use rbms_config::ScoreTarget;
    use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
    let by_hand = Config { play: rbms_config::PlayOptions { autoplay: false, ..Config::default().play }, ..Config::default() };
    let mut moved = by_hand.clone();
    moved.play.autoplay = true;
    moved.judge.target = ScoreTarget::Max;
    let sample = || PlaySample::at_tempo(SKIN_TYPE_PLAY_7KEYS, &by_hand, SampleTempo::Steady(150)).expect("seven keys have a sample");

    let for_its_skin = sample();
    assert!(matches!(for_its_skin.told(&by_hand), (PlayKind::Play, 0)));
    assert!(matches!(for_its_skin.told(&moved), (PlayKind::Play, 0)), "a run shown for its skin followed a play option");

    let for_the_options = sample().ending(SampleEnding::BeginsAgain);
    let ceiling = for_the_options.session.judge().total_notes() * 2;
    assert!(matches!(for_the_options.told(&moved), (PlayKind::Autoplay, target) if target == ceiling), "the ceiling of the chart is not the target");
    let (kind, target) =
        for_the_options.told(&Config { judge: rbms_config::JudgeOptions { target: ScoreTarget::RateAaa, ..by_hand.judge.clone() }, ..by_hand.clone() });
    assert!(matches!(kind, PlayKind::Play), "a run nobody asked the game to play is told as one it plays");
    assert!(target > 0 && target < ceiling, "a rate under the ceiling is paced at {target} of {ceiling}");
    assert_eq!(for_the_options.held_against(&moved, 0).target_score, ceiling, "the timers are told another target than the figures");
}
