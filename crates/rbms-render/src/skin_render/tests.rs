//! Unit tests for the parts of the skin renderer that stand apart from any one object: the
//! coordinate change, the cell animation, the digit layouts, the play screen's timers and what a
//! whole screen answers about itself.

use rbms_skin::dst::{Acc, DestinationTrack, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::SkinLayer;
use rbms_skin::property::generated::{
    OFFSET_HIDDEN_COVER, OFFSET_LANECOVER, OFFSET_LIFT, OPTION_1P_EARLY, OPTION_1P_GOOD, OPTION_1P_LATE, OPTION_1P_PERFECT, OPTION_2P_EARLY, OPTION_2P_GOOD,
    OPTION_2P_PERFECT, OPTION_GAUGE_EX, OPTION_GAUGE_EX_2P, OPTION_GAUGE_GROOVE, OPTION_GAUGE_GROOVE_2P, OPTION_GAUGE_HARD, OPTION_GAUGE_HARD_2P,
};
use rbms_skin::timer::{TimerId, TimerState};

use super::object::{
    Body, DigitLayout, FloatBody, NumberBody, SkinObject, Sprite, ValueSource, fraction_glyphs, fraction_sign, integer_glyphs, integer_padding,
};
use super::state::{PlayViewState, SkinHotAction};
use super::{FrameExtra, SkinFrame, SkinScreen, SkinViewport, TEXT_PIXELS_PER_SCALE};
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
fn the_viewport_reads_a_screen_row_back_into_the_documents_own_space() {
    let viewport = SkinViewport::new((256.0, 144.0), (512.0, 288.0));
    let placed = viewport.place(SkinRect::new(8.0, 16.0, 32.0, 64.0));

    assert_eq!(viewport.document_y(placed.y), 80.0, "a top edge reads back as how far above the document's foot it sits");
    assert_eq!(viewport.document_y(0.0), 144.0, "the top of the screen is the document's ceiling");

    let flat = SkinViewport::new((256.0, 144.0), (512.0, 0.0));
    assert_eq!(flat.document_y(120.0), 144.0, "a screen with no height answers the ceiling rather than dividing by zero");
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

/// A run with no lane state to report, which is what the timer memories above are exercised with:
/// they are about the judgement, the combo and the gauge rather than about the keys.
fn no_lanes() -> super::screen::PlayLanes<'static> {
    super::screen::PlayLanes { lanes: &[], judged_side: 0 }
}

#[test]
fn the_first_play_frame_starts_no_timer_by_itself() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([3, 0, 0, 0, 0, 0], 3, 50.0), 10, 1_000, &no_lanes());

    assert!(timers.is_off(rbms_skin::timer::timer_id::JUDGE_1P), "the first frame has nothing to compare against, so nothing is treated as new");
    assert!(timers.is_on(rbms_skin::timer::timer_id::COMBO_1P), "a combo that is already running is reported as running");
}

#[test]
fn a_new_judgement_restarts_the_judge_timer() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([1, 0, 0, 0, 0, 0], 1, 50.0), 100, 1_000, &no_lanes());
    memory.update(&mut timers, &hud([2, 0, 0, 0, 0, 0], 2, 50.0), 100, 1_500, &no_lanes());

    assert_eq!(timers.get(rbms_skin::timer::timer_id::JUDGE_1P), Some(1_500), "the pop-up is measured from the moment the input was judged");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::COMBO_1P), Some(1_500), "a longer combo restarts its own flash");
}

#[test]
fn a_broken_combo_switches_its_timer_off() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([2, 0, 0, 0, 0, 0], 2, 50.0), 100, 1_000, &no_lanes());
    memory.update(&mut timers, &hud([2, 0, 0, 0, 1, 0], 0, 48.0), 100, 1_200, &no_lanes());

    assert!(timers.is_off(rbms_skin::timer::timer_id::COMBO_1P), "a combo of nothing has no flash to animate");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::JUDGE_1P), Some(1_200), "the poor was still a judgement");
}

#[test]
fn a_full_combo_and_a_full_gauge_stay_on_while_they_last() {
    let mut memory = super::screen::PlayTimers::new();
    let mut timers = TimerState::new();
    memory.update(&mut timers, &hud([4, 0, 0, 0, 0, 0], 4, 100.0), 4, 1_000, &no_lanes());
    assert_eq!(timers.get(rbms_skin::timer::timer_id::FULLCOMBO_1P), Some(1_000));
    assert_eq!(timers.get(rbms_skin::timer::timer_id::GAUGE_MAX_1P), Some(1_000));

    memory.update(&mut timers, &hud([5, 0, 0, 0, 0, 0], 5, 100.0), 5, 1_400, &no_lanes());
    assert_eq!(timers.get(rbms_skin::timer::timer_id::FULLCOMBO_1P), Some(1_000), "a timer that is already on keeps the moment it started");
    assert_eq!(timers.get(rbms_skin::timer::timer_id::GAUGE_INCLEASE_1P), None, "a gauge that did not rise does not flash");

    memory.update(&mut timers, &hud([5, 0, 0, 0, 1, 0], 0, 90.0), 6, 1_800, &no_lanes());
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

/// A play adapter over `hud`, which is also the simplest state source these tests have to hand.
fn play_state<'a>(hud: &'a crate::hud::HudView<'a>) -> PlayViewState<'a> {
    PlayViewState {
        hud,
        title: "",
        song_ms: 0,
        duration_ms: 0,
        bpm: 0.0,
        hispeed: 1.0,
        autoplay: false,
        now_ms: 0,
        offsets: None,
        field: None,
        shade: crate::playfield::LaneShade::default(),
        judged_side: 0,
        gauge_kind: 0,
        artist: "",
        level: 0,
        bpm_min: 0.0,
        bpm_max: 0.0,
        bpm_main: 0.0,
        target_ex: None,
        target_delta: "",
    }
}

#[test]
fn a_judgement_is_reported_on_the_field_it_was_played_on() {
    let hud = crate::hud::HudView { last_judge: Some(2), last_fast: true, ..hud([0; 6], 0, 0.0) };
    let left = play_state(&hud);
    let right = PlayViewState { judged_side: 1, ..play_state(&hud) };

    assert!(left.boolean(OPTION_1P_GOOD), "the field the input was played on reports the judgement the run took");
    assert!(!left.boolean(OPTION_2P_GOOD), "and the other field stays quiet, so a double document does not flash both pop-ups at once");
    assert!(right.boolean(OPTION_2P_GOOD) && !right.boolean(OPTION_1P_GOOD), "an input on the right-hand field is reported there instead");
    assert!(!left.boolean(OPTION_1P_PERFECT), "a field reports only the judgement the run actually took");
    assert!(
        right.boolean(OPTION_2P_PERFECT + 2),
        "the reference stops naming the second field's band at its third judgement, but a pop-up reads all six of them"
    );
    assert!(left.boolean(OPTION_1P_EARLY) && !left.boolean(OPTION_2P_EARLY), "an early hit is early on the field it landed on");
    assert!(!left.boolean(OPTION_1P_LATE), "and is not also late on it");
}

/// A document labels the gauge it is drawing from the three gauge options, so the play adapter has
/// to answer them: without that, every label is hidden and every `op` naming the opposite of one is
/// drawn, whatever gauge is actually being played.
#[test]
fn the_gauge_being_played_answers_the_options_a_document_labels_it_from() {
    let hud = hud([0; 6], 0, 0.0);
    let on = |kind: usize, id: i32| PlayViewState { gauge_kind: kind, ..play_state(&hud) }.boolean(id);

    for kind in 0..=2 {
        assert!(on(kind, OPTION_GAUGE_GROOVE), "gauge {kind} is cleared by filling it and is not named as one");
        assert!(!on(kind, OPTION_GAUGE_HARD), "gauge {kind} is not survived");
    }
    for kind in 3..=5 {
        assert!(on(kind, OPTION_GAUGE_HARD), "gauge {kind} is survived and is not named as one");
        assert!(!on(kind, OPTION_GAUGE_GROOVE), "gauge {kind} is not cleared by filling it");
    }
    for kind in [0, 1, 4, 5, 7, 8] {
        assert!(on(kind, OPTION_GAUGE_EX), "gauge {kind} drains at the EX rate and is not named as one");
    }
    for kind in [2, 3, 6] {
        assert!(!on(kind, OPTION_GAUGE_EX), "gauge {kind} does not drain at the EX rate");
    }
    assert!(on(6, OPTION_GAUGE_HARD) && !on(6, OPTION_GAUGE_GROOVE), "a course gauge is survived like the rest of its half of the table");

    assert!(on(2, OPTION_GAUGE_GROOVE_2P) && on(3, OPTION_GAUGE_HARD_2P) && on(4, OPTION_GAUGE_EX_2P), "both bands name the one gauge the run carries");
    assert!(!on(2, -OPTION_GAUGE_GROOVE), "and a document asking for the gauge it is not playing on still gets an answer, not the default");
}

#[test]
fn the_running_field_publishes_the_cover_offsets_a_document_places_its_own_covers_with() {
    let hud = hud([0; 6], 0, 0.0);
    let mut field = crate::skin::Skin::default_for(rbms_model::Mode::BEAT_7K, 1280.0, 720.0);
    field.top_y = 100.0;
    field.judge_y = 500.0;
    field.lift_height = 40.0;
    let shade = crate::playfield::LaneShade { cover: 0.25, hidden: 0.5 };
    let state = PlayViewState { field: Some(&field), shade, ..play_state(&hud) };

    assert_eq!(state.offset(OFFSET_LIFT).map(|offset| offset.y), Some(40.0), "the lift offset is how far the judgement line was raised");
    assert_eq!(
        state.offset(OFFSET_LANECOVER).map(|offset| offset.y),
        Some(-100.0),
        "the cover the player pulls down from the ceiling moves by its share of the visible field"
    );
    assert_eq!(state.offset(OFFSET_HIDDEN_COVER).map(|offset| offset.y), Some(200.0), "and the hidden band rises from the judgement line by its own");

    let bare = PlayViewState { field: Some(&field), ..play_state(&hud) };
    let hidden = bare.offset(OFFSET_HIDDEN_COVER).expect("a hidden band that is switched off still answers");
    assert_eq!((hidden.y, hidden.a), (0.0, -255.0), "a hidden band nobody asked for is published as fully transparent rather than as nothing at all");
    assert!(play_state(&hud).offset(OFFSET_LANECOVER).is_none(), "a screen with no field running leaves the three to the player's own nudges");
}

/// A destination that holds one rectangle still, fully opaque.
fn still(rect: SkinRect) -> DestinationTrack {
    DestinationTrack {
        frames: vec![Keyframe { time_ms: 0, rect, clip: None, acc: Acc::default(), color: SkinColor::rgba(255, 255, 255, 255), angle_deg: 0.0 }],
        ..DestinationTrack::default()
    }
}

#[test]
fn a_document_with_no_wheel_still_offers_the_buttons_its_hotspot_table_names() {
    let rect = SkinRect::new(10.0, 20.0, 40.0, 12.0);
    let button =
        SkinObject { id: "button".to_owned(), layer: SkinLayer::Foreground, track: still(rect), stretch: StretchKind::from_id(-1), body: Body::Background };
    let screen = SkinScreen {
        authored: (256.0, 144.0),
        objects: vec![button],
        textures: Vec::new(),
        families: Vec::new(),
        hotspots: vec![("button".to_owned(), SkinHotAction::ModalClose), ("nowhere".to_owned(), SkinHotAction::Search)],
        warnings: Vec::new(),
    };

    let timers = TimerState::new();
    let hud = hud([0; 6], 0, 0.0);
    let state = play_state(&hud);
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: None, extra: FrameExtra::None };

    let spots = screen.hotspots(&frame);
    assert_eq!(spots.len(), 1, "the entry naming an object the document never declared is dropped");
    assert_eq!(spots[0].action, SkinHotAction::ModalClose);
    assert_eq!(spots[0].rect, Rect::new(rect.x, rect.y, rect.w, rect.h), "the rectangle comes back in the document's own coordinates");
}
