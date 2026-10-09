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

/// What an off timer reads as: the reference's `Long.MIN_VALUE`, kept as the same sentinel so a
/// value crosses into a script and back without a second representation.
pub const TIMER_OFF: i64 = i64::MIN;

/// Microseconds in the millisecond a keyframe time is written in (`TimerProperty.get`, which divides
/// the stored value by this).
pub const MICROS_PER_MILLI: i64 = 1_000;

/// Which timers are on, and since when.
///
/// Every value is the microsecond the timer switched on, on the same clock the caller passes to
/// [`crate::dst::resolve`], and an off timer reads [`TIMER_OFF`]. The reference keeps the built-in
/// ids in a flat array filled with that sentinel; an absent entry here is the same thing, and it
/// covers the band a skin declares for itself as well.
#[derive(Debug, Default, Clone)]
pub struct TimerState {
    on: HashMap<TimerId, i64>,
}

impl TimerState {
    /// A state with every timer off.
    pub fn new() -> Self {
        Self::default()
    }

    /// Switches a timer on as of `now_us`, restarting it if it was already on
    /// (`TimerManager.setMicroTimer`). Storing [`TIMER_OFF`] switches it off, as it does there.
    pub fn set_on(&mut self, id: TimerId, now_us: i64) {
        if now_us == TIMER_OFF {
            self.on.remove(&id);
        } else {
            self.on.insert(id, now_us);
        }
    }

    /// Switches a timer off (`TimerManager.setTimerOff`).
    pub fn off(&mut self, id: TimerId) {
        self.on.remove(&id);
    }

    /// Switches a timer on or off, leaving an already-on timer at the moment it started rather than
    /// restarting it (`TimerManager.switchTimer`).
    pub fn switch(&mut self, id: TimerId, on: bool, now_us: i64) {
        if !on {
            self.off(id);
        } else if !self.is_on(id) {
            self.set_on(id, now_us);
        }
    }

    /// Whether the timer is on (`TimerManager.isTimerOn`).
    pub fn is_on(&self, id: TimerId) -> bool {
        self.value_us(id) != TIMER_OFF
    }

    /// The microsecond the timer switched on, or [`TIMER_OFF`] (`TimerManager.getMicroTimer`).
    pub fn value_us(&self, id: TimerId) -> i64 {
        self.on.get(&id).copied().unwrap_or(TIMER_OFF)
    }

    /// Switches every timer off, as a screen change does (`TimerManager.setMainState`).
    pub fn clear(&mut self) {
        self.on.clear();
    }
}
