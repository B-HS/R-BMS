//! Enum/string mapping between the engine types (`rbms_judge`/`rbms_chart`) and this crate's IR
//! contract, plus the persisted settings vocabulary. Pure lookups, no app state.
//!
//! Behind the optional `mapping` feature so the DTO/HTTP core stays free of engine dependencies.

use rbms_chart::shuffle::NoteOption;
use rbms_judge::gauge::GaugeIndex;
use rbms_judge::{ClearType, GaugeKind, JudgeProperty};
use rbms_model::Mode;

use crate::RandomOption;

/// JUDGE WIDTH percentage that leaves the windows exactly as the chart defines them. Anything above
/// it widens the windows and counts as an assist (no score submission).
pub const JUDGE_RATE_UNMODIFIED: i32 = 100;

/// Parse a persisted gauge token back into the engine gauge, defaulting to NORMAL.
pub fn gauge_from_name(s: &str) -> GaugeKind {
    match s.to_ascii_lowercase().as_str() {
        "assist" | "assisteasy" => GaugeKind::AssistEasy,
        "easy" => GaugeKind::Easy,
        "hard" => GaugeKind::Hard,
        "exhard" => GaugeKind::ExHard,
        "hazard" => GaugeKind::Hazard,
        _ => GaugeKind::Normal,
    }
}

/// The IR clear lamp for an engine clear type.
pub fn ir_clear(c: ClearType) -> crate::ClearLamp {
    use crate::ClearLamp as L;
    match c {
        ClearType::NoPlay => L::NoPlay,
        ClearType::Failed => L::Failed,
        ClearType::AssistEasy => L::AssistEasy,
        ClearType::LightAssistEasy => L::LightAssistEasy,
        ClearType::Easy => L::Easy,
        ClearType::Normal => L::Normal,
        ClearType::Hard => L::Hard,
        ClearType::ExHard => L::ExHard,
        ClearType::FullCombo => L::FullCombo,
        ClearType::Perfect => L::Perfect,
        ClearType::Max => L::Max,
    }
}

/// The IR gauge type for an engine gauge.
pub fn ir_gauge(g: GaugeKind) -> crate::GaugeType {
    use crate::GaugeType as G;
    match g {
        GaugeKind::AssistEasy => G::AssistEasy,
        GaugeKind::Easy => G::Easy,
        GaugeKind::Normal => G::Normal,
        GaugeKind::Hard => G::Hard,
        GaugeKind::ExHard => G::ExHard,
        GaugeKind::Hazard => G::Hazard,
    }
}

/// The IR gauge type for one of the nine gauges a chart is played with: [`ir_gauge`] for a gauge a
/// player selects, and the course gauge of the same name for the three a course stage runs on.
pub fn ir_gauge_of_index(g: GaugeIndex) -> crate::GaugeType {
    use crate::GaugeType as G;
    match g {
        GaugeIndex::AssistEasy => G::AssistEasy,
        GaugeIndex::Easy => G::Easy,
        GaugeIndex::Normal => G::Normal,
        GaugeIndex::Hard => G::Hard,
        GaugeIndex::ExHard => G::ExHard,
        GaugeIndex::Hazard => G::Hazard,
        GaugeIndex::Class => G::Class,
        GaugeIndex::ExClass => G::ExClass,
        GaugeIndex::ExHardClass => G::ExHardClass,
    }
}

/// The IR random option for an engine note option (engine `Rotate` is the IR's `Spiral`).
pub fn ir_random(n: NoteOption) -> RandomOption {
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
pub fn ir_lntype(lnmode: i32) -> i32 {
    match lnmode {
        2 => 1,
        3 => 2,
        _ => 0,
    }
}

/// Assist level of a run that used no assist at all, the only level that keeps the score.
pub const NO_ASSIST: u8 = 0;

/// Assist level of an auto-played lane: the reference raises `assist` to 1 for the battle option
/// that auto-plays the scratch (`BMSPlayer.java:248-252`), which demotes the lamp to
/// `LightAssistEasy` (`BMSPlayer.java:866`).
pub const LIGHT_ASSIST: u8 = 1;

/// Assist level of a custom judge — any judge width or long-note margin widened past 100%
/// (`BMSPlayer.java:208-214`). Demotes the lamp to `AssistEasy`.
pub const CUSTOM_JUDGE_ASSIST: u8 = 2;

/// Tag reported for a run with an auto-played scratch lane.
pub const AUTO_SCRATCH_FLAG: &str = "AUTO_SCRATCH";

/// Tag reported for a run judged more leniently than the chart asks.
pub const CUSTOM_JUDGE_FLAG: &str = "CUSTOM_JUDGE";

/// How much assist a run used, on the reference implementation's own 0/1/2 scale
/// (`BMSPlayer.java:200-252`). The level decides which lamp the run is demoted to, and the
/// submission names the assists themselves ([`assist_flags`]).
///
/// What a level costs beyond the lamp is the client's rule, not this crate's. In rbms every run
/// played by hand is kept and submitted whatever its level (the player's decision of 2026-10-14):
/// a SCRATCH AUTO run, whose turntable notes are taken off the chart it is judged on, sets bests
/// like any other, and only a [`CUSTOM_JUDGE_ASSIST`] run sets none, as the reference's cleared
/// `score` flag has it.
pub fn assist_level(scratch_auto: bool, custom_judge: bool) -> u8 {
    if custom_judge {
        return CUSTOM_JUDGE_ASSIST;
    }
    if scratch_auto {
        return LIGHT_ASSIST;
    }
    NO_ASSIST
}

/// Assist tags reported with a submission, one per active assist. Mirrors the reference implementation's assist level
/// sources for the options this client exposes: an auto-played lane (`AutoplayModifier` raises
/// `AssistLevel.ASSIST`, `BMSPlayer.java:233-234`) and a judge window or long-note margin widened
/// past 100% (`BMSPlayer.java:208-214`). Empty when the run used no assist.
pub fn assist_flags(scratch_auto: bool, custom_judge: bool) -> Vec<String> {
    let mut out = Vec::new();
    if scratch_auto {
        out.push(AUTO_SCRATCH_FLAG.to_string());
    }
    if custom_judge {
        out.push(CUSTOM_JUDGE_FLAG.to_string());
    }
    out
}

/// How many judgments in `counts` (indexed PG, GR, GD, BD, PR, MS) actually broke the combo, per
/// the mode's `JudgeProperty.combo` table. Not the same as `minbp`: on the BEAT-7K family an empty
/// POOR (index 5) keeps the combo, while 5-key and PMS reset on it.
pub fn combo_breaks(mode: &Mode, counts: [u32; 6]) -> u32 {
    let combo = JudgeProperty::for_mode(mode).combo;
    counts.iter().zip(combo).filter(|(_, keeps)| !keeps).map(|(n, _)| *n).sum()
}

/// The chart a score of `model` is submitted for: its hashes, with the title, artist and level the
/// chart itself writes, which is how the server can name a chart nobody has submitted before.
pub fn submitted_chart(model: &rbms_model::Model) -> crate::SubmittedChart {
    let id = crate::ChartId { md5: model.md5.clone(), sha256: model.sha256.clone() };
    crate::SubmittedChart::described(id, &model.meta.title, &model.meta.artist, &model.meta.play_level)
}

/// Canonical gauge token for settings storage (matches `gauge_from_name`'s vocabulary, so it
/// round-trips — unlike the display name, which has spaces/hyphens).
pub fn gauge_token(g: GaugeKind) -> &'static str {
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

    const ALL_GAUGES: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];
    const ALL_CLEARS: [ClearType; 11] = [
        ClearType::NoPlay,
        ClearType::Failed,
        ClearType::AssistEasy,
        ClearType::LightAssistEasy,
        ClearType::Easy,
        ClearType::Normal,
        ClearType::Hard,
        ClearType::ExHard,
        ClearType::FullCombo,
        ClearType::Perfect,
        ClearType::Max,
    ];

    #[test]
    fn assist_flags_empty_for_an_unassisted_run() {
        assert!(assist_flags(false, false).is_empty());
        assert_eq!(assist_level(false, false), NO_ASSIST);
    }

    #[test]
    fn assist_flags_report_auto_scratch_and_custom_judge() {
        assert_eq!(assist_flags(true, false), vec![AUTO_SCRATCH_FLAG.to_string()]);
        assert_eq!(assist_flags(false, true), vec![CUSTOM_JUDGE_FLAG.to_string()]);
        assert_eq!(assist_flags(true, true), vec![AUTO_SCRATCH_FLAG.to_string(), CUSTOM_JUDGE_FLAG.to_string()]);
    }

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
            assert_eq!(gauge_from_name(s), GaugeKind::Normal, "{s:?} => Normal default");
        }
    }

    #[test]
    fn ir_clear_total_mapping_and_distinct() {
        use crate::ClearLamp as L;
        let expected = [
            (ClearType::NoPlay, L::NoPlay),
            (ClearType::Failed, L::Failed),
            (ClearType::AssistEasy, L::AssistEasy),
            (ClearType::LightAssistEasy, L::LightAssistEasy),
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
        let lamps: Vec<L> = ALL_CLEARS.iter().map(|&c| ir_clear(c)).collect();
        for i in 0..lamps.len() {
            for j in (i + 1)..lamps.len() {
                assert_ne!(lamps[i], lamps[j], "ir_clear is injective");
            }
        }
    }

    #[test]
    fn ir_clear_emits_light_assist_easy_only_for_the_engine_lamp_of_that_name() {
        use crate::ClearLamp as L;
        let emitting: Vec<ClearType> = ALL_CLEARS.into_iter().filter(|&c| ir_clear(c) == L::LightAssistEasy).collect();
        assert_eq!(emitting, [ClearType::LightAssistEasy]);
    }

    #[test]
    fn ir_gauge_total_mapping_and_distinct() {
        use crate::GaugeType as G;
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
        use crate::GaugeType as G;
        for g in ALL_GAUGES {
            assert!(!matches!(ir_gauge(g), G::Class | G::ExClass | G::ExHardClass));
        }
    }

    #[test]
    fn ir_gauge_of_index_agrees_with_ir_gauge_and_names_each_course_gauge() {
        use crate::GaugeType as G;
        for g in ALL_GAUGES {
            assert_eq!(ir_gauge_of_index(GaugeIndex::from_kind(g)), ir_gauge(g), "{g:?} is sent as the gauge it is");
        }
        let course: Vec<G> = [GaugeIndex::Class, GaugeIndex::ExClass, GaugeIndex::ExHardClass].into_iter().map(ir_gauge_of_index).collect();
        assert_eq!(course, [G::Class, G::ExClass, G::ExHardClass]);
    }

    #[test]
    fn ir_random_total_mapping_with_rotate_to_spiral() {
        use crate::RandomOption as R;
        let expected = [
            (NoteOption::Off, R::Off),
            (NoteOption::Mirror, R::Mirror),
            (NoteOption::Random, R::Random),
            (NoteOption::SRandom, R::SRandom),
            (NoteOption::RRandom, R::RRandom),
            (NoteOption::Rotate, R::Spiral),
            (NoteOption::HRandom, R::HRandom),
            (NoteOption::AllScratch, R::AllScratch),
        ];
        for (n, r) in expected {
            assert_eq!(ir_random(n), r, "{n:?} maps to its IR random");
        }
    }

    #[test]
    fn ir_random_covers_every_note_option_injectively() {
        use crate::RandomOption as R;
        let mapped: Vec<R> = NoteOption::ALL.iter().map(|&n| ir_random(n)).collect();
        assert_eq!(mapped.len(), NoteOption::ALL.len(), "every NoteOption is mapped");
        for i in 0..mapped.len() {
            for j in (i + 1)..mapped.len() {
                assert_ne!(mapped[i], mapped[j], "ir_random is injective");
            }
        }
        assert!(mapped.iter().all(|&r| r != R::Converge), "Converge is never produced");
    }

    #[test]
    fn ir_random_name_round_trips_via_note_option_from_str() {
        for n in NoteOption::ALL {
            let parsed = NoteOption::from_str(n.label());
            assert_eq!(parsed, n, "{n:?} label round-trips through from_str");
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
    fn a_submitted_chart_carries_what_the_chart_calls_itself() {
        let meta = rbms_model::ModelMeta { title: "Born".into(), artist: "29".into(), play_level: "16".into(), ..Default::default() };
        let model = rbms_model::Model {
            mode: Mode::BEAT_7K,
            meta,
            wavmap: Vec::new(),
            bgamap: Vec::new(),
            init_bpm: 150.0,
            timelines: Vec::new(),
            md5: "0077fe17f69a9db8922c16ac00df960f".into(),
            sha256: "ab".repeat(32),
        };
        let chart = submitted_chart(&model);
        assert_eq!(chart.id, crate::ChartId { md5: model.md5.clone(), sha256: model.sha256.clone() });
        assert_eq!((chart.title.as_deref(), chart.artist.as_deref(), chart.level), (Some("Born"), Some("29"), Some(16)));
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
