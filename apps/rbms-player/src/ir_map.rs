//! Enum/string mapping between the engine types (`rbms_judge`/`rbms_chart`) and the IR contract
//! (`rbms_ir`) plus the persisted settings vocabulary. Extracted from `main.rs` — pure lookups,
//! no app state.

use rbms_chart::shuffle::NoteOption;
use rbms_ir::RandomOption;
use rbms_judge::{ClearType, GaugeKind, JudgeProperty};
use rbms_model::Mode;

use crate::JUDGE_RATE_UNMODIFIED;

pub(crate) fn gauge_from_name(s: &str) -> GaugeKind {
    match s.to_ascii_lowercase().as_str() {
        "assist" | "assisteasy" => GaugeKind::AssistEasy,
        "easy" => GaugeKind::Easy,
        "hard" => GaugeKind::Hard,
        "exhard" => GaugeKind::ExHard,
        "hazard" => GaugeKind::Hazard,
        _ => GaugeKind::Normal,
    }
}

pub(crate) fn ir_clear(c: ClearType) -> rbms_ir::ClearLamp {
    use rbms_ir::ClearLamp as L;
    match c {
        ClearType::NoPlay => L::NoPlay,
        ClearType::Failed => L::Failed,
        ClearType::AssistEasy => L::AssistEasy,
        ClearType::Easy => L::Easy,
        ClearType::Normal => L::Normal,
        ClearType::Hard => L::Hard,
        ClearType::ExHard => L::ExHard,
        ClearType::FullCombo => L::FullCombo,
        ClearType::Perfect => L::Perfect,
        ClearType::Max => L::Max,
    }
}

pub(crate) fn ir_gauge(g: GaugeKind) -> rbms_ir::GaugeType {
    use rbms_ir::GaugeType as G;
    match g {
        GaugeKind::AssistEasy => G::AssistEasy,
        GaugeKind::Easy => G::Easy,
        GaugeKind::Normal => G::Normal,
        GaugeKind::Hard => G::Hard,
        GaugeKind::ExHard => G::ExHard,
        GaugeKind::Hazard => G::Hazard,
    }
}

pub(crate) fn ir_random(n: NoteOption) -> RandomOption {
    match n {
        NoteOption::Off => RandomOption::Off,
        NoteOption::Mirror => RandomOption::Mirror,
        NoteOption::Random => RandomOption::Random,
        NoteOption::SRandom => RandomOption::SRandom,
        NoteOption::RRandom => RandomOption::RRandom,
        NoteOption::Rotate => RandomOption::Spiral,
        NoteOption::HRandom => RandomOption::HRandom,
        NoteOption::AllScratch => RandomOption::AllScratch,
    }
}

/// Map the chart's `#LNMODE` (0=undefined→LN, 1=LN, 2=CN, 3=HCN) to the IR `lntype` encoding
/// (0=LN, 1=CN, 2=HCN — the rbms backend `data-model.md`/`compatibility.md` contract, which mirrors
/// `LnKind` ordering). The reference implementation folds undefined to plain LN, so 0 and 1 both yield 0.
pub(crate) fn ir_lntype(lnmode: i32) -> i32 {
    match lnmode {
        2 => 1,
        3 => 2,
        _ => 0,
    }
}

/// Assist tags reported with a submission, one per active assist. Mirrors the reference implementation's assist level
/// sources for the options this client exposes: an auto-played lane (`AutoplayModifier` raises
/// `AssistLevel.ASSIST`, `BMSPlayer.java:233-234`) and a judge window widened past 100%
/// (`BMSPlayer.java:207-213`). Empty when the run used no assist.
pub(crate) fn assist_flags(scratch_auto: bool, judge_rate: i32) -> Vec<String> {
    let mut out = Vec::new();
    if scratch_auto {
        out.push("AUTO_SCRATCH".to_string());
    }
    if judge_rate > JUDGE_RATE_UNMODIFIED {
        out.push("CUSTOM_JUDGE".to_string());
    }
    out
}

/// How many judgments in `counts` (indexed PG, GR, GD, BD, PR, MS) actually broke the combo, per
/// the mode's `JudgeProperty.combo` table. Not the same as `minbp`: on the BEAT-7K family an empty
/// POOR (index 5) keeps the combo, while 5-key and PMS reset on it.
pub(crate) fn combo_breaks(mode: &Mode, counts: [u32; 6]) -> u32 {
    let combo = JudgeProperty::for_mode(mode).combo;
    counts.iter().zip(combo).filter(|(_, keeps)| !keeps).map(|(n, _)| *n).sum()
}

/// Canonical gauge token for settings storage (matches `gauge_from_name`'s vocabulary, so it
/// round-trips — unlike the display name `gauge_name` which has spaces/hyphens).
pub(crate) fn gauge_token(g: GaugeKind) -> &'static str {
    match g {
        GaugeKind::AssistEasy => "assist",
        GaugeKind::Easy => "easy",
        GaugeKind::Normal => "normal",
        GaugeKind::Hard => "hard",
        GaugeKind::ExHard => "exhard",
        GaugeKind::Hazard => "hazard",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_GAUGES: [GaugeKind; 6] = [
        GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal,
        GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard,
    ];
    const ALL_CLEARS: [ClearType; 10] = [
        ClearType::NoPlay, ClearType::Failed, ClearType::AssistEasy, ClearType::Easy,
        ClearType::Normal, ClearType::Hard, ClearType::ExHard, ClearType::FullCombo,
        ClearType::Perfect, ClearType::Max,
    ];

    #[test]
    fn assist_flags_empty_for_an_unassisted_run() {
        assert!(assist_flags(false, 100).is_empty());
        assert!(assist_flags(false, 50).is_empty(), "a narrowed judge window is not an assist");
    }

    #[test]
    fn assist_flags_report_auto_scratch_and_custom_judge() {
        assert_eq!(assist_flags(true, 100), vec!["AUTO_SCRATCH".to_string()]);
        assert_eq!(assist_flags(false, 105), vec!["CUSTOM_JUDGE".to_string()]);
        assert_eq!(assist_flags(true, 200), vec!["AUTO_SCRATCH".to_string(), "CUSTOM_JUDGE".to_string()]);
    }

    // --- gauge_from_name <-> gauge_token round-trip ---

    #[test]
    fn gauge_token_round_trips_through_gauge_from_name() {
        for g in ALL_GAUGES {
            assert_eq!(gauge_from_name(gauge_token(g)), g, "{g:?} token round-trips");
        }
    }

    #[test]
    fn gauge_tokens_are_distinct_and_lowercase() {
        let mut toks: Vec<&str> = ALL_GAUGES.iter().map(|&g| gauge_token(g)).collect();
        for t in &toks {
            assert_eq!(*t, t.to_ascii_lowercase(), "token {t:?} is lowercase");
            assert!(!t.contains(' ') && !t.contains('-'), "token {t:?} is settings-safe");
        }
        let n = toks.len();
        toks.sort_unstable();
        toks.dedup();
        assert_eq!(toks.len(), n, "tokens distinct");
    }

    #[test]
    fn gauge_from_name_is_case_insensitive() {
        assert_eq!(gauge_from_name("HARD"), GaugeKind::Hard);
        assert_eq!(gauge_from_name("Hard"), GaugeKind::Hard);
        assert_eq!(gauge_from_name("ExHard"), GaugeKind::ExHard);
        assert_eq!(gauge_from_name("EXHARD"), GaugeKind::ExHard);
    }

    #[test]
    fn gauge_from_name_assist_aliases() {
        assert_eq!(gauge_from_name("assist"), GaugeKind::AssistEasy);
        assert_eq!(gauge_from_name("assisteasy"), GaugeKind::AssistEasy);
        assert_eq!(gauge_from_name("ASSISTEASY"), GaugeKind::AssistEasy);
    }

    #[test]
    fn gauge_from_name_unknown_defaults_to_normal() {
        for s in ["", "bogus", "normalish", "ex-hard", "very easy", "NORMAL "] {
            // note: "NORMAL " (trailing space) and "ex-hard" are NOT recognized tokens, so they
            // fall through to the Normal default.
            assert_eq!(gauge_from_name(s), GaugeKind::Normal, "{s:?} => Normal default");
        }
    }

    // --- ir_clear total mapping ---

    #[test]
    fn ir_clear_total_mapping_and_distinct() {
        use rbms_ir::ClearLamp as L;
        let expected = [
            (ClearType::NoPlay, L::NoPlay),
            (ClearType::Failed, L::Failed),
            (ClearType::AssistEasy, L::AssistEasy),
            (ClearType::Easy, L::Easy),
            (ClearType::Normal, L::Normal),
            (ClearType::Hard, L::Hard),
            (ClearType::ExHard, L::ExHard),
            (ClearType::FullCombo, L::FullCombo),
            (ClearType::Perfect, L::Perfect),
            (ClearType::Max, L::Max),
        ];
        for (c, l) in expected {
            assert_eq!(ir_clear(c), l, "{c:?} maps to its IR lamp");
        }
        // injective: 10 distinct IR lamps for the 10 ClearTypes
        let lamps: Vec<L> = ALL_CLEARS.iter().map(|&c| ir_clear(c)).collect();
        for i in 0..lamps.len() {
            for j in (i + 1)..lamps.len() {
                assert_ne!(lamps[i], lamps[j], "ir_clear is injective");
            }
        }
    }

    #[test]
    fn ir_clear_never_emits_light_assist_easy() {
        // The IR has a LightAssistEasy lamp rbms never produces (no source ClearType maps to it).
        use rbms_ir::ClearLamp as L;
        assert!(ALL_CLEARS.iter().all(|&c| ir_clear(c) != L::LightAssistEasy));
    }

    // --- ir_gauge total mapping ---

    #[test]
    fn ir_gauge_total_mapping_and_distinct() {
        use rbms_ir::GaugeType as G;
        let expected = [
            (GaugeKind::AssistEasy, G::AssistEasy),
            (GaugeKind::Easy, G::Easy),
            (GaugeKind::Normal, G::Normal),
            (GaugeKind::Hard, G::Hard),
            (GaugeKind::ExHard, G::ExHard),
            (GaugeKind::Hazard, G::Hazard),
        ];
        for (g, t) in expected {
            assert_eq!(ir_gauge(g), t, "{g:?} maps to its IR gauge");
        }
        let gauges: Vec<G> = ALL_GAUGES.iter().map(|&g| ir_gauge(g)).collect();
        for i in 0..gauges.len() {
            for j in (i + 1)..gauges.len() {
                assert_ne!(gauges[i], gauges[j], "ir_gauge is injective");
            }
        }
    }

    #[test]
    fn ir_gauge_never_emits_class_gauges() {
        // IR has Class/ExClass/ExHardClass (dan-course gauges) rbms never plays; none are produced.
        use rbms_ir::GaugeType as G;
        for g in ALL_GAUGES {
            assert!(!matches!(ir_gauge(g), G::Class | G::ExClass | G::ExHardClass));
        }
    }

    // --- ir_random total mapping ---

    #[test]
    fn ir_random_total_mapping_with_rotate_to_spiral() {
        use rbms_ir::RandomOption as R;
        let expected = [
            (NoteOption::Off, R::Off),
            (NoteOption::Mirror, R::Mirror),
            (NoteOption::Random, R::Random),
            (NoteOption::SRandom, R::SRandom),
            (NoteOption::RRandom, R::RRandom),
            (NoteOption::Rotate, R::Spiral),   // engine "Rotate" == IR "Spiral"
            (NoteOption::HRandom, R::HRandom),
            (NoteOption::AllScratch, R::AllScratch),
        ];
        for (n, r) in expected {
            assert_eq!(ir_random(n), r, "{n:?} maps to its IR random");
        }
    }

    #[test]
    fn ir_random_covers_every_note_option_injectively() {
        use rbms_ir::RandomOption as R;
        let mapped: Vec<R> = NoteOption::ALL.iter().map(|&n| ir_random(n)).collect();
        assert_eq!(mapped.len(), NoteOption::ALL.len(), "every NoteOption is mapped");
        for i in 0..mapped.len() {
            for j in (i + 1)..mapped.len() {
                assert_ne!(mapped[i], mapped[j], "ir_random is injective");
            }
        }
        // IR-only options that rbms never produces
        assert!(mapped.iter().all(|&r| r != R::Converge), "Converge is never produced");
    }

    #[test]
    fn ir_random_name_round_trips_via_note_option_from_str() {
        // A persisted random token (NoteOption::label) parses back to the same NoteOption, which
        // then maps to a stable IR option.
        for n in NoteOption::ALL {
            let parsed = NoteOption::from_str(n.label());
            assert_eq!(parsed, n, "{:?} label round-trips through from_str", n);
            assert_eq!(ir_random(parsed), ir_random(n));
        }
    }

    #[test]
    fn combo_breaks_excludes_the_empty_poor_on_seven_keys() {
        assert_eq!(combo_breaks(&Mode::BEAT_7K, [9, 8, 7, 6, 5, 4]), 11);
        assert_eq!(combo_breaks(&Mode::BEAT_14K, [0, 0, 0, 2, 3, 100]), 5, "empty poors never break a 7K-family combo");
    }

    #[test]
    fn combo_breaks_counts_the_empty_poor_on_five_keys_and_pms() {
        assert_eq!(combo_breaks(&Mode::BEAT_5K, [9, 8, 7, 6, 5, 4]), 15);
        assert_eq!(combo_breaks(&Mode::POPN_9K, [9, 8, 7, 6, 5, 4]), 15);
    }

    #[test]
    fn combo_breaks_is_zero_for_a_clean_run() {
        assert_eq!(combo_breaks(&Mode::BEAT_7K, [500, 40, 3, 0, 0, 0]), 0);
    }

    #[test]
    fn ir_lntype_maps_lnmode_to_backend_encoding() {
        assert_eq!(ir_lntype(0), 0, "undefined -> LN");
        assert_eq!(ir_lntype(1), 0, "explicit LN");
        assert_eq!(ir_lntype(2), 1, "CN");
        assert_eq!(ir_lntype(3), 2, "HCN");
        assert_eq!(ir_lntype(99), 0, "unknown -> LN fallback");
    }
}
