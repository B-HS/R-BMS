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
