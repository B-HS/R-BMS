//! What a skin's browser shows of the chart under the cursor that only a parse of the chart can
//! tell: how its notes are spread second by second and how its tempo moves, for the judgement graph
//! (`type` 0) and the tempo graph; how many notes there are of each kind and on each lane; the tempo
//! most of them are played at; and its NOTES RADAR.
//!
//! The reference's browser reads all of this from the chart information its song database keeps
//! (`SongInformation`). The song database here keeps the radar and nothing else of it, so the rest
//! is measured from the chart itself once the cursor has rested on it, which is when the browser
//! reads the chart's detail too.
//!
//! The chart is parsed on one worker thread, which the frame loop never waits for. A request the
//! cursor has already left is dropped by the worker when a newer one reached it first, and its
//! answer is dropped by the browser when it comes back after a newer request was made, so turning
//! the wheel through a long list costs one parse at a time and shows only the chart under the
//! cursor.
//!
//! The radar the database holds is shown from the moment the cursor rests, without waiting for the
//! parse, and is shown for a chart whose file cannot be read. A chart whose radar the database does
//! not hold, or holds as measured under older definitions, is measured by the same parse, and the
//! browser keeps what was measured in the database so the next visit reads it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use rbms_chart::{RADAR_AXIS_COUNT, radar_display};
use rbms_library::SongEntry;
use rbms_library::songdb::{RadarRow, SongDb};
use rbms_model::Mode;
use rbms_render::FrameSeries;
use rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT;

use crate::AppShared;
use crate::skin_host::chart::{ChartMeta, LaneNotes};
use crate::skin_host::overview::{ChartOverview, lane_notes};

/// What one parse of a chart found.
struct Analysed {
    overview: ChartOverview,
    lanes: Option<LaneNotes>,
    radar: RadarRow,
}

/// One request to the worker: the chart to parse.
struct Job {
    /// Which request this is. An answer is taken in only while it answers the newest request.
    id: u64,
    path: PathBuf,
}

/// What the worker found of one chart, or `None` in `analysed` for a file that is not there or is
/// not a chart.
struct Reply {
    id: u64,
    analysed: Option<Analysed>,
}

/// The two ends the frame loop holds of the worker thread.
struct Worker {
    jobs: Sender<Job>,
    replies: Receiver<Reply>,
}

/// What has come of the newest request.
enum Found {
    /// The worker has not answered yet.
    Waiting,
    /// The file is not there or is not a chart.
    Unreadable,
    Read(Box<Analysed>),
}

/// The chart the newest request was made for.
struct Asked {
    id: u64,
    /// Its place in the library, and its file: a library that was replaced can hold another chart
    /// at the same place.
    index: usize,
    path: PathBuf,
    mode: Mode,
    /// The radar the database held for it when it was asked for, measured under the current
    /// definitions.
    stored: Option<RadarRow>,
    found: Found,
}

impl Asked {
    /// What the parse found, once it has.
    fn analysed(&self) -> Option<&Analysed> {
        if let Found::Read(analysed) = &self.found { Some(analysed) } else { None }
    }
}

/// What the parse of the chart under the browser's cursor found, kept until the cursor moves on.
#[derive(Default)]
pub(super) struct ChartAnalysis {
    /// The worker thread, started by the first request.
    worker: Option<Worker>,
    requests: u64,
    asked: Option<Asked>,
    /// Whether the chart asked for is the one under the cursor now, so it is shown. A cursor that
    /// has moved and not yet rested shows nothing of the chart it left.
    shown: bool,
}

impl ChartAnalysis {
    /// Follow the cursor: take in whatever the worker has found since the last frame, and ask for the
    /// chart the cursor has rested on when it is not the one asked for already.
    ///
    /// `settled` is the place in the library of the chart the cursor has rested on, or `None` when it
    /// rests on anything else. A browser no skin draws asks for nothing, as nothing would show it.
    pub(super) fn follow(&mut self, shared: &AppShared, settled: Option<usize>) {
        let drawn = shared.has_skin_document(SKIN_TYPE_MUSIC_SELECT);
        let focus =
            settled.filter(|index| drawn && Some(*index) == shared.focused_song_index()).and_then(|index| Some((index, shared.library.songs().get(index)?)));
        self.follow_focus(shared.song_db.as_ref(), focus);
    }

    /// [`Self::follow`] with the chart under the cursor already looked up, and the song database it
    /// is kept in when there is one.
    fn follow_focus(&mut self, db: Option<&SongDb>, focus: Option<(usize, &SongEntry)>) {
        self.take_replies(db);
        self.shown = focus.is_some();
        let Some((index, entry)) = focus else {
            return;
        };
        if self.asked.as_ref().is_some_and(|asked| asked.index == index && asked.path == entry.path) {
            return;
        }
        self.ask(db, index, entry);
    }

    /// Make the request for `entry`, which replaces whatever was asked before.
    fn ask(&mut self, db: Option<&SongDb>, index: usize, entry: &SongEntry) {
        self.requests += 1;
        let id = self.requests;
        let stored = db.and_then(|db| crate::library::stored_radar(db, &entry.path));
        let worker = self.worker.get_or_insert_with(spawn_worker);
        let found = match worker.jobs.send(Job { id, path: entry.path.clone() }) {
            Ok(()) => Found::Waiting,
            Err(_) => {
                self.worker = None;
                Found::Unreadable
            }
        };
        self.asked = Some(Asked { id, index, path: entry.path.clone(), mode: entry.mode, stored, found });
    }

    /// Take in every answer the worker has sent since the last frame. A worker that has gone away
    /// leaves the request it had unanswered, as a chart that could not be read.
    fn take_replies(&mut self, db: Option<&SongDb>) {
        loop {
            let Some(worker) = &self.worker else {
                return;
            };
            match worker.replies.try_recv() {
                Ok(reply) => self.take(db, reply),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.worker_gone();
                    return;
                }
            }
        }
    }

    /// Take in one answer, when it answers the newest request. A radar measured of a chart the
    /// database held none of under the current definitions is kept there.
    fn take(&mut self, db: Option<&SongDb>, reply: Reply) {
        let Some(asked) = self.asked.as_mut().filter(|asked| asked.id == reply.id) else {
            return;
        };
        asked.found = match reply.analysed {
            Some(analysed) => {
                if asked.stored.is_none()
                    && let Some(db) = db
                {
                    crate::library::keep_radar(db, &asked.path, &analysed.radar);
                }
                Found::Read(Box::new(analysed))
            }
            None => Found::Unreadable,
        };
    }

    /// The worker is gone: the request it had will not be answered, and the next request starts
    /// another.
    fn worker_gone(&mut self) {
        self.worker = None;
        if let Some(asked) = self.asked.as_mut().filter(|asked| matches!(asked.found, Found::Waiting)) {
            asked.found = Found::Unreadable;
        }
    }

    /// The request for the chart under the cursor, while it is shown.
    fn shown(&self) -> Option<&Asked> {
        self.asked.as_ref().filter(|_| self.shown)
    }

    /// The NOTES RADAR of the chart under the cursor as shown, 0 to 200 on each axis: the one the
    /// database holds, or else the one the parse measured. `None` for a chart whose mode has none,
    /// and before either is known.
    pub(super) fn notes_radar(&self) -> Option<[f32; RADAR_AXIS_COUNT]> {
        let asked = self.shown()?;
        let radar = asked.stored.or_else(|| asked.analysed().map(|analysed| analysed.radar))?;
        radar.raw.map(|raw| radar_display(&raw, &asked.mode))
    }

    /// `chart`, the chart under the cursor, with what the parse found of it and its radar.
    pub(super) fn describe<'a>(&self, chart: ChartMeta<'a>) -> ChartMeta<'a> {
        let notes_radar = self.notes_radar();
        let Some(analysed) = self.shown().and_then(Asked::analysed) else {
            return ChartMeta { notes_radar, ..chart };
        };
        let overview = &analysed.overview;
        ChartMeta {
            note_counts: Some(overview.note_counts),
            main_bpm: Some(overview.main_bpm).filter(|bpm| *bpm > 0.0).or(chart.main_bpm),
            lane_notes: analysed.lanes,
            notes_radar,
            ..chart
        }
    }

    /// The two series the graphs of the chart under the cursor plot, or none before the parse has
    /// found them.
    pub(super) fn series(&self) -> FrameSeries<'_> {
        self.shown().and_then(Asked::analysed).map_or_else(FrameSeries::default, |analysed| analysed.overview.series())
    }

    /// Wait for the worker to answer the newest request, for the tests that read what it found.
    #[cfg(test)]
    fn settle(&mut self, db: Option<&SongDb>) {
        while self.asked.as_ref().is_some_and(|asked| matches!(asked.found, Found::Waiting)) {
            let Some(worker) = &self.worker else {
                return;
            };
            match worker.replies.recv() {
                Ok(reply) => self.take(db, reply),
                Err(_) => self.worker_gone(),
            }
        }
    }
}

/// Starts the thread that parses charts, which runs until the browser that started it is gone.
fn spawn_worker() -> Worker {
    let (jobs, incoming) = channel();
    let (outgoing, replies) = channel();
    std::thread::spawn(move || analyse_requests(&incoming, &outgoing));
    Worker { jobs, replies }
}

/// The worker's loop: takes the newest request there is, drops the ones before it, and answers it.
fn analyse_requests(jobs: &Receiver<Job>, replies: &Sender<Reply>) {
    while let Ok(mut job) = jobs.recv() {
        while let Ok(newer) = jobs.try_recv() {
            job = newer;
        }
        if replies.send(Reply { id: job.id, analysed: analyse(&job.path) }).is_err() {
            return;
        }
    }
}

/// Parses the chart at `path`, whatever format it is written in, and measures it.
fn analyse(path: &Path) -> Option<Analysed> {
    let bytes = std::fs::read(path).ok()?;
    let model = crate::app_play::chart_model(&bytes, &path.to_string_lossy())?;
    Some(Analysed { overview: ChartOverview::of_model(&model), lanes: lane_notes(&model), radar: RadarRow::measure(&model, &model.mode) })
}

#[cfg(test)]
mod tests;
