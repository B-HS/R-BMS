//! The gauge's arithmetic, checked on the numbers alone and without a renderer.

use super::*;

/// The seven-key groove gauge the reference clears at eighty of a hundred, which never falls below
/// two.
const GROOVE: GaugeScale = GaugeScale::new(2.0, 100.0, 80.0);

/// A gauge that clears at nothing and empties to nothing, as the hard gauges do.
const SURVIVAL: GaugeScale = GaugeScale::new(0.0, 100.0, 0.0);

/// The nine-key gauge, which runs to a hundred and twenty and clears at eighty-five.
const POPN: GaugeScale = GaugeScale::new(2.0, 120.0, 85.0);

/// Which node fills each cell when `count` nodes are spread over the table.
fn table(count: usize) -> Option<[Option<usize>; GAUGE_SLOTS]> {
    let spread = spread_of(count)?;
    let mut slots = [None; GAUGE_SLOTS];
    for node in 0..count {
        for slot in spread.cells(node) {
            slots[slot] = Some(node);
        }
    }
    Some(slots)
}

/// The node a table put in the cell of gauge column `column` and state `state`.
fn node_at(slots: &[Option<usize>; GAUGE_SLOTS], column: usize, state: usize) -> Option<usize> {
    slots[column * SLOTS_PER_GAUGE + state]
}

#[test]
fn four_nodes_fill_every_gauge_alike_and_lend_the_lit_pair_to_the_leading_part() {
    let slots = table(4).expect("four nodes are a shape the table is spread over");
    for column in 0..6 {
        let row: Vec<_> = (0..SLOTS_PER_GAUGE).map(|state| node_at(&slots, column, state)).collect();
        assert_eq!(row, vec![Some(0), Some(1), Some(2), Some(3), Some(0), Some(1)], "gauge column {column}");
    }
}

#[test]
fn eight_nodes_give_normal_and_hard_the_first_four_and_every_other_gauge_the_rest() {
    let slots = table(8).expect("eight nodes are a shape the table is spread over");
    for column in 0..6 {
        let first = if column == 2 || column == 3 { 0 } else { 4 };
        let row: Vec<_> = (0..SLOTS_PER_GAUGE).map(|state| node_at(&slots, column, state)).collect();
        assert_eq!(row, vec![Some(first), Some(first + 1), Some(first + 2), Some(first + 3), Some(first), Some(first + 1)], "gauge column {column}");
    }
}

#[test]
fn twelve_nodes_add_a_leading_pair_of_their_own_to_each_of_the_two_groups() {
    let slots = table(12).expect("twelve nodes are a shape the table is spread over");
    for column in 0..6 {
        let (first, leading) = if column == 2 || column == 3 { (0, 8) } else { (4, 10) };
        let row: Vec<_> = (0..SLOTS_PER_GAUGE).map(|state| node_at(&slots, column, state)).collect();
        assert_eq!(row, vec![Some(first), Some(first + 1), Some(first + 2), Some(first + 3), Some(leading), Some(leading + 1)], "gauge column {column}");
    }
}

#[test]
fn thirty_six_nodes_fill_the_table_one_to_one_and_no_other_count_fills_it_at_all() {
    let slots = table(GAUGE_SLOTS).expect("thirty-six nodes are a shape the table is spread over");
    assert!(slots.iter().enumerate().all(|(slot, node)| *node == Some(slot)));
    for count in [0, 1, 3, 5, 6, 9, 16, 24, 35, 37] {
        assert!(spread_of(count).is_none(), "{count} nodes");
    }
}

#[test]
fn each_gauge_type_reads_six_cells_of_its_own_and_the_course_gauges_borrow_three() {
    let firsts: Vec<_> = (0..GAUGE_TYPES).map(first_slot).collect();
    assert_eq!(firsts, vec![0, 6, 12, 18, 24, 30, 18, 24, 30], "class reads hard, ex-class ex-hard and ex-hard-class hazard");
}

#[test]
fn a_part_reads_the_lit_the_unlit_or_the_leading_cell_by_where_the_gauge_stands() {
    let cells = |lit: i32, animation: i32| (1..=6).map(|part| part_slot(GaugeAnimation::Random, part, lit, animation)).collect::<Vec<_>>();
    assert_eq!(cells(4, 0), vec![SLOT_LIT, SLOT_LIT, SLOT_LIT, SLOT_LEADING, SLOT_UNLIT, SLOT_UNLIT]);
    assert_eq!(cells(4, 2), vec![SLOT_LIT, SLOT_UNLIT, SLOT_UNLIT, SLOT_LEADING, SLOT_UNLIT, SLOT_UNLIT], "two parts behind the leading one go dark");
    assert_eq!(
        cells(4, 9),
        vec![SLOT_UNLIT, SLOT_UNLIT, SLOT_UNLIT, SLOT_LEADING, SLOT_UNLIT, SLOT_UNLIT],
        "and more than there are leaves only the leading one"
    );
    assert_eq!(cells(0, 0), vec![SLOT_UNLIT; 6], "an empty gauge has no leading part");
    for animation in [GaugeAnimation::Increase, GaugeAnimation::Decrease] {
        assert_eq!(part_slot(animation, 3, 4, 1), SLOT_UNLIT, "{animation:?} darkens the same stretch");
        assert_eq!(part_slot(animation, 2, 4, 1), SLOT_LIT);
    }
}

#[test]
fn a_flickering_gauge_darkens_nothing_and_has_no_leading_cell_of_its_own() {
    let cells: Vec<_> = (1..=6).map(|part| part_slot(GaugeAnimation::Flickering, part, 4, 3)).collect();
    assert_eq!(cells, vec![SLOT_LIT, SLOT_LIT, SLOT_LIT, SLOT_LIT, SLOT_UNLIT, SLOT_UNLIT], "the leading part is drawn lit and faded over afterwards");
}

#[test]
fn a_part_is_below_the_clear_line_when_the_value_its_far_edge_stands_for_is() {
    let below: Vec<_> = (1..=10).map(|part| below_border(part, 10, GROOVE)).collect();
    assert_eq!(below, vec![true, true, true, true, true, true, true, false, false, false], "the part that ends on eighty is not below eighty");
    assert!((1..=50).all(|part| !below_border(part, 50, SURVIVAL)), "a gauge that clears at nothing has no part below its line");
    assert_eq!((1..=50).filter(|part| below_border(*part, 50, POPN)).count(), 35, "part 36 of 50 stands for 86.4 of 120");
}

#[test]
fn the_lit_parts_are_the_value_over_the_scale_truncated_and_never_none_for_a_gauge_that_holds_anything() {
    assert_eq!(lit_parts(74.0, 50, 100.0), 37);
    assert_eq!(lit_parts(100.0, 50, 100.0), 50);
    assert_eq!(lit_parts(99.9, 50, 100.0), 49, "a part is lit only once the gauge has all of it");
    assert_eq!(lit_parts(0.5, 50, 100.0), 1, "a sliver still lights one part");
    assert_eq!(lit_parts(0.0, 50, 100.0), 0);
    assert_eq!(lit_parts(-3.0, 50, 100.0), 0);
    assert_eq!(lit_parts(120.0, 50, 120.0), 50);
    assert_eq!(lit_parts(85.0, 50, 120.0), 35);
}

#[test]
fn a_score_screens_gauge_rises_from_its_least_to_the_runs_last_value_between_the_two_times() {
    let at = |time_ms: i64| filling_value(90.0, GROOVE, time_ms, 0, 500);
    assert_eq!(at(0), 2.0, "the pace gives nothing yet, and the gauge never reads below its least");
    assert_eq!(at(5), 2.0, "a hundredth of the way is one, still under the least");
    assert_eq!(at(125), 25.0);
    assert_eq!(at(250), 50.0);
    assert_eq!(at(449), 89.8);
    assert_eq!(at(450), 90.0, "it rises at the pace that fills the whole gauge, so it reaches ninety a tenth early");
    assert_eq!(at(499), 90.0);
    assert_eq!(at(500), 90.0);
    assert_eq!(at(60_000), 90.0);
}

#[test]
fn before_its_start_time_a_score_screens_gauge_stands_at_its_least() {
    let at = |time_ms: i64| filling_value(90.0, GROOVE, time_ms, 1_000, 3_000);
    assert_eq!(at(0), 2.0);
    assert_eq!(at(999), 2.0);
    assert_eq!(at(1_000), 2.0);
    assert_eq!(at(2_000), 50.0);
    assert_eq!(at(3_000), 90.0);
    assert_eq!(filling_value(0.0, SURVIVAL, 250, 0, 500), 0.0, "a run that ended empty never rises");
    assert_eq!(filling_value(40.0, SURVIVAL, 300, 300, 300), 40.0, "a record with no time to fill in shows the value the moment it starts");
    assert_eq!(filling_value(40.0, SURVIVAL, 299, 300, 300), 0.0);
}

#[test]
fn the_three_darkening_animations_step_once_an_interval_and_not_at_the_first_frame() {
    let mut motion = GaugeMotion::new(50);
    motion.step(GaugeAnimation::Decrease, 3, 33, 0);
    assert_eq!(motion.animation, 0, "nothing is later than the time the state starts at");
    let seen: Vec<_> = [1, 20, 34, 35, 68, 200, 201, 234, 235, 269]
        .into_iter()
        .map(|time_ms| {
            motion.step(GaugeAnimation::Decrease, 3, 33, time_ms);
            motion.animation
        })
        .collect();
    assert_eq!(seen, vec![1, 1, 1, 2, 2, 3, 3, 0, 0, 1], "one step per frame that finds the interval over, however late the frame is");

    let mut motion = GaugeMotion::new(50);
    let seen: Vec<_> = (1..=5)
        .map(|step| {
            motion.step(GaugeAnimation::Increase, 3, 33, step * 100);
            motion.animation
        })
        .collect();
    assert_eq!(seen, vec![3, 2, 1, 0, 3], "the reference's increase walks the dark stretch down from the range");
}

#[test]
fn the_random_animation_stays_within_its_range_and_is_the_same_every_run() {
    let run = || {
        let mut motion = GaugeMotion::new(50);
        (1..=200)
            .map(|step| {
                motion.step(GaugeAnimation::Random, 3, 33, step * 34);
                motion.animation
            })
            .collect::<Vec<_>>()
    };
    let seen = run();
    assert!(seen.iter().all(|animation| (0..=3).contains(animation)), "{seen:?}");
    assert!((0..=3).all(|value| seen.contains(&value)), "two hundred draws reach every length");
    assert_eq!(seen, run());

    let mut held = GaugeMotion::new(50);
    held.step(GaugeAnimation::Random, 3, 1_000, 1);
    let first = held.animation;
    held.step(GaugeAnimation::Random, 3, 1_000, 900);
    assert_eq!(held.animation, first, "a length is kept until its interval is over");
}

#[test]
fn a_range_or_a_cycle_the_reference_would_divide_by_zero_for_leaves_the_animation_at_rest() {
    let mut motion = GaugeMotion::new(50);
    motion.step(GaugeAnimation::Increase, -1, 33, 100);
    assert_eq!(motion.animation, 0);
    motion.step(GaugeAnimation::Decrease, -1, 33, 200);
    assert_eq!(motion.animation, 0);
    motion.step(GaugeAnimation::Random, -1, 33, 300);
    assert_eq!(motion.animation, 0);
    motion.step(GaugeAnimation::Flickering, 3, 0, 400);
    assert_eq!(motion.animation, 0);
}

#[test]
fn the_flicker_is_how_far_the_scene_is_into_its_cycle() {
    let mut motion = GaugeMotion::new(50);
    for (time_ms, animation) in [(0, 0), (25, 25), (99, 99), (100, 0), (1_234, 34)] {
        motion.step(GaugeAnimation::Flickering, 3, 100, time_ms);
        assert_eq!(motion.animation, animation, "at {time_ms} ms");
    }
}

#[test]
fn the_leading_part_of_a_flickering_gauge_fades_up_over_half_a_cycle_and_down_over_the_other() {
    assert_eq!(flicker_share(0, 100), 0.0);
    assert_eq!(flicker_share(49, 100), 1.0, "the last millisecond of the first half is full");
    assert_eq!(flicker_share(50, 100), 1.0, "and so is the first of the second");
    assert_eq!(flicker_share(99, 100), 0.0);
    assert_eq!(flicker_share(24, 100), 24.0 / 49.0);
    assert_eq!(flicker_share(75, 100), 24.0 / 49.0);
    assert!(flicker_share(16, 33) > 1.0, "an odd cycle passes full for a millisecond, as the reference's quotient does");
    assert!(flicker_share(0, 2).is_nan(), "and a cycle of two has nothing to divide by");
}

#[test]
fn a_faded_opacity_is_truncated_held_within_a_channel_and_nothing_for_no_number_at_all() {
    assert_eq!(faded_alpha(255, 0.5), 127);
    assert_eq!(faded_alpha(255, 24.0 / 49.0), 124);
    assert_eq!(faded_alpha(200, 1.0), 200);
    assert_eq!(faded_alpha(128, flicker_share(16, 33)), 132, "a share above one is not cut back to one before it is applied");
    assert_eq!(faded_alpha(255, 1.5), 255);
    assert_eq!(faded_alpha(255, -0.25), 0);
    assert_eq!(faded_alpha(255, f32::NAN), 0);
}

#[test]
fn a_chart_played_in_another_mode_is_cut_into_parts_every_clear_line_falls_on_the_edge_of() {
    assert_eq!(retuned_parts(50, &[GROOVE, SURVIVAL]), 50, "eighty of a hundred already falls on the edge of a fiftieth");
    assert_eq!(retuned_parts(50, &[GaugeScale::new(2.0, 100.0, 75.0)]), 64, "seventy-five first does at sixty-four parts of 1.5625 each");
    assert_eq!(retuned_parts(50, &[POPN]), 96, "eighty-five of a hundred and twenty first does at ninety-six parts of 1.25 each");
    assert_eq!(retuned_parts(50, &[GROOVE, POPN, SURVIVAL]), 96, "the gauge that needs the most parts decides");
    assert_eq!(retuned_parts(50, &[GaugeScale::new(2.0, 100.0, 77.7)]), 50, "a line no count divides leaves the record's parts");
    assert_eq!(retuned_parts(50, &[GaugeScale::new(0.0, f32::INFINITY, 1.0)]), 50, "a scale with no top is not searched");
}

#[test]
fn the_parts_are_checked_against_the_mode_once_and_only_when_it_changed() {
    let scales = [POPN; GAUGE_TYPES];
    let mut motion = GaugeMotion::new(50);
    motion.check_mode(&GaugeFrame::playing(2, 50.0, scales));
    assert_eq!(motion.parts, 50, "the chart's own mode keeps the record's parts");
    motion.check_mode(&GaugeFrame::playing(2, 50.0, scales).with_mode_changed(true));
    assert_eq!(motion.parts, 50, "and the check is not made a second time");

    let mut motion = GaugeMotion::new(50);
    motion.check_mode(&GaugeFrame::playing(2, 50.0, scales).with_mode_changed(true));
    assert_eq!(motion.parts, 96);
}

#[test]
fn the_document_types_name_the_four_animations_and_no_other() {
    assert_eq!(
        [0, 1, 2, 3].map(GaugeAnimation::from_id),
        [Some(GaugeAnimation::Random), Some(GaugeAnimation::Increase), Some(GaugeAnimation::Decrease), Some(GaugeAnimation::Flickering)]
    );
    assert_eq!([-1, 4, 99].map(GaugeAnimation::from_id), [None; 3]);
}

#[test]
fn a_gauge_known_only_by_its_column_and_clear_line_is_a_percent_gauge_the_host_fills() {
    let gauge = GaugeFrame::of_kind(3, 80.0);
    assert_eq!((gauge.gauge_type, gauge.value, gauge.result, gauge.mode_changed), (3, None, false, false));
    assert_eq!(gauge.shown_scale(), Some(GaugeScale::new(0.0, 100.0, 80.0)));
    assert_eq!(GaugeFrame::finished(GAUGE_TYPES, 1.0, [GROOVE; GAUGE_TYPES]).shown_scale(), None, "a type past the last names no gauge");
    assert!(GaugeFrame::finished(4, 1.0, [GROOVE; GAUGE_TYPES]).result);
}
