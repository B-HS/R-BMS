//! Per-mode groove gauge tables and the damage modifiers applied when a gauge is built.

use rbms_model::Mode;

use crate::data::{GaugeModifier, GaugeParams, GaugeSet, builtin_gauge_tables};
use crate::gauge::GaugeIndex;

/// Softening bands shared by the HARD gauge of every non-LR2 set (`GaugeProperty.java:90`).
/// A damage of `d` taken at a gauge value below the first matching threshold becomes `d * mult`.
pub const GUTS_HARD: &[(f32, f32)] = &[(10.0, 0.4), (20.0, 0.5), (30.0, 0.6), (40.0, 0.7), (50.0, 0.8)];

/// Softening bands of the course CLASS gauges (`GaugeProperty.java:93`).
pub const GUTS_CLASS: &[(f32, f32)] = &[(5.0, 0.4), (10.0, 0.5), (15.0, 0.6), (20.0, 0.7), (25.0, 0.8)];

/// The single softening band the LR2 set uses for HARD, CLASS and EXCLASS (`GaugeProperty.java:120`).
pub const GUTS_LR2: &[(f32, f32)] = &[(30.0, 0.6)];

/// No softening: damage is taken in full at every gauge value.
pub const GUTS_NONE: &[(f32, f32)] = &[];

/// Which of the reference implementation's five gauge tables a chart plays on
/// (`GaugeProperty.java:12-61`). [`GaugeSetId::for_mode`] picks one from the chart's mode;
/// [`GaugeSetId::Lr2`] is never picked that way because the reference reaches it only through a
/// course constraint (`GrooveGauge.java:142-144`), so this engine exposes it as a user setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GaugeSetId {
    FiveKeys,
    #[default]
    SevenKeys,
    Pms,
    Keyboard,
    Lr2,
}

/// Data-file keys of the five-key set, one per mode that plays on it.
const KEYS_FIVE: &[&str] = &["BEAT_5K", "BEAT_10K"];
const KEYS_SEVEN: &[&str] = &["BEAT_7K", "BEAT_14K"];
const KEYS_PMS: &[&str] = &["POPN_9K"];
const KEYS_KEYBOARD: &[&str] = &[crate::data::KEYBOARD_24K_KEY];

/// Data-file key of the LR2 set. It is not a mode id: the set is selected by the player, not by
/// the chart.
pub const LR2_GAUGE_KEY: &str = "LR2";

const KEYS_LR2: &[&str] = &[LR2_GAUGE_KEY];

impl GaugeSetId {
    /// How many gauge tables the reference implementation defines.
    pub const COUNT: usize = 5;

    /// Every set, in `GaugeProperty` declaration order.
    pub const ALL: [GaugeSetId; GaugeSetId::COUNT] = [GaugeSetId::FiveKeys, GaugeSetId::SevenKeys, GaugeSetId::Pms, GaugeSetId::Keyboard, GaugeSetId::Lr2];

    /// Row of [`GAUGE_TABLE`] this set occupies.
    pub fn index(self) -> usize {
        match self {
            GaugeSetId::FiveKeys => 0,
            GaugeSetId::SevenKeys => 1,
            GaugeSetId::Pms => 2,
            GaugeSetId::Keyboard => 3,
            GaugeSetId::Lr2 => 4,
        }
    }

    /// Every key the bundled gauge data file carries this set under. Modes that share a set each
    /// get their own key so a lookup by mode name resolves without a fallback, mirroring how the
    /// judge data file repeats a shared timing row.
    pub fn data_keys(self) -> &'static [&'static str] {
        match self {
            GaugeSetId::FiveKeys => KEYS_FIVE,
            GaugeSetId::SevenKeys => KEYS_SEVEN,
            GaugeSetId::Pms => KEYS_PMS,
            GaugeSetId::Keyboard => KEYS_KEYBOARD,
            GaugeSetId::Lr2 => KEYS_LR2,
        }
    }

    /// Primary data-file key of this set.
    pub fn data_key(self) -> &'static str {
        self.data_keys()[0]
    }

    /// The set `mode` plays on (`BMSPlayerRule.java:13-17`): five-key modes use FIVEKEYS, pop'n
    /// modes PMS, 24-key modes KEYBOARD and everything else SEVENKEYS.
    pub fn for_mode(mode: &Mode) -> GaugeSetId {
        match mode.name {
            "BEAT_5K" | "BEAT_10K" => GaugeSetId::FiveKeys,
            "POPN_5K" | "POPN_9K" => GaugeSetId::Pms,
            "KEYBOARD_24K" | "KEYBOARD_24K_DOUBLE" => GaugeSetId::Keyboard,
            _ => GaugeSetId::SevenKeys,
        }
    }
}

/// One row of the reference implementation's `GaugeProperty.GaugeElementProperty`
/// (`GaugeProperty.java:157-165`) in a form that can be a `const`: the gauge's bounds, its
/// per-judge deltas in judge order (PG, GR, GD, BD, PR, MS) and its softening bands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaugeElement {
    pub modifier: GaugeModifier,
    pub min: f32,
    pub max: f32,
    pub init: f32,
    pub border: f32,
    pub deltas: [f32; 6],
    pub guts: &'static [(f32, f32)],
}

impl GaugeElement {
    /// The owned, serialisable form of this row.
    pub fn to_params(self) -> GaugeParams {
        GaugeParams {
            modifier: self.modifier,
            min: self.min,
            max: self.max,
            init: self.init,
            border: self.border,
            deltas: self.deltas,
            guts: self.guts.to_vec(),
        }
    }
}

const fn element(modifier: GaugeModifier, min: f32, max: f32, init: f32, border: f32, deltas: [f32; 6], guts: &'static [(f32, f32)]) -> GaugeElement {
    GaugeElement { modifier, min, max, init, border, deltas, guts }
}

/// The reference implementation's whole gauge table (`GaugeProperty.java:77-125`): five sets of
/// nine gauges, indexed by [`GaugeSetId::index`] then [`GaugeIndex::index`].
pub const GAUGE_TABLE: [[GaugeElement; GaugeIndex::COUNT]; GaugeSetId::COUNT] = [
    [
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 50.0, [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 75.0, [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 75.0, [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], GUTS_NONE),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.0, 0.0, 0.0, -5.0, -10.0, -5.0], GUTS_NONE),
        element(GaugeModifier::ModifyDamage, 0.0, 100.0, 100.0, 0.0, [0.0, 0.0, 0.0, -10.0, -20.0, -10.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.0, 0.0, 0.0, -100.0, -100.0, -100.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.01, 0.01, 0.0, -0.5, -1.0, -0.5], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.01, 0.01, 0.0, -1.0, -2.0, -1.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.01, 0.01, 0.0, -2.5, -5.0, -2.5], GUTS_NONE),
    ],
    [
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 60.0, [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], GUTS_NONE),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -5.0, -10.0, -5.0], GUTS_HARD),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -8.0, -16.0, -8.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.06, -1.5, -3.0, -1.5], GUTS_CLASS),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -3.0, -6.0, -3.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -5.0, -10.0, -5.0], GUTS_NONE),
    ],
    [
        element(GaugeModifier::Total, 2.0, 120.0, 30.0, 65.0, [1.0, 1.0, 0.5, -1.0, -2.0, -2.0], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 120.0, 30.0, 85.0, [1.0, 1.0, 0.5, -1.0, -3.0, -3.0], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 120.0, 30.0, 85.0, [1.0, 1.0, 0.5, -2.0, -6.0, -6.0], GUTS_NONE),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -5.0, -10.0, -10.0], GUTS_HARD),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -10.0, -15.0, -15.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -100.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.06, -1.5, -3.0, -3.0], GUTS_CLASS),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -3.0, -6.0, -6.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -5.0, -10.0, -10.0], GUTS_NONE),
    ],
    [
        element(GaugeModifier::Total, 2.0, 100.0, 30.0, 50.0, [1.0, 1.0, 0.5, -1.0, -2.0, -1.0], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 70.0, [1.0, 1.0, 0.5, -1.0, -3.0, -1.0], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 70.0, [1.0, 1.0, 0.5, -2.0, -4.0, -2.0], GUTS_NONE),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.2, 0.2, 0.1, -4.0, -8.0, -4.0], GUTS_HARD),
        element(GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.2, 0.1, 0.0, -6.0, -12.0, -6.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.1, 0.0, -100.0, -100.0, -100.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.2, 0.1, -1.5, -3.0, -1.5], GUTS_CLASS),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.2, 0.1, -3.0, -6.0, -3.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.1, 0.0, -5.0, -10.0, -5.0], GUTS_NONE),
    ],
    [
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 60.0, [1.2, 1.2, 0.6, -3.2, -4.8, -1.6], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.2, 1.2, 0.6, -3.2, -4.8, -1.6], GUTS_NONE),
        element(GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -4.0, -6.0, -2.0], GUTS_NONE),
        element(GaugeModifier::ModifyDamage, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -6.0, -10.0, -2.0], GUTS_LR2),
        element(GaugeModifier::ModifyDamage, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -12.0, -20.0, -2.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], GUTS_NONE),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -2.0, -3.0, -2.0], GUTS_LR2),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -6.0, -10.0, -2.0], GUTS_LR2),
        element(GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -12.0, -20.0, -2.0], GUTS_NONE),
    ],
];

/// The compiled-in row for one gauge of one set.
pub fn element_of(set: GaugeSetId, index: GaugeIndex) -> GaugeElement {
    GAUGE_TABLE[set.index()][index.index()]
}

/// The parameters a gauge of `set` is built from, read from the bundled `data/gauge.ron` with
/// [`GAUGE_TABLE`] as the fallback for a file with no row to answer with. The parity guard
/// `bundled_gauge_data_matches_the_compiled_in_table` asserts the two agree.
pub fn params(set: GaugeSetId, index: GaugeIndex) -> GaugeParams {
    match builtin_gauge_tables().get(set.data_key()) {
        Some(row) => row.at(index).clone(),
        None => element_of(set, index).to_params(),
    }
}

impl GaugeSet {
    /// The parameters of one of the nine gauges, by reference gauge index.
    pub fn at(&self, index: GaugeIndex) -> &GaugeParams {
        match index {
            GaugeIndex::AssistEasy => &self.assist_easy,
            GaugeIndex::Easy => &self.easy,
            GaugeIndex::Normal => &self.normal,
            GaugeIndex::Hard => &self.hard,
            GaugeIndex::ExHard => &self.exhard,
            GaugeIndex::Hazard => &self.hazard,
            GaugeIndex::Class => &self.class,
            GaugeIndex::ExClass => &self.exclass,
            GaugeIndex::ExHardClass => &self.exhardclass,
        }
    }
}

/// TOTAL thresholds of MODIFY_DAMAGE (`GrooveGauge.java:277`). The multiplier used is the one at
/// the first index whose threshold the chart's TOTAL reaches.
const MODIFY_DAMAGE_FIX1_TOTAL: [f64; 10] = [240.0, 230.0, 210.0, 200.0, 180.0, 160.0, 150.0, 130.0, 120.0, 0.0];

/// Damage multipliers of MODIFY_DAMAGE, aligned with [`MODIFY_DAMAGE_FIX1_TOTAL`]
/// (`GrooveGauge.java:278`).
const MODIFY_DAMAGE_FIX1_TABLE: [f32; 10] = [1.0, 1.11, 1.25, 1.5, 1.666, 2.0, 2.5, 3.333, 5.0, 10.0];

/// First note count of the halving walk that builds the note-count multiplier
/// (`GrooveGauge.java:281`).
const MODIFY_DAMAGE_FIRST_NOTE_STEP: i32 = 1000;

/// Weight the halving walk starts at, doubling on every step (`GrooveGauge.java:282`).
const MODIFY_DAMAGE_FIRST_WEIGHT: f32 = 0.002;

/// Reference implementation `GrooveGauge.GaugeModifier.MODIFY_DAMAGE` (`GrooveGauge.java:274-291`):
/// a low chart TOTAL or a short chart makes damage bite harder. Gains pass through untouched.
///
/// The halving walk runs until `note` reaches 1 whatever the chart's note count is, exactly as the
/// reference's `note > totalNotes || note > 1` condition does; ending it early changes the result.
pub fn modify_damage(f: f32, total: f64, notes: usize) -> f32 {
    if f >= 0.0 {
        return f;
    }
    let mut i = 0;
    while i < MODIFY_DAMAGE_FIX1_TOTAL.len() - 1 && total < MODIFY_DAMAGE_FIX1_TOTAL[i] {
        i += 1;
    }
    let n = notes.min(i32::MAX as usize) as i32;
    let mut fix2 = 1.0f32;
    let mut note = MODIFY_DAMAGE_FIRST_NOTE_STEP;
    let mut weight = MODIFY_DAMAGE_FIRST_WEIGHT;
    while note > n || note > 1 {
        fix2 += weight * (note - n.max(note / 2)) as f32;
        note /= 2;
        weight *= 2.0;
    }
    f * MODIFY_DAMAGE_FIX1_TABLE[i].max(fix2)
}

#[cfg(test)]
mod gauge_table_tests {
    use super::*;
    use crate::data::CURRENT_GAUGE_DATA_VERSION;

    type PinnedElement = (GaugeModifier, f32, f32, f32, f32, [f32; 6], &'static [(f32, f32)]);

    /// The reference implementation's `GaugeProperty.GaugeElementProperty` literals
    /// (`GaugeProperty.java:77-125`), transcribed once so a change to [`GAUGE_TABLE`] has to be
    /// deliberate. Rows run FIVEKEYS, SEVENKEYS, PMS, KEYBOARD, LR2; columns run ASSISTEASY, EASY,
    /// NORMAL, HARD, EXHARD, HAZARD, CLASS, EXCLASS, EXHARDCLASS.
    const PINNED: [[PinnedElement; 9]; 5] = [
        [
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 50.0, [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 75.0, [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 75.0, [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], GUTS_NONE),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.0, 0.0, 0.0, -5.0, -10.0, -5.0], GUTS_NONE),
            (GaugeModifier::ModifyDamage, 0.0, 100.0, 100.0, 0.0, [0.0, 0.0, 0.0, -10.0, -20.0, -10.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.0, 0.0, 0.0, -100.0, -100.0, -100.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.01, 0.01, 0.0, -0.5, -1.0, -0.5], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.01, 0.01, 0.0, -1.0, -2.0, -1.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.01, 0.01, 0.0, -2.5, -5.0, -2.5], GUTS_NONE),
        ],
        [
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 60.0, [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], GUTS_NONE),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -5.0, -10.0, -5.0], GUTS_HARD),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -8.0, -16.0, -8.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.06, -1.5, -3.0, -1.5], GUTS_CLASS),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -3.0, -6.0, -3.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -5.0, -10.0, -5.0], GUTS_NONE),
        ],
        [
            (GaugeModifier::Total, 2.0, 120.0, 30.0, 65.0, [1.0, 1.0, 0.5, -1.0, -2.0, -2.0], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 120.0, 30.0, 85.0, [1.0, 1.0, 0.5, -1.0, -3.0, -3.0], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 120.0, 30.0, 85.0, [1.0, 1.0, 0.5, -2.0, -6.0, -6.0], GUTS_NONE),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -5.0, -10.0, -10.0], GUTS_HARD),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -10.0, -15.0, -15.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -100.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.06, -1.5, -3.0, -3.0], GUTS_CLASS),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.12, 0.03, -3.0, -6.0, -6.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -5.0, -10.0, -10.0], GUTS_NONE),
        ],
        [
            (GaugeModifier::Total, 2.0, 100.0, 30.0, 50.0, [1.0, 1.0, 0.5, -1.0, -2.0, -1.0], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 70.0, [1.0, 1.0, 0.5, -1.0, -3.0, -1.0], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 70.0, [1.0, 1.0, 0.5, -2.0, -4.0, -2.0], GUTS_NONE),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.2, 0.2, 0.1, -4.0, -8.0, -4.0], GUTS_HARD),
            (GaugeModifier::LimitIncrement, 0.0, 100.0, 100.0, 0.0, [0.2, 0.1, 0.0, -6.0, -12.0, -6.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.1, 0.0, -100.0, -100.0, -100.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.2, 0.1, -1.5, -3.0, -1.5], GUTS_CLASS),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.2, 0.1, -3.0, -6.0, -3.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.2, 0.1, 0.0, -5.0, -10.0, -5.0], GUTS_NONE),
        ],
        [
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 60.0, [1.2, 1.2, 0.6, -3.2, -4.8, -1.6], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.2, 1.2, 0.6, -3.2, -4.8, -1.6], GUTS_NONE),
            (GaugeModifier::Total, 2.0, 100.0, 20.0, 80.0, [1.0, 1.0, 0.5, -4.0, -6.0, -2.0], GUTS_NONE),
            (GaugeModifier::ModifyDamage, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -6.0, -10.0, -2.0], GUTS_LR2),
            (GaugeModifier::ModifyDamage, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -12.0, -20.0, -2.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], GUTS_NONE),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -2.0, -3.0, -2.0], GUTS_LR2),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -6.0, -10.0, -2.0], GUTS_LR2),
            (GaugeModifier::None, 0.0, 100.0, 100.0, 0.0, [0.1, 0.1, 0.05, -12.0, -20.0, -2.0], GUTS_NONE),
        ],
    ];

    #[test]
    fn gauge_table_pins_all_45_elements() {
        for set in GaugeSetId::ALL {
            for index in GaugeIndex::ALL {
                let got = element_of(set, index);
                let (modifier, min, max, init, border, deltas, guts) = PINNED[set.index()][index.index()];
                assert_eq!(got.modifier, modifier, "{set:?} {index:?} modifier");
                assert_eq!(got.min, min, "{set:?} {index:?} min");
                assert_eq!(got.max, max, "{set:?} {index:?} max");
                assert_eq!(got.init, init, "{set:?} {index:?} init");
                assert_eq!(got.border, border, "{set:?} {index:?} border");
                assert_eq!(got.deltas, deltas, "{set:?} {index:?} deltas");
                assert_eq!(got.guts, guts, "{set:?} {index:?} guts");
            }
        }
    }

    #[test]
    fn guts_tables_pin_their_reference_values() {
        assert_eq!(GUTS_HARD, [(10.0, 0.4), (20.0, 0.5), (30.0, 0.6), (40.0, 0.7), (50.0, 0.8)]);
        assert_eq!(GUTS_CLASS, [(5.0, 0.4), (10.0, 0.5), (15.0, 0.6), (20.0, 0.7), (25.0, 0.8)]);
        assert_eq!(GUTS_LR2, [(30.0, 0.6)]);
        assert!(GUTS_NONE.is_empty());
    }

    #[test]
    fn guts_appear_exactly_where_the_reference_puts_them() {
        let with_guts: Vec<(GaugeSetId, GaugeIndex)> = GaugeSetId::ALL
            .into_iter()
            .flat_map(|s| GaugeIndex::ALL.into_iter().map(move |i| (s, i)))
            .filter(|&(s, i)| !element_of(s, i).guts.is_empty())
            .collect();
        assert_eq!(
            with_guts,
            [
                (GaugeSetId::SevenKeys, GaugeIndex::Hard),
                (GaugeSetId::SevenKeys, GaugeIndex::Class),
                (GaugeSetId::Pms, GaugeIndex::Hard),
                (GaugeSetId::Pms, GaugeIndex::Class),
                (GaugeSetId::Keyboard, GaugeIndex::Hard),
                (GaugeSetId::Keyboard, GaugeIndex::Class),
                (GaugeSetId::Lr2, GaugeIndex::Hard),
                (GaugeSetId::Lr2, GaugeIndex::Class),
                (GaugeSetId::Lr2, GaugeIndex::ExClass),
            ],
            "the five-key set softens nothing"
        );
    }

    #[test]
    fn pms_total_gauges_reach_120_from_30() {
        for index in [GaugeIndex::AssistEasy, GaugeIndex::Easy, GaugeIndex::Normal] {
            let e = element_of(GaugeSetId::Pms, index);
            assert_eq!((e.max, e.init), (120.0, 30.0), "{index:?} is the only set with a 120 ceiling");
        }
    }

    #[test]
    fn keyboard_assist_easy_starts_at_30_while_easy_and_normal_start_at_20() {
        assert_eq!(element_of(GaugeSetId::Keyboard, GaugeIndex::AssistEasy).init, 30.0);
        assert_eq!(element_of(GaugeSetId::Keyboard, GaugeIndex::Easy).init, 20.0, "the reference's own mixed init is kept as written");
        assert_eq!(element_of(GaugeSetId::Keyboard, GaugeIndex::Normal).init, 20.0);
    }

    #[test]
    fn modify_damage_is_used_by_five_key_exhard_and_lr2_survival_gauges() {
        let rows: Vec<(GaugeSetId, GaugeIndex)> = GaugeSetId::ALL
            .into_iter()
            .flat_map(|s| GaugeIndex::ALL.into_iter().map(move |i| (s, i)))
            .filter(|&(s, i)| element_of(s, i).modifier == GaugeModifier::ModifyDamage)
            .collect();
        assert_eq!(rows, [(GaugeSetId::FiveKeys, GaugeIndex::ExHard), (GaugeSetId::Lr2, GaugeIndex::Hard), (GaugeSetId::Lr2, GaugeIndex::ExHard)]);
    }

    #[test]
    fn set_for_mode_routes_like_the_reference_rule_table() {
        assert_eq!(GaugeSetId::for_mode(&Mode::BEAT_5K), GaugeSetId::FiveKeys);
        assert_eq!(GaugeSetId::for_mode(&Mode::BEAT_10K), GaugeSetId::FiveKeys);
        assert_eq!(GaugeSetId::for_mode(&Mode::BEAT_7K), GaugeSetId::SevenKeys);
        assert_eq!(GaugeSetId::for_mode(&Mode::BEAT_14K), GaugeSetId::SevenKeys);
        assert_eq!(GaugeSetId::for_mode(&Mode::POPN_9K), GaugeSetId::Pms);
        let keyboard = Mode { name: crate::data::KEYBOARD_24K_KEY, ..Mode::BEAT_7K };
        assert_eq!(GaugeSetId::for_mode(&keyboard), GaugeSetId::Keyboard);
        let unknown = Mode { name: "NOT_A_MODE", ..Mode::BEAT_7K };
        assert_eq!(GaugeSetId::for_mode(&unknown), GaugeSetId::SevenKeys, "an unknown mode falls back to the seven-key set");
    }

    #[test]
    fn lr2_is_never_reached_from_a_mode() {
        for mode in Mode::ALL {
            assert_ne!(GaugeSetId::for_mode(mode), GaugeSetId::Lr2, "{} must not select LR2", mode.name);
        }
    }

    #[test]
    fn set_indices_are_dense_and_ordered() {
        for (i, set) in GaugeSetId::ALL.into_iter().enumerate() {
            assert_eq!(set.index(), i, "{set:?}");
        }
    }

    #[test]
    fn bundled_gauge_data_matches_the_compiled_in_table() {
        let tables = builtin_gauge_tables();
        assert_eq!(tables.version, CURRENT_GAUGE_DATA_VERSION);
        for set in GaugeSetId::ALL {
            for key in set.data_keys() {
                let row = tables.get(key).unwrap_or_else(|| panic!("{key} row present"));
                for index in GaugeIndex::ALL {
                    assert_eq!(*row.at(index), element_of(set, index).to_params(), "{key} {index:?}");
                }
            }
        }
    }

    #[test]
    fn the_default_set_is_the_one_the_default_data_key_names() {
        assert_eq!(GaugeSetId::default(), GaugeSetId::SevenKeys);
        assert_eq!(GaugeSetId::default().data_key(), crate::data::DEFAULT_GAUGE_KEY, "the fallback row and the default set must be the same table");
    }

    #[test]
    fn bundled_gauge_data_has_exactly_the_expected_keys() {
        let keys: Vec<&str> = builtin_gauge_tables().gauges.keys().map(String::as_str).collect();
        assert_eq!(keys, ["BEAT_10K", "BEAT_14K", "BEAT_5K", "BEAT_7K", crate::data::KEYBOARD_24K_KEY, LR2_GAUGE_KEY, "POPN_9K"]);
    }

    #[test]
    fn params_reads_the_data_file_for_every_set() {
        for set in GaugeSetId::ALL {
            for index in GaugeIndex::ALL {
                assert_eq!(params(set, index), element_of(set, index).to_params(), "{set:?} {index:?}");
            }
        }
    }

    #[test]
    fn modify_damage_leaves_gains_untouched() {
        for gain in [0.0f32, 0.01, 0.15, 1.0] {
            assert_eq!(modify_damage(gain, 60.0, 20), gain, "gain {gain} passes through");
        }
    }

    #[test]
    fn modify_damage_matches_reference_samples() {
        let samples: [(f64, usize, f32); 6] =
            [(240.0, 1000, -1.0), (200.0, 500, -1.5), (130.0, 100, -3.333), (60.0, 20, -10.0), (300.0, 2000, -1.0), (240.0, 1, -10.096)];
        for (total, notes, expected) in samples {
            let got = modify_damage(-1.0, total, notes);
            assert!((got - expected).abs() < 1e-3, "total {total} notes {notes}: got {got}, want {expected}");
        }
    }

    #[test]
    fn modify_damage_note_count_multiplier_wins_on_a_very_short_chart() {
        let long = modify_damage(-1.0, 240.0, 1000);
        let short = modify_damage(-1.0, 240.0, 1);
        assert_eq!(long, -1.0, "a 1000-note chart takes the TOTAL band multiplier of 1.0");
        assert!(short < long, "a one-note chart takes the larger note-count multiplier instead: {short} vs {long}");
    }

    #[test]
    fn modify_damage_picks_the_first_total_band_at_or_above_the_chart_total() {
        assert_eq!(modify_damage(-1.0, 240.0, 1000), -1.0, "TOTAL 240 is the first band, multiplier 1.0");
        assert!((modify_damage(-1.0, 239.0, 1000) - -1.11).abs() < 1e-4, "just under 240 falls to the 1.11 band");
        assert!((modify_damage(-1.0, 0.0, 1000) - -10.0).abs() < 1e-4, "TOTAL 0 lands on the last band");
    }

    #[test]
    fn modify_damage_scales_with_the_multiplier_and_keeps_the_sign() {
        assert!((modify_damage(-2.0, 239.0, 1000) - -2.22).abs() < 1e-4);
        assert!(modify_damage(-2.0, 239.0, 1000) < 0.0, "damage stays damage");
    }
}
