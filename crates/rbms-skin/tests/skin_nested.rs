//! The destinations a document nests inside its repeating objects.
//!
//! The rules asserted here are the reference implementation's -- a note set's bar lines, a
//! judgement pop-up's images and counts, a song wheel's per-slot objects, and the relative flag the
//! play loader sets on judgement counts alone.

use std::path::{Path, PathBuf};

use rbms_model::Mode;
use rbms_skin::loader::{LoadedSkin, NamedTrack, SkinLoadOptions, SkinUserConfig, load_skin};

/// A seed every wildcard test pins, so a draw is the same on every machine.
const TEST_SEED: u64 = 7;

/// How many lanes the play fixture's note set covers: a scratch and seven keys.
const PLAY_LANES: usize = 8;

/// How many judgements the play fixture's pop-up carries an image and a count for.
const JUDGEMENTS: usize = 6;

/// How many slots the browser fixture's song wheel draws.
const WHEEL_SLOTS: usize = 5;

/// The fixtures directory this file reads from.
fn nested_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("nested")
}

/// Options over the nested fixtures with the wildcard draw pinned.
fn seeded<'a>(root: &'a Path, user: &'a SkinUserConfig, mode: Mode) -> SkinLoadOptions<'a> {
    let mut options = SkinLoadOptions::new(root, user, mode);
    options.rng_seed = Some(TEST_SEED);
    options
}

/// One of the nested fixtures, loaded with the choices a player has made.
fn load_fixture(name: &str, mode: Mode, user: &SkinUserConfig) -> LoadedSkin {
    let root = nested_root();
    load_skin(&root.join(name), seeded(&root, user, mode)).unwrap_or_else(|error| panic!("the {name} fixture should load, got {error}"))
}

/// The play fixture, whose note set arrives through an include.
fn load_play() -> LoadedSkin {
    load_fixture("play.json5", Mode::BEAT_7K, &SkinUserConfig::default())
}

/// The browser fixture, which carries the song wheel.
fn load_select() -> LoadedSkin {
    load_fixture("select.json5", Mode::BEAT_7K, &SkinUserConfig::default())
}

/// The ids of one assembled list, in slot order.
fn ids(tracks: &[NamedTrack]) -> Vec<&str> {
    tracks.iter().map(|named| named.id.as_str()).collect()
}

#[test]
fn a_note_sets_nested_lists_are_assembled_even_when_the_set_arrives_through_an_include() {
    let skin = load_play();
    assert_eq!(skin.def.note.as_ref().expect("the include should have brought the note set in").dst.len(), PLAY_LANES);
    assert_eq!(ids(&skin.nested.note_group), ["bar-line", "bar-line-thin"]);
    assert_eq!(ids(&skin.nested.note_bpm), ["bpm-line"]);
    assert_eq!(ids(&skin.nested.note_stop), ["stop-line"]);
    assert_eq!(ids(&skin.nested.note_time), ["time-line"]);
    assert_eq!(skin.nested.note_group[0].track.frames.len(), 1, "a bar line the document draws keeps its keyframes");
}

#[test]
fn a_slot_the_documents_own_choices_rule_out_keeps_its_place_with_a_track_that_never_draws() {
    let skin = load_play();
    let thin = &skin.nested.note_group[1];
    assert_eq!(thin.id, "bar-line-thin", "the second bar line stays the second, whether or not it draws");
    assert!(thin.track.frames.is_empty(), "a slot whose condition cannot hold resolves to nothing");
}

#[test]
fn a_judgement_pop_up_is_keyed_by_its_object_id_and_carries_an_image_and_a_count_each() {
    let skin = load_play();
    let judge = skin.nested.judge.get("judge-1p").expect("the pop-up should be keyed by the id the document gave it");
    assert_eq!(judge.images.len(), JUDGEMENTS);
    assert_eq!(judge.numbers.len(), JUDGEMENTS);
    assert_eq!(ids(&judge.images), ["judge-pg", "judge-gr", "judge-gd", "judge-bd", "judge-pr", "judge-ms"]);
    assert_eq!(ids(&judge.numbers), ["combo-pg", "combo-gr", "combo-gd", "combo-bd", "combo-pr", "combo-ms"]);
}

#[test]
fn a_judgement_count_is_relative_and_its_image_is_not() {
    let skin = load_play();
    let judge = skin.nested.judge.get("judge-1p").expect("the play fixture should carry a pop-up");
    assert!(judge.numbers.iter().all(|named| named.track.relative), "the play loader sets the relative flag on judgement counts");
    assert!(!judge.images.iter().any(|named| named.track.relative), "and on nothing else");
}

#[test]
fn a_song_wheel_is_assembled_one_track_per_slot_across_every_list() {
    let skin = load_select();
    let wheel = skin.nested.songlist.as_ref().expect("the browser fixture should carry a song wheel");
    assert_eq!(wheel.listoff.len(), WHEEL_SLOTS);
    assert_eq!(wheel.liston.len(), WHEEL_SLOTS);
    assert_eq!(wheel.text.len(), WHEEL_SLOTS);
    assert_eq!(wheel.level.len(), WHEEL_SLOTS);
    assert_eq!(wheel.lamp.len(), WHEEL_SLOTS);
    assert_eq!(wheel.playerlamp.len(), WHEEL_SLOTS);
    assert_eq!(wheel.rivallamp.len(), WHEEL_SLOTS);
    assert_eq!(wheel.trophy.len(), WHEEL_SLOTS);
    assert_eq!(wheel.label.len(), WHEEL_SLOTS);
    assert_eq!(wheel.graph.as_ref().map(|named| named.id.as_str()), Some("bar-graph"));
}

#[test]
fn a_song_wheel_slot_keeps_the_rectangle_the_document_gave_it() {
    let skin = load_select();
    let wheel = skin.nested.songlist.as_ref().expect("the browser fixture should carry a song wheel");
    let rows: Vec<f32> = wheel.listoff.iter().map(|named| named.track.frames[0].rect.y).collect();
    assert_eq!(rows, [60.0, 100.0, 140.0, 180.0, 220.0], "the slots stay in the order and at the pitch the document wrote");
}

#[test]
fn a_document_with_no_repeating_objects_assembles_no_nested_tracks() {
    let skin = load_fixture("result.json5", Mode::BEAT_7K, &SkinUserConfig::default());
    assert!(skin.nested.note_group.is_empty());
    assert!(skin.nested.note_bpm.is_empty());
    assert!(skin.nested.note_stop.is_empty());
    assert!(skin.nested.note_time.is_empty());
    assert!(skin.nested.judge.is_empty());
    assert!(skin.nested.songlist.is_none());
}

#[test]
fn assembling_the_nested_lists_leaves_the_document_itself_intact() {
    let skin = load_select();
    let wheel = skin.def.songlist.as_ref().expect("the song wheel record should still be on the document");
    assert_eq!(wheel.center, 2);
    assert_eq!(wheel.clickable, [0, 1, 2, 3, 4]);
    assert_eq!(skin.def.judge.len(), 0, "the browser fixture declares no pop-up");
    let play = load_play();
    assert_eq!(play.def.judge.len(), 1, "and the play fixture's pop-up record is put back after assembly");
}
