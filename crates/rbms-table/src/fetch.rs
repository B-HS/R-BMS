//! Reaching a difficulty table from the address a user registered.
//!
//! The address every BMS tool is given is the table's web page; its `bmstable` meta tag names the
//! header JSON, and the header's `data_url` names the body. An address may also be the header or
//! the body itself. What the first answer is decides the rest, so the shape is read from the
//! answer rather than guessed from the address.
//!
//! The requests go through a function rather than straight to the network, so the whole chain —
//! relative addresses, redirects, a table split over several bodies — is exercised by tests that
//! never leave the process.

use std::io::Read;
use std::time::{Duration, Instant};

use crate::dto::{TableEntry, TableHeader};
use crate::error::{TableError, TableStage};
use crate::{DifficultyTable, html, parse_entries, parse_header, without_bom};

/// How long a table's server may go without answering: the wait for the answer to start, and for
/// each further piece of it. It is the client's own timeout, which the blocking client applies to
/// each wait rather than to a request, so a server that sends a little at a time never runs into it.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(15);

/// How long the requests of one table may take together. This is the limit a server that keeps
/// sending slowly runs into; the wait already begun when it passes is still sat out, so a load is
/// over within this and one [`SILENCE_TIMEOUT`].
pub(crate) const LOAD_TIMEOUT: Duration = Duration::from_secs(30);

/// How much of an answer is asked for at a time, which is how often the time left is looked at.
const READ_CHUNK_BYTES: usize = 64 * 1024;

/// What the player calls itself to a table's server.
const USER_AGENT: &str = "rbms-table";

/// The largest answer read from a table's server. The biggest tables in use are a few megabytes;
/// this is what keeps an address that streams without end from filling memory.
pub(crate) const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// The file a header is looked for under when the address was a body.
const SIBLING_HEADER: &str = "header.json";

/// One answer: the bytes, and the address they finally came from once any redirect was followed —
/// which is the address a relative one inside them is relative to.
pub(crate) struct Fetched {
    pub(crate) bytes: Vec<u8>,
    pub(crate) url: String,
}

/// What an answer is, read from its first meaningful byte.
enum Shape {
    /// A JSON array: a table body.
    Body,
    /// A JSON object: a table header.
    Header,
    /// Anything else, which for a table address is the web page that presents the table.
    Page,
}

fn shape_of(bytes: &[u8]) -> Shape {
    match without_bom(bytes).iter().find(|byte| !byte.is_ascii_whitespace()) {
        Some(b'[') => Shape::Body,
        Some(b'{') => Shape::Header,
        _ => Shape::Page,
    }
}

/// Resolve a possibly-relative `data_url`/header location against the base URL per RFC 3986
/// (handles relative, root-relative `/abs`, absolute, and `..` segments). Uses reqwest's
/// re-exported `url::Url`; falls back to the raw `rel` if the base is unparseable.
pub(crate) fn join_url(base: &str, rel: &str) -> String {
    match reqwest::Url::parse(base).and_then(|b| b.join(rel)) {
        Ok(u) => u.to_string(),
        Err(_) => rel.to_string(),
    }
}

/// Load the table registered as `address`, asking `get` for each address on the way.
pub(crate) fn fetch_with(address: &str, get: &mut dyn FnMut(&str) -> Result<Fetched, TableError>) -> Result<DifficultyTable, TableError> {
    let first = get(address).map_err(|e| e.at(TableStage::Address, address))?;
    match shape_of(&first.bytes) {
        Shape::Body => {
            let (entries, skipped) = parse_entries(&first.bytes).map_err(|e| e.at(TableStage::Body, &first.url))?;
            let header = get(&join_url(&first.url, SIBLING_HEADER)).ok().and_then(|sibling| parse_header(&sibling.bytes).ok());
            Ok(DifficultyTable::from_parts(header, entries).skipping(skipped))
        }
        Shape::Header => table_of_header(&first, get),
        Shape::Page => {
            let page = String::from_utf8_lossy(&first.bytes);
            let named = html::table_header_address(&page).ok_or_else(|| TableError::NoTableMeta.at(TableStage::Address, &first.url))?;
            let header_url = join_url(&first.url, &named);
            let header = get(&header_url).map_err(|e| e.at(TableStage::Header, &header_url))?;
            table_of_header(&header, get)
        }
    }
}

/// Read a header and every body it names. The bodies are relative to the header, not to the page
/// that named the header.
fn table_of_header(fetched: &Fetched, get: &mut dyn FnMut(&str) -> Result<Fetched, TableError>) -> Result<DifficultyTable, TableError> {
    let header: TableHeader = parse_header(&fetched.bytes).map_err(|e| e.at(TableStage::Header, &fetched.url))?;
    if header.data_url.is_empty() {
        return Err(TableError::MissingDataUrl.at(TableStage::Header, &fetched.url));
    }
    let mut entries: Vec<TableEntry> = Vec::new();
    let mut skipped = 0;
    for named in &header.data_url {
        let body_url = join_url(&fetched.url, named);
        let body = get(&body_url).map_err(|e| e.at(TableStage::Body, &body_url))?;
        let (more, dropped) = parse_entries(&body.bytes).map_err(|e| e.at(TableStage::Body, &body.url))?;
        entries.extend(more);
        skipped += dropped;
    }
    Ok(DifficultyTable::from_parts(Some(header), entries).skipping(skipped))
}

/// The client every request of one table load goes through.
pub(crate) fn build_client() -> Result<reqwest::blocking::Client, TableError> {
    Ok(reqwest::blocking::Client::builder().timeout(SILENCE_TIMEOUT).user_agent(USER_AGENT).build()?)
}

/// One request over the network for a table load that has to be over by `deadline`, refusing an
/// answer larger than a table is allowed to be.
pub(crate) fn http_get(client: &reqwest::blocking::Client, url: &str, deadline: Instant) -> Result<Fetched, TableError> {
    let mut left = || deadline.saturating_duration_since(Instant::now());
    if left().is_zero() {
        return Err(TableError::TimedOut { limit: LOAD_TIMEOUT });
    }
    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(TableError::HttpStatus(response.status()));
    }
    let landed = response.url().to_string();
    let bytes = read_within(response, &mut left)?;
    Ok(Fetched { bytes, url: landed })
}

/// All of `answer`, read a piece at a time for as long as `left` says the load has time left.
///
/// The time is asked for before each piece rather than measured here, so that an answer which never
/// stops arriving is tested without waiting for it.
pub(crate) fn read_within(mut answer: impl Read, left: &mut dyn FnMut() -> Duration) -> Result<Vec<u8>, TableError> {
    let mut bytes = Vec::new();
    let mut chunk = vec![0; READ_CHUNK_BYTES];
    loop {
        if left().is_zero() {
            return Err(TableError::TimedOut { limit: LOAD_TIMEOUT });
        }
        let read = match answer.read(&mut chunk) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(TableError::Read(error)),
        };
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(TableError::TooLarge { limit: MAX_RESPONSE_BYTES });
        }
    }
}

#[cfg(test)]
mod tests;
