use super::*;

/// A chart long enough that both time bounds have room to move.
const LONG_CHART_MS: i32 = 180_000;

/// A property on a seven-key chart, as the panel would build it before any editing.
fn property(last_tl_ms: i32) -> PracticeProperty {
    PracticeProperty::for_chart(Mode::BEAT_7K, last_tl_ms)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms-practice-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temp dir");
    dir
}

#[test]
fn round_down_truncates_toward_zero_on_both_sides_of_the_origin() {
    assert_eq!(round_down_to(1_250, TIME_ROUND_MS), 1_200);
    assert_eq!(round_down_to(1_200, TIME_ROUND_MS), 1_200);
    assert_eq!(round_down_to(-1_250, TIME_ROUND_MS), -1_200, "the reference divides in Java, which truncates toward zero rather than flooring");
}

#[test]
fn start_time_stops_two_seconds_short_of_the_last_timeline_rounded_down() {
    let mut p = property(LONG_CHART_MS);
    set_start_ms(&mut p, LONG_CHART_MS + 50, LONG_CHART_MS);
    assert_eq!(p.start_ms, round_down_to(LONG_CHART_MS + 50 - 2_000, 100), "the ceiling is the rounded-down last timeline less two seconds");
    assert_eq!(p.start_ms, 178_000);
}

#[test]
fn start_time_stops_at_zero_going_down() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 50;
    set_start_ms(&mut p, LONG_CHART_MS, -50);
    assert_eq!(p.start_ms, 0, "the floor is zero, not a negative step");
}

#[test]
fn raising_start_time_pushes_end_time_up_with_no_rounding() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 2_550;
    p.end_ms = 3_000;
    set_start_ms(&mut p, LONG_CHART_MS, 2_650);
    assert_eq!(p.start_ms, 2_650);
    assert_eq!(p.end_ms, 3_650, "the push is a plain max of start plus one second, with no rounding of its own");
}

#[test]
fn lowering_start_time_leaves_end_time_where_it_was() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 5_000;
    p.end_ms = 5_100;
    set_start_ms(&mut p, LONG_CHART_MS, 4_900);
    assert_eq!(p.start_ms, 4_900);
    assert_eq!(p.end_ms, 5_100, "only an increase drags the end along");
}

#[test]
fn end_time_reaches_one_second_past_the_last_timeline_rounded_down() {
    let mut p = property(LONG_CHART_MS);
    set_end_ms(&mut p, LONG_CHART_MS + 50, i32::MAX);
    assert_eq!(p.end_ms, round_down_to(LONG_CHART_MS + 50 + 1_000, 100));
    assert_eq!(p.end_ms, 181_000);
}

#[test]
fn end_time_floors_on_the_rounded_down_start_plus_one_second() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 2_550;
    p.end_ms = 3_600;
    set_end_ms(&mut p, LONG_CHART_MS, 3_500);
    assert_eq!(p.end_ms, 3_500, "one step down is still above the floor");
    set_end_ms(&mut p, LONG_CHART_MS, 0);
    assert_eq!(p.end_ms, round_down_to(2_550 + 1_000, 100), "the floor rounds down, unlike the push from START TIME");
    assert_eq!(p.end_ms, 3_500);
}

/// The reference works out a ceiling before zero for a chart shorter than two seconds and lets
/// START TIME follow it there. Such a chart is practised from its beginning here.
#[test]
fn a_chart_shorter_than_the_start_time_tail_starts_at_its_beginning() {
    let mut p = property(500);
    set_start_ms(&mut p, 500, 100);
    assert_eq!(p.start_ms, 0, "a slice began before or past the only place such a chart starts at");
    set_end_ms(&mut p, 500, 0);
    assert_eq!(p.end_ms, MIN_PLAY_SPAN_MS, "the shortest slice is a second, whatever the chart's length");
    set_end_ms(&mut p, 500, i32::MAX);
    assert_eq!(p.end_ms, 1_500);
    let panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, 500, None, 260.0);
    let start = panel.number(PracticeElement::StartTime).expect("START TIME holds a number");
    assert_eq!((start.spec.min, start.spec.max, start.steps), (0, 0, 0), "a row with nowhere to go has more than one step");
}

#[test]
fn the_gauge_cycles_through_all_nine_and_wraps_both_ways() {
    let mut p = property(LONG_CHART_MS);
    p.gauge_type = 0;
    for want in 1..GaugeIndex::COUNT {
        cycle_gauge_type(&mut p, Mode::BEAT_7K, true);
        assert_eq!(usize::from(p.gauge_type), want);
    }
    cycle_gauge_type(&mut p, Mode::BEAT_7K, true);
    assert_eq!(p.gauge_type, 0, "past the last gauge is the first one again");
    cycle_gauge_type(&mut p, Mode::BEAT_7K, false);
    assert_eq!(usize::from(p.gauge_type), GaugeIndex::COUNT - 1, "and below the first is the last");
}

#[test]
fn a_popn_chart_caps_the_starting_gauge_once_a_survival_gauge_is_selected() {
    let mut p = property(LONG_CHART_MS);
    p.gauge_type = 2;
    p.start_gauge = 120;
    cycle_gauge_type(&mut p, Mode::POPN_9K, true);
    assert_eq!(p.gauge_type, PMS_SURVIVAL_GAUGE_INDEX);
    assert_eq!(p.start_gauge, PMS_START_GAUGE_CAP, "a survival gauge on a pop'n chart caps the starting amount at 100");
}

#[test]
fn the_popn_cap_leaves_a_groove_gauge_and_other_modes_alone() {
    let mut groove = property(LONG_CHART_MS);
    groove.gauge_type = 0;
    groove.start_gauge = 120;
    cycle_gauge_type(&mut groove, Mode::POPN_9K, true);
    assert_eq!(groove.start_gauge, 120, "EASY is below the survival gauges, so nothing is capped");

    let mut beat = property(LONG_CHART_MS);
    beat.gauge_type = 2;
    beat.start_gauge = 120;
    cycle_gauge_type(&mut beat, Mode::BEAT_7K, true);
    assert_eq!(beat.start_gauge, 120, "the cap is a pop'n rule only");
}

#[test]
fn the_gauge_value_clamps_to_the_selected_gauges_own_maximum() {
    let mut p = property(LONG_CHART_MS);
    let max = element_of(p.gauge_set(), p.gauge_index()).max as i32;
    set_start_gauge(&mut p, max + 1);
    assert_eq!(p.start_gauge, max, "the gauge's own maximum is the ceiling");
    set_start_gauge(&mut p, GAUGE_VALUE_MIN - 1);
    assert_eq!(p.start_gauge, GAUGE_VALUE_MIN, "and one is the floor");
}

#[test]
fn the_judge_rate_the_total_and_the_frequency_hold_their_bounds() {
    let mut p = property(LONG_CHART_MS);
    set_judge_rate(&mut p, JUDGE_RATE_MAX + 1);
    assert_eq!(p.judge_rate, JUDGE_RATE_MAX);
    set_judge_rate(&mut p, JUDGE_RATE_MIN - 1);
    assert_eq!(p.judge_rate, JUDGE_RATE_MIN);

    set_total(&mut p, TOTAL_MIN - 1.0);
    assert_eq!(p.total, TOTAL_MIN);
    set_total(&mut p, TOTAL_MAX + 1.0);
    assert_eq!(p.total, TOTAL_MAX);

    set_freq(&mut p, FREQ_MIN - 1);
    assert_eq!(p.freq, FREQ_MIN);
    set_freq(&mut p, FREQ_MAX + 1);
    assert_eq!(p.freq, FREQ_MAX);
}

#[test]
fn the_gauge_table_cycles_and_resets_the_starting_amount_to_the_new_tables_own() {
    let mut p = property(LONG_CHART_MS);
    assert_eq!(p.gauge_set(), GaugeSetId::SevenKeys, "a seven-key chart starts on the seven-key table");
    p.start_gauge = 1;
    cycle_gauge_set(&mut p, true);
    assert_eq!(p.gauge_set(), GaugeSetId::Pms);
    assert_eq!(p.start_gauge, element_of(GaugeSetId::Pms, p.gauge_index()).init as i32);
    cycle_gauge_set(&mut p, false);
    assert_eq!(p.gauge_set(), GaugeSetId::SevenKeys, "and back again");
}

#[test]
fn the_gauge_set_cycle_reaches_iidx_and_comes_back() {
    let mut p = property(LONG_CHART_MS);
    let first = p.gauge_set();
    let mut seen = Vec::new();
    for _ in 0..GaugeSetId::COUNT {
        cycle_gauge_set(&mut p, true);
        seen.push(p.gauge_set());
    }
    assert!(seen.contains(&GaugeSetId::Iidx), "the IIDX table is one of the practice tables: {seen:?}");
    assert_eq!(p.gauge_set(), first, "a whole turn lands on the table it started from");
    assert_eq!(gauge_set_token(GaugeSetId::Iidx), "IIDX");
    assert_eq!(gauge_set_from_token("IIDX"), Some(GaugeSetId::Iidx));
}

#[test]
fn a_mode_without_a_scratch_lane_cannot_practise_all_scratch() {
    let mut p = property(LONG_CHART_MS);
    let mut seen = Vec::new();
    for _ in 0..NoteOption::ALL.len() {
        cycle_option(&mut p, Mode::POPN_9K, true);
        seen.push(p.option());
    }
    assert!(!seen.contains(&NoteOption::AllScratch), "a nine-key pop'n chart has no scratch lane to shuffle onto");
    let allowed = &NoteOption::ALL[..NoteOption::ALL.len() - 1];
    for option in allowed {
        assert!(seen.contains(option), "{option:?} is missing from the shortened ring");
    }
    assert_eq!(seen.iter().filter(|o| **o == NoteOption::Off).count(), 1, "one whole turn of the shortened ring passes OFF once");
}

#[test]
fn a_mode_with_a_scratch_lane_reaches_every_shuffle() {
    let mut p = property(LONG_CHART_MS);
    let mut seen = Vec::new();
    for _ in 0..NoteOption::ALL.len() {
        cycle_option(&mut p, Mode::BEAT_7K, true);
        seen.push(p.option());
    }
    for option in NoteOption::ALL {
        assert!(seen.contains(&option), "{option:?} is missing from the ring");
    }
}

#[test]
fn the_gauge_tables_tokens_round_trip() {
    for set in GaugeSetId::ALL {
        assert_eq!(gauge_set_from_token(gauge_set_token(set)), Some(set));
    }
    assert_eq!(gauge_set_from_token("NOT A TABLE"), None);
}

#[test]
fn every_gauge_has_a_name_and_no_two_share_one() {
    let names: Vec<&str> = GaugeIndex::ALL.iter().map(|index| gauge_index_name(*index)).collect();
    assert_eq!(names.len(), GaugeIndex::COUNT);
    for (at, name) in names.iter().enumerate() {
        assert!(!name.is_empty(), "gauge {at} has no name");
        assert_eq!(names.iter().filter(|other| *other == name).count(), 1, "{name} is used twice");
    }
    assert_eq!(names[0], "ASSIST EASY");
    assert_eq!(names[GaugeIndex::COUNT - 1], "EXHARD GRADE");
}

#[test]
fn practice_times_read_as_minutes_seconds_and_tenths() {
    let mut panel = long_panel();
    for (ms, reads) in [(0, "0:00.0"), (1_500, "0:01.5"), (61_200, "1:01.2"), (61_250, "1:01.2"), (178_000, "2:58.0")] {
        panel.property.start_ms = ms;
        assert_eq!(panel.value_text(PracticeElement::StartTime), reads);
    }
    for (ms, reads) in [(1_000, "0:01.0"), (181_000, "3:01.0")] {
        panel.property.end_ms = ms;
        assert_eq!(panel.value_text(PracticeElement::EndTime), reads);
    }
    let hour = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, 3_600_000, None, 260.0);
    assert_eq!(hour.value_text(PracticeElement::EndTime), "60:01.0", "minutes past an hour are still minutes");
}

#[test]
fn the_last_timeline_is_read_off_the_model_in_milliseconds() {
    let mut model = Model {
        mode: Mode::BEAT_7K,
        meta: rbms_model::ModelMeta::default(),
        wavmap: Vec::new(),
        bgamap: Vec::new(),
        init_bpm: 120.0,
        timelines: Vec::new(),
        md5: String::new(),
        sha256: String::new(),
    };
    assert_eq!(last_timeline_ms(&model), 0, "a chart with no timeline has nothing to practise");
    model.timelines.push(rbms_model::TimeLine::empty(Mode::BEAT_7K.key, 0, 0.0, 120.0));
    model.timelines.push(rbms_model::TimeLine::empty(Mode::BEAT_7K.key, 90_500_000, 4.0, 120.0));
    assert_eq!(last_timeline_ms(&model), 90_500);
}

#[test]
fn a_property_read_from_a_file_is_pulled_back_inside_the_charts_own_bounds() {
    let mut p = PracticeProperty {
        start_ms: 999_999,
        end_ms: 1_000,
        gauge_type: 200,
        gauge_set: "NOT A TABLE".to_string(),
        start_gauge: 9_999,
        judge_rate: 9_999,
        freq: 9_999,
        total: 99_999.0,
        random: "NOT A SHUFFLE".to_string(),
    };
    p.sanitise(Mode::BEAT_7K, LONG_CHART_MS);
    assert_eq!(p.start_ms, round_down_to(LONG_CHART_MS - 2_000, 100));
    assert_eq!(p.end_ms, round_down_to(p.start_ms + 1_000, 100), "the end is floored on the start that was itself pulled back first");
    assert_eq!(p.end_ms, 179_000);
    assert_eq!(p.gauge_type, DEFAULT_GAUGE_TYPE);
    assert_eq!(p.gauge_set(), GaugeSetId::SevenKeys);
    assert_eq!(p.start_gauge, element_of(GaugeSetId::SevenKeys, GaugeIndex::Normal).max as i32);
    assert_eq!(p.judge_rate, JUDGE_RATE_MAX);
    assert_eq!(p.freq, FREQ_MAX);
    assert_eq!(p.total, TOTAL_MAX);
    assert_eq!(p.option(), NoteOption::Off, "an unknown shuffle falls back rather than laying the chart out at random");
}

#[test]
fn a_fresh_panel_takes_the_whole_chart_and_the_charts_own_total() {
    let panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    assert_eq!(panel.property.start_ms, 0);
    assert_eq!(panel.property.end_ms, LONG_CHART_MS + 1_000);
    assert_eq!(panel.property.total, 260.0, "an unset TOTAL takes the chart's own");
    assert_eq!(panel.property.judge_rate, JUDGE_RATE_UNMODIFIED);
    assert_eq!(panel.phase(), PracticePhase::Panel);
    assert_eq!(panel.chart_key(), "md5");
}

#[test]
fn a_saved_property_wins_over_the_chart_defaults() {
    let saved = PracticeProperty { start_ms: 30_000, end_ms: 45_000, judge_rate: 75, ..PracticeProperty::for_chart(Mode::BEAT_7K, LONG_CHART_MS) };
    let panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, Some(saved), 260.0);
    assert_eq!(panel.property.start_ms, 30_000);
    assert_eq!(panel.property.end_ms, 45_000);
    assert_eq!(panel.property.judge_rate, 75);
}

/// The rows that hold a number, and the change of the property one step of each is.
const NUMBER_ROWS: [PracticeElement; 6] = [
    PracticeElement::StartTime,
    PracticeElement::EndTime,
    PracticeElement::GaugeValue,
    PracticeElement::JudgeRank,
    PracticeElement::Total,
    PracticeElement::Freq,
];

/// The rows that hold a choice out of a ring.
const CHOICE_ROWS: [PracticeElement; 3] = [PracticeElement::GaugeType, PracticeElement::GaugeCategory, PracticeElement::Random];

/// A panel on a long seven-key chart with every number clear of both its ends.
fn long_panel() -> PracticePanel {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.start_ms = 20_000;
    panel.property.end_ms = 40_000;
    panel.property.start_gauge = 50;
    panel
}

/// What one row holds, as the number the property stores for it.
fn stored(panel: &PracticePanel, element: PracticeElement) -> f64 {
    let p = &panel.property;
    match element {
        PracticeElement::StartTime => f64::from(p.start_ms),
        PracticeElement::EndTime => f64::from(p.end_ms),
        PracticeElement::GaugeValue => f64::from(p.start_gauge),
        PracticeElement::JudgeRank => f64::from(p.judge_rate),
        PracticeElement::Total => p.total,
        PracticeElement::Freq => f64::from(p.freq),
        PracticeElement::GaugeType => f64::from(p.gauge_type),
        PracticeElement::GaugeCategory | PracticeElement::Random => f64::NAN,
    }
}

/// How much of what the property stores one step of a row is: a tenth of a second of a time, and
/// one of everything else.
fn one_step_of(element: PracticeElement) -> f64 {
    match element {
        PracticeElement::StartTime | PracticeElement::EndTime => f64::from(TIME_ROUND_MS),
        _ => 1.0,
    }
}

#[test]
fn every_row_is_a_number_or_a_choice_and_has_a_label_and_a_value() {
    let panel = long_panel();
    for element in PracticeElement::ALL {
        assert!(!element.label().is_empty(), "{element:?} has no label");
        assert!(!panel.value_text(element).is_empty(), "{element:?} reads as nothing");
        assert_eq!(panel.number(element).is_some(), NUMBER_ROWS.contains(&element), "{element:?}");
        assert_eq!(panel.number(element).is_none(), CHOICE_ROWS.contains(&element), "{element:?}");
    }
}

/// Every row that holds a number is counted between the reference's two ends of it, in the
/// smallest change the row shows: a tenth of a second of a time, one of the others.
#[test]
fn every_number_row_is_counted_in_its_smallest_change_between_its_two_ends() {
    let panel = long_panel();
    let ends = |element| panel.number(element).map(|value| (value.spec.min, value.spec.max, value.spec.per_one, value.spec.decimals, value.spec.clock));
    assert_eq!(ends(PracticeElement::StartTime), Some((0, 1_780, 10, 1, true)), "from the chart's beginning to two seconds short of its last timeline");
    assert_eq!(ends(PracticeElement::EndTime), Some((10, 1_810, 10, 1, true)), "from the shortest slice to a second past the last timeline");
    assert_eq!(ends(PracticeElement::GaugeValue), Some((1, 100, 1, 0, false)));
    assert_eq!(ends(PracticeElement::JudgeRank), Some((1, 400, 1, 0, false)));
    assert_eq!(ends(PracticeElement::Total), Some((10, 5_000, 1, 0, false)));
    assert_eq!(ends(PracticeElement::Freq), Some((50, 200, 1, 0, false)));
    for element in NUMBER_ROWS {
        let value = panel.number(element).expect("the row holds a number");
        assert_eq!(
            value.spec.shown(value.steps) * one_step_of(element) * value.spec.per_one as f64,
            stored(&panel, element),
            "{element:?} does not stand where it is stored"
        );
        let slider_ends_at = if element == PracticeElement::Total { TOTAL_SLIDER_MAX } else { value.spec.max };
        assert_eq!(value.slider_max, slider_ends_at, "{element:?}: the slider does not end where it is meant to");
        assert!(!value.locked, "{element:?} cannot be set");
    }
}

/// The slider of TOTAL and its arrows end at a thousand, where the totals charts have end, and the
/// row is stepped from where it stands once a typed figure has put it past that.
#[test]
fn total_is_stepped_as_far_as_its_slider_goes_and_from_where_it_stands_past_that() {
    let mut panel = long_panel();
    panel.set_number(PracticeElement::Total, 995);
    let value = panel.number(PracticeElement::Total).expect("TOTAL holds a number");
    assert_eq!((value.stepped(1, true), value.track().max), (TOTAL_SLIDER_MAX, TOTAL_SLIDER_MAX), "an arrow carried TOTAL past where its slider ends");
    panel.set_number(PracticeElement::Total, 2_500);
    let value = panel.number(PracticeElement::Total).expect("TOTAL holds a number");
    assert_eq!((value.stepped(1, false), value.stepped(-1, true), value.fraction()), (2_501, 2_490, 1.0));
}

/// A row carried along its slider is put where the pointer says like any row, except that START
/// TIME pushes END TIME only for as long as it is past it: END TIME is back where it stood when the
/// slider was taken hold of as soon as START TIME is clear of it again. Set by a key or a typed
/// figure, START TIME leaves END TIME where it pushed it, as the reference does.
#[test]
fn start_time_carried_along_its_slider_pushes_end_time_only_while_it_is_past_it() {
    let mut panel = long_panel();
    let taken = panel.property.clone();
    assert_eq!((taken.start_ms, taken.end_ms), (20_000, 40_000));
    assert!(panel.drag_number(PracticeElement::StartTime, 1_500, &taken));
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (150_000, 151_000), "START TIME passed END TIME instead of carrying it");
    assert!(panel.drag_number(PracticeElement::StartTime, 395, &taken));
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (39_500, 40_500), "END TIME is less than a second past START TIME");
    assert!(panel.drag_number(PracticeElement::StartTime, 390, &taken));
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (39_000, 40_000), "END TIME did not come back to where it stood");
    assert!(panel.drag_number(PracticeElement::StartTime, 200, &taken));
    assert_eq!(panel.property, taken, "a drag that came back to where it began left the property changed");
    assert!(!panel.drag_number(PracticeElement::StartTime, 200, &taken), "a row carried to the step it stands on was written");
    panel.drag_number(PracticeElement::StartTime, i64::MAX, &taken);
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (178_000, 179_000));
    panel.drag_number(PracticeElement::StartTime, i64::MIN, &taken);
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (0, 40_000));

    let mut keyed = long_panel();
    keyed.set_number(PracticeElement::StartTime, 1_500);
    keyed.set_number(PracticeElement::StartTime, 200);
    assert_eq!((keyed.property.start_ms, keyed.property.end_ms), (20_000, 151_000), "a START TIME set by a key brought END TIME back");

    for element in [PracticeElement::EndTime, PracticeElement::GaugeValue, PracticeElement::JudgeRank, PracticeElement::Total, PracticeElement::Freq] {
        let (mut dragged, mut set) = (long_panel(), long_panel());
        let steps = dragged.number(element).expect("the row holds a number").steps + 7;
        assert_eq!(dragged.drag_number(element, steps, &taken), set.set_number(element, steps), "{element:?}");
        assert_eq!(dragged.property, set.property, "{element:?} is carried somewhere else than it is set");
    }
}

/// One step of a row is the smallest change the row shows and ten steps ten of them: the coarse
/// steps of the reference's panel (five of TOTAL, five percent of the speed) are gone.
#[test]
fn one_step_of_every_number_row_is_its_smallest_change_and_ten_steps_ten_of_them() {
    for element in NUMBER_ROWS {
        let mut panel = long_panel();
        let before = stored(&panel, element);
        let value = panel.number(element).expect("the row holds a number");
        assert!(panel.set_number(element, value.stepped(1, false)), "{element:?} did not move");
        assert_eq!(stored(&panel, element), before + one_step_of(element), "{element:?}: one step is not the smallest change");
        let value = panel.number(element).expect("the row holds a number");
        assert!(panel.set_number(element, value.stepped(1, true)));
        assert_eq!(stored(&panel, element), before + 11.0 * one_step_of(element), "{element:?}: a wide step is not ten");
        let value = panel.number(element).expect("the row holds a number");
        panel.set_number(element, value.stepped(-1, true));
        let value = panel.number(element).expect("the row holds a number");
        panel.set_number(element, value.stepped(-1, false));
        assert_eq!(stored(&panel, element), before, "{element:?}: the way back is not the way there");
    }
}

#[test]
fn a_number_row_stops_at_both_its_ends() {
    for element in NUMBER_ROWS {
        let mut panel = long_panel();
        let spec = panel.number(element).expect("the row holds a number").spec;
        panel.set_number(element, i64::MAX);
        assert_eq!(panel.number(element).map(|value| value.steps), Some(spec.max), "{element:?} went past its upper end");
        let value = panel.number(element).expect("the row holds a number");
        assert!(!panel.set_number(element, value.stepped(1, true)), "{element:?} moved past its upper end");
    }
    for element in [PracticeElement::StartTime, PracticeElement::GaugeValue, PracticeElement::JudgeRank, PracticeElement::Total, PracticeElement::Freq] {
        let mut panel = long_panel();
        let spec = panel.number(element).expect("the row holds a number").spec;
        panel.set_number(element, i64::MIN);
        assert_eq!(panel.number(element).map(|value| value.steps), Some(spec.min), "{element:?} went past its lower end");
    }
}

/// END TIME is held a second above START TIME however far down it is put, and START TIME carries
/// END TIME up with it rather than passing it.
#[test]
fn the_two_times_keep_a_second_between_them_wherever_they_are_put() {
    let mut panel = long_panel();
    assert!(panel.set_number(PracticeElement::EndTime, 0));
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (20_000, 21_000), "END TIME did not stop a second above START TIME");
    let end = panel.number(PracticeElement::EndTime).expect("END TIME holds a number");
    assert_eq!((end.steps, end.spec.min), (210, 10), "the row is counted from the earliest any slice ends, and stands above that");
    assert!(!panel.set_number(PracticeElement::EndTime, end.stepped(-1, false)), "a step took END TIME below START TIME");
    assert!(!panel.set_number(PracticeElement::EndTime, end.stepped(-1, true)));

    assert!(panel.set_number(PracticeElement::StartTime, 300));
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (30_000, 31_000), "START TIME passed END TIME instead of carrying it");
    assert!(panel.set_number(PracticeElement::StartTime, 100));
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (10_000, 31_000), "START TIME brought END TIME back down with it");

    panel.set_number(PracticeElement::StartTime, i64::MAX);
    assert_eq!((panel.property.start_ms, panel.property.end_ms), (178_000, 179_000), "the last place a slice starts leaves a second of it");
    panel.set_number(PracticeElement::EndTime, i64::MAX);
    assert_eq!(panel.property.end_ms, 181_000);
}

/// The top of GAUGE VALUE is the top of the gauge that is selected on the table that is selected,
/// so it moves when either does.
#[test]
fn the_gauge_value_ends_where_the_selected_gauge_does() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::POPN_9K, LONG_CHART_MS, None, 260.0);
    let top = |panel: &PracticePanel| panel.number(PracticeElement::GaugeValue).map(|value| value.spec.max);
    let groove = element_of(GaugeSetId::Pms, GaugeIndex::Normal).max as i64;
    let survival = element_of(GaugeSetId::Pms, GaugeIndex::Hard).max as i64;
    assert!(groove > survival, "the pop'n table tops its groove gauge above its survival gauge");
    assert_eq!(top(&panel), Some(groove));
    panel.set_number(PracticeElement::GaugeValue, i64::MAX);
    assert_eq!(i64::from(panel.property.start_gauge), groove);
    assert!(panel.cycle(PracticeElement::GaugeType, true));
    assert_eq!(panel.property.gauge_index(), GaugeIndex::Hard);
    assert_eq!((top(&panel), i64::from(panel.property.start_gauge)), (Some(survival), survival), "the starting amount was left above the new gauge's top");
    panel.set_number(PracticeElement::GaugeValue, i64::MAX);
    assert_eq!(i64::from(panel.property.start_gauge), survival);
}

/// A row asked for the step it stands on is not written, so a number finer than the row's steps
/// is kept until the row is really moved.
#[test]
fn a_row_put_on_the_step_it_stands_on_keeps_what_it_holds() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.5);
    assert_eq!(panel.property.total, 260.5);
    let total = panel.number(PracticeElement::Total).expect("TOTAL holds a number");
    assert_eq!((total.steps, panel.value_text(PracticeElement::Total).as_str()), (260, "260"));
    assert!(!panel.set_number(PracticeElement::Total, 260));
    assert_eq!(panel.property.total, 260.5, "a TOTAL put where it stood lost its fraction");
    assert!(panel.set_number(PracticeElement::Total, 261));
    assert_eq!(panel.property.total, 261.0);

    panel.property.start_ms = 20_050;
    assert!(!panel.set_number(PracticeElement::StartTime, 200));
    assert_eq!(panel.property.start_ms, 20_050);
    assert!(panel.set_number(PracticeElement::StartTime, 201));
    assert_eq!(panel.property.start_ms, 20_100);
}

/// The key that resets a row puts it back on what a fresh panel on this chart holds: the whole
/// chart, the gauge amount and the judge width and speed a chart is first practised under, and the
/// chart's own TOTAL to the fraction.
#[test]
fn a_reset_row_goes_back_to_what_a_fresh_panel_holds() {
    let fresh = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.5);
    for element in NUMBER_ROWS {
        let mut panel = fresh.clone();
        assert_eq!(panel.number(element).map(|value| value.shipped), fresh.number(element).map(|value| value.steps), "{element:?}");
        assert!(!panel.reset_number(element), "{element:?}: a fresh row was moved by being reset");
        let value = panel.number(element).expect("the row holds a number");
        let away = if value.steps == value.spec.max { value.steps - 30 } else { value.steps + 30 };
        assert!(panel.set_number(element, away));
        assert!(panel.reset_number(element), "{element:?} was not put back");
        assert_eq!(panel.property, fresh.property, "{element:?}: the row is not back where a fresh panel has it, or another row moved");
    }
    for element in CHOICE_ROWS {
        let mut panel = fresh.clone();
        panel.cycle(element, true);
        let moved = panel.property.clone();
        assert!(!panel.reset_number(element), "{element:?} holds no number to put back");
        assert_eq!(panel.property, moved);
    }
}

/// A row that holds a choice is stepped round its ring and takes no number; a row that holds a
/// number is not stepped round anything.
#[test]
fn a_choice_is_cycled_and_a_number_is_set_and_neither_takes_the_other() {
    for element in CHOICE_ROWS {
        let mut panel = long_panel();
        let before = panel.value_text(element);
        assert!(!panel.set_number(element, 3), "{element:?} took a number");
        assert_eq!(panel.value_text(element), before);
        assert!(panel.cycle(element, true));
        assert_ne!(panel.value_text(element), before, "{element:?} did not move");
        assert!(panel.cycle(element, false));
        assert_eq!(panel.value_text(element), before, "{element:?}: the way back is not the way there");
    }
    for element in NUMBER_ROWS {
        let mut panel = long_panel();
        let before = panel.property.clone();
        assert!(!panel.cycle(element, true), "{element:?} was cycled");
        assert_eq!(panel.property, before);
    }
}

#[test]
fn editing_a_row_leaves_every_other_row_alone() {
    for element in [PracticeElement::GaugeValue, PracticeElement::JudgeRank, PracticeElement::Total, PracticeElement::Freq, PracticeElement::EndTime] {
        let mut panel = long_panel();
        let before = panel.clone();
        let value = panel.number(element).expect("the row holds a number");
        assert!(panel.set_number(element, value.steps + 1));
        for other in PracticeElement::ALL.into_iter().filter(|other| *other != element) {
            assert_eq!(panel.value_text(other), before.value_text(other), "{element:?} moved {other:?}");
        }
    }
}

/// What a row is set to is what a slice is cut from and played under: the two times in
/// microseconds, and the gauge amount, the judge width, the speed and TOTAL as they stand.
#[test]
fn what_the_rows_are_set_to_is_what_a_slice_is_started_with() {
    let mut panel = long_panel();
    for (element, steps) in [
        (PracticeElement::StartTime, 123),
        (PracticeElement::EndTime, 456),
        (PracticeElement::GaugeValue, 40),
        (PracticeElement::JudgeRank, 83),
        (PracticeElement::Total, 317),
        (PracticeElement::Freq, 73),
    ] {
        assert!(panel.set_number(element, steps), "{element:?}");
    }
    let session = panel.start();
    assert_eq!((session.start_us, session.end_us), (12_300_000, 45_600_000));
    assert_eq!(session.start_gauge, 40.0);
    assert_eq!(session.judge_rate_percent, 83);
    assert_eq!(session.total, Some(317.0));
    assert_eq!(session.freq_percent, 73);
    assert_eq!(PracticeClock::from_session(&session).chart_time_us(1_000_000), 12_300_000 + 730_000, "the speed did not reach the running clock");
}

#[test]
fn starting_a_slice_hands_over_the_range_in_microseconds_with_the_gauge_locked() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.start_ms = 12_300;
    panel.property.end_ms = 45_600;
    panel.property.gauge_type = 3;
    panel.property.start_gauge = 40;
    panel.property.judge_rate = 80;
    panel.property.random = NoteOption::Mirror.label().to_string();
    let session = panel.start();
    assert_eq!(panel.phase(), PracticePhase::Playing);
    assert_eq!(session.start_us, 12_300_000);
    assert_eq!(session.end_us, 45_600_000);
    assert_eq!(session.gauge, GaugeIndex::Hard);
    assert_eq!(session.gauge_set, GaugeSetId::SevenKeys);
    assert_eq!(session.start_gauge, 40.0);
    assert_eq!(session.judge_rate_percent, 80);
    assert_eq!(session.freq_percent, FREQ_UNMODIFIED);
    assert_eq!(session.total, Some(260.0));
    assert_eq!(session.option, NoteOption::Mirror);
    assert!(session.gauge_locked, "an emptied gauge must not cut a practice slice short");
}

/// The gauge lock is what turns the one auto-shift that ends a run on an emptied gauge into one
/// that plays on; every other mode already plays on and is the player's to keep.
#[test]
fn a_locked_gauge_plays_on_where_the_setting_would_fail_the_run() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    let mut session = panel.start();
    for configured in GaugeAutoShift::ALL {
        let expected = match configured {
            GaugeAutoShift::None => GaugeAutoShift::Continue,
            other => other,
        };
        assert_eq!(session.gauge_shift(configured), expected, "{configured:?}");
    }
    session.gauge_locked = false;
    assert_eq!(session.gauge_shift(GaugeAutoShift::None), GaugeAutoShift::None, "an unlocked gauge is the player's setting");
}

#[test]
fn an_unset_total_is_handed_over_as_the_charts_own() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.total = TOTAL_FROM_CHART;
    assert_eq!(panel.start().total, None, "zero means the chart's own TOTAL, not a TOTAL of zero");
}

#[test]
fn the_slice_ends_at_its_own_end_rather_than_at_the_last_note() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.start_ms = 0;
    panel.property.end_ms = 20_000;
    let session = panel.start();
    assert!(!session.is_past_end(19_999_999));
    assert!(session.is_past_end(20_000_000));
    assert!(session.is_past_end(20_000_001));
}

#[test]
fn a_slice_hands_the_frequency_to_the_running_clock() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.freq = FREQ_MAX;
    let session = panel.start();
    assert_eq!(session.freq_percent, FREQ_MAX);
}

#[test]
fn the_practice_clock_maps_engine_time_to_the_absolute_chart_slice() {
    for freq_percent in [FREQ_MIN, FREQ_UNMODIFIED, FREQ_MAX] {
        let clock = PracticeClock::new(30_000_000, freq_percent);
        assert_eq!(clock.chart_time_us(0), 30_000_000, "{freq_percent}% must start at the selected chart time");
        assert_eq!(clock.chart_time_us(10_000_000), 30_000_000 + 10_000_000 * i64::from(freq_percent) / i64::from(FREQ_UNMODIFIED));
        let engine_us = 12_345_678;
        let round_trip_us = clock.engine_time_us(clock.chart_time_us(engine_us));
        assert!((round_trip_us - engine_us).abs() <= 1, "{freq_percent}% round-trip was {round_trip_us} for {engine_us}");
    }
}

#[test]
fn the_normal_practice_clock_is_the_existing_song_axis() {
    let clock = PracticeClock::normal();
    for time_us in [-1_000_000, 0, 1_500_000] {
        assert_eq!(clock.chart_time_us(time_us), time_us);
        assert_eq!(clock.engine_time_us(time_us), time_us);
    }
}

#[test]
fn the_practice_clock_saturates_at_the_limits_of_the_song_axis() {
    let clock = PracticeClock::new(0, FREQ_MAX);
    assert_eq!(clock.chart_time_us(i64::MAX), i64::MAX);
    assert_eq!(clock.chart_time_us(i64::MIN), i64::MIN);
    let slow_clock = PracticeClock::new(0, FREQ_MIN);
    assert_eq!(slow_clock.engine_time_us(i64::MAX), i64::MAX);
    assert_eq!(slow_clock.engine_time_us(i64::MIN), i64::MIN);
}

#[test]
fn finishing_a_slice_returns_to_the_panel_with_the_property_untouched() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.start_ms = 30_000;
    panel.property.end_ms = 50_000;
    let before = panel.property.clone();
    panel.start();
    panel.finish();
    assert_eq!(panel.phase(), PracticePhase::Panel);
    assert_eq!(panel.property, before, "however a slice ended, the next attempt starts from the same values");
}

#[test]
fn a_practice_run_is_never_recorded_and_never_submitted() {
    assert_eq!(practice_block_reason(true), Some(PRACTICE_BLOCK_REASON));
    assert_eq!(practice_block_reason(true), Some("practice"));
    assert_eq!(practice_block_reason(false), None, "an ordinary run is not blocked by this rule");
}

#[test]
fn the_book_remembers_a_property_per_chart_and_survives_a_round_trip() {
    let dir = temp_dir("book");
    let path = practice_path(&dir.join("settings.ron"));
    assert_eq!(path.file_name().and_then(|n| n.to_str()), Some(PRACTICE_FILE));

    let mut book = PracticeBook::load(&path);
    assert!(book.is_empty(), "a fresh install has practised nothing");
    let mut first = PracticeProperty::for_chart(Mode::BEAT_7K, LONG_CHART_MS);
    first.start_ms = 30_000;
    book.put("md5-a", first.clone());
    book.put("md5-b", PracticeProperty::for_chart(Mode::POPN_9K, 60_000));
    book.save(&path);

    let read = PracticeBook::load(&path);
    assert_eq!(read.len(), 2);
    assert_eq!(read.get("md5-a"), Some(first));
    assert_eq!(read.get("md5-c"), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_book_that_cannot_be_read_starts_empty_rather_than_stopping_practice() {
    let dir = temp_dir("corrupt");
    let path = practice_path(&dir.join("settings.ron"));
    std::fs::write(&path, "this is not ron").expect("a file to be written");
    assert!(PracticeBook::load(&path).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_book_written_by_an_older_build_keeps_the_fields_it_has() {
    let text = "(charts: {\"md5-a\": (start_ms: 4000, end_ms: 9000)})";
    let book: PracticeBook = ron::from_str(text).expect("missing fields fall back to their defaults");
    let property = book.get("md5-a").expect("the chart is in the book");
    assert_eq!(property.start_ms, 4_000);
    assert_eq!(property.end_ms, 9_000);
    assert_eq!(property.judge_rate, JUDGE_RATE_UNMODIFIED, "a field the older build did not write takes its default");
    assert_eq!(property.gauge_type, DEFAULT_GAUGE_TYPE);
}
