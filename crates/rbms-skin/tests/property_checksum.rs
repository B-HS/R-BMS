//! Guards the generated property table and the routing table built on top of it.
//!
//! The constant table is machine-extracted and committed, so nothing here re-reads the reference:
//! the tests re-derive the checksum from the committed tables, check the counts against the
//! declaration totals the extraction reported, and then check that every band of ids the registry
//! claims is well formed and lands on exactly one source.

use rbms_skin::dst::DrawStateSource;
use rbms_skin::property::generated::*;
use rbms_skin::property::{
    DefaultState, FLOAT_MAX, FLOAT_MIN, MAPPINGS, Mapping, PropertyKind, SkinStateSource, StateSource, UNMAPPED_BOOLEAN, UNMAPPED_FLOAT, UNMAPPED_INTEGER,
    UNMAPPED_STRING, UnmappedLog, clamp_float, normalize_boolean_id, source_of,
};
use rbms_skin::timer::{TIMER_CONSTANT_COUNT, timer_id};

/// FNV-1a 64-bit offset basis, matching `tools/gen-skin-property.rs`.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// How many `public static final int` declarations the reference source holds, counted with
/// `grep -c`. The extraction must account for every one of them across the two generated tables.
const REFERENCE_DECLARATIONS: usize = 968;

/// The prefixes the generator emits, in the order it hashes them.
const PREFIXES: [&str; 12] = ["OPTION", "NUMBER", "RATE", "SLIDER", "BARGRAPH", "FLOAT", "STRING", "BUTTON", "OFFSET", "VALUE", "IMAGE", "EVENT"];

/// How many `OFFSET_*` declarations share an id with an earlier one.
///
/// The reference gives the 1P, 2P and 3P judge and judge-detail offsets the same two ids, so four
/// of the sixteen declarations are aliases rather than distinct offsets.
const OFFSET_ALIASED_DECLARATIONS: usize = 4;

fn derive_checksum() -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for (_, table) in ALL_PROPERTY_TABLES {
        for (value, name) in *table {
            for byte in format!("{name}={value}\n").into_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    }
    hash
}

fn per_prefix_count(prefix: &str) -> usize {
    match prefix {
        "OPTION" => OPTION_CONSTANT_COUNT,
        "NUMBER" => NUMBER_CONSTANT_COUNT,
        "RATE" => RATE_CONSTANT_COUNT,
        "SLIDER" => SLIDER_CONSTANT_COUNT,
        "BARGRAPH" => BARGRAPH_CONSTANT_COUNT,
        "FLOAT" => FLOAT_CONSTANT_COUNT,
        "STRING" => STRING_CONSTANT_COUNT,
        "BUTTON" => BUTTON_CONSTANT_COUNT,
        "OFFSET" => OFFSET_CONSTANT_COUNT,
        "VALUE" => VALUE_CONSTANT_COUNT,
        "IMAGE" => IMAGE_CONSTANT_COUNT,
        "EVENT" => EVENT_CONSTANT_COUNT,
        other => panic!("{other} has no committed count"),
    }
}

fn ids_of(table: &[(i32, &str)]) -> Vec<i32> {
    table.iter().map(|(value, _)| *value).collect()
}

fn declared_ids(kind: PropertyKind) -> Vec<i32> {
    kind.tables().iter().flat_map(|table| ids_of(table)).collect()
}

#[test]
fn property_table_checksum_matches() {
    assert_eq!(derive_checksum(), PROPERTY_TABLE_CHECKSUM, "the committed table and its checksum disagree; regenerate rather than hand-edit");
}

#[test]
fn property_tables_are_in_generator_order() {
    let order: Vec<&str> = ALL_PROPERTY_TABLES.iter().map(|(prefix, _)| *prefix).collect();
    assert_eq!(order, PREFIXES, "the table index no longer lists the prefixes in the order the checksum hashes them");
    for (prefix, table) in ALL_PROPERTY_TABLES {
        assert_eq!(table.len(), per_prefix_count(prefix), "{prefix} table length and committed count disagree");
        assert!(!table.is_empty(), "{prefix} table is empty");
    }
}

#[test]
fn property_table_covers_every_declaration() {
    assert_eq!(REFERENCE_CONSTANT_COUNT, REFERENCE_DECLARATIONS, "the extraction saw a different number of declarations than the source holds");
    assert_eq!(
        PROPERTY_CONSTANT_COUNT + TIMER_CONSTANT_COUNT,
        REFERENCE_CONSTANT_COUNT,
        "the property and timer tables together no longer account for every declaration"
    );
    let summed: usize = PREFIXES.iter().map(|prefix| per_prefix_count(prefix)).sum();
    assert_eq!(summed, PROPERTY_CONSTANT_COUNT, "the per-prefix counts do not sum to the property total");
}

#[test]
fn every_table_entry_carries_its_prefix() {
    for (prefix, table) in ALL_PROPERTY_TABLES {
        for (value, name) in *table {
            assert!(name.starts_with(prefix), "{name} = {value} sits in the {prefix} table");
            assert!(name.len() > prefix.len() + 1, "{name} is only a prefix");
        }
    }
}

#[test]
fn property_names_are_unique_within_a_prefix() {
    for (prefix, table) in ALL_PROPERTY_TABLES {
        let mut names: Vec<&str> = table.iter().map(|(_, name)| *name).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "{prefix} declares a name twice");
    }
}

#[test]
fn property_ids_are_unique_except_the_aliased_offsets() {
    for (prefix, table) in ALL_PROPERTY_TABLES {
        let mut ids = ids_of(table);
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        let aliases = total - ids.len();
        let expected = if *prefix == "OFFSET" { OFFSET_ALIASED_DECLARATIONS } else { 0 };
        assert_eq!(aliases, expected, "{prefix} has an unexpected number of ids declared more than once");
    }
}

#[test]
fn declared_ids_are_non_negative_apart_from_the_random_marker() {
    for (prefix, table) in ALL_PROPERTY_TABLES {
        for (value, name) in *table {
            if *value == OPTION_RANDOM_VALUE {
                assert_eq!(*name, "OPTION_RANDOM_VALUE", "only the random marker may carry a negative id");
                continue;
            }
            assert!(*value >= 0, "{prefix} declares {name} with a negative id {value}");
        }
    }
}

#[test]
fn slider_and_bargraph_ids_alias_the_rate_space() {
    let rate = ids_of(ALL_RATE);
    let extras: Vec<i32> = ids_of(ALL_SLIDER).into_iter().chain(ids_of(ALL_BARGRAPH)).filter(|id| !rate.contains(id)).collect();
    assert_eq!(
        extras,
        [SLIDER_PRACTICE_POSITION, BARGRAPH_RATE_SCORE],
        "a slider or bar graph id no longer aliases a rate id, so the float accessor may be ambiguous"
    );
}

#[test]
fn the_two_float_id_spaces_do_not_overlap() {
    let rate = ids_of(ALL_RATE);
    for id in ids_of(ALL_FLOAT) {
        assert!(!rate.contains(&id), "float id {id} collides with a rate id, so one float accessor cannot serve both");
    }
}

#[test]
fn slider_and_bargraph_ids_sit_in_their_documented_bands() {
    for id in ids_of(ALL_SLIDER) {
        assert!((SLIDER_MUSICSELECT_POSITION..=SLIDER_PRACTICE_POSITION).contains(&id), "slider id {id} left its band");
    }
    for id in ids_of(ALL_BARGRAPH) {
        assert!((BARGRAPH_MUSIC_PROGRESS..=BARGRAPH_RATE_EXSCORE).contains(&id), "bar graph id {id} left its band");
    }
}

#[test]
fn property_kind_declaration_counts_match_the_tables() {
    assert_eq!(PropertyKind::Boolean.declaration_count(), OPTION_CONSTANT_COUNT);
    assert_eq!(PropertyKind::Integer.declaration_count(), NUMBER_CONSTANT_COUNT);
    assert_eq!(PropertyKind::String.declaration_count(), STRING_CONSTANT_COUNT);
    assert_eq!(PropertyKind::Timer.declaration_count(), TIMER_CONSTANT_COUNT);
    assert_eq!(PropertyKind::Float.declaration_count(), RATE_CONSTANT_COUNT + FLOAT_CONSTANT_COUNT + SLIDER_CONSTANT_COUNT + BARGRAPH_CONSTANT_COUNT);
}

#[test]
fn property_kind_names_its_ids() {
    assert_eq!(PropertyKind::Boolean.name(OPTION_1P_AAA), Some("OPTION_1P_AAA"));
    assert_eq!(PropertyKind::Integer.name(NUMBER_COMBO), Some("NUMBER_COMBO"));
    assert_eq!(PropertyKind::String.name(STRING_TITLE), Some("STRING_TITLE"));
    assert_eq!(PropertyKind::Float.name(RATE_MASTERVOLUME), Some("RATE_MASTERVOLUME"), "the canonical rate name wins over its slider alias");
    assert_eq!(PropertyKind::Float.name(FLOAT_HISPEED), Some("FLOAT_HISPEED"));
    assert_eq!(PropertyKind::Timer.name(timer_id::PLAY.get()), Some("PLAY"), "the timer table strips its only prefix");
    assert_ne!(
        PropertyKind::Integer.name(OPTION_1P_AAA),
        PropertyKind::Boolean.name(OPTION_1P_AAA),
        "each kind owns its own id space, so one number names two different properties"
    );
    assert!(PropertyKind::Boolean.is_declared(OPTION_SONGBAR));
    assert!(!PropertyKind::Boolean.is_declared(i32::MAX), "an id the reference never declares is not declared");
}

#[test]
fn mappings_are_ordered_and_disjoint() {
    for mapping in MAPPINGS {
        assert!(mapping.first_id <= mapping.last_id, "{mapping:?} runs backwards");
    }
    for (index, left) in MAPPINGS.iter().enumerate() {
        for right in &MAPPINGS[index + 1..] {
            if left.kind != right.kind {
                continue;
            }
            let overlaps = left.first_id <= right.last_id && right.first_id <= left.last_id;
            assert!(!overlaps, "{left:?} and {right:?} claim the same ids");
        }
    }
}

#[test]
fn every_mapped_id_has_a_source() {
    for mapping in MAPPINGS {
        assert_eq!(source_of(mapping.kind, mapping.first_id), Some(mapping.source), "{mapping:?} does not route its own first id");
        assert_eq!(source_of(mapping.kind, mapping.last_id), Some(mapping.source), "{mapping:?} does not route its own last id");
    }
    for kind in PropertyKind::ALL {
        for id in declared_ids(kind) {
            let Some(mapping) = MAPPINGS.iter().find(|mapping| mapping.kind == kind && mapping.covers(id)) else {
                continue;
            };
            assert_eq!(source_of(kind, id), Some(mapping.source), "{kind:?} id {id} routes somewhere other than its own band");
        }
    }
}

#[test]
fn the_play_band_is_wired_first() {
    let play = [
        (PropertyKind::Integer, NUMBER_SCORE2),
        (PropertyKind::Integer, NUMBER_COMBO),
        (PropertyKind::Integer, NUMBER_GROOVEGAUGE),
        (PropertyKind::Integer, NUMBER_TOTALNOTES2),
        (PropertyKind::Integer, NUMBER_SCORE_RATE),
        (PropertyKind::Integer, NUMBER_TOTAL_RATE),
        (PropertyKind::Integer, NUMBER_NOWBPM),
        (PropertyKind::Float, RATE_SCORE),
        (PropertyKind::Float, RATE_MUSIC_PROGRESS),
        (PropertyKind::Float, FLOAT_GROOVEGAUGE_1P),
        (PropertyKind::Boolean, OPTION_1P_AAA),
        (PropertyKind::Boolean, OPTION_1P_100),
        (PropertyKind::Boolean, OPTION_1P_PERFECT),
        (PropertyKind::Boolean, OPTION_AUTOPLAYON),
        (PropertyKind::Timer, timer_id::PLAY.get()),
        (PropertyKind::Timer, timer_id::READY.get()),
        (PropertyKind::Timer, timer_id::JUDGE_1P.get()),
        (PropertyKind::Timer, timer_id::BOMB_1P_KEY1.get()),
        (PropertyKind::Timer, timer_id::KEYON_1P_SCRATCH.get()),
        (PropertyKind::Timer, timer_id::RHYTHM.get()),
    ];
    for (kind, id) in play {
        assert_eq!(source_of(kind, id), Some(StateSource::PlaySession), "{kind:?} id {id} is not wired to the play session");
    }

    let judged = [
        (PropertyKind::Integer, NUMBER_PERFECT),
        (PropertyKind::Integer, NUMBER_POOR),
        (PropertyKind::Float, RATE_PGREAT),
        (PropertyKind::Float, FLOAT_PERFECT_RATE),
        (PropertyKind::Float, FLOAT_TIMING_AVERAGE),
        (PropertyKind::Boolean, OPTION_PERFECT_EXIST),
    ];
    for (kind, id) in judged {
        assert_eq!(source_of(kind, id), Some(StateSource::JudgeCounters), "{kind:?} id {id} is not wired to the judge counters");
    }
}

#[test]
fn the_select_result_decide_and_keyconfig_bands_are_wired() {
    let expected = [
        (PropertyKind::Boolean, OPTION_SONGBAR, StateSource::SongSelect),
        (PropertyKind::Boolean, OPTION_SELECT_BAR_HARD_CLEARED, StateSource::SongSelect),
        (PropertyKind::Integer, NUMBER_PLAYLEVEL, StateSource::SongSelect),
        (PropertyKind::String, STRING_ARTIST, StateSource::SongSelect),
        (PropertyKind::Float, RATE_MUSICSELECT_POSITION, StateSource::SongSelect),
        (PropertyKind::Timer, timer_id::SONGBAR_CHANGE.get(), StateSource::SongSelect),
        (PropertyKind::Boolean, OPTION_RESULT_CLEAR, StateSource::ScoreResult),
        (PropertyKind::Boolean, OPTION_UPDATE_SCORE, StateSource::ScoreResult),
        (PropertyKind::Integer, NUMBER_TARGET_SCORE, StateSource::ScoreResult),
        (PropertyKind::Float, RATE_TARGETSCORE, StateSource::ScoreResult),
        (PropertyKind::Timer, timer_id::RESULTGRAPH_BEGIN.get(), StateSource::ScoreResult),
        (PropertyKind::Boolean, OPTION_NOW_LOADING, StateSource::Decide),
        (PropertyKind::Integer, NUMBER_LOADING_PROGRESS, StateSource::Decide),
        (PropertyKind::Float, RATE_LOAD_PROGRESS, StateSource::Decide),
        (PropertyKind::String, rbms_skin::property::STRING_KEYNAME_FIRST, StateSource::KeyConfig),
        (PropertyKind::String, rbms_skin::property::STRING_KEYNAME_EXTENDED_LAST, StateSource::KeyConfig),
        (PropertyKind::Integer, NUMBER_HIGHSCORE, StateSource::ScoreStore),
        (PropertyKind::Boolean, OPTION_IR_LOADED, StateSource::InternetRanking),
        (PropertyKind::Float, RATE_MASTERVOLUME, StateSource::PlayerConfig),
        (PropertyKind::String, STRING_SKIN_NAME, StateSource::SkinCustomize),
    ];
    for (kind, id, source) in expected {
        assert_eq!(source_of(kind, id), Some(source), "{kind:?} id {id} is not wired to {source:?}");
    }
}

#[test]
fn a_negative_option_routes_like_its_positive() {
    assert_eq!(normalize_boolean_id(-OPTION_1P_AAA), OPTION_1P_AAA);
    assert_eq!(source_of(PropertyKind::Boolean, -OPTION_1P_AAA), source_of(PropertyKind::Boolean, OPTION_1P_AAA));
    assert_eq!(normalize_boolean_id(i32::MIN), i32::MAX, "the fold must not overflow on the extreme id");
}

#[test]
fn unmapped_ids_have_no_source() {
    assert_eq!(source_of(PropertyKind::Boolean, OPTION_STATE_PRACTICE), None, "practice is not implemented, so it must not claim a source");
    assert_eq!(source_of(PropertyKind::Integer, NUMBER_CURRENT_FPS), None);
    assert_eq!(source_of(PropertyKind::String, STRING_PRACTICE_ITEM1), None);
    assert_eq!(source_of(PropertyKind::Timer, timer_id::CUSTOM_BEGIN.get()), None);
}

#[test]
fn an_unanswered_read_returns_its_documented_default() {
    let state = DefaultState;
    assert_eq!(state.boolean(OPTION_1P_AAA), UNMAPPED_BOOLEAN);
    assert_eq!(state.integer(NUMBER_COMBO), UNMAPPED_INTEGER);
    assert_eq!(state.float(RATE_SCORE), UNMAPPED_FLOAT);
    assert_eq!(state.string(STRING_TITLE), UNMAPPED_STRING);
    assert_eq!(state.timer(timer_id::PLAY.get()), None);
    assert_eq!(rbms_skin::dst::OffsetSource::offset(&state, 0), None);
}

#[test]
fn the_unmapped_log_counts_each_id_once() {
    let log = UnmappedLog::new();
    assert_eq!(log.check(PropertyKind::Integer, NUMBER_COMBO), Ok(StateSource::PlaySession));
    assert_eq!(log.misses(), 0, "a routed read is not a miss");

    assert_eq!(log.check(PropertyKind::Integer, NUMBER_CURRENT_FPS), Err(true), "the first miss reports itself as first");
    assert_eq!(log.check(PropertyKind::Integer, NUMBER_CURRENT_FPS), Err(false), "a repeat miss must not ask to be logged again");
    assert_eq!(log.misses(), 2, "both reads are counted even though only one is logged");
    assert_eq!(log.distinct(), vec![(PropertyKind::Integer, NUMBER_CURRENT_FPS)]);

    log.clear();
    assert_eq!(log.misses(), 0);
    assert!(log.distinct().is_empty());
}

#[test]
fn the_unmapped_log_folds_a_negative_option() {
    let log = UnmappedLog::new();
    assert_eq!(log.check(PropertyKind::Boolean, OPTION_STATE_PRACTICE), Err(true));
    assert_eq!(log.check(PropertyKind::Boolean, -OPTION_STATE_PRACTICE), Err(false), "an option and its negation are one id");
    assert_eq!(log.distinct(), vec![(PropertyKind::Boolean, OPTION_STATE_PRACTICE)]);
}

#[test]
fn a_float_read_is_clamped() {
    assert_eq!(clamp_float(0.5), 0.5);
    assert_eq!(clamp_float(-1.0), FLOAT_MIN);
    assert_eq!(clamp_float(2.0), FLOAT_MAX);
    assert_eq!(clamp_float(f32::NAN), UNMAPPED_FLOAT);
    assert_eq!(clamp_float(f32::INFINITY), FLOAT_MAX);
}

#[test]
fn a_mapping_covers_its_own_endpoints() {
    let mapping = Mapping { kind: PropertyKind::Integer, first_id: NUMBER_PERFECT, last_id: NUMBER_POOR, source: StateSource::JudgeCounters };
    assert!(mapping.covers(NUMBER_PERFECT));
    assert!(mapping.covers(NUMBER_POOR));
    assert!(!mapping.covers(NUMBER_PERFECT - 1));
    assert!(!mapping.covers(NUMBER_POOR + 1));
}
