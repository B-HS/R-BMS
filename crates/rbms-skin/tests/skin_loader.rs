//! The loader: reading a document within its ceiling, parsing it strictly and then leniently,
//! resolving the clauses and includes that encode the player's choices, filling in the keyframe
//! fields a document leaves out, and turning what is left into tracks.
//!
//! The fixtures under `tests/fixtures` are written in the shape the reference implementation's
//! documents use, and every rule asserted here is one of that implementation's, apart from the two
//! this player adds on purpose: a size ceiling on the parser, and the containment rule that keeps a
//! document inside its own directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::SkinError;
use rbms_skin::dst::{Acc, DrawCondition, LOOP_ONCE, OffsetSource, SkinOffset, SkinRect, TimerRef, resolve};
use rbms_skin::loader::{
    DEFAULT_MAX_DOCUMENT_BYTES, Filtering, LoadedSkin, MAX_INCLUDE_DEPTH, OPTION_RANDOM_VALUE, ParserKind, SKIN_TYPE_DECIDE, SKIN_TYPE_KEY_CONFIG,
    SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_PLAY_7KEYS, SKIN_TYPE_RESULT, SkinLoadOptions, SkinUserConfig, StretchKind, enabled_options, filtering_for,
    is_supported_skin_type, load_header, load_skin, mode_skin_type, parse_document, read_document, selected_option, skin_type_mode, stretch_rect,
};
use rbms_skin::model::{Destination, PropertyDef, PropertyItem};
use rbms_skin::timer::{TimerId, TimerState};

/// A seed every wildcard test pins, so a draw is the same on every machine.
const TEST_SEED: u64 = 7;

/// The option the minimal document's `Panel` row turns on by default.
const PANEL_ON: i32 = 901;

/// The option the minimal document's `Panel` row turns on when the player switches it.
const PANEL_OFF: i32 = 902;

/// The fixtures directory this file reads from.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

/// The minimal fixture's directory, which is also its root.
fn minimal_root() -> PathBuf {
    fixtures().join("minimal")
}

/// A scratch directory that removes itself, for the tests that need documents of their own.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rbms-loader-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory should be creatable");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// Writes one document into the scratch directory and returns its path.
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).expect("the document should be writable");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Options over `root` with the wildcard draw pinned.
fn seeded<'a>(root: &'a Path, user: &'a SkinUserConfig) -> SkinLoadOptions<'a> {
    let mut options = SkinLoadOptions::new(root, user, Mode::BEAT_7K);
    options.rng_seed = Some(TEST_SEED);
    options
}

/// The minimal fixture, loaded with the player's choices.
fn load_minimal(user: &SkinUserConfig) -> LoadedSkin {
    let root = minimal_root();
    load_skin(&root.join("skin.json"), seeded(&root, user)).expect("the minimal fixture should load")
}

/// The ids of a load's objects, in document order.
fn object_ids(skin: &LoadedSkin) -> Vec<&str> {
    skin.destinations.iter().map(|named| named.id.as_str()).collect()
}

/// One object's track.
fn track<'a>(skin: &'a LoadedSkin, id: &str) -> &'a rbms_skin::dst::DestinationTrack {
    &skin.destinations.iter().find(|named| named.id == id).unwrap_or_else(|| panic!("the load should carry the object {id:?}")).track
}

/// A customisation row offering two items.
fn two_item_row(default: Option<&str>) -> PropertyDef {
    PropertyDef {
        category: "layout".to_owned(),
        name: "Panel".to_owned(),
        item: vec![PropertyItem { name: "on".to_owned(), op: PANEL_ON }, PropertyItem { name: "off".to_owned(), op: PANEL_OFF }],
        def: default.map(str::to_owned),
    }
}

/// A destination written the way a document writes one.
fn destination(text: &str) -> Destination {
    serde_json::from_str(text).expect("the destination fragment should parse")
}

#[test]
fn a_document_over_the_ceiling_is_refused_before_it_is_parsed() {
    let scratch = Scratch::new("ceiling");
    let path = scratch.write("skin.json", r#"{ "type": 5 }"#);
    let outcome = read_document(&path, 4);
    match outcome {
        Err(SkinError::TooLarge { actual, limit, .. }) => {
            assert_eq!(limit, 4);
            assert!(actual > limit, "the refusal should report the size it saw");
        }
        other => panic!("a document over the ceiling must not be parsed, got {other:?}"),
    }
}

#[test]
fn the_default_ceiling_is_the_one_the_loader_documents() {
    assert_eq!(DEFAULT_MAX_DOCUMENT_BYTES, 8 * 1024 * 1024);
}

#[test]
fn a_byte_order_mark_does_not_stop_the_strict_parser() {
    let scratch = Scratch::new("bom");
    let path = scratch.write("skin.json", "\u{feff}{ \"type\": 5, \"name\": \"marked\" }");
    let text = read_document(&path, DEFAULT_MAX_DOCUMENT_BYTES).expect("the document should be readable");
    let (document, parser) = parse_document(&path, &text).expect("a marked document is still valid JSON");

    assert_eq!(parser, ParserKind::Json);
    assert_eq!(document.name, "marked");
}

#[test]
fn a_valid_document_takes_the_strict_parser() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert_eq!(skin.parser, ParserKind::Json);
}

#[cfg(feature = "json5")]
#[test]
fn a_document_with_comments_and_trailing_commas_takes_the_lenient_parser() {
    let root = fixtures().join("lenient");
    let user = SkinUserConfig::default();
    let skin = load_skin(&root.join("skin.json5"), seeded(&root, &user)).expect("the lenient fixture should load");

    assert_eq!(skin.parser, ParserKind::Json5);
    assert_eq!(skin.def.name, "lenient fixture");
    assert_eq!(object_ids(&skin), vec!["solo"]);
}

#[cfg(not(feature = "json5"))]
#[test]
fn a_build_without_the_lenient_parser_reports_the_strict_ones_complaint() {
    let root = fixtures().join("lenient");
    let user = SkinUserConfig::default();
    let outcome = load_skin(&root.join("skin.json5"), seeded(&root, &user));
    assert!(matches!(outcome, Err(SkinError::Parse { .. })), "a document only the lenient parser reads is refused, got {outcome:?}");
}

#[test]
fn a_document_neither_parser_can_read_is_reported_with_a_position() {
    let scratch = Scratch::new("broken");
    let path = scratch.write("skin.json", "{ this is not json at all ][ }");
    let text = read_document(&path, DEFAULT_MAX_DOCUMENT_BYTES).expect("the document should be readable");

    match parse_document(&path, &text) {
        Err(SkinError::Parse { line, column, .. }) => {
            assert!(line > 0 && column > 0, "the position should be one-based, got {line}:{column}");
        }
        other => panic!("an unreadable document should report where it failed, got {other:?}"),
    }
}

#[test]
fn a_header_reads_what_the_configuration_screen_lists_without_loading_the_document() {
    let root = minimal_root();
    let user = SkinUserConfig::default();
    let header = load_header(&root.join("skin.json"), seeded(&root, &user)).expect("the header should load");

    assert_eq!(header.skin_type, 5);
    assert_eq!(header.name, "fixture select");
    assert_eq!(header.author, "rbms tests");
    assert_eq!((header.width, header.height), (1280, 720));
    assert_eq!(header.properties.len(), 1);
    assert_eq!(header.offsets.len(), 1);
    assert_eq!(header.custom_files.len(), 1);
    assert_eq!(header.custom_files[0].candidates, vec!["Random", "groove.png", "hard.png", "shell.png"]);
}

#[test]
fn a_header_outside_the_root_is_refused() {
    let root = minimal_root();
    let user = SkinUserConfig::default();
    let outcome = load_header(&fixtures().join("escape").join("skin.json"), seeded(&root, &user));
    assert!(matches!(outcome, Err(SkinError::PathEscape(_))), "a document outside the root must not be read, got {outcome:?}");
}

#[test]
fn a_document_that_declares_no_type_is_refused() {
    let scratch = Scratch::new("no-type");
    let path = scratch.write("skin.json", r#"{ "name": "typeless" }"#);
    let user = SkinUserConfig::default();
    assert!(matches!(load_skin(&path, seeded(scratch.path(), &user)), Err(SkinError::TypeMissing)));
}

#[test]
fn a_document_whose_screen_this_build_does_not_draw_is_refused() {
    let scratch = Scratch::new("odd-type");
    let path = scratch.write("skin.json", r#"{ "type": 99 }"#);
    let user = SkinUserConfig::default();
    assert!(matches!(load_skin(&path, seeded(scratch.path(), &user)), Err(SkinError::TypeUnsupported(99))));
}

#[test]
fn the_play_types_name_modes_and_the_other_screens_name_none() {
    assert_eq!(skin_type_mode(SKIN_TYPE_PLAY_7KEYS), Some(Mode::BEAT_7K));
    assert_eq!(skin_type_mode(2), Some(Mode::BEAT_14K));
    assert_eq!(skin_type_mode(5), None);

    assert!(is_supported_skin_type(SKIN_TYPE_PLAY_7KEYS), "the seven-key play screen is drawn");
    assert!(is_supported_skin_type(SKIN_TYPE_MUSIC_SELECT), "music select is drawn");
    assert!(!is_supported_skin_type(99), "a screen nobody declares is not drawn");
}

/// A run reaches for a document by the mode it is played in, which has to be the same table read
/// backwards -- a mode that names a type must be the mode that type names.
#[test]
fn the_mode_of_a_play_type_names_that_type_back() {
    for skin_type in [SKIN_TYPE_PLAY_7KEYS, 1, 2, 3, 4, 16] {
        let mode = skin_type_mode(skin_type).expect("a play type names a mode");
        assert_eq!(mode_skin_type(mode), Some(skin_type), "type {skin_type} and its mode disagree");
    }
    for screen in [SKIN_TYPE_MUSIC_SELECT, SKIN_TYPE_DECIDE, SKIN_TYPE_RESULT, SKIN_TYPE_KEY_CONFIG] {
        assert_eq!(skin_type_mode(screen), None, "screen {screen} is not a play screen");
    }
}

#[test]
fn a_load_resolves_every_image_source_and_font_it_names() {
    let skin = load_minimal(&SkinUserConfig::default());

    assert!(skin.sources["0"].ends_with("images/frame.png"), "a plain path resolves to itself: {}", skin.sources["0"].display());
    assert!(skin.fonts["0"].ends_with("fonts/base.ttf"), "a font wildcard resolves to the one file there is");
    let gauge = skin.sources["1"].file_name().and_then(|name| name.to_str()).expect("the gauge source resolves to a file");
    assert!(["groove.png", "hard.png", "shell.png"].contains(&gauge), "the gauge wildcard drew {gauge}");
    assert!(skin.warnings.is_empty(), "the minimal fixture resolves cleanly: {:?}", skin.warnings);
}

#[test]
fn the_file_the_player_chose_wins_over_the_wildcard_draw() {
    let mut user = SkinUserConfig::default();
    user.filepaths.insert("Gauge".to_owned(), "hard.png".to_owned());
    let skin = load_minimal(&user);

    assert!(skin.sources["1"].ends_with("gauge/hard.png"), "resolved {}", skin.sources["1"].display());
    let (key, name) = skin.filemap().iter().next().expect("the chosen slot should contribute a filemap entry");
    assert!(key.ends_with("gauge/*.png"), "the filemap key is the pattern the document wrote: {key}");
    assert_eq!(name, "hard.png");
}

#[test]
fn a_load_keeps_the_file_slots_it_scanned_for_the_configuration_screen() {
    let skin = load_minimal(&SkinUserConfig::default());

    assert_eq!(skin.custom_files.len(), 1);
    assert_eq!(skin.custom_files[0].name, "Gauge");
    assert_eq!(skin.custom_files[0].candidates, vec!["Random", "groove.png", "hard.png", "shell.png"]);
}

#[test]
fn a_load_is_reproducible_for_the_same_seed() {
    let user = SkinUserConfig::default();
    let first = load_minimal(&user);
    let second = load_minimal(&user);
    assert_eq!(first.sources, second.sources, "a pinned seed draws the same files twice");
}

#[test]
fn a_source_that_leaves_the_root_costs_its_object_and_not_the_skin() {
    let root = fixtures().join("escape");
    let user = SkinUserConfig::default();
    let skin = load_skin(&root.join("skin.json"), seeded(&root, &user)).expect("one unreadable file must not fail the document");

    assert!(skin.sources.is_empty(), "a source outside the root resolves to nothing");
    assert_eq!(skin.warnings.len(), 1);
    assert!(skin.warnings[0].contains("resolves outside the skin root"), "warned {:?}", skin.warnings[0]);
}

#[test]
fn a_stored_file_choice_that_climbs_out_of_the_root_is_refused_like_any_other_path() {
    let mut user = SkinUserConfig::default();
    user.filepaths.insert("Gauge".to_owned(), "../../secret.txt".to_owned());
    let root = minimal_root();
    let skin = load_skin(&root.join("skin.json"), seeded(&root, &user)).expect("a bad stored choice must not fail the document");

    assert!(!skin.sources.contains_key("1"), "the slot the choice substitutes into resolves to nothing");
    assert!(
        skin.warnings.iter().any(|warning| warning.contains("resolves outside the skin root")),
        "a settings file is not a licence to read outside the root: {:?}",
        skin.warnings
    );
}

#[test]
fn the_options_the_players_choices_turn_on_follow_the_rows_they_belong_to() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert!(skin.enabled_options.contains(&PANEL_ON), "the row's own default is on when nobody has chosen");

    let mut user = SkinUserConfig::default();
    user.properties.insert("Panel".to_owned(), PANEL_OFF);
    let switched = load_minimal(&user);
    assert!(switched.enabled_options.contains(&PANEL_OFF));
    assert!(!switched.enabled_options.contains(&PANEL_ON));
}

#[test]
fn a_guarded_clause_list_keeps_the_clause_whose_option_is_on() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert!(object_ids(&skin).contains(&"panel-on"));
    assert!(!object_ids(&skin).contains(&"panel-off"));

    let mut user = SkinUserConfig::default();
    user.properties.insert("Panel".to_owned(), PANEL_OFF);
    let switched = load_minimal(&user);
    assert!(object_ids(&switched).contains(&"panel-off"));
    assert!(!object_ids(&switched).contains(&"panel-on"));
}

#[test]
fn a_clause_list_nothing_satisfies_contributes_no_object() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert!(!object_ids(&skin).contains(&"never"), "an unmatched clause list must drop out rather than become a null object");
}

#[test]
fn an_included_file_takes_the_place_of_the_object_that_names_it() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert!(object_ids(&skin).contains(&"included-panel"));
    assert_eq!(track(&skin, "included-panel").frames[0].rect, SkinRect::new(9.0, 9.0, 9.0, 9.0));
}

#[test]
fn a_first_keyframe_fills_its_unset_fields_from_the_type_defaults() {
    let skin = load_minimal(&SkinUserConfig::default());
    let first = track(&skin, "frame").frames[0];

    assert_eq!(first.time_ms, 0);
    assert_eq!(first.rect, SkinRect::new(10.0, 20.0, 100.0, 50.0));
    assert_eq!((first.color.r, first.color.g, first.color.b, first.color.a), (255, 255, 255, 255));
    assert_eq!(track(&skin, "frame").acc, Acc::Accelerate, "the first keyframe's easing is the object's");
    assert!(first.clip.is_none(), "an unset clip on the first keyframe means the object is not clipped");
}

#[test]
fn a_later_keyframe_inherits_every_field_it_does_not_restate() {
    let skin = load_minimal(&SkinUserConfig::default());
    let frames = &track(&skin, "frame").frames;
    let middle = frames[1];

    assert_eq!(middle.time_ms, 500);
    assert_eq!(middle.rect, SkinRect::new(110.0, 20.0, 100.0, 50.0), "only x was restated");
    assert_eq!(middle.color.a, 255);
    assert_eq!(frames[2].rect, SkinRect::new(110.0, 20.0, 100.0, 50.0), "the last keyframe restates neither position nor size");
    assert_eq!(frames[2].color.a, 0);
}

#[test]
fn a_clip_becomes_a_rectangle_only_once_all_four_of_its_edges_are_known() {
    let skin = load_minimal(&SkinUserConfig::default());
    let frames = &track(&skin, "frame").frames;

    assert!(frames[1].clip.is_none());
    assert_eq!(frames[2].clip, Some(SkinRect::new(4.0, 8.0, 640.0, 360.0)));
}

#[test]
fn the_documents_angle_turns_the_other_way_from_the_renderers() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert_eq!(track(&skin, "frame").frames[0].angle_deg, -30.0, "a document turns anticlockwise on a y-up axis, the renderer clockwise on a y-down one");
}

#[test]
fn keyframes_arrive_in_ascending_time_order() {
    let mut skin = load_minimal(&SkinUserConfig::default());
    let shuffled = destination(r#"{ "id": "shuffled", "dst": [{ "time": 900, "x": 3 }, { "time": 100, "x": 1 }, { "time": 500, "x": 2 }] }"#);
    let built = skin.build_track(&shuffled, false).expect("the track should build").expect("its conditions hold");

    let times: Vec<i64> = built.frames.iter().map(|frame| frame.time_ms).collect();
    assert_eq!(times, vec![100, 500, 900], "a document may write its keyframes in any order");
    let ordered: Vec<i64> = track(&skin, "frame").frames.iter().map(|frame| frame.time_ms).collect();
    assert_eq!(ordered, vec![0, 500, 1000]);
}

#[test]
fn a_colour_channel_outside_the_byte_range_is_held_to_it() {
    let mut skin = load_minimal(&SkinUserConfig::default());
    let loud = destination(r#"{ "id": "loud", "dst": [{ "time": 0, "a": 300, "r": -5, "g": 128 }] }"#);
    let built = skin.build_track(&loud, false).expect("the track should build").expect("its conditions hold");
    let colour = built.frames[0].color;

    assert_eq!((colour.a, colour.r, colour.g), (255, 0, 128));
}

#[test]
fn a_keyframe_time_wider_than_the_field_costs_its_object_and_not_the_skin() {
    let scratch = Scratch::new("wide-time");
    let path = scratch
        .write("skin.json", r#"{ "type": 5, "destination": [{ "id": "wide", "dst": [{ "time": 4294967296 }] }, { "id": "fine", "dst": [{ "time": 0 }] }] }"#);
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("one impossible keyframe must not fail the document");

    assert_eq!(object_ids(&skin), vec!["fine"]);
    assert!(skin.warnings[0].contains("dst.time"), "warned {:?}", skin.warnings[0]);
}

/// The frame clock half way along the one-second tracks below, in microseconds.
const HALF_WAY_US: i64 = 500_000;

/// Loads a one-object document and resolves that object half way along its second.
fn x_half_way(name: &str, keyframes: &str) -> (Acc, f32) {
    let scratch = Scratch::new(name);
    let path = scratch.write("skin.json", &format!(r#"{{ "type": 5, "destination": [{{ "id": "eased", "loop": -1, "dst": {keyframes} }}] }}"#));
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document should load");
    let track = track(&skin, "eased");
    let resolved = resolve(track, HALF_WAY_US, &TimerState::new(), &user, None).expect("the object draws half way along");
    (track.acc, resolved.rect.x)
}

#[test]
fn the_objects_easing_is_the_first_one_declared_even_when_the_keyframes_are_written_backwards() {
    let (acc, x) = x_half_way("acc-reversed", r#"[{ "time": 1000, "x": 1000, "acc": 2 }, { "time": 0, "x": 0, "acc": 1 }]"#);
    assert_eq!(acc, Acc::Decelerate, "the keyframe written first claims the easing, though it sorts last");
    assert_eq!(x, 750.0, "1 - (0.5 - 1)^2 of the way, not the 0.5^2 the earliest keyframe's own value would give");
}

#[test]
fn the_objects_easing_is_the_first_non_zero_one_in_declaration_order() {
    let (acc, x) = x_half_way("acc-first-non-zero", r#"[{ "time": 0, "x": 0 }, { "time": 1000, "x": 1000, "acc": 1 }, { "time": 2000, "acc": 2 }]"#);
    assert_eq!(acc, Acc::Accelerate, "an unset first keyframe leaves the slot open for the next one");
    assert_eq!(x, 250.0, "and the pair before the keyframe that declared it is shaped by it too");

    let (acc, _) = x_half_way("acc-inherited", r#"[{ "time": 1000, "x": 1000, "acc": 3 }, { "time": 0, "x": 0 }]"#);
    assert_eq!(acc, Acc::Step, "a later keyframe that restates nothing inherits the claimed value rather than replacing it");
}

#[test]
fn an_easing_the_reference_does_not_name_still_claims_the_object() {
    let (acc, x) = x_half_way("acc-unknown", r#"[{ "time": 0, "x": 0, "acc": 4 }, { "time": 1000, "x": 1000, "acc": 1 }]"#);
    assert_eq!(acc, Acc::Linear, "4 is not zero, so it takes the slot, and it shapes nothing");
    assert_eq!(x, 500.0);

    let (acc, _) = x_half_way("acc-negative", r#"[{ "time": 0, "x": 0, "acc": -1 }, { "time": 1000, "x": 1000, "acc": 2 }]"#);
    assert_eq!(acc, Acc::Linear);
}

#[test]
fn a_positive_offset_r_turns_an_object_the_way_a_larger_document_angle_does() {
    const NUDGED_SLOT: i32 = 20;
    let scratch = Scratch::new("offset-r");
    let path = scratch.write(
        "skin.json",
        r#"{ "type": 5, "destination": [
            { "id": "nudged", "offset": 20, "dst": [{ "time": 0, "angle": 30 }] },
            { "id": "written", "dst": [{ "time": 0, "angle": 45 }] }
        ] }"#,
    );
    let mut user = SkinUserConfig::default();
    user.offsets.insert(NUDGED_SLOT, SkinOffset { r: 15.0, ..SkinOffset::default() });
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document should load");

    let nudged = resolve(track(&skin, "nudged"), 0, &TimerState::new(), &user, None).expect("the nudged object draws");
    let written = resolve(track(&skin, "written"), 0, &TimerState::new(), &user, None).expect("the written object draws");
    assert_eq!(nudged.angle_deg, written.angle_deg, "`angle: 30` with `r: 15` is `angle: 45`, because the reference adds the two in the document's own space");
    assert_eq!(nudged.angle_deg, -45.0, "counter-clockwise in the document is negative on a screen that turns clockwise");
}

#[test]
fn the_offset_list_is_the_documents_offsets_with_its_single_offset_appended() {
    let skin = load_minimal(&SkinUserConfig::default());
    assert_eq!(track(&skin, "frame").offsets, vec![21, 20]);
    assert_eq!(track(&skin, "panel-on").offsets, vec![0], "the single offset is appended even when it is zero");
}

#[test]
fn the_drawing_fields_the_document_wrote_are_carried_through() {
    let skin = load_minimal(&SkinUserConfig::default());
    let built = track(&skin, "frame");

    assert_eq!(built.timer, Some(TimerRef::Id(TimerId(1))));
    assert_eq!(built.loop_ms, LOOP_ONCE);
    assert_eq!((built.blend, built.filter, built.center), (2, 1, 4));
    assert_eq!(built.stretch, 1);
    let mouse = built.mouse_rect.expect("the document wrote a pointer rectangle");
    assert_eq!((mouse.x, mouse.y, mouse.w, mouse.h), (0.0, 0.0, 32.0, 32.0));
}

/// A document's own customisation ids cannot change while it is loaded, so they are settled while
/// the track is built and never reach the frame: the fixture's `frame` object is gated on `901` and
/// `-902`, both of which hold under the default choice, and the track it builds carries no
/// condition at all (`Skin.prepare` empties the option list of every object it keeps).
#[test]
fn a_condition_the_documents_own_choices_settle_never_reaches_a_frame() {
    let skin = load_minimal(&SkinUserConfig::default());
    let conditions = &track(&skin, "frame").draw_conditions;

    assert!(conditions.is_empty(), "901, -902 and a zero were all settled at load: {conditions:?}");
}

/// The other side of the same rule: switch the row to its second item and the object that asked for
/// the first one is gone from the draw list rather than being asked about every frame.
#[test]
fn an_object_the_documents_own_choices_rule_out_is_dropped_at_load() {
    let mut user = SkinUserConfig::default();
    user.properties.insert("Panel".to_owned(), PANEL_OFF);
    let skin = load_minimal(&user);

    let ids: Vec<&str> = skin.destinations.iter().map(|named| named.id.as_str()).collect();
    assert!(!ids.contains(&"frame"), "the object wanted 901 and the player chose 902: {ids:?}");
    assert!(ids.contains(&"panel-off"), "and the clause the choice does select is still there: {ids:?}");
}

#[test]
fn an_option_this_build_does_not_implement_drops_its_condition() {
    let scratch = Scratch::new("unknown-option");
    let path = scratch.write("skin.json", r#"{ "type": 5, "destination": [{ "id": "gated", "op": [777, 888], "dst": [{ "time": 0 }] }] }"#);
    let user = SkinUserConfig::default();
    let mut options = seeded(scratch.path(), &user);
    options.known_option = |id| id != 888;
    let skin = load_skin(&path, options).expect("the document should load");
    let conditions = &track(&skin, "gated").draw_conditions;

    assert_eq!(conditions.len(), 1, "the unimplemented option drops out rather than hiding the object: {conditions:?}");
    assert_eq!(conditions[0], DrawCondition::Option(777));
}

/// The build's own predicate has no say over a document's customisation ids, which are declared by
/// the document. A predicate that knows nothing must neither hide the object nor leave the id to be
/// asked about at draw time, where no state source answers it.
#[test]
fn an_option_the_document_declares_is_settled_whatever_the_build_predicate_says() {
    let root = minimal_root();
    let user = SkinUserConfig::default();
    let mut options = seeded(&root, &user);
    options.known_option = |_| false;
    let skin = load_skin(&root.join("skin.json"), options).expect("the fixture should load");

    let ids: Vec<&str> = skin.destinations.iter().map(|named| named.id.as_str()).collect();
    assert!(ids.contains(&"frame"), "the chosen variant survives a predicate that knows nothing: {ids:?}");
    assert!(track(&skin, "frame").draw_conditions.is_empty(), "and its conditions were settled rather than left for a frame");
}

#[cfg(feature = "lua")]
#[test]
fn a_script_names_a_draw_condition_of_its_own_and_a_name_stays_a_name() {
    let mut skin = load_minimal(&SkinUserConfig::default());
    let mixed = destination(r#"{ "id": "mixed", "op": [901, "number(10) > 0", "!autoplay_on"], "draw": "timer(41) > 0", "dst": [{ "time": 0 }] }"#);

    let built = skin.build_track(&mixed, false).expect("the track should build").expect("its conditions hold");
    let conditions = &built.draw_conditions;
    assert_eq!(conditions.len(), 3, "the document's own option was settled at load and the three strings are left: {conditions:?}");
    assert!(matches!(conditions[0], DrawCondition::Function(_)), "a string no table knows is a script: {conditions:?}");
    assert_eq!(conditions[1], DrawCondition::Name("!autoplay_on".to_owned()), "a property name is kept as the document wrote it, negation included");
    assert!(matches!(conditions[2], DrawCondition::Function(_)), "and `draw` comes last: {conditions:?}");
    assert_eq!(skin.runtime().expect("a build with Lua gives a document an interpreter").function_count(), 2, "one function per script, none for the name");
    assert!(skin.warnings.is_empty(), "{:?}", skin.warnings);
}

#[cfg(feature = "lua")]
#[test]
fn a_script_that_will_not_compile_costs_its_condition_and_not_the_skin() {
    let scratch = Scratch::new("bad-lua");
    let path = scratch.write(
        "skin.json",
        r#"{ "type": 5, "destination": [{ "id": "broken", "op": ["1 +"], "dst": [{ "time": 0 }] }, { "id": "fine", "dst": [{ "time": 0 }] }] }"#,
    );
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("one broken script must not fail the document");

    assert_eq!(object_ids(&skin), vec!["broken", "fine"], "the reference reads a script that will not compile as no condition at all");
    assert!(track(&skin, "broken").draw_conditions.is_empty());
    assert!(skin.warnings[0].contains("1 +"), "the warning should quote the script, got {:?}", skin.warnings[0]);
    assert!(!skin.warnings[0].contains("rbms-skin/src"), "a player-facing warning must not name this crate's own files: {:?}", skin.warnings[0]);
}

#[cfg(feature = "lua")]
#[test]
fn every_kind_of_field_a_document_writes_a_script_in_is_compiled_at_load() {
    use rbms_skin::model::{EventRef, FloatWriterRef, PropertyRef, StringWriterRef};

    let scratch = Scratch::new("script-fields");
    let path = scratch.write(
        "skin.json",
        r#"{
            "type": 5,
            "image": [{ "id": "button", "src": "0", "timer": "timer(41)", "act": "event_exec(13)" }],
            "value": [{ "id": "count", "src": "0", "value": "number(71) + 1" }, { "id": "named", "src": "0", "value": "folder_max" }],
            "floatvalue": [{ "id": "share", "src": "0", "value": "float_number(110) / 2" }],
            "text": [{ "id": "title", "font": "0", "value": "text(10) .. '!'", "event": "set_title(...)" }],
            "slider": [{ "id": "volume", "src": "0", "value": "volume_sys()", "event": "set_volume_sys(...)" }],
            "graph": [{ "id": "rate", "src": "0", "value": "rate() / 100" }],
            "customEvents": [{ "id": 1000, "action": "event_exec(14)", "condition": "option(40)" }],
            "customTimers": [{ "id": 10000, "timer": "timer_observe_boolean(function() return option(40) end)" }],
            "destination": [{ "id": "button", "timer": "41", "dst": [{ "time": 0 }] }]
        }"#,
    );
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document should load");
    assert_eq!(skin.warnings, Vec::<String>::new());

    let def = &skin.def;
    assert!(matches!(def.image[0].timer, Some(PropertyRef::Func(_))));
    assert!(matches!(def.image[0].act, Some(EventRef::Lua(_))));
    assert!(matches!(def.value[0].value, Some(PropertyRef::Func(_))));
    assert_eq!(def.value[1].value, Some(PropertyRef::Name("folder_max".to_owned())), "a number the reference knows by name is that number");
    assert!(matches!(def.floatvalue[0].value, Some(PropertyRef::Func(_))));
    assert!(matches!(def.text[0].value, Some(PropertyRef::Func(_))));
    assert!(matches!(def.text[0].event, Some(StringWriterRef::Lua(_))));
    assert!(matches!(def.slider[0].value, Some(PropertyRef::Func(_))));
    assert!(matches!(def.slider[0].event, Some(FloatWriterRef::Lua(_))));
    assert!(matches!(def.graph[0].value, Some(PropertyRef::Func(_))));
    assert!(matches!(def.custom_events[0].action, Some(EventRef::Lua(_))));
    assert!(matches!(def.custom_events[0].condition, Some(PropertyRef::Func(_))));
    assert!(matches!(def.custom_timers[0].timer, Some(PropertyRef::Func(_))));
    assert!(matches!(track(&skin, "button").timer, Some(TimerRef::Lua(_))), "a timer has no names, so even a string of digits is a script");
    assert_eq!(skin.runtime().expect("the document has an interpreter").function_count(), 13);
}

#[cfg(not(feature = "lua"))]
#[test]
fn a_build_without_lua_refuses_a_document_that_needs_it_rather_than_reading_it_as_false() {
    let scratch = Scratch::new("no-lua");
    let path = scratch.write("skin.json", r#"{ "type": 5, "destination": [{ "id": "gated", "op": ["number(10) > 0"], "dst": [{ "time": 0 }] }] }"#);
    let user = SkinUserConfig::default();
    let outcome = load_skin(&path, seeded(scratch.path(), &user));

    assert!(matches!(outcome, Err(SkinError::LuaUnavailable)), "a script this build cannot evaluate must fail loudly, got {outcome:?}");
}

#[cfg(feature = "lua")]
#[test]
fn a_timer_script_that_cannot_be_tried_costs_the_object_its_timer_and_says_so() {
    let scratch = Scratch::new("script-timer");
    let path = scratch.write("skin.json", r#"{ "type": 5, "destination": [{ "id": "timed", "timer": "no_such_function(10)", "dst": [{ "time": 0 }] }] }"#);
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("the document should load");

    assert!(track(&skin, "timed").timer.is_none(), "a timer script is called once as it is compiled, and one that raises names no timer");
    assert!(skin.warnings[0].contains("no_such_function(10)"), "warned {:?}", skin.warnings[0]);
}

#[test]
fn the_relative_flag_is_off_unless_the_screen_asks_for_it() {
    let mut skin = load_minimal(&SkinUserConfig::default());
    let counter = destination(r#"{ "id": "judge-count", "dst": [{ "time": 0 }] }"#);

    assert!(!track(&skin, "frame").relative, "a document has no field for it");
    assert!(!skin.build_track(&counter, false).expect("builds").expect("its conditions hold").relative);
    assert!(skin.build_track(&counter, true).expect("builds").expect("its conditions hold").relative, "the play screen sets it on judge counts alone");
}

#[test]
fn a_players_choice_the_row_does_not_offer_falls_back_to_the_documents_default() {
    let row = two_item_row(Some("off"));
    assert_eq!(selected_option(&row, Some(PANEL_ON)), PANEL_ON, "a choice the row offers is taken");
    assert_eq!(selected_option(&row, Some(404)), PANEL_OFF, "a choice it does not offer falls back to `def`");
}

#[test]
fn a_row_with_no_default_falls_back_to_its_first_item() {
    assert_eq!(selected_option(&two_item_row(None), None), PANEL_ON);
    assert_eq!(selected_option(&two_item_row(Some("nothing named this")), None), PANEL_ON);
}

#[test]
fn a_row_with_no_items_selects_the_random_value() {
    let empty = PropertyDef { category: String::new(), name: "Empty".to_owned(), item: Vec::new(), def: None };
    assert_eq!(selected_option(&empty, None), OPTION_RANDOM_VALUE);
}

#[test]
fn every_row_contributes_the_option_it_is_switched_to() {
    let rows = vec![two_item_row(Some("off"))];
    let mut chosen = BTreeMap::new();
    assert_eq!(enabled_options(&rows, &chosen).into_iter().collect::<Vec<i32>>(), vec![PANEL_OFF]);

    chosen.insert("Panel".to_owned(), PANEL_ON);
    assert_eq!(enabled_options(&rows, &chosen).into_iter().collect::<Vec<i32>>(), vec![PANEL_ON]);
}

#[test]
fn an_include_that_nests_deeper_than_the_limit_is_dropped_with_a_warning() {
    let scratch = Scratch::new("include-cycle");
    scratch.write("part.json", r#"{ "include": "part.json" }"#);
    let path = scratch.write("skin.json", r#"{ "type": 5, "destination": [{ "include": "part.json" }] }"#);
    let user = SkinUserConfig::default();
    let skin = load_skin(&path, seeded(scratch.path(), &user)).expect("a circular include must not fail the document");

    assert!(skin.destinations.is_empty());
    assert!(skin.warnings[0].contains(&MAX_INCLUDE_DEPTH.to_string()), "warned {:?}", skin.warnings[0]);
}

#[test]
fn an_include_that_leaves_the_root_is_dropped_with_a_warning() {
    let scratch = Scratch::new("include-escape");
    let root = scratch.path().join("root");
    std::fs::create_dir_all(&root).expect("the root should be creatable");
    std::fs::write(scratch.path().join("outside.json"), r#"{ "id": "smuggled" }"#).expect("the outside file should be writable");
    let path = root.join("skin.json");
    std::fs::write(&path, r#"{ "type": 5, "destination": [{ "include": "../outside.json" }] }"#).expect("the document should be writable");
    let user = SkinUserConfig::default();

    let skin = load_skin(&path, seeded(&root, &user)).expect("an include outside the root must not fail the document");
    assert!(skin.destinations.is_empty(), "an include outside the root brings nothing in");
    assert!(skin.warnings[0].contains("resolves outside the skin root"), "warned {:?}", skin.warnings[0]);
}

#[test]
fn an_included_file_is_held_to_the_same_ceiling_as_the_document() {
    let scratch = Scratch::new("include-ceiling");
    let padding = "x".repeat(400);
    scratch.write("part.json", &format!("{{ \"id\": \"{padding}\" }}"));
    let path = scratch.write("skin.json", r#"{ "type": 5, "destination": [{ "include": "part.json" }] }"#);
    let user = SkinUserConfig::default();
    let mut options = seeded(scratch.path(), &user);
    options.max_document_bytes = 200;

    let skin = load_skin(&path, options).expect("the document itself is under the ceiling");
    assert!(skin.destinations.is_empty());
    assert!(skin.warnings[0].contains("byte limit"), "an oversized include is refused like any other document: {:?}", skin.warnings[0]);
}

#[test]
fn the_stretch_ids_a_document_writes_name_the_modes_the_reference_declares() {
    assert_eq!(StretchKind::from_id(-1), StretchKind::Stretch, "an unset stretch fills the rectangle");
    assert_eq!(StretchKind::from_id(0), StretchKind::Stretch);
    assert_eq!(StretchKind::from_id(1), StretchKind::FitInner);
    assert_eq!(StretchKind::from_id(2), StretchKind::FitOuter);
    assert_eq!(StretchKind::from_id(3), StretchKind::FitOuterTrimmed);
    assert_eq!(StretchKind::from_id(4), StretchKind::FitWidth);
    assert_eq!(StretchKind::from_id(5), StretchKind::FitWidthTrimmed);
    assert_eq!(StretchKind::from_id(6), StretchKind::FitHeight);
    assert_eq!(StretchKind::from_id(7), StretchKind::FitHeightTrimmed);
    assert_eq!(StretchKind::from_id(8), StretchKind::NoExpanding);
    assert_eq!(StretchKind::from_id(9), StretchKind::NoResize);
    assert_eq!(StretchKind::from_id(10), StretchKind::NoResizeTrimmed);
    assert_eq!(StretchKind::from_id(404), StretchKind::Stretch, "an id nobody declares fills the rectangle too");
}

/// A wide rectangle over a square source, where the horizontal scale (2) is the larger one.
const WIDE_RECT: SkinRect = SkinRect::new(0.0, 0.0, 200.0, 100.0);

/// The square source [`WIDE_RECT`] is fitted with, set away from its texture's corner.
const SQUARE_SOURCE: SkinRect = SkinRect::new(10.0, 20.0, 100.0, 100.0);

/// A tall rectangle over a wider source, where the vertical scale (3) is larger than the horizontal
/// one (1.5).
const TALL_RECT: SkinRect = SkinRect::new(10.0, 20.0, 90.0, 150.0);

/// The source [`TALL_RECT`] is fitted with.
const LANDSCAPE_SOURCE: SkinRect = SkinRect::new(4.0, 6.0, 60.0, 50.0);

/// A rectangle smaller than its source on both axes.
const SMALL_RECT: SkinRect = SkinRect::new(0.0, 0.0, 40.0, 30.0);

/// The source [`SMALL_RECT`] is fitted with, twice as wide and five thirds as tall.
const LARGE_SOURCE: SkinRect = SkinRect::new(3.0, 5.0, 80.0, 50.0);

#[test]
fn mode_0_stretch_fills_the_rectangle_with_the_whole_source() {
    assert_eq!(stretch_rect(StretchKind::Stretch, WIDE_RECT, SQUARE_SOURCE), (WIDE_RECT, SQUARE_SOURCE));
    assert_eq!(stretch_rect(StretchKind::Stretch, SMALL_RECT, LARGE_SOURCE), (SMALL_RECT, LARGE_SOURCE));
}

#[test]
fn mode_1_fit_inner_letterboxes_on_the_axis_with_room_to_spare() {
    assert_eq!(
        stretch_rect(StretchKind::FitInner, WIDE_RECT, SQUARE_SOURCE),
        (SkinRect::new(50.0, 0.0, 100.0, 100.0), SQUARE_SOURCE),
        "scaleX 2 > scaleY 1, so `fitWidth(100 * 1)`"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitInner, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(10.0, 57.5, 90.0, 75.0), LANDSCAPE_SOURCE),
        "scaleX 1.5 <= scaleY 3, so `fitHeight(50 * 1.5)` about the centre y of 95"
    );
}

#[test]
fn mode_2_fit_outer_overflows_on_the_axis_that_is_short() {
    assert_eq!(
        stretch_rect(StretchKind::FitOuter, WIDE_RECT, SQUARE_SOURCE),
        (SkinRect::new(0.0, -50.0, 200.0, 200.0), SQUARE_SOURCE),
        "scaleX 2 >= scaleY 1, so `fitHeight(100 * 2)`"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitOuter, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(-35.0, 20.0, 180.0, 150.0), LANDSCAPE_SOURCE),
        "scaleX 1.5 < scaleY 3, so `fitWidth(60 * 3)` about the centre x of 55"
    );
}

#[test]
fn mode_3_fit_outer_trimmed_cuts_the_overflow_out_of_the_source() {
    assert_eq!(
        stretch_rect(StretchKind::FitOuterTrimmed, WIDE_RECT, SQUARE_SOURCE),
        (WIDE_RECT, SkinRect::new(10.0, 45.0, 100.0, 50.0)),
        "`fitHeightTrimmed(2)`: 100 / 2 = 50 source rows about the source's centre row 70"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitOuterTrimmed, TALL_RECT, LANDSCAPE_SOURCE),
        (TALL_RECT, SkinRect::new(19.0, 6.0, 30.0, 50.0)),
        "`fitWidthTrimmed(3)`: 90 / 3 = 30 source columns about the source's centre column 34"
    );
}

#[test]
fn mode_4_fit_width_matches_the_width_and_lets_the_height_follow() {
    assert_eq!(
        stretch_rect(StretchKind::FitWidth, WIDE_RECT, SQUARE_SOURCE),
        (SkinRect::new(0.0, -50.0, 200.0, 200.0), SQUARE_SOURCE),
        "`fitHeight(100 * 200 / 100)`"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitWidth, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(10.0, 57.5, 90.0, 75.0), LANDSCAPE_SOURCE),
        "`fitHeight(50 * 90 / 60)`"
    );
}

#[test]
fn mode_5_fit_width_trimmed_cuts_a_too_tall_image_and_centres_a_short_one() {
    assert_eq!(
        stretch_rect(StretchKind::FitWidthTrimmed, WIDE_RECT, SQUARE_SOURCE),
        (WIDE_RECT, SkinRect::new(10.0, 45.0, 100.0, 50.0)),
        "at scale 2 the image is 200 tall in a 100 tall rectangle, so half its rows are read"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitWidthTrimmed, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(10.0, 57.5, 90.0, 75.0), LANDSCAPE_SOURCE),
        "at scale 1.5 the image is 75 tall in a 150 tall rectangle, so it is centred untrimmed"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitWidthTrimmed, SkinRect::new(0.0, 0.0, 30.0, 20.0), SkinRect::new(0.0, 7.0, 40.0, 45.0)),
        (SkinRect::new(0.0, 0.0, 30.0, 20.0), SkinRect::new(0.0, 16.0, 40.0, 26.0)),
        "at scale 0.75 the 20 rows shown are 26.67 source rows from 16.17, and both land on whole pixels the way `(int)` does"
    );
}

#[test]
fn mode_6_fit_height_matches_the_height_and_lets_the_width_follow() {
    assert_eq!(
        stretch_rect(StretchKind::FitHeight, WIDE_RECT, SQUARE_SOURCE),
        (SkinRect::new(50.0, 0.0, 100.0, 100.0), SQUARE_SOURCE),
        "`fitWidth(100 * 100 / 100)`"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitHeight, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(-35.0, 20.0, 180.0, 150.0), LANDSCAPE_SOURCE),
        "`fitWidth(60 * 150 / 50)`"
    );
}

#[test]
fn mode_7_fit_height_trimmed_cuts_a_too_wide_image_and_centres_a_narrow_one() {
    assert_eq!(
        stretch_rect(StretchKind::FitHeightTrimmed, WIDE_RECT, SQUARE_SOURCE),
        (SkinRect::new(50.0, 0.0, 100.0, 100.0), SQUARE_SOURCE),
        "at scale 1 the image is 100 wide in a 200 wide rectangle, so it is centred untrimmed"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitHeightTrimmed, TALL_RECT, LANDSCAPE_SOURCE),
        (TALL_RECT, SkinRect::new(19.0, 6.0, 30.0, 50.0)),
        "at scale 3 the image is 180 wide in a 90 wide rectangle, so half its columns are read"
    );
    assert_eq!(
        stretch_rect(StretchKind::FitHeightTrimmed, SkinRect::new(0.0, 0.0, 20.0, 30.0), SkinRect::new(7.0, 0.0, 45.0, 40.0)),
        (SkinRect::new(0.0, 0.0, 20.0, 30.0), SkinRect::new(16.0, 0.0, 26.0, 40.0)),
        "the horizontal twin of the truncation in mode 5"
    );
}

#[test]
fn mode_8_no_expanding_shrinks_to_fit_but_never_enlarges() {
    assert_eq!(
        stretch_rect(StretchKind::NoExpanding, SMALL_RECT, LARGE_SOURCE),
        (SkinRect::new(0.0, 2.5, 40.0, 25.0), LARGE_SOURCE),
        "min(1, 40 / 80, 30 / 50) = 0.5, so the image is drawn at 40 x 25"
    );
    assert_eq!(
        stretch_rect(StretchKind::NoExpanding, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(25.0, 70.0, 60.0, 50.0), LANDSCAPE_SOURCE),
        "min(1, 1.5, 3) = 1, so a rectangle with room to spare draws the image at its own size"
    );
}

#[test]
fn mode_9_no_resize_draws_the_source_at_its_own_size_about_the_centre() {
    assert_eq!(stretch_rect(StretchKind::NoResize, WIDE_RECT, SQUARE_SOURCE), (SkinRect::new(50.0, 0.0, 100.0, 100.0), SQUARE_SOURCE));
    assert_eq!(
        stretch_rect(StretchKind::NoResize, SMALL_RECT, LARGE_SOURCE),
        (SkinRect::new(-20.0, -10.0, 80.0, 50.0), LARGE_SOURCE),
        "a source larger than the rectangle spills over every edge"
    );
}

#[test]
fn mode_10_no_resize_trimmed_cuts_what_mode_9_would_spill() {
    assert_eq!(
        stretch_rect(StretchKind::NoResizeTrimmed, SMALL_RECT, LARGE_SOURCE),
        (SMALL_RECT, SkinRect::new(23.0, 15.0, 40.0, 30.0)),
        "40 columns about the centre column 43 and 30 rows about the centre row 30"
    );
    assert_eq!(
        stretch_rect(StretchKind::NoResizeTrimmed, TALL_RECT, LANDSCAPE_SOURCE),
        (SkinRect::new(25.0, 70.0, 60.0, 50.0), LANDSCAPE_SOURCE),
        "a source that fits is centred untrimmed, exactly as mode 9 has it"
    );
    assert_eq!(
        stretch_rect(StretchKind::NoResizeTrimmed, SkinRect::new(0.0, 0.0, 40.0, 150.0), LARGE_SOURCE),
        (SkinRect::new(0.0, 50.0, 40.0, 50.0), SkinRect::new(23.0, 5.0, 40.0, 50.0)),
        "each axis is settled on its own: the width is trimmed, the height is centred"
    );
}

#[test]
fn a_source_with_no_area_leaves_the_rectangle_alone() {
    let rect = SkinRect::new(0.0, 0.0, 200.0, 100.0);
    for kind in (0..=10).map(StretchKind::from_id) {
        let no_width = SkinRect::new(0.0, 0.0, 0.0, 100.0);
        let no_height = SkinRect::new(0.0, 0.0, 100.0, 0.0);
        assert_eq!(stretch_rect(kind, rect, no_width), (rect, no_width), "{kind:?}");
        assert_eq!(stretch_rect(kind, rect, no_height), (rect, no_height), "{kind:?}");
    }
}

#[test]
fn filtering_is_off_until_a_filtered_image_is_actually_resized() {
    let resized = SkinRect::new(0.0, 0.0, 200.0, 100.0);
    let one_to_one = SkinRect::new(0.0, 0.0, 100.0, 100.0);
    let source = (100.0, 100.0);

    assert_eq!(filtering_for(0, resized, source), Filtering::Nearest, "a document that asks for no filtering gets none");
    assert_eq!(filtering_for(1, one_to_one, source), Filtering::Nearest, "an image drawn at its own size is not filtered");
    assert_eq!(filtering_for(1, resized, source), Filtering::Linear);
}

#[test]
fn a_players_choices_survive_a_round_trip_through_the_settings_file() {
    let mut user = SkinUserConfig { path: "skins/select/skin.json".to_owned(), ..SkinUserConfig::default() };
    user.properties.insert("Panel".to_owned(), PANEL_OFF);
    user.filepaths.insert("Gauge".to_owned(), "hard.png".to_owned());
    user.offsets.insert(20, SkinOffset { x: 4.0, y: -8.0, w: 1.5, h: 0.0, r: 90.0, a: 0.5 });

    let text = serde_json::to_string(&user).expect("the choices should serialise");
    let restored: SkinUserConfig = serde_json::from_str(&text).expect("the choices should deserialise");

    assert_eq!(restored.path, user.path);
    assert_eq!(restored.properties, user.properties);
    assert_eq!(restored.filepaths, user.filepaths);
    assert_eq!(restored.offsets, user.offsets);
}

#[test]
fn a_stored_nudge_reaches_the_interpolator_through_the_offset_source() {
    let mut user = SkinUserConfig::default();
    user.offsets.insert(20, SkinOffset { x: 4.0, y: -8.0, ..SkinOffset::default() });

    assert_eq!(user.offset(20), Some(SkinOffset { x: 4.0, y: -8.0, ..SkinOffset::default() }));
    assert_eq!(user.offset(21), None, "a slot nobody nudged offers nothing");
}

/// A list nested inside a list is still a chain of guarded clauses picking one record, because that
/// is what an element of a list is read as. The fixture next door writes its variants that way.
#[test]
fn a_list_nested_in_a_list_still_picks_one_clause() {
    let skin = load_minimal(&SkinUserConfig::default());
    let ids: Vec<&str> = skin.destinations.iter().map(|named| named.id.as_str()).collect();
    assert!(ids.contains(&"panel-on"), "the satisfied clause is the one that survives: {ids:?}");
    assert!(!ids.contains(&"panel-off"), "and only one of them does: {ids:?}");
}
