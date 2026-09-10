use super::{ALL_TIMER, TIMER_CONSTANT_COUNT, TIMER_TABLE_CHECKSUM, TimerId, TimerState, timer_id, timer_name};

/// FNV-1a 64-bit offset basis, matching `tools/gen-skin-timer.rs`.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// How many `TIMER_*` declarations the reference source holds, counted with `grep -c`. The whole
/// file holds 968 `public static final int` declarations across every prefix.
const REFERENCE_TIMER_DECLARATIONS: usize = 151;

/// Entries of [`ALL_TIMER`] that mark a band edge rather than naming a timer.
const BAND_MARKERS: [&str; 3] = ["MAX", "CUSTOM_BEGIN", "CUSTOM_END"];

fn derive_checksum() -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for (value, name) in ALL_TIMER {
        for byte in format!("{name}={value}\n").into_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

#[test]
fn timer_table_checksum_matches() {
    assert_eq!(derive_checksum(), TIMER_TABLE_CHECKSUM, "the committed table and its checksum disagree; regenerate rather than hand-edit");
}

#[test]
fn timer_table_holds_every_declaration() {
    assert_eq!(TIMER_CONSTANT_COUNT, REFERENCE_TIMER_DECLARATIONS, "the extraction lost or gained declarations");
    assert_eq!(ALL_TIMER.len(), TIMER_CONSTANT_COUNT, "the index array and the declared count disagree");
}

#[test]
fn timer_ids_and_names_are_unique() {
    let mut ids: Vec<i32> = ALL_TIMER.iter().map(|(value, _)| *value).collect();
    ids.sort_unstable();
    let mut deduped = ids.clone();
    deduped.dedup();
    assert_eq!(ids.len(), deduped.len(), "two declarations share an id");

    let mut names: Vec<&str> = ALL_TIMER.iter().map(|(_, name)| *name).collect();
    names.sort_unstable();
    let mut unique_names = names.clone();
    unique_names.dedup();
    assert_eq!(names.len(), unique_names.len(), "two declarations share a name");
}

#[test]
fn spot_checked_ids_match_the_reference() {
    let expected: [(TimerId, i32, &str); 26] = [
        (timer_id::STARTINPUT, 1, "STARTINPUT"),
        (timer_id::FADEOUT, 2, "FADEOUT"),
        (timer_id::FAILED, 3, "FAILED"),
        (timer_id::SONGBAR_MOVE, 10, "SONGBAR_MOVE"),
        (timer_id::SONGBAR_CHANGE, 11, "SONGBAR_CHANGE"),
        (timer_id::README_END, 16, "README_END"),
        (timer_id::PANEL1_ON, 21, "PANEL1_ON"),
        (timer_id::PANEL6_OFF, 36, "PANEL6_OFF"),
        (timer_id::READY, 40, "READY"),
        (timer_id::PLAY, 41, "PLAY"),
        (timer_id::GAUGE_MAX_2P, 45, "GAUGE_MAX_2P"),
        (timer_id::JUDGE_1P, 46, "JUDGE_1P"),
        (timer_id::JUDGE_3P, 247, "JUDGE_3P"),
        (timer_id::COMBO_1P, 446, "COMBO_1P"),
        (timer_id::COMBO_3P, 448, "COMBO_3P"),
        (timer_id::FULLCOMBO_2P, 49, "FULLCOMBO_2P"),
        (timer_id::SCORE_AAA, 350, "SCORE_AAA"),
        (timer_id::SCORE_TARGET, 352, "SCORE_TARGET"),
        (timer_id::BOMB_1P_SCRATCH, 50, "BOMB_1P_SCRATCH"),
        (timer_id::HOLD_1P_KEY1, 71, "HOLD_1P_KEY1"),
        (timer_id::HCN_ACTIVE_1P_KEY1, 251, "HCN_ACTIVE_1P_KEY1"),
        (timer_id::HCN_DAMAGE_1P_KEY1, 271, "HCN_DAMAGE_1P_KEY1"),
        (timer_id::KEYOFF_2P_KEY9, 139, "KEYOFF_2P_KEY9"),
        (timer_id::RHYTHM, 140, "RHYTHM"),
        (timer_id::RESULT_UPDATESCORE, 152, "RESULT_UPDATESCORE"),
        (timer_id::PM_CHARA_1P_NEUTRAL, 900, "PM_CHARA_1P_NEUTRAL"),
    ];
    for (id, value, name) in expected {
        assert_eq!(id.get(), value, "{name} carries the wrong id");
        assert_eq!(timer_name(id), Some(name), "{name} is not the extracted name of {value}");
    }
}

#[test]
fn ir_timers_are_consecutive() {
    assert_eq!(timer_id::IR_CONNECT_BEGIN.get(), 172);
    assert_eq!(timer_id::IR_CONNECT_SUCCESS.get(), 173);
    assert_eq!(timer_id::IR_CONNECT_FAIL.get(), 174);
}

#[test]
fn every_typed_constant_has_a_table_entry() {
    for (value, name) in ALL_TIMER {
        assert_eq!(timer_name(TimerId(*value)), Some(*name));
    }
    for marker in BAND_MARKERS {
        assert!(ALL_TIMER.iter().any(|(_, name)| *name == marker), "{marker} left the table");
    }
}

#[test]
fn unknown_ids_have_no_name() {
    assert_eq!(timer_name(TimerId(-1)), None);
    assert_eq!(timer_name(TimerId(9)), None);
}

#[test]
fn id_bands_follow_the_reference_ranges() {
    assert!(timer_id::PLAY.is_builtin());
    assert!(!timer_id::PLAY.is_custom());
    assert!(timer_id::MAX.is_builtin());
    assert!(!TimerId(timer_id::MAX.get() + 1).is_builtin());
    assert!(!TimerId(-1).is_builtin());
    assert!(timer_id::CUSTOM_BEGIN.is_custom());
    assert!(timer_id::CUSTOM_END.is_custom());
    assert!(!TimerId(timer_id::CUSTOM_BEGIN.get() - 1).is_custom());
    assert!(!TimerId(timer_id::CUSTOM_END.get() + 1).is_custom());
}

#[test]
fn timers_start_off() {
    let state = TimerState::new();
    assert!(state.is_off(timer_id::PLAY));
    assert!(!state.is_on(timer_id::PLAY));
    assert_eq!(state.get(timer_id::PLAY), None);
    assert_eq!(state.elapsed(timer_id::PLAY, 1_000), None);
}

#[test]
fn switching_on_records_the_moment() {
    let mut state = TimerState::new();
    state.set_on(timer_id::PLAY, 2_500);
    assert!(state.is_on(timer_id::PLAY));
    assert_eq!(state.get(timer_id::PLAY), Some(2_500));
    assert_eq!(state.elapsed(timer_id::PLAY, 2_500), Some(0));
    assert_eq!(state.elapsed(timer_id::PLAY, 3_000), Some(500));
    assert_eq!(state.elapsed(timer_id::PLAY, 2_000), Some(-500));
}

#[test]
fn setting_on_again_restarts_but_switching_does_not() {
    let mut state = TimerState::new();
    state.set_on(timer_id::JUDGE_1P, 100);
    state.set_on(timer_id::JUDGE_1P, 400);
    assert_eq!(state.get(timer_id::JUDGE_1P), Some(400), "set_on restarts a running timer");

    state.switch(timer_id::READY, true, 100);
    state.switch(timer_id::READY, true, 400);
    assert_eq!(state.get(timer_id::READY), Some(100), "switch leaves a running timer alone");
}

#[test]
fn switching_off_forgets_the_moment() {
    let mut state = TimerState::new();
    state.set_on(timer_id::FADEOUT, 10);
    state.set_off(timer_id::FADEOUT);
    assert!(state.is_off(timer_id::FADEOUT));

    state.set_on(timer_id::FADEOUT, 20);
    state.switch(timer_id::FADEOUT, false, 30);
    assert_eq!(state.get(timer_id::FADEOUT), None);
}

#[test]
fn clear_switches_every_timer_off() {
    let mut state = TimerState::new();
    state.set_on(timer_id::PLAY, 1);
    state.set_on(timer_id::RHYTHM, 2);
    state.clear();
    assert!(state.is_off(timer_id::PLAY));
    assert!(state.is_off(timer_id::RHYTHM));
}

#[test]
fn custom_band_timers_share_the_state() {
    let mut state = TimerState::new();
    let custom = TimerId(timer_id::CUSTOM_BEGIN.get() + 7);
    state.set_on(custom, 5_000);
    assert_eq!(state.elapsed(custom, 5_250), Some(250));
    assert!(custom.is_custom());
}
