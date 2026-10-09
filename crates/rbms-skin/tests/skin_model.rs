//! The serde mirror: that a document's defaults, its two ways of naming a property, and its unset
//! animation fields all survive the trip into Rust unchanged.

use rbms_skin::dst::{LuaFnId, TimerRef};
use rbms_skin::model::{
    Animation, CustomEvent, Destination, DestinationOption, EventRef, FloatWriterRef, ImageDef, ImageSet, PropertyRef, SkinDef, SliderDef, StringWriterRef,
    TextDef,
};
use rbms_skin::timer::TimerId;

/// Reads a document fragment the way the loader's strict path does.
fn parse<T: serde::de::DeserializeOwned>(text: &str) -> T {
    serde_json::from_str(text).expect("fixture fragment should parse")
}

#[test]
fn document_defaults_match_the_reference_initialisers() {
    let document: SkinDef = parse("{}");
    assert_eq!(document.skin_type, -1);
    assert_eq!(document.w, 1280);
    assert_eq!(document.h, 720);
    assert_eq!(document.judgetimer, 1);
    assert_eq!(document.finishmargin, 0);
    assert!(document.destination.is_empty());
    assert!(document.note.is_none());
}

#[test]
fn unknown_keys_are_ignored() {
    let document: SkinDef = parse(r#"{ "name": "kept", "somethingNobodyImplements": [1, 2, 3] }"#);
    assert_eq!(document.name, "kept");
}

#[test]
fn a_type_key_is_read_despite_being_a_rust_keyword() {
    let document: SkinDef = parse(r#"{ "type": 7 }"#);
    assert_eq!(document.skin_type, 7);
}

#[test]
fn a_property_reference_reads_an_id() {
    let value: PropertyRef = parse("41");
    assert_eq!(value, PropertyRef::Id(41));
    assert_eq!(value.id(), Some(41));
    assert_eq!(value.expr(), None);
}

#[test]
fn a_property_reference_reads_a_negative_id() {
    assert_eq!(parse::<PropertyRef>("-902"), PropertyRef::Id(-902));
}

#[test]
fn a_property_reference_reads_an_expression() {
    let value: PropertyRef = parse(r#""skin.number(10) > 0""#);
    assert_eq!(value.expr(), Some("skin.number(10) > 0"));
    assert_eq!(value.id(), None);
}

#[test]
fn a_property_reference_narrows_a_whole_float() {
    assert_eq!(parse::<PropertyRef>("41.0"), PropertyRef::Id(41));
}

#[test]
fn a_property_reference_rejects_an_id_too_large_for_the_field() {
    let outcome = serde_json::from_str::<PropertyRef>("9999999999");
    assert!(outcome.is_err(), "an id past 32 bits must not be silently wrapped");
}

#[test]
fn an_animation_leaves_unwritten_fields_unset() {
    let frame: Animation = parse(r#"{ "time": 100, "x": 5 }"#);
    assert_eq!(frame.time, Some(100));
    assert_eq!(frame.x, Some(5));
    assert_eq!(frame.y, None);
    assert_eq!(frame.a, None);
    assert_eq!(frame.clip_x, None);
    assert_eq!(frame.angle, None);
}

#[test]
fn an_animation_distinguishes_an_explicit_zero_from_an_absent_field() {
    let frame: Animation = parse(r#"{ "a": 0 }"#);
    assert_eq!(frame.a, Some(0));
    assert_eq!(frame.r, None);
}

#[test]
fn a_draw_condition_reads_the_bare_id_form() {
    let destination: Destination = parse(r#"{ "op": [901, 902] }"#);
    let ids: Vec<i32> = destination.op.iter().map(|option| option.id).collect();
    assert_eq!(ids, vec![901, 902]);
    assert!(destination.op.iter().all(|option| option.property.is_none()));
}

#[test]
fn a_draw_condition_reads_the_bare_expression_form() {
    let destination: Destination = parse(r#"{ "op": ["skin.boolean(1)"] }"#);
    assert_eq!(destination.op[0].id, 0);
    assert_eq!(destination.op[0].property.as_ref().and_then(PropertyRef::expr), Some("skin.boolean(1)"));
}

#[test]
fn a_draw_condition_reads_the_spelled_out_object_form() {
    let destination: Destination = parse(r#"{ "op": [{ "id": 905 }, { "property": "skin.boolean(2)" }] }"#);
    assert_eq!(destination.op[0].id, 905);
    assert_eq!(destination.op[1].property.as_ref().and_then(PropertyRef::expr), Some("skin.boolean(2)"));
}

#[test]
fn a_destination_defaults_its_stretch_to_unset() {
    let destination: Destination = parse("{}");
    assert_eq!(destination.stretch, -1);
    assert_eq!(destination.loop_ms, 0);
    assert!(destination.timer.is_none());
    assert!(destination.mouse_rect.is_none());
}

#[test]
fn a_destination_reads_the_loop_key_despite_being_a_rust_keyword() {
    let destination: Destination = parse(r#"{ "loop": 1200 }"#);
    assert_eq!(destination.loop_ms, 1200);
}

#[test]
fn an_image_divides_into_one_cell_by_default() {
    let image: ImageDef = parse(r#"{ "id": "frame" }"#);
    assert_eq!(image.divx, 1);
    assert_eq!(image.divy, 1);
    assert_eq!(image.click, 0);
}

#[test]
fn a_slider_is_changeable_by_default() {
    let slider: SliderDef = parse(r#"{ "id": "volume" }"#);
    assert!(slider.changeable);
    assert!(!slider.is_ref_num);
}

#[test]
fn text_carries_the_reference_outline_and_shadow_defaults() {
    let text: TextDef = parse(r#"{ "id": "title" }"#);
    assert_eq!(text.outline_color, "ffffff00");
    assert_eq!(text.shadow_color, "ffffff00");
    assert_eq!(text.overflow, 0);
    assert!(!text.editable);
}

#[test]
fn graph_palettes_carry_the_reference_defaults() {
    let document: SkinDef = parse(r#"{ "gaugegraph": [{ "id": "gauge" }], "bpmgraph": [{ "id": "bpm" }], "hiterrorvisualizer": [{ "id": "hit" }] }"#);
    assert_eq!(document.gaugegraph[0].assist_clear_bg_color, "440044");
    assert_eq!(document.gaugegraph[0].hazard_line_color, "cccccc");
    assert_eq!(document.bpmgraph[0].line_width, 2);
    assert_eq!(document.hiterrorvisualizer[0].width, 301);
    assert_eq!(document.hiterrorvisualizer[0].judge_width_millis, 150);
}

#[test]
fn a_note_set_starts_at_full_expansion() {
    let document: SkinDef = parse(r#"{ "note": { "id": "notes" } }"#);
    let notes = document.note.expect("the note set should be read");
    assert_eq!(notes.expansionrate, vec![100, 100]);
    assert!(notes.dst2.is_none());
}

#[test]
fn camel_case_keys_keep_their_document_spelling() {
    let document: SkinDef = parse(r#"{ "hiddenCover": [{ "id": "hide" }], "customTimers": [{ "id": 900, "timer": 41 }] }"#);
    assert_eq!(document.hidden_cover[0].disappear_line, -1);
    assert!(document.hidden_cover[0].disappear_line_follows_lift);
    assert_eq!(document.custom_timers[0].timer.as_ref().and_then(PropertyRef::id), Some(41));
}

#[test]
fn a_json_document_only_ever_spells_an_id_or_source() {
    for text in ["41", "-902", "41.0", r#""is_autoplay""#, r#""main_state.option(1)""#] {
        let value: PropertyRef = parse(text);
        assert!(matches!(value, PropertyRef::Id(_) | PropertyRef::Expr(_)), "{text} read as {value:?}");
        assert_eq!(value.function(), None, "a function value cannot be written in JSON");
        assert_eq!(value.name(), None, "and telling a name from source is the loader's job, not the parser's");
    }
}

#[test]
fn a_function_reference_answers_as_a_function_and_nothing_else() {
    let value = PropertyRef::Func(LuaFnId(8));
    assert_eq!(value.function(), Some(LuaFnId(8)));
    assert_eq!(value.id(), None);
    assert_eq!(value.name(), None);
    assert_eq!(value.expr(), None);
}

#[test]
fn a_named_reference_keeps_the_name_as_it_was_written() {
    let value = PropertyRef::Name("!is_autoplay".to_owned());
    assert_eq!(value.name(), Some("!is_autoplay"), "the negating prefix is part of the name");
    assert_eq!(value.id(), None);
    assert_eq!(value.function(), None);
    assert_eq!(value.expr(), None);
}

#[test]
fn a_reference_names_a_timer_only_as_an_id_or_a_function() {
    assert_eq!(PropertyRef::Id(41).timer(), Some(TimerRef::Id(TimerId(41))));
    assert_eq!(PropertyRef::Func(LuaFnId(3)).timer(), Some(TimerRef::Lua(LuaFnId(3))));
    assert_eq!(PropertyRef::Name("timer".to_owned()).timer(), None, "the reference has no timer names");
    assert_eq!(PropertyRef::Expr("main_state.timer(41)".to_owned()).timer(), None, "uncompiled source computes nothing");
}

#[test]
fn an_image_keeps_the_event_its_click_runs() {
    let image: ImageDef = parse(r#"{ "id": "button", "act": 13, "click": 2 }"#);
    assert_eq!(image.act, Some(EventRef::Id(13)));
    assert_eq!(image.click, 2);

    let scripted: ImageDef = parse(r#"{ "id": "button", "act": "main_state.event_exec(13)" }"#);
    assert_eq!(scripted.act, Some(EventRef::Script("main_state.event_exec(13)".to_owned())));

    let plain: ImageDef = parse(r#"{ "id": "frame" }"#);
    assert_eq!(plain.act, None, "an image with no event is not clickable");
}

#[test]
fn an_image_set_and_a_custom_event_keep_theirs_the_same_way() {
    let set: ImageSet = parse(r#"{ "id": "toggle", "images": ["off", "on"], "act": 42.0, "click": 1 }"#);
    assert_eq!(set.act, Some(EventRef::Id(42)), "a whole float narrows the way a property id does");
    assert_eq!(set.click, 1);

    let event: CustomEvent = parse(r#"{ "id": 1000, "action": "play()", "condition": 5, "minInterval": 200 }"#);
    assert_eq!(event.action, Some(EventRef::Script("play()".to_owned())));
    assert_eq!(event.condition, Some(PropertyRef::Id(5)));
}

#[test]
fn a_slider_keeps_where_a_drag_writes() {
    let slider: SliderDef = parse(r#"{ "id": "volume", "type": 17, "event": 17 }"#);
    assert_eq!(slider.event, Some(FloatWriterRef::Id(17)));

    let scripted: SliderDef = parse(r#"{ "id": "volume", "event": "set_volume" }"#);
    assert_eq!(scripted.event, Some(FloatWriterRef::Script("set_volume".to_owned())));

    let plain: SliderDef = parse(r#"{ "id": "volume" }"#);
    assert_eq!(plain.event, None);
}

#[test]
fn a_text_keeps_where_typing_writes_and_has_no_id_form() {
    let text: TextDef = parse(r#"{ "id": "search", "event": "search_word" }"#);
    assert_eq!(text.event, Some(StringWriterRef::Script("search_word".to_owned())));

    let numbered: TextDef = parse(r#"{ "id": "search", "event": 30 }"#);
    assert_eq!(numbered.event, None, "a string writer has no id lookup, so a number names nothing");

    let null: TextDef = parse(r#"{ "id": "search", "event": null }"#);
    assert_eq!(null.event, None);
    let absent: TextDef = parse(r#"{ "id": "search" }"#);
    assert_eq!(absent.event, None);
}

#[test]
fn the_function_forms_of_events_and_writers_compare_by_handle() {
    assert_eq!(EventRef::Lua(LuaFnId(1)), EventRef::Lua(LuaFnId(1)));
    assert_ne!(EventRef::Lua(LuaFnId(1)), EventRef::Lua(LuaFnId(2)));
    assert_ne!(EventRef::Lua(LuaFnId(1)), EventRef::Id(1), "a handle is not an event id");
    assert_ne!(FloatWriterRef::Lua(LuaFnId(1)), FloatWriterRef::Id(1));
    assert_ne!(StringWriterRef::Lua(LuaFnId(1)), StringWriterRef::Name("1".to_owned()));
}

#[test]
fn an_entry_with_neither_an_id_nor_a_property_is_no_condition() {
    assert!(DestinationOption::UNCONDITIONAL.is_unconditional());
    assert!(DestinationOption::default().is_unconditional(), "which is also what an entry starts as");
    assert_eq!(DestinationOption::UNCONDITIONAL.id, 0);
    assert!(DestinationOption::UNCONDITIONAL.property.is_none());

    let destination: Destination = parse(r#"{ "op": [0, {}, 901, "skin.boolean(1)"] }"#);
    let unconditional: Vec<bool> = destination.op.iter().map(DestinationOption::is_unconditional).collect();
    assert_eq!(unconditional, vec![true, true, false, false], "a zero id and an empty object gate nothing; an id and a property do");
}

#[test]
fn a_function_in_an_op_list_is_a_property_not_an_id() {
    let option = DestinationOption { id: 0, property: Some(PropertyRef::Func(LuaFnId(4))) };
    assert!(!option.is_unconditional());
    assert_eq!(option.property.as_ref().and_then(PropertyRef::function), Some(LuaFnId(4)));
}
