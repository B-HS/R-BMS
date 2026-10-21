use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::StoreError;

/// The long-note key of a record whose chart states its own long-note flavour, so the player's LN
/// MODE setting could not have changed how it was judged. Records written before the key existed
/// default to it and so all compare against each other, which is what they used to do.
///
/// The reference keys a stored score by LN MODE only when the chart leaves the flavour undefined
/// (`PlayDataAccessor.java:200-205` passes `containsUndefinedLongNote() ? lnmode : 0`), because that
/// is the only case where the setting changes the note count and therefore the EX denominator.
pub const SCORE_LN_MODE_FROM_CHART: &str = "CHART";

/// Version of the judging rules a record was produced under. Bumped whenever a rule change makes
/// older records not directly comparable; records written before the field existed default to 0.
///
/// Version 2 is the judge-parity pass: nine parallel gauges off the reference gauge tables, the
/// `LightAssistEasy` lamp and the reference gauge-to-lamp mapping, charge-note deferral and the
/// hell-charge gauge tick. A version 1 record was judged by rules this build no longer produces, so
/// it keeps loading and reads as stale.
///
/// Version 3 changes SCRATCH AUTO alone ([`SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION`]): every
/// other record of version 2 was judged as this build judges it.
///
/// Version 4 records a run as the reference does ([`REFERENCE_RECORD_RULE_VERSION`]): `avgjudge`
/// is the mean distance of every note from its input, a run the gauge ended is charged for the notes
/// it never reached, a course stage keeps its own combo, a full combo is the combo standing at the
/// end, and a TOTAL the player set is an assist. The same generation is judged in event time and
/// on charts tidied as the reference tidies them; see [`REFERENCE_RECORD_RULE_VERSION`].
pub const SCORE_RULE_VERSION: u32 = 4;

/// The first rule version whose records are what the reference records of a run
/// (`BMSPlayer.java:863-930`).
///
/// * `avgjudge` is `ScoreData.avgjudge`: the mean distance of every note from its input, a note
///   never hit counting a second. Before, it was the size of the mean signed offset of the hits
///   alone, in which early and late cancel out, so an older value is no measure of the same thing
///   and never competes with one of this version for a best.
/// * `minbp` of a run the gauge ended counts the notes it never reached. An older record left them
///   out, and they are added back from its counts wherever it is compared
///   ([`crate::scoredb::charged_minbp`]).
/// * `combo` of a course stage is the stage's own, and a full combo is the combo standing at the
///   end. An older record keeps the combo and the lamp it was given.
/// * A run played with a TOTAL of the player's own is assisted and sets no best. An older record
///   of such a run cannot be told from any other.
///
/// A record of this version was also judged by the engine of the same release, which an older
/// record was not:
///
/// * Every input is judged at the instant it carries, and what time alone settles (a note let
///   pass, a release held over, a mine, a hell-charge tick) at the instant it falls due, so the
///   frame rate no longer moves a judgement; a controller change is judged where the controller
///   layer stamped it rather than at the next poll, up to a frame earlier; and a run stops being
///   judged the instant its gauge empties. An older record differs only where a frame boundary
///   decided it, and is compared as it stands.
/// * A plain note inside a long note's body on its lane is background sound, and a long-note head
///   that never ends is dropped, so some charts have fewer notes than they had (the song database's
///   parser version 3). An older record keeps the note count and EX ceiling it was played on and is
///   compared as it stands; only those charts can hold an older best scored out of more notes.
pub const REFERENCE_RECORD_RULE_VERSION: u32 = 4;

/// The first rule version this build judges a run played by hand exactly as it did then: the
/// judge-parity pass, whose assisted lamps are demoted as this build demotes them.
pub(crate) const JUDGE_PARITY_RULE_VERSION: u32 = 2;

/// The first rule version in which SCRATCH AUTO takes the turntable's notes off the chart a run is
/// judged on, so that they count for nothing, and in which such a run sets bests like any other. A
/// SCRATCH AUTO record of an earlier version counted the turntable as perfect greats played for the
/// player and set no best; it is history and a lamp, and no score.
pub const SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION: u32 = 3;

/// The assist level of a run whose only assist is SCRATCH AUTO (`rbms_ir::mapping::LIGHT_ASSIST`).
/// Every other non-zero level is a widened judge, or an assist an imported record did not name.
pub const SCRATCH_AUTO_ASSIST: i32 = 1;

/// Lamp id of `rbms_judge::ClearType::LightAssistEasy`: the lamp of a clear on the ASSIST EASY gauge
/// and the lamp a SCRATCH AUTO clear is demoted to, the one lamp an assist grade splits. Kept as a
/// number for the reason [`crate::scoredb`] keeps its lamp ids as numbers.
pub const LIGHT_ASSIST_CLEAR_ID: u8 = 3;

/// Assist grade id of a record with no grade (`rbms_judge::AssistGrade::None`).
pub const ASSIST_GRADE_NONE: u8 = 0;
/// Assist grade id of a light-assist clear demoted from an EASY gauge clear (ASSIST EASY).
pub const ASSIST_GRADE_EASY: u8 = 1;
/// Assist grade id of a light-assist clear demoted from a NORMAL or CLASS gauge clear (ASSIST CLEAR).
pub const ASSIST_GRADE_CLEAR: u8 = 2;
/// Assist grade id of a light-assist clear demoted from a HARD or EXCLASS gauge clear (ASSIST HARD).
pub const ASSIST_GRADE_HARD: u8 = 3;
/// Assist grade id of a light-assist clear demoted from an EX-HARD, HAZARD or EXHARDCLASS gauge clear
/// (ASSIST EXHARD).
pub const ASSIST_GRADE_EXHARD: u8 = 4;
/// How many assist grade ids there are, [`ASSIST_GRADE_NONE`] included.
pub const ASSIST_GRADE_COUNT: u8 = 5;

/// The grade a demoted clear on each stored gauge id folds to, indexed by the id
/// ([`crate::scoredb::gauge_id`], the order of `rbms_judge::gauge::GaugeIndex`): the lamp the gauge
/// awards a clear, full combos left out, as `rbms_judge::assist_grade` folds it. The player's tests
/// hold the two to the same answer for every gauge.
pub(crate) const ASSIST_GRADE_OF_GAUGE: [u8; 9] = [
    ASSIST_GRADE_NONE,
    ASSIST_GRADE_EASY,
    ASSIST_GRADE_CLEAR,
    ASSIST_GRADE_HARD,
    ASSIST_GRADE_EXHARD,
    ASSIST_GRADE_EXHARD,
    ASSIST_GRADE_CLEAR,
    ASSIST_GRADE_HARD,
    ASSIST_GRADE_EXHARD,
];

/// The assist grade of a stored run: the lamp id it was awarded, the gauge id it finished on, its
/// assist level and its rule version, as `scorelog` holds them.
///
/// Only a [`LIGHT_ASSIST_CLEAR_ID`] lamp has one, and then the grade of the gauge
/// ([`ASSIST_GRADE_OF_GAUGE`]); on the ASSIST EASY gauge, whose own lamp that is, it is none. A run
/// whose assist level was never recorded ([`crate::scoredb::ASSIST_UNRECORDED`]) and one of a rule
/// older than the judge-parity pass, which stored lamps nothing demoted, have none either.
pub fn assist_grade_of(clear: u8, gauge: i32, assist: i32, rule_version: u32) -> u8 {
    if clear != LIGHT_ASSIST_CLEAR_ID || assist == crate::scoredb::ASSIST_UNRECORDED || is_stale_rule_version(rule_version) {
        return ASSIST_GRADE_NONE;
    }
    usize::try_from(gauge).ok().and_then(|index| ASSIST_GRADE_OF_GAUGE.get(index)).copied().unwrap_or(ASSIST_GRADE_NONE)
}

/// A best lamp as it is compared: the lamp id first and the assist grade inside it, so an ASSIST
/// HARD clear outranks an ASSIST CLEAR one and both stay below an EASY clear.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordLamp {
    /// `ClearType` id of the lamp, 0 to 10.
    pub clear: u8,
    /// Assist grade id inside that lamp ([`assist_grade_of`]).
    pub assist_grade: u8,
}

/// Whether a record was judged under a rule version this build no longer produces for a run played
/// by hand, so what is shown next to it is the presenting layer's decision rather than this
/// crate's.
pub fn is_stale_rule_version(rule_version: u32) -> bool {
    rule_version < JUDGE_PARITY_RULE_VERSION
}

/// Whether a run recorded at assist level `assist` under `rule_version` may set a chart's bests:
/// one with no assist, and one played with SCRATCH AUTO since that stopped counting the turntable.
/// A widened judge never does, and neither does a SCRATCH AUTO run of an older rule.
pub fn sets_best(assist: i32, rule_version: u32) -> bool {
    match assist {
        0 => true,
        SCRATCH_AUTO_ASSIST => rule_version >= SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION,
        _ => false,
    }
}

/// One persisted play result, kept locally so play history and replays survive without a score
/// server. `clear` is the reference implementation's `ClearType` id (0..10) so lamps round-trip; `replay_file` is
/// the basename (under `replays/`) of the saved replay, when one was recorded.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoreRecord {
    pub md5: String,
    pub title: String,
    pub mode: String,
    pub clear: u8,
    pub ex_score: u32,
    pub max_ex: u32,
    pub counts: [u32; 6],
    #[serde(default)]
    pub empty_poor: u32,
    pub max_combo: u32,
    pub total_notes: u32,
    pub gauge: String,
    pub gauge_value: f32,
    pub random: String,
    pub played_at: i64,
    #[serde(default)]
    pub replay_file: Option<String>,
    /// Judging-rule version this record was produced under (see [`SCORE_RULE_VERSION`]).
    #[serde(default)]
    pub rule_version: u32,
    /// The long-note flavour this run has to be compared within: the LN MODE it was played under
    /// when the chart left its long notes unstated, and [`SCORE_LN_MODE_FROM_CHART`] otherwise.
    /// Charge notes are judged at both ends, so a run forced to CN has twice the notes and twice the
    /// EX ceiling of the same chart played as plain long notes.
    #[serde(default = "chart_ln_mode_key")]
    pub ln_mode: String,
    /// The run may not set bests ([`sets_best`]): a widened judge window, or SCRATCH AUTO under a
    /// rule older than [`SCRATCH_AUTO_LEAVES_THE_TURNTABLE_RULE_VERSION`]. It is kept as history but
    /// excluded from the stored bests — the reference implementation clears the same `score` flag
    /// and gates exscore/minbp/combo on it (`ScoreData.java:548,566,572,578`). Records written
    /// before the field existed default to `false`.
    #[serde(default)]
    pub assisted: bool,
    /// Assist level of the run as `scorelog.assist` holds it: 0 none, [`SCRATCH_AUTO_ASSIST`] for
    /// SCRATCH AUTO alone, 2 for a widened judge or a TOTAL of the player's own. A record written
    /// before the field existed reads as [`crate::scoredb::ASSIST_UNRECORDED`].
    #[serde(default = "unrecorded_assist")]
    pub assist: i32,
    /// The `scorelog` row this record was read from or written as, which is how a screen showing the
    /// record finds where the play stands with a score server. `None` for a record that never had a
    /// row (one read from `scores.ron`), and never written to that file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_id: Option<i64>,
}

fn chart_ln_mode_key() -> String {
    SCORE_LN_MODE_FROM_CHART.to_string()
}

fn unrecorded_assist() -> i32 {
    crate::scoredb::ASSIST_UNRECORDED
}

impl ScoreRecord {
    /// The assist grade of the run ([`assist_grade_of`]): 1 to 4 for a light-assist clear graded by
    /// the gauge it finished on, 0 otherwise.
    pub fn assist_grade(&self) -> u8 {
        assist_grade_of(self.clear, crate::scoredb::gauge_id(&self.gauge), self.assist, self.rule_version)
    }

    /// The run's lamp as bests are compared.
    pub fn lamp(&self) -> RecordLamp {
        RecordLamp { clear: self.clear, assist_grade: self.assist_grade() }
    }
}

/// Wire shape of `scores.ron`: the record list and nothing else, so the lookup index below never
/// reaches the file and old files keep parsing byte for byte.
#[derive(Default, Deserialize)]
#[serde(default)]
struct ScoreRecords {
    records: Vec<ScoreRecord>,
}

impl From<ScoreRecords> for ScoreBook {
    fn from(wire: ScoreRecords) -> ScoreBook {
        ScoreBook::from_records(wire.records)
    }
}

/// All local play records. Append-only in practice; queried by chart md5 for the select screen.
///
/// The record list is read through [`ScoreBook::records`] and only ever grown through
/// [`ScoreBook::push`], so the md5 lookup index cannot fall out of step with it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(from = "ScoreRecords")]
pub struct ScoreBook {
    records: Vec<ScoreRecord>,
    /// Lowercased md5 to the positions in `records` that carry it, so a per-chart query costs one
    /// hash lookup instead of a scan of the whole book on every select-screen row.
    #[serde(skip)]
    index: HashMap<String, Vec<usize>>,
}

impl ScoreBook {
    /// Build a book from records already in memory, indexing them as it goes.
    pub fn from_records(records: Vec<ScoreRecord>) -> ScoreBook {
        let mut book = ScoreBook { records, index: HashMap::new() };
        book.rebuild_index();
        book
    }

    pub fn load(path: &Path) -> ScoreBook {
        match std::fs::read_to_string(path) {
            Ok(s) => ron::from_str(&s).unwrap_or_else(|e| {
                let backup = path.with_extension("ron.bak");
                let _ = std::fs::rename(path, &backup);
                eprintln!("scores parse failed ({e}); backed up to {} and starting empty", backup.display());
                ScoreBook::default()
            }),
            Err(_) => ScoreBook::default(),
        }
    }

    /// Serialize and durably replace `path`, reporting why on failure.
    pub fn try_save(&self, path: &Path) -> Result<(), StoreError> {
        let s = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?;
        write_atomic(path, &s).map_err(StoreError::Write)
    }

    /// [`ScoreBook::try_save`] for the fire-and-forget call sites: a failed save is reported on
    /// stderr and never interrupts play.
    pub fn save(&self, path: &Path) {
        match self.try_save(path) {
            Ok(()) => {}
            Err(StoreError::Write(e)) => eprintln!("scores write failed ({}): {e}", path.display()),
            Err(e) => eprintln!("scores save failed: {e}"),
        }
    }

    /// Every record in the book, in insertion order.
    pub fn records(&self) -> &[ScoreRecord] {
        &self.records
    }

    pub fn push(&mut self, record: ScoreRecord) {
        self.index.entry(record.md5.to_ascii_lowercase()).or_default().push(self.records.len());
        self.records.push(record);
    }

    /// Stop every record that names the replay `file` from offering it, the file being gone. Returns
    /// how many records named it.
    pub fn forget_replay(&mut self, file: &str) -> usize {
        let mut forgotten = 0;
        for record in self.records.iter_mut().filter(|record| record.replay_file.as_deref() == Some(file)) {
            record.replay_file = None;
            forgotten += 1;
        }
        forgotten
    }

    /// Rebuild the md5 lookup index from the record list. Called after a load.
    pub fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, r) in self.records.iter().enumerate() {
            self.index.entry(r.md5.to_ascii_lowercase()).or_default().push(i);
        }
    }

    /// Positions in `records` holding a chart's plays, in insertion order.
    fn positions(&self, md5: &str) -> &[usize] {
        self.index.get(&md5.to_ascii_lowercase()).map(Vec::as_slice).unwrap_or(&[])
    }

    /// All records for a chart, newest first.
    pub fn for_md5(&self, md5: &str) -> Vec<&ScoreRecord> {
        let mut v: Vec<&ScoreRecord> = self.positions(md5).iter().map(|&i| &self.records[i]).collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.played_at));
        v
    }

    /// Records for a chart that may set its bests: assisted runs are history only.
    fn scoring_records(&self, md5: &str) -> impl Iterator<Item = &ScoreRecord> {
        self.positions(md5).iter().map(|&i| &self.records[i]).filter(|r| !r.assisted)
    }

    /// Whether a record may be compared against a run played under `ln_mode`. Two runs of the same
    /// chart under different LN MODEs are not the same chart: charge notes are judged twice, so
    /// their note counts and EX ceilings differ.
    fn comparable_ln_mode(record: &ScoreRecord, ln_mode: &str) -> bool {
        record.ln_mode == ln_mode
    }

    /// Best EX on a chart, folded without the allocate-and-sort of `for_md5` (called every frame for
    /// the live score graph). `None` if the chart has no unassisted records.
    ///
    /// Every record counts, whatever LN MODE it was played under; use
    /// [`best_ex_for_md5_in_ln_mode`](Self::best_ex_for_md5_in_ln_mode) to compare within one.
    pub fn best_ex_for_md5(&self, md5: &str) -> Option<u32> {
        self.scoring_records(md5).map(|r| r.ex_score).max()
    }

    /// Best EX among the records of a chart that were played under `ln_mode`, the key described on
    /// [`ScoreRecord::ln_mode`].
    pub fn best_ex_for_md5_in_ln_mode(&self, md5: &str, ln_mode: &str) -> Option<u32> {
        self.scoring_records(md5).filter(|r| Self::comparable_ln_mode(r, ln_mode)).map(|r| r.ex_score).max()
    }

    /// The record of a chart played under `ln_mode` that holds its best EX and has a replay to go
    /// with it: of the runs that scored that EX, the oldest that kept its replay. The reference keeps
    /// the run that first reached a best, since a run only replaces it with a higher EX
    /// (`ScoreData.java:550-564`); a run that reached it with no replay kept (AUTO REPLAY off, a save
    /// that failed, a replay since pruned) gives way to the next run of the same EX. `None` when no
    /// run of the best EX has a replay.
    pub fn best_record_with_replay_in_ln_mode(&self, md5: &str, ln_mode: &str) -> Option<&ScoreRecord> {
        let best = self.best_ex_for_md5_in_ln_mode(md5, ln_mode)?;
        self.scoring_records(md5).filter(|r| Self::comparable_ln_mode(r, ln_mode)).find(|r| r.ex_score == best && r.replay_file.is_some())
    }

    /// Best clear-lamp id on a chart (highest `ClearType` id), folded without allocation — used for
    /// the per-row clear-lamp LED in the select list. `None` if the chart has no records. The lamp id
    /// of [`ScoreBook::best_lamp_for_md5`].
    pub fn best_clear_for_md5(&self, md5: &str) -> Option<u8> {
        self.best_lamp_for_md5(md5).map(|lamp| lamp.clear)
    }

    /// Best clear-lamp id among the records of a chart that were played under `ln_mode`.
    pub fn best_clear_for_md5_in_ln_mode(&self, md5: &str, ln_mode: &str) -> Option<u8> {
        self.best_lamp_for_md5_in_ln_mode(md5, ln_mode).map(|lamp| lamp.clear)
    }

    /// Best lamp on a chart, the lamp id first and the assist grade inside it ([`RecordLamp`]).
    /// `None` if the chart has no records.
    ///
    /// Assisted runs count here, unlike [`ScoreBook::best_ex_for_md5`]: the reference implementation
    /// records the lamp of an assisted run too, having first demoted it to
    /// `AssistEasy`/`LightAssistEasy` (`BMSPlayer.java:864-874`), which is what the player writes
    /// into the record. A demoted lamp can only raise the LED as far as the assist allows — but only
    /// a record this build wrote carries a demoted lamp at all, so an assisted record from an older
    /// rule version is left out rather than allowed to raise the LED with a lamp nothing demoted.
    pub fn best_lamp_for_md5(&self, md5: &str) -> Option<RecordLamp> {
        self.lamp_records(md5).map(ScoreRecord::lamp).max()
    }

    /// Best lamp among the records of a chart that were played under `ln_mode`.
    pub fn best_lamp_for_md5_in_ln_mode(&self, md5: &str, ln_mode: &str) -> Option<RecordLamp> {
        self.lamp_records(md5).filter(|r| Self::comparable_ln_mode(r, ln_mode)).map(ScoreRecord::lamp).max()
    }

    /// Records of a chart whose lamp counts, as [`ScoreBook::best_lamp_for_md5`] describes.
    fn lamp_records(&self, md5: &str) -> impl Iterator<Item = &ScoreRecord> {
        self.positions(md5).iter().map(|&i| &self.records[i]).filter(|r| !r.assisted || !is_stale_rule_version(r.rule_version))
    }
}
