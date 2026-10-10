//! Unit tests for the parts of the skin renderer that stand apart from any one screen: the
//! coordinate change, the cell animation, the digit layouts, the play screen's timers, and how each
//! of the basic objects -- image, reference image, number, slider, graph -- reaches the screen.
//!
//! The basic objects are drawn onto a [`CpuCanvas`] through the dispatch a real frame goes through,
//! from textures of a few flat colours, so what is asserted is the pixel a screen would have shown.

use rbms_skin::dst::{DestinationTrack, Keyframe, LuaDrawEval, LuaFnId, SkinColor, SkinRect, TimerRef};
use rbms_skin::loader::StretchKind;
use rbms_skin::model::PropertyRef;
use rbms_skin::property::generated::{BUTTON_LNMODE, FLOAT_HISPEED, NUMBER_PLAYLEVEL, RATE_MUSICSELECT_POSITION};
use rbms_skin::property::{DefaultState, MapHost};
use rbms_skin::timer::{MICROS_PER_MILLI, TIMER_OFF, TimerId, TimerState};

use super::draw::{ImageSelect, Placement, draw_object};
use super::object::{
    Body, DigitLayout, FloatBody, GraphBody, ImageBody, NumberBody, SkinObject, SliderBody, Sprite, ValueSource, fraction_glyphs, fraction_sign,
    integer_glyphs, integer_padding,
};
use super::refs::{ReferenceImage, ReferenceImages, build_reference};
use super::text::TEXT_PIXELS_PER_SCALE;
use super::{FrameData, NoExpressions, SkinFrame, SkinViewport};
use crate::ctx::with_render_ctx;
use crate::{BlendMode, Color, CpuCanvas, Rect, Renderer, TextureFilter, TextureId, UvRect};

/// A sprite over a texture of `size`, cut into `columns` x `rows` cells.
fn sprite(size: (u32, u32), columns: u32, rows: u32, timer: Option<TimerRef>, cycle: i32) -> Sprite {
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
    assert_eq!(sprite((32, 8), 4, 1, None, 0).animation_index(4, 5_000 * MICROS_PER_MILLI, &timers, None), 0, "no cycle means no animation");
    assert_eq!(
        sprite((32, 8), 4, 1, Some(TimerRef::Id(TimerId(1))), 400).animation_index(4, 5_000 * MICROS_PER_MILLI, &timers, None),
        0,
        "a timer that is off holds the first cell"
    );
}

#[test]
fn an_animation_steps_through_its_cells_and_wraps() {
    let mut timers = TimerState::new();
    timers.set_on(TimerId(1), 1_000 * MICROS_PER_MILLI);
    let sprite = sprite((32, 8), 4, 1, Some(TimerRef::Id(TimerId(1))), 400);

    assert_eq!(sprite.animation_index(4, 1_000 * MICROS_PER_MILLI, &timers, None), 0, "the moment the timer starts is the first cell");
    assert_eq!(sprite.animation_index(4, 1_100 * MICROS_PER_MILLI, &timers, None), 1);
    assert_eq!(sprite.animation_index(4, 1_300 * MICROS_PER_MILLI, &timers, None), 3);
    assert_eq!(sprite.animation_index(4, 1_400 * MICROS_PER_MILLI, &timers, None), 0, "one whole cycle is back to the start");
    assert_eq!(sprite.animation_index(4, 900 * MICROS_PER_MILLI, &timers, None), 0, "a moment before the timer started is the first cell too");
}

#[test]
fn an_animation_truncates_the_clock_and_its_timer_to_milliseconds_separately() {
    let mut timers = TimerState::new();
    let sprite = sprite((32, 8), 4, 1, Some(TimerRef::Id(TimerId(1))), 4);

    timers.set_on(TimerId(1), 1_999);
    assert_eq!(sprite.animation_index(4, 2_000, &timers, None), 1, "millisecond 2 less millisecond 1 is one whole millisecond, a cell of a 4 ms cycle");

    timers.set_on(TimerId(1), 1_000);
    assert_eq!(sprite.animation_index(4, 1_999, &timers, None), 0, "999 us into the same millisecond is no time at all");
}

/// The function handle the scripted evaluator answers for; every other handle gets the fallback.
const SCRIPTED_FUNCTION: LuaFnId = LuaFnId(6);

/// The property name the scripted evaluator answers for.
const SCRIPTED_NAME: &str = "playtime";

/// An evaluator that knows one function and one name, standing in for a loaded Lua skin.
struct Scripted {
    integer: i32,
    float: f32,
    text: &'static str,
    /// The microsecond the scripted timer function reports.
    started_us: i64,
}

impl LuaDrawEval for Scripted {
    fn call_integer(&self, function: LuaFnId) -> i32 {
        if function == SCRIPTED_FUNCTION { self.integer } else { 0 }
    }

    fn call_float(&self, function: LuaFnId) -> f32 {
        if function == SCRIPTED_FUNCTION { self.float } else { 0.0 }
    }

    fn call_text(&self, function: LuaFnId) -> String {
        if function == SCRIPTED_FUNCTION { self.text.to_owned() } else { String::new() }
    }

    fn call_timer(&self, function: LuaFnId) -> i64 {
        if function == SCRIPTED_FUNCTION { self.started_us } else { TIMER_OFF }
    }

    fn named_integer(&self, name: &str) -> i32 {
        if name == SCRIPTED_NAME { self.integer } else { 0 }
    }

    fn named_float(&self, name: &str) -> f32 {
        if name == SCRIPTED_NAME { self.float } else { 0.0 }
    }

    fn named_text(&self, name: &str) -> String {
        if name == SCRIPTED_NAME { self.text.to_owned() } else { String::new() }
    }
}

#[test]
fn a_function_value_is_read_through_the_evaluator() {
    let lua = Scripted { integer: 573, float: 0.25, text: "ALBIDA", started_us: 0 };
    let source = ValueSource::Function(SCRIPTED_FUNCTION);
    assert!(source.is_named());
    assert_eq!(source.integer(&DefaultState, Some(&lua)), 573);
    assert_eq!(source.float(&DefaultState, Some(&lua)), 0.25);
    assert_eq!(source.text(&DefaultState, Some(&lua)), "ALBIDA");
}

#[test]
fn a_named_value_is_read_through_the_evaluator() {
    let lua = Scripted { integer: 573, float: 0.25, text: "ALBIDA", started_us: 0 };
    let source = ValueSource::Name(SCRIPTED_NAME.to_owned());
    assert!(source.is_named());
    assert_eq!(source.integer(&DefaultState, Some(&lua)), 573);
    assert_eq!(source.float(&DefaultState, Some(&lua)), 0.25);
    assert_eq!(source.text(&DefaultState, Some(&lua)), "ALBIDA");
    assert_eq!(ValueSource::Name("nothing".to_owned()).integer(&DefaultState, Some(&lua)), 0, "a name nothing answers to is the fallback");
}

#[test]
fn a_function_or_named_value_with_nothing_to_evaluate_it_reads_as_the_fallback() {
    for source in [ValueSource::Function(SCRIPTED_FUNCTION), ValueSource::Name(SCRIPTED_NAME.to_owned())] {
        for lua in [None, Some(&NoExpressions as &dyn LuaDrawEval)] {
            assert_eq!(source.integer(&DefaultState, lua), 0, "{source:?}");
            assert_eq!(source.float(&DefaultState, lua), 0.0, "{source:?}");
            assert_eq!(source.text(&DefaultState, lua), "", "{source:?}");
        }
    }
}

#[test]
fn a_function_value_reaches_its_reader_exactly_as_the_script_gave_it() {
    let lua = Scripted { integer: 0, float: f32::NAN, text: "", started_us: 0 };
    assert!(
        ValueSource::Function(SCRIPTED_FUNCTION).float(&DefaultState, Some(&lua)).is_nan(),
        "a script's NaN is not tidied on the way: a fractional number is hidden by it and a slider rests on it, each by its own rule"
    );
}

#[test]
fn a_cell_animation_follows_a_timer_the_skin_computes() {
    let timers = TimerState::new();
    let sprite = sprite((32, 8), 4, 1, Some(TimerRef::Lua(SCRIPTED_FUNCTION)), 400);

    let running = Scripted { integer: 0, float: 0.0, text: "", started_us: 1_000 * MICROS_PER_MILLI };
    let lua: Option<&dyn LuaDrawEval> = Some(&running);
    assert_eq!(sprite.animation_index(4, 1_000 * MICROS_PER_MILLI, &timers, lua), 0);
    assert_eq!(sprite.animation_index(4, 1_100 * MICROS_PER_MILLI, &timers, lua), 1, "elapsed is measured from what the function answered");
    assert_eq!(sprite.animation_index(4, 1_300 * MICROS_PER_MILLI, &timers, lua), 3);

    let stopped = Scripted { integer: 0, float: 0.0, text: "", started_us: TIMER_OFF };
    assert_eq!(sprite.animation_index(4, 1_300 * MICROS_PER_MILLI, &timers, Some(&stopped)), 0, "a function that answers off holds the first cell");
    assert_eq!(sprite.animation_index(4, 1_300 * MICROS_PER_MILLI, &timers, Some(&NoExpressions)), 0, "so does an evaluator that cannot call it");
    assert_eq!(sprite.animation_index(4, 1_300 * MICROS_PER_MILLI, &timers, None), 0, "and so does having no evaluator");
}

/// How a frame evaluates what a Lua skin wrote as functions, asked of a skin's own interpreter
/// rather than of a stand-in: every fixture here is a small `.luaskin` written out, loaded and
/// built into a screen, so a function is called exactly as a frame of the application calls it.
mod lua_functions {
    use std::path::{Path, PathBuf};

    use rbms_skin::loader::lua_skin::{LuaSkinOptions, load_lua_skin};
    use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig};
    use rbms_skin::lua::{FrameBudget, LuaBudget, LuaFnKind};
    use rbms_skin::property::generated::NUMBER_PLAYLEVEL;
    use rbms_skin::property::{HostCall, MapHost};
    use rbms_skin::timer::TimerState;

    use super::super::draw::{float_value, number_value, share};
    use super::super::frame::PreparedFrame;
    use super::super::object::Body;
    use super::super::{FrameData, SkinAssets, SkinFrame, SkinImage, SkinScreen};
    use super::glyph;
    use crate::ctx::RenderCtx;
    use crate::font::TextContext;
    use crate::{BYTES_PER_PIXEL, Color, CpuCanvas, Renderer};

    /// The size every fixture is authored at and drawn on, so a document pixel is a screen pixel.
    const SCREEN: (u32, u32) = (1280, 720);

    /// The file stem of the fixtures' digit strip. Every other source is a plain white panel.
    const STRIP_STEM: &str = "strip";

    /// Cells in the digit strip, and the size of one of them in pixels.
    const STRIP_CELLS: u32 = 10;
    const STRIP_CELL: (u32, u32) = (4, 8);

    /// The size of the white panel, two sixteen-pixel cells side by side.
    const PANEL: (u32, u32) = (32, 16);

    /// The option a fixture's gated functions answer with.
    const GATE_OPTION: i32 = 900;

    /// Instructions one call may execute in the runaway fixture: far more than a function that ends
    /// needs, and few enough that one that does not end is cut off at once.
    const FEW_CALL_INSTRUCTIONS: u64 = 50_000;

    /// A screen pixel inside the one digit the fixtures draw at the document's bottom left corner.
    const DIGIT_PIXEL: (u32, u32) = (1, SCREEN.1 - 4);

    /// A screen row that crosses the sixteen-pixel tile the fixtures stand on the document's foot.
    const TILE_ROW: u32 = SCREEN.1 - 8;

    /// The first screen column the tile is looked for from, clear of the digit beside it.
    const TILE_SEARCH_FROM: u32 = 50;

    /// How long before the frame the runaway fixture's timer reports it switched on, in microseconds.
    const TILE_ELAPSED_US: i64 = 250_000;

    /// What every fixture starts with: `signed(id, answer)` makes a function that tells the host it
    /// ran, by running event `id`, and then answers; `gated(id)` does the same and answers whether
    /// [`GATE_OPTION`] is on; `failing(id, message)` tells the host and then raises.
    const PRELUDE: &str = r#"
local main_state = require("main_state")
local function signed(id, answer)
    return function()
        main_state.event_exec(id)
        return answer
    end
end
local function gated(id)
    return function()
        main_state.event_exec(id)
        return main_state.option(GATE_OPTION)
    end
end
local function failing(id, message)
    return function()
        main_state.event_exec(id)
        error(message)
    end
end
local corner = { { x = 0, y = 0, w = 4, h = 8 } }
local at = { { x = 100, y = 0, w = 16, h = 16 } }
local sources = { { id = 0, path = "panel.tex" }, { id = 1, path = "strip.tex" } }
local function digits(id, value)
    return { id = id, src = 1, x = 0, y = 0, w = 40, h = 8, divx = 10, digit = 1, value = value }
end
"#;

    /// One object of every kind that reads a value, each from a function that returns something
    /// other than the type the object wants.
    const VALUES_SKIN: &str = r#"
local pair = { "first", "second" }
return {
    type = 5, name = "values", w = 1280, h = 720, source = sources,
    image = { { id = "first", src = 0, x = 0, y = 0, w = 16, h = 16 }, { id = "second", src = 0, x = 16, y = 0, w = 16, h = 16 } },
    imageset = {
        { id = "cut", images = pair, value = function() return 1.9 end },
        { id = "below", images = pair, value = function() return -1 end },
        { id = "past", images = pair, value = function() return "7" end },
        { id = "silent", images = pair, value = function() end },
    },
    value = {
        digits("whole", function() return 7.9 end),
        digits("spelt", function() return "42" end),
        digits("nothing", function() end),
        digits("flag", function() return true end),
    },
    floatvalue = {
        { id = "ratio", src = 1, x = 0, y = 0, w = 40, h = 8, divx = 10, iketa = 2, fketa = 1, value = function() return "2.5" end },
        { id = "void", src = 1, x = 0, y = 0, w = 40, h = 8, divx = 10, iketa = 2, fketa = 1, value = function() end },
    },
    text = {
        { id = "quotient", font = 0, size = 12, value = function() return 10 / 4 end },
        { id = "unsaid", font = 0, size = 12, value = function() end },
    },
    slider = { { id = "knob", src = 0, x = 0, y = 0, w = 16, h = 16, range = 48, angle = 1, value = function() return "0.25" end } },
    graph = { { id = "meter", src = 0, x = 0, y = 0, w = 16, h = 16, angle = 1, value = function() return 3 end } },
    destination = {
        { id = "cut", dst = at }, { id = "below", dst = at }, { id = "past", dst = at }, { id = "silent", dst = at },
        { id = "whole", dst = at }, { id = "spelt", dst = at }, { id = "nothing", dst = at }, { id = "flag", dst = at },
        { id = "ratio", dst = at }, { id = "void", dst = at },
        { id = "quotient", dst = at }, { id = "unsaid", dst = at },
        { id = "knob", dst = at }, { id = "meter", dst = at },
    },
}
"#;

    /// Two objects and both of their sources follow one timer function, signed 1. The number reads
    /// its value from a function signed 2. A third object, gated by a function signed 3 and timed by
    /// one signed 4, shows the image that animates on the shared timer.
    const SHARED_SKIN: &str = r#"
local shared = signed(1, 0)
return {
    type = 5, name = "shared", w = 1280, h = 720, source = sources,
    image = { { id = "tile", src = 0, x = 0, y = 0, w = 32, h = 16, divx = 2, timer = shared, cycle = 100 } },
    value = { { id = "count", src = 1, x = 0, y = 0, w = 40, h = 8, divx = 10, digit = 1, timer = shared, cycle = 100, value = signed(2, 7) } },
    destination = {
        { id = "tile", timer = shared, dst = at },
        { id = "count", timer = shared, dst = corner },
        { id = "tile", draw = gated(3), timer = signed(4, 0), dst = { { x = 200, y = 0, w = 16, h = 16 } } },
    },
}
"#;

    /// How often the reference would call [`SHARED_SKIN`]'s shared timer in a frame: twice for each
    /// of the two destinations that follow it and twice for each of the three sources read through
    /// it (`TimerProperty.isOff` and then `get`).
    const SHARED_REFERENCE_CALLS: u32 = 10;

    /// How often a frame reads that timer, which is once where the reference reads it twice.
    const SHARED_READS: u32 = SHARED_REFERENCE_CALLS / 2;

    /// A number, a text, a condition and a timer whose functions raise, signed 1 to 4.
    const FAILING_SKIN: &str = r#"
return {
    type = 5, name = "failing", w = 1280, h = 720, source = sources,
    image = { { id = "tile", src = 0, x = 0, y = 0, w = 16, h = 16 } },
    value = { digits("count", failing(1, "no count")) },
    text = { { id = "label", font = 0, size = 12, value = failing(2, "no label") } },
    destination = {
        { id = "count", dst = corner },
        { id = "label", dst = at },
        { id = "tile", draw = failing(3, "no gate"), dst = at },
        { id = "tile", timer = failing(4, "no timer"), dst = at },
    },
}
"#;

    /// A number and a timed tile whose functions never return while [`GATE_OPTION`] is on.
    const RUNAWAY_SKIN: &str = r#"
local function stuck()
    while main_state.option(GATE_OPTION) do end
end
return {
    type = 5, name = "runaway", w = 1280, h = 720, source = sources,
    image = { { id = "tile", src = 0, x = 0, y = 0, w = 16, h = 16 } },
    value = { digits("count", function() stuck() return main_state.number(LEVEL_NUMBER) end) },
    destination = {
        { id = "count", dst = corner },
        { id = "tile", timer = function() stuck() return main_state.time() - TILE_ELAPSED_US end,
          dst = { { time = 0, x = 100, y = 0, w = 16, h = 16 }, { time = 1000, x = 200 } } },
    },
}
"#;

    /// Stand-in pictures for the fixtures' two sources: a strip of ten digit cells, each the colour
    /// [`glyph`] gives its index, and a white panel.
    struct FixtureAssets;

    impl SkinAssets for FixtureAssets {
        fn image(&mut self, path: &Path) -> Option<SkinImage> {
            if path.file_stem().is_some_and(|stem| stem == STRIP_STEM) {
                let width = STRIP_CELLS * STRIP_CELL.0;
                let row = (0..width).flat_map(|column| {
                    let (r, g, b) = glyph(column / STRIP_CELL.0);
                    [r, g, b, u8::MAX]
                });
                let rgba: Vec<u8> = (0..STRIP_CELL.1).flat_map(|_| row.clone()).collect();
                return SkinImage::new(width, STRIP_CELL.1, rgba);
            }
            SkinImage::new(PANEL.0, PANEL.1, vec![u8::MAX; (PANEL.0 * PANEL.1) as usize * BYTES_PER_PIXEL])
        }
    }

    /// A fixture loaded and built, with the canvas it draws on. Its folder is removed with it.
    struct Fixture {
        root: PathBuf,
        skin: LoadedSkin,
        screen: SkinScreen,
        canvas: CpuCanvas,
        text: TextContext,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    impl Fixture {
        /// Writes `body` out as a skin, loads it against `host` under `budget` and builds its screen.
        fn new(tag: &str, body: &str, host: &MapHost, budget: LuaBudget) -> Fixture {
            crate::font::use_embedded_fonts_only();
            let root = std::env::temp_dir().join(format!("rbms-render-lua-functions-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("the scratch folder is writable");
            for source in ["panel.tex", "strip.tex"] {
                std::fs::write(root.join(source), []).expect("the source file is writable");
            }
            let source = format!("{PRELUDE}{body}")
                .replace("GATE_OPTION", &GATE_OPTION.to_string())
                .replace("LEVEL_NUMBER", &NUMBER_PLAYLEVEL.to_string())
                .replace("TILE_ELAPSED_US", &TILE_ELAPSED_US.to_string());
            let entry = root.join("fixture.luaskin");
            std::fs::write(&entry, source).expect("the skin is writable");

            let user = SkinUserConfig::default();
            let load = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&root, &user, rbms_model::Mode::BEAT_7K) };
            let skin = load_lua_skin(&entry, &LuaSkinOptions { load, budget }, host).expect("the fixture loads");
            let mut canvas = CpuCanvas::new(SCREEN.0, SCREEN.1);
            let mut text = TextContext::embedded_only();
            let screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut FixtureAssets);
            assert_eq!(screen.warnings(), &[] as &[String], "the fixture builds whole");
            host.take_calls();
            Fixture { root, skin, screen, canvas, text }
        }

        /// The interpreter the fixture was loaded into.
        fn runtime(&self) -> &rbms_skin::lua::SkinLua {
            self.skin.runtime().expect("a Lua skin has an interpreter")
        }

        /// The first stage of one frame against `host`, inside one binding of it.
        fn prepare(&self, host: &MapHost) -> PreparedFrame {
            let timers = TimerState::new();
            self.runtime().frame(host, |bound| self.screen.prepare(&SkinFrame { lua: Some(bound), ..unbound(host, &timers) })).expect("the host binds")
        }

        /// The second stage of that frame on a cleared canvas, with nothing bound to the interpreter,
        /// answering how many objects reached the screen.
        fn draw(&mut self, host: &MapHost, prepared: &PreparedFrame) -> usize {
            let timers = TimerState::new();
            self.canvas.clear(Color::BLACK);
            let mut ctx = RenderCtx::new(crate::theme::theme(), &mut self.text);
            self.screen.draw_prepared(&mut ctx, &mut self.canvas, &unbound(host, &timers), prepared)
        }

        /// The colour the one digit at the document's corner was drawn in.
        fn digit(&self) -> (u8, u8, u8) {
            let pixel = self.canvas.pixel_at(DIGIT_PIXEL.0, DIGIT_PIXEL.1);
            (pixel.r, pixel.g, pixel.b)
        }

        /// The screen column the tile standing on the document's foot begins at.
        fn tile_left(&self) -> Option<u32> {
            (TILE_SEARCH_FROM..SCREEN.0).find(|column| self.canvas.pixel_at(*column, TILE_ROW) != Color::BLACK)
        }
    }

    /// One frame of `host` at its own clock, with no interpreter in it.
    fn unbound<'a>(host: &'a MapHost, timers: &'a TimerState) -> SkinFrame<'a> {
        SkinFrame { now_us: host.now_us, timers, state: host, lua: None, mouse: None, data: FrameData::default() }
    }

    /// The ids of the events a host was told to run since it was last asked, oldest first: which of
    /// a fixture's signed functions ran, in the order they did.
    fn signatures(host: &MapHost) -> Vec<i32> {
        host.take_calls().into_iter().filter_map(|call| if let HostCall::Event { id, .. } = call { Some(id) } else { None }).collect()
    }

    /// What a function returns is read by the rule of the kind that asked for it, whatever its Lua
    /// type, and each kind of object asks through the same evaluator: an image set, a number and a
    /// slider scaled between endpoints want a whole number (`toint`), a fractional number, a slider
    /// and a graph a number (`tofloat`), and a text a string (`tojstring`). Nothing returned is zero
    /// or the word `nil`, never the "no value" that hides a number.
    #[test]
    fn every_kind_of_object_reads_its_function_value_by_the_rule_of_its_type() {
        let host = MapHost::new();
        let fixture = Fixture::new("values", VALUES_SKIN, &host, LuaBudget::default());
        let timers = TimerState::new();
        let objects = &fixture.screen.objects;
        assert_eq!(objects.len(), 14);

        fixture
            .runtime()
            .frame(&host, |bound| {
                let frame = SkinFrame { lua: Some(bound), ..unbound(&host, &timers) };
                let slot = |index: usize| match &objects[index].body {
                    Body::Image(body) => body.select.slot(body.variants.len(), &frame),
                    other => panic!("object {index} should be an image set: {other:?}"),
                };
                assert_eq!(slot(0), Some(1), "a fraction is cut towards zero");
                assert_eq!(slot(1), None, "a negative choice hides the image");
                assert_eq!(slot(2), Some(0), "a numeral in a string is a number, and one past the last image shows the first");
                assert_eq!(slot(3), Some(0), "nothing returned is zero");

                let whole = |index: usize| match &objects[index].body {
                    Body::Number(body) => number_value(&body.value, &frame),
                    other => panic!("object {index} should be a number: {other:?}"),
                };
                assert_eq!([whole(4), whole(5), whole(6), whole(7)], [Some(7), Some(42), Some(0), Some(0)], "7.9, \"42\", nothing and true");

                let fraction = |index: usize| match &objects[index].body {
                    Body::Float(body) => float_value(body, &frame),
                    other => panic!("object {index} should be a fractional number: {other:?}"),
                };
                assert_eq!([fraction(8), fraction(9)], [Some(2.5), Some(0.0)], "\"2.5\" and nothing");

                let line = |index: usize| match &objects[index].body {
                    Body::Text(body) => body.value.text(frame.state, frame.lua).into_owned(),
                    other => panic!("object {index} should be a text: {other:?}"),
                };
                assert_eq!([line(10), line(11)], ["2.5", "nil"], "a number is spelt out, and nothing is spelt as the reference spells it");

                match (&objects[12].body, &objects[13].body) {
                    (Body::Slider(knob), Body::Graph(meter)) => {
                        assert_eq!(share(&knob.value, knob.ref_num, &frame), 0.25);
                        assert_eq!(share(&meter.value, meter.ref_num, &frame), 3.0, "a share past the whole is not held back");
                    }
                    other => panic!("the last two objects should be a slider and a graph: {other:?}"),
                }
            })
            .expect("the host binds");
        assert!(fixture.runtime().diagnostics().function_failures.is_empty(), "{:?}", fixture.runtime().diagnostics().function_failures);
    }

    /// A timer function is called once a frame however many destinations and sources read it, and
    /// the functions behind a condition that fails are not called at all.
    #[test]
    fn a_frame_calls_a_timer_function_once_for_every_object_and_source_that_reads_it() {
        let mut host = MapHost::new();
        let mut fixture = Fixture::new("shared", SHARED_SKIN, &host, LuaBudget::default());

        let gated_off = fixture.prepare(&host);
        assert_eq!(signatures(&host), [1, 2, 3], "the shared timer once, the number's value, and the gate that failed; never the timer behind it");
        assert_eq!(gated_off.visible_count(), 2);
        let cost = fixture.runtime().frame_cost();
        assert_eq!((cost.calls, cost.reused), (3, SHARED_READS - 1), "every read of the shared timer after the first was answered without a call");
        assert_eq!(fixture.draw(&host, &gated_off), 2, "both objects that follow the shared timer are drawn from its one answer");
        assert_eq!(signatures(&host), Vec::<i32>::new(), "and drawing calls nothing");

        host.booleans.insert(GATE_OPTION, true);
        let gated_on = fixture.prepare(&host);
        assert_eq!(signatures(&host), [1, 2, 3, 4], "the next frame calls each function again, the gated object's timer now among them");
        let cost = fixture.runtime().frame_cost();
        assert_eq!((cost.calls, cost.reused), (4, SHARED_READS - 1));
        assert_eq!(fixture.draw(&host, &gated_on), 3);
    }

    /// A function that raises costs its object one frame of one value, the way the reference has it:
    /// a number reads zero, a text reads empty, a condition fails and a timer is off. It is recorded
    /// and called again on the next frame.
    #[test]
    fn a_function_that_raises_reads_as_its_default_and_is_called_again_on_the_next_frame() {
        let host = MapHost::new();
        let mut fixture = Fixture::new("failing", FAILING_SKIN, &host, LuaBudget::default());
        let frames = 2;
        for _ in 0..frames {
            let prepared = fixture.prepare(&host);
            assert_eq!(signatures(&host), [1, 2, 3, 4], "every function is called, on every frame");
            assert_eq!(prepared.visible_count(), 2, "the number and the text are placed; the failed condition and the timer that is off leave theirs out");
            assert_eq!(fixture.draw(&host, &prepared), 1, "the empty text draws nothing");
            assert_eq!(fixture.digit(), glyph(0), "and the number shows a zero");
        }

        let failures = fixture.runtime().diagnostics().function_failures;
        let recorded: Vec<(LuaFnKind, u64)> = failures.iter().map(|failure| (failure.kind, failure.count)).collect();
        assert_eq!(recorded, [(LuaFnKind::Integer, frames), (LuaFnKind::Text, frames), (LuaFnKind::Boolean, frames), (LuaFnKind::Timer, frames)]);
        let messages: Vec<&str> = failures.iter().map(|failure| failure.first_message.as_str()).collect();
        for (message, expected) in messages.iter().zip(["no count", "no label", "no gate", "no timer"]) {
            assert!(message.contains(expected), "{message:?} should carry {expected:?}");
        }
    }

    /// A call the budget cuts off does not blank its object: the function reads as it read on the
    /// frame before, so a number keeps its digit and a timed object stays on its track.
    #[test]
    fn a_function_cut_off_by_the_budget_keeps_the_value_of_the_frame_before() {
        let budget = LuaBudget { frame: FrameBudget { max_call_instructions: FEW_CALL_INSTRUCTIONS, ..FrameBudget::default() }, ..LuaBudget::default() };
        let mut host = MapHost::new();
        host.now_us = 1_000_000;
        host.integers.insert(NUMBER_PLAYLEVEL, 5);
        let mut fixture = Fixture::new("runaway", RUNAWAY_SKIN, &host, budget);

        let before = fixture.prepare(&host);
        assert_eq!(fixture.draw(&host, &before), 2);
        assert_eq!((fixture.digit(), fixture.tile_left()), (glyph(5), Some(125)), "a quarter of a second along a slide of a hundred pixels a second");

        host.booleans.insert(GATE_OPTION, true);
        host.integers.insert(NUMBER_PLAYLEVEL, 9);
        host.now_us = 1_100_000;
        let cut = fixture.prepare(&host);
        assert_eq!(cut.visible_count(), 2, "neither object is left out of the frame its function ran away on");
        assert_eq!(fixture.draw(&host, &cut), 2);
        assert_eq!(
            (fixture.digit(), fixture.tile_left()),
            (glyph(5), Some(135)),
            "the number keeps its digit, and the tile moves on from the moment its timer last reported"
        );
        assert_eq!(fixture.runtime().diagnostics().frames_over_budget, 1);

        host.booleans.insert(GATE_OPTION, false);
        host.now_us = 1_200_000;
        let after = fixture.prepare(&host);
        assert_eq!(fixture.draw(&host, &after), 2);
        assert_eq!((fixture.digit(), fixture.tile_left()), (glyph(9), Some(125)), "the next frame calls both again and reads afresh");
        assert_eq!(fixture.runtime().diagnostics().frames_over_budget, 1);
    }
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

/// The canvas the basic objects are drawn on. A document authored at this size maps one to one, so
/// a document pixel is a screen pixel.
const STAGE: (u32, u32) = (64, 48);

/// A document authored at half the canvas, which every length in it is doubled from.
const HALF_STAGE: (u32, u32) = (32, 24);

/// A document authored at three times the canvas, which every length in it is cut to a third from.
const TRIPLE_STAGE: (u32, u32) = (192, 144);

/// What the canvas is cleared to, so a pixel nothing reached can be told from one drawn black.
const BACKDROP: (u8, u8, u8) = (90, 90, 90);

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 255, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);
const YELLOW: (u8, u8, u8) = (255, 255, 0);
const BLACK: (u8, u8, u8) = (0, 0, 0);
const WHITE: (u8, u8, u8) = (255, 255, 255);

/// A canvas with nothing on it yet.
fn stage() -> CpuCanvas {
    let mut canvas = CpuCanvas::new(STAGE.0, STAGE.1);
    wipe(&mut canvas);
    canvas
}

/// Takes everything off the canvas and leaves its textures registered.
fn wipe(canvas: &mut CpuCanvas) {
    canvas.clear(Color::rgb(BACKDROP.0, BACKDROP.1, BACKDROP.2));
}

/// The colour of one canvas pixel, without its alpha.
fn rgb(canvas: &CpuCanvas, x: u32, y: u32) -> (u8, u8, u8) {
    let pixel = canvas.pixel_at(x, y);
    (pixel.r, pixel.g, pixel.b)
}

/// The first column of `row` that is not the backdrop.
fn left_edge(canvas: &CpuCanvas, row: u32) -> Option<u32> {
    (0..STAGE.0).find(|column| rgb(canvas, *column, row) != BACKDROP)
}

/// The first row of `column` that is not the backdrop.
fn top_edge(canvas: &CpuCanvas, column: u32) -> Option<u32> {
    (0..STAGE.1).find(|row| rgb(canvas, column, *row) != BACKDROP)
}

/// A texture of `columns` x `rows` opaque texels, registered under `key`.
fn swatch_texture(canvas: &mut CpuCanvas, key: &str, columns: u32, rows: u32, texels: &[(u8, u8, u8)]) -> TextureId {
    let rgba: Vec<u8> = texels.iter().flat_map(|(r, g, b)| [*r, *g, *b, u8::MAX]).collect();
    canvas.register_texture(key, &rgba, columns, rows)
}

/// A one-cell sprite over a texture of `columns` x `rows` opaque texels.
fn swatch(canvas: &mut CpuCanvas, key: &str, columns: u32, rows: u32, texels: &[(u8, u8, u8)]) -> Sprite {
    let tex = swatch_texture(canvas, key, columns, rows, texels);
    Sprite { tex, size: (columns, rows), origin: (0, 0), cell: (columns, rows), columns: 1, rows: 1, timer: None, cycle: 0 }
}

/// The colour digit strip cell `index` is drawn in, so a drawn place names its glyph.
fn glyph(index: u32) -> (u8, u8, u8) {
    (10 + index as u8 * 10, 0, 0)
}

/// A strip of `cells` one-texel glyph cells, each a colour of its own.
fn glyph_strip(canvas: &mut CpuCanvas, key: &str, cells: u32) -> Sprite {
    let texels: Vec<(u8, u8, u8)> = (0..cells).map(glyph).collect();
    let tex = swatch_texture(canvas, key, cells, 1, &texels);
    Sprite { tex, size: (cells, 1), origin: (0, 0), cell: (1, 1), columns: cells, rows: 1, timer: None, cycle: 0 }
}

/// A destination that holds one rectangle still, in one colour.
fn held(rect: SkinRect, color: SkinColor) -> DestinationTrack {
    DestinationTrack { frames: vec![Keyframe { time_ms: 0, rect, clip: None, color, angle_deg: 0.0 }], ..DestinationTrack::default() }
}

/// One draw-list entry over a body and the rectangle its destination holds, opaque and untinted.
fn object_at(rect: SkinRect, body: Body) -> SkinObject {
    SkinObject { track: held(rect, SkinColor::rgba(u8::MAX, u8::MAX, u8::MAX, u8::MAX)), stretch: StretchKind::Stretch, body }
}

/// The object a negative destination id names.
fn reference_at(rect: SkinRect, id: &str) -> SkinObject {
    object_at(rect, build_reference(id).expect("a negative whole number names a reference image"))
}

/// Draws one object of a document authored at `authored` on a canvas that already holds its
/// textures, answering whether anything reached the screen.
fn paint(canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost, authored: (u32, u32), images: ReferenceImages) -> bool {
    let timers = TimerState::new();
    let data = FrameData { images, ..FrameData::default() };
    let frame = SkinFrame { now_us: host.now_us, timers: &timers, state: host, lua: None, mouse: None, data };
    let viewport = SkinViewport::new((authored.0 as f32, authored.1 as f32), (STAGE.0 as f32, STAGE.1 as f32));
    with_render_ctx(|ctx| draw_object(ctx, canvas, object, &viewport, &frame))
}

/// Draws one object of a document authored at the canvas's own size with no reference image.
fn paint_plain(canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost) -> bool {
    paint(canvas, object, host, STAGE, ReferenceImages::default())
}

/// Whether an object is left standing by the prepare stage against `host`.
fn prepared(object: &SkinObject, host: &MapHost) -> bool {
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: host.now_us, timers: &timers, state: host, lua: None, mouse: None, data: FrameData::default() };
    object.prepare(&frame).is_some()
}

#[test]
fn the_black_and_the_white_pixel_are_the_renderers_own() {
    let mut canvas = stage();
    let host = MapHost::new();
    let rect = SkinRect::new(8.0, 8.0, 16.0, 8.0);

    assert!(paint_plain(&mut canvas, &reference_at(rect, "-110"), &host), "a frame that carries no image at all still draws the black pixel");
    assert_eq!(rgb(&canvas, 10, 34), BLACK, "the rectangle is filled with it, eight rows up from the document's foot");
    assert_eq!(rgb(&canvas, 23, 39), BLACK);
    assert_eq!(rgb(&canvas, 24, 34), BACKDROP, "and nothing past its right edge is touched");
    assert_eq!(rgb(&canvas, 10, 31), BACKDROP, "nor above its top");

    assert!(paint_plain(&mut canvas, &reference_at(rect, "-111"), &host));
    assert_eq!(rgb(&canvas, 10, 34), WHITE, "the white pixel is the other one");

    let tinted = SkinObject { track: held(rect, SkinColor::rgba(u8::MAX, 0, 0, u8::MAX)), ..reference_at(rect, "-111") };
    assert!(paint_plain(&mut canvas, &tinted, &host));
    assert_eq!(rgb(&canvas, 10, 34), RED, "which is what a document fills a rectangle in a colour of its own with");
}

#[test]
fn a_charts_picture_comes_from_the_frame_and_is_left_out_without_one() {
    let host = MapHost::new();
    let rect = SkinRect::new(8.0, 8.0, 16.0, 16.0);
    let mut canvas = stage();
    let picture = swatch_texture(&mut canvas, "picture", 2, 2, &[RED, BLUE, GREEN, YELLOW]);

    let carried = [
        ("-100", ReferenceImages { stagefile: Some(picture), ..ReferenceImages::default() }),
        ("-101", ReferenceImages { backbmp: Some(picture), ..ReferenceImages::default() }),
        ("-102", ReferenceImages { banner: Some(picture), ..ReferenceImages::default() }),
    ];
    for (id, images) in carried {
        let object = reference_at(rect, id);
        let mut canvas_without = stage();
        assert!(!paint_plain(&mut canvas_without, &object, &host), "{id} has no picture on a frame that carries none");
        assert_eq!(left_edge(&canvas_without, 30), None, "so nothing is drawn for it");
        assert!(prepared(&object, &host), "though it is an object all the same, and is prepared like one");

        wipe(&mut canvas);
        assert!(paint(&mut canvas, &object, &host, STAGE, images), "{id} draws the picture its own slot of the frame carries");
        assert_eq!((rgb(&canvas, 8, 24), rgb(&canvas, 23, 24)), (RED, BLUE), "the picture's top row is the rectangle's top row");
        assert_eq!((rgb(&canvas, 8, 39), rgb(&canvas, 23, 39)), (GREEN, YELLOW), "and its bottom row the bottom");
        assert_eq!(
            (rgb(&canvas, 15, 30), rgb(&canvas, 16, 30)),
            (RED, BLUE),
            "a document that asked for no filtering gets none, however far the picture is stretched"
        );
    }

    let others = ReferenceImages { stagefile: Some(picture), ..ReferenceImages::default() };
    assert!(!paint(&mut canvas, &reference_at(rect, "-102"), &host, STAGE, others), "one slot does not answer for another");
}

#[test]
fn a_negative_id_the_reference_has_no_picture_under_is_an_object_that_never_draws() {
    let host = MapHost::new();
    let rect = SkinRect::new(8.0, 8.0, 16.0, 16.0);
    let mut canvas = stage();
    let picture = swatch_texture(&mut canvas, "picture", 1, 1, &[RED]);
    let every = ReferenceImages { stagefile: Some(picture), backbmp: Some(picture), banner: Some(picture) };

    for id in ["-1", "-103", "-109", "-112", "-2147483648"] {
        let object = reference_at(rect, id);
        assert!(prepared(&object, &host), "{id} is an object: its conditions and its timer are looked at every frame");
        assert!(!paint(&mut canvas, &object, &host, STAGE, every), "{id} names no picture, so it is never drawn");
    }
    assert_eq!(left_edge(&canvas, 30), None);

    for id in ["0", "110", "bg", "-1.5", " -110", ""] {
        assert!(build_reference(id).is_none(), "{id:?} is not a negative whole number and names an object of the document's own");
    }
    assert_eq!(ReferenceImage::of_destination("-110"), Some(ReferenceImage::Black));
    assert_eq!(ReferenceImage::of_destination("-103"), None);
}

#[test]
fn a_reference_image_is_fitted_by_its_stretch_mode() {
    let host = MapHost::new();
    let rect = SkinRect::new(8.0, 8.0, 16.0, 16.0);
    let mut canvas = stage();
    let wide = swatch_texture(&mut canvas, "wide", 4, 2, &[GREEN; 8]);
    let images = ReferenceImages { stagefile: Some(wide), ..ReferenceImages::default() };

    let fitted = SkinObject { stretch: StretchKind::FitInner, ..reference_at(rect, "-100") };
    assert!(paint(&mut canvas, &fitted, &host, STAGE, images));
    assert_eq!(top_edge(&canvas, 10), Some(28), "a picture twice as wide as it is tall keeps that shape, centred in the square it was given");
    assert_eq!(rgb(&canvas, 10, 35), GREEN);
    assert_eq!(rgb(&canvas, 10, 36), BACKDROP, "and leaves the rest of the square alone");

    wipe(&mut canvas);
    let unresized = SkinObject { stretch: StretchKind::NoResize, ..reference_at(rect, "-110") };
    assert!(paint(&mut canvas, &unresized, &host, STAGE, images));
    assert_eq!((left_edge(&canvas, 31), top_edge(&canvas, 15)), (Some(15), Some(31)), "the black pixel drawn at its own size is one pixel, in the middle");
    assert_eq!(rgb(&canvas, 16, 31), BACKDROP);
}

#[test]
fn a_negative_width_mirrors_an_image_back_from_its_corner() {
    let host = MapHost::new();
    let mut canvas = stage();
    let sprite = swatch(&mut canvas, "pair", 2, 1, &[RED, BLUE]);
    let image = |rect: SkinRect| object_at(rect, Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First }));

    assert!(paint_plain(&mut canvas, &image(SkinRect::new(24.0, 8.0, 16.0, 8.0)), &host));
    assert_eq!((rgb(&canvas, 24, 36), rgb(&canvas, 39, 36)), (RED, BLUE), "the right way round, red is on the left");

    wipe(&mut canvas);
    assert!(paint_plain(&mut canvas, &image(SkinRect::new(40.0, 8.0, -16.0, 8.0)), &host), "a negative width is drawn, not dropped");
    assert_eq!((rgb(&canvas, 24, 36), rgb(&canvas, 31, 36)), (BLUE, BLUE), "it reaches back from x, so the far half is what was the right half");
    assert_eq!((rgb(&canvas, 32, 36), rgb(&canvas, 39, 36)), (RED, RED), "and the picture's left edge sits on x itself");
    assert_eq!((rgb(&canvas, 23, 36), rgb(&canvas, 40, 36)), (BACKDROP, BACKDROP), "over exactly the columns between x + w and x");
}

#[test]
fn a_negative_height_mirrors_an_image_down_from_its_corner() {
    let host = MapHost::new();
    let mut canvas = stage();
    let sprite = swatch(&mut canvas, "stack", 1, 2, &[RED, BLUE]);
    let image = |rect: SkinRect| object_at(rect, Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First }));

    assert!(paint_plain(&mut canvas, &image(SkinRect::new(8.0, 14.0, 8.0, 16.0)), &host));
    assert_eq!((rgb(&canvas, 10, 18), rgb(&canvas, 10, 33)), (RED, BLUE), "the right way up, the picture's top row is on top");

    wipe(&mut canvas);
    assert!(paint_plain(&mut canvas, &image(SkinRect::new(8.0, 30.0, 8.0, -16.0)), &host), "a negative height is drawn, not dropped");
    assert_eq!((rgb(&canvas, 10, 18), rgb(&canvas, 10, 25)), (BLUE, BLUE), "it hangs down from y, with the picture's foot on y itself");
    assert_eq!((rgb(&canvas, 10, 26), rgb(&canvas, 10, 33)), (RED, RED), "and its top row at the bottom");
    assert_eq!((rgb(&canvas, 10, 17), rgb(&canvas, 10, 34)), (BACKDROP, BACKDROP), "over exactly the rows between y + h and y");
}

#[test]
fn a_mirrored_quad_keeps_its_anchor_on_the_same_screen_point() {
    let viewport = SkinViewport::new((64.0, 48.0), (64.0, 48.0));
    let anchored = |center: i32| {
        let mut object = reference_at(SkinRect::new(0.0, 0.0, 1.0, 1.0), "-110");
        object.track.center = center;
        object
    };
    let quad = |object: &SkinObject| {
        let place = Placement { object, blend: BlendMode::Alpha, tint: Color::rgb(u8::MAX, u8::MAX, u8::MAX), angle_deg: 30.0, viewport: &viewport };
        place.quad(Rect { x: 40.0, y: 20.0, w: -16.0, h: 8.0 }, UvRect::new(0.25, 0.0, 0.75, 1.0), TextureFilter::Nearest)
    };

    let corner = anchored(1);
    let turned = quad(&corner);
    assert_eq!((turned.dst.x, turned.dst.y, turned.dst.w, turned.dst.h), (24.0, 20.0, 16.0, 8.0), "the backend is handed the rectangle the right way round");
    assert_eq!((turned.src.u0, turned.src.u1), (0.75, 0.25), "reading its texture backwards across");
    assert_eq!((turned.src.v0, turned.src.v1), (0.0, 1.0), "and forwards down, since only the width was negative");
    assert_eq!(turned.center, (16.0, 8.0), "an anchor on the object's own corner stays on x, which is now the rectangle's right edge, at its foot");

    let far = anchored(3);
    assert_eq!(quad(&far).center, (0.0, 8.0), "the opposite anchor is a whole negative width from x, which is the rectangle's left edge");

    let middle = anchored(0);
    assert_eq!(quad(&middle).center, (8.0, 4.0), "and the middle is the middle either way");
}

#[test]
fn an_image_set_is_picked_in_the_image_index_space() {
    let mut canvas = stage();
    let red = swatch(&mut canvas, "red", 1, 1, &[RED]);
    let blue = swatch(&mut canvas, "blue", 1, 1, &[BLUE]);
    let rect = SkinRect::new(8.0, 8.0, 8.0, 8.0);
    let image = object_at(rect, Body::Image(ImageBody { variants: vec![Some((red, 0, 1)), Some((blue, 0, 1))], select: ImageSelect::of_index(BUTTON_LNMODE) }));
    let mut shown = |index: Option<i32>| {
        let mut host = MapHost::new();
        host.integers.insert(BUTTON_LNMODE, 1);
        if let Some(index) = index {
            host.image_indices.insert(BUTTON_LNMODE, index);
        }
        wipe(&mut canvas);
        let drawn = paint_plain(&mut canvas, &image, &host);
        (drawn, prepared(&image, &host), rgb(&canvas, 10, 34))
    };

    assert_eq!(shown(Some(0)), (true, true, RED), "index zero is the first image, whatever the number under the same id says");
    assert_eq!(shown(Some(1)), (true, true, BLUE));
    assert_eq!(shown(Some(2)), (true, true, RED), "an index past the last image shows the first");
    assert_eq!(shown(Some(-1)), (false, false, BACKDROP), "a negative index hides the image before anything else about it is looked at");
    assert_eq!(shown(None), (false, false, BACKDROP), "and an index the host has nothing under is a negative one");
}

#[test]
fn a_ref_the_reference_has_no_index_under_always_shows_the_first_set() {
    assert!(matches!(ImageSelect::of_index(0), ImageSelect::First), "a `ref` that was left out selects nothing");
    assert!(matches!(ImageSelect::of_index(NUMBER_PLAYLEVEL), ImageSelect::First), "nor does an id that is only a number");
    assert!(matches!(ImageSelect::of_index(-1), ImageSelect::First));
    assert!(matches!(ImageSelect::of_index(BUTTON_LNMODE), ImageSelect::Index(BUTTON_LNMODE)));

    let mut canvas = stage();
    let red = swatch(&mut canvas, "red", 1, 1, &[RED]);
    let blue = swatch(&mut canvas, "blue", 1, 1, &[BLUE]);
    let variants = vec![Some((red, 0, 1)), Some((blue, 0, 1))];
    let image = object_at(SkinRect::new(8.0, 8.0, 8.0, 8.0), Body::Image(ImageBody { variants, select: ImageSelect::of_index(NUMBER_PLAYLEVEL) }));
    let mut host = MapHost::new();
    host.integers.insert(NUMBER_PLAYLEVEL, 1);
    host.image_indices.insert(NUMBER_PLAYLEVEL, 1);
    assert!(paint_plain(&mut canvas, &image, &host));
    assert_eq!(rgb(&canvas, 10, 34), RED, "nothing the host says under such an id moves the image off its first set");
}

#[test]
fn an_image_set_keeps_the_place_of_an_image_it_could_not_load() {
    let mut canvas = stage();
    let blue = swatch(&mut canvas, "blue", 1, 1, &[BLUE]);
    let image = object_at(
        SkinRect::new(8.0, 8.0, 8.0, 8.0),
        Body::Image(ImageBody { variants: vec![None, Some((blue, 0, 1))], select: ImageSelect::of_index(BUTTON_LNMODE) }),
    );
    let mut shown = |index: i32| {
        let mut host = MapHost::new();
        host.image_indices.insert(BUTTON_LNMODE, index);
        wipe(&mut canvas);
        (paint_plain(&mut canvas, &image, &host), rgb(&canvas, 10, 34))
    };

    assert_eq!(shown(1), (true, BLUE), "the image after the missing one is still the second");
    assert_eq!(shown(0), (false, BACKDROP), "the missing one draws nothing rather than borrowing its neighbour");
    assert_eq!(shown(2), (false, BACKDROP), "and an index past the end falls back to the first, which is the missing one");
}

#[test]
fn an_image_sets_value_is_a_number_and_its_ref_an_image_index() {
    let by_ref = ImageSelect::of_set(None, BUTTON_LNMODE);
    assert!(matches!(by_ref, ImageSelect::Index(BUTTON_LNMODE)), "with no `value`, the set reads its `ref`");

    let by_value = ImageSelect::of_set(Some(&PropertyRef::Id(NUMBER_PLAYLEVEL)), BUTTON_LNMODE);
    assert!(matches!(by_value, ImageSelect::Value(ValueSource::Id(NUMBER_PLAYLEVEL))), "a `value` written as an id is looked up as a number");

    let unknown = ImageSelect::of_set(Some(&PropertyRef::Id(-7)), BUTTON_LNMODE);
    assert!(matches!(unknown, ImageSelect::Index(BUTTON_LNMODE)), "one the reference has no number under is no `value` at all, so the `ref` stands");

    let scripted = ImageSelect::of_set(Some(&PropertyRef::Func(SCRIPTED_FUNCTION)), BUTTON_LNMODE);
    assert!(matches!(scripted, ImageSelect::Value(ValueSource::Function(SCRIPTED_FUNCTION))), "and a function is asked every frame");

    let mut host = MapHost::new();
    host.integers.insert(NUMBER_PLAYLEVEL, 1);
    host.image_indices.insert(NUMBER_PLAYLEVEL, 0);
    let timers = TimerState::new();
    let frame = SkinFrame { now_us: 0, timers: &timers, state: &host, lua: None, mouse: None, data: FrameData::default() };
    assert_eq!(by_value.slot(2, &frame), Some(1), "the number, not the image index under the same id, picks the image");
}

#[test]
fn an_image_cut_into_sets_animates_inside_the_set_it_shows() {
    let mut canvas = stage();
    let strip = Sprite { cell: (1, 1), columns: 4, cycle: 200, ..swatch(&mut canvas, "cells", 4, 1, &[RED, GREEN, BLUE, YELLOW]) };
    let image = object_at(
        SkinRect::new(8.0, 8.0, 8.0, 8.0),
        Body::Image(ImageBody { variants: vec![Some((strip, 0, 2)), Some((strip, 2, 2))], select: ImageSelect::of_index(BUTTON_LNMODE) }),
    );
    let mut shown = |set: i32, now_ms: i64| {
        let mut host = MapHost::new();
        host.image_indices.insert(BUTTON_LNMODE, set);
        host.now_us = now_ms * MICROS_PER_MILLI;
        wipe(&mut canvas);
        paint_plain(&mut canvas, &image, &host);
        rgb(&canvas, 10, 34)
    };

    assert_eq!((shown(0, 0), shown(0, 100)), (RED, GREEN), "the first set steps through its own two cells over one cycle");
    assert_eq!((shown(1, 0), shown(1, 100)), (BLUE, YELLOW), "and the second through its own, never the first's");
    assert_eq!(shown(1, 200), BLUE, "a whole cycle later each is back on its first cell");
}

/// A whole number of `digits` places read from the level, drawn from a ten-cell strip.
fn level_number(canvas: &mut CpuCanvas, digits: u32, align: i32, value: ValueSource) -> NumberBody {
    NumberBody {
        sprite: glyph_strip(canvas, "digits", 10),
        layout: DigitLayout::integer(10),
        digits,
        zero_padding: 0,
        space: 0.0,
        align,
        value,
        offsets: Vec::new(),
    }
}

/// A fractional number read from the hi-speed, drawn from a twelve-cell strip.
fn speed_fraction(canvas: &mut CpuCanvas, integer_digits: i32, align: i32, value: ValueSource) -> FloatBody {
    FloatBody {
        sprite: glyph_strip(canvas, "fraction", 12),
        layout: DigitLayout::fraction(12),
        integer_digits,
        fraction_digits: 0,
        sign: false,
        zero_padding: 0,
        space: 0.0,
        align,
        gain: 1.0,
        value,
        offsets: Vec::new(),
    }
}

#[test]
fn a_number_with_no_value_draws_no_places() {
    let rect = SkinRect::new(8.0, 8.0, 8.0, 8.0);
    let mut canvas = stage();
    let read = object_at(rect, Body::Number(level_number(&mut canvas, 2, 0, ValueSource::Id(NUMBER_PLAYLEVEL))));
    let unnamed = object_at(rect, Body::Number(level_number(&mut canvas, 2, 0, ValueSource::None)));

    let mut host = MapHost::new();
    assert!(
        !prepared(&read, &host) && !paint_plain(&mut canvas, &read, &host),
        "a property with nothing to report hides the number rather than drawing a zero"
    );
    for sentinel in [i32::MIN, i32::MAX] {
        host.integers.insert(NUMBER_PLAYLEVEL, sentinel);
        assert!(!prepared(&read, &host) && !paint_plain(&mut canvas, &read, &host), "{sentinel} is the reference's way of saying so");
    }
    assert_eq!(left_edge(&canvas, 36), None);

    host.integers.insert(NUMBER_PLAYLEVEL, 0);
    assert!(!prepared(&unnamed, &host) && !paint_plain(&mut canvas, &unnamed, &host), "a number that names no property has nothing to report either");
    assert!(paint_plain(&mut canvas, &read, &host), "an actual zero is a value");
    assert_eq!((rgb(&canvas, 12, 36), rgb(&canvas, 20, 36)), (BACKDROP, glyph(0)), "and fills the last place");

    let placeless = object_at(rect, Body::Number(level_number(&mut canvas, 0, 0, ValueSource::Id(NUMBER_PLAYLEVEL))));
    assert!(!paint_plain(&mut canvas, &placeless, &host), "a number the document gave no places draws none");
}

#[test]
fn per_place_nudges_are_screen_pixels_whatever_the_document_scale() {
    let mut canvas = stage();
    let mut host = MapHost::new();
    host.integers.insert(NUMBER_PLAYLEVEL, 5);
    let mut body = level_number(&mut canvas, 1, 0, ValueSource::Id(NUMBER_PLAYLEVEL));
    body.offsets = vec![(4.0, 2.0, 2.0, 0.0)];
    let number = object_at(SkinRect::new(4.0, 4.0, 4.0, 4.0), Body::Number(body));

    assert!(paint(&mut canvas, &number, &host, HALF_STAGE, ReferenceImages::default()));
    assert_eq!(left_edge(&canvas, 36), Some(12), "the place sits at document x 4, which is screen x 8, and is then nudged four screen pixels, not eight");
    assert_eq!((rgb(&canvas, 21, 36), rgb(&canvas, 22, 36)), (glyph(5), BACKDROP), "its doubled width of eight grows by two screen pixels");
    assert_eq!(top_edge(&canvas, 14), Some(30), "and it rises two screen pixels from the row its destination put it on");
}

#[test]
fn a_whole_number_and_a_fraction_read_align_the_opposite_way() {
    let rect = SkinRect::new(8.0, 8.0, 8.0, 8.0);
    let mut host = MapHost::new();
    host.integers.insert(NUMBER_PLAYLEVEL, 7);
    host.floats.insert(FLOAT_HISPEED, 7.0);
    let drawn_at = |align: i32, fraction: bool| {
        let mut canvas = stage();
        let body = if fraction {
            Body::Float(speed_fraction(&mut canvas, 2, align, ValueSource::Id(FLOAT_HISPEED)))
        } else {
            Body::Number(level_number(&mut canvas, 2, align, ValueSource::Id(NUMBER_PLAYLEVEL)))
        };
        assert!(paint_plain(&mut canvas, &object_at(rect, body), &host));
        assert_eq!(rgb(&canvas, left_edge(&canvas, 36).expect("the digit is drawn"), 36), glyph(7));
        left_edge(&canvas, 36)
    };

    assert_eq!(drawn_at(0, false), Some(16), "a whole number's blank place leads it, so left as it falls the seven is flush right");
    assert_eq!(drawn_at(1, false), Some(8), "and align one pulls it over the blank to the left");
    assert_eq!(drawn_at(0, true), Some(8), "a fraction's blank place trails it, so left as it falls the seven is flush left");
    assert_eq!(drawn_at(1, true), Some(16), "and the same align one pushes it over the blank to the right");
    assert_eq!((drawn_at(2, false), drawn_at(2, true)), (Some(12), Some(12)), "anything else centres both");
}

#[test]
fn a_fraction_with_nothing_to_show_is_left_out() {
    let rect = SkinRect::new(8.0, 8.0, 8.0, 8.0);
    let mut canvas = stage();
    let read = object_at(rect, Body::Float(speed_fraction(&mut canvas, 2, 0, ValueSource::Id(FLOAT_HISPEED))));
    let hidden = |canvas: &mut CpuCanvas, object: &SkinObject, host: &MapHost| !prepared(object, host) && !paint_plain(canvas, object, host);

    let mut host = MapHost::new();
    assert!(hidden(&mut canvas, &read, &host), "a number the host does not carry is no value, not a zero");
    for value in [f32::NAN, f32::INFINITY, f32::MAX, f32::from_bits(1)] {
        host.floats.insert(FLOAT_HISPEED, value);
        assert!(hidden(&mut canvas, &read, &host), "{value:e} is not a number the reference shows");
    }

    host.floats.insert(FLOAT_HISPEED, 2.5);
    assert!(paint_plain(&mut canvas, &read, &host), "a hi-speed of two and a half is an ordinary value, past one though it is");
    assert_eq!(rgb(&canvas, 10, 36), glyph(2));

    let unnamed = object_at(rect, Body::Float(speed_fraction(&mut canvas, 2, 0, ValueSource::None)));
    assert!(hidden(&mut canvas, &unnamed, &host), "a fraction that names no property shows nothing, not a zero");

    let placeless = object_at(rect, Body::Float(speed_fraction(&mut canvas, 0, 0, ValueSource::Id(FLOAT_HISPEED))));
    assert!(hidden(&mut canvas, &placeless, &host), "nor does one the document gave no places");

    let mut overflowing = speed_fraction(&mut canvas, 2, 0, ValueSource::Id(FLOAT_HISPEED));
    overflowing.gain = f32::MAX;
    assert!(hidden(&mut canvas, &object_at(rect, Body::Float(overflowing)), &host), "nor one whose gain takes it off the number line");

    let mut timing = speed_fraction(&mut canvas, 2, 0, ValueSource::Id(FLOAT_HISPEED));
    timing.layout = DigitLayout::fraction(24);
    timing.sprite = glyph_strip(&mut canvas, "signed", 24);
    host.floats.insert(FLOAT_HISPEED, -3.0);
    wipe(&mut canvas);
    assert!(paint_plain(&mut canvas, &object_at(rect, Body::Float(timing)), &host), "a measurement below zero is an ordinary value too");
    assert_eq!(rgb(&canvas, 10, 36), glyph(12 + 3), "and reads its digits from the strip's negative half");
}

/// A red handle four pixels wide that slides along `direction` by `range` document pixels.
fn handle(canvas: &mut CpuCanvas, direction: i32, range: f32, ref_num: Option<(i32, i32)>) -> SliderBody {
    let value = ValueSource::Id(if ref_num.is_some() { NUMBER_PLAYLEVEL } else { RATE_MUSICSELECT_POSITION });
    SliderBody { sprite: swatch(canvas, "handle", 1, 1, &[RED]), direction, range, value, ref_num }
}

#[test]
fn a_slider_travels_as_far_as_its_share_says_and_no_share_is_no_travel() {
    let rect = SkinRect::new(20.0, 8.0, 4.0, 8.0);
    let mut canvas = stage();
    let sideways = object_at(rect, Body::Slider(handle(&mut canvas, 1, 10.0, None)));
    let back = object_at(rect, Body::Slider(handle(&mut canvas, 3, 10.0, None)));
    let upwards = object_at(rect, Body::Slider(handle(&mut canvas, 0, 10.0, None)));
    let downwards = object_at(rect, Body::Slider(handle(&mut canvas, 2, 10.0, None)));
    let mut at = |object: &SkinObject, share: Option<f32>| {
        let mut host = MapHost::new();
        if let Some(share) = share {
            host.rates.insert(RATE_MUSICSELECT_POSITION, share);
        }
        host.floats.insert(RATE_MUSICSELECT_POSITION, 0.75);
        wipe(&mut canvas);
        assert!(paint_plain(&mut canvas, object, &host), "the handle is drawn wherever it is");
        let row = (0..STAGE.0).filter_map(|column| top_edge(&canvas, column)).min().expect("the handle is on the canvas");
        (left_edge(&canvas, row).expect("the handle is on the canvas"), row)
    };

    assert_eq!(at(&sideways, Some(0.5)), (25, 32), "half of ten pixels to the right");
    assert_eq!(at(&sideways, Some(1.5)), (35, 32), "a share past one carries the handle past the end of its track");
    assert_eq!(at(&sideways, Some(-0.5)), (15, 32), "and a negative one back before its start");
    assert_eq!(at(&sideways, Some(f32::NAN)), (20, 32), "a share that is not a number is no travel at all");
    assert_eq!(at(&sideways, None), (20, 32), "and so is a rate the host does not carry, whatever the float space says under the same id");
    assert_eq!(at(&back, Some(0.5)), (15, 32), "direction three goes left");
    assert_eq!(at(&upwards, Some(0.5)), (20, 27), "direction zero goes up the screen");
    assert_eq!(at(&downwards, Some(0.5)), (20, 37), "and direction two down it");
}

#[test]
fn a_sliders_range_is_a_whole_number_of_screen_pixels() {
    let mut canvas = stage();
    let slider = object_at(SkinRect::new(24.0, 24.0, 12.0, 24.0), Body::Slider(handle(&mut canvas, 1, 5.0, None)));
    let mut host = MapHost::new();
    host.rates.insert(RATE_MUSICSELECT_POSITION, 1.0);

    assert!(paint(&mut canvas, &slider, &host, TRIPLE_STAGE, ReferenceImages::default()));
    assert_eq!(
        left_edge(&canvas, 36),
        Some(9),
        "five document pixels at a third of their size are one and two thirds on screen, which the reference keeps as one"
    );
    assert_eq!((rgb(&canvas, 12, 36), rgb(&canvas, 13, 36)), (RED, BACKDROP), "so the four-pixel handle ends on a whole pixel as well");
}

#[test]
fn a_whole_number_between_two_endpoints_is_a_share_of_the_way() {
    let rect = SkinRect::new(20.0, 8.0, 4.0, 8.0);
    let mut canvas = stage();
    let at = |endpoints: (i32, i32), level: i32, canvas: &mut CpuCanvas| {
        let slider = object_at(rect, Body::Slider(handle(canvas, 1, 10.0, Some(endpoints))));
        let mut host = MapHost::new();
        host.integers.insert(NUMBER_PLAYLEVEL, level);
        wipe(canvas);
        assert!(paint_plain(canvas, &slider, &host));
        left_edge(canvas, 36).expect("the handle is on the canvas") as i32 - rect.x as i32
    };

    assert_eq!(at((0, 10), 5, &mut canvas), 5, "halfway between the endpoints is half the range");
    assert_eq!((at((0, 10), 12, &mut canvas), at((0, 10), -3, &mut canvas)), (10, 0), "past the far end is all of it and before the near end none");
    assert_eq!(at((10, 0), 4, &mut canvas), 6, "endpoints written the other way round measure from the first all the same");
    assert_eq!((at((10, 0), -3, &mut canvas), at((10, 0), 12, &mut canvas)), (10, 0), "with their far end at the smaller number");
    assert_eq!(at((4, 4), 4, &mut canvas), 0, "two endpoints that are one number divide nothing by nothing, which is no travel");
}

/// A bar of four one-texel columns, each a colour of its own, read from the browser's position.
fn bar(canvas: &mut CpuCanvas, direction: i32) -> GraphBody {
    let sprite =
        if direction == 1 { swatch(canvas, "column", 1, 4, &[RED, GREEN, BLUE, YELLOW]) } else { swatch(canvas, "row", 4, 1, &[RED, GREEN, BLUE, YELLOW]) };
    GraphBody { sprite, direction, value: ValueSource::Id(RATE_MUSICSELECT_POSITION), ref_num: None }
}

#[test]
fn a_graph_is_not_held_to_its_destination() {
    let mut canvas = stage();
    let graph = object_at(SkinRect::new(8.0, 8.0, 16.0, 8.0), Body::Graph(bar(&mut canvas, 0)));
    let mut drawn = |share: f32, columns: [u32; 3]| {
        let mut host = MapHost::new();
        host.rates.insert(RATE_MUSICSELECT_POSITION, share);
        wipe(&mut canvas);
        let reached = paint_plain(&mut canvas, &graph, &host);
        (reached, columns.map(|column| rgb(&canvas, column, 36)))
    };

    assert_eq!(drawn(0.5, [8, 12, 16]), (true, [RED, GREEN, BACKDROP]), "half the bar is the left half of its source over the left half of its destination");
    assert_eq!(drawn(2.0, [8, 20, 24]), (true, [RED, YELLOW, YELLOW]), "a share past one is drawn, not held at one: the bar runs on past its destination");
    assert_eq!(drawn(2.0, [38, 39, 40]), (true, [YELLOW, YELLOW, BACKDROP]), "to twice its width, reading past the end of its source");
    assert_eq!(drawn(-0.5, [0, 7, 8]), (true, [RED, RED, BACKDROP]), "a negative share is drawn too, backwards from the left edge");
    for share in [0.0, f32::NAN] {
        assert_eq!(drawn(share, [0, 8, 23]), (false, [BACKDROP; 3]), "a share of {share} draws no bar");
    }
}

#[test]
fn a_graph_crops_its_source_to_whole_texels() {
    let mut canvas = stage();
    let graph = object_at(SkinRect::new(8.0, 4.0, 8.0, 40.0), Body::Graph(bar(&mut canvas, 1)));
    let mut host = MapHost::new();
    host.rates.insert(RATE_MUSICSELECT_POSITION, 0.6);

    assert!(paint_plain(&mut canvas, &graph, &host));
    assert_eq!(top_edge(&canvas, 10), Some(20), "six tenths of forty rows, standing on the destination's foot");
    assert_eq!(rgb(&canvas, 10, 43), YELLOW, "with the foot of the source at the foot of the bar");
    assert_eq!(
        (rgb(&canvas, 10, 20), rgb(&canvas, 10, 31), rgb(&canvas, 10, 32)),
        (BLUE, BLUE, YELLOW),
        "six tenths of four texels is two whole texels, not two and four tenths: the row above them never shows"
    );
}

#[test]
fn filtering_is_decided_by_the_size_on_screen() {
    let checker: Vec<(u8, u8, u8)> = (0..16).map(|index| if (index % 4 + index / 4) % 2 == 0 { BLACK } else { WHITE }).collect();
    let pure_in = |rect: SkinRect, filter: i32, span: u32| {
        let mut canvas = stage();
        let sprite = swatch(&mut canvas, "checker", 4, 4, &checker);
        let mut image = object_at(rect, Body::Image(ImageBody { variants: vec![Some((sprite, 0, 1))], select: ImageSelect::First }));
        image.track.filter = filter;
        assert!(paint(&mut canvas, &image, &MapHost::new(), HALF_STAGE, ReferenceImages::default()));
        let (left, top) = (left_edge(&canvas, 39).unwrap_or(8), 40 - span);
        (0..span).all(|row| (0..span).all(|column| [BLACK, WHITE].contains(&rgb(&canvas, left + column, top + row))))
    };

    assert!(
        pure_in(SkinRect::new(4.0, 4.0, 2.0, 2.0), 1, 4),
        "two document pixels are four on screen, which is the source's own size: drawn texel for pixel, it is not filtered even though the document asked"
    );
    assert!(
        !pure_in(SkinRect::new(4.0, 4.0, 4.0, 4.0), 1, 8),
        "four document pixels match the source in the document's space but are eight on screen, so the picture is resized and filtered"
    );
    assert!(pure_in(SkinRect::new(4.0, 4.0, 4.0, 4.0), 0, 8), "and a document that asked for no filtering gets none at any size");
}
