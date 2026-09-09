use crate::Judge;

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

enum Modifier {
    Total,
    LimitIncrement,
    None,
}

struct Spec {
    modifier: Modifier,
    min: f32,
    max: f32,
    init: f32,
    border: f32,
    deltas: [f32; 6],
    guts: &'static [(f32, f32)],
}

const HARD_GUTS: &[(f32, f32)] = &[(10.0, 0.4), (20.0, 0.5), (30.0, 0.6), (40.0, 0.7), (50.0, 0.8)];

fn spec(kind: GaugeKind) -> Spec {
    use GaugeKind::*;
    use Modifier::*;
    match kind {
        AssistEasy => Spec { modifier: Total, min: 2.0, max: 100.0, init: 20.0, border: 60.0, deltas: [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], guts: &[] },
        Easy => Spec { modifier: Total, min: 2.0, max: 100.0, init: 20.0, border: 80.0, deltas: [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], guts: &[] },
        Normal => Spec { modifier: Total, min: 2.0, max: 100.0, init: 20.0, border: 80.0, deltas: [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], guts: &[] },
        Hard => Spec { modifier: LimitIncrement, min: 0.0, max: 100.0, init: 100.0, border: 0.0, deltas: [0.15, 0.12, 0.03, -5.0, -10.0, -5.0], guts: HARD_GUTS },
        ExHard => Spec { modifier: LimitIncrement, min: 0.0, max: 100.0, init: 100.0, border: 0.0, deltas: [0.15, 0.06, 0.0, -8.0, -16.0, -8.0], guts: &[] },
        Hazard => Spec { modifier: Modifier::None, min: 0.0, max: 100.0, init: 100.0, border: 0.0, deltas: [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], guts: &[] },
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
    guts: &'static [(f32, f32)],
}

impl Gauge {
    pub fn new(kind: GaugeKind, total: f64, notes: usize) -> Self {
        let s = spec(kind);
        let notes = notes.max(1) as f64;
        let total = if total > 0.0 { total } else { rbms_model::default_total(notes as usize) };
        let mut deltas = s.deltas;
        match s.modifier {
            Modifier::Total => {
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d = (*d as f64 * total / notes) as f32;
                    }
                }
            }
            Modifier::LimitIncrement => {
                let pg = (((2.0 * total - 320.0) / notes).min(0.15)).max(0.0) as f32;
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d *= pg / 0.15;
                    }
                }
            }
            Modifier::None => {}
        }
        Gauge { kind, value: s.init, min: s.min, max: s.max, border: s.border, deltas, guts: s.guts }
    }

    pub fn update(&mut self, judge: Judge) {
        if self.value <= 0.0 {
            return;
        }
        let mut inc = self.deltas[judge as usize];
        if inc < 0.0 {
            for g in self.guts {
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

#[cfg(test)]
mod gauge_tests {
    use super::*;

    const ALL_KINDS: [GaugeKind; 6] =
        [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];

    // --- init / spec wiring -------------------------------------------------

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
        // is_cleared() is `value >= border && value > 0`. The init values let us probe the border:
        // AssistEasy init 20 < border 60 (not cleared at start); survival init 100 >= border 0.
        assert!(!Gauge::new(GaugeKind::AssistEasy, 200.0, 10).is_cleared(), "AE border 60 > init 20");
        assert!(!Gauge::new(GaugeKind::Easy, 200.0, 10).is_cleared(), "Easy border 80 > init 20");
        assert!(!Gauge::new(GaugeKind::Normal, 200.0, 10).is_cleared(), "Normal border 80 > init 20");
        assert!(Gauge::new(GaugeKind::Hard, 200.0, 10).is_cleared(), "Hard border 0 <= init 100");
        assert!(Gauge::new(GaugeKind::ExHard, 200.0, 10).is_cleared());
        assert!(Gauge::new(GaugeKind::Hazard, 200.0, 10).is_cleared());
    }

    // --- TOTAL modifier -----------------------------------------------------

    #[test]
    fn total_modifier_scales_positive_deltas_by_total_over_notes() {
        // Normal PG delta is 1.0, scaled by total/notes. total=200 notes=4 -> 50.0 per PG.
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 4);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value() - before, 50.0, "1.0 * 200/4 = 50");
    }

    #[test]
    fn total_modifier_does_not_scale_negative_deltas() {
        // Normal BAD delta is a fixed -3.0 regardless of total/notes (only positives are scaled).
        // Use a high value so guts (Normal has none) and clamping are irrelevant.
        let mut g = Gauge::new(GaugeKind::Normal, 1000.0, 1);
        // raise to ~90 first via one PG (1.0*1000/1=1000, clamped to max 100)
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 100.0, "PG overshoots, clamps to max 100");
        g.update(Judge::Bad);
        assert_eq!(g.value(), 97.0, "BAD is a flat -3.0");
    }

    #[test]
    fn total_zero_falls_back_to_200() {
        // total <= 0 defaults to 200. Normal PG: 1.0 * 200/2 = 100 -> clamps to max.
        let mut g = Gauge::new(GaugeKind::Normal, 0.0, 2);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 100.0);
        // negative total also defaults to 200.
        let mut g2 = Gauge::new(GaugeKind::Normal, -5.0, 2);
        g2.update(Judge::PerfectGreat);
        assert_eq!(g2.value(), 100.0);
    }

    #[test]
    fn notes_zero_is_clamped_to_one() {
        // notes.max(1): 0 notes must not divide by zero; PG gain = 1.0 * 200/1 = 200 -> max.
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 0);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 100.0);
    }

    // --- LIMIT_INCREMENT modifier ------------------------------------------

    #[test]
    fn limit_increment_caps_pg_gain_at_015() {
        // Hard pg gain capped at 0.15 when (2*total-320)/notes >= 0.15. total=300 notes=1 -> 280/1.
        let mut g = Gauge::new(GaugeKind::Hard, 300.0, 1);
        // start at 100 (max). Drain via a BAD to leave room, then PG.
        g.update(Judge::Bad); // -5.0 -> 95 (no guts above 50)
        assert_eq!(g.value(), 95.0);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert!((g.value() - before - 0.15).abs() < 1e-4, "PG gain capped at 0.15, got {}", g.value() - before);
    }

    #[test]
    fn limit_increment_shrinks_gain_below_cap() {
        // (2*total-320)/notes below 0.15: total=162.5 notes=100 -> (325-320)/100 = 5/100 = 0.05.
        // PG scaled by pg/0.15: 0.15 * (0.05/0.15) = 0.05.
        let mut g = Gauge::new(GaugeKind::Hard, 162.5, 100);
        g.update(Judge::Bad); // make room: 100 -> 95
        let before = g.value();
        g.update(Judge::PerfectGreat);
        let gained = g.value() - before;
        assert!((gained - 0.05).abs() < 1e-4, "PG gain {gained} ~= 0.05");
    }

    #[test]
    fn limit_increment_clamps_negative_pg_to_zero_gain() {
        // (2*total-320) negative -> pg capped to 0.0, so positive deltas become 0: PG gives nothing.
        let mut g = Gauge::new(GaugeKind::Hard, 100.0, 1); // 2*100-320 = -120 -> pg 0
        g.update(Judge::Bad); // 100 -> 95
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), before, "PG yields no gain when pg-cap is 0");
    }

    // --- HARD guts softening -----------------------------------------------

    #[test]
    fn hard_guts_soften_damage_at_low_values() {
        // HARD_GUTS: value<10 -> 0.4. A BAD (-5.0) at value 5 becomes -2.0 -> 3.0.
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
        // The loop breaks on the FIRST band where value < g.0. value 45 is < 50 (mul 0.8) but not <40.
        let mut g = Gauge::new(GaugeKind::Hard, 1000.0, 1);
        drain_to(&mut g, 45.0);
        let v = g.value();
        let expected = (v - 5.0 * 0.8).clamp(0.0, 100.0);
        g.update(Judge::Bad);
        assert!((g.value() - expected).abs() < 1e-4, "value~45 uses 0.8 band: got {}, want {expected}", g.value());
    }

    #[test]
    fn no_guts_above_top_band_full_damage() {
        // value >= 50 -> no band matches, full -5.0 damage.
        let mut g = Gauge::new(GaugeKind::Hard, 1000.0, 1); // init 100
        let before = g.value();
        g.update(Judge::Bad);
        assert_eq!(before - g.value(), 5.0, "full BAD damage above guts bands");
    }

    #[test]
    fn exhard_has_no_guts() {
        // ExHard guts is empty: BAD is a flat -8.0 even at very low value.
        let mut g = Gauge::new(GaugeKind::ExHard, 1000.0, 1);
        drain_to(&mut g, 5.0); // ExHard miss/bad -8 each
        let v = g.value();
        let expected = (v - 8.0).clamp(0.0, 100.0);
        g.update(Judge::Bad);
        assert!((g.value() - expected).abs() < 1e-4, "no guts softening for ExHard: got {}, want {expected}", g.value());
    }

    // --- clamping & dead gauge ---------------------------------------------

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
        // Normal min is 2.0; draining never reaches 0, so update() keeps applying (value>0).
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 1);
        for _ in 0..100 {
            g.update(Judge::Poor); // -6.0 each
        }
        assert_eq!(g.value(), 2.0, "Total gauge floors at min 2.0");
        // still alive: a PG can lift it back up.
        g.update(Judge::PerfectGreat);
        assert!(g.value() > 2.0, "Total gauge at min is not permanently dead");
    }

    #[test]
    fn survival_gauge_dies_at_zero_and_stays_dead() {
        // Hazard BAD is -100 -> value 0 (min 0). update() then returns early forever, even on PG.
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
            g.update(Judge::Miss); // -10 softened by guts at low values, eventually reaches 0
        }
        assert_eq!(g.value(), 0.0);
        g.update(Judge::PerfectGreat);
        assert_eq!(g.value(), 0.0, "once 0, survival gauge never recovers");
    }

    // --- per-judge deltas: GD / GR signs -----------------------------------

    #[test]
    fn good_is_positive_for_total_zero_for_exhard() {
        // Normal GD delta 0.5 (positive, scaled). ExHard GD delta 0.0 (no change).
        let mut n = Gauge::new(GaugeKind::Normal, 200.0, 1);
        n.update(Judge::Bad); // make headroom: 20 -> 17
        let before = n.value();
        n.update(Judge::Good);
        assert!(n.value() > before, "Normal GOOD raises gauge");

        let mut x = Gauge::new(GaugeKind::ExHard, 200.0, 1);
        x.update(Judge::Bad); // 100 -> 92
        let before = x.value();
        x.update(Judge::Good);
        assert_eq!(x.value(), before, "ExHard GOOD delta is 0.0");
    }

    #[test]
    fn is_cleared_requires_strictly_positive_value() {
        // Hard border is 0.0. A gauge sitting exactly at 0 must report NOT cleared.
        let mut g = Gauge::new(GaugeKind::Hard, 200.0, 1);
        for _ in 0..200 {
            g.update(Judge::Miss);
        }
        assert_eq!(g.value(), 0.0);
        assert!(!g.is_cleared(), "value==border==0 is not cleared because value must be > 0");
    }

    // --- clear_lamp tiers ---------------------------------------------------

    fn cleared_normal_gauge() -> Gauge {
        Gauge::new(GaugeKind::Normal, 200.0, 1) // init 20 >= border 80? no. lift it.
    }

    #[test]
    fn lamp_noplay_when_zero_notes() {
        let g = Gauge::new(GaugeKind::Normal, 200.0, 0);
        assert_eq!(clear_lamp(&g, &[0; 6], 0, 0), ClearType::NoPlay);
    }

    #[test]
    fn lamp_failed_when_not_cleared() {
        let g = cleared_normal_gauge(); // value 20 < border 80 -> not cleared
        assert_eq!(clear_lamp(&g, &[1, 0, 0, 0, 0, 0], 1, 1), ClearType::Failed);
    }

    fn high_normal() -> Gauge {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 1);
        for _ in 0..10 {
            g.update(Judge::PerfectGreat); // -> max 100 >= border 80
        }
        g
    }

    #[test]
    fn lamp_max_all_pg() {
        let g = high_normal();
        // counts all PG (index 0), no GR/GD, no break, max_combo == total.
        assert_eq!(clear_lamp(&g, &[5, 0, 0, 0, 0, 0], 5, 5), ClearType::Max);
    }

    #[test]
    fn lamp_perfect_when_only_great_no_good() {
        let g = high_normal();
        // GR present (index 1), no GD (index 2), no break -> Perfect.
        assert_eq!(clear_lamp(&g, &[4, 1, 0, 0, 0, 0], 5, 5), ClearType::Perfect);
    }

    #[test]
    fn lamp_fullcombo_when_good_present_no_break() {
        let g = high_normal();
        // GD present (index 2), no break, full combo -> FullCombo.
        assert_eq!(clear_lamp(&g, &[3, 1, 1, 0, 0, 0], 5, 5), ClearType::FullCombo);
    }

    #[test]
    fn lamp_drops_to_gauge_kind_when_combo_broken() {
        let g = high_normal();
        // A single BAD breaks the combo -> falls through to gauge-kind lamp (Normal).
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 1, 0, 0], 4, 5), ClearType::Normal);
    }

    #[test]
    fn lamp_drops_to_gauge_kind_when_combo_short() {
        let g = high_normal();
        // No break, but max_combo < total (e.g. a swept MISS that was re-counted) -> kind lamp.
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 0, 0, 0], 4, 5), ClearType::Normal);
    }

    #[test]
    fn lamp_kind_mapping_for_each_gauge() {
        // With a broken combo and a cleared gauge, the lamp equals the gauge family.
        let pairs = [
            (GaugeKind::AssistEasy, ClearType::AssistEasy),
            (GaugeKind::Easy, ClearType::Easy),
            (GaugeKind::Normal, ClearType::Normal),
            (GaugeKind::Hard, ClearType::Hard),
            (GaugeKind::ExHard, ClearType::ExHard),
            (GaugeKind::Hazard, ClearType::ExHard),
        ];
        for (kind, lamp) in pairs {
            // Build a cleared gauge of this kind: survival start cleared; total gauges need lifting.
            let mut g = Gauge::new(kind, 200.0, 1);
            for _ in 0..200 {
                g.update(Judge::PerfectGreat);
            }
            assert!(g.is_cleared(), "{kind:?} should be cleared after many PG");
            // broken combo via a BAD count.
            assert_eq!(clear_lamp(&g, &[3, 0, 0, 1, 0, 0], 3, 4), lamp, "{kind:?}");
        }
    }

    #[test]
    fn lamp_break_detected_via_bd_or_poor_but_not_empty_poor() {
        let g = high_normal();
        // BAD only.
        assert_eq!(clear_lamp(&g, &[3, 0, 0, 1, 0, 0], 4, 4), ClearType::Normal);
        assert_eq!(clear_lamp(&g, &[3, 0, 0, 0, 1, 0], 4, 4), ClearType::Normal);
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 0, 0, 1], 4, 4), ClearType::Max);
    }

    #[test]
    fn lamp_max_requires_full_combo_count() {
        // Even all-PG, if max_combo != total_notes it is NOT Max (falls to kind lamp).
        let g = high_normal();
        assert_eq!(clear_lamp(&g, &[4, 0, 0, 0, 0, 0], 3, 4), ClearType::Normal);
    }

    // helpers ---------------------------------------------------------------

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
