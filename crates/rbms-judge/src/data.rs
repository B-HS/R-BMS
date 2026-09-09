use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rbms_model::Mode;
use serde::{Deserialize, Serialize};

use crate::gauge::GaugeKind;
use crate::windows::{JudgeProperty, JudgeWindows, MissCondition};

/// Schema version of the bundled `data/judge.ron`.
pub const CURRENT_JUDGE_DATA_VERSION: u32 = 1;

/// Schema version of the bundled `data/gauge.ron`.
pub const CURRENT_GAUGE_DATA_VERSION: u32 = 1;

/// Table key for the reference implementation's KEYBOARD (24K) row. That mode is not wired into
/// [`rbms_model::Mode`] yet, so no chart selects it; the row is carried so the data file stays a
/// complete mirror of the reference tables.
pub const KEYBOARD_24K_KEY: &str = "KEYBOARD_24K";

/// Gauge table key used when a mode has no row of its own: the seven-key set, which the reference
/// implementation also falls back to for an unrecognised mode (`BMSPlayerRule.java:17`).
pub const DEFAULT_GAUGE_KEY: &str = "BEAT_7K";

const BUILTIN_JUDGE_RON: &str = include_str!("../data/judge.ron");
const BUILTIN_GAUGE_RON: &str = include_str!("../data/gauge.ron");

static BUILTIN_JUDGE: OnceLock<JudgeTables> = OnceLock::new();
static BUILTIN_GAUGE: OnceLock<GaugeTables> = OnceLock::new();

/// Why a judge or gauge data file could not be turned into tables.
#[derive(Debug, thiserror::Error)]
pub enum JudgeDataError {
    #[error("cannot read judge data file {path}: {source}")]
    Read { path: PathBuf, source: std::io::Error },
    #[error("cannot parse judge data file {path}: {source}")]
    Parse { path: PathBuf, source: Box<ron::error::SpannedError> },
    #[error("judge data file {path} declares version {found}, expected {expected}")]
    Version { path: PathBuf, found: u32, expected: u32 },
}

/// Serialisable form of [`JudgeWindows`]: the five `(late, early)` bounds of one timing table.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JudgeWindowsData {
    pub pg: (i64, i64),
    pub gr: (i64, i64),
    pub gd: (i64, i64),
    pub bd: (i64, i64),
    pub ms: Option<(i64, i64)>,
}

/// Serialisable form of [`JudgeProperty`]: one mode's four timing tables plus its release margins
/// and per-judge combo/vanish policy.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JudgePropertyData {
    pub note: JudgeWindowsData,
    pub scratch: JudgeWindowsData,
    pub ln_end: JudgeWindowsData,
    pub ln_scratch_end: JudgeWindowsData,
    pub longnote_margin_us: i64,
    pub longscratch_margin_us: i64,
    pub combo: [bool; 6],
    pub judge_vanish: [bool; 6],
    pub miss_condition: MissCondition,
}

/// A whole judge data file: one [`JudgePropertyData`] per mode id.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JudgeTables {
    pub version: u32,
    pub properties: BTreeMap<String, JudgePropertyData>,
}

/// Reference implementation `GrooveGauge.GaugeModifier`: how a gauge's positive deltas are
/// rescaled for the chart at construction time.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum GaugeModifier {
    Total,
    LimitIncrement,
    ModifyDamage,
    None,
}

/// One row of the reference implementation's `GaugeProperty.GaugeElementProperty`: the gauge's
/// bounds, its per-judge deltas in judge order (PG, GR, GD, BD, PR, MS) and its "guts" softening
/// bands as ascending `(threshold, damage_multiplier)` pairs.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GaugeParams {
    pub modifier: GaugeModifier,
    pub min: f32,
    pub max: f32,
    pub init: f32,
    pub border: f32,
    pub deltas: [f32; 6],
    pub guts: Vec<(f32, f32)>,
}

/// The nine gauges one mode offers, in the reference implementation's index order. The last three
/// are the course gauges; this engine builds and updates them for parity but never selects one,
/// having no course mode. `GaugeSet::at` reaches all nine by [`crate::gauge::GaugeIndex`].
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GaugeSet {
    pub assist_easy: GaugeParams,
    pub easy: GaugeParams,
    pub normal: GaugeParams,
    pub hard: GaugeParams,
    pub exhard: GaugeParams,
    pub hazard: GaugeParams,
    pub class: GaugeParams,
    pub exclass: GaugeParams,
    pub exhardclass: GaugeParams,
}

/// A whole gauge data file: one [`GaugeSet`] per mode id.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct GaugeTables {
    pub version: u32,
    pub gauges: BTreeMap<String, GaugeSet>,
}

impl From<JudgeWindows> for JudgeWindowsData {
    fn from(w: JudgeWindows) -> Self {
        JudgeWindowsData { pg: w.pg, gr: w.gr, gd: w.gd, bd: w.bd, ms: w.ms }
    }
}

impl From<JudgeWindowsData> for JudgeWindows {
    fn from(w: JudgeWindowsData) -> Self {
        JudgeWindows { pg: w.pg, gr: w.gr, gd: w.gd, bd: w.bd, ms: w.ms }
    }
}

impl From<JudgeProperty> for JudgePropertyData {
    fn from(p: JudgeProperty) -> Self {
        JudgePropertyData {
            note: p.note.into(),
            scratch: p.scratch.into(),
            ln_end: p.ln_end.into(),
            ln_scratch_end: p.ln_scratch_end.into(),
            longnote_margin_us: p.longnote_margin,
            longscratch_margin_us: p.longscratch_margin,
            combo: p.combo,
            judge_vanish: p.judge_vanish,
            miss_condition: p.miss_condition,
        }
    }
}

impl From<JudgePropertyData> for JudgeProperty {
    fn from(p: JudgePropertyData) -> Self {
        JudgeProperty {
            note: p.note.into(),
            scratch: p.scratch.into(),
            ln_end: p.ln_end.into(),
            ln_scratch_end: p.ln_scratch_end.into(),
            longnote_margin: p.longnote_margin_us,
            longscratch_margin: p.longscratch_margin_us,
            combo: p.combo,
            judge_vanish: p.judge_vanish,
            miss_condition: p.miss_condition,
        }
    }
}

impl JudgeTables {
    /// The row for a mode id, e.g. `"BEAT_7K"`.
    pub fn get(&self, mode_id: &str) -> Option<&JudgePropertyData> {
        self.properties.get(mode_id)
    }

    /// The row for `mode`, matching [`JudgeProperty::for_mode`] on the bundled data.
    pub fn for_mode(&self, mode: &Mode) -> Option<&JudgePropertyData> {
        self.get(mode.name)
    }
}

impl GaugeSet {
    /// The parameters for one selectable gauge of this set.
    pub fn get(&self, kind: GaugeKind) -> &GaugeParams {
        match kind {
            GaugeKind::AssistEasy => &self.assist_easy,
            GaugeKind::Easy => &self.easy,
            GaugeKind::Normal => &self.normal,
            GaugeKind::Hard => &self.hard,
            GaugeKind::ExHard => &self.exhard,
            GaugeKind::Hazard => &self.hazard,
        }
    }
}

impl GaugeTables {
    /// The set for a mode id, e.g. `"BEAT_7K"`.
    pub fn get(&self, mode_id: &str) -> Option<&GaugeSet> {
        self.gauges.get(mode_id)
    }

    /// The set for `mode`, falling back to [`DEFAULT_GAUGE_KEY`] for a mode the file has no row
    /// for, which is the reference implementation's own fallback (`BMSPlayerRule.java:17`).
    pub fn for_mode(&self, mode: &Mode) -> Option<&GaugeSet> {
        self.get(mode.name).or_else(|| self.get(DEFAULT_GAUGE_KEY))
    }
}

/// The judge tables compiled into the binary. Parsed once; the bundled file is covered by
/// `builtin_judge_tables_match_the_program_defaults`, so a parse failure here is a build defect.
pub fn builtin_judge_tables() -> &'static JudgeTables {
    BUILTIN_JUDGE.get_or_init(|| ron::from_str(BUILTIN_JUDGE_RON).expect("bundled data/judge.ron parses"))
}

/// The gauge tables compiled into the binary. See [`builtin_judge_tables`].
pub fn builtin_gauge_tables() -> &'static GaugeTables {
    BUILTIN_GAUGE.get_or_init(|| ron::from_str(BUILTIN_GAUGE_RON).expect("bundled data/gauge.ron parses"))
}

fn read_to_string(path: &Path) -> Result<String, JudgeDataError> {
    std::fs::read_to_string(path).map_err(|source| JudgeDataError::Read { path: path.to_path_buf(), source })
}

fn check_version(path: &Path, found: u32, expected: u32) -> Result<(), JudgeDataError> {
    if found == expected { Ok(()) } else { Err(JudgeDataError::Version { path: path.to_path_buf(), found, expected }) }
}

/// Load a user judge table override from disk.
pub fn load_judge_tables(path: &Path) -> Result<JudgeTables, JudgeDataError> {
    let text = read_to_string(path)?;
    let tables: JudgeTables = ron::from_str(&text).map_err(|source| JudgeDataError::Parse { path: path.to_path_buf(), source: Box::new(source) })?;
    check_version(path, tables.version, CURRENT_JUDGE_DATA_VERSION)?;
    Ok(tables)
}

/// Load a user gauge table override from disk.
pub fn load_gauge_tables(path: &Path) -> Result<GaugeTables, JudgeDataError> {
    let text = read_to_string(path)?;
    let tables: GaugeTables = ron::from_str(&text).map_err(|source| JudgeDataError::Parse { path: path.to_path_buf(), source: Box::new(source) })?;
    check_version(path, tables.version, CURRENT_GAUGE_DATA_VERSION)?;
    Ok(tables)
}

#[cfg(test)]
mod data_tests {
    use super::*;
    use crate::gauge::default_params;

    const MODES: [Mode; 6] = [Mode::BEAT_5K, Mode::BEAT_7K, Mode::BEAT_10K, Mode::BEAT_14K, Mode::POPN_9K, Mode::KEYBOARD_24K];
    const ALL_KINDS: [GaugeKind; 6] = [GaugeKind::AssistEasy, GaugeKind::Easy, GaugeKind::Normal, GaugeKind::Hard, GaugeKind::ExHard, GaugeKind::Hazard];

    #[test]
    fn builtin_judge_tables_parse_at_the_current_version() {
        assert_eq!(builtin_judge_tables().version, CURRENT_JUDGE_DATA_VERSION);
    }

    #[test]
    fn builtin_judge_tables_cover_every_mode_and_the_keyboard_row() {
        let keys: Vec<&str> = builtin_judge_tables().properties.keys().map(String::as_str).collect();
        assert_eq!(keys, ["BEAT_10K", "BEAT_14K", "BEAT_5K", "BEAT_7K", KEYBOARD_24K_KEY, "POPN_9K"]);
    }

    #[test]
    fn builtin_judge_tables_match_the_program_defaults() {
        let tables = builtin_judge_tables();
        for mode in MODES {
            let loaded = tables.for_mode(&mode).unwrap_or_else(|| panic!("{} row present", mode.name));
            assert_eq!(*loaded, JudgePropertyData::from(JudgeProperty::defaults_for_mode(&mode)), "{} row", mode.name);
        }
    }

    #[test]
    fn the_engine_judges_against_the_data_file_not_the_compiled_in_table() {
        for mode in MODES {
            let from_data = *builtin_judge_tables().for_mode(&mode).unwrap_or_else(|| panic!("{} row present", mode.name));
            assert_eq!(JudgePropertyData::from(JudgeProperty::for_mode(&mode)), from_data, "{} is read from the data file", mode.name);
        }
    }

    #[test]
    fn a_mode_the_data_file_has_no_row_for_falls_back_to_the_compiled_in_table() {
        let unknown = Mode { name: "NOT_IN_THE_DATA_FILE", ..Mode::BEAT_7K };
        assert!(builtin_judge_tables().for_mode(&unknown).is_none(), "the fixture mode really is absent");
        assert_eq!(JudgePropertyData::from(JudgeProperty::for_mode(&unknown)), JudgePropertyData::from(JudgeProperty::defaults_for_mode(&unknown)));
    }

    #[test]
    fn builtin_judge_tables_reach_all_four_reference_rows() {
        let tables = builtin_judge_tables();
        let rows = [
            ("BEAT_5K", JudgeProperty::FIVEKEYS),
            ("BEAT_10K", JudgeProperty::FIVEKEYS),
            ("BEAT_7K", JudgeProperty::SEVENKEYS),
            ("BEAT_14K", JudgeProperty::SEVENKEYS),
            ("POPN_9K", JudgeProperty::PMS),
            (KEYBOARD_24K_KEY, JudgeProperty::KEYBOARD),
        ];
        for (key, prop) in rows {
            assert_eq!(*tables.get(key).unwrap_or_else(|| panic!("{key} row present")), JudgePropertyData::from(prop), "{key}");
        }
    }

    #[test]
    fn judge_rows_survive_a_round_trip_through_the_runtime_type() {
        for mode in MODES {
            let data = *builtin_judge_tables().for_mode(&mode).unwrap();
            let back: JudgePropertyData = JudgeProperty::from(data).into();
            assert_eq!(back, data, "{}", mode.name);
        }
    }

    #[test]
    fn builtin_gauge_tables_parse_at_the_current_version() {
        assert_eq!(builtin_gauge_tables().version, CURRENT_GAUGE_DATA_VERSION);
    }

    #[test]
    fn builtin_gauge_tables_match_the_program_defaults() {
        let set = builtin_gauge_tables().get(DEFAULT_GAUGE_KEY).expect("default gauge row present");
        for kind in ALL_KINDS {
            assert_eq!(*set.get(kind), default_params(kind), "{kind:?}");
        }
    }

    #[test]
    fn the_engine_builds_gauges_from_the_data_file_not_the_compiled_in_row() {
        let set = builtin_gauge_tables().get(DEFAULT_GAUGE_KEY).expect("default gauge row present");
        for kind in ALL_KINDS {
            assert_eq!(crate::gauge::params(kind), *set.get(kind), "{kind:?} is read from the data file");
        }
    }

    #[test]
    fn every_mode_resolves_to_the_gauge_set_its_rule_names() {
        let tables = builtin_gauge_tables();
        for mode in MODES {
            let set = tables.for_mode(&mode).unwrap_or_else(|| panic!("{} gauge set", mode.name));
            let expected = tables.get(crate::gauge_tables::GaugeSetId::for_mode(&mode).data_key()).expect("set row present");
            assert_eq!(set, expected, "{}", mode.name);
        }
    }

    #[test]
    fn a_mode_the_gauge_file_has_no_row_for_falls_back_to_the_default_set() {
        let tables = builtin_gauge_tables();
        let unknown = Mode { name: "NOT_IN_THE_DATA_FILE", ..Mode::BEAT_7K };
        assert_eq!(tables.for_mode(&unknown), tables.get(DEFAULT_GAUGE_KEY));
    }

    #[test]
    fn gauge_params_from_data_build_the_same_gauge_as_the_program_defaults() {
        let set = builtin_gauge_tables().get(DEFAULT_GAUGE_KEY).unwrap();
        for kind in ALL_KINDS {
            for (total, notes) in [(200.0, 10), (0.0, 1000), (300.0, 1000), (162.5, 100)] {
                let from_data = crate::gauge::Gauge::from_params(kind, set.get(kind), total, notes);
                let from_defaults = crate::gauge::Gauge::from_params(kind, &default_params(kind), total, notes);
                assert_eq!(from_data.value(), from_defaults.value(), "{kind:?} {total} {notes}");
                let mut a = from_data;
                let mut b = from_defaults;
                for judge in [crate::Judge::PerfectGreat, crate::Judge::Great, crate::Judge::Good, crate::Judge::Bad, crate::Judge::Poor] {
                    a.update(judge);
                    b.update(judge);
                    assert_eq!(a.value(), b.value(), "{kind:?} {total} {notes} after {judge:?}");
                }
            }
        }
    }

    #[test]
    fn tables_round_trip_through_ron() {
        let judge = builtin_judge_tables();
        let text = ron::ser::to_string_pretty(judge, ron::ser::PrettyConfig::default()).unwrap();
        assert_eq!(&ron::from_str::<JudgeTables>(&text).unwrap(), judge);

        let gauge = builtin_gauge_tables();
        let text = ron::ser::to_string_pretty(gauge, ron::ser::PrettyConfig::default()).unwrap();
        assert_eq!(&ron::from_str::<GaugeTables>(&text).unwrap(), gauge);
    }

    #[test]
    fn load_judge_tables_reports_a_missing_file() {
        let err = load_judge_tables(Path::new("does-not-exist-judge.ron")).unwrap_err();
        assert!(matches!(err, JudgeDataError::Read { .. }), "{err}");
    }

    #[test]
    fn load_judge_tables_reports_a_bad_parse() {
        let dir = std::env::temp_dir().join(format!("rbms-judge-data-parse-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("judge.ron");
        std::fs::write(&path, "(this is not ron").unwrap();
        let err = load_judge_tables(&path).unwrap_err();
        assert!(matches!(err, JudgeDataError::Parse { .. }), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_judge_tables_rejects_an_unknown_version() {
        let dir = std::env::temp_dir().join(format!("rbms-judge-data-version-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("judge.ron");
        let mut tables = builtin_judge_tables().clone();
        tables.version = CURRENT_JUDGE_DATA_VERSION + 1;
        std::fs::write(&path, ron::ser::to_string(&tables).unwrap()).unwrap();
        let err = load_judge_tables(&path).unwrap_err();
        assert!(
            matches!(err, JudgeDataError::Version { found, expected, .. } if found == CURRENT_JUDGE_DATA_VERSION + 1 && expected == CURRENT_JUDGE_DATA_VERSION),
            "{err}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_round_trips_the_bundled_files() {
        let dir = std::env::temp_dir().join(format!("rbms-judge-data-load-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let judge_path = dir.join("judge.ron");
        std::fs::write(&judge_path, BUILTIN_JUDGE_RON).unwrap();
        assert_eq!(&load_judge_tables(&judge_path).unwrap(), builtin_judge_tables());

        let gauge_path = dir.join("gauge.ron");
        std::fs::write(&gauge_path, BUILTIN_GAUGE_RON).unwrap();
        assert_eq!(&load_gauge_tables(&gauge_path).unwrap(), builtin_gauge_tables());
        std::fs::remove_dir_all(&dir).ok();
    }
}
