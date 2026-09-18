//! The parts of a document that are fired rather than drawn: the timers it computes for itself,
//! the events it declares, the objects a click acts on and the pointer rectangle that gates one.
//!
//! The shapes are the reference implementation's (`JsonSkin.CustomTimer`, `CustomEvent`, and the
//! `act`/`click` pair `JsonSkinObjectLoader` reads off an `image` or an `imageset`). What this file
//! holds is that they survive the loader: a field read into the wrong form, or dropped on the way
//! through the guarded-clause pass, would leave a document's buttons silently dead.

use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::dst::MouseRect;
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::model::PropertyRef;

/// A seed the wildcard draw is pinned to, so a load is the same on every machine.
const TEST_SEED: u64 = 7;

/// The document's own timers, in the order the fixture declares them.
const TIMER_EXPRESSION: i32 = 10_001;
const TIMER_FROM_ID: i32 = 10_002;
const TIMER_PASSIVE: i32 = 10_003;

/// The document's own events, in the order the fixture declares them.
const EVENT_CONDITIONAL: i32 = 5_000;
const EVENT_BUILTIN_ACTION: i32 = 5_001;
const EVENT_CHAINED: i32 = 5_002;

/// The reference's number for the browser's sort order, which the fixture's first button names.
const EVENT_SORT: i32 = 12;

/// Its number for the hi-speed row, which the fixture's strip names.
const EVENT_HISPEED: i32 = 57;

/// Its number for the gauge row, which the fixture's image set names.
const EVENT_GAUGE: i32 = 40;

/// The built-in timer the fixture's second custom timer is written as a plain id of.
const TIMER_PLAY: i32 = 41;

/// The interval the fixture's conditional event waits out between firings.
const MIN_INTERVAL_MS: i32 = 250;

/// The fixtures directory this file reads from.
fn events_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("events")
}

/// The fixture document, loaded with a player who has chosen nothing.
fn load_events() -> LoadedSkin {
    let root = events_root();
    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), ..SkinLoadOptions::new(&root, &user, Mode::BEAT_7K) };
    load_skin(&root.join("select.json5"), options).unwrap_or_else(|error| panic!("the events fixture should load, got {error}"))
}

#[test]
fn a_documents_own_timers_keep_both_forms_and_the_passive_one() {
    let skin = load_events();
    let timers = &skin.def.custom_timers;
    assert_eq!(timers.len(), 3, "every declared timer survived the load");
    assert_eq!(timers.iter().map(|timer| timer.id).collect::<Vec<_>>(), vec![TIMER_EXPRESSION, TIMER_FROM_ID, TIMER_PASSIVE]);

    assert_eq!(
        timers[0].timer.as_ref().and_then(PropertyRef::expr),
        Some("skin.boolean(900) and skin.time() or nil"),
        "a timer written as an expression is kept as its source, to be compiled once"
    );
    assert_eq!(timers[1].timer.as_ref().and_then(PropertyRef::id), Some(TIMER_PLAY), "and one written as a number is kept as the id it names");
    assert!(timers[2].timer.is_none(), "a timer with no value at all is passive: only an event ever moves it");
}

#[test]
fn a_documents_own_events_keep_their_action_condition_and_interval() {
    let skin = load_events();
    let events = &skin.def.custom_events;
    assert_eq!(events.len(), 3);
    assert_eq!(events.iter().map(|event| event.id).collect::<Vec<_>>(), vec![EVENT_CONDITIONAL, EVENT_BUILTIN_ACTION, EVENT_CHAINED]);

    let conditional = &events[0];
    assert_eq!(conditional.action.as_ref().and_then(PropertyRef::expr), Some("skin.set_timer(10003)"), "an action written as an expression is its source");
    assert_eq!(conditional.condition.as_ref().and_then(PropertyRef::expr), Some("skin.number(70) > 0"), "and so is its condition");
    assert_eq!(conditional.min_interval, MIN_INTERVAL_MS, "the interval is read from the camel-cased member the reference writes");

    assert_eq!(events[1].action.as_ref().and_then(PropertyRef::id), Some(EVENT_SORT), "an action written as a number is the event of that number");
    assert!(events[1].condition.is_none(), "an event with no condition is only ever fired by something naming it");
    assert_eq!(events[1].min_interval, 0, "and a document that names no interval waits none");

    assert_eq!(events[2].action.as_ref().and_then(PropertyRef::id), Some(EVENT_BUILTIN_ACTION), "one event may name another as its action");
    assert_eq!(events[2].condition.as_ref().and_then(PropertyRef::id), Some(900), "a condition written as a number is the option of that number");
}

#[test]
fn an_image_and_an_image_set_each_carry_the_event_a_click_runs() {
    let skin = load_events();
    let image = |id: &str| skin.def.image.iter().find(|image| image.id == id).unwrap_or_else(|| panic!("the fixture declares {id}")).clone();

    let sort = image("sort-button");
    assert_eq!(sort.act.as_ref().and_then(PropertyRef::id), Some(EVENT_SORT), "a numbered act is the reference's own event");
    assert_eq!(sort.click, 0, "and the click kind is read beside it");

    assert_eq!(image("speed-strip").click, 2, "the kind that splits the rectangle left and right");
    assert_eq!(image("speed-strip").act.as_ref().and_then(PropertyRef::id), Some(EVENT_HISPEED));

    let own = image("own-button");
    assert_eq!(own.act.as_ref().and_then(PropertyRef::expr), Some("skin.set_timer(10003)"), "an act written as a string is an expression, not an event name");
    assert_eq!(own.click, 3, "the kind that splits it bottom and top");

    let plain = image("plain");
    assert!(plain.act.is_none(), "an object with no act answers no click");
    assert_eq!(plain.click, 0, "and its click kind is the default rather than a missing field");

    let set = skin.def.imageset.first().expect("the fixture declares one image set");
    assert_eq!(set.act.as_ref().and_then(PropertyRef::id), Some(EVENT_GAUGE), "an image set carries the same pair an image does");
    assert_eq!(set.click, 1, "the kind that turns the step around");
}

#[test]
fn a_pointer_rectangle_reaches_the_track_that_is_gated_on_it() {
    let skin = load_events();
    let track = |id: &str| skin.destinations.iter().find(|named| named.id == id).unwrap_or_else(|| panic!("the fixture draws {id}")).track.mouse_rect;

    assert_eq!(
        track("plain"),
        Some(MouseRect { x: -4.0, y: -4.0, w: 48.0, h: 20.0 }),
        "the rectangle is carried into the assembled track, in the document's own units and relative to the object's own region"
    );
    assert!(track("sort-button").is_none(), "a destination that names none is drawn wherever the pointer is");
}
