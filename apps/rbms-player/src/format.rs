//! Pure formatting / palette helpers for the UI (no app state). Extracted from `main.rs`: label
//! strings, IIDX/beatoraja colour palettes, timestamp/duration formatting, and `ClearType` id
//! round-tripping for score persistence.

use rbms_judge::{ClearType, GaugeKind};
use rbms_model::Mode;
use rbms_render::Color;

pub(crate) fn gauge_name(g: GaugeKind) -> &'static str {
    match g {
        GaugeKind::AssistEasy => "ASSIST EASY",
        GaugeKind::Easy => "EASY",
        GaugeKind::Normal => "NORMAL",
        GaugeKind::Hard => "HARD",
        GaugeKind::ExHard => "EX-HARD",
        GaugeKind::Hazard => "HAZARD",
    }
}

pub(crate) fn mode_color(mode: Mode) -> Color {
    match mode.key {
        6 => Color::GREEN,
        8 => Color::BLUE,
        9 => Color::rgb(230, 120, 200),
        12 => Color::rgb(90, 200, 170),
        16 => Color::ORANGE,
        _ => Color::GRAY,
    }
}

/// Format an epoch-millis timestamp as `YYYY-MM-DD HH:MM` (UTC) for the record list. Uses
/// Hinnant's civil-from-days algorithm so no calendar crate is needed.
pub(crate) fn fmt_datetime(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, tod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (hh, mm) = (tod / 3600, (tod % 3600) / 60);
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = era * 400 + yoe + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}")
}

/// BMS `#DIFFICULTY` slot name (1–5), the IIDX-style difficulty label.
pub(crate) fn difficulty_name(d: i32) -> &'static str {
    match d {
        1 => "BEGINNER",
        2 => "NORMAL",
        3 => "HYPER",
        4 => "ANOTHER",
        5 => "INSANE",
        _ => "—",
    }
}

/// `#DIFFICULTY` slot colour (BEGINNER→INSANE), the IIDX/LR2 difficulty palette used for the level badge.
pub(crate) fn difficulty_color(d: i32) -> Color {
    match d {
        1 => Color::rgb(80, 220, 120),
        2 => Color::rgb(90, 180, 240),
        3 => Color::rgb(240, 200, 70),
        4 => Color::rgb(240, 90, 90),
        5 => Color::rgb(200, 120, 230),
        _ => Color::GRAY,
    }
}

/// `#RANK` as a name + judge-width percent (beatoraja: 0 VERY HARD … 4 VERY EASY; `#RANK 2` = NORMAL = 75%).
pub(crate) fn rank_label(rank: i32) -> String {
    let name = match rank {
        0 => "VERY HARD",
        1 => "HARD",
        2 => "NORMAL",
        3 => "EASY",
        4 => "VERY EASY",
        _ => "?",
    };
    format!("{name} {}%", rbms_judge::rank_to_judgerank(rank))
}

/// A µs duration as `m:ss`.
pub(crate) fn fmt_duration(us: i64) -> String {
    let secs = (us / 1_000_000).max(0);
    format!("{}:{:02}", secs / 60, secs % 60)
}

pub(crate) fn mode_short(mode: Mode) -> &'static str {
    match mode.key {
        6 => "5K",
        8 => "7K",
        9 => "9K",
        12 => "10K",
        16 => "14K",
        _ => "?",
    }
}

/// beatoraja `ClearType` id (`ClearType.java`) for a lamp — persisted in score records so the
/// lamp round-trips. (LightAssistEasy=3 is unused; rbms has no separate light-assist lamp.)
pub(crate) fn clear_type_id(c: ClearType) -> u8 {
    match c {
        ClearType::NoPlay => 0,
        ClearType::Failed => 1,
        ClearType::AssistEasy => 2,
        ClearType::Easy => 4,
        ClearType::Normal => 5,
        ClearType::Hard => 6,
        ClearType::ExHard => 7,
        ClearType::FullCombo => 8,
        ClearType::Perfect => 9,
        ClearType::Max => 10,
    }
}

pub(crate) fn clear_type_from_id(id: u8) -> ClearType {
    match id {
        1 => ClearType::Failed,
        2 | 3 => ClearType::AssistEasy,
        4 => ClearType::Easy,
        5 => ClearType::Normal,
        6 => ClearType::Hard,
        7 => ClearType::ExHard,
        8 => ClearType::FullCombo,
        9 => ClearType::Perfect,
        10 => ClearType::Max,
        _ => ClearType::NoPlay,
    }
}

/// Clear-lamp label + colour. Colours are beatoraja's official lamp palette
/// (`select/SkinDistributionGraph.LAMP`, ARGB → RGB).
pub(crate) fn clear_label_color(c: ClearType) -> (&'static str, Color) {
    match c {
        ClearType::NoPlay => ("NO PLAY", Color::rgb(64, 64, 64)),
        ClearType::Failed => ("FAILED", Color::rgb(0, 0, 128)),
        ClearType::AssistEasy => ("ASSIST EASY", Color::rgb(128, 0, 128)),
        ClearType::Easy => ("EASY", Color::rgb(64, 255, 64)),
        ClearType::Normal => ("CLEAR", Color::rgb(0, 192, 240)),
        ClearType::Hard => ("HARD", Color::rgb(255, 255, 255)),
        ClearType::ExHard => ("EX-HARD", Color::rgb(136, 255, 255)),
        ClearType::FullCombo => ("FULL COMBO", Color::rgb(255, 255, 136)),
        ClearType::Perfect => ("PERFECT", Color::rgb(136, 136, 255)),
        ClearType::Max => ("MAX", Color::rgb(0, 0, 255)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CLEARS: [ClearType; 10] = [
        ClearType::NoPlay, ClearType::Failed, ClearType::AssistEasy, ClearType::Easy,
        ClearType::Normal, ClearType::Hard, ClearType::ExHard, ClearType::FullCombo,
        ClearType::Perfect, ClearType::Max,
    ];
    const ALL_GAUGES: [GaugeKind; 6] = [
        GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal,
        GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard,
    ];

    // --- fmt_datetime ---

    #[test]
    fn fmt_datetime_epoch_and_known_points() {
        assert_eq!(fmt_datetime(0), "1970-01-01 00:00");
        assert_eq!(fmt_datetime(86_400_000), "1970-01-02 00:00");
        // exactly one hour and one minute past epoch
        assert_eq!(fmt_datetime(3_660_000), "1970-01-01 01:01");
        assert_eq!(fmt_datetime(1_700_000_000_000), "2023-11-14 22:13");
    }

    #[test]
    fn fmt_datetime_truncates_within_a_minute() {
        // sub-minute ms are floored, not rounded.
        assert_eq!(fmt_datetime(59_999), "1970-01-01 00:00");
        assert_eq!(fmt_datetime(60_000), "1970-01-01 00:01");
    }

    #[test]
    fn fmt_datetime_uses_floored_division_for_negative_times() {
        // 1ms before epoch: div_euclid floors toward -inf, so this is 23:59 on 1969-12-31, not a
        // wrapped/garbage value.
        assert_eq!(fmt_datetime(-1), "1969-12-31 23:59");
        assert_eq!(fmt_datetime(-86_400_000), "1969-12-31 00:00");
    }

    #[test]
    fn fmt_datetime_leap_day_2000() {
        // 2000-02-29 is a valid leap day (year divisible by 400). Epoch ms for 2000-02-29 12:00 UTC.
        // days from epoch to 2000-02-29 = 11016; +0.5 day for noon.
        let ms = (11016_i64 * 86400 + 12 * 3600) * 1000;
        assert_eq!(fmt_datetime(ms), "2000-02-29 12:00");
    }

    #[test]
    fn fmt_datetime_year_boundary_rollover() {
        // 1999-12-31 23:59 then the next minute is 2000-01-01 00:00.
        let new_year_2000 = 946_684_800_000_i64; // 2000-01-01 00:00:00 UTC
        assert_eq!(fmt_datetime(new_year_2000), "2000-01-01 00:00");
        assert_eq!(fmt_datetime(new_year_2000 - 60_000), "1999-12-31 23:59");
    }

    #[test]
    fn fmt_datetime_shape_is_always_16_chars() {
        for ms in [0_i64, -1, 1_700_000_000_000, 4_102_444_800_000] {
            let s = fmt_datetime(ms);
            assert_eq!(s.len(), 16, "YYYY-MM-DD HH:MM is 16 chars for {ms}");
            assert_eq!(&s[4..5], "-");
            assert_eq!(&s[7..8], "-");
            assert_eq!(&s[10..11], " ");
            assert_eq!(&s[13..14], ":");
        }
    }

    // --- clear_type id round-trip ---

    #[test]
    fn clear_type_id_round_trips_for_every_lamp() {
        for c in ALL_CLEARS {
            assert_eq!(clear_type_from_id(clear_type_id(c)), c, "{c:?} round-trips");
        }
    }

    #[test]
    fn clear_type_ids_are_the_beatoraja_values() {
        assert_eq!(clear_type_id(ClearType::NoPlay), 0);
        assert_eq!(clear_type_id(ClearType::Failed), 1);
        assert_eq!(clear_type_id(ClearType::AssistEasy), 2);
        assert_eq!(clear_type_id(ClearType::Easy), 4, "id 3 (LightAssistEasy) is skipped");
        assert_eq!(clear_type_id(ClearType::Normal), 5);
        assert_eq!(clear_type_id(ClearType::Hard), 6);
        assert_eq!(clear_type_id(ClearType::ExHard), 7);
        assert_eq!(clear_type_id(ClearType::FullCombo), 8);
        assert_eq!(clear_type_id(ClearType::Perfect), 9);
        assert_eq!(clear_type_id(ClearType::Max), 10);
    }

    #[test]
    fn clear_type_ids_are_strictly_monotonic() {
        // The lamp id must increase with lamp strength so best_clear (a max over ids) is meaningful.
        let ids: Vec<u8> = ALL_CLEARS.iter().map(|&c| clear_type_id(c)).collect();
        for w in ids.windows(2) {
            assert!(w[0] < w[1], "lamp ids strictly increasing: {ids:?}");
        }
    }

    #[test]
    fn clear_type_from_id_legacy_light_assist_maps_to_assist_easy() {
        // beatoraja id 3 == LightAssistEasy, which rbms has no separate lamp for; it folds into
        // AssistEasy. (Asymmetric: clear_type_id(AssistEasy) == 2, never 3.)
        assert_eq!(clear_type_from_id(3), ClearType::AssistEasy);
        assert_eq!(clear_type_id(clear_type_from_id(3)), 2, "3 folds down to the 2 lamp on re-encode");
    }

    #[test]
    fn clear_type_from_id_unknown_ids_are_no_play() {
        for id in [11_u8, 12, 50, 200, 255] {
            assert_eq!(clear_type_from_id(id), ClearType::NoPlay, "unknown id {id} => NoPlay");
        }
    }

    // --- difficulty helpers ---

    #[test]
    fn difficulty_name_covers_slots_and_falls_back() {
        assert_eq!(difficulty_name(1), "BEGINNER");
        assert_eq!(difficulty_name(2), "NORMAL");
        assert_eq!(difficulty_name(3), "HYPER");
        assert_eq!(difficulty_name(4), "ANOTHER");
        assert_eq!(difficulty_name(5), "INSANE");
        for d in [0, 6, -1, 99] {
            assert_eq!(difficulty_name(d), "—", "out-of-range slot {d} uses the dash");
        }
    }

    #[test]
    fn difficulty_color_distinct_per_slot_and_gray_fallback() {
        let colors: Vec<Color> = (1..=5).map(difficulty_color).collect();
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "slots {} and {} differ in colour", i + 1, j + 1);
            }
        }
        assert_eq!(difficulty_color(0), Color::GRAY);
        assert_eq!(difficulty_color(99), Color::GRAY);
    }

    #[test]
    fn difficulty_name_and_color_agree_on_valid_range() {
        // Both helpers treat exactly 1..=5 as valid; outside that both use their fallback.
        for d in -2..=8 {
            let named = difficulty_name(d) != "—";
            let colored = difficulty_color(d) != Color::GRAY;
            assert_eq!(named, colored, "name/color validity agree at d={d}");
        }
    }

    // --- rank_label ---

    #[test]
    fn rank_label_names_and_percent() {
        assert_eq!(rank_label(0), "VERY HARD 25%");
        assert_eq!(rank_label(1), "HARD 50%");
        assert_eq!(rank_label(2), "NORMAL 75%");
        assert_eq!(rank_label(3), "EASY 100%");
        assert_eq!(rank_label(4), "VERY EASY 125%");
    }

    #[test]
    fn rank_label_out_of_range_falls_back_to_normal_judgerank() {
        assert_eq!(rank_label(-1), "? 75%", "negative rank: ? name, NORMAL 75% fallback");
        assert_eq!(rank_label(5), "? 75%", "rank 5: ? name, NORMAL 75% fallback");
    }

    // --- fmt_duration ---

    #[test]
    fn fmt_duration_minutes_and_seconds() {
        assert_eq!(fmt_duration(0), "0:00");
        assert_eq!(fmt_duration(1_000_000), "0:01");
        assert_eq!(fmt_duration(59_000_000), "0:59");
        assert_eq!(fmt_duration(60_000_000), "1:00");
        assert_eq!(fmt_duration(125_000_000), "2:05");
    }

    #[test]
    fn fmt_duration_truncates_sub_second_and_clamps_negative() {
        assert_eq!(fmt_duration(1_999_999), "0:01", "sub-second part truncated");
        assert_eq!(fmt_duration(-5_000_000), "0:00", "negative durations clamp to 0:00");
    }

    #[test]
    fn fmt_duration_seconds_always_two_digits() {
        for us in [0_i64, 5_000_000, 65_000_000, 605_000_000] {
            let s = fmt_duration(us);
            let secs_part = s.split(':').nth(1).unwrap();
            assert_eq!(secs_part.len(), 2, "seconds zero-padded in {s:?}");
        }
    }

    // --- mode_short / mode_color ---

    #[test]
    fn mode_short_per_key_count() {
        assert_eq!(mode_short(Mode::BEAT_5K), "5K");
        assert_eq!(mode_short(Mode::BEAT_7K), "7K");
        assert_eq!(mode_short(Mode::POPN_9K), "9K");
        assert_eq!(mode_short(Mode::BEAT_10K), "10K");
        assert_eq!(mode_short(Mode::BEAT_14K), "14K");
    }

    #[test]
    fn every_default_mode_has_a_known_short_label() {
        // mode_short covers all five default key counts (6/8/9/12/16).
        for &m in Mode::ALL {
            assert_ne!(mode_short(m), "?", "{} has a short label", m.name);
        }
    }

    #[test]
    fn mode_color_covers_every_default_mode() {
        // Every built-in mode gets a dedicated colour (incl. BEAT_10K, key=12), matching mode_short —
        // the two helpers agree on which modes are "known".
        for &m in Mode::ALL {
            assert_ne!(mode_color(m), Color::GRAY, "{} has a dedicated colour", m.name);
            assert_ne!(mode_short(m), "?", "{} has a short label", m.name);
        }
        assert_eq!(mode_short(Mode::BEAT_10K), "10K");
        assert_ne!(mode_color(Mode::BEAT_10K), Color::GRAY, "10K now has a dedicated colour");
    }

    #[test]
    fn mode_color_distinct_across_default_modes() {
        let colors: Vec<Color> = Mode::ALL.iter().map(|&m| mode_color(m)).collect();
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "{} and {} differ in colour", Mode::ALL[i].name, Mode::ALL[j].name);
            }
        }
    }

    #[test]
    fn mode_color_and_short_fallback_on_unknown_key_count() {
        let weird = Mode { name: "WEIRD", key: 3, ..Mode::BEAT_7K };
        assert_eq!(mode_color(weird), Color::GRAY);
        assert_eq!(mode_short(weird), "?");
    }

    // --- gauge_name / clear_label_color ---

    #[test]
    fn gauge_name_distinct_nonempty_per_kind() {
        let mut names: Vec<&str> = ALL_GAUGES.iter().map(|&g| gauge_name(g)).collect();
        for n in &names {
            assert!(!n.is_empty());
        }
        let len = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), len, "gauge names are distinct");
    }

    #[test]
    fn clear_label_color_distinct_label_and_color_per_lamp() {
        let pairs: Vec<(&str, Color)> = ALL_CLEARS.iter().map(|&c| clear_label_color(c)).collect();
        let mut labels: Vec<&str> = pairs.iter().map(|(l, _)| *l).collect();
        for l in &labels {
            assert!(!l.is_empty());
        }
        let len = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), len, "every lamp has a distinct label");
        // colours: each lamp's colour should also be distinct (lamp LED palette)
        for i in 0..pairs.len() {
            for j in (i + 1)..pairs.len() {
                assert_ne!(pairs[i].1, pairs[j].1, "lamp colours distinct: {:?} vs {:?}", pairs[i].0, pairs[j].0);
            }
        }
    }

    #[test]
    fn clear_label_color_normal_lamp_is_named_clear() {
        // The "Normal" ClearType is shown as "CLEAR" (beatoraja naming), a surprising-but-correct
        // mapping worth pinning.
        assert_eq!(clear_label_color(ClearType::Normal).0, "CLEAR");
    }
}
