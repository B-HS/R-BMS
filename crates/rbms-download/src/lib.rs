#![forbid(unsafe_code)]

//! Getting a chart archive that a difficulty table links to, and unpacking it without trusting it.
//!
//! A link is judged from its address alone ([`classify_link`]): only a direct zip archive is fetched
//! by the player, and everything else is left to the system's browser ([`browsable_url`] vets the
//! address first). A fetched archive is held to limits on its size and time ([`download_to`]) and
//! unpacked into a folder it cannot leave ([`extract_zip`]), after which [`settle`] gives the folder
//! a name that takes nothing else's place.

mod error;
mod extract;
mod fetch;
mod link;
mod name;

pub use error::DownloadError;
pub use extract::{EXTRACT_MAX_ENTRIES, EXTRACT_MAX_ENTRY_BYTES, EXTRACT_MAX_TOTAL_BYTES, ExtractLimits, ExtractReport, extract_zip, is_kept_extension};
pub use fetch::{Answer, DOWNLOAD_MAX_BYTES, DOWNLOAD_SILENCE, DOWNLOAD_TOTAL, DownloadLimits, Progress, download_to, download_with};
pub use link::{LinkKind, MAX_BROWSABLE_URL_CHARS, browsable_url, classify_link};
pub use name::{safe_folder_name, settle};

const BYTES_PER_MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * BYTES_PER_MIB;

#[cfg(test)]
mod tests;
