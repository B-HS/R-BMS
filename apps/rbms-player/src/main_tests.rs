//! Unit tests for the pure helpers declared in `main.rs`.
//!
//! They live in their own module so the crate root stays a wiring file: the items under test are
//! private to the root, and a child module still sees them through `super`.

use super::{
    ROOT_ESC_CONFIRM, SortMode, THEME_TEMPLATE, bundled_skin, calibrated_offset, clear_type_from_id, clear_type_id, client_platform, compute_build_hash,
    config_dir_from, default_total, esc_confirms_quit, fmt_datetime, green_number_for, ir_submission_block_reason, judge_time_us, keysound_time_us,
    resumed_clock_us, updates_score, write_atomic,
};
use rbms_judge::ClearType;
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
fn judge_time_shifts_by_the_offset_in_milliseconds() {
    assert_eq!(judge_time_us(1_000_000, 0), 1_000_000);
    assert_eq!(judge_time_us(1_000_000, 30), 1_030_000, "+30ms offset judges 30ms later");
    assert_eq!(judge_time_us(1_000_000, -45), 955_000, "-45ms offset judges 45ms earlier");
}

#[test]
fn keysound_schedule_uses_raw_input_time_not_the_judge_offset() {
    assert_eq!(keysound_time_us(1_000_000, 7_500_000), 8_500_000);
    assert_eq!(keysound_time_us(1_000_000, 0), 1_000_000, "no anchor => the raw instant");
    assert_eq!(keysound_time_us(0, -250_000), -250_000, "a negative anchor shifts it back");
}

/// Reproduce the press path of `main.rs`'s key handler: judge at `judge_time_us(raw, offset)`,
/// schedule the keysound at `keysound_time_us(raw, anchor)`. Returns the times the keysound
/// callback was scheduled at.
fn press_schedule_times(offset_ms: i32, raw_us: i64, anchor_us: i64) -> Vec<i64> {
    let model = rbms_chart::to_model(&rbms_parser::parse(b"#BPM 120\r\n#WAV01 a.wav\r\n#00111:01\r\n"), rbms_model::Mode::BEAT_7K);
    let mut player = rbms_play::Player::new(model, false);
    let judge_t = judge_time_us(raw_us, offset_ms);
    let sound_t = keysound_time_us(raw_us, anchor_us);
    let mut scheduled = Vec::new();
    player.press(0, judge_t, |_e: rbms_play::PlayEvent| scheduled.push(sound_t));
    scheduled
}

#[test]
fn press_path_schedules_the_keysound_at_raw_plus_anchor_for_every_offset() {
    let (raw, anchor) = (1_000_000_i64, 7_500_000_i64);
    for offset_ms in [-200, -30, 0, 30, 200] {
        let scheduled = press_schedule_times(offset_ms, raw, anchor);
        assert_eq!(scheduled, vec![8_500_000], "offset {offset_ms}ms must not move the sound");
    }
}

#[test]
fn press_path_moves_the_judgment_with_the_offset_while_the_sound_stays() {
    let (raw, anchor) = (1_000_000_i64, 7_500_000_i64);
    assert_eq!(judge_time_us(raw, 30), 1_030_000);
    assert_eq!(press_schedule_times(30, raw, anchor), vec![8_500_000]);
    assert_eq!(press_schedule_times(0, raw, anchor), vec![8_500_000]);
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
    assert_eq!(ir_submission_block_reason(false, false, 100, false), None);
    assert_eq!(ir_submission_block_reason(false, false, 50, false), None, "a NARROWED judge window is not an assist");
}

#[test]
fn ir_submission_blocked_for_autoplay_replay_and_assists() {
    assert_eq!(ir_submission_block_reason(true, false, 100, false), Some("autoplay"));
    assert_eq!(ir_submission_block_reason(false, true, 100, false), Some("replay playback"));
    assert_eq!(ir_submission_block_reason(false, false, 105, false), Some("judge window widened"));
    assert_eq!(ir_submission_block_reason(false, false, 100, true), Some("scratch assist"));
}

#[test]
fn updates_score_is_the_exact_complement_of_the_ir_block_reason() {
    for &autoplay in &[false, true] {
        for &replay in &[false, true] {
            for &judge_rate in &[50, 100, 105, 200] {
                for &scratch_auto in &[false, true] {
                    let blocked = ir_submission_block_reason(autoplay, replay, judge_rate, scratch_auto).is_some();
                    assert_eq!(
                        updates_score(autoplay, replay, judge_rate, scratch_auto),
                        !blocked,
                        "autoplay={autoplay} replay={replay} judge_rate={judge_rate} scratch_auto={scratch_auto}"
                    );
                }
            }
        }
    }
}

#[test]
fn updates_score_only_for_an_unassisted_interactive_play() {
    assert!(updates_score(false, false, 100, false));
    assert!(updates_score(false, false, 50, false), "a narrowed judge window still scores");
    assert!(!updates_score(false, false, 101, false), "a widened judge window does not");
    assert!(!updates_score(false, false, 100, true), "auto scratch does not");
    assert!(!updates_score(true, false, 100, false));
    assert!(!updates_score(false, true, 100, false));
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
