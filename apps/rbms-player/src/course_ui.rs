//! The course tab on the select screen and the course result screen: rows, constraint badges and
//! stage progress.
//!
//! A course is listed rather than sorted into the song browser, so the browser grows a second tab
//! ([`SelectTab`]) whose rows are courses. A course row says how many stages it has, which
//! constraints it plays under, and whether every one of its charts is actually in the library —
//! the reference will not start a course whose charts it cannot all find
//! (`BarRenderer.java:154-155`).
//!
//! [`CourseOverrides`] is the other half: what the constraint set does to the play settings when
//! the course starts. It answers with values rather than writing them anywhere, so the settings the
//! player chose are still theirs when the course ends.
//!
//! The module is complete but not yet called: the call sites named in
//! `docs/plan/phase-g-wiring/G3-course.md` belong to the integration branch, and the allow below
//! goes away with the first of them.

use std::path::{Path, PathBuf};

use rbms_chart::shuffle::NoteOption;
use rbms_config::PlayOptions;
use rbms_course::{Course, CourseChart, CourseConstraint, CourseRun};
use rbms_judge::gauge_tables::GaugeSetId;
use rbms_judge::ln::LnMode;
use rbms_library::Library;

/// Directory under the config directory that course files live in.
pub(crate) const COURSE_DIR_NAME: &str = "courses";

/// Note scroll a course that forbids speed changes is played at (`LaneRenderer.java:96-101`).
const LOCKED_HISPEED: f64 = 1.0;

/// Lane shade a course that forbids speed changes is played with — none, cover and lift and hidden
/// alike (`LaneRenderer.java:97-99`).
const LOCKED_LANE_SHADE: f32 = 0.0;

/// Reference random-option id above which a `grade_random` course rejects the choice
/// (`MusicSelector.java:493-499`, `random > 5`). The ids run OFF, MIRROR, RANDOM, R-RANDOM,
/// S-RANDOM, SPIRAL, H-RANDOM, ALL-SCR, so everything through SPIRAL is allowed.
const GRADE_RANDOM_ALLOWED: [NoteOption; 6] =
    [NoteOption::Off, NoteOption::Mirror, NoteOption::Random, NoteOption::SRandom, NoteOption::RRandom, NoteOption::Rotate];

/// Which of the two lists the song browser is showing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SelectTab {
    #[default]
    Songs,
    Courses,
}

impl SelectTab {
    /// The tab's name as the header draws it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            SelectTab::Songs => "SONGS",
            SelectTab::Courses => "COURSE",
        }
    }

    /// The tab the browser moves to when the tab key is pressed.
    pub(crate) fn next(self) -> SelectTab {
        match self {
            SelectTab::Songs => SelectTab::Courses,
            SelectTab::Courses => SelectTab::Songs,
        }
    }
}

/// Where course files are kept.
pub(crate) fn courses_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(COURSE_DIR_NAME)
}

/// One course as the browser holds it: the course itself plus which of its stages the library
/// cannot supply.
#[derive(Clone, Debug)]
pub(crate) struct CourseEntry {
    pub(crate) course: Course,
    /// Stage indices whose chart is not in the library. A course with any of these cannot start.
    pub(crate) missing: Vec<usize>,
}

impl CourseEntry {
    /// Whether every stage resolves, which is what the reference requires before a course may be
    /// started (`BarRenderer.java:154-155`).
    pub(crate) fn is_playable(&self) -> bool {
        self.missing.is_empty()
    }
}

/// The course tab's list and its cursor.
#[derive(Clone, Debug, Default)]
pub(crate) struct CourseList {
    entries: Vec<CourseEntry>,
    cursor: usize,
}

impl CourseList {
    /// Read every course file under `dir` and resolve each one against `library`.
    pub(crate) fn load(dir: &Path, library: &Library) -> CourseList {
        let mut list = CourseList::from_courses(rbms_course::load_dir(dir));
        list.resolve_in_library(library);
        list
    }

    /// Hold an already-loaded course list, with nothing resolved yet.
    pub(crate) fn from_courses(courses: Vec<Course>) -> CourseList {
        CourseList { entries: courses.into_iter().map(|course| CourseEntry { course, missing: Vec::new() }).collect(), cursor: 0 }
    }

    /// Recompute which stages are missing, which the browser does whenever the library changes.
    pub(crate) fn resolve(&mut self, exists: impl Fn(&CourseChart) -> bool) {
        for entry in &mut self.entries {
            entry.missing = entry.course.charts.iter().enumerate().filter(|(_, chart)| !exists(chart)).map(|(i, _)| i).collect();
        }
    }

    /// Resolve against a scanned library, which indexes charts by MD5.
    pub(crate) fn resolve_in_library(&mut self, library: &Library) {
        self.resolve(|chart| library_index(library, chart).is_some());
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn entries(&self) -> &[CourseEntry] {
        &self.entries
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    /// The course under the cursor.
    pub(crate) fn focused(&self) -> Option<&CourseEntry> {
        self.entries.get(self.cursor)
    }

    /// Move the cursor by `delta` rows, wrapping at both ends the way the song list does.
    pub(crate) fn move_cursor(&mut self, delta: isize) {
        if self.entries.is_empty() {
            self.cursor = 0;
            return;
        }
        let len = self.entries.len() as isize;
        let next = (self.cursor as isize + delta).rem_euclid(len);
        self.cursor = next as usize;
    }

    /// Put the cursor on a row directly, ignoring one that is not there.
    pub(crate) fn focus(&mut self, index: usize) {
        if index < self.entries.len() {
            self.cursor = index;
        }
    }

    /// The rows to draw, in list order.
    pub(crate) fn rows(&self) -> Vec<CourseRow> {
        self.entries.iter().map(CourseRow::of).collect()
    }
}

/// One drawn course row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CourseRow {
    pub(crate) title: String,
    /// Stage count, plus how many stages the library is missing when it is missing any.
    pub(crate) detail: String,
    pub(crate) badges: Vec<&'static str>,
    pub(crate) playable: bool,
}

/// Badge shown for a course whose file marks it as not for release, which is playable but never
/// submitted (`CourseResult.java:96`).
const UNRELEASED_BADGE: &str = "UNRELEASED";

impl CourseRow {
    fn of(entry: &CourseEntry) -> CourseRow {
        let stages = entry.course.stage_count();
        let detail = if entry.missing.is_empty() { format!("{stages} STAGES") } else { format!("{stages} STAGES - {} MISSING", entry.missing.len()) };
        let mut badges: Vec<&'static str> = entry.course.constraints.iter().map(|c| constraint_badge(*c)).collect();
        if !entry.course.release {
            badges.push(UNRELEASED_BADGE);
        }
        CourseRow { title: entry.course.name.clone(), detail, badges, playable: entry.is_playable() }
    }
}

/// Where a course chart sits in the library, by MD5.
///
/// A course entry that carries only a SHA-256 does not resolve yet: the scanned library is indexed
/// by MD5 alone. The song database branch adds the second index, and the wiring document names the
/// one call that changes.
pub(crate) fn library_index(library: &Library, chart: &CourseChart) -> Option<usize> {
    if chart.md5.is_empty() {
        return None;
    }
    library.indices_for_md5(&chart.md5).first().copied()
}

/// The short name a constraint is drawn under.
pub(crate) fn constraint_badge(constraint: CourseConstraint) -> &'static str {
    match constraint {
        CourseConstraint::Class => "GRADE",
        CourseConstraint::Mirror => "GRADE MIRROR",
        CourseConstraint::Random => "GRADE RANDOM",
        CourseConstraint::NoSpeed => "NO SPEED",
        CourseConstraint::NoGood => "NO GOOD",
        CourseConstraint::NoGreat => "NO GREAT",
        CourseConstraint::GaugeLr2 => "LR2 GAUGE",
        CourseConstraint::Gauge5Keys => "5K GAUGE",
        CourseConstraint::Gauge7Keys => "7K GAUGE",
        CourseConstraint::Gauge9Keys => "9K GAUGE",
        CourseConstraint::Gauge24Keys => "24K GAUGE",
        CourseConstraint::Ln => "LN",
        CourseConstraint::Cn => "CN",
        CourseConstraint::Hcn => "HCN",
    }
}

/// Which note options a grade course leaves open (`MusicSelector.java:476-500`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RandomLock {
    /// Not a grade course: every option stands.
    #[default]
    Free,
    /// Grade: the chart is played exactly as written.
    Off,
    /// Grade, mirror allowed: MIRROR stands, anything else falls back to OFF.
    Mirror,
    /// Grade, random allowed: everything through SPIRAL stands, the rest falls back to OFF.
    Random,
}

impl RandomLock {
    /// The note option the run actually plays under, given the one the player chose.
    pub(crate) fn clamp(self, chosen: NoteOption) -> NoteOption {
        match self {
            RandomLock::Free => chosen,
            RandomLock::Off => NoteOption::Off,
            RandomLock::Mirror => {
                if chosen == NoteOption::Mirror {
                    NoteOption::Mirror
                } else {
                    NoteOption::Off
                }
            }
            RandomLock::Random => {
                if GRADE_RANDOM_ALLOWED.contains(&chosen) {
                    chosen
                } else {
                    NoteOption::Off
                }
            }
        }
    }

    /// Whether the course leaves this option alone.
    pub(crate) fn allows(self, chosen: NoteOption) -> bool {
        self.clamp(chosen) == chosen
    }
}

/// What a course's constraint set does to the play settings for the length of the run.
///
/// Every answer is returned rather than stored: the run is played with these values and the
/// player's own settings are untouched.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CourseOverrides {
    /// The course forbids changing the note scroll, which also pins the lane shades
    /// (`LaneRenderer.java:95-102`, `BMSPlayer.java:419-423`).
    pub(crate) speed_locked: bool,
    pub(crate) random: RandomLock,
    /// Gauge table the course forces, or `None` to keep the one in force (`GrooveGauge.java:123-149`).
    pub(crate) gauge_set: Option<GaugeSetId>,
    /// Long-note flavour the course forces, or `None` to keep the one in force
    /// (`MusicSelector.java:501-503`).
    pub(crate) ln_mode: Option<LnMode>,
    /// Constraints this build stores and shows but does not yet enforce, because enforcing them
    /// needs the judge tables to be selectable per run.
    pub(crate) unapplied: Vec<&'static str>,
}

impl CourseOverrides {
    /// Read a course's constraint set.
    pub(crate) fn of(course: &Course) -> CourseOverrides {
        let mut out = CourseOverrides::default();
        for constraint in &course.constraints {
            match constraint {
                CourseConstraint::Class => out.random = RandomLock::Off,
                CourseConstraint::Mirror => out.random = RandomLock::Mirror,
                CourseConstraint::Random => out.random = RandomLock::Random,
                CourseConstraint::NoSpeed => out.speed_locked = true,
                CourseConstraint::NoGood | CourseConstraint::NoGreat => out.unapplied.push(constraint_badge(*constraint)),
                CourseConstraint::GaugeLr2 => out.gauge_set = Some(GaugeSetId::Lr2),
                CourseConstraint::Gauge5Keys => out.gauge_set = Some(GaugeSetId::FiveKeys),
                CourseConstraint::Gauge7Keys => out.gauge_set = Some(GaugeSetId::SevenKeys),
                CourseConstraint::Gauge9Keys => out.gauge_set = Some(GaugeSetId::Pms),
                CourseConstraint::Gauge24Keys => out.gauge_set = Some(GaugeSetId::Keyboard),
                CourseConstraint::Ln => out.ln_mode = Some(LnMode::LongNote),
                CourseConstraint::Cn => out.ln_mode = Some(LnMode::ChargeNote),
                CourseConstraint::Hcn => out.ln_mode = Some(LnMode::HellChargeNote),
            }
        }
        out
    }

    /// The long-note flavour the run plays under.
    pub(crate) fn ln_mode_for(&self, chosen: LnMode) -> LnMode {
        self.ln_mode.unwrap_or(chosen)
    }

    /// The gauge table the run plays on.
    pub(crate) fn gauge_set_for(&self, chosen: Option<GaugeSetId>) -> Option<GaugeSetId> {
        self.gauge_set.or(chosen)
    }

    /// Rewrite a copy of the play settings for the run. The caller keeps the original and puts it
    /// back when the course ends.
    pub(crate) fn apply_to(&self, play: &mut PlayOptions) {
        play.random = self.random.clamp(play.random);
        if self.speed_locked {
            play.hispeed = LOCKED_HISPEED;
            play.constant_speed = false;
            play.cover = LOCKED_LANE_SHADE;
            play.lift = LOCKED_LANE_SHADE;
            play.hidden = LOCKED_LANE_SHADE;
            play.enable_cover = false;
            play.enable_lift = false;
            play.enable_hidden = false;
        }
    }

    /// Whether the hi-speed and lane-shade keys do anything during this course
    /// (`BMSPlayer.java:419-423` disables the control).
    pub(crate) fn accepts_speed_input(&self) -> bool {
        !self.speed_locked
    }
}

/// The stage line the loading and play screens show during a course.
pub(crate) fn stage_label(run: &CourseRun) -> String {
    format!("STAGE {} / {}", run.stage_number(), run.course.stage_count())
}

/// The rows the course result screen lists, as label and value.
pub(crate) fn result_rows(run: &CourseRun) -> Vec<(String, String)> {
    let mut rows = vec![
        ("EX SCORE".to_string(), format!("{} / {}", run.totals.ex, run.totals.max_ex)),
        ("SCORE RATE".to_string(), format!("{:.2}%", run.totals.score_rate())),
        ("MAX COMBO".to_string(), run.totals.max_combo.to_string()),
        ("MISS COUNT".to_string(), run.totals.min_bp().to_string()),
        ("GAUGE".to_string(), format!("{:.1}%", run.carry_gauge)),
    ];
    if let Some(stage) = run.failed_at {
        rows.push(("FAILED AT".to_string(), format!("STAGE {}", stage + 1)));
    }
    if let Some(trophy) = run.trophy() {
        rows.push(("TROPHY".to_string(), trophy.name.clone()));
    }
    rows
}

#[cfg(test)]
mod tests;
