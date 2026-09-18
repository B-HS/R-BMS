//! Unit tests for the two objects a screen rather than a run feeds: the skin preview pane and the
//! practice panel's row pane.
//!
//! Both are drawn onto a [`CpuCanvas`] through the very dispatch a real frame goes through, so what
//! is asserted is the pixel a screen would have shown rather than a call that was made.

use rbms_skin::dst::{Acc, DestinationTrack, Keyframe, SkinColor, SkinRect};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::SkinLayer;
use rbms_skin::timer::TimerState;

use super::draw::draw_object;
use super::object::{Body, PracticeBody, PreviewBody, SkinObject};
use super::state::{
    PRACTICE_ROW_FOCUSED_FIRST, PRACTICE_ROW_LABEL_FIRST, PRACTICE_ROW_MAX, PRACTICE_ROW_VALUE_FIRST, PRACTICE_SCREEN_OPEN, PracticeRows, PracticeViewState,
};
use super::{FrameExtra, SkinFrame, SkinViewport};
use crate::ctx::with_render_ctx;
use crate::font::use_embedded_fonts_only;
use crate::{Color, CpuCanvas, Renderer};
use rbms_skin::property::{SkinStateSource, UNMAPPED_STRING};

/// The canvas every test draws on, which is also the size the objects are placed in, so a document
/// pixel is a screen pixel.
const CANVAS: (u32, u32) = (640, 360);

/// The rectangle both panes are given, well inside the canvas.
const PANE: SkinRect = SkinRect { x: 40.0, y: 40.0, w: 400.0, h: 280.0 };

/// The colour the preview texture is painted, picked so nothing else on a cleared canvas is it.
const PREVIEW_MARK: Color = Color::rgb(12, 200, 90);

/// A destination that holds one rectangle still, fully opaque.
fn still(rect: SkinRect) -> DestinationTrack {
    DestinationTrack {
        frames: vec![Keyframe { time_ms: 0, rect, clip: None, acc: Acc::default(), color: SkinColor::rgba(255, 255, 255, 255), angle_deg: 0.0 }],
        ..DestinationTrack::default()
    }
}

/// One draw-list entry over a body and the rectangle its destination holds.
fn object(id: &str, body: Body) -> SkinObject {
    SkinObject { id: id.to_owned(), layer: SkinLayer::Foreground, track: still(PANE), stretch: StretchKind::from_id(-1), body }
}

/// The labels and values the practice tests hand the panel.
fn rows() -> (Vec<&'static str>, Vec<String>) {
    let labels = vec!["START", "END", "GAUGE"];
    let values = vec!["0:00".to_string(), "1:30".to_string(), "NORMAL".to_string()];
    (labels, values)
}

/// Draws one object onto a fresh canvas with `extra` as the frame's screen-shaped state, and
/// answers the canvas and whether the object drew.
fn draw(object: &SkinObject, state: &dyn SkinStateSource, extra: FrameExtra<'_>) -> (CpuCanvas, bool) {
    use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    canvas.clear(Color::rgb(0, 0, 0));
    let timers = TimerState::default();
    let viewport = SkinViewport::new((CANVAS.0 as f32, CANVAS.1 as f32), (CANVAS.0 as f32, CANVAS.1 as f32));
    let frame = SkinFrame { now_ms: 0, timers: &timers, state, lua: None, mouse: None, background: None, extra };
    let drawn = with_render_ctx(|ctx| draw_object(ctx, &mut canvas, object, &viewport, &frame));
    (canvas, drawn)
}

/// How many pixels of `canvas` are exactly `colour`.
fn count_of(canvas: &CpuCanvas, colour: Color) -> usize {
    (0..CANVAS.1).flat_map(|y| (0..CANVAS.0).map(move |x| (x, y))).filter(|(x, y)| canvas.pixel_at(*x, *y) == colour).count()
}

/// How many pixels of `canvas` are not black, which is what a line of text leaves behind.
fn lit(canvas: &CpuCanvas) -> usize {
    (0..CANVAS.1).flat_map(|y| (0..CANVAS.0).map(move |x| (x, y))).filter(|(x, y)| canvas.pixel_at(*x, *y) != Color::rgb(0, 0, 0)).count()
}

/// The practice state the row tests read through.
fn practice_state<'a>(rows: &'a PracticeRows<'a>) -> PracticeViewState<'a> {
    PracticeViewState { rows, now_ms: 0, offsets: None }
}

#[test]
fn the_preview_pane_draws_the_texture_the_host_handed_it_over_its_whole_rectangle() {
    use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(CANVAS.0, CANVAS.1);
    let pixels: Vec<u8> = (0..4).flat_map(|_| [PREVIEW_MARK.r, PREVIEW_MARK.g, PREVIEW_MARK.b, 255]).collect();
    let tex = canvas.register_texture("preview", &pixels, 2, 2);
    let object = object("preview", Body::SkinPreview(PreviewBody { tex: Some(tex) }));

    canvas.clear(Color::rgb(0, 0, 0));
    let timers = TimerState::default();
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 0, visible: 0 };
    let state = practice_state(&rows);
    let viewport = SkinViewport::new((CANVAS.0 as f32, CANVAS.1 as f32), (CANVAS.0 as f32, CANVAS.1 as f32));
    let frame = SkinFrame { now_ms: 0, timers: &timers, state: &state, lua: None, mouse: None, background: None, extra: FrameExtra::None };
    let drawn = with_render_ctx(|ctx| draw_object(ctx, &mut canvas, &object, &viewport, &frame));

    assert!(drawn, "the preview pane did not draw");
    assert_eq!(count_of(&canvas, PREVIEW_MARK), (PANE.w * PANE.h) as usize, "the preview did not cover its whole rectangle");
}

/// A pane with no texture is a screen with nothing to preview -- or one whose preview would be
/// itself -- and must leave the canvas exactly as it found it.
#[test]
fn a_preview_pane_with_no_texture_draws_nothing() {
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 0, visible: 0 };
    let state = practice_state(&rows);
    let object = object("preview", Body::SkinPreview(PreviewBody::default()));
    let (canvas, drawn) = draw(&object, &state, FrameExtra::None);
    assert!(!drawn, "a pane with nothing to show reported a draw");
    assert_eq!(lit(&canvas), 0, "a pane with nothing to show painted something");
}

/// A pane asking for rows draws none of them: the count is all it sets, and the document's own
/// objects read the ids.
#[test]
fn a_practice_pane_asking_for_rows_draws_none_of_them_itself() {
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 1, visible: 10 };
    let state = practice_state(&rows);
    let object = object("practice", Body::Practice(PracticeBody { visible_items: 10 }));
    let (canvas, drawn) = draw(&object, &state, FrameExtra::Practice(&rows));
    assert!(!drawn, "a pane that only sets a row count reported a draw");
    assert_eq!(lit(&canvas), 0, "a pane that only sets a row count painted something");
}

/// A pane asking for no rows draws the list itself, inside its own rectangle.
#[test]
fn a_practice_pane_asking_for_no_rows_draws_the_list_inside_its_rectangle() {
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 1, visible: labels.len() };
    let state = practice_state(&rows);
    let object = object("practice", Body::Practice(PracticeBody { visible_items: 0 }));
    let (canvas, drawn) = draw(&object, &state, FrameExtra::Practice(&rows));

    assert!(drawn, "the fallback list did not draw");
    assert!(lit(&canvas) > 0, "the fallback list painted nothing");
    let outside = (0..CANVAS.1)
        .flat_map(|y| (0..CANVAS.0).map(move |x| (x, y)))
        .filter(|(x, y)| canvas.pixel_at(*x, *y) != Color::rgb(0, 0, 0))
        .filter(|(x, y)| {
            let (x, y) = (*x as f32, *y as f32);
            let across = PANE.x..PANE.x + PANE.w;
            let down = CANVAS.1 as f32 - (PANE.y + PANE.h)..CANVAS.1 as f32 - PANE.y;
            !across.contains(&x) || !down.contains(&y)
        })
        .count();
    assert_eq!(outside, 0, "the fallback list drew outside the pane's rectangle");
}

/// A pane asking for no rows with no practice state behind it is a play frame, where the pane is
/// not the screen being drawn.
#[test]
fn a_practice_pane_with_no_rows_behind_it_draws_nothing() {
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 0, visible: 3 };
    let state = practice_state(&rows);
    let object = object("practice", Body::Practice(PracticeBody { visible_items: 0 }));
    let (canvas, drawn) = draw(&object, &state, FrameExtra::None);
    assert!(!drawn);
    assert_eq!(lit(&canvas), 0);
}

#[test]
fn the_practice_state_answers_a_row_per_private_id_and_stops_at_the_last_one() {
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 2, visible: PRACTICE_ROW_MAX };
    let state = practice_state(&rows);

    assert_eq!(state.string(PRACTICE_ROW_LABEL_FIRST), "START");
    assert_eq!(state.string(PRACTICE_ROW_VALUE_FIRST + 1), "1:30");
    assert_eq!(state.string(PRACTICE_ROW_LABEL_FIRST + 2), "GAUGE");
    assert_eq!(state.string(PRACTICE_ROW_LABEL_FIRST + 3), UNMAPPED_STRING, "a row past the panel's own answered anyway");
    assert_eq!(state.string(PRACTICE_ROW_VALUE_FIRST + PRACTICE_ROW_MAX as i32), UNMAPPED_STRING, "the value band runs past its own length");
}

#[test]
fn the_practice_state_focuses_exactly_one_row_and_says_the_panel_is_open() {
    use rbms_skin::dst::DrawStateSource;
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 2, visible: PRACTICE_ROW_MAX };
    let state = practice_state(&rows);

    assert!(state.boolean(PRACTICE_SCREEN_OPEN), "the panel did not say it was open");
    let focused: Vec<usize> = (0..PRACTICE_ROW_MAX).filter(|row| state.boolean(PRACTICE_ROW_FOCUSED_FIRST + *row as i32)).collect();
    assert_eq!(focused, vec![2], "exactly one row is focused");
}

/// A document may ask for fewer rows than the panel has, and then only those are answered.
#[test]
fn a_document_asking_for_fewer_rows_than_the_panel_has_gets_only_those() {
    let (labels, values) = rows();
    let rows = PracticeRows { labels: &labels, values: &values, focused: 0, visible: 2 };
    let state = practice_state(&rows);

    assert_eq!(rows.len(), 2);
    assert_eq!(state.string(PRACTICE_ROW_LABEL_FIRST + 1), "END");
    assert_eq!(state.string(PRACTICE_ROW_LABEL_FIRST + 2), UNMAPPED_STRING, "a row the document did not ask for answered anyway");
}
