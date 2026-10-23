use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use reqwest::redirect;

use crate::BYTES_PER_MIB;
use crate::error::DownloadError;
use crate::link::{is_web, web_url};

/// The largest download taken. Chart archives are a few kilobytes of differences up to the odd
/// whole song pack; this keeps an address that streams without end from filling the disk.
pub const DOWNLOAD_MAX_BYTES: u64 = 512 * BYTES_PER_MIB;

/// How long a server may go without answering: the wait for the answer to start, and for each
/// further piece of it. It is the client's own timeout, which the blocking client applies to each
/// wait rather than to a request, so a server that sends a little at a time never runs into it.
pub const DOWNLOAD_SILENCE: Duration = Duration::from_secs(30);

/// How long one download may take altogether, whatever it keeps sending.
pub const DOWNLOAD_TOTAL: Duration = Duration::from_secs(15 * 60);

const MAX_REDIRECTS: usize = 10;
const USER_AGENT: &str = "rbms-download";

/// How much of an answer is asked for at a time, which is how often the time left and the wish to
/// stop are looked at.
const READ_CHUNK_BYTES: usize = 64 * 1024;

const LOCAL_FILE_HEADER_MAGIC: [u8; 4] = *b"PK\x03\x04";
const EMPTY_ARCHIVE_MAGIC: [u8; 4] = *b"PK\x05\x06";

/// What bounds a download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadLimits {
    /// The most bytes that are taken; more than this is refused, by the announced size before
    /// reading and by the bytes that arrive after.
    pub max_bytes: u64,
    /// How long a wait for the server may last.
    pub silence: Duration,
    /// How long the whole download may take.
    pub total: Duration,
}

impl Default for DownloadLimits {
    fn default() -> DownloadLimits {
        DownloadLimits { max_bytes: DOWNLOAD_MAX_BYTES, silence: DOWNLOAD_SILENCE, total: DOWNLOAD_TOTAL }
    }
}

/// How far a job is, for a screen to read while a worker thread does the job. Every field is
/// written by the worker and read by anyone.
#[derive(Debug, Default)]
pub struct Progress {
    /// Bytes of the download that are in the file so far.
    pub received: AtomicU64,
    /// Bytes the server said it would send; 0 when it did not say.
    pub expected: AtomicU64,
    /// Which step the job is on, one of [`Progress::IDLE`], [`Progress::DOWNLOADING`] and
    /// [`Progress::EXTRACTING`], or a value of the caller's own after those.
    pub phase: AtomicU8,
}

impl Progress {
    /// Nothing has begun.
    pub const IDLE: u8 = 0;
    /// A download is under way; [`download_to`] and [`download_with`] set this.
    pub const DOWNLOADING: u8 = 1;
    /// The download is unpacked; the caller sets this around [`crate::extract_zip`].
    pub const EXTRACTING: u8 = 2;
}

/// A server's answer, once it has begun to arrive.
pub struct Answer {
    /// The address the answer finally came from, once any redirect was followed.
    pub url: String,
    /// The size the server announced, if it did.
    pub content_length: Option<u64>,
    /// The rest of the answer.
    pub body: Box<dyn Read>,
}

fn http_get(client: &reqwest::blocking::Client, url: &str) -> Result<Answer, DownloadError> {
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(DownloadError::HttpStatus(response.status()));
    }
    Ok(Answer { url: response.url().to_string(), content_length: response.content_length(), body: Box::new(response) })
}

fn build_client(silence: Duration) -> Result<reqwest::blocking::Client, DownloadError> {
    let policy = redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > MAX_REDIRECTS {
            attempt.error("too many redirects")
        } else if !is_web(attempt.url()) {
            attempt.error("a redirect leaves http and https")
        } else {
            attempt.follow()
        }
    });
    Ok(reqwest::blocking::Client::builder().timeout(silence).user_agent(USER_AGENT).redirect(policy).build()?)
}

/// Fetch `url` into the file `file`. The file is made, or made again, and holds only a zip archive:
/// an answer that does not begin like one is refused, a web page standing in for a file among them.
/// When the download does not succeed no file is left behind.
///
/// `progress` is updated as the bytes arrive, and `cancel` is looked at between pieces of the answer.
pub fn download_to(url: &str, file: &Path, limits: &DownloadLimits, progress: &Progress, cancel: &AtomicBool) -> Result<(), DownloadError> {
    let client = build_client(limits.silence)?;
    download_with(url, file, limits, progress, cancel, &mut |address| http_get(&client, address))
}

/// [`download_to`] with the request made by `get`, so that the whole of it can be exercised by
/// tests that never leave the process.
pub fn download_with(
    url: &str,
    file: &Path,
    limits: &DownloadLimits,
    progress: &Progress,
    cancel: &AtomicBool,
    get: &mut dyn FnMut(&str) -> Result<Answer, DownloadError>,
) -> Result<(), DownloadError> {
    let deadline = Instant::now() + limits.total;
    download_within(url, file, limits, progress, cancel, &mut || deadline.saturating_duration_since(Instant::now()), get)
}

/// [`download_with`] with the time left asked of `left` before each piece of the answer rather than
/// measured here, so that an answer which never stops arriving is tested without waiting for it.
pub(crate) fn download_within(
    url: &str,
    file: &Path,
    limits: &DownloadLimits,
    progress: &Progress,
    cancel: &AtomicBool,
    left: &mut dyn FnMut() -> Duration,
    get: &mut dyn FnMut(&str) -> Result<Answer, DownloadError>,
) -> Result<(), DownloadError> {
    if web_url(url).is_none() {
        return Err(DownloadError::NotWebAddress);
    }
    progress.received.store(0, Ordering::Relaxed);
    progress.expected.store(0, Ordering::Relaxed);
    progress.phase.store(Progress::DOWNLOADING, Ordering::Relaxed);
    check_may_go_on(limits, cancel, left)?;
    let answer = get(url)?;
    if web_url(&answer.url).is_none() {
        return Err(DownloadError::NotWebAddress);
    }
    if let Some(announced) = answer.content_length {
        if announced > limits.max_bytes {
            return Err(DownloadError::TooLarge { limit: limits.max_bytes });
        }
        progress.expected.store(announced, Ordering::Relaxed);
    }
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let result = {
        let mut out = File::create(file)?;
        stream_into(answer.body, &mut out, limits, progress, cancel, left)
    };
    if result.is_err() {
        let _ = fs::remove_file(file);
    }
    result
}

fn check_may_go_on(limits: &DownloadLimits, cancel: &AtomicBool, left: &mut dyn FnMut() -> Duration) -> Result<(), DownloadError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(DownloadError::Cancelled);
    }
    if left().is_zero() {
        return Err(DownloadError::TimedOut { limit: limits.total });
    }
    Ok(())
}

fn stream_into(
    mut body: Box<dyn Read>,
    out: &mut File,
    limits: &DownloadLimits,
    progress: &Progress,
    cancel: &AtomicBool,
    left: &mut dyn FnMut() -> Duration,
) -> Result<(), DownloadError> {
    let mut head = Vec::with_capacity(LOCAL_FILE_HEADER_MAGIC.len());
    body.by_ref().take(LOCAL_FILE_HEADER_MAGIC.len() as u64).read_to_end(&mut head).map_err(DownloadError::Read)?;
    if head != LOCAL_FILE_HEADER_MAGIC && head != EMPTY_ARCHIVE_MAGIC {
        return Err(DownloadError::NotAZip);
    }
    out.write_all(&head)?;
    let mut received = head.len() as u64;
    progress.received.store(received, Ordering::Relaxed);
    let mut chunk = vec![0; READ_CHUNK_BYTES];
    loop {
        check_may_go_on(limits, cancel, left)?;
        let read = match body.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(DownloadError::Read(error)),
        };
        received += read as u64;
        if received > limits.max_bytes {
            return Err(DownloadError::TooLarge { limit: limits.max_bytes });
        }
        out.write_all(&chunk[..read])?;
        progress.received.store(received, Ordering::Relaxed);
    }
}
