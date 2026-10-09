//! The seams between the property registry, the loader and the interpolator.
//!
//! Each wave of Phase E built one of those three against the spec's description of the others, so
//! the contracts they meet at are asserted here rather than in any one of their own test files. The
//! central claim is that a screen writes *one* state implementation: the same value answers the
//! registry, gates a draw and backs a Lua expression, with no adapter in between.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::dst::{DrawCondition, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinOffset, SkinRect, TimerRef, draw_conditions_from_ops, prepare};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::model::Destination;
use rbms_skin::property::{DefaultState, MAPPINGS, NameSpace, PropertyKind, SkinHost, UNMAPPED_CLOCK_US, UnmappedLog, reference_implements, source_of};
use rbms_skin::timer::{TIMER_OFF, TimerId, TimerState};

/// The clock every frame in this file is drawn against, in microseconds.
const FRAME_NOW_US: i64 = 1_234_000;

/// The timer the sample track hangs its keyframes off.
const TRACK_TIMER: TimerId = TimerId(1);

/// The fixture's own customisation id, declared by its `property` block rather than by the
/// generated registry.
const DECLARED_OPTION: i32 = 901;

/// The fixture's other customisation id, which its `Panel` row turns on when the player switches
/// away from the default.
const ALTERNATE_OPTION: i32 = 902;

/// An option the generated registry declares, standing in for an engine option a screen wires up.
const REGISTRY_OPTION: i32 = rbms_skin::property::generated::boolean::OPTION_PANEL1;

/// An id no document and no registry declares: as an option, one that neither the reference nor
/// any skin's customisation rows give a meaning to.
const UNDECLARED_OPTION: i32 = 88_888;

/// An integer id the sample state answers for.
const SAMPLE_NUMBER: i32 = 110;

/// The value stored under [`SAMPLE_NUMBER`].
const SAMPLE_NUMBER_VALUE: i32 = 42;

/// The offset id the sample state moves an object by.
const SAMPLE_OFFSET: i32 = 20;

/// How far [`SAMPLE_OFFSET`] moves an object along x.
const SAMPLE_OFFSET_X: f32 = 7.0;

/// A seed that pins every wildcard draw in this file.
const TEST_SEED: u64 = 7;

/// One screen's game state, written the way a screen in `apps/rbms-player` will write it.
///
/// It implements the registry trait and nothing else: [`OffsetSource`] and [`DrawStateSource`] come
/// with it as supertraits, which is the whole point of the arrangement being tested.
#[derive(Debug, Default)]
struct PlayerState {
    booleans: BTreeSet<i32>,
    integers: BTreeMap<i32, i32>,
    offsets: BTreeMap<i32, SkinOffset>,
    timers: BTreeMap<i32, i64>,
    now: i64,
}

impl OffsetSource for PlayerState {
    fn offset(&self, id: i32) -> Option<SkinOffset> {
        self.offsets.get(&id).copied()
    }
}

impl DrawStateSource for PlayerState {
    fn boolean(&self, id: i32) -> Option<bool> {
        Some(if id < 0 { !self.booleans.contains(&-id) } else { self.booleans.contains(&id) })
    }
}

impl SkinHost for PlayerState {
    fn integer(&self, id: i32) -> i32 {
        self.integers.get(&id).copied().unwrap_or_default()
    }

    fn float(&self, _id: i32) -> f32 {
        0.0
    }

    fn text(&self, _id: i32) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn timer_us(&self, id: i32) -> i64 {
        self.timers.get(&id).copied().unwrap_or(TIMER_OFF)
    }

    fn now_us(&self) -> i64 {
        self.now
    }
}

/// A state with the sample values every test below reads.
fn sample_state() -> PlayerState {
    let mut state = PlayerState { now: FRAME_NOW_US, ..PlayerState::default() };
    state.booleans.insert(DECLARED_OPTION);
    state.integers.insert(SAMPLE_NUMBER, SAMPLE_NUMBER_VALUE);
    state.offsets.insert(SAMPLE_OFFSET, SkinOffset { x: SAMPLE_OFFSET_X, ..SkinOffset::default() });
    state
}

/// A one-keyframe track that draws a fixed rectangle whenever its conditions hold.
fn sample_track() -> rbms_skin::dst::DestinationTrack {
    rbms_skin::dst::DestinationTrack {
        timer: Some(TimerRef::Id(TRACK_TIMER)),
        frames: vec![Keyframe {
            time_ms: 0,
            rect: SkinRect::new(0.0, 0.0, 10.0, 10.0),
            clip: None,
            color: SkinColor::rgba(255, 255, 255, 255),
            angle_deg: 0.0,
        }],
        ..rbms_skin::dst::DestinationTrack::default()
    }
}

/// The timer table with the sample track's timer already running.
fn running_timers() -> TimerState {
    let mut timers = TimerState::new();
    timers.set_on(TRACK_TIMER, 0);
    timers
}

/// The minimal fixture's directory, which is also its skin root.
fn minimal_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("minimal")
}

/// Loads the minimal fixture with its wildcard draws pinned.
fn load_minimal() -> rbms_skin::loader::LoadedSkin {
    let user = SkinUserConfig::default();
    let root = minimal_root();
    let mut options = SkinLoadOptions::new(&root, &user, Mode::BEAT_7K);
    options.rng_seed = Some(TEST_SEED);
    load_skin(&root.join("skin.json"), options).expect("the minimal fixture should load")
}

#[test]
fn one_state_implementation_serves_the_registry_and_the_interpolator() {
    let state = sample_state();

    assert_eq!(state.integer(SAMPLE_NUMBER), SAMPLE_NUMBER_VALUE, "the registry reads it directly");

    let gating: &dyn DrawStateSource = &state;
    assert_eq!(gating.boolean(DECLARED_OPTION), Some(true), "the same value upcasts to what the interpolator gates on");

    let resolved = prepare(&sample_track(), FRAME_NOW_US, &running_timers(), gating, None, (0.0, 0.0), None);
    assert!(resolved.is_some(), "a track with no conditions draws against the upcast state");
}

#[test]
fn the_registry_trait_carries_every_accessor_the_lua_whitelist_delegates_to() {
    let state = sample_state();
    let source: &dyn SkinHost = &state;

    assert_eq!(source.boolean(DECLARED_OPTION), Some(true));
    assert_eq!(source.integer(SAMPLE_NUMBER), SAMPLE_NUMBER_VALUE);
    assert_eq!(source.float(SAMPLE_NUMBER), 0.0);
    assert_eq!(source.text(SAMPLE_NUMBER), "");
    assert_eq!(source.timer_us(TRACK_TIMER.get()), TIMER_OFF);
    assert_eq!(source.now_us(), FRAME_NOW_US);
}

#[test]
fn the_default_state_answers_the_documented_clock() {
    assert_eq!(DefaultState.now_us(), UNMAPPED_CLOCK_US, "a screen with no clock yet resolves at the origin");
}

#[test]
fn an_offset_moves_a_drawn_object_through_the_same_state_value() {
    let state = sample_state();
    let mut track = sample_track();
    track.offsets.push(SAMPLE_OFFSET);

    let resolved = prepare(&track, FRAME_NOW_US, &running_timers(), &state, None, (0.0, 0.0), None).expect("the track draws");
    assert_eq!(resolved.rect.x, SAMPLE_OFFSET_X, "the offset came from the registry implementation's supertrait");
}

#[test]
fn neither_the_generated_registry_nor_the_reference_claims_a_documents_custom_option() {
    assert!(
        !PropertyKind::Boolean.is_declared(DECLARED_OPTION),
        "the fixture's customisation ids are the document's own, so the engine registry must not claim them"
    );
    assert!(!reference_implements(NameSpace::Boolean, DECLARED_OPTION), "and the reference has no built-in option there to answer in their place");
    assert!(reference_implements(NameSpace::Boolean, -REGISTRY_OPTION), "while an engine option is built in under either sign");
}

/// A document's own customisation ids gate its objects at load: the id never reaches a state
/// source, because no state source answers a number a document invented for itself.
#[test]
fn a_documents_own_options_gate_at_load_and_never_reach_a_frame() {
    let skin = load_minimal();
    let ids: Vec<&str> = skin.destinations.iter().map(|named| named.id.as_str()).collect();

    assert!(ids.contains(&"panel-on"), "the chosen variant is drawn: {ids:?}");
    assert!(!ids.contains(&"panel-off"), "and the one nobody chose is not: {ids:?}");
    assert!(
        skin.destinations.iter().all(|named| !named.track.draw_conditions.contains(&DrawCondition::Option(DECLARED_OPTION))),
        "a settled customisation id must not be left for a frame to ask about"
    );
}

#[test]
fn the_loader_reports_every_option_the_document_declares() {
    let skin = load_minimal();
    assert!(skin.declared_options.contains(&DECLARED_OPTION), "the on variant is declared");
    assert!(skin.declared_options.contains(&ALTERNATE_OPTION), "so is the off variant, though the player has not chosen it");
    assert!(skin.enabled_options.contains(&DECLARED_OPTION), "and only the chosen one is enabled");
    assert!(!skin.enabled_options.contains(&ALTERNATE_OPTION));
}

/// The reference's own list of built-in options is what sorts an `op` list: an id on it is a
/// condition the state answers every frame, and an id off it is never shown to the state at all.
#[test]
fn an_op_the_reference_implements_gates_a_frame_and_one_it_does_not_never_reaches_one() {
    let builtin = |id: i32| reference_implements(NameSpace::Boolean, id);
    let lists = draw_conditions_from_ops(&[-REGISTRY_OPTION, UNDECLARED_OPTION, -UNDECLARED_OPTION], builtin);
    assert_eq!(lists.conditions, vec![DrawCondition::Option(-REGISTRY_OPTION)], "a built-in op becomes a condition, sign included");
    assert_eq!(lists.options, vec![UNDECLARED_OPTION, -UNDECLARED_OPTION], "anything else is left for the skin's own options to answer when it is prepared");

    let mut track = sample_track();
    track.draw_conditions = lists.conditions;
    let mut state = sample_state();
    assert!(
        prepare(&track, FRAME_NOW_US, &running_timers(), &state, None, (0.0, 0.0), None).is_some(),
        "the state does not hold the option, so its negation draws"
    );
    state.booleans.insert(REGISTRY_OPTION);
    assert!(prepare(&track, FRAME_NOW_US, &running_timers(), &state, None, (0.0, 0.0), None).is_none(), "and stops drawing once it does");
}

/// An id nobody gives a meaning to removes its object when the skin is prepared, whichever sign the
/// document wrote it with (`Skin.prepare`), and a state that would answer anything is not asked.
#[test]
fn an_op_nobody_answers_removes_its_object_before_any_state_is_asked() {
    let mut skin = load_minimal();
    for op in [UNDECLARED_OPTION, -UNDECLARED_OPTION] {
        let destination: Destination =
            serde_json::from_str(&format!(r#"{{ "id": "stray", "op": [{op}], "dst": [{{ "time": 0 }}] }}"#)).expect("the destination should parse");
        assert!(skin.build_track(&destination, false).expect("builds").is_none(), "op {op} names no property and no option of the skin");
    }
}

#[test]
fn only_a_judge_count_object_is_built_relative() {
    let mut skin = load_minimal();

    let destination = Destination::default();
    assert!(!skin.build_track(&destination, false).expect("builds").expect("its conditions hold").relative);
    assert!(skin.build_track(&destination, true).expect("builds").expect("its conditions hold").relative);
}

#[test]
fn every_declared_property_band_routes_to_a_state_source() {
    for mapping in MAPPINGS {
        let routed = source_of(mapping.kind, mapping.first_id);
        assert!(routed.is_some(), "{:?} id {} is declared but routes nowhere", mapping.kind, mapping.first_id);
    }
}

#[test]
fn an_id_no_mapping_covers_is_counted_once_and_never_panics() {
    let log = UnmappedLog::new();
    assert_eq!(log.check(PropertyKind::Integer, UNDECLARED_OPTION), Err(true), "the first miss is the one that warns");
    assert_eq!(log.check(PropertyKind::Integer, UNDECLARED_OPTION), Err(false), "a repeat is counted but silent");
    assert_eq!(log.misses(), 2);
    assert_eq!(log.distinct(), vec![(PropertyKind::Integer, UNDECLARED_OPTION)]);
}

/// The seam between the registry and the Lua runtime, which only exists in a build with the
/// feature. The runtime takes the registry's own trait rather than a second one of its own name, so
/// a screen hands the same value to both without an adapter.
///
/// The scripts here are the ones a JSON document writes as strings: the loader compiles them into
/// the interpreter the loaded skin owns, where the game state is published as globals.
#[cfg(feature = "lua")]
mod with_lua {
    use super::*;
    use rbms_skin::lua::LuaFnKind;

    /// A destination gated on one script, as a document would write it.
    fn gated_on(script: &str) -> Destination {
        serde_json::from_str(&format!(r#"{{ "id": "gated", "timer": {}, "op": ["{script}"], "dst": [{{ "time": 0, "w": 10, "h": 10 }}] }}"#, TRACK_TIMER.0))
            .expect("the destination should parse")
    }

    #[test]
    fn the_runtime_takes_the_registry_trait_itself() {
        let state = sample_state();
        let skin = load_minimal();
        let runtime = skin.runtime().expect("a build with Lua gives a document an interpreter");

        let number = runtime.compile(&format!("number({SAMPLE_NUMBER})"), LuaFnKind::Integer).expect("the script should compile");
        let seen = runtime.frame(&state as &dyn SkinHost, |frame| frame.call_integer(number)).expect("the frame binds");
        assert_eq!(seen, SAMPLE_NUMBER_VALUE, "the registry implementation backs `number` with no adapter in between");
    }

    #[test]
    fn one_state_value_answers_the_registry_the_interpolator_and_a_script() {
        let state = sample_state();
        let mut skin = load_minimal();
        let track = skin.build_track(&gated_on(&format!("option({DECLARED_OPTION})")), false).expect("the track builds").expect("nothing rules it out");
        assert!(matches!(track.draw_conditions.as_slice(), [DrawCondition::Function(_)]), "the script is a function condition: {:?}", track.draw_conditions);

        let runtime = skin.runtime().expect("the document has an interpreter");
        let drawn =
            runtime.frame(&state, |frame| prepare(&track, FRAME_NOW_US, &running_timers(), &state, Some(frame), (0.0, 0.0), None)).expect("the frame binds");
        assert!(drawn.is_some(), "the same value gated the draw and answered the script");
        assert!(runtime.diagnostics().function_failures.is_empty(), "{:?}", runtime.diagnostics().function_failures);
    }

    #[test]
    fn a_false_script_hides_its_object_rather_than_failing_the_frame() {
        let state = PlayerState { now: FRAME_NOW_US, ..PlayerState::default() };
        let mut skin = load_minimal();
        let track = skin.build_track(&gated_on(&format!("option({DECLARED_OPTION})")), false).expect("the track builds").expect("nothing rules it out");

        let runtime = skin.runtime().expect("the document has an interpreter");
        let drawn =
            runtime.frame(&state, |frame| prepare(&track, FRAME_NOW_US, &running_timers(), &state, Some(frame), (0.0, 0.0), None)).expect("the frame binds");
        assert!(drawn.is_none());
        assert!(prepare(&track, FRAME_NOW_US, &running_timers(), &state, None, (0.0, 0.0), None).is_none(), "and so does having nothing to ask");
    }

    #[test]
    fn the_script_clock_is_the_clock_the_frame_is_drawn_against() {
        let state = sample_state();
        let skin = load_minimal();
        let runtime = skin.runtime().expect("the document has an interpreter");

        let clock = runtime.compile("time()", LuaFnKind::Integer).expect("the script should compile");
        let seen = runtime.frame(&state, |frame| frame.call_integer(clock)).expect("the frame binds");
        assert_eq!(i64::from(seen), FRAME_NOW_US, "`time()` reads the registry's own clock, which is the one prepare is given for the same frame");
    }

    #[test]
    fn a_script_still_cannot_reach_a_file_outside_the_skin_root() {
        let state = sample_state();
        let skin = load_minimal();
        let runtime = skin.runtime().expect("the document has an interpreter");
        assert!(minimal_root().join("..").join("secret.txt").is_file(), "the file the scripts reach for exists, one folder above the skin");

        for refused in ["io.open('../secret.txt') == nil", "not pcall(dofile, '../secret.txt')", "not pcall(require, '..secret')", "os.execute == nil"] {
            let script = runtime.compile(refused, LuaFnKind::Boolean).expect("the check itself should compile");
            let held = runtime.frame(&state, |frame| frame.call_boolean(script)).expect("the frame binds");
            assert!(held, "{refused} must hold whatever state is bound");
        }
    }
}
