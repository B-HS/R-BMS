//! The presence without a real Discord: the frames and the messages, what each screen says, the
//! waits and the rate, the connection against a stand-in app on a loopback socket (and on a Unix
//! socket), and the application's side.
//!
//! Time is the instant a test hands in; nothing sleeps. A stand-in app and the link take turns on
//! the socket, so they are in step without waiting on a clock. The read limit on either side is only
//! there so a failing test fails instead of hanging.

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::activity::{Assets, ChartLine, CourseStage, LARGE_IMAGE_KEY, MAX_TEXT_BYTES, PresenceView, RunKind, Screen, Starts, Timestamps, fit_text};
use super::ipc::{self, Connect, EVT_ERROR, FrameError, HEADER_LEN, IpcStream, MAX_FRAME_BYTES, Message, Opcode};
use super::link::{Backoff, CLOSE_INVALID_CLIENT_ID, Link, RATE_LIMIT, RATE_WINDOW, RECONNECT_FIRST, RECONNECT_MAX, REFRESH_INTERVAL, RateLimit};
use super::*;
use crate::stage::{DecideState, LoadingState, SelectState, SettingsState};
use crate::{App, LaunchOptions, Transition};

/// How long either end waits for the other before the test is failed rather than hung.
const SAFETY_LIMIT: Duration = Duration::from_secs(5);

/// The process id the link is built with.
const TEST_PID: u32 = 4242;

/// The instants a test starts its clock at.
const APP_START: u64 = 1_700_000_000;
const RUN_START: u64 = 1_700_000_100;
const COURSE_START: u64 = 1_700_000_050;

/// How many times the tests that pass through a socket are run in one go, to show they do not
/// depend on how the two ends happen to be scheduled.
const REPEATS: usize = 5;

impl IpcStream for TcpStream {
    fn set_read_limit(&self, limit: Option<Duration>) -> io::Result<()> {
        self.set_read_timeout(limit)
    }
}

/// Connects to a stand-in app on the loopback, counting the attempts.
struct TcpConnector {
    addr: SocketAddr,
    attempts: Arc<AtomicUsize>,
}

impl Connect for TcpConnector {
    fn connect(&mut self) -> io::Result<Box<dyn IpcStream>> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(TcpStream::connect(self.addr)?))
    }
}

/// Refuses every connection, counting the attempts.
struct RefusingConnector {
    attempts: Arc<AtomicUsize>,
}

impl Connect for RefusingConnector {
    fn connect(&mut self) -> io::Result<Box<dyn IpcStream>> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::new(io::ErrorKind::ConnectionRefused, "nothing is listening"))
    }
}

fn activity(details: &str) -> Activity {
    Activity {
        details: Some(details.to_string()),
        state: Some("7K ANOTHER 12".to_string()),
        timestamps: Timestamps { start: RUN_START },
        assets: Assets { large_image: LARGE_IMAGE_KEY, large_text: Some("R-BMS".to_string()) },
        instance: false,
    }
}

fn starts() -> Starts {
    Starts { app: APP_START, run: Some(RUN_START), course: Some(COURSE_START) }
}

fn chart<'a>(title: &'a str, subtitle: &'a str) -> ChartLine<'a> {
    ChartLine { title, subtitle, mode: Some(rbms_model::Mode::BEAT_7K), difficulty: 4, level: "12".to_string(), table_level: String::new() }
}

fn view(screen: Screen<'_>) -> PresenceView<'_> {
    PresenceView { screen, course: None }
}

fn lines(activity: &Activity) -> (Option<&str>, Option<&str>, Option<&str>, u64) {
    (activity.details.as_deref(), activity.state.as_deref(), activity.assets.large_text.as_deref(), activity.timestamps.start)
}

fn app_label() -> String {
    format!("R-BMS {}", env!("CARGO_PKG_VERSION"))
}

#[test]
fn a_frame_is_its_opcode_and_length_little_endian_then_its_body_in_one_buffer() {
    let frame = ipc::encode(Opcode::Frame, b"{}");
    assert_eq!(frame, [1, 0, 0, 0, 2, 0, 0, 0, b'{', b'}']);
    assert_eq!(ipc::encode(Opcode::Handshake, b"").len(), HEADER_LEN);
    assert_eq!(ipc::encode(Opcode::Pong, b"x")[..4], 4u32.to_le_bytes());
}

#[test]
fn a_frame_is_read_whole_or_not_at_all_and_a_bad_head_is_refused() {
    let frame = ipc::encode(Opcode::Ping, b"abc");
    let (message, used) = ipc::decode(&frame).expect("a frame").expect("whole");
    assert_eq!(message, Message { opcode: Opcode::Ping, body: b"abc".to_vec() });
    assert_eq!(used, frame.len());
    assert_eq!(ipc::decode(&frame[..HEADER_LEN - 1]), Ok(None), "a part of a head is not a frame yet");
    assert_eq!(ipc::decode(&frame[..frame.len() - 1]), Ok(None), "a part of a body is not a frame yet");
    let mut long = ipc::encode(Opcode::Frame, b"");
    let too_long = (MAX_FRAME_BYTES + 1) as u32;
    long[4..HEADER_LEN].copy_from_slice(&too_long.to_le_bytes());
    assert_eq!(ipc::decode(&long), Err(FrameError::TooLong(too_long)));
    let mut unknown = ipc::encode(Opcode::Frame, b"");
    unknown[..4].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(ipc::decode(&unknown), Err(FrameError::UnknownOpcode(9)));
    assert!(ipc::read_message(&mut &long[..]).is_err(), "a stream with a frame too long is broken");
}

#[test]
fn the_handshake_names_the_version_and_the_application() {
    assert_eq!(ipc::handshake_body(DISCORD_APPLICATION_ID), r#"{"v":1,"client_id":"1563156237539999865"}"#);
}

#[test]
fn set_activity_carries_the_pid_a_string_nonce_and_only_the_fields_that_are_set() {
    let mut shown = activity("Song");
    shown.state = None;
    let body: Value = serde_json::from_str(&ipc::set_activity_body(TEST_PID, Some(&shown), 7)).expect("JSON");
    assert_eq!(
        body,
        json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": TEST_PID,
                "activity": {
                    "details": "Song",
                    "timestamps": { "start": RUN_START },
                    "assets": { "large_image": "rbms", "large_text": "R-BMS" },
                    "instance": false
                }
            },
            "nonce": "7"
        })
    );
    assert!(body["args"]["activity"].get("type").is_none(), "the type is left to the app, which shows Playing");
    let cleared: Value = serde_json::from_str(&ipc::set_activity_body(TEST_PID, None, 8)).expect("JSON");
    assert_eq!(cleared, json!({ "cmd": "SET_ACTIVITY", "args": { "pid": TEST_PID }, "nonce": "8" }), "clearing sends no activity");
}

#[test]
fn a_text_is_trimmed_quoted_when_short_and_cut_on_a_character_boundary_when_long() {
    assert_eq!(fit_text("   "), None);
    assert_eq!(fit_text("A").as_deref(), Some("\"A\""));
    assert_eq!(fit_text(" OK ").as_deref(), Some("OK"));
    let exact = "a".repeat(MAX_TEXT_BYTES);
    assert_eq!(fit_text(&exact).as_deref(), Some(exact.as_str()));
    let japanese = "譜面".repeat(MAX_TEXT_BYTES);
    let cut = fit_text(&japanese).expect("a long title is kept");
    assert!(cut.len() <= MAX_TEXT_BYTES, "{} bytes", cut.len());
    assert!(cut.ends_with("..."));
    assert!(cut.trim_end_matches("...").chars().all(|c| c == '譜' || c == '面'), "a character was split: {cut}");
    assert!(cut.chars().count() >= super::activity::MIN_TEXT_CHARS);
    let shifted = format!("a{}", "譜".repeat(MAX_TEXT_BYTES));
    assert!(fit_text(&shifted).expect("kept").len() <= MAX_TEXT_BYTES);
}

#[test]
fn every_screen_says_what_the_table_of_the_design_says() {
    let s = starts();
    let app = app_label();
    let browsing = activity_for(&view(Screen::Browsing), &s);
    assert_eq!(lines(&browsing), (Some("Choosing a song"), None, Some(app.as_str()), APP_START));
    assert_eq!(browsing.assets.large_image, "rbms");
    assert_eq!(lines(&activity_for(&view(Screen::Menus), &s)), (Some("In the menus"), None, Some(app.as_str()), APP_START));
    assert_eq!(lines(&activity_for(&view(Screen::Scanning), &s)), (Some("Scanning the song folders"), None, Some(app.as_str()), APP_START));

    let loading = activity_for(&view(Screen::Loading(chart("Song", "[SPA]"))), &s);
    assert_eq!(lines(&loading), (Some("Song [SPA]"), Some("Loading · 7K ANOTHER 12"), Some(app.as_str()), APP_START));

    let tabled = ChartLine { table_level: "sl5".to_string(), ..chart("Song", "") };
    let by_hand = activity_for(&view(Screen::Playing { chart: tabled, kind: RunKind::ByHand }), &s);
    assert_eq!(lines(&by_hand), (Some("Song"), Some("7K ANOTHER 12 · sl5"), Some(app.as_str()), RUN_START));
    for (kind, said) in [
        (RunKind::Autoplay, "Autoplay · 7K ANOTHER 12"),
        (RunKind::Replay, "Watching a replay · 7K ANOTHER 12"),
        (RunKind::Practice, "Practice · 7K ANOTHER 12"),
    ] {
        let run = activity_for(&view(Screen::Playing { chart: chart("Song", ""), kind }), &s);
        assert_eq!(lines(&run), (Some("Song"), Some(said), Some(app.as_str()), RUN_START), "{kind:?}");
    }
    let panel = activity_for(&view(Screen::PracticePanel(chart("Song", ""))), &s);
    assert_eq!(lines(&panel), (Some("Song"), Some("Practice · 7K ANOTHER 12"), Some(app.as_str()), APP_START));

    let unnamed = ChartLine { mode: Some(rbms_model::Mode::POPN_9K), difficulty: 0, level: String::new(), ..chart("Song", "") };
    let unnamed_mode = ChartLine { mode: Some(rbms_model::Mode { key: 3, ..rbms_model::Mode::BEAT_7K }), ..chart("Song", "") };
    assert_eq!(activity_for(&view(Screen::Playing { chart: unnamed, kind: RunKind::ByHand }), &s).state.as_deref(), Some("9K"));
    assert_eq!(activity_for(&view(Screen::Playing { chart: unnamed_mode, kind: RunKind::ByHand }), &s).state.as_deref(), Some("ANOTHER 12"));

    let result = Screen::Result { title: "Song", clear_label: "HARD CLEAR", mode_label: "7K", chart: None };
    assert_eq!(lines(&activity_for(&view(result), &s)), (Some("Song"), Some("Result · HARD CLEAR"), Some("7K"), APP_START));

    let course = Some(CourseStage { name: "Dan 5", number: 2, count: 4 });
    let in_course = PresenceView { screen: Screen::Playing { chart: chart("Song", ""), kind: RunKind::ByHand }, course };
    assert_eq!(lines(&activity_for(&in_course, &s)), (Some("2/4 · Dan 5"), Some("Song"), Some("7K ANOTHER 12"), COURSE_START));
    let decided = PresenceView { screen: Screen::Loading(chart("Song", "")), course };
    assert_eq!(lines(&activity_for(&decided, &s)), (Some("2/4 · Dan 5"), Some("Song"), Some("7K ANOTHER 12"), COURSE_START));
    let stage_result =
        PresenceView { screen: Screen::Result { title: "Song", clear_label: "CLEAR", mode_label: "7K", chart: Some(chart("Song", "")) }, course };
    assert_eq!(lines(&activity_for(&stage_result, &s)), (Some("2/4 · Dan 5"), Some("Song"), Some("7K ANOTHER 12"), COURSE_START));

    let cleared = activity_for(&view(Screen::CourseResult { name: "Dan 5", cleared: true }), &s);
    assert_eq!(lines(&cleared), (Some("Dan 5"), Some("Course cleared"), Some(app.as_str()), APP_START));
    let failed = activity_for(&view(Screen::CourseResult { name: "Dan 5", cleared: false }), &s);
    assert_eq!(failed.state.as_deref(), Some("Course failed"));
}

#[test]
fn the_wait_between_attempts_doubles_to_a_minute_and_starts_over_after_a_connection() {
    let mut backoff = Backoff::new();
    let waits: Vec<Duration> = (0..9).map(|_| backoff.take()).collect();
    let seconds = |s: f64| Duration::from_secs_f64(s);
    assert_eq!(waits, [0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 60.0, 60.0].map(seconds));
    backoff.reset();
    assert_eq!(backoff.take(), RECONNECT_FIRST);
    assert_eq!(backoff.give_up(), RECONNECT_MAX);
    assert_eq!(backoff.take(), RECONNECT_MAX, "a refusal no attempt can change keeps the longest wait");
}

#[test]
fn no_more_than_five_updates_go_in_twenty_seconds() {
    let t0 = Instant::now();
    let mut limit = RateLimit::default();
    for second in 0..RATE_LIMIT as u64 {
        let at = t0 + Duration::from_secs(second);
        assert_eq!(limit.next_free(at), at);
        limit.record(at);
    }
    let sixth = t0 + Duration::from_secs(RATE_LIMIT as u64);
    assert_eq!(limit.next_free(sixth), t0 + RATE_WINDOW, "the sixth waits for the first to leave the window");
    assert_eq!(limit.next_free(t0 + RATE_WINDOW), t0 + RATE_WINDOW);
}

#[test]
fn the_unix_sockets_are_looked_for_under_the_first_folder_named_and_the_sandboxes_under_the_runtime_folder() {
    let env =
        |pairs: &'static [(&'static str, &'static str)]| move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| OsString::from(value));
    let all = ipc::unix_candidates(env(&[("XDG_RUNTIME_DIR", "/run/user/1000"), ("TMPDIR", "/t"), ("TMP", "/u"), ("TEMP", "/v")]));
    assert_eq!(all.len(), 30);
    assert_eq!(all[0], PathBuf::from("/run/user/1000/discord-ipc-0"));
    assert_eq!(all[9], PathBuf::from("/run/user/1000/discord-ipc-9"));
    assert_eq!(all[10], PathBuf::from("/run/user/1000/app/com.discordapp.Discord/discord-ipc-0"));
    assert_eq!(all[20], PathBuf::from("/run/user/1000/snap.discord/discord-ipc-0"));
    let tmpdir = ipc::unix_candidates(env(&[("TMPDIR", "/t"), ("TMP", "/u")]));
    assert_eq!(tmpdir.len(), 10, "the sandboxes are looked for only under the runtime folder");
    assert_eq!(tmpdir[3], PathBuf::from("/t/discord-ipc-3"));
    assert_eq!(ipc::unix_candidates(env(&[("TMP", "/u"), ("TEMP", "/v")]))[0], PathBuf::from("/u/discord-ipc-0"));
    assert_eq!(ipc::unix_candidates(env(&[("TEMP", "/v")]))[0], PathBuf::from("/v/discord-ipc-0"));
    assert_eq!(ipc::unix_candidates(env(&[]))[0], PathBuf::from("/tmp/discord-ipc-0"));
}

#[test]
fn the_windows_pipes_are_numbered_nought_to_nine() {
    let names = ipc::windows_pipe_names();
    assert_eq!(names.len(), 10);
    assert_eq!(names[0], PathBuf::from(r"\\?\pipe\discord-ipc-0"));
    assert_eq!(names[9], PathBuf::from(r"\\?\pipe\discord-ipc-9"));
}

#[test]
fn the_connector_of_this_machine_is_made_without_connecting() {
    let _connector = ipc::system_connector();
}

#[test]
fn the_frame_thread_says_an_activity_once_and_a_setting_only_when_it_changes() {
    let mut presence = DiscordPresence::recording(true);
    presence.show(activity("Song"));
    presence.show(activity("Song"));
    presence.follow_setting(true);
    assert_eq!(presence.recorded(), [Command::Show(Some(activity("Song")))]);
    presence.follow_setting(false);
    presence.follow_setting(false);
    presence.follow_setting(true);
    assert_eq!(
        presence.recorded(),
        [Command::Show(Some(activity("Song"))), Command::Enable(false), Command::Enable(true), Command::Show(Some(activity("Song")))]
    );
}

/// A stand-in app on the loopback and the connector to it.
fn stand_in() -> (TcpListener, TcpConnector, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let attempts = Arc::new(AtomicUsize::new(0));
    let connector = TcpConnector { addr: listener.local_addr().expect("its address"), attempts: attempts.clone() };
    (listener, connector, attempts)
}

/// Run `script` on the next connection the stand-in takes.
fn serve<T: Send + 'static>(listener: TcpListener, script: impl FnOnce(&mut TcpStream, &TcpListener) -> T + Send + 'static) -> JoinHandle<T> {
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the link connects");
        stream.set_read_timeout(Some(SAFETY_LIMIT)).expect("a read limit");
        script(&mut stream, &listener)
    })
}

fn accept(listener: &TcpListener) -> TcpStream {
    let (stream, _) = listener.accept().expect("the link connects again");
    stream.set_read_timeout(Some(SAFETY_LIMIT)).expect("a read limit");
    stream
}

/// Take the handshake and answer it with READY.
fn greet(stream: &mut (impl Read + Write)) -> Value {
    let hello = ipc::read_message(stream).expect("a handshake");
    assert_eq!(hello.opcode, Opcode::Handshake);
    ipc::write_message(stream, Opcode::Frame, br#"{"cmd":"DISPATCH","evt":"READY","data":{"v":1}}"#).expect("READY goes");
    serde_json::from_slice(&hello.body).expect("JSON")
}

/// Take one command and answer it as the app does.
fn answer(stream: &mut (impl Read + Write)) -> Value {
    let command = ipc::read_message(stream).expect("a command");
    assert_eq!(command.opcode, Opcode::Frame);
    let body: Value = serde_json::from_slice(&command.body).expect("JSON");
    let reply = json!({ "cmd": body["cmd"], "data": {}, "evt": null, "nonce": body["nonce"] });
    ipc::write_message(stream, Opcode::Frame, reply.to_string().as_bytes()).expect("the answer goes");
    body
}

/// Whether the stream ends with nothing more sent down it.
fn ends(stream: &mut impl Read) -> bool {
    ipc::read_message(stream).is_err()
}

fn link(connector: impl Connect + 'static, enabled: bool, now: Instant) -> Link {
    Link::new(Box::new(connector), DISCORD_APPLICATION_ID, TEST_PID, enabled, now)
}

fn sent_activity(activity: &Activity) -> Value {
    serde_json::to_value(activity).expect("JSON")
}

#[test]
fn a_connection_shakes_hands_then_sends_the_activity_and_takes_its_answer() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| (greet(stream), answer(stream)));
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        let (hello, command) = app.join().expect("the stand-in ran");
        assert_eq!(hello, json!({ "v": 1, "client_id": DISCORD_APPLICATION_ID }));
        assert_eq!(command["cmd"], "SET_ACTIVITY");
        assert_eq!(command["args"]["pid"], TEST_PID);
        assert_eq!(command["nonce"], "1");
        assert_eq!(command["args"]["activity"], sent_activity(&activity("Song")));
        assert!(link.is_connected());
        assert_eq!(link.shown(), Some(&activity("Song")));
    }
}

#[test]
fn a_ping_before_the_answer_is_given_its_pong() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            greet(stream);
            let command = ipc::read_message(stream).expect("a command");
            ipc::write_message(stream, Opcode::Ping, b"{\"beat\":1}").expect("a ping goes");
            let pong = ipc::read_message(stream).expect("a pong");
            let body: Value = serde_json::from_slice(&command.body).expect("JSON");
            ipc::write_message(stream, Opcode::Frame, json!({ "cmd": "SET_ACTIVITY", "nonce": body["nonce"] }).to_string().as_bytes()).expect("the answer");
            pong
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        assert_eq!(app.join().expect("the stand-in ran"), Message { opcode: Opcode::Pong, body: b"{\"beat\":1}".to_vec() });
        assert_eq!(link.shown(), Some(&activity("Song")));
    }
}

#[test]
fn an_unknown_application_closes_the_connection_and_the_next_attempt_waits_a_minute() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            ipc::read_message(stream).expect("a handshake");
            let close = json!({ "code": CLOSE_INVALID_CLIENT_ID, "message": "Invalid Client ID" });
            ipc::write_message(stream, Opcode::Close, close.to_string().as_bytes()).expect("the close goes");
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        app.join().expect("the stand-in ran");
        assert!(!link.is_connected());
        assert_eq!(link.retry_at(), Some(t0 + RECONNECT_MAX));
    }
}

#[test]
fn a_connection_the_app_dropped_is_made_again_after_the_wait_with_the_newest_activity() {
    for _ in 0..REPEATS {
        let (listener, connector, attempts) = stand_in();
        let app = serve(listener, |stream, listener| {
            greet(stream);
            answer(stream);
            let _ = stream.shutdown(std::net::Shutdown::Both);
            let mut again = accept(listener);
            greet(&mut again);
            answer(&mut again)
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("First"))), t0);
        link.work(t0);
        let t1 = t0 + Duration::from_secs(1);
        link.handle(Command::Show(Some(activity("Second"))), t1);
        link.work(t1);
        assert!(!link.is_connected(), "the dropped connection was not noticed");
        assert_eq!(link.retry_at(), Some(t1 + RECONNECT_FIRST));
        link.handle(Command::Show(Some(activity("Newest"))), t1);
        link.work(t1 + RECONNECT_FIRST - Duration::from_millis(1));
        assert_eq!(attempts.load(Ordering::SeqCst), 1, "it connected again before the wait was over");
        link.work(t1 + RECONNECT_FIRST);
        let command = app.join().expect("the stand-in ran");
        assert_eq!(command["args"]["activity"], sent_activity(&activity("Newest")));
        assert_eq!(command["nonce"], "1", "a new connection counts its nonces from one");
        assert_eq!(link.shown(), Some(&activity("Newest")));
    }
}

#[test]
fn with_no_app_to_connect_to_the_waits_grow_and_switched_off_nothing_is_tried() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let t0 = Instant::now();
    let mut link = link(RefusingConnector { attempts: attempts.clone() }, true, t0);
    link.work(t0);
    assert_eq!(attempts.load(Ordering::SeqCst), 0, "nothing to show, nothing is tried");
    link.handle(Command::Show(Some(activity("Song"))), t0);
    link.work(t0);
    assert_eq!(link.retry_at(), Some(t0 + RECONNECT_FIRST));
    link.work(t0 + RECONNECT_FIRST);
    assert_eq!(link.retry_at(), Some(t0 + RECONNECT_FIRST * 3), "the second wait is twice the first");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    link.handle(Command::Enable(false), t0 + RECONNECT_FIRST);
    link.work(t0 + RECONNECT_MAX * 2);
    assert_eq!(attempts.load(Ordering::SeqCst), 2, "a link switched off tried to connect");
    assert_eq!(link.next_due(t0 + RECONNECT_MAX * 2), None);

    let off = Arc::new(AtomicUsize::new(0));
    let mut never = super::link::Link::new(Box::new(RefusingConnector { attempts: off.clone() }), DISCORD_APPLICATION_ID, TEST_PID, false, t0);
    never.handle(Command::Show(Some(activity("Song"))), t0);
    never.work(t0 + RECONNECT_MAX);
    assert_eq!(off.load(Ordering::SeqCst), 0, "a link that starts switched off connects");
}

#[test]
fn switching_off_clears_the_profile_and_closes_and_nothing_is_tried_after() {
    for _ in 0..REPEATS {
        let (listener, connector, attempts) = stand_in();
        let app = serve(listener, |stream, _| {
            greet(stream);
            answer(stream);
            let clear = answer(stream);
            (clear, ends(stream))
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        link.handle(Command::Enable(false), t0);
        let (clear, ended) = app.join().expect("the stand-in ran");
        assert_eq!(clear["args"], json!({ "pid": TEST_PID }), "the clearing names no activity");
        assert!(ended, "the connection stayed open");
        assert!(!link.is_connected());
        link.work(t0 + RECONNECT_MAX);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn a_refused_activity_keeps_the_connection_and_is_not_sent_again() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            greet(stream);
            let command = ipc::read_message(stream).expect("a command");
            let body: Value = serde_json::from_slice(&command.body).expect("JSON");
            let refusal = json!({ "cmd": "SET_ACTIVITY", "evt": EVT_ERROR, "data": { "code": 4000, "message": "bad" }, "nonce": body["nonce"] });
            ipc::write_message(stream, Opcode::Frame, refusal.to_string().as_bytes()).expect("the refusal goes");
            ends(stream)
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        assert!(link.is_connected());
        link.work(t0 + Duration::from_secs(1));
        assert_eq!(link.next_due(t0 + Duration::from_secs(1)), None, "a refused activity is due again");
        link.work(t0 + REFRESH_INTERVAL + Duration::from_secs(1));
        drop(link);
        assert!(app.join().expect("the stand-in ran"), "the refused activity was sent again");
    }
}

#[test]
fn a_frame_in_place_of_ready_closes_the_connection_and_waits() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            ipc::read_message(stream).expect("a handshake");
            ipc::write_message(stream, Opcode::Frame, br#"{"cmd":"DISPATCH","evt":"SOMETHING_ELSE"}"#).expect("the frame goes");
            ends(stream)
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        assert!(!link.is_connected());
        assert_eq!(link.retry_at(), Some(t0 + RECONNECT_FIRST));
        drop(link);
        assert!(app.join().expect("the stand-in ran"));
    }
}

#[test]
fn updates_past_the_rate_wait_and_only_the_newest_of_them_goes() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            greet(stream);
            let sent: Vec<Value> = (0..=RATE_LIMIT).map(|_| answer(stream)).collect();
            (sent, ends(stream))
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        for second in 0..RATE_LIMIT as u64 {
            let at = t0 + Duration::from_secs(second);
            link.handle(Command::Show(Some(activity(&format!("Song {second}")))), at);
            link.work(at);
        }
        let held = t0 + Duration::from_secs(RATE_LIMIT as u64);
        link.handle(Command::Show(Some(activity("Held"))), held);
        link.work(held);
        assert_eq!(link.next_due(held), Some(t0 + RATE_WINDOW));
        link.handle(Command::Show(Some(activity("Newest"))), held);
        link.work(t0 + RATE_WINDOW - Duration::from_millis(1));
        link.work(t0 + RATE_WINDOW);
        link.work(t0 + RATE_WINDOW);
        drop(link);
        let (sent, ended) = app.join().expect("the stand-in ran");
        assert_eq!(sent.last().map(|command| command["args"]["activity"].clone()), Some(sent_activity(&activity("Newest"))));
        assert!(sent.iter().all(|command| command["args"]["activity"]["details"] != "Held"), "a replaced update was sent");
        assert!(ended, "the same activity was sent twice");
    }
}

#[test]
fn the_activity_shown_is_sent_again_after_a_while_so_a_dropped_app_is_noticed() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            greet(stream);
            answer(stream);
            answer(stream)
        });
        let t0 = Instant::now();
        let mut link = link(connector, true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        assert_eq!(link.next_due(t0), Some(t0 + REFRESH_INTERVAL));
        link.work(t0 + REFRESH_INTERVAL);
        let again = app.join().expect("the stand-in ran");
        assert_eq!(again["nonce"], "2");
        assert_eq!(again["args"]["activity"], sent_activity(&activity("Song")));
    }
}

#[test]
fn stopping_the_thread_clears_the_profile_and_the_thread_says_it_ended() {
    for _ in 0..REPEATS {
        let (listener, connector, _) = stand_in();
        let app = serve(listener, |stream, _| {
            greet(stream);
            let shown = answer(stream);
            let clear = answer(stream);
            (shown, clear, ends(stream))
        });
        let mut presence = DiscordPresence::spawn_with(Box::new(connector), Instant::now, true);
        assert!(presence.has_thread());
        presence.show(activity("Song"));
        assert!(presence.stop_within(SAFETY_LIMIT), "the thread did not say it stopped");
        let (shown, clear, ended) = app.join().expect("the stand-in ran");
        assert_eq!(shown["args"]["activity"], sent_activity(&activity("Song")));
        assert_eq!(clear["args"], json!({ "pid": std::process::id() }));
        assert!(ended);
        assert!(!presence.stop_within(SAFETY_LIMIT), "a stopped presence has no thread to wait for");
    }
}

#[cfg(unix)]
mod unix_socket {
    use std::os::unix::net::UnixListener;

    use super::*;

    /// A folder of its own under the temporary folder, short enough for a socket path.
    fn socket_folder(tag: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("rbms-dp-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a socket folder");
        folder
    }

    fn connect_through_tmpdir(tag: &str, slot: usize) {
        let folder = socket_folder(tag);
        let listener = UnixListener::bind(folder.join(format!("discord-ipc-{slot}"))).expect("a socket");
        let app = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the link connects");
            stream.set_read_timeout(Some(SAFETY_LIMIT)).expect("a read limit");
            greet(&mut stream);
            answer(&mut stream)
        });
        let env_folder = folder.clone();
        let candidates = ipc::unix_candidates(move |name| (name == "TMPDIR").then(|| env_folder.clone().into_os_string()));
        let t0 = Instant::now();
        let mut link = link(ipc::UnixConnector::new(candidates), true, t0);
        link.handle(Command::Show(Some(activity("Song"))), t0);
        link.work(t0);
        let command = app.join().expect("the stand-in ran");
        assert_eq!(command["args"]["activity"], sent_activity(&activity("Song")));
        assert_eq!(link.shown(), Some(&activity("Song")));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_unix_connector_finds_the_socket_under_the_folder_tmpdir_names() {
        for repeat in 0..REPEATS {
            connect_through_tmpdir(&format!("u1-{repeat}"), 0);
        }
    }

    #[test]
    fn the_unix_connector_passes_over_a_missing_socket_to_the_next_number() {
        for repeat in 0..REPEATS {
            connect_through_tmpdir(&format!("u2-{repeat}"), 1);
        }
    }
}

/// The chart the application tests play: seven keys, ANOTHER, level 12.
const CHART: &[u8] = b"#PLAYER 1\n#TITLE Presence\n#SUBTITLE [SPA]\n#PLAYLEVEL 12\n#DIFFICULTY 4\n#BPM 120\n#WAV01 a.wav\n#00111:0101\n#00118:01\n";
const CHART_NAME: &str = "presence.bms";

/// An application of its own folder, nothing played by itself.
fn test_app(tag: &str) -> App {
    rbms_render::font::use_embedded_fonts_only();
    let folder = std::env::temp_dir().join(format!("rbms-discord-presence-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let mut config = rbms_config::Config::default();
    config.play.autoplay = false;
    App::new(String::new(), config, LaunchOptions::default(), folder.join("settings.ron"))
}

fn shown_lines(command: &Command) -> (Option<&str>, Option<&str>) {
    match command {
        Command::Show(Some(activity)) => (activity.details.as_deref(), activity.state.as_deref()),
        other => panic!("{other:?} is not an activity"),
    }
}

#[test]
fn the_screens_of_a_chart_picked_and_played_are_said_in_turn_and_a_run_says_nothing_more() {
    let mut app = test_app("a1");
    assert!(!app.shared.discord.has_thread(), "a unit test reached for a real Discord");
    app.switch(Transition::To(Stage::Select(Box::new(SelectState::new()))));
    let decide = DecideState::loaded(&app.shared, crate::app_play::loaded_chart_for_tests(CHART, CHART_NAME));
    app.switch(Transition::To(Stage::Decide(Box::new(decide))));
    app.switch(Transition::To(Stage::Loading(LoadingState::assets(crate::app_play::loaded_chart_for_tests(CHART, CHART_NAME)))));
    let play = app.shared.enter_loaded_chart(crate::app_play::loaded_chart_for_tests(CHART, CHART_NAME));
    assert!(matches!(play, Stage::Play(_)));
    app.switch(Transition::To(play));
    let said: Vec<_> = app.shared.discord.recorded().iter().map(shown_lines).collect();
    assert_eq!(said.last().copied(), Some((Some("Presence [SPA]"), Some("7K ANOTHER 12"))), "the run is not on the profile: {said:?}");
    assert!(said.contains(&(Some("Choosing a song"), None)), "{said:?}");
    assert!(said.contains(&(Some("Presence [SPA]"), Some("Loading · 7K ANOTHER 12"))), "{said:?}");
    let before = app.shared.discord.recorded().len();
    for _ in 0..REPEATS {
        app.shared.discord.follow_setting(app.shared.config.network.discord_status);
    }
    assert_eq!(app.shared.discord.recorded().len(), before, "a frame of the run said something");
}

#[test]
fn switching_the_row_off_and_on_says_so_once_each_and_shows_the_screen_again() {
    let mut app = test_app("a2");
    app.switch(Transition::To(Stage::Settings(SettingsState::new())));
    let shown = app.shared.discord.recorded().last().cloned().expect("the settings screen was said");
    let before = app.shared.discord.recorded().len();
    app.shared.config.network.discord_status = false;
    app.shared.discord.follow_setting(app.shared.config.network.discord_status);
    app.shared.discord.follow_setting(app.shared.config.network.discord_status);
    assert_eq!(app.shared.discord.recorded()[before..], [Command::Enable(false)]);
    app.shared.config.network.discord_status = true;
    app.shared.discord.follow_setting(app.shared.config.network.discord_status);
    assert_eq!(app.shared.discord.recorded()[before + 1..], [Command::Enable(true), shown]);
}

#[test]
fn the_way_out_tells_the_thread_to_stop() {
    let mut app = test_app("a3");
    app.wind_down();
    assert_eq!(app.shared.discord.recorded().last(), Some(&Command::Stop));
}

#[test]
fn the_row_is_listed_under_its_own_heading_at_the_end_of_the_network_group_and_described() {
    use crate::stage::settings::catalog;
    use rbms_config::{SettingId, SettingTab, descriptor};
    let last = catalog::sections(SettingTab::Network).last().copied().expect("the group has sections");
    assert_eq!(last, catalog::Section { heading: "DISCORD", rows: &[SettingId::DiscordStatus] });
    assert!(catalog::group_summary(SettingTab::Network).contains("DISCORD"));
    let said = catalog::description(SettingId::DiscordStatus);
    assert_ne!(said, descriptor(SettingId::DiscordStatus).help, "the row has sentences of its own");
    assert!(said.contains("Discord desktop app"), "{said}");
}
