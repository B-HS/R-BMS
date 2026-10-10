//! Tests of the editable text: which texts the reference lets one type into, where a press on one
//! lands, what it starts from, and what it draws while it is typed into.
//!
//! The drawing is done onto a [`CpuCanvas`] through the dispatch a real frame goes through, in the
//! bundled face loaded the way a skin loads its own font, so what is asserted is the pixels a screen
//! would have shown. The screens that are built from a document use a small Lua skin written out here.

use std::path::PathBuf;

use rbms_skin::dst::{DestinationTrack, Keyframe, LuaFnId, SkinColor, SkinRect};
use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, StretchKind};
use rbms_skin::lua::LuaBudget;
use rbms_skin::model::{StringWriterRef, TextDef};
use rbms_skin::property::MapHost;
use rbms_skin::property::generated::{STRING_SEARCHWORD, STRING_TITLE};
use rbms_skin::timer::TimerState;

use super::{Composition, SkinTextWriter, TextEntry, TextEntryStart, input_bounds, is_editable, text_input_body, text_writer};
use crate::ctx::RenderCtx;
use crate::font::{BlockAlign, BlockFit, BlockSpec, TextContext};
use crate::skin_render::draw::draw_resolved;
use crate::skin_render::object::{Body, SkinObject};
use crate::skin_render::text::{FontRef, Fonts};
use crate::skin_render::{FrameData, SkinAction, SkinAssets, SkinFrame, SkinPointer, SkinPointerButton, SkinScreen, SkinViewport};
use crate::{Color, CpuCanvas, Renderer, Theme};

/// The canvas every line is drawn on, which is also the size the document is authored at.
const STAGE: (u32, u32) = (640, 240);

/// What the canvas is cleared to.
const BACKDROP: Color = Color { r: 100, g: 100, b: 100, a: u8::MAX };

/// The id the document registers its font under.
const FONT_ID: &str = "0";

/// The em size the lines are drawn at, and the room they are given.
const EM: i32 = 40;
const ROOM: f32 = 300.0;

/// The anchor the left-aligned lines start from, and how far above the document's foot they sit.
const LEFT: f32 = 100.0;
const FOOT: f32 = 100.0;

/// A string id the reference has no property under that would be written.
const READ_ONLY_STRING: i32 = STRING_TITLE;

/// The widest a caret bar may be on this stage: the reference's cursor is two pixels wide.
const CARET_PX: i32 = 2;

fn engine() -> (TextContext, Vec<(String, FontRef)>) {
    let mut text = TextContext::embedded_only();
    let family = text.load_font(include_bytes!("../../../../../assets/fonts/Inter-Regular.ttf").to_vec()).expect("the bundled font loads as a skin font");
    (text, vec![(FONT_ID.to_string(), FontRef::Family(family))])
}

fn record(reference: i32) -> TextDef {
    TextDef { font: FONT_ID.to_string(), size: EM, reference, ..TextDef::default() }
}

fn track(x: f32) -> DestinationTrack {
    let white = SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX);
    let rect = SkinRect::new(x, FOOT, ROOM, EM as f32);
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color: white, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

fn object(def: &TextDef, families: &Fonts, x: f32) -> SkinObject {
    SkinObject { track: track(x), stretch: StretchKind::Stretch, body: Body::TextInput(text_input_body(def, families)) }
}

fn stage() -> CpuCanvas {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    canvas.clear(BACKDROP);
    canvas
}

/// Draws the object at index `index` of its screen the way a frame does, with `entry` being typed into.
fn paint(text: &mut TextContext, canvas: &mut CpuCanvas, object: &SkinObject, index: usize, entry: Option<TextEntry<'_>>) {
    let timers = TimerState::new();
    let host = MapHost::new();
    let data = FrameData { entry, ..FrameData::default() };
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &host, lua: None, mouse: None, data };
    let viewport = SkinViewport::new((STAGE.0 as f32, STAGE.1 as f32), (STAGE.0 as f32, STAGE.1 as f32));
    let mut ctx = RenderCtx::new(Theme::default(), text);
    let resolved = object.prepare(&frame).expect("the text is placed");
    draw_resolved(&mut ctx, canvas, object, index, &viewport, &frame, &resolved);
}

/// Every column of the canvas that holds a pixel other than the backdrop, within the rows of the text's
/// destination.
fn lit_columns(canvas: &CpuCanvas) -> Vec<i32> {
    let top = STAGE.1 - (FOOT as u32 + EM as u32);
    (0..STAGE.0).filter(|x| (top..top + EM as u32).any(|y| canvas.pixel_at(*x, y) != BACKDROP)).map(|x| x as i32).collect()
}

/// Every pixel of a canvas, row by row, which is what two canvases are compared by.
fn pixels(canvas: &CpuCanvas) -> Vec<Color> {
    (0..STAGE.1).flat_map(|y| (0..STAGE.0).map(move |x| (x, y))).map(|(x, y)| canvas.pixel_at(x, y)).collect()
}

fn entry<'a>(typed: &'a str, caret: usize, composing: Option<Composition<'a>>) -> TextEntry<'a> {
    TextEntry { object: 0, typed, caret, composing }
}

#[test]
fn a_text_is_editable_when_the_document_says_so_or_the_reference_can_write_its_ref_and_no_event_is_named() {
    let searching = record(STRING_SEARCHWORD);
    assert!(is_editable(&searching), "a text whose ref has a writer is editable without saying so");
    assert_eq!(text_writer(&searching), Some(SkinTextWriter::Id(STRING_SEARCHWORD)));

    let plain = record(READ_ONLY_STRING);
    assert!(!is_editable(&plain), "a text whose ref has no writer is just text");
    assert_eq!(text_writer(&plain), None);

    let declared = TextDef { editable: true, ..record(READ_ONLY_STRING) };
    assert!(is_editable(&declared), "a text the document marks editable is");
    assert_eq!(text_writer(&declared), None, "but it has nowhere to write, so it will not take focus");

    let function = StringWriterRef::Lua(LuaFnId(7));
    let scripted = TextDef { event: Some(function.clone()), ..record(STRING_SEARCHWORD) };
    assert!(!is_editable(&scripted), "a text with an event of its own is editable only when it says so");
    assert_eq!(text_writer(&scripted), Some(SkinTextWriter::Function(LuaFnId(7))), "and its event wins over its ref");
    assert!(is_editable(&TextDef { editable: true, ..scripted }));

    let named = TextDef { event: Some(StringWriterRef::Name("searchword".to_string())), editable: true, ..record(0) };
    assert_eq!(text_writer(&named), Some(SkinTextWriter::Id(STRING_SEARCHWORD)), "a name the reference has a writer under is that writer");
    let unwritable = TextDef { event: Some(StringWriterRef::Name("title".to_string())), ..record(STRING_SEARCHWORD) };
    assert_eq!(text_writer(&unwritable), Some(SkinTextWriter::Id(STRING_SEARCHWORD)), "a name with no writer names no event, so the ref's writer stands");
    assert!(is_editable(&unwritable));
}

#[test]
fn the_bounds_a_press_is_taken_in_follow_the_alignment_of_the_text() {
    let region = SkinRect::new(300.0, 50.0, 100.0, 30.0);
    assert_eq!(input_bounds(region, 0), region, "a left-aligned text is typed into where it is");
    assert_eq!(input_bounds(region, 1), SkinRect::new(250.0, 50.0, 100.0, 30.0), "a centred text from half its width before its x");
    assert_eq!(input_bounds(region, 2), SkinRect::new(200.0, 50.0, 100.0, 30.0), "a right-aligned text ends at its x");
    assert_eq!(input_bounds(region, 9), region, "an alignment the reference has no case for is the left end");
}

#[test]
fn the_caret_stands_between_the_glyphs_a_line_is_composed_with() {
    let (mut text, families) = engine();
    let family = families[0].1.family().expect("the bundled font is one of the text engine's");
    let spec = |line: &'static str, align: BlockAlign, fit: BlockFit| BlockSpec {
        text: line,
        family,
        em_px: EM as f32,
        design_px: EM as f32,
        width: ROOM,
        align,
        fit,
        max_dim: u32::MAX,
    };

    let line = spec("hello", BlockAlign::Left, BlockFit::Overflow);
    let stops: Vec<f32> = (0..=5).map(|at| text.caret_x(&line, at).expect("the font has a capital")).collect();
    assert!(stops.windows(2).all(|pair| pair[0] < pair[1]), "each character moves the caret on: {stops:?}");
    assert!(stops[0] >= 0.0 && stops[0] < 4.0, "the caret starts at the anchor: {}", stops[0]);
    assert_eq!(text.caret_x(&line, 99), Some(stops[5]), "past the end is the end");

    let block = text.compose_block(&line).expect("the line has ink");
    assert!(
        (stops[5] - block.layout_width).abs() <= 4.0,
        "the end of the line is where its ink ends, give or take the last glyph's bearing: {stops:?} {}",
        block.layout_width
    );

    let spaced = spec("hi ", BlockAlign::Left, BlockFit::Overflow);
    let trimmed = spec("hi", BlockAlign::Left, BlockFit::Overflow);
    assert!(text.caret_x(&spaced, 3).unwrap() > text.caret_x(&trimmed, 2).unwrap(), "a space the player typed moves the caret even though it has no ink");

    let empty = spec("", BlockAlign::Left, BlockFit::Overflow);
    assert_eq!(text.caret_x(&empty, 0), Some(0.0));
    assert_eq!(text.caret_x(&spec("", BlockAlign::Center, BlockFit::Overflow), 0), Some(ROOM / 2.0), "an empty centred line has its caret in the middle");
    assert_eq!(text.caret_x(&spec("", BlockAlign::Right, BlockFit::Overflow), 0), Some(ROOM));

    let right = spec("hello", BlockAlign::Right, BlockFit::Overflow);
    let end = text.caret_x(&right, 5).expect("a right-aligned line");
    assert!((end - ROOM).abs() <= 4.0, "a right-aligned line ends at the far edge of its room, where its caret is: {end}");
    let shrunk = spec("hello hello hello hello hello hello", BlockAlign::Left, BlockFit::Shrink);
    let squeezed = text.caret_x(&shrunk, 35).expect("a squeezed line");
    assert!(squeezed <= ROOM + 4.0, "a squeezed line keeps its caret inside its room: {squeezed}");
}

#[test]
fn what_is_being_typed_is_drawn_in_place_of_what_the_text_shows_and_a_caret_follows_it() {
    let (mut text, families) = engine();
    let shown = TextDef { constant_text: Some("SHOWN".to_string()), editable: true, ..record(0) };
    let object = object(&shown, &families, LEFT);

    let mut idle = stage();
    paint(&mut text, &mut idle, &object, 0, None);
    let idle_columns = lit_columns(&idle);
    assert!(!idle_columns.is_empty(), "the text shows what the document gave it while nothing is typed");

    let mut typing = stage();
    paint(&mut text, &mut typing, &object, 0, Some(entry("i", 1, None)));
    let typed_columns = lit_columns(&typing);
    assert_ne!(pixels(&idle), pixels(&typing), "the typed line replaces the shown one");
    let bar_right = *typed_columns.last().expect("something is drawn");
    assert!(bar_right > LEFT as i32 && bar_right < LEFT as i32 + EM, "one narrow letter and its caret stay close to the anchor: {typed_columns:?}");
    assert!(
        typed_columns.contains(&(bar_right - CARET_PX + 1)),
        "the caret is a bar as wide as the reference's cursor at the end of what was typed: {typed_columns:?}"
    );

    let mut elsewhere = stage();
    paint(&mut text, &mut elsewhere, &object, 3, Some(entry("i", 1, None)));
    assert_eq!(pixels(&elsewhere), pixels(&idle), "an entry for another object of the screen does not change this one");

    let mut empty = stage();
    paint(&mut text, &mut empty, &object, 0, Some(entry("", 0, None)));
    let caret_only = lit_columns(&empty);
    assert!(!caret_only.is_empty() && caret_only.len() <= CARET_PX as usize + 2, "an empty entry is drawn as its caret alone: {caret_only:?}");
    assert!(caret_only.iter().all(|x| (*x - LEFT as i32).abs() <= 4), "standing at the anchor: {caret_only:?}");
}

#[test]
fn the_caret_stands_where_the_player_put_it_inside_what_was_typed() {
    let (mut text, families) = engine();
    let def = TextDef { editable: true, ..record(0) };
    let object = object(&def, &families, LEFT);
    let caret_at = |text: &mut TextContext, caret: usize| {
        let mut canvas = stage();
        paint(text, &mut canvas, &object, 0, Some(entry("MMMM", caret, None)));
        lit_columns(&canvas)
    };

    let at_start = caret_at(&mut text, 0);
    let at_end = caret_at(&mut text, 4);
    assert!(at_start.last() < at_end.last(), "a caret at the end of the line reaches further than one at its start: {at_start:?} {at_end:?}");
    assert!(at_start.first() <= at_end.first());
}

#[test]
fn text_an_input_method_is_composing_is_drawn_at_the_caret_underlined_and_the_caret_follows_its_own() {
    let (mut text, families) = engine();
    let def = TextDef { editable: true, ..record(0) };
    let object = object(&def, &families, LEFT);

    let mut plain = stage();
    paint(&mut text, &mut plain, &object, 0, Some(entry("ab", 2, None)));
    let mut composing = stage();
    paint(&mut text, &mut composing, &object, 0, Some(entry("ab", 2, Some(Composition { text: "cd", caret: Some(2) }))));
    let mut hidden = stage();
    paint(&mut text, &mut hidden, &object, 0, Some(entry("ab", 2, Some(Composition { text: "cd", caret: None }))));

    let plain_columns = lit_columns(&plain);
    let composing_columns = lit_columns(&composing);
    assert!(composing_columns.last() > plain_columns.last(), "composing text extends the line: {composing_columns:?} {plain_columns:?}");
    assert_ne!(pixels(&composing), pixels(&hidden), "an input method that asks for no caret has it hidden");
    let underline_row = STAGE.1 - FOOT as u32 - 1;
    let underlined = (0..STAGE.0).filter(|x| composing.pixel_at(*x, underline_row) != BACKDROP).count();
    assert!(underlined > 0, "the composing text is underlined along the bottom of the destination");
    let underlined_hidden = (0..STAGE.0).filter(|x| hidden.pixel_at(*x, underline_row) != BACKDROP).count();
    assert!(underlined_hidden > 0 && underlined_hidden <= underlined, "the underline stays when the caret is hidden");
    let plain_underlined = (0..STAGE.0).filter(|x| plain.pixel_at(*x, underline_row) != BACKDROP).count();
    assert!(plain_underlined < underlined, "text that is not being composed is not underlined");
}

/// A Lua skin with four texts: the search box with nothing said about it, a title that is only text,
/// one with an event of its own that does not say it is editable, and an editable one with nowhere to
/// write.
const TEXT_SKIN: &str = r#"
return {
    type = 6, name = "texts", w = 640, h = 240,
    font = { { id = 0, path = "face.ttf" } },
    text = {
        { id = "search", font = 0, size = 40, ref = SEARCH_ID, constantText = "never shown" },
        { id = "title", font = 0, size = 40, ref = TITLE_ID },
        { id = "named", font = 0, size = 40, constantText = "Shown", event = function(text) end },
        { id = "stuck", font = 0, size = 40, constantText = "Stuck", editable = true },
    },
    destination = {
        { id = "search", dst = { { x = 20, y = 180, w = 300, h = 40 } } },
        { id = "title", dst = { { x = 20, y = 120, w = 300, h = 40 } } },
        { id = "named", dst = { { x = 20, y = 60, w = 300, h = 40 } } },
        { id = "stuck", dst = { { x = 340, y = 180, w = 280, h = 40 } } },
    },
}
"#;

/// Objects of [`TEXT_SKIN`], by their place in the screen.
const SEARCH: usize = 0;
const TITLE: usize = 1;
const NAMED: usize = 2;
const STUCK: usize = 3;

struct NoAssets;

impl SkinAssets for NoAssets {
    fn image(&mut self, _path: &std::path::Path) -> Option<crate::skin_render::SkinImage> {
        None
    }
}

struct Fixture {
    root: PathBuf,
    skin: LoadedSkin,
    screen: SkinScreen,
    host: MapHost,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(tag: &str) -> Fixture {
        crate::font::use_embedded_fonts_only();
        let root = std::env::temp_dir().join(format!("rbms-render-text-input-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        std::fs::write(root.join("face.ttf"), include_bytes!("../../../../../assets/fonts/Inter-Regular.ttf")).expect("the font is writable");
        let source = TEXT_SKIN.replace("SEARCH_ID", &STRING_SEARCHWORD.to_string()).replace("TITLE_ID", &STRING_TITLE.to_string());
        let entry = root.join("texts.luaskin");
        std::fs::write(&entry, source).expect("the skin is writable");

        let mut host = MapHost::new();
        host.texts.insert(STRING_TITLE, "Song title".to_string());
        let user = SkinUserConfig::default();
        let load = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, rbms_model::Mode::BEAT_7K) };
        let skin = load_lua_skin(&entry, &LuaSkinOptions { load, budget: LuaBudget::default() }, &host).expect("the fixture loads");
        let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
        let mut text = TextContext::embedded_only();
        let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut NoAssets);
        assert_eq!(screen.warnings(), &[] as &[String], "the fixture builds whole");
        Fixture { root, skin, screen, host }
    }

    fn prepare(&self) -> crate::skin_render::PreparedFrame {
        let timers = TimerState::new();
        let runtime = self.skin.runtime().expect("a Lua skin has an interpreter");
        runtime
            .frame(&self.host, |bound| {
                self.screen.prepare(&SkinFrame { now_us: 0, timers: &timers, state: &self.host, lua: Some(bound), mouse: None, data: FrameData::default() })
            })
            .expect("the host binds")
    }

    fn press(&self, at: (f32, f32)) -> Vec<SkinAction> {
        self.screen.pointer(&self.prepare(), at, SkinPointer::Press(SkinPointerButton::Left))
    }

    fn start(&self, object: usize) -> Option<TextEntryStart> {
        let timers = TimerState::new();
        let runtime = self.skin.runtime().expect("a Lua skin has an interpreter");
        runtime
            .frame(&self.host, |bound| {
                self.screen.text_entry_start(
                    object,
                    &SkinFrame { now_us: 0, timers: &timers, state: &self.host, lua: Some(bound), mouse: None, data: FrameData::default() },
                )
            })
            .expect("the host binds")
    }
}

#[test]
fn a_press_on_an_editable_text_of_a_built_screen_asks_to_type_into_it() {
    let fixture = Fixture::new("press");
    assert_eq!(
        fixture.press((100.0, 200.0)),
        vec![SkinAction::FocusText { object: SEARCH }],
        "the search box takes the press although the document never called it editable"
    );
    assert_eq!(fixture.press((100.0, 140.0)), Vec::new(), "a title is only text");
    assert_eq!(fixture.press((100.0, 80.0)), Vec::new(), "a text with its own event is typed into only when it is marked editable");
    assert_eq!(fixture.press((400.0, 200.0)), vec![SkinAction::FocusText { object: STUCK }], "an editable text with nowhere to write still takes the press");
    assert_eq!(fixture.press((600.0, 20.0)), Vec::new());
}

#[test]
fn typing_into_a_text_starts_from_what_it_shows_and_only_where_there_is_somewhere_to_write() {
    let fixture = Fixture::new("start");
    assert_eq!(
        fixture.start(SEARCH),
        Some(TextEntryStart { writer: SkinTextWriter::Id(STRING_SEARCHWORD), shown: String::new() }),
        "the search word reads empty, whatever constant text the document wrote beside its ref"
    );
    assert_eq!(fixture.start(STUCK), None, "an editable text with no writer takes no focus");
    assert_eq!(fixture.start(TITLE), None, "a title is not editable");
    assert_eq!(fixture.start(99), None, "an object the screen does not have is not either");
    let named = fixture.start(NAMED);
    assert_eq!(named, None, "the text that names an event but is not editable takes none");
}

#[test]
fn the_bounds_of_an_editable_text_come_from_the_last_prepared_frame() {
    let fixture = Fixture::new("bounds");
    let map = fixture.screen.input_map(&fixture.prepare());
    assert!(map.text_holds(SEARCH, (20.0, 180.0)), "the corner of the search box is inside it");
    assert!(map.text_holds(SEARCH, (320.0, 220.0)), "and so is the far corner");
    assert!(!map.text_holds(SEARCH, (321.0, 200.0)), "a point past its right edge is not");
    assert!(!map.text_holds(SEARCH, (100.0, 179.0)), "nor one under it");
    assert!(map.text_holds(STUCK, (600.0, 200.0)));
    assert!(!map.text_holds(TITLE, (100.0, 140.0)), "a title is no editable text");
    assert!(!map.text_holds(99, (100.0, 140.0)));
}
