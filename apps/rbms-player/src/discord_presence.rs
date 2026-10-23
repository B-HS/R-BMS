//! What is being played, shown on the player's Discord profile (Rich Presence), through the Discord
//! desktop app's local socket.
//!
//! The frame thread only says what the profile should show: when the screen changes
//! ([`follow_screen`], from [`crate::App::switch`]) and when the DISCORD STATUS row changes
//! ([`DiscordPresence::follow_setting`], a comparison of two booleans every frame). An activity that
//! is the one said last is not said again, so a run being played says nothing. Everything that
//! touches the socket runs on the `rbms-discord` thread ([`link`]): connecting, the handshake,
//! sending, connecting again with a growing wait while the app is not there, and keeping to the rate
//! the app takes updates at.
//!
//! On the way out the thread is told to stop and is waited for briefly, never joined: a pipe that
//! does not answer cannot hold the exit up. The app clears what a process showed once its socket
//! closes, so a thread cut short leaves nothing behind.
//!
//! A unit test build starts no thread: what the frame thread says is written down instead, so no
//! test of the application reaches a real Discord.

mod activity;
mod ipc;
mod link;
#[cfg(test)]
mod tests;

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use activity::Activity;
use activity::{ChartLine, CourseStage, PresenceView, RunKind, Screen, Starts, activity_for};
use ipc::Connect;
use link::Link;

use crate::AppShared;
use crate::stage::loading::LoadingTask;
use crate::stage::{Stage, StageId};

/// The Discord application R-BMS shows as. Not a secret: every client sends it.
pub(crate) const DISCORD_APPLICATION_ID: &str = "1563156237539999865";

/// The name of the thread that talks to the desktop app.
const DISCORD_THREAD_NAME: &str = "rbms-discord";

/// How long the way out waits for the thread to clear the profile before it goes on without it.
const EXIT_PATIENCE: Duration = Duration::from_millis(250);

/// What the frame thread tells the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    /// Show this activity, or nothing.
    Show(Option<Activity>),
    /// The DISCORD STATUS row was switched on or off.
    Enable(bool),
    /// The application is ending: clear the profile and stop.
    Stop,
}

/// Where the commands go.
enum Outlet {
    /// To the thread, and the signal it gives once it has stopped.
    Thread { tx: Sender<Command>, done: Receiver<()> },
    /// Written down, in a unit test build.
    #[cfg(test)]
    Recorder(Vec<Command>),
    /// Nowhere: the thread could not be started, or it has been stopped.
    Idle,
}

/// The handle the frame thread holds.
pub(crate) struct DiscordPresence {
    outlet: Outlet,
    /// The DISCORD STATUS value the thread was last told.
    enabled: bool,
    /// The activity the thread was last told.
    last: Option<Activity>,
    /// When the application, the run and the course started, which the time on the profile counts
    /// from.
    starts: Starts,
}

impl DiscordPresence {
    /// The presence the application runs with: a thread of its own in the game, a recorder in a
    /// unit test.
    pub(crate) fn for_this_build(enabled: bool) -> DiscordPresence {
        #[cfg(test)]
        return DiscordPresence::recording(enabled);
        #[cfg(not(test))]
        DiscordPresence::spawn_with(ipc::system_connector(), Instant::now, enabled)
    }

    /// A presence whose thread connects through `connector`, on the time `clock` reads.
    pub(crate) fn spawn_with(connector: Box<dyn Connect>, clock: fn() -> Instant, enabled: bool) -> DiscordPresence {
        let (tx, rx) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        let link = Link::new(connector, DISCORD_APPLICATION_ID, std::process::id(), enabled, clock());
        let spawned = std::thread::Builder::new().name(DISCORD_THREAD_NAME.into()).spawn(move || link.run(rx, clock, done_tx));
        let outlet = match spawned {
            Ok(_) => Outlet::Thread { tx, done },
            Err(_) => Outlet::Idle,
        };
        DiscordPresence::with_outlet(outlet, enabled)
    }

    /// A presence that writes down what it is told.
    #[cfg(test)]
    pub(crate) fn recording(enabled: bool) -> DiscordPresence {
        DiscordPresence::with_outlet(Outlet::Recorder(Vec::new()), enabled)
    }

    fn with_outlet(outlet: Outlet, enabled: bool) -> DiscordPresence {
        DiscordPresence { outlet, enabled, last: None, starts: Starts { app: unix_seconds_now(), run: None, course: None } }
    }

    /// What a recording presence has been told, oldest first.
    #[cfg(test)]
    pub(crate) fn recorded(&self) -> &[Command] {
        match &self.outlet {
            Outlet::Recorder(commands) => commands,
            _ => &[],
        }
    }

    /// Whether the presence has a thread it talks to.
    #[cfg(test)]
    pub(crate) fn has_thread(&self) -> bool {
        matches!(self.outlet, Outlet::Thread { .. })
    }

    fn send(&mut self, command: Command) {
        match &mut self.outlet {
            Outlet::Thread { tx, .. } => {
                if tx.send(command).is_err() {
                    self.outlet = Outlet::Idle;
                }
            }
            #[cfg(test)]
            Outlet::Recorder(commands) => commands.push(command),
            Outlet::Idle => {}
        }
    }

    /// Show `activity`, unless it is what was shown last.
    pub(crate) fn show(&mut self, activity: Activity) {
        if self.last.as_ref() == Some(&activity) {
            return;
        }
        self.last = Some(activity.clone());
        self.send(Command::Show(Some(activity)));
    }

    /// Follow the DISCORD STATUS row: tell the thread when it changed, and once it is on again, what
    /// to show.
    pub(crate) fn follow_setting(&mut self, enabled: bool) {
        if enabled == self.enabled {
            return;
        }
        self.enabled = enabled;
        self.send(Command::Enable(enabled));
        if let Some(activity) = self.last.clone().filter(|_| enabled) {
            self.send(Command::Show(Some(activity)));
        }
    }

    /// Move the starts the time is counted from for a screen at `now`: a run starts each time the
    /// play screen is arrived at, and a course when one is first seen running.
    fn mark(&mut self, run_begins: bool, in_course: bool, now: u64) -> Starts {
        if run_begins {
            self.starts.run = Some(now);
        }
        self.starts.course = match (in_course, self.starts.course) {
            (false, _) => None,
            (true, Some(start)) => Some(start),
            (true, None) => Some(now),
        };
        self.starts
    }

    /// Tell the thread to clear the profile and stop, and wait for it no longer than the way out
    /// can.
    pub(crate) fn stop(&mut self) {
        self.stop_within(EXIT_PATIENCE);
    }

    /// [`DiscordPresence::stop`], waiting up to `patience`. Whether the thread said it stopped.
    pub(crate) fn stop_within(&mut self, patience: Duration) -> bool {
        self.send(Command::Stop);
        match std::mem::replace(&mut self.outlet, Outlet::Idle) {
            Outlet::Thread { done, .. } => done.recv_timeout(patience).is_ok(),
            #[cfg(test)]
            recorder @ Outlet::Recorder(_) => {
                self.outlet = recorder;
                true
            }
            Outlet::Idle => false,
        }
    }
}

/// The time now in Unix seconds, which the profile counts elapsed time from.
fn unix_seconds_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or_default()
}

/// Tell the profile what `stage` shows. Called once a screen has been arrived at.
pub(crate) fn follow_screen(stage: &Stage, shared: &mut AppShared) {
    let starts = shared.discord.mark(stage.id() == StageId::Play, shared.course_run.is_some(), unix_seconds_now());
    let activity = activity_for(&view_of(stage, shared), &starts);
    shared.discord.show(activity);
}

/// The level of the difficulty table the browser has open, which is where a chart picked now was
/// picked from, and nothing in a course.
fn open_table_level(shared: &AppShared) -> String {
    match shared.course_run {
        Some(_) => String::new(),
        None => crate::stage::decide::table_of_the_open_folder(shared).1,
    }
}

/// What a parsed chart is, as the profile names it.
fn model_line<'a>(model: &'a rbms_model::Model, table_level: String) -> ChartLine<'a> {
    ChartLine {
        title: &model.meta.title,
        subtitle: &model.meta.subtitle,
        mode: Some(model.mode),
        difficulty: model.meta.difficulty,
        level: model.meta.play_level.clone(),
        table_level,
    }
}

/// What a chart of the library is, as the profile names it.
fn entry_line(entry: &rbms_library::SongEntry, table_level: String) -> ChartLine<'_> {
    ChartLine { title: &entry.title, subtitle: &entry.subtitle, mode: Some(entry.mode), difficulty: entry.difficulty, level: entry.level.clone(), table_level }
}

/// How the run on the play screen is being played.
fn run_kind(play: &crate::stage::PlayState, shared: &AppShared) -> RunKind {
    if play.practice.is_some() {
        return RunKind::Practice;
    }
    if shared.replay.is_some() {
        return RunKind::Replay;
    }
    if shared.run_plays_itself() { RunKind::Autoplay } else { RunKind::ByHand }
}

/// The screen as the profile sees it.
pub(crate) fn view_of<'a>(stage: &'a Stage, shared: &'a AppShared) -> PresenceView<'a> {
    let (screen, part_of_course) = match stage {
        Stage::Select(_) | Stage::Folders(_) | Stage::Tables(_) => (Screen::Browsing, false),
        Stage::Settings(_) | Stage::KeyConfig(_) | Stage::SkinMenu(_) | Stage::PlayMenu(_) => (Screen::Menus, false),
        Stage::Loading(loading) => match loading.task() {
            LoadingTask::Scan { .. } => (Screen::Scanning, false),
            LoadingTask::Song(index) => {
                let chart = shared.library.songs().get(*index).map(|entry| entry_line(entry, open_table_level(shared))).unwrap_or_default();
                (Screen::Loading(chart), true)
            }
            LoadingTask::Assets(assets) => (Screen::Loading(model_line(assets.model(), open_table_level(shared))), true),
        },
        Stage::Decide(decide) => {
            let (chart, table_level) = decide.chart_and_table_level();
            let level = Some(chart.level).filter(|level| *level > 0).map(|level| level.to_string()).unwrap_or_default();
            let line = ChartLine {
                title: &chart.title,
                subtitle: &chart.subtitle,
                mode: chart.mode,
                difficulty: chart.difficulty,
                level,
                table_level: table_level.to_string(),
            };
            (Screen::Loading(line), true)
        }
        Stage::Play(play) => (Screen::Playing { chart: model_line(play.session.model(), open_table_level(shared)), kind: run_kind(play, shared) }, true),
        Stage::Practice(_) => match &shared.practice_chart {
            Some(chart) => (Screen::PracticePanel(model_line(chart.model(), String::new())), false),
            None => (Screen::Menus, false),
        },
        Stage::Result(result) => {
            let view = result.view();
            let chart = shared.course_run.as_ref().and_then(|run| run.course.charts.get(played_stage(run).saturating_sub(1)));
            let entry = chart.and_then(|chart| crate::course_ui::library_index(&shared.library, chart)).and_then(|index| shared.library.songs().get(index));
            let chart = entry.map(|entry| entry_line(entry, String::new()));
            (Screen::Result { title: &view.title, clear_label: view.clear_label, mode_label: view.mode_label, chart }, true)
        }
        Stage::CourseResult(state) => (Screen::CourseResult { name: state.course_name(), cleared: state.cleared() }, false),
    };
    let number = |run: &rbms_course::CourseRun| match stage {
        Stage::Result(_) => played_stage(run),
        _ => run.stage_number(),
    };
    let course = shared.course_run.as_ref().filter(|_| part_of_course).map(|run| CourseStage {
        name: &run.course.name,
        number: number(run),
        count: run.course.stage_count(),
    });
    PresenceView { screen, course }
}

/// The one-based number of the course stage a result screen reports: the run has moved past a stage
/// it survived, and stays on one it failed.
fn played_stage(run: &rbms_course::CourseRun) -> usize {
    run.failed_at.map(|failed| failed + 1).unwrap_or(run.index).clamp(1, run.course.stage_count().max(1))
}
