//! The three option panels of a browser a skin draws: which one the held keys call up, the timers
//! a skin slides it in and out on, and what each key does while it is up.
//!
//! The reference has no panel that is opened and closed: every frame it reads START, SELECT and the
//! detail key, and the panel they call up is up for exactly as long as they are held
//! (`MusicSelectInputProcessor.input`, `MusicSelector.setPanelState`). START alone is the play
//! options, SELECT alone the assist options, and both together -- or the detail key with neither --
//! the detail options ([`super::keys::Panel::held`]).
//!
//! A panel has a pair of timers. The first is started when it comes up and the second when it goes,
//! each switching the other off, so a skin animates the arrival against the one and the departure
//! against the other; going from one panel straight to another starts the departure of the first
//! and the arrival of the second on the same frame. Which panel is up is also what the options 21
//! to 23 answer ([`PanelState::number`]).
//!
//! The keys of a panel are the reference's table ([`super::keys`]) and do what its three branches
//! do ([`press_act`]): the first panel steps the random options, the gauge, the double option and
//! the fixed tempo by their events and scrolls the target with the turntable and the mouse wheel;
//! the second flips the assists; the third steps the background animation, the gauge auto shift and
//! the judge timing, and moves the note display time while its two keys are held. Of the
//! reference's seven assists only the constant speed and the legacy notes are settings here, and
//! the custom judge window, the judge area, the processed note mark, the BPM guide and no mines are
//! taken and do nothing. The double option is stepped, though no run applies it.
//!
//! That is the table of a skin that does not say it knows this player's own ids. To one that does,
//! the keys the reference gives to what this player has not got step what it has instead: on the
//! first panel the gauge before and the fixed tempo before, on the second SCRATCH AUTO, KEEP GREEN
//! NUMBER and the three covers switched on or off, and on the third the judge algorithm with the
//! turntable, each by its event.
//!
//! The note display time is the travel time a held green number is held at
//! ([`super::events::step_duration`]). Its keys repeat as the reference's do: a step when a key
//! first goes down, another once the long wait is over and one at every short wait after, ten times
//! as far once more than fifty of them have gone by (`MusicSelectInputProcessor.java:259-281`).
//!
//! The sounds are the reference's: a panel opening sounds once when a panel key goes down with none
//! held before, and a panel closing once when the last of them comes up, so going from one panel to
//! another is silent.

use rbms_config::{AdjustOutcome, SettingId, adjust};
use rbms_render::skin_render::frame::{SCROLL_DURATION_HIGH_MS, SCROLL_DURATION_LOW_MS};
use rbms_skin::timer::{TimerId, timer_id};

use super::SelectState;
use super::events::SettingEvent;
use super::keys::{KeyLayout, Panel, SelectKey};
use crate::{AppShared, KeyCode, SystemSound};

/// The step an event takes towards the next value of its setting, and towards the one before
/// (`executeEvent(type, 1)` and `executeEvent(type, -1)`; an event run with no argument has zero,
/// which is the next).
const NEXT: i32 = 1;
const PREVIOUS: i32 = -1;
const NO_ARGUMENT: i32 = 0;

/// The keys of the keyboard that scroll the first panel's target beside the turntable
/// (`ControlKeys.DOWN` with `TARGET_UP`, `ControlKeys.UP` with `TARGET_DOWN`).
const TARGET_UP_KEY: KeyCode = KeyCode::ArrowDown;
const TARGET_DOWN_KEY: KeyCode = KeyCode::ArrowUp;

/// The timers of each panel: the one started when it comes up and the one started when it goes.
const PANEL_TIMERS: [(Panel, TimerId, TimerId); 3] = [
    (Panel::Play, timer_id::PANEL1_ON, timer_id::PANEL1_OFF),
    (Panel::Assist, timer_id::PANEL2_ON, timer_id::PANEL2_OFF),
    (Panel::Detail, timer_id::PANEL3_ON, timer_id::PANEL3_OFF),
];

/// The pair of timers of `panel`, or `None` for no panel.
fn timers_of(panel: Panel) -> Option<(TimerId, TimerId)> {
    PANEL_TIMERS.iter().find(|(of, ..)| *of == panel).map(|(_, on, off)| (*on, *off))
}

/// One of the second panel's switches the reference keeps, which it flips in the player's settings
/// directly rather than by an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AssistSwitch {
    /// A judge window of the player's own.
    CustomJudge,
    /// A constant scroll speed (`scrollMode` between 0 and 1).
    Constant,
    /// Long notes drawn as plain notes (`longnoteMode` between 0 and 1).
    LegacyNote,
}

impl AssistSwitch {
    /// The row of the settings screen the switch is, or `None` for the custom judge window, which is
    /// not a switch here: a run is one when any of its judge widths has been moved.
    const fn row(self) -> Option<SettingId> {
        match self {
            AssistSwitch::Constant => Some(SettingId::SpeedFix),
            AssistSwitch::LegacyNote => Some(SettingId::LegacyNote),
            AssistSwitch::CustomJudge => None,
        }
    }
}

/// What a key of an open panel asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelAct {
    /// Run the event of a setting with this first argument.
    Setting { event: SettingEvent, arg1: i32 },
    /// Flip an assist.
    Assist(AssistSwitch),
    /// The sound of the target moving one place.
    Scratch,
    /// Run the event of the note display time with these two arguments.
    Duration { arg1: i32, arg2: i32 },
}

/// What a press of the key carrying `role` asks of the panel it belongs to, or `None` for a role no
/// panel takes a press of (`MusicSelectInputProcessor.input`, the three panel branches) and for the
/// reference's assists this player has no setting for: a key carrying one of those is the panel's
/// and asks for nothing.
pub(super) fn press_act(role: SelectKey) -> Option<PanelAct> {
    let setting = |event: SettingEvent, arg1: i32| Some(PanelAct::Setting { event, arg1 });
    match role {
        SelectKey::Option1Down | SelectKey::Option2Down => setting(SettingEvent::Random, NEXT),
        SelectKey::Option1Up | SelectKey::Option2Up => setting(SettingEvent::Random, PREVIOUS),
        SelectKey::GaugeDown => setting(SettingEvent::Gauge, NEXT),
        SelectKey::GaugeUp => setting(SettingEvent::Gauge, PREVIOUS),
        SelectKey::OptionDpDown => setting(SettingEvent::DoubleOption, NEXT),
        SelectKey::OptionDpUp => setting(SettingEvent::DoubleOption, PREVIOUS),
        SelectKey::HsFixDown => setting(SettingEvent::FixHiSpeed, NEXT),
        SelectKey::HsFixUp => setting(SettingEvent::FixHiSpeed, PREVIOUS),
        SelectKey::JudgeWindowUp => Some(PanelAct::Assist(AssistSwitch::CustomJudge)),
        SelectKey::Constant => Some(PanelAct::Assist(AssistSwitch::Constant)),
        SelectKey::LegacyNote => Some(PanelAct::Assist(AssistSwitch::LegacyNote)),
        SelectKey::ScratchAuto => setting(SettingEvent::ScratchAuto, NEXT),
        SelectKey::KeepGreenNumber => setting(SettingEvent::KeepGreenNumber, NEXT),
        SelectKey::LaneCover => setting(SettingEvent::LaneCover, NEXT),
        SelectKey::Lift => setting(SettingEvent::Lift, NEXT),
        SelectKey::Hidden => setting(SettingEvent::Hidden, NEXT),
        SelectKey::BgaDown => setting(SettingEvent::Bga, NO_ARGUMENT),
        SelectKey::GaugeAutoShiftDown => setting(SettingEvent::GaugeAutoShift, NO_ARGUMENT),
        SelectKey::NotesDisplayTimingDown => setting(SettingEvent::JudgeTiming, PREVIOUS),
        SelectKey::NotesDisplayTimingUp => setting(SettingEvent::JudgeTiming, NO_ARGUMENT),
        SelectKey::NotesDisplayTimingAutoAdjust => setting(SettingEvent::JudgeTimingAuto, NO_ARGUMENT),
        SelectKey::JudgeAlgorithmUp => setting(SettingEvent::JudgeAlgorithm, PREVIOUS),
        SelectKey::JudgeAlgorithmDown => setting(SettingEvent::JudgeAlgorithm, NEXT),
        _ => None,
    }
}

/// What a movement of the first panel's target asks for: for every place towards the top of the
/// list the target's event with the argument for the one before and its sound, and for every place
/// the other way the event with the argument for the next (`mov > 0` and `mov < 0`).
pub(super) fn target_acts(moved: i32) -> Vec<PanelAct> {
    let arg1 = if moved > 0 { PREVIOUS } else { NEXT };
    (0..moved.unsigned_abs()).flat_map(|_| [PanelAct::Setting { event: SettingEvent::Target, arg1 }, PanelAct::Scratch]).collect()
}

/// How many repeats of a held note display time key go by before a repeat moves it further
/// (`countChangeDuration > 50`), and how far such a repeat moves it, in milliseconds.
const DURATION_FAST_AFTER_REPEATS: u32 = 50;
const DURATION_FAST_STEP_MS: i32 = 10;

/// The second argument of a note display time event that names no step of its own, which moves it
/// a millisecond.
const DURATION_PLAIN_STEP: i32 = 0;

/// Which of the third panel's two note display time keys is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DurationHold {
    /// `DURATION_DOWN`, which wins when both are down.
    Down,
    /// `DURATION_UP`.
    Up,
    Neither,
}

/// Which way the keys that scroll the target are held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TargetHold {
    /// `TARGET_UP`, which wins when both are down.
    Up,
    /// `TARGET_DOWN`.
    Down,
    Neither,
}

/// The option panels as the browser last left them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanelState {
    /// The panel that is up (`MusicSelector.panelstate`).
    open: Panel,
    /// Whether a panel key has been held since the sound of a panel opening, and whether every
    /// panel key has been up since (`isOptionKeyPressed`, `isOptionKeyReleased`).
    key_pressed: bool,
    key_released: bool,
    /// The wall clock's millisecond a held target key next moves the target at, or zero when none
    /// has been held for a while (`duration`).
    target_repeat_at_ms: i64,
    /// The wall clock's millisecond a held note display time key next moves it at, or zero while
    /// neither is held (`timeChangeDuration`), and how many times one has repeated since it went
    /// down (`countChangeDuration`).
    duration_repeat_at_ms: i64,
    duration_repeats: u32,
}

impl Default for PanelState {
    fn default() -> PanelState {
        PanelState { open: Panel::Closed, key_pressed: false, key_released: false, target_repeat_at_ms: 0, duration_repeat_at_ms: 0, duration_repeats: 0 }
    }
}

impl PanelState {
    /// Whether a panel is up.
    pub(super) fn is_open(&self) -> bool {
        self.open != Panel::Closed
    }

    /// The panel that is up as the options 21 to 23 number it: 1 to 3, and 0 for none.
    pub(super) fn number(&self) -> u8 {
        match self.open {
            Panel::Closed => 0,
            Panel::Play => 1,
            Panel::Assist => 2,
            Panel::Detail => 3,
        }
    }

    /// Follow the panel keys for the sounds of a panel opening and closing, and answer the one this
    /// frame makes. Opening sounds when a panel key is held and every one of them has been up since
    /// it last sounded; closing sounds when they are all up after that.
    fn option_key_sound(&mut self, held: Panel) -> Option<SystemSound> {
        if held == Panel::Closed {
            self.key_released = true;
            return std::mem::take(&mut self.key_pressed).then_some(SystemSound::OptionClose);
        }
        if !std::mem::take(&mut self.key_released) {
            return None;
        }
        self.key_pressed = true;
        Some(SystemSound::OptionOpen)
    }

    /// How far the target moves this frame, towards the top of its list when positive: by the
    /// wheel, or one place when a key is first held and one more every time its repeat comes due
    /// (`MusicSelectInputProcessor.input`, the START branch).
    ///
    /// A held key replaces what the wheel turned on the frame it moves. The repeat is forgotten only
    /// once it has come due with no key held, so a key tapped again before then moves nothing: that
    /// is the reference's own.
    fn target_moves(&mut self, wheel: i32, hold: TargetHold, now_ms: i64) -> i32 {
        let step = match hold {
            TargetHold::Up => 1,
            TargetHold::Down => -1,
            TargetHold::Neither => {
                if now_ms > self.target_repeat_at_ms {
                    self.target_repeat_at_ms = 0;
                }
                return wheel;
            }
        };
        let mut moved = wheel;
        if self.target_repeat_at_ms == 0 {
            moved = step;
            self.target_repeat_at_ms = now_ms + i64::from(SCROLL_DURATION_LOW_MS);
        }
        if now_ms > self.target_repeat_at_ms {
            self.target_repeat_at_ms = now_ms + i64::from(SCROLL_DURATION_HIGH_MS);
            moved = step;
        }
        moved
    }
}

impl PanelState {
    /// What the held note display time keys ask for this frame: the event's two arguments, or
    /// `None` on a frame that moves nothing (`MusicSelectInputProcessor.java:259-281`).
    ///
    /// A key that has just gone down moves the time at once and waits the long wait; held past it,
    /// it moves the time again at every short wait, and by ten once fifty repeats have gone by. With
    /// neither key held the wait and the count are forgotten.
    fn duration_moves(&mut self, hold: DurationHold, now_ms: i64) -> Option<(i32, i32)> {
        let arg1 = match hold {
            DurationHold::Down => PREVIOUS,
            DurationHold::Up => NEXT,
            DurationHold::Neither => {
                self.duration_repeat_at_ms = 0;
                self.duration_repeats = 0;
                return None;
            }
        };
        if self.duration_repeat_at_ms == 0 {
            self.duration_repeat_at_ms = now_ms + i64::from(SCROLL_DURATION_LOW_MS);
            return Some((arg1, DURATION_PLAIN_STEP));
        }
        if now_ms <= self.duration_repeat_at_ms {
            return None;
        }
        self.duration_repeats = self.duration_repeats.saturating_add(1);
        self.duration_repeat_at_ms = now_ms + i64::from(SCROLL_DURATION_HIGH_MS);
        Some((arg1, if self.duration_repeats > DURATION_FAST_AFTER_REPEATS { DURATION_FAST_STEP_MS } else { DURATION_PLAIN_STEP }))
    }
}

impl SelectState {
    /// Put up the panel the held keys call up, or take down the one that is up
    /// (`MusicSelector.setPanelState`), with the sound of a panel opening or closing when the keys
    /// make one.
    ///
    /// A panel that goes starts its departure timer and switches its arrival timer off, and one that
    /// comes starts its arrival timer and switches its departure timer off. Nothing is switched for
    /// a panel that stays as it is.
    ///
    /// A turntable direction already down when the panel changes is not a press on the panel that
    /// follows ([`super::keys::SelectKeys::forget_turntable_changes`]).
    pub(super) fn show_panel(&mut self, shared: &mut AppShared, held: Panel) {
        if let Some(sound) = self.panel.option_key_sound(held) {
            shared.play_system_sound(sound);
        }
        if self.panel.open == held {
            return;
        }
        self.keys.forget_turntable_changes();
        let now_us = shared.skin_now_us();
        if let Some((on, off)) = timers_of(self.panel.open) {
            shared.skin_timers.set_on(off, now_us);
            shared.skin_timers.off(on);
        }
        if let Some((on, off)) = timers_of(held) {
            shared.skin_timers.set_on(on, now_us);
            shared.skin_timers.off(off);
        }
        self.panel.open = held;
    }

    /// One frame of the keys of the panel that is up: the presses the panel takes, in the order the
    /// reference asks about them, on the first panel the scroll of the target, and on the third the
    /// note display time its two keys move while they are held.
    pub(super) fn panel_keys(&mut self, shared: &mut AppShared, layout: KeyLayout, panel: Panel, now_ms: i64) {
        let mut acts: Vec<PanelAct> = self.keys.panel_presses(layout, panel).into_iter().filter_map(press_act).collect();
        if panel == Panel::Play {
            let held = &shared.keyconfig.held;
            let hold = if self.keys.pressed(layout, SelectKey::TargetUp, false) || held.is_down(TARGET_UP_KEY) {
                TargetHold::Up
            } else if self.keys.pressed(layout, SelectKey::TargetDown, false) || held.is_down(TARGET_DOWN_KEY) {
                TargetHold::Down
            } else {
                TargetHold::Neither
            };
            let wheel = self.take_wheel();
            acts.extend(target_acts(self.panel.target_moves(wheel, hold, now_ms)));
        }
        if panel == Panel::Detail {
            let hold = if self.keys.pressed(layout, SelectKey::DurationDown, false) {
                DurationHold::Down
            } else if self.keys.pressed(layout, SelectKey::DurationUp, false) {
                DurationHold::Up
            } else {
                DurationHold::Neither
            };
            acts.extend(self.panel.duration_moves(hold, now_ms).map(|(arg1, arg2)| PanelAct::Duration { arg1, arg2 }));
        }
        for act in acts {
            self.run_panel_act(shared, act);
        }
    }

    /// Carry out one thing a key of a panel asked for.
    fn run_panel_act(&mut self, shared: &mut AppShared, act: PanelAct) {
        match act {
            PanelAct::Setting { event, arg1 } => self.run_setting_event(shared, event, arg1),
            PanelAct::Assist(switch) => {
                let flipped = switch.row().is_some_and(|row| adjust(&mut shared.config, row, NEXT) == AdjustOutcome::Changed);
                if flipped {
                    self.options_dirty = true;
                    shared.play_system_sound(SystemSound::OptionChange);
                }
            }
            PanelAct::Scratch => shared.play_system_sound(SystemSound::Scratch),
            PanelAct::Duration { arg1, arg2 } => self.run_duration_event(shared, arg1, arg2),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first panel's rows of the key table: every role its START branch asks about, the event it
    /// runs and the argument it runs it with.
    #[test]
    fn the_first_panels_keys_run_the_events_the_reference_runs() {
        let rows = [
            (SelectKey::Option1Down, SettingEvent::Random, NEXT),
            (SelectKey::Option1Up, SettingEvent::Random, PREVIOUS),
            (SelectKey::GaugeDown, SettingEvent::Gauge, NEXT),
            (SelectKey::GaugeUp, SettingEvent::Gauge, PREVIOUS),
            (SelectKey::Option2Down, SettingEvent::Random, NEXT),
            (SelectKey::HsFixDown, SettingEvent::FixHiSpeed, NEXT),
            (SelectKey::HsFixUp, SettingEvent::FixHiSpeed, PREVIOUS),
        ];
        for (role, event, arg1) in rows {
            assert_eq!(press_act(role), Some(PanelAct::Setting { event, arg1 }), "{role:?}");
        }
    }

    /// The first panel's roles only the reference's table puts on keys: the double option after and
    /// before, and the second side's random option back, which this player deals by the one option.
    #[test]
    fn the_reference_roles_of_the_first_panel_step_the_double_option_and_the_random_back() {
        let rows = [
            (SelectKey::OptionDpDown, SettingEvent::DoubleOption, NEXT),
            (SelectKey::OptionDpUp, SettingEvent::DoubleOption, PREVIOUS),
            (SelectKey::Option2Up, SettingEvent::Random, PREVIOUS),
        ];
        for (role, event, arg1) in rows {
            assert_eq!(press_act(role), Some(PanelAct::Setting { event, arg1 }), "{role:?}");
        }
    }

    /// The reference's assists this player has no setting for are taken by the panel and ask for
    /// nothing, as the judge area, the processed note mark, the BPM guide and no mines did before
    /// their keys were given to options this player has.
    #[test]
    fn the_reference_assists_this_player_has_not_got_ask_for_nothing() {
        for role in [SelectKey::JudgeArea, SelectKey::MarkNote, SelectKey::BpmGuide, SelectKey::NoMine] {
            assert_eq!(press_act(role), None, "{role:?}");
        }
    }

    /// The second panel's rows: the three keys the reference's assists keep flip them, and the rest
    /// switch this player's own options by their events.
    #[test]
    fn the_second_panels_keys_flip_the_assists() {
        let rows = [
            (SelectKey::JudgeWindowUp, AssistSwitch::CustomJudge),
            (SelectKey::Constant, AssistSwitch::Constant),
            (SelectKey::LegacyNote, AssistSwitch::LegacyNote),
        ];
        for (role, switch) in rows {
            assert_eq!(press_act(role), Some(PanelAct::Assist(switch)), "{role:?}");
        }
        let settings = rows.map(|(_, switch)| switch.row());
        assert_eq!(settings, [None, Some(SettingId::SpeedFix), Some(SettingId::LegacyNote)]);

        let switches = [
            (SelectKey::ScratchAuto, SettingEvent::ScratchAuto),
            (SelectKey::KeepGreenNumber, SettingEvent::KeepGreenNumber),
            (SelectKey::LaneCover, SettingEvent::LaneCover),
            (SelectKey::Lift, SettingEvent::Lift),
            (SelectKey::Hidden, SettingEvent::Hidden),
        ];
        for (role, event) in switches {
            assert_eq!(press_act(role), Some(PanelAct::Setting { event, arg1: NEXT }), "{role:?}");
        }
    }

    /// The third panel's rows: the two events it runs with no argument step to the next value, the
    /// judge timing goes down with the one key and up with the other, and the turntable steps the
    /// judge algorithm as the first panel's steps the target.
    #[test]
    fn the_third_panels_keys_run_the_events_the_reference_runs() {
        let rows = [
            (SelectKey::BgaDown, SettingEvent::Bga, NO_ARGUMENT),
            (SelectKey::GaugeAutoShiftDown, SettingEvent::GaugeAutoShift, NO_ARGUMENT),
            (SelectKey::NotesDisplayTimingDown, SettingEvent::JudgeTiming, PREVIOUS),
            (SelectKey::NotesDisplayTimingUp, SettingEvent::JudgeTiming, NO_ARGUMENT),
            (SelectKey::NotesDisplayTimingAutoAdjust, SettingEvent::JudgeTimingAuto, NO_ARGUMENT),
            (SelectKey::JudgeAlgorithmUp, SettingEvent::JudgeAlgorithm, PREVIOUS),
            (SelectKey::JudgeAlgorithmDown, SettingEvent::JudgeAlgorithm, NEXT),
        ];
        for (role, event, arg1) in rows {
            assert_eq!(press_act(role), Some(PanelAct::Setting { event, arg1 }), "{role:?}");
        }
    }

    /// The roles of the list, the two that scroll the target and the two the third panel reads as
    /// held are no press of a panel.
    #[test]
    fn a_role_no_panel_takes_a_press_of_asks_for_nothing() {
        let others = [
            SelectKey::Play,
            SelectKey::Auto,
            SelectKey::Replay,
            SelectKey::Up,
            SelectKey::Down,
            SelectKey::FolderOpen,
            SelectKey::FolderClose,
            SelectKey::Practice,
            SelectKey::TargetUp,
            SelectKey::TargetDown,
            SelectKey::DurationUp,
            SelectKey::DurationDown,
        ];
        for role in others {
            assert_eq!(press_act(role), None, "{role:?}");
        }
    }

    /// Towards the top of the list is the target before, towards the bottom the one after, and
    /// every place sounds.
    #[test]
    fn the_target_moves_a_place_for_every_step_with_a_sound_each() {
        let step = |arg1: i32| PanelAct::Setting { event: SettingEvent::Target, arg1 };
        assert!(target_acts(0).is_empty());
        assert_eq!(target_acts(2), [step(PREVIOUS), PanelAct::Scratch, step(PREVIOUS), PanelAct::Scratch]);
        assert_eq!(target_acts(-1), [step(NEXT), PanelAct::Scratch]);
    }

    /// One sound when a panel key goes down, none while it stays down or another joins it, and one
    /// when the last comes up. Keys held before any was ever up make no sound.
    #[test]
    fn a_panel_sounds_once_as_its_keys_go_down_and_once_as_they_come_up() {
        let mut panel = PanelState::default();
        assert_eq!(panel.option_key_sound(Panel::Play), None, "held from before the browser first saw the keys up");
        assert_eq!(panel.option_key_sound(Panel::Closed), None);
        assert_eq!(panel.option_key_sound(Panel::Play), Some(SystemSound::OptionOpen));
        assert_eq!(panel.option_key_sound(Panel::Play), None);
        assert_eq!(panel.option_key_sound(Panel::Detail), None, "from one panel to another");
        assert_eq!(panel.option_key_sound(Panel::Assist), None);
        assert_eq!(panel.option_key_sound(Panel::Closed), Some(SystemSound::OptionClose));
        assert_eq!(panel.option_key_sound(Panel::Closed), None);
        assert_eq!(panel.option_key_sound(Panel::Assist), Some(SystemSound::OptionOpen));
    }

    /// A held key moves the target at once, again after the long wait and then at every short one.
    #[test]
    fn a_held_target_key_moves_at_once_then_repeats() {
        let low = i64::from(SCROLL_DURATION_LOW_MS);
        let high = i64::from(SCROLL_DURATION_HIGH_MS);
        let start = 1_000_000;
        let mut panel = PanelState::default();
        assert_eq!(panel.target_moves(0, TargetHold::Up, start), 1);
        assert_eq!(panel.target_moves(0, TargetHold::Up, start + low), 0, "the wait has to be over, not just reached");
        assert_eq!(panel.target_moves(0, TargetHold::Up, start + low + 1), 1);
        assert_eq!(panel.target_moves(0, TargetHold::Up, start + low + high), 0);
        assert_eq!(panel.target_moves(0, TargetHold::Up, start + low + high + 2), 1);

        let mut panel = PanelState::default();
        assert_eq!(panel.target_moves(0, TargetHold::Down, start), -1);
        assert_eq!(panel.target_moves(0, TargetHold::Down, start + low + 1), -1);
    }

    /// The wheel moves the target by its notches, a held key replaces them on the frame it moves,
    /// and a key tapped again before its first wait is over moves nothing.
    #[test]
    fn the_wheel_scrolls_the_target_and_a_key_tapped_twice_quickly_moves_once() {
        let start = 1_000_000;
        let mut panel = PanelState::default();
        assert_eq!(panel.target_moves(3, TargetHold::Neither, start), 3);
        assert_eq!(panel.target_moves(-2, TargetHold::Neither, start), -2);
        assert_eq!(panel.target_moves(3, TargetHold::Down, start), -1, "the key's step replaced the wheel's");

        assert_eq!(panel.target_moves(0, TargetHold::Neither, start + 10), 0);
        assert_eq!(panel.target_moves(0, TargetHold::Down, start + 20), 0, "the first wait was not over");
        assert_eq!(panel.target_moves(2, TargetHold::Down, start + 30), 2, "the wheel still turns it meanwhile");
        let later = start + i64::from(SCROLL_DURATION_LOW_MS) + 100;
        assert_eq!(panel.target_moves(0, TargetHold::Neither, later), 0);
        assert_eq!(panel.target_moves(0, TargetHold::Down, later + 10), -1, "the wait was forgotten once it came due with no key held");
    }

    /// A held note display time key moves it at once, again after the long wait, then at every
    /// short one, and ten times as far after fifty repeats. Letting go forgets the wait and the
    /// count, so the next press moves it at once and by one.
    #[test]
    fn a_held_note_display_time_key_repeats_and_then_moves_ten_times_as_far() {
        let low = i64::from(SCROLL_DURATION_LOW_MS);
        let high = i64::from(SCROLL_DURATION_HIGH_MS);
        let mut now = 1_000_000;
        let mut panel = PanelState::default();
        assert_eq!(panel.duration_moves(DurationHold::Up, now), Some((NEXT, DURATION_PLAIN_STEP)));
        assert_eq!(panel.duration_moves(DurationHold::Up, now + low), None, "the wait has to be over, not just reached");
        now += low + 1;
        assert_eq!(panel.duration_moves(DurationHold::Up, now), Some((NEXT, DURATION_PLAIN_STEP)));
        assert_eq!(panel.duration_moves(DurationHold::Up, now + high), None);
        for repeat in 2..=DURATION_FAST_AFTER_REPEATS {
            now += high + 1;
            assert_eq!(panel.duration_moves(DurationHold::Up, now), Some((NEXT, DURATION_PLAIN_STEP)), "repeat {repeat}");
        }
        now += high + 1;
        assert_eq!(panel.duration_moves(DurationHold::Up, now), Some((NEXT, DURATION_FAST_STEP_MS)), "the fifty-first repeat");

        assert_eq!(panel.duration_moves(DurationHold::Neither, now + 1), None);
        assert_eq!(panel.duration_moves(DurationHold::Down, now + 2), Some((PREVIOUS, DURATION_PLAIN_STEP)), "let go and pressed again");
    }

    #[test]
    fn the_panels_are_numbered_as_the_options_number_them() {
        let numbered = [Panel::Closed, Panel::Play, Panel::Assist, Panel::Detail].map(|open| PanelState { open, ..PanelState::default() }.number());
        assert_eq!(numbered, [0, 1, 2, 3]);
        assert_eq!(timers_of(Panel::Closed), None);
    }
}
