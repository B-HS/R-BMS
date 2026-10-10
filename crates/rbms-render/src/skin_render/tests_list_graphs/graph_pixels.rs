//! What the judgement graph and the tempo graph put on a canvas, checked pixel by pixel.
//!
//! Each fixture is drawn at the size it is authored at on a canvas of its own, and its graphs are
//! given rectangles the same size as the textures they paint, so one texel is one pixel and a
//! chip, a ruler or a step of the tempo line can be addressed by the coordinates the reference's
//! pixmap would give it. A pixmap's first row is the bottom of the graph, so every row here is read
//! up from the foot of the rectangle.

use std::collections::BTreeSet;

use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::timer::{MICROS_PER_MILLI, TimerState, timer_id};

use super::{Nothing, Scratch, SolidAssets};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::frame::{GAUGE_TYPES, GaugeScale};
use crate::skin_render::graphs::PlayCursor;
use crate::skin_render::{
    BpmTimeline, FrameData, FrameSeries, GaugeFrame, GaugeHistory, NoteDistribution, SkinFrame, SkinObjectKind, SkinScreen, TimingHistogram,
};
use crate::{Color, CpuCanvas, QuadParams, Rect, Renderer, TextureId};

const CANVAS_W: u32 = 200;
const CANVAS_H: u32 = 160;

/// Where the judgement graph is placed, in document coordinates: twelve seconds of twenty rows, which
/// is a texture of exactly this size.
const JUDGE_X: u32 = 10;
const JUDGE_Y: u32 = 20;
const JUDGE_W: u32 = 60;
const JUDGE_H: u32 = 100;

/// Where the tempo graph is placed.
const BPM_X: u32 = 100;
const BPM_Y: u32 = 20;
const BPM_W: u32 = 61;
const BPM_H: u32 = 30;

/// The row of the canvas the foot of both graphs rests on, one below the lowest row they cover.
const FOOT: u32 = CANVAS_H - JUDGE_Y;

const SECONDS: usize = 12;

const WHITE: Color = Color::rgb(255, 255, 255);

/// A fixture document, its screen, and the canvas it is drawn on.
struct Rig {
    _scratch: Scratch,
    canvas: CpuCanvas,
    text: TextContext,
    screen: SkinScreen,
    timers: TimerState,
}

impl Rig {
    /// A document whose judgement graph and tempo graph are the records `judge` and `bpm` write.
    fn new(tag: &str, judge: &str, bpm: &str) -> Rig {
        let scratch = Scratch::new(tag);
        let body = format!(
            r#"{{
                "type": 6, "name": "graph fixture", "w": {CANVAS_W}, "h": {CANVAS_H},
                "judgegraph": [{judge}],
                "bpmgraph": [{bpm}],
                "destination": [
                    {{ "id": "judge-graph", "dst": [{{ "x": {JUDGE_X}, "y": {JUDGE_Y}, "w": {JUDGE_W}, "h": {JUDGE_H} }}] }},
                    {{ "id": "bpm-graph", "dst": [{{ "x": {BPM_X}, "y": {BPM_Y}, "w": {BPM_W}, "h": {BPM_H} }}] }}
                ]
            }}"#
        );
        let path = scratch.root.join("graphs.json");
        std::fs::write(&path, body).expect("the document is writable");
        let user = SkinUserConfig::default();
        let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
        let skin = load_skin(&path, options).expect("the generated document loads");

        let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
        let mut text = TextContext::embedded_only();
        let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut SolidAssets);
        Rig { _scratch: scratch, canvas, text, screen, timers: TimerState::new() }
    }

    /// The default fixture: both graphs as a record with nothing but an id writes them.
    fn plain(tag: &str) -> Rig {
        Rig::new(tag, r#"{ "id": "judge-graph" }"#, r#"{ "id": "bpm-graph" }"#)
    }

    /// Draws one frame at `now_ms` over a cleared canvas and answers how many objects reached it.
    fn draw(&mut self, series: FrameSeries<'_>, now_ms: i64) -> usize {
        let state = Nothing;
        let data = FrameData { series, ..FrameData::default() };
        let frame = SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers: &self.timers, state: &state, lua: None, mouse: None, data };
        self.canvas.clear(Color::BLACK);
        let mut ctx = RenderCtx::new(crate::theme::theme(), &mut self.text);
        self.screen.draw(&mut ctx, &mut self.canvas, &frame)
    }

    /// The pixel `(x, row)` of the judgement graph's texture, `row` counted up from its foot.
    fn judge(&self, x: u32, row: u32) -> Color {
        self.canvas.pixel_at(JUDGE_X + x, FOOT - 1 - row)
    }

    /// The pixel `(x, row)` of the tempo graph's texture, `row` counted up from its foot.
    fn tempo(&self, x: u32, row: u32) -> Color {
        self.canvas.pixel_at(BPM_X + x, FOOT - 1 - row)
    }
}

/// A run of twelve seconds with nothing in them.
fn quiet<const N: usize>() -> [[u32; N]; SECONDS] {
    [[0; N]; SECONDS]
}

fn notes_of<'a>(kinds: &'a [[u32; 7]]) -> FrameSeries<'a> {
    FrameSeries { notes: Some(NoteDistribution { kinds, ..NoteDistribution::default() }), ..FrameSeries::default() }
}

/// What a play screen reports of the judgements a run has taken so far.
fn playing(judgements: &[[u32; 6]], cursor: PlayCursor) -> FrameSeries<'_> {
    FrameSeries { notes: Some(NoteDistribution { judgements, playing: Some(cursor), ..NoteDistribution::default() }), ..FrameSeries::default() }
}

fn tempo_of(timeline: BpmTimeline<'_>) -> FrameSeries<'_> {
    FrameSeries { bpm: Some(timeline), ..FrameSeries::default() }
}

/// Counts the textures a renderer holds, and can be told it holds few of them.
struct Counting {
    inner: CpuCanvas,
    live: BTreeSet<u32>,
    limit: u32,
}

impl Counting {
    fn new() -> Counting {
        Counting { inner: CpuCanvas::new(CANVAS_W, CANVAS_H), live: BTreeSet::new(), limit: u32::MAX }
    }
}

impl Renderer for Counting {
    fn size(&self) -> (u32, u32) {
        self.inner.size()
    }

    fn clear(&mut self, color: Color) {
        self.inner.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.inner.fill_rect(rect, color);
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        let id = self.inner.register_texture(key, rgba, width, height);
        self.live.insert(id.0);
        id
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.live.remove(&tex.0);
        self.inner.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.inner.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.inner.draw_textured_quad(tex, params);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.inner.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.inner.pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        self.limit
    }
}

/// Twelve seconds in which the first holds two scratch notes, three key notes and a mine, and the
/// fourth a scratch long note's end and body.
fn busy() -> [[u32; 7]; SECONDS] {
    let mut rows = quiet();
    rows[0] = [0, 0, 2, 0, 0, 3, 1];
    rows[3] = [1, 1, 0, 0, 0, 0, 0];
    rows
}

#[test]
fn the_distribution_is_chips_stacked_from_the_foot_of_the_graph_in_the_colour_of_their_kind() {
    let mut rig = Rig::plain("dist-chips");
    let rows = busy();
    rig.draw(notes_of(&rows), 500);

    let scratch = Color::rgb(0xff, 0x44, 0x44);
    let key = Color::rgb(0xcc, 0xcc, 0xcc);
    let mine = Color::rgb(0x88, 0x00, 0x00);
    assert_eq!([0, 5].map(|row| rig.judge(1, row)), [scratch, scratch], "two scratch notes fill the two lowest rows");
    assert_eq!([3, 8].map(|row| rig.judge(3, row)), [scratch, scratch], "a chip is four pixels wide and four tall");
    assert_eq!([10, 15, 20].map(|row| rig.judge(1, row)), [key, key, key], "the three key notes sit on them");
    assert_eq!(rig.judge(1, 25), mine, "and the mine on those");
    assert_eq!(rig.judge(1, 30), Color::BLACK, "above the last note there is only the ground");
    assert_eq!(rig.judge(4, 0), Color::BLACK, "one pixel of ground follows a chip across");
    assert_eq!(rig.judge(1, 4), Color::BLACK, "and one above it");

    assert_eq!(rig.judge(15, 0), Color::rgb(0x44, 0xff, 0x44), "the fourth second starts with the first kind of note");
    assert_eq!(rig.judge(15, 5), Color::rgb(0x22, 0x88, 0x22), "and goes on with the second");
}

#[test]
fn the_ground_is_banded_and_ruled_behind_the_chips() {
    let mut rig = Rig::plain("dist-ground");
    let rows = quiet();
    rig.draw(notes_of(&rows), 500);

    assert_eq!(rig.judge(20, 10), Color::BLACK, "the field is black at four fifths opacity over a black canvas");
    assert_eq!(rig.judge(20, 60), Color::rgb(17, 17, 0), "the band ten rows up is dark yellow");
    assert_eq!(rig.judge(0, 10), Color::rgb(63, 63, 63), "second zero is a minute mark");
    assert_eq!(rig.judge(50, 10), Color::rgb(31, 31, 31), "second ten is a ten second mark");
    assert_eq!(rig.judge(25, 10), Color::BLACK);
}

#[test]
fn a_record_can_turn_the_ground_off() {
    let mut rig = Rig::new("dist-no-ground", r#"{ "id": "judge-graph", "backTexOff": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let mut rows = quiet();
    rows[0][2] = 1;
    rig.draw(notes_of(&rows), 500);

    assert_eq!(rig.judge(20, 60), Color::BLACK, "no band");
    assert_eq!(rig.judge(50, 10), Color::BLACK, "no ruler");
    assert_eq!(rig.judge(1, 1), Color::rgb(0xff, 0x44, 0x44), "and the chips are drawn all the same");
}

#[test]
fn the_chips_are_revealed_from_the_left_over_the_delay_and_the_ground_is_not() {
    let mut rig = Rig::plain("dist-reveal");
    let mut rows = quiet();
    rows[0][2] = 1;
    rows[8][2] = 1;
    let chip = Color::rgb(0xff, 0x44, 0x44);

    rig.draw(notes_of(&rows), 0);
    assert_eq!(rig.judge(1, 1), Color::BLACK, "nothing is revealed when the scene starts");
    assert_eq!(rig.judge(50, 10), Color::rgb(31, 31, 31), "but the ground is all there");

    rig.draw(notes_of(&rows), 250);
    assert_eq!(rig.judge(1, 1), chip, "half way through the delay the first half of the width is shown");
    assert_eq!(rig.judge(41, 1), Color::BLACK, "so a chip at second eight is not shown yet");

    rig.draw(notes_of(&rows), 500);
    assert_eq!(rig.judge(41, 1), chip, "and at the delay it is");
}

#[test]
fn a_record_chooses_the_gap_and_the_order() {
    let mut rig = Rig::new("dist-style", r#"{ "id": "judge-graph", "orderReverse": 1, "noGap": 1, "noGapX": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let mut rows = quiet();
    rows[0][2] = 1;
    rows[0][5] = 1;
    rig.draw(notes_of(&rows), 500);

    let key = Color::rgb(0xcc, 0xcc, 0xcc);
    let scratch = Color::rgb(0xff, 0x44, 0x44);
    assert_eq!(rig.judge(1, 1), key, "reversed, the key note comes first");
    assert_eq!(rig.judge(1, 6), scratch, "and the scratch note on it");
    assert_eq!(rig.judge(4, 4), key, "with no gap a chip fills its whole five by five pitch");
    assert_eq!(rig.judge(4, 9), scratch);
}

#[test]
fn the_other_two_types_count_judgements_in_their_own_colours() {
    let mut judged = Rig::new("dist-type1", r#"{ "id": "judge-graph", "type": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let mut rows: [[u32; 6]; SECONDS] = quiet();
    rows[0] = [2, 1, 0, 0, 0, 1];
    judged.draw(FrameSeries { notes: Some(NoteDistribution { judgements: &rows, ..NoteDistribution::default() }), ..FrameSeries::default() }, 500);
    assert_eq!([0, 5, 10, 15].map(|row| judged.judge(1, row)), [0x555555, 0x555555, 0x0088ff, 0xff0000].map(rgb), "unjudged, then the best, then the poor");

    let mut early_late = Rig::new("dist-type2", r#"{ "id": "judge-graph", "type": 2 }"#, r#"{ "id": "bpm-graph" }"#);
    let mut rows: [[u32; 10]; SECONDS] = quiet();
    rows[0] = [0, 1, 0, 0, 0, 0, 0, 0, 0, 1];
    early_late.draw(FrameSeries { notes: Some(NoteDistribution { early_late: &rows, ..NoteDistribution::default() }), ..FrameSeries::default() }, 500);
    assert_eq!([0, 5].map(|row| early_late.judge(1, row)), [0x44ff44, 0x442200].map(rgb), "the best judgement, then the poorest hit late");

    let mut popn = Rig::new("dist-popn", r#"{ "id": "judge-graph", "type": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let mut rows: [[u32; 6]; SECONDS] = quiet();
    rows[0][1] = 1;
    let notes = NoteDistribution { judgements: &rows, popn: true, ..NoteDistribution::default() };
    popn.draw(FrameSeries { notes: Some(notes), ..FrameSeries::default() }, 500);
    assert_eq!(popn.judge(1, 0), rgb(0xff5eb0), "pop'n colours its judgements differently");
}

fn rgb(hex: u32) -> Color {
    let [_, r, g, b] = hex.to_be_bytes();
    Color::rgb(r, g, b)
}

#[test]
fn a_graph_asks_for_the_series_of_its_own_type_and_draws_nothing_without_it() {
    let mut rig = Rig::plain("dist-missing");
    let rows: [[u32; 6]; SECONDS] = quiet();
    let counts = [1, 0, 0, 0, 0, 0];
    let wrong = FrameSeries { notes: Some(NoteDistribution { judgements: &rows, ..NoteDistribution::of_judgements(&counts) }), ..FrameSeries::default() };
    assert_eq!(rig.draw(wrong, 500), 0, "a type 0 graph does not read the judgements, and the totals have no seconds");
    assert_eq!(rig.judge(50, 10), Color::BLACK, "so not even its ground is drawn");

    assert_eq!(rig.draw(FrameSeries::default(), 500), 0, "and neither does a frame with no distribution at all");
    assert_eq!(rig.draw(notes_of(&[]), 500), 0, "or one with no seconds in it");
}

#[test]
fn a_type_the_reference_has_no_graph_for_draws_nothing_and_says_so() {
    let mut rig = Rig::new("dist-type3", r#"{ "id": "judge-graph", "type": 3 }"#, r#"{ "id": "bpm-graph" }"#);
    assert!(rig.screen.warnings().iter().any(|warning| warning.contains("judge-graph") && warning.contains("type 3")), "{:?}", rig.screen.warnings());
    let rows = busy();
    assert_eq!(rig.draw(notes_of(&rows), 500), 0);
}

#[test]
fn a_new_set_of_numbers_of_the_same_length_is_painted_at_once_off_the_play_screen() {
    let mut rig = Rig::plain("dist-new-song");
    let mut first = quiet();
    first[0][2] = 1;
    let mut second = quiet();
    second[0][5] = 1;

    rig.draw(notes_of(&first), 500);
    assert_eq!(rig.judge(1, 1), Color::rgb(0xff, 0x44, 0x44));
    rig.draw(notes_of(&second), 510);
    assert_eq!(rig.judge(1, 1), Color::rgb(0xcc, 0xcc, 0xcc), "another chart, however long it is, replaces the first as soon as it is drawn");
}

#[test]
fn on_the_play_screen_the_judgements_are_compared_with_the_screen_only_every_750_ms() {
    let mut rig = Rig::new("dist-live", r#"{ "id": "judge-graph", "type": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let mut before: [[u32; 6]; SECONDS] = quiet();
    before[0][0] = 1;
    let mut after: [[u32; 6]; SECONDS] = quiet();
    after[0][1] = 1;
    let live = |rows| playing(rows, PlayCursor::default());

    rig.draw(live(&before), 1_000);
    assert_eq!(rig.judge(1, 1), rgb(0x555555), "the first frame paints what there is");
    rig.draw(live(&after), 1_100);
    assert_eq!(rig.judge(1, 1), rgb(0x555555), "a hit a moment later is not on the graph yet");
    rig.draw(live(&after), 1_750);
    assert_eq!(rig.judge(1, 1), rgb(0x555555), "nor at exactly 750 ms after the last look");
    rig.draw(live(&after), 1_751);
    assert_eq!(rig.judge(1, 1), rgb(0x0088ff), "but it is after");
}

#[test]
fn the_play_screen_draws_the_range_and_the_position_and_moves_them_every_50_ms() {
    let mut rig = Rig::new("dist-cursor", r#"{ "id": "judge-graph", "type": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let rows: [[u32; 6]; SECONDS] = quiet();
    rig.timers.set_on(timer_id::PLAY, 0);
    let cursor = PlayCursor { start_ms: Some(2_000), end_ms: Some(8_000), speed: Some(8.0) };
    let live = |rows| playing(rows, cursor);

    rig.draw(live(&rows), 100);
    assert_eq!(rig.judge(10, 20), Color::rgb(128, 255, 128), "the start of the range, two seconds of twelve across");
    assert_eq!(rig.judge(40, 20), Color::rgb(255, 128, 128), "and its end");
    assert_eq!(rig.judge(4, 20), WHITE, "the run is 100 ms in, eight times over");
    assert_eq!(rig.judge(8, 20), Color::BLACK);

    rig.draw(live(&rows), 140);
    assert_eq!(rig.judge(4, 20), WHITE, "forty milliseconds later the cursor has not moved");
    assert_eq!(rig.judge(8, 20), Color::BLACK);
    rig.draw(live(&rows), 160);
    assert_eq!(rig.judge(8, 20), WHITE, "and after fifty it has");
    assert_eq!(rig.judge(4, 20), Color::BLACK);

    let shorter: [[u32; 6]; 6] = [[0; 6]; 6];
    rig.draw(live(&shorter), 170);
    assert_eq!(rig.judge(12, 20), WHITE, "a chart that replaces the first has its cursor painted at once, on its own, narrower texture");

    let mut quiet_rig = Rig::new("dist-no-cursor", r#"{ "id": "judge-graph", "type": 1 }"#, r#"{ "id": "bpm-graph" }"#);
    let on_result = FrameSeries { notes: Some(NoteDistribution { judgements: &rows, ..NoteDistribution::default() }), ..FrameSeries::default() };
    quiet_rig.draw(on_result, 500);
    assert_eq!(quiet_rig.judge(10, 20), Color::BLACK, "off the play screen there is no cursor");
}

#[test]
fn a_texture_the_renderer_cannot_hold_is_not_built() {
    let mut rig = Rig::plain("dist-too-wide");
    let mut tracked = Counting::new();
    tracked.limit = (SECONDS * 5 - 1) as u32;
    let rows = busy();
    let timers = TimerState::new();
    let state = Nothing;
    let data = FrameData { series: notes_of(&rows), ..FrameData::default() };
    let frame = SkinFrame { now_us: 500 * MICROS_PER_MILLI, timers: &timers, state: &state, lua: None, mouse: None, data };
    let mut ctx = RenderCtx::new(crate::theme::theme(), &mut rig.text);

    assert_eq!(rig.screen.draw(&mut ctx, &mut tracked, &frame), 0, "twelve seconds need sixty texels, one more than this renderer holds");
    assert!(tracked.live.is_empty(), "so nothing was uploaded");
}

#[test]
fn the_tempo_line_is_drawn_with_the_fastest_tempo_at_the_top() {
    let mut rig = Rig::plain("bpm-steps");
    let changes = [(150.0, 0.0), (300.0, 30_000.0), (150.0, 60_000.0)];
    rig.draw(tempo_of(BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, Some(60_000))), 0);

    let main = Color::rgb(0, 255, 0);
    let highest = Color::rgb(255, 0, 0);
    assert_eq!([14, 15].map(|row| rig.tempo(10, row)), [main, main], "the opening tempo runs along the main level");
    assert_eq!(rig.tempo(10, 13), Color::BLACK);
    assert_eq!([18, 19].map(|row| rig.tempo(45, row)), [highest, highest], "and the faster half runs above it");
    assert_eq!(rig.tempo(30, 16), Color::rgb(127, 127, 127), "the two are joined by an upright");
    assert_eq!(rig.tempo(60, 14), main, "and the last tempo runs to the right edge");
}

#[test]
fn the_tempo_line_is_revealed_from_the_left_over_its_delay() {
    let mut rig = Rig::new("bpm-delay", r#"{ "id": "judge-graph" }"#, r#"{ "id": "bpm-graph", "delay": 1000 }"#);
    let changes = [(150.0, 0.0), (300.0, 30_000.0), (150.0, 60_000.0)];
    let timeline = || tempo_of(BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, Some(60_000)));

    rig.draw(timeline(), 0);
    assert_eq!(rig.tempo(10, 14), Color::BLACK, "nothing yet");
    rig.draw(timeline(), 500);
    assert_eq!(rig.tempo(10, 14), Color::rgb(0, 255, 0), "half the width is shown after half the delay");
    assert_eq!(rig.tempo(45, 18), Color::BLACK, "30 of 61 pixels");
    rig.draw(timeline(), 1_000);
    assert_eq!(rig.tempo(45, 18), Color::rgb(255, 0, 0));
}

#[test]
fn a_record_chooses_the_colours_and_the_thickness_of_the_tempo_line() {
    let record = r##"{ "id": "bpm-graph", "lineWidth": 4, "mainBPMColor": "#ff800080", "transitionLineColor": "00ffff" }"##;
    let mut rig = Rig::new("bpm-style", r#"{ "id": "judge-graph" }"#, record);
    let changes = [(150.0, 0.0), (300.0, 30_000.0)];
    rig.draw(tempo_of(BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, Some(60_000))), 0);

    let main = Color::rgb(0xff, 0x80, 0x00);
    let level = (0.5 * f64::from(BPM_H - 4)) as u32;
    assert_eq!([0, 3].map(|row| rig.tempo(5, level + row)), [main, main], "four pixels thick, in the first six digits of what was written");
    assert_eq!(rig.tempo(5, level + 4), Color::BLACK);
    assert_eq!(rig.tempo(5, level - 1), Color::BLACK);
}

#[test]
fn a_tempo_graph_that_has_nothing_to_join_draws_nothing() {
    let mut rig = Rig::plain("bpm-blank");
    assert_eq!(rig.draw(tempo_of(BpmTimeline::of_chart(&[(150.0, 0.0)], 150.0, 150.0, 150.0, None)), 0), 0);
    assert_eq!(rig.draw(tempo_of(BpmTimeline::of_chart(&[(150.0, 0.0), (160.0, 100.0)], 0.0, 0.0, 0.0, None)), 0), 0, "and neither does a main tempo of zero");
    assert_eq!(rig.draw(FrameSeries::default(), 0), 0);
}

#[test]
fn progress_points_still_draw_a_tempo_line() {
    let mut rig = Rig::plain("bpm-progress");
    let points = [(0.0, 150.0), (0.5, 300.0)];
    assert_eq!(rig.draw(tempo_of(BpmTimeline::new(&points)), 0), 1);
    assert_eq!(rig.tempo(10, 14), Color::rgb(0, 255, 0), "the first tempo is the main one");
    assert_eq!(rig.tempo(50, 18), Color::rgb(255, 0, 0), "and the second runs higher, to the right edge");
}

#[test]
fn the_tempo_line_is_painted_again_only_when_the_chart_changes() {
    let mut rig = Rig::plain("bpm-song");
    let first = [(150.0, 0.0), (300.0, 30_000.0), (150.0, 60_000.0)];
    let second = [(150.0, 0.0), (75.0, 30_000.0), (150.0, 60_000.0)];
    rig.draw(tempo_of(BpmTimeline::of_chart(&first, 150.0, 150.0, 300.0, Some(60_000))), 0);
    assert_eq!(rig.tempo(45, 18), Color::rgb(255, 0, 0));
    rig.draw(tempo_of(BpmTimeline::of_chart(&second, 150.0, 75.0, 150.0, Some(60_000))), 0);
    assert_eq!(rig.tempo(45, 18), Color::BLACK, "another chart takes the first one's place");
    assert_eq!(rig.tempo(45, 9), Color::rgb(0, 0, 255), "in the colour of the slowest tempo");
}

#[test]
fn both_graphs_hand_their_textures_back_with_the_screen() {
    let scratch = Scratch::new("release");
    let body = format!(
        r#"{{ "type": 6, "name": "graph fixture", "w": {CANVAS_W}, "h": {CANVAS_H},
            "judgegraph": [{{ "id": "judge-graph", "type": 1 }}], "bpmgraph": [{{ "id": "bpm-graph" }}],
            "destination": [
                {{ "id": "judge-graph", "dst": [{{ "x": {JUDGE_X}, "y": {JUDGE_Y}, "w": {JUDGE_W}, "h": {JUDGE_H} }}] }},
                {{ "id": "bpm-graph", "dst": [{{ "x": {BPM_X}, "y": {BPM_Y}, "w": {BPM_W}, "h": {BPM_H} }}] }}
            ] }}"#
    );
    let path = scratch.root.join("graphs.json");
    std::fs::write(&path, body).expect("the document is writable");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&path, options).expect("the generated document loads");

    let mut tracked = Counting::new();
    let mut text = TextContext::embedded_only();
    let mut screen = SkinScreen::build(&mut tracked, &mut text, &skin, &mut SolidAssets);
    assert_eq!(screen.count_of(SkinObjectKind::JudgeGraph) + screen.count_of(SkinObjectKind::BpmGraph), 2);
    assert!(tracked.live.is_empty(), "a screen uploads nothing for a graph until it draws it");

    let judgements: [[u32; 6]; SECONDS] = quiet();
    let changes = [(150.0, 0.0), (300.0, 30_000.0)];
    let series = FrameSeries {
        notes: Some(NoteDistribution { judgements: &judgements, ..NoteDistribution::default() }),
        bpm: Some(BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, Some(60_000))),
        ..FrameSeries::default()
    };
    let timers = TimerState::new();
    let state = Nothing;
    let frame = SkinFrame {
        now_us: 500 * MICROS_PER_MILLI,
        timers: &timers,
        state: &state,
        lua: None,
        mouse: None,
        data: FrameData { series, ..FrameData::default() },
    };
    let mut ctx = RenderCtx::new(crate::theme::theme(), &mut text);
    assert_eq!(screen.draw(&mut ctx, &mut tracked, &frame), 2);
    assert_eq!(tracked.live.len(), 3, "the distribution's ground and chips, and the tempo line");

    screen.draw(&mut ctx, &mut tracked, &frame);
    assert_eq!(tracked.live.len(), 3, "drawing again uploads nothing new");

    screen.release(&mut tracked);
    assert!(tracked.live.is_empty(), "and releasing the screen hands every one of them back: {:?}", tracked.live);
}

#[test]
fn the_gauge_and_timing_graphs_hand_their_textures_back_with_the_screen() {
    let scratch = Scratch::new("release-run");
    let body = format!(
        r#"{{ "type": 7, "name": "run graph fixture", "w": {CANVAS_W}, "h": {CANVAS_H},
            "gaugegraph": [{{ "id": "gauge-graph" }}], "timingdistributiongraph": [{{ "id": "timing-graph" }}],
            "destination": [
                {{ "id": "gauge-graph", "dst": [{{ "x": {JUDGE_X}, "y": {JUDGE_Y}, "w": {JUDGE_W}, "h": {JUDGE_H} }}] }},
                {{ "id": "timing-graph", "dst": [{{ "x": {BPM_X}, "y": {BPM_Y}, "w": {BPM_W}, "h": {BPM_H} }}] }}
            ] }}"#
    );
    let path = scratch.root.join("run-graphs.json");
    std::fs::write(&path, body).expect("the document is writable");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&path, options).expect("the generated document loads");

    let mut tracked = Counting::new();
    let mut text = TextContext::embedded_only();
    let mut screen = SkinScreen::build(&mut tracked, &mut text, &skin, &mut SolidAssets);
    assert_eq!(screen.count_of(SkinObjectKind::GaugeGraph) + screen.count_of(SkinObjectKind::TimingDistribution), 2);
    assert!(tracked.live.is_empty(), "a screen uploads nothing for a graph until it draws it");

    let samples = [20.0, 40.0, 60.0];
    let bins = [0, 3, 9, 3, 0];
    let series = FrameSeries { gauge_history: Some(GaugeHistory::new(&samples)), timing: Some(TimingHistogram::new(&bins)), ..FrameSeries::default() };
    let gauge = GaugeFrame::finished(2, 60.0, [GaugeScale::new(2.0, 100.0, 80.0); GAUGE_TYPES]);
    let timers = TimerState::new();
    let state = Nothing;
    let frame = SkinFrame {
        now_us: 500 * MICROS_PER_MILLI,
        timers: &timers,
        state: &state,
        lua: None,
        mouse: None,
        data: FrameData { series, gauge: Some(gauge), ..FrameData::default() },
    };
    let mut ctx = RenderCtx::new(crate::theme::theme(), &mut text);
    assert_eq!(screen.draw(&mut ctx, &mut tracked, &frame), 2);
    assert_eq!(tracked.live.len(), 3, "the gauge graph's ground and line, and the timing spread");

    screen.draw(&mut ctx, &mut tracked, &frame);
    assert_eq!(tracked.live.len(), 3, "drawing again uploads nothing new");

    screen.release(&mut tracked);
    assert!(tracked.live.is_empty(), "and releasing the screen hands every one of them back: {:?}", tracked.live);
}

#[test]
fn a_screen_drawn_onto_another_renderer_uploads_its_graphs_there() {
    let mut rig = Rig::plain("dist-other-canvas");
    let rows = busy();
    rig.draw(notes_of(&rows), 500);

    let mut other = CpuCanvas::new(CANVAS_W, CANVAS_H);
    other.clear(Color::BLACK);
    let state = Nothing;
    let data = FrameData { series: notes_of(&rows), ..FrameData::default() };
    let frame = SkinFrame { now_us: 500 * MICROS_PER_MILLI, timers: &rig.timers, state: &state, lua: None, mouse: None, data };
    let mut ctx = RenderCtx::new(crate::theme::theme(), &mut rig.text);
    assert_eq!(rig.screen.draw(&mut ctx, &mut other, &frame), 1);
    assert_eq!(other.pixel_at(JUDGE_X + 1, FOOT - 1), Color::rgb(0xff, 0x44, 0x44), "the chips are on the second canvas too");
}

#[test]
fn a_judgement_graph_gives_way_to_a_later_record_with_the_same_id() {
    let scratch = Scratch::new("break-same");
    let body = format!(
        r#"{{ "type": 6, "name": "same id", "w": {CANVAS_W}, "h": {CANVAS_H},
            "judgegraph": [{{ "id": "same" }}], "bpmgraph": [{{ "id": "same" }}],
            "destination": [{{ "id": "same", "dst": [{{ "x": 0, "y": 0, "w": 50, "h": 50 }}] }}] }}"#
    );
    let path = scratch.root.join("same.json");
    std::fs::write(&path, body).expect("the document is writable");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&path, options).expect("the generated document loads");
    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut SolidAssets);

    let kinds = (screen.count_of(SkinObjectKind::JudgeGraph), screen.count_of(SkinObjectKind::BpmGraph));
    assert_eq!(kinds, (0, 1), "the loader breaks out of the judgement graph and goes on to the tempo graph, which it finds");
}
