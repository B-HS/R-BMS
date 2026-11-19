//! Two controller elements held together: when the pair fires, what the press that makes it fires
//! beside it, what it holds, ESC going down and coming up, and what the key editor is told.

use super::*;

const MS: i64 = 1_000;
const AXIS: u32 = 3;

/// The two buttons a control or a shortcut is bound to held together, and the second one's lane.
const FIRST: u32 = 21;
const SECOND: u32 = 22;
const LANE: usize = 0;

fn seven_key() -> Mode {
    Mode::BEAT_7K
}

fn quiet() -> PadConfig {
    PadConfig { debounce_ms: 0, ..PadConfig::default() }
}

fn lane(press: bool) -> PadEvent {
    PadEvent::Lane { lane: LANE, dir: ScratchDir::Forward, press }
}

fn buttons(down: &[u32]) -> PadSnapshot {
    let mut snapshot = PadSnapshot::default();
    for code in [FIRST, SECOND] {
        snapshot.set_button(code, down.contains(&code));
    }
    snapshot
}

const RELEASE: PadMark = PadMark { pressed: false, made_a_chord: false };

/// SEARCH on two buttons held together, HI-SPEED UP on the first alone and a lane on the second.
fn chorded() -> PadConfig {
    let mut cfg = quiet();
    cfg.bind_chord(Bindable::Shortcut(Shortcut::Search), PadBinding::Button(FIRST), PadBinding::Button(SECOND));
    cfg.set_control(ControlAction::HiSpeedUp, Some(PadBinding::Button(FIRST)));
    cfg.set_lane(seven_key(), LANE, Some(PadBinding::Button(SECOND)));
    cfg
}

/// Two elements held together fire once, when the second goes down, in either order. The press that
/// makes them fires nothing its element drives alone but its lane, which is marked as the pair's;
/// the first element alone did what it does before the pair was made.
#[test]
fn two_elements_held_together_fire_once_in_either_order_and_the_making_press_plays_only_its_lane() {
    let cfg = chorded();
    let search = PadEvent::Shortcut(Shortcut::Search);
    let mut mapper = PadMapper::new();
    assert_eq!(mapper.resolve_marked(0, &cfg, seven_key(), &buttons(&[FIRST])), [(PadEvent::Control(ControlAction::HiSpeedUp), PadMark::default())]);
    assert_eq!(
        mapper.resolve_marked(MS, &cfg, seven_key(), &buttons(&[FIRST, SECOND])),
        [(search, PadMark::default()), (lane(true), PadMark { pressed: true, made_a_chord: true })],
        "the pair, and the lane of the press that made it"
    );
    assert!(mapper.resolve(MS * 2, &cfg, seven_key(), &buttons(&[FIRST, SECOND])).is_empty(), "holding the pair repeats nothing");
    assert_eq!(mapper.resolve(MS * 3, &cfg, seven_key(), &buttons(&[FIRST])), [lane(false)], "breaking the pair fires nothing but the lane's release");

    let mut mapper = PadMapper::new();
    assert_eq!(mapper.resolve(0, &cfg, seven_key(), &buttons(&[SECOND])), [lane(true)]);
    assert_eq!(mapper.resolve(MS, &cfg, seven_key(), &buttons(&[SECOND, FIRST])), [search], "the other order makes the pair, and HI-SPEED UP is held back");
}

/// A pair is held while both its elements are; an element of a held pair holds nothing it drives
/// alone but a lane, so the pair the player bound wins over START held with a key.
#[test]
fn a_pair_is_held_while_both_are_down_and_takes_its_elements_from_what_they_hold_alone() {
    let mut cfg = quiet();
    cfg.bind_chord(Bindable::Control(ControlAction::LiftUp), PadBinding::Standard(StandardButton::Start), PadBinding::Button(SECOND));
    cfg.set_lane(seven_key(), LANE, Some(PadBinding::Button(SECOND)));
    let mut mapper = PadMapper::new();
    let mut snapshot = PadSnapshot::default();
    snapshot.set_standard(StandardButton::Start, true);
    mapper.resolve(0, &cfg, seven_key(), &snapshot);
    assert_eq!(mapper.held(&cfg, seven_key()), [PadEvent::Control(ControlAction::Start)]);
    snapshot.set_button(SECOND, true);
    mapper.resolve(MS, &cfg, seven_key(), &snapshot);
    assert_eq!(mapper.held(&cfg, seven_key()), [lane(true), PadEvent::Control(ControlAction::LiftUp)], "START is the pair's while the pair is held");
    snapshot.set_button(SECOND, false);
    mapper.resolve(MS * 2, &cfg, seven_key(), &snapshot);
    assert!(mapper.held(&cfg, seven_key()).is_empty(), "START held on through the pair would read as START pressed a second time");
    snapshot.set_standard(StandardButton::Start, false);
    mapper.resolve(MS * 3, &cfg, seven_key(), &snapshot);
    snapshot.set_standard(StandardButton::Start, true);
    mapper.resolve(MS * 4, &cfg, seven_key(), &snapshot);
    assert_eq!(mapper.held(&cfg, seven_key()), [PadEvent::Control(ControlAction::Start)], "and START's again once it goes down again");
}

/// A controller that goes away lets go of everything it held, for the key editor as for play: a
/// button held when it went is not the first of two with one pressed once it is back.
#[test]
fn a_capture_lets_go_of_what_a_controller_held_when_it_went_away() {
    let mut capture = PadCapture::default();
    capture.note(PadChange::Button { code: FIRST, standard: None, pressed: true });
    capture.note(PadChange::Axis { code: AXIS, value: -0.9 });
    capture.note(PadChange::Disconnected);
    capture.note(PadChange::Button { code: SECOND, standard: None, pressed: true });
    let minus = PadBinding::Axis { axis: AXIS, positive: false };
    assert_eq!(
        capture.take(),
        [(PadBinding::Button(FIRST), true), (minus, true), (PadBinding::Button(FIRST), false), (minus, false), (PadBinding::Button(SECOND), true)]
    );
    capture.note(PadChange::Disconnected);
    assert_eq!(capture.take(), [(PadBinding::Button(SECOND), false)], "only what is still held is let go");
}

/// ESC goes up as well as down, alone or as a pair, and the mark says which.
#[test]
fn esc_fires_on_the_press_and_on_the_release_alone_or_as_a_pair() {
    let escape = PadEvent::Control(ControlAction::Escape);
    let mut cfg = quiet();
    cfg.set_control(ControlAction::Escape, Some(PadBinding::Button(FIRST)));
    let mut mapper = PadMapper::new();
    assert_eq!(mapper.resolve_marked(0, &cfg, seven_key(), &buttons(&[FIRST])), [(escape, PadMark::default())]);
    assert_eq!(mapper.resolve_marked(MS, &cfg, seven_key(), &buttons(&[])), [(escape, RELEASE)]);

    cfg.bind_chord(Bindable::Control(ControlAction::Escape), PadBinding::Button(FIRST), PadBinding::Button(SECOND));
    let mut mapper = PadMapper::new();
    assert!(mapper.resolve(0, &cfg, seven_key(), &buttons(&[SECOND])).is_empty());
    assert_eq!(mapper.resolve_marked(MS, &cfg, seven_key(), &buttons(&[SECOND, FIRST])), [(escape, PadMark::default())]);
    assert_eq!(mapper.resolve_marked(MS * 2, &cfg, seven_key(), &buttons(&[FIRST])), [(escape, RELEASE)]);
    assert!(mapper.resolve(MS * 3, &cfg, seven_key(), &buttons(&[])).is_empty(), "the pair was already broken");
}

/// The pair is read through the debounce: a release of either element inside the window waits for
/// it, and the pair breaks when it is taken.
#[test]
fn a_pair_breaks_when_the_debounce_lets_the_release_through() {
    let cfg = PadConfig { debounce_ms: DEFAULT_DEBOUNCE_MS, ..chorded() };
    let mut mapper = PadMapper::new();
    let mut snapshot = PadSnapshot::default();
    let change = |code: u32, pressed: bool| PadChange::Button { code, standard: None, pressed };
    let changes = [(0, change(FIRST, true)), (MS, change(SECOND, true)), (2 * MS, change(SECOND, false))];
    let events = mapper.resolve_changes(&changes, 3 * MS, &cfg, seven_key(), &mut snapshot);
    assert_eq!(events, [(PadEvent::Control(ControlAction::HiSpeedUp), 0), (PadEvent::Shortcut(Shortcut::Search), MS), (lane(true), MS)]);
    assert!(mapper.held(&cfg, seven_key()).contains(&PadEvent::Shortcut(Shortcut::Search)), "the release waits for the window");
    let window = i64::from(DEFAULT_DEBOUNCE_MS) * MS;
    assert_eq!(mapper.resolve_changes(&[], MS + window, &cfg, seven_key(), &mut snapshot), [(lane(false), MS + window)]);
    assert!(!mapper.held(&cfg, seven_key()).contains(&PadEvent::Shortcut(Shortcut::Search)));
}

/// The key editor is told each element going down and coming up: a button by its code, and an axis
/// direction as it passes the capture threshold and comes back inside it.
#[test]
fn a_capture_reports_each_element_going_down_and_coming_up() {
    let mut capture = PadCapture::default();
    let button = |pressed: bool| PadChange::Button { code: FIRST, standard: None, pressed };
    capture.note(button(true));
    for value in [0.1, -0.9, -0.95, 0.8, 0.0] {
        capture.note(PadChange::Axis { code: AXIS, value });
    }
    capture.note(button(false));
    let minus = PadBinding::Axis { axis: AXIS, positive: false };
    let plus = PadBinding::Axis { axis: AXIS, positive: true };
    assert_eq!(
        capture.take(),
        [(PadBinding::Button(FIRST), true), (minus, true), (minus, false), (plus, true), (plus, false), (PadBinding::Button(FIRST), false)]
    );
    assert!(capture.take().is_empty(), "taking clears it");
    for _ in 0..CAPTURE_EVENTS_KEPT * 2 {
        capture.note(button(true));
    }
    assert_eq!(capture.take().len(), CAPTURE_EVENTS_KEPT, "what nobody asks for is bounded");
}

/// An element pair round-trips through the file, and is labelled as the editor shows it.
#[test]
fn an_element_pair_survives_the_file_and_reads_back_its_label() {
    let mut cfg = PadConfig::default();
    cfg.bind_chord(Bindable::Control(ControlAction::Escape), PadBinding::Button(FIRST), PadBinding::Axis { axis: AXIS, positive: false });
    let back: PadConfig = ron::from_str(&ron::ser::to_string(&cfg).expect("written")).expect("read back");
    assert_eq!(back, cfg);
    assert_eq!(PadBinding::chord_label(PadBinding::Button(FIRST), back.control_chord(ControlAction::Escape)), format!("BUTTON {FIRST} + AXIS {AXIS} -"));
}

/// The turntable of the seven-key layout read as one on [`AXIS`].
fn on_a_turntable() -> PadConfig {
    let mut cfg = quiet();
    cfg.set_lane(seven_key(), seven_key().scratch[0], Some(PadBinding::AnalogScratch { axis: AXIS }));
    cfg
}

/// An axis a lane reads as a turntable spins, and is never one of two elements held together: the
/// pair is not bound, in either place; a pair with another axis in it still is.
#[test]
fn a_direction_of_a_turntable_is_not_bound_into_a_pair() {
    let escape = Bindable::Control(ControlAction::Escape);
    let turntable_up = PadBinding::Axis { axis: AXIS, positive: true };
    let turntable_down = PadBinding::Axis { axis: AXIS, positive: false };
    for (first, second) in [(PadBinding::Button(FIRST), turntable_up), (turntable_down, PadBinding::Button(FIRST))] {
        let mut cfg = on_a_turntable();
        cfg.bind_chord(escape, first, second);
        assert_eq!((cfg.control_binding(ControlAction::Escape), cfg.control_chord(ControlAction::Escape)), (None, None), "{first:?} + {second:?}");
    }
    let other_axis = PadBinding::Axis { axis: AXIS + 1, positive: true };
    let mut cfg = on_a_turntable();
    cfg.bind_chord(escape, PadBinding::Button(FIRST), other_axis);
    assert_eq!(cfg.control_chord(ControlAction::Escape), Some(other_axis));
    assert!(cfg.is_on_a_turntable(turntable_up) && !cfg.is_on_a_turntable(other_axis) && !cfg.is_on_a_turntable(PadBinding::Button(FIRST)));
}

/// A file that holds a pair with a direction of a turntable in it, written before such a pair was
/// refused or by hand, is read without the pair: the control or the shortcut reads what it ships
/// with on the controller, and a pair of two buttons is kept.
#[test]
fn a_pair_on_a_turntable_is_let_go_of_when_the_file_is_read() {
    let mut cfg = on_a_turntable();
    let turntable_up = PadBinding::Axis { axis: AXIS, positive: true };
    cfg.controls.insert(ControlAction::Start.config_key().to_string(), PadBinding::Button(FIRST));
    cfg.control_chords.insert(ControlAction::Start.config_key().to_string(), turntable_up);
    cfg.shortcuts.insert(Shortcut::Search.config_key().to_string(), turntable_up);
    cfg.shortcut_chords.insert(Shortcut::Search.config_key().to_string(), PadBinding::Button(SECOND));
    cfg.bind_chord(Bindable::Shortcut(Shortcut::Sort), PadBinding::Button(FIRST), PadBinding::Button(SECOND));
    let mut back: PadConfig = ron::from_str(&ron::ser::to_string(&cfg).expect("written")).expect("read back");
    back.sanitise();
    let start = (back.control_binding(ControlAction::Start), back.control_chord(ControlAction::Start));
    assert_eq!(start, (Some(PadBinding::Standard(StandardButton::Start)), None));
    assert_eq!((back.shortcut_binding(Shortcut::Search), back.shortcut_chord(Shortcut::Search)), (None, None));
    assert_eq!(back.shortcut_chord(Shortcut::Sort), Some(PadBinding::Button(SECOND)), "a pair of two buttons was let go of");
    assert_eq!(back.lane_binding(seven_key(), seven_key().scratch[0]), Some(PadBinding::AnalogScratch { axis: AXIS }));
}
