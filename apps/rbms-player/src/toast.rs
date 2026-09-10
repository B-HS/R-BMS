//! The live end of the notification bus: what [`crate::notify`] collected, aged out and ready to
//! be drawn over whichever screen is up.
//!
//! The queue owns the lifetime rules — how many are shown and for how long — because it is the part
//! that has the clock, and the cut that keeps a message on screen. Laying a strip out is
//! `rbms_render::toast`.

use std::time::{Duration, Instant};

use rbms_render::{ToastLevel, ToastView};

use crate::notify::{self, Level};
use crate::stage::Canvas;

/// How many messages are shown at once. Anything older is dropped rather than queued behind them:
/// a stack that scrolls is unreadable and the terminal has the full history anyway.
pub(crate) const TOAST_LIMIT: usize = 5;

/// How long an ordinary message stays up.
pub(crate) const TOAST_LIFETIME: Duration = Duration::from_secs(4);

/// How long a failure stays up. Twice as long, because it is the one the user has to act on.
pub(crate) const TOAST_ERROR_LIFETIME: Duration = Duration::from_secs(8);

/// How much of a message is shown. A strip is laid out from its own text, so an unbounded message —
/// a failure quoting a long path — would be laid out wider than the screen and start off the left
/// edge, taking the readable half of it with it.
pub(crate) const TOAST_TEXT_CHARS: usize = 88;

/// Marks a message that had its tail cut.
const TOAST_ELLIPSIS: char = '\u{2026}';

/// One message with the instant it appeared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Toast {
    pub(crate) level: ToastLevel,
    pub(crate) text: String,
    pub(crate) born: Instant,
}

impl Toast {
    /// How long this message stays up.
    fn lifetime(&self) -> Duration {
        match self.level {
            ToastLevel::Error => TOAST_ERROR_LIFETIME,
            _ => TOAST_LIFETIME,
        }
    }

    /// Whether the message is still worth showing at `now`.
    fn alive(&self, now: Instant) -> bool {
        now.duration_since(self.born) < self.lifetime()
    }
}

/// The messages currently on screen, oldest first.
#[derive(Debug, Default)]
pub(crate) struct ToastQueue {
    toasts: Vec<Toast>,
}

impl ToastQueue {
    /// Show a message. The oldest is dropped once the queue is full, so the newest is always
    /// visible.
    pub(crate) fn push(&mut self, level: ToastLevel, text: impl Into<String>, now: Instant) {
        self.toasts.push(Toast { level, text: shorten(&text.into()), born: now });
        while self.toasts.len() > TOAST_LIMIT {
            self.toasts.remove(0);
        }
    }

    /// Take everything reported since the last frame and age out what has had its time. Called once
    /// a frame, so a message reported from a worker thread reaches the screen on the next one.
    pub(crate) fn pump(&mut self, now: Instant) {
        let mut reported = Vec::new();
        notify::drain(&mut reported);
        for (level, text) in reported {
            self.push(toast_level(level), text, now);
        }
        self.expire(now);
    }

    /// Drop the messages that have had their time, without taking anything new off the bus.
    fn expire(&mut self, now: Instant) {
        self.toasts.retain(|toast| toast.alive(now));
    }

    /// The messages still worth showing, oldest first.
    pub(crate) fn active(&self) -> &[Toast] {
        &self.toasts
    }
}

/// A message cut to [`TOAST_TEXT_CHARS`], with the cut marked. Shorter messages are left alone, so
/// what the terminal printed and what the strip shows are the same string in the ordinary case.
fn shorten(text: &str) -> String {
    if text.chars().count() <= TOAST_TEXT_CHARS {
        return text.to_string();
    }
    let kept: String = text.chars().take(TOAST_TEXT_CHARS - 1).collect();
    format!("{kept}{TOAST_ELLIPSIS}")
}

/// How a reported level is shown.
pub(crate) fn toast_level(level: Level) -> ToastLevel {
    match level {
        Level::Info => ToastLevel::Info,
        Level::Warn => ToastLevel::Warn,
        Level::Error => ToastLevel::Error,
    }
}

/// Draw the live messages over the screen that is up.
///
/// Drawn last of everything, so a failure reported while a chart is on screen is readable over it.
/// An empty queue paints nothing at all, which is what lets every other screen's snapshot stay
/// still while the bus runs underneath.
pub(crate) fn draw(queue: &ToastQueue, canvas: &mut Canvas<'_>) {
    if queue.active().is_empty() {
        return;
    }
    let views: Vec<ToastView> = queue.active().iter().map(|toast| ToastView { level: toast.level, text: toast.text.clone() }).collect();
    rbms_render::render_toasts(canvas, &views);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_with(now: Instant, count: usize) -> ToastQueue {
        let mut queue = ToastQueue::default();
        for i in 0..count {
            queue.push(ToastLevel::Info, format!("message {i}"), now);
        }
        queue
    }

    #[test]
    fn a_fresh_queue_has_nothing_to_show() {
        assert!(ToastQueue::default().active().is_empty());
    }

    #[test]
    fn a_message_is_shown_with_the_level_it_was_reported_at() {
        let now = Instant::now();
        let mut queue = ToastQueue::default();
        queue.push(ToastLevel::Error, "chart not found", now);
        assert_eq!(queue.active().len(), 1);
        assert_eq!(queue.active()[0].level, ToastLevel::Error);
        assert_eq!(queue.active()[0].text, "chart not found");
    }

    #[test]
    fn the_oldest_message_is_pushed_out_once_the_queue_is_full() {
        let now = Instant::now();
        let queue = queue_with(now, TOAST_LIMIT + 3);
        assert_eq!(queue.active().len(), TOAST_LIMIT);
        assert_eq!(queue.active()[0].text, "message 3", "the three oldest were dropped");
        assert_eq!(queue.active()[TOAST_LIMIT - 1].text, format!("message {}", TOAST_LIMIT + 2), "the newest is always shown");
    }

    #[test]
    fn a_message_is_dropped_once_its_time_is_up() {
        let now = Instant::now();
        let mut queue = ToastQueue::default();
        queue.push(ToastLevel::Info, "shown", now);
        queue.expire(now + TOAST_LIFETIME - Duration::from_millis(1));
        assert_eq!(queue.active().len(), 1, "it is still inside its lifetime");
        queue.expire(now + TOAST_LIFETIME);
        assert!(queue.active().is_empty(), "and gone once past it");
    }

    /// A failure is the one message the user has to act on, so it outlives the rest.
    #[test]
    fn a_failure_stays_up_longer_than_anything_else() {
        let now = Instant::now();
        let mut queue = ToastQueue::default();
        queue.push(ToastLevel::Info, "info", now);
        queue.push(ToastLevel::Warn, "warn", now);
        queue.push(ToastLevel::Error, "error", now);
        queue.expire(now + TOAST_LIFETIME);
        let left: Vec<&str> = queue.active().iter().map(|toast| toast.text.as_str()).collect();
        assert_eq!(left, vec!["error"]);
        queue.expire(now + TOAST_ERROR_LIFETIME);
        assert!(queue.active().is_empty());
    }

    /// A failure quoting a long path is laid out from its own text, so an uncut message would be
    /// wider than the screen and start off the left edge.
    #[test]
    fn a_message_too_long_for_the_screen_keeps_its_head_and_says_it_was_cut() {
        let long: String = std::iter::repeat_n('x', TOAST_TEXT_CHARS + 40).collect();
        let now = Instant::now();
        let mut queue = ToastQueue::default();
        queue.push(ToastLevel::Error, long.clone(), now);
        let shown = &queue.active()[0].text;
        assert_eq!(shown.chars().count(), TOAST_TEXT_CHARS);
        assert_eq!(shown.chars().last(), Some(TOAST_ELLIPSIS));
        assert!(long.starts_with(&shown[..shown.len() - TOAST_ELLIPSIS.len_utf8()]), "the head of the message is what survives");
    }

    #[test]
    fn a_message_that_fits_is_shown_exactly_as_it_was_reported() {
        let exact: String = std::iter::repeat_n('y', TOAST_TEXT_CHARS).collect();
        assert_eq!(shorten(&exact), exact, "a message at the limit is not cut");
        assert_eq!(shorten("short"), "short");
    }

    #[test]
    fn every_reported_level_maps_to_one_of_its_own() {
        assert_eq!(toast_level(Level::Info), ToastLevel::Info);
        assert_eq!(toast_level(Level::Warn), ToastLevel::Warn);
        assert_eq!(toast_level(Level::Error), ToastLevel::Error);
    }

    #[test]
    fn pumping_takes_what_was_reported_and_leaves_the_bus_empty() {
        notify::exclusive(|| {
            let now = Instant::now();
            notify::notify(Level::Warn, "table load failed");
            let mut queue = ToastQueue::default();
            queue.pump(now);
            assert_eq!(queue.active().len(), 1);
            assert_eq!(queue.active()[0].level, ToastLevel::Warn);
            assert_eq!(queue.active()[0].text, "table load failed");
            queue.pump(now);
            assert_eq!(queue.active().len(), 1, "pumping again must not show the same message twice");
        });
    }
}
