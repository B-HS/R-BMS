//! Gamepad input: button debouncing, two-key scratch axes and the analog turntable state machine.
//!
//! A controller reaches play through the same two things a keyboard does — a lane with a spin
//! direction, or an in-play control — so everything here ends at [`PadEvent`], which the frame
//! loop feeds to exactly the code a key press already feeds.
//!
//! Three things separate a controller from a keyboard, and each is answered here:
//!
//! * **Bounce.** A microswitch reports a burst of changes for one press. The reference
//!   implementation deafens an element for 16 ms after every accepted change
//!   (`BMControllerInputProcessor.java:61,136`) and so does [`ButtonGate`].
//! * **Axes that are keys.** A cheap scratch is two directions of one axis, each held once the
//!   axis passes a deadzone. That is [`PadBinding::Axis`].
//! * **Axes that spin.** A turntable reports an absolute position that wraps, and needs the state
//!   machine in [`analog`] to become a held lane. That is [`PadBinding::AnalogScratch`].
//!
//! The debounce state lives on the *element* — the physical button or axis direction — not on the
//! lane it is bound to, which is how the reference indexes it and what lets one element bound to
//! two lanes fire both without bouncing twice.
//!
//! Nothing here reads a device: [`PadMapper`] resolves a [`PadSnapshot`] of raw element values,
//! and [`PadState`] is the thin layer that fills that snapshot from gilrs. The mapping and both
//! turntable algorithms are therefore testable with no controller plugged in.
//!
//! Wired into the frame loop by the integration branch; see `docs/plan/phase-g-wiring/G5-gamepad.md`.

#![allow(dead_code)]

mod analog;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::time::Instant;

use rbms_model::Mode;
use rbms_play::ScratchDir;
use serde::{Deserialize, Serialize};

use crate::keyconfig::{ControlAction, mode_config_key};
use crate::notify::{Level, notify};

pub use analog::*;

/// How long an element is deaf after an accepted change. Reference implementation
/// `BMControllerInputProcessor.java:61`.
pub const DEFAULT_DEBOUNCE_MS: u32 = 16;

/// What the debounce window is clamped to on load. Reference implementation
/// `PlayModeConfig.java:148`.
pub const DEBOUNCE_MS_RANGE: RangeInclusive<u32> = 0..=100;

/// How far a two-key scratch axis travels before its direction counts as held. The reference
/// compares against a fixed 0.9 (`BMControllerInputProcessor.java:232-237`); rbms makes it a
/// setting because a stick bound as two keys never reaches that.
pub const DEFAULT_AXIS_DEADZONE: f32 = 0.2;

/// What the deadzone is clamped to, so a config can neither hold a lane down forever nor make it
/// unreachable.
pub const AXIS_DEADZONE_RANGE: RangeInclusive<f32> = 0.05..=0.95;

/// How far an axis must travel before the key editor offers it as a binding, kept well clear of
/// the resting jitter of a stick.
const CAPTURE_AXIS_THRESHOLD: f32 = 0.5;

const US_PER_MS: i64 = 1_000;

/// Where a scratch input came from. Only [`ScratchSource::Keyboard`] and [`ScratchSource::Pad`]
/// are produced today; the mouse wheel arm is the seam the reference implementation's
/// `MouseScratchInput.java` lands on, kept here so adding it does not reshape [`PadEvent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScratchSource {
    Keyboard,
    Pad,
    Mouse,
}

/// A device that produces the same lane and control events a gamepad does.
///
/// [`PadState`] is not an implementor: it needs the key config and the running mode to resolve an
/// event, which a MIDI backend does not. The trait is the shape such a backend is expected to
/// take, so that adding one is a new file rather than a new event type.
pub trait ExternalInput {
    fn poll(&mut self, now_us: i64) -> Vec<PadEvent>;
}

/// What one lane or control is bound to on a controller.
///
/// A button is stored as the platform's own event code rather than a gilrs `Button`, because an
/// arcade controller reports most of its buttons as `Button::Unknown` and the codes are the only
/// thing that tells them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PadBinding {
    Button(u32),
    /// One direction of one axis, held once it passes the deadzone.
    Axis {
        axis: u32,
        positive: bool,
    },
    /// A turntable. On a scratch lane it drives both spin directions from one binding, the way the
    /// reference drives its `AXISn_PLUS` and `AXISn_MINUS` from one shared algorithm.
    AnalogScratch {
        axis: u32,
    },
}

impl PadBinding {
    /// Short display token for the key editor's PAD column.
    pub fn label(self) -> String {
        match self {
            PadBinding::Button(code) => format!("BUTTON {code}"),
            PadBinding::Axis { axis, positive } => format!("AXIS {axis} {}", if positive { "+" } else { "-" }),
            PadBinding::AnalogScratch { axis } => format!("ANALOG {axis}"),
        }
    }

    /// The same axis read as a turntable, or `None` for a binding that has no axis to spin.
    pub fn as_analog_scratch(self) -> Option<PadBinding> {
        match self {
            PadBinding::Axis { axis, .. } | PadBinding::AnalogScratch { axis } => Some(PadBinding::AnalogScratch { axis }),
            PadBinding::Button(_) => None,
        }
    }
}

/// What the player's controller is bound to, stored inside the key config file.
///
/// It ships with nothing bound, so a fresh install behaves exactly as it did before a controller
/// was readable at all. `enabled` is on for that reason: an empty binding table is already inert,
/// and leaving it on means plugging a controller in and binding it is one screen rather than two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PadConfig {
    pub enabled: bool,
    /// Which controller to listen to, by name. `None` listens to every connected one, which is
    /// what a player with a single controller wants and what survives it being replugged.
    pub device_name: Option<String>,
    /// Forward bindings per mode, keyed by [`mode_config_key`] and indexed by lane.
    pub lanes: BTreeMap<String, Vec<Option<PadBinding>>>,
    /// The binding each scratch lane is spun backwards with, shaped like `lanes`. A turntable
    /// needs no entry here: it drives both directions from the forward slot.
    pub scratch_reverse: BTreeMap<String, Vec<Option<PadBinding>>>,
    /// In-play controls, keyed by [`ControlAction::label`].
    pub controls: BTreeMap<String, PadBinding>,
    pub debounce_ms: u32,
    pub analog_mode: AnalogMode,
    pub analog_threshold: u32,
    pub axis_deadzone: f32,
}

impl Default for PadConfig {
    fn default() -> Self {
        PadConfig {
            enabled: true,
            device_name: None,
            lanes: BTreeMap::new(),
            scratch_reverse: BTreeMap::new(),
            controls: BTreeMap::new(),
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            analog_mode: AnalogMode::default(),
            analog_threshold: DEFAULT_ANALOG_THRESHOLD,
            axis_deadzone: DEFAULT_AXIS_DEADZONE,
        }
    }
}

impl PadConfig {
    /// Pull every tunable back inside its range, so a hand-edited file cannot produce a controller
    /// that never releases or never fires. Called on load, and mirrored by the accessors below so
    /// an unsanitised config still behaves.
    pub fn sanitise(&mut self) {
        self.debounce_ms = self.effective_debounce_ms();
        self.analog_threshold = self.effective_analog_threshold();
        self.axis_deadzone = self.effective_axis_deadzone();
    }

    pub fn effective_debounce_ms(&self) -> u32 {
        self.debounce_ms.clamp(*DEBOUNCE_MS_RANGE.start(), *DEBOUNCE_MS_RANGE.end())
    }

    pub fn effective_analog_threshold(&self) -> u32 {
        self.analog_threshold.clamp(*ANALOG_THRESHOLD_RANGE.start(), *ANALOG_THRESHOLD_RANGE.end())
    }

    pub fn effective_axis_deadzone(&self) -> f32 {
        if self.axis_deadzone.is_nan() {
            return DEFAULT_AXIS_DEADZONE;
        }
        self.axis_deadzone.clamp(*AXIS_DEADZONE_RANGE.start(), *AXIS_DEADZONE_RANGE.end())
    }

    pub fn lane_binding(&self, mode: Mode, lane: usize) -> Option<PadBinding> {
        self.lanes.get(mode_config_key(mode)).and_then(|row| row.get(lane)).copied().flatten()
    }

    pub fn set_lane(&mut self, mode: Mode, lane: usize, binding: Option<PadBinding>) {
        set_slot(&mut self.lanes, mode, lane, binding);
    }

    pub fn scratch_reverse_binding(&self, mode: Mode, lane: usize) -> Option<PadBinding> {
        self.scratch_reverse.get(mode_config_key(mode)).and_then(|row| row.get(lane)).copied().flatten()
    }

    pub fn set_scratch_reverse(&mut self, mode: Mode, lane: usize, binding: Option<PadBinding>) {
        set_slot(&mut self.scratch_reverse, mode, lane, binding);
    }

    pub fn control_binding(&self, action: ControlAction) -> Option<PadBinding> {
        self.controls.get(action.label()).copied()
    }

    pub fn set_control(&mut self, action: ControlAction, binding: Option<PadBinding>) {
        match binding {
            Some(binding) => {
                self.controls.insert(action.label().to_string(), binding);
            }
            None => {
                self.controls.remove(action.label());
            }
        }
    }

    /// Bindings that fire more than one thing at once for `mode`, so the editor can flag them the
    /// way it flags a shared key.
    pub fn collisions(&self, mode: Mode) -> std::collections::HashSet<PadBinding> {
        let mut seen = std::collections::HashSet::new();
        let mut dup = std::collections::HashSet::new();
        let lanes = (0..mode.key).filter_map(|lane| self.lane_binding(mode, lane));
        let reverse = (0..mode.key).filter(|&lane| mode.is_scratch(lane)).filter_map(|lane| self.scratch_reverse_binding(mode, lane));
        let controls = ControlAction::ALL.into_iter().filter_map(|action| self.control_binding(action));
        for binding in lanes.chain(reverse).chain(controls) {
            if !seen.insert(binding) {
                dup.insert(binding);
            }
        }
        dup
    }

    /// The axes this config reads as turntables for `mode`, in lane order.
    fn analog_axes(&self, mode: Mode) -> Vec<u32> {
        let mut axes = Vec::new();
        for lane in 0..mode.key {
            let Some(PadBinding::AnalogScratch { axis }) = self.lane_binding(mode, lane) else { continue };
            if !axes.contains(&axis) {
                axes.push(axis);
            }
        }
        axes
    }

    /// Every element this config listens to for `mode`, paired with what it drives, in the order
    /// the reference walks its elements: a lane's forward direction before its backward one, and
    /// an axis' plus direction before its minus.
    fn plan(&self, mode: Mode) -> Vec<(PadElement, PadTarget)> {
        let mut plan = Vec::new();
        for lane in 0..mode.key {
            let Some(binding) = self.lane_binding(mode, lane) else { continue };
            match binding {
                PadBinding::Button(code) => plan.push((PadElement::Button(code), PadTarget::lane(lane, ScratchDir::Forward))),
                PadBinding::Axis { axis, positive } => plan.push((PadElement::AxisDir { axis, positive }, PadTarget::lane(lane, ScratchDir::Forward))),
                PadBinding::AnalogScratch { axis } => {
                    plan.push((PadElement::AxisDir { axis, positive: true }, PadTarget::lane(lane, ScratchDir::Forward)));
                    if mode.is_scratch(lane) {
                        plan.push((PadElement::AxisDir { axis, positive: false }, PadTarget::lane(lane, ScratchDir::Backward)));
                    }
                }
            }
        }
        for lane in (0..mode.key).filter(|&lane| mode.is_scratch(lane)) {
            match self.scratch_reverse_binding(mode, lane) {
                Some(PadBinding::Button(code)) => plan.push((PadElement::Button(code), PadTarget::lane(lane, ScratchDir::Backward))),
                Some(PadBinding::Axis { axis, positive }) => plan.push((PadElement::AxisDir { axis, positive }, PadTarget::lane(lane, ScratchDir::Backward))),
                Some(PadBinding::AnalogScratch { .. }) | None => {}
            }
        }
        for action in ControlAction::ALL {
            match self.control_binding(action) {
                Some(PadBinding::Button(code)) => plan.push((PadElement::Button(code), PadTarget::Control(action))),
                Some(PadBinding::Axis { axis, positive }) => plan.push((PadElement::AxisDir { axis, positive }, PadTarget::Control(action))),
                Some(PadBinding::AnalogScratch { .. }) | None => {}
            }
        }
        plan
    }
}

fn set_slot(rows: &mut BTreeMap<String, Vec<Option<PadBinding>>>, mode: Mode, lane: usize, binding: Option<PadBinding>) {
    let row = rows.entry(mode_config_key(mode).to_string()).or_insert_with(|| vec![None; mode.key]);
    if row.len() <= lane {
        row.resize(lane + 1, None);
    }
    row[lane] = binding;
}

/// One physical thing on the controller. Debounce is kept per element, not per lane, so an element
/// bound twice bounces once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PadElement {
    Button(u32),
    AxisDir { axis: u32, positive: bool },
}

/// What an element drives when it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PadTarget {
    Lane { lane: usize, dir: ScratchDir },
    Control(ControlAction),
}

impl PadTarget {
    fn lane(lane: usize, dir: ScratchDir) -> PadTarget {
        PadTarget::Lane { lane, dir }
    }
}

/// What a controller frame produced.
///
/// A lane carries its spin direction because rbms models a scratch as one lane spun two ways,
/// where the reference models it as two separate lane assignments. A control carries no state: it
/// fires on the press, like the key that does the same job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadEvent {
    Lane { lane: usize, dir: ScratchDir, press: bool },
    Control(ControlAction),
}

/// One element's accepted state and when it last changed.
#[derive(Clone, Copy, Debug)]
struct ButtonGate {
    pressed: bool,
    changed_us: i64,
}

impl ButtonGate {
    fn new() -> ButtonGate {
        ButtonGate { pressed: false, changed_us: i64::MIN }
    }

    /// Whether the debounce window has elapsed. Reference implementation
    /// `BMControllerInputProcessor.java:136`, inequality included.
    fn is_open(&self, now_us: i64, debounce_ms: u32) -> bool {
        now_us >= self.changed_us.saturating_add(i64::from(debounce_ms) * US_PER_MS)
    }

    fn set(&mut self, pressed: bool, now_us: i64) -> bool {
        if self.pressed == pressed {
            return false;
        }
        self.pressed = pressed;
        self.changed_us = now_us;
        true
    }
}

/// The raw value of every element the device last reported.
#[derive(Clone, Debug, Default)]
pub struct PadSnapshot {
    buttons: BTreeMap<u32, bool>,
    axes: BTreeMap<u32, f32>,
}

impl PadSnapshot {
    pub fn set_button(&mut self, code: u32, pressed: bool) {
        self.buttons.insert(code, pressed);
    }

    pub fn set_axis(&mut self, code: u32, value: f32) {
        self.axes.insert(code, value);
    }

    pub fn button(&self, code: u32) -> bool {
        self.buttons.get(&code).copied().unwrap_or(false)
    }

    pub fn axis(&self, code: u32) -> f32 {
        self.axes.get(&code).copied().unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.buttons.clear();
        self.axes.clear();
    }
}

/// Turns raw element values into lane and control events, holding the debounce and turntable state
/// that spans frames.
#[derive(Debug, Default)]
pub struct PadMapper {
    gates: BTreeMap<PadElement, ButtonGate>,
    analog: BTreeMap<u32, AnalogScratch>,
    analog_shape: Option<(AnalogMode, u32)>,
}

impl PadMapper {
    pub fn new() -> PadMapper {
        PadMapper::default()
    }

    /// Resolve one frame. Every element the config listens to is read at most once, and only when
    /// its debounce window is open — which is also what keeps a turntable's idle counter advancing
    /// at the same rate the reference advances it.
    pub fn resolve(&mut self, now_us: i64, cfg: &PadConfig, mode: Mode, snapshot: &PadSnapshot) -> Vec<PadEvent> {
        if !cfg.enabled {
            return Vec::new();
        }
        self.retune(cfg, mode);
        let plan = cfg.plan(mode);

        let debounce = cfg.effective_debounce_ms();
        let deadzone = cfg.effective_axis_deadzone();
        let mut order: Vec<PadElement> = Vec::new();
        for (element, _) in &plan {
            if !order.contains(element) {
                order.push(*element);
            }
        }

        let mut fired: BTreeMap<PadElement, bool> = BTreeMap::new();
        for element in order {
            if !self.gates.get(&element).is_none_or(|gate| gate.is_open(now_us, debounce)) {
                continue;
            }
            let raw = self.read(element, deadzone, snapshot);
            if self.gates.entry(element).or_insert_with(ButtonGate::new).set(raw, now_us) {
                fired.insert(element, raw);
            }
        }

        plan.into_iter()
            .filter_map(|(element, target)| {
                let pressed = *fired.get(&element)?;
                match target {
                    PadTarget::Lane { lane, dir } => Some(PadEvent::Lane { lane, dir, press: pressed }),
                    PadTarget::Control(action) => pressed.then_some(PadEvent::Control(action)),
                }
            })
            .collect()
    }

    /// Forget every turntable when the algorithm or its threshold changes, so a machine mid-spin
    /// under the old settings cannot leave a lane held under the new ones.
    fn retune(&mut self, cfg: &PadConfig, mode: Mode) {
        let shape = (cfg.analog_mode, cfg.effective_analog_threshold());
        if self.analog_shape != Some(shape) {
            self.analog.clear();
            self.analog_shape = Some(shape);
        }
        if cfg.analog_mode == AnalogMode::Off {
            return;
        }
        for axis in cfg.analog_axes(mode) {
            self.analog.entry(axis).or_insert_with(|| AnalogScratch::new(shape.0, shape.1));
        }
    }

    fn read(&mut self, element: PadElement, deadzone: f32, snapshot: &PadSnapshot) -> bool {
        match element {
            PadElement::Button(code) => snapshot.button(code),
            PadElement::AxisDir { axis, positive } => {
                let value = snapshot.axis(axis);
                match self.analog.get_mut(&axis) {
                    Some(machine) => machine.input(value, positive),
                    None if positive => value > deadzone,
                    None => value < -deadzone,
                }
            }
        }
    }
}

/// The controller, and the snapshot of it a frame is resolved against.
///
/// Held by the app for as long as it runs: the gilrs context is what knows a controller was
/// plugged in after start-up.
pub struct PadState {
    gilrs: gilrs::Gilrs,
    snapshot: PadSnapshot,
    mapper: PadMapper,
    capture: Option<PadBinding>,
    /// The controller's own clock. The debounce window is a difference between two of its
    /// readings, so it must not be the run clock, which is reset at the top of every chart.
    epoch: Instant,
}

impl PadState {
    /// Open the controller layer, or `None` if this machine has none to open, or the config asks
    /// for none — in which case the player carries on with the keyboard exactly as before.
    ///
    /// `enabled` is read once, here: it is a file-level switch for a player whose controller
    /// reports input it should not, not a row in the editor, so switching it takes a restart.
    ///
    /// gilrs' own deadzone and jitter filters are switched off: a turntable ticks at 0.009 and
    /// reports positions near zero as readily as anywhere else, so both filters would eat the
    /// movement the analog algorithms are looking for.
    ///
    /// A platform with no controller layer at all is not reported: it is not something the player
    /// asked for and not something they can act on. A layer that failed to open is.
    pub fn new(cfg: &PadConfig) -> Option<PadState> {
        if !cfg.enabled {
            return None;
        }
        match gilrs::GilrsBuilder::new().with_default_filters(false).build() {
            Ok(gilrs) => Some(PadState { gilrs, snapshot: PadSnapshot::default(), mapper: PadMapper::new(), capture: None, epoch: Instant::now() }),
            Err(gilrs::Error::NotImplemented(_)) => None,
            Err(e) => {
                notify(Level::Warn, format!("gamepad input unavailable ({e}); keyboard only"));
                None
            }
        }
    }

    /// The names of every connected controller, for the device row of the key editor.
    pub fn device_names(&self) -> Vec<String> {
        self.gilrs.gamepads().map(|(_, pad)| pad.name().to_string()).collect()
    }

    /// Microseconds since this controller layer was opened.
    pub fn now_us(&self) -> i64 {
        self.epoch.elapsed().as_micros() as i64
    }

    /// One frame of controller input read against the controller's own clock, which is the call
    /// the frame loop makes.
    pub fn poll_now(&mut self, cfg: &PadConfig, mode: Mode) -> Vec<PadEvent> {
        let now_us = self.now_us();
        self.poll(now_us, cfg, mode)
    }

    /// One frame of controller input, already debounced.
    pub fn poll(&mut self, now_us: i64, cfg: &PadConfig, mode: Mode) -> Vec<PadEvent> {
        self.pump(cfg);
        self.mapper.resolve(now_us, cfg, mode, &self.snapshot)
    }

    /// The binding the player last produced, for the key editor's "press something" mode. Taking
    /// it clears it, so the next binding captured is the next thing pressed.
    pub fn take_capture(&mut self, cfg: &PadConfig) -> Option<PadBinding> {
        self.pump(cfg);
        self.capture.take()
    }

    /// Drop whatever was pressed before capture mode was entered.
    pub fn drain(&mut self, cfg: &PadConfig) {
        self.pump(cfg);
        self.capture = None;
    }

    fn pump(&mut self, cfg: &PadConfig) {
        while let Some(event) = self.gilrs.next_event() {
            if matches!(event.event, gilrs::EventType::Disconnected) {
                self.snapshot.clear();
                continue;
            }
            if !self.accepts(cfg, event.id) {
                continue;
            }
            match event.event {
                gilrs::EventType::ButtonPressed(_, code) => {
                    let code = code.into_u32();
                    self.snapshot.set_button(code, true);
                    self.capture = Some(PadBinding::Button(code));
                }
                gilrs::EventType::ButtonReleased(_, code) => self.snapshot.set_button(code.into_u32(), false),
                gilrs::EventType::AxisChanged(_, value, code) => {
                    let axis = code.into_u32();
                    self.snapshot.set_axis(axis, value);
                    if value.abs() >= CAPTURE_AXIS_THRESHOLD {
                        self.capture = Some(PadBinding::Axis { axis, positive: value > 0.0 });
                    }
                }
                _ => {}
            }
        }
    }

    fn accepts(&self, cfg: &PadConfig, id: gilrs::GamepadId) -> bool {
        match cfg.device_name.as_deref() {
            None => true,
            Some(want) => self.gilrs.connected_gamepad(id).is_some_and(|pad| pad.name() == want),
        }
    }
}
