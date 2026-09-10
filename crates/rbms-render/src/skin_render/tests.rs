//! Unit tests for the parts of the skin renderer that are pure arithmetic: the coordinate change,
//! the cell animation, and the digit layouts.

use rbms_skin::dst::{SkinColor, SkinRect};
use rbms_skin::timer::{TimerId, TimerState};

use super::object::{DigitLayout, FloatBody, NumberBody, Sprite, ValueSource, fraction_glyphs, fraction_sign, integer_glyphs, integer_padding};
use super::{SkinViewport, TEXT_PIXELS_PER_SCALE};
use crate::{Color, Rect, TextureId};

/// A sprite over a texture of `size`, cut into `columns` x `rows` cells.
fn sprite(size: (u32, u32), columns: u32, rows: u32, timer: Option<TimerId>, cycle: i32) -> Sprite {
    Sprite { tex: TextureId(0), size, origin: (0, 0), cell: (size.0 / columns, size.1 / rows), columns, rows, timer, cycle }
}

/// A number object drawing `digits` places from a strip of `cells`.
fn number(cells: u32, digits: u32, zero_padding: i32) -> NumberBody {
    NumberBody {
        sprite: sprite((cells * 4, 8), cells, 1, None, 0),
        layout: DigitLayout::integer(cells),
        digits,
        zero_padding,
        space: 0.0,
        align: 0,
        value: ValueSource::None,
        offsets: Vec::new(),
    }
}

#[test]
fn the_viewport_flips_the_vertical_axis_and_scales_both() {
    let viewport = SkinViewport::new((256.0, 144.0), (512.0, 288.0));
    let placed = viewport.place(SkinRect::new(8.0, 16.0, 32.0, 64.0));

    assert_eq!(placed.x, 16.0, "the horizontal axis only scales");
    assert_eq!(placed.y, 128.0, "a document measures upwards from its bottom, so the top edge is height minus y minus h");
    assert_eq!((placed.w, placed.h), (64.0, 128.0));
    assert_eq!((viewport.scale_x(), viewport.scale_y()), (2.0, 2.0));
}

#[test]
fn a_document_filling_its_own_space_fills_the_screen() {
    let viewport = SkinViewport::new((256.0, 144.0), (1280.0, 720.0));
    let placed = viewport.place(SkinRect::new(0.0, 0.0, 256.0, 144.0));
    assert_eq!((placed.x, placed.y, placed.w, placed.h), (0.0, 0.0, 1280.0, 720.0));
}

#[test]
fn a_document_with_no_extent_maps_one_to_one_rather_than_dividing_by_zero() {
    let viewport = SkinViewport::new((0.0, 0.0), (640.0, 480.0));
    assert_eq!((viewport.scale_x(), viewport.scale_y()), (1.0, 1.0));
}

#[test]
fn the_geometry_types_convert_field_for_field() {
    let rect: Rect = SkinRect::new(1.0, 2.0, 3.0, 4.0).into();
    assert_eq!((rect.x, rect.y, rect.w, rect.h), (1.0, 2.0, 3.0, 4.0), "the conversion does not flip; the viewport does");
    let color: Color = SkinColor::rgba(9, 8, 7, 6).into();
    assert_eq!(color, Color { r: 9, g: 8, b: 7, a: 6 });
}

#[test]
fn cells_are_numbered_across_before_down() {
    let sprite = sprite((32, 16), 4, 2, None, 0);
    assert_eq!(sprite.cells(), 8);
    assert_eq!(sprite.cell_size(), (8.0, 8.0));

    let first = sprite.uv(0);
    assert_eq!((first.u0, first.v0, first.u1, first.v1), (0.0, 0.0, 0.25, 0.5));
    let second_row = sprite.uv(4);
    assert_eq!((second_row.u0, second_row.v0), (0.0, 0.5), "cell four starts the second row");
}

#[test]
fn a_cell_index_past_the_last_one_holds_at_the_last() {
    let sprite = sprite((32, 16), 4, 2, None, 0);
    assert_eq!(sprite.uv(99), sprite.uv(7), "an out-of-range cell reads the final one rather than sampling outside the texture");
}

#[test]
fn an_animation_holds_still_without_a_cycle_or_a_running_timer() {
    let timers = TimerState::new();
    assert_eq!(sprite((32, 8), 4, 1, None, 0).animation_index(4, 5_000, &timers), 0, "no cycle means no animation");
    assert_eq!(sprite((32, 8), 4, 1, Some(TimerId(1)), 400).animation_index(4, 5_000, &timers), 0, "a timer that is off holds the first cell");
}

#[test]
fn an_animation_steps_through_its_cells_and_wraps() {
    let mut timers = TimerState::new();
    timers.set_on(TimerId(1), 1_000);
    let sprite = sprite((32, 8), 4, 1, Some(TimerId(1)), 400);

    assert_eq!(sprite.animation_index(4, 1_000, &timers), 0, "the moment the timer starts is the first cell");
    assert_eq!(sprite.animation_index(4, 1_100, &timers), 1);
    assert_eq!(sprite.animation_index(4, 1_300, &timers), 3);
    assert_eq!(sprite.animation_index(4, 1_400, &timers), 0, "one whole cycle is back to the start");
    assert_eq!(sprite.animation_index(4, 900, &timers), 0, "a moment before the timer started is the first cell too");
}

#[test]
fn an_integer_strip_is_cut_by_how_many_cells_it_has() {
    let shape = |layout: DigitLayout| (layout.glyphs, layout.sets, layout.negative);
    assert_eq!(shape(DigitLayout::integer(10)), (10, 1, false));
    assert_eq!(shape(DigitLayout::integer(20)), (10, 2, false));
    assert_eq!(shape(DigitLayout::integer(11)), (11, 1, false));
    assert_eq!(shape(DigitLayout::integer(24)), (12, 1, true), "a multiple of twenty-four carries a negative half");
}

#[test]
fn a_negative_set_starts_halfway_through_its_stride() {
    let layout = DigitLayout::integer(48);
    assert_eq!(layout.cell(0, false, 0), Some(0));
    assert_eq!(layout.cell(0, true, 0), Some(12));
    assert_eq!(layout.cell(1, false, 0), Some(24));
    assert_eq!(layout.cell(1, true, 0), Some(36));

    let unsigned = DigitLayout::integer(30);
    assert_eq!(unsigned.cell(2, true, 0), Some(20), "a strip with no negative half ignores the sign");
    assert_eq!(unsigned.cell(0, false, 10), None, "a slot past the end of a set reads nothing rather than the next set");
}

#[test]
fn a_fraction_strip_recognises_the_five_shapes_the_reference_lists() {
    let shape = |layout: DigitLayout| (layout.glyphs, layout.sets, layout.negative);
    assert_eq!(shape(DigitLayout::fraction(26)), (13, 1, true));
    assert_eq!(shape(DigitLayout::fraction(24)), (12, 1, true));
    assert_eq!(shape(DigitLayout::fraction(22)), (12, 1, true), "eleven cells per sign still answer twelve slots");
    assert_eq!(shape(DigitLayout::fraction(12)), (12, 1, false));
    assert_eq!(shape(DigitLayout::fraction(11)), (12, 1, false), "eleven cells still answer twelve slots");
    assert_eq!(shape(DigitLayout::fraction(7)), (12, 1, false), "anything else is read as twelve glyphs");
}

/// The two fractional strips that carry eleven cells per sign share cell zero between the ordinary
/// and the alternate zero, and keep the decimal point in the eleventh cell rather than the twelfth
/// slot (`JsonSkinObjectLoader`'s `% 11` and `% 22` branches). Reading slot for cell there would
/// leave every decimal point on such a strip blank.
#[test]
fn an_eleven_cell_fraction_strip_shares_its_zero_and_keeps_its_point() {
    let single = DigitLayout::fraction(22);
    assert_eq!(single.cell(0, false, 0), Some(0), "digits come first");
    assert_eq!(single.cell(0, false, 9), Some(9));
    assert_eq!(single.cell(0, false, 10), Some(0), "the alternate zero shares the ordinary one");
    assert_eq!(single.cell(0, false, 11), Some(10), "the decimal point follows the ten digits");
    assert_eq!(single.cell(0, true, 0), Some(11), "the negative half starts after eleven cells");
    assert_eq!(single.cell(0, true, 10), Some(11));
    assert_eq!(single.cell(0, true, 11), Some(21));

    let unsigned = DigitLayout::fraction(22 / 2);
    assert_eq!(unsigned.cell(0, false, 11), Some(10), "the same table without a negative half");
    assert_eq!(unsigned.cell(1, false, 11), Some(21), "and it repeats every eleven cells");
}

#[test]
fn leading_places_are_blank_unless_the_document_asks_for_padding() {
    let blank = integer_glyphs(&number(10, 4, 0), 42);
    assert_eq!(blank.as_slice(), [None, None, Some(4), Some(2)]);

    let zeroes = integer_glyphs(&number(10, 4, 1), 42);
    assert_eq!(zeroes.as_slice(), [Some(0), Some(0), Some(4), Some(2)]);

    let alternate = integer_glyphs(&number(11, 4, 2), 42);
    assert_eq!(alternate.as_slice(), [Some(10), Some(10), Some(4), Some(2)], "padding of two draws the strip's alternate zero");
}

#[test]
fn a_zero_still_fills_its_last_place() {
    assert_eq!(integer_glyphs(&number(10, 3, 0), 0).as_slice(), [None, None, Some(0)], "the units place is always drawn, even for nothing");
}

#[test]
fn a_number_wider_than_its_places_keeps_the_places_it_has() {
    assert_eq!(integer_glyphs(&number(10, 2, 0), 12_345).as_slice(), [Some(4), Some(5)], "the low places are the ones that fit");
}

#[test]
fn a_signed_strip_keeps_a_place_for_the_sign() {
    let signed = integer_glyphs(&number(24, 4, 1), 42);
    assert_eq!(signed.as_slice(), [Some(11), Some(0), Some(4), Some(2)], "the leading place holds the sign glyph of whichever half is drawn");
}

#[test]
fn a_fraction_is_laid_out_with_its_point_between_the_two_halves() {
    let body = FloatBody {
        sprite: sprite((52, 8), 13, 1, None, 0),
        layout: DigitLayout::fraction(13),
        integer_digits: 2,
        fraction_digits: 2,
        sign: false,
        zero_padding: 1,
        space: 0.0,
        align: 0,
        gain: 1.0,
        value: ValueSource::None,
        offsets: Vec::new(),
    };
    let places = fraction_glyphs(&body, 12.34);
    assert_eq!(places.as_slice().len(), 5, "two whole places, the point, and two fractional places");
    assert_eq!(places.as_slice()[2], Some(11), "the middle place is the decimal point");
    assert_eq!(places.as_slice()[0], Some(1));
    assert_eq!(places.as_slice()[1], Some(2));
    assert_eq!(places.as_slice()[3], Some(3));
}

#[test]
fn the_text_scale_factor_matches_the_engine_it_is_handed_to() {
    let requested_px = 17.0;
    let scale = requested_px / TEXT_PIXELS_PER_SCALE;
    let doubled = crate::font::text_width("RBMS", scale * 2.0);
    let single = crate::font::text_width("RBMS", scale);
    assert!(single > 0.0, "the bundled font measures the string");
    let ratio = doubled / single;
    assert!((ratio - 2.0).abs() < 0.2, "twice the scale is about twice the width, so the factor is a plain linear conversion (got {ratio})");
}

/// A HUD snapshot with nothing happening, which each timer test then changes one part of.
fn hud(counts: [u32; 6], combo: u32, gauge: f32) -> crate::hud::HudView<'static> {
    crate::hud::HudView {
        mode_label: "7K",
        combo,
        last_judge: None,
        last_fast: false,
        fast: [0; crate::result::LANE_KIND_COUNT],
        slow: [0; crate::result::LANE_KIND_COUNT],
        counts,
        ex_score: 0,
        gauge,
        green_number: 0.0,
        white_number: 0.0,
        judge_text_y: 0.0,
        max_ex: 0,
        best_ex: None,
        pace: None,
    }
}

#[test]
fn the_first_play_frame_starts_no_timer_by_itself() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([3, 0, 0, 0, 0, 0], 3, 50.0), 10, 1_000);

    assert!(timers.is_off(rbms_skin::timer::timer_id::JUDGE_1P), "the first frame has nothing to compare against, so nothing is treated as new");
    assert!(timers.is_on(rbms_skin::timer::timer_id::COMBO_1P), "a combo that is already running is reported as running");
}

#[test]
fn a_new_judgement_restarts_the_judge_timer() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([1, 0, 0, 0, 0, 0], 1, 50.0), 100, 1_000);
    memory.update(&mut timers, &hud([2, 0, 0, 0, 0, 0], 2, 50.0), 100, 1_500);

    assert_eq!(timers.get(rbms_skin::timer::timer_id::JUDGE_1P), Some(1_500), "the pop-up is measured from the moment the input was judged");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::COMBO_1P), Some(1_500), "a longer combo restarts its own flash");
}

#[test]
fn a_broken_combo_switches_its_timer_off() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([2, 0, 0, 0, 0, 0], 2, 50.0), 100, 1_000);
    memory.update(&mut timers, &hud([2, 0, 0, 0, 1, 0], 0, 48.0), 100, 1_200);

    assert!(timers.is_off(rbms_skin::timer::timer_id::COMBO_1P), "a combo of nothing has no flash to animate");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::JUDGE_1P), Some(1_200), "the poor was still a judgement");
}

#[test]
fn a_full_combo_and_a_full_gauge_stay_on_while_they_last() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([4, 0, 0, 0, 0, 0], 4, 100.0), 4, 1_000);
    assert_eq!(timers.get(rbms_skin::timer::timer_id::FULLCOMBO_1P), Some(1_000));
    assert_eq!(timers.get(rbms_skin::timer::timer_id::GAUGE_MAX_1P), Some(1_000));

    memory.update(&mut timers, &hud([5, 0, 0, 0, 0, 0], 5, 100.0), 5, 1_400);
    assert_eq!(timers.get(rbms_skin::timer::timer_id::FULLCOMBO_1P), Some(1_000), "a timer that is already on keeps the moment it started");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::GAUGE_INCLEASE_1P), None, "a gauge that did not rise does not flash");

    memory.update(&mut timers, &hud([5, 0, 0, 0, 1, 0], 0, 90.0), 6, 1_800);
    assert!(timers.is_off(rbms_skin::timer::timer_id::FULLCOMBO_1P), "the combo broke, so the full-combo timer goes off");
    assert!(timers.is_off(rbms_skin::timer::timer_id::GAUGE_MAX_1P));
}

#[test]
fn starting_and_failing_a_run_switch_the_timers_that_mark_them() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    timers.set_on(rbms_skin::timer::timer_id::READY, 0);
    memory.start(&mut timers, 500);
    assert!(timers.is_off(rbms_skin::timer::timer_id::READY));
    assert_eq!(timers.get(rbms_skin::timer::timer_id::PLAY), Some(500));

    memory.fail(&mut timers, 9_000);
    assert_eq!(timers.get(rbms_skin::timer::timer_id::FAILED), Some(9_000));
}

#[test]
fn moving_the_song_wheel_restarts_the_movement_timers_in_the_direction_it_went() {
    let mut memory = super::screen::SelectTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, 4, 1_000);
    assert!(timers.is_off(rbms_skin::timer::timer_id::SONGBAR_MOVE), "the first frame is where the wheel already was");

    memory.update(&mut timers, 5, 1_100);
    assert_eq!(timers.get(rbms_skin::timer::timer_id::SONGBAR_MOVE), Some(1_100));
    assert_eq!(timers.get(rbms_skin::timer::timer_id::SONGBAR_MOVE_DOWN), Some(1_100));
    assert!(timers.is_off(rbms_skin::timer::timer_id::SONGBAR_MOVE_UP));

    memory.update(&mut timers, 2, 1_300);
    assert_eq!(timers.get(rbms_skin::timer::timer_id::SONGBAR_MOVE_UP), Some(1_300), "going the other way starts the other direction's timer");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::SONGBAR_CHANGE), Some(1_300));
}

/// A value definition with both of its padding fields set, so which one a strip reads is visible.
fn padded(padding: i32, zeropadding: i32) -> rbms_skin::model::ValueDef {
    rbms_skin::model::ValueDef { padding, zeropadding, ..rbms_skin::model::ValueDef::default() }
}

/// The reference reads two different fields depending on how the strip is cut
/// (`JsonSkinObjectLoader`): the twenty-four-cell strip reads `zeropadding`, the ten-cell strip
/// reads `padding`, and the eleven-cell strip is forced to the alternate zero because its eleventh
/// cell is that glyph. Reading `zeropadding` everywhere puts blank places where the reference puts
/// the alternate zero, and the alignment shift then moves the whole number.
#[test]
fn an_integer_strip_reads_the_padding_field_its_own_shape_names() {
    assert_eq!(integer_padding(&DigitLayout::integer(24), &padded(1, 2)), 2, "a signed strip reads zeropadding");
    assert_eq!(integer_padding(&DigitLayout::integer(10), &padded(1, 2)), 1, "a ten-cell strip reads padding");
    assert_eq!(integer_padding(&DigitLayout::integer(11), &padded(0, 0)), 2, "an eleven-cell strip is the alternate zero whatever the document wrote");
}

/// `isSignvisible` is only honoured by the one fractional layout that has a sign glyph.
#[test]
fn a_sign_place_is_kept_only_by_the_strip_that_has_a_glyph_for_it() {
    assert!(fraction_sign(true, &DigitLayout::fraction(26)), "the twenty-six-cell strip carries a sign");
    for cells in [24, 22, 12, 11, 7] {
        assert!(!fraction_sign(true, &DigitLayout::fraction(cells)), "a {cells}-cell strip has no sign glyph to draw");
    }
    assert!(!fraction_sign(false, &DigitLayout::fraction(26)), "and a document that did not ask for one does not get one");
}

/// A `FLOAT_*` property is a plain measurement, not a share of anything: a hi-speed multiplier goes
/// past one and an average timing goes below zero. Only the rate half of the float id space is a
/// share, and only that half is narrowed.
#[test]
fn a_plain_measurement_reaches_a_drawn_number_unnarrowed() {
    use rbms_skin::property::{FLOAT_MAX, clamp_float, sanitize_float};

    assert_eq!(sanitize_float(2.5), 2.5, "a hi-speed of two and a half is an ordinary value");
    assert_eq!(sanitize_float(-12.4), -12.4, "and so is a timing average before the note");
    assert_eq!(sanitize_float(f32::NAN), 0.0, "a value the number line has no room for is not one");
    assert_eq!(sanitize_float(f32::INFINITY), 0.0);
    assert_eq!(clamp_float(2.5), FLOAT_MAX, "a rate is still a share of something and is still narrowed");
}
