//! Unit tests for the song wheel: which bar lands on which slot, what each bar is cut from and named
//! with, where its parts land, the order they are drawn in, how a slide moves them and which bar a
//! press lands on.
//!
//! Every drawing test loads a document written out here, builds it into a screen and draws it
//! through a renderer that writes down each quad. A quad says which cell of the fixture's one sheet
//! it was cut from and where it landed, so "the lamp of the bar on slot one" is a fact that can be
//! read back rather than a colour that has to be guessed at. The sheet is laid out in bands -- bars
//! along the top, then lamps, trophies, labels, one row of digits per difficulty and the graph --
//! and every texel has a colour of its own for the few tests that look at pixels.

use std::path::{Path, PathBuf};

use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::property::MapHost;
use rbms_skin::property::generated::OPTION_PANEL1;
use rbms_skin::timer::{MICROS_PER_MILLI, TimerState};

use super::bars::{BAR_LABELS, BAR_LAMPS, BAR_LEVELS, BAR_TEXTS, BAR_TROPHIES};
use super::{
    BarDistribution, BarHold, BarKind, BarScroll, BarScroller, BarTrophy, LAMP_KINDS, RANK_KINDS, SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS, SongBar,
    SongBars,
};
use crate::ctx::RenderCtx;
use crate::font::TextContext;
use crate::skin_render::frame::PreparedFrame;
use crate::skin_render::{FrameData, SkinAction, SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinPointerButton, SkinScreen};
use crate::{BYTES_PER_PIXEL, Color, CpuCanvas, QuadParams, Rect, Renderer, TextureId};

/// The size the fixture is authored at, which is also the canvas it is drawn on unless a test says
/// otherwise.
const DOC: (u32, u32) = (400, 300);

/// Slots the fixture wheel declares, and the one it calls its centre.
const SLOTS: usize = 5;
const CENTER: usize = 2;

/// Where a bar under the cursor and any other bar start across the document.
const ON_X: i32 = 200;
const OFF_X: i32 = 220;

/// The size of every bar.
const BAR_W: i32 = 160;
const BAR_H: i32 = 20;

/// Where the top slot's bar starts up the document, and how far apart the slots are unless a test
/// packs them closer.
const TOP_Y: i32 = 240;
const PITCH: i32 = 40;

/// The side of the fixture's one sheet.
const SHEET: u32 = 64;

/// How much greener a texel is than the one above it, and how much greener one band of small cells
/// is than the band above it.
const TEXEL_GREEN_STEP: u32 = 4;
const TEXEL_BAND_STEP: u32 = 16;

/// Where each band of the sheet starts down it, and how large a cell of it is.
const BAR_BAND: i32 = 0;
const BAR_CELL: (i32, i32) = (8, 4);
const LAMP_BAND: i32 = 4;
const TROPHY_BAND: i32 = 8;
const LABEL_BAND: i32 = 12;
const SMALL_CELL: i32 = 4;
const LEVEL_BAND: i32 = 16;
const DIGIT_CELL: (i32, i32) = (4, 2);
const GRAPH_BAND: i32 = 32;

/// Images the fixture's bar set holds, one per kind of bar.
const BAR_IMAGES: usize = 7;

/// Where each part sits on a bar, measured from the bar's own corner.
const LAMP_AT: (i32, i32, i32, i32) = (-10, 0, 8, 20);
const PLAYER_LAMP_AT: (i32, i32, i32, i32) = (-10, 10, 8, 10);
const RIVAL_LAMP_AT: (i32, i32, i32, i32) = (-10, 0, 8, 10);
const TROPHY_AT: (i32, i32, i32, i32) = (140, -6, 16, 16);
const LEVEL_AT: (i32, i32, i32, i32) = (4, 5, 6, 10);
const LABEL_AT: (i32, i32, i32, i32) = (110, 3, 12, 14);
const GRAPH_AT: (i32, i32, i32, i32) = (20, 1, 110, 2);
const TEXT_AT: (i32, i32, i32, i32) = (30, 4, 100, 10);

/// How far apart across a bar two labels, and two texts, are placed, so which one was drawn can be
/// read off where it landed.
const LABEL_STEP: i32 = 2;
const TEXT_STEP: i32 = 1;

/// The red a text of kind zero is tinted, and how much redder each kind after it is, so a title's
/// tint says which of the wheel's texts drew it.
const TEXT_RED: i32 = 100;
const TEXT_RED_STEP: i32 = 10;

/// The `align` each difficulty's level number is declared with: flush right, centred for the
/// fifth and flush left for the sixth.
const LEVEL_ALIGN: [i32; BAR_LEVELS] = [0, 0, 0, 0, 2, 1, 0];

/// The difficulty whose level strip carries an alternate zero.
const ELEVEN_CELL_LEVEL: usize = 6;

/// The glyph of that alternate zero.
const ALTERNATE_ZERO: usize = 10;

/// How far a title's quad may land from the corner of its text's destination: the glyphs of a line
/// do not fill the box they are laid out in.
const TEXT_SLACK: f32 = 6.0;

/// How far two coordinates that are meant to be the same may differ.
const NEAR: f32 = 0.01;

/// The prefix of the key a text object registers its composed line under.
const TEXT_TEXTURE_KEY: &str = "rbms.skin.text.";

/// The bundled face, written out as the fixture's own font file.
const FONT: &[u8] = include_bytes!("../../../../../assets/fonts/Inter-Regular.ttf");

/// A source that decodes to a sheet whose every texel has a colour of its own.
struct SheetAssets;

impl SkinAssets for SheetAssets {
    fn image(&mut self, _path: &Path) -> Option<SkinImage> {
        let mut rgba = Vec::with_capacity((SHEET * SHEET) as usize * BYTES_PER_PIXEL);
        for y in 0..SHEET {
            for x in 0..SHEET {
                rgba.extend_from_slice(&texel(x, y).to_array());
            }
        }
        SkinImage::new(SHEET, SHEET, rgba)
    }
}

/// The colour of the sheet's texel at `(x, y)`.
fn texel(x: u32, y: u32) -> Texel {
    Texel { r: (x * TEXEL_GREEN_STEP) as u8, g: (y * TEXEL_GREEN_STEP) as u8, b: u8::MAX / 2 }
}

/// An opaque texel of the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Texel {
    r: u8,
    g: u8,
    b: u8,
}

impl Texel {
    fn to_array(self) -> [u8; BYTES_PER_PIXEL] {
        [self.r, self.g, self.b, u8::MAX]
    }
}

/// One quad as it was submitted.
#[derive(Debug, Clone, Copy)]
struct Quad {
    tex: TextureId,
    params: QuadParams,
}

/// A canvas that writes down every quad drawn on it and every texture it is given or handed back.
struct Recorder {
    inner: CpuCanvas,
    quads: Vec<Quad>,
    registered: Vec<(String, TextureId)>,
    released: Vec<TextureId>,
}

impl Recorder {
    fn new(size: (u32, u32)) -> Recorder {
        Recorder { inner: CpuCanvas::new(size.0, size.1), quads: Vec::new(), registered: Vec::new(), released: Vec::new() }
    }

    /// How many times a text object has composed a line into a texture.
    fn composed(&self) -> usize {
        self.registered.iter().filter(|(key, _)| key.starts_with(TEXT_TEXTURE_KEY)).count()
    }

    /// The text textures that have been registered and not handed back.
    fn held(&self) -> usize {
        let mut live: Vec<TextureId> = self.registered.iter().filter(|(key, _)| key.starts_with(TEXT_TEXTURE_KEY)).map(|(_, tex)| *tex).collect();
        live.sort();
        live.dedup();
        live.iter().filter(|tex| self.inner.texture_size(**tex).is_some()).count()
    }
}

impl Renderer for Recorder {
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
        let tex = self.inner.register_texture(key, rgba, width, height);
        self.registered.push((key.to_owned(), tex));
        tex
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.released.push(tex);
        self.inner.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.inner.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.quads.push(Quad { tex, params });
        self.inner.draw_textured_quad(tex, params);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.inner.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.inner.pop_clip();
    }
}

/// What a quad was cut from, read off the band of the sheet its source lies in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Piece {
    /// A bar, by the image of the bar set.
    Bar(usize),
    Lamp(usize),
    Trophy(usize),
    Label(usize),
    /// A digit of a level, by the difficulty of its strip and the glyph.
    Digit(usize, usize),
    /// A segment of the graph, by the lamp or rank it counts.
    Graph(usize),
    /// A composed title.
    Text,
}

/// A scratch folder holding the generated document, its sheet and its font, removed when the test
/// ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-wheel-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// What one generated document varies from the shared fixture.
#[derive(Clone, Copy)]
struct Wheel<'a> {
    /// How far apart the slots are.
    pitch: i32,
    /// The images of the bar set, by id.
    bar_set: &'a [&'a str],
    /// The id every slot's two destinations name.
    bar_id: &'a str,
    /// Members added to every `listoff` destination's keyframe.
    off_tint: &'a str,
    /// Members added to every destination of the lists they are named for.
    bar_extra: &'a str,
    lamp_extra: &'a str,
    level_extra: &'a str,
    /// The graph definition's `type`.
    graph_type: i32,
    /// Destinations declared after the wheel, each starting with a comma.
    over: &'a str,
}

impl Default for Wheel<'_> {
    fn default() -> Self {
        Wheel {
            pitch: PITCH,
            bar_set: &["bar-0", "bar-1", "bar-2", "bar-3", "bar-4", "bar-5", "bar-6"],
            bar_id: "bars",
            off_tint: "",
            bar_extra: "",
            lamp_extra: "",
            level_extra: "",
            graph_type: -1,
            over: "",
        }
    }
}

/// `count` records, each written by `record` from its number, joined as the members of a list.
fn records(count: usize, record: impl Fn(usize) -> String) -> String {
    (0..count).map(record).collect::<Vec<_>>().join(",")
}

/// `count` images of `name`, cut side by side from the band at `band`.
fn images(name: &str, count: usize, band: i32, cell: (i32, i32)) -> String {
    records(count, |index| format!(r#"{{"id":"{name}-{index}","src":"sheet","x":{},"y":{band},"w":{},"h":{}}}"#, cell.0 * index as i32, cell.0, cell.1))
}

/// One nested destination naming `id` at `at`, with `extra` added to the destination itself.
fn nested(id: &str, at: (i32, i32, i32, i32), extra: &str) -> String {
    format!(r#"{{"id":"{id}","dst":[{{"x":{},"y":{},"w":{},"h":{}}}]{extra}}}"#, at.0, at.1, at.2, at.3)
}

/// Where slot `slot`'s bar starts across the document when nothing is sliding.
fn bar_x(slot: usize) -> i32 {
    if slot == CENTER { ON_X } else { OFF_X }
}

/// Where slot `slot`'s bar starts up the document.
fn slot_y(slot: usize, pitch: i32) -> i32 {
    TOP_Y - pitch * slot as i32
}

/// The fixture document: a five-slot wheel carrying every kind of part a wheel can carry.
fn document(wheel: &Wheel<'_>) -> String {
    let kinds = if wheel.graph_type == -1 { LAMP_KINDS } else { RANK_KINDS };
    let levels = records(BAR_LEVELS, |index| {
        let cells = if index == ELEVEN_CELL_LEVEL { 11 } else { 10 };
        let (y, w) = (LEVEL_BAND + DIGIT_CELL.1 * index as i32, DIGIT_CELL.0 * cells);
        let align = LEVEL_ALIGN[index];
        format!(r#"{{"id":"level-{index}","src":"sheet","x":0,"y":{y},"w":{w},"h":{},"divx":{cells},"digit":2,"align":{align}}}"#, DIGIT_CELL.1)
    });
    let bar = |x: i32, tint: &str| {
        records(SLOTS, |slot| {
            let y = slot_y(slot, wheel.pitch);
            format!(r#"{{"id":"{}","dst":[{{"x":{x},"y":{y},"w":{BAR_W},"h":{BAR_H}{tint}}}]{}}}"#, wheel.bar_id, wheel.bar_extra)
        })
    };
    let texts = records(BAR_TEXTS, |index| {
        let (x, red) = (TEXT_AT.0 + TEXT_STEP * index as i32, TEXT_RED + TEXT_RED_STEP * index as i32);
        format!(r#"{{"id":"title","dst":[{{"x":{x},"y":{},"w":{},"h":{},"r":{red}}}]}}"#, TEXT_AT.1, TEXT_AT.2, TEXT_AT.3)
    });
    let labels =
        records(BAR_LABELS, |index| nested(&format!("label-{index}"), (LABEL_AT.0 + LABEL_STEP * index as i32, LABEL_AT.1, LABEL_AT.2, LABEL_AT.3), ""));
    let lamps = |at: (i32, i32, i32, i32)| records(BAR_LAMPS, |index| nested(&format!("lamp-{index}"), at, wheel.lamp_extra));
    format!(
        r#"{{
            "type": 5, "name": "wheel", "w": {}, "h": {},
            "source": [{{ "id": "sheet", "path": "sheet.tex" }}],
            "font": [{{ "id": "0", "path": "font.ttf" }}],
            "image": [{bars},{lamp_images},{trophies},{label_images},{{"id":"cover","src":"sheet","x":60,"y":60,"w":4,"h":4,"act":19}}],
            "imageset": [{{ "id": "bars", "images": [{set}] }}],
            "value": [{levels}],
            "graph": [{{ "id": "graph", "src": "sheet", "x": 0, "y": {GRAPH_BAND}, "w": {kinds}, "h": 2, "divx": {kinds}, "type": {graph_type} }}],
            "text": [{{ "id": "title", "font": "0", "size": 10 }}],
            "songlist": {{
                "id": "wheel", "center": {CENTER}, "clickable": [1, 2, 3],
                "liston": [{on}], "listoff": [{off}],
                "text": [{texts}], "level": [{level}],
                "lamp": [{lamp}], "playerlamp": [{player_lamp}], "rivallamp": [{rival_lamp}],
                "trophy": [{trophy}], "label": [{labels}],
                "graph": {graph}
            }},
            "destination": [{{ "id": "wheel" }}{over}]
        }}"#,
        DOC.0,
        DOC.1,
        bars = images("bar", BAR_IMAGES, BAR_BAND, BAR_CELL),
        lamp_images = images("lamp", BAR_LAMPS, LAMP_BAND, (SMALL_CELL, SMALL_CELL)),
        trophies = images("trophy", BAR_TROPHIES, TROPHY_BAND, (SMALL_CELL, SMALL_CELL)),
        label_images = images("label", BAR_LABELS, LABEL_BAND, (SMALL_CELL, SMALL_CELL)),
        set = wheel.bar_set.iter().map(|id| format!(r#""{id}""#)).collect::<Vec<_>>().join(","),
        graph_type = wheel.graph_type,
        on = bar(ON_X, ""),
        off = bar(OFF_X, wheel.off_tint),
        level = records(BAR_LEVELS, |index| nested(&format!("level-{index}"), LEVEL_AT, wheel.level_extra)),
        lamp = lamps(LAMP_AT),
        player_lamp = lamps(PLAYER_LAMP_AT),
        rival_lamp = lamps(RIVAL_LAMP_AT),
        trophy = records(BAR_TROPHIES, |index| nested(&format!("trophy-{index}"), TROPHY_AT, "")),
        graph = nested("graph", GRAPH_AT, ""),
        over = wheel.over,
    )
}

/// A fixture document, the screen built from it and the canvas it is drawn on.
struct Rig {
    _scratch: Scratch,
    canvas: Recorder,
    text: TextContext,
    screen: SkinScreen,
    host: MapHost,
}

impl Rig {
    /// The shared fixture, drawn at the size it is authored at.
    fn new(tag: &str) -> Rig {
        Rig::with(tag, &Wheel::default(), DOC)
    }

    /// A document that varies from the shared fixture, drawn on a canvas of `size`.
    fn with(tag: &str, wheel: &Wheel<'_>, size: (u32, u32)) -> Rig {
        let scratch = Scratch::new(tag);
        std::fs::write(scratch.root.join("sheet.tex"), "sheet").expect("the source file is writable");
        std::fs::write(scratch.root.join("font.ttf"), FONT).expect("the font file is writable");
        let path = scratch.root.join("wheel.json");
        std::fs::write(&path, document(wheel)).expect("the document is writable");
        let user = SkinUserConfig::default();
        let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
        let skin = load_skin(&path, options).expect("the generated document loads");

        let mut canvas = Recorder::new(size);
        let mut text = TextContext::embedded_only();
        let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut SheetAssets);
        Rig { _scratch: scratch, canvas, text, screen, host: MapHost::default() }
    }

    /// The texture the fixture's sheet was registered as.
    fn sheet(&self) -> TextureId {
        self.canvas.registered.iter().find(|(key, _)| !key.starts_with(TEXT_TEXTURE_KEY)).map(|(_, tex)| *tex).expect("the sheet was registered")
    }

    /// Prepares one frame over `list`, `now_ms` into the scene.
    fn prepare(&mut self, list: &SongBars<'_>, now_ms: i64) -> PreparedFrame {
        let timers = TimerState::new();
        let data = FrameData { bars: Some(list), ..FrameData::default() };
        self.screen.prepare(&SkinFrame { now_us: now_ms * MICROS_PER_MILLI, timers: &timers, state: &self.host, lua: None, mouse: None, data })
    }

    /// Draws one frame over `list` onto a cleared canvas and answers the quads it came to, in the
    /// order they were submitted.
    fn draw(&mut self, list: &SongBars<'_>) -> Vec<Quad> {
        let timers = TimerState::new();
        let data = FrameData { bars: Some(list), ..FrameData::default() };
        let frame = SkinFrame { now_us: 0, timers: &timers, state: &self.host, lua: None, mouse: None, data };
        self.canvas.clear(Color::BLACK);
        self.canvas.quads.clear();
        let mut ctx = RenderCtx::new(crate::theme::theme(), &mut self.text);
        self.screen.draw(&mut ctx, &mut self.canvas, &frame);
        self.canvas.quads.clone()
    }

    /// What `quad` was cut from.
    fn piece(&self, quad: &Quad) -> Piece {
        if quad.tex != self.sheet() {
            return Piece::Text;
        }
        let x = (quad.params.src.u0 * SHEET as f32).round() as i32;
        let y = (quad.params.src.v0 * SHEET as f32).round() as i32;
        match y {
            BAR_BAND => Piece::Bar((x / BAR_CELL.0) as usize),
            LAMP_BAND => Piece::Lamp((x / SMALL_CELL) as usize),
            TROPHY_BAND => Piece::Trophy((x / SMALL_CELL) as usize),
            LABEL_BAND => Piece::Label((x / SMALL_CELL) as usize),
            GRAPH_BAND => Piece::Graph(x as usize),
            _ => Piece::Digit(((y - LEVEL_BAND) / DIGIT_CELL.1) as usize, (x / DIGIT_CELL.0) as usize),
        }
    }

    /// Draws one frame and answers what each quad was cut from and where it landed.
    fn pieces(&mut self, list: &SongBars<'_>) -> Vec<(Piece, Rect)> {
        let quads = self.draw(list);
        quads.iter().map(|quad| (self.piece(quad), quad.params.dst)).collect()
    }
}

/// Where a document rectangle lands on a canvas the size of the document.
fn on_screen(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(x as f32, (DOC.1 as i32 - (y + h)) as f32, w as f32, h as f32)
}

/// Where a part at `at` on the bar whose corner is `(x, y)` lands.
fn part_on(x: i32, y: i32, at: (i32, i32, i32, i32)) -> Rect {
    on_screen(x + at.0, y + at.1, at.2, at.3)
}

/// Whether two rectangles are the same to within [`NEAR`].
fn same(a: Rect, b: Rect) -> bool {
    [(a.x, b.x), (a.y, b.y), (a.w, b.w), (a.h, b.h)].iter().all(|(a, b)| (a - b).abs() < NEAR)
}

/// The rectangles the pieces `wanted` picks landed on, in the order they were drawn.
fn landed(pieces: &[(Piece, Rect)], wanted: impl Fn(&Piece) -> bool) -> Vec<Rect> {
    pieces.iter().filter(|(piece, _)| wanted(piece)).map(|(_, rect)| *rect).collect()
}

/// The pieces of a frame, in the order they were drawn.
fn kinds(pieces: &[(Piece, Rect)]) -> Vec<Piece> {
    pieces.iter().map(|(piece, _)| *piece).collect()
}

/// A plain bar that carries no level, no trophy and no label.
fn table(index: usize) -> SongBar {
    SongBar::new(BarKind::Table, format!("TABLE {index}"))
}

/// A chart that is on disk.
fn song(title: &str) -> SongBar {
    SongBar::new(BarKind::Song { exists: true }, title)
}

/// A wheel's worth of plain bars, more than the fixture has slots for.
fn tables(count: usize) -> Vec<SongBar> {
    (0..count).map(table).collect()
}

/// Where the bars of a frame landed, slot by slot.
fn bar_rects(pieces: &[(Piece, Rect)]) -> Vec<Rect> {
    landed(pieces, |piece| matches!(piece, Piece::Bar(_)))
}

#[test]
fn the_ring_puts_the_cursor_on_the_centre_slot_and_goes_round_the_ends_of_the_list() {
    let three = tables(3);
    let list = SongBars::new(&three, 1);
    let on_slots: Vec<Option<usize>> = (0..SLOTS).map(|slot| list.bar_on(slot, CENTER as i32)).collect();
    assert_eq!(on_slots, [Some(2), Some(0), Some(1), Some(2), Some(0)], "a list shorter than the wheel repeats");

    let none: Vec<SongBar> = Vec::new();
    assert_eq!(SongBars::new(&none, 0).bar_on(CENTER, CENTER as i32), None, "an empty list puts nothing anywhere");

    let many = tables(10_000);
    let list = SongBars::new(&many, 9_999);
    assert_eq!(list.bar_on(CENTER, CENTER as i32), Some(9_999));
    assert_eq!(list.bar_on(CENTER + 2, CENTER as i32), Some(1), "the bars below the last one are the first ones");
    assert_eq!(list.bar_on(0, CENTER as i32), Some(9_997));
    assert_eq!(SongBars::new(&many, 0).bar_on(0, CENTER as i32), Some(9_998), "and the bars above the first one are the last ones");

    assert_eq!(SongBars::new(&three, 0).bar_on(0, -1), Some(1), "a centre above the wheel is still counted from");
    assert_eq!(SongBars::new(&three, 0).bar_on(0, 59), Some(1), "and so is one below it");
}

#[test]
fn every_kind_of_bar_is_cut_from_the_image_the_reference_cuts_it_from() {
    let cut = [
        (BarKind::Song { exists: true }, Some(0)),
        (BarKind::Folder, Some(1)),
        (BarKind::Table, Some(2)),
        (BarKind::Executable, Some(2)),
        (BarKind::RandomCourse { complete: true }, Some(2)),
        (BarKind::Course { complete: true }, Some(3)),
        (BarKind::Course { complete: false }, Some(4)),
        (BarKind::RandomCourse { complete: false }, Some(4)),
        (BarKind::Song { exists: false }, Some(4)),
        (BarKind::Command, Some(5)),
        (BarKind::Search, Some(6)),
        (BarKind::Other, None),
    ];
    for (kind, image) in cut {
        assert_eq!(kind.image_index(), image, "{kind:?}");
    }
    let opens = [BarKind::Folder, BarKind::Table, BarKind::Command, BarKind::Search];
    let plays = [BarKind::Song { exists: true }, BarKind::Executable, BarKind::Course { complete: true }, BarKind::RandomCourse { complete: true }];
    assert!(opens.iter().all(|kind| kind.is_directory()) && plays.iter().all(|kind| !kind.is_directory()));
}

#[test]
fn a_title_is_named_by_the_text_of_its_kind_and_falls_back_when_the_document_declares_none() {
    let every = |_: usize| true;
    let only = |declared: &'static [usize]| move |index: usize| declared.contains(&index);
    let new = |kind: BarKind| SongBar { is_new: true, ..SongBar::new(kind, "") };
    let settled = |kind: BarKind| SongBar::new(kind, "");
    let chart = BarKind::Song { exists: true };

    assert_eq!(settled(chart).text_index(0, every), 2);
    assert_eq!(new(chart).text_index(0, every), 3);
    assert_eq!(settled(BarKind::Folder).text_index(1, every), 4);
    assert_eq!(new(BarKind::Folder).text_index(1, every), 5);
    for image in 2..BAR_IMAGES {
        assert_eq!(new(BarKind::Table).text_index(image, every), image + 4, "a bar past the folders has one text, new or not");
    }

    assert_eq!(settled(chart).text_index(0, only(&[0, 1])), 0, "a chart with no text of its own takes the general one");
    assert_eq!(new(chart).text_index(0, only(&[0, 1])), 1, "and a new one takes the general new one");
    assert_eq!(new(BarKind::Folder).text_index(1, only(&[0, 4])), 1, "which is asked for even when the document left it out");
    assert_eq!(settled(BarKind::Folder).text_index(1, only(&[0, 5])), 0);
    assert_eq!(settled(BarKind::Search).text_index(6, only(&[0, 1])), 0);
}

#[test]
fn a_level_a_trophy_and_a_label_are_picked_as_the_reference_picks_them() {
    let chart = |difficulty: i32| SongBar { difficulty, ..song("") };
    assert_eq!([0, 3, 6, 7, -1, 99].map(|difficulty| chart(difficulty).level_index()), [Some(0), Some(3), Some(6), Some(0), Some(0), Some(0)]);
    assert_eq!(
        SongBar { difficulty: 3, ..SongBar::new(BarKind::Song { exists: false }, "") }.level_index(),
        None,
        "a chart that is not on disk shows no level"
    );
    assert_eq!(table(0).level_index(), None);

    let course = |trophy: Option<BarTrophy>| SongBar { trophy, ..SongBar::new(BarKind::Course { complete: false }, "") };
    assert_eq!(
        [Some(BarTrophy::Bronze), Some(BarTrophy::Silver), Some(BarTrophy::Gold), None].map(|trophy| course(trophy).trophy_index()),
        [Some(0), Some(1), Some(2), None]
    );
    assert_eq!(SongBar { trophy: Some(BarTrophy::Gold), ..song("") }.trophy_index(), None, "only a course carries one");

    let with = |features: u32| SongBar { features, ..song("") };
    let (open, long, charge, hell) =
        (SongBar::FEATURE_UNDEFINED_LN, SongBar::FEATURE_LONG_NOTE, SongBar::FEATURE_CHARGE_NOTE, SongBar::FEATURE_HELL_CHARGE_NOTE);
    assert_eq!(with(0).long_note_label(2), None);
    assert_eq!([0, 1, 2].map(|mode| with(open).long_note_label(mode)), [Some(0), Some(3), Some(4)], "an open chart takes the player's mode");
    assert_eq!(with(open).long_note_label(3), None, "a mode the reference has no label under draws none");
    assert_eq!(with(open).long_note_label(-1), None);
    assert_eq!(with(long).long_note_label(2), Some(0), "a chart that names its kind ignores the mode");
    assert_eq!(with(long | charge).long_note_label(0), Some(3), "and takes the hardest kind it names");
    assert_eq!(with(open | long).long_note_label(1), Some(3), "or the mode, when that is harder");
    assert_eq!(with(hell).long_note_label(0), Some(4));

    let missing = SongBar { features: long | SongBar::FEATURE_MINE_NOTE | SongBar::FEATURE_RANDOM, ..SongBar::new(BarKind::Song { exists: false }, "") };
    assert!(missing.long_note_label(0).is_none() && !missing.has_mines() && !missing.has_random(), "a chart that is not on disk shows no label");
    let whole = SongBar { features: SongBar::FEATURE_MINE_NOTE, ..SongBar::new(BarKind::Course { complete: true }, "") };
    let partial = SongBar { features: SongBar::FEATURE_MINE_NOTE, ..SongBar::new(BarKind::Course { complete: false }, "") };
    assert!(whole.has_mines() && !partial.has_mines(), "a course shows its charts' labels only when all of them are on disk");
}

/// The bars of the frame most tests draw: one of each kind that draws something of its own, each on
/// the slot of its own number.
fn mixed() -> Vec<SongBar> {
    let mut counts = BarDistribution::default();
    (counts.lamps[0], counts.lamps[5], counts.lamps[10]) = (1, 2, 1);
    vec![
        SongBar {
            level: 7,
            difficulty: 3,
            lamp: 5,
            features: SongBar::FEATURE_LONG_NOTE | SongBar::FEATURE_MINE_NOTE | SongBar::FEATURE_RANDOM,
            ..song("CHART")
        },
        SongBar { is_new: true, distribution: Some(Box::new(counts)), ..SongBar::new(BarKind::Folder, "FOLDER") },
        SongBar { trophy: Some(BarTrophy::Gold), lamp: 8, ..SongBar::new(BarKind::Course { complete: true }, "COURSE") },
        SongBar::new(BarKind::Table, "TABLE"),
        SongBar { level: 12, difficulty: 2, ..SongBar::new(BarKind::Song { exists: false }, "MISSING") },
    ]
}

#[test]
fn a_wheel_is_drawn_pass_by_pass_with_every_part_on_its_own_bar() {
    let mut rig = Rig::new("passes");
    assert_eq!((rig.screen.count_of(SkinObjectKind::SongList), rig.screen.warnings()), (1, &[][..]));
    let bars = mixed();
    let quads = rig.draw(&SongBars::new(&bars, CENTER));
    let pieces: Vec<(Piece, Rect)> = quads.iter().map(|quad| (rig.piece(quad), quad.params.dst)).collect();

    let expected = [
        Piece::Bar(0),
        Piece::Bar(1),
        Piece::Bar(3),
        Piece::Bar(2),
        Piece::Bar(4),
        Piece::Graph(10),
        Piece::Graph(5),
        Piece::Graph(0),
        Piece::Text,
        Piece::Text,
        Piece::Text,
        Piece::Text,
        Piece::Text,
        Piece::Trophy(2),
        Piece::Lamp(5),
        Piece::Lamp(0),
        Piece::Lamp(8),
        Piece::Lamp(0),
        Piece::Lamp(0),
        Piece::Digit(3, 7),
        Piece::Label(0),
        Piece::Label(2),
        Piece::Label(1),
    ];
    assert_eq!(kinds(&pieces), expected, "bars, graphs, titles, trophies, lamps, levels, labels: each pass over every slot before the next");

    let bar = |slot: usize| (bar_x(slot), slot_y(slot, PITCH));
    for (slot, rect) in bar_rects(&pieces).iter().enumerate() {
        let (x, y) = bar(slot);
        assert!(same(*rect, on_screen(x, y, BAR_W, BAR_H)), "the bar of slot {slot} landed on {rect:?}");
    }

    let (folder_x, folder_y) = bar(1);
    let segments = landed(&pieces, |piece| matches!(piece, Piece::Graph(_)));
    let quarter = GRAPH_AT.2 as f32 / 4.0;
    let from = (folder_x + GRAPH_AT.0) as f32;
    let top = (DOC.1 as i32 - (folder_y + GRAPH_AT.1 + GRAPH_AT.3)) as f32;
    let shares = [(0.0, 1.0), (1.0, 2.0), (3.0, 1.0)];
    for (segment, (before, share)) in segments.iter().zip(shares) {
        let wanted = Rect::new(from + before * quarter, top, share * quarter, GRAPH_AT.3 as f32);
        assert!(same(*segment, wanted), "a graph segment landed on {segment:?}, not {wanted:?}");
    }

    let titles: Vec<&Quad> = quads.iter().filter(|quad| rig.piece(quad) == Piece::Text).collect();
    let text_kinds = [2, 5, 7, 6, 8];
    for (slot, (title, kind)) in titles.iter().zip(text_kinds).enumerate() {
        let (x, y) = bar(slot);
        let corner = part_on(x, y, (TEXT_AT.0 + TEXT_STEP * kind, TEXT_AT.1, TEXT_AT.2, TEXT_AT.3));
        assert_eq!(i32::from(title.params.tint.r), TEXT_RED + TEXT_RED_STEP * kind, "the title of slot {slot} is drawn by text {kind}");
        let dst = title.params.dst;
        assert!((dst.x - corner.x).abs() <= TEXT_SLACK && (dst.y - corner.y).abs() <= TEXT_SLACK, "the title of slot {slot} landed on {dst:?}");
    }

    let (course_x, course_y) = bar(CENTER);
    assert!(same(landed(&pieces, |piece| matches!(piece, Piece::Trophy(_)))[0], part_on(course_x, course_y, TROPHY_AT)));
    for (slot, lamp) in landed(&pieces, |piece| matches!(piece, Piece::Lamp(_))).iter().enumerate() {
        let (x, y) = bar(slot);
        assert!(same(*lamp, part_on(x, y, LAMP_AT)), "the lamp of slot {slot} landed on {lamp:?}");
    }

    let (chart_x, chart_y) = bar(0);
    let digit = landed(&pieces, |piece| matches!(piece, Piece::Digit(..)))[0];
    assert!(
        same(digit, part_on(chart_x, chart_y, (LEVEL_AT.0 + LEVEL_AT.2, LEVEL_AT.1, LEVEL_AT.2, LEVEL_AT.3))),
        "a one-digit level sits in the second place"
    );
    let labels = landed(&pieces, |piece| matches!(piece, Piece::Label(_)));
    for (label, index) in labels.iter().zip([0, 2, 1]) {
        assert!(same(*label, part_on(chart_x, chart_y, (LABEL_AT.0 + LABEL_STEP * index, LABEL_AT.1, LABEL_AT.2, LABEL_AT.3))));
    }
}

/// With the slots packed edge to edge a trophy reaches down over the bar below it. Drawn bar by bar
/// that bar's image would cover it; drawn pass by pass it lies on top.
#[test]
fn a_part_of_one_bar_lies_over_the_image_of_the_bar_below_it() {
    let mut rig = Rig::with("overlap", &Wheel { pitch: BAR_H, ..Wheel::default() }, DOC);
    let mut bars = tables(SLOTS);
    bars[CENTER] = SongBar { trophy: Some(BarTrophy::Gold), ..SongBar::new(BarKind::Course { complete: true }, "COURSE") };
    rig.draw(&SongBars::new(&bars, CENTER));

    let below = slot_y(CENTER, BAR_H) - 3;
    let pixel = rig.canvas.inner.pixel_at((ON_X + TROPHY_AT.0 + 8) as u32, DOC.1 - below as u32 - 1);
    assert_eq!(
        u32::from(pixel.g) / TEXEL_BAND_STEP,
        TROPHY_BAND as u32 / SMALL_CELL as u32,
        "the trophy's own band of the sheet shows over the next bar: {pixel:?}"
    );
}

#[test]
fn a_level_is_laid_out_by_the_align_and_the_padding_of_its_own_strip() {
    let mut rig = Rig::new("levels");
    let chart = |level: i32, difficulty: i32| SongBar { level, difficulty, ..song("CHART") };
    let digits = |rig: &mut Rig, level: i32, difficulty: i32| {
        let bars = vec![chart(level, difficulty)];
        let pieces = rig.pieces(&SongBars::new(&bars, 0));
        let on_cursor = pieces.iter().filter(|(piece, rect)| {
            matches!(piece, Piece::Digit(..))
                && same(Rect { x: 0.0, w: 0.0, ..*rect }, Rect { x: 0.0, w: 0.0, ..part_on(ON_X, slot_y(CENTER, PITCH), LEVEL_AT) })
        });
        on_cursor.map(|(piece, rect)| (*piece, rect.x - (ON_X + LEVEL_AT.0) as f32)).collect::<Vec<_>>()
    };
    let place = LEVEL_AT.2 as f32;

    assert_eq!(digits(&mut rig, 12, 3), [(Piece::Digit(3, 1), 0.0), (Piece::Digit(3, 2), place)]);
    assert_eq!(digits(&mut rig, 7, 3), [(Piece::Digit(3, 7), place)], "flush right leaves the blank place where it falls");
    assert_eq!(digits(&mut rig, 7, 4), [(Piece::Digit(4, 7), place / 2.0)], "a centred number is pulled half a place over it");
    assert_eq!(digits(&mut rig, 7, 5), [(Piece::Digit(5, 7), 0.0)], "and a flush left one a whole place");
    assert_eq!(digits(&mut rig, 7, 9), [(Piece::Digit(0, 7), place)], "a difficulty past the last is drawn by the first number");
    assert_eq!(digits(&mut rig, 0, 3), [(Piece::Digit(3, 0), place)], "a level of nothing is still a zero");
    assert_eq!(digits(&mut rig, 123, 3), [(Piece::Digit(3, 2), 0.0), (Piece::Digit(3, 3), place)], "a level too long for its places loses its front");
    assert_eq!(
        digits(&mut rig, 7, ELEVEN_CELL_LEVEL as i32),
        [(Piece::Digit(ELEVEN_CELL_LEVEL, ALTERNATE_ZERO), 0.0), (Piece::Digit(ELEVEN_CELL_LEVEL, 7), place)],
        "an eleven-cell strip fills the blank with its alternate zero"
    );
    assert_eq!(digits(&mut rig, i32::MIN, 3), [], "a level that says nothing draws nothing");
}

#[test]
fn a_bar_is_cut_from_the_image_of_its_kind_and_a_set_too_short_for_it_gives_its_first() {
    let every: Vec<SongBar> = [BarKind::Song { exists: true }, BarKind::Folder, BarKind::Table, BarKind::Course { complete: true }, BarKind::Search]
        .map(|kind| SongBar::new(kind, "BAR"))
        .to_vec();
    let mut rig = Rig::new("kinds");
    let pieces = rig.pieces(&SongBars::new(&every, CENTER));
    assert_eq!(landed(&pieces, |piece| matches!(piece, Piece::Bar(_))).len(), SLOTS);
    let cut: Vec<Piece> = kinds(&pieces).into_iter().filter(|piece| matches!(piece, Piece::Bar(_))).collect();
    assert_eq!(cut, [Piece::Bar(0), Piece::Bar(1), Piece::Bar(2), Piece::Bar(3), Piece::Bar(6)]);

    let short = Wheel { bar_set: &["bar-0", "bar-1", "bar-2"], ..Wheel::default() };
    let mut rig = Rig::with("short-set", &short, DOC);
    let cut: Vec<Piece> = kinds(&rig.pieces(&SongBars::new(&every, CENTER))).into_iter().filter(|piece| matches!(piece, Piece::Bar(_))).collect();
    assert_eq!(cut, [Piece::Bar(0), Piece::Bar(1), Piece::Bar(2), Piece::Bar(0), Piece::Bar(0)], "a kind the set has no image for is cut from the first");
}

/// A hole in the bar set leaves a bar of that kind without its image and with everything else, and a
/// set whose first image is the hole leaves the wheel with no slot at all, because a slot is
/// prepared from the first image before any bar lands on it.
#[test]
fn a_set_that_names_an_image_the_document_lacks_draws_no_bar_of_that_kind() {
    let holed = Wheel { bar_set: &["bar-0", "nothing", "bar-2", "bar-3", "bar-4", "bar-5", "bar-6"], ..Wheel::default() };
    let mut rig = Rig::with("holed-set", &holed, DOC);
    let mut bars = tables(SLOTS);
    bars[1] = SongBar::new(BarKind::Folder, "FOLDER");
    let pieces = kinds(&rig.pieces(&SongBars::new(&bars, CENTER)));
    assert_eq!(pieces.iter().filter(|piece| matches!(piece, Piece::Bar(_))).count(), SLOTS - 1, "the folder's own image is the hole");
    assert_eq!(pieces.iter().filter(|piece| **piece == Piece::Text).count(), SLOTS, "and its title is drawn all the same");

    let headless = Wheel { bar_set: &["nothing", "bar-1", "bar-2"], ..Wheel::default() };
    let mut rig = Rig::with("headless-set", &headless, DOC);
    assert_eq!(rig.draw(&SongBars::new(&bars, CENTER)).len(), 0);
}

#[test]
fn a_bar_the_reference_does_not_draw_leaves_its_slot_bare_and_an_empty_list_leaves_every_slot_bare() {
    let mut rig = Rig::new("bare");
    let mut bars = tables(SLOTS);
    bars[3] = SongBar::new(BarKind::Other, "SAME FOLDER");
    let pieces = rig.pieces(&SongBars::new(&bars, CENTER));
    let rects = bar_rects(&pieces);
    assert_eq!(rects.len(), SLOTS - 1);
    assert!(rects.iter().all(|rect| !same(*rect, on_screen(OFF_X, slot_y(3, PITCH), BAR_W, BAR_H))), "nothing lands on the fourth slot");
    assert_eq!(kinds(&pieces).iter().filter(|piece| **piece == Piece::Text).count(), SLOTS - 1, "not even its title");

    let none: Vec<SongBar> = Vec::new();
    assert_eq!(rig.draw(&SongBars::new(&none, 0)).len(), 0, "a list with nothing in it draws no wheel");
}

#[test]
fn a_list_shorter_than_the_wheel_fills_every_slot_and_composes_each_title_once() {
    let mut rig = Rig::new("short-list");
    let bars = tables(2);
    let quads = rig.draw(&SongBars::new(&bars, 0));
    let titles: Vec<TextureId> = quads.iter().filter(|quad| rig.piece(quad) == Piece::Text).map(|quad| quad.tex).collect();
    assert_eq!(titles.len(), SLOTS, "every slot names its bar");
    assert_eq!((titles[0], titles[1]), (titles[2], titles[3]), "the same bar on two slots is drawn from the one line");
    assert_ne!(titles[0], titles[1]);
    assert_eq!(rig.canvas.composed(), 2, "and that line was composed once");
}

/// Turning the wheel by one bar keeps the titles of the bars that stay on it and composes the one
/// that came on, and handing the screen back hands every title back with it.
#[test]
fn turning_the_wheel_composes_only_the_title_that_came_on() {
    let mut rig = Rig::new("turn");
    let bars = tables(SLOTS * 2);
    rig.draw(&SongBars::new(&bars, CENTER));
    assert_eq!((rig.canvas.composed(), rig.canvas.held()), (SLOTS, SLOTS));

    rig.draw(&SongBars::new(&bars, CENTER + 1));
    assert_eq!((rig.canvas.composed(), rig.canvas.held()), (SLOTS + 1, SLOTS), "one line composed, one handed back");

    rig.screen.release(&mut rig.canvas);
    assert_eq!(rig.canvas.held(), 0);
}

/// A lamp is prepared once a frame and then drawn on every bar without being asked again, so one
/// its condition hides keeps the place it last had. A level and a bar's own image are asked for
/// every bar and do go away.
#[test]
fn a_hidden_lamp_stays_where_it_was_last_placed_and_a_hidden_level_and_bar_go_away() {
    let gated = r#","op":[21]"#;
    let lamps_of = |pieces: &[(Piece, Rect)]| landed(pieces, |piece| matches!(piece, Piece::Lamp(_)));
    let bars = vec![SongBar { level: 7, ..song("CHART") }];
    let list = SongBars::new(&bars, 0);

    let mut rig = Rig::with("stale-lamp", &Wheel { lamp_extra: gated, ..Wheel::default() }, DOC);
    assert_eq!(lamps_of(&rig.pieces(&list)).len(), 0, "a lamp that was never placed draws nothing");
    rig.host.booleans.insert(OPTION_PANEL1, true);
    let shown = lamps_of(&rig.pieces(&list));
    assert_eq!(shown.len(), SLOTS);
    rig.host.booleans.insert(OPTION_PANEL1, false);
    let hidden = lamps_of(&rig.pieces(&list));
    assert!(shown.iter().zip(&hidden).all(|(shown, hidden)| same(*shown, *hidden)) && hidden.len() == SLOTS, "a hidden lamp is drawn where it last was");

    let mut rig = Rig::with("fresh-level", &Wheel { level_extra: gated, ..Wheel::default() }, DOC);
    let digits = |pieces: Vec<(Piece, Rect)>| pieces.iter().filter(|(piece, _)| matches!(piece, Piece::Digit(..))).count();
    rig.host.booleans.insert(OPTION_PANEL1, true);
    assert_eq!(digits(rig.pieces(&list)), SLOTS);
    rig.host.booleans.insert(OPTION_PANEL1, false);
    assert_eq!(digits(rig.pieces(&list)), 0, "a hidden level is not drawn");

    let mut rig = Rig::with("fresh-bar", &Wheel { bar_extra: gated, ..Wheel::default() }, DOC);
    rig.host.booleans.insert(OPTION_PANEL1, true);
    assert_eq!(bar_rects(&rig.pieces(&list)).len(), SLOTS);
    rig.host.booleans.insert(OPTION_PANEL1, false);
    assert_eq!(rig.draw(&list).len(), 0, "a hidden bar takes everything on it with it");
}

#[test]
fn a_rival_splits_every_lamp_into_the_players_and_the_rivals() {
    let mut rig = Rig::new("rival");
    let bars = vec![SongBar { lamp: 6, rival_lamp: 3, ..table(0) }];
    let alone = rig.pieces(&SongBars::new(&bars, 0));
    assert_eq!(kinds(&alone).iter().filter(|piece| matches!(piece, Piece::Lamp(_))).count(), SLOTS);
    assert!(kinds(&alone).iter().all(|piece| !matches!(piece, Piece::Lamp(lamp) if *lamp != 6)));

    let against = rig.pieces(&SongBars { rival: true, ..SongBars::new(&bars, 0) });
    let lamps: Vec<(Piece, Rect)> = against.into_iter().filter(|(piece, _)| matches!(piece, Piece::Lamp(_))).collect();
    assert_eq!(lamps.len(), SLOTS * 2);
    let top = slot_y(0, PITCH);
    assert_eq!(lamps[0].0, Piece::Lamp(6));
    assert!(same(lamps[0].1, part_on(OFF_X, top, PLAYER_LAMP_AT)), "the player's lamp takes the upper half");
    assert_eq!(lamps[1].0, Piece::Lamp(3));
    assert!(same(lamps[1].1, part_on(OFF_X, top, RIVAL_LAMP_AT)), "and the rival's the lower");

    let unknown = vec![SongBar { lamp: 11, rival_lamp: -1, ..table(0) }];
    let pieces = rig.pieces(&SongBars { rival: true, ..SongBars::new(&unknown, 0) });
    assert!(kinds(&pieces).iter().all(|piece| !matches!(piece, Piece::Lamp(_))), "a lamp the wheel has no image under draws nothing");
}

/// A folder's graph sets no colour of its own: it is drawn in whatever the bar image drawn before it
/// left behind, which is the tint of the last slot's bar.
#[test]
fn a_folders_graph_is_drawn_in_the_colour_the_last_bar_left_and_only_while_folders_are_counted() {
    let tinted = Wheel { off_tint: r#","r":200,"g":100,"b":50"#, ..Wheel::default() };
    let mut rig = Rig::with("graph-tint", &tinted, DOC);
    let mut counts = BarDistribution::default();
    (counts.lamps[2], counts.lamps[9]) = (3, 1);
    let mut bars = tables(SLOTS);
    bars[0] = SongBar { distribution: Some(Box::new(counts)), ..SongBar::new(BarKind::Folder, "FOLDER") };
    bars[1] = SongBar { distribution: Some(Box::new(counts)), ..song("NOT A FOLDER") };
    bars[3] = SongBar { distribution: Some(Box::default()), ..SongBar::new(BarKind::Folder, "UNCOUNTED") };

    let quads = rig.draw(&SongBars::new(&bars, CENTER));
    let segments: Vec<&Quad> = quads.iter().filter(|quad| matches!(rig.piece(quad), Piece::Graph(_))).collect();
    assert_eq!(segments.iter().map(|quad| rig.piece(quad)).collect::<Vec<_>>(), [Piece::Graph(9), Piece::Graph(2)], "one folder, best lamp first");
    assert!(segments.iter().all(|quad| quad.params.tint == Color { r: 200, g: 100, b: 50, a: 255 }), "in the tint of the bar drawn last");
    let widths: Vec<f32> = segments.iter().map(|quad| quad.params.dst.w).collect();
    assert!((widths[0] - GRAPH_AT.2 as f32 / 4.0).abs() < NEAR && (widths[1] - GRAPH_AT.2 as f32 * 3.0 / 4.0).abs() < NEAR);

    let uncounted = rig.pieces(&SongBars { folder_lamps: false, ..SongBars::new(&bars, CENTER) });
    assert!(kinds(&uncounted).iter().all(|piece| !matches!(piece, Piece::Graph(_))), "with folders left uncounted no graph is drawn");
}

#[test]
fn a_rank_graph_shares_the_bar_by_rank_over_the_charts_counted_by_lamp() {
    let mut rig = Rig::with("graph-rank", &Wheel { graph_type: -2, ..Wheel::default() }, DOC);
    let mut counts = BarDistribution::default();
    counts.lamps[4] = 4;
    (counts.ranks[27], counts.ranks[12], counts.ranks[0]) = (1, 2, 1);
    let mut bars = tables(SLOTS);
    bars[CENTER] = SongBar { distribution: Some(Box::new(counts)), ..SongBar::new(BarKind::Folder, "FOLDER") };

    let pieces = rig.pieces(&SongBars::new(&bars, CENTER));
    let segments: Vec<(Piece, Rect)> = pieces.into_iter().filter(|(piece, _)| matches!(piece, Piece::Graph(_))).collect();
    assert_eq!(kinds(&segments), [Piece::Graph(27), Piece::Graph(12), Piece::Graph(0)]);
    let quarter = GRAPH_AT.2 as f32 / 4.0;
    let from = (ON_X + GRAPH_AT.0) as f32;
    for ((_, rect), (before, share)) in segments.iter().zip([(0.0, 1.0), (1.0, 2.0), (3.0, 1.0)]) {
        assert!((rect.x - (from + before * quarter)).abs() < NEAR && (rect.w - share * quarter).abs() < NEAR, "a rank segment landed on {rect:?}");
    }
}

/// A frame of plain bars with the wheel `remaining` of a slot's travel from rest, towards the next
/// bar when `forward`.
fn sliding(bars: &[SongBar], forward: bool, travel_ms: i32, left_ms: i64) -> SongBars<'_> {
    const NOW_MS: i64 = 1_700_000_000_000;
    let scroll = BarScroll { duration_ms: NOW_MS + left_ms, angle: if forward { travel_ms } else { -travel_ms }, now_ms: NOW_MS };
    SongBars { scroll, ..SongBars::new(bars, CENTER) }
}

#[test]
fn a_slide_carries_every_bar_in_from_the_slot_beside_it_along_a_straight_line() {
    let mut rig = Rig::new("slide");
    let bars = tables(SLOTS * 2);
    let rest: Vec<(i32, i32)> = (0..SLOTS).map(|slot| (bar_x(slot), slot_y(slot, PITCH))).collect();
    let at = |x: f32, y: f32| Rect::new(x, DOC.1 as f32 - (y + BAR_H as f32), BAR_W as f32, BAR_H as f32);
    let between = |slot: usize, beside: usize, share: f32| {
        let (from, to) = (rest[slot], rest[beside]);
        at((from.0 as f32 + (to.0 - from.0) as f32 * share).trunc(), (from.1 as f32 + (to.1 - from.1) as f32 * share).trunc())
    };

    let started = bar_rects(&rig.pieces(&sliding(&bars, true, 300, 300)));
    for (slot, landed) in started.iter().enumerate().take(SLOTS - 1) {
        assert!(same(*landed, between(slot, slot + 1, 1.0)), "a slide towards the next bar starts each bar on the slot below: {landed:?}");
    }
    assert!(same(started[SLOTS - 1], between(SLOTS - 1, SLOTS - 1, 0.0)), "the last slot has no slot below to come from");

    let halfway = bar_rects(&rig.pieces(&sliding(&bars, true, 300, 150)));
    assert!(same(halfway[1], between(1, CENTER, 0.5)), "half the time is half the way, across as well as down: {:?}", halfway[1]);
    let third = bar_rects(&rig.pieces(&sliding(&bars, true, 300, 100)));
    assert!(same(third[0], at(OFF_X as f32, 226.0)), "a place between two pixels is cut to the lower one: {:?}", third[0]);

    let back = bar_rects(&rig.pieces(&sliding(&bars, false, 300, 150)));
    assert!(same(back[CENTER], between(CENTER, CENTER - 1, 0.5)), "a slide towards the bar before comes from the slot above: {:?}", back[CENTER]);
    assert!(same(back[0], between(0, 0, 0.0)), "and the first slot has no slot above");

    let queued = bar_rects(&rig.pieces(&sliding(&bars, true, 15, 30)));
    let (x, y) = (rest[CENTER].0 as f32, (rest[1].1 - 2 * PITCH) as f32);
    assert!(same(queued[1], at(x, y)), "with a second notch queued a bar starts two slots down and one across: {:?}", queued[1]);

    let over = bar_rects(&rig.pieces(&sliding(&bars, true, 300, 0)));
    assert!((0..SLOTS).all(|slot| same(over[slot], between(slot, slot, 0.0))), "a slide whose time is up leaves every bar at rest");
}

/// The cut to a whole pixel is made on the screen, after the document has been scaled to it.
#[test]
fn a_sliding_bar_is_cut_to_a_whole_screen_pixel_whatever_size_the_document_is_drawn_at() {
    let half = (DOC.0 / 2, DOC.1 / 2);
    let mut rig = Rig::with("slide-scaled", &Wheel::default(), half);
    let bars = tables(SLOTS * 2);
    let third = bar_rects(&rig.pieces(&sliding(&bars, true, 300, 100)));
    let wanted = Rect::new(OFF_X as f32 / 2.0, (DOC.1 as f32 - 226.0 - BAR_H as f32) / 2.0, BAR_W as f32 / 2.0, BAR_H as f32 / 2.0);
    assert!(same(third[0], wanted), "two hundred and twenty-six and two thirds is a hundred and thirteen pixels up a half-size screen: {:?}", third[0]);
}

#[test]
fn the_parts_of_a_sliding_bar_slide_with_it() {
    let mut rig = Rig::new("slide-parts");
    let bars = tables(SLOTS * 2);
    let pieces = rig.pieces(&sliding(&bars, true, 300, 150));
    let lamps = landed(&pieces, |piece| matches!(piece, Piece::Lamp(_)));
    let bar = bar_rects(&pieces)[0];
    assert!(same(lamps[0], Rect::new(bar.x + LAMP_AT.0 as f32, bar.y + (BAR_H - LAMP_AT.1 - LAMP_AT.3) as f32, LAMP_AT.2 as f32, LAMP_AT.3 as f32)));
}

/// The one action a press came to.
fn pressed(actions: &[SkinAction]) -> SkinAction {
    match actions {
        [action] => *action,
        other => panic!("expected exactly one action, got {other:?}"),
    }
}

#[test]
fn a_press_on_a_clickable_bar_names_the_bar_and_any_other_button_closes_the_folder() {
    let mut rig = Rig::new("press");
    let bars = tables(SLOTS * 2);
    let list = SongBars::new(&bars, 7);
    let prepared = rig.prepare(&list, 0);
    let map = rig.screen.input_map(&prepared);
    assert_eq!(map.len(), 3, "the three slots the document called clickable");

    let inside = |slot: usize| ((bar_x(slot) + 5) as f32, (slot_y(slot, PITCH) + 5) as f32);
    assert_eq!(pressed(&map.press(SkinPointerButton::Left, inside(1))), SkinAction::SelectBar { slot: 1, offset: -1, bar: 6 });
    assert_eq!(pressed(&map.press(SkinPointerButton::Left, inside(CENTER))), SkinAction::SelectBar { slot: CENTER, offset: 0, bar: 7 });
    assert_eq!(pressed(&map.press(SkinPointerButton::Left, inside(3))), SkinAction::SelectBar { slot: 3, offset: 1, bar: 8 });
    for button in [SkinPointerButton::Right, SkinPointerButton::Middle, SkinPointerButton::Back, SkinPointerButton::Forward] {
        assert_eq!(pressed(&map.press(button, inside(3))), SkinAction::CloseBar, "{button:?}");
    }
    assert_eq!(map.press(SkinPointerButton::Left, inside(0)), Vec::new(), "a slot the document did not call clickable takes nothing");
    assert_eq!(map.press(SkinPointerButton::Left, (10.0, 10.0)), Vec::new());
    assert_eq!(map.drag(inside(CENTER)), Vec::new(), "a drag is a slider's and never a bar's");

    let edge = ((OFF_X + BAR_W) as f32, (slot_y(1, PITCH) + BAR_H) as f32);
    assert_eq!(pressed(&map.press(SkinPointerButton::Left, edge)), SkinAction::SelectBar { slot: 1, offset: -1, bar: 6 }, "a bar's edges are part of it");
}

/// A bar is pressed where its slot puts it, not where a slide is carrying it, and a bar the wheel
/// does not draw takes nothing.
#[test]
fn a_press_is_judged_against_a_bar_at_rest_and_only_against_a_bar_that_is_drawn() {
    let mut rig = Rig::new("press-slide");
    let mut bars = tables(SLOTS * 2);
    let prepared = rig.prepare(&sliding(&bars, true, 300, 300), 0);
    let map = rig.screen.input_map(&prepared);
    let at_rest = ((OFF_X + BAR_W - 2) as f32, (slot_y(1, PITCH) + BAR_H - 2) as f32);
    assert_eq!(pressed(&map.press(SkinPointerButton::Left, at_rest)), SkinAction::SelectBar { slot: 1, offset: -1, bar: 1 });
    let passing = ((ON_X + 2) as f32, (slot_y(CENTER, PITCH) + BAR_H + 5) as f32);
    assert_eq!(map.press(SkinPointerButton::Left, passing), Vec::new(), "the gap a bar slides through is not the bar");

    bars[1] = SongBar::new(BarKind::Other, "SAME FOLDER");
    let prepared = rig.prepare(&SongBars::new(&bars, CENTER), 0);
    let map = rig.screen.input_map(&prepared);
    assert_eq!(map.len(), 2);
    assert_eq!(map.press(SkinPointerButton::Left, at_rest), Vec::new());

    let none: Vec<SongBar> = Vec::new();
    let prepared = rig.prepare(&SongBars::new(&none, 0), 0);
    assert!(rig.screen.input_map(&prepared).is_empty(), "an empty list has no bar to press");
}

/// The wheel takes a press where it stands among the document's objects: an image drawn after it
/// and over one of its bars is offered the press first.
#[test]
fn an_object_drawn_over_a_bar_takes_the_press_before_the_bar_does() {
    let cover = format!(r#",{{"id":"cover","dst":[{{"x":{OFF_X},"y":{},"w":40,"h":{BAR_H}}}]}}"#, slot_y(1, PITCH));
    let mut rig = Rig::with("press-cover", &Wheel { over: &cover, ..Wheel::default() }, DOC);
    let bars = tables(SLOTS);
    let prepared = rig.prepare(&SongBars::new(&bars, CENTER), 0);
    let map = rig.screen.input_map(&prepared);

    let covered = ((OFF_X + 5) as f32, (slot_y(1, PITCH) + 5) as f32);
    assert!(matches!(pressed(&map.press(SkinPointerButton::Left, covered)), SkinAction::Event { .. }));
    let beside = ((OFF_X + 60) as f32, (slot_y(1, PITCH) + 5) as f32);
    assert_eq!(pressed(&map.press(SkinPointerButton::Left, beside)), SkinAction::SelectBar { slot: 1, offset: -1, bar: 1 });
}

/// A slot's bar has to name an image set: the reference looks a bar up nowhere else, so a wheel
/// whose slots name a plain image has no slot it can draw and is refused.
#[test]
fn a_wheel_whose_bars_name_a_plain_image_is_refused_and_says_so() {
    let rig = Rig::with("plain-image", &Wheel { bar_id: "bar-0", ..Wheel::default() }, DOC);
    assert_eq!(rig.screen.count_of(SkinObjectKind::SongList), 0);
    assert!(rig.screen.warnings().iter().any(|warning| warning.contains("bar-0") && warning.contains("not an image set")), "{:?}", rig.screen.warnings());
}

#[test]
fn a_held_key_moves_one_bar_waits_and_then_repeats() {
    const START: i64 = 1_000_000;
    let low = i64::from(SCROLL_DURATION_LOW_MS);
    let high = i64::from(SCROLL_DURATION_HIGH_MS);
    let mut scroller = BarScroller::default();

    assert_eq!(scroller.input(0, BarHold::Next, START), 1, "the press itself moves the cursor");
    assert_eq!(scroller.at(START), BarScroll { duration_ms: START + low, angle: SCROLL_DURATION_LOW_MS, now_ms: START });
    assert_eq!(scroller.at(START).remaining(), Some(1.0));
    assert_eq!(scroller.input(0, BarHold::Next, START + low), 0, "and nothing more until the first slide is over");
    assert_eq!(scroller.input(0, BarHold::Next, START + low + 1), 1);
    assert_eq!(scroller.at(START + low + 1), BarScroll { duration_ms: START + low + 1 + high, angle: SCROLL_DURATION_HIGH_MS, now_ms: START + low + 1 });
    assert_eq!(scroller.input(0, BarHold::Next, START + low + 1 + high), 0);
    assert_eq!(scroller.input(0, BarHold::Next, START + low + 2 + high), 1, "each repeat is a short slide");

    assert_eq!(scroller.input(0, BarHold::None, START + low + 3 + high), 0);
    assert!(scroller.at(START + low + 3 + high).remaining().is_some(), "letting go leaves the slide under way to finish");
    assert_eq!(scroller.input(0, BarHold::None, START + low + 3 + 2 * high), 0);
    assert_eq!(scroller.at(START + low + 3 + 2 * high).duration_ms, 0, "and a finished slide leaves the wheel at rest");

    assert_eq!(scroller.input(0, BarHold::Previous, START), -1);
    assert_eq!(scroller.at(START + low / 2).remaining(), Some(0.5), "a slide back is measured the same way");
    assert_eq!(scroller.at(START).neighbour(0), None);
    assert_eq!(scroller.at(START).neighbour(3), Some(2));
}

#[test]
fn the_mouse_wheel_queues_two_notches_and_travels_faster_with_two() {
    const START: i64 = 1_000_000;
    let mut scroller = BarScroller::default();

    assert_eq!(scroller.input(1, BarHold::None, START), 1);
    assert_eq!(scroller.at(START), BarScroll { duration_ms: START + 120, angle: 120, now_ms: START });
    assert_eq!(scroller.input(1, BarHold::None, START), 1);
    assert_eq!(scroller.at(START), BarScroll { duration_ms: START + 30, angle: 15, now_ms: START }, "a second notch before the first has moved");
    assert_eq!(scroller.at(START).remaining(), Some(2.0), "starts each bar two slots away");
    assert_eq!(scroller.input(3, BarHold::None, START + 11), 3, "every notch moves the cursor, but no more than two are queued");
    assert_eq!(scroller.at(START + 11), BarScroll { duration_ms: START + 41, angle: 15, now_ms: START + 11 });

    assert_eq!(scroller.input(-1, BarHold::None, START + 12), -1);
    assert_eq!(scroller.at(START + 12), BarScroll { duration_ms: START + 12, angle: 0, now_ms: START + 12 }, "a notch the other way cancels the one queued");
    assert_eq!(scroller.at(START + 12).remaining(), None);

    assert_eq!(scroller.input(-1, BarHold::None, START + 500), -1);
    assert_eq!(scroller.at(START + 500), BarScroll { duration_ms: START + 620, angle: -120, now_ms: START + 500 });

    let mut held = BarScroller::default();
    assert_eq!(held.input(-1, BarHold::Next, START), -1, "a key held while a slide is under way does not move the cursor again");
    let mut rested = BarScroller::default();
    rested.input(1, BarHold::None, START);
    rested.reset_input(START + 121);
    assert_eq!(rested.at(START + 121).duration_ms, 0, "a slide that ended while a panel was open is let go");
}
