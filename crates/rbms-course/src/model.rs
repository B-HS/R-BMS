//! The course document: the ordered chart list, the constraint set that narrows how it may be
//! played, and the trophy rules a finished run is graded against.
//!
//! [`Course::validate`] is the whole of the reference implementation's `CourseData.validate()`
//! (`CourseData.java:99-131`), which normalises far more often than it rejects: a nameless course
//! is named, an untitled chart is titled, a second constraint in the same exclusive group is
//! dropped and an unqualifiable trophy is removed, and only four situations make a course fail
//! outright. Rejecting on the normalisable cases would drop most real course files.

use serde::de::{Deserializer, Error as DeError};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Name given to a course whose file leaves `name` empty (`CourseData.java:102-104`). The course is
/// kept, not rejected.
pub const DEFAULT_COURSE_NAME: &str = "No Course Title";

/// Prefix of the name given to a chart entry with no title of its own; the one-based stage number
/// follows it (`CourseData.java:109-111`).
const CHART_TITLE_PREFIX: &str = "course ";

/// How many mutually exclusive constraint groups there are. A course keeps at most one constraint
/// per group (`CourseData.java:118-124`, a five-slot bucket array).
pub const CONSTRAINT_GROUP_COUNT: usize = 5;

/// Byte written between the fields of the course hash so no two different field splits can produce
/// the same input string.
const HASH_SEPARATOR: &[u8] = b"\n";

/// A trophy whose miss-rate ceiling is not above this is unqualifiable and is dropped
/// (`CourseData.java:270`, `missrate > 0`).
const TROPHY_MISSRATE_FLOOR: f32 = 0.0;

/// A trophy whose score-rate floor is not below this is unqualifiable and is dropped
/// (`CourseData.java:270`, `scorerate < 100`).
const TROPHY_SCORERATE_CEILING: f32 = 100.0;

/// Percent scale for the miss-rate and score-rate a run is graded on.
const PERCENT: f64 = 100.0;

/// The theoretical EX score per note, which is what a score rate is measured against
/// (`GradeBar.java:88`, `notes * 2`).
const EX_PER_NOTE: u32 = 2;

/// One restriction a course places on how it may be played.
///
/// Each constraint belongs to an exclusive [`group`](CourseConstraint::group); a course keeps only
/// the first one declared from each group. The wire spelling is [`token`](CourseConstraint::token),
/// the vocabulary difficulty tables publish their courses in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CourseConstraint {
    Class,
    Mirror,
    Random,
    NoSpeed,
    NoGood,
    NoGreat,
    GaugeLr2,
    Gauge5Keys,
    Gauge7Keys,
    Gauge9Keys,
    Gauge24Keys,
    Ln,
    Cn,
    Hcn,
}

impl CourseConstraint {
    /// How many constraints there are.
    pub const COUNT: usize = 14;

    /// Every constraint, in declaration order.
    pub const ALL: [CourseConstraint; CourseConstraint::COUNT] = [
        CourseConstraint::Class,
        CourseConstraint::Mirror,
        CourseConstraint::Random,
        CourseConstraint::NoSpeed,
        CourseConstraint::NoGood,
        CourseConstraint::NoGreat,
        CourseConstraint::GaugeLr2,
        CourseConstraint::Gauge5Keys,
        CourseConstraint::Gauge7Keys,
        CourseConstraint::Gauge9Keys,
        CourseConstraint::Gauge24Keys,
        CourseConstraint::Ln,
        CourseConstraint::Cn,
        CourseConstraint::Hcn,
    ];

    /// Wire spelling of this constraint (`CourseData.java:146-199`).
    pub fn token(self) -> &'static str {
        match self {
            CourseConstraint::Class => "grade",
            CourseConstraint::Mirror => "grade_mirror",
            CourseConstraint::Random => "grade_random",
            CourseConstraint::NoSpeed => "no_speed",
            CourseConstraint::NoGood => "no_good",
            CourseConstraint::NoGreat => "no_great",
            CourseConstraint::GaugeLr2 => "gauge_lr2",
            CourseConstraint::Gauge5Keys => "gauge_5k",
            CourseConstraint::Gauge7Keys => "gauge_7k",
            CourseConstraint::Gauge9Keys => "gauge_9k",
            CourseConstraint::Gauge24Keys => "gauge_24k",
            CourseConstraint::Ln => "ln",
            CourseConstraint::Cn => "cn",
            CourseConstraint::Hcn => "hcn",
        }
    }

    /// Parse a wire spelling, case-insensitively. `None` for anything outside the fourteen.
    pub fn from_token(s: &str) -> Option<CourseConstraint> {
        CourseConstraint::ALL.into_iter().find(|c| c.token().eq_ignore_ascii_case(s))
    }

    /// Which exclusive group this constraint belongs to: 0 grade, 1 hi-speed, 2 judge, 3 gauge
    /// table, 4 long-note flavour (`CourseData.java:146-199`, the second enum argument).
    pub fn group(self) -> u8 {
        match self {
            CourseConstraint::Class | CourseConstraint::Mirror | CourseConstraint::Random => 0,
            CourseConstraint::NoSpeed => 1,
            CourseConstraint::NoGood | CourseConstraint::NoGreat => 2,
            CourseConstraint::GaugeLr2
            | CourseConstraint::Gauge5Keys
            | CourseConstraint::Gauge7Keys
            | CourseConstraint::Gauge9Keys
            | CourseConstraint::Gauge24Keys => 3,
            CourseConstraint::Ln | CourseConstraint::Cn | CourseConstraint::Hcn => 4,
        }
    }

    /// The name a course file written by the reference implementation carries, which serialises its
    /// enum by identifier rather than by [`token`](CourseConstraint::token). Reading both is what
    /// lets a hand-made course file and a table-published one load the same way.
    fn reference_name(self) -> &'static str {
        match self {
            CourseConstraint::Class => "CLASS",
            CourseConstraint::Mirror => "MIRROR",
            CourseConstraint::Random => "RANDOM",
            CourseConstraint::NoSpeed => "NO_SPEED",
            CourseConstraint::NoGood => "NO_GOOD",
            CourseConstraint::NoGreat => "NO_GREAT",
            CourseConstraint::GaugeLr2 => "GAUGE_LR2",
            CourseConstraint::Gauge5Keys => "GAUGE_5KEYS",
            CourseConstraint::Gauge7Keys => "GAUGE_7KEYS",
            CourseConstraint::Gauge9Keys => "GAUGE_9KEYS",
            CourseConstraint::Gauge24Keys => "GAUGE_24KEYS",
            CourseConstraint::Ln => "LN",
            CourseConstraint::Cn => "CN",
            CourseConstraint::Hcn => "HCN",
        }
    }

    /// Parse either spelling. No token collides with a reference name under a case-insensitive
    /// comparison except where both name the same constraint, so the order of the two lookups
    /// cannot change the answer.
    pub fn from_any(s: &str) -> Option<CourseConstraint> {
        CourseConstraint::from_token(s).or_else(|| CourseConstraint::ALL.into_iter().find(|c| c.reference_name().eq_ignore_ascii_case(s)))
    }
}

impl Serialize for CourseConstraint {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.token())
    }
}

impl<'de> Deserialize<'de> for CourseConstraint {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<CourseConstraint, D::Error> {
        let raw = String::deserialize(deserializer)?;
        CourseConstraint::from_any(&raw).ok_or_else(|| D::Error::custom(format!("unknown course constraint {raw:?}")))
    }
}

/// Read a constraint list, dropping spellings this build does not know instead of failing the whole
/// course. The reference does the same when it builds a course from a difficulty table
/// (`TableDataAccessor.java:198-199` filters the unresolved ones out).
fn constraints_from_wire<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<CourseConstraint>, D::Error> {
    let raw = Vec::<String>::deserialize(deserializer)?;
    Ok(raw.iter().filter_map(|token| CourseConstraint::from_any(token)).collect())
}

/// One grade a finished course can earn: the highest miss rate and the lowest score rate that still
/// qualify for it (`CourseData.java:229-273`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrophyRule {
    pub name: String,
    pub missrate: f32,
    pub scorerate: f32,
}

impl TrophyRule {
    /// Whether this rule can ever be met (`CourseData.java:268-271`). A rule needing a miss rate at
    /// or below zero, or a score rate at or above the theoretical maximum, is unreachable and is
    /// dropped by [`Course::validate`].
    ///
    /// The reference tests `name != null`, which an absent name decodes to; a JSON `""` is a name it
    /// would keep and this drops, because an unnamed trophy has nothing to show or submit.
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.missrate > TROPHY_MISSRATE_FLOOR && self.scorerate < TROPHY_SCORERATE_CEILING
    }

    /// Whether a run with these totals earns this trophy (`GradeBar.java:84-88`). Both bounds are
    /// inclusive: the rule states the worst miss rate and the lowest score rate still accepted.
    pub fn qualifies(&self, notes: u32, min_bp: u32, ex_score: u32) -> bool {
        if notes == 0 {
            return false;
        }
        let missrate = f64::from(min_bp) * PERCENT / f64::from(notes);
        let scorerate = f64::from(ex_score) * PERCENT / f64::from(notes * EX_PER_NOTE);
        f64::from(self.missrate) >= missrate && f64::from(self.scorerate) <= scorerate
    }
}

/// One stage of a course, identified by the two chart hashes so the same course file resolves
/// against any library.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CourseChart {
    pub md5: String,
    pub sha256: String,
    pub title: String,
}

impl CourseChart {
    /// The hash this chart contributes to [`Course::hash`]: its SHA-256 when it has one, and its
    /// MD5 otherwise, so a course file that carries only the older hash still gets a stable id.
    pub fn hash_key(&self) -> &str {
        if self.sha256.is_empty() { &self.md5 } else { &self.sha256 }
    }

    /// Whether this entry identifies a chart at all (`SongData.java:619-621`).
    pub fn is_identified(&self) -> bool {
        !self.md5.is_empty() || !self.sha256.is_empty()
    }
}

/// A course: an ordered run of charts played as one, under one carried gauge.
///
/// The field aliases are the spellings a course file written by the reference implementation uses,
/// so such a file loads unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Course {
    pub name: String,
    #[serde(alias = "hash", alias = "song")]
    pub charts: Vec<CourseChart>,
    #[serde(alias = "constraint", deserialize_with = "constraints_from_wire")]
    pub constraints: Vec<CourseConstraint>,
    #[serde(alias = "trophy")]
    pub trophies: Vec<TrophyRule>,
    pub release: bool,
}

impl Default for Course {
    fn default() -> Course {
        Course { name: String::new(), charts: Vec::new(), constraints: Vec::new(), trophies: Vec::new(), release: true }
    }
}

impl Course {
    /// Normalise this course and answer whether it may be kept (`CourseData.java:99-131`).
    ///
    /// Normalisation is the common path: an empty name becomes [`DEFAULT_COURSE_NAME`], an untitled
    /// chart becomes `course {n}`, a second constraint from a group already spoken for is dropped,
    /// and a trophy that can never be met is removed. Only two situations reject the course — no
    /// charts at all, and a chart entry carrying neither hash.
    ///
    /// Collapsing the constraints leaves them ordered by [`group`](CourseConstraint::group) rather
    /// than by declaration, which is what the reference's five-slot bucket does.
    pub fn validate(&mut self) -> bool {
        if self.charts.is_empty() {
            return false;
        }
        if self.name.is_empty() {
            self.name = DEFAULT_COURSE_NAME.to_string();
        }
        for (i, chart) in self.charts.iter_mut().enumerate() {
            if chart.title.is_empty() {
                chart.title = format!("{CHART_TITLE_PREFIX}{}", i + 1);
            }
            if !chart.is_identified() {
                return false;
            }
        }
        self.constraints = collapse_constraints(&self.constraints);
        self.trophies.retain(TrophyRule::is_valid);
        true
    }

    /// Whether this is a grade course, i.e. one carrying a group-0 constraint
    /// (`CourseData.java:82-89`).
    pub fn is_class(&self) -> bool {
        self.constraints.iter().any(|c| c.group() == 0)
    }

    /// The id a course is submitted and ranked under: the SHA-256 of the name and the chart hashes,
    /// in stage order, each followed by a separator so no two different courses can hash alike.
    ///
    /// This is rbms's own `course_hash` for the IR superset, not the reference's cache key — that
    /// one hashes the chart hashes and the constraint tokens and leaves the name out
    /// (`RankingDataCache.java:84-96`).
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());
        hasher.update(HASH_SEPARATOR);
        for chart in &self.charts {
            hasher.update(chart.hash_key().as_bytes());
            hasher.update(HASH_SEPARATOR);
        }
        format!("{:x}", hasher.finalize())
    }

    /// The constraint this course carries from `group`, if any.
    pub fn constraint_in_group(&self, group: u8) -> Option<CourseConstraint> {
        self.constraints.iter().copied().find(|c| c.group() == group)
    }

    /// How many stages the course has.
    pub fn stage_count(&self) -> usize {
        self.charts.len()
    }
}

/// Keep the first constraint declared from each exclusive group and drop the rest, emitting what
/// survives in group order (`CourseData.java:118-124`).
fn collapse_constraints(list: &[CourseConstraint]) -> Vec<CourseConstraint> {
    let mut slots: [Option<CourseConstraint>; CONSTRAINT_GROUP_COUNT] = [None; CONSTRAINT_GROUP_COUNT];
    for constraint in list {
        let slot = &mut slots[usize::from(constraint.group())];
        if slot.is_none() {
            *slot = Some(*constraint);
        }
    }
    slots.into_iter().flatten().collect()
}
