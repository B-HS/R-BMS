//! What a controller frame produces: the debounce window, the two-key axis, and both turntable
//! algorithms driven through every transition of their state tables.

use super::*;

const AXIS: u32 = 3;
const BUTTON: u32 = 589_825;
const SCRATCH_LANE: usize = 7;
const MS: i64 = 1_000;

/// The bindings of `mode` that two things read at the same moment both have, which is what the key
/// editor draws in red.
fn collisions(cfg: &PadConfig, mode: Mode) -> std::collections::HashSet<PadBinding> {
    let bound: Vec<(Bindable, Option<PadBinding>)> = Bindable::all(mode).into_iter().map(|target| (target, cfg.binding_of(mode, target))).collect();
    bound
        .iter()
        .filter_map(|(target, binding)| {
            binding.filter(|_| bound.iter().any(|(other, theirs)| other != target && theirs == binding && target.read_with(*other)))
        })
        .collect()
}

fn seven_key() -> Mode {
    Mode::BEAT_7K
}

fn cfg_with_lane(lane: usize, binding: PadBinding) -> PadConfig {
    let mut cfg = PadConfig { debounce_ms: 0, ..PadConfig::default() };
    cfg.set_lane(seven_key(), lane, Some(binding));
    cfg
}

fn snapshot_button(pressed: bool) -> PadSnapshot {
    let mut snapshot = PadSnapshot::default();
    snapshot.set_button(BUTTON, pressed);
    snapshot
}

fn snapshot_axis(value: f32) -> PadSnapshot {
    let mut snapshot = PadSnapshot::default();
    snapshot.set_axis(AXIS, value);
    snapshot
}

/// Hold the axis still for `frames` further calls of one direction, reporting what each returned.
fn hold(machine: &mut AnalogScratch, value: f32, plus: bool, frames: usize) -> Vec<bool> {
    (0..frames).map(|_| machine.input(value, plus)).collect()
}

#[test]
fn a_movement_inside_the_axis_range_is_counted_in_ticks() {
    assert_eq!(compute_analog_diff(0.0, 0.0), 0, "a reading that did not move is no ticks");
    assert_eq!(compute_analog_diff(0.0, TICK_MAX_SIZE), 1);
    assert_eq!(compute_analog_diff(0.0, -TICK_MAX_SIZE), -1);
    assert_eq!(compute_analog_diff(0.0, TICK_MAX_SIZE * 2.0), 2);
    assert_eq!(compute_analog_diff(0.0, -TICK_MAX_SIZE * 2.0), -2);
}

#[test]
fn a_part_tick_counts_as_a_whole_one_away_from_zero() {
    assert_eq!(compute_analog_diff(0.0, TICK_MAX_SIZE / 2.0), 1, "a rightward part tick rounds up");
    assert_eq!(compute_analog_diff(0.0, -TICK_MAX_SIZE / 2.0), -1, "a leftward part tick rounds down");
}

#[test]
fn a_movement_across_the_seam_is_the_short_way_round() {
    assert_eq!(compute_analog_diff(0.995, -0.995), 2, "wrapping off the right edge is a small rightward spin");
    assert_eq!(compute_analog_diff(-0.995, 0.995), -2, "wrapping off the left edge is a small leftward spin");
}

#[test]
fn the_first_reading_only_records_the_position() {
    for mode in [AnalogMode::V1, AnalogMode::V2] {
        let mut machine = AnalogScratch::new(mode, 4);
        assert!(!machine.input(0.5, true), "{mode:?} holds nothing on the reading that starts it");
        assert!(!machine.is_active(), "{mode:?}");
    }
}

#[test]
fn version_one_holds_the_lane_on_the_first_movement() {
    let mut machine = AnalogScratch::new(AnalogMode::V1, 4);
    machine.input(0.0, true);
    assert!(machine.input(TICK_MAX_SIZE, true), "one movement is enough for version 1");
    assert!(machine.is_right());
}

#[test]
fn version_one_lets_go_after_the_threshold_of_idle_calls() {
    let threshold = 4;
    let mut machine = AnalogScratch::new(AnalogMode::V1, threshold);
    machine.input(0.0, true);
    assert!(machine.input(TICK_MAX_SIZE, true));

    let held = hold(&mut machine, TICK_MAX_SIZE, true, threshold as usize + 1);
    let (kept, released) = held.split_at(threshold as usize);
    assert!(kept.iter().all(|held| *held), "the spin survives exactly {threshold} idle calls: {held:?}");
    assert_eq!(released, [false], "and is let go on the call after that");
}

#[test]
fn version_one_follows_a_reversal_without_letting_go() {
    let mut machine = AnalogScratch::new(AnalogMode::V1, 8);
    machine.input(0.0, true);
    assert!(machine.input(TICK_MAX_SIZE, true), "spinning right holds the rightward lane");

    assert!(!machine.input(0.0, true), "the rightward lane is let go");
    assert!(machine.is_active(), "but the spin itself never stopped");
    assert!(!machine.is_right());
    assert!(machine.input(0.0, false), "and the leftward lane is now held");
}

#[test]
fn version_two_wants_two_ticks_before_it_holds_the_lane() {
    let mut machine = AnalogScratch::new(AnalogMode::V2, 8);
    machine.input(0.0, true);
    assert!(!machine.input(TICK_MAX_SIZE, true), "one tick of travel is not yet a spin");
    assert!(machine.input(TICK_MAX_SIZE * 2.0, true), "the second tick makes it one");
}

#[test]
fn version_two_holds_at_once_when_one_movement_is_two_ticks() {
    let mut machine = AnalogScratch::new(AnalogMode::V2, 8);
    machine.input(0.0, true);
    assert!(machine.input(TICK_MAX_SIZE * 2.0, true));
}

#[test]
fn version_two_lets_go_after_twice_the_threshold() {
    let threshold = 3;
    let mut machine = AnalogScratch::new(AnalogMode::V2, threshold);
    machine.input(0.0, true);
    assert!(machine.input(TICK_MAX_SIZE * 2.0, true));

    let idle = threshold as usize * 2;
    let held = hold(&mut machine, TICK_MAX_SIZE * 2.0, true, idle + 1);
    let (kept, released) = held.split_at(idle);
    assert!(kept.iter().all(|held| *held), "version 2 waits twice as long as version 1: {held:?}");
    assert_eq!(released, [false]);
}

#[test]
fn version_two_drops_the_hold_and_the_ticks_when_the_spin_reverses() {
    let mut machine = AnalogScratch::new(AnalogMode::V2, 8);
    machine.input(0.0, true);
    assert!(machine.input(TICK_MAX_SIZE * 2.0, true));

    assert!(!machine.input(0.0, true), "reversing stops the spin outright");
    assert!(!machine.is_active());
    assert!(!machine.input(0.0, false), "and the other direction is not held either");
    assert!(!machine.input(-TICK_MAX_SIZE, false), "the ticks were dropped, so travel has to build up again");
    assert!(machine.input(-TICK_MAX_SIZE * 2.0, false));
}

#[test]
fn a_turntable_that_is_switched_off_never_holds_anything() {
    let mut machine = AnalogScratch::new(AnalogMode::Off, 8);
    for value in [0.0, TICK_MAX_SIZE, 1.0, -1.0] {
        assert!(!machine.input(value, true));
        assert!(!machine.input(value, false));
    }
}

#[test]
fn the_threshold_is_clamped_when_a_machine_is_built() {
    assert_eq!(AnalogScratch::new(AnalogMode::V1, 0).threshold(), *ANALOG_THRESHOLD_RANGE.start());
    assert_eq!(AnalogScratch::new(AnalogMode::V1, u32::MAX).threshold(), *ANALOG_THRESHOLD_RANGE.end());
    assert_eq!(AnalogScratch::new(AnalogMode::V1, DEFAULT_ANALOG_THRESHOLD).threshold(), DEFAULT_ANALOG_THRESHOLD);
}

#[test]
fn a_fresh_pad_config_ships_the_reference_defaults_and_nothing_bound() {
    let cfg = PadConfig::default();
    assert_eq!(cfg.debounce_ms, DEFAULT_DEBOUNCE_MS);
    assert_eq!(cfg.analog_threshold, DEFAULT_ANALOG_THRESHOLD);
    assert_eq!(cfg.analog_threshold, 100, "reference implementation PlayModeConfig.java:514");
    assert_eq!(cfg.analog_mode, AnalogMode::Off);
    assert!((cfg.axis_deadzone - DEFAULT_AXIS_DEADZONE).abs() < f32::EPSILON);
    assert!(cfg.lanes.is_empty() && cfg.scratch_reverse.is_empty() && cfg.controls.is_empty());
    for &mode in Mode::ALL {
        assert!((0..mode.key).all(|lane| cfg.lane_binding(mode, lane).is_none()), "{}", mode.name);
    }
}

#[test]
fn sanitise_pulls_every_tunable_back_into_its_range() {
    let mut cfg = PadConfig { debounce_ms: 5_000, analog_threshold: 0, axis_deadzone: 9.0, ..PadConfig::default() };
    cfg.sanitise();
    assert_eq!(cfg.debounce_ms, *DEBOUNCE_MS_RANGE.end());
    assert_eq!(cfg.analog_threshold, *ANALOG_THRESHOLD_RANGE.start());
    assert!((cfg.axis_deadzone - *AXIS_DEADZONE_RANGE.end()).abs() < f32::EPSILON);

    let mut wide = PadConfig { analog_threshold: u32::MAX, axis_deadzone: f32::NAN, ..PadConfig::default() };
    wide.sanitise();
    assert_eq!(wide.analog_threshold, *ANALOG_THRESHOLD_RANGE.end());
    assert!((wide.axis_deadzone - DEFAULT_AXIS_DEADZONE).abs() < f32::EPSILON, "a deadzone that is not a number falls back");
}

#[test]
fn every_binding_shape_round_trips_through_the_config_file() {
    let mut cfg = PadConfig { device_name: Some("Some Controller".into()), analog_mode: AnalogMode::V2, ..PadConfig::default() };
    cfg.set_lane(seven_key(), 0, Some(PadBinding::Button(BUTTON)));
    cfg.set_lane(seven_key(), 1, Some(PadBinding::Axis { axis: AXIS, positive: false }));
    cfg.set_lane(seven_key(), SCRATCH_LANE, Some(PadBinding::AnalogScratch { axis: AXIS }));
    cfg.set_scratch_reverse(seven_key(), SCRATCH_LANE, Some(PadBinding::Axis { axis: AXIS, positive: true }));
    cfg.set_control(ControlAction::HiSpeedUp, Some(PadBinding::Button(BUTTON + 1)));

    let text = ron::ser::to_string_pretty(&cfg, ron::ser::PrettyConfig::default()).expect("a pad config serialises");
    let back: PadConfig = ron::from_str(&text).expect("and parses back");
    assert_eq!(back, cfg);
}

#[test]
fn a_binding_bound_twice_is_reported_as_a_collision() {
    let shared = PadBinding::Button(BUTTON);
    let mut cfg = cfg_with_lane(0, shared);
    cfg.set_lane(seven_key(), 1, Some(PadBinding::Button(BUTTON + 1)));
    assert!(collisions(&cfg, seven_key()).is_empty());

    cfg.set_control(ControlAction::CoverUp, Some(shared));
    assert_eq!(collisions(&cfg, seven_key()), std::iter::once(shared).collect());
}

#[test]
fn clearing_a_binding_leaves_the_lane_unbound() {
    let mut cfg = cfg_with_lane(0, PadBinding::Button(BUTTON));
    cfg.set_lane(seven_key(), 0, None);
    assert_eq!(cfg.lane_binding(seven_key(), 0), None);
    assert!(collisions(&cfg, seven_key()).is_empty());
}

#[test]
fn a_button_press_and_release_reach_the_lane_it_is_bound_to() {
    let cfg = cfg_with_lane(0, PadBinding::Button(BUTTON));
    let mut mapper = PadMapper::new();
    assert_eq!(mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true)), [PadEvent::Lane { lane: 0, dir: ScratchDir::Forward, press: true }]);
    assert!(mapper.resolve(MS, &cfg, seven_key(), &snapshot_button(true)).is_empty(), "a held button repeats nothing");
    assert_eq!(mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_button(false)), [PadEvent::Lane { lane: 0, dir: ScratchDir::Forward, press: false }]);
}

#[test]
fn a_change_inside_the_debounce_window_is_ignored_and_one_after_it_is_taken() {
    let cfg = PadConfig { debounce_ms: DEFAULT_DEBOUNCE_MS, ..cfg_with_lane(0, PadBinding::Button(BUTTON)) };
    let mut mapper = PadMapper::new();
    assert_eq!(mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true)).len(), 1);

    assert!(
        mapper.resolve(MS * 15, &cfg, seven_key(), &snapshot_button(false)).is_empty(),
        "15 ms is inside the 16 ms window the reference implementation keeps"
    );
    assert_eq!(
        mapper.resolve(MS * 17, &cfg, seven_key(), &snapshot_button(false)),
        [PadEvent::Lane { lane: 0, dir: ScratchDir::Forward, press: false }],
        "17 ms is outside it"
    );
}

#[test]
fn the_debounce_window_opens_exactly_on_its_own_edge() {
    let cfg = PadConfig { debounce_ms: DEFAULT_DEBOUNCE_MS, ..cfg_with_lane(0, PadBinding::Button(BUTTON)) };
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true));
    assert_eq!(mapper.resolve(MS * i64::from(DEFAULT_DEBOUNCE_MS), &cfg, seven_key(), &snapshot_button(false)).len(), 1);
}

#[test]
fn an_axis_bound_as_two_keys_holds_each_direction_past_the_deadzone() {
    let mut cfg = cfg_with_lane(SCRATCH_LANE, PadBinding::Axis { axis: AXIS, positive: true });
    cfg.set_scratch_reverse(seven_key(), SCRATCH_LANE, Some(PadBinding::Axis { axis: AXIS, positive: false }));
    let deadzone = cfg.effective_axis_deadzone();
    let mut mapper = PadMapper::new();

    assert!(mapper.resolve(0, &cfg, seven_key(), &snapshot_axis(deadzone)).is_empty(), "resting on the deadzone holds nothing");
    assert_eq!(mapper.resolve(MS, &cfg, seven_key(), &snapshot_axis(1.0)), [PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: true }]);
    assert_eq!(
        mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_axis(-1.0)),
        [
            PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: false },
            PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Backward, press: true },
        ]
    );
}

#[test]
fn a_turntable_drives_both_spin_directions_of_its_scratch_lane() {
    let cfg = PadConfig { analog_mode: AnalogMode::V1, analog_threshold: 8, ..cfg_with_lane(SCRATCH_LANE, PadBinding::AnalogScratch { axis: AXIS }) };
    let mut mapper = PadMapper::new();

    assert!(mapper.resolve(0, &cfg, seven_key(), &snapshot_axis(0.0)).is_empty(), "the reading that starts the machine holds nothing");
    assert_eq!(
        mapper.resolve(MS, &cfg, seven_key(), &snapshot_axis(TICK_MAX_SIZE * 2.0)),
        [PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: true }],
        "spinning right holds the lane forwards"
    );
    assert_eq!(
        mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_axis(0.0)),
        [
            PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: false },
            PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Backward, press: true },
        ],
        "and spinning back the other way swaps the direction in one frame"
    );
}

#[test]
fn a_turntable_on_a_key_lane_only_drives_it_forwards() {
    let cfg = PadConfig { analog_mode: AnalogMode::V1, analog_threshold: 8, ..cfg_with_lane(0, PadBinding::AnalogScratch { axis: AXIS }) };
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, seven_key(), &snapshot_axis(0.0));
    let events = mapper.resolve(MS, &cfg, seven_key(), &snapshot_axis(TICK_MAX_SIZE * 2.0));
    assert_eq!(events, [PadEvent::Lane { lane: 0, dir: ScratchDir::Forward, press: true }]);
    assert!(
        mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_axis(0.0)).iter().all(|event| !matches!(event, PadEvent::Lane { dir: ScratchDir::Backward, .. })),
        "a key lane has no backward spin"
    );
}

#[test]
fn a_turntable_binding_falls_back_to_two_keys_while_the_algorithm_is_off() {
    let cfg = cfg_with_lane(SCRATCH_LANE, PadBinding::AnalogScratch { axis: AXIS });
    assert_eq!(cfg.analog_mode, AnalogMode::Off);
    let mut mapper = PadMapper::new();
    assert_eq!(
        mapper.resolve(0, &cfg, seven_key(), &snapshot_axis(1.0)),
        [PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: true }],
        "with no algorithm the axis is read against the deadzone, as the reference implementation reads it"
    );
}

#[test]
fn changing_the_algorithm_forgets_the_spin_that_was_in_progress() {
    let mut cfg = PadConfig { analog_mode: AnalogMode::V1, analog_threshold: 8, ..cfg_with_lane(SCRATCH_LANE, PadBinding::AnalogScratch { axis: AXIS }) };
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, seven_key(), &snapshot_axis(0.0));
    assert_eq!(mapper.resolve(MS, &cfg, seven_key(), &snapshot_axis(TICK_MAX_SIZE * 2.0)).len(), 1);

    cfg.analog_mode = AnalogMode::V2;
    let events = mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_axis(TICK_MAX_SIZE * 2.0));
    assert_eq!(events, [PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: false }], "the lane the old algorithm was holding is let go");
}

#[test]
fn a_control_fires_on_the_press_and_not_on_the_release() {
    let mut cfg = PadConfig { debounce_ms: 0, ..PadConfig::default() };
    cfg.set_control(ControlAction::HiSpeedUp, Some(PadBinding::Button(BUTTON)));
    let mut mapper = PadMapper::new();

    assert_eq!(mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true)), [PadEvent::Control(ControlAction::HiSpeedUp)]);
    assert!(mapper.resolve(MS, &cfg, seven_key(), &snapshot_button(false)).is_empty(), "letting go steps nothing");
    assert_eq!(mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_button(true)), [PadEvent::Control(ControlAction::HiSpeedUp)]);
}

#[test]
fn one_element_bound_to_two_things_drives_both_of_them() {
    let mut cfg = cfg_with_lane(0, PadBinding::Button(BUTTON));
    cfg.set_lane(seven_key(), 1, Some(PadBinding::Button(BUTTON)));
    let mut mapper = PadMapper::new();
    assert_eq!(
        mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true)),
        [PadEvent::Lane { lane: 0, dir: ScratchDir::Forward, press: true }, PadEvent::Lane { lane: 1, dir: ScratchDir::Forward, press: true },]
    );
}

#[test]
fn a_lane_beyond_the_running_mode_is_not_read() {
    let mut cfg = PadConfig { debounce_ms: 0, ..PadConfig::default() };
    cfg.set_lane(Mode::BEAT_14K, 15, Some(PadBinding::Button(BUTTON)));
    let mut mapper = PadMapper::new();
    assert!(mapper.resolve(0, &cfg, Mode::BEAT_7K, &snapshot_button(true)).is_empty(), "7K never sees the P2 rows of a 14K binding");
    assert_eq!(mapper.resolve(0, &cfg, Mode::BEAT_14K, &snapshot_button(true)).len(), 1);
}

#[test]
fn a_reverse_binding_on_a_key_lane_is_ignored() {
    let mut cfg = cfg_with_lane(0, PadBinding::Button(BUTTON));
    cfg.set_scratch_reverse(seven_key(), 0, Some(PadBinding::Button(BUTTON + 1)));
    let mut snapshot = snapshot_button(false);
    snapshot.set_button(BUTTON + 1, true);
    let mut mapper = PadMapper::new();
    assert!(mapper.resolve(0, &cfg, seven_key(), &snapshot).is_empty(), "only a scratch lane can be spun backwards");
}

#[test]
fn nothing_is_produced_while_the_controller_is_switched_off() {
    let cfg = PadConfig { enabled: false, ..cfg_with_lane(0, PadBinding::Button(BUTTON)) };
    let mut mapper = PadMapper::new();
    assert!(mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true)).is_empty());
}

#[test]
fn a_binding_reads_back_the_label_the_editor_shows() {
    assert_eq!(PadBinding::Button(BUTTON).label(), format!("BUTTON {BUTTON}"));
    assert_eq!(PadBinding::Axis { axis: AXIS, positive: true }.label(), "AXIS 3 +");
    assert_eq!(PadBinding::Axis { axis: AXIS, positive: false }.label(), "AXIS 3 -");
    assert_eq!(PadBinding::AnalogScratch { axis: AXIS }.label(), "ANALOG 3");
}

#[test]
fn an_axis_can_be_promoted_to_a_turntable_but_a_button_cannot() {
    let axis = PadBinding::Axis { axis: AXIS, positive: false };
    assert_eq!(axis.as_analog_scratch(), Some(PadBinding::AnalogScratch { axis: AXIS }));
    assert_eq!(PadBinding::AnalogScratch { axis: AXIS }.as_analog_scratch(), Some(PadBinding::AnalogScratch { axis: AXIS }));
    assert_eq!(PadBinding::Button(BUTTON).as_analog_scratch(), None);
}

#[test]
fn every_analog_mode_has_its_own_label() {
    let labels: std::collections::HashSet<&str> = AnalogMode::ALL.iter().map(|mode| mode.label()).collect();
    assert_eq!(labels.len(), AnalogMode::ALL.len());
    assert!(labels.iter().all(|label| !label.is_empty()));
}

fn snapshot_standard(button: StandardButton, pressed: bool) -> PadSnapshot {
    let mut snapshot = PadSnapshot::default();
    snapshot.set_standard(button, pressed);
    snapshot
}

fn quiet() -> PadConfig {
    PadConfig { debounce_ms: 0, ..PadConfig::default() }
}

/// START and SELECT start on the pad's own menu buttons, with nothing written to the file; every
/// other control, and every lane, starts unbound.
#[test]
fn start_and_select_read_the_standard_menu_buttons_until_rebound() {
    let cfg = PadConfig::default();
    assert_eq!(cfg.control_binding(ControlAction::Start), Some(PadBinding::Standard(StandardButton::Start)));
    assert_eq!(cfg.control_binding(ControlAction::Select), Some(PadBinding::Standard(StandardButton::Select)));
    assert!(cfg.controls.is_empty(), "the defaults are read, not stored");
    for action in ControlAction::ALL.into_iter().filter(|a| !matches!(a, ControlAction::Start | ControlAction::Select)) {
        assert_eq!(cfg.control_binding(action), None, "{action:?}");
    }
    assert!(collisions(&cfg, seven_key()).is_empty(), "and the two defaults share nothing");
}

/// A binding the player sets wins over the default, and clearing it brings the default back.
#[test]
fn a_rebound_start_replaces_the_standard_button_and_clearing_it_restores_it() {
    let mut cfg = PadConfig::default();
    cfg.set_control(ControlAction::Start, Some(PadBinding::Button(BUTTON)));
    assert_eq!(cfg.control_binding(ControlAction::Start), Some(PadBinding::Button(BUTTON)));
    cfg.set_control(ControlAction::Start, None);
    assert_eq!(cfg.control_binding(ControlAction::Start), Some(PadBinding::Standard(StandardButton::Start)));
}

/// A pad block written before START existed (it carries other controls, none of these) reads them on
/// the standard buttons, and a block with one of them bound keeps that binding.
#[test]
fn an_older_pad_block_reads_start_and_select_on_the_standard_buttons() {
    let older: PadConfig = ron::from_str(r#"(controls: {"HI-SPEED UP": Button(4)}, debounce_ms: 4)"#).expect("an older pad block parses");
    assert_eq!(older.control_binding(ControlAction::HiSpeedUp), Some(PadBinding::Button(4)));
    assert_eq!(older.control_binding(ControlAction::Start), Some(PadBinding::Standard(StandardButton::Start)));
    assert_eq!(older.control_binding(ControlAction::Select), Some(PadBinding::Standard(StandardButton::Select)));

    let bound: PadConfig = ron::from_str(r#"(controls: {"START": Button(9)})"#).expect("a pad block with START bound parses");
    assert_eq!(bound.control_binding(ControlAction::Start), Some(PadBinding::Button(9)));
    assert_eq!(bound.control_binding(ControlAction::Select), Some(PadBinding::Standard(StandardButton::Select)));
}

#[test]
fn a_standard_binding_round_trips_through_the_config_file() {
    let mut cfg = PadConfig::default();
    cfg.set_control(ControlAction::Select, Some(PadBinding::Standard(StandardButton::Start)));
    let text = ron::ser::to_string_pretty(&cfg, ron::ser::PrettyConfig::default()).expect("a pad config serialises");
    let back: PadConfig = ron::from_str(&text).expect("and parses back");
    assert_eq!(back, cfg);
}

#[test]
fn a_standard_binding_reads_back_the_label_the_editor_shows() {
    assert_eq!(PadBinding::Standard(StandardButton::Start).label(), "STANDARD START");
    assert_eq!(PadBinding::Standard(StandardButton::Select).label(), "STANDARD SELECT");
    assert_eq!(PadBinding::Standard(StandardButton::Start).as_analog_scratch(), None);
}

/// The standard START button fires the control on its press only, like any other control, and the
/// mapper still reports it held until it is let go.
#[test]
fn the_standard_start_button_fires_once_and_stays_held_until_released() {
    let cfg = quiet();
    let mut mapper = PadMapper::new();
    assert!(mapper.held(&cfg, seven_key()).is_empty());

    assert_eq!(mapper.resolve(0, &cfg, seven_key(), &snapshot_standard(StandardButton::Start, true)), [PadEvent::Control(ControlAction::Start)]);
    assert_eq!(mapper.held(&cfg, seven_key()), [PadEvent::Control(ControlAction::Start)]);
    assert!(mapper.resolve(MS, &cfg, seven_key(), &snapshot_standard(StandardButton::Start, true)).is_empty(), "holding repeats nothing");
    assert_eq!(mapper.held(&cfg, seven_key()), [PadEvent::Control(ControlAction::Start)], "and it is still down");

    assert!(mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_standard(StandardButton::Start, false)).is_empty(), "a control has no release event");
    assert!(mapper.held(&cfg, seven_key()).is_empty(), "but it is no longer held");
}

/// START and SELECT are separate buttons: one down never reads as the other.
#[test]
fn the_two_menu_buttons_are_held_independently() {
    let cfg = quiet();
    let mut mapper = PadMapper::new();
    let mut snapshot = PadSnapshot::default();
    snapshot.set_standard(StandardButton::Select, true);
    mapper.resolve(0, &cfg, seven_key(), &snapshot);
    assert_eq!(mapper.held(&cfg, seven_key()), [PadEvent::Control(ControlAction::Select)]);

    snapshot.set_standard(StandardButton::Start, true);
    mapper.resolve(MS, &cfg, seven_key(), &snapshot);
    let held = mapper.held(&cfg, seven_key());
    assert!(held.contains(&PadEvent::Control(ControlAction::Start)) && held.contains(&PadEvent::Control(ControlAction::Select)));
}

/// The native code of a button that gilrs also names START does not drive START: only the standard
/// name does, so an arcade controller whose codes are its own is not misread.
#[test]
fn a_native_button_code_does_not_press_the_standard_start() {
    let cfg = quiet();
    let mut mapper = PadMapper::new();
    assert!(mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true)).is_empty());
    assert!(mapper.held(&cfg, seven_key()).is_empty());
}

/// A lane's held state follows its button, and a lane two elements drive stays held until both let go.
#[test]
fn a_lane_is_held_while_any_element_bound_to_it_is() {
    let mut cfg = cfg_with_lane(SCRATCH_LANE, PadBinding::Button(BUTTON));
    cfg.set_scratch_reverse(seven_key(), SCRATCH_LANE, Some(PadBinding::Button(BUTTON + 1)));
    let mut mapper = PadMapper::new();
    let forward = PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Forward, press: true };
    let backward = PadEvent::Lane { lane: SCRATCH_LANE, dir: ScratchDir::Backward, press: true };

    mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true));
    assert_eq!(mapper.held(&cfg, seven_key()), [forward]);

    let mut both = snapshot_button(true);
    both.set_button(BUTTON + 1, true);
    mapper.resolve(MS, &cfg, seven_key(), &both);
    assert_eq!(mapper.held(&cfg, seven_key()), [forward, backward], "both directions of one turntable can be held");

    mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot_button(false));
    assert!(mapper.held(&cfg, seven_key()).is_empty());
}

#[test]
fn an_element_bound_to_two_lanes_holds_both() {
    let mut cfg = cfg_with_lane(0, PadBinding::Button(BUTTON));
    cfg.set_lane(seven_key(), 1, Some(PadBinding::Button(BUTTON)));
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true));
    assert_eq!(mapper.held(&cfg, seven_key()).len(), 2);
}

/// A controller switched off holds nothing, whatever it last reported.
#[test]
fn nothing_is_held_while_the_controller_is_switched_off() {
    let cfg = cfg_with_lane(0, PadBinding::Button(BUTTON));
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, seven_key(), &snapshot_button(true));
    let off = PadConfig { enabled: false, ..cfg };
    assert!(mapper.held(&off, seven_key()).is_empty());
}

/// What is held is read against the mode that is up: a lane the mode does not have is not held.
#[test]
fn a_lane_beyond_the_running_mode_is_not_held() {
    let mut cfg = quiet();
    cfg.set_lane(Mode::BEAT_14K, 15, Some(PadBinding::Button(BUTTON)));
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, Mode::BEAT_14K, &snapshot_button(true));
    assert_eq!(mapper.held(&cfg, Mode::BEAT_14K).len(), 1);
    assert!(mapper.held(&cfg, Mode::BEAT_7K).is_empty());
}

/// A pad block written while the control rows were still called what they were is read by the names
/// it holds, and a control bound now is stored under a name of the same kind rather than under what
/// its row is called on screen.
#[test]
fn a_pad_block_from_before_the_rows_were_renamed_still_binds_its_controls() {
    let older: PadConfig =
        ron::from_str(r#"(controls: {"COVER UP": Button(4), "COVER DOWN": Button(5), "LIFT UP": Button(6)})"#).expect("an older pad block parses");
    assert_eq!(older.control_binding(ControlAction::CoverUp), Some(PadBinding::Button(4)));
    assert_eq!(older.control_binding(ControlAction::CoverDown), Some(PadBinding::Button(5)));
    assert_eq!(older.control_binding(ControlAction::LiftUp), Some(PadBinding::Button(6)));
    assert!(older.shortcuts.is_empty());

    let mut cfg = PadConfig::default();
    cfg.set_control(ControlAction::CoverToggle, Some(PadBinding::Button(BUTTON)));
    assert_eq!(cfg.controls.keys().collect::<Vec<_>>(), ["COVER TOGGLE"]);
    assert!(!cfg.controls.contains_key(ControlAction::CoverToggle.label()), "the binding is stored under the words on screen");
}

/// A shortcut bound to a button fires once on the press, like a control, and is held for as long as
/// the button is.
#[test]
fn a_shortcut_on_a_button_fires_once_on_the_press() {
    let mut cfg = quiet();
    cfg.set_shortcut(Shortcut::Search, Some(PadBinding::Button(BUTTON)));
    let mut mapper = PadMapper::new();
    let mut snapshot = PadSnapshot::default();
    snapshot.set_button(BUTTON, true);
    assert_eq!(mapper.resolve(0, &cfg, seven_key(), &snapshot), vec![PadEvent::Shortcut(Shortcut::Search)]);
    assert_eq!(mapper.resolve(MS, &cfg, seven_key(), &snapshot), Vec::new(), "a button that stays down fired again");
    assert_eq!(mapper.held(&cfg, seven_key()), vec![PadEvent::Shortcut(Shortcut::Search)]);
    snapshot.set_button(BUTTON, false);
    assert_eq!(mapper.resolve(2 * MS, &cfg, seven_key(), &snapshot), Vec::new(), "letting go fired");
    assert!(mapper.held(&cfg, seven_key()).is_empty());
}

/// The same button can open a screen from the browser and play a lane in a run: the two are never
/// read at the same moment, so the editor does not flag them. Two shortcuts of one screen on one
/// button are flagged.
#[test]
fn a_button_shared_across_screens_is_not_a_clash_and_one_shared_on_a_screen_is() {
    let mut cfg = PadConfig::default();
    cfg.set_lane(seven_key(), 0, Some(PadBinding::Button(BUTTON)));
    cfg.set_shortcut(Shortcut::Folders, Some(PadBinding::Button(BUTTON)));
    assert!(collisions(&cfg, seven_key()).is_empty());
    cfg.set_shortcut(Shortcut::Tables, Some(PadBinding::Button(BUTTON)));
    assert_eq!(collisions(&cfg, seven_key()), std::iter::once(PadBinding::Button(BUTTON)).collect());
}

/// The shortcuts that are read as a key held down are the keyboard's alone: a controller is not
/// given one, whatever the file says.
#[test]
fn a_shortcut_the_keyboard_alone_reads_takes_no_button() {
    let mut cfg = PadConfig::default();
    for shortcut in [Shortcut::OptionsPin, Shortcut::OptionsHold, Shortcut::RefDetail] {
        cfg.set_shortcut(shortcut, Some(PadBinding::Button(BUTTON)));
        assert_eq!(cfg.shortcut_binding(shortcut), None, "{shortcut:?}");
    }
    let edited: PadConfig = ron::from_str(r#"(shortcuts: {"options_hold": Button(3), "search": Button(4)})"#).expect("the pad block parses");
    assert_eq!(edited.shortcut_binding(Shortcut::OptionsHold), None);
    assert_eq!(edited.shortcut_binding(Shortcut::Search), Some(PadBinding::Button(4)));
}

/// Every kind of thing is bound and read back through the one door the key editor uses.
#[test]
fn the_editor_binds_and_reads_every_kind_of_row_the_same_way() {
    let mode = seven_key();
    let mut cfg = PadConfig::default();
    let rows = [Bindable::Lane(2), Bindable::ScratchReverse(SCRATCH_LANE), Bindable::Control(ControlAction::CoverToggle), Bindable::Shortcut(Shortcut::Sort)];
    for (index, target) in rows.into_iter().enumerate() {
        let binding = PadBinding::Button(BUTTON + index as u32);
        cfg.bind(mode, target, Some(binding));
        assert_eq!(cfg.binding_of(mode, target), Some(binding), "{target:?}");
        cfg.bind(mode, target, None);
        assert_eq!(cfg.binding_of(mode, target), None, "{target:?}");
    }
    cfg.bind(mode, Bindable::Control(ControlAction::Start), Some(PadBinding::Button(BUTTON)));
    cfg.bind(mode, Bindable::Control(ControlAction::Start), None);
    assert_eq!(
        cfg.binding_of(mode, Bindable::Control(ControlAction::Start)),
        Some(PadBinding::Standard(StandardButton::Start)),
        "START goes back to the pad's own"
    );
}

/// What one tick of a real turntable moves its axis by, a little under the most a tick can be
/// (`BMControllerInputProcessor.java:81`).
const HARDWARE_TICK: f32 = 0.00784;

/// A turntable's travel is counted in ticks, forward up and backward down, from its first reading
/// on, and taking the count empties it.
#[test]
fn a_turntable_counts_the_ticks_it_turned_since_they_were_last_taken() {
    let cfg = PadConfig { analog_mode: AnalogMode::V2, ..cfg_with_lane(SCRATCH_LANE, PadBinding::AnalogScratch { axis: AXIS }) };
    let mut mapper = PadMapper::new();
    mapper.resolve(0, &cfg, seven_key(), &PadSnapshot::default());
    assert_eq!(mapper.take_turn(), Some(0), "an axis the device has not reported has not turned");

    let resting = 0.5;
    let tick = HARDWARE_TICK;
    mapper.resolve(MS, &cfg, seven_key(), &snapshot_axis(resting));
    assert_eq!(mapper.take_turn(), Some(0), "where the disc stood when it was first read is not a turn");

    mapper.resolve(2 * MS, &cfg, seven_key(), &snapshot_axis(resting + tick));
    mapper.resolve(3 * MS, &cfg, seven_key(), &snapshot_axis(resting + tick * 3.0));
    assert_eq!(mapper.take_turn(), Some(3), "three ticks forward over two frames");
    assert_eq!(mapper.take_turn(), Some(0), "taking the count empties it");

    mapper.resolve(4 * MS, &cfg, seven_key(), &snapshot_axis(resting + tick));
    assert_eq!(mapper.take_turn(), Some(-2), "two ticks back");

    mapper.resolve(5 * MS, &cfg, seven_key(), &snapshot_axis(resting + tick));
    assert_eq!(mapper.take_turn(), Some(0), "a disc that stands still turns nothing");
}

/// A controller with no axis read as a turntable has no turn to count: its scratch is keys.
#[test]
fn a_controller_without_an_analog_turntable_has_no_turn() {
    let mut mapper = PadMapper::new();
    let keys = cfg_with_lane(SCRATCH_LANE, PadBinding::Button(BUTTON));
    mapper.resolve(0, &keys, seven_key(), &snapshot_button(true));
    assert_eq!(mapper.take_turn(), None);

    let off = cfg_with_lane(SCRATCH_LANE, PadBinding::AnalogScratch { axis: AXIS });
    assert_eq!(off.analog_mode, AnalogMode::Off);
    mapper.resolve(MS, &off, seven_key(), &snapshot_axis(0.2));
    mapper.resolve(2 * MS, &off, seven_key(), &snapshot_axis(0.4));
    assert_eq!(mapper.take_turn(), None, "an axis read as two keys is not counted in ticks");
}
