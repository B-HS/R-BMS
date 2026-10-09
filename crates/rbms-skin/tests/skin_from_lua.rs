//! The table converter: what a Lua skin's returned table becomes in the document model.
//!
//! Every table here is a literal written for the test. None of them is run as a skin file: the
//! converter takes a value, and where that value came from is the loader's business.

#![cfg(feature = "lua")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mlua::Value;
use rbms_skin::dst::LuaFnId;
use rbms_skin::loader::from_lua::{FromLua, skin_def_from_lua};
use rbms_skin::lua::{LuaFnKind, LuaMode, LuaPass, SkinLua, SkinLuaConfig};
use rbms_skin::model::{DestinationOption, EventRef, FloatWriterRef, PropertyRef, SKIN_TYPE_UNSET, SkinDef, StringWriterRef};
use rbms_skin::property::DefaultState;

/// The seed the interpreter's random draws are pinned to. No table here draws one.
const TEST_SEED: u64 = 42;

/// The entry file the conversions claim to come from, for their messages.
const ENTRY: &str = "sample.luaskin";

/// The key a sample table reports its own count of reference fields under. No record has a field of
/// this name, so the converter ignores it.
const MADE_KEY: &str = "made";

/// The environment variable that names an external skin pack for the optional test.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The extension of a Lua skin's entry file.
const LUA_SKIN_EXTENSION: &str = "luaskin";

/// A directory that exists, for the interpreter's root. Nothing in it is read.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

/// The names the test's stand-in name table knows, one or two for each kind that has names.
fn known_name(kind: LuaFnKind, name: &str) -> bool {
    matches!(
        (kind, name),
        (LuaFnKind::Boolean, "is_autoplay")
            | (LuaFnKind::Integer, "combo")
            | (LuaFnKind::Float, "music_progress")
            | (LuaFnKind::Text, "title")
            | (LuaFnKind::Event, "open_ir")
            | (LuaFnKind::FloatWriter, "master_volume")
            | (LuaFnKind::TextWriter, "search_word")
    )
}

/// One conversion: the document, what it warned about, and the interpreter that holds its functions.
struct Converted {
    skin: SkinDef,
    warnings: Vec<String>,
    runtime: SkinLua,
    value: Value,
}

impl Converted {
    /// What a function handle was registered as.
    fn kind(&self, function: LuaFnId) -> LuaFnKind {
        self.runtime.kind_of(function).expect("the handle should be one the interpreter handed out")
    }

    /// The kind of the function a property field holds.
    fn property_kind(&self, property: &Option<PropertyRef>) -> LuaFnKind {
        self.kind(property.as_ref().and_then(PropertyRef::function).expect("the field should hold a function"))
    }
}

/// The ids of a list of records, joined for one comparison.
fn joined<'a>(ids: impl Iterator<Item = &'a String>) -> String {
    ids.map(String::as_str).collect::<Vec<_>>().join(",")
}

/// Builds the table `source` returns and converts it.
fn convert(source: &str) -> Converted {
    let mut config = SkinLuaConfig::new(&root());
    config.seed = Some(TEST_SEED);
    let runtime = SkinLua::new(config).expect("the interpreter should build");
    let value: Value = runtime.lua().load(source).eval().expect("the sample chunk should run");
    let mut warnings = Vec::new();
    let skin = skin_def_from_lua(&value, &mut FromLua { lua: &runtime, path: Path::new(ENTRY), warnings: &mut warnings, known_name })
        .expect("a table should always convert");
    Converted { skin, warnings, runtime, value }
}

#[test]
fn a_fraction_in_an_integer_field_is_cut_toward_zero() {
    let converted = convert(
        r#"return {
            type = 6.9, w = 1920 / 7, h = "720", fadeout = -1.9, scene = true, input = "soon", close = 0 / 0,
            image = { { id = "a", x = 1.9, y = -1.9, w = 10 / 4, divx = "3", cycle = {} } },
            destination = { { id = "a", loop = -1.5, offsets = { 2.5, "7" }, dst = { { time = 0.9, x = 1.9, y = -1.9, a = 255.99 } } } },
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!((skin.skin_type, skin.w, skin.h, skin.fadeout, skin.scene, skin.input, skin.close), (6, 274, 720, -1, 0, 0, 0));

    let image = &skin.image[0];
    assert_eq!((image.x, image.y, image.w, image.divx, image.cycle), (1, -1, 2, 3, 0));
    assert_eq!((image.h, image.divy), (0, 1), "a field the table leaves out keeps the record's default");

    let destination = &skin.destination[0];
    assert_eq!(destination.loop_ms, -1);
    assert_eq!(destination.offsets, [2, 7]);
    let frame = destination.dst[0];
    assert_eq!((frame.time, frame.x, frame.y, frame.a), (Some(0), Some(1), Some(-1), Some(255)));
    assert_eq!((frame.w, frame.acc, frame.clip_x), (None, None, None), "an animation field the table leaves out stays unset");
}

#[test]
fn only_nil_and_false_are_false_in_a_boolean_field() {
    let converted = convert(
        r#"return {
            offset = { { name = "shift", id = 3, x = 0, y = false, w = "", h = {}, r = 1 } },
            text = { { id = "t", editable = 0, wrapping = false } },
            slider = { { id = "kept" }, { id = "fixed", changeable = false }, { id = "zero", changeable = 0 } },
            judge = { { id = "j", shift = 0 } },
        }"#,
    );
    let offset = &converted.skin.offset[0];
    assert_eq!((offset.x, offset.y, offset.w, offset.h, offset.r, offset.a), (true, false, true, true, true, false));
    assert_eq!(offset.id, 3);

    let text = &converted.skin.text[0];
    assert!(text.editable, "the number zero is true");
    assert!(!text.wrapping);

    let changeable: Vec<bool> = converted.skin.slider.iter().map(|slider| slider.changeable).collect();
    assert_eq!(changeable, [true, false, true]);
    assert!(converted.skin.judge[0].shift);
}

#[test]
fn a_number_in_a_string_field_is_spelled_as_the_reference_spells_it() {
    let converted = convert(
        r#"return {
            name = 12, author = false,
            source = { { id = 5, path = "a.png" }, { id = 5.5, path = 7 }, { id = 10 / 2 }, { id = 1 / 3 }, { id = -0.0 } },
            imageset = { { id = 1, images = { 1, 2.5, "x" } } },
            property = { { name = "p", def = 3 } },
            text = { { id = "t", font = 0, constantText = 1.5 } },
            destination = { { id = -110 } },
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!((skin.name.as_str(), skin.author.as_str()), ("12", "false"));

    let ids: Vec<&str> = skin.source.iter().map(|source| source.id.as_str()).collect();
    assert_eq!(ids, ["5", "5.5", "5", "0.33333334", "0"]);
    assert_eq!(skin.source[1].path, "7");
    assert_eq!(skin.source[2].path, "", "a string field the table leaves out stays empty");

    assert_eq!(skin.imageset[0].id, "1");
    assert_eq!(skin.imageset[0].images, ["1", "2.5", "x"]);
    assert_eq!(skin.property[0].def.as_deref(), Some("3"));
    assert_eq!(skin.text[0].font, "0");
    assert_eq!(skin.text[0].constant_text.as_deref(), Some("1.5"));
    assert_eq!(skin.destination[0].id, "-110");
}

#[test]
fn a_key_that_names_no_field_is_ignored() {
    let converted = convert(
        r#"return {
            type = 5, Type = 9, TYPE = 10, bogus = function() end, [1] = "stray", [true] = "stray",
            image = { { id = "a", W = 5, colour = "red", [1] = "x", [2.5] = "y", nested = { id = "no" } } },
            destination = { { id = "a", layer = 3, dst = { { x = 1, z = 9 } } } },
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!(skin.skin_type, 5);
    assert_eq!((skin.image.len(), skin.image[0].id.as_str(), skin.image[0].w), (1, "a", 0));
    assert_eq!(skin.destination[0].dst[0].x, Some(1));
    assert_eq!(converted.runtime.function_count(), 0, "a function under a key no field has is not a reference");
    assert!(converted.warnings.is_empty());
}

#[test]
fn a_list_closes_its_holes_and_takes_every_value_whatever_its_key() {
    let converted = convert(
        r#"
        local sparse = {}
        sparse[1] = { id = "a" }
        sparse[3] = { id = "c" }
        sparse[7] = { id = "g" }
        local backwards = {}
        for index = 6, 1, -1 do backwards[index] = { id = index } end
        return {
            image = sparse,
            value = { { id = "x" }, nil, { id = "z" } },
            source = backwards,
            imageset = { { id = "first" }, { id = "second" }, extra = { id = "named" } },
            graph = { only = { id = "mapped" } },
            slider = { [0] = { id = "zero" }, [1] = { id = "one" } },
            text = 5,
            font = "nope",
            judge = true,
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!(joined(skin.image.iter().map(|image| &image.id)), "a,c,g");
    assert_eq!(joined(skin.value.iter().map(|value| &value.id)), "x,z");
    assert_eq!(joined(skin.source.iter().map(|source| &source.id)), "1,2,3,4,5,6");
    assert_eq!(joined(skin.imageset.iter().map(|set| &set.id)), "first,second,named");
    assert_eq!(joined(skin.graph.iter().map(|graph| &graph.id)), "mapped", "a map where a list belongs gives its values");
    assert_eq!(joined(skin.slider.iter().map(|slider| &slider.id)), "one,zero", "sequence keys come before every other key");
    assert!(skin.text.is_empty() && skin.font.is_empty() && skin.judge.is_empty(), "anything but a table is an empty list");
}

#[test]
fn a_record_field_is_set_by_anything_and_filled_only_by_a_table() {
    let converted = convert(
        r#"return {
            note = { "not", "a", "record" },
            gauge = 3,
            bga = false,
            songlist = "list",
            image = { "bare", 7, { id = "real" } },
            destination = { { id = "a", mouseRect = 1 }, { id = "b", mouseRect = { x = 1.5, y = 2, w = 3, h = 4 } }, { id = "c" } },
        }"#,
    );
    let skin = &converted.skin;
    let note = skin.note.as_ref().expect("a key that holds anything sets the record");
    assert!(note.id.is_empty() && note.note.is_empty());
    assert_eq!(note.expansionrate, [100, 100], "a record no table filled is the default record");
    assert_eq!(skin.gauge.as_ref().map(|gauge| (gauge.parts, gauge.range, gauge.endtime)), Some((50, 3, 500)));
    assert_eq!(skin.bga.as_ref().map(|bga| bga.id.as_str()), Some(""));
    assert!(skin.songlist.as_ref().is_some_and(|list| list.graph.is_none() && list.liston.is_empty()));
    assert!(skin.practice.is_none() && skin.skinpreview.is_none() && skin.skin_select.is_none());

    let images: Vec<(&str, i32)> = skin.image.iter().map(|image| (image.id.as_str(), image.divx)).collect();
    assert_eq!(images, [("", 1), ("", 1), ("real", 1)], "a list element that is not a table is a default record");

    let rects: Vec<Option<(i32, i32, i32, i32)>> = skin.destination.iter().map(|entry| entry.mouse_rect.map(|rect| (rect.x, rect.y, rect.w, rect.h))).collect();
    assert_eq!(rects, [Some((0, 0, 0, 0)), Some((1, 2, 3, 4)), None]);
}

#[test]
fn a_value_that_is_not_a_table_is_a_document_with_no_type() {
    for source in ["return 5", "return nil", "return 'skin'", "return function() end"] {
        let converted = convert(source);
        assert_eq!(converted.skin.skin_type, SKIN_TYPE_UNSET, "{source}");
        assert_eq!((converted.skin.w, converted.skin.h, converted.skin.judgetimer), (1280, 720, 1), "{source}");
        assert!(converted.skin.destination.is_empty() && converted.skin.note.is_none(), "{source}");
        assert_eq!(converted.runtime.function_count(), 0, "{source}");
    }
}

#[test]
fn a_function_in_every_reference_field_is_registered_under_the_kind_of_its_field() {
    let converted = convert(
        r#"
        local made = 0
        local function fresh()
            made = made + 1
            local own = made
            return function() return own end
        end
        local function drawn(id) return { id = id, timer = fresh(), draw = fresh(), op = { fresh(), 7, fresh() } } end
        local skin = {
            type = 0,
            image = { { id = "image", timer = fresh(), act = fresh() } },
            imageset = { { id = "set", value = fresh(), act = fresh() } },
            value = { { id = "number", timer = fresh(), value = fresh(), offset = { { timer = fresh(), value = fresh() } } } },
            floatvalue = { { id = "float", timer = fresh(), value = fresh(), offset = { { timer = fresh(), value = fresh() } } } },
            text = { { id = "text", value = fresh(), event = fresh() } },
            slider = { { id = "slider", timer = fresh(), value = fresh(), event = fresh() } },
            graph = { { id = "graph", timer = fresh(), value = fresh() } },
            hiddenCover = { { id = "hidden", timer = fresh() } },
            liftCover = { { id = "lift", timer = fresh() } },
            customEvents = { { id = 1000, action = fresh(), condition = fresh() } },
            customTimers = { { id = 10000, timer = fresh() } },
            destination = { drawn("image") },
            note = { id = "note", group = { drawn("group") }, bpm = { drawn("bpm") }, stop = { drawn("stop") }, time = { drawn("time") } },
            judge = { { id = "judge", images = { drawn("great") }, numbers = { drawn("combo") } } },
            songlist = {
                id = "list",
                listoff = { drawn("off") }, liston = { drawn("on") }, text = { drawn("bartext") }, level = { drawn("level") },
                lamp = { drawn("lamp") }, playerlamp = { drawn("mine") }, rivallamp = { drawn("rival") }, trophy = { drawn("trophy") },
                label = { drawn("label") }, graph = drawn("bargraph"),
            },
        }
        skin.made = made
        return skin"#,
    );
    let Value::Table(table) = &converted.value else { panic!("the sample is a table") };
    let made: usize = table.raw_get(MADE_KEY).expect("the sample counts its own functions");
    assert_eq!(converted.runtime.function_count(), made, "every function the table holds in a reference field is registered");
    assert!(converted.warnings.is_empty());

    let skin = &converted.skin;
    assert_eq!(converted.property_kind(&skin.image[0].timer), LuaFnKind::Timer);
    assert!(matches!(skin.image[0].act, Some(EventRef::Lua(function)) if converted.kind(function) == LuaFnKind::Event));
    assert_eq!(converted.property_kind(&skin.imageset[0].value), LuaFnKind::Integer);
    assert!(matches!(skin.imageset[0].act, Some(EventRef::Lua(_))));
    assert_eq!(converted.property_kind(&skin.value[0].timer), LuaFnKind::Timer);
    assert_eq!(converted.property_kind(&skin.value[0].value), LuaFnKind::Integer);
    assert_eq!(converted.property_kind(&skin.value[0].offset[0].value), LuaFnKind::Integer);
    assert_eq!(converted.property_kind(&skin.floatvalue[0].value), LuaFnKind::Float);
    assert_eq!(converted.property_kind(&skin.floatvalue[0].offset[0].timer), LuaFnKind::Timer);
    assert_eq!(converted.property_kind(&skin.text[0].value), LuaFnKind::Text);
    assert!(matches!(skin.text[0].event, Some(StringWriterRef::Lua(function)) if converted.kind(function) == LuaFnKind::TextWriter));
    assert_eq!(converted.property_kind(&skin.slider[0].value), LuaFnKind::Float);
    assert!(matches!(skin.slider[0].event, Some(FloatWriterRef::Lua(function)) if converted.kind(function) == LuaFnKind::FloatWriter));
    assert_eq!(converted.property_kind(&skin.graph[0].value), LuaFnKind::Float);
    assert_eq!(converted.property_kind(&skin.hidden_cover[0].timer), LuaFnKind::Timer);
    assert_eq!(converted.property_kind(&skin.lift_cover[0].timer), LuaFnKind::Timer);
    assert!(matches!(skin.custom_events[0].action, Some(EventRef::Lua(function)) if converted.kind(function) == LuaFnKind::Event));
    assert_eq!(converted.property_kind(&skin.custom_events[0].condition), LuaFnKind::Boolean);
    assert_eq!(converted.property_kind(&skin.custom_timers[0].timer), LuaFnKind::Timer);

    let note = skin.note.as_ref().expect("the note record");
    let judge = &skin.judge[0];
    let list = skin.songlist.as_ref().expect("the song list record");
    let tracks = [
        &skin.destination[0],
        &note.group[0],
        &note.bpm[0],
        &note.stop[0],
        &note.time[0],
        &judge.images[0],
        &judge.numbers[0],
        &list.listoff[0],
        &list.liston[0],
        &list.text[0],
        &list.level[0],
        &list.lamp[0],
        &list.playerlamp[0],
        &list.rivallamp[0],
        &list.trophy[0],
        &list.label[0],
        list.graph.as_ref().expect("the bar graph"),
    ];
    for track in tracks {
        assert_eq!(converted.property_kind(&track.timer), LuaFnKind::Timer, "{}", track.id);
        assert_eq!(converted.property_kind(&track.draw), LuaFnKind::Boolean, "{}", track.id);
        assert_eq!(track.op.len(), 3, "{}", track.id);
        assert_eq!(converted.property_kind(&track.op[0].property), LuaFnKind::Boolean, "{}", track.id);
        assert_eq!(track.op[1], DestinationOption { id: 7, property: None }, "{}", track.id);
        assert_eq!(converted.property_kind(&track.op[2].property), LuaFnKind::Boolean, "{}", track.id);
    }
}

#[test]
fn a_function_shared_by_two_fields_of_one_kind_is_one_handle() {
    let converted = convert(
        r#"
        local shown = function() return true end
        return { destination = { { id = "a", draw = shown }, { id = "b", draw = shown, op = { shown } }, { id = "c", timer = shown } } }"#,
    );
    let destinations = &converted.skin.destination;
    let first = destinations[0].draw.as_ref().and_then(PropertyRef::function).expect("a function handle");
    assert_eq!(destinations[1].draw, Some(PropertyRef::Func(first)));
    assert_eq!(destinations[1].op, [DestinationOption { id: 0, property: Some(PropertyRef::Func(first)) }]);
    assert_ne!(destinations[2].timer, Some(PropertyRef::Func(first)), "the same function read as another kind is another handle");
    assert_eq!(converted.runtime.function_count(), 2);
}

#[test]
fn a_number_in_a_reference_field_is_an_id() {
    let converted = convert(
        r#"return {
            image = { { id = "a", timer = 41, act = 13 }, { id = "b", timer = -1, act = "14" }, { id = "c", timer = "42.9", act = -3.9 } },
            value = { { id = "v", value = 100.7 } },
            text = { { id = "t", value = "12", event = 17 } },
            slider = { { id = "s", value = 6, event = 17 } },
            destination = { { id = "a", draw = 40, op = { 1, -2, "3", 4.9, 0, " 0x10 " } }, { id = "b", draw = -40.9 }, { id = "c", draw = "33" } },
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!(skin.image[0].timer, Some(PropertyRef::Id(41)));
    assert_eq!(skin.image[0].act, Some(EventRef::Id(13)));
    assert_eq!(skin.image[1].timer, None, "a negative id names no timer");
    assert_eq!(skin.image[1].act, Some(EventRef::Id(14)), "a string that spells a number is that number");
    assert_eq!(skin.image[2].timer, Some(PropertyRef::Id(42)));
    assert_eq!(skin.image[2].act, Some(EventRef::Id(-3)));
    assert_eq!(skin.value[0].value, Some(PropertyRef::Id(100)));
    assert_eq!(skin.text[0].value, Some(PropertyRef::Id(12)));
    assert_eq!(skin.slider[0].value, Some(PropertyRef::Id(6)));
    assert_eq!(skin.slider[0].event, Some(FloatWriterRef::Id(17)));

    assert_eq!(skin.text[0].event, None, "a text writer has no ids");
    assert_eq!(converted.warnings.len(), 1, "the number was tried as a script, which it is not: {:?}", converted.warnings);
    assert!(converted.warnings[0].starts_with(ENTRY));

    let ids: Vec<i32> = skin.destination[0].op.iter().map(|option| option.id).collect();
    assert_eq!(ids, [1, -2, 3, 4, 0, 16]);
    assert!(skin.destination[0].op.iter().all(|option| option.property.is_none()));
    let draws: Vec<Option<PropertyRef>> = skin.destination.iter().map(|entry| entry.draw.clone()).collect();
    assert_eq!(draws, [Some(PropertyRef::Id(40)), Some(PropertyRef::Id(-40)), Some(PropertyRef::Id(33))]);
    assert_eq!(converted.runtime.function_count(), 0);
}

#[test]
fn a_string_in_a_reference_field_is_a_name_when_one_is_known_and_a_script_otherwise() {
    let converted = convert(
        r#"return {
            image = { { id = "a", act = "open_ir", timer = "is_autoplay" }, { id = "b", act = "pressed = 1", timer = "5000 + 1" } },
            value = { { id = "v", value = "combo" } },
            floatvalue = { { id = "f", value = "combo" }, { id = "g", value = "music_progress" } },
            text = { { id = "t", value = "title", event = "search_word" }, { id = "u", value = "'fixed'", event = "typed = 1" } },
            slider = { { id = "s", value = "music_progress", event = "master_volume" } },
            destination = {
                { id = "a", draw = "is_autoplay", op = { "is_autoplay", "!is_autoplay", "1 == 1" } },
                { id = "b", draw = "!!is_autoplay" },
                { id = "c", draw = "gauge > 80" },
                { id = "d", draw = "!gauge" },
            },
        }"#,
    );
    let skin = &converted.skin;
    let name = |text: &str| Some(PropertyRef::Name(text.to_owned()));

    assert_eq!(skin.image[0].act, Some(EventRef::Name("open_ir".to_owned())));
    assert_eq!(converted.property_kind(&skin.image[0].timer), LuaFnKind::Timer, "a timer has no names, so a name is a script there");
    assert!(matches!(skin.image[1].act, Some(EventRef::Lua(function)) if converted.kind(function) == LuaFnKind::Event), "an event script is a statement");
    assert_eq!(converted.property_kind(&skin.image[1].timer), LuaFnKind::Timer);

    assert_eq!(skin.value[0].value, name("combo"));
    assert_eq!(converted.property_kind(&skin.floatvalue[0].value), LuaFnKind::Float, "a name is known per kind");
    assert_eq!(skin.floatvalue[1].value, name("music_progress"));
    assert_eq!(skin.text[0].value, name("title"));
    assert_eq!(skin.text[0].event, Some(StringWriterRef::Name("search_word".to_owned())));
    assert_eq!(converted.property_kind(&skin.text[1].value), LuaFnKind::Text);
    assert!(matches!(skin.text[1].event, Some(StringWriterRef::Lua(function)) if converted.kind(function) == LuaFnKind::TextWriter));
    assert_eq!(skin.slider[0].value, name("music_progress"));
    assert_eq!(skin.slider[0].event, Some(FloatWriterRef::Name("master_volume".to_owned())));

    let first = &skin.destination[0];
    assert_eq!(first.draw, name("is_autoplay"));
    assert_eq!(first.op[0], DestinationOption { id: 0, property: name("is_autoplay") });
    assert_eq!(first.op[1], DestinationOption { id: 0, property: name("!is_autoplay") }, "the name is kept as the skin wrote it");
    assert_eq!(converted.property_kind(&first.op[2].property), LuaFnKind::Boolean);
    assert_eq!(skin.destination[1].draw, name("!!is_autoplay"));
    assert_eq!(converted.property_kind(&skin.destination[2].draw), LuaFnKind::Boolean);

    assert_eq!(skin.destination[3].draw, None, "a negated name nobody knows is a script that does not compile");
    assert_eq!(converted.warnings.len(), 1, "{:?}", converted.warnings);
    assert!(converted.warnings[0].contains("!gauge"), "{:?}", converted.warnings);
}

#[test]
fn a_script_that_does_not_compile_costs_only_its_own_field() {
    let converted = convert(
        r#"return {
            image = { { id = "a", x = 4, act = "return return", timer = "(" } },
            value = { { id = "v", digit = 3, value = "==" } },
            destination = { { id = "a", draw = "1 +", op = { 5, "then", 6 }, dst = { { x = 9 } } } },
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!((skin.image[0].x, &skin.image[0].act, &skin.image[0].timer), (4, &None, &None));
    assert_eq!((skin.value[0].digit, &skin.value[0].value), (3, &None));
    let destination = &skin.destination[0];
    assert_eq!(destination.draw, None);
    assert_eq!(
        destination.op,
        [DestinationOption { id: 5, property: None }, DestinationOption::UNCONDITIONAL, DestinationOption { id: 6, property: None }],
        "a condition that names nothing keeps its place and gates nothing"
    );
    assert_eq!(destination.dst[0].x, Some(9));
    assert_eq!(converted.warnings.len(), 5, "{:?}", converted.warnings);
    assert_eq!(converted.runtime.function_count(), 0);
}

#[test]
fn a_boolean_or_a_table_in_a_reference_field_is_no_reference() {
    let converted = convert(
        r#"return {
            image = { { id = "a", timer = true, act = {} } },
            value = { { id = "v", value = false } },
            text = { { id = "t", value = {}, event = true } },
            slider = { { id = "s", value = true, event = false } },
            destination = { { id = "a", draw = true, op = { true, false, {} } }, { id = "b", draw = false, timer = {} }, { id = "c", op = 5 } },
        }"#,
    );
    let skin = &converted.skin;
    assert_eq!((&skin.image[0].timer, &skin.image[0].act), (&None, &None));
    assert_eq!(skin.value[0].value, None);
    assert_eq!((&skin.text[0].value, &skin.text[0].event), (&None, &None));
    assert_eq!((&skin.slider[0].value, &skin.slider[0].event), (&None, &None));
    assert_eq!(skin.destination[0].draw, None, "a Lua boolean is not a condition");
    assert_eq!(skin.destination[0].op, [DestinationOption::UNCONDITIONAL; 3]);
    assert_eq!((&skin.destination[1].draw, &skin.destination[1].timer), (&None, &None));
    assert!(skin.destination[2].op.is_empty(), "a condition list that is not a table is empty");
    assert_eq!(converted.runtime.function_count(), 0);
    assert!(converted.warnings.is_empty());
}

#[test]
fn the_lists_inside_note_gauge_judge_and_song_list_are_read() {
    let converted = convert(
        r#"
        local visible = function() return true end
        return {
            type = 0,
            note = {
                id = "notes", note = { 1, 2, "n3" }, lnbodyActive = { 4 }, hcnbodyMiss = { 5.5 }, mine = { 6, nil, 8 },
                dst = { { x = 10.5, y = 20, w = 30, h = 40 }, { x = 50 } }, dst2 = 99.9, expansionrate = { 150.5, "80" }, size = { 1.5, 2, "3.25" },
                group = { { id = "group", draw = visible, timer = 143, dst = { { x = 1 } } } },
                time = { { id = "time", op = { 2, visible } }, { id = "time2", offset = 3.5 } },
            },
            gauge = { id = "gauge", nodes = { 1, 2, 3, "g4" }, parts = 48.7, type = 2 },
            judge = {
                {
                    id = "judge", index = 1.2, shift = 0,
                    images = { { id = "great", draw = visible, loop = -1, dst = { { time = 0, x = 1 }, { time = 500 } } } },
                    numbers = { { id = "count", timer = 46, offsets = { 3, 4.5 } }, { id = "count2" } },
                },
            },
            songlist = {
                id = "list", center = 4.2, clickable = { 1, 2.9 },
                liston = { { id = "on", dst = { { x = 1 } } } }, lamp = { { id = "lamp", draw = visible }, { id = "lamp2" } },
                graph = { id = "bars", op = { -5 }, dst = { { x = 2 } } },
            },
            value = { { id = "v", offset = { { id = "first", value = visible, x = 1.5 }, { id = "second", timer = 3 } } } },
        }"#,
    );
    let skin = &converted.skin;
    let shown = Some(PropertyRef::Func(skin.note.as_ref().and_then(|note| note.group[0].draw.as_ref()).and_then(PropertyRef::function).expect("a handle")));

    let note = skin.note.as_ref().expect("the note record");
    assert_eq!(note.id, "notes");
    assert_eq!(note.note, ["1", "2", "n3"]);
    assert_eq!(note.lnbody_active, ["4"]);
    assert_eq!(note.hcnbody_miss, ["5.5"]);
    assert_eq!(note.mine, ["6", "8"]);
    assert_eq!((note.dst.len(), note.dst[0].x, note.dst[0].h, note.dst[1].x, note.dst[1].y), (2, Some(10), Some(40), Some(50), None));
    assert_eq!(note.dst2, Some(99));
    assert_eq!(note.expansionrate, [150, 80]);
    assert_eq!(note.size, [1.5, 2.0, 3.25]);
    assert_eq!((note.group[0].id.as_str(), &note.group[0].timer, note.group[0].dst[0].x), ("group", &Some(PropertyRef::Id(143)), Some(1)));
    assert_eq!(note.time[0].op, [DestinationOption { id: 2, property: None }, DestinationOption { id: 0, property: shown.clone() }]);
    assert_eq!(note.time[1].offset, 3);
    assert!(note.bpm.is_empty() && note.stop.is_empty());

    let gauge = skin.gauge.as_ref().expect("the gauge record");
    assert_eq!(gauge.nodes, ["1", "2", "3", "g4"]);
    assert_eq!((gauge.parts, gauge.gauge_type, gauge.cycle), (48, 2, 33));

    let judge = &skin.judge[0];
    assert_eq!((judge.id.as_str(), judge.index, judge.shift), ("judge", 1, true));
    assert_eq!((&judge.images[0].draw, judge.images[0].loop_ms, judge.images[0].dst[1].time), (&shown, -1, Some(500)));
    assert_eq!(judge.numbers[0].timer, Some(PropertyRef::Id(46)));
    assert_eq!(judge.numbers[0].offsets, [3, 4]);
    assert_eq!(judge.numbers[1].id, "count2");

    let list = skin.songlist.as_ref().expect("the song list record");
    assert_eq!(list.center, 4);
    assert_eq!(list.clickable, [1, 2]);
    assert_eq!(list.liston[0].dst[0].x, Some(1));
    assert_eq!((&list.lamp[0].draw, &list.lamp[1].draw), (&shown, &None));
    let graph = list.graph.as_ref().expect("the bar graph");
    assert_eq!((graph.id.as_str(), graph.dst[0].x), ("bars", Some(2)));
    assert_eq!(graph.op, [DestinationOption { id: -5, property: None }]);

    let offsets = &skin.value[0].offset;
    assert_eq!((offsets[0].id.as_str(), offsets[0].x), ("first", 1));
    assert_eq!(converted.property_kind(&offsets[0].value), LuaFnKind::Integer);
    assert_ne!(offsets[0].value, shown);
    assert_eq!((offsets[1].id.as_str(), &offsets[1].timer), ("second", &Some(PropertyRef::Id(3))));
    assert_eq!(converted.runtime.function_count(), 2, "the one function is a condition in four places and a number in one");
}

#[test]
fn a_font_fallback_is_a_path_or_a_table_that_names_one() {
    let converted = convert(
        r#"return {
            font = {
                { id = 0, path = "main.ttf", type = 1.0, fallback = { "second.ttf", 7, { path = "third.ttf", type = 2.9 }, { path = true, type = "x" }, { path = 8 }, false } },
                { id = "plain", path = "plain.fnt" },
            },
        }"#,
    );
    let fonts = &converted.skin.font;
    assert_eq!((fonts[0].id.as_str(), fonts[0].path.as_str(), fonts[0].font_type), ("0", "main.ttf", 1));
    let fallbacks: Vec<(&str, i32)> = fonts[0].fallback.iter().map(|fallback| (fallback.path.as_str(), fallback.font_type)).collect();
    assert_eq!(fallbacks, [("second.ttf", 0), ("7", 0), ("third.ttf", 2), ("", 0), ("8", 0), ("", 0)]);
    assert!(fonts[1].fallback.is_empty());
}

/// Every file under `root`, relative to it.
fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                found.insert(relative.to_path_buf());
            }
        }
    }
    found
}

#[test]
fn an_external_skin_pack_converts_every_header_it_returns() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let overlay = std::env::temp_dir().join(format!("rbms-from-lua-{}-overlay", std::process::id()));
    let before = files_under(&pack);
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&pack)
        .expect("the pack should be listable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(LUA_SKIN_EXTENSION)))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "the pack holds no Lua skin");

    let mut rows = 0;
    for entry in &entries {
        let config = SkinLuaConfig { overlay: Some(overlay.clone()), seed: Some(TEST_SEED), mode: LuaMode::HeaderOnly, ..SkinLuaConfig::new(&pack) };
        let runtime = SkinLua::new(config).expect("the interpreter should build");
        let header =
            runtime.run_entry(entry, LuaPass::Header, &DefaultState).unwrap_or_else(|error| panic!("{} should return its header: {error}", entry.display()));
        let mut warnings = Vec::new();
        let skin = skin_def_from_lua(&header, &mut FromLua { lua: &runtime, path: entry, warnings: &mut warnings, known_name })
            .unwrap_or_else(|error| panic!("{} should convert: {error}", entry.display()));

        assert_ne!(skin.skin_type, SKIN_TYPE_UNSET, "{} declares a type", entry.display());
        assert!(!skin.name.is_empty(), "{} declares a name", entry.display());
        assert!(skin.property.iter().all(|row| !row.name.is_empty() && !row.item.is_empty()), "{} names every option and its items", entry.display());
        assert!(skin.filepath.iter().all(|row| !row.name.is_empty() && !row.path.is_empty()), "{} names every file slot and its pattern", entry.display());
        assert!(warnings.is_empty(), "{}: {warnings:?}", entry.display());
        rows += skin.property.len() + skin.filepath.len() + skin.offset.len();
    }
    let _ = std::fs::remove_dir_all(&overlay);
    assert!(rows > 0, "the pack offers nothing to customise");
    assert_eq!(files_under(&pack), before, "converting the pack changed its folder");
}

/// One document written twice, so the two readers can be held against each other. References are
/// ids throughout: a string there means source to one reader and a name or a script to the other.
const SAME_DOCUMENT_JSON: &str = r#"{
    "type": 6, "name": "Decide", "author": "rbms", "w": 1920, "h": 1080, "fadeout": 500, "input": 300, "scene": 3000,
    "category": [{ "name": "Main", "item": ["Background", "Image"] }],
    "property": [{ "category": "Main", "name": "Background", "item": [{ "name": "On", "op": 900 }, { "name": "Off", "op": 901 }], "def": "On" }],
    "filepath": [{ "name": "Image", "path": "bg/*.png", "def": "default" }],
    "offset": [{ "name": "Shift", "id": 40, "x": true, "y": true }],
    "source": [{ "id": "0", "path": "system.png" }],
    "font": [{ "id": "0", "path": "font.ttf", "type": 1, "fallback": [{ "path": "fallback.ttf", "type": 1 }] }],
    "image": [{ "id": "bg", "src": "0", "x": 0, "y": 0, "w": 1920, "h": 1080, "divx": 2, "timer": 2, "cycle": 100, "act": 13, "click": 1 }],
    "imageset": [{ "id": "set", "ref": 50, "value": 51, "images": ["bg", "bg"], "act": 14 }],
    "value": [{ "id": "num", "src": "0", "divx": 10, "digit": 4, "ref": 100, "value": 101, "offset": [{ "id": "off", "x": 3, "value": 102 }] }],
    "floatvalue": [{ "id": "rate", "src": "0", "iketa": 3, "fketa": 2, "gain": 0.5, "isSignvisible": true, "value": 110 }],
    "text": [{ "id": "title", "font": "0", "size": 24, "ref": 10, "value": 12, "overflow": 1, "outlineWidth": 1.5, "constantText": "hi", "wrapping": true }],
    "slider": [{ "id": "sl", "src": "0", "angle": 2, "range": 100, "type": 6, "value": 6, "event": 6, "changeable": false }],
    "graph": [{ "id": "gr", "src": "0", "angle": 0, "type": 1, "value": 140, "isRefNum": true, "min": 0, "max": 100 }],
    "hiddenCover": [{ "id": "hidden", "src": "0", "timer": 3, "disapearLine": 200, "isDisapearLineLinkLift": false }],
    "note": { "id": "notes", "note": ["bg", "bg"], "dst": [{ "x": 10, "y": 20, "w": 30, "h": 40 }], "dst2": 5, "size": [1.5, 2.0],
        "group": [{ "id": "bg", "timer": 143, "dst": [{ "x": 1 }] }] },
    "gauge": { "id": "gauge", "nodes": ["a", "b"], "parts": 48 },
    "bga": { "id": "bga" },
    "judge": [{ "id": "judge", "index": 0, "shift": true,
        "images": [{ "id": "bg", "loop": -1, "dst": [{ "time": 0, "x": 1 }, { "time": 100, "a": 0 }] }], "numbers": [{ "id": "num", "timer": 46 }] }],
    "songlist": { "id": "list", "center": 5, "clickable": [4, 5, 6], "liston": [{ "id": "bg", "dst": [{ "x": 10 }] }], "graph": { "id": "gr", "op": [2, -3] } },
    "customEvents": [{ "id": 1000, "action": 13, "condition": 40, "minInterval": 100 }],
    "customTimers": [{ "id": 10000, "timer": 41 }],
    "destination": [{
        "id": "bg", "blend": 2, "filter": 1, "timer": 2, "loop": 300, "center": 1, "offset": 40, "offsets": [41, 42], "stretch": 3,
        "op": [900, -901], "draw": 40, "mouseRect": { "x": 1, "y": 2, "w": 3, "h": 4 },
        "dst": [
            { "time": 0, "x": 0, "y": 0, "w": 1920, "h": 1080, "acc": 1, "a": 255, "r": 255, "g": 128, "b": 0, "angle": 90 },
            { "time": 300, "clip_x": 0, "clip_y": 0, "clip_w": 10, "clip_h": 10 }
        ]
    }]
}"#;

/// [`SAME_DOCUMENT_JSON`] as a Lua skin would return it.
const SAME_DOCUMENT_LUA: &str = r#"return {
    type = 6, name = "Decide", author = "rbms", w = 1920, h = 1080, fadeout = 500, input = 300, scene = 3000,
    category = { { name = "Main", item = { "Background", "Image" } } },
    property = { { category = "Main", name = "Background", item = { { name = "On", op = 900 }, { name = "Off", op = 901 } }, def = "On" } },
    filepath = { { name = "Image", path = "bg/*.png", def = "default" } },
    offset = { { name = "Shift", id = 40, x = true, y = true } },
    source = { { id = 0, path = "system.png" } },
    font = { { id = 0, path = "font.ttf", type = 1, fallback = { { path = "fallback.ttf", type = 1 } } } },
    image = { { id = "bg", src = 0, x = 0, y = 0, w = 1920, h = 1080, divx = 2, timer = 2, cycle = 100, act = 13, click = 1 } },
    imageset = { { id = "set", ref = 50, value = 51, images = { "bg", "bg" }, act = 14 } },
    value = { { id = "num", src = 0, divx = 10, digit = 4, ref = 100, value = 101, offset = { { id = "off", x = 3, value = 102 } } } },
    floatvalue = { { id = "rate", src = 0, iketa = 3, fketa = 2, gain = 0.5, isSignvisible = true, value = 110 } },
    text = { { id = "title", font = 0, size = 24, ref = 10, value = 12, overflow = 1, outlineWidth = 1.5, constantText = "hi", wrapping = true } },
    slider = { { id = "sl", src = 0, angle = 2, range = 100, type = 6, value = 6, event = 6, changeable = false } },
    graph = { { id = "gr", src = 0, angle = 0, type = 1, value = 140, isRefNum = true, min = 0, max = 100 } },
    hiddenCover = { { id = "hidden", src = 0, timer = 3, disapearLine = 200, isDisapearLineLinkLift = false } },
    note = { id = "notes", note = { "bg", "bg" }, dst = { { x = 10, y = 20, w = 30, h = 40 } }, dst2 = 5, size = { 1.5, 2.0 },
        group = { { id = "bg", timer = 143, dst = { { x = 1 } } } } },
    gauge = { id = "gauge", nodes = { "a", "b" }, parts = 48 },
    bga = { id = "bga" },
    judge = { { id = "judge", index = 0, shift = true,
        images = { { id = "bg", loop = -1, dst = { { time = 0, x = 1 }, { time = 100, a = 0 } } } }, numbers = { { id = "num", timer = 46 } } } },
    songlist = { id = "list", center = 5, clickable = { 4, 5, 6 }, liston = { { id = "bg", dst = { { x = 10 } } } }, graph = { id = "gr", op = { 2, -3 } } },
    customEvents = { { id = 1000, action = 13, condition = 40, minInterval = 100 } },
    customTimers = { { id = 10000, timer = 41 } },
    destination = { {
        id = "bg", blend = 2, filter = 1, timer = 2, loop = 300, center = 1, offset = 40, offsets = { 41, 42 }, stretch = 3,
        op = { 900, -901 }, draw = 40, mouseRect = { x = 1, y = 2, w = 3, h = 4 },
        dst = {
            { time = 0, x = 0, y = 0, w = 1920, h = 1080, acc = 1, a = 255, r = 255, g = 128, b = 0, angle = 90 },
            { time = 300, clip_x = 0, clip_y = 0, clip_w = 10, clip_h = 10 },
        },
    } },
}"#;

#[test]
fn a_lua_table_and_a_json_document_that_say_the_same_thing_are_the_same_document() {
    let from_json: SkinDef = serde_json::from_str(SAME_DOCUMENT_JSON).expect("the JSON document should parse");
    let converted = convert(SAME_DOCUMENT_LUA);
    assert_eq!(format!("{:#?}", converted.skin), format!("{from_json:#?}"));
    assert!(converted.warnings.is_empty());
    assert_eq!(converted.skin.destination[0].dst.len(), 2);
    assert_eq!(converted.skin.note.as_ref().map(|note| note.group.len()), Some(1));
}
