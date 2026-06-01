//! Enum/string mapping between the engine types (`rbms_judge`/`rbms_chart`) and the IR contract
//! (`rbms_ir`) plus the persisted settings vocabulary. Extracted from `main.rs` — pure lookups,
//! no app state.

use rbms_chart::shuffle::NoteOption;
use rbms_ir::RandomOption;
use rbms_judge::{ClearType, GaugeKind};

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
}
