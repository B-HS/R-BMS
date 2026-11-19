//! Gamepad input: button debouncing, two-key scratch axes and the analog turntable state machine.
//!
//! A controller reaches play through the same two things a keyboard does — a lane with a spin
//! direction, or an in-play control — so everything here ends at [`PadEvent`], which the frame
//! loop feeds to exactly the code a key press already feeds.
//!
//! Three things separate a controller from a keyboard, and each is answered here:
//!
//! * **Bounce.** A microswitch reports a burst of changes for one press. A button and a two-key
//!   axis direction are debounced the way LR2 debounces every key and button
//!   ([`crate::input_debounce`], the same gate the keyboard's lane keys go through): a press is
//!   never late, a release inside PAD DEBOUNCE of its press waits for the window to end. A
//!   turntable direction keeps the reference implementation's lock, which deafens it for the
//!   window after every accepted change (`BMControllerInputProcessor.java:61,136`,
//!   [`PadMapper::read_turntables`]), because the turntable algorithms count the readings that lock
//!   lets through. Both read and write the same state of the element, so an axis that turns from
//!   a turntable into two keys, or back, when the algorithm is switched carries what it held.
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
//! layer stamped it ([`PadMapper::resolve_changes_marked`]): a press and a release that both arrive
//! between two frames are both played, each at its own time, and the debounce window is measured
//! between those instants. Only a turntable is still read once a poll, since its state machines
//! count readings rather than time.
//!
//! Wired into the frame loop by the integration branch; see `docs/plan/phase-g-wiring/G5-gamepad.md`.

mod analog;
#[cfg(test)]
mod chord_tests;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::time::{Duration, Instant, SystemTime};

use rbms_model::Mode;
use rbms_play::ScratchDir;
use serde::{Deserialize, Serialize};

use crate::input_debounce::{Lr2Debounce, ReportedAt};
use crate::keyconfig::{Bindable, ControlAction, Shortcut, mode_config_key};
use crate::notify::{Level, notify};

pub use analog::*;

/// PAD DEBOUNCE as shipped: how long after a press a release waits, and how long a turntable
/// direction is deaf after an accepted change ([`rbms_config::DEFAULT_PAD_DEBOUNCE_MS`]).
pub const DEFAULT_DEBOUNCE_MS: u32 = rbms_config::DEFAULT_PAD_DEBOUNCE_MS;

/// What the debounce window is clamped to on load: the two ends of the PAD DEBOUNCE row, the
/// reference's own bound (`PlayModeConfig.java:148`).
pub const DEBOUNCE_MS_RANGE: RangeInclusive<u32> = rbms_config::PAD_DEBOUNCE_MIN_MS..=rbms_config::PAD_DEBOUNCE_MAX_MS;

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

    /// Whether it is pressed and let go like a key -- a button, a menu button or a direction of an
    /// axis -- and so can be one of two elements held together. A turntable spins.
    pub fn is_a_key(self) -> bool {
        !matches!(self, PadBinding::AnalogScratch { .. })
    }

    /// The display token of two elements held together, as the key editor shows them.
    pub fn chord_label(first: PadBinding, second: Option<PadBinding>) -> String {
        match second {
            Some(second) => format!("{}{}{}", first.label(), crate::keyconfig::CHORD_LABEL_JOIN, second.label()),
            None => first.label(),
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
    /// The second element of each control the player bound to two elements held together, keyed as
    /// `controls` is; the first is the control's entry there. A key config written before two
    /// elements could be held together has no such table and binds every control to one element.
    pub control_chords: BTreeMap<String, PadBinding>,
    /// The second element of each shortcut bound to two elements held together, keyed as
    /// `shortcuts` is.
    pub shortcut_chords: BTreeMap<String, PadBinding>,
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
            control_chords: BTreeMap::new(),
            shortcut_chords: BTreeMap::new(),
            debounce_ms: DEFAULT_DEBOUNCE_MS,
            analog_mode: AnalogMode::default(),
            analog_threshold: DEFAULT_ANALOG_THRESHOLD,
            axis_deadzone: DEFAULT_AXIS_DEADZONE,
        }
    }
}

impl PadConfig {
    /// Pull every tunable back inside its range, so a hand-edited file cannot produce a controller
    /// that never releases or never fires, and let go of every pair of elements with a direction of
    /// a turntable in it ([`PadConfig::is_on_a_turntable`]), which a file written before such a pair
    /// was refused may hold. Called whenever a config is read, and mirrored by the accessors below
    /// so an unsanitised config still behaves.
    pub fn sanitise(&mut self) {
        self.debounce_ms = self.effective_debounce_ms();
        self.analog_threshold = self.effective_analog_threshold();
        self.axis_deadzone = self.effective_axis_deadzone();
        self.forget_pairs_on_turntables();
    }

    /// The turntables a lane of some mode reads, each as [`PadBinding::AnalogScratch`].
    fn turntables(&self) -> Vec<PadBinding> {
        self.lanes.values().flatten().flatten().copied().filter(|binding| !binding.is_a_key()).collect()
    }

    /// Whether `binding` is an axis, or a direction of one, that a lane of some mode reads as a
    /// turntable. Such an axis spins, and is never one of two elements held together: a pair is
    /// bound for every mode, and in the mode whose turntable it is the axis would be read as both.
    pub fn is_on_a_turntable(&self, binding: PadBinding) -> bool {
        binding.as_analog_scratch().is_some_and(|axis| self.turntables().contains(&axis))
    }

    /// Let go of every control and shortcut bound to a pair one of whose elements is on a turntable:
    /// both of its elements, so that it reads what it ships with.
    fn forget_pairs_on_turntables(&mut self) {
        let turntables = self.turntables();
        let on_one = |binding: &PadBinding| binding.as_analog_scratch().is_some_and(|axis| turntables.contains(&axis));
        for (firsts, seconds) in [(&mut self.controls, &mut self.control_chords), (&mut self.shortcuts, &mut self.shortcut_chords)] {
            let broken: Vec<String> =
                seconds.iter().filter(|(name, second)| on_one(second) || firsts.get(*name).is_some_and(on_one)).map(|(name, _)| name.clone()).collect();
            for name in broken {
                seconds.remove(&name);
                firsts.remove(&name);
            }
        }
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

    /// Bind a control to one element, or with `None` drop the player's binding so it reads its
    /// default again. Either way a second element it was held with is dropped.
    pub fn set_control(&mut self, action: ControlAction, binding: Option<PadBinding>) {
        self.control_chords.remove(action.config_key());
        match binding {
            Some(binding) => {
                self.controls.insert(action.config_key().to_string(), binding);
            }
            None => {
                self.controls.remove(action.config_key());
            }
        }
    }

    /// The element a control's binding is held together with, when the player bound it to two.
    pub fn control_chord(&self, action: ControlAction) -> Option<PadBinding> {
        self.controls.get(action.config_key()).and(self.control_chords.get(action.config_key()).copied())
    }

    /// What a shortcut is bound to on the controller, which is nothing until the player binds it,
    /// and nothing ever for a shortcut a controller cannot be given ([`Shortcut::takes_a_pad`]).
    pub fn shortcut_binding(&self, shortcut: Shortcut) -> Option<PadBinding> {
        self.shortcuts.get(shortcut.config_key()).copied().filter(|_| shortcut.takes_a_pad())
    }

    /// The element a shortcut's binding is held together with, when the player bound it to two.
    pub fn shortcut_chord(&self, shortcut: Shortcut) -> Option<PadBinding> {
        self.shortcut_binding(shortcut).and(self.shortcut_chords.get(shortcut.config_key()).copied())
    }

    /// Bind a shortcut to one element, or with `None` unbind it. Either way a second element it was
    /// held with is dropped.
    pub fn set_shortcut(&mut self, shortcut: Shortcut, binding: Option<PadBinding>) {
        self.shortcut_chords.remove(shortcut.config_key());
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

    /// The element something's binding is held together with on the controller, when it is bound
    /// to two. A lane and a reverse spin never are.
    pub fn chord_of(&self, target: Bindable) -> Option<PadBinding> {
        match target {
            Bindable::Control(action) => self.control_chord(action),
            Bindable::Shortcut(shortcut) => self.shortcut_chord(shortcut),
            Bindable::Lane(_) | Bindable::ScratchReverse(_) => None,
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

    /// Bind a control or a shortcut to two elements held together. Two of one element, a turntable
    /// or a direction of one ([`PadConfig::is_on_a_turntable`]), a lane, a reverse spin and a
    /// shortcut a controller cannot be given are not bound at all.
    pub fn bind_chord(&mut self, target: Bindable, first: PadBinding, second: PadBinding) {
        if first == second || !first.is_a_key() || !second.is_a_key() || self.is_on_a_turntable(first) || self.is_on_a_turntable(second) {
            return;
        }
        let key = match target {
            Bindable::Control(action) => {
                self.set_control(action, Some(first));
                action.config_key()
            }
            Bindable::Shortcut(shortcut) if shortcut.takes_a_pad() => {
                self.set_shortcut(shortcut, Some(first));
                shortcut.config_key()
            }
            Bindable::Shortcut(_) | Bindable::Lane(_) | Bindable::ScratchReverse(_) => return,
        };
        let chords = if matches!(target, Bindable::Control(_)) { &mut self.control_chords } else { &mut self.shortcut_chords };
        chords.insert(key.to_string(), second);
    }

    /// Drop every binding the player made, leaving the tuning of the controller itself alone.
    pub fn restore_bindings(&mut self) {
        self.lanes.clear();
        self.scratch_reverse.clear();
        self.controls.clear();
        self.shortcuts.clear();
        self.control_chords.clear();
        self.shortcut_chords.clear();
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

    /// Every element this config listens to for `mode`, paired with what it drives alone, in the
    /// order the reference walks its elements: a lane's forward direction before its backward one,
    /// and an axis' plus direction before its minus. Then every pair of elements a control or a
    /// shortcut is bound to held together, with what the two drive.
    fn plan(&self, mode: Mode) -> PadPlan {
        let mut singles = Vec::new();
        for lane in 0..mode.key {
            let Some(binding) = self.lane_binding(mode, lane) else { continue };
            match binding {
                PadBinding::AnalogScratch { axis } => {
                    singles.push((PadElement::AxisDir { axis, positive: true }, PadTarget::lane(lane, ScratchDir::Forward)));
                    if mode.is_scratch(lane) {
                        singles.push((PadElement::AxisDir { axis, positive: false }, PadTarget::lane(lane, ScratchDir::Backward)));
                    }
                }
                key => singles.extend(PadElement::of(key).map(|element| (element, PadTarget::lane(lane, ScratchDir::Forward)))),
            }
        }
        for lane in (0..mode.key).filter(|&lane| mode.is_scratch(lane)) {
            singles.extend(
                self.scratch_reverse_binding(mode, lane).and_then(PadElement::of).map(|element| (element, PadTarget::lane(lane, ScratchDir::Backward))),
            );
        }
        let controls = ControlAction::ALL.into_iter().map(|action| (PadTarget::Control(action), self.control_binding(action), self.control_chord(action)));
        let shortcuts =
            Shortcut::ALL.into_iter().map(|shortcut| (PadTarget::Shortcut(shortcut), self.shortcut_binding(shortcut), self.shortcut_chord(shortcut)));
        let mut chords = Vec::new();
        for (target, first, second) in controls.chain(shortcuts) {
            let Some(first) = first.and_then(PadElement::of) else { continue };
            match second.and_then(PadElement::of) {
                Some(second) => chords.push(([first, second], target)),
                None => singles.push((first, target)),
            }
        }
        PadPlan { singles, chords }
    }
}

/// What a config listens to for one mode: every element with what it drives alone, and every pair
/// of elements with what the two held together drive.
struct PadPlan {
    singles: Vec<(PadElement, PadTarget)>,
    chords: Vec<([PadElement; 2], PadTarget)>,
}

impl PadPlan {
    /// Every element of the plan, in plan order, each once: the single ones, then the pairs.
    fn elements(&self) -> Vec<PadElement> {
        let mut order: Vec<PadElement> = Vec::new();
        let all = self.singles.iter().map(|(element, _)| *element).chain(self.chords.iter().flat_map(|(pair, _)| *pair));
        for element in all {
            if !order.contains(&element) {
                order.push(element);
            }
        }
        order
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

impl PadElement {
    /// The element a binding that is pressed like a key is read from, or `None` for a turntable,
    /// which is two elements read by its own rules.
    fn of(binding: PadBinding) -> Option<PadElement> {
        match binding {
            PadBinding::Button(code) => Some(PadElement::Button(code)),
            PadBinding::Standard(button) => Some(PadElement::Standard(button)),
            PadBinding::Axis { axis, positive } => Some(PadElement::AxisDir { axis, positive }),
            PadBinding::AnalogScratch { .. } => None,
        }
    }
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
    /// fire only on the way down, but for ESC, which goes up as well as down as the Escape key does
    /// ([`PadMark::pressed`] says which).
    fn event(self, pressed: bool) -> Option<PadEvent> {
        match self {
            PadTarget::Lane { lane, dir } => Some(PadEvent::Lane { lane, dir, press: pressed }),
            PadTarget::Control(action) => (pressed || action == ControlAction::Escape).then_some(PadEvent::Control(action)),
            PadTarget::Shortcut(shortcut) => pressed.then_some(PadEvent::Shortcut(shortcut)),
        }
    }

    /// The event that says the target is held.
    fn held_event(self) -> PadEvent {
        match self {
            PadTarget::Lane { lane, dir } => PadEvent::Lane { lane, dir, press: true },
            PadTarget::Control(action) => PadEvent::Control(action),
            PadTarget::Shortcut(shortcut) => PadEvent::Shortcut(shortcut),
        }
    }
}

/// What the controller layer knows of an event beyond the event itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadMark {
    /// Whether what made the event went down rather than up: what tells ESC going down from ESC
    /// coming up.
    pub pressed: bool,
    /// Whether it is a lane pressed by the element whose press made two elements held together.
    /// The lane is played in a run, and is the pair's alone anywhere else.
    pub made_a_chord: bool,
}

impl Default for PadMark {
    /// What an event that came with no mark is taken as: a plain press.
    fn default() -> Self {
        PadMark { pressed: true, made_a_chord: false }
    }
}

/// What a controller frame produced.
///
/// A lane carries its spin direction because rbms models a scratch as one lane spun two ways,
/// where the reference models it as two separate lane assignments. A control carries no state: it
/// fires on the press, like the key that does the same job, and so does a shortcut; ESC alone fires
/// on the release too, and its [`PadMark`] says which. What is still held afterwards is read back
/// with [`PadState::held`].
///
/// A control or a shortcut bound to two elements fires when the second of them goes down, in either
/// order. That press does not also fire what the element alone drives, but for a lane, which is
/// played.
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
    /// What every element was last handed over as. A button and a two-key axis direction are
    /// offered to it as LR2 debounces a key; a turntable direction is handed over by the
    /// reference's lock ([`PadMapper::read_turntables`]).
    gates: BTreeMap<PadElement, Lr2Debounce<i64>>,
    analog: BTreeMap<u32, AnalogScratch>,
    analog_shape: Option<(AnalogMode, u32)>,
    /// Where each turntable stood when its travel was last counted.
    turn_read: BTreeMap<u32, f32>,
    /// How many ticks the turntables have travelled since the count was last taken, a tick towards
    /// the plus end of an axis counted up ([`PadMapper::take_turn`]).
    turned: i32,
    /// Every element handed over as down that has been one of two held together since it went
    /// down. It is the pair's until it is let go, so breaking a pair by letting go of its other
    /// element does not read as a second press of what it drives alone ([`PadMapper::held`]).
    kept_for_pairs: BTreeSet<PadElement>,
}

impl PadMapper {
    pub fn new() -> PadMapper {
        PadMapper::default()
    }

    /// Resolve one frame from a snapshot alone, at `now_us`: every release the debounce kept out
    /// and has let through by then, then every button and two-key axis direction that changed, then
    /// every turntable direction, read only when its window is open — which is also what keeps a
    /// turntable's idle counter advancing at the same rate the reference advances it. The frame
    /// loop resolves change by change instead ([`PadMapper::resolve_changes_marked`]); this is what the
    /// tests of the element rules drive.
    #[cfg(test)]
    pub fn resolve(&mut self, now_us: i64, cfg: &PadConfig, mode: Mode, snapshot: &PadSnapshot) -> Vec<PadEvent> {
        self.resolve_marked(now_us, cfg, mode, snapshot).into_iter().map(|(event, _)| event).collect()
    }

    /// [`PadMapper::resolve`], with what each event carries beyond itself.
    #[cfg(test)]
    pub fn resolve_marked(&mut self, now_us: i64, cfg: &PadConfig, mode: Mode, snapshot: &PadSnapshot) -> Vec<(PadEvent, PadMark)> {
        if !cfg.enabled {
            return Vec::new();
        }
        self.retune(cfg, mode);
        self.count_turn(snapshot);
        let plan = cfg.plan(mode);
        let mut opened = Vec::new();
        self.take_opened(now_us, cfg, &plan, &mut opened);
        let mut events: Vec<(PadEvent, PadMark)> = opened.into_iter().map(|(event, _, mark)| (event, mark)).collect();
        events.extend(self.read_keys(now_us, cfg, &plan, snapshot));
        events.extend(self.read_turntables(now_us, cfg, &plan, snapshot));
        events
    }

    /// Resolve one poll of raw changes into the events they make, each with its own instant,
    /// earliest first ([`PadMapper::resolve_changes_marked`], without the marks).
    #[cfg(test)]
    pub fn resolve_changes(
        &mut self,
        changes: &[(i64, PadChange)],
        now_us: i64,
        cfg: &PadConfig,
        mode: Mode,
        snapshot: &mut PadSnapshot,
    ) -> Vec<(PadEvent, i64)> {
        self.resolve_changes_marked(changes, now_us, cfg, mode, snapshot).into_iter().map(|(event, at_us, _)| (event, at_us)).collect()
    }

    /// Resolve one poll of raw changes, each at the instant on the controller clock it happened at,
    /// into the events they make, each with its own instant and its [`PadMark`], earliest first.
    /// `snapshot` is brought up to date on the way.
    ///
    /// A button or two-key axis direction is read change by change, as LR2 reads a key: a press is
    /// taken the instant it came, a release inside the debounce window after its press is taken the
    /// instant the window ends if the element still reads up then, and not at all if it was pressed
    /// again before, and a press within one of LR2's polls of a release taken is the switch
    /// bouncing and is not taken. A press and its release inside one poll are therefore both
    /// played. A turntable is read once, at `now_us`, after every change has been taken in.
    pub fn resolve_changes_marked(
        &mut self,
        changes: &[(i64, PadChange)],
        now_us: i64,
        cfg: &PadConfig,
        mode: Mode,
        snapshot: &mut PadSnapshot,
    ) -> Vec<(PadEvent, i64, PadMark)> {
        if !cfg.enabled {
            changes.iter().for_each(|(_, change)| change.apply(snapshot));
            return Vec::new();
        }
        self.retune(cfg, mode);
        let plan = cfg.plan(mode);
        let mut events = Vec::new();
        for &(at_us, change) in changes {
            self.take_opened(at_us, cfg, &plan, &mut events);
            change.apply(snapshot);
            let keys = self.read_keys(at_us, cfg, &plan, snapshot);
            events.extend(keys.into_iter().map(|(event, mark)| (event, at_us, mark)));
        }
        self.take_opened(now_us, cfg, &plan, &mut events);
        self.count_turn(snapshot);
        let turntables = self.read_turntables(now_us, cfg, &plan, snapshot);
        events.extend(turntables.into_iter().map(|(event, mark)| (event, now_us, mark)));
        events
    }

    /// Whether `element` is a direction of an axis read as a turntable.
    fn is_turntable(&self, element: PadElement) -> bool {
        matches!(element, PadElement::AxisDir { axis, .. } if self.analog.contains_key(&axis))
    }

    /// The distinct elements of `plan` that `wanted` keeps, in plan order.
    fn elements(&self, plan: &PadPlan, wanted: impl Fn(&PadMapper, PadElement) -> bool) -> Vec<PadElement> {
        plan.elements().into_iter().filter(|element| wanted(self, *element)).collect()
    }

    /// Whether the debounce has handed `element` over as down.
    fn handed(&self, element: PadElement) -> bool {
        self.gates.get(&element).is_some_and(Lr2Debounce::handed)
    }

    /// The events of `plan` that the elements in `fired` changing to the state each holds there
    /// make, each with its mark: first every pair of elements that came to be held together or
    /// stopped being so, then every element's own targets, in plan order. The press that makes a
    /// pair fires nothing its element drives alone but a lane.
    fn events_of(&self, plan: &PadPlan, fired: &BTreeMap<PadElement, bool>) -> Vec<(PadEvent, PadMark)> {
        let before = |element: PadElement| fired.get(&element).map_or(self.handed(element), |now| !now);
        let mut made_by: Vec<PadElement> = Vec::new();
        let mut events = Vec::new();
        for ([first, second], target) in &plan.chords {
            let was = before(*first) && before(*second);
            let is = self.handed(*first) && self.handed(*second);
            if is == was {
                continue;
            }
            if is {
                made_by.extend([*first, *second].into_iter().filter(|element| fired.get(element) == Some(&true)));
            }
            events.extend(target.event(is).map(|event| (event, PadMark { pressed: is, made_a_chord: false })));
        }
        for (element, target) in &plan.singles {
            let Some(&pressed) = fired.get(element) else { continue };
            let made_a_chord = pressed && made_by.contains(element);
            if made_a_chord && !matches!(target, PadTarget::Lane { .. }) {
                continue;
            }
            events.extend(target.event(pressed).map(|event| (event, PadMark { pressed, made_a_chord })));
        }
        events
    }

    /// [`PadMapper::events_of`], after which every element of a pair that is held is kept as the
    /// pair's and every element let go is no longer ([`PadMapper::kept_for_pairs`]).
    fn take_events_of(&mut self, plan: &PadPlan, fired: &BTreeMap<PadElement, bool>) -> Vec<(PadEvent, PadMark)> {
        let events = self.events_of(plan, fired);
        let together: Vec<PadElement> =
            plan.chords.iter().filter(|(pair, _)| pair.iter().all(|element| self.handed(*element))).flat_map(|(pair, _)| *pair).collect();
        self.kept_for_pairs.extend(together);
        let gates = &self.gates;
        self.kept_for_pairs.retain(|element| gates.get(element).is_some_and(Lr2Debounce::handed));
        events
    }

    /// Offer every button and two-key axis direction of `plan` to its debounce at `at_us`, as it
    /// reads in `snapshot`. What is taken is handed back as the plan's events ([`PadMapper::events_of`]).
    fn read_keys(&mut self, at_us: i64, cfg: &PadConfig, plan: &PadPlan, snapshot: &PadSnapshot) -> Vec<(PadEvent, PadMark)> {
        let window = Duration::from_millis(u64::from(cfg.effective_debounce_ms()));
        let deadzone = cfg.effective_axis_deadzone();
        let mut fired: BTreeMap<PadElement, bool> = BTreeMap::new();
        for element in self.elements(plan, |mapper, element| !mapper.is_turntable(element)) {
            let raw = self.read(element, deadzone, snapshot);
            if self.gates.entry(element).or_default().offer(raw, at_us, window) {
                fired.insert(element, raw);
            }
        }
        self.take_events_of(plan, &fired)
    }

    /// Read every turntable direction of `plan` at `at_us`: each at most once, and only once the
    /// window has passed since its last accepted change, either way -- the reference
    /// implementation's lock (`BMControllerInputProcessor.java:136`, inequality included), whose
    /// readings the turntable algorithms count. What changed is handed back as the plan's events.
    fn read_turntables(&mut self, at_us: i64, cfg: &PadConfig, plan: &PadPlan, snapshot: &PadSnapshot) -> Vec<(PadEvent, PadMark)> {
        let window = Duration::from_millis(u64::from(cfg.effective_debounce_ms()));
        let deadzone = cfg.effective_axis_deadzone();
        let mut fired: BTreeMap<PadElement, bool> = BTreeMap::new();
        for element in self.elements(plan, |mapper, element| mapper.is_turntable(element)) {
            if self.gates.get(&element).and_then(Lr2Debounce::last_handed_at).is_some_and(|changed_us| at_us < changed_us.after(window)) {
                continue;
            }
            let raw = self.read(element, deadzone, snapshot);
            if self.gates.entry(element).or_default().hand_over(raw, at_us) {
                fired.insert(element, raw);
            }
        }
        self.take_events_of(plan, &fired)
    }

    /// Take every release of a button or two-key axis direction the debounce kept out and has let
    /// through by `until_us`, at the instant its window ended, earliest first.
    fn take_opened(&mut self, until_us: i64, cfg: &PadConfig, plan: &PadPlan, events: &mut Vec<(PadEvent, i64, PadMark)>) {
        let window = Duration::from_millis(u64::from(cfg.effective_debounce_ms()));
        let mut opened: Vec<(i64, PadElement)> = self
            .elements(plan, |mapper, element| !mapper.is_turntable(element))
            .into_iter()
            .filter_map(|element| Some((self.gates.get(&element)?.release_opened_by(until_us, window)?, element)))
            .collect();
        opened.sort_by_key(|(opens_us, _)| *opens_us);
        for (opens_us, element) in opened {
            if let Some(gate) = self.gates.get_mut(&element) {
                gate.hand_over(false, opens_us);
            }
            let released = BTreeMap::from([(element, false)]);
            events.extend(self.take_events_of(plan, &released).into_iter().map(|(event, mark)| (event, opens_us, mark)));
        }
    }

    /// Everything the controller is holding down as of the last [`PadMapper::resolve_changes_marked`], as
    /// the events that put it there, each once. It reads the accepted (debounced) state of every
    /// element the config listens to, so a lane held by two elements stays held until both let go.
    ///
    /// A control or a shortcut bound to two elements is held while both are. An element that is
    /// one of two held together that way does not hold what it drives alone, but for a lane: the
    /// pair the player bound wins over START or SELECT held with a key of the key table. Nor does
    /// one that was until it is let go ([`PadMapper::kept_for_pairs`]).
    pub fn held(&self, cfg: &PadConfig, mode: Mode) -> Vec<PadEvent> {
        if !cfg.enabled {
            return Vec::new();
        }
        let plan = cfg.plan(mode);
        let together: Vec<&([PadElement; 2], PadTarget)> = plan.chords.iter().filter(|(pair, _)| pair.iter().all(|element| self.handed(*element))).collect();
        let taken: Vec<PadElement> = together.iter().flat_map(|(pair, _)| *pair).chain(self.kept_for_pairs.iter().copied()).collect();
        let alone = plan
            .singles
            .iter()
            .filter(|(element, target)| self.handed(*element) && (matches!(target, PadTarget::Lane { .. }) || !taken.contains(element)))
            .map(|(_, target)| *target);
        let mut held = Vec::new();
        for event in alone.chain(together.into_iter().map(|(_, target)| *target)).map(PadTarget::held_event) {
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

/// How many presses and releases the key editor's capture keeps while nobody asks for them. The
/// frame loop notes every change, in the editor or not, so what is kept is bounded.
const CAPTURE_EVENTS_KEPT: usize = 16;

/// What the key editor is told of a controller while it waits for a binding: each button pressed or
/// let go by its native code, and each direction of an axis pressed once the axis passes
/// [`CAPTURE_AXIS_THRESHOLD`] that way and let go once it comes back inside it. Two elements held
/// together are told apart from one pressed and let go by the order of these
/// ([`crate::keyconfig::ChordCapture`]). A controller that goes away lets go of every button and
/// axis direction it held.
#[derive(Debug, Default)]
pub struct PadCapture {
    /// The direction each axis is counted as pressed in.
    axes: BTreeMap<u32, bool>,
    /// The buttons counted as pressed.
    buttons: BTreeSet<u32>,
    events: std::collections::VecDeque<(PadBinding, bool)>,
}

impl PadCapture {
    /// Take in one change of the controller.
    pub fn note(&mut self, change: PadChange) {
        match change {
            PadChange::Button { code, pressed, .. } => {
                if pressed {
                    self.buttons.insert(code);
                } else {
                    self.buttons.remove(&code);
                }
                self.push(PadBinding::Button(code), pressed);
            }
            PadChange::Axis { code, value } => {
                let now = (value.abs() >= CAPTURE_AXIS_THRESHOLD).then_some(value > 0.0);
                let was = self.axes.get(&code).copied();
                if now == was {
                    return;
                }
                if let Some(positive) = was {
                    self.push(PadBinding::Axis { axis: code, positive }, false);
                }
                match now {
                    Some(positive) => {
                        self.axes.insert(code, positive);
                        self.push(PadBinding::Axis { axis: code, positive }, true);
                    }
                    None => {
                        self.axes.remove(&code);
                    }
                }
            }
            PadChange::Disconnected => {
                let buttons = std::mem::take(&mut self.buttons).into_iter().map(PadBinding::Button);
                let axes = std::mem::take(&mut self.axes).into_iter().map(|(axis, positive)| PadBinding::Axis { axis, positive });
                for binding in buttons.chain(axes) {
                    self.push(binding, false);
                }
            }
        }
    }

    fn push(&mut self, binding: PadBinding, pressed: bool) {
        self.events.push_back((binding, pressed));
        while self.events.len() > CAPTURE_EVENTS_KEPT {
            self.events.pop_front();
        }
    }

    /// Every press and release noted since this was last asked, oldest first.
    pub fn take(&mut self) -> Vec<(PadBinding, bool)> {
        self.events.drain(..).collect()
    }

    /// Drop what was noted, keeping which way each axis stands.
    pub fn forget(&mut self) {
        self.events.clear();
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
    capture: PadCapture,
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
                capture: PadCapture::default(),
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

    /// Everything the controller did since the last poll, each with the instant it happened at and
    /// its [`PadMark`], earliest first, already debounced. This is the call the frame loop makes: the
    /// instants are the ones gilrs stamped each change with as it arrived, not the frame's, so a
    /// press is judged at the moment it was made however late in its frame it is read.
    pub fn poll_now(&mut self, cfg: &PadConfig, mode: Mode) -> Vec<(PadEvent, Instant, PadMark)> {
        let now_us = self.now_us();
        let changes = self.pump(cfg, now_us, SystemTime::now());
        let events = self.mapper.resolve_changes_marked(&changes, now_us, cfg, mode, &mut self.snapshot);
        events.into_iter().map(|(event, at_us, mark)| (event, self.epoch + Duration::from_micros(at_us.max(0) as u64), mark)).collect()
    }

    /// What the controller is holding down right now, as of the last poll.
    pub fn held(&self, cfg: &PadConfig, mode: Mode) -> Vec<PadEvent> {
        self.mapper.held(cfg, mode)
    }

    /// How far the turntables have turned since this was last asked ([`PadMapper::take_turn`]).
    pub fn take_turn(&mut self) -> Option<i32> {
        self.mapper.take_turn()
    }

    /// What the player pressed and let go since this was last asked, for the key editor's "press
    /// something" mode ([`PadCapture`]). Taking it clears it.
    pub fn take_capture(&mut self, cfg: &PadConfig) -> Vec<(PadBinding, bool)> {
        self.pump_into_snapshot(cfg);
        self.capture.take()
    }

    /// Drop whatever was pressed before capture mode was entered.
    pub fn drain(&mut self, cfg: &PadConfig) {
        self.pump_into_snapshot(cfg);
        self.capture.forget();
    }

    /// Take in what the controller did without playing any of it, for the key editor.
    fn pump_into_snapshot(&mut self, cfg: &PadConfig) {
        let changes = self.pump(cfg, self.now_us(), SystemTime::now());
        changes.into_iter().for_each(|(_, change)| change.apply(&mut self.snapshot));
    }

    /// Every change gilrs has queued since the last pump, each at the instant it was stamped with
    /// on the controller clock, noting what was pressed and let go for the key editor on the way.
    fn pump(&mut self, cfg: &PadConfig, now_us: i64, wall_now: SystemTime) -> Vec<(i64, PadChange)> {
        let mut changes = Vec::new();
        while let Some(event) = self.gilrs.next_event() {
            let at_us = stamped_instant_us(event.time, wall_now, now_us, self.last_change_us);
            if matches!(event.event, gilrs::EventType::Disconnected) {
                self.capture.note(PadChange::Disconnected);
                changes.push((at_us, PadChange::Disconnected));
                continue;
            }
            if !self.accepts(cfg, event.id) {
                continue;
            }
            let change = match event.event {
                gilrs::EventType::ButtonPressed(button, code) => {
                    PadChange::Button { code: code.into_u32(), standard: StandardButton::from_gilrs(button), pressed: true }
                }
                gilrs::EventType::ButtonReleased(button, code) => {
                    PadChange::Button { code: code.into_u32(), standard: StandardButton::from_gilrs(button), pressed: false }
                }
                gilrs::EventType::AxisChanged(_, value, code) => PadChange::Axis { code: code.into_u32(), value },
                _ => continue,
            };
            self.capture.note(change);
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
