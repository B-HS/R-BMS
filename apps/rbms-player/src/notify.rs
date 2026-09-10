//! The one place the program says something went wrong.
//!
//! Every failure the app can survive — a chart that would not open, a table that would not load, a
//! settings file that would not save — is reported here instead of straight to the terminal. The
//! message still reaches standard error exactly as it did, so a run from a shell reads the same;
//! what is new is that the app keeps the recent ones, so the screen can show them too.
//!
//! The sink is process-wide and locked, so a worker thread may report from wherever it is without
//! having to reach the frame loop.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

/// How loud a message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    /// Something worth knowing that changed nothing.
    Info,
    /// Something did not go to plan but the app carried on with a fallback.
    Warn,
    /// Something the user asked for did not happen.
    Error,
}

/// How many messages are held before the oldest is dropped. A run that fails once a frame must not
/// grow the queue without end, and nothing older than the last few is worth showing.
const NOTIFY_CAPACITY: usize = 64;

/// The process-wide queue of messages not yet taken by the frame loop.
fn queue() -> &'static Mutex<VecDeque<(Level, String)>> {
    static QUEUE: OnceLock<Mutex<VecDeque<(Level, String)>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Report something. The message goes to standard error as it always has, and is queued for
/// whatever is showing the screen to pick up.
///
/// A poisoned lock is not worth failing the app over — a message is dropped rather than a run.
pub(crate) fn notify(level: Level, message: impl Into<String>) {
    let message = message.into();
    eprintln!("{message}");
    let Ok(mut queue) = queue().lock() else {
        return;
    };
    if queue.len() == NOTIFY_CAPACITY {
        queue.pop_front();
    }
    queue.push_back((level, message));
}

/// Take everything reported since the last call, oldest first, appending onto `out`.
pub(crate) fn drain(out: &mut Vec<(Level, String)>) {
    let Ok(mut queue) = queue().lock() else {
        return;
    };
    out.extend(queue.drain(..));
}

/// Throw away everything queued, so one test cannot see another's messages.
#[cfg(test)]
pub(crate) fn clear() {
    if let Ok(mut queue) = queue().lock() {
        queue.clear();
    }
}

/// Run `body` with the process-wide queue to itself, empty before and after.
///
/// The bus is one queue for the whole process and the tests run side by side, so anything that
/// reads it has to take its turn or it will see another test's message.
#[cfg(test)]
pub(crate) fn exclusive<T>(body: impl FnOnce() -> T) -> T {
    static GATE: Mutex<()> = Mutex::new(());
    let guard = GATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    clear();
    let outcome = body();
    clear();
    drop(guard);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drained() -> Vec<(Level, String)> {
        let mut out = Vec::new();
        drain(&mut out);
        out
    }

    #[test]
    fn a_reported_message_comes_back_out_with_its_level() {
        exclusive(|| {
            notify(Level::Error, "chart not found");
            assert_eq!(drained(), vec![(Level::Error, "chart not found".to_string())]);
        });
    }

    #[test]
    fn messages_come_back_in_the_order_they_were_reported() {
        exclusive(|| {
            notify(Level::Info, "one");
            notify(Level::Warn, "two");
            notify(Level::Error, "three");
            let texts: Vec<String> = drained().into_iter().map(|(_, text)| text).collect();
            assert_eq!(texts, vec!["one", "two", "three"]);
        });
    }

    #[test]
    fn draining_empties_the_queue() {
        exclusive(|| {
            notify(Level::Info, "once");
            assert_eq!(drained().len(), 1);
            assert!(drained().is_empty(), "a message is handed over once, not every frame");
        });
    }

    /// A failure that repeats every frame while nothing is draining must not grow the queue
    /// without end.
    #[test]
    fn the_queue_drops_the_oldest_rather_than_growing_without_end() {
        exclusive(|| {
            for i in 0..NOTIFY_CAPACITY * 2 {
                notify(Level::Warn, format!("message {i}"));
            }
            let taken = drained();
            assert_eq!(taken.len(), NOTIFY_CAPACITY);
            assert_eq!(taken.first().map(|(_, text)| text.as_str()), Some(format!("message {}", NOTIFY_CAPACITY).as_str()));
            assert_eq!(taken.last().map(|(_, text)| text.as_str()), Some(format!("message {}", NOTIFY_CAPACITY * 2 - 1).as_str()));
        });
    }

    /// The one file that still writes straight to standard error: this one, because it is the
    /// passthrough itself.
    const STDERR_ALLOWED: [&str; 1] = ["notify.rs"];

    fn source_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                source_files(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    /// Every failure the app can survive is reported through this bus, so it can reach the screen
    /// rather than only a terminal nobody is watching. A new one written straight to standard error
    /// would be invisible in a windowed run, which is what this catches.
    #[test]
    fn the_app_reports_through_the_bus_rather_than_straight_to_standard_error() {
        assert_eq!(STDERR_ALLOWED.len(), 1, "the passthrough is the only file that may write straight to standard error");
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        source_files(&src, &mut files);
        assert!(files.len() > 20, "the source tree was not found at {}", src.display());
        let mut offenders: Vec<String> = Vec::new();
        for file in files {
            let name = file.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            if STDERR_ALLOWED.contains(&name.as_str()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            for (at, line) in text.lines().enumerate() {
                if line.contains("eprintln!") {
                    offenders.push(format!("{}:{}", file.display(), at + 1));
                }
            }
        }
        assert!(offenders.is_empty(), "these write straight to standard error instead of reporting through the bus: {offenders:?}");
    }

    #[test]
    fn draining_an_empty_queue_appends_nothing() {
        exclusive(|| {
            let mut out = vec![(Level::Info, "kept".to_string())];
            drain(&mut out);
            assert_eq!(out.len(), 1, "an empty queue leaves what the caller already had");
        });
    }
}
