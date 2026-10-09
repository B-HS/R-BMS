//! The `io` library, confined to the skin root and writing to the overlay.
//!
//! The reference lets a skin read and write anywhere under its own folder
//! (`SkinLuaAccessor.RestrictedIoLib`), and real skins use that to keep histories and rotation
//! lists. This build never writes into a skin folder: every write goes to the overlay directory, and
//! every read looks in the overlay first so a skin reads back what it wrote. Nothing of the C `io`
//! library is opened; the whole library is the table built here.
//!
//! Where a file is looked for:
//!
//! - A name is an absolute path under the skin root or a path relative to it, and is reduced to its
//!   root-relative form by [`SkinPaths`](super::SkinPaths). A name that leaves the root -- by an
//!   absolute path, by `..`, or through a symbolic link -- is refused.
//! - Mode `r` opens the overlay's copy when there is one and the root's otherwise, for reading only.
//! - Every other mode opens the overlay's copy for reading and writing, creating its parent
//!   directories. `w` empties it. `a` and `r+` keep what is there, so when the overlay has no copy
//!   yet and the root has the file, the root's file is copied into the overlay first. The skin
//!   root is never opened for writing.
//! - Without an overlay every mode but `r` fails.
//!
//! What the calls answer follows the reference's LuaJ library, not the C one, wherever a skin can
//! tell the difference:
//!
//! - A file system failure is answered as `nil, "io error: <reason>"` and is not raised: a missing
//!   file, a name outside the root, no overlay, `io.popen`. Skins test `io.open(path) == nil` to
//!   ask whether a file exists. The exceptions are the calls that have no way to answer -- opening
//!   the file of `io.lines`, `io.input` and `io.output`, and a failure inside a line iterator --
//!   which raise the same message.
//! - Misuse is raised: a closed file, a mode or a format that is not one.
//! - `write` accepts strings and numbers, any number of them including none, and answers the file,
//!   so `io.open(p, "a"):write(s):close()` chains.
//! - Reading a line drops every carriage return in it, so a file saved on Windows reads the same.
//!   The formats are `*l`, `*L`, `*a`, `*n` and a byte count, with or without the star. A number
//!   that runs to the end of the file is not read: the reference looks one byte ahead after it and
//!   fails there, and so does this.
//! - `io.lines(name)` closes its file when the lines run out. `file:lines()` leaves it open.
//! - The standard streams cannot be closed. `stdin` is empty. What is written to `stdout` or
//!   `stderr` -- which is where `io.write` goes until `io.output` says otherwise -- is recorded in
//!   the log beside what the skin printed, one entry per line.
//! - `flush` and `setvbuf` succeed and do nothing: every write reaches the file at once. The
//!   reference's `flush` also forces the file onto the disk; this one does not, so a frame never
//!   waits for a disk.
//! - `io.tmpfile` answers a new file `lua-<unique>.tmp` at the top of the overlay, open for reading
//!   and writing. As in the reference nothing removes it afterwards.
//! - One read answers at most 64 MiB. A larger one fails instead of being cut short.
//! - A skin holds at most 64 files open at once. Each one is a descriptor of the whole process, and
//!   a skin that used them all up would leave the player unable to open a chart or a sound. Skins
//!   routinely open a file to ask whether it exists and never close it, so before an open is
//!   refused the collector is run, which closes every file the skin can no longer reach; the open
//!   answers `nil, "io error: too many open files"` only when that many are still held.
//!
//! Nothing here raises a Rust panic on malformed input.

use std::cell::Cell;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mlua::{AnyUserData, Function, IntoLuaMulti, Lua, MetaMethod, MultiValue, UserData, UserDataMethods, Value, Variadic};

use super::budget::{Meter, budget_error};
use super::{LuaShared, SkinPaths, bad_argument, coerce, package};
use crate::SkinError;

/// The global, and the module name, the library is published under.
const IO_GLOBAL: &str = "io";

/// The registry name the default input file is kept under.
const DEFAULT_INPUT_KEY: &str = "rbms.skin.io.input";

/// The registry name the default output file is kept under.
const DEFAULT_OUTPUT_KEY: &str = "rbms.skin.io.output";

/// What every answered failure starts with (`IoLib.errorresult`).
const FAILURE_PREFIX: &str = "io error: ";

/// What a read-mode open of a file that is not there fails with (`RestrictedIoLib.openFile`).
const NOT_FOUND: &str = "Lua file not found: ";

/// What an open of a name outside the skin root fails with (`RestrictedIoLib.resolve`).
const ACCESS_DENIED: &str = "Lua skin file access denied: ";

/// What `io.popen` fails with (`RestrictedIoLib.openProgram`).
const POPEN_REFUSED: &str = "Lua io.popen is not allowed";

/// What closing a standard stream is answered with (`IoLib.ioclose`).
const STANDARD_CLOSE_REFUSED: &str = "cannot close standard file";

/// What using a closed file raises (`IoLib.checkopen`).
const CLOSED_FILE_USED: &str = "attempt to use a closed file";

/// What stepping the line iterator of a closed file raises (`IoLib._lines_iter`).
const ITERATED_FILE_CLOSED: &str = "file is already closed";

/// What reading past the end while looking for a number fails with.
const NUMBER_AT_END: &str = "end of file";

/// What `io.type` answers for an open file.
const OPEN_FILE_TYPE: &str = "file";

/// What `io.type` answers for a closed file.
const CLOSED_FILE_TYPE: &str = "closed file";

/// The mode a file is opened in when none is given.
const READ_MODE: &str = "r";

/// The mode `io.popen` would write to its program in.
const WRITE_MODE: &str = "w";

/// The mark that a mode also allows the other direction.
const UPDATE_MARK: u8 = b'+';

/// The mark that a mode is binary, which changes nothing here.
const BINARY_MARK: u8 = b'b';

/// The mark a read format may open with.
const FORMAT_MARK: u8 = b'*';

/// The first letter of each kind of mode: read, write, append.
const MODE_READ: u8 = b'r';
const MODE_WRITE: u8 = b'w';
const MODE_APPEND: u8 = b'a';

/// The letter that names each read format.
const FORMAT_NUMBER: u8 = b'n';
const FORMAT_LINE: u8 = b'l';
const FORMAT_WHOLE_LINE: u8 = b'L';
const FORMAT_ALL: u8 = b'a';

/// The names `seek` knows a starting point by.
const SEEK_FROM_START: &str = "set";
const SEEK_FROM_CURRENT: &str = "cur";
const SEEK_FROM_END: &str = "end";

/// The buffering modes `setvbuf` accepts.
const BUFFER_MODES: [&str; 3] = ["no", "full", "line"];

const LINE_FEED: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';

/// The blanks skipped before a number (`IoLib.freadnumber`).
const NUMBER_BLANKS: &[u8] = b" \t\r\n";

/// The signs a number may open with.
const NUMBER_SIGNS: &[u8] = b"-+";

/// The digits of a number.
const NUMBER_DIGITS: &[u8] = b"0123456789";

/// The point between a number's whole part and its fraction.
const NUMBER_POINT: &[u8] = b".";

/// Bytes one read may answer. A skin's data files are a few kilobytes; the ceiling keeps a skin that
/// reads a movie as text from pulling it into memory whole, outside what the interpreter's own
/// memory limit can see.
const MAX_READ_BYTES: u64 = 64 * 1024 * 1024;

/// Files of the disk one skin may hold open at once. The player's own limit can be as low as 256
/// for the whole process.
const MAX_OPEN_FILES: usize = 64;

/// What an open past [`MAX_OPEN_FILES`] fails with.
const TOO_MANY_OPEN_FILES: &str = "too many open files";

/// Bytes of interpreter memory whose collection is charged as one instruction, when an open has to
/// run the collector to find room.
const COLLECTED_BYTES_PER_INSTRUCTION: usize = 64;

/// How a temporary file's name starts and ends (`RestrictedIoLib.tmpFile`).
const TEMP_FILE_PREFIX: &str = "lua-";
const TEMP_FILE_SUFFIX: &str = ".tmp";

/// Names tried before `io.tmpfile` gives up on finding a free one.
const TEMP_FILE_ATTEMPTS: u32 = 16;

/// Sets the temporary files of one process apart from each other.
static TEMP_FILE_SERIAL: AtomicU64 = AtomicU64::new(0);

/// Why a call did not produce its result.
enum Fault {
    /// The file system refused. Answered to the skin as `nil, "io error: <reason>"`.
    Refused(String),
    /// The skin misused the library. Raised.
    Raised(mlua::Error),
}

impl Fault {
    /// The error to raise where a refusal cannot be answered as a value.
    fn raise(self) -> mlua::Error {
        match self {
            Self::Refused(reason) => mlua::Error::runtime(format!("{FAILURE_PREFIX}{reason}")),
            Self::Raised(error) => error,
        }
    }
}

impl From<std::io::Error> for Fault {
    fn from(error: std::io::Error) -> Self {
        Self::Refused(error.to_string())
    }
}

impl From<mlua::Error> for Fault {
    fn from(error: mlua::Error) -> Self {
        Self::Raised(error)
    }
}

/// Turns a call's outcome into what the skin receives: its values, or `nil` and a message.
fn settle(lua: &Lua, outcome: Result<MultiValue, Fault>) -> mlua::Result<MultiValue> {
    match outcome {
        Ok(values) => Ok(values),
        Err(Fault::Refused(reason)) => (Value::Nil, format!("{FAILURE_PREFIX}{reason}")).into_lua_multi(lua),
        Err(Fault::Raised(error)) => Err(error),
    }
}

/// What a mode string asks for (`IoLib.rawopenfile`).
#[derive(Clone, Copy)]
struct Mode {
    /// The file must exist and is not emptied.
    read: bool,
    /// The file is kept and writing starts at its end.
    append: bool,
    /// The other direction is allowed as well.
    update: bool,
}

impl Mode {
    /// Reading only: what `io.lines` and `io.input` open a named file in.
    const READ: Self = Self { read: true, append: false, update: false };

    /// Emptying the file: what `io.output` opens a named file in.
    const WRITE: Self = Self { read: false, append: false, update: false };

    /// Reads a mode: `r`, `w` or `a`, then an optional `+`, then any number of `b`.
    fn parse(mode: &str) -> Option<Self> {
        let bytes = mode.as_bytes();
        let first = *bytes.first()?;
        let valid = bytes.iter().enumerate().all(|(index, byte)| match index {
            0 => [MODE_READ, MODE_WRITE, MODE_APPEND].contains(byte),
            1 => *byte == UPDATE_MARK || *byte == BINARY_MARK,
            _ => *byte == BINARY_MARK,
        });
        valid.then_some(Self { read: first == MODE_READ, append: first == MODE_APPEND, update: bytes.get(1) == Some(&UPDATE_MARK) })
    }

    /// Whether the file is only ever read, which is the one case the skin root's own file is opened.
    fn read_only(self) -> bool {
        self.read && !self.update
    }
}

/// Where `seek` measures from.
#[derive(Clone, Copy)]
enum Whence {
    Start,
    Current,
    End,
}

impl Whence {
    fn parse(name: &str) -> Option<Self> {
        match name {
            SEEK_FROM_START => Some(Self::Start),
            SEEK_FROM_CURRENT => Some(Self::Current),
            SEEK_FROM_END => Some(Self::End),
            _ => None,
        }
    }
}

/// One item a read is asked for (`IoLib.ioread`).
enum Format {
    /// At most this many bytes.
    Count(i64),
    /// A number.
    Number,
    /// The next line, with or without what ended it.
    Line { keep_end: bool },
    /// Everything that is left.
    All,
    /// Not a format. Raised only when the read gets as far as this item.
    Invalid,
}

impl Format {
    fn of(value: &Value) -> Self {
        match value {
            Value::Integer(_) | Value::Number(_) => Self::Count(i64::from(coerce::to_int(value))),
            Value::String(text) => {
                let borrowed = text.as_bytes();
                let bytes: &[u8] = &borrowed;
                match bytes.strip_prefix(&[FORMAT_MARK]).unwrap_or(bytes).first().copied() {
                    Some(FORMAT_NUMBER) => Self::Number,
                    Some(FORMAT_LINE) => Self::Line { keep_end: false },
                    Some(FORMAT_WHOLE_LINE) => Self::Line { keep_end: true },
                    Some(FORMAT_ALL) => Self::All,
                    _ => Self::Invalid,
                }
            }
            _ => Self::Invalid,
        }
    }
}

/// How many files of the disk one interpreter's skin holds open.
#[derive(Debug, Default)]
pub(crate) struct OpenFiles(Rc<Cell<usize>>);

impl OpenFiles {
    /// Makes sure one more file may be opened, or says why not.
    ///
    /// At the ceiling the collector runs first, because a file the skin dropped without closing is
    /// only closed when it is collected. The collection is charged to the meter by the memory it
    /// walked, so a skin that keeps asking at the ceiling runs out of budget instead of time.
    fn make_room(&self, lua: &Lua, meter: &Meter) -> Result<(), Fault> {
        if self.0.get() < MAX_OPEN_FILES {
            return Ok(());
        }
        lua.gc_collect()?;
        if !meter.charge((lua.used_memory() / COLLECTED_BYTES_PER_INSTRUCTION) as u64) {
            return Err(Fault::Raised(budget_error()));
        }
        if self.0.get() < MAX_OPEN_FILES { Ok(()) } else { Err(Fault::Refused(TOO_MANY_OPEN_FILES.to_owned())) }
    }

    /// Takes `file` into the count, for as long as the value answered lives.
    fn count(&self, file: File) -> CountedFile {
        self.0.set(self.0.get() + 1);
        CountedFile { reader: BufReader::new(file), open_files: Rc::clone(&self.0) }
    }
}

/// A file of the disk, counted among its interpreter's open files until it is dropped.
struct CountedFile {
    reader: BufReader<File>,
    open_files: Rc<Cell<usize>>,
}

impl Drop for CountedFile {
    fn drop(&mut self) {
        self.open_files.set(self.open_files.get().saturating_sub(1));
    }
}

/// What a file's bytes come from and go to.
enum Stream {
    /// A file on disk. The reader's buffer is only ever read-ahead: a write first puts the file back
    /// where the skin believes it is.
    Disk(CountedFile),
    /// Nothing to read and nowhere to write: the standard input, and any file once it is closed.
    Empty,
    /// Nothing to read; what is written goes to the log. The standard output and error.
    Log(Rc<LuaShared>),
}

impl Stream {
    /// The next byte, left in place. `None` at the end.
    fn peek(&mut self) -> std::io::Result<Option<u8>> {
        match self {
            Self::Disk(file) => Ok(file.reader.fill_buf()?.first().copied()),
            Self::Empty | Self::Log(_) => Ok(None),
        }
    }

    /// Steps over the byte [`Self::peek`] showed.
    fn skip(&mut self) {
        if let Self::Disk(file) = self {
            file.reader.consume(1);
        }
    }

    /// Up to `count` bytes from where the file stands.
    fn take(&mut self, count: u64) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        if let Self::Disk(file) = self {
            file.reader.by_ref().take(count.min(MAX_READ_BYTES + 1)).read_to_end(&mut bytes)?;
        }
        within_limit(bytes)
    }

    /// The bytes up to and including the next line feed, or up to the end.
    fn take_line(&mut self) -> std::io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        if let Self::Disk(file) = self {
            file.reader.by_ref().take(MAX_READ_BYTES + 1).read_until(LINE_FEED, &mut bytes)?;
        }
        within_limit(bytes)
    }

    /// Writes `bytes` where the file stands.
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Disk(file) => {
                let reader = &mut file.reader;
                if !reader.buffer().is_empty() {
                    let position = reader.stream_position()?;
                    reader.seek(SeekFrom::Start(position))?;
                }
                reader.get_mut().write_all(bytes)
            }
            Self::Empty => Ok(()),
            Self::Log(shared) => {
                let text = String::from_utf8_lossy(bytes);
                for line in text.split(char::from(LINE_FEED)).map(|line| line.trim_end_matches(char::from(CARRIAGE_RETURN))) {
                    if !line.is_empty() {
                        shared.log.printed(line);
                    }
                }
                Ok(())
            }
        }
    }

    /// Moves to `offset` from `whence`, never before the start, and answers where that is
    /// (`RestrictedFile.seek`).
    fn seek(&mut self, whence: Whence, offset: i64) -> std::io::Result<u64> {
        let Self::Disk(file) = self else {
            return Ok(0);
        };
        let reader = &mut file.reader;
        let base = match whence {
            Whence::Start => 0,
            Whence::Current => reader.stream_position()?,
            Whence::End => reader.get_ref().metadata()?.len(),
        };
        let position = i64::try_from(base).unwrap_or(i64::MAX).saturating_add(offset).max(0);
        reader.seek(SeekFrom::Start(position.unsigned_abs()))
    }
}

/// `bytes`, unless there are more of them than one read may answer.
fn within_limit(bytes: Vec<u8>) -> std::io::Result<Vec<u8>> {
    if bytes.len() as u64 > MAX_READ_BYTES {
        return Err(std::io::Error::other(format!("a read of more than {MAX_READ_BYTES} bytes")));
    }
    Ok(bytes)
}

/// One file as a skin holds it: the value `io.open` answers.
struct Handle {
    stream: Stream,
    /// One of the three standard streams, which cannot be closed.
    standard: bool,
    closed: bool,
}

impl Handle {
    fn disk(file: CountedFile) -> Self {
        Self { stream: Stream::Disk(file), standard: false, closed: false }
    }

    fn standard(stream: Stream) -> Self {
        Self { stream, standard: true, closed: false }
    }

    /// Raises when the file has been closed (`IoLib.checkopen`).
    fn check_open(&self) -> mlua::Result<()> {
        if self.closed { Err(mlua::Error::runtime(CLOSED_FILE_USED)) } else { Ok(()) }
    }

    /// Lets go of the file.
    fn close(&mut self) {
        self.stream = Stream::Empty;
        self.closed = true;
    }

    /// The next line, or `None` when the file ended before giving anything (`IoLib.freaduntil`).
    ///
    /// Without `keep_end` the line feed is dropped and so is every carriage return, wherever in the
    /// line it stands. A file that ends in nothing but carriage returns therefore ends in no line.
    fn read_line(&mut self, keep_end: bool) -> std::io::Result<Option<Vec<u8>>> {
        let mut line = self.stream.take_line()?;
        if line.is_empty() {
            return Ok(None);
        }
        if keep_end {
            return Ok(Some(line));
        }
        let ended = line.last() == Some(&LINE_FEED);
        if ended {
            line.pop();
        }
        line.retain(|byte| *byte != CARRIAGE_RETURN);
        Ok((ended || !line.is_empty()).then_some(line))
    }

    /// Up to `count` bytes, or `None` at the end of the file (`IoLib.freadbytes`).
    fn read_count(&mut self, count: u64) -> std::io::Result<Option<Vec<u8>>> {
        if count == 0 {
            return Ok(self.stream.peek()?.map(|_| Vec::new()));
        }
        let bytes = self.stream.take(count)?;
        Ok((!bytes.is_empty()).then_some(bytes))
    }

    /// A number, or `None` when what stands there is not one (`IoLib.freadnumber`).
    ///
    /// The reference reads blanks, signs, digits, points and digits in that order, looking one byte
    /// ahead each time, and its files answer that look at the end of the file with a failure. So a
    /// number is read only when something follows it, and reaching the end anywhere on the way
    /// fails the whole read.
    fn read_number(&mut self) -> Result<Option<f64>, Fault> {
        let mut text = Vec::new();
        self.read_run(NUMBER_BLANKS, None)?;
        self.read_run(NUMBER_SIGNS, Some(&mut text))?;
        self.read_run(NUMBER_DIGITS, Some(&mut text))?;
        self.read_run(NUMBER_POINT, Some(&mut text))?;
        self.read_run(NUMBER_DIGITS, Some(&mut text))?;
        if text.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&text);
        text.parse().map(Some).map_err(|_| Fault::Raised(mlua::Error::runtime(format!("malformed number '{text}'"))))
    }

    /// Steps over every next byte that is one of `accepted`, keeping them when asked to.
    fn read_run(&mut self, accepted: &[u8], mut kept: Option<&mut Vec<u8>>) -> Result<(), Fault> {
        loop {
            let Some(byte) = self.stream.peek()? else {
                return Err(Fault::Refused(NUMBER_AT_END.to_owned()));
            };
            if !accepted.contains(&byte) {
                return Ok(());
            }
            self.stream.skip();
            if let Some(kept) = kept.as_deref_mut() {
                kept.push(byte);
            }
        }
    }

    /// Reads one item per format, stopping after the first that the file could not give
    /// (`IoLib.ioread`). No formats at all reads one line.
    fn read_formats(&mut self, lua: &Lua, formats: &[Value]) -> Result<Vec<Value>, Fault> {
        if formats.is_empty() {
            return Ok(vec![text_value(lua, self.read_line(false)?)?]);
        }
        let mut values = Vec::with_capacity(formats.len());
        for (index, format) in formats.iter().enumerate() {
            let value = match Format::of(format) {
                Format::Count(count) => {
                    let count = u64::try_from(count).map_err(|_| bad_argument(index + 1, "read", "invalid count"))?;
                    text_value(lua, self.read_count(count)?)?
                }
                Format::Number => self.read_number()?.map_or(Value::Nil, Value::Number),
                Format::Line { keep_end } => text_value(lua, self.read_line(keep_end)?)?,
                Format::All => text_value(lua, Some(self.stream.take(u64::MAX)?))?,
                Format::Invalid => return Err(bad_argument(index + 1, "read", "invalid format").into()),
            };
            let ended = value.is_nil();
            values.push(value);
            if ended {
                break;
            }
        }
        Ok(values)
    }
}

/// The bytes a read gave as a Lua string, or `nil` when it gave none.
fn text_value(lua: &Lua, bytes: Option<Vec<u8>>) -> mlua::Result<Value> {
    match bytes {
        Some(bytes) => lua.create_string(bytes).map(Value::String),
        None => Ok(Value::Nil),
    }
}

impl UserData for Handle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("read", |lua, (file, formats): (AnyUserData, Variadic<Value>)| settle(lua, read(lua, &file, &formats)));
        methods.add_function("write", |lua, (file, values): (AnyUserData, Variadic<Value>)| settle(lua, write(lua, file, &values)));
        methods.add_function("lines", |lua, (file, formats): (AnyUserData, Variadic<Value>)| {
            file.borrow::<Handle>()?.check_open()?;
            lines(lua, file, false, formats.into_iter().collect())
        });
        methods.add_function("close", |lua, file: AnyUserData| close(lua, &file));
        methods.add_function("flush", |_, file: AnyUserData| {
            file.borrow::<Handle>()?.check_open()?;
            Ok(true)
        });
        methods.add_function("setvbuf", |_, (file, mode, _size): (AnyUserData, Value, Value)| {
            let mode = name_argument(&mode, 2, "setvbuf")?;
            if !BUFFER_MODES.contains(&mode.as_str()) {
                return Err(bad_argument(2, "setvbuf", "invalid mode"));
            }
            file.borrow::<Handle>()?.check_open()?;
            Ok(true)
        });
        methods.add_function("seek", |lua, (file, whence, offset): (AnyUserData, Value, Value)| {
            let whence = match whence {
                Value::Nil => Whence::Current,
                named => Whence::parse(&name_argument(&named, 2, "seek")?).ok_or_else(|| bad_argument(2, "seek", "invalid option"))?,
            };
            let offset = match offset {
                Value::Nil => 0,
                Value::Integer(_) | Value::Number(_) => coerce::to_long(&offset),
                other => return Err(bad_argument(3, "seek", &format!("number expected, got {}", other.type_name()))),
            };
            let mut handle = file.borrow_mut::<Handle>()?;
            handle.check_open()?;
            let outcome = handle.stream.seek(whence, offset).map_err(Fault::from).and_then(|position| Ok((position as f64).into_lua_multi(lua)?));
            settle(lua, outcome)
        });
        methods.add_meta_function(MetaMethod::ToString, |_, file: AnyUserData| Ok(format!("{OPEN_FILE_TYPE}: {:p}", file.to_pointer())));
    }
}

/// A string argument, which may also be given as a number (`Varargs.checkjstring`).
fn name_argument(value: &Value, position: usize, function: &str) -> mlua::Result<String> {
    match value {
        Value::String(_) | Value::Integer(_) | Value::Number(_) => Ok(coerce::to_jstring(value)),
        other => Err(bad_argument(position, function, &format!("string expected, got {}", other.type_name()))),
    }
}

/// `file:read(...)` and `io.read(...)`.
fn read(lua: &Lua, file: &AnyUserData, formats: &[Value]) -> Result<MultiValue, Fault> {
    let mut handle = file.borrow_mut::<Handle>()?;
    handle.check_open()?;
    Ok(MultiValue::from_vec(handle.read_formats(lua, formats)?))
}

/// `file:write(...)` and `io.write(...)`: every value written in order, and the file answered.
fn write(lua: &Lua, file: AnyUserData, values: &[Value]) -> Result<MultiValue, Fault> {
    let mut bytes = Vec::new();
    for (index, value) in values.iter().enumerate() {
        match value {
            Value::String(text) => bytes.extend_from_slice(&text.as_bytes()),
            Value::Integer(_) | Value::Number(_) => bytes.extend_from_slice(coerce::to_jstring(value).as_bytes()),
            other => return Err(bad_argument(index + 1, "write", &format!("string expected, got {}", other.type_name())).into()),
        }
    }
    {
        let mut handle = file.borrow_mut::<Handle>()?;
        handle.check_open()?;
        handle.stream.write(&bytes)?;
    }
    Ok(file.into_lua_multi(lua)?)
}

/// `file:close()` and `io.close(file)`.
fn close(lua: &Lua, file: &AnyUserData) -> mlua::Result<MultiValue> {
    let mut handle = file.borrow_mut::<Handle>()?;
    handle.check_open()?;
    if handle.standard {
        return (Value::Nil, STANDARD_CLOSE_REFUSED).into_lua_multi(lua);
    }
    handle.close();
    true.into_lua_multi(lua)
}

/// The iterator `lines` answers (`IoLib._lines_iter`).
///
/// Each step is a read with the formats `lines` was given. A failure inside a step is raised, since
/// a `for` loop has nowhere to receive a message. With `close_at_end` the file is closed by the step
/// that finds it exhausted.
fn lines(lua: &Lua, file: AnyUserData, close_at_end: bool, formats: Vec<Value>) -> mlua::Result<Function> {
    lua.create_function(move |lua, ()| {
        let mut handle = file.borrow_mut::<Handle>()?;
        if handle.closed {
            return Err(mlua::Error::runtime(ITERATED_FILE_CLOSED));
        }
        let values = handle.read_formats(lua, &formats).map_err(Fault::raise)?;
        if close_at_end && values.first().is_none_or(Value::is_nil) && handle.stream.peek().map_err(|error| Fault::from(error).raise())?.is_none() {
            handle.close();
        }
        Ok(MultiValue::from_vec(values))
    })
}

/// What a name the skin's view of the file system will not resolve is refused with: the reference's
/// wording for one that leaves the root (`RestrictedIoLib.resolve`), the cause for anything else.
fn refusal(named: &str, error: &SkinError) -> String {
    match error {
        SkinError::PathEscape(_) => format!("{ACCESS_DENIED}{named}"),
        other => other.to_string(),
    }
}

/// Where reading the file a skin named reads from: the overlay's copy when there is one, the skin
/// root's file otherwise. The file need not exist. The error is the message to refuse with.
///
/// This and [`write_target`] are the two questions every library that touches files asks, so the
/// Java `File` facade asks them here too and refuses in the same words.
pub(crate) fn read_source(paths: &SkinPaths, named: &str) -> Result<PathBuf, String> {
    paths.readable(named).map_err(|error| refusal(named, &error))
}

/// Where writing the file a skin named writes to: always a path inside the overlay, never one in
/// the skin root. Nothing is created. Refused when the name leaves the root, when it is the root
/// itself, and when the skin has no overlay.
pub(crate) fn write_target(paths: &SkinPaths, named: &str) -> Result<PathBuf, String> {
    let logical = paths.logical(named).map_err(|error| refusal(named, &error))?;
    if logical.as_os_str().is_empty() {
        return Err(format!("{ACCESS_DENIED}{named}"));
    }
    paths.writable(named).map_err(|error| refusal(named, &error))
}

/// Opens the file a skin named (`RestrictedIoLib.openFile`), by the rules in the module
/// documentation. Nothing is created or emptied for an open that is refused for want of room.
fn open(lua: &Lua, shared: &LuaShared, named: &str, mode: Mode) -> Result<Handle, Fault> {
    let paths = &shared.paths;
    if mode.read {
        let source = read_source(paths, named).map_err(Fault::Refused)?;
        if !source.is_file() {
            return Err(Fault::Refused(format!("{NOT_FOUND}{named}")));
        }
        if mode.read_only() {
            shared.open_files.make_room(lua, &shared.meter)?;
            return Ok(Handle::disk(shared.open_files.count(File::open(source)?)));
        }
    }

    let target = write_target(paths, named).map_err(Fault::Refused)?;
    shared.open_files.make_room(lua, &shared.meter)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let keeps_content = mode.append || mode.read;
    if keeps_content && !target.exists() {
        let original = paths.script(named).map_err(|error| Fault::Refused(refusal(named, &error)))?;
        if original.is_file() {
            copy_into_overlay(&original, &target)?;
        }
    }
    let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(!keeps_content).open(&target)?;
    if mode.append {
        file.seek(SeekFrom::End(0))?;
    }
    Ok(Handle::disk(shared.open_files.count(file)))
}

/// Gives the overlay its own copy of a file of the skin root, so that appending to it or updating it
/// changes the copy. The copy is a new file with this process's permissions, not the original's:
/// a skin unpacked read-only must still get a copy that can be written. A copy that fails half way
/// is removed, because a truncated copy would hide the whole original from then on.
pub(crate) fn copy_into_overlay(original: &Path, target: &Path) -> std::io::Result<()> {
    let copied = File::open(original).and_then(|mut source| {
        let mut copy = File::create(target)?;
        std::io::copy(&mut source, &mut copy)
    });
    if let Err(error) = copied {
        let _ = std::fs::remove_file(target);
        return Err(error);
    }
    Ok(())
}

/// A new, empty file in the overlay, open for reading and writing (`RestrictedIoLib.tmpFile`).
/// As in the reference, the file is left behind when the skin is done with it.
fn temporary(lua: &Lua, shared: &LuaShared) -> Result<Handle, Fault> {
    shared.open_files.make_room(lua, &shared.meter)?;
    let moment = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    for _ in 0..TEMP_FILE_ATTEMPTS {
        let serial = TEMP_FILE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let name = format!("{TEMP_FILE_PREFIX}{moment:x}-{serial}{TEMP_FILE_SUFFIX}");
        let target = write_target(&shared.paths, &name).map_err(Fault::Refused)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match OpenOptions::new().read(true).write(true).create_new(true).open(&target) {
            Ok(file) => return Ok(Handle::disk(shared.open_files.count(file))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(Fault::Refused(format!("no free temporary file name in {TEMP_FILE_ATTEMPTS} attempts")))
}

/// The file a default stream currently is.
fn default_stream(lua: &Lua, key: &str) -> mlua::Result<AnyUserData> {
    lua.named_registry_value(key)
}

/// `io.input` and `io.output`: answers the default stream, after replacing it when given a file or
/// the name of one to open in `mode`. A name that cannot be opened is raised.
fn redirect(lua: &Lua, shared: &LuaShared, key: &str, mode: Mode, function: &str, target: Value) -> mlua::Result<AnyUserData> {
    let file = match target {
        Value::Nil => return default_stream(lua, key),
        Value::String(_) | Value::Integer(_) | Value::Number(_) => {
            lua.create_userdata(open(lua, shared, &coerce::to_jstring(&target), mode).map_err(Fault::raise)?)?
        }
        Value::UserData(file) => {
            file.borrow::<Handle>()?.check_open()?;
            file
        }
        other => return Err(bad_argument(1, function, &format!("file expected, got {}", other.type_name()))),
    };
    lua.set_named_registry_value(key, &file)?;
    Ok(file)
}

/// Publishes the `io` library.
pub(crate) fn install(lua: &Lua, shared: &Rc<LuaShared>) -> mlua::Result<()> {
    let io = lua.create_table()?;

    let stdin = lua.create_userdata(Handle::standard(Stream::Empty))?;
    let stdout = lua.create_userdata(Handle::standard(Stream::Log(Rc::clone(shared))))?;
    let stderr = lua.create_userdata(Handle::standard(Stream::Log(Rc::clone(shared))))?;
    lua.set_named_registry_value(DEFAULT_INPUT_KEY, &stdin)?;
    lua.set_named_registry_value(DEFAULT_OUTPUT_KEY, &stdout)?;
    io.set("stdin", stdin)?;
    io.set("stdout", stdout)?;
    io.set("stderr", stderr)?;

    let opener = Rc::clone(shared);
    io.set(
        "open",
        lua.create_function(move |lua, (path, mode): (Value, Value)| {
            let path = name_argument(&path, 1, "open")?;
            let mode = if mode.is_nil() { READ_MODE.to_owned() } else { name_argument(&mode, 2, "open")? };
            let mode = Mode::parse(&mode).ok_or_else(|| bad_argument(2, "open", &format!("invalid mode: '{mode}'")))?;
            let outcome = open(lua, &opener, &path, mode).and_then(|handle| Ok(lua.create_userdata(handle)?.into_lua_multi(lua)?));
            settle(lua, outcome)
        })?,
    )?;

    let liner = Rc::clone(shared);
    io.set(
        "lines",
        lua.create_function(move |lua, arguments: Variadic<Value>| {
            let mut arguments = arguments.into_iter();
            match arguments.next().unwrap_or(Value::Nil) {
                Value::Nil => {
                    let file = default_stream(lua, DEFAULT_INPUT_KEY)?;
                    file.borrow::<Handle>()?.check_open()?;
                    lines(lua, file, false, arguments.collect())
                }
                path => {
                    let path = name_argument(&path, 1, "lines")?;
                    let handle = open(lua, &liner, &path, Mode::READ).map_err(Fault::raise)?;
                    lines(lua, lua.create_userdata(handle)?, true, arguments.collect())
                }
            }
        })?,
    )?;

    io.set(
        "close",
        lua.create_function(|lua, file: Value| match file {
            Value::Nil => close(lua, &default_stream(lua, DEFAULT_OUTPUT_KEY)?),
            Value::UserData(file) => close(lua, &file),
            other => Err(bad_argument(1, "close", &format!("file expected, got {}", other.type_name()))),
        })?,
    )?;

    io.set("read", lua.create_function(|lua, formats: Variadic<Value>| settle(lua, read(lua, &default_stream(lua, DEFAULT_INPUT_KEY)?, &formats)))?)?;
    io.set("write", lua.create_function(|lua, values: Variadic<Value>| settle(lua, write(lua, default_stream(lua, DEFAULT_OUTPUT_KEY)?, &values)))?)?;

    io.set(
        "type",
        lua.create_function(|_, value: Value| {
            let Value::UserData(file) = value else {
                return Ok(None);
            };
            Ok(file.borrow::<Handle>().ok().map(|handle| if handle.closed { CLOSED_FILE_TYPE } else { OPEN_FILE_TYPE }))
        })?,
    )?;

    let input = Rc::clone(shared);
    io.set("input", lua.create_function(move |lua, target: Value| redirect(lua, &input, DEFAULT_INPUT_KEY, Mode::READ, "input", target))?)?;
    let output = Rc::clone(shared);
    io.set("output", lua.create_function(move |lua, target: Value| redirect(lua, &output, DEFAULT_OUTPUT_KEY, Mode::WRITE, "output", target))?)?;

    io.set(
        "flush",
        lua.create_function(|lua, ()| {
            default_stream(lua, DEFAULT_OUTPUT_KEY)?.borrow::<Handle>()?.check_open()?;
            Ok(true)
        })?,
    )?;

    let temporary_owner = Rc::clone(shared);
    io.set(
        "tmpfile",
        lua.create_function(move |lua, ()| {
            let outcome = temporary(lua, &temporary_owner).and_then(|handle| Ok(lua.create_userdata(handle)?.into_lua_multi(lua)?));
            settle(lua, outcome)
        })?,
    )?;

    io.set(
        "popen",
        lua.create_function(|lua, (program, mode): (Value, Value)| {
            name_argument(&program, 1, "popen")?;
            let mode = if mode.is_nil() { READ_MODE.to_owned() } else { name_argument(&mode, 2, "popen")? };
            if mode != READ_MODE && mode != WRITE_MODE {
                return Err(bad_argument(2, "popen", &format!("invalid value: '{mode}'; must be one of 'r' or 'w'")));
            }
            settle(lua, Err(Fault::Refused(POPEN_REFUSED.to_owned())))
        })?,
    )?;

    lua.globals().set(IO_GLOBAL, &io)?;
    package::preload(lua, IO_GLOBAL, Value::Table(io))
}
