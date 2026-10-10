use rbms_render::skin_render::{SkinTextWriter, TextEntryStart};
use rbms_skin::property::generated::{
    BUTTON_LNMODE, RATE_BGMVOLUME, RATE_KEYVOLUME, RATE_MASTERVOLUME, RATE_MUSICSELECT_POSITION, RATE_RANKING_POSITION, STRING_SEARCHWORD, STRING_TITLE,
};
use winit::event::Ime;

use super::*;
use crate::skin_host::{Cluster, RequestQueue};

fn down(code: KeyCode, text: Option<&'static str>) -> KeyInput<'static> {
    KeyInput { code, pressed: true, released: false, text }
}

fn repeating(code: KeyCode, text: Option<&'static str>) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: false, text }
}

fn up(code: KeyCode) -> KeyInput<'static> {
    KeyInput { code, pressed: false, released: true, text: None }
}

fn session(shown: &str) -> TextSession {
    TextSession::new(6, 3, TextEntryStart { writer: SkinTextWriter::Id(STRING_SEARCHWORD), shown: shown.to_string() })
}

fn app(tag: &str) -> crate::App {
    let home = std::env::temp_dir().join(format!("rbms-writers-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("the scratch folder is writable");
    crate::App::new(String::new(), crate::Config::default(), crate::LaunchOptions::default(), home.join("settings.ron"))
}

#[test]
fn only_the_three_volume_rates_set_a_volume() {
    assert_eq!(rate_bus(RATE_MASTERVOLUME), Some(VolumeBus::System));
    assert_eq!(rate_bus(RATE_KEYVOLUME), Some(VolumeBus::Key));
    assert_eq!(rate_bus(RATE_BGMVOLUME), Some(VolumeBus::Background));
    for other in [RATE_MUSICSELECT_POSITION, RATE_RANKING_POSITION, 0, 7, 20, 110] {
        assert_eq!(rate_bus(other), None, "rate {other} is no volume");
    }
}

#[test]
fn a_volume_is_set_on_its_own_bus_and_held_to_the_range_a_volume_row_can_produce() {
    let mut audio = AudioOptions::default();
    let master = audio.master;

    assert!(set_volume(&mut audio, VolumeBus::System, 0.25));
    assert_eq!((audio.system, audio.key, audio.bg), (0.25, DEFAULT_BUS_VOLUME, DEFAULT_BUS_VOLUME), "only the bus asked for moves");
    assert!(set_volume(&mut audio, VolumeBus::Key, 0.75));
    assert!(set_volume(&mut audio, VolumeBus::Background, 0.1));
    assert_eq!((audio.system, audio.key, audio.bg), (0.25, 0.75, 0.1));
    assert_eq!(audio.master, master, "the master gain of the engine is no skin's volume");

    assert!(!set_volume(&mut audio, VolumeBus::System, 0.25), "writing the volume it already has moves nothing");
    assert!(set_volume(&mut audio, VolumeBus::System, 7.5));
    assert_eq!(audio.system, 1.0, "past the loudest is the loudest");
    assert!(set_volume(&mut audio, VolumeBus::System, -3.0));
    assert_eq!(audio.system, 0.0, "below the quietest is the quietest");
    assert!(set_volume(&mut audio, VolumeBus::System, f32::NAN));
    assert_eq!(audio.system, DEFAULT_BUS_VOLUME, "a value that is not a number is the default, as in the reference's validation");
    assert!(set_volume(&mut audio, VolumeBus::System, f32::INFINITY));
    assert_eq!(audio.system, 1.0);
}

#[test]
fn a_volume_request_is_carried_out_at_once_and_any_other_request_is_left_alone() {
    let mut app = app("volumes");
    let settings_untouched = |app: &mut crate::App| app.shared.take_skin_settings_dirty();

    assert!(carry_out(&mut app.shared, &ClusterRequest::WriteRate { id: RATE_KEYVOLUME, value: 0.3 }));
    assert_eq!(app.shared.config.audio.key, 0.3, "a slider's write sets the volume");
    assert!(settings_untouched(&mut app), "and asks for the settings to be written");

    assert!(carry_out(&mut app.shared, &ClusterRequest::SetVolume { bus: VolumeBus::Background, value: 4.0 }));
    assert_eq!(app.shared.config.audio.bg, 1.0, "main_state.set_volume_bg sets it too, held to the range");
    assert!(settings_untouched(&mut app));

    let current = app.shared.config.audio.system;
    assert!(carry_out(&mut app.shared, &ClusterRequest::WriteRate { id: RATE_MASTERVOLUME, value: current }));
    assert!(!settings_untouched(&mut app), "a write that moves nothing asks for nothing to be saved");

    for request in [
        ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, value: 0.5 },
        ClusterRequest::WriteRate { id: RATE_RANKING_POSITION, value: 0.5 },
        ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: "x".to_string() },
        ClusterRequest::Event { id: BUTTON_LNMODE, arg1: 1, arg2: 0 },
    ] {
        assert!(!carry_out(&mut app.shared, &request), "{request:?} is not a volume");
    }
    assert_eq!(app.shared.config.audio.key, 0.3);
    assert!(!settings_untouched(&mut app));
}

#[test]
fn the_browser_carries_out_the_cursor_and_the_search_and_nothing_else() {
    assert_eq!(browser_write(&ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, value: 0.4 }), Some(BrowserWrite::Position(0.4)));
    assert_eq!(browser_write(&ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: "word".to_string() }), Some(BrowserWrite::Search("word".to_string())));
    assert_eq!(browser_write(&ClusterRequest::WriteRate { id: RATE_RANKING_POSITION, value: 0.4 }), None, "the ranking list is the result screen's");
    assert_eq!(browser_write(&ClusterRequest::WriteRate { id: RATE_MASTERVOLUME, value: 0.4 }), None);
    assert_eq!(browser_write(&ClusterRequest::WriteText { id: STRING_TITLE, value: "x".to_string() }), None, "the reference writes no other string");
    assert_eq!(browser_write(&ClusterRequest::Event { id: BUTTON_LNMODE, arg1: 1, arg2: 0 }), None);
}

#[test]
fn a_share_of_a_list_falls_on_the_row_the_reference_puts_it_on() {
    assert_eq!(scaled_index(0.0, 10), Some(0));
    assert_eq!(scaled_index(0.5, 10), Some(5));
    assert_eq!(scaled_index(0.999, 10), Some(9));
    assert_eq!(scaled_index(0.34, 3), Some(1));
    assert_eq!(scaled_index(0.33, 3), Some(0), "the share is cut down, not rounded");
    assert_eq!(scaled_index(1.0, 10), None, "the end of the scrollbar writes exactly one, which the reference does not take");
    assert_eq!(scaled_index(-0.01, 10), None);
    assert_eq!(scaled_index(f32::NAN, 10), None);
    assert_eq!(scaled_index(0.5, 0), None, "an empty list has no row to fall on");
}

#[test]
fn a_request_queue_gives_up_only_the_requests_it_is_asked_for() {
    let mut queue = RequestQueue::default();
    let position = ClusterRequest::WriteRate { id: RATE_MUSICSELECT_POSITION, value: 0.5 };
    let event = ClusterRequest::Event { id: BUTTON_LNMODE, arg1: 1, arg2: 0 };
    let word = ClusterRequest::WriteText { id: STRING_SEARCHWORD, value: "w".to_string() };
    queue.push(Cluster::Select, event.clone());
    queue.push(Cluster::Select, position.clone());
    queue.push(Cluster::Result, position.clone());
    queue.push(Cluster::Select, word.clone());

    let taken = queue.take_if(Cluster::Select, |request| browser_write(request).is_some());
    assert_eq!(taken, [position.clone(), word], "the writes come out oldest first");
    assert_eq!(queue.take(Cluster::Select), [event], "the event is still waiting for whoever answers it");
    assert_eq!(queue.take(Cluster::Result), [position], "another cluster's request is not touched even when it matches");
}

#[test]
fn typing_starts_from_what_the_text_shows_with_the_caret_after_it() {
    let mut typing = session("abc");
    assert_eq!((typing.screen(), typing.object()), (6, 3));
    assert_eq!((typing.entry().typed, typing.entry().caret, typing.entry().object), ("abc", 3, 3));
    assert_eq!(typing.key(&down(KeyCode::KeyD, Some("d"))), Typing::Going);
    assert_eq!(typing.key(&down(KeyCode::ArrowLeft, None)), Typing::Going);
    assert_eq!(typing.key(&down(KeyCode::KeyX, Some("x"))), Typing::Going);
    assert_eq!((typing.entry().typed, typing.entry().caret), ("abcxd", 4), "a letter lands at the caret");
    assert_eq!(typing.key(&down(KeyCode::Home, None)), Typing::Going);
    assert_eq!(typing.key(&down(KeyCode::Delete, None)), Typing::Going);
    assert_eq!(typing.key(&repeating(KeyCode::Delete, None)), Typing::Going);
    assert_eq!(typing.entry().typed, "cxd", "a key held down repeats");
    assert_eq!(typing.key(&down(KeyCode::End, None)), Typing::Going);
    assert_eq!(typing.key(&down(KeyCode::Backspace, None)), Typing::Going);
    assert_eq!(typing.entry().typed, "cx");
}

#[test]
fn enter_confirms_and_escape_cancels_once_per_press() {
    let mut typing = session("");
    assert_eq!(typing.key(&down(KeyCode::KeyA, Some("a"))), Typing::Going);
    assert_eq!(typing.key(&repeating(KeyCode::Enter, None)), Typing::Going, "Enter held down is not a second confirmation");
    assert_eq!(typing.key(&repeating(KeyCode::Escape, None)), Typing::Going);
    assert_eq!(typing.key(&up(KeyCode::Enter)), Typing::Going);
    assert_eq!(typing.key(&down(KeyCode::NumpadEnter, None)), Typing::Confirmed, "the numpad's Enter is Enter");
    assert_eq!(typing.confirm(), TextWrite { writer: SkinTextWriter::Id(STRING_SEARCHWORD), text: "a".to_string() });

    let mut typing = session("x");
    assert_eq!(typing.key(&down(KeyCode::Enter, None)), Typing::Confirmed);
    assert_eq!(typing.confirm().text, "x", "confirming with nothing typed writes what the text showed");
    assert_eq!(session("x").key(&down(KeyCode::Escape, None)), Typing::Cancelled);
}

#[test]
fn a_letter_typed_with_the_paste_modifier_down_is_not_typed() {
    let mut typing = session("");
    typing.key(&down(KeyCode::ControlLeft, None));
    typing.key(&down(KeyCode::KeyA, Some("a")));
    assert_eq!(typing.entry().typed, "", "a letter under the modifier is a shortcut");
    typing.key(&up(KeyCode::ControlLeft));
    typing.key(&down(KeyCode::KeyA, Some("a")));
    assert_eq!(typing.entry().typed, "a", "and a letter is a letter again once it is up");

    typing.key(&down(KeyCode::SuperLeft, None));
    typing.key(&down(KeyCode::KeyB, Some("b")));
    typing.key(&up(KeyCode::SuperLeft));
    assert_eq!(typing.entry().typed, "a", "the platform key of a Mac does the same");
}

#[test]
fn what_an_input_method_composes_is_shown_and_never_confirmed_until_it_commits() {
    let mut typing = session("ab");
    typing.apply_ime(&Ime::Enabled);
    typing.apply_ime(&Ime::Preedit("한".to_string(), Some((0, "한".len()))));
    let entry = typing.entry();
    assert_eq!((entry.typed, entry.caret), ("ab", 2), "the line itself does not hold the composition");
    let composing = entry.composing.expect("a composition is shown");
    assert_eq!((composing.text, composing.caret), ("한", Some(0)), "the input method's caret is counted in characters from the start of it");

    typing.apply_ime(&Ime::Preedit("한글".to_string(), Some(("한".len(), "한글".len()))));
    assert_eq!(typing.entry().composing.expect("still composing").caret, Some(1));
    typing.apply_ime(&Ime::Preedit("한".to_string(), None));
    assert_eq!(typing.entry().composing.expect("still composing").caret, None, "an input method that wants no caret says so");

    typing.apply_ime(&Ime::Preedit(String::new(), None));
    typing.apply_ime(&Ime::Commit("한".to_string()));
    assert!(typing.entry().composing.is_none());
    assert_eq!((typing.entry().typed, typing.entry().caret), ("ab한", 3));

    typing.apply_ime(&Ime::Preedit("あ".to_string(), None));
    assert_eq!(typing.key(&down(KeyCode::Enter, None)), Typing::Confirmed);
    assert_eq!(typing.confirm().text, "ab한", "a syllable that was still being composed is not written");

    let mut typing = session("");
    typing.apply_ime(&Ime::Preedit("あ".to_string(), None));
    typing.apply_ime(&Ime::Disabled);
    assert!(typing.entry().composing.is_none(), "an input method that goes away takes its composition with it");
}
