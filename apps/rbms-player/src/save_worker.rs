//! The one thread the frame thread hands its file writes to.
//!
//! A durable write waits on the disk: the file is flushed before it is renamed over the old one, and
//! that flush costs milliseconds on a fast disk and tens of them on a slow one or under a virus
//! scanner. The result screen, Escape out of a run, a run started again, and the answers the network
//! brings in mid-run all save something, and none of them may stall a frame on it. So they hand the
//! write here and go on. A run's play trace is laid out and serialised here too, not only written:
//! the frame thread hands over what it is made of ([`SaveWorker::keep_trace`]).
//!
//! One thread takes every write, in the order they were handed over, so two writes of one file never
//! race each other's temp file. A settings save that a later one replaces before the thread reaches
//! it is not written at all: only the last one matters. What fails comes back to the frame thread
//! ([`AppShared::poll_saves`]), which says so and undoes what pointed at the file.
//!
//! The application waits for the thread as its event loop ends ([`AppShared::finish_saves`], from
//! `ApplicationHandler::exiting`), so a write handed over is finished before the process exits even
//! where the platform exits from inside the loop and nothing is dropped, as macOS does on Quit. The
//! thread is also joined when the worker is dropped.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

use rbms_store::scoredb::{PendingRun, ScoreDb};
use rbms_store::{Replay, StagedWrite};

use crate::notify::{Level, notify};
use crate::play_trace::TraceSource;
use crate::*;

/// What a failed trace write says when the trace made no body.
const TRACE_NOT_SERIALISED: &str = "it does not serialise";

/// Where the trace of a stored run is kept: the run's row and the database it is in, and the file
/// it goes on the side in, holding `run`, when the database does not take it.
pub(crate) struct StoredRun {
    pub(crate) log_id: i64,
    pub(crate) db_path: PathBuf,
    pub(crate) on_the_side: PathBuf,
    /// The run as a file kept on the side holds it, without its trace, which is added there.
    pub(crate) run: PendingRun,
}

/// What a write that failed was writing, which decides what the frame thread does about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Saving {
    /// The settings file.
    Settings,
    /// A run's replay, named as its record names it.
    Replay { file: String },
    /// A run kept on the side while the score database could not take it.
    PendingRun { file: PathBuf },
    /// The trace of a run whose row is stored ([`crate::play_trace`]).
    Trace { log_id: i64 },
    /// Traces made for runs stored without one, and how far the making has got
    /// ([`crate::trace_backfill`]).
    BackfilledTraces,
}

/// One write that failed, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaveFailure {
    pub(crate) what: Saving,
    pub(crate) error: String,
}

type Work = Box<dyn FnOnce() -> Result<(), String> + Send>;

/// One write handed to the thread. A later write that names the same `replaces` makes this one
/// pointless, and the thread skips it.
struct Job {
    what: Saving,
    replaces: Option<PathBuf>,
    work: Work,
}

enum Message {
    Job(Job),
    /// Answer once every write handed over before this one is done.
    Flush(Sender<()>),
}

/// The handle the frame thread holds.
pub(crate) struct SaveWorker {
    /// Where writes go to the thread; `None` once it is let go, or when it could not be started and
    /// writes are done as they are handed over.
    tx: Option<Sender<Message>>,
    thread: Option<JoinHandle<()>>,
    /// Writes are done on the caller's thread as they are handed over, failures reported here: a
    /// test that reads a file straight after saving it stays deterministic. A test of the thread
    /// itself asks for one.
    #[cfg(test)]
    inline: Option<Sender<SaveFailure>>,
    failures: Receiver<SaveFailure>,
}

impl SaveWorker {
    /// The worker the application runs with: a thread of its own in the game, the caller's thread in
    /// a unit test.
    pub(crate) fn for_this_build() -> SaveWorker {
        #[cfg(test)]
        return SaveWorker::inline();
        #[cfg(not(test))]
        SaveWorker::spawn()
    }

    /// A worker with a thread of its own.
    pub(crate) fn spawn() -> SaveWorker {
        SaveWorker::spawn_after(None)
    }

    /// A worker whose thread writes nothing until `gate` is answered or dropped, so a test can look
    /// at what the frame thread left before the writes land.
    #[cfg(test)]
    pub(crate) fn spawn_gated(gate: Receiver<()>) -> SaveWorker {
        SaveWorker::spawn_after(Some(gate))
    }

    fn spawn_after(gate: Option<Receiver<()>>) -> SaveWorker {
        let (tx, rx) = mpsc::channel();
        let (failures_tx, failures) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("rbms-save".into())
            .spawn(move || {
                if let Some(gate) = gate {
                    let _ = gate.recv();
                }
                write_in_order(&rx, &failures_tx);
            })
            .ok();
        if thread.is_none() {
            notify(Level::Error, "save thread not started: files are written as they are saved");
        }
        SaveWorker {
            tx: thread.as_ref().map(|_| tx),
            thread,
            #[cfg(test)]
            inline: None,
            failures,
        }
    }

    /// A worker that writes on the caller's thread.
    #[cfg(test)]
    pub(crate) fn inline() -> SaveWorker {
        let (failures_tx, failures) = mpsc::channel();
        SaveWorker { tx: None, thread: None, inline: Some(failures_tx), failures }
    }

    /// Hand a write over. It runs after every write handed over before it.
    fn hand_over(&self, job: Job) {
        #[cfg(test)]
        if let Some(failures) = &self.inline {
            return run_reporting(job, &|failure| {
                let _ = failures.send(failure);
            });
        }
        let unsent = match &self.tx {
            Some(tx) => match tx.send(Message::Job(job)) {
                Ok(()) => None,
                Err(mpsc::SendError(Message::Job(job))) => Some(job),
                Err(mpsc::SendError(Message::Flush(_))) => None,
            },
            None => Some(job),
        };
        if let Some(job) = unsent {
            run_reporting(job, &|failure| report_now(&failure));
        }
    }

    /// Write `config` to the settings file at `path`. A later save of the same file before this one
    /// is written replaces it.
    pub(crate) fn save_settings(&self, config: Config, path: PathBuf) {
        let target = path.clone();
        self.hand_over(Job {
            what: Saving::Settings,
            replaces: Some(target),
            work: Box::new(move || rbms_config::save(&config, &path).map_err(|e| format!("{} ({})", e, path.display()))),
        });
    }

    /// Write `replay` into the temp file `staged` already opened, and put it in place.
    pub(crate) fn save_replay(&self, file: String, replay: Replay, staged: StagedWrite) {
        self.hand_over(Job {
            what: Saving::Replay { file },
            replaces: None,
            work: Box::new(move || {
                let text = replay.to_ron().map_err(|e| e.to_string())?;
                let shown = staged.target().display().to_string();
                staged.finish(text.as_bytes()).map_err(|e| format!("{e} ({shown})"))
            }),
        });
    }

    /// Write what `contents` makes, made on the thread, to `path`, a file nothing else writes.
    pub(crate) fn save_text(&self, what: Saving, path: PathBuf, contents: impl FnOnce() -> String + Send + 'static) {
        self.hand_over(Job {
            what,
            replaces: None,
            work: Box::new(move || rbms_store::write_atomic(&path, &contents()).map_err(|e| format!("{e} ({})", path.display()))),
        });
    }

    /// Lay out the trace `source` makes, write its body and keep it beside the stored run `stored`
    /// names. A database that does not take it leaves the run's row as it is, and the trace goes on
    /// the side instead, in a file of its run the next opening of the database reads it in from
    /// ([`rbms_store::scoredb::ScoreDb::import_pending_runs`]); the failure is reported either way.
    pub(crate) fn keep_trace(&self, source: TraceSource, stored: StoredRun) {
        self.hand_over(Job {
            what: Saving::Trace { log_id: stored.log_id },
            replaces: None,
            work: Box::new(move || {
                let body = source.body().ok_or_else(|| TRACE_NOT_SERIALISED.to_string())?;
                let refused = match ScoreDb::open(&stored.db_path).and_then(|mut db| db.keep_trace(stored.log_id, &body)) {
                    Ok(()) => return Ok(()),
                    Err(e) => e.to_string(),
                };
                let path = stored.on_the_side;
                let run = PendingRun { trace: Some(body), ..stored.run };
                let text = run.to_ron().map_err(|e| format!("{refused}; not kept on the side either: {e}"))?;
                rbms_store::write_atomic(&path, &text).map_err(|e| format!("{refused}; not kept on the side either: {e} ({})", path.display()))?;
                Err(format!("{refused}; kept on the side until the score database takes it ({})", path.display()))
            }),
        });
    }

    /// Keep what `batch`, a batch of the traces made for runs stored without one, came to in the
    /// score database at `db_path`, in one write ([`ScoreDb::keep_backfilled_traces`]): its traces,
    /// the runs that will never get one, and its mark, which moves only from where the batch was
    /// made on, so a batch written after one that failed moves it no further. A run that has a trace
    /// by now keeps its own.
    pub(crate) fn keep_backfilled_traces(&self, db_path: PathBuf, batch: rbms_store::scoredb::BackfillBatch) {
        self.hand_over(Job {
            what: Saving::BackfilledTraces,
            replaces: None,
            work: Box::new(move || ScoreDb::open(&db_path).and_then(|mut db| db.keep_backfilled_traces(&batch)).map(|_| ()).map_err(|e| e.to_string())),
        });
    }

    /// Write `run`, a run the score database could not take, to `path`, a file nothing else writes,
    /// with the body of the trace `source` makes laid out here. `text` is the run as it stands
    /// without the trace, written when the trace makes no body.
    pub(crate) fn keep_run_on_the_side(&self, path: PathBuf, run: PendingRun, text: String, source: TraceSource) {
        self.save_text(Saving::PendingRun { file: path.clone() }, path, move || {
            source.body().and_then(|body| PendingRun { trace: Some(body), ..run }.to_ron().ok()).unwrap_or(text)
        });
    }

    /// A receiver answered once every write handed over so far is done, for a thread other than
    /// the frame thread to wait on: the records sync waits on it before it reads a replay a run has
    /// just kept, and the frame thread waits on nothing. `None` when there is nothing to wait for,
    /// the writes being done as they are handed over or the thread being gone.
    pub(crate) fn settled(&self) -> Option<Receiver<()>> {
        let tx = self.tx.as_ref()?;
        let (ack, done) = mpsc::channel();
        tx.send(Message::Flush(ack)).ok()?;
        Some(done)
    }

    /// Wait until every write handed over so far is done.
    pub(crate) fn flush(&self) {
        if let Some(tx) = &self.tx {
            let (ack, done) = mpsc::channel();
            if tx.send(Message::Flush(ack)).is_ok() {
                let _ = done.recv();
            }
        }
    }

    /// The writes that failed since the last look.
    pub(crate) fn failures(&self) -> Vec<SaveFailure> {
        self.failures.try_iter().collect()
    }
}

impl Drop for SaveWorker {
    /// Let the thread finish every write it was handed, then end it.
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The thread's loop: take what is waiting, skip a write a later one replaces, and do the rest in
/// order.
fn write_in_order(rx: &Receiver<Message>, failures: &Sender<SaveFailure>) {
    while let Ok(first) = rx.recv() {
        let mut batch = vec![first];
        batch.extend(rx.try_iter());
        let replaced = replaced_writes(&batch);
        for (at, message) in batch.into_iter().enumerate() {
            match message {
                Message::Job(_) if replaced.contains(&at) => {}
                Message::Job(job) => run_reporting(job, &|failure| {
                    let _ = failures.send(failure);
                }),
                Message::Flush(ack) => {
                    let _ = ack.send(());
                }
            }
        }
    }
}

/// The positions in `batch` of the writes a later write in it replaces. A flush between two writes
/// keeps the earlier one: the flush answers that what was handed over before it is on disk, so a
/// write after it replaces nothing before it.
fn replaced_writes(batch: &[Message]) -> Vec<usize> {
    let mut later: Vec<&Path> = Vec::new();
    let mut replaced = Vec::new();
    for (at, message) in batch.iter().enumerate().rev() {
        let path = match message {
            Message::Flush(_) => {
                later.clear();
                continue;
            }
            Message::Job(Job { replaces: Some(path), .. }) => path,
            Message::Job(_) => continue,
        };
        if later.contains(&path.as_path()) {
            replaced.push(at);
        } else {
            later.push(path);
        }
    }
    replaced
}

fn run_reporting(job: Job, report: &dyn Fn(SaveFailure)) {
    let Job { what, work, .. } = job;
    if let Err(error) = work() {
        report(SaveFailure { what, error });
    }
}

/// Say what failed on a write that could not be handed to the thread and has no frame to wait for.
fn report_now(failure: &SaveFailure) {
    notify(Level::Error, failure_line(failure));
}

/// What the message bus says of a write that failed.
fn failure_line(failure: &SaveFailure) -> String {
    match &failure.what {
        Saving::Settings => format!("settings save failed: {}", failure.error),
        Saving::Replay { .. } => format!("replay not saved: {}", failure.error),
        Saving::PendingRun { .. } => format!("run not kept: {}", failure.error),
        Saving::Trace { .. } => format!("trace of the run not stored: {}", failure.error),
        Saving::BackfilledTraces => format!("traces of earlier runs not stored: {}", failure.error),
    }
}

impl AppShared {
    /// Wait for every write handed to the save thread and take what it reports. The application
    /// calls this as its event loop ends, which on macOS (Quit in the menu, or the Dock) is the last
    /// it hears before the process exits: no `Drop` runs after it.
    pub(crate) fn finish_saves(&mut self) {
        self.saves.flush();
        self.poll_saves();
    }

    /// Take what the save thread reports: say what failed, and stop the records from offering a
    /// replay that was never written.
    pub(crate) fn poll_saves(&mut self) {
        for failure in self.saves.failures() {
            if let Saving::Replay { file } = &failure.what {
                self.scores.forget_replay(file);
                if let Some(db) = self.scoredb.as_mut()
                    && let Err(e) = db.clear_replay_refs(std::slice::from_ref(file))
                {
                    notify(Level::Warn, format!("unsaved replay still referenced: {e}"));
                }
            }
            notify(Level::Error, failure_line(&failure));
        }
    }
}

#[cfg(test)]
mod tests;
