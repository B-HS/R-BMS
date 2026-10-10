//! The arithmetic of the distribution graph, checked on the pixmaps it paints and without a renderer.

use super::*;

const GROUND: Rgba = [0, 0, 0, 204];
const RED: Rgba = [255, 0, 0, 255];
const GREEN: Rgba = [0, 255, 0, 255];
const BLUE: Rgba = [0, 0, 255, 255];
const NONE: Rgba = [0; 4];

const PLAIN: ChipStyle = ChipStyle { reversed: false, wide: false, tall: false };

/// Three classes in three clearly different colours, so a stack reads off a column of pixels.
fn palette() -> [Rgba; 3] {
    [RED, GREEN, BLUE]
}

/// The pixels of one chip-sized square at `(second, row)`, as a texture lays them out.
fn chip_at(pixmap: &Pixmap, second: usize, row: usize) -> Rgba {
    pixmap.pixel(second * PITCH, row * PITCH)
}

#[test]
fn a_graph_starts_twenty_rows_tall_and_grows_by_tens_to_a_hundred() {
    for (count, rows) in [(0, 20), (20, 20), (21, 30), (29, 30), (30, 40), (95, 100), (100, 100), (500, 100)] {
        assert_eq!(row_count(&[count, 0, 0], 3), rows, "a second of {count} notes");
    }
}

#[test]
fn the_rows_depend_on_the_order_the_seconds_come_in() {
    assert_eq!(row_count(&[25, 0, 30, 0], 2), 30, "thirty is not above the thirty rows twenty-five already asked for");
    assert_eq!(row_count(&[30, 0], 2), 40, "while thirty on its own asks for forty");
    assert_eq!(row_count(&[10, 5, 0, 12], 2), 20, "a count is the sum over the second's classes");
}

#[test]
fn the_ground_has_its_field_bands_and_rulers() {
    let seconds = 12;
    let rows = 20;
    let mut ground = Pixmap::new(seconds * PITCH, rows * PITCH);
    paint_ground(&mut ground, seconds, rows);

    assert_eq!(ground.pixel(20, 10), GROUND, "the field is black at four fifths opacity");
    assert_eq!(ground.pixel(20, 50), [17, 17, 0, 255], "the band that starts ten rows up is the first shade of yellow");
    assert_eq!(ground.pixel(20, 99), [17, 17, 0, 255], "and it is ten rows tall");
    assert_eq!(ground.pixel(0, 10), [63, 63, 63, 255], "second zero is a minute mark");
    assert_eq!(ground.pixel(50, 10), [31, 31, 31, 255], "second ten is a ten second mark");
    assert_eq!(ground.pixel(25, 10), GROUND, "any other second has none");
    assert_eq!(ground.pixel(50, 99), [31, 31, 31, 255], "and a mark runs the whole height of the field");

    let mut tall = Pixmap::new(seconds * PITCH, 40 * PITCH);
    paint_ground(&mut tall, seconds, 40);
    assert_eq!(tall.pixel(20, 100), [35, 35, 0, 255], "the band at row twenty is the next shade up");
    assert_eq!(tall.pixel(20, 150), [53, 53, 0, 255], "and row thirty is the one after");
}

#[test]
fn chips_stack_from_the_foot_one_row_per_note_in_class_order() {
    let mut chips = Pixmap::new(2 * PITCH, 20 * PITCH);
    let data = [2, 1, 3, 0, 0, 0];
    paint_chips(&mut chips, &data, &palette(), 20, PLAIN, 0..2);

    assert_eq!((0..7).map(|row| chip_at(&chips, 0, row)).collect::<Vec<_>>(), vec![RED, RED, GREEN, BLUE, BLUE, BLUE, NONE]);
    assert_eq!(chip_at(&chips, 1, 0), NONE, "a second with no notes has no chips");
}

#[test]
fn a_chip_is_four_pixels_square_and_leaves_a_gap_before_the_next() {
    let mut chips = Pixmap::new(PITCH, 4 * PITCH);
    paint_chips(&mut chips, &[2, 0, 0], &palette(), 4, PLAIN, 0..1);

    assert_eq!(chips.pixel(3, 3), RED, "the chip's far corner");
    assert_eq!(chips.pixel(4, 0), NONE, "the gap to its right");
    assert_eq!(chips.pixel(0, 4), NONE, "the gap above it");
    assert_eq!(chips.pixel(0, 5), RED, "and the next chip begins a pitch up");
}

#[test]
fn a_record_that_wants_no_gap_fills_the_pitch_on_the_axis_it_names() {
    let mut across = Pixmap::new(PITCH, 4 * PITCH);
    paint_chips(&mut across, &[2, 0, 0], &palette(), 4, ChipStyle { wide: true, ..PLAIN }, 0..1);
    assert_eq!((across.pixel(4, 0), across.pixel(0, 4)), (RED, NONE), "noGapX closes the gap beside a chip only");

    let mut up = Pixmap::new(PITCH, 4 * PITCH);
    paint_chips(&mut up, &[2, 0, 0], &palette(), 4, ChipStyle { tall: true, ..PLAIN }, 0..1);
    assert_eq!((up.pixel(4, 0), up.pixel(0, 4)), (NONE, RED), "noGap closes the gap above a chip only");
}

#[test]
fn a_reversed_stack_starts_with_the_last_class() {
    let mut chips = Pixmap::new(PITCH, 20 * PITCH);
    paint_chips(&mut chips, &[1, 1, 2], &palette(), 20, ChipStyle { reversed: true, ..PLAIN }, 0..1);
    assert_eq!((0..4).map(|row| chip_at(&chips, 0, row)).collect::<Vec<_>>(), vec![BLUE, BLUE, GREEN, RED]);
}

#[test]
fn chips_past_the_top_row_are_cut() {
    let mut chips = Pixmap::new(PITCH, 3 * PITCH);
    paint_chips(&mut chips, &[1, 1, 5], &palette(), 3, PLAIN, 0..1);
    assert_eq!((0..3).map(|row| chip_at(&chips, 0, row)).collect::<Vec<_>>(), vec![RED, GREEN, BLUE], "the three rows there are are filled and no more");
    assert_eq!(chips.pixel(0, 3 * PITCH), NONE, "and nothing lands past the texture");
}

#[test]
fn painting_a_span_again_clears_it_and_leaves_the_seconds_around_it() {
    let mut chips = Pixmap::new(3 * PITCH, 20 * PITCH);
    paint_chips(&mut chips, &[1, 0, 0, 1, 0, 0, 1, 0, 0], &palette(), 20, PLAIN, 0..3);
    paint_chips(&mut chips, &[1, 0, 0, 0, 1, 0, 1, 0, 0], &palette(), 20, PLAIN, 1..2);

    assert_eq!(chip_at(&chips, 0, 0), RED, "before the span");
    assert_eq!(chip_at(&chips, 1, 0), GREEN, "the span is what it is now");
    assert_eq!(chip_at(&chips, 2, 0), RED, "after the span");
}

#[test]
fn the_seconds_that_changed_are_the_first_to_the_last_differing_row() {
    let old = [1, 0, 0, 0, 0, 0, 2, 2];
    let new = [1, 0, 0, 1, 0, 0, 2, 0];
    assert_eq!(changed_seconds(&old, &new, 2), Some(1..4));
    assert_eq!(changed_seconds(&old, &old, 2), None);
    assert_eq!(changed_seconds(&old, &[1, 0, 0, 0, 0, 0, 2, 2], 2), None);
    assert_eq!(changed_seconds(&[0, 0], &[0, 1], 2), Some(0..1));
}

#[test]
fn the_chips_are_revealed_in_proportion_to_the_scene_time_until_the_delay() {
    assert_eq!(revealed(0, 500), 0.0);
    assert_eq!(revealed(250, 500), 0.5);
    assert_eq!(revealed(500, 500), 1.0);
    assert_eq!(revealed(9_000, 500), 1.0);
    assert_eq!(revealed(0, 0), 1.0, "a record with no delay shows everything at once");
}

#[test]
fn the_cursor_marks_the_range_and_the_position_with_integer_arithmetic() {
    let seconds = 12;
    let mut cursor = Pixmap::new(seconds * PITCH, 20 * PITCH);
    let marks = PlayCursor { start_ms: Some(2_000), end_ms: Some(8_000), speed: None };
    paint_cursor(&mut cursor, seconds, marks, Some(3_000));

    assert_eq!(cursor.pixel(10, 50), START_CURSOR, "two seconds of twelve is a sixth of sixty pixels in");
    assert_eq!(cursor.pixel(12, 50), START_CURSOR, "three pixels wide");
    assert_eq!(cursor.pixel(13, 50), NONE);
    assert_eq!(cursor.pixel(40, 99), END_CURSOR, "the end of the range runs the full height");
    assert_eq!(cursor.pixel(15, 0), NOW_CURSOR, "and the playing position is where three seconds of twelve falls");

    paint_cursor(&mut cursor, seconds, PlayCursor::default(), None);
    assert_eq!(cursor.pixel(10, 50), NONE, "a cursor is cleared before it is painted again");
    assert_eq!(cursor.pixel(15, 0), NONE, "and no position paints no mark");
}

#[test]
fn a_practice_speed_scales_the_playing_position_and_a_negative_mark_is_none() {
    let seconds = 12;
    let mut cursor = Pixmap::new(seconds * PITCH, 20 * PITCH);
    let marks = PlayCursor { start_ms: Some(-1), end_ms: Some(-5), speed: Some(2.0) };
    paint_cursor(&mut cursor, seconds, marks, Some(3_000));

    assert_eq!(cursor.pixel(30, 5), NOW_CURSOR, "three seconds at double speed is six seconds in");
    assert_eq!(cursor.pixel(15, 5), NONE);
    assert_eq!((0..cursor.size().0).filter(|x| cursor.pixel(*x, 5) != NONE).count(), 3, "and the negative marks drew nothing");

    paint_cursor(&mut cursor, seconds, PlayCursor { speed: Some(0.0), ..marks }, Some(3_000));
    assert_eq!(cursor.pixel(15, 5), NOW_CURSOR, "a speed of zero is no speed at all");
}

#[test]
fn each_kind_reads_its_own_series_and_has_its_own_palette() {
    let kinds = [[1, 2, 3, 4, 5, 6, 7]];
    let judgements = [[1, 2, 3, 4, 5, 6], [6, 5, 4, 3, 2, 1]];
    let notes = NoteDistribution { kinds: &kinds, judgements: &judgements, ..NoteDistribution::default() };

    assert_eq!(notes.rows(Kind::NoteKinds), (&[1, 2, 3, 4, 5, 6, 7][..], NOTE_KINDS));
    assert_eq!(notes.rows(Kind::Judgements), (&[1, 2, 3, 4, 5, 6, 6, 5, 4, 3, 2, 1][..], JUDGEMENTS));
    assert_eq!(notes.rows(Kind::EarlyLate), (&[][..], EARLY_LATE_BUCKETS), "a series the screen did not supply is empty");

    assert_eq!(Kind::NoteKinds.colors(false).len(), NOTE_KINDS);
    assert_eq!(Kind::Judgements.colors(false).len(), JUDGEMENTS);
    assert_eq!(Kind::EarlyLate.colors(false).len(), EARLY_LATE_BUCKETS);
    assert_eq!(Kind::NoteKinds.colors(true), Kind::NoteKinds.colors(false), "the note kinds have no second palette");
    assert_ne!(Kind::Judgements.colors(true), Kind::Judgements.colors(false));
    assert_eq!(Kind::Judgements.colors(true)[1], 0xff5eb0, "pop'n's best judgement is pink");

    assert_eq!([0, 1, 2, 3].map(Kind::of), [Some(Kind::NoteKinds), Some(Kind::Judgements), Some(Kind::EarlyLate), None]);
    assert_eq!(Kind::of(-1), None);
}

#[test]
fn run_totals_alone_carry_no_seconds() {
    let totals = [3, 2, 1, 0, 0, 0];
    let notes = NoteDistribution::of_judgements(&totals);
    assert_eq!(notes.judged, &totals);
    for kind in [Kind::NoteKinds, Kind::Judgements, Kind::EarlyLate] {
        assert!(notes.rows(kind).0.is_empty());
    }
}

/// A palette as the colours a pixmap stores for it.
fn stored(kind: Kind, popn: bool) -> Vec<Rgba> {
    kind.colors(popn).iter().map(|color| hex_color(*color)).collect()
}

#[test]
fn the_judgement_graph_is_coloured_by_the_references_table_from_unjudged_to_poor() {
    let expected: [Rgba; JUDGEMENTS] =
        [[0x55, 0x55, 0x55, 255], [0x00, 0x88, 0xff, 255], [0x00, 0xff, 0x88, 255], [0xff, 0xff, 0x00, 255], [0xff, 0x88, 0x00, 255], [0xff, 0x00, 0x00, 255]];
    assert_eq!(stored(Kind::Judgements, false), expected);

    let popn: [Rgba; JUDGEMENTS] =
        [[0x55, 0x55, 0x55, 255], [0xff, 0x5e, 0xb0, 255], [0xff, 0xbe, 0x32, 255], [0xdc, 0x46, 0x3c, 255], [0x6c, 0xc6, 0xff, 255], [0x6c, 0xc6, 0xff, 255]];
    assert_eq!(stored(Kind::Judgements, true), popn);
}

#[test]
fn the_early_late_graph_is_coloured_blue_for_early_and_orange_for_late_each_darker_for_a_worse_judgement() {
    let expected: [Rgba; EARLY_LATE_BUCKETS] = [
        [0x55, 0x55, 0x55, 255],
        [0x44, 0xff, 0x44, 255],
        [0x00, 0x88, 0xff, 255],
        [0x00, 0x66, 0xcc, 255],
        [0x00, 0x44, 0x88, 255],
        [0x00, 0x22, 0x44, 255],
        [0xff, 0x88, 0x00, 255],
        [0xcc, 0x66, 0x00, 255],
        [0x88, 0x44, 0x00, 255],
        [0x44, 0x22, 0x00, 255],
    ];
    assert_eq!(stored(Kind::EarlyLate, false), expected);

    let mut popn = expected;
    popn[1] = [0xff, 0x5e, 0xb0, 255];
    assert_eq!(stored(Kind::EarlyLate, true), popn, "only the best judgement has a colour of its own in the nine-key mode");
}

#[test]
fn a_second_of_judgements_stacks_from_unjudged_at_the_foot_to_poor_on_top() {
    let palette = stored(Kind::Judgements, false);
    let mut chips = Pixmap::new(PITCH, 20 * PITCH);
    paint_chips(&mut chips, &[1, 3, 2, 1, 1, 2], &palette, 20, PLAIN, 0..1);
    let stack: Vec<Rgba> = (0..11).map(|row| chip_at(&chips, 0, row)).collect();
    let expected = [vec![palette[0]], vec![palette[1]; 3], vec![palette[2]; 2], vec![palette[3]], vec![palette[4]], vec![palette[5]; 2], vec![NONE]].concat();
    assert_eq!(stack, expected);

    let mut reversed = Pixmap::new(PITCH, 20 * PITCH);
    paint_chips(&mut reversed, &[1, 3, 2, 1, 1, 2], &palette, 20, ChipStyle { reversed: true, ..PLAIN }, 0..1);
    let stack: Vec<Rgba> = (0..11).map(|row| chip_at(&reversed, 0, row)).collect();
    let expected = [vec![palette[5]; 2], vec![palette[4]], vec![palette[3]], vec![palette[2]; 2], vec![palette[1]; 3], vec![palette[0]], vec![NONE]].concat();
    assert_eq!(stack, expected, "a record that reverses the order puts the poors at the foot");
}

#[test]
fn a_second_of_early_and_late_stacks_the_best_then_the_early_ones_then_the_late_ones() {
    let palette = stored(Kind::EarlyLate, false);
    let mut chips = Pixmap::new(PITCH, 20 * PITCH);
    paint_chips(&mut chips, &[0, 2, 1, 1, 0, 1, 2, 0, 1, 1], &palette, 20, PLAIN, 0..1);
    let stack: Vec<Rgba> = (0..10).map(|row| chip_at(&chips, 0, row)).collect();
    let expected =
        [vec![palette[1]; 2], vec![palette[2]], vec![palette[3]], vec![palette[5]], vec![palette[6]; 2], vec![palette[8]], vec![palette[9]], vec![NONE]]
            .concat();
    assert_eq!(stack, expected, "a class that counts nothing takes no row");
}
