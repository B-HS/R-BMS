//! What a contract file is made of besides the run: a score server on loopback that answers like
//! the real one and keeps every request it is sent as the bytes that arrived, and the JSON a body is
//! read into, checked as, and written to the contract folder as.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

/// Where the capturing server listens: loopback, on a port the system picks.
const LOOPBACK: &str = "127.0.0.1:0";

/// How long a test waits for a request it expects before it calls the request lost.
const REQUEST_WAIT: Duration = Duration::from_secs(20);

/// The status lines the capturing server answers with.
const CREATED: &str = "201 Created";
const OK: &str = "200 OK";
const NOT_FOUND: &str = "404 Not Found";

/// What the capturing server says it is on `/health`: the deployment's own answer, with the flag that
/// says it takes traces.
const HEALTH: &str = r#"{"name":"rbms-ir","version":"0.1.0","ir_compat":"superset-1","capabilities":{"ranking":true,"player_best":true,"rivals":true,"courses":true,"replays":true,"tables":true,"settings_sync":true,"folders":true,"accounts":true,"lr2ir_compat":false,"traces":true}}"#;

/// The `updated_at` the capturing server says it stored a settings blob under.
const STORED_SETTINGS_STAMP: i64 = 1;

/// The largest whole number a JavaScript number holds exactly, `2^53 - 1`: what the server's Zod
/// `int()` takes.
pub(super) const MAX_SAFE_INTEGER: i128 = (1 << 53) - 1;

/// How wide a list or a map of plain values may be written on one line of a contract file.
const INLINE_WIDTH: usize = 120;

/// The indent of one level of a contract file.
const INDENT: &str = "    ";

/// One request as it reached the server.
pub(super) struct Request {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) body: String,
}

/// A score server on loopback. It says it takes traces, takes every score, course, replay and trace
/// as new, stores every settings blob, knows nothing else, and hands on each request it was sent in
/// the order they came.
pub(super) struct CaptureServer {
    pub(super) base: String,
    requests: Receiver<Request>,
}

impl CaptureServer {
    pub(super) fn spawn() -> CaptureServer {
        let listener = TcpListener::bind(LOOPBACK).expect("loopback binds");
        let base = format!("http://{}", listener.local_addr().expect("the bound address"));
        let (tx, requests) = channel();
        std::thread::spawn(move || {
            let mut answered = 0_u32;
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    return;
                };
                let Some(request) = read_request(&stream) else {
                    continue;
                };
                answered += 1;
                let _ = stream.write_all(answer(&request, answered).as_bytes());
                let _ = stream.flush();
                if tx.send(request).is_err() {
                    return;
                }
            }
        });
        CaptureServer { base, requests }
    }

    /// The next request of `method` to a path that starts with `path`, passing over every other.
    pub(super) fn next(&self, method: &str, path: &str) -> Request {
        let deadline = Instant::now() + REQUEST_WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let request = self.requests.recv_timeout(left).unwrap_or_else(|_| panic!("no {method} {path} reached the server"));
            if request.method == method && request.path.starts_with(path) {
                return request;
            }
        }
    }
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let mut parts = first.split_whitespace();
    let (method, path) = (parts.next()?.to_owned(), parts.next()?.to_owned());
    let mut length = 0_usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if line.trim_end().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0_u8; length];
    reader.read_exact(&mut body).ok()?;
    Some(Request { method, path, body: String::from_utf8(body).ok()? })
}

/// What the server answers `request`, the `nth` it was sent, in the shapes the real one answers.
fn answer(request: &Request, nth: u32) -> String {
    let stored = |id: String| {
        format!(r#"{{"accepted":true,"rank":1,"previous_best":null,"message":"saved","ranked":true,"flags":[],"is_new_best":true,"score_id":"{id}"}}"#)
    };
    let (status, body) = match (request.method.as_str(), request.path.as_str()) {
        ("POST", "/scores") => (CREATED, stored(format!("sc_{nth}"))),
        ("POST", "/courses") => (CREATED, stored(format!("cs_{nth}"))),
        ("POST", path) if path.starts_with("/charts/") && path.ends_with("/replays") => {
            (CREATED, format!(r#"{{"id":"rp_{nth}","url":"/replays/rp_{nth}","event_count":0}}"#))
        }
        ("POST", path) if path.starts_with("/charts/") && path.ends_with("/traces") => (CREATED, format!(r#"{{"id":"tr_{nth}"}}"#)),
        ("GET", "/health") => (OK, HEALTH.to_owned()),
        ("PUT", path) if path.contains("/settings/") => (OK, format!(r#"{{"updated_at":{STORED_SETTINGS_STAMP}}}"#)),
        _ => (NOT_FOUND, r#"{"success":false,"error":{"code":"NOT_FOUND","message":"not found"}}"#.to_owned()),
    };
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

/// A JSON value as it was written: numbers and strings keep the text they were sent as, so a file
/// says exactly what went over the wire.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Json {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    List(Vec<Json>),
    Map(Vec<(String, Json)>),
}

impl Json {
    /// Read `text`, which must be one JSON value and nothing more.
    pub(super) fn parse(text: &str) -> Json {
        let mut reader = JsonReader { bytes: text.as_bytes(), at: 0 };
        let value = reader.value();
        reader.skip_space();
        assert_eq!(reader.at, reader.bytes.len(), "text after the JSON value");
        value
    }

    /// The value at `key` of a map, to be changed.
    pub(super) fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
        match self {
            Json::Map(entries) => entries.iter_mut().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }

    /// The text of a string value, as it was escaped on the wire.
    pub(super) fn text(&self) -> Option<&str> {
        match self {
            Json::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The same value with every map's keys in order, which is how a contract file lists them.
    pub(super) fn sorted(self) -> Json {
        match self {
            Json::List(items) => Json::List(items.into_iter().map(Json::sorted).collect()),
            Json::Map(entries) => {
                let mut entries: Vec<_> = entries.into_iter().map(|(key, value)| (key, value.sorted())).collect();
                entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                Json::Map(entries)
            }
            scalar => scalar,
        }
    }

    /// Every place in the value that holds `null`, and every number, by its path (`options.hispeed`,
    /// `events[].t_us`).
    pub(super) fn leaves(&self) -> Vec<(String, &Json)> {
        let mut found = Vec::new();
        self.collect_leaves(String::new(), &mut found);
        found
    }

    fn collect_leaves<'a>(&'a self, path: String, found: &mut Vec<(String, &'a Json)>) {
        match self {
            Json::List(items) => items.iter().for_each(|item| item.collect_leaves(format!("{path}[]"), found)),
            Json::Map(entries) => entries.iter().for_each(|(key, value)| {
                let at = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                value.collect_leaves(at, found);
            }),
            leaf => found.push((path, leaf)),
        }
    }

    /// The value as a contract file holds it: indented, a list or map of plain values on one line
    /// when it fits, and a newline at the end.
    pub(super) fn render(&self) -> String {
        let mut out = String::new();
        self.write(0, &mut out);
        out.push('\n');
        out
    }

    fn is_scalar(&self) -> bool {
        !matches!(self, Json::List(_) | Json::Map(_))
    }

    fn inline(&self) -> String {
        match self {
            Json::Null => "null".to_owned(),
            Json::Bool(value) => value.to_string(),
            Json::Number(raw) => raw.clone(),
            Json::Text(raw) => format!("\"{raw}\""),
            Json::List(items) if items.is_empty() => "[]".to_owned(),
            Json::Map(entries) if entries.is_empty() => "{}".to_owned(),
            Json::List(items) => format!("[{}]", items.iter().map(Json::inline).collect::<Vec<_>>().join(", ")),
            Json::Map(entries) => {
                format!("{{ {} }}", entries.iter().map(|(key, value)| format!("\"{key}\": {}", value.inline())).collect::<Vec<_>>().join(", "))
            }
        }
    }

    fn write(&self, depth: usize, out: &mut String) {
        let plain = match self {
            Json::List(items) => items.iter().all(Json::is_scalar),
            Json::Map(entries) => entries.iter().all(|(_, value)| value.is_scalar()),
            _ => true,
        };
        let inline = self.inline();
        if plain && depth * INDENT.len() + inline.len() <= INLINE_WIDTH {
            out.push_str(&inline);
            return;
        }
        let inner = INDENT.repeat(depth + 1);
        let (open, close, count) = match self {
            Json::List(items) => ('[', ']', items.len()),
            Json::Map(entries) => ('{', '}', entries.len()),
            _ => {
                out.push_str(&inline);
                return;
            }
        };
        out.push(open);
        out.push('\n');
        for index in 0..count {
            out.push_str(&inner);
            match self {
                Json::List(items) => items[index].write(depth + 1, out),
                Json::Map(entries) => {
                    out.push_str(&format!("\"{}\": ", entries[index].0));
                    entries[index].1.write(depth + 1, out);
                }
                _ => {}
            }
            if index + 1 < count {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str(&INDENT.repeat(depth));
        out.push(close);
    }
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl JsonReader<'_> {
    fn skip_space(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_space();
        *self.bytes.get(self.at).expect("the JSON ends early")
    }

    fn expect(&mut self, byte: u8) {
        assert_eq!(self.peek(), byte, "malformed JSON at byte {}", self.at);
        self.at += 1;
    }

    fn word(&mut self, word: &str, value: Json) -> Json {
        assert!(self.bytes[self.at..].starts_with(word.as_bytes()), "malformed JSON at byte {}", self.at);
        self.at += word.len();
        value
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => self.map(),
            b'[' => self.list(),
            b'"' => Json::Text(self.string()),
            b't' => self.word("true", Json::Bool(true)),
            b'f' => self.word("false", Json::Bool(false)),
            b'n' => self.word("null", Json::Null),
            _ => self.number(),
        }
    }

    fn string(&mut self) -> String {
        self.expect(b'"');
        let start = self.at;
        while self.bytes[self.at] != b'"' {
            self.at += if self.bytes[self.at] == b'\\' { 2 } else { 1 };
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.at]).expect("the JSON is UTF-8").to_owned();
        self.at += 1;
        raw
    }

    fn number(&mut self) -> Json {
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(|byte| byte.is_ascii_digit() || b"+-.eE".contains(byte)) {
            self.at += 1;
        }
        assert!(self.at > start, "malformed JSON at byte {start}");
        Json::Number(std::str::from_utf8(&self.bytes[start..self.at]).expect("digits").to_owned())
    }

    fn list(&mut self) -> Json {
        self.expect(b'[');
        let mut items = Vec::new();
        if self.peek() == b']' {
            self.at += 1;
            return Json::List(items);
        }
        loop {
            items.push(self.value());
            match self.peek() {
                b',' => self.at += 1,
                _ => {
                    self.expect(b']');
                    return Json::List(items);
                }
            }
        }
    }

    fn map(&mut self) -> Json {
        self.expect(b'{');
        let mut entries = Vec::new();
        if self.peek() == b'}' {
            self.at += 1;
            return Json::Map(entries);
        }
        loop {
            self.skip_space();
            let key = self.string();
            self.expect(b':');
            entries.push((key, self.value()));
            match self.peek() {
                b',' => self.at += 1,
                _ => {
                    self.expect(b'}');
                    return Json::Map(entries);
                }
            }
        }
    }
}
