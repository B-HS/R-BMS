//! The serde mirror: that a document's defaults, its two ways of naming a property, and its unset
//! animation fields all survive the trip into Rust unchanged.

use rbms_skin::model::{Animation, Destination, ImageDef, PropertyRef, SkinDef, SliderDef, TextDef};

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
