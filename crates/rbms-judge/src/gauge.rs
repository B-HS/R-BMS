use crate::Judge;
use crate::data::{GaugeModifier, GaugeParams};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeKind {
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    Hazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    NoPlay,
    Failed,
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    FullCombo,
    Perfect,
    Max,
}

const HARD_GUTS: &[(f32, f32)] = &[(10.0, 0.4), (20.0, 0.5), (30.0, 0.6), (40.0, 0.7), (50.0, 0.8)];

/// The parameters a gauge is built from, read from the bundled `data/gauge.ron`.
///
/// The data file is the source the engine builds gauges from; [`default_params`] is the
/// compiled-in fallback for a file with no row to answer with, and the parity guard in
/// [`crate::data`] asserts the two agree field for field.
pub fn params(kind: GaugeKind) -> GaugeParams {
    match crate::data::builtin_gauge_tables().get(crate::data::DEFAULT_GAUGE_KEY) {
        Some(set) => set.get(kind).clone(),
        None => default_params(kind),
    }
}

/// The program-default parameters for `kind`, mirroring the reference implementation's
/// `GaugeProperty.SEVENKEYS` row of `GaugeElementProperty` values. The bundled `data/gauge.ron`
/// must reproduce these values exactly; the parity guard in [`crate::data`] asserts that against
/// this function.
pub fn default_params(kind: GaugeKind) -> GaugeParams {
    let p = |modifier, min, max, init, border, deltas, guts: &[(f32, f32)]| GaugeParams { modifier, min, max, init, border, deltas, guts: guts.to_vec() };
    match kind {
        GaugeKind::AssistEasy => p(GaugeModifier::Total, 2.0, 100.0, 20.0, 60.0, [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], &[]),
        GaugeKind::Easy => p(GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], &[]),
        GaugeKind::Normal => p(GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], &[]),
        GaugeKind::Hard => p(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -5.0, -10.0, -5.0], HARD_GUTS),
        GaugeKind::ExHard => p(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -8.0, -16.0, -8.0], &[]),
        GaugeKind::Hazard => p(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], &[]),
    }
}

/// One groove gauge (reference implementation `GrooveGauge`). Deltas are pre-modified at construction
/// by the gauge's modifier (TOTAL scales gains by chart total/notes; LIMIT_INCREMENT
/// caps the gain). A note's judge adds `deltas[judge]`, with HARD "guts" softening damage
/// at low values. Cleared when `value >= border` and `> 0` at the end.
#[derive(Debug, Clone)]
pub struct Gauge {
    kind: GaugeKind,
    value: f32,
    min: f32,
    max: f32,
    border: f32,
    deltas: [f32; 6],
    guts: Vec<(f32, f32)>,
}

/// Upper bound on the LIMIT_INCREMENT PGREAT gain and the divisor its scaling is expressed
/// against (reference implementation `GrooveGauge.GaugeModifier.LIMIT_INCREMENT`).
const LIMIT_INCREMENT_PGREAT_CAP: f64 = 0.15;

/// Constant term of the LIMIT_INCREMENT gain formula `(2 * total - 320) / notes`.
const LIMIT_INCREMENT_TOTAL_OFFSET: f64 = 320.0;

impl Gauge {
    pub fn new(kind: GaugeKind, total: f64, notes: usize) -> Self {
        Self::from_params(kind, &params(kind), total, notes)
    }

    /// Build a gauge from explicit parameters instead of the program defaults, so a data-driven
    /// gauge table (see [`crate::data::GaugeTables`]) can feed the same construction path.
    pub fn from_params(kind: GaugeKind, params: &GaugeParams, total: f64, notes: usize) -> Self {
        let notes = notes.max(1) as f64;
        let total = if total > 0.0 { total } else { rbms_model::default_total(notes as usize) };
        let mut deltas = params.deltas;
        match params.modifier {
            GaugeModifier::Total => {
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d = (*d as f64 * total / notes) as f32;
                    }
                }
            }
            GaugeModifier::LimitIncrement => {
                let pg = ((2.0 * total - LIMIT_INCREMENT_TOTAL_OFFSET) / notes).clamp(0.0, LIMIT_INCREMENT_PGREAT_CAP) as f32;
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d *= pg / LIMIT_INCREMENT_PGREAT_CAP as f32;
                    }
                }
            }
            GaugeModifier::None => {}
        }
        Gauge { kind, value: params.init, min: params.min, max: params.max, border: params.border, deltas, guts: params.guts.clone() }
    }

    pub fn update(&mut self, judge: Judge) {
        if self.value <= 0.0 {
            return;
        }
        let mut inc = self.deltas[judge as usize];
        if inc < 0.0 {
            for g in &self.guts {
                if self.value < g.0 {
                    inc *= g.1;
                    break;
                }
            }
        }
        self.value = (self.value + inc).clamp(self.min, self.max);
    }

    /// Add `delta` directly (mine damage). Reference implementation `GrooveGauge.addValue` -> `Gauge.setValue`:
    /// a dead gauge (<= 0) stays frozen, otherwise the result is clamped into `[min, max]`.
    pub fn add_value(&mut self, delta: f32) {
        if self.value <= 0.0 {
            return;
        }
        self.value = (self.value + delta).clamp(self.min, self.max);
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn kind(&self) -> GaugeKind {
        self.kind
    }

    pub fn is_cleared(&self) -> bool {
        self.value > 0.0 && self.value >= self.border
    }
}

/// Resolve the clear lamp from the final gauge state and judge tally.
pub fn clear_lamp(gauge: &Gauge, counts: &[u32; 6], max_combo: u32, total_notes: u32) -> ClearType {
    if total_notes == 0 {
        return ClearType::NoPlay;
    }
    if !gauge.is_cleared() {
        return ClearType::Failed;
    }
    let broke = counts[3] + counts[4] > 0;
    if !broke && max_combo == total_notes {
        if counts[1] == 0 && counts[2] == 0 {
            return ClearType::Max;
        }
        if counts[2] == 0 {
            return ClearType::Perfect;
        }
        return ClearType::FullCombo;
    }
    match gauge.kind {
        GaugeKind::AssistEasy => ClearType::AssistEasy,
        GaugeKind::Easy => ClearType::Easy,
        GaugeKind::Normal => ClearType::Normal,
        GaugeKind::Hard => ClearType::Hard,
        GaugeKind::ExHard | GaugeKind::Hazard => ClearType::ExHard,
    }
}

/// Reference implementation `ClearType` id (`ClearType.java`) for a lamp — persisted in score
/// records and sent to the IR so the lamp round-trips. Id 3 (LightAssistEasy) is never produced
/// because this engine has no separate light-assist lamp.
pub fn clear_type_id(c: ClearType) -> u8 {
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

/// Inverse of [`clear_type_id`]. Legacy id 3 (LightAssistEasy) folds down to
/// [`ClearType::AssistEasy`]; anything unknown reads back as [`ClearType::NoPlay`].
pub fn clear_type_from_id(id: u8) -> ClearType {
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

#[cfg(test)]
mod clear_type_id_tests {
    use super::*;

    const ALL_CLEARS: [ClearType; 10] = [
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
    ];

    #[test]
    fn clear_type_id_round_trips_for_every_lamp() {
        for c in ALL_CLEARS {
            assert_eq!(clear_type_from_id(clear_type_id(c)), c, "{c:?} round-trips");
        }
    }

    #[test]
    fn clear_type_ids_are_the_reference_values() {
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
        let ids: Vec<u8> = ALL_CLEARS.iter().map(|&c| clear_type_id(c)).collect();
        for w in ids.windows(2) {
            assert!(w[0] < w[1], "lamp ids must increase with lamp strength: {w:?}");
        }
    }

    #[test]
    fn clear_type_from_id_legacy_light_assist_maps_to_assist_easy() {
        assert_eq!(clear_type_from_id(3), ClearType::AssistEasy);
        assert_eq!(clear_type_id(clear_type_from_id(3)), 2, "3 folds down to the 2 lamp on re-encode");
    }

    #[test]
    fn clear_type_from_id_unknown_ids_are_no_play() {
        for id in [11u8, 12, 200, u8::MAX] {
            assert_eq!(clear_type_from_id(id), ClearType::NoPlay, "unknown id {id} => NoPlay");
        }
    }
}

#[cfg(test)]
mod gauge_tests {
    use super::*;

    const ALL_KINDS: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];

    #[test]
    fn total_gauges_init_at_20() {
        for k in [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal] {
            let g = Gauge::new(k, 200.0, 10);
            assert_eq!(g.value(), 20.0, "{k:?} init");
        }
    }

    #[test]
    fn survival_gauges_init_at_100() {
        for k in [GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard] {
            let g = Gauge::new(k, 200.0, 10);
            assert_eq!(g.value(), 100.0, "{k:?} init");
        }
    }

    #[test]
    fn kind_is_preserved() {
        for k in ALL_KINDS {
            assert_eq!(Gauge::new(k, 200.0, 10).kind(), k);
        }
    }

    #[test]
    fn borders_match_spec() {
        assert!(!Gauge::new(GaugeKind::AssistEasy, 200.0, 10).is_cleared(), "AE border 60 > init 20");
        assert!(!Gauge::new(GaugeKind::Easy, 200.0, 10).is_cleared(), "Easy border 80 > init 20");
        assert!(!Gauge::new(GaugeKind::Normal, 200.0, 10).is_cleared(), "Normal border 80 > init 20");
        assert!(Gauge::new(GaugeKind::Hard, 200.0, 10).is_cleared(), "Hard border 0 <= init 100");
        assert!(Gauge::new(GaugeKind::ExHard, 200.0, 10).is_cleared());
        assert!(Gauge::new(GaugeKind::Hazard, 200.0, 10).is_cleared());
    }

    #[test]
    fn total_modifier_scales_positive_deltas_by_total_over_notes() {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 4);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value() - before, 50.0, "1.0 * 200/4 = 50");
    }

    #[test]
    fn total_modifier_does_not_scale_negative_deltas() {
        let mut g = Gauge::new(GaugeKind::Normal, 1000.0, 1);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 100.0, "PG overshoots, clamps to max 100");
        g.update(Judge::Bad);
        assert_eq!(g.value(), 97.0, "BAD is a flat -3.0");
    }

    #[test]
    fn total_zero_falls_back_to_200() {
        let mut g = Gauge::new(GaugeKind::Normal, 0.0, 2);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 100.0);
        let mut g2 = Gauge::new(GaugeKind::Normal, -5.0, 2);
        g2.update(Judge::PerfectGreat);
        assert_eq!(g2.value(), 100.0);
    }

    #[test]
    fn notes_zero_is_clamped_to_one() {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 0);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 100.0);
    }

    #[test]
    fn limit_increment_caps_pg_gain_at_015() {
        let mut g = Gauge::new(GaugeKind::Hard, 300.0, 1);
        g.update(Judge::Bad);
        assert_eq!(g.value(), 95.0);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert!((g.value() - before - 0.15).abs() < 1e-4, "PG gain capped at 0.15, got {}", g.value() - before);
    }

    #[test]
    fn limit_increment_shrinks_gain_below_cap() {
        let mut g = Gauge::new(GaugeKind::Hard, 162.5, 100);
        g.update(Judge::Bad);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        let gained = g.value() - before;
        assert!((gained - 0.05).abs() < 1e-4, "PG gain {gained} ~= 0.05");
    }

    #[test]
    fn limit_increment_clamps_negative_pg_to_zero_gain() {
        let mut g = Gauge::new(GaugeKind::Hard, 100.0, 1);
        g.update(Judge::Bad);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), before, "PG yields no gain when pg-cap is 0");
    }

    #[test]
    fn hard_guts_soften_damage_at_low_values() {
        let mut g = Gauge::new(GaugeKind::Hard, 1000.0, 1);
        drain_to(&mut g, 5.0);
        let v = g.value();
        assert!(v <= 10.0 && v > 0.0, "value {v} in guts<10 band");
        let expected = (v - 5.0 * 0.4).clamp(0.0, 100.0);
        g.update(Judge::Bad);
        assert!((g.value() - expected).abs() < 1e-4, "guts 0.4 softening: got {}, want {expected}", g.value());
    }

    #[test]
    fn hard_guts_band_boundaries_pick_first_match() {
        let mut g = Gauge::new(GaugeKind::Hard, 1000.0, 1);
        drain_to(&mut g, 45.0);
        let v = g.value();
        let expected = (v - 5.0 * 0.8).clamp(0.0, 100.0);
        g.update(Judge::Bad);
        assert!((g.value() - expected).abs() < 1e-4, "value~45 uses 0.8 band: got {}, want {expected}", g.value());
    }

    #[test]
    fn no_guts_above_top_band_full_damage() {
        let mut g = Gauge::new(GaugeKind::Hard, 1000.0, 1);
        let before = g.value();
        g.update(Judge::Bad);
        assert_eq!(before - g.value(), 5.0, "full BAD damage above guts bands");
    }

    #[test]
    fn exhard_has_no_guts() {
        let mut g = Gauge::new(GaugeKind::ExHard, 1000.0, 1);
        drain_to(&mut g, 5.0);
        let v = g.value();
        let expected = (v - 8.0).clamp(0.0, 100.0);
        g.update(Judge::Bad);
        assert!((g.value() - expected).abs() < 1e-4, "no guts softening for ExHard: got {}, want {expected}", g.value());
    }

    #[test]
    fn value_clamps_to_max() {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 1);
        for _ in 0..10 {
            g.update(Judge::PerfectGreat);
        }
        assert_eq!(g.value(), 100.0, "never exceeds max");
    }

    #[test]
    fn total_gauge_clamps_to_min_2_and_stays_alive() {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 1);
        for _ in 0..100 {
            g.update(Judge::Poor);
        }
        assert_eq!(g.value(), 2.0, "Total gauge floors at min 2.0");
        g.update(Judge::PerfectGreat);
        assert!(g.value() > 2.0, "Total gauge at min is not permanently dead");
    }

    #[test]
    fn survival_gauge_dies_at_zero_and_stays_dead() {
        let mut g = Gauge::new(GaugeKind::Hazard, 200.0, 1);
        g.update(Judge::Bad);
        assert_eq!(g.value(), 0.0, "Hazard min is 0");
        assert!(!g.is_cleared(), "dead gauge is not cleared (value > 0 required)");
        for _ in 0..50 {
            g.update(Judge::PerfectGreat);
        }
        assert_eq!(g.value(), 0.0, "dead survival gauge is permanent");
    }

    #[test]
    fn hard_gauge_dead_at_zero_is_permanent() {
        let mut g = Gauge::new(GaugeKind::Hard, 200.0, 1);
        for _ in 0..200 {
            g.update(Judge::Miss);
        }
        assert_eq!(g.value(), 0.0);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 0.0, "once 0, survival gauge never recovers");
    }

    #[test]
    fn good_is_positive_for_total_zero_for_exhard() {
        let mut n = Gauge::new(GaugeKind::Normal, 200.0, 1);
        n.update(Judge::Bad);
        let before = n.value();
        n.update(Judge::Good);
        assert!(n.value() > before, "Normal GOOD raises gauge");

        let mut x = Gauge::new(GaugeKind::ExHard, 200.0, 1);
        x.update(Judge::Bad);
        let before = x.value();
        x.update(Judge::Good);
        assert_eq!(x.value(), before, "ExHard GOOD delta is 0.0");
    }

    #[test]
    fn is_cleared_requires_strictly_positive_value() {
        let mut g = Gauge::new(GaugeKind::Hard, 200.0, 1);
        for _ in 0..200 {
            g.update(Judge::Miss);
        }
        assert_eq!(g.value(), 0.0);
        assert!(!g.is_cleared(), "value==border==0 is not cleared because value must be > 0");
    }

    fn cleared_normal_gauge() -> Gauge {
        Gauge::new(GaugeKind::Normal, 200.0, 1)
    }

    #[test]
    fn lamp_noplay_when_zero_notes() {
        let g = Gauge::new(GaugeKind::Normal, 200.0, 0);
        assert_eq!(clear_lamp(&g, &[0; 6], 0, 0), ClearType::NoPlay);
    }

    #[test]
    fn lamp_failed_when_not_cleared() {
        let g = cleared_normal_gauge();
        assert_eq!(clear_lamp(&g, &[1, 0, 0, 0, 0, 0], 1, 1), ClearType::Failed);
    }

    fn high_normal() -> Gauge {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 1);
        for _ in 0..10 {
            g.update(Judge::PerfectGreat);
        }
        g
    }

    #[test]
    fn lamp_max_all_pg() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[5, 0, 0, 0, 0, 0], 5, 5), ClearType::Max);
    }

    #[test]
    fn lamp_perfect_when_only_great_no_good() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[4, 1, 0, 0, 0, 0], 5, 5), ClearType::Perfect);
    }

    #[test]
    fn lamp_fullcombo_when_good_present_no_break() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[3, 1, 1, 0, 0, 0], 5, 5), ClearType::FullCombo);
    }

    #[test]
    fn lamp_drops_to_gauge_kind_when_combo_broken() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 1, 0, 0], 4, 5), ClearType::Normal);
    }

    #[test]
    fn lamp_drops_to_gauge_kind_when_combo_short() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 0, 0, 0], 4, 5), ClearType::Normal);
    }

    #[test]
    fn lamp_kind_mapping_for_each_gauge() {
        let pairs = [
            (GaugeKind::AssistEasy, ClearType::AssistEasy),
            (GaugeKind::Easy, ClearType::Easy),
            (GaugeKind::Normal, ClearType::Normal),
            (GaugeKind::Hard, ClearType::Hard),
            (GaugeKind::ExHard, ClearType::ExHard),
            (GaugeKind::Hazard, ClearType::ExHard),
        ];
        for (kind, lamp) in pairs {
            let mut g = Gauge::new(kind, 200.0, 1);
            for _ in 0..200 {
                g.update(Judge::PerfectGreat);
            }
            assert!(g.is_cleared(), "{kind:?} should be cleared after many PG");
            assert_eq!(clear_lamp(&g, &[3, 0, 0, 1, 0, 0], 3, 4), lamp, "{kind:?}");
        }
    }

    #[test]
    fn lamp_break_detected_via_bd_or_poor_but_not_empty_poor() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[3, 0, 0, 1, 0, 0], 4, 4), ClearType::Normal);
        assert_eq!(clear_lamp(&g, &[3, 0, 0, 0, 1, 0], 4, 4), ClearType::Normal);
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 0, 0, 1], 4, 4), ClearType::Max);
    }

    #[test]
    fn lamp_max_requires_full_combo_count() {
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 0, 0, 0], 3, 4), ClearType::Normal);
    }

    /// Drain a survival gauge toward `target` (approximately, stopping at or below it) using MISS.
    fn drain_to(g: &mut Gauge, target: f32) {
        let mut guard = 0;
        while g.value() > target && guard < 10_000 {
            g.update(Judge::Miss);
            guard += 1;
        }
    }
}

#[cfg(test)]
mod total_modifier_tests {
    use super::*;

    #[test]
    fn normal_total_300_over_1000_notes_gains_0_3_per_pgreat() {
        let mut g = Gauge::new(GaugeKind::Normal, 300.0, 1000);
        g.update(Judge::PerfectGreat);
        assert!((g.value() - 20.3).abs() < 1e-4, "20.0 + 0.3, got {}", g.value());
        g.update(Judge::Good);
        assert!((g.value() - 20.45).abs() < 1e-4, "GOOD delta 0.5 * 0.3 = 0.15, got {}", g.value());
    }

    #[test]
    fn hard_limit_increment_clamps_the_pgreat_gain_at_0_15() {
        let mut g = Gauge::new(GaugeKind::Hard, 300.0, 1000);
        g.update(Judge::Bad);
        let after_bad = g.value();
        g.update(Judge::PerfectGreat);
        assert!((g.value() - (after_bad + 0.15)).abs() < 1e-4, "full 0.15 gain, got {}", g.value());
    }

    #[test]
    fn hard_limit_increment_scales_down_on_a_low_total_chart() {
        let mut g = Gauge::new(GaugeKind::Hard, 200.0, 1000);
        g.update(Judge::Bad);
        let after_bad = g.value();
        g.update(Judge::PerfectGreat);
        assert!((g.value() - (after_bad + 0.08)).abs() < 1e-4, "reduced 0.08 gain, got {}", g.value());
    }

    #[test]
    fn missing_total_falls_back_to_the_standard_default_formula() {
        let mut g = Gauge::new(GaugeKind::Normal, 0.0, 1000);
        g.update(Judge::PerfectGreat);
        assert!((g.value() - 20.460_909).abs() < 1e-3, "got {}", g.value());
    }
}
