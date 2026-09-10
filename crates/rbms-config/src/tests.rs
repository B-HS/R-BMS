//! The configuration document: its shipped values, the vocabularies its rows are stored under, and
//! the skin choices a player makes for each screen.
//!
//! Three neighbours carry the rest, so no one file has to be read whole to change one row: `audio`
//! for the sound rows and the steps they move in, `library` for the folder and table lists, and
//! `persistence` for migration and for what reaches the disk.

mod audio;
mod library;
mod persistence;

use rbms_chart::shuffle::NoteOption;
use rbms_judge::GaugeKind;
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::gauge::GaugeAutoShift;
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;

use super::*;
use rbms_skin::dst::{OffsetSource, SkinOffset};

/// The `SkinType` id of the song browser, used to prove that two screens keep their own choices.
const MUSIC_SELECT_SCREEN: i32 = 5;

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
    assert_eq!(c.judge.target, ScoreTarget::RateAaa);
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
/// Every target the row offers survives a round trip through the file, and the eleven fixed rates
/// name the eleven the reference has (`TargetProperty.java:117-141`).
#[test]
fn every_target_the_row_offers_survives_the_file_it_is_stored_in() {
    let mut ids: Vec<&str> = Vec::new();
    let mut rates = 0;
    for target in ScoreTarget::ALL {
        assert_eq!(target_from_token(target.token()), target, "{target:?} did not come back from its own token");
        assert_eq!(target_from_token(&target.token().to_ascii_lowercase()), target, "{target:?} is case sensitive");
        if let Some(id) = target.rate_id() {
            rates += 1;
            ids.push(id);
        }
    }
    assert_eq!(rates, 11, "the row offers a different number of fixed rates from the reference");
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "two targets name the same rate");
    assert_eq!(TARGET_LABELS.len(), ScoreTarget::ALL.len(), "the row's names and its values are different lengths");
}

#[test]
fn an_unknown_judge_token_falls_back_to_the_shipped_value() {
    assert_eq!(algorithm_from_token("nonsense"), JudgeAlgorithm::default());
    assert_eq!(ln_mode_from_token(""), LnMode::LongNote);
    assert_eq!(gauge_auto_shift_from_token("SIDEWAYS"), GaugeAutoShift::None);
    assert_eq!(gauge_set_from_token("BEAT_7K"), None, "a mode key is not a choice this row offers");
    assert_eq!(target_from_token("nonsense"), ScoreTarget::RateAaa);
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
    c.play.hispeed_step = 0.05;
    c.play.fix_hispeed = FixHiSpeed::MinBpm;
    c.play.lane_option = LaneOption::BattleAutoScratch;
    c.play.legacy_note = true;
    c.play.enable_lift = true;
    c.play.enable_cover = false;
    c.play.hidden = 0.22;
    c.play.enable_hidden = true;
    c.play.lanecover_step_fine = 0.002;
    c.play.play_escape = PlayEscape::Hold;
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
    c.display.result_graphs = false;
    c.display.show_white_number = true;
    c.display.judge_text_y = 0.42;
    c.display.letterbox = true;
    c.display.five_key_layout = true;
    c.library.preview = false;
    c.library.preview_volume = 0.4;
    c.library.preview_fade_ms = 350;
    c.library.sort = SortMode::LastUpdate;
    c.library.favorite_only = true;
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
    c.skin.folder = Some("/skins".into());
    c.skin.screen = MUSIC_SELECT_SCREEN;
    c.skin.select(MUSIC_SELECT_SCREEN, Some("/skins/browser/browser.json".into()));
    let custom = c.skin.customise("/skins/browser/browser.json");
    custom.properties.insert("LANE COVER".into(), 902);
    custom.filepaths.insert("BACKGROUND".into(), "night.png".into());
    custom.offsets.insert(12, SkinOffset { x: -4.0, y: 8.0, w: 0.0, h: 0.0, r: 90.0, a: -32.0 });

    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back, c, "every field survives the round trip");
}

/// The renderer reads what the player nudged through the offset ids the document declared, and
/// answers nothing for an id nothing was moved under -- which is what leaves that destination where
/// its author put it.
#[test]
fn a_stored_customisation_answers_the_offset_ids_it_holds() {
    let mut c = Config::default();
    let nudge = SkinOffset { x: -4.0, y: 8.0, w: 0.0, h: 0.0, r: 90.0, a: -32.0 };
    c.skin.customise("browser.json").offsets.insert(12, nudge);

    let stored = c.skin.customisation("browser.json").expect("the document has choices");
    assert_eq!(stored.offset(12), Some(nudge));
    assert_eq!(stored.offset(13), None, "an id nothing was nudged under reported a nudge");
}

/// The screen a customisation belongs to is addressed by the `SkinType` id the document writes, so
/// a document moved between folders keeps neither more nor less than its own choices.
#[test]
fn a_skin_choice_is_kept_per_screen_and_per_document() {
    let mut c = Config::default();
    assert_eq!(c.skin.screen, DEFAULT_SKIN_SCREEN);
    assert_eq!(c.skin.document(DEFAULT_SKIN_SCREEN), None, "a fresh install draws every screen with the built-in one");

    c.skin.select(DEFAULT_SKIN_SCREEN, Some("play.json".into()));
    c.skin.select(MUSIC_SELECT_SCREEN, Some("select.json".into()));
    assert_eq!(c.skin.document(DEFAULT_SKIN_SCREEN), Some("play.json"));
    assert_eq!(c.skin.document(MUSIC_SELECT_SCREEN), Some("select.json"));

    c.skin.customise("play.json").properties.insert("NOTE".into(), 15);
    assert_eq!(c.skin.user_config("play.json").properties.get("NOTE"), Some(&15));
    assert!(c.skin.user_config("select.json").properties.is_empty(), "one document's choices reached another");

    c.skin.select(DEFAULT_SKIN_SCREEN, None);
    assert_eq!(c.skin.document(DEFAULT_SKIN_SCREEN), None, "the built-in screen is what no entry means");
    assert_eq!(c.skin.user_config("play.json").properties.get("NOTE"), Some(&15), "unselecting a document threw its choices away");

    c.skin.forget("play.json");
    assert!(c.skin.user_config("play.json").properties.is_empty());
}

/// A document is handed to the loader under the path it was found at, with the three choice maps
/// the player filled in, so the loader needs to know nothing about the settings file.
#[test]
fn the_loader_is_handed_the_choices_made_for_the_document_it_is_loading() {
    let mut c = Config::default();
    let custom = c.skin.customise("play.json");
    custom.properties.insert("GAUGE".into(), 45);
    custom.filepaths.insert("NOTE".into(), "square.png".into());
    custom.offsets.insert(3, SkinOffset { y: 12.0, ..SkinOffset::default() });

    let user = c.skin.user_config("play.json");
    assert_eq!(user.path, "play.json");
    assert_eq!(user.properties.get("GAUGE"), Some(&45));
    assert_eq!(user.filepaths.get("NOTE").map(String::as_str), Some("square.png"));
    assert_eq!(user.offset(3), Some(SkinOffset { y: 12.0, ..SkinOffset::default() }));
    assert_eq!(user.offset(4), None, "an offset nobody nudged came back as a nudge of nothing");
}

/// A hand-edited file naming a screen no `SkinType` declares is pulled back onto one that does,
/// rather than leaving the SKIN tab pointing at a row it cannot show.
#[test]
fn a_hand_edited_skin_group_is_pulled_back_onto_a_screen_that_exists() {
    let mut c = Config::default();
    c.skin.screen = SKIN_SCREEN_LABELS.len() as i32;
    c.skin.folder = Some("   ".into());
    c.skin.selected.insert(-1, "ghost.json".into());
    c.skin.selected.insert(MUSIC_SELECT_SCREEN, "  ".into());
    c.sanitise();
    assert_eq!(c.skin.screen, DEFAULT_SKIN_SCREEN);
    assert_eq!(c.skin.folder, None, "a blank folder is no folder");
    assert!(c.skin.selected.is_empty(), "a screen that does not exist, and a document with no name, were both kept");
}

/// The three new option axes are stored as separator-free tokens, exactly like the gauge next to
/// them, so a reworded label leaves what is on disk alone.
#[test]
fn the_new_play_option_axes_are_stored_as_their_tokens() {
    let mut c = Config::default();
    c.play.fix_hispeed = FixHiSpeed::StartBpm;
    c.play.lane_option = LaneOption::Battle;
    c.play.play_escape = PlayEscape::Double;
    c.library.sort = SortMode::MissCount;
    let text = ron_of(&c);
    for token in ["\"STARTBPM\"", "\"BATTLE\"", "\"DOUBLE\"", "\"MISSCOUNT\""] {
        assert!(text.contains(token), "{token} is not in the file: {text}");
    }
    let back: Config = ron::from_str(&text).expect("tokens parse back");
    assert_eq!(back.play.fix_hispeed, FixHiSpeed::StartBpm);
    assert_eq!(back.play.lane_option, LaneOption::Battle);
    assert_eq!(back.play.play_escape, PlayEscape::Double);
    assert_eq!(back.library.sort, SortMode::MissCount);
}

/// A file written before these rows existed still loads: every new field falls back to the value a
/// fresh install holds, which is the one that leaves the program behaving as it did.
#[test]
fn a_file_from_before_the_new_rows_loads_with_them_at_their_shipped_values() {
    let c: Config =
        ron::from_str(r#"(schema_version: 2, play: (hispeed: 3.0), display: (skin: "WIDE"), library: (preview: false))"#).expect("an older file parses");
    assert!((c.play.hispeed - 3.0).abs() < 1e-9);
    assert!((c.play.hispeed_step - HISPEED_STEP).abs() < 1e-9);
    assert_eq!(c.play.fix_hispeed, FixHiSpeed::default());
    assert_eq!(c.play.lane_option, LaneOption::Off);
    assert_eq!(c.play.play_escape, PlayEscape::Immediate);
    assert!(!c.play.legacy_note);
    assert!(c.play.enable_cover);
    assert!(!c.play.enable_lift);
    assert!(!c.play.enable_hidden);
    assert!((c.play.hidden - LANE_SHADE_MIN).abs() < 1e-9);
    assert!((c.play.lanecover_step_fine - DEFAULT_LANE_SHADE_FINE_STEP).abs() < 1e-9);
    assert!(c.display.result_graphs);
    assert!(!c.display.show_white_number);
    assert!(!c.display.letterbox);
    assert!(!c.display.five_key_layout);
    assert!((c.display.judge_text_y - JUDGE_TEXT_Y_FROM_SKIN).abs() < 1e-9);
    assert_eq!(c.library.sort, SortMode::Default);
    assert!(!c.library.favorite_only);
    assert!((c.library.preview_volume - DEFAULT_PREVIEW_VOLUME).abs() < 1e-6);
    assert_eq!(c.library.preview_fade_ms, DEFAULT_PREVIEW_FADE_MS);
}

/// Every new value a hand-edited file could put out of range is pulled back by `sanitise`, which is
/// what runs over whatever comes off disk or down from an account.
#[test]
fn the_new_ranged_values_are_pulled_back_into_their_rows_ranges() {
    let mut c = Config::default();
    c.play.hispeed_step = 99.0;
    c.play.hidden = -4.0;
    c.play.lanecover_step_fine = 5.0;
    c.display.judge_text_y = 8.0;
    c.library.preview_volume = -1.0;
    c.library.preview_fade_ms = 99_999;
    c.sanitise();
    assert!((c.play.hispeed_step - HISPEED_STEP_MAX).abs() < 1e-9);
    assert!((c.play.hidden - LANE_SHADE_MIN).abs() < 1e-9);
    assert!((c.play.lanecover_step_fine - LANE_SHADE_FINE_STEP_MAX).abs() < 1e-9);
    assert!((c.display.judge_text_y - JUDGE_TEXT_Y_MAX).abs() < 1e-9);
    assert!((c.library.preview_volume - PREVIEW_VOLUME_MIN).abs() < 1e-6);
    assert_eq!(c.library.preview_fade_ms, PREVIEW_FADE_MAX_MS);
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
