//! The connection to the desktop app, as the `rbms-discord` thread keeps it: connecting and
//! connecting again, the handshake, the activity sent, and how often it may be sent.
//!
//! Every step takes the instant it runs at, so a test drives the whole of it on its own clock
//! without sleeping. Only [`Link::run`] reads a real clock.

use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use super::Command;
use super::activity::Activity;
use super::ipc::{self, Connect, EVT_ERROR, EVT_READY, IpcStream, Opcode};

/// The first wait before connecting again, doubled after every failure.
pub(crate) const RECONNECT_FIRST: Duration = Duration::from_millis(500);

/// The longest wait before connecting again, and the one given straight away when the app refuses
/// the connection for a reason another attempt cannot change.
pub(crate) const RECONNECT_MAX: Duration = Duration::from_secs(60);

/// How long a read waits for the app before the connection is taken for broken, where the stream
/// can be given a limit (the Unix socket).
pub(crate) const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// The window the activity updates are counted in, and how many it may hold.
pub(crate) const RATE_WINDOW: Duration = Duration::from_secs(20);
pub(crate) const RATE_LIMIT: usize = 5;

/// How long the activity shown goes without being sent again. Sending it again is how a connection
/// the app dropped is noticed while the screen stays the same, so the profile comes back once the
/// app does.
pub(crate) const REFRESH_INTERVAL: Duration = Duration::from_secs(20);

/// The close codes after which the next attempt waits the longest: an unknown application, a
/// protocol version the app does not speak, and too many updates.
pub(crate) const CLOSE_INVALID_CLIENT_ID: u64 = 4000;
pub(crate) const CLOSE_RATE_LIMITED: u64 = 4002;
pub(crate) const CLOSE_INVALID_VERSION: u64 = 4004;

/// The waits between attempts to connect: from [`RECONNECT_FIRST`], doubling to [`RECONNECT_MAX`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Backoff {
    next: Duration,
}

impl Backoff {
    pub(crate) fn new() -> Backoff {
        Backoff { next: RECONNECT_FIRST }
    }

    /// The wait after a failure, doubling the one after it.
    pub(crate) fn take(&mut self) -> Duration {
        let wait = self.next;
        self.next = (wait * 2).min(RECONNECT_MAX);
        wait
    }

    /// The longest wait, now and after.
    pub(crate) fn give_up(&mut self) -> Duration {
        self.next = RECONNECT_MAX;
        RECONNECT_MAX
    }

    /// Back to the first wait, after a connection that worked.
    pub(crate) fn reset(&mut self) {
        self.next = RECONNECT_FIRST;
    }
}

/// The updates sent in the last [`RATE_WINDOW`], so no more than [`RATE_LIMIT`] go in one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RateLimit {
    sent_at: VecDeque<Instant>,
}

impl RateLimit {
    /// When the next update may go: `now`, or when the oldest in the window leaves it.
    pub(crate) fn next_free(&self, now: Instant) -> Instant {
        let mut in_window = self.sent_at.iter().filter(|at| **at + RATE_WINDOW > now);
        let oldest = in_window.next().copied();
        match (oldest, 1 + in_window.count()) {
            (Some(oldest), count) if count >= RATE_LIMIT => oldest + RATE_WINDOW,
            _ => now,
        }
    }

    /// Count an update sent at `now`.
    pub(crate) fn record(&mut self, now: Instant) {
        self.sent_at.retain(|at| *at + RATE_WINDOW > now);
        self.sent_at.push_back(now);
    }
}

/// Where the connection is.
enum State {
    /// Turned off: nothing is connected and nothing is tried.
    Off,
    /// Not connected, and not to be tried before `retry_at`.
    Waiting { retry_at: Instant },
    /// Connected; `sent` is what the app shows for this process, `sent_at` when it was sent, and
    /// `accepted` whether the app took it, as only an accepted activity is sent again to keep it up.
    Connected { stream: Box<dyn IpcStream>, sent: Option<Activity>, sent_at: Instant, nonce: u64, accepted: bool },
}

/// How the app answered a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Accepted,
    Refused,
}

/// Why a connection ended.
#[derive(Debug)]
enum Broken {
    /// A read, a write or a frame failed.
    Io,
    /// The app closed it, with this code when it gave one.
    Closed(Option<u64>),
}

impl From<io::Error> for Broken {
    fn from(_: io::Error) -> Broken {
        Broken::Io
    }
}

/// Whether the thread goes on after a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Stop,
}

/// The connection and everything that decides what goes down it.
pub(crate) struct Link {
    connector: Box<dyn Connect>,
    client_id: &'static str,
    pid: u32,
    enabled: bool,
    wanted: Option<Activity>,
    state: State,
    backoff: Backoff,
    limit: RateLimit,
}

impl Link {
    /// A link for the application `client_id`, sent as process `pid`, switched on or off.
    pub(crate) fn new(connector: Box<dyn Connect>, client_id: &'static str, pid: u32, enabled: bool, now: Instant) -> Link {
        let state = if enabled { State::Waiting { retry_at: now } } else { State::Off };
        Link { connector, client_id, pid, enabled, wanted: None, state, backoff: Backoff::new(), limit: RateLimit::default() }
    }

    /// Whether a stream to the app is open.
    #[cfg(test)]
    pub(crate) fn is_connected(&self) -> bool {
        matches!(self.state, State::Connected { .. })
    }

    /// When the next connection is tried, while one is waited for.
    #[cfg(test)]
    pub(crate) fn retry_at(&self) -> Option<Instant> {
        match self.state {
            State::Waiting { retry_at } => Some(retry_at),
            _ => None,
        }
    }

    /// What the app was last told to show, while connected.
    #[cfg(test)]
    pub(crate) fn shown(&self) -> Option<&Activity> {
        match &self.state {
            State::Connected { sent, .. } => sent.as_ref(),
            _ => None,
        }
    }

    /// Take one command. Turning off clears what the app shows and closes the stream, and so does
    /// stopping.
    pub(crate) fn handle(&mut self, command: Command, now: Instant) -> Flow {
        match command {
            Command::Show(activity) => self.wanted = activity,
            Command::Enable(true) => {
                self.enabled = true;
                if matches!(self.state, State::Off) {
                    self.backoff.reset();
                    self.state = State::Waiting { retry_at: now };
                }
            }
            Command::Enable(false) => {
                self.enabled = false;
                self.close(now);
            }
            Command::Stop => {
                self.close(now);
                return Flow::Stop;
            }
        }
        Flow::Continue
    }

    /// Clear what the app shows for this process, if it is connected, and drop the stream.
    fn close(&mut self, now: Instant) {
        if let State::Connected { mut stream, nonce, .. } = std::mem::replace(&mut self.state, State::Off) {
            self.limit.record(now);
            let _ = exchange(stream.as_mut(), &ipc::set_activity_body(self.pid, None, nonce), nonce);
        }
    }

    /// When there is next something to do without a command: a connection to try, an update the
    /// rate limit held back, or the activity to send again.
    pub(crate) fn next_due(&self, now: Instant) -> Option<Instant> {
        match &self.state {
            State::Off => None,
            State::Waiting { retry_at } => (self.enabled && self.wanted.is_some()).then_some(*retry_at),
            State::Connected { sent, .. } if *sent != self.wanted => Some(self.limit.next_free(now)),
            State::Connected { sent: Some(_), sent_at, accepted: true, .. } => Some((*sent_at + REFRESH_INTERVAL).max(self.limit.next_free(now))),
            State::Connected { .. } => None,
        }
    }

    /// Do what is due at `now`: connect when a connection is waited for and there is something to
    /// show, then send what is to be shown when it is not shown yet or is due again.
    pub(crate) fn work(&mut self, now: Instant) {
        if let State::Waiting { retry_at } = self.state
            && self.enabled
            && self.wanted.is_some()
            && now >= retry_at
        {
            self.state = match self.open() {
                Ok(stream) => State::Connected { stream, sent: None, sent_at: now, nonce: 0, accepted: false },
                Err(broken) => State::Waiting { retry_at: now + self.wait_after(&broken) },
            };
        }
        let due = match &self.state {
            State::Connected { sent, sent_at, accepted, .. } => {
                let refresh = *accepted && sent.is_some() && now >= *sent_at + REFRESH_INTERVAL;
                (*sent != self.wanted || refresh) && self.limit.next_free(now) <= now
            }
            _ => false,
        };
        if due {
            self.send(now);
        }
    }

    /// Connect and shake hands.
    fn open(&mut self) -> Result<Box<dyn IpcStream>, Broken> {
        let mut stream = self.connector.connect()?;
        stream.set_read_limit(Some(REPLY_TIMEOUT))?;
        ipc::write_message(stream.as_mut(), Opcode::Handshake, ipc::handshake_body(self.client_id).as_bytes())?;
        let ready = read_frame(stream.as_mut())?;
        match ready.evt.as_deref() {
            Some(EVT_READY) => Ok(stream),
            _ => Err(Broken::Io),
        }
    }

    /// Send the wanted activity, or the clearing of it, and wait for the answer.
    fn send(&mut self, now: Instant) {
        let State::Connected { stream, sent, sent_at, nonce, accepted } = &mut self.state else {
            return;
        };
        *nonce += 1;
        self.limit.record(now);
        let body = ipc::set_activity_body(self.pid, self.wanted.as_ref(), *nonce);
        match exchange(stream.as_mut(), &body, *nonce) {
            Ok(answer) => {
                *sent = self.wanted.clone();
                *sent_at = now;
                *accepted = answer == Answer::Accepted;
                if *accepted {
                    self.backoff.reset();
                }
            }
            Err(broken) => self.state = State::Waiting { retry_at: now + self.wait_after(&broken) },
        }
    }

    /// The wait before the next attempt after a connection ended so.
    fn wait_after(&mut self, broken: &Broken) -> Duration {
        match broken {
            Broken::Closed(Some(CLOSE_INVALID_CLIENT_ID | CLOSE_RATE_LIMITED | CLOSE_INVALID_VERSION)) => self.backoff.give_up(),
            Broken::Closed(_) | Broken::Io => self.backoff.take(),
        }
    }

    /// Wait for commands and act on them until told to stop, then say so on `done`. `clock` is the
    /// time; a closed channel is a stop.
    pub(crate) fn run(mut self, commands: Receiver<Command>, clock: fn() -> Instant, done: Sender<()>) {
        loop {
            let now = clock();
            self.work(now);
            let command = match self.next_due(now) {
                Some(at) => match commands.recv_timeout(at.saturating_duration_since(now)) {
                    Ok(command) => command,
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => Command::Stop,
                },
                None => commands.recv().unwrap_or(Command::Stop),
            };
            if self.handle(command, clock()) == Flow::Stop {
                let _ = done.send(());
                return;
            }
        }
    }
}

/// Read frames until one that is not a ping, answering each ping with a pong. A close ends the
/// connection.
fn read_frame(stream: &mut dyn IpcStream) -> Result<ipc::Reply, Broken> {
    loop {
        let message = ipc::read_message(stream)?;
        match message.opcode {
            Opcode::Frame => return Ok(ipc::read_reply(&message.body)),
            Opcode::Ping => ipc::write_message(stream, Opcode::Pong, &message.body)?,
            Opcode::Pong => {}
            Opcode::Close => return Err(Broken::Closed(ipc::read_reply(&message.body).code)),
            Opcode::Handshake => return Err(Broken::Io),
        }
    }
}

/// Send one command and read until its answer, passing over the frames that answer something else.
/// A refusal (`evt: ERROR`) is an answer too: the same command would be refused again.
fn exchange(stream: &mut dyn IpcStream, body: &str, nonce: u64) -> Result<Answer, Broken> {
    ipc::write_message(stream, Opcode::Frame, body.as_bytes())?;
    let nonce = nonce.to_string();
    loop {
        let reply = read_frame(stream)?;
        if reply.nonce.as_deref() == Some(nonce.as_str()) {
            return Ok(if reply.evt.as_deref() == Some(EVT_ERROR) { Answer::Refused } else { Answer::Accepted });
        }
    }
}
