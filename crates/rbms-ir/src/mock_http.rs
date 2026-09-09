use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::HttpScoreServer;

/// Per-request budget used by the tests; short enough that the timeout case stays fast.
pub const TEST_TIMEOUT: Duration = Duration::from_millis(300);
/// How long a test waits for the server thread to hand over a captured request.
pub const CAPTURE_WAIT: Duration = Duration::from_secs(10);

/// Minimal single-shot HTTP/1.1 server on loopback: serves the canned responses in order,
/// one per connection, and reports each raw request (head + body) back over a channel.
///
/// The server thread is deliberately detached. Joining it from `Drop` would block forever on
/// `accept()` whenever a test unwinds before connecting, and the test harness has no
/// per-test timeout to break that out.
pub struct TestServer {
    pub base: String,
    requests: mpsc::Receiver<String>,
}

impl TestServer {
    pub fn spawn(responses: Vec<String>) -> TestServer {
        TestServer::spawn_with_delay(responses, Duration::from_millis(0))
    }

    pub fn spawn_with_delay(responses: Vec<String>, delay: Duration) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");
        let (tx, requests) = mpsc::channel();
        thread::spawn(move || {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                if let Some(req) = read_request(&stream) {
                    let _ = tx.send(req);
                }
                thread::sleep(delay);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        TestServer { base: format!("http://{addr}"), requests }
    }

    pub fn client(&self) -> HttpScoreServer {
        HttpScoreServer::try_with_timeout(&self.base, None, TEST_TIMEOUT).expect("client builds")
    }

    pub fn client_with_token(&self, token: &str) -> HttpScoreServer {
        HttpScoreServer::try_with_timeout(&self.base, Some(token.to_string()), TEST_TIMEOUT).expect("client builds")
    }

    pub fn next_request(&self) -> String {
        self.requests.recv_timeout(CAPTURE_WAIT).expect("server captured a request")
    }
}

fn read_request(stream: &TcpStream) -> Option<String> {
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let end_of_head = line == "\r\n" || line == "\n";
        head.push_str(&line);
        if end_of_head {
            break;
        }
    }
    let mut content_length = 0usize;
    for line in head.lines() {
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).ok()?;
    }
    head.push_str(&String::from_utf8_lossy(&body));
    Some(head)
}

pub fn response(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

pub fn ok(body: &str) -> String {
    response("200 OK", body)
}

pub fn created(body: &str) -> String {
    response("201 Created", body)
}

pub fn no_content() -> String {
    "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
}
