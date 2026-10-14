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
//! A button or a two-key axis is resolved change by change, each at the instant the controller
//! layer stamped it ([`PadMapper::resolve_changes`]): a press and a release that both arrive
//! between two frames are both played, each at its own time, and the debounce window is measured
//! between those instants. Only a turntable is still read once a poll, since its state machines
//! count readings rather than time.
//!
//! Wired into the frame loop by the integration branch; see `docs/plan/phase-g-wiring/G5-gamepad.md`.

mod analog;
#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::time::{Duration, Instant, SystemTime};

use rbms_model::Mode;
use rbms_play::ScratchDir;
use serde::{Deserialize, Serialize};

use crate::keyconfig::{Bindable, ControlAction, Shortcut, mode_config_key};
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

/// A button every controller layout names the same way, whatever event code the platform gives it.
///
/// Only the two menu buttons are modelled: they are what START and SELECT ship bound to, on a pad
/// whose native codes are not known in advance. Everything else on a controller is bound by the
/// native code the key editor captures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StandardButton {
    Start,
    Select,
}

impl StandardButton {
    fn from_gilrs(button: gilrs::Button) -> Option<StandardButton> {
        match button {
            gilrs::Button::Start => Some(StandardButton::Start),
            gilrs::Button::Select => Some(StandardButton::Select),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            StandardButton::Start => "START",
            StandardButton::Select => "SELECT",
        }
    }
}

/// What one lane or control is bound to on a controller.
///
/// A button is stored as the platform's own event code rather than a gilrs `Button`, because an
/// arcade controller reports most of its buttons as `Button::Unknown` and the codes are the only
/// thing that tells them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PadBinding {
    Button(u32),
    /// A menu button named by what it is rather than by its code.
    Standard(StandardButton),
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
            PadBinding::Standard(button) => format!("STANDARD {}", button.label()),
            PadBinding::Axis { axis, positive } => format!("AXIS {axis} {}", if positive { "+" } else { "-" }),
            PadBinding::AnalogScratch { axis } => format!("ANALOG {axis}"),
        }
    }

    /// The same axis read as a turntable, or `None` for a binding that has no axis to spin.
    pub fn as_analog_scratch(self) -> Option<PadBinding> {
        match self {
            PadBinding::Axis { axis, .. } | PadBinding::AnalogScratch { axis } => Some(PadBinding::AnalogScratch { axis }),
            PadBinding::Button(_) | PadBinding::Standard(_) => None,
        }
    }
}

/// What a control reads on a controller until the player binds it to something else: START and
/// SELECT are the pad's own menu buttons, every other control starts unbound.
fn default_control_binding(action: ControlAction) -> Option<PadBinding> {
    match action {
        ControlAction::Start => Some(PadBinding::Standard(StandardButton::Start)),
        ControlAction::Select => Some(PadBinding::Standard(StandardButton::Select)),
        _ => None,
    }
}

/// What the player's controller is bound to, stored inside the key config file.
///
/// It ships with no lane bound, so a fresh install plays exactly as it did before a controller was
/// readable at all; START and SELECT alone start on the pad's menu buttons, which only the screens
/// that read them react to. `enabled` is on for that reason: a table with no lane bound is already
/// inert, and leaving it on means plugging a controller in and binding it is one screen rather
/// than two.
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
    /// Control bindings the player has set, keyed by [`ControlAction::config_key`]. A control with
    /// no entry reads its default (see [`PadConfig::control_binding`]).
    pub controls: BTreeMap<String, PadBinding>,
    /// Shortcut bindings the player has set, keyed by [`Shortcut::config_key`]. None ships bound,
    /// and a key config written before a controller could be given one reads none.
    pub shortcuts: BTreeMap<String, PadBinding>,
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
            shortcuts: BTreeMap::new(),
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

    /// What a control is bound to: the player's binding if there is one, otherwise its default,
    /// which is the standard menu button for START and SELECT and nothing for the rest. A key
    /// config written before START and SELECT existed therefore reads them on those buttons.
    pub fn control_binding(&self, action: ControlAction) -> Option<PadBinding> {
        self.controls.get(action.config_key()).copied().or_else(|| default_control_binding(action))
    }

    /// Bind a control, or with `None` drop the player's binding so it reads its default again.
    pub fn set_control(&mut self, action: ControlAction, binding: Option<PadBinding>) {
        match binding {
            Some(binding) => {
                self.controls.insert(action.config_key().to_string(), binding);
            }
            None => {
                self.controls.remove(action.config_key());
            }
        }
    }

    /// What a shortcut is bound to on the controller, which is nothing until the player binds it,
    /// and nothing ever for a shortcut a controller cannot be given ([`Shortcut::takes_a_pad`]).
    pub fn shortcut_binding(&self, shortcut: Shortcut) -> Option<PadBinding> {
        self.shortcuts.get(shortcut.config_key()).copied().filter(|_| shortcut.takes_a_pad())
    }

    /// Bind a shortcut, or with `None` unbind it.
    pub fn set_shortcut(&mut self, shortcut: Shortcut, binding: Option<PadBinding>) {
        match binding {
            Some(binding) if shortcut.takes_a_pad() => {
                self.shortcuts.insert(shortcut.config_key().to_string(), binding);
            }
            Some(_) | None => {
                self.shortcuts.remove(shortcut.config_key());
            }
        }
    }

    /// What something is bound to on the controller, for the mode being edited.
    pub fn binding_of(&self, mode: Mode, target: Bindable) -> Option<PadBinding> {
        match target {
            Bindable::Lane(lane) => self.lane_binding(mode, lane),
            Bindable::ScratchReverse(lane) => self.scratch_reverse_binding(mode, lane),
            Bindable::Control(action) => self.control_binding(action),
            Bindable::Shortcut(shortcut) => self.shortcut_binding(shortcut),
        }
    }

    /// Bind something on the controller, or with `None` take the player's binding away. START and
    /// SELECT then read the pad's own menu buttons again, as they do in a fresh key config.
    pub fn bind(&mut self, mode: Mode, target: Bindable, binding: Option<PadBinding>) {
        match target {
            Bindable::Lane(lane) => self.set_lane(mode, lane, binding),
            Bindable::ScratchReverse(lane) => self.set_scratch_reverse(mode, lane, binding),
            Bindable::Control(action) => self.set_control(action, binding),
            Bindable::Shortcut(shortcut) => self.set_shortcut(shortcut, binding),
        }
    }

    /// Drop every binding the player made, leaving the tuning of the controller itself alone.
    pub fn restore_bindings(&mut self) {
        self.lanes.clear();
        self.scratch_reverse.clear();
        self.controls.clear();
        self.shortcuts.clear();
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
                PadBinding::Standard(button) => plan.push((PadElement::Standard(button), PadTarget::lane(lane, ScratchDir::Forward))),
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
                Some(PadBinding::Standard(button)) => plan.push((PadElement::Standard(button), PadTarget::lane(lane, ScratchDir::Backward))),
                Some(PadBinding::Axis { axis, positive }) => plan.push((PadElement::AxisDir { axis, positive }, PadTarget::lane(lane, ScratchDir::Backward))),
                Some(PadBinding::AnalogScratch { .. }) | None => {}
            }
        }
        for action in ControlAction::ALL {
            match self.control_binding(action) {
                Some(PadBinding::Button(code)) => plan.push((PadElement::Button(code), PadTarget::Control(action))),
                Some(PadBinding::Standard(button)) => plan.push((PadElement::Standard(button), PadTarget::Control(action))),
                Some(PadBinding::Axis { axis, positive }) => plan.push((PadElement::AxisDir { axis, positive }, PadTarget::Control(action))),
                Some(PadBinding::AnalogScratch { .. }) | None => {}
            }
        }
        for shortcut in Shortcut::ALL {
            match self.shortcut_binding(shortcut) {
                Some(PadBinding::Button(code)) => plan.push((PadElement::Button(code), PadTarget::Shortcut(shortcut))),
                Some(PadBinding::Standard(button)) => plan.push((PadElement::Standard(button), PadTarget::Shortcut(shortcut))),
                Some(PadBinding::Axis { axis, positive }) => plan.push((PadElement::AxisDir { axis, positive }, PadTarget::Shortcut(shortcut))),
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
    Standard(StandardButton),
    AxisDir { axis: u32, positive: bool },
}

/// What an element drives when it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PadTarget {
    Lane { lane: usize, dir: ScratchDir },
    Control(ControlAction),
    Shortcut(Shortcut),
}

impl PadTarget {
    fn lane(lane: usize, dir: ScratchDir) -> PadTarget {
        PadTarget::Lane { lane, dir }
    }

    /// The event this target makes when its element changes to `pressed`. A control and a shortcut
    /// fire only on the way down.
    fn event(self, pressed: bool) -> Option<PadEvent> {
        match self {
            PadTarget::Lane { lane, dir } => Some(PadEvent::Lane { lane, dir, press: pressed }),
            PadTarget::Control(action) => pressed.then_some(PadEvent::Control(action)),
            PadTarget::Shortcut(shortcut) => pressed.then_some(PadEvent::Shortcut(shortcut)),
        }
    }
}

/// What a controller frame produced.
///
/// A lane carries its spin direction because rbms models a scratch as one lane spun two ways,
/// where the reference models it as two separate lane assignments. A control carries no state: it
/// fires on the press, like the key that does the same job, and so does a shortcut. What is still
/// held afterwards is read back with [`PadState::held`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadEvent {
    Lane { lane: usize, dir: ScratchDir, press: bool },
    Control(ControlAction),
    Shortcut(Shortcut),
}

/// One raw change the controller reported, before any mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PadChange {
    Button { code: u32, standard: Option<StandardButton>, pressed: bool },
    Axis { code: u32, value: f32 },
    Disconnected,
}

impl PadChange {
    fn apply(self, snapshot: &mut PadSnapshot) {
        match self {
            PadChange::Button { code, standard, pressed } => {
                snapshot.set_button(code, pressed);
                if let Some(standard) = standard {
                    snapshot.set_standard(standard, pressed);
                }
            }
            PadChange::Axis { code, value } => snapshot.set_axis(code, value),
            PadChange::Disconnected => snapshot.clear(),
        }
    }
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
        now_us >= self.opens_at(debounce_ms)
    }

    /// The first instant the debounce window has elapsed at.
    fn opens_at(&self, debounce_ms: u32) -> i64 {
        self.changed_us.saturating_add(i64::from(debounce_ms) * US_PER_MS)
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
    standard: BTreeMap<StandardButton, bool>,
    axes: BTreeMap<u32, f32>,
}

impl PadSnapshot {
    pub fn set_button(&mut self, code: u32, pressed: bool) {
        self.buttons.insert(code, pressed);
    }

    pub fn set_standard(&mut self, button: StandardButton, pressed: bool) {
        self.standard.insert(button, pressed);
    }

    pub fn set_axis(&mut self, code: u32, value: f32) {
        self.axes.insert(code, value);
    }

    pub fn button(&self, code: u32) -> bool {
        self.buttons.get(&code).copied().unwrap_or(false)
    }

    pub fn standard(&self, button: StandardButton) -> bool {
        self.standard.get(&button).copied().unwrap_or(false)
    }

    pub fn axis(&self, code: u32) -> f32 {
        self.axes.get(&code).copied().unwrap_or_default()
    }

    /// Where an axis stands, or `None` for one the device has not reported yet.
    pub fn axis_reading(&self, code: u32) -> Option<f32> {
        self.axes.get(&code).copied()
    }

    pub fn clear(&mut self) {
        self.buttons.clear();
        self.standard.clear();
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
    /// Where each turntable stood when its travel was last counted.
    turn_read: BTreeMap<u32, f32>,
    /// How many ticks the turntables have travelled since the count was last taken, a tick towards
    /// the plus end of an axis counted up ([`PadMapper::take_turn`]).
    turned: i32,
}

impl PadMapper {
    pub fn new() -> PadMapper {
        PadMapper::default()
    }

    /// Resolve one frame from a snapshot alone: every element the config listens to read at
    /// `now_us`, at most once and only when its debounce window is open — which is also what keeps
    /// a turntable's idle counter advancing at the same rate the reference advances it. The frame
    /// loop resolves change by change instead ([`PadMapper::resolve_changes`]); this is what the
    /// tests of the element rules drive.
    #[cfg(test)]
    pub fn resolve(&mut self, now_us: i64, cfg: &PadConfig, mode: Mode, snapshot: &PadSnapshot) -> Vec<PadEvent> {
        if !cfg.enabled {
            return Vec::new();
        }
        self.retune(cfg, mode);
        self.count_turn(snapshot);
        let plan = cfg.plan(mode);
        self.read_elements(now_us, cfg, &plan, snapshot, |_, _| true)
    }

    /// Resolve one poll of raw changes, each at the instant on the controller clock it happened at,
    /// into the events they make, each with its own instant, earliest first. `snapshot` is brought
    /// up to date on the way.
    ///
    /// A button or two-key axis direction is read change by change: a change the debounce window
    /// keeps out is taken the instant the window opens, if the element still reads that way then,
    /// and one a later change undid before that is never taken at all, which is what the window is
    /// for. A press and its release inside one poll are therefore both played. A turntable is read
    /// once, at `now_us`, after every change has been taken in.
    pub fn resolve_changes(
        &mut self,
        changes: &[(i64, PadChange)],
        now_us: i64,
        cfg: &PadConfig,
        mode: Mode,
        snapshot: &mut PadSnapshot,
    ) -> Vec<(PadEvent, i64)> {
        if !cfg.enabled {
            changes.iter().for_each(|(_, change)| change.apply(snapshot));
            return Vec::new();
        }
        self.retune(cfg, mode);
        let plan = cfg.plan(mode);
        let mut events = Vec::new();
        for &(at_us, change) in changes {
            self.take_opened(at_us, cfg, &plan, snapshot, &mut events);
            change.apply(snapshot);
            let keys = self.read_elements(at_us, cfg, &plan, snapshot, |mapper, element| !mapper.is_turntable(element));
            events.extend(keys.into_iter().map(|event| (event, at_us)));
        }
        self.take_opened(now_us, cfg, &plan, snapshot, &mut events);
        self.count_turn(snapshot);
        let turntables = self.read_elements(now_us, cfg, &plan, snapshot, |mapper, element| mapper.is_turntable(element));
        events.extend(turntables.into_iter().map(|event| (event, now_us)));
        events
    }

    /// Whether `element` is a direction of an axis read as a turntable.
    fn is_turntable(&self, element: PadElement) -> bool {
        matches!(element, PadElement::AxisDir { axis, .. } if self.analog.contains_key(&axis))
    }

    /// Read every element of `plan` that `wanted` keeps, at `at_us`: each at most once, and only
    /// while its debounce window is open. What changed is handed back as the plan's events, in plan
    /// order.
    fn read_elements(
        &mut self,
        at_us: i64,
        cfg: &PadConfig,
        plan: &[(PadElement, PadTarget)],
        snapshot: &PadSnapshot,
        wanted: impl Fn(&PadMapper, PadElement) -> bool,
    ) -> Vec<PadEvent> {
        let debounce = cfg.effective_debounce_ms();
        let deadzone = cfg.effective_axis_deadzone();
        let mut order: Vec<PadElement> = Vec::new();
        for (element, _) in plan {
            if !order.contains(element) && wanted(self, *element) {
                order.push(*element);
            }
        }
        let mut fired: BTreeMap<PadElement, bool> = BTreeMap::new();
        for element in order {
            if !self.gates.get(&element).is_none_or(|gate| gate.is_open(at_us, debounce)) {
                continue;
            }
            let raw = self.read(element, deadzone, snapshot);
            if self.gates.entry(element).or_insert_with(ButtonGate::new).set(raw, at_us) {
                fired.insert(element, raw);
            }
        }
        plan.iter().filter_map(|(element, target)| target.event(*fired.get(element)?)).collect()
    }

    /// Take every key change the debounce window kept out and has opened for by `until_us`, at the
    /// instant it opened, earliest first.
    fn take_opened(&mut self, until_us: i64, cfg: &PadConfig, plan: &[(PadElement, PadTarget)], snapshot: &PadSnapshot, events: &mut Vec<(PadEvent, i64)>) {
        let debounce = cfg.effective_debounce_ms();
        let deadzone = cfg.effective_axis_deadzone();
        let mut opened: Vec<(i64, PadElement, bool)> = Vec::new();
        for (element, _) in plan {
            if self.is_turntable(*element) || opened.iter().any(|(_, seen, _)| seen == element) {
                continue;
            }
            let raw = self.read(*element, deadzone, snapshot);
            let Some(gate) = self.gates.get(element).filter(|gate| gate.pressed != raw) else { continue };
            let opens_us = gate.opens_at(debounce);
            if opens_us <= until_us {
                opened.push((opens_us, *element, raw));
            }
        }
        opened.sort_by_key(|(opens_us, _, _)| *opens_us);
        for (opens_us, element, raw) in opened {
            if self.gates.entry(element).or_insert_with(ButtonGate::new).set(raw, opens_us) {
                events
                    .extend(plan.iter().filter(|(planned, _)| *planned == element).filter_map(|(_, target)| target.event(raw)).map(|event| (event, opens_us)));
            }
        }
    }

    /// Everything the controller is holding down as of the last [`PadMapper::resolve_changes`], as the
    /// events that put it there, each once. It reads the accepted (debounced) state of every
    /// element the config listens to, so a lane held by two elements stays held until both let go.
    pub fn held(&self, cfg: &PadConfig, mode: Mode) -> Vec<PadEvent> {
        if !cfg.enabled {
            return Vec::new();
        }
        let mut held = Vec::new();
        for (element, target) in cfg.plan(mode) {
            if !self.gates.get(&element).is_some_and(|gate| gate.pressed) {
                continue;
            }
            let event = match target {
                PadTarget::Lane { lane, dir } => PadEvent::Lane { lane, dir, press: true },
                PadTarget::Control(action) => PadEvent::Control(action),
                PadTarget::Shortcut(shortcut) => PadEvent::Shortcut(shortcut),
            };
            if !held.contains(&event) {
                held.push(event);
            }
        }
        held
    }

    /// Forget every turntable when the algorithm or its threshold changes, so a machine mid-spin
    /// under the old settings cannot leave a lane held under the new ones.
    fn retune(&mut self, cfg: &PadConfig, mode: Mode) {
        let shape = (cfg.analog_mode, cfg.effective_analog_threshold());
        if self.analog_shape != Some(shape) {
            self.analog.clear();
            self.turn_read.clear();
            self.turned = 0;
            self.analog_shape = Some(shape);
        }
        if cfg.analog_mode == AnalogMode::Off {
            return;
        }
        for axis in cfg.analog_axes(mode) {
            self.analog.entry(axis).or_insert_with(|| AnalogScratch::new(shape.0, shape.1));
        }
    }

    /// Add what each turntable travelled since the last frame to the count, in the ticks the
    /// reference counts a turn in (`BMSPlayerInputProcessor.getAnalogDiff`). An axis is counted from
    /// its first reading on, so the jump from nothing to wherever the disc happens to stand is not
    /// a turn.
    fn count_turn(&mut self, snapshot: &PadSnapshot) {
        for &axis in self.analog.keys() {
            let Some(now) = snapshot.axis_reading(axis) else { continue };
            if let Some(before) = self.turn_read.insert(axis, now) {
                self.turned = self.turned.saturating_add(compute_analog_diff(before, now));
            }
        }
    }

    /// How many ticks the turntables have turned since this was last asked, a turn that holds the
    /// scratch lane's forward direction counted up, or `None` when no axis is read as a turntable:
    /// the lanes are then keys, and are read as held or not.
    ///
    /// It is what the reference moves a cover by while START is held
    /// (`ControlInputProcessor.changeCoverValue`, for an analogue key). The reference drops a turn
    /// older than 200 ms; here whoever reads it asks every frame, and drops what it has no use for.
    pub fn take_turn(&mut self) -> Option<i32> {
        if self.analog.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.turned))
    }

    fn read(&mut self, element: PadElement, deadzone: f32, snapshot: &PadSnapshot) -> bool {
        match element {
            PadElement::Button(code) => snapshot.button(code),
            PadElement::Standard(button) => snapshot.standard(button),
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

/// Where an event gilrs stamped at `stamped` happened on the controller clock: as long before
/// `now_us` as the wall clock says it is old. The wall clock is not monotonic, so the instant is
/// held between `earliest_us`, the last one handed out, and `now_us`.
pub fn stamped_instant_us(stamped: SystemTime, wall_now: SystemTime, now_us: i64, earliest_us: i64) -> i64 {
    let age_us = wall_now.duration_since(stamped).unwrap_or(Duration::ZERO).as_micros();
    let at_us = now_us.saturating_sub(i64::try_from(age_us).unwrap_or(i64::MAX));
    at_us.clamp(earliest_us.min(now_us), now_us)
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
    /// The instant on that clock the last change was stamped at, which no later one goes before.
    last_change_us: i64,
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
            Ok(gilrs) => Some(PadState {
                gilrs,
                snapshot: PadSnapshot::default(),
                mapper: PadMapper::new(),
                capture: None,
                epoch: Instant::now(),
                last_change_us: i64::MIN,
            }),
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

    /// Everything the controller did since the last poll, each with the instant it happened at,
    /// earliest first, already debounced. This is the call the frame loop makes: the instants are
    /// the ones gilrs stamped each change with as it arrived, not the frame's, so a press is judged
    /// at the moment it was made however late in its frame it is read.
    pub fn poll_now(&mut self, cfg: &PadConfig, mode: Mode) -> Vec<(PadEvent, Instant)> {
        let now_us = self.now_us();
        let changes = self.pump(cfg, now_us, SystemTime::now());
        let events = self.mapper.resolve_changes(&changes, now_us, cfg, mode, &mut self.snapshot);
        events.into_iter().map(|(event, at_us)| (event, self.epoch + Duration::from_micros(at_us.max(0) as u64))).collect()
    }

    /// What the controller is holding down right now, as of the last poll.
    pub fn held(&self, cfg: &PadConfig, mode: Mode) -> Vec<PadEvent> {
        self.mapper.held(cfg, mode)
    }

    /// How far the turntables have turned since this was last asked ([`PadMapper::take_turn`]).
    pub fn take_turn(&mut self) -> Option<i32> {
        self.mapper.take_turn()
    }

    /// The binding the player last produced, for the key editor's "press something" mode. Taking
    /// it clears it, so the next binding captured is the next thing pressed.
    pub fn take_capture(&mut self, cfg: &PadConfig) -> Option<PadBinding> {
        self.pump_into_snapshot(cfg);
        self.capture.take()
    }

    /// Drop whatever was pressed before capture mode was entered.
    pub fn drain(&mut self, cfg: &PadConfig) {
        self.pump_into_snapshot(cfg);
        self.capture = None;
    }

    /// Take in what the controller did without playing any of it, for the key editor.
    fn pump_into_snapshot(&mut self, cfg: &PadConfig) {
        let changes = self.pump(cfg, self.now_us(), SystemTime::now());
        changes.into_iter().for_each(|(_, change)| change.apply(&mut self.snapshot));
    }

    /// Every change gilrs has queued since the last pump, each at the instant it was stamped with
    /// on the controller clock, noting the last thing pressed for the key editor on the way.
    fn pump(&mut self, cfg: &PadConfig, now_us: i64, wall_now: SystemTime) -> Vec<(i64, PadChange)> {
        let mut changes = Vec::new();
        while let Some(event) = self.gilrs.next_event() {
            let at_us = stamped_instant_us(event.time, wall_now, now_us, self.last_change_us);
            if matches!(event.event, gilrs::EventType::Disconnected) {
                changes.push((at_us, PadChange::Disconnected));
                continue;
            }
            if !self.accepts(cfg, event.id) {
                continue;
            }
            let change = match event.event {
                gilrs::EventType::ButtonPressed(button, code) => {
                    let code = code.into_u32();
                    self.capture = Some(PadBinding::Button(code));
                    PadChange::Button { code, standard: StandardButton::from_gilrs(button), pressed: true }
                }
                gilrs::EventType::ButtonReleased(button, code) => {
                    PadChange::Button { code: code.into_u32(), standard: StandardButton::from_gilrs(button), pressed: false }
                }
                gilrs::EventType::AxisChanged(_, value, code) => {
                    let axis = code.into_u32();
                    if value.abs() >= CAPTURE_AXIS_THRESHOLD {
                        self.capture = Some(PadBinding::Axis { axis, positive: value > 0.0 });
                    }
                    PadChange::Axis { code: axis, value }
                }
                _ => continue,
            };
            self.last_change_us = at_us;
            changes.push((at_us, change));
        }
        changes
    }

    fn accepts(&self, cfg: &PadConfig, id: gilrs::GamepadId) -> bool {
        match cfg.device_name.as_deref() {
            None => true,
            Some(want) => self.gilrs.connected_gamepad(id).is_some_and(|pad| pad.name() == want),
        }
    }
}
