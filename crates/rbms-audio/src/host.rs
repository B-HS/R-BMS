//! The one thread every question about the audio host is asked from.
//!
//! cpal builds the object it enumerates devices through once and keeps it in a process-wide slot
//! for the rest of the run. On Windows that object belongs to the COM apartment of whichever thread
//! built it, and a thread leaving closes its apartment and unloads the libraries behind it, which
//! leaves the process-wide slot pointing at freed memory: the next question, asked from any thread
//! at all, reads a dangling vtable and the process dies with an access violation rather than a
//! panic. Nothing in cpal's API can be asked to rebuild it.
//!
//! So the host is questioned from one thread, started the first time it is needed and never left,
//! which keeps the apartment that owns the enumerator alive for as long as the process is.

use std::sync::OnceLock;
use std::sync::mpsc::{Sender, channel};

use cpal::traits::{DeviceTrait, HostTrait};

/// The name the host thread carries, so a stack trace and a panic message both name it.
const HOST_THREAD_NAME: &str = "rbms-audio-host";

/// One question, boxed so questions of different answer types share a queue.
type Question = Box<dyn FnOnce() + Send>;

/// The queue the host thread reads, starting the thread on the first question.
fn questions() -> &'static Sender<Question> {
    static QUESTIONS: OnceLock<Sender<Question>> = OnceLock::new();
    QUESTIONS.get_or_init(|| {
        let (send, receive) = channel::<Question>();
        std::thread::Builder::new()
            .name(HOST_THREAD_NAME.to_owned())
            .spawn(move || {
                while let Ok(question) = receive.recv() {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(question));
                }
            })
            .expect("the audio host thread starts");
        send
    })
}

/// Asks the host thread one question and waits for its answer.
///
/// A question that panics, or a thread that could not be started, answers with the default rather
/// than taking the caller down: the host thread itself survives either way, because it is the
/// apartment behind it that everything else depends on.
fn ask<T: Default + Send + 'static>(question: impl FnOnce() -> T + Send + 'static) -> T {
    let (send, receive) = channel::<T>();
    if questions().send(Box::new(move || drop(send.send(question())))).is_err() {
        return T::default();
    }
    receive.recv().unwrap_or_default()
}

/// Every output device the host offers, named as [`AudioOptions::device_name`] is matched against.
///
/// A device whose description cannot be read is left out rather than offered under a name the
/// engine would then fail to find, and a host that cannot be enumerated answers with nothing, which
/// leaves a caller on the system default.
///
/// Walking a host costs tens of milliseconds and the asking thread waits for the answer, so a
/// screen asks on the way in rather than on every keystroke.
///
/// [`AudioOptions::device_name`]: crate::AudioOptions::device_name
pub fn output_device_names() -> Vec<String> {
    ask(|| {
        cpal::default_host()
            .output_devices()
            .map(|devices| devices.filter_map(|device| device.description().ok().map(|description| description.name().to_string())).collect())
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Threads that ask, so the question comes from more than one and from none that stays.
    const ASKERS: usize = 4;

    /// The shape the Windows crash had: a thread asks, is answered, and leaves. Before the host
    /// thread existed the first of these built the enumerator in its own apartment and took it down
    /// again on the way out, and the second read freed memory.
    #[test]
    fn asking_from_threads_that_come_and_go_answers_the_same_list_every_time() {
        let first = output_device_names();
        for _ in 0..ASKERS {
            let answer = std::thread::spawn(output_device_names).join().expect("the asking thread finished");
            assert_eq!(answer, first, "two questions about the same host answered differently");
        }
    }

    /// The invariant the fix rests on, which holds on every platform even though only Windows
    /// punishes breaking it: whoever asks, the answer is worked out on the one thread that stays.
    #[test]
    fn every_question_is_answered_on_the_host_thread_whichever_thread_asks() {
        let answered_on: Vec<Option<String>> = (0..ASKERS)
            .map(|_| std::thread::spawn(|| ask(|| std::thread::current().name().map(str::to_owned))).join().expect("the asking thread finished"))
            .collect();
        assert!(answered_on.iter().all(|name| name.as_deref() == Some(HOST_THREAD_NAME)), "a question was answered off the host thread: {answered_on:?}");
    }
}
