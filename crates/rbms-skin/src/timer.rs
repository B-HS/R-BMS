//! The skin timer registry and the timer state a skin animates against.
//!
//! A skin names timers by integer id, and every destination animation measures its keyframe times
//! from the moment its timer switched on. The id table is machine-extracted from the reference
//! implementation's `SkinProperty.java` into [`generated`]; nothing here is translated by hand.

mod generated;

#[cfg(test)]
mod tests;

pub use generated::{ALL_TIMER, TIMER_CONSTANT_COUNT, TIMER_TABLE_CHECKSUM, timer_id};

use std::collections::HashMap;

/// A skin timer id.
///
/// Ids `0..=`[`timer_id::MAX`] are the built-in timers the player drives; the band from
/// [`timer_id::CUSTOM_BEGIN`] to [`timer_id::CUSTOM_END`] is reserved for timers a skin declares
/// itself (`Skin.getMicroCustomTimer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerId(pub i32);

impl TimerId {
    /// The raw id, for the integer-keyed property lookups a skin document carries.
    pub const fn get(self) -> i32 {
        self.0
    }

    /// Whether the id falls in the built-in range the reference keeps in a flat array
    /// (`TimerManager.getMicroTimer`, `id >= 0 && id < TIMER_MAX + 1`).
    pub const fn is_builtin(self) -> bool {
        self.0 >= 0 && self.0 <= timer_id::MAX.0
    }

    /// Whether the id falls in the band reserved for skin-declared timers.
    pub const fn is_custom(self) -> bool {
        self.0 >= timer_id::CUSTOM_BEGIN.0 && self.0 <= timer_id::CUSTOM_END.0
    }
}

/// The extracted name of a timer id, without its `TIMER_` prefix, for logs and the skin debug view.
///
/// [`ALL_TIMER`] is in source declaration order rather than sorted, so this scans it. Three of its
/// entries name band edges rather than timers: `MAX`, `CUSTOM_BEGIN` and `CUSTOM_END`.
pub fn timer_name(id: TimerId) -> Option<&'static str> {
    ALL_TIMER.iter().find(|(value, _)| *value == id.0).map(|(_, name)| *name)
}

/// Which timers are on, and since when.
///
/// The reference keeps an "off" timer as `Long.MIN_VALUE` in a flat array; an absent entry here is
/// that sentinel. Times are milliseconds on the same clock the caller passes to
/// [`crate::dst::resolve`], so a timer switched on at `now_ms` reads as zero elapsed on that frame.
#[derive(Debug, Default, Clone)]
pub struct TimerState {
    on: HashMap<TimerId, i64>,
}

impl TimerState {
    /// A state with every timer off.
    pub fn new() -> Self {
        Self::default()
    }

    /// Switches a timer on as of `now_ms`, restarting it if it was already on
    /// (`TimerManager.setTimerOn`).
    pub fn set_on(&mut self, id: TimerId, now_ms: i64) {
        self.on.insert(id, now_ms);
    }

    /// Switches a timer off (`TimerManager.setTimerOff`).
    pub fn set_off(&mut self, id: TimerId) {
        self.on.remove(&id);
    }

    /// Switches a timer on or off, leaving an already-on timer at the moment it started rather than
    /// restarting it (`TimerManager.switchTimer`).
    pub fn switch(&mut self, id: TimerId, on: bool, now_ms: i64) {
        if on {
            self.on.entry(id).or_insert(now_ms);
        } else {
            self.on.remove(&id);
        }
    }

    /// Whether the timer is on.
    pub fn is_on(&self, id: TimerId) -> bool {
        self.on.contains_key(&id)
    }

    /// Whether the timer is off (`TimerProperty.isOff`).
    pub fn is_off(&self, id: TimerId) -> bool {
        !self.is_on(id)
    }

    /// The millisecond the timer switched on, or `None` while it is off (`TimerProperty.get`).
    pub fn get(&self, id: TimerId) -> Option<i64> {
        self.on.get(&id).copied()
    }

    /// Milliseconds since the timer switched on, or `None` while it is off
    /// (`TimerManager.getNowTime(id)`, which reports zero for an off timer; the caller decides what
    /// an absent value means).
    pub fn elapsed(&self, id: TimerId, now_ms: i64) -> Option<i64> {
        self.get(id).map(|started| now_ms - started)
    }

    /// Switches every timer off, as a screen change does (`TimerManager.setMainState`).
    pub fn clear(&mut self) {
        self.on.clear();
    }
}
