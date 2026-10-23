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
use rbms_judge::gauge_tables::{CourseGaugeMode, GaugeSetId};
use rbms_judge::ln::LnMode;

use super::*;
use rbms_skin::dst::{OffsetSource, SkinColor, SkinOffset};

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
    assert_eq!(c.judge.target_fallback, ScoreTarget::RateAaa, "a fresh install paces against the shipped target when its own score is missing");
    assert_eq!(c.judge.target_rate_percent, TARGET_RATE_DEFAULT_PERCENT);
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
    assert!(c.network.auto_upload_trace, "a run's note-by-note record goes up after its score by default (decision Q5 of 2026-10-19)");
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
    for mode in CourseGaugeMode::ALL {
        assert_eq!(course_gauge_from_token(course_gauge_token(mode)), mode);
    }
    for target in ScoreTarget::ALL {
        assert_eq!(target_from_token(target.token()), target);
    }
}

/// A fresh install plays courses on the LR2 table, which is the player's decision of 2026-10-23.
#[test]
fn the_course_gauge_defaults_to_lr2() {
    assert_eq!(Config::default().judge.course_gauge, CourseGaugeMode::Lr2);
    assert_eq!(CourseGaugeMode::default(), CourseGaugeMode::Lr2);
}

/// The COURSE GAUGE row prints the tokens the file stores, in the order the setting cycles through
/// the modes, and every one of them comes back from the file as the mode it was written from.
#[test]
fn course_gauge_tokens_round_trip() {
    let tokens: Vec<&str> = CourseGaugeMode::ALL.into_iter().map(course_gauge_token).collect();
    assert_eq!(tokens, COURSE_GAUGE_LABELS, "the row prints other names than the file stores");
    assert_eq!(tokens, ["LR2", "BEATORAJA", "IIDX"]);
    for mode in CourseGaugeMode::ALL {
        assert_eq!(course_gauge_from_token(&course_gauge_token(mode).to_ascii_lowercase()), mode, "{mode:?} is case sensitive");
        let mut config = Config::default();
        config.judge.course_gauge = mode;
        let text = ron_of(&config);
        assert!(text.contains(&format!("course_gauge: \"{}\"", course_gauge_token(mode))), "{mode:?} is not stored as its token");
        let back: Config = ron::from_str(&text).expect("a config round-trips");
        assert_eq!(back.judge.course_gauge, mode);
    }
}

/// A token no mode is stored under reads as the shipped LR2 rather than failing the whole load.
#[test]
fn an_unknown_course_gauge_token_reads_lr2() {
    for unknown in ["nonsense", "", "BEAT_7K", "AUTO"] {
        assert_eq!(course_gauge_from_token(unknown), CourseGaugeMode::Lr2, "`{unknown}`");
        let text = format!(r#"(schema_version: {CURRENT_SCHEMA_VERSION}, judge: (course_gauge: "{unknown}"))"#);
        let read: Config = ron::from_str(&text).expect("a file with an unreadable course gauge parses");
        assert_eq!(read.judge.course_gauge, CourseGaugeMode::Lr2, "`{unknown}`");
    }
}

/// A settings file written before the COURSE GAUGE row holds none, and a file of the current schema
/// that lacks it reads as the shipped table without a migration.
#[test]
fn a_settings_file_without_a_course_gauge_reads_lr2() {
    let without = format!("(schema_version: {CURRENT_SCHEMA_VERSION}, judge: (offset_ms: 5))");
    let before: Config = ron::from_str(&without).expect("a file from before the row loads");
    assert_eq!(before.judge.offset_ms, 5);
    assert_eq!(before.judge.course_gauge, CourseGaugeMode::Lr2);
    let (migrated, from) = migrate(&without).expect("a file of the current schema loads");
    assert_eq!(from, None, "adding the row needed no new schema");
    assert_eq!(migrated.judge.course_gauge, CourseGaugeMode::Lr2);
    let stated: Config =
        ron::from_str(&format!(r#"(schema_version: {CURRENT_SCHEMA_VERSION}, judge: (course_gauge: "IIDX"))"#)).expect("a stated course gauge loads");
    assert_eq!(stated.judge.course_gauge, CourseGaugeMode::Iidx);
}

/// The single-song GAUGE SET row keeps offering the chart's own table and LR2 only: the IIDX table
/// the engine gained is reached through COURSE GAUGE, so it must not leak into the row's cycle, its
/// tokens or what a file may state.
#[test]
fn the_single_song_gauge_set_cycle_stays_auto_and_lr2() {
    assert_eq!(GAUGE_SET_CYCLE, [None, Some(GaugeSetId::Lr2)]);
    assert_eq!(GAUGE_SET_LABELS, [AUTO_VALUE, "LR2"]);
    assert_eq!(gauge_set_from_token("IIDX"), None, "a table only a course reaches read as a choice of the single-song row");
    assert_eq!(gauge_set_from_token("iidx"), None);
    let read: Config =
        ron::from_str(&format!(r#"(schema_version: {CURRENT_SCHEMA_VERSION}, judge: (gauge_set: "IIDX"))"#)).expect("a file stating the IIDX table parses");
    assert_eq!(read.judge.gauge_set, None);
    let mut config = Config::default();
    for _ in 0..=GAUGE_SET_CYCLE.len() {
        adjust(&mut config, SettingId::GaugeSet, 1);
        assert!(GAUGE_SET_CYCLE.contains(&config.judge.gauge_set), "the row stepped to {:?}", config.judge.gauge_set);
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

/// The fallback row offers exactly the targets that are worked out from the chart alone, each
/// under its own name and its own token, and none that names somebody's score.
#[test]
fn the_fallback_row_offers_the_targets_that_are_always_known() {
    assert_eq!(TARGET_FALLBACK_LABELS.len(), ScoreTarget::FALLBACKS.len(), "the row's names and its values are different lengths");
    assert_eq!(ScoreTarget::FALLBACKS.len(), TARGET_FALLBACK_COUNT);
    for (at, target) in ScoreTarget::FALLBACKS.into_iter().enumerate() {
        assert!(target.is_always_known() && target.missing_score().is_none(), "{target:?} needs a score and cannot stand in for another");
        assert_eq!(TARGET_FALLBACK_LABELS[at], target.label(), "the row names {target:?} differently from the target row");
        assert_eq!(target_fallback_from_token(target.token()), target);
    }
    for target in [ScoreTarget::LocalBest, ScoreTarget::IrBest, ScoreTarget::Rival] {
        assert!(!target.is_always_known() && target.missing_score().is_some(), "{target:?} is worked out from the chart alone");
        assert_eq!(target_fallback_from_token(target.token()), ScoreTarget::default(), "{target:?} was taken for a fallback");
    }
    assert_eq!(ScoreTarget::ALL[..TARGET_FALLBACK_COUNT], ScoreTarget::FALLBACKS, "the fallbacks are the first targets of the row, in its order");
}

/// What a row says of a target whose score may not be there: the target's own name, then what is
/// missing and what stands in.
#[test]
fn a_target_that_may_have_no_score_says_what_stands_in_for_it() {
    assert_eq!(ScoreTarget::LocalBest.label_with_fallback(ScoreTarget::RateAaa, 85), "LOCAL BEST (NO RECORD: AAA)");
    assert_eq!(ScoreTarget::IrBest.label_with_fallback(ScoreTarget::Max, 85), "IR BEST (NO IR SCORE: MAX)");
    assert_eq!(ScoreTarget::Rival.label_with_fallback(ScoreTarget::RankNext, 85), "RIVAL (NO RIVAL SCORE: RANK NEXT)");
    assert_eq!(ScoreTarget::LocalBest.label_then(ScoreTarget::RateAaa, 85), "LOCAL BEST > AAA");
    assert_eq!(ScoreTarget::Rival.label_then(ScoreTarget::RankNext, 85), "RIVAL > RANK NEXT");
    assert_eq!(ScoreTarget::RateA.label_then(ScoreTarget::Max, 85), "RATE A", "a target that is always known is its name alone");
    assert_eq!(ScoreTarget::LocalBest.missing_note(ScoreTarget::RateAaa, 85).as_deref(), Some("NO RECORD: PACED AGAINST RATE AAA INSTEAD"));
    assert_eq!(ScoreTarget::IrBest.missing_note(ScoreTarget::Max, 85).as_deref(), Some("NO IR SCORE: YOUR LOCAL BEST, THEN MAX"));
    assert_eq!(ScoreTarget::IrBest.label_then(ScoreTarget::RateMaxMinus, 85), "IR BEST > LOCAL > MAX-", "the local record is tried before the fallback");
    assert_eq!(ScoreTarget::RankNext.missing_note(ScoreTarget::Max, 85), None);
    assert_eq!(ScoreTarget::RateAaaPlus.short_name_at(85), "AAA+");
    assert_eq!(ScoreTarget::RateMaxMinus.short_name_at(85), "MAX-");
    assert_eq!(ScoreTarget::LocalBest.short_name_at(85), "LOCAL BEST");
    assert_eq!(ScoreTarget::RateCustom.short_name_at(72), "72%", "a typed share is written as the share");
    assert_eq!(ScoreTarget::RateCustom.name_at(72), "SCORE RATE 72%");
    assert_eq!(ScoreTarget::RateAa.name_at(72), "RATE AA");
    assert_eq!(ScoreTarget::LocalBest.label_with_fallback(ScoreTarget::RateCustom, 72), "LOCAL BEST (NO RECORD: 72%)");
    assert_eq!(ScoreTarget::LocalBest.label_then(ScoreTarget::RateCustom, 72), "LOCAL BEST > 72%");
    assert_eq!(ScoreTarget::LocalBest.missing_note(ScoreTarget::RateCustom, 72).as_deref(), Some("NO RECORD: PACED AGAINST SCORE RATE 72% INSTEAD"));
    for target in ScoreTarget::FALLBACKS {
        assert_eq!(target.label_with_fallback(ScoreTarget::RateA, 85), target.label(), "{target:?} is always known and has nothing to fall back from");
    }
}

/// A file stores a target by its token, and one that stored the label a row prints (written by
/// hand, or by a build that stored it so) is still the target it names. Reading it as the shipped
/// target would silently pace a LOCAL BEST player against RATE AAA.
#[test]
fn a_target_stored_by_its_printed_label_is_still_that_target() {
    for target in ScoreTarget::ALL {
        assert_eq!(target_from_token(target.label()), target, "{target:?} is lost when stored as `{}`", target.label());
        assert_eq!(target_from_token(&target.label().to_ascii_lowercase().replace(' ', "_")), target, "{target:?} is lost when stored in snake case");
    }
    assert_eq!(target_from_token("  local best "), ScoreTarget::LocalBest);
    assert_eq!(target_from_token("RATE AAA+"), ScoreTarget::RateAaaPlus);
    assert_eq!(target_from_token("RATE AAA-"), ScoreTarget::RateAaaMinus);
}

/// A file from before the fallback existed reads with the shipped one, whatever target it holds; a
/// file that states one reads it back; and one that states a target that needs a score to exist
/// reads as the shipped one rather than a fallback that could itself be missing.
#[test]
fn the_fallback_target_reads_from_older_files_and_from_bad_ones() {
    let older: Config = ron::from_str(r#"(schema_version: 5, judge: (target: "LOCALBEST"))"#).expect("a file with no fallback parses");
    assert_eq!((older.judge.target, older.judge.target_fallback), (ScoreTarget::LocalBest, ScoreTarget::RateAaa));
    let stated: Config =
        ron::from_str(r#"(schema_version: 5, judge: (target: "LOCALBEST", target_fallback: "RATEAPLUS"))"#).expect("a file with a fallback parses");
    assert_eq!((stated.judge.target, stated.judge.target_fallback), (ScoreTarget::LocalBest, ScoreTarget::RateAPlus));
    assert_eq!(older.judge.target_rate_percent, TARGET_RATE_DEFAULT_PERCENT, "a file from before the custom rate reads with the shipped one");
    let stated_rate: Config =
        ron::from_str(r#"(schema_version: 5, judge: (target: "RATECUSTOM", target_fallback: "RATECUSTOM", target_rate_percent: 91))"#).expect("parses");
    assert_eq!(
        (stated_rate.judge.target, stated_rate.judge.target_fallback, stated_rate.judge.target_rate_percent),
        (ScoreTarget::RateCustom, ScoreTarget::RateCustom, 91)
    );
    for (stated, read) in [(0, TARGET_RATE_MIN_PERCENT), (-5, TARGET_RATE_MIN_PERCENT), (101, TARGET_RATE_MAX_PERCENT), (50, 50)] {
        let mut config = Config::default();
        config.judge.target_rate_percent = stated;
        config.sanitise();
        assert_eq!(config.judge.target_rate_percent, read, "a stated rate of {stated}");
    }
    for bad in ["LOCALBEST", "IRBEST", "RIVAL", "nonsense", ""] {
        let text = format!(r#"(schema_version: 5, judge: (target_fallback: "{bad}"))"#);
        let read: Config = ron::from_str(&text).expect("a file with an unreadable fallback parses");
        assert_eq!(read.judge.target_fallback, ScoreTarget::default(), "`{bad}` was taken for a fallback");
    }
    let mut set_by_hand = Config::default();
    set_by_hand.judge.target_fallback = ScoreTarget::Rival;
    set_by_hand.sanitise();
    assert_eq!(set_by_hand.judge.target_fallback, ScoreTarget::default(), "sanitising left a fallback that needs a score");
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
    assert_eq!(config.judge.offset_ms, 20, "the offset is carried over to the end it is counted from now");
    assert!(config.judge.auto_offset, "the rest of the group comes across untouched");
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
    c.play.key_debounce_ms = 5;
    c.play.scratch_auto = true;
    c.play.scratch_auto_color = Some(SkinColor::rgba(255, 64, 192, 128));
    c.play.autoplay = false;
    c.play.random = NoteOption::Mirror;
    c.play.constant_speed = true;
    c.play.total_override = 320.0;
    c.play.auto_replay = false;
    c.play.hispeed_step = 0.05;
    c.play.fix_hispeed = FixHiSpeed::MinBpm;
    c.play.keep_green_number = true;
    c.play.duration_ms = 613;
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
    c.judge.course_gauge = CourseGaugeMode::Iidx;
    c.judge.gauge_auto_shift = GaugeAutoShift::BestClear;
    c.judge.bottom_shiftable_gauge = GaugeKind::Normal;
    c.judge.target = ScoreTarget::IrBest;
    c.judge.target_fallback = ScoreTarget::RateAaPlus;
    c.judge.target_rate_percent = 72;
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
    c.display.window_resolution = WindowResolution::FullHd1080;
    c.display.window_mode = WindowMode::Borderless;
    c.display.five_key_layout = true;
    c.library.preview = false;
    c.library.preview_volume = 0.4;
    c.library.preview_fade_ms = 350;
    c.library.sort = SortMode::LastUpdate;
    c.library.favorite_only = true;
    c.library.songs_folder = Some("/songs".into());
    c.library.folders = vec!["/a".into(), "/b/c".into()];
    c.library.tables = vec![TableSource { name: "Insane".into(), location: "https://example.com/insane.json".into(), group: String::new() }];
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
    c.skin.pack = Some("/skins/pack".into());
    c.skin.screen = MUSIC_SELECT_SCREEN;
    c.skin.select(MUSIC_SELECT_SCREEN, Some("/skins/browser/browser.json".into()));
    let custom = c.skin.customise("/skins/browser/browser.json");
    custom.properties.insert("LANE COVER".into(), 902);
    custom.filepaths.insert("BACKGROUND".into(), "night.png".into());
    custom.offsets.insert(12, SkinOffset { x: -4.0, y: 8.0, w: 0.0, h: 0.0, r: 90.0, a: -32.0 });
    custom.colors.insert(5, SkinColor::rgba(48, 224, 80, 128));

    let back: Config = ron::from_str(&ron_of(&c)).expect("a config round-trips");
    assert_eq!(back, c, "every field survives the round trip");
}

/// A settings file from before the SCRATCH AUTO colour holds none, which reads as the shipped green.
#[test]
fn a_settings_file_without_a_scratch_auto_colour_holds_the_shipped_one() {
    let before: Config = ron::from_str("(play: (scratch_auto: true))").expect("a file from before the colour loads");
    assert!(before.play.scratch_auto);
    assert_eq!(before.play.scratch_auto_color, None);
    assert_eq!(Config::default().play.scratch_auto_color, None);
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

/// A colour is read the way a nudge is: by the id the document declared, answering nothing for an id
/// nothing was set under, which is what leaves that destination in the colour its author gave it.
#[test]
fn a_stored_customisation_answers_the_colour_ids_it_holds() {
    let mut c = Config::default();
    let chosen = SkinColor::rgba(48, 224, 80, 128);
    c.skin.customise("play.json").colors.insert(5, chosen);

    let stored = c.skin.customisation("play.json").expect("the document has choices");
    assert_eq!(stored.color(5), Some(chosen));
    assert_eq!(stored.color(6), None, "an id nothing was set under reported a colour");
    assert_eq!(c.skin.user_config("play.json").colors.get(&5), Some(&chosen), "the loader is handed the colour");
    assert!(c.skin.user_config("select.json").colors.is_empty(), "one document's colour reached another");
}

/// A settings file from before colours existed holds no `colors`, and a hand-written colour that
/// names no alpha is a visible one.
#[test]
fn a_skin_customisation_without_colours_loads_and_one_without_alpha_is_opaque() {
    let before: Config = ron::from_str(r#"(skin: (custom: { "play.json": (properties: { "NOTE": 15 }) }))"#).expect("a file from before colours loads");
    let stored = before.skin.customisation("play.json").expect("the document has choices");
    assert_eq!(stored.properties.get("NOTE"), Some(&15));
    assert!(stored.colors.is_empty());

    let written: Config = ron::from_str(r#"(skin: (custom: { "play.json": (colors: { 3: (r: 200, g: 10) }) }))"#).expect("a hand-written colour loads");
    assert_eq!(written.skin.user_config("play.json").colors.get(&3), Some(&SkinColor::rgba(200, 10, 0, u8::MAX)));
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
/// rather than leaving the skin menu pointing at a row it cannot show.
#[test]
fn a_hand_edited_skin_group_is_pulled_back_onto_a_screen_that_exists() {
    let mut c = Config::default();
    c.skin.screen = SKIN_SCREEN_LABELS.len() as i32;
    c.skin.folder = Some("   ".into());
    c.skin.pack = Some(" ".into());
    c.skin.selected.insert(-1, "ghost.json".into());
    c.skin.selected.insert(MUSIC_SELECT_SCREEN, "  ".into());
    c.sanitise();
    assert_eq!(c.skin.screen, DEFAULT_SKIN_SCREEN);
    assert_eq!(c.skin.folder, None, "a blank folder is no folder");
    assert_eq!(c.skin.pack, None, "a blank pack is no pack");
    assert!(c.skin.selected.is_empty(), "a screen that does not exist, and a document with no name, were both kept");
}

/// A settings file written before a skin pack could be named has no `pack` line. It loads with no
/// pack and with every choice it did hold, and at the schema version it was written at: the new line
/// has a default, so nothing has to be migrated.
#[test]
fn a_file_from_before_the_skin_pack_loads_with_no_pack_and_everything_it_chose() {
    let text = format!(
        r#"(schema_version: {CURRENT_SCHEMA_VERSION}, skin: (folder: Some("/skins"), screen: {MUSIC_SELECT_SCREEN}, selected: {{{MUSIC_SELECT_SCREEN}: "/skins/browser.json"}}, custom: {{"/skins/browser.json": (properties: {{"COVER": 902}})}}))"#
    );
    let c: Config = ron::from_str(&text).expect("a file with no pack line parses");
    assert_eq!(c.skin.pack, None);
    assert_eq!(c.skin.pack_folder(), None);
    assert_eq!(c.skin.folder.as_deref(), Some("/skins"));
    assert_eq!(c.skin.document(MUSIC_SELECT_SCREEN), Some("/skins/browser.json"));
    assert_eq!(c.skin.user_config("/skins/browser.json").properties.get("COVER"), Some(&902));

    let mut named = c.clone();
    named.skin.pack = Some("/skins/pack".into());
    let back: Config = ron::from_str(&ron_of(&named)).expect("a file with a pack line parses");
    assert_eq!(back.skin.pack_folder(), Some("/skins/pack"));
    assert_eq!(back.skin.document(MUSIC_SELECT_SCREEN), Some("/skins/browser.json"), "naming a pack dropped a choice made by hand");
    assert_eq!(display_value(&back, SettingId::SkinPack), "/skins/pack");
    assert_eq!(display_value(&c, SettingId::SkinPack), NONE_VALUE);
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
    assert!(!c.play.keep_green_number, "a file that never asked for a held green number scrolls by its stored speed, as it did");
    assert_eq!(c.play.duration_ms, DEFAULT_DURATION_MS);
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
    assert!(c.display.letterbox, "a file that never stated the row takes the shipped fit");
    assert_eq!(c.display.window_resolution, WindowResolution::Hd720);
    assert_eq!(c.display.window_mode, WindowMode::Windowed);
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
    c.play.key_debounce_ms = 99_999;
    c.sanitise();
    assert!((c.play.hispeed_step - HISPEED_STEP_MAX).abs() < 1e-9);
    assert!((c.play.hidden - LANE_SHADE_MIN).abs() < 1e-9);
    assert!((c.play.lanecover_step_fine - LANE_SHADE_FINE_STEP_MAX).abs() < 1e-9);
    assert!((c.display.judge_text_y - JUDGE_TEXT_Y_MAX).abs() < 1e-9);
    assert!((c.library.preview_volume - PREVIEW_VOLUME_MIN).abs() < 1e-6);
    assert_eq!(c.library.preview_fade_ms, PREVIEW_FADE_MAX_MS);
    assert_eq!(c.play.key_debounce_ms, KEY_DEBOUNCE_MAX_MS);
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
    c.play.duration_ms = 0;
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
    assert_eq!(c.play.duration_ms, DURATION_MIN_MS);
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

/// The `SkinType` id of the screen shown once a song is chosen.
const DECIDE_SCREEN: i32 = 6;

/// The `SkinType` id of the result screen.
const RESULT_SCREEN: i32 = 7;

/// The skin folder a configuration that names none looks in, on a machine the fixtures below pretend
/// to have been written on.
const DEFAULT_SKIN_ROOT: &str = "/home/p/.config/rbms/skin";

/// A document the fixtures keep outside every skin folder.
const OUTSIDE_DOCUMENT: &str = "/elsewhere/mine/decide.json5";

/// The DISPLAY preset the retired bundle was switched on with, spelled the way the build that
/// offered it stored it.
fn retired_preset() -> String {
    crate::legacy::RETIRED_BUNDLE_DIRECTORY.replace('-', " ").to_ascii_uppercase()
}

/// The directory one generation of the retired bundle was installed into; the first carried no
/// number.
fn retired_directory(generation: Option<u32>) -> String {
    let first = crate::legacy::RETIRED_BUNDLE_DIRECTORY;
    match generation {
        Some(number) => format!("{first}{}{number}", crate::legacy::RETIRED_BUNDLE_GENERATION_INFIX),
        None => first.to_string(),
    }
}

/// A file written by the last build that shipped a skin bundle of its own comes back with every
/// trace of that bundle gone -- the preset, the screens pointing into its folders, the choices stored
/// for those documents and the bundle-wide choices -- while a document the player picked from
/// anywhere else keeps both its screen and its choices.
#[test]
fn a_bundled_skin_document_migrates_to_the_built_in_screens_and_keeps_what_the_player_chose() {
    let current = format!("{DEFAULT_SKIN_ROOT}/{}/play-7k.json5", retired_directory(Some(3)));
    let first = format!("{DEFAULT_SKIN_ROOT}/{}/select.json5", retired_directory(None));
    let unselected = format!("{DEFAULT_SKIN_ROOT}/{}/result.json5", retired_directory(Some(2)));
    let neighbour = format!("{DEFAULT_SKIN_ROOT}/aurora/result.json5");
    let raw = format!(
        r#"(
            schema_version: 2,
            play: (hispeed: 3.5),
            display: (skin: "{preset}"),
            skin: (
                screen: {MUSIC_SELECT_SCREEN},
                selected: {{
                    {DEFAULT_SKIN_SCREEN}: "{current}",
                    {MUSIC_SELECT_SCREEN}: "{first}",
                    {DECIDE_SCREEN}: "{OUTSIDE_DOCUMENT}",
                    {RESULT_SCREEN}: "{neighbour}",
                }},
                custom: {{
                    "{current}": (properties: {{ "LANE COVER": 902 }}),
                    "{unselected}": (properties: {{ "BACKGROUND": 911 }}),
                    "{OUTSIDE_DOCUMENT}": (properties: {{ "EFFECT": 991 }}, filepaths: {{ "BACKDROP": "night.png" }}),
                    "{neighbour}": (offsets: {{ 40: (x: 6.0, y: 0.0, w: 0.0, h: 0.0, r: 0.0, a: 0.0) }}),
                }},
                shared: {{ "{bundle}": (properties: {{ "PLAY SIDE": 901 }}), "aurora": (properties: {{ "PLAY SIDE": 900 }}) }},
            ),
        )"#,
        preset = retired_preset(),
        bundle = retired_directory(Some(3)),
    );

    let (config, from) = migrate(&raw).expect("a schema 2 file migrates");
    assert_eq!(from, Some(BUNDLED_SKIN_SCHEMA_VERSION));
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.display.skin, DEFAULT_SKIN, "the preset that switched the retired bundle on was kept");
    assert!((config.play.hispeed - 3.5).abs() < 1e-9, "the rest of the document did not come across");
    assert_eq!(config.skin.screen, MUSIC_SELECT_SCREEN, "the screen the skin menu was left on moved");

    assert_eq!(config.skin.document(DEFAULT_SKIN_SCREEN), None, "a screen still points at the retired bundle's current generation");
    assert_eq!(config.skin.document(MUSIC_SELECT_SCREEN), None, "a screen still points at the retired bundle's first generation");
    assert_eq!(config.skin.document(DECIDE_SCREEN), Some(OUTSIDE_DOCUMENT), "a document the player picked from outside the skin folder was dropped");
    assert_eq!(config.skin.document(RESULT_SCREEN), Some(neighbour.as_str()), "a document from another folder of the skin folder was dropped");

    assert_eq!(config.skin.customisation(&current), None, "choices were kept for a document of the retired bundle");
    assert_eq!(config.skin.customisation(&unselected), None, "choices were kept for a retired document no screen was using");
    let outside = config.skin.user_config(OUTSIDE_DOCUMENT);
    assert_eq!(outside.properties.get("EFFECT"), Some(&991), "the player's own document lost a choice");
    assert_eq!(outside.filepaths.get("BACKDROP").map(String::as_str), Some("night.png"));
    assert_eq!(config.skin.user_config(&neighbour).offset(40), Some(SkinOffset { x: 6.0, ..SkinOffset::default() }));
    assert!(config.skin.user_config(&neighbour).properties.is_empty(), "a bundle-wide choice outlived the scope it was stored under");

    let written = ron_of(&config);
    for dropped in ["shared", crate::legacy::RETIRED_BUNDLE_DIRECTORY] {
        assert!(!written.contains(dropped), "{dropped} is still written: {written}");
    }
    let (again, from) = migrate(&written).expect("the migrated document parses");
    assert_eq!(from, None, "the file the migration wrote is not current");
    assert_eq!(again, config);
}

/// A bundle was only ever installed directly below the skin folder, so that is the only place a
/// path is taken for one of its documents: the folder the player named when they named one. A
/// folder that merely starts with the same words, a document lying in the skin folder itself and a
/// copy kept somewhere else all belong to the player.
#[test]
fn only_a_folder_an_install_wrote_below_the_skin_folder_is_taken_for_the_retired_bundle() {
    let installed = format!("/skins/{}/select.json5", retired_directory(Some(2)));
    let renamed = format!("/skins/{}-mine/play-7k.json5", retired_directory(None));
    let copied = format!("/backup/{}/decide.json5", retired_directory(Some(3)));
    let loose = "/skins/result.json5";
    let raw = format!(
        r#"(
            schema_version: 2,
            display: (skin: "WIDE"),
            skin: (
                folder: Some("/skins"),
                selected: {{
                    {DEFAULT_SKIN_SCREEN}: "{renamed}",
                    {MUSIC_SELECT_SCREEN}: "{installed}",
                    {DECIDE_SCREEN}: "{copied}",
                    {RESULT_SCREEN}: "{loose}",
                }},
                custom: {{ "{installed}": (properties: {{ "LANE COVER": 902 }}), "{renamed}": (properties: {{ "LANE COVER": 903 }}) }},
            ),
        )"#
    );

    let (config, from) = migrate(&raw).expect("a schema 2 file migrates");
    assert_eq!(from, Some(BUNDLED_SKIN_SCHEMA_VERSION));
    assert_eq!(config.display.skin, "WIDE", "a preset that is still offered was reset");
    assert_eq!(config.skin.folder.as_deref(), Some("/skins"));
    assert_eq!(config.skin.document(MUSIC_SELECT_SCREEN), None, "a document installed below the named folder was kept");
    assert_eq!(config.skin.customisation(&installed), None);
    assert_eq!(config.skin.document(DEFAULT_SKIN_SCREEN), Some(renamed.as_str()), "a folder the player named for themselves was taken for the bundle");
    assert_eq!(config.skin.user_config(&renamed).properties.get("LANE COVER"), Some(&903));
    assert_eq!(config.skin.document(DECIDE_SCREEN), Some(copied.as_str()), "a copy kept outside the skin folder was taken for the bundle");
    assert_eq!(config.skin.document(RESULT_SCREEN), Some(loose), "a document lying in the skin folder itself was dropped");
}

/// The preset is retired from every schema that could have held it, and a file already on the
/// current schema is read as written: the migration runs once, on the way in from an older build.
#[test]
fn the_retired_preset_is_dropped_from_every_older_schema_and_a_current_file_is_left_alone() {
    let preset = retired_preset();
    for version in [SINGLE_JUDGE_WIDTH_SCHEMA_VERSION, BUNDLED_SKIN_SCHEMA_VERSION] {
        let (config, from) =
            migrate(&format!(r#"(schema_version: {version}, display: (skin: "{}"))"#, preset.to_ascii_lowercase())).expect("an older file migrates");
        assert_eq!(from, Some(version));
        assert_eq!(config.display.skin, DEFAULT_SKIN, "schema {version} kept the retired preset");
    }
    let (flat, from) = migrate(&format!(r#"(skin: "{preset}")"#)).expect("a flat file migrates");
    assert_eq!(from, Some(LEGACY_SCHEMA_VERSION));
    assert_eq!(flat.display.skin, DEFAULT_SKIN, "the flat schema kept the retired preset");

    let kept = format!("{DEFAULT_SKIN_ROOT}/{}/select.json5", retired_directory(Some(3)));
    let mut current = Config::default();
    current.skin.select(MUSIC_SELECT_SCREEN, Some(kept.clone()));
    let (config, from) = migrate(&ron_of(&current)).expect("a current file parses");
    assert_eq!(from, None);
    assert_eq!(config.skin.document(MUSIC_SELECT_SCREEN), Some(kept.as_str()), "a selection made under the current schema was dropped on load");
}

/// An older file holds no window keys, so it migrates without a schema bump and opens the window
/// the way every earlier build did.
#[test]
fn a_file_from_before_the_window_rows_opens_the_shipped_window() {
    let (config, from) = migrate(&format!(r#"(schema_version: {CURRENT_SCHEMA_VERSION}, display: (skin: "WIDE", letterbox: true))"#))
        .expect("a current-schema file without window keys parses");
    assert_eq!(from, None, "adding the rows needed no new schema version");
    assert_eq!(config.display.window_resolution.size(), (1280, 720));
    assert_eq!(config.display.window_mode, WindowMode::Windowed);
    assert!(config.display.letterbox, "the keys that were there are kept");
}

#[test]
fn the_window_choices_are_stored_under_their_tokens_and_unknown_ones_fall_back() {
    let mut c = Config::default();
    c.display.window_resolution = WindowResolution::QuadHd1440;
    c.display.window_mode = WindowMode::Borderless;
    let text = ron_of(&c);
    assert!(text.contains("\"2560x1440\"") && text.contains("\"BORDERLESS\""), "{text}");

    let (config, _) = migrate(&format!(r#"(schema_version: {CURRENT_SCHEMA_VERSION}, display: (window_resolution: "7680x4320", window_mode: "EXCLUSIVE"))"#))
        .expect("unknown tokens still parse");
    assert_eq!(config.display.window_resolution, WindowResolution::Hd720);
    assert_eq!(config.display.window_mode, WindowMode::Windowed);
}

#[test]
fn a_borderless_window_is_always_fitted_to_its_shape() {
    let mut display = DisplayOptions { letterbox: false, ..DisplayOptions::default() };
    assert!(!display.fits_screen_shape(), "an ordinary window stretches unless LETTERBOX is on");
    display.letterbox = true;
    assert!(display.fits_screen_shape());
    display.letterbox = false;
    display.window_mode = WindowMode::Borderless;
    assert!(display.fits_screen_shape(), "a monitor that is not 16:9 would stretch the screen");
}

/// A skin is authored for one shape, so a fresh install keeps that shape inside an ordinary window
/// instead of stretching it to whatever the window is.
#[test]
fn a_fresh_install_fits_the_screen_inside_an_ordinary_window() {
    let display = DisplayOptions::default();
    assert_eq!(display.window_mode, WindowMode::Windowed);
    assert!(display.letterbox && DEFAULT_LETTERBOX);
    assert!(display.fits_screen_shape());
}

/// The shipped fit is only what a file that says nothing gets, and what a file of a schema that
/// never let the player choose is moved to once. A player who switches LETTERBOX off under the
/// current schema keeps it off, and one who switched it on keeps it on.
#[test]
fn a_stored_letterbox_choice_outlives_the_shipped_one() {
    let (off, _) = migrate(&format!("(schema_version: {CURRENT_SCHEMA_VERSION}, display: (letterbox: false))")).expect("a current file parses");
    assert!(!off.display.letterbox, "a stored OFF was overridden by the shipped default");
    assert!(!off.display.fits_screen_shape());
    let (on, _) = migrate(&format!("(schema_version: {CURRENT_SCHEMA_VERSION}, display: (letterbox: true))")).expect("a current file parses");
    assert!(on.display.letterbox);
    let (silent, _) = migrate(&format!("(schema_version: {CURRENT_SCHEMA_VERSION}, display: (bga: false))")).expect("a current file parses");
    assert!(silent.display.letterbox, "a file that does not state the row takes the shipped fit");

    let mut stored = Config::default();
    stored.display.letterbox = false;
    let (back, _) = migrate(&ron_of(&stored)).expect("the file this build writes parses");
    assert!(!back.display.letterbox, "the file this build writes does not state the row it holds");
}

/// Builds up to the one that introduced the shipped fit saved the whole configuration on every
/// change, so `letterbox: false` in their files is the old default rather than a choice. Reading
/// one turns the row on; the schema it is saved under afterwards keeps whatever the player does.
#[test]
fn an_older_schema_letterbox_is_turned_on_once_and_the_later_choice_sticks() {
    for stated in ["false", "true"] {
        let (config, from) =
            migrate(&format!("(schema_version: {UNCHOSEN_LETTERBOX_SCHEMA_VERSION}, display: (letterbox: {stated}))")).expect("an older file migrates");
        assert_eq!(from, Some(UNCHOSEN_LETTERBOX_SCHEMA_VERSION));
        assert!(config.display.letterbox, "schema {UNCHOSEN_LETTERBOX_SCHEMA_VERSION} with letterbox {stated} was left off");
        assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);

        let mut chosen = config;
        chosen.display.letterbox = false;
        let (kept, from) = migrate(&ron_of(&chosen)).expect("the migrated file parses");
        assert_eq!(from, None);
        assert!(!kept.display.letterbox, "the choice made after the migration was overridden by it");
    }
    let (flat, from) = migrate("(bga: false)").expect("a flat file migrates");
    assert_eq!(from, Some(LEGACY_SCHEMA_VERSION));
    assert!(flat.display.letterbox, "a file with no schema takes the shipped fit");
}
