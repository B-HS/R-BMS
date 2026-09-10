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
fn the_time_step_is_the_plain_one_unless_turbo_is_held() {
    assert_eq!(time_step_ms(false, false), 100);
    assert_eq!(time_step_ms(false, true), 100, "an analog control alone changes nothing");
    assert_eq!(time_step_ms(true, false), 2_500);
    assert_eq!(time_step_ms(true, true), 1_000, "turbo on an analog control moves less per press");
}

#[test]
fn start_time_stops_two_seconds_short_of_the_last_timeline_rounded_down() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = LONG_CHART_MS;
    clamp_start(&mut p, LONG_CHART_MS + 50, false, false, true);
    assert_eq!(p.start_ms, round_down_to(LONG_CHART_MS + 50 - 2_000, 100), "the ceiling is the rounded-down last timeline less two seconds");
    assert_eq!(p.start_ms, 178_000);
}

#[test]
fn start_time_stops_at_zero_going_down() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 50;
    clamp_start(&mut p, LONG_CHART_MS, false, false, false);
    assert_eq!(p.start_ms, 0, "the floor is zero, not a negative step");
}

#[test]
fn raising_start_time_pushes_end_time_up_with_no_rounding() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 2_550;
    p.end_ms = 3_000;
    clamp_start(&mut p, LONG_CHART_MS, false, false, true);
    assert_eq!(p.start_ms, 2_650);
    assert_eq!(p.end_ms, 3_650, "the push is a plain max of start plus one second, with no rounding of its own");
}

#[test]
fn lowering_start_time_leaves_end_time_where_it_was() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 5_000;
    p.end_ms = 5_100;
    clamp_start(&mut p, LONG_CHART_MS, false, false, false);
    assert_eq!(p.start_ms, 4_900);
    assert_eq!(p.end_ms, 5_100, "only an increase drags the end along");
}

#[test]
fn start_time_takes_the_turbo_and_analog_steps() {
    let mut turbo = property(LONG_CHART_MS);
    clamp_start(&mut turbo, LONG_CHART_MS, true, false, true);
    assert_eq!(turbo.start_ms, 2_500);

    let mut analog = property(LONG_CHART_MS);
    clamp_start(&mut analog, LONG_CHART_MS, true, true, true);
    assert_eq!(analog.start_ms, 1_000);

    let mut plain = property(LONG_CHART_MS);
    clamp_start(&mut plain, LONG_CHART_MS, false, false, true);
    assert_eq!(plain.start_ms, 100);
}

#[test]
fn end_time_reaches_one_second_past_the_last_timeline_rounded_down() {
    let mut p = property(LONG_CHART_MS);
    p.end_ms = LONG_CHART_MS + 1_000;
    clamp_end(&mut p, LONG_CHART_MS + 50, true, false, true);
    assert_eq!(p.end_ms, round_down_to(LONG_CHART_MS + 50 + 1_000, 100));
    assert_eq!(p.end_ms, 181_000);
}

#[test]
fn end_time_floors_on_the_rounded_down_start_plus_one_second() {
    let mut p = property(LONG_CHART_MS);
    p.start_ms = 2_550;
    p.end_ms = 3_600;
    clamp_end(&mut p, LONG_CHART_MS, false, false, false);
    assert_eq!(p.end_ms, 3_500, "one plain step down is still above the floor");
    clamp_end(&mut p, LONG_CHART_MS, true, false, false);
    assert_eq!(p.end_ms, round_down_to(2_550 + 1_000, 100), "the floor rounds down, unlike the push from START TIME");
    assert_eq!(p.end_ms, 3_500);
}

#[test]
fn end_time_takes_the_turbo_and_analog_steps() {
    let base = 60_000;
    for (turbo, analog, want) in [(false, false, base + 100), (true, false, base + 2_500), (true, true, base + 1_000)] {
        let mut p = property(LONG_CHART_MS);
        p.end_ms = base;
        clamp_end(&mut p, LONG_CHART_MS, turbo, analog, true);
        assert_eq!(p.end_ms, want, "turbo={turbo} analog={analog}");
    }
}

#[test]
fn a_chart_shorter_than_the_start_time_tail_gives_a_ceiling_below_zero() {
    let mut p = property(500);
    p.start_ms = 0;
    clamp_start(&mut p, 500, false, false, true);
    assert_eq!(p.start_ms, round_down_to(500 - 2_000, 100), "there is nowhere to start such a chart, and the reference says so with a negative ceiling");
    assert_eq!(p.start_ms, -1_500);
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
    p.start_gauge = max;
    adjust_gauge_value(&mut p, false, true);
    assert_eq!(p.start_gauge, max, "the gauge's own maximum is the ceiling");
    p.start_gauge = GAUGE_VALUE_MIN;
    adjust_gauge_value(&mut p, false, false);
    assert_eq!(p.start_gauge, GAUGE_VALUE_MIN, "and one is the floor");
}

#[test]
fn turbo_from_the_bottom_of_the_gauge_value_lands_on_one_whole_step() {
    let mut p = property(LONG_CHART_MS);
    p.start_gauge = GAUGE_VALUE_MIN;
    adjust_gauge_value(&mut p, true, true);
    assert_eq!(p.start_gauge, GAUGE_VALUE_STEP_TURBO, "the bottom jumps to the step itself rather than to one past it");
    adjust_gauge_value(&mut p, true, true);
    assert_eq!(p.start_gauge, GAUGE_VALUE_STEP_TURBO * 2, "and moves by whole steps from there");
}

#[test]
fn the_judge_rate_holds_its_bounds_and_jumps_from_the_bottom_on_turbo() {
    let mut p = property(LONG_CHART_MS);
    p.judge_rate = JUDGE_RATE_MAX;
    adjust_judge_rate(&mut p, true, true);
    assert_eq!(p.judge_rate, JUDGE_RATE_MAX);
    p.judge_rate = JUDGE_RATE_MIN;
    adjust_judge_rate(&mut p, false, false);
    assert_eq!(p.judge_rate, JUDGE_RATE_MIN);
    adjust_judge_rate(&mut p, true, true);
    assert_eq!(p.judge_rate, JUDGE_RATE_STEP_TURBO);
    adjust_judge_rate(&mut p, false, true);
    assert_eq!(p.judge_rate, JUDGE_RATE_STEP_TURBO + JUDGE_RATE_STEP);
}

#[test]
fn total_and_frequency_take_their_own_analog_steps() {
    let mut p = property(LONG_CHART_MS);
    p.total = 300.0;
    adjust_total(&mut p, false, false, true);
    assert_eq!(p.total, 305.0);
    adjust_total(&mut p, true, false, true);
    assert_eq!(p.total, 330.0);
    adjust_total(&mut p, false, true, true);
    assert_eq!(p.total, 331.0);
    adjust_total(&mut p, true, true, true);
    assert_eq!(p.total, 351.0);

    p.freq = FREQ_UNMODIFIED;
    adjust_freq(&mut p, false, false, true);
    assert_eq!(p.freq, 105);
    adjust_freq(&mut p, true, false, true);
    assert_eq!(p.freq, 130);
    adjust_freq(&mut p, false, true, true);
    assert_eq!(p.freq, 131);
    adjust_freq(&mut p, true, true, true);
    assert_eq!(p.freq, 141);
}

#[test]
fn total_and_frequency_hold_their_bounds() {
    let mut p = property(LONG_CHART_MS);
    p.total = TOTAL_MIN;
    adjust_total(&mut p, true, false, false);
    assert_eq!(p.total, TOTAL_MIN);
    p.total = TOTAL_MAX;
    adjust_total(&mut p, true, false, true);
    assert_eq!(p.total, TOTAL_MAX);

    p.freq = FREQ_MIN;
    adjust_freq(&mut p, true, false, false);
    assert_eq!(p.freq, FREQ_MIN);
    p.freq = FREQ_MAX;
    adjust_freq(&mut p, true, false, true);
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
    assert_eq!(format_practice_time(0), " 0:00.0");
    assert_eq!(format_practice_time(1_500), " 0:01.5");
    assert_eq!(format_practice_time(61_200), " 1:01.2");
    assert_eq!(format_practice_time(600_000), "10:00.0");
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
    assert_eq!(panel.focused(), PracticeElement::StartTime);
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

#[test]
fn the_cursor_rings_round_the_rows_both_ways() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.move_cursor(false);
    assert_eq!(panel.focused(), PracticeElement::Random, "above the first row is the last");
    panel.move_cursor(true);
    assert_eq!(panel.focused(), PracticeElement::StartTime);
    for _ in 0..PracticeElement::ALL.len() {
        panel.move_cursor(true);
    }
    assert_eq!(panel.focused(), PracticeElement::StartTime, "a whole turn comes back to where it started");
}

#[test]
fn every_row_has_a_label_a_value_and_an_edit_that_changes_something() {
    for (at, element) in PracticeElement::ALL.iter().enumerate() {
        assert!(!element.label().is_empty(), "{element:?} has no label");
        let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
        panel.property.start_ms = 20_000;
        panel.property.end_ms = 40_000;
        while panel.focused() != *element {
            panel.move_cursor(true);
        }
        let before = panel.value_text(*element);
        panel.adjust(true, false, false);
        assert_ne!(panel.value_text(*element), before, "row {at} ({element:?}) did not move");
    }
}

#[test]
fn editing_a_row_leaves_every_other_row_alone() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.start_ms = 20_000;
    panel.property.end_ms = 40_000;
    let before = panel.property.clone();
    while panel.focused() != PracticeElement::JudgeRank {
        panel.move_cursor(true);
    }
    panel.adjust(true, false, false);
    assert_eq!(panel.property.judge_rate, before.judge_rate + JUDGE_RATE_STEP);
    assert_eq!(PracticeProperty { judge_rate: before.judge_rate, ..panel.property.clone() }, before);
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
    assert_eq!(session.total, Some(260.0));
    assert_eq!(session.option, NoteOption::Mirror);
    assert!(session.gauge_locked, "an emptied gauge must not cut a practice slice short");
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
fn a_slice_always_plays_at_the_unaltered_speed_whatever_the_panel_says() {
    let mut panel = PracticePanel::new("md5".to_string(), Mode::BEAT_7K, LONG_CHART_MS, None, 260.0);
    panel.property.freq = FREQ_MAX;
    let session = panel.start();
    assert_eq!(session.freq_percent(), FREQ_UNMODIFIED, "the mixer resamples nothing, so the chosen speed is not applied yet");
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
