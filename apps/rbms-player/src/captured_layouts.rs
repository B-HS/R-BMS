//! The layouts the player captured on a result screen, kept next to the scores in the config
//! directory, and opened on LAYOUT PICK to be looked at and played again.
//!
//! A capture is the run's arrangement and the seed it was laid out with: what a run is asked to
//! borrow ([`crate::borrowed_layout::LayoutRequest`]), so a run on a capture is laid out exactly as
//! the captured run was and is recorded and sent with the same arrangement and seed. Only an
//! arrangement that moves whole lanes is captured (RANDOM, R-RANDOM, MIRROR and ROTATE): it is the
//! one a lane order written out can show. An arrangement that lays the notes out one by one is not.
//!
//! The file holds each capture once by its chart, arrangement and seed, newest first, and says
//! beside it what the eye needs without the chart at hand: the title, the mode, the lane order a
//! side, when it was captured and what the run came to.

use std::path::{Path, PathBuf};

use rbms_chart::shuffle::{NoteOption, lane_permutation, layout_of_permutation, layout_row_text, random_seed_limit};
use rbms_model::Mode;
use serde::{Deserialize, Serialize};

use crate::notify::{Level, notify_text};
use crate::text::menus::layout_pick::CaptureText;
use crate::write_atomic;

/// Name of the file the captures are kept in, next to `scores.ron`.
pub(crate) const CAPTURED_LAYOUTS_FILE: &str = "captured_layouts.ron";

/// One captured layout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct CapturedLayout {
    /// The chart it was played on.
    pub(crate) md5: String,
    /// The chart's title and subtitle, for the list.
    pub(crate) title: String,
    /// The chart's mode by name ([`Mode::name`]).
    pub(crate) mode: String,
    /// The arrangement as a replay file names it ([`NoteOption::label`]).
    pub(crate) random_label: String,
    /// The seed the run was laid out with.
    pub(crate) seed: u64,
    /// The lane order of each side, as LAYOUT PICK writes it (`3517246`).
    pub(crate) layout: Vec<String>,
    /// When it was captured, in milliseconds since the epoch.
    pub(crate) captured_at: i64,
    /// The EX score and the lamp the run came to.
    pub(crate) ex_score: u32,
    pub(crate) lamp: String,
}

impl CapturedLayout {
    /// Whether the two are the same chart laid out by the same arrangement and seed.
    pub(crate) fn same_layout(&self, other: &CapturedLayout) -> bool {
        self.md5.eq_ignore_ascii_case(&other.md5) && self.random_label.eq_ignore_ascii_case(&other.random_label) && self.seed == other.seed
    }

    /// The arrangement it was played on.
    pub(crate) fn option(&self) -> NoteOption {
        NoteOption::from_str(&self.random_label)
    }

    /// The lane order of each side, worked out again from the arrangement and the seed on `mode`;
    /// `None` when the arrangement does not move whole lanes or keeps no key lane on its side.
    pub(crate) fn rows(&self, mode: Mode) -> Option<Vec<Vec<u8>>> {
        lane_order(mode, self.option(), self.seed)
    }

    /// The lane order written out, sides apart by a slash.
    pub(crate) fn layout_text(&self) -> String {
        self.layout.join(" / ")
    }
}

/// The lane order `option` lays `mode` out in from `seed`, a row a side, when it moves whole lanes.
fn lane_order(mode: Mode, option: NoteOption, seed: u64) -> Option<Vec<Vec<u8>>> {
    captures_lanes(option).then(|| layout_of_permutation(mode, &lane_permutation(option, mode, seed))).flatten()
}

/// Whether `option` moves whole lanes, which is what a lane order can be written for and what is
/// captured.
pub(crate) fn captures_lanes(option: NoteOption) -> bool {
    matches!(option, NoteOption::Random | NoteOption::RRandom | NoteOption::Mirror | NoteOption::Rotate)
}

/// Why the run on a result screen is not captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureRefusal {
    /// The screen was built without a run.
    NoRun,
    /// AUTOPLAY played the run.
    Autoplay,
    /// The run is a replay being watched.
    Replay,
    /// The run is a stage of a course or a practice slice.
    Course,
    /// The run was played on OFF, which has nothing to capture.
    Off,
    /// The arrangement lays the notes out one by one, named as a replay file names it.
    NoteByNote(String),
    /// LAYOUT PICK does not lay out charts of the mode, named.
    Mode(String),
}

impl CaptureRefusal {
    /// What the screen says of it.
    pub(crate) fn report(&self) {
        match self {
            CaptureRefusal::NoRun => notify_text(Level::Warn, CaptureText::RefusedNoRun, &[]),
            CaptureRefusal::Autoplay => notify_text(Level::Warn, CaptureText::RefusedAutoplay, &[]),
            CaptureRefusal::Replay => notify_text(Level::Warn, CaptureText::RefusedReplay, &[]),
            CaptureRefusal::Course => notify_text(Level::Warn, CaptureText::RefusedCourse, &[]),
            CaptureRefusal::Off => notify_text(Level::Warn, CaptureText::RefusedOff, &[]),
            CaptureRefusal::NoteByNote(label) => notify_text(Level::Warn, CaptureText::RefusedNoteByNote, &[label]),
            CaptureRefusal::Mode(name) => notify_text(Level::Warn, CaptureText::RefusedMode, &[name]),
        }
    }
}

/// How a run came to be played, as far as capturing it goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RunKind {
    pub(crate) plays_itself: bool,
    pub(crate) replay: bool,
    pub(crate) course_or_practice: bool,
}

/// A run as a result screen knows it, for a capture of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CapturedRun<'a> {
    pub(crate) md5: &'a str,
    pub(crate) title: &'a str,
    pub(crate) subtitle: &'a str,
    pub(crate) mode: Mode,
    pub(crate) option: NoteOption,
    pub(crate) seed: u64,
    pub(crate) ex_score: u32,
    pub(crate) lamp: &'a str,
}

impl CapturedRun<'_> {
    /// The capture of the run, played as `kind` says, or why there is none. It is dated when it is
    /// kept ([`CapturedLayouts::keep`]).
    pub(crate) fn capture(&self, kind: RunKind) -> Result<CapturedLayout, CaptureRefusal> {
        if kind.plays_itself {
            return Err(CaptureRefusal::Autoplay);
        }
        if kind.replay {
            return Err(CaptureRefusal::Replay);
        }
        if kind.course_or_practice {
            return Err(CaptureRefusal::Course);
        }
        if self.option == NoteOption::Off {
            return Err(CaptureRefusal::Off);
        }
        if !captures_lanes(self.option) {
            return Err(CaptureRefusal::NoteByNote(self.option.label().to_string()));
        }
        let rows = random_seed_limit(self.mode).and_then(|_| lane_order(self.mode, self.option, self.seed));
        let rows = rows.ok_or_else(|| CaptureRefusal::Mode(self.mode.name.to_string()))?;
        Ok(CapturedLayout {
            md5: self.md5.to_string(),
            title: format!("{} {}", self.title, self.subtitle).trim().to_string(),
            mode: self.mode.name.to_string(),
            random_label: self.option.label().to_string(),
            seed: self.seed,
            layout: rows.iter().map(|row| layout_row_text(row)).collect(),
            captured_at: 0,
            ex_score: self.ex_score,
            lamp: self.lamp.to_string(),
        })
    }
}

/// What keeping a capture came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kept {
    /// It was added.
    Added,
    /// The same layout of the chart was kept already, and is left as it was.
    AlreadyKept,
}

/// The captures as the file holds them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct CapturedFile {
    /// Newest first.
    layouts: Vec<CapturedLayout>,
}

/// The captures, and where they are kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CapturedLayouts {
    file: CapturedFile,
    path: PathBuf,
}

impl CapturedLayouts {
    /// Read the captures at `path`. A file that is not there yet is none -- the state of a fresh
    /// install -- and one that cannot be read is reported and taken as none, so a broken file never
    /// stops a screen from opening. Such a file is written over by the next capture.
    pub(crate) fn load(path: &Path) -> CapturedLayouts {
        let path = path.to_path_buf();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return CapturedLayouts { file: CapturedFile::default(), path };
        };
        match ron::from_str::<CapturedFile>(&text) {
            Ok(file) => CapturedLayouts { file, path },
            Err(e) => {
                notify_text(Level::Warn, CaptureText::NotLoaded, &[&e]);
                CapturedLayouts { file: CapturedFile::default(), path }
            }
        }
    }

    /// The captures kept beside the settings file `settings_path`, like the score book.
    pub(crate) fn beside(settings_path: &Path) -> CapturedLayouts {
        CapturedLayouts::load(&captured_layouts_path(settings_path))
    }

    /// Write the captures out, reporting a failed write rather than losing it silently.
    fn save(&self) {
        let text = match ron::ser::to_string_pretty(&self.file, ron::ser::PrettyConfig::default()) {
            Ok(text) => text,
            Err(e) => {
                notify_text(Level::Error, CaptureText::SaveFailed, &[&e]);
                return;
            }
        };
        if let Err(e) = write_atomic(&self.path, &text) {
            notify_text(Level::Error, CaptureText::SaveFailedAt, &[&self.path.display(), &e]);
        }
    }

    /// Keep `capture` dated `now_ms`, newest first, and write the file at once; a layout of the
    /// chart kept already is left as it was.
    pub(crate) fn keep(&mut self, capture: CapturedLayout, now_ms: i64) -> Kept {
        if self.file.layouts.iter().any(|kept| kept.same_layout(&capture)) {
            return Kept::AlreadyKept;
        }
        self.file.layouts.insert(0, CapturedLayout { captured_at: now_ms, ..capture });
        self.save();
        Kept::Added
    }

    /// Let go of the capture of the same layout as `capture`, and write the file at once. `false`
    /// when there was none.
    pub(crate) fn remove(&mut self, capture: &CapturedLayout) -> bool {
        let before = self.file.layouts.len();
        self.file.layouts.retain(|kept| !kept.same_layout(capture));
        let removed = self.file.layouts.len() != before;
        if removed {
            self.save();
        }
        removed
    }

    /// Every capture, newest first.
    pub(crate) fn all(&self) -> &[CapturedLayout] {
        &self.file.layouts
    }

    /// The captures of the chart `md5`, newest first.
    pub(crate) fn of_chart(&self, md5: &str) -> Vec<CapturedLayout> {
        self.file.layouts.iter().filter(|kept| kept.md5.eq_ignore_ascii_case(md5)).cloned().collect()
    }
}

impl crate::AppShared {
    /// Keep `capture`, now, and say what came of it.
    pub(crate) fn keep_captured_layout(&mut self, capture: CapturedLayout) -> Kept {
        let now_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default();
        let shown = format!("{} {}", capture.random_label, capture.layout_text());
        let kept = self.captured_layouts.keep(capture, now_ms);
        let said = if kept == Kept::Added { CaptureText::Captured } else { CaptureText::AlreadyCaptured };
        notify_text(Level::Info, said, &[&shown]);
        kept
    }
}

/// Where the captures live for a given settings file: next to it, like the score book.
pub(crate) fn captured_layouts_path(settings_path: &Path) -> PathBuf {
    settings_path.parent().map(|dir| dir.join(CAPTURED_LAYOUTS_FILE)).unwrap_or_else(|| PathBuf::from(CAPTURED_LAYOUTS_FILE))
}

#[cfg(test)]
mod tests;
