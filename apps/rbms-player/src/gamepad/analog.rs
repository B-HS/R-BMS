//! The analog turntable: how a controller axis that spins without end becomes a held scratch.
//!
//! A turntable does not report "pressed". It reports an absolute position that wraps from `+1`
//! straight back to `-1`, so the only thing a player's spin looks like is a run of small position
//! changes. Turning that into a lane that is held down and then let go is the job of the two state
//! machines here, ported from the reference implementation's `BMControllerInputProcessor.java`
//! (version 1 at `:359-410`, version 2 at `:444-491`).
//!
//! Both machines share a shape. A movement marks the axis as spinning and remembers which way; a
//! run of calls with no movement eventually marks it stopped. What separates them is how eagerly
//! they believe a spin has started and how long they wait before calling it over: version 1 trusts
//! the very first movement and expires after `threshold` idle calls, version 2 wants two ticks of
//! travel before it believes anything and expires after twice as many.
//!
//! The counter is a count of calls, not of milliseconds. It advances once per query, and a
//! turntable bound to a scratch lane is queried twice per frame (once per direction), exactly as
//! the reference queries its `AXISn_PLUS` and `AXISn_MINUS` elements against one shared machine.

use serde::{Deserialize, Serialize};

/// The smallest movement a turntable reports. Reference implementation
/// `BMControllerInputProcessor.java:81`; real hardware ticks at 0.00784 to 0.00787, and the
/// constant is the ceiling of those.
pub const TICK_MAX_SIZE: f32 = 0.009;

/// Idle calls before a spin is called over. Reference implementation `PlayModeConfig.java:514`.
pub const DEFAULT_ANALOG_THRESHOLD: u32 = 100;

/// What the threshold is clamped to on load. Reference implementation `PlayModeConfig.java:150`.
pub const ANALOG_THRESHOLD_RANGE: std::ops::RangeInclusive<u32> = 1..=1000;

/// The position a machine holds before it has ever read the axis. Any value above `1` is outside
/// the axis range, which is how the reference marks "no reading yet" (`:344`, `:428`).
const UNREAD_POSITION: f32 = 10.0;

/// Ticks of travel version 2 wants before it believes a spin has started.
const TICKS_TO_ACTIVATE: i32 = 2;

/// How much longer than the threshold version 2 waits before calling a spin over.
const V2_EXPIRY_FACTOR: u64 = 2;

/// Which turntable algorithm a controller is read with, or none at all.
///
/// [`AnalogMode::Off`] is not a third algorithm: it means the axis is read as two ordinary keys
/// against a deadzone instead, which is what the reference does when analog scratch is switched off
/// (`BMControllerInputProcessor.java:232-238`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalogMode {
    #[default]
    Off,
    V1,
    V2,
}

impl AnalogMode {
    pub const ALL: [AnalogMode; 3] = [AnalogMode::Off, AnalogMode::V1, AnalogMode::V2];

    pub fn label(self) -> &'static str {
        match self {
            AnalogMode::Off => "OFF",
            AnalogMode::V1 => "V1",
            AnalogMode::V2 => "V2",
        }
    }
}

/// How many ticks the axis travelled between two readings, signed by direction.
///
/// A difference wider than the whole axis range is a wrap through the seam rather than a jump
/// across the disc, and is folded back by a turn and a half tick. Reference implementation
/// `BMControllerInputProcessor.java:218`.
pub fn compute_analog_diff(old_value: f32, new_value: f32) -> i32 {
    let span = 2.0 + TICK_MAX_SIZE / 2.0;
    let mut diff = new_value - old_value;
    if diff > 1.0 {
        diff -= span;
    } else if diff < -1.0 {
        diff += span;
    }
    diff /= TICK_MAX_SIZE;
    let ticks = if diff > 0.0 { diff.ceil() } else { diff.floor() };
    ticks as i32
}

/// One axis' turntable state. One machine per axis, queried once per direction.
#[derive(Clone, Debug)]
pub struct AnalogScratch {
    mode: AnalogMode,
    threshold: u32,
    counter: u64,
    tick_counter: i32,
    old_x: f32,
    active: bool,
    right: bool,
}

impl AnalogScratch {
    /// A machine that has not read the axis yet. The threshold is clamped here as well as on load,
    /// so a hand-edited key config cannot produce a machine that expires on every call.
    pub fn new(mode: AnalogMode, threshold: u32) -> AnalogScratch {
        AnalogScratch {
            mode,
            threshold: threshold.clamp(*ANALOG_THRESHOLD_RANGE.start(), *ANALOG_THRESHOLD_RANGE.end()),
            counter: 1,
            tick_counter: 0,
            old_x: UNREAD_POSITION,
            active: false,
            right: false,
        }
    }

    pub fn mode(&self) -> AnalogMode {
        self.mode
    }

    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Whether the axis is currently spinning, in whichever direction it was last seen going.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Whether the current spin is the rightward one.
    pub fn is_right(&self) -> bool {
        self.right
    }

    /// Read the axis once for one direction: `plus` asks whether the rightward lane is held.
    ///
    /// Every call advances the idle counter, so a lane bound to a turntable must query both
    /// directions every frame or the spin outlives the spinning.
    pub fn input(&mut self, current_x: f32, plus: bool) -> bool {
        match self.mode {
            AnalogMode::Off => false,
            AnalogMode::V1 => self.input_v1(current_x, plus),
            AnalogMode::V2 => self.input_v2(current_x, plus),
        }
    }

    fn held(&self, plus: bool) -> bool {
        if plus { self.active && self.right } else { self.active && !self.right }
    }

    /// True on the call that first reads the axis, which only records the position.
    fn absorbed_first_reading(&mut self, current_x: f32) -> bool {
        if self.old_x > 1.0 {
            self.old_x = current_x;
            self.active = false;
            return true;
        }
        false
    }

    fn input_v1(&mut self, current_x: f32, plus: bool) -> bool {
        if self.absorbed_first_reading(current_x) {
            return false;
        }

        if self.old_x != current_x {
            let now_right = v1_direction(self.old_x, current_x);
            if self.active && self.right != now_right {
                self.right = now_right;
            } else if !self.active {
                self.active = true;
                self.right = now_right;
            }
            self.counter = 0;
            self.old_x = current_x;
        }

        if self.counter > u64::from(self.threshold) && self.active {
            self.active = false;
            self.counter = 0;
        }

        if self.counter == u64::MAX {
            self.counter = 0;
        }
        self.counter += 1;

        self.held(plus)
    }

    fn input_v2(&mut self, current_x: f32, plus: bool) -> bool {
        if self.absorbed_first_reading(current_x) {
            return false;
        }

        if self.old_x != current_x {
            let ticks = compute_analog_diff(self.old_x, current_x);
            let now_right = ticks >= 0;
            if self.active && self.right != now_right {
                self.right = now_right;
                self.active = false;
                self.tick_counter = 0;
            } else if !self.active {
                if self.tick_counter == 0 || self.counter <= u64::from(self.threshold) {
                    self.tick_counter = self.tick_counter.saturating_add(ticks.saturating_abs());
                }
                if self.tick_counter >= TICKS_TO_ACTIVATE {
                    self.active = true;
                    self.right = now_right;
                }
            }
            self.counter = 0;
            self.old_x = current_x;
        }

        if self.counter > u64::from(self.threshold) * V2_EXPIRY_FACTOR {
            self.active = false;
            self.tick_counter = 0;
            self.counter = 0;
        }
        self.counter += 1;

        self.held(plus)
    }
}

/// Which way version 1 reads a movement: whichever of the two ways round the disc is shorter.
/// Reference implementation `BMControllerInputProcessor.java:365-377`.
fn v1_direction(old_x: f32, current_x: f32) -> bool {
    if old_x < current_x {
        return (current_x - old_x) <= (1.0 - current_x + old_x);
    }
    if old_x > current_x {
        return (old_x - current_x) > ((current_x + 1.0) - old_x);
    }
    false
}
