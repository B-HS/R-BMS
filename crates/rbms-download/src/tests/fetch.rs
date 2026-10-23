use std::cell::Cell;
use std::fs;
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use super::{ArchiveBuilder, CHART, Fixture, not_cancelled};
use crate::fetch::download_within;
use crate::{Answer, DOWNLOAD_MAX_BYTES, DownloadError, DownloadLimits, Progress, download_to, download_with};

const URL: &str = "https://files.test/pack.zip";
const ZIP_HEAD: [u8; 4] = *b"PK\x03\x04";
const SMALL_CAP_BYTES: u64 = 100 * 1024;
const TEST_TOTAL: Duration = Duration::from_secs(600);
const LOOPBACK_SILENCE: Duration = Duration::from_secs(10);

fn limits() -> DownloadLimits {
    DownloadLimits { max_bytes: SMALL_CAP_BYTES, silence: LOOPBACK_SILENCE, total: TEST_TOTAL }
}

fn a_zip() -> Vec<u8> {
    ArchiveBuilder::new().deflated("a.bms", CHART).build()
}

fn answer(bytes: Vec<u8>) -> Answer {
    Answer { url: URL.to_string(), content_length: Some(bytes.len() as u64), body: Box::new(Cursor::new(bytes)) }
}

fn unannounced(body: impl Read + 'static) -> Answer {
    Answer { url: URL.to_string(), content_length: None, body: Box::new(body) }
}

fn run(file: &std::path::Path, cancel: &AtomicBool, answer: Answer) -> (Result<(), DownloadError>, Progress) {
    let progress = Progress::default();
    let mut once = Some(answer);
    let result = download_with(URL, file, &limits(), &progress, cancel, &mut |_| Ok(once.take().expect("asked once")));
    (result, progress)
}

/// A body that begins like a zip archive and never ends.
struct Endless {
    position: usize,
}

impl Read for Endless {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for byte in buf.iter_mut() {
            *byte = ZIP_HEAD.get(self.position).copied().unwrap_or(0);
            self.position += 1;
        }
        Ok(buf.len())
    }
}

/// A body that says whether anyone read it.
struct Watched(Rc<Cell<bool>>);

impl Read for Watched {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        self.0.set(true);
        Ok(0)
    }
}

/// A body that asks for the download to be stopped after its first piece.
struct StopsAfterFirst {
    cancel: Arc<AtomicBool>,
    reads: usize,
}

impl Read for StopsAfterFirst {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reads += 1;
        if self.reads > 2 {
            self.cancel.store(true, Ordering::Relaxed);
        }
        let piece = if self.reads == 1 { &ZIP_HEAD[..] } else { &[0u8; 16][..] };
        let given = piece.len().min(buf.len());
        buf[..given].copy_from_slice(&piece[..given]);
        Ok(given)
    }
}

#[test]
fn a_zip_answer_is_written_to_the_file_and_counted() {
    let fixture = Fixture::new("fetch-ok");
    let file = fixture.path("staging").join("pack.zip");
    let bytes = a_zip();
    let (result, progress) = run(&file, &not_cancelled(), answer(bytes.clone()));
    result.expect("the download succeeds");
    assert_eq!(fs::read(&file).expect("the file is there"), bytes);
    assert_eq!(progress.received.load(Ordering::Relaxed), bytes.len() as u64);
    assert_eq!(progress.expected.load(Ordering::Relaxed), bytes.len() as u64);
    assert_eq!(progress.phase.load(Ordering::Relaxed), Progress::DOWNLOADING);
}

#[test]
fn an_empty_zip_archive_is_taken() {
    let fixture = Fixture::new("fetch-empty-zip");
    let file = fixture.path("empty.zip");
    let mut bytes = b"PK\x05\x06".to_vec();
    bytes.extend_from_slice(&[0; 18]);
    run(&file, &not_cancelled(), answer(bytes.clone())).0.expect("an empty archive is an archive");
    assert_eq!(fs::read(&file).expect("the file is there"), bytes);
}

#[test]
fn a_download_past_the_cap_is_stopped_and_leaves_no_file() {
    let fixture = Fixture::new("fetch-cap");
    let file = fixture.path("pack.zip");
    let (result, progress) = run(&file, &not_cancelled(), unannounced(Endless { position: 0 }));
    assert!(matches!(result, Err(DownloadError::TooLarge { limit: SMALL_CAP_BYTES })), "{result:?}");
    assert!(!file.exists());
    assert!(progress.received.load(Ordering::Relaxed) <= SMALL_CAP_BYTES);
}

#[test]
fn a_download_that_ends_exactly_at_the_cap_is_taken() {
    let fixture = Fixture::new("fetch-at-cap");
    let file = fixture.path("pack.zip");
    let mut bytes = ZIP_HEAD.to_vec();
    bytes.resize(SMALL_CAP_BYTES as usize, 0);
    run(&file, &not_cancelled(), unannounced(Cursor::new(bytes))).0.expect("the cap itself is allowed");
    assert_eq!(fs::metadata(&file).expect("the file is there").len(), SMALL_CAP_BYTES);
}

#[test]
fn a_content_length_past_the_cap_is_refused_before_reading() {
    let fixture = Fixture::new("fetch-announced");
    let file = fixture.path("pack.zip");
    let was_read = Rc::new(Cell::new(false));
    let announced = Answer { url: URL.to_string(), content_length: Some(SMALL_CAP_BYTES + 1), body: Box::new(Watched(Rc::clone(&was_read))) };
    let (result, _) = run(&file, &not_cancelled(), announced);
    assert!(matches!(result, Err(DownloadError::TooLarge { limit: SMALL_CAP_BYTES })), "{result:?}");
    assert!(!was_read.get());
    assert!(!file.exists());
}

#[test]
fn a_server_that_goes_silent_runs_out_of_time() {
    let fixture = Fixture::new("fetch-time");
    let file = fixture.path("pack.zip");
    let progress = Progress::default();
    let mut asked = 0;
    let mut left = || {
        asked += 1;
        if asked > 2 { Duration::ZERO } else { Duration::from_secs(1) }
    };
    let mut once = Some(unannounced(Endless { position: 0 }));
    let result = download_within(URL, &file, &limits(), &progress, &not_cancelled(), &mut left, &mut |_| Ok(once.take().expect("asked once")));
    assert!(matches!(result, Err(DownloadError::TimedOut { limit: TEST_TOTAL })), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn an_html_answer_is_not_a_zip() {
    let fixture = Fixture::new("fetch-html");
    let file = fixture.path("pack.zip");
    let page = b"<!doctype html><title>Sign in to download</title>".to_vec();
    let (result, _) = run(&file, &not_cancelled(), answer(page));
    assert!(matches!(result, Err(DownloadError::NotAZip)), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn an_answer_shorter_than_a_zip_header_is_not_a_zip() {
    let fixture = Fixture::new("fetch-short");
    let file = fixture.path("pack.zip");
    for body in [&b""[..], &b"P"[..], &b"PK"[..], &b"PK\x03"[..]] {
        let (result, _) = run(&file, &not_cancelled(), answer(body.to_vec()));
        assert!(matches!(result, Err(DownloadError::NotAZip)), "{body:?}: {result:?}");
        assert!(!file.exists());
    }
}

#[test]
fn a_cancelled_download_leaves_nothing() {
    let fixture = Fixture::new("fetch-cancel");
    let file = fixture.path("pack.zip");
    let cancelled_before = AtomicBool::new(true);
    let asked = Rc::new(Cell::new(false));
    let progress = Progress::default();
    let result = download_with(URL, &file, &limits(), &progress, &cancelled_before, &mut |_| {
        asked.set(true);
        Ok(answer(a_zip()))
    });
    assert!(matches!(result, Err(DownloadError::Cancelled)), "{result:?}");
    assert!(!asked.get() && !file.exists());

    let cancel = Arc::new(AtomicBool::new(false));
    let midway = unannounced(StopsAfterFirst { cancel: Arc::clone(&cancel), reads: 0 });
    let (result, _) = run(&file, &cancel, midway);
    assert!(matches!(result, Err(DownloadError::Cancelled)), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn a_redirect_to_another_scheme_is_refused() {
    let fixture = Fixture::new("fetch-scheme");
    let file = fixture.path("pack.zip");
    let was_read = Rc::new(Cell::new(false));
    let landed = Answer { url: "ftp://files.test/pack.zip".to_string(), content_length: None, body: Box::new(Watched(Rc::clone(&was_read))) };
    let (result, _) = run(&file, &not_cancelled(), landed);
    assert!(matches!(result, Err(DownloadError::NotWebAddress)), "{result:?}");
    assert!(!was_read.get() && !file.exists());
}

#[test]
fn an_address_that_is_not_on_the_web_is_refused_before_asking() {
    let fixture = Fixture::new("fetch-address");
    let file = fixture.path("pack.zip");
    let asked = Cell::new(false);
    for url in ["file:///etc/passwd", "ftp://files.test/a.zip", "javascript:alert(1)", "files.test/a.zip", ""] {
        let result = download_with(url, &file, &limits(), &Progress::default(), &not_cancelled(), &mut |_| {
            asked.set(true);
            Ok(answer(a_zip()))
        });
        assert!(matches!(result, Err(DownloadError::NotWebAddress)), "{url:?}: {result:?}");
    }
    assert!(!asked.get() && !file.exists());
}

#[test]
fn a_failed_request_is_passed_on_and_leaves_nothing() {
    let fixture = Fixture::new("fetch-failed");
    let file = fixture.path("pack.zip");
    let result =
        download_with(URL, &file, &limits(), &Progress::default(), &not_cancelled(), &mut |_| Err(DownloadError::HttpStatus(reqwest::StatusCode::NOT_FOUND)));
    assert!(matches!(result, Err(DownloadError::HttpStatus(status)) if status == reqwest::StatusCode::NOT_FOUND), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn the_default_limits_are_the_ones_the_player_uses() {
    let limits = DownloadLimits::default();
    assert_eq!(limits.max_bytes, DOWNLOAD_MAX_BYTES);
    assert_eq!(DOWNLOAD_MAX_BYTES, 512 * 1024 * 1024);
}

fn read_request_head(stream: &TcpStream) {
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            return;
        }
    }
}

fn reply(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n", body.len());
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let mut bytes = head.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// A server on loopback that answers one connection with each reply in turn. The thread is left to
/// end on its own, as it would block forever on a connection that a failing test never makes.
fn serve(replies: impl FnOnce(&str) -> Vec<Vec<u8>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let base = format!("http://{}", listener.local_addr().expect("local address"));
    let replies = replies(&base);
    thread::spawn(move || {
        for reply in replies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            read_request_head(&stream);
            let _ = stream.write_all(&reply);
            let _ = stream.flush();
        }
    });
    base
}

fn fetch_over_loopback(base: &str, path: &str, file: &std::path::Path) -> Result<(), DownloadError> {
    download_to(&format!("{base}{path}"), file, &limits(), &Progress::default(), &not_cancelled())
}

#[test]
fn a_zip_is_fetched_over_http() {
    let fixture = Fixture::new("net-ok");
    let file = fixture.path("pack.zip");
    let bytes = a_zip();
    let base = serve(|_| vec![reply("200 OK", &[("Content-Type", "application/zip")], &bytes)]);
    fetch_over_loopback(&base, "/pack.zip", &file).expect("the download succeeds");
    assert_eq!(fs::read(&file).expect("the file is there"), bytes);
}

#[test]
fn a_redirect_that_stays_on_the_web_is_followed() {
    let fixture = Fixture::new("net-redirect");
    let file = fixture.path("pack.zip");
    let bytes = a_zip();
    let base = serve(|base| vec![reply("302 Found", &[("Location", &format!("{base}/real.zip"))], b""), reply("200 OK", &[], &bytes)]);
    fetch_over_loopback(&base, "/pack.zip", &file).expect("the download succeeds");
    assert_eq!(fs::read(&file).expect("the file is there"), bytes);
}

#[test]
fn a_redirect_off_the_web_is_not_followed() {
    let fixture = Fixture::new("net-redirect-off");
    let file = fixture.path("pack.zip");
    let base = serve(|_| vec![reply("302 Found", &[("Location", "ftp://files.test/pack.zip")], b"")]);
    let result = fetch_over_loopback(&base, "/pack.zip", &file);
    assert!(matches!(&result, Err(DownloadError::Request(error)) if error.is_redirect()), "{result:?}");
    assert!(!file.exists());
}

const REDIRECTS_FOLLOWED: usize = 10;

fn redirecting_then_serving(base: &str, redirects: usize, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut replies: Vec<Vec<u8>> = (0..redirects).map(|_| reply("302 Found", &[("Location", &format!("{base}/again"))], b"")).collect();
    replies.push(reply("200 OK", &[], bytes));
    replies
}

#[test]
fn ten_redirects_are_followed() {
    let fixture = Fixture::new("net-redirects-ten");
    let file = fixture.path("pack.zip");
    let bytes = a_zip();
    let base = serve(|base| redirecting_then_serving(base, REDIRECTS_FOLLOWED, &bytes));
    fetch_over_loopback(&base, "/pack.zip", &file).expect("the download succeeds");
    assert_eq!(fs::read(&file).expect("the file is there"), bytes);
}

#[test]
fn an_eleventh_redirect_is_given_up_on() {
    let fixture = Fixture::new("net-redirects-eleven");
    let file = fixture.path("pack.zip");
    let base = serve(|base| redirecting_then_serving(base, REDIRECTS_FOLLOWED + 1, &a_zip()));
    let result = fetch_over_loopback(&base, "/pack.zip", &file);
    assert!(matches!(&result, Err(DownloadError::Request(error)) if error.is_redirect()), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn an_error_status_is_reported_and_leaves_nothing() {
    let fixture = Fixture::new("net-status");
    let file = fixture.path("pack.zip");
    let base = serve(|_| vec![reply("404 Not Found", &[], b"gone")]);
    let result = fetch_over_loopback(&base, "/pack.zip", &file);
    assert!(matches!(&result, Err(DownloadError::HttpStatus(status)) if *status == reqwest::StatusCode::NOT_FOUND), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn a_page_served_in_place_of_the_file_is_not_a_zip() {
    let fixture = Fixture::new("net-page");
    let file = fixture.path("pack.zip");
    let base = serve(|_| vec![reply("200 OK", &[("Content-Type", "text/html")], b"<html>please log in</html>")]);
    let result = fetch_over_loopback(&base, "/pack.zip", &file);
    assert!(matches!(result, Err(DownloadError::NotAZip)), "{result:?}");
    assert!(!file.exists());
}

#[test]
fn an_announced_size_past_the_cap_is_refused_over_http() {
    let fixture = Fixture::new("net-announced");
    let file = fixture.path("pack.zip");
    let mut body = ZIP_HEAD.to_vec();
    body.resize(SMALL_CAP_BYTES as usize + 1, 0);
    let base = serve(|_| vec![reply("200 OK", &[], &body)]);
    let result = fetch_over_loopback(&base, "/pack.zip", &file);
    assert!(matches!(result, Err(DownloadError::TooLarge { limit: SMALL_CAP_BYTES })), "{result:?}");
    assert!(!file.exists());
}
