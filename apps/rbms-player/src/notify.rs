//! The one place the program says something went wrong.
//!
//! Every failure the app can survive — a chart that would not open, a table that would not load, a
//! settings file that would not save — is reported here instead of straight to the terminal. The
//! message still reaches standard error exactly as it did, so a run from a shell reads the same;
//! what is new is that the app keeps the recent ones, so the screen can show them too.
//!
//! The sink is process-wide and locked, so a worker thread may report from wherever it is without
//! having to reach the frame loop.
//!
//! A message a player acts on is worded by a catalog entry ([`notify_text`]) or by a function of the
//! language ([`notify_with`]) rather than reported as a finished string. Standard error gets it in
//! English at once, as a run from a shell has always read it. The queue keeps the entry and its
//! values, and the frame thread words it in the language it has installed when it takes the message
//! ([`drain`]), so a message reported from a worker thread, or before the language was settled,
//! still reaches the screen in the language the screen is drawn in.

use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::{Mutex, OnceLock};

use rbms_model::Language;
use rbms_model::translations::{Catalog, fill};

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

/// What a queued message is made of until it is taken.
enum Said {
    /// A finished string, as it was reported.
    Plain(String),
    /// A message worded in whichever language it is taken in.
    Worded(Box<dyn Fn(Language) -> String + Send>),
}

impl Said {
    /// The message in `language`.
    fn in_language(&self, language: Language) -> String {
        match self {
            Said::Plain(text) => text.clone(),
            Said::Worded(word) => word(language),
        }
    }
}

/// The process-wide queue of messages not yet taken by the frame loop.
fn queue() -> &'static Mutex<VecDeque<(Level, Said)>> {
    static QUEUE: OnceLock<Mutex<VecDeque<(Level, Said)>>> = OnceLock::new();
    QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Report something. The message goes to standard error as it always has, and is queued for
/// whatever is showing the screen to pick up.
///
/// A poisoned lock is not worth failing the app over — a message is dropped rather than a run.
pub(crate) fn notify(level: Level, message: impl Into<String>) {
    let message = message.into();
    eprintln!("{message}");
    enqueue(level, Said::Plain(message));
}

/// Report a catalog entry with the values its placeholders take.
///
/// Standard error gets the entry in English at once. The queue keeps the entry and the values, and
/// the message is worded in the language the frame thread has installed when it takes it
/// ([`drain`]).
pub(crate) fn notify_text(level: Level, text: impl Catalog, args: &[&dyn Display]) {
    let strings = text.strings();
    let args: Vec<String> = args.iter().map(ToString::to_string).collect();
    notify_with(level, move |language| {
        let values: Vec<&dyn Display> = args.iter().map(|value| value as &dyn Display).collect();
        fill(strings[language.id()], &values)
    });
}

/// Report a message that `word` words in a language, for one that more than one catalog entry goes
/// into. Standard error gets it in English at once, and the queue keeps `word` to call again with
/// the language the frame thread has installed when it takes the message ([`drain`]).
pub(crate) fn notify_with(level: Level, word: impl Fn(Language) -> String + Send + 'static) {
    eprintln!("{}", word(Language::English));
    enqueue(level, Said::Worded(Box::new(word)));
}

/// Put a message on the queue, dropping the oldest when it is full.
fn enqueue(level: Level, said: Said) {
    if is_kept_out_of_the_queue() {
        return;
    }
    let Ok(mut queue) = queue().lock() else {
        return;
    };
    if queue.len() == NOTIFY_CAPACITY {
        queue.pop_front();
    }
    queue.push_back((level, said));
}

/// Take everything reported since the last call, oldest first, appending onto `out`, each message
/// worded in the language this thread has installed ([`rbms_render::text::language`]).
pub(crate) fn drain(out: &mut Vec<(Level, String)>) {
    let Ok(mut queue) = queue().lock() else {
        return;
    };
    let taken: Vec<(Level, Said)> = queue.drain(..).collect();
    drop(queue);
    let language = rbms_render::text::language();
    out.extend(taken.into_iter().map(|(level, said)| (level, said.in_language(language))));
}

/// Whether the queue is held by a test on another thread, in which case what this thread reports is
/// printed and not queued.
#[cfg(test)]
fn is_kept_out_of_the_queue() -> bool {
    HOLDER.lock().is_ok_and(|holder| holder.is_some_and(|thread| thread != std::thread::current().id()))
}

/// Outside a test the queue is never held.
#[cfg(not(test))]
const fn is_kept_out_of_the_queue() -> bool {
    false
}

/// The thread of the test that has the queue to itself, if one has.
#[cfg(test)]
static HOLDER: Mutex<Option<std::thread::ThreadId>> = Mutex::new(None);

/// A test's hold on the queue, let go of when it is dropped, whether the test ended or panicked.
#[cfg(test)]
struct Hold;

#[cfg(test)]
impl Hold {
    fn take() -> Hold {
        *HOLDER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(std::thread::current().id());
        Hold
    }
}

#[cfg(test)]
impl Drop for Hold {
    fn drop(&mut self) {
        *HOLDER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
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
/// reads it has to take its turn or it will see another test's message. A test that does not take
/// its turn can still report -- what it says is printed -- but while another test holds the queue,
/// what it says is not queued, so it cannot put a message in front of the test that holds it.
#[cfg(test)]
pub(crate) fn exclusive<T>(body: impl FnOnce() -> T) -> T {
    static GATE: Mutex<()> = Mutex::new(());
    let guard = GATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let held = Hold::take();
    clear();
    let outcome = body();
    clear();
    drop(held);
    drop(guard);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    rbms_model::translations! {
        /// Words of a message a test reports.
        enum SampleText {
            ChartCount => ("{0} CHARTS", "차트 {0}개", "{0}譜面"),
            Swapped => ("{0} OF {1}", "{1} 중 {0}", "{1}の{0}"),
        }
    }

    fn drained() -> Vec<(Level, String)> {
        let mut out = Vec::new();
        drain(&mut out);
        out
    }

    #[test]
    fn a_catalog_message_is_worded_in_english_by_a_thread_that_installed_no_language() {
        exclusive(|| {
            notify_text(Level::Info, SampleText::ChartCount, &[&12]);
            notify_text(Level::Warn, SampleText::Swapped, &[&"A", &"B"]);
            assert_eq!(drained(), vec![(Level::Info, "12 CHARTS".to_string()), (Level::Warn, "A OF B".to_string())]);
        });
    }

    #[test]
    fn a_catalog_message_is_worded_when_it_is_taken_in_the_language_installed_then() {
        exclusive(|| {
            notify_text(Level::Info, SampleText::ChartCount, &[&12]);
            rbms_render::text::set_language(Language::Korean);
            let korean = drained();
            notify_text(Level::Info, SampleText::Swapped, &[&"A", &"B"]);
            rbms_render::text::set_language(Language::Japanese);
            let japanese = drained();
            rbms_render::text::set_language(Language::English);
            assert_eq!(korean, vec![(Level::Info, "차트 12개".to_string())]);
            assert_eq!(japanese, vec![(Level::Info, "BのA".to_string())]);
        });
    }

    #[test]
    fn a_catalog_message_is_not_worded_in_the_language_of_the_thread_that_reports_it() {
        exclusive(|| {
            rbms_render::text::set_language(Language::Korean);
            notify_text(Level::Info, SampleText::ChartCount, &[&12]);
            rbms_render::text::set_language(Language::English);
            assert_eq!(drained(), vec![(Level::Info, "12 CHARTS".to_string())], "the message waits for the thread that shows it");
        });
    }

    #[test]
    fn a_message_worded_by_a_function_is_asked_again_in_the_language_it_is_taken_in() {
        exclusive(|| {
            notify_with(Level::Warn, |language| format!("said in {}", language.code()));
            rbms_render::text::set_language(Language::Korean);
            let taken = drained();
            rbms_render::text::set_language(Language::English);
            assert_eq!(taken, vec![(Level::Warn, "said in ko".to_string())]);
        });
    }

    #[test]
    fn a_plain_message_is_not_worded_again_in_another_language() {
        exclusive(|| {
            notify(Level::Error, "chart not found");
            rbms_render::text::set_language(Language::Japanese);
            let taken = drained();
            rbms_render::text::set_language(Language::English);
            assert_eq!(taken, vec![(Level::Error, "chart not found".to_string())]);
        });
    }

    #[test]
    fn a_catalog_message_keeps_the_order_it_was_reported_in_among_plain_ones() {
        exclusive(|| {
            notify(Level::Info, "first");
            notify_text(Level::Info, SampleText::ChartCount, &[&1]);
            notify_with(Level::Info, |_| "third".to_string());
            let texts: Vec<String> = drained().into_iter().map(|(_, text)| text).collect();
            assert_eq!(texts, vec!["first", "1 CHARTS", "third"]);
        });
    }

    #[test]
    fn a_placeholder_with_no_value_is_left_as_written() {
        exclusive(|| {
            notify_text(Level::Info, SampleText::Swapped, &[&"only"]);
            assert_eq!(drained(), vec![(Level::Info, "only OF {1}".to_string())]);
        });
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
