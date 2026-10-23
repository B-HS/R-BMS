use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

/// Everything that can go wrong while fetching a chart archive and unpacking it.
///
/// The `Display` text is what the player shows next to "download failed", so each variant says
/// what happened in a clause that reads on its own.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// The address, or the address a redirect led to, is not an http or https address with a host.
    #[error("the address is not an http or https address")]
    NotWebAddress,
    /// The server answered with a non-success status.
    #[error("HTTP {0}")]
    HttpStatus(reqwest::StatusCode),
    /// The download is larger than a chart archive is allowed to be.
    #[error("the download is larger than {limit} bytes")]
    TooLarge { limit: u64 },
    /// The download took longer than it is given.
    #[error("the download did not finish within {} seconds", .limit.as_secs())]
    TimedOut { limit: Duration },
    /// The answer stopped arriving part-way.
    #[error("the answer could not be read: {0}")]
    Read(std::io::Error),
    /// The HTTP client could not be built, or the request itself failed.
    #[error("{}", request_reason(.0))]
    Request(#[from] reqwest::Error),
    /// The answer does not begin like a zip archive, as a web page standing in for a file does not.
    #[error("the answer is not a zip archive")]
    NotAZip,
    /// The caller asked for the work to stop.
    #[error("cancelled")]
    Cancelled,
    /// A file or folder on this machine could not be read or written.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The archive is damaged or uses something this reader does not support.
    #[error("the archive cannot be read: {0}")]
    Archive(zip::result::ZipError),
    /// The archive's list of entries is larger than an archive of the allowed size would have.
    #[error("the archive's list of entries is larger than {limit} bytes")]
    IndexTooLarge { limit: u64 },
    /// The archive holds more entries than it is allowed to.
    #[error("the archive has more than {limit} entries")]
    TooManyEntries { limit: usize },
    /// One file in the archive unpacks to more than a single file is allowed to be.
    #[error("{name} unpacks to more than {limit} bytes")]
    EntryTooLarge { name: String, limit: u64 },
    /// The archive as a whole unpacks to more than it is allowed to.
    #[error("the archive unpacks to more than {limit} bytes")]
    UnpackedTooLarge { limit: u64 },
    /// The archive holds no chart, so there is nothing to play from it.
    #[error("the archive has no chart in it")]
    NoChart,
    /// The folder to unpack into already has something in it.
    #[error("{} is not empty", .0.display())]
    StagingNotEmpty(PathBuf),
    /// A name meant for a single folder is empty, climbs out of its parent or is not one every platform takes.
    #[error("{0:?} is not a plain folder name")]
    UnsafeName(String),
    /// Every numbered variant of a folder name that was tried is taken.
    #[error("every numbered variant of {name:?} is taken")]
    NameTaken { name: String },
}

/// Why a request failed, in a line. The HTTP client's own message names the address, which the
/// caller already has, so the kind of failure and the innermost cause are what is worth showing.
fn request_reason(error: &reqwest::Error) -> String {
    let kind = if error.is_timeout() {
        "the request timed out"
    } else if error.is_connect() {
        "could not connect"
    } else if error.is_redirect() {
        "the redirects did not end on the web"
    } else if error.is_body() || error.is_decode() {
        "the response could not be read"
    } else {
        "the request failed"
    };
    let mut cause: Option<&dyn Error> = error.source();
    while let Some(deeper) = cause.and_then(Error::source) {
        cause = Some(deeper);
    }
    match cause {
        Some(cause) => format!("{kind} ({cause})"),
        None => kind.to_string(),
    }
}
