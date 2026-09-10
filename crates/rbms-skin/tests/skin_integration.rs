//! The seams between the property registry, the loader and the interpolator.
//!
//! Each wave of Phase E built one of those three against the spec's description of the others, so
//! the contracts they meet at are asserted here rather than in any one of their own test files. The
//! central claim is that a screen writes *one* state implementation: the same value answers the
//! registry, gates a draw and backs a Lua expression, with no adapter in between.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::dst::{DrawCondition, DrawStateSource, Keyframe, OffsetSource, SkinColor, SkinOffset, SkinRect, draw_conditions_from_ops, prepare};
use rbms_skin::loader::{SkinLoadOptions, SkinUserConfig, StretchKind, every_option_known, load_skin};
use rbms_skin::model::Destination;
use rbms_skin::property::{DefaultState, MAPPINGS, PropertyKind, SkinStateSource, UNMAPPED_CLOCK_MS, UnmappedLog, source_of};
use rbms_skin::timer::{TimerId, TimerState};

/// The clock every frame in this file is drawn against.
const FRAME_NOW_MS: i64 = 1_234;

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

/// A boolean id no document and no registry declares, standing in for an option this build has yet
/// to implement.
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
    fn boolean(&self, id: i32) -> bool {
        if id < 0 { !self.booleans.contains(&-id) } else { self.booleans.contains(&id) }
    }
}

impl SkinStateSource for PlayerState {
    fn integer(&self, id: i32) -> i32 {
        self.integers.get(&id).copied().unwrap_or_default()
    }

    fn float(&self, _id: i32) -> f32 {
        0.0
    }

    fn string(&self, _id: i32) -> &str {
        ""
    }

    fn timer(&self, id: i32) -> Option<i64> {
        self.timers.get(&id).copied()
    }

    fn now_ms(&self) -> i64 {
        self.now
    }
}

/// A state with the sample values every test below reads.
fn sample_state() -> PlayerState {
    let mut state = PlayerState { now: FRAME_NOW_MS, ..PlayerState::default() };
    state.booleans.insert(DECLARED_OPTION);
    state.integers.insert(SAMPLE_NUMBER, SAMPLE_NUMBER_VALUE);
    state.offsets.insert(SAMPLE_OFFSET, SkinOffset { x: SAMPLE_OFFSET_X, ..SkinOffset::default() });
    state
}

/// A one-keyframe track that draws a fixed rectangle whenever its conditions hold.
fn sample_track() -> rbms_skin::dst::DestinationTrack {
    rbms_skin::dst::DestinationTrack {
        timer: Some(TRACK_TIMER),
        frames: vec![Keyframe {
            time_ms: 0,
            rect: SkinRect::new(0.0, 0.0, 10.0, 10.0),
            clip: None,
            acc: rbms_skin::dst::Acc::Linear,
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

/// Loads the minimal fixture under one option predicate, with its wildcard draws pinned.
fn load_minimal(known_option: fn(i32) -> bool) -> rbms_skin::loader::LoadedSkin {
    let user = SkinUserConfig::default();
    let root = minimal_root();
    let mut options = SkinLoadOptions::new(&root, &user, Mode::BEAT_7K);
    options.rng_seed = Some(TEST_SEED);
    options.known_option = known_option;
    load_skin(&root.join("skin.json"), options).expect("the minimal fixture should load")
}

#[test]
fn one_state_implementation_serves_the_registry_and_the_interpolator() {
    let state = sample_state();

    assert_eq!(state.integer(SAMPLE_NUMBER), SAMPLE_NUMBER_VALUE, "the registry reads it directly");

    let gating: &dyn DrawStateSource = &state;
    assert!(gating.boolean(DECLARED_OPTION), "the same value upcasts to what the interpolator gates on");

    let resolved = prepare(&sample_track(), FRAME_NOW_MS, &running_timers(), gating, None, (0.0, 0.0), None);
    assert!(resolved.is_some(), "a track with no conditions draws against the upcast state");
}

#[test]
fn the_registry_trait_carries_every_accessor_the_lua_whitelist_delegates_to() {
    let state = sample_state();
    let source: &dyn SkinStateSource = &state;

    assert!(source.boolean(DECLARED_OPTION));
    assert_eq!(source.integer(SAMPLE_NUMBER), SAMPLE_NUMBER_VALUE);
    assert_eq!(source.float(SAMPLE_NUMBER), 0.0);
    assert_eq!(source.string(SAMPLE_NUMBER), "");
    assert_eq!(source.timer(TRACK_TIMER.get()), None);
    assert_eq!(source.now_ms(), FRAME_NOW_MS);
}

#[test]
fn the_default_state_answers_the_documented_clock() {
    assert_eq!(DefaultState.now_ms(), UNMAPPED_CLOCK_MS, "a screen with no clock yet resolves at the origin");
}

#[test]
fn an_offset_moves_a_drawn_object_through_the_same_state_value() {
    let state = sample_state();
    let mut track = sample_track();
    track.offsets.push(SAMPLE_OFFSET);

    let resolved = prepare(&track, FRAME_NOW_MS, &running_timers(), &state, None, (0.0, 0.0), None).expect("the track draws");
    assert_eq!(resolved.rect.x, SAMPLE_OFFSET_X, "the offset came from the registry implementation's supertrait");
}

#[test]
fn the_registry_membership_test_drops_into_the_loader_as_its_option_predicate() {
    let skin = load_minimal(|id| PropertyKind::Boolean.is_declared(id));
    assert!(!skin.destinations.is_empty(), "the document still assembles its destinations");
}

#[test]
fn the_generated_registry_declares_no_document_custom_option() {
    assert!(
        !PropertyKind::Boolean.is_declared(DECLARED_OPTION),
        "the fixture's customisation ids are the document's own, so the engine registry must not claim them"
    );
}

/// A document's own customisation ids gate its objects whatever the build's predicate knows, and
/// they gate them at load: the id never reaches a state source, because no state source answers a
/// number a document invented for itself.
#[test]
fn a_documents_own_options_gate_even_under_a_registry_only_predicate() {
    let ids = |skin: &rbms_skin::loader::LoadedSkin| skin.destinations.iter().map(|named| named.id.clone()).collect::<Vec<String>>();
    let strict = load_minimal(|id| PropertyKind::Boolean.is_declared(id));
    let permissive = load_minimal(every_option_known);

    assert_eq!(ids(&strict), ids(&permissive), "the injected predicate must not change how a document's own options gate");
    assert!(ids(&strict).contains(&"panel-on".to_owned()), "the chosen variant is drawn: {:?}", ids(&strict));
    assert!(!ids(&strict).contains(&"panel-off".to_owned()), "and the one nobody chose is not: {:?}", ids(&strict));
    assert!(
        strict.destinations.iter().all(|named| !named.track.draw_conditions.contains(&DrawCondition::Option(DECLARED_OPTION))),
        "a settled customisation id must not be left for a frame to ask about"
    );
}

#[test]
fn the_loader_reports_every_option_the_document_declares() {
    let skin = load_minimal(every_option_known);
    assert!(skin.declared_options.contains(&DECLARED_OPTION), "the on variant is declared");
    assert!(skin.declared_options.contains(&ALTERNATE_OPTION), "so is the off variant, though the player has not chosen it");
    assert!(skin.enabled_options.contains(&DECLARED_OPTION), "and only the chosen one is enabled");
    assert!(!skin.enabled_options.contains(&ALTERNATE_OPTION));
}

#[test]
fn the_default_predicate_treats_every_option_as_implemented() {
    assert!(every_option_known(UNDECLARED_OPTION), "a caller with no registry must not hide objects");
}

#[test]
fn an_option_the_registry_does_not_declare_leaves_its_object_visible() {
    let declared = draw_conditions_from_ops(&[REGISTRY_OPTION], |id| PropertyKind::Boolean.is_declared(id));
    assert_eq!(declared, vec![DrawCondition::Option(REGISTRY_OPTION)], "a declared op becomes a condition");

    let undeclared = draw_conditions_from_ops(&[UNDECLARED_OPTION], |id| PropertyKind::Boolean.is_declared(id));
    assert!(undeclared.is_empty(), "an op no property implements is dropped rather than read as false");

    let mut track = sample_track();
    track.draw_conditions = undeclared;
    let state = sample_state();
    assert!(
        prepare(&track, FRAME_NOW_MS, &running_timers(), &state, None, (0.0, 0.0), None).is_some(),
        "so the object stays on screen instead of vanishing on an unimplemented build"
    );
}

#[test]
fn a_stretch_the_renderer_does_not_implement_is_reported_rather_than_hidden() {
    let track = sample_track();
    assert!(StretchKind::from_id(track.stretch).is_supported(), "the unspecified default is drawable");

    let unsupported: Vec<StretchKind> = (0..16).map(StretchKind::from_id).filter(|kind| !kind.is_supported()).collect();
    assert!(!unsupported.is_empty(), "the classifier names at least one mode the renderer falls back on");
}

#[test]
fn only_a_judge_count_object_is_built_relative() {
    let mut skin = load_minimal(every_option_known);

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

/// The seam between the registry and the Lua evaluator, which only exists in a build with the
/// feature. The evaluator takes the registry's own trait rather than a second one of its own name,
/// so a screen hands the same value to both without an adapter.
#[cfg(feature = "lua")]
mod with_lua {
    use super::*;
    use rbms_skin::dst::{DrawCondition, LuaDrawEval};
    use rbms_skin::loader::Budget;
    use rbms_skin::lua::LuaSandbox;

    /// A sandbox rooted at the fixture directory, with the shipping budget.
    fn sandbox() -> LuaSandbox {
        LuaSandbox::new(&minimal_root(), Budget::default()).expect("the sandbox should build")
    }

    #[test]
    fn the_evaluator_takes_the_registry_trait_itself() {
        let state = sample_state();
        let source: &dyn SkinStateSource = &state;
        let sandbox = sandbox();

        let number = sandbox.eval_int("skin.number(110)", source).expect("the expression should run");
        assert_eq!(number, SAMPLE_NUMBER_VALUE, "the registry implementation backs skin.number with no adapter in between");
    }

    #[test]
    fn one_state_value_answers_the_registry_the_interpolator_and_an_expression() {
        let state = sample_state();
        let sandbox = sandbox();
        let expression = sandbox.compile("skin.boolean(901)").expect("the expression should compile");

        let mut track = sample_track();
        track.draw_conditions = vec![DrawCondition::Lua(expression)];

        let frame = sandbox.frame(&state);
        let resolved = prepare(&track, FRAME_NOW_MS, &running_timers(), &state, Some(&frame), (0.0, 0.0), None);
        assert!(resolved.is_some(), "the same value gated the draw and answered the expression");
        assert_eq!(frame.calls(), 1, "the expression was evaluated once for the frame");
    }

    #[test]
    fn a_false_expression_hides_its_object_rather_than_failing_the_frame() {
        let state = PlayerState { now: FRAME_NOW_MS, ..PlayerState::default() };
        let sandbox = sandbox();
        let expression = sandbox.compile("skin.boolean(901)").expect("the expression should compile");

        let mut track = sample_track();
        track.draw_conditions = vec![DrawCondition::Lua(expression)];

        let frame = sandbox.frame(&state);
        assert!(prepare(&track, FRAME_NOW_MS, &running_timers(), &state, Some(&frame), (0.0, 0.0), None).is_none());
        assert_eq!(frame.eval_draw(expression), Some(false), "and the evaluator reports it plainly");
    }

    #[test]
    fn the_expression_clock_is_the_clock_the_frame_is_drawn_against() {
        let state = sample_state();
        let sandbox = sandbox();

        let seen = sandbox.eval_int("skin.time()", &state as &dyn SkinStateSource).expect("the expression should run");
        assert_eq!(i64::from(seen), FRAME_NOW_MS, "skin.time() reads the registry's own clock, which is the one prepare is given for the same frame");
    }

    #[test]
    fn an_expression_still_cannot_reach_the_filesystem_through_the_registry() {
        let state = sample_state();
        let sandbox = sandbox();
        for forbidden in ["io", "os", "require", "load", "dofile"] {
            let reachable = sandbox.eval_bool(&format!("{forbidden} ~= nil"), &state as &dyn SkinStateSource).expect("the check itself should run");
            assert!(!reachable, "{forbidden} must stay out of reach whatever state is bound");
        }
    }
}
