//! What the timing visualiser and the hit error visualiser put on a canvas, checked pixel by pixel.
//!
//! Each fixture is drawn with the records' own defaults for size (a ruler of 301 pixels over 301
//! milliseconds, one pixel to each) on rectangles the size of what they paint, so a millisecond of
//! error is a pixel and the reference's arithmetic can be addressed in canvas coordinates.

use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::timer::TimerState;

use super::{Nothing, Scratch, SolidAssets};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::{FrameData, FrameSeries, RecentHits, SkinFrame, SkinScreen};
use crate::{Color, CpuCanvas, Renderer};

/// The size the fixture document is authored at.
const DOC_W: u32 = 320;
const DOC_H: u32 = 160;

/// The width of both records, which is also how many milliseconds either way their rulers reach
/// plus the middle one.
const RECORD_W: u32 = 301;

/// Where the ruler sits in the document (from the foot) and its height, and where it lands on a
/// canvas at the authored size (from the top).
const RULER_X: u32 = 10;
const RULER_Y: u32 = 120;
const RULER_H: u32 = 20;
const RULER_TOP: u32 = DOC_H - RULER_Y - RULER_H;

/// Where the mirrored copy of the ruler sits: the same span, written from its right edge with a
/// negative width.
const MIRROR_Y: u32 = 90;
const MIRROR_X: u32 = RULER_X + RECORD_W;
const MIRROR_TOP: u32 = DOC_H - MIRROR_Y - RULER_H;

/// The strip's place and height, a window of thirty hits tall, and where it lands from the top.
const STRIP_X: u32 = 10;
const STRIP_Y: u32 = 10;
const STRIP_H: u32 = 60;
const STRIP_TOP: u32 = DOC_H - STRIP_Y - STRIP_H;

/// The column of a 301 pixel record that is the middle: 150 milliseconds in.
const MIDDLE: u32 = 150;

/// Windows that put the best judgement within eight milliseconds of the middle, the next within
/// twenty, the third within forty and the bad one seventy late and fifty early.
const WINDOWS: [[i32; 2]; 5] = [[-8, 8], [-20, 20], [-40, 40], [-70, 50], [-150, 150]];

const PERFECT: Color = Color::rgb(0x00, 0x00, 0xFF);
const GREAT: Color = Color::rgb(0x00, 0xFF, 0x00);
const GOOD: Color = Color::rgb(0xFF, 0xFF, 0x00);
const BAD: Color = Color::rgb(0xFF, 0x00, 0x00);
const POOR: Color = Color::rgb(0x80, 0x80, 0x80);

/// A fixture document, its screen, and the canvas it is drawn on.
struct Rig {
    _scratch: Scratch,
    canvas: CpuCanvas,
    text: TextContext,
    screen: SkinScreen,
    timers: TimerState,
}

impl Rig {
    /// A document whose two records are written out in full, on a canvas `scale` times its size.
    fn new(tag: &str, scale: u32) -> Rig {
        let scratch = Scratch::new(tag);
        let body = format!(
            r#"{{
                "type": 6, "name": "visualiser fixture", "w": {DOC_W}, "h": {DOC_H},
                "timingvisualizer": [{{
                    "id": "ruler", "drawDecay": 0,
                    "PGColor": "0000FFFF", "GRColor": "00FF00FF", "GDColor": "FFFF00FF", "BDColor": "FF0000FF", "PRColor": "808080FF"
                }}],
                "hiterrorvisualizer": [{{
                    "id": "errors", "drawDecay": 0, "emaMode": 0,
                    "PGColor": "0000FFFF", "GRColor": "00FF00FF", "GDColor": "FFFF00FF", "BDColor": "FF0000FF", "PRColor": "808080FF"
                }}],
                "destination": [
                    {{ "id": "ruler", "dst": [{{ "x": {RULER_X}, "y": {RULER_Y}, "w": {RECORD_W}, "h": {RULER_H} }}] }},
                    {{ "id": "ruler", "dst": [{{ "x": {MIRROR_X}, "y": {MIRROR_Y}, "w": -{RECORD_W}, "h": {RULER_H} }}] }},
                    {{ "id": "errors", "dst": [{{ "x": {STRIP_X}, "y": {STRIP_Y}, "w": {RECORD_W}, "h": {STRIP_H} }}] }}
                ]
            }}"#
        );
        let path = scratch.root.join("visualisers.json");
        std::fs::write(&path, body).expect("the document is writable");
        let user = SkinUserConfig::default();
        let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
        let skin = load_skin(&path, options).expect("the generated document loads");

        let mut canvas = CpuCanvas::new(DOC_W * scale, DOC_H * scale);
        let mut text = TextContext::embedded_only();
        let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut SolidAssets);
        Rig { _scratch: scratch, canvas, text, screen, timers: TimerState::new() }
    }

    /// Draws one frame over a cleared canvas and answers how many objects reached it.
    fn draw(&mut self, recent: Option<RecentHits<'_>>) -> usize {
        let state = Nothing;
        let data = FrameData { series: FrameSeries { recent_hits: recent, ..FrameSeries::default() }, ..FrameData::default() };
        let frame = SkinFrame { now_us: 0, timers: &self.timers, state: &state, lua: None, mouse: None, data };
        self.canvas.clear(Color::BLACK);
        let mut ctx = RenderCtx::new(crate::theme::theme(), &mut self.text);
        self.screen.draw(&mut ctx, &mut self.canvas, &frame)
    }

    /// The pixel of the ruler at the column `x` of its texture, `row` down from its top.
    fn ruler(&self, x: u32, row: u32) -> Color {
        self.canvas.pixel_at(RULER_X + x, RULER_TOP + row)
    }

    /// The pixel of the mirrored ruler at the column `x` of its span, `row` down from its top.
    fn mirror(&self, x: u32, row: u32) -> Color {
        self.canvas.pixel_at(RULER_X + x, MIRROR_TOP + row)
    }

    /// The pixel of the strip at the column `x` of its texture, `row` down from its top.
    fn strip(&self, x: u32, row: u32) -> Color {
        self.canvas.pixel_at(STRIP_X + x, STRIP_TOP + row)
    }
}

#[test]
fn a_frame_without_hits_draws_neither_visualiser() {
    let mut rig = Rig::new("vis-none", 1);
    assert_eq!(rig.draw(None), 0, "a screen that is not a play screen carries no recent hits");
    assert_eq!(rig.ruler(MIDDLE, 5), Color::BLACK);
    assert_eq!(rig.strip(MIDDLE, 5), Color::BLACK);
}

#[test]
fn the_ruler_shows_each_window_and_a_hit_that_came_early_is_a_green_line_to_the_right() {
    let mut rig = Rig::new("vis-ruler", 1);
    let hits = [(-35_i64, 0_u8), (20, 1)];
    assert_eq!(rig.draw(Some(RecentHits::new(&hits).with_judge_area(WINDOWS))), 3, "the ruler, its mirrored copy and the strip");

    assert_eq!(rig.ruler(MIDDLE - 3, 5), PERFECT, "inside the best window");
    assert_eq!(rig.ruler(MIDDLE + 12, 5), GREAT, "inside the next");
    assert_eq!(rig.ruler(MIDDLE + 31, 5), GOOD);
    assert_eq!(rig.ruler(MIDDLE - 61, 5), BAD);
    assert_eq!(rig.ruler(MIDDLE, 5), Color::rgb(192, 192, 192), "the middle pixel is white with a tick over it");

    let early = MIDDLE + 20;
    let late = MIDDLE - 35;
    assert_eq!(rig.ruler(early, 5), Color::rgb(0, 255, 0), "the newest hit is the record's line colour at full alpha, twenty milliseconds right of the middle");
    let older = rig.ruler(late, 5);
    assert_eq!((older.g, older.b), (255, 0), "the older one is the same green over its own window colour: {older:?}");
    assert!((1..=4).contains(&older.r), "a hundredth fainter, so a trace of the yellow shows through: {older:?}");
    assert_eq!((rig.ruler(early + 1, 5), rig.ruler(early - 1, 5)), (GOOD, GREAT), "and a line is one pixel wide");
    assert_eq!((rig.ruler(early, 0), rig.ruler(early, 19)), (Color::rgb(0, 255, 0), Color::rgb(0, 255, 0)), "from the top of the ruler to its foot");
}

#[test]
fn a_ruler_written_with_a_negative_width_is_mirrored_and_its_lines_stay_where_they_were() {
    let mut rig = Rig::new("vis-mirror", 1);
    let hits = [(20_i64, 0_u8)];
    rig.draw(Some(RecentHits::new(&hits).with_judge_area(WINDOWS)));

    assert_eq!((rig.ruler(MIDDLE - 57, 5), rig.ruler(MIDDLE + 57, 5)), (BAD, POOR), "the bad window is seventy late and fifty early");
    assert_eq!((rig.mirror(MIDDLE - 57, 5), rig.mirror(MIDDLE + 57, 5)), (POOR, BAD), "and the mirrored ruler has them the other way round");
    assert_eq!(rig.mirror(MIDDLE + 20, 5), Color::rgb(0, 255, 0), "the line of a hit twenty milliseconds early is on the same side of the middle on both");
}

#[test]
fn a_hit_beyond_the_ruler_leaves_no_line() {
    let mut rig = Rig::new("vis-beyond", 1);
    let hits = [(151_i64, 0_u8), (-151, 0)];
    rig.draw(Some(RecentHits::new(&hits)));
    assert!((0..RECORD_W).all(|x| rig.ruler(x, 5).r > 64 || rig.ruler(x, 5).g < 128), "a bare ruler with nothing green on it");
}

#[test]
fn the_lines_keep_the_records_pixels_when_the_document_is_drawn_at_twice_the_size() {
    let mut rig = Rig::new("vis-scale", 2);
    let hits = [(20_i64, 0_u8)];
    rig.draw(Some(RecentHits::new(&hits)));

    let row = (RULER_TOP + RULER_H / 2) * 2;
    let found: Vec<u32> = (0..DOC_W * 2)
        .filter(|x| {
            let pixel = rig.canvas.pixel_at(*x, row);
            pixel.g > 128 && pixel.r < 64 && pixel.b < 64
        })
        .collect();
    let middle_of_rectangle = RULER_X as f32 * 2.0 + (RECORD_W as f32 * 2.0 - 1.0) / 2.0;
    let expected = (middle_of_rectangle + 20.0) as u32;
    assert!(found.iter().all(|x| x.abs_diff(expected) <= 1), "the line sits twenty pixels, not forty, from the middle: {found:?} against {expected}");
    assert!(!found.is_empty(), "and it is drawn");
}

#[test]
fn the_strip_puts_an_early_hit_left_of_the_middle_line_coloured_by_its_window() {
    let mut rig = Rig::new("vis-strip", 1);
    let hits = [(-50_i64, 2_u8), (20, 1)];
    assert_eq!(rig.draw(Some(RecentHits::new(&hits).with_judge_area(WINDOWS))), 3);

    assert_eq!(rig.strip(MIDDLE, 30), Color::rgb(255, 255, 255), "the middle line");
    assert_eq!(rig.strip(MIDDLE - 20, 30), GOOD, "twenty milliseconds early is twenty pixels left, and exactly on the edge of the great window it is a good");
    assert_eq!(rig.strip(MIDDLE + 50, 30), BAD, "fifty late is fifty right, inside the bad window");
    assert_eq!(rig.strip(MIDDLE - 20, 0), GOOD, "without decay the mark runs the strip's whole height");
    assert_eq!(rig.strip(MIDDLE - 20, 59), GOOD);
    assert_eq!(rig.strip(MIDDLE - 19, 30), Color::BLACK, "and is one pixel wide");
}

#[test]
fn the_strip_follows_the_run_when_a_hundred_hits_are_already_on_show() {
    let mut rig = Rig::new("vis-wrap", 1);
    let first: Vec<(i64, u8)> = (0..100).map(|_| (5_i64, 0_u8)).collect();
    rig.draw(Some(RecentHits::new(&first).with_judge_area(WINDOWS)));
    assert_eq!(rig.strip(MIDDLE - 5, 30), PERFECT);
    assert_eq!(rig.strip(MIDDLE - 30, 30), Color::BLACK);

    let second: Vec<(i64, u8)> = (0..100).map(|at| if at == 99 { (30_i64, 2_u8) } else { (5_i64, 0_u8) }).collect();
    rig.draw(Some(RecentHits::new(&second).with_judge_area(WINDOWS).with_recorded(101)));
    assert_eq!(rig.strip(MIDDLE - 30, 30), GOOD, "one more hit recorded, though the hundred kept are as many as before");
}
