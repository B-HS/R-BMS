//! A play document's `bga` object, end to end: read from a document, placed more than once, and
//! drawn from what a frame supplies.
//!
//! The document is written for the test and uses the shape of the pack the object was written
//! against: one `bga` placed twice, first with a stretch of its own and translucent, then plainly.

use std::path::{Path, PathBuf};

use rbms_render::skin_render::frame::{BgaEvent, BgaExpand, BgaPicture, BgaPlayhead, BgaTextures, DEFAULT_MISS_LAYER_DURATION_MS};
use rbms_render::{BgaFrame, Color, CpuCanvas, FrameData, RenderCtx, Renderer, SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen, TextContext};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::property::DefaultState;
use rbms_skin::timer::TimerState;

/// The side of the square canvas the document is authored at and drawn on.
const SIDE: u32 = 600;

/// The picture's size: twice as wide as it is tall, so a fit that keeps its shape is visible.
const PICTURE: (u32, u32) = (300, 150);

/// A plain stand-in for every grey the object does not draw on.
const GROUND: Color = Color { r: 200, g: 200, b: 200, a: 255 };

/// The opacity of the document's first placement.
const FIRST_ALPHA: u8 = 100;

/// A folder the test writes its document in, removed when it is dropped.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-bga-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }

    fn write(&self, name: &str, body: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::write(&path, body).expect("the scratch file is writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A document with no pictures of its own.
struct NoAssets;

impl SkinAssets for NoAssets {
    fn image(&mut self, _path: &Path) -> Option<SkinImage> {
        None
    }
}

/// `PICTURE` pixels of one colour.
fn flat(color: [u8; 4]) -> Vec<u8> {
    (0..PICTURE.0 * PICTURE.1).flat_map(|_| color).collect()
}

fn pixel(canvas: &CpuCanvas, x: u32, y: u32) -> [u8; 4] {
    let color = canvas.pixel_at(x, y);
    [color.r, color.g, color.b, color.a]
}

/// Loads `body` as a document, builds its screen on `canvas` and draws one frame carrying `bga`
/// over a grey ground. Answers how many `bga` objects the screen holds and how many objects drew.
fn draw_document(tag: &str, body: &str, canvas: &mut CpuCanvas, bga: BgaFrame) -> (usize, usize) {
    rbms_render::font::use_embedded_fonts_only();
    let scratch = Scratch::new(tag);
    let document = scratch.write("skin.json", body);
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&scratch.root, &user, rbms_model::Mode::BEAT_7K) };
    let skin = load_skin(&document, options).expect("the document loads");
    let mut text = TextContext::embedded_only();
    let screen = SkinScreen::build(canvas, &mut text, &skin, &mut NoAssets);

    let timers = TimerState::new();
    canvas.clear(GROUND);
    let mut ctx = RenderCtx::new(rbms_render::theme(), &mut text);
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &DefaultState, lua: None, mouse: None, data: FrameData { bga, ..FrameData::default() } };
    let drawn = screen.draw(&mut ctx, canvas, &frame);
    (screen.count_of(SkinObjectKind::Background), drawn)
}

/// A document that places its `bga` twice: over the whole canvas with a stretch of its own and
/// translucent, and then plainly.
fn two_placements() -> String {
    format!(
        r#"{{
            "type": 0, "w": {SIDE}, "h": {SIDE},
            "bga": {{ "id": "bga" }},
            "destination": [
                {{ "id": "bga", "stretch": 0, "dst": [{{ "time": 0, "x": 0, "y": 0, "w": {SIDE}, "h": {SIDE}, "a": {FIRST_ALPHA} }}] }},
                {{ "id": "bga", "dst": [{{ "time": 0, "x": 0, "y": 0, "w": {SIDE}, "h": {SIDE} }}] }}
            ]
        }}"#
    )
}

#[test]
fn a_document_that_places_its_background_twice_draws_it_twice_each_with_its_own_stretch_and_opacity() {
    let mut canvas = CpuCanvas::new(SIDE, SIDE);
    let green = flat([0, 255, 0, 255]);
    let image = canvas.register_texture("test.bga", &green, PICTURE.0, PICTURE.1);

    let (objects, drawn) = draw_document("twice", &two_placements(), &mut canvas, BgaFrame::playing(Some(image), None, BgaExpand::KeepAspectRatio));

    assert_eq!(objects, 2, "each placement of the bga is an object of its own");
    assert_eq!(drawn, 2, "each placement draws what the frame carries");
    assert_eq!(pixel(&canvas, 300, 300), [0, 255, 0, 255], "the plain placement is not on top, opaque");
    let beyond_the_fit = pixel(&canvas, 300, 50);
    assert!(
        (118..=126).contains(&beyond_the_fit[0]) && (216..=226).contains(&beyond_the_fit[1]),
        "the stretch the first placement names did not fill the rectangle at its own opacity: {beyond_the_fit:?}"
    );
}

#[test]
fn the_player_setting_fits_the_placement_that_names_no_stretch() {
    let mut canvas = CpuCanvas::new(SIDE, SIDE);
    let green = flat([0, 255, 0, 255]);
    let image = canvas.register_texture("test.bga", &green, PICTURE.0, PICTURE.1);
    let ground = [GROUND.r, GROUND.g, GROUND.b, GROUND.a];
    let single = r#"{
        "type": 0, "w": 600, "h": 600,
        "bga": { "id": "bga" },
        "destination": [{ "id": "bga", "dst": [{ "time": 0, "x": 0, "y": 0, "w": 600, "h": 600 }] }]
    }"#;

    draw_document("full", single, &mut canvas, BgaFrame::playing(Some(image), None, BgaExpand::Full));
    assert_eq!(pixel(&canvas, 300, 50), [0, 255, 0, 255], "FULL left the top of the rectangle empty");

    draw_document("keep", single, &mut canvas, BgaFrame::playing(Some(image), None, BgaExpand::KeepAspectRatio));
    assert_eq!(pixel(&canvas, 300, 50), ground, "KEEP_ASPECT_RATIO stretched the picture over the whole rectangle");
    assert_eq!(pixel(&canvas, 300, 300), [0, 255, 0, 255]);

    draw_document("off", single, &mut canvas, BgaFrame::playing(Some(image), None, BgaExpand::Off));
    assert_eq!(pixel(&canvas, 100, 300), ground, "OFF enlarged the picture");
    assert_eq!(pixel(&canvas, 300, 300), [0, 255, 0, 255]);
}

#[test]
fn the_chart_decides_what_the_documents_background_shows_from_before_play_to_the_miss_layer() {
    let mut canvas = CpuCanvas::new(SIDE, SIDE);
    let mut textures = BgaTextures::default();
    let (base, miss) = (flat([0, 0, 255, 255]), flat([255, 0, 0, 255]));
    let mut head = BgaPlayhead::new(vec![BgaEvent { time_ms: 0, base: 0, layer: -1, miss: Some(vec![1]) }]);
    let pictures = [&base, &miss];
    let mut shown = |canvas: &mut CpuCanvas, play_ms: i64, missed_at_ms: i64| {
        head.prepare(play_ms);
        head.start_miss(missed_at_ms, DEFAULT_MISS_LAYER_DURATION_MS);
        let frame = textures.frame(canvas, head.pick(), BgaExpand::Full, |number| {
            let rgba = pictures.get(usize::try_from(number).ok()?)?;
            Some(BgaPicture { generation: u64::from(number.unsigned_abs()), width: PICTURE.0, height: PICTURE.1, rgba })
        });
        draw_document("chart", &two_placements(), canvas, frame);
        pixel(canvas, 300, 300)
    };

    assert_eq!(shown(&mut canvas, -1, 0), [0, 0, 0, 255], "before play the rectangle is black");
    assert_eq!(shown(&mut canvas, 100, 0), [0, 0, 255, 255], "while playing it shows the chart's picture");
    assert_eq!(shown(&mut canvas, 600, 500), [255, 0, 0, 255], "right after a miss it shows the miss layer");
    assert_eq!(shown(&mut canvas, 1_100, 500), [0, 0, 255, 255], "once the miss layer has run its time the picture is back");
}
