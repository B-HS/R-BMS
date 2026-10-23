use reqwest::Url;

/// The longest address handed to the system's browser, counted in characters.
pub const MAX_BROWSABLE_URL_CHARS: usize = 2048;

const STELLA_HOST: &str = "stellabms.xyz";
const STELLA_UPLOAD_PREFIX: &str = "/upload/";
const ZIP_PATH_SUFFIX: &str = ".zip";

/// What a link from a table is, judged from the link alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// A file the player can fetch and unpack itself: it is a zip archive.
    DirectZip,
    /// Anything else with a web address: a page, a cloud folder, another kind of archive, a lone chart.
    Browser,
    /// Not a web address at all.
    Unusable,
}

/// Whether `url` is an address on the web: http or https, with a host.
pub(crate) fn is_web(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https") && url.host_str().is_some_and(|host| !host.is_empty())
}

/// `raw` as an address on the web. Spaces around it are not part of it.
pub(crate) fn web_url(raw: &str) -> Option<Url> {
    Url::parse(raw.trim()).ok().filter(is_web)
}

/// Judge what a link from a table leads to. Only the path decides: `page.html?file=a.zip` is a page.
pub fn classify_link(raw: &str) -> LinkKind {
    let Some(url) = web_url(raw) else {
        return LinkKind::Unusable;
    };
    let path = url.path();
    let is_stella_upload = url.host_str() == Some(STELLA_HOST) && path.strip_prefix(STELLA_UPLOAD_PREFIX).is_some_and(|rest| !rest.is_empty());
    if is_stella_upload || path.to_ascii_lowercase().ends_with(ZIP_PATH_SUFFIX) { LinkKind::DirectZip } else { LinkKind::Browser }
}

/// `raw` as an address that may be handed to the system's browser: http or https with a host,
/// no white space or control character, at most [`MAX_BROWSABLE_URL_CHARS`], normalised.
///
/// The result always begins with `http`, so it cannot be read as an option by the program that opens it.
pub fn browsable_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > MAX_BROWSABLE_URL_CHARS || trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let normalised = web_url(trimmed)?.to_string();
    (normalised.len() <= MAX_BROWSABLE_URL_CHARS).then_some(normalised)
}
