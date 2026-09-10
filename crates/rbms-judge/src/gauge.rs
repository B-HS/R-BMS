use crate::Judge;
use crate::data::{GaugeModifier, GaugeParams};
use crate::gauge_tables::{GaugeSetId, element_of, modify_damage};

/// A gauge a player can select. These are the reference implementation's gauge slots 0..5, the
/// range `PlayerConfig.gauge` is clamped to (`PlayerConfig.java:906`); the three course gauges have
/// no entry here because no player setting reaches them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeKind {
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    Hazard,
}

/// One of the nine gauges a chart is played with, in the reference implementation's index order
/// (`GrooveGauge.java:20-28`). All nine advance on every judgment; the selected one decides the
/// clear. [`GaugeIndex::Class`] and up are the course gauges, which this engine builds and updates
/// for parity but never selects, having no course mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GaugeIndex {
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    Hazard,
    Class,
    ExClass,
    ExHardClass,
}

impl GaugeIndex {
    /// How many gauges a chart is played with.
    pub const COUNT: usize = 9;

    /// Every gauge slot, in reference index order.
    pub const ALL: [GaugeIndex; GaugeIndex::COUNT] = [
        GaugeIndex::AssistEasy,
        GaugeIndex::Easy,
        GaugeIndex::Normal,
        GaugeIndex::Hard,
        GaugeIndex::ExHard,
        GaugeIndex::Hazard,
        GaugeIndex::Class,
        GaugeIndex::ExClass,
        GaugeIndex::ExHardClass,
    ];

    /// This slot's reference index (`GrooveGauge.java:20-28`).
    pub fn index(self) -> usize {
        match self {
            GaugeIndex::AssistEasy => 0,
            GaugeIndex::Easy => 1,
            GaugeIndex::Normal => 2,
            GaugeIndex::Hard => 3,
            GaugeIndex::ExHard => 4,
            GaugeIndex::Hazard => 5,
            GaugeIndex::Class => 6,
            GaugeIndex::ExClass => 7,
            GaugeIndex::ExHardClass => 8,
        }
    }

    /// The slot at a reference index, or `None` past the last gauge.
    pub fn from_index(index: usize) -> Option<GaugeIndex> {
        GaugeIndex::ALL.get(index).copied()
    }

    /// The slot a selectable gauge occupies.
    pub fn from_kind(kind: GaugeKind) -> GaugeIndex {
        match kind {
            GaugeKind::AssistEasy => GaugeIndex::AssistEasy,
            GaugeKind::Easy => GaugeIndex::Easy,
            GaugeKind::Normal => GaugeIndex::Normal,
            GaugeKind::Hard => GaugeIndex::Hard,
            GaugeKind::ExHard => GaugeIndex::ExHard,
            GaugeKind::Hazard => GaugeIndex::Hazard,
        }
    }

    /// The selectable gauge this slot is, or `None` for a course gauge.
    pub fn kind(self) -> Option<GaugeKind> {
        match self {
            GaugeIndex::AssistEasy => Some(GaugeKind::AssistEasy),
            GaugeIndex::Easy => Some(GaugeKind::Easy),
            GaugeIndex::Normal => Some(GaugeKind::Normal),
            GaugeIndex::Hard => Some(GaugeKind::Hard),
            GaugeIndex::ExHard => Some(GaugeKind::ExHard),
            GaugeIndex::Hazard => Some(GaugeKind::Hazard),
            GaugeIndex::Class | GaugeIndex::ExClass | GaugeIndex::ExHardClass => None,
        }
    }

    /// Whether this is one of the three course gauges (`GrooveGauge.java:103-105`).
    pub fn is_course(self) -> bool {
        self >= GaugeIndex::Class
    }
}

/// The clear lamp a play earns. Ids are the reference implementation's (`ClearType.java:10-20`);
/// [`ClearType::LightAssistEasy`] is the lamp of an ASSIST EASY gauge clear, while
/// [`ClearType::AssistEasy`] is the lamp a play is demoted to when it used assist options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    NoPlay,
    Failed,
    AssistEasy,
    LightAssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    FullCombo,
    Perfect,
    Max,
}

impl ClearType {
    /// Whether this lamp is a clear at all, which every lamp above [`ClearType::Failed`] is.
    ///
    /// A run that was never played and one that ran out of gauge are the only two that are not, so
    /// the question is answered once here rather than restated wherever it is asked.
    pub fn is_cleared(self) -> bool {
        !matches!(self, ClearType::NoPlay | ClearType::Failed)
    }
}

/// How much assist a play ran with (`BMSPlayer.java:865-867`). Any assist skips the full-combo
/// lamps and demotes the clear lamp; which lamp it demotes to depends on the level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssistLevel {
    #[default]
    None,
    /// A single light assist, e.g. AUTO SCRATCH.
    Light,
    /// Anything stronger, e.g. a widened judge window or long-note margin.
    Full,
}

/// The lamp a clear on `index` awards (`ClearType.java:10-20`, the `gaugetype` column). The three
/// course gauges share the lamps of their non-course counterparts, and a HAZARD clear is a full
/// combo by construction because a single BAD or POOR ends the play.
pub fn lamp_for_gauge(index: GaugeIndex) -> ClearType {
    match index {
        GaugeIndex::AssistEasy => ClearType::LightAssistEasy,
        GaugeIndex::Easy => ClearType::Easy,
        GaugeIndex::Normal | GaugeIndex::Class => ClearType::Normal,
        GaugeIndex::Hard | GaugeIndex::ExClass => ClearType::Hard,
        GaugeIndex::ExHard | GaugeIndex::ExHardClass => ClearType::ExHard,
        GaugeIndex::Hazard => ClearType::FullCombo,
    }
}

/// Demote a lamp for a play that used assist options (`BMSPlayer.java:865-867`): one light assist
/// leaves [`ClearType::LightAssistEasy`], anything stronger [`ClearType::AssistEasy`], and the
/// full-combo lamps are unreachable either way. A play that did not clear keeps its lamp.
pub fn assist_downgrade(lamp: ClearType, assist: AssistLevel) -> ClearType {
    if matches!(lamp, ClearType::NoPlay | ClearType::Failed) {
        return lamp;
    }
    match assist {
        AssistLevel::None => lamp,
        AssistLevel::Light => ClearType::LightAssistEasy,
        AssistLevel::Full => ClearType::AssistEasy,
    }
}

/// The parameters a gauge is built from, read from the bundled `data/gauge.ron`.
///
/// The data file is the source the engine builds gauges from; [`default_params`] is the
/// compiled-in fallback for a file with no row to answer with, and the parity guard in
/// [`crate::gauge_tables`] asserts the two agree field for field.
pub fn params(kind: GaugeKind) -> GaugeParams {
    crate::gauge_tables::params(GaugeSetId::default(), GaugeIndex::from_kind(kind))
}

/// The program-default parameters for `kind`: the reference implementation's
/// `GaugeProperty.SEVENKEYS` row, which is what a chart with no set of its own plays on.
pub fn default_params(kind: GaugeKind) -> GaugeParams {
    element_of(GaugeSetId::default(), GaugeIndex::from_kind(kind)).to_params()
}

/// One groove gauge (reference implementation `GrooveGauge.Gauge`). Deltas are pre-modified at
/// construction by the gauge's modifier (TOTAL scales gains by chart total/notes; LIMIT_INCREMENT
/// caps the gain; MODIFY_DAMAGE deepens damage on a low-total or short chart). A note's judge adds
/// `deltas[judge]`, with "guts" softening damage at low values. Cleared when `value >= border` and
/// `> 0` at the end.
#[derive(Debug, Clone)]
pub struct Gauge {
    index: GaugeIndex,
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
        Self::at_index(GaugeIndex::from_kind(kind), params, total, notes)
    }

    /// Build any of the nine gauge slots, including the course gauges no [`GaugeKind`] names.
    pub fn at_index(index: GaugeIndex, params: &GaugeParams, total: f64, notes: usize) -> Self {
        let notes = notes.max(1);
        let divisor = notes as f64;
        let total = if total > 0.0 { total } else { rbms_model::default_total(notes) };
        let mut deltas = params.deltas;
        match params.modifier {
            GaugeModifier::Total => {
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d = (*d as f64 * total / divisor) as f32;
                    }
                }
            }
            GaugeModifier::LimitIncrement => {
                let pg = ((2.0 * total - LIMIT_INCREMENT_TOTAL_OFFSET) / divisor).clamp(0.0, LIMIT_INCREMENT_PGREAT_CAP) as f32;
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d *= pg / LIMIT_INCREMENT_PGREAT_CAP as f32;
                    }
                }
            }
            GaugeModifier::ModifyDamage => {
                for d in deltas.iter_mut() {
                    *d = modify_damage(*d, total, notes);
                }
            }
            GaugeModifier::None => {}
        }
        Gauge { index, value: params.init, min: params.min, max: params.max, border: params.border, deltas, guts: params.guts.clone() }
    }

    pub fn update(&mut self, judge: Judge) {
        self.update_with_rate(judge, 1.0);
    }

    /// Apply a judgment scaled by `rate` (reference implementation `GrooveGauge.Gauge.update`,
    /// `GrooveGauge.java:229-240`). A hell-charge tick feeds a fractional rate; a normal judgment
    /// feeds 1.0. Guts softening is decided from the value before the change, as in the reference.
    pub fn update_with_rate(&mut self, judge: Judge, rate: f32) {
        if self.value <= 0.0 {
            return;
        }
        let mut inc = self.deltas[judge as usize] * rate;
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

    /// Which of the nine slots this gauge is.
    pub fn index(&self) -> GaugeIndex {
        self.index
    }

    /// The selectable gauge this is, or `None` for a course gauge.
    pub fn kind(&self) -> Option<GaugeKind> {
        self.index.kind()
    }

    /// Reference implementation `Gauge.isQualified` (`GrooveGauge.java:246-248`).
    pub fn is_cleared(&self) -> bool {
        self.value > 0.0 && self.value >= self.border
    }

    /// Reference implementation `Gauge.isMax` (`GrooveGauge.java:250-252`).
    pub fn is_max(&self) -> bool {
        self.value == self.max
    }

    /// The lamp a clear on this gauge awards.
    pub fn lamp(&self) -> ClearType {
        lamp_for_gauge(self.index)
    }
}

/// How the selected gauge may move during play (`PlayerConfig.java:163-167`). Ids 0..4 are the
/// reference's, so a stored setting round-trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GaugeAutoShift {
    /// The play fails the moment the selected gauge empties.
    #[default]
    None,
    /// The play continues on an empty gauge.
    Continue,
    /// An emptied survival gauge drops to NORMAL and the play continues.
    SurvivalToGroove,
    /// Every frame, take the strongest gauge still clearing.
    BestClear,
    /// Every frame, take the strongest gauge still clearing at or below the selected one.
    SelectToUnder,
}

impl GaugeAutoShift {
    /// Every mode, in reference id order.
    pub const ALL: [GaugeAutoShift; 5] =
        [GaugeAutoShift::None, GaugeAutoShift::Continue, GaugeAutoShift::SurvivalToGroove, GaugeAutoShift::BestClear, GaugeAutoShift::SelectToUnder];

    /// This mode's reference id.
    pub fn id(self) -> u8 {
        match self {
            GaugeAutoShift::None => 0,
            GaugeAutoShift::Continue => 1,
            GaugeAutoShift::SurvivalToGroove => 2,
            GaugeAutoShift::BestClear => 3,
            GaugeAutoShift::SelectToUnder => 4,
        }
    }

    /// The mode with a reference id, or `None` for an id the reference does not define. The
    /// reference clamps instead (`PlayerConfig.java:907`); callers that must accept any stored
    /// number should fall back to [`GaugeAutoShift::default`].
    pub fn from_id(id: u8) -> Option<GaugeAutoShift> {
        GaugeAutoShift::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// The gauges the GAS floor may be set to (`PlayerConfig.java:908` clamps it to ASSIST EASY..NORMAL).
pub const BOTTOM_SHIFTABLE_GAUGES: [GaugeKind; 3] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal];

/// Clamp a GAS floor into the range the reference allows.
pub fn clamp_bottom_shiftable(kind: GaugeKind) -> GaugeKind {
    if BOTTOM_SHIFTABLE_GAUGES.contains(&kind) { kind } else { GaugeKind::Normal }
}

/// What a frame's gauge auto-shift decided about the play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeShiftOutcome {
    /// Play on.
    Continue,
    /// The selected gauge is empty and this shift mode does not rescue it (`BMSPlayer.java:653-661`).
    Failed,
}

/// The nine gauges a chart is played with (reference implementation `GrooveGauge`,
/// `GrooveGauge.java:18-121`). Every judgment advances all nine; the selected slot is the one the
/// display shows and the one that decides the clear, and gauge auto-shift may move it during play.
#[derive(Debug, Clone)]
pub struct GrooveGauge {
    set: GaugeSetId,
    original: GaugeIndex,
    selected: GaugeIndex,
    gauges: [Gauge; GaugeIndex::COUNT],
}

impl GrooveGauge {
    /// Build all nine gauges of `set` for a chart of `total` and `notes`, displaying `kind`.
    pub fn new(set: GaugeSetId, kind: GaugeKind, total: f64, notes: usize) -> Self {
        Self::at_index(set, GaugeIndex::from_kind(kind), total, notes)
    }

    /// Build all nine gauges of `set`, displaying any slot including a course gauge.
    pub fn at_index(set: GaugeSetId, selected: GaugeIndex, total: f64, notes: usize) -> Self {
        let gauges = std::array::from_fn(|i| {
            let index = GaugeIndex::ALL[i];
            Gauge::at_index(index, &crate::gauge_tables::params(set, index), total, notes)
        });
        GrooveGauge { set, original: selected, selected, gauges }
    }

    /// Which table these gauges come from.
    pub fn set(&self) -> GaugeSetId {
        self.set
    }

    /// Apply a judgment to all nine gauges (`GrooveGauge.java:48-50`).
    pub fn update(&mut self, judge: Judge) {
        self.update_with_rate(judge, 1.0);
    }

    /// Apply a judgment scaled by `rate` to all nine gauges (`GrooveGauge.java:57-61`).
    pub fn update_with_rate(&mut self, judge: Judge, rate: f32) {
        for gauge in self.gauges.iter_mut() {
            gauge.update_with_rate(judge, rate);
        }
    }

    /// Add `delta` to all nine gauges — mine damage hits every gauge (`GrooveGauge.java:63-67`).
    pub fn add_value(&mut self, delta: f32) {
        for gauge in self.gauges.iter_mut() {
            gauge.add_value(delta);
        }
    }

    /// Value of the selected gauge.
    pub fn value(&self) -> f32 {
        self.selected().value()
    }

    /// Value of one gauge, whether or not it is selected.
    pub fn value_at(&self, index: GaugeIndex) -> f32 {
        self.gauge_at(index).value()
    }

    /// The selected gauge.
    pub fn selected(&self) -> &Gauge {
        self.gauge_at(self.selected)
    }

    /// One of the nine gauges.
    pub fn gauge_at(&self, index: GaugeIndex) -> &Gauge {
        &self.gauges[index.index()]
    }

    /// Which slot is selected.
    pub fn selected_index(&self) -> GaugeIndex {
        self.selected
    }

    /// Select another slot (`GrooveGauge.java:95-97`).
    pub fn select(&mut self, index: GaugeIndex) {
        self.selected = index;
    }

    /// Whether the selected gauge clears (`GrooveGauge.java:87-89`).
    pub fn is_cleared(&self) -> bool {
        self.selected().is_cleared()
    }

    /// Whether auto-shift moved the selection away from the gauge the player chose
    /// (`GrooveGauge.java:99-101`). The reference stores `-1` instead of a gauge id in that case.
    pub fn is_type_changed(&self) -> bool {
        self.original != self.selected
    }

    /// Whether a course gauge is selected (`GrooveGauge.java:103-105`).
    pub fn is_course_gauge(&self) -> bool {
        self.selected.is_course()
    }

    /// The lamp the selected gauge awards (`GrooveGauge.java:111-113`).
    pub fn lamp(&self) -> ClearType {
        self.selected().lamp()
    }

    /// Run one frame of gauge auto-shift (`BMSPlayer.java:638-672`).
    ///
    /// `configured` is the gauge the player chose and `bottom` the floor the selection may fall to;
    /// both only matter to the two modes that re-pick every frame. The other three act only once
    /// the selected gauge is empty, and only [`GaugeAutoShift::None`] ends the play there.
    pub fn auto_shift(&mut self, mode: GaugeAutoShift, configured: GaugeKind, bottom: GaugeKind) -> GaugeShiftOutcome {
        if matches!(mode, GaugeAutoShift::BestClear | GaugeAutoShift::SelectToUnder) {
            self.shift_to_best(mode, configured, bottom);
            return GaugeShiftOutcome::Continue;
        }
        if self.value() != 0.0 {
            return GaugeShiftOutcome::Continue;
        }
        match mode {
            GaugeAutoShift::None => GaugeShiftOutcome::Failed,
            GaugeAutoShift::SurvivalToGroove => {
                if !self.is_course_gauge() {
                    self.selected = GaugeIndex::Normal;
                }
                GaugeShiftOutcome::Continue
            }
            _ => GaugeShiftOutcome::Continue,
        }
    }

    /// The per-frame re-pick of `BMSPlayer.java:639-650`: scan from the floor up to the ceiling the
    /// mode allows and keep the highest gauge that is still clearing.
    fn shift_to_best(&mut self, mode: GaugeAutoShift, configured: GaugeKind, bottom: GaugeKind) {
        let course = self.is_course_gauge();
        let configured = GaugeIndex::from_kind(configured).index();
        let len = match mode {
            GaugeAutoShift::BestClear => {
                if course {
                    GaugeIndex::ExHardClass.index() + 1
                } else {
                    GaugeIndex::Hazard.index() + 1
                }
            }
            _ => {
                if course {
                    let course_offset = GaugeIndex::Class.index() - GaugeIndex::Normal.index();
                    (configured.max(GaugeIndex::Normal.index()) + course_offset).min(GaugeIndex::ExHardClass.index()) + 1
                } else {
                    configured + 1
                }
            }
        };
        let floor = if course { GaugeIndex::Class.index() } else { self.selected.index().min(GaugeIndex::from_kind(clamp_bottom_shiftable(bottom)).index()) };
        let mut chosen = floor;
        for i in floor..len {
            if self.gauges[i].value() > 0.0 && self.gauges[i].is_cleared() {
                chosen = i;
            }
        }
        if let Some(index) = GaugeIndex::from_index(chosen) {
            self.selected = index;
        }
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
    gauge.lamp()
}

/// Reference implementation `ClearType` id (`ClearType.java:10-20`) for a lamp — persisted in score
/// records and sent to the IR so the lamp round-trips.
pub fn clear_type_id(c: ClearType) -> u8 {
    match c {
        ClearType::NoPlay => 0,
        ClearType::Failed => 1,
        ClearType::AssistEasy => 2,
        ClearType::LightAssistEasy => 3,
        ClearType::Easy => 4,
        ClearType::Normal => 5,
        ClearType::Hard => 6,
        ClearType::ExHard => 7,
        ClearType::FullCombo => 8,
        ClearType::Perfect => 9,
        ClearType::Max => 10,
    }
}

/// Inverse of [`clear_type_id`]; anything unknown reads back as [`ClearType::NoPlay`]
/// (`ClearType.java:42-49`).
pub fn clear_type_from_id(id: u8) -> ClearType {
    match id {
        1 => ClearType::Failed,
        2 => ClearType::AssistEasy,
        3 => ClearType::LightAssistEasy,
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

    /// Only a run that was never played and a run that ran out of gauge fall short of a clear;
    /// every lamp above them counts, which is what a result screen and a skin both ask.
    #[test]
    fn every_lamp_above_failed_counts_as_a_clear() {
        assert!(!ClearType::NoPlay.is_cleared());
        assert!(!ClearType::Failed.is_cleared());
        for c in ALL_CLEARS.into_iter().filter(|c| !matches!(c, ClearType::NoPlay | ClearType::Failed)) {
            assert!(c.is_cleared(), "{c:?} is a clear lamp but did not count as one");
        }
    }

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
        assert_eq!(clear_type_id(ClearType::LightAssistEasy), 3);
        assert_eq!(clear_type_id(ClearType::Easy), 4);
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
    fn clear_type_from_id_unknown_ids_are_no_play() {
        for id in [11u8, 12, 200, u8::MAX] {
            assert_eq!(clear_type_from_id(id), ClearType::NoPlay, "unknown id {id} => NoPlay");
        }
    }
}

#[cfg(test)]
mod gauge_index_tests {
    use super::*;

    const ALL_KINDS: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];

    #[test]
    fn indices_are_dense_and_in_reference_order() {
        for (i, index) in GaugeIndex::ALL.into_iter().enumerate() {
            assert_eq!(index.index(), i, "{index:?}");
            assert_eq!(GaugeIndex::from_index(i), Some(index));
        }
        assert_eq!(GaugeIndex::from_index(GaugeIndex::COUNT), None);
    }

    #[test]
    fn selectable_gauges_occupy_the_first_six_slots() {
        for (i, kind) in ALL_KINDS.into_iter().enumerate() {
            let index = GaugeIndex::from_kind(kind);
            assert_eq!(index.index(), i, "{kind:?}");
            assert_eq!(index.kind(), Some(kind));
            assert!(!index.is_course(), "{kind:?} is not a course gauge");
        }
    }

    #[test]
    fn course_gauges_have_no_selectable_kind() {
        for index in [GaugeIndex::Class, GaugeIndex::ExClass, GaugeIndex::ExHardClass] {
            assert_eq!(index.kind(), None, "{index:?}");
            assert!(index.is_course(), "{index:?}");
        }
    }

    #[test]
    fn lamp_mapping_matches_the_reference_table() {
        let pairs = [
            (GaugeIndex::AssistEasy, ClearType::LightAssistEasy),
            (GaugeIndex::Easy, ClearType::Easy),
            (GaugeIndex::Normal, ClearType::Normal),
            (GaugeIndex::Hard, ClearType::Hard),
            (GaugeIndex::ExHard, ClearType::ExHard),
            (GaugeIndex::Hazard, ClearType::FullCombo),
            (GaugeIndex::Class, ClearType::Normal),
            (GaugeIndex::ExClass, ClearType::Hard),
            (GaugeIndex::ExHardClass, ClearType::ExHard),
        ];
        for (index, lamp) in pairs {
            assert_eq!(lamp_for_gauge(index), lamp, "{index:?}");
        }
    }

    #[test]
    fn assist_easy_gauge_maps_to_light_assist_easy() {
        assert_eq!(lamp_for_gauge(GaugeIndex::AssistEasy), ClearType::LightAssistEasy, "ClearType.java:13 lists gauge 0 under LightAssistEasy");
    }

    #[test]
    fn hazard_clear_maps_to_full_combo() {
        assert_eq!(lamp_for_gauge(GaugeIndex::Hazard), ClearType::FullCombo, "ClearType.java:18 lists gauge 5 under FullCombo");
    }

    #[test]
    fn assist_downgrade_replaces_every_cleared_lamp() {
        for lamp in [ClearType::Easy, ClearType::Normal, ClearType::Hard, ClearType::ExHard, ClearType::FullCombo, ClearType::Perfect, ClearType::Max] {
            assert_eq!(assist_downgrade(lamp, AssistLevel::Light), ClearType::LightAssistEasy, "{lamp:?} with one light assist");
            assert_eq!(assist_downgrade(lamp, AssistLevel::Full), ClearType::AssistEasy, "{lamp:?} with a full assist");
            assert_eq!(assist_downgrade(lamp, AssistLevel::None), lamp, "{lamp:?} without assist");
        }
    }

    #[test]
    fn assist_downgrade_leaves_a_play_that_did_not_clear_alone() {
        for lamp in [ClearType::NoPlay, ClearType::Failed] {
            for assist in [AssistLevel::None, AssistLevel::Light, AssistLevel::Full] {
                assert_eq!(assist_downgrade(lamp, assist), lamp, "{lamp:?} {assist:?}");
            }
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
            assert_eq!(Gauge::new(k, 200.0, 10).kind(), Some(k));
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
        assert!(g.is_max(), "a gauge sitting at its ceiling reports max");
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

    #[test]
    fn update_rate_scales_the_delta() {
        let mut full = Gauge::new(GaugeKind::Normal, 200.0, 4);
        let mut half = Gauge::new(GaugeKind::Normal, 200.0, 4);
        full.update_with_rate(Judge::Bad, 1.0);
        half.update_with_rate(Judge::Bad, 0.5);
        assert_eq!(20.0 - full.value(), 3.0, "a full BAD is -3.0");
        assert_eq!(20.0 - half.value(), 1.5, "a half-rate BAD is -1.5");
    }

    #[test]
    fn update_rate_scales_gains_too() {
        let mut g = Gauge::new(GaugeKind::Normal, 200.0, 4);
        g.update_with_rate(Judge::PerfectGreat, 0.5);
        assert_eq!(g.value() - 20.0, 25.0, "half of the 50.0 gain");
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
            (GaugeKind::AssistEasy, ClearType::LightAssistEasy),
            (GaugeKind::Easy, ClearType::Easy),
            (GaugeKind::Normal, ClearType::Normal),
            (GaugeKind::Hard, ClearType::Hard),
            (GaugeKind::ExHard, ClearType::ExHard),
            (GaugeKind::Hazard, ClearType::FullCombo),
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

#[cfg(test)]
mod modify_damage_tests {
    use super::*;
    use crate::gauge_tables::params as set_params;

    #[test]
    fn five_key_exhard_takes_deepened_damage_on_a_low_total_chart() {
        let index = GaugeIndex::ExHard;
        let params = set_params(GaugeSetId::FiveKeys, index);
        let mut low = Gauge::at_index(index, &params, 130.0, 100);
        let mut high = Gauge::at_index(index, &params, 300.0, 100);
        low.update(Judge::Bad);
        high.update(Judge::Bad);
        assert!((100.0 - high.value() - 10.0).abs() < 1e-3, "a rich chart takes the written -10.0, got {}", 100.0 - high.value());
        assert!((100.0 - low.value() - 33.33).abs() < 1e-2, "TOTAL 130 multiplies damage by 3.333, got {}", 100.0 - low.value());
    }

    #[test]
    fn modify_damage_is_applied_once_at_construction_not_per_judgment() {
        let index = GaugeIndex::ExHard;
        let params = set_params(GaugeSetId::FiveKeys, index);
        let mut g = Gauge::at_index(index, &params, 130.0, 100);
        let start = g.value();
        g.update(Judge::Bad);
        let first = start - g.value();
        let before_second = g.value();
        g.update(Judge::Bad);
        let second = before_second - g.value();
        assert!((first - second).abs() < 1e-3, "the same judge costs the same every time: {first} then {second}");
    }

    #[test]
    fn modify_damage_leaves_gains_alone() {
        let index = GaugeIndex::Hard;
        let params = set_params(GaugeSetId::Lr2, index);
        let mut g = Gauge::at_index(index, &params, 130.0, 100);
        g.update(Judge::Bad);
        let before = g.value();
        g.update(Judge::PerfectGreat);
        assert!((g.value() - before - 0.1).abs() < 1e-4, "the LR2 HARD PG gain of 0.1 is untouched, got {}", g.value() - before);
    }
}

#[cfg(test)]
mod groove_gauge_tests {
    use super::*;

    fn seven_key(kind: GaugeKind) -> GrooveGauge {
        GrooveGauge::new(GaugeSetId::SevenKeys, kind, 200.0, 100)
    }

    #[test]
    fn all_nine_gauges_are_built_with_their_own_initial_values() {
        let g = seven_key(GaugeKind::Normal);
        let values: Vec<f32> = GaugeIndex::ALL.into_iter().map(|i| g.value_at(i)).collect();
        assert_eq!(values, [20.0, 20.0, 20.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0]);
    }

    #[test]
    fn nine_gauges_advance_together() {
        let mut g = seven_key(GaugeKind::Normal);
        let before: Vec<f32> = GaugeIndex::ALL.into_iter().map(|i| g.value_at(i)).collect();
        g.update(Judge::Bad);
        let after: Vec<f32> = GaugeIndex::ALL.into_iter().map(|i| g.value_at(i)).collect();
        let deltas: Vec<f32> = before.iter().zip(&after).map(|(b, a)| a - b).collect();
        assert_eq!(deltas, [-1.5, -1.5, -3.0, -5.0, -8.0, -100.0, -1.5, -3.0, -5.0], "every gauge takes its own BAD damage");
    }

    #[test]
    fn a_judgment_moves_gauges_the_player_did_not_select() {
        let mut g = seven_key(GaugeKind::Normal);
        g.update(Judge::PerfectGreat);
        assert!(g.value_at(GaugeIndex::Easy) > 20.0, "the unselected EASY gauge advanced too");
        assert_eq!(g.selected_index(), GaugeIndex::Normal, "the selection did not move");
    }

    #[test]
    fn mine_damage_hits_every_gauge() {
        let mut g = seven_key(GaugeKind::Normal);
        g.add_value(-5.0);
        for index in GaugeIndex::ALL {
            let expected = if index.index() < GaugeIndex::Hard.index() { 15.0 } else { 95.0 };
            assert_eq!(g.value_at(index), expected, "{index:?}");
        }
    }

    #[test]
    fn value_and_clear_follow_the_selection() {
        let mut g = seven_key(GaugeKind::Normal);
        assert_eq!(g.value(), 20.0);
        assert!(!g.is_cleared(), "NORMAL starts below its border of 80");
        g.select(GaugeIndex::Hard);
        assert_eq!(g.value(), 100.0);
        assert!(g.is_cleared(), "HARD starts at 100 with a border of 0");
        assert_eq!(g.lamp(), ClearType::Hard);
    }

    #[test]
    fn selecting_another_gauge_marks_the_selection_changed() {
        let mut g = seven_key(GaugeKind::Normal);
        assert!(!g.is_type_changed());
        g.select(GaugeIndex::Easy);
        assert!(g.is_type_changed(), "the score records -1 for a shifted gauge");
        g.select(GaugeIndex::Normal);
        assert!(!g.is_type_changed(), "shifting back is not a change");
    }

    #[test]
    fn course_gauges_are_only_reported_when_selected() {
        let mut g = seven_key(GaugeKind::Normal);
        assert!(!g.is_course_gauge());
        g.select(GaugeIndex::ExClass);
        assert!(g.is_course_gauge());
        assert_eq!(g.lamp(), ClearType::Hard);
    }

    #[test]
    fn each_set_builds_its_own_gauges() {
        let pms = GrooveGauge::new(GaugeSetId::Pms, GaugeKind::Normal, 200.0, 100);
        assert_eq!(pms.value_at(GaugeIndex::Normal), 30.0, "PMS gauges start at 30");
        let keyboard = GrooveGauge::new(GaugeSetId::Keyboard, GaugeKind::Normal, 200.0, 100);
        assert_eq!(keyboard.value_at(GaugeIndex::AssistEasy), 30.0);
        assert_eq!(keyboard.value_at(GaugeIndex::Easy), 20.0);
        assert_eq!(keyboard.set(), GaugeSetId::Keyboard);
    }

    #[test]
    fn a_fractional_rate_softens_the_damage_of_every_gauge() {
        let start = seven_key(GaugeKind::Normal);
        let mut full = seven_key(GaugeKind::Normal);
        let mut half = seven_key(GaugeKind::Normal);
        full.update_with_rate(Judge::Bad, 1.0);
        half.update_with_rate(Judge::Bad, 0.5);
        for index in GaugeIndex::ALL {
            let full_loss = start.value_at(index) - full.value_at(index);
            let half_loss = start.value_at(index) - half.value_at(index);
            assert!(half_loss < full_loss, "{index:?} loses {half_loss} at half rate but {full_loss} at full rate");
        }
        assert_eq!(20.0 - half.value_at(GaugeIndex::Normal), 1.5, "half of the NORMAL BAD damage of 3.0");
    }
}

#[cfg(test)]
mod auto_shift_tests {
    use super::*;

    fn gauge_at(kind: GaugeKind) -> GrooveGauge {
        GrooveGauge::new(GaugeSetId::SevenKeys, kind, 200.0, 100)
    }

    fn drain(g: &mut GrooveGauge, index: GaugeIndex) {
        let mut guard = 0;
        while g.value_at(index) > 0.0 && guard < 10_000 {
            g.update(Judge::Poor);
            guard += 1;
        }
    }

    #[test]
    fn gas_ids_are_the_reference_values() {
        for (i, mode) in GaugeAutoShift::ALL.into_iter().enumerate() {
            assert_eq!(mode.id(), i as u8, "{mode:?}");
            assert_eq!(GaugeAutoShift::from_id(i as u8), Some(mode));
        }
        assert_eq!(GaugeAutoShift::from_id(5), None, "the reference clamps ids to 0..4");
        assert_eq!(GaugeAutoShift::default(), GaugeAutoShift::None);
    }

    #[test]
    fn gas_none_fails_once_the_selected_gauge_empties() {
        let mut g = gauge_at(GaugeKind::Hard);
        assert_eq!(g.auto_shift(GaugeAutoShift::None, GaugeKind::Hard, GaugeKind::AssistEasy), GaugeShiftOutcome::Continue);
        drain(&mut g, GaugeIndex::Hard);
        assert_eq!(g.auto_shift(GaugeAutoShift::None, GaugeKind::Hard, GaugeKind::AssistEasy), GaugeShiftOutcome::Failed);
        assert_eq!(g.selected_index(), GaugeIndex::Hard, "a failing play does not shift");
    }

    #[test]
    fn gas_continue_keeps_playing_on_an_empty_gauge() {
        let mut g = gauge_at(GaugeKind::Hard);
        drain(&mut g, GaugeIndex::Hard);
        assert_eq!(g.auto_shift(GaugeAutoShift::Continue, GaugeKind::Hard, GaugeKind::AssistEasy), GaugeShiftOutcome::Continue);
        assert_eq!(g.selected_index(), GaugeIndex::Hard, "CONTINUE does not move the selection");
    }

    #[test]
    fn gas_survival_to_groove_only_on_zero() {
        let mut g = gauge_at(GaugeKind::Hard);
        assert_eq!(g.auto_shift(GaugeAutoShift::SurvivalToGroove, GaugeKind::Hard, GaugeKind::AssistEasy), GaugeShiftOutcome::Continue);
        assert_eq!(g.selected_index(), GaugeIndex::Hard, "a live gauge is left alone");
        drain(&mut g, GaugeIndex::Hard);
        g.auto_shift(GaugeAutoShift::SurvivalToGroove, GaugeKind::Hard, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::Normal, "an emptied survival gauge drops to NORMAL");
    }

    #[test]
    fn gas_survival_to_groove_leaves_a_course_gauge_alone() {
        let mut g = gauge_at(GaugeKind::Hard);
        g.select(GaugeIndex::ExClass);
        drain(&mut g, GaugeIndex::ExClass);
        g.auto_shift(GaugeAutoShift::SurvivalToGroove, GaugeKind::Hard, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::ExClass, "course gauges are not rerouted to NORMAL");
    }

    #[test]
    fn gas_best_clear_picks_highest_qualified() {
        let mut g = gauge_at(GaugeKind::AssistEasy);
        g.auto_shift(GaugeAutoShift::BestClear, GaugeKind::AssistEasy, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::Hazard, "every survival gauge is still full, so the strongest wins");
        drain(&mut g, GaugeIndex::Hazard);
        g.auto_shift(GaugeAutoShift::BestClear, GaugeKind::AssistEasy, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::ExHard, "the emptied HAZARD gauge drops out of the running");
    }

    #[test]
    fn gas_best_clear_never_leaves_the_non_course_gauges() {
        let mut g = gauge_at(GaugeKind::Normal);
        g.auto_shift(GaugeAutoShift::BestClear, GaugeKind::Normal, GaugeKind::Normal);
        assert!(!g.selected_index().is_course(), "a non-course play stops at HAZARD");
    }

    #[test]
    fn gas_select_to_under_never_climbs_above_the_selected_gauge() {
        let mut g = gauge_at(GaugeKind::Normal);
        g.auto_shift(GaugeAutoShift::SelectToUnder, GaugeKind::Normal, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::AssistEasy, "only ASSIST EASY, EASY and NORMAL are in range and none clears yet");
        for _ in 0..200 {
            g.update(Judge::PerfectGreat);
        }
        g.auto_shift(GaugeAutoShift::SelectToUnder, GaugeKind::Normal, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::Normal, "with every gauge over its border the configured one wins");
    }

    #[test]
    fn gas_select_to_under_respects_bottom_shiftable() {
        let mut g = gauge_at(GaugeKind::Normal);
        g.auto_shift(GaugeAutoShift::SelectToUnder, GaugeKind::Normal, GaugeKind::Normal);
        assert_eq!(g.selected_index(), GaugeIndex::Normal, "the floor keeps the selection at NORMAL");
        let mut assist_floor = gauge_at(GaugeKind::Normal);
        assist_floor.auto_shift(GaugeAutoShift::SelectToUnder, GaugeKind::Normal, GaugeKind::AssistEasy);
        assert_eq!(assist_floor.selected_index(), GaugeIndex::AssistEasy, "a lower floor lets the selection fall further");
    }

    #[test]
    fn gas_floor_is_clamped_to_the_three_allowed_gauges() {
        assert_eq!(clamp_bottom_shiftable(GaugeKind::AssistEasy), GaugeKind::AssistEasy);
        assert_eq!(clamp_bottom_shiftable(GaugeKind::Easy), GaugeKind::Easy);
        assert_eq!(clamp_bottom_shiftable(GaugeKind::Normal), GaugeKind::Normal);
        assert_eq!(clamp_bottom_shiftable(GaugeKind::Hard), GaugeKind::Normal, "HARD is not a legal floor");
        assert_eq!(clamp_bottom_shiftable(GaugeKind::Hazard), GaugeKind::Normal);
        let mut g = gauge_at(GaugeKind::ExHard);
        g.auto_shift(GaugeAutoShift::SelectToUnder, GaugeKind::ExHard, GaugeKind::Hazard);
        assert_eq!(g.selected_index(), GaugeIndex::ExHard, "an illegal floor is clamped to NORMAL, which is below the selection");
    }

    #[test]
    fn gas_best_clear_starts_from_the_floor_not_the_current_selection() {
        let mut g = gauge_at(GaugeKind::ExHard);
        drain(&mut g, GaugeIndex::ExHard);
        g.auto_shift(GaugeAutoShift::BestClear, GaugeKind::ExHard, GaugeKind::AssistEasy);
        assert_eq!(g.selected_index(), GaugeIndex::Hard, "with EXHARD and HAZARD gone the strongest survivor is HARD");
    }

    #[test]
    fn gas_re_pick_runs_every_frame_not_only_at_zero() {
        let mut g = gauge_at(GaugeKind::AssistEasy);
        assert!(g.value() > 0.0, "the selected gauge is alive");
        g.auto_shift(GaugeAutoShift::BestClear, GaugeKind::AssistEasy, GaugeKind::AssistEasy);
        assert_ne!(g.selected_index(), GaugeIndex::AssistEasy, "BESTCLEAR shifts while the gauge is still alive");
    }
}
