use std::error::Error;
use std::fmt;

/// Which step of loading a difficulty table a failure belongs to.
///
/// A table is reached in up to three requests — the address the user registered, the header it
/// leads to and the body the header names — and "the table did not load" is only useful with the
/// step and the address that went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStage {
    /// The address that was registered: a table page, a header or a body.
    Address,
    /// The header JSON a table page named.
    Header,
    /// A body JSON the header named.
    Body,
}

impl fmt::Display for TableStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TableStage::Address => "address",
            TableStage::Header => "header",
            TableStage::Body => "body",
        })
    }
}

/// Everything that can go wrong while loading a BMS difficulty table.
///
/// The `Display` text is the message the player surfaces to the user, so it stays stable
/// across the migration away from the string-typed loading helpers.
#[derive(Debug, thiserror::Error)]
pub enum TableError {
    /// The body (`data.json`) was not a JSON array of table entries.
    #[error("body parse: {0}")]
    BodyParse(serde_json::Error),
    /// The header (`header.json`) was not a JSON object describing a table.
    #[error("header parse: {0}")]
    HeaderParse(serde_json::Error),
    /// A header was fetched but carries no `data_url`, so the body cannot be located.
    #[error("header.json has no data_url")]
    MissingDataUrl,
    /// The address answered with a page that names no table header and is not table JSON itself.
    #[error("the page has no <meta name=\"bmstable\"> tag and is not table JSON")]
    NoTableMeta,
    /// The server answered with a non-success status.
    #[error("HTTP {0}")]
    HttpStatus(reqwest::StatusCode),
    /// The answer was larger than a difficulty table is allowed to be.
    #[error("the response is larger than {limit} bytes")]
    TooLarge { limit: usize },
    /// The table's requests together took longer than a table load is given.
    #[error("the table did not arrive within {} seconds", .limit.as_secs())]
    TimedOut { limit: std::time::Duration },
    /// The answer stopped arriving part-way.
    #[error("the response could not be read: {0}")]
    Read(std::io::Error),
    /// The HTTP client could not be built, or the request itself failed.
    #[error("{}", request_reason(.0))]
    Request(#[from] reqwest::Error),
    /// One step of the load failed: which step, the address it was asking, and why.
    #[error("{stage} {url}: {source}")]
    At { stage: TableStage, url: String, source: Box<TableError> },
}

impl TableError {
    /// This failure as the failure of one step of the load.
    pub(crate) fn at(self, stage: TableStage, url: &str) -> TableError {
        TableError::At { stage, url: url.to_string(), source: Box::new(self) }
    }
}

/// Why a request failed, in a line. The HTTP client's own message names the address — which the
/// step that failed already carries — and leaves the cause to its source chain, so the kind of
/// failure and the innermost cause are what is worth showing.
fn request_reason(error: &reqwest::Error) -> String {
    let kind = if error.is_timeout() {
        "the request timed out"
    } else if error.is_connect() {
        "could not connect"
    } else if error.is_redirect() {
        "the redirects did not end"
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
