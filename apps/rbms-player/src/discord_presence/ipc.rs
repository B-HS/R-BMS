//! The wire the Discord desktop app is spoken to over: its local socket (a named pipe on Windows),
//! the frames that go down it, and the few messages the presence sends.
//!
//! A frame is an opcode and a length, both little-endian `u32`, and then that many bytes of JSON.
//! A frame is written in one write, as the official `discord-rpc` writes it: a pipe that takes the
//! head and the body in two writes can mix them up with another writer's.

#[cfg(any(unix, test))]
use std::ffi::OsString;
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::activity::Activity;

/// The size of a frame's head: the opcode and the length of the body.
pub(crate) const HEADER_LEN: usize = 8;

/// The longest body a frame is read with, as `discord-rpc` caps its buffer. A longer one is taken
/// for a broken stream.
pub(crate) const MAX_FRAME_BYTES: usize = 64 * 1024;

/// The version of the RPC protocol the handshake asks for.
const RPC_VERSION: u32 = 1;

/// How many sockets the desktop app may listen on: `discord-ipc-0` to `discord-ipc-9`, one per app
/// running (Stable, PTB, Canary).
const SOCKET_SLOTS: u32 = 10;

/// The name every socket of the desktop app starts with, its number following.
const SOCKET_STEM: &str = "discord-ipc-";

/// The variables a Unix socket's folder is looked up in, the first one set winning.
#[cfg(any(unix, test))]
const UNIX_BASE_VARS: [&str; 4] = [XDG_RUNTIME_DIR, "TMPDIR", "TMP", "TEMP"];

/// The variable that names the per-user runtime folder, under which a sandboxed app has its own.
#[cfg(any(unix, test))]
const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";

/// The folder the sockets are in when no variable names one.
#[cfg(any(unix, test))]
const UNIX_FALLBACK_DIR: &str = "/tmp";

/// The folders under the runtime folder the Flatpak and the Snap builds of the desktop app put their
/// sockets in.
#[cfg(any(unix, test))]
const SANDBOXED_SOCKET_DIRS: [&str; 2] = ["app/com.discordapp.Discord", "snap.discord"];

/// What a Windows pipe of the desktop app is named before its stem.
#[cfg(any(windows, test))]
const WINDOWS_PIPE_PREFIX: &str = r"\\?\pipe\";

/// What a frame is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opcode {
    Handshake,
    Frame,
    Close,
    Ping,
    Pong,
}

impl Opcode {
    /// The number the opcode is written as.
    pub(crate) fn code(self) -> u32 {
        match self {
            Opcode::Handshake => 0,
            Opcode::Frame => 1,
            Opcode::Close => 2,
            Opcode::Ping => 3,
            Opcode::Pong => 4,
        }
    }

    /// The opcode a number is, if it is one.
    pub(crate) fn from_code(code: u32) -> Option<Opcode> {
        [Opcode::Handshake, Opcode::Frame, Opcode::Close, Opcode::Ping, Opcode::Pong].into_iter().find(|opcode| opcode.code() == code)
    }
}

/// One frame read off the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    pub(crate) opcode: Opcode,
    pub(crate) body: Vec<u8>,
}

/// Why the bytes read are not a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameError {
    /// The head names a body longer than [`MAX_FRAME_BYTES`].
    TooLong(u32),
    /// The head names an opcode the protocol does not have.
    UnknownOpcode(u32),
}

impl From<FrameError> for io::Error {
    fn from(error: FrameError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, format!("{error:?}"))
    }
}

/// The bytes of one frame, head and body in one buffer so they go down in one write.
pub(crate) fn encode(opcode: Opcode, body: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(HEADER_LEN + body.len());
    frame.extend_from_slice(&opcode.code().to_le_bytes());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(body);
    frame
}

/// The opcode and the length of the body a frame's head names, or why it is not a frame.
fn read_head(head: [u8; HEADER_LEN]) -> Result<(Opcode, usize), FrameError> {
    let [o0, o1, o2, o3, l0, l1, l2, l3] = head;
    let code = u32::from_le_bytes([o0, o1, o2, o3]);
    let length = u32::from_le_bytes([l0, l1, l2, l3]);
    let opcode = Opcode::from_code(code).ok_or(FrameError::UnknownOpcode(code))?;
    match usize::try_from(length) {
        Ok(len) if len <= MAX_FRAME_BYTES => Ok((opcode, len)),
        _ => Err(FrameError::TooLong(length)),
    }
}

/// The first frame in `bytes` and how many bytes it took, `None` while the bytes stop before its
/// end. The stream is read a frame at a time by [`read_message`]; this is the same reading of a
/// buffer, for the tests.
#[cfg(test)]
pub(crate) fn decode(bytes: &[u8]) -> Result<Option<(Message, usize)>, FrameError> {
    let Some(head) = bytes.first_chunk::<HEADER_LEN>() else {
        return Ok(None);
    };
    let (opcode, len) = read_head(*head)?;
    let end = HEADER_LEN + len;
    Ok(bytes.get(HEADER_LEN..end).map(|body| (Message { opcode, body: body.to_vec() }, end)))
}

/// Read one whole frame off `stream`. The end of the stream is an error: the app has closed it.
pub(crate) fn read_message(stream: &mut dyn Read) -> io::Result<Message> {
    let mut head = [0u8; HEADER_LEN];
    stream.read_exact(&mut head)?;
    let (opcode, len) = read_head(head)?;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    Ok(Message { opcode, body })
}

/// Write one frame to `stream` in one write.
pub(crate) fn write_message(stream: &mut dyn Write, opcode: Opcode, body: &[u8]) -> io::Result<()> {
    stream.write_all(&encode(opcode, body))?;
    stream.flush()
}

#[derive(Serialize)]
struct Handshake<'a> {
    v: u32,
    client_id: &'a str,
}

/// The body of the handshake that opens a connection for the application `client_id`.
pub(crate) fn handshake_body(client_id: &str) -> String {
    serde_json::to_string(&Handshake { v: RPC_VERSION, client_id }).unwrap_or_default()
}

#[derive(Serialize)]
struct SetActivity<'a> {
    cmd: &'static str,
    args: SetActivityArgs<'a>,
    nonce: String,
}

#[derive(Serialize)]
struct SetActivityArgs<'a> {
    pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    activity: Option<&'a Activity>,
}

/// The command `SET_ACTIVITY` is sent as.
const SET_ACTIVITY: &str = "SET_ACTIVITY";

/// The body that shows `activity` as process `pid`'s, or clears what it shows when there is none.
pub(crate) fn set_activity_body(pid: u32, activity: Option<&Activity>, nonce: u64) -> String {
    serde_json::to_string(&SetActivity { cmd: SET_ACTIVITY, args: SetActivityArgs { pid, activity }, nonce: nonce.to_string() }).unwrap_or_default()
}

/// What a frame from the app says, as far as the presence reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    /// The event a dispatch is (`READY`), or `ERROR` for a command the app refused.
    pub(crate) evt: Option<String>,
    /// The nonce of the command this answers.
    pub(crate) nonce: Option<String>,
    /// The code a refusal or a close gives.
    pub(crate) code: Option<u64>,
}

/// The event the app answers a handshake with.
pub(crate) const EVT_READY: &str = "READY";

/// The event the app answers a command it refused with.
pub(crate) const EVT_ERROR: &str = "ERROR";

/// Read a frame's body. A body that is not JSON reads as one that says nothing.
pub(crate) fn read_reply(body: &[u8]) -> Reply {
    let value: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    let code = value.get("code").or_else(|| value.get("data").and_then(|data| data.get("code"))).and_then(Value::as_u64);
    Reply { evt: text("evt"), nonce: text("nonce"), code }
}

/// A stream to the desktop app: the socket, the pipe, or what a test stands in for them.
pub(crate) trait IpcStream: Read + Write + Send {
    /// Give up on a read after `limit`. A stream that cannot does nothing.
    fn set_read_limit(&self, _limit: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
impl IpcStream for UnixStream {
    fn set_read_limit(&self, limit: Option<Duration>) -> io::Result<()> {
        self.set_read_timeout(limit)
    }
}

#[cfg(windows)]
impl IpcStream for std::fs::File {}

/// What opens a stream to the desktop app.
pub(crate) trait Connect: Send {
    fn connect(&mut self) -> io::Result<Box<dyn IpcStream>>;
}

/// Open the first of `candidates` that `open` opens, or say why the last one did not.
#[cfg(any(unix, windows))]
fn first_that_opens<S: IpcStream + 'static>(candidates: &[PathBuf], open: impl Fn(&Path) -> io::Result<S>) -> io::Result<Box<dyn IpcStream>> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "no Discord socket to try");
    for candidate in candidates {
        match open(candidate) {
            Ok(stream) => return Ok(Box::new(stream)),
            Err(error) => last = error,
        }
    }
    Err(last)
}

/// The numbers a socket of the desktop app may have.
fn socket_names() -> impl Iterator<Item = String> {
    (0..SOCKET_SLOTS).map(|slot| format!("{SOCKET_STEM}{slot}"))
}

/// The sockets the desktop app may listen on, in the order they are tried: the ten under the first
/// folder `var` names, and when the runtime folder is named, the ten its Flatpak and its Snap builds
/// put under it.
#[cfg(any(unix, test))]
pub(crate) fn unix_candidates(var: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let named = |name: &str| var(name).filter(|value| !value.is_empty()).map(PathBuf::from);
    let base = UNIX_BASE_VARS.iter().find_map(|name| named(name)).unwrap_or_else(|| PathBuf::from(UNIX_FALLBACK_DIR));
    let runtime = named(XDG_RUNTIME_DIR);
    let sandboxed = runtime.into_iter().flat_map(|runtime| SANDBOXED_SOCKET_DIRS.map(|dir| runtime.join(dir)));
    std::iter::once(base).chain(sandboxed).flat_map(|dir| socket_names().map(move |name| dir.join(name))).collect()
}

/// The pipes the desktop app may listen on, in the order they are tried.
#[cfg(any(windows, test))]
pub(crate) fn windows_pipe_names() -> Vec<PathBuf> {
    socket_names().map(|name| PathBuf::from(format!("{WINDOWS_PIPE_PREFIX}{name}"))).collect()
}

/// Opens the first Unix socket of the desktop app that answers.
#[cfg(unix)]
pub(crate) struct UnixConnector {
    candidates: Vec<PathBuf>,
}

#[cfg(unix)]
impl UnixConnector {
    pub(crate) fn new(candidates: Vec<PathBuf>) -> UnixConnector {
        UnixConnector { candidates }
    }
}

#[cfg(unix)]
impl Connect for UnixConnector {
    fn connect(&mut self) -> io::Result<Box<dyn IpcStream>> {
        first_that_opens(&self.candidates, |path| UnixStream::connect(path))
    }
}

/// Opens the first named pipe of the desktop app that opens. A pipe that is busy is passed over like
/// one that is not there: the standard library has no way to wait for it, and the next attempt
/// comes after the reconnect interval.
#[cfg(windows)]
pub(crate) struct PipeConnector {
    candidates: Vec<PathBuf>,
}

#[cfg(windows)]
impl Connect for PipeConnector {
    fn connect(&mut self) -> io::Result<Box<dyn IpcStream>> {
        first_that_opens(&self.candidates, |path| std::fs::OpenOptions::new().read(true).write(true).open(path))
    }
}

/// A connector on a system the desktop app does not run on: it never connects.
#[cfg(not(any(unix, windows)))]
struct NoConnector;

#[cfg(not(any(unix, windows)))]
impl Connect for NoConnector {
    fn connect(&mut self) -> io::Result<Box<dyn IpcStream>> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "Discord has no socket on this system"))
    }
}

/// The connector to the desktop app on this machine.
#[cfg(unix)]
pub(crate) fn system_connector() -> Box<dyn Connect> {
    Box::new(UnixConnector::new(unix_candidates(|name| std::env::var_os(name))))
}

/// The connector to the desktop app on this machine.
#[cfg(windows)]
pub(crate) fn system_connector() -> Box<dyn Connect> {
    Box::new(PipeConnector { candidates: windows_pipe_names() })
}

/// The connector to the desktop app on this machine.
#[cfg(not(any(unix, windows)))]
pub(crate) fn system_connector() -> Box<dyn Connect> {
    Box::new(NoConnector)
}
