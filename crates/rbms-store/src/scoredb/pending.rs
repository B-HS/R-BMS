//! Plays kept on the side while the score database cannot take them: one file per play, holding the
//! row it would have been and the body it is to be sent as, read into the database the next time it
//! opens ([`ScoreDb::import_pending_runs`]).
//!
//! A database that will not open (written by a newer build, damaged, locked, or in a folder that
//! cannot be written) would otherwise leave every play of the session in memory alone, gone when the
//! game closes.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::sync::KeptSubmission;
use super::{PlayLog, ScoreDb, ScoreDbError};
use crate::error::StoreError;

/// What the file of a play kept on the side ends in.
pub const PENDING_RUN_SUFFIX: &str = ".ron";

/// One play kept on the side: the row, whether it may set bests, and the submission it is to be sent
/// as with the account it was made under.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingRun {
    pub log: PlayLog,
    pub updates_best: bool,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub played_as: Option<String>,
}

impl PendingRun {
    /// The play as the document its file holds.
    pub fn to_ron(&self) -> Result<String, StoreError> {
        Ok(ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())?)
    }

    /// A play read back from the document its file holds.
    pub fn from_ron(text: &str) -> Result<PendingRun, StoreError> {
        Ok(ron::from_str(text)?)
    }
}

/// What one import of the plays kept on the side did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PendingImport {
    /// Plays read into the database.
    pub imported: usize,
    /// Files that could not be read or parsed, left where they are.
    pub unreadable: Vec<PathBuf>,
    /// Files whose play was read in but which could not be removed afterwards.
    pub stuck: Vec<PathBuf>,
    /// Files whose play the database refused, with why, left where they are. The files after one
    /// are still read in.
    pub refused: Vec<(PathBuf, String)>,
}

impl ScoreDb {
    /// Read every play kept on the side in `dir` into the database, oldest file name first, each in
    /// the transaction [`ScoreDb::record_play_kept`] writes a play in, and remove its file once it is
    /// in. A play the database already holds (its file was read in before but could not be removed)
    /// is not read in again, only its file removed. A play the database refuses is reported and its
    /// file left; the ones after it are still read in. A missing directory is nothing to import.
    pub fn import_pending_runs(&mut self, dir: &Path) -> Result<PendingImport, ScoreDbError> {
        let mut report = PendingImport::default();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(report),
            Err(e) => return Err(ScoreDbError::Io(e)),
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with(PENDING_RUN_SUFFIX)))
            .collect();
        files.sort();
        for file in files {
            let run = match std::fs::read_to_string(&file).map_err(StoreError::Read).and_then(|text| PendingRun::from_ron(&text)) {
                Ok(run) => run,
                Err(_) => {
                    report.unreadable.push(file);
                    continue;
                }
            };
            let kept = run.body.as_deref().map(|body| KeptSubmission { body, played_as: run.played_as.as_deref() });
            let held = match self.holds_play(&run.log) {
                Ok(held) => held,
                Err(e) => {
                    report.refused.push((file, e.to_string()));
                    continue;
                }
            };
            if !held {
                if let Err(e) = self.record_play_kept(&run.log, run.updates_best, kept) {
                    report.refused.push((file, e.to_string()));
                    continue;
                }
                report.imported += 1;
            }
            if std::fs::remove_file(&file).is_err() {
                report.stuck.push(file);
            }
        }
        Ok(report)
    }

    /// Whether the history already holds `log`: a play of the same chart, play mode and long-note
    /// flavour that ended in the same second. Two plays of one chart cannot end in one second.
    fn holds_play(&self, log: &PlayLog) -> Result<bool, ScoreDbError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM scorelog WHERE chart_key = ?1 AND mode = ?2 AND ln_mode = ?3 AND date = ?4)",
            rusqlite::params![log.chart_key, log.mode, log.ln_mode, log.date],
            |row| row.get(0),
        )?)
    }
}
