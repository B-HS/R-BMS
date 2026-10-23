use super::*;
use crate::to_model;
use rbms_parser::parse;

const EPS: f64 = 1e-9;

const DISPLAY_TOLERANCE: f64 = 1e-3;

const RADAR_MODES: [Mode; 4] = [Mode::BEAT_5K, Mode::BEAT_7K, Mode::BEAT_10K, Mode::BEAT_14K];

fn chart_in(header: &str, body: &str, mode: Mode) -> Model {
    to_model(&parse(format!("{header}#WAV01 a.wav\r\n{body}").as_bytes()), mode)
}

fn chart(header: &str, body: &str) -> Model {
    chart_in(header, body, Mode::BEAT_7K)
}

fn raw_with(header: &str, body: &str) -> RadarRaw {
    radar_raw(&chart(header, body), &Mode::BEAT_7K).unwrap()
}

fn raw_of(body: &str) -> RadarRaw {
    raw_with("#BPM 240\r\n", body)
}

fn raw_in(body: &str, mode: Mode) -> RadarRaw {
    radar_raw(&chart_in("#BPM 240\r\n", body, mode), &mode).unwrap()
}

fn taps(channel: &str, seconds: impl IntoIterator<Item = u32>) -> String {
    seconds.into_iter().map(|second| format!("#{second:03}{channel}:01\r\n")).collect()
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < EPS, "{actual} != {expected}");
}

fn only(axis: RadarAxis, value: f64) -> RadarRaw {
    let mut raw = RadarRaw::default();
    match axis {
        RadarAxis::Notes => raw.notes = value,
        RadarAxis::Peak => raw.peak = value,
        RadarAxis::Scratch => raw.scratch = value,
        RadarAxis::Soflan => raw.soflan = value,
        RadarAxis::Charge => raw.charge = value,
        RadarAxis::Chord => raw.chord = value,
    }
    raw
}

fn curves_of(mode: &Mode) -> &'static [Curve; RADAR_AXIS_COUNT] {
    match radar_style(mode).unwrap() {
        RadarStyle::Single => &SINGLE_CURVES,
        RadarStyle::Double => &DOUBLE_CURVES,
    }
}

#[test]
fn axis_discriminants_are_the_axis_indices() {
    for (index, axis) in RadarAxis::ALL.iter().enumerate() {
        assert_eq!(*axis as usize, index);
    }
    let raw = RadarRaw { notes: 1.0, peak: 2.0, scratch: 3.0, soflan: 4.0, charge: 5.0, chord: 6.0 };
    assert_eq!(RadarAxis::ALL.map(|axis| raw.value(axis)), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn notes_is_the_press_count_over_the_span_from_first_to_last_press() {
    assert_close(raw_of(&taps("11", 0..=10)).notes, 1.1);
}

#[test]
fn span_has_a_one_second_floor() {
    assert_close(raw_of("#00011:0101\r\n").notes, 2.0);
    let single = raw_of(&taps("11", [0]));
    assert_close(single.notes, 1.0);
    assert_close(single.peak, 1.0);
}

#[test]
fn a_chart_without_press_events_is_all_zero() {
    assert_eq!(raw_of(""), RadarRaw::default());
    assert_eq!(raw_of("#00001:01\r\n"), RadarRaw::default());
}

#[test]
fn an_absent_axis_is_positive_zero() {
    let plain = raw_of(&taps("11", 0..=10));
    for raw in [raw_of(""), plain] {
        for axis in [RadarAxis::Soflan, RadarAxis::Charge, RadarAxis::Chord] {
            assert!(raw.value(axis).is_sign_positive(), "{axis:?}");
        }
    }
    assert_eq!(plain.charge, 0.0);
    assert_eq!(plain.soflan, 0.0);
}

#[test]
fn mines_are_not_counted() {
    let raw = raw_of(&(taps("11", [0, 10]) + &taps("D1", [5])));
    assert_close(raw.notes, 0.2);
    assert_close(raw.peak, 1.0);
}

#[test]
fn peak_window_is_four_seconds_and_half_open() {
    assert_close(raw_of(&taps("11", 0..=10)).peak, 4.0);
}

#[test]
fn peak_finds_the_densest_window_anywhere() {
    let burst = "#02011:0101010101010101\r\n";
    assert_close(raw_of(&(taps("11", [0, 10, 30]) + burst)).peak, 8.0);
}

#[test]
fn peak_counts_every_lane_and_simultaneous_presses() {
    let raw = raw_of(&(taps("11", [0, 10]) + &taps("12", [0]) + &taps("16", [0])));
    assert_close(raw.peak, 3.0);
}

#[test]
fn peak_does_not_count_long_note_bodies() {
    let raw = raw_of(&(taps("51", [0, 3]) + &taps("12", [10])));
    assert_close(raw.peak, 1.0);
    assert_close(raw.notes, 0.2);
}

#[test]
fn scratch_counts_only_scratch_lane_presses() {
    let raw = raw_of(&(taps("16", [0, 10]) + &taps("11", [5])));
    assert_close(raw.notes, 0.3);
    assert_close(raw.scratch, 0.2);
}

#[test]
fn a_scratch_long_note_counts_as_one_scratch_press() {
    let raw = raw_of(&(taps("56", [0, 3]) + &taps("11", [10])));
    assert_close(raw.scratch, 0.1);
    assert_close(raw.charge, 0.3);
}

#[test]
fn double_play_sums_both_sides_and_both_scratch_lanes() {
    let raw = raw_in(&(taps("16", [0]) + &taps("26", [10]) + &taps("11", [5]) + &taps("21", [5])), Mode::BEAT_14K);
    assert_close(raw.notes, 0.4);
    assert_close(raw.scratch, 0.2);
    assert_close(raw.chord, 0.02);
}

#[test]
fn the_five_key_and_ten_key_modes_have_a_radar() {
    assert_close(raw_in(&taps("16", [0, 10]), Mode::BEAT_5K).scratch, 0.2);
    assert_close(raw_in(&(taps("16", [0]) + &taps("26", [10])), Mode::BEAT_10K).scratch, 0.2);
}

#[test]
fn modes_without_a_radar_return_none() {
    for mode in [Mode::POPN_9K, Mode::KEYBOARD_24K] {
        let model = chart_in("#BPM 240\r\n", &taps("11", 0..=10), mode);
        assert_eq!(radar_raw(&model, &mode), None, "{}", mode.name);
    }
}

#[test]
fn chord_weights_follow_the_published_progression() {
    let expected = [(2, 0.2), (3, 0.45), (4, 0.75), (5, 1.10), (6, 1.50), (7, 1.95), (8, 2.45)];
    for (simultaneous, weight) in expected {
        assert_close(chord_weight(simultaneous), weight);
    }
}

#[test]
fn chord_sums_the_weight_of_each_simultaneous_row() {
    let body = taps("11", [0, 1, 10]) + &taps("12", [0, 1]) + &taps("13", [1]);
    assert_close(raw_of(&body).chord, (0.2 + 0.45) / 10.0);
}

#[test]
fn chord_counts_the_scratch_and_a_full_row_of_eight() {
    let row: String = ["11", "12", "13", "14", "15", "16", "18", "19"].iter().map(|channel| taps(channel, [0])).collect();
    let raw = raw_of(&(row + &taps("11", [10])));
    assert_close(raw.chord, 2.45 / 10.0);
    assert_close(raw.peak, 8.0);
}

#[test]
fn chord_ignores_presses_at_different_times() {
    assert_close(raw_of(&(taps("11", [0, 2, 4]) + &taps("12", [1, 3, 10]))).chord, 0.0);
}

#[test]
fn a_lone_long_note_scores_its_head() {
    assert_close(raw_of(&(taps("51", [0, 3]) + &taps("12", [10]))).charge, 0.3);
}

#[test]
fn charge_adds_notes_pressed_with_the_head() {
    assert_close(raw_of(&(taps("51", [0, 3]) + &taps("12", [0, 10]))).charge, 0.6);
}

#[test]
fn charge_adds_notes_pressed_while_held_through_the_tail_time() {
    assert_close(raw_of(&(taps("51", [0, 3]) + &taps("12", [1, 2, 3, 4, 10]))).charge, 1.5);
}

#[test]
fn charge_of_simultaneous_long_notes_grows_with_the_square() {
    let raw = raw_of(&(taps("51", [0, 3]) + &taps("52", [0, 3]) + &taps("53", [0, 3]) + &taps("14", [10])));
    assert_close(raw.charge, 2.7);
}

#[test]
fn charge_counts_a_long_note_head_inside_another_but_not_a_tail() {
    let raw = raw_of(&(taps("51", [0, 4]) + &taps("52", [1, 2]) + &taps("14", [10])));
    assert_close(raw.charge, 1.0);
}

#[test]
fn the_long_note_flavour_does_not_change_any_axis() {
    let body = taps("51", [0, 3]) + &taps("12", [1, 10]) + &taps("56", [4, 6]);
    let undefined = raw_with("#BPM 240\r\n", &body);
    assert_close(undefined.charge, (3.0 + 4.0 + 3.0) / 10.0);
    for lnmode in 1..=3 {
        let stated = raw_with(&format!("#BPM 240\r\n#LNMODE {lnmode}\r\n"), &body);
        assert_eq!(stated, undefined, "#LNMODE {lnmode}");
    }
}

#[test]
fn a_steady_tempo_has_no_soflan() {
    assert_close(raw_of(&taps("11", 0..=10)).soflan, 0.0);
}

#[test]
fn soflan_weights_notes_after_a_change_by_how_soon_they_follow() {
    let body = "#00011:01\r\n#00108:01\r\n#00111:0101\r\n#00211:01\r\n#00311:01\r\n";
    let raw = raw_with("#BPM 120\r\n#BPM01 240\r\n", body);
    assert_close(raw.soflan, 1.0 + 0.75 + 0.5);
}

#[test]
fn the_next_change_closes_the_previous_window() {
    let body = "#00108:01\r\n#00208:02\r\n#00111:0101\r\n#00211:01010100\r\n#00311:01\r\n";
    let raw = raw_with("#BPM 120\r\n#BPM01 240\r\n#BPM02 120\r\n", body);
    assert_close(raw.soflan, 1.0 + 0.75 + 1.0 + 0.75 + 0.5);
}

#[test]
fn the_change_ratio_is_clamped() {
    let jump = raw_with("#BPM 120\r\n#BPM01 480\r\n", "#00108:01\r\n#00111:01\r\n");
    assert_close(jump.soflan, SOFLAN_RATIO_MAX);
    let nudge = raw_with("#BPM 120\r\n#BPM01 121\r\n", "#00108:01\r\n#00111:01\r\n");
    assert_close(nudge.soflan, SOFLAN_RATIO_MIN);
}

#[test]
fn a_slowdown_scores_the_same_as_the_matching_speedup() {
    let faster = raw_with("#BPM 120\r\n#BPM01 180\r\n", "#00108:01\r\n#00111:01\r\n");
    let slower = raw_with("#BPM 180\r\n#BPM01 120\r\n", "#00108:01\r\n#00111:01\r\n");
    assert_close(faster.soflan, 0.5);
    assert_close(faster.soflan, slower.soflan);
}

#[test]
fn a_stop_and_the_resume_after_it_each_open_a_window() {
    let raw = raw_with("#BPM 240\r\n#STOP01 48\r\n", "#00109:01\r\n#00111:0101\r\n");
    assert_close(raw.soflan, 2.0 * SOFLAN_RATIO_MAX);
}

#[test]
fn a_scroll_change_counts_like_the_same_tempo_change() {
    let scrolled = raw_with("#BPM 120\r\n#SCROLL01 2.0\r\n", "#001SC:01\r\n#00111:01\r\n");
    let retimed = raw_with("#BPM 120\r\n#BPM01 240\r\n", "#00108:01\r\n#00111:01\r\n");
    assert_close(scrolled.soflan, 1.0);
    assert_close(scrolled.soflan, retimed.soflan);
}

#[test]
fn a_tempo_set_on_the_first_timeline_is_the_starting_tempo() {
    let raw = raw_with("#BPM 120\r\n#BPM01 240\r\n", "#00008:01\r\n#00011:0101\r\n");
    assert_close(raw.soflan, 0.0);
}

#[test]
fn hostile_timelines_still_give_finite_values() {
    let mut model = chart("#BPM 240\r\n", &(taps("11", 0..=10) + &taps("51", [2, 5])));
    let tempos = [f64::MAX, 1e-300, 120.0, f64::INFINITY];
    let scrolls = [f64::INFINITY, f64::NAN, -2.0, 0.0];
    let count = model.timelines.len() as i64;
    for (index, timeline) in model.timelines.iter_mut().enumerate() {
        timeline.bpm = tempos[index % tempos.len()];
        timeline.scroll = scrolls[index % scrolls.len()];
        timeline.time_us = i64::MAX - count + index as i64;
    }
    let raw = radar_raw(&model, &Mode::BEAT_7K).unwrap();
    for axis in RadarAxis::ALL {
        assert!(raw.value(axis).is_finite(), "{axis:?}");
    }
    for display in radar_display(&raw, &Mode::BEAT_7K) {
        assert!((0.0..=RADAR_DISPLAY_MAX).contains(&display), "{display}");
    }
}

#[test]
fn every_curve_rises_strictly_from_a_positive_start() {
    for curves in [&SINGLE_CURVES, &DOUBLE_CURVES] {
        for curve in curves {
            assert!(curve[0] > 0.0);
            assert!(curve.windows(2).all(|pair| pair[0] < pair[1]), "{curve:?}");
        }
    }
}

#[test]
fn peak_curves_match_the_community_measurement_where_it_overlaps() {
    assert_eq!(SINGLE_CURVES[RadarAxis::Peak as usize], [70.0, 108.0, 125.0, 136.0]);
    assert_eq!(DOUBLE_CURVES[RadarAxis::Peak as usize][..2], [70.0, 113.0]);
    assert_eq!(DOUBLE_CURVES[RadarAxis::Peak as usize][3], 170.0);
}

#[test]
fn breakpoints_display_as_100_150_175_and_200() {
    for mode in RADAR_MODES {
        let curves = curves_of(&mode);
        for axis in RadarAxis::ALL {
            let index = axis as usize;
            for (breakpoint, expected) in DISPLAY_AT_BREAKPOINT.iter().enumerate() {
                let raw = only(axis, curves[index][breakpoint] / AXIS_SCALE[index]);
                let display = radar_display(&raw, &mode)[index] as f64;
                assert!((display - expected).abs() < DISPLAY_TOLERANCE, "{} {axis:?} p{breakpoint}: {display}", mode.name);
            }
        }
    }
}

#[test]
fn display_is_zero_at_zero_and_linear_up_to_the_first_breakpoint() {
    for mode in RADAR_MODES {
        let curves = curves_of(&mode);
        for axis in RadarAxis::ALL {
            let index = axis as usize;
            assert_eq!(radar_display(&only(axis, 0.0), &mode)[index], 0.0);
            let half = radar_display(&only(axis, curves[index][0] / AXIS_SCALE[index] / 2.0), &mode)[index] as f64;
            assert!((half - DISPLAY_AT_BREAKPOINT[0] / 2.0).abs() < DISPLAY_TOLERANCE, "{} {axis:?}: {half}", mode.name);
        }
    }
}

#[test]
fn display_clamps_to_the_range_and_survives_bad_input() {
    for axis in RadarAxis::ALL {
        let index = axis as usize;
        for past_the_end in [1e18, f64::MAX, f64::INFINITY] {
            assert_eq!(radar_display(&only(axis, past_the_end), &Mode::BEAT_7K)[index], RADAR_DISPLAY_MAX, "{axis:?} {past_the_end}");
        }
        for below_zero in [-1.0, f64::NEG_INFINITY, f64::NAN] {
            assert_eq!(radar_display(&only(axis, below_zero), &Mode::BEAT_7K)[index], 0.0, "{axis:?} {below_zero}");
        }
    }
}

#[test]
fn display_never_decreases_as_the_raw_value_grows() {
    const STEPS: usize = 2_000;
    for mode in RADAR_MODES {
        let curves = curves_of(&mode);
        for axis in RadarAxis::ALL {
            let index = axis as usize;
            let limit = curves[index][BREAKPOINT_COUNT - 1] * 1.5 / AXIS_SCALE[index];
            let mut last = 0.0;
            for step in 0..=STEPS {
                let display = radar_display(&only(axis, limit * step as f64 / STEPS as f64), &mode)[index];
                assert!(display >= last && display <= RADAR_DISPLAY_MAX, "{} {axis:?} step {step}", mode.name);
                last = display;
            }
        }
    }
}

#[test]
fn single_and_double_play_use_different_curves() {
    let raw = only(RadarAxis::Notes, 18.0);
    let single = radar_display(&raw, &Mode::BEAT_7K)[RadarAxis::Notes as usize] as f64;
    let double = radar_display(&raw, &Mode::BEAT_14K)[RadarAxis::Notes as usize] as f64;
    assert!((single - (175.0 + 25.0 * 0.3 / 2.8)).abs() < DISPLAY_TOLERANCE, "{single}");
    assert!((double - (175.0 + 25.0 * 0.1 / 2.1)).abs() < DISPLAY_TOLERANCE, "{double}");
    assert_eq!(radar_display(&raw, &Mode::BEAT_5K), radar_display(&raw, &Mode::BEAT_7K));
    assert_eq!(radar_display(&raw, &Mode::BEAT_10K), radar_display(&raw, &Mode::BEAT_14K));
}

#[test]
fn display_of_a_mode_without_a_radar_is_all_zero() {
    let raw = RadarRaw { notes: 30.0, peak: 300.0, scratch: 30.0, soflan: 1e6, charge: 1e6, chord: 1e6 };
    for mode in [Mode::POPN_9K, Mode::KEYBOARD_24K] {
        assert_eq!(radar_display(&raw, &mode), [0.0; RADAR_AXIS_COUNT]);
    }
}

#[test]
fn a_chart_flows_from_raw_to_display() {
    let raw = raw_of(&(taps("11", 0..=10) + &taps("16", [0, 5, 10])));
    let display = radar_display(&raw, &Mode::BEAT_7K);
    for axis in [RadarAxis::Notes, RadarAxis::Peak, RadarAxis::Scratch, RadarAxis::Chord] {
        assert!(display[axis as usize] > 0.0, "{axis:?}");
    }
    assert_eq!(display[RadarAxis::Soflan as usize], 0.0);
    assert_eq!(display[RadarAxis::Charge as usize], 0.0);
}
