use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::GaugeAutoShift;
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;

use super::*;

const ALL_GAUGES: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rbms_config_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn ron_of(config: &Config) -> String {
    ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default()).expect("a config serialises")
}

#[test]
fn default_values_are_sane() {
    let c = Config::default();
    assert_eq!(c.schema_version, CURRENT_SCHEMA_VERSION);
    assert!((c.play.hispeed - DEFAULT_HISPEED).abs() < 1e-9);
    assert_eq!(c.play.gauge, GaugeKind::Normal);
    assert_eq!(c.play.random, NoteOption::Off);
    assert!(c.play.autoplay);
    assert!(c.display.bga);
    assert!(c.play.auto_replay);
    assert_eq!(c.judge.judge_rate_key, UNMODIFIED_JUDGE_RATES);
    assert_eq!(c.judge.judge_rate_scratch, UNMODIFIED_JUDGE_RATES);
    assert_eq!(c.judge.longnote_margin_rate, LN_MARGIN_DEFAULT_PERCENT);
    assert_eq!(c.judge.judge_algorithm, JudgeAlgorithm::default(), "the shipped algorithm is the one the engine judges with by default");
    assert_eq!(c.judge.ln_mode, LnMode::LongNote);
    assert_eq!(c.judge.gauge_set, None);
    assert_eq!(c.judge.gauge_auto_shift, GaugeAutoShift::None);
    assert_eq!(c.judge.bottom_shiftable_gauge, GaugeKind::AssistEasy);
    assert_eq!(c.judge.target, ScoreTarget::LocalBest);
    assert_eq!(c.judge.offset_ms, 0);
    assert!(c.library.preview);
    assert_eq!(c.library.songs_folder, None);
    assert!(c.library.folders.is_empty());
    assert!(c.library.tables.is_empty());
    assert_eq!(c.display.font_path, None);
    assert_eq!(c.display.skin, DEFAULT_SKIN);
    assert_eq!(c.network.server_url, None);
    assert_eq!(c.network.player_id, DEFAULT_PLAYER_ID, "an unconfigured client submits under the only id the server accepts without a token");
    assert_eq!(c.network.ir_token, None);
    assert_eq!(c.network.ir_login_id, None);
    assert_eq!(c.network.ir_email, None);
    assert!(!c.network.sync_settings, "settings sync is opt-in");
    assert!(c.network.auto_upload_replay, "replay upload rides along with a ranked submit by default");
    assert!(c.network.rivals.is_empty());
    assert_eq!(c.audio.device, None, "the system default device until one is picked");
    assert_eq!(c.audio.buffer_frames, None);
    assert_eq!(c.audio.sample_rate, None);
    assert_eq!(c.audio.polyphony, DEFAULT_POLYPHONY_VOICES);
    assert!((c.audio.master - DEFAULT_MASTER_VOLUME).abs() < 1e-6);
    assert!((c.audio.key - DEFAULT_BUS_VOLUME).abs() < 1e-6);
    assert!((c.audio.bg - DEFAULT_BUS_VOLUME).abs() < 1e-6);
    assert!((c.audio.system - DEFAULT_BUS_VOLUME).abs() < 1e-6);
}

/// The JUDGE tab is stored as tokens rather than Rust variant names, so the file stays readable and
/// the account blob keeps a vocabulary the score server can speak. Every token must come back as the
/// value it was written from.
#[test]
fn every_judge_token_round_trips_through_its_own_vocabulary() {
    for algorithm in JudgeAlgorithm::ALL {
        assert_eq!(algorithm_from_token(algorithm_token(algorithm)), algorithm);
    }
    for mode in LnMode::ALL {
        assert_eq!(ln_mode_from_token(ln_mode_token(mode)), mode);
    }
    for shift in GaugeAutoShift::ALL {
        assert_eq!(gauge_auto_shift_from_token(gauge_auto_shift_token(shift)), shift);
    }
    for set in GAUGE_SET_CYCLE {
        assert_eq!(gauge_set_from_token(gauge_set_token(set)), set);
    }
    for target in ScoreTarget::ALL {
        assert_eq!(target_from_token(target.token()), target);
    }
}

/// A token no vocabulary knows falls back to the shipped value rather than failing the whole load,
/// which is what keeps a hand-edited or foreign file usable.
#[test]
fn an_unknown_judge_token_falls_back_to_the_shipped_value() {
    assert_eq!(algorithm_from_token("nonsense"), JudgeAlgorithm::default());
    assert_eq!(ln_mode_from_token(""), LnMode::LongNote);
    assert_eq!(gauge_auto_shift_from_token("SIDEWAYS"), GaugeAutoShift::None);
    assert_eq!(gauge_set_from_token("BEAT_7K"), None, "a mode key is not a choice this row offers");
    assert_eq!(target_from_token("nonsense"), ScoreTarget::LocalBest);
    let c: Config = ron::from_str(r#"(schema_version: 2, judge: (judge_algorithm: "nonsense", ln_mode: "?", gauge_set: "?"))"#).expect("a fragment parses");
    assert_eq!(c.judge.judge_algorithm, JudgeAlgorithm::default());
    assert_eq!(c.judge.ln_mode, LnMode::LongNote);
    assert_eq!(c.judge.gauge_set, None);
}

/// A document written by the build that had one JUDGE WIDTH row spreads that one percentage over
/// the six the rows became, so an upgrade keeps judging the way the user set it.
#[test]
fn a_single_judge_width_document_spreads_over_every_tier() {
    let (config, from) = migrate(r#"(schema_version: 1, judge: (offset_ms: -20, auto_offset: true, judge_rate: 80))"#).expect("a schema 1 file migrates");
    assert_eq!(from, Some(SINGLE_JUDGE_WIDTH_SCHEMA_VERSION));
    assert_eq!(config.judge.judge_rate_key, [80; JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(config.judge.judge_rate_scratch, [80; JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(config.judge.offset_ms, -20, "the rest of the group comes across untouched");
    assert!(config.judge.auto_offset);
    assert_eq!(config.judge.longnote_margin_rate, LN_MARGIN_DEFAULT_PERCENT, "a row that did not exist takes its shipped value");
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
}

/// A schema 1 document that never moved the JUDGE WIDTH row migrates to the shipped widths rather
/// than to whatever a missing field would otherwise leave behind.
#[test]
fn a_single_judge_width_document_without_the_row_migrates_to_the_shipped_widths() {
    let (config, from) = migrate(r#"(schema_version: 1, play: (hispeed: 3.0))"#).expect("a schema 1 file migrates");
    assert_eq!(from, Some(SINGLE_JUDGE_WIDTH_SCHEMA_VERSION));
    assert_eq!(config.judge.judge_rate_key, UNMODIFIED_JUDGE_RATES);
    assert_eq!(config.judge.judge_rate_scratch, UNMODIFIED_JUDGE_RATES);
    assert!((config.play.hispeed - 3.0).abs() < 1e-9);
}

#[test]
fn ron_round_trip_preserves_every_field() {
    let mut c = Config::default();
    c.play.hispeed = 3.25;
    c.play.gauge = GaugeKind::Hard;
    c.play.lift = 0.15;
    c.play.cover = 0.4;
    c.play.scratch_left = true;
    c.play.scratch_auto = true;
    c.play.autoplay = false;
    c.play.random = NoteOption::Mirror;
    c.play.constant_speed = true;
    c.play.total_override = 320.0;
    c.play.auto_replay = false;
    c.judge.offset_ms = -33;
    c.judge.auto_offset = true;
    c.judge.judge_rate_key = [150, 145, 140];
    c.judge.judge_rate_scratch = [90, 95, 105];
    c.judge.longnote_margin_rate = 120;
    c.judge.judge_algorithm = JudgeAlgorithm::Combo;
    c.judge.ln_mode = LnMode::HellChargeNote;
    c.judge.gauge_set = Some(GaugeSetId::Lr2);
    c.judge.gauge_auto_shift = GaugeAutoShift::BestClear;
    c.judge.bottom_shiftable_gauge = GaugeKind::Normal;
    c.judge.target = ScoreTarget::IrBest;
    c.display.bga = false;
    c.display.skin = "WIDE".into();
    c.display.debug = true;
    c.display.font_path = Some("/tmp/f.ttf".into());
    c.display.score_graph = false;
    c.display.replay_analysis = false;
    c.library.preview = false;
    c.library.songs_folder = Some("/songs".into());
    c.library.folders = vec!["/a".into(), "/b/c".into()];
    c.library.tables = vec![TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into() }];
    c.network.server_url = Some("https://ir.example/api".into());
    c.network.player_id = "dj".into();
    c.network.ir_token = Some("tok-123".into());
    c.network.ir_login_id = Some("dj".into());
    c.network.ir_email = Some("dj@example.test".into());
    c.network.sync_settings = true;
    c.network.auto_upload_replay = false;
    c.network.rivals = vec!["rivalone".into(), "rivaltwo".into()];
    c.audio.device = Some("Studio Monitors".into());
    c.audio.buffer_frames = Some(384);
    c.audio.sample_rate = Some(96_000);
    c.audio.polyphony = 256;
    c.audio.master = 0.85;
    c.audio.key = 0.7;
    c.audio.bg = 0.35;
    c.audio.system = 0.15;

    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back, c, "every field survives the round trip");
}

#[test]
fn a_partial_document_keeps_given_fields_and_defaults_the_rest() {
    let c: Config = ron::from_str(r#"(schema_version: 1, play: (hispeed: 7.5, gauge: "easy"))"#).expect("a fragment parses");
    assert!((c.play.hispeed - 7.5).abs() < 1e-9, "explicit field kept");
    assert_eq!(c.play.gauge, GaugeKind::Easy);
    assert_eq!(c.play.random, NoteOption::Off, "missing field defaulted");
    assert!(c.library.preview);
    assert_eq!(c.judge.judge_rate_key, UNMODIFIED_JUDGE_RATES, "an absent group falls back whole");
    assert_eq!(c.audio.polyphony, DEFAULT_POLYPHONY_VOICES);
}

#[test]
fn empty_unit_ron_is_all_defaults() {
    let c: Config = ron::from_str("()").expect("serde(default) empty unit parses");
    assert_eq!(c, Config::default());
}

#[test]
fn an_unknown_key_is_ignored_rather_than_failing_the_load() {
    let c: Config = ron::from_str(r#"(schema_version: 1, invented_by_a_newer_build: 7, play: (hispeed: 4.0))"#).expect("unknown keys are ignored");
    assert!((c.play.hispeed - 4.0).abs() < 1e-9);
}

#[test]
fn the_gauge_and_note_option_are_stored_as_their_settings_tokens() {
    let mut c = Config::default();
    c.play.gauge = GaugeKind::ExHard;
    c.play.random = NoteOption::SRandom;
    let text = ron_of(&c);
    assert!(text.contains("\"exhard\""), "the gauge is written as its token: {text}");
    assert!(text.contains("\"S-RANDOM\""), "the note option is written as its label: {text}");
    let back: Config = ron::from_str(&text).expect("tokens parse back");
    assert_eq!(back.play.gauge, GaugeKind::ExHard);
    assert_eq!(back.play.random, NoteOption::SRandom);
}

#[test]
fn a_gauge_token_round_trips_for_every_gauge() {
    for g in ALL_GAUGES {
        assert_eq!(gauge_from_name(gauge_token(g)), g, "{g:?} token round-trips");
        assert_eq!(gauge_token(g), gauge_token(g).to_ascii_lowercase(), "the token is settings-safe");
    }
    assert_eq!(gauge_from_name("HARD"), GaugeKind::Hard, "parsing is case insensitive");
    assert_eq!(gauge_from_name("assisteasy"), GaugeKind::AssistEasy, "the long spelling is accepted");
    assert_eq!(gauge_from_name("nonsense"), GaugeKind::Normal, "an unknown token falls back to NORMAL");
}

#[test]
fn sanitise_pulls_a_hand_edited_document_back_into_range() {
    let mut c = Config::default();
    c.play.hispeed = 99.0;
    c.play.lift = 4.0;
    c.play.cover = -1.0;
    c.play.total_override = -50.0;
    c.judge.offset_ms = -9_000;
    c.judge.judge_rate_key = [4_000, 4_000, 4_000];
    c.judge.judge_rate_scratch = [-100, -100, -100];
    c.judge.longnote_margin_rate = 9_000;
    c.judge.bottom_shiftable_gauge = GaugeKind::ExHard;
    c.display.skin = "  ".into();
    c.sanitise();
    assert!((c.play.hispeed - HISPEED_MAX).abs() < 1e-9);
    assert!((c.play.lift - LANE_SHADE_MAX).abs() < 1e-6);
    assert!((c.play.cover - LANE_SHADE_MIN).abs() < 1e-6);
    assert!((c.play.total_override - TOTAL_FROM_CHART).abs() < 1e-9);
    assert_eq!(c.judge.offset_ms, JUDGE_OFFSET_MIN_MS);
    assert_eq!(c.judge.judge_rate_key, [JUDGE_RATE_MAX_PERCENT; JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(c.judge.judge_rate_scratch, [JUDGE_RATE_MIN_PERCENT; JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(c.judge.longnote_margin_rate, LN_MARGIN_MAX_PERCENT);
    assert_eq!(c.judge.bottom_shiftable_gauge, GaugeKind::Normal, "a gauge the auto-shift floor cannot hold clamps into range");
    assert_eq!(c.display.skin, DEFAULT_SKIN, "a blank skin name falls back instead of resolving to nothing");
}

#[test]
fn sanitise_uppercases_a_skin_name_and_stamps_the_schema() {
    let mut c = Config { schema_version: 0, ..Config::default() };
    c.display.skin = "wide".into();
    c.sanitise();
    assert_eq!(c.display.skin, "WIDE");
    assert_eq!(c.schema_version, CURRENT_SCHEMA_VERSION);
}

#[test]
fn restoring_audio_settings_clamps_a_hand_edited_file() {
    let mut c = Config::default();
    c.audio.device = Some("   ".into());
    c.audio.buffer_frames = Some(0);
    c.audio.sample_rate = Some(0);
    c.audio.polyphony = 100_000;
    c.audio.master = 4.0;
    c.audio.key = -1.0;
    c.audio.bg = f32::NAN;
    c.audio.system = 0.25;
    c.sanitise();
    assert_eq!(c.audio.device, None, "a blank device name means the system default");
    assert_eq!(c.audio.buffer_frames, None, "zero frames is not a buffer size");
    assert_eq!(c.audio.sample_rate, None);
    assert_eq!(c.audio.polyphony, AUDIO_POLYPHONY_MAX_VOICES);
    assert!((c.audio.master - AUDIO_VOLUME_MAX_GAIN).abs() < 1e-6);
    assert!((c.audio.key - AUDIO_VOLUME_MIN_GAIN).abs() < 1e-6);
    assert!((c.audio.bg - DEFAULT_BUS_VOLUME).abs() < 1e-6, "a NaN gain falls back instead of silencing the bus");
    assert!((c.audio.system - 0.25).abs() < 1e-6);
    assert!(!c.audio.reopen_pending(), "restoring settings is not a parameter change");
}

#[test]
fn a_pending_reopen_is_not_part_of_the_document_two_configurations_are_compared_by() {
    let mut pending = Config::default();
    pending.audio.mark_reopen_pending();
    assert_eq!(pending, Config::default(), "a row waiting to be applied has not changed what would be written");
    assert!(pending.audio.reopen_pending(), "and the flag itself is still set");

    let mut moved = Config::default();
    moved.audio.polyphony += AUDIO_POLYPHONY_STEP_VOICES;
    assert_ne!(moved, Config::default(), "a parameter that actually moved is a difference");

    let mut sanitised = pending.clone();
    sanitised.sanitise();
    assert_eq!(sanitised.audio, pending.audio, "clearing the flag cannot change how the same document compares");
}

#[test]
fn the_shipped_audio_defaults_match_the_config_defaults() {
    assert_eq!(AudioOptions::default(), Config::default().audio);
}

#[test]
fn the_pending_reopen_flag_is_set_and_cleared_explicitly() {
    let mut audio = AudioOptions::default();
    assert!(!audio.reopen_pending());
    audio.mark_reopen_pending();
    assert!(audio.reopen_pending());
    audio.clear_reopen_pending();
    assert!(!audio.reopen_pending());
}

#[test]
fn the_pending_reopen_flag_is_never_written_to_disk() {
    let mut c = Config::default();
    c.audio.mark_reopen_pending();
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert!(!back.audio.reopen_pending(), "a pending reopen is live state, not configuration");
}

#[test]
fn a_volume_steps_by_five_percent_and_stops_at_the_ends() {
    assert_eq!(volume_percent(step_volume(0.5, 1)), 55);
    assert_eq!(volume_percent(step_volume(0.5, -1)), 45);
    assert_eq!(volume_percent(step_volume(1.0, 1)), AUDIO_VOLUME_MAX_PERCENT, "the loudest step stays at the top");
    assert_eq!(volume_percent(step_volume(0.0, -1)), 0, "a muted row cannot go negative");
    assert_eq!(volume_percent(step_volume(0.02, -1)), 0, "a partial step down lands on mute, not below it");
}

#[test]
fn stepping_a_volume_up_and_back_down_lands_on_the_same_percent() {
    let steps = (AUDIO_VOLUME_MAX_PERCENT - volume_percent(DEFAULT_BUS_VOLUME)) / AUDIO_VOLUME_STEP_PERCENT;
    let mut gain = DEFAULT_BUS_VOLUME;
    for _ in 0..steps {
        gain = step_volume(gain, 1);
    }
    assert_eq!(volume_percent(gain), AUDIO_VOLUME_MAX_PERCENT);
    for _ in 0..steps {
        gain = step_volume(gain, -1);
    }
    assert_eq!(volume_percent(gain), volume_percent(DEFAULT_BUS_VOLUME), "whole-percent steps do not drift");
}

#[test]
fn a_volume_row_only_ever_shows_a_multiple_of_its_step() {
    let mut gain = 0.0;
    for _ in 0..=(AUDIO_VOLUME_MAX_PERCENT / AUDIO_VOLUME_STEP_PERCENT) {
        assert_eq!(volume_percent(gain) % AUDIO_VOLUME_STEP_PERCENT, 0, "a step left the row off the percent grid");
        gain = step_volume(gain, 1);
    }
    assert_eq!(volume_percent(gain), AUDIO_VOLUME_MAX_PERCENT, "stepping past the loudest value stays there");
}

#[test]
fn volume_percent_reports_whole_percent_within_the_range() {
    assert_eq!(volume_percent(0.0), 0);
    assert_eq!(volume_percent(0.5), 50);
    assert_eq!(volume_percent(1.0), AUDIO_VOLUME_MAX_PERCENT);
    assert_eq!(volume_percent(9.0), AUDIO_VOLUME_MAX_PERCENT, "an out-of-range gain still shows a legal percent");
    assert_eq!(volume_percent(-2.0), 0);
}

#[test]
fn clamping_a_gain_falls_back_only_for_a_non_number() {
    assert!((clamp_volume(f32::NAN, DEFAULT_BUS_VOLUME) - DEFAULT_BUS_VOLUME).abs() < 1e-6);
    assert!((clamp_volume(2.0, DEFAULT_BUS_VOLUME) - AUDIO_VOLUME_MAX_GAIN).abs() < 1e-6);
    assert!((clamp_volume(-2.0, DEFAULT_BUS_VOLUME) - AUDIO_VOLUME_MIN_GAIN).abs() < 1e-6);
    assert!((clamp_volume(0.25, DEFAULT_BUS_VOLUME) - 0.25).abs() < 1e-6);
}

#[test]
fn an_optional_choice_row_cycles_auto_first_and_wraps_both_ways() {
    let choices = AUDIO_BUFFER_FRAMES_CHOICES;
    assert_eq!(cycle_optional_u32(None, &choices, 1), Some(choices[0]));
    assert_eq!(cycle_optional_u32(Some(choices[0]), &choices, -1), None);
    assert_eq!(cycle_optional_u32(Some(choices[choices.len() - 1]), &choices, 1), None, "past the last size wraps to AUTO");
    assert_eq!(cycle_optional_u32(None, &choices, -1), Some(choices[choices.len() - 1]));
    assert_eq!(cycle_optional_u32(Some(333), &choices, 1), Some(choices[0]), "a size the row never offers steps from AUTO");
}

#[test]
fn the_sample_rate_row_offers_auto_and_the_four_rates() {
    assert_eq!(cycle_optional_u32(None, &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), Some(44_100));
    assert_eq!(cycle_optional_u32(Some(44_100), &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), Some(48_000));
    assert_eq!(cycle_optional_u32(Some(96_000), &AUDIO_SAMPLE_RATE_HZ_CHOICES, 1), None);
}

#[test]
fn the_device_row_cycles_the_default_and_the_reported_names() {
    let names = vec!["Built-in Output".to_string(), "Studio Monitors".to_string()];
    assert_eq!(cycle_device(None, &names, 1).as_deref(), Some("Built-in Output"));
    assert_eq!(cycle_device(Some("Built-in Output"), &names, 1).as_deref(), Some("Studio Monitors"));
    assert_eq!(cycle_device(Some("Studio Monitors"), &names, 1), None, "past the last device is the system default");
    assert_eq!(cycle_device(None, &names, -1).as_deref(), Some("Studio Monitors"));
    assert_eq!(
        cycle_device(Some("Unplugged Interface"), &names, 1).as_deref(),
        Some("Built-in Output"),
        "a device the host no longer reports steps from the default"
    );
    assert_eq!(cycle_device(None, &[], 1), None, "with no devices the row stays on the system default");
}

#[test]
fn polyphony_steps_by_sixty_four_within_the_voice_range() {
    assert_eq!(step_polyphony(DEFAULT_POLYPHONY_VOICES, 1), DEFAULT_POLYPHONY_VOICES + AUDIO_POLYPHONY_STEP_VOICES);
    assert_eq!(step_polyphony(DEFAULT_POLYPHONY_VOICES, -1), DEFAULT_POLYPHONY_VOICES - AUDIO_POLYPHONY_STEP_VOICES);
    assert_eq!(step_polyphony(AUDIO_POLYPHONY_MIN_VOICES, -1), AUDIO_POLYPHONY_MIN_VOICES);
    assert_eq!(step_polyphony(AUDIO_POLYPHONY_MAX_VOICES, 1), AUDIO_POLYPHONY_MAX_VOICES);
    assert_eq!(step_polyphony(0, -1), AUDIO_POLYPHONY_MIN_VOICES, "a hand-edited zero is pulled back into range");
}

#[test]
fn the_folder_list_defaults_to_empty_and_keeps_its_order() {
    assert!(LibraryOptions::default().folders.is_empty());
    let mut c = Config::default();
    c.library.folders = vec!["/a".into(), "/b/c".into(), "D:\\songs".into()];
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back.library.folders, vec!["/a".to_string(), "/b/c".into(), "D:\\songs".into()]);
}

#[test]
fn the_table_list_defaults_to_empty_and_keeps_its_order() {
    assert!(LibraryOptions::default().tables.is_empty());
    let mut c = Config::default();
    c.library.tables = vec![
        TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into() },
        TableSource { name: String::new(), location: "/local/table.json".into() },
    ];
    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back.library.tables.len(), 2, "count preserved");
    assert_eq!(back.library.tables[0].name, "Insane");
    assert_eq!(back.library.tables[0].location, "https://example.com/insane.json");
    assert_eq!(back.library.tables[1].name, "", "empty name preserved");
    assert_eq!(back.library.tables[1].location, "/local/table.json");
}

#[test]
fn merging_absent_or_empty_legacy_lists_leaves_the_library_alone() {
    let mut c = Config::default();
    merge_legacy_lists(&mut c, None, None);
    assert!(c.library.folders.is_empty(), "a missing folders file adds nothing");
    assert!(c.library.tables.is_empty(), "a missing tables file adds nothing");
    merge_legacy_lists(&mut c, Some("()"), Some("()"));
    assert!(c.library.folders.is_empty(), "an empty unit list adds nothing");
    assert!(c.library.tables.is_empty());
}

#[test]
fn merging_a_malformed_legacy_list_is_skipped_rather_than_fatal() {
    let mut c = Config::default();
    c.library.folders = vec!["/songs".into()];
    merge_legacy_lists(&mut c, Some("@@@ not ron @@@"), Some("not ron at all"));
    assert_eq!(c.library.folders, vec!["/songs".to_string()], "an unreadable list leaves the library as it was");
    assert!(c.library.tables.is_empty());
}

#[test]
fn merging_legacy_lists_appends_only_what_is_missing() {
    let mut c = Config::default();
    c.library.folders = vec!["/songs".into()];
    c.library.tables = vec![TableSource { name: "kept".into(), location: "/local/table.json".into() }];
    merge_legacy_lists(
        &mut c,
        Some(r#"(folders: ["/songs", "/more"])"#),
        Some(r#"(tables: [(name: "renamed", location: "/local/table.json"), (name: "Insane", location: "https://example.com/insane.json")])"#),
    );
    assert_eq!(c.library.folders, vec!["/songs".to_string(), "/more".into()], "a folder already listed is not duplicated");
    assert_eq!(c.library.tables.len(), 2, "a table with a known location is not duplicated");
    assert_eq!(c.library.tables[0].name, "kept", "the configuration's own entry wins");
    assert_eq!(c.library.tables[1].location, "https://example.com/insane.json");
}

#[test]
fn a_versionless_document_migrates_from_the_flat_schema() {
    let (config, from) = migrate(r#"(hispeed: 3.0, gauge: "HARD", random: "MIRROR", judge_rate: 120, rivals: ["friend"])"#).expect("a v0 file migrates");
    assert_eq!(from, Some(LEGACY_SCHEMA_VERSION));
    assert!((config.play.hispeed - 3.0).abs() < 1e-9);
    assert_eq!(config.play.gauge, GaugeKind::Hard);
    assert_eq!(config.play.random, NoteOption::Mirror);
    assert_eq!(config.judge.judge_rate_key, [120; JUDGE_WIDTH_TIER_COUNT], "the one JUDGE WIDTH the flat file held covers every tier");
    assert_eq!(config.judge.judge_rate_scratch, [120; JUDGE_WIDTH_TIER_COUNT]);
    assert_eq!(config.network.rivals, vec!["friend".to_string()]);
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION, "the migrated document is stamped with the current schema");
}

#[test]
fn a_flat_file_written_before_the_audio_tab_still_migrates() {
    let (config, from) =
        migrate(r#"(hispeed: 3.0, gauge: "HARD", server_url: Some("https://ir.example/api"), player_id: "dj")"#).expect("a pre-audio file migrates");
    assert_eq!(from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(config.audio, AudioOptions::default(), "a file with no audio keys migrates to the shipped defaults");
    assert_eq!(config.network.player_id, "dj");
}

#[test]
fn a_current_document_needs_no_migration() {
    let (config, from) = migrate(&ron_of(&Config::default())).expect("a v1 file parses");
    assert_eq!(from, None);
    assert_eq!(config, Config::default());
}

#[test]
fn migration_clamps_what_the_old_file_held() {
    let (config, _) = migrate("(hispeed: 99.0, judge_rate: 4000, vol_master: 9.0, audio_polyphony: 100000)").expect("a v0 file migrates");
    assert!((config.play.hispeed - HISPEED_MAX).abs() < 1e-9);
    assert_eq!(config.judge.judge_rate_key, [JUDGE_RATE_MAX_PERCENT; JUDGE_WIDTH_TIER_COUNT]);
    assert!((config.audio.master - AUDIO_VOLUME_MAX_GAIN).abs() < 1e-6);
    assert_eq!(config.audio.polyphony, AUDIO_POLYPHONY_MAX_VOICES);
}

#[test]
fn a_newer_schema_is_refused_rather_than_migrated() {
    let error = migrate("(schema_version: 99)").expect_err("a newer schema cannot be read");
    match error {
        ConfigError::Migrate { from, .. } => assert_eq!(from, 99),
        other => panic!("expected a migrate error, got {other:?}"),
    }
}

#[test]
fn a_malformed_document_is_a_parse_error() {
    let error = migrate("definitely not ron )))").expect_err("nonsense is not a configuration");
    assert!(matches!(error, ConfigError::Parse(_)), "got {error:?}");
}

#[test]
fn load_missing_file_writes_and_returns_defaults() {
    let dir = temp_dir("missing");
    let path = dir.join("settings.ron");
    assert!(!path.exists());
    let outcome = load(&path).expect("a missing file is not an error");
    assert!(path.exists(), "load() of a missing settings file writes defaults out");
    assert_eq!(outcome.config, Config::default());
    assert_eq!(outcome.migrated_from, None);
    let again = load(&path).expect("the freshly written file loads");
    assert_eq!(again.config, Config::default(), "re-loading the freshly written file is stable");
    assert_eq!(again.migrated_from, None, "the file it just wrote is already current");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_malformed_file_backs_up_and_returns_defaults() {
    let dir = temp_dir("bad");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    std::fs::write(&path, "definitely not ron )))").expect("write");
    let outcome = load(&path).expect("a malformed file falls back rather than failing");
    assert_eq!(outcome.config, Config::default(), "malformed file => defaults");
    assert_eq!(outcome.backup.as_deref(), Some(path.with_extension("ron.bak").as_path()));
    assert!(path.with_extension("ron.bak").exists(), "malformed file backed up");
    assert!(!path.exists(), "the corrupt file is renamed away");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_newer_file_is_left_untouched_by_load() {
    let dir = temp_dir("newer");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    let text = "(schema_version: 99, play: (hispeed: 6.0))";
    std::fs::write(&path, text).expect("write");
    let error = load(&path).expect_err("a newer schema is reported");
    assert!(matches!(error, ConfigError::Migrate { from: 99, .. }), "got {error:?}");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), text, "the file a newer build wrote is not overwritten");
    assert!(!path.with_extension("ron.bak").exists(), "a readable newer file is not backed up either");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_then_load_round_trips_on_disk() {
    let dir = temp_dir("io");
    let path = dir.join("nested/settings.ron");
    let mut c = Config::default();
    c.play.hispeed = 5.5;
    c.library.folders = vec!["/songs".into()];
    c.library.tables = vec![TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into() }];
    for id in tab_rows(SettingTab::Judge, &c) {
        assert_eq!(adjust(&mut c, id, 1), AdjustOutcome::Changed, "{id:?} steps");
    }
    save(&c, &path).expect("save creates the parent directory");
    assert!(path.exists());
    let outcome = load(&path).expect("the saved file loads");
    assert_eq!(outcome.config, c, "every JUDGE row survives a save and a restart");
    assert_eq!(outcome.config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(outcome.migrated_from, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_absorbs_the_legacy_sibling_lists_of_a_versionless_file() {
    let dir = temp_dir("siblings");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    std::fs::write(&path, "(hispeed: 4.0)").expect("write");
    std::fs::write(dir.join(LEGACY_FOLDERS_FILE), r#"(folders: ["/songs", "/more"])"#).expect("write");
    std::fs::write(dir.join(LEGACY_TABLES_FILE), r#"(tables: [(name: "Insane", location: "https://example.com/insane.json")])"#).expect("write");

    let outcome = load(&path).expect("a v0 file with siblings loads");
    assert_eq!(outcome.migrated_from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(outcome.config.library.folders, vec!["/songs".to_string(), "/more".into()]);
    assert_eq!(outcome.config.library.tables.len(), 1);
    assert!(dir.join(LEGACY_FOLDERS_FILE).exists(), "the original list is left on disk so an older build still runs");
    assert!(dir.join(LEGACY_TABLES_FILE).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_current_file_ignores_the_legacy_sibling_lists() {
    let dir = temp_dir("current_siblings");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("settings.ron");
    save(&Config::default(), &path).expect("save");
    std::fs::write(dir.join(LEGACY_FOLDERS_FILE), r#"(folders: ["/removed"])"#).expect("write");

    let outcome = load(&path).expect("a v1 file loads");
    assert!(outcome.config.library.folders.is_empty(), "a folder removed after the migration does not come back");
    let _ = std::fs::remove_dir_all(&dir);
}
