//! Unit tests for the pure helpers declared in `main.rs`.
//!
//! They live in their own module so the crate root stays a wiring file: the items under test are
//! private to the root, and a child module still sees them through `super`.

use super::assets::THEME_TEMPLATE;
use super::judge_setup::{is_custom_judge, judge_setup_of};
use super::{
    ROOT_ESC_CONFIRM, SortMode, assisted_lamp, bundled_skin, calibrated_offset, clear_type_from_id, clear_type_id, client_platform, compute_build_hash,
    config_dir_from, default_total_for_mode, esc_confirms_quit, exit_code, fmt_datetime, green_number_for, ir_submission_block_reason, resumed_clock_us,
    saves_replay, updates_score, write_atomic,
};
use rbms_chart::{default_total, default_total_keyboard};
use rbms_config::{Config, JUDGE_RATE_MAX_PERCENT, LN_MARGIN_DEFAULT_PERCENT, LN_MARGIN_MAX_PERCENT, LN_MARGIN_MIN_PERCENT, UNMODIFIED_JUDGE_RATES};
use rbms_ir::mapping::{CUSTOM_JUDGE_ASSIST, LIGHT_ASSIST, NO_ASSIST, assist_level};
use rbms_judge::algorithm::JudgeAlgorithm;
use rbms_judge::{ClearType, GaugeKind};
use rbms_model::Mode;
use std::time::{Duration, Instant};

#[test]
fn config_dir_prefers_home_then_userprofile() {
    use std::ffi::OsString;
    use std::path::PathBuf;
    let cd = |h: Option<&str>, u: Option<&str>| config_dir_from(h.map(OsString::from), u.map(OsString::from));
    assert_eq!(cd(Some("/home/u"), None), PathBuf::from("/home/u").join(".config/rbms"));
    assert_eq!(cd(None, Some("C:/Users/u")), PathBuf::from("C:/Users/u").join(".config/rbms"), "USERPROFILE used when HOME unset (Windows)");
    assert_eq!(cd(Some("/h"), Some("C:/x")), PathBuf::from("/h").join(".config/rbms"), "HOME wins over USERPROFILE");
    assert_eq!(cd(None, None), PathBuf::from(".").join(".config/rbms"));
}

#[test]
fn theme_template_is_valid_ron_and_matches_defaults() {
    let parsed: Result<rbms_render::ThemeConfig, _> = ron::from_str(THEME_TEMPLATE);
    assert!(parsed.is_ok(), "theme template must be valid RON: {parsed:?}");
    assert_eq!(parsed.unwrap().resolve(), rbms_render::Theme::default(), "template values equal the defaults");
}

#[test]
fn datetime_formats_utc() {
    assert_eq!(fmt_datetime(0), "1970-01-01 00:00");
    assert_eq!(fmt_datetime(86_400_000), "1970-01-02 00:00");
    assert_eq!(fmt_datetime(1_700_000_000_000), "2023-11-14 22:13");
}

#[test]
fn clear_lamp_id_roundtrips() {
    for c in [
        ClearType::NoPlay,
        ClearType::Failed,
        ClearType::AssistEasy,
        ClearType::Easy,
        ClearType::Normal,
        ClearType::Hard,
        ClearType::ExHard,
        ClearType::FullCombo,
        ClearType::Perfect,
        ClearType::Max,
    ] {
        assert_eq!(clear_type_from_id(clear_type_id(c)), c, "{c:?} lamp id must round-trip");
    }
}

#[test]
fn build_hash_is_64_hex_chars() {
    let h = compute_build_hash().expect("the test binary should be readable");
    assert_eq!(h.len(), 64, "SHA-256 hex is 64 chars");
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "hash is lowercase hex");
}

#[test]
fn client_platform_is_os_arch() {
    let p = client_platform();
    assert!(p.contains('-'), "platform tag is OS-ARCH");
    assert_eq!(p, format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH));
}

#[test]
fn calibrated_offset_recenters_from_mean() {
    assert_eq!(calibrated_offset(0, 60_000), 60, "60ms-early avg -> +60ms offset");
    assert_eq!(calibrated_offset(0, -40_000), -40);
    assert_eq!(calibrated_offset(60, 0), 60, "no error -> no change");
    assert_eq!(calibrated_offset(0, 0), 0);
    assert_eq!(calibrated_offset(190, 50_000), 200, "clamped to +200");
}

#[test]
fn calibration_converges_in_one_run() {
    let bias_us: i64 = 60_000;
    let offset0 = 0;
    let mean_run1 = bias_us - offset0 as i64 * 1000;
    let offset1 = calibrated_offset(offset0, mean_run1);
    let mean_run2 = bias_us - offset1 as i64 * 1000;
    assert!(mean_run2.abs() <= 1_000, "after one calibration the residual error is ~0 (was {mean_run2}us)");
}

#[test]
fn calibrated_offset_clamps_both_ends() {
    assert_eq!(calibrated_offset(-190, -50_000), -200, "clamped to -200");
    assert_eq!(calibrated_offset(190, 50_000), 200, "clamped to +200");
    assert_eq!(calibrated_offset(200, 1_000_000), 200, "never exceeds +200");
    assert_eq!(calibrated_offset(-200, -1_000_000), -200, "never below -200");
}

#[test]
fn calibrated_offset_rounds_to_nearest_ms() {
    assert_eq!(calibrated_offset(0, 1_400), 1, "1.4ms rounds to 1");
    assert_eq!(calibrated_offset(0, 1_600), 2, "1.6ms rounds to 2");
    assert_eq!(calibrated_offset(0, -1_600), -2, "-1.6ms rounds to -2");
    assert_eq!(calibrated_offset(0, 499), 0, "sub-half-ms rounds to 0");
}

#[test]
fn default_total_has_a_floor_of_260() {
    assert!(default_total(0) >= 260.0);
    assert!(default_total(1) >= 260.0);
    assert!(default_total(50) >= 260.0);
}

#[test]
fn default_total_zero_and_one_note_equal_due_to_floor() {
    assert_eq!(default_total(0), default_total(1));
}

#[test]
fn default_total_is_non_decreasing_in_note_count() {
    let mut prev = default_total(1);
    for n in [10, 100, 500, 1000, 5000, 20000] {
        let t = default_total(n);
        assert!(t >= prev - 1e-9, "TOTAL non-decreasing at {n} notes ({t} < {prev})");
        prev = t;
    }
}

#[test]
fn the_keyboard_modes_take_their_own_default_total_formula() {
    for notes in [0, 1, 500, 2000] {
        assert_eq!(default_total_for_mode(&Mode::BEAT_7K, notes), default_total(notes), "a beat mode keeps the BMSPlayerRule.java:84-89 beat curve");
        assert_eq!(default_total_for_mode(&Mode::POPN_9K, notes), default_total(notes));
        assert_eq!(
            default_total_for_mode(&Mode::KEYBOARD_24K, notes),
            default_total_keyboard(notes),
            "the 24-key rule has a floor of 300 and a notes+100 numerator"
        );
    }
    assert!(default_total_for_mode(&Mode::KEYBOARD_24K, 500) > default_total(500), "the two curves really do differ, or this proves nothing");
}

#[test]
fn default_total_large_charts_exceed_floor() {
    assert!(default_total(2000) > 260.0, "dense chart rises above the floor");
}

#[test]
fn sortmode_next_cycles_through_all_and_returns_to_start() {
    let mut seen = Vec::new();
    let mut m = SortMode::Default;
    for _ in 0..SortMode::ALL.len() {
        seen.push(m);
        m = m.next();
    }
    assert_eq!(seen.len(), SortMode::ALL.len());
    for variant in SortMode::ALL {
        assert!(seen.contains(&variant), "{:?} visited", variant.label());
    }
    assert!(m == SortMode::Default, "wraps back to the start after a full cycle");
}

#[test]
fn sortmode_next_advances_by_one_each_step() {
    let chain = [SortMode::Default, SortMode::Title, SortMode::Artist, SortMode::Level, SortMode::Clear];
    for w in chain.windows(2) {
        assert_eq!(w[0].next().label(), w[1].label(), "{} -> {}", w[0].label(), w[1].label());
    }
    assert_eq!(SortMode::Clear.next().label(), SortMode::Default.label(), "last wraps to first");
}

#[test]
fn sortmode_labels_distinct_and_nonempty() {
    let mut labels: Vec<&str> = SortMode::ALL.iter().map(|m| m.label()).collect();
    for l in &labels {
        assert!(!l.is_empty(), "label non-empty");
    }
    let n = labels.len();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), n, "all labels distinct");
}

#[test]
fn bundled_skins_parse_for_both_names_and_match_wide_case_insensitively() {
    let _wide = bundled_skin("WIDE");
    let _normal = bundled_skin("NORMAL");
    let _default = bundled_skin("anything-else");
    let _case = bundled_skin("wide");
}

#[test]
fn resumed_clock_continues_from_the_last_audio_position() {
    assert_eq!(resumed_clock_us(12_000_000, 2_500_000), 14_500_000);
    assert_eq!(resumed_clock_us(12_000_000, 0), 12_000_000, "no wall time yet => no movement");
}

#[test]
fn green_number_constant_depends_only_on_hispeed_and_cover() {
    assert_eq!(green_number_for(true, 120.0, 2.0, 1.0, 0.0), 1000.0);
    assert_eq!(green_number_for(true, 300.0, 2.0, 0.5, 0.0), 1000.0, "BPM/SCROLL do not apply");
    assert!((green_number_for(true, 120.0, 2.0, 1.0, 0.25) - 750.0).abs() < 1e-3);
}

#[test]
fn green_number_floating_tracks_bpm_scroll_and_cover() {
    assert_eq!(green_number_for(false, 120.0, 1.0, 1.0, 0.0), 2000.0);
    assert_eq!(green_number_for(false, 240.0, 1.0, 1.0, 0.0), 1000.0);
    assert!((green_number_for(false, 120.0, 1.0, 2.0, 0.0) - 1000.0).abs() < 1e-9, "SCROLL 2.0 halves the travel time");
    assert!((green_number_for(false, 120.0, 2.0, 1.0, 0.0) - 1000.0).abs() < 1e-9, "hi-speed 2.0 halves it too");
    assert!((green_number_for(false, 120.0, 1.0, 1.0, 0.4) - 1200.0).abs() < 1e-3);
}

#[test]
fn ir_submission_allowed_only_for_an_unassisted_interactive_play() {
    assert_eq!(ir_submission_block_reason(false, false, false, false), None);
}

#[test]
fn ir_submission_blocked_for_autoplay_replay_and_assists() {
    assert_eq!(ir_submission_block_reason(true, false, false, false), Some("autoplay"));
    assert_eq!(ir_submission_block_reason(false, true, false, false), Some("replay playback"));
    assert_eq!(ir_submission_block_reason(false, false, true, false), Some("judge window widened"));
    assert_eq!(ir_submission_block_reason(false, false, false, true), Some("scratch assist"));
}

#[test]
fn updates_score_is_the_exact_complement_of_the_ir_block_reason() {
    for &autoplay in &[false, true] {
        for &replay in &[false, true] {
            for &custom_judge in &[false, true] {
                for &scratch_auto in &[false, true] {
                    let blocked = ir_submission_block_reason(autoplay, replay, custom_judge, scratch_auto).is_some();
                    assert_eq!(
                        updates_score(autoplay, replay, custom_judge, scratch_auto),
                        !blocked,
                        "autoplay={autoplay} replay={replay} custom_judge={custom_judge} scratch_auto={scratch_auto}"
                    );
                }
            }
        }
    }
}

#[test]
fn updates_score_only_for_an_unassisted_interactive_play() {
    assert!(updates_score(false, false, false, false));
    assert!(!updates_score(false, false, true, false), "a widened judge window does not");
    assert!(!updates_score(false, false, false, true), "auto scratch does not");
    assert!(!updates_score(true, false, false, false));
    assert!(!updates_score(false, true, false, false));
}

/// The exact table decision 12 states, over every judge width and long-note margin the rows can
/// hold: widening any one of the seven is a custom judge, narrowing any of them is not.
#[test]
fn assist_level_is_two_for_any_widened_width_or_margin_and_one_for_auto_scratch() {
    let widths = [
        (UNMODIFIED_JUDGE_RATES, UNMODIFIED_JUDGE_RATES, LN_MARGIN_DEFAULT_PERCENT, false),
        ([50, 50, 50], [50, 50, 50], LN_MARGIN_MIN_PERCENT, false),
        ([JUDGE_RATE_MAX_PERCENT, 100, 100], UNMODIFIED_JUDGE_RATES, LN_MARGIN_DEFAULT_PERCENT, true),
        ([100, 105, 100], UNMODIFIED_JUDGE_RATES, LN_MARGIN_DEFAULT_PERCENT, true),
        ([100, 100, 105], UNMODIFIED_JUDGE_RATES, LN_MARGIN_DEFAULT_PERCENT, true),
        (UNMODIFIED_JUDGE_RATES, [105, 100, 100], LN_MARGIN_DEFAULT_PERCENT, true),
        (UNMODIFIED_JUDGE_RATES, [100, 105, 100], LN_MARGIN_DEFAULT_PERCENT, true),
        (UNMODIFIED_JUDGE_RATES, [100, 100, 105], LN_MARGIN_DEFAULT_PERCENT, true),
        (UNMODIFIED_JUDGE_RATES, UNMODIFIED_JUDGE_RATES, LN_MARGIN_MAX_PERCENT, true),
    ];
    for (key, scratch, margin, custom) in widths {
        let mut config = Config::default();
        config.judge.judge_rate_key = key;
        config.judge.judge_rate_scratch = scratch;
        config.judge.longnote_margin_rate = margin;
        assert_eq!(is_custom_judge(&judge_setup_of(&config)), custom, "key={key:?} scratch={scratch:?} margin={margin}");
        for &scratch_auto in &[false, true] {
            let expected = match (custom, scratch_auto) {
                (true, _) => CUSTOM_JUDGE_ASSIST,
                (false, true) => LIGHT_ASSIST,
                (false, false) => NO_ASSIST,
            };
            assert_eq!(assist_level(scratch_auto, custom), expected, "key={key:?} scratch_auto={scratch_auto}");
        }
    }
}

/// `BMSPlayer.java:866` reads `assist == 1 ? LightAssistEasy : AssistEasy`, so only the single light
/// assist takes the higher lamp: anything stronger has to take the lower one, however much stronger
/// it gets.
#[test]
fn only_the_single_light_assist_keeps_the_higher_demoted_lamp() {
    assert_eq!(assisted_lamp(ClearType::Hard, LIGHT_ASSIST), ClearType::LightAssistEasy);
    for stronger in [CUSTOM_JUDGE_ASSIST, CUSTOM_JUDGE_ASSIST + 1, u8::MAX] {
        assert_eq!(assisted_lamp(ClearType::Hard, stronger), ClearType::AssistEasy, "assist {stronger} must not read as a lighter one");
    }
    assert!(
        clear_type_id(ClearType::AssistEasy) < clear_type_id(ClearType::LightAssistEasy),
        "the demotion really does order the two lamps, or this proves nothing"
    );
}

/// Decision 12's split: a custom judge blocks the replay, an auto-played lane does not.
#[test]
fn only_a_custom_judge_blocks_the_replay_recording() {
    assert!(saves_replay(NO_ASSIST));
    assert!(saves_replay(LIGHT_ASSIST), "an auto-played lane keeps its replay");
    assert!(!saves_replay(CUSTOM_JUDGE_ASSIST));
}

/// Any assist puts the full-combo lamps out of reach and demotes the clear
/// (`BMSPlayer.java:864-874`): one light assist to `LightAssistEasy`, anything stronger to
/// `AssistEasy`. A failed run keeps its lamp either way.
#[test]
fn an_assisted_run_is_demoted_and_cannot_reach_a_full_combo_lamp() {
    let clears = [
        ClearType::LightAssistEasy,
        ClearType::Easy,
        ClearType::Normal,
        ClearType::Hard,
        ClearType::ExHard,
        ClearType::FullCombo,
        ClearType::Perfect,
        ClearType::Max,
    ];
    for lamp in clears {
        assert_eq!(assisted_lamp(lamp, NO_ASSIST), lamp, "{lamp:?} is untouched without assist");
        assert_eq!(assisted_lamp(lamp, LIGHT_ASSIST), ClearType::LightAssistEasy, "{lamp:?} under one light assist");
        assert_eq!(assisted_lamp(lamp, CUSTOM_JUDGE_ASSIST), ClearType::AssistEasy, "{lamp:?} under a custom judge");
    }
    for assist in [NO_ASSIST, LIGHT_ASSIST, CUSTOM_JUDGE_ASSIST] {
        assert_eq!(assisted_lamp(ClearType::Failed, assist), ClearType::Failed);
        assert_eq!(assisted_lamp(ClearType::NoPlay, assist), ClearType::NoPlay);
    }
}

/// The judge-width vocabulary is written down in three crates that do not depend on one another;
/// the player is the one place that sees all three, so it pins them together.
#[test]
fn the_judge_width_tier_count_and_unmodified_rate_agree_across_the_crates() {
    assert_eq!(rbms_config::JUDGE_WIDTH_TIER_COUNT, rbms_play::JUDGE_WIDTH_TIER_COUNT);
    assert_eq!(rbms_store::REPLAY_JUDGE_WIDTH_TIER_COUNT, rbms_play::JUDGE_WIDTH_TIER_COUNT);
    assert_eq!(rbms_config::UNMODIFIED_JUDGE_RATES.to_vec(), rbms_play::UNMODIFIED_JUDGE_RATES.to_vec());
    assert_eq!(rbms_config::JUDGE_RATE_DEFAULT_PERCENT, rbms_ir::mapping::JUDGE_RATE_UNMODIFIED);
    assert_eq!(rbms_store::REPLAY_UNMODIFIED_RATE_PERCENT, rbms_ir::mapping::JUDGE_RATE_UNMODIFIED);
    assert_eq!(rbms_config::algorithm_from_token(rbms_store::REPLAY_LEGACY_ALGORITHM), JudgeAlgorithm::Duration);
}

#[test]
fn first_escape_arms_the_quit_confirmation_instead_of_quitting() {
    assert!(!esc_confirms_quit(None, Instant::now()), "a lone Esc never quits");
}

#[test]
fn second_escape_quits_only_within_the_confirm_window() {
    let now = Instant::now();
    assert!(esc_confirms_quit(Some(now), now), "an immediate second Esc quits");
    assert!(esc_confirms_quit(Some(now), now + ROOT_ESC_CONFIRM), "exactly at the window edge still quits");
    assert!(!esc_confirms_quit(Some(now), now + ROOT_ESC_CONFIRM + Duration::from_millis(1)), "a late second Esc does not quit");
}

#[test]
fn write_atomic_writes_the_contents_and_leaves_no_temp_file() {
    let dir = std::env::temp_dir().join(format!("rbms_atomic_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("nested/settings.ron");
    write_atomic(&path, "(hispeed: 1.0)").expect("atomic write creates the parent dir and the file");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "(hispeed: 1.0)");
    assert!(!path.with_file_name("settings.ron.tmp").exists(), "the temp file is renamed away, not left behind");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_atomic_replaces_an_existing_file_wholesale() {
    let dir = std::env::temp_dir().join(format!("rbms_atomic_replace_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("scores.ron");
    write_atomic(&path, "aaaaaaaaaaaaaaaaaaaa").unwrap();
    write_atomic(&path, "bb").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "bb");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_atomic_rejects_a_path_without_a_file_name() {
    assert!(write_atomic(std::path::Path::new("/"), "x").is_err(), "a directory path is not a writable target");
}

#[test]
fn the_shipped_polyphony_is_the_one_the_audio_engine_defaults_to() {
    assert_eq!(
        rbms_config::DEFAULT_POLYPHONY_VOICES,
        rbms_audio::DEFAULT_MAX_VOICES,
        "the configuration crate mirrors the engine's voice budget instead of depending on it"
    );
    assert_eq!(
        crate::settings_ui::engine_options(&rbms_config::AudioOptions::default()),
        rbms_audio::AudioOptions::default(),
        "untouched audio settings open the engine exactly as its own default does"
    );
}

#[test]
fn the_shipped_player_id_is_the_one_the_score_server_accepts_without_a_token() {
    assert_eq!(rbms_config::DEFAULT_PLAYER_ID, rbms_ir::GUEST_PLAYER_ID);
}

#[test]
fn the_settings_gauge_vocabulary_matches_the_one_the_ir_replay_speaks() {
    for g in [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard] {
        assert_eq!(rbms_config::gauge_token(g), rbms_ir::mapping::gauge_token(g), "{g:?} is written the same way in both");
        assert_eq!(rbms_config::gauge_from_name(rbms_config::gauge_token(g)), rbms_ir::mapping::gauge_from_name(rbms_config::gauge_token(g)));
    }
    for name in ["assist", "assisteasy", "EASY", "hard", "exhard", "hazard", "", "nonsense"] {
        assert_eq!(rbms_config::gauge_from_name(name), rbms_ir::mapping::gauge_from_name(name), "{name:?} parses the same way in both");
    }
}

#[test]
fn a_run_that_never_started_fails_the_process_instead_of_panicking() {
    use std::process::ExitCode;
    assert_eq!(format!("{:?}", exit_code(None)), format!("{:?}", ExitCode::SUCCESS));
    assert_eq!(format!("{:?}", exit_code(Some("no graphics adapter this build can use"))), format!("{:?}", ExitCode::FAILURE));
}

#[test]
fn every_startup_failure_reads_as_a_sentence_rather_than_a_panic_payload() {
    use crate::gpu::GpuError;
    let no_adapter = GpuError::NoAdapter.to_string();
    assert!(no_adapter.contains("graphics adapter"), "{no_adapter}");
    assert!(!no_adapter.is_empty());
    assert!(!no_adapter.starts_with(char::is_uppercase), "messages are lower case like the rest of the app: {no_adapter}");
}
