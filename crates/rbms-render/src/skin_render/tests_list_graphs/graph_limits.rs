//! What each graph does when the pixmap it would paint is past what a graph may hold.
//!
//! A graph's pixmap is as large as something else says: the destination it is drawn over, a number
//! its record gives, or the length of the series a screen hands it. Each fixture here is one graph
//! on a renderer that names no longest edge, drawn once at an ordinary size and once with the one
//! thing that sizes its pixmap put just past the bound. The first draws and uploads a texture; the
//! second draws nothing and uploads none, which is what shows the graph asked before it painted.

use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::timer::TimerState;

use super::graph_pixels::{CANVAS_H, CANVAS_W, Counting};
use super::{HARD_GAUGE, Nothing, Scratch, SolidAssets, shown};
use crate::ctx::RenderCtx;
use crate::font::{BLOCK_MAX_PIXELS, TextContext};
use crate::skin_render::{BpmTimeline, FrameData, FrameSeries, GaugeHistory, NoteDistribution, RecentHits, SkinFrame, SkinScreen, TimingHistogram};
use crate::{Renderer, UNLIMITED_TEXTURE_SIZE};

/// The extent of a destination a graph is ordinarily drawn over.
const ORDINARY: (u32, u32) = (60, 40);

/// The edge of a square destination one row and one column past the largest a graph paints.
const PAST_SQUARE_EDGE: u32 = 4097;

/// The judgement graph paints this many pixels to a second and is asked for at its tallest.
const JUDGE_PIXELS_PER_SECOND: u64 = 5;
const JUDGE_TALLEST: u64 = 500;

/// The most seconds the judgement graph paints: one more is past the bound.
const MOST_SECONDS: usize = 6710;

/// A timing graph this many columns wide, over a bin this full, is past the bound: the bin is
/// rounded up to [`PAST_ROWS`].
const PAST_COLUMNS: u32 = 4100;
const PAST_BIN: u32 = 4100;
const PAST_ROWS: u64 = 4110;

/// A hit error strip keeps at most this many hits at [`ROWS_PER_HIT`] each, so this width is past
/// the bound.
const PAST_STRIP_WIDTH: u32 = 83_887;
const MOST_HITS: u32 = 100;
const ROWS_PER_HIT: u64 = 2;

/// A ruler reaching this many milliseconds either way is one column past the bound.
const PAST_RULER_REACH: u32 = 8_388_608;

/// The judgement windows the visualisers are drawn against.
const WINDOWS: [[i32; 2]; 5] = [[-8, 8], [-20, 20], [-40, 40], [-70, 50], [-150, 150]];

/// How many objects one frame of a document holding the single graph `record` drew, and how many
/// textures the renderer held afterwards. The graph is the `section` record `id`, drawn over a
/// destination of `extent`.
fn drawn(tag: &str, section: &str, record: &str, extent: (u32, u32), series: FrameSeries<'_>) -> (usize, usize) {
    let scratch = Scratch::new(tag);
    let (width, height) = extent;
    let body = format!(
        r#"{{
            "type": 6, "name": "limit fixture", "w": {CANVAS_W}, "h": {CANVAS_H},
            "{section}": [{record}],
            "destination": [{{ "id": "graph", "dst": [{{ "x": 0, "y": 0, "w": {width}, "h": {height} }}] }}]
        }}"#
    );
    let path = scratch.root.join("limit.json");
    std::fs::write(&path, body).expect("the document is writable");
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&path, options).expect("the generated document loads");

    let mut renderer = Counting::new();
    assert_eq!(renderer.max_texture_size(), UNLIMITED_TEXTURE_SIZE, "the renderer names a longest edge after all");
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(&mut renderer, &mut text, &skin, &mut SolidAssets);
    let before = renderer.live.len();
    let timers = TimerState::new();
    let state = Nothing;
    let gauge = series.gauge_history.map(|_| shown(HARD_GAUGE, 0.0));
    let data = FrameData { series, gauge, ..FrameData::default() };
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &state, lua: None, mouse: None, data };
    let mut ctx = RenderCtx::new(crate::theme::theme(), &mut text);
    let objects = screen.draw(&mut ctx, &mut renderer, &frame);
    (objects, renderer.live.len() - before)
}

/// Asserts that the graph drew and uploaded what it painted.
fn assert_painted(what: &str, (objects, textures): (usize, usize)) {
    assert_eq!(objects, 1, "{what}: the graph did not draw at an ordinary size, so its limit is not what the fixture reaches");
    assert!(textures > 0, "{what}: the graph drew without a texture");
}

/// Asserts that the graph drew nothing and uploaded nothing.
fn assert_refused(what: &str, (objects, textures): (usize, usize)) {
    assert_eq!(objects, 0, "{what}: a graph past the bound was drawn");
    assert_eq!(textures, 0, "{what}: a graph past the bound was painted and uploaded");
}

#[test]
fn the_fixtures_reach_just_past_the_bound_and_no_further() {
    let bound = BLOCK_MAX_PIXELS as u64;
    let square = |edge: u32| u64::from(edge) * u64::from(edge);
    let judge = |seconds: usize| seconds as u64 * JUDGE_PIXELS_PER_SECOND * JUDGE_TALLEST;
    let strip = |width: u32| u64::from(width) * u64::from(MOST_HITS) * ROWS_PER_HIT;
    let ruler = |reach: u32| u64::from(reach) * 2 + 1;
    assert!(square(PAST_SQUARE_EDGE) > bound && square(PAST_SQUARE_EDGE - 1) <= bound);
    assert!(judge(MOST_SECONDS + 1) > bound && judge(MOST_SECONDS) <= bound);
    assert!(u64::from(PAST_COLUMNS) * PAST_ROWS > bound);
    assert!(strip(PAST_STRIP_WIDTH) > bound && strip(PAST_STRIP_WIDTH - 1) <= bound);
    assert!(ruler(PAST_RULER_REACH) > bound && ruler(PAST_RULER_REACH - 1) <= bound);
}

#[test]
fn a_tempo_graph_over_a_destination_past_the_bound_is_not_painted() {
    let changes = [(150.0, 0.0), (300.0, 30_000.0), (150.0, 60_000.0)];
    let series = || FrameSeries { bpm: Some(BpmTimeline::of_chart(&changes, 150.0, 150.0, 300.0, Some(60_000))), ..FrameSeries::default() };
    let record = r#"{ "id": "graph" }"#;
    assert_painted("tempo", drawn("limit-bpm", "bpmgraph", record, ORDINARY, series()));
    assert_refused("tempo", drawn("limit-bpm-past", "bpmgraph", record, (PAST_SQUARE_EDGE, PAST_SQUARE_EDGE), series()));
}

#[test]
fn a_gauge_graph_over_a_destination_past_the_bound_is_not_painted() {
    let samples = [20.0, 40.0, 60.0, 50.0];
    let series = || FrameSeries { gauge_history: Some(GaugeHistory::new(&samples)), ..FrameSeries::default() };
    let record = r#"{ "id": "graph" }"#;
    assert_painted("gauge", drawn("limit-gauge", "gaugegraph", record, ORDINARY, series()));
    assert_refused("gauge", drawn("limit-gauge-past", "gaugegraph", record, (PAST_SQUARE_EDGE, PAST_SQUARE_EDGE), series()));
}

#[test]
fn a_judgement_graph_of_a_chart_too_long_for_the_bound_is_not_painted() {
    let seconds = |count: usize| {
        let mut rows = vec![[0u32; 7]; count];
        rows[0] = [0, 0, 2, 0, 0, 3, 1];
        rows
    };
    let record = r#"{ "id": "graph" }"#;
    let (most, past) = (seconds(MOST_SECONDS), seconds(MOST_SECONDS + 1));
    let series = |kinds| FrameSeries { notes: Some(NoteDistribution { kinds, ..NoteDistribution::default() }), ..FrameSeries::default() };
    assert_painted("judgement", drawn("limit-judge", "judgegraph", record, ORDINARY, series(&most)));
    assert_refused("judgement", drawn("limit-judge-past", "judgegraph", record, ORDINARY, series(&past)));
}

#[test]
fn a_timing_graph_whose_columns_and_rows_are_past_the_bound_is_not_painted() {
    let (spread, tall) = ([1, 4, 9, 4, 1], [1, 4, PAST_BIN, 4, 1]);
    let series = |bins| FrameSeries { timing: Some(TimingHistogram::new(bins)), ..FrameSeries::default() };
    let wide = format!(r#"{{ "id": "graph", "width": {PAST_COLUMNS}, "lineWidth": 1 }}"#);
    assert_painted("timing", drawn("limit-timing", "timingdistributiongraph", &wide, ORDINARY, series(&spread)));
    assert_refused("timing", drawn("limit-timing-past", "timingdistributiongraph", &wide, ORDINARY, series(&tall)));
}

#[test]
fn a_hit_error_strip_wider_than_the_bound_allows_is_not_painted() {
    let hits = [(-35_i64, 0_u8), (20, 1)];
    let series = || FrameSeries { recent_hits: Some(RecentHits::new(&hits).with_judge_area(WINDOWS)), ..FrameSeries::default() };
    let ordinary = format!(r#"{{ "id": "graph", "windowLength": {MOST_HITS} }}"#);
    let wide = format!(r#"{{ "id": "graph", "windowLength": {MOST_HITS}, "width": {PAST_STRIP_WIDTH} }}"#);
    assert_painted("hit error", drawn("limit-errors", "hiterrorvisualizer", &ordinary, ORDINARY, series()));
    assert_refused("hit error", drawn("limit-errors-past", "hiterrorvisualizer", &wide, ORDINARY, series()));
}

#[test]
fn a_timing_ruler_reaching_further_than_the_bound_allows_is_not_painted() {
    let hits = [(-35_i64, 0_u8), (20, 1)];
    let series = || FrameSeries { recent_hits: Some(RecentHits::new(&hits).with_judge_area(WINDOWS)), ..FrameSeries::default() };
    let far = format!(r#"{{ "id": "graph", "judgeWidthMillis": {PAST_RULER_REACH} }}"#);
    assert_painted("ruler", drawn("limit-ruler", "timingvisualizer", r#"{ "id": "graph" }"#, ORDINARY, series()));
    assert_refused("ruler", drawn("limit-ruler-past", "timingvisualizer", &far, ORDINARY, series()));
}
