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
    /// The server answered with a non-success status.
    #[error("HTTP {status} for {url}")]
    HttpStatus { status: reqwest::StatusCode, url: String },
    /// The HTTP client could not be built, or the request itself failed.
    #[error("{0}")]
    Request(#[from] reqwest::Error),
    /// The fetch failed and no on-disk cache exists to fall back to.
    #[error("table fetch failed ({fetch}); no cache at {path}")]
    CacheMissing { fetch: Box<TableError>, path: String },
    /// The fetch failed and the on-disk cache could not be parsed.
    #[error("table fetch failed ({fetch}); cache parse: {source}")]
    CacheParse { fetch: Box<TableError>, source: serde_json::Error },
}
