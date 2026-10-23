//! The settings screen as a whole: its rail, its sections, the row kinds and what sets each, the
//! mark of a row that is not what it ships as and the keys that put rows back, the list when it is
//! longer than the screen, and what the panel and the line of keys say of the row under the cursor.
//!
//! With [`CAPTURE_DIR_ENV`] naming a folder, every group is also saved there at the size the screen
//! is laid out for and at full HD, with the states a still frame cannot be reached without: a
//! slider being dragged, a number and a line of text being typed, rows that are not what they ship
//! as, the question about a group, a row a skin does not read, and a list longer than the screen.

use rbms_config::{Config, cycle_values, descriptor, display_value, number_spec, number_steps};
use rbms_render::text_width;

use super::number_tests::{DRAWN_SIZES, Screen, along, bare_app, middle};
use super::*;
use crate::settings_view::{confirm_no_rect, row_back, row_forward, row_open, row_segment, row_switch};
use crate::stage::HeadlessCanvas;
use crate::stage::preview_menu::MARGIN;

/// Environment variable naming the folder captures are saved in. Nothing is saved without it.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// How many times over a group's rows are listed to make a list longer than the screen.
const LIST_COPIES: usize = 3;

/// The longest name a key has, which is what the line of keys has to have room for three of.
const LONGEST_KEY: &str = "BACKSLASH";

/// How far the wheel is turned in the tests that scroll with it.
const WHEEL_LINES: f32 = 3.0;

/// A judge width over what the chart asks for, and a volume that is not what it ships as.
const WIDENED_PERCENT: i64 = 120;
const QUIETER_PERCENT: i64 = 30;

fn ctx(app: &mut App) -> FrameCtx<'_> {
    FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }
}

/// The screen on one group over an app of this test's own, at the size it is laid out for. It is
/// built without asking the host which output devices it has, which the screen does on the way in
/// and these tests do not need: a test that steps the device row names the devices itself.
fn screen_on(tag: &str, tab: SettingTab) -> Screen {
    let app = bare_app(tag);
    let mut state = SettingsState::on_tab(tab);
    state.refresh(&app.shared);
    let mut screen = Screen { app, state, pixels: HeadlessCanvas::new(CW, CH), size: (CW, CH) };
    screen.frame();
    screen
}

/// What a row reads in the configuration, as the table of rows says it.
fn value_of(screen: &Screen, id: SettingId) -> String {
    screen.app.shared.setting_line(id).1
}

/// One row as the screen draws it, read afresh.
fn line_of(screen: &mut Screen, id: SettingId) -> SettingsLine {
    screen.state.dirty = true;
    screen.state.ensure_rows(&screen.app.shared);
    screen.state.lines[screen.at(id)].clone()
}

/// How many rows the rail says are not what they ship as, for the group on show.
fn changed_on_the_rail(screen: &mut Screen) -> usize {
    let scene = screen.scene();
    scene.groups[scene.group].changed
}

/// Every row of the screen the zero key puts back, with the group it is in.
fn resettable_rows() -> Vec<(SettingTab, SettingId)> {
    let config = Config::default();
    let rows = SettingTab::SHOWN.into_iter().flat_map(|tab| rbms_config::tab_rows(tab, &config).into_iter().map(move |id| (tab, id)));
    rows.filter(|(_, id)| is_resettable(*id)).collect()
}

/// Make one row something it does not ship as, without opening a picker: a number is put at an end
/// of its range, the font and the folders of the system sounds and of the downloads are named in
/// the configuration, the output device is stepped onto one the host reported, and any other row is
/// stepped forward once.
fn change(screen: &mut Screen, id: SettingId) {
    if let Some(spec) = number_spec(id) {
        let shipped = screen.steps_of(id);
        screen.app.shared.set_row_number(SettingRow::Fixed(id), if shipped == spec.max { spec.min } else { spec.max });
    } else if id == SettingId::Font {
        screen.app.shared.config.display.font_path = Some("/fonts/a-face-of-the-test.ttf".to_string());
    } else if id == SettingId::SoundFolder {
        screen.app.shared.config.audio.sound_folder = Some("/sounds/a-set-of-the-test".to_string());
    } else if id == SettingId::DownloadFolder {
        screen.app.shared.config.library.download_folder = Some("/downloads/a-folder-of-the-test".to_string());
    } else {
        screen.focus(id);
        screen.key(KeyCode::ArrowRight);
    }
    screen.state.dirty = true;
    screen.frame();
}

/// List the rows of the open group several times over, so the list is longer than the screen. The
/// rows are the same rows, which is all the keys that move the cursor and the wheel need.
fn lengthen(screen: &mut Screen) {
    let (entries, rows, lines) = (screen.state.list.entries.clone(), screen.state.rows.clone(), screen.state.lines.clone());
    for copy in 1..LIST_COPIES {
        screen.state.list.entries.extend(entries.iter().map(|entry| match entry {
            ListItem::Row(at) => ListItem::Row(at + copy * rows.len()),
            heading => *heading,
        }));
        screen.state.rows.extend(rows.iter().copied());
        screen.state.lines.extend(lines.iter().cloned());
    }
    assert!(screen.state.list.entries.len() > visible_rows(), "the fixture needs a list longer than the screen");
}

/// Whether the row under the cursor is among the entries on show.
fn cursor_is_on_show(screen: &Screen) -> bool {
    let at = screen.state.list.entries.iter().position(|entry| *entry == ListItem::Row(screen.state.sel)).expect("the cursor is on a row of the list");
    (screen.state.list.top..screen.state.list.top + visible_rows()).contains(&at)
}

/// Every group lists exactly the rows the table gives it, under the headings of its sections and in
/// their order. As the screen ships every group is on show whole but NETWORK, which the DISCORD
/// section made two entries longer than the screen, and DISPLAY, which the LANGUAGE section did: each
/// draws the rows on show and scrolls to the rest.
#[test]
fn every_group_lists_its_rows_under_the_headings_of_its_sections() {
    for tab in SettingTab::SHOWN {
        let screen = screen_on("layout-sections", tab);
        let listed = screen.app.shared.settings_rows(tab);
        assert_eq!(screen.state.rows.len(), listed.len(), "{tab:?}");
        assert!(listed.iter().all(|row| screen.state.rows.contains(row)), "{tab:?} lost a row");
        let headings: Vec<&str> =
            screen.state.list.entries.iter().filter_map(|entry| if let ListItem::Heading(name) = entry { Some(*name) } else { None }).collect();
        assert_eq!(headings, catalog::sections(tab).iter().map(|section| section.heading).collect::<Vec<_>>(), "{tab:?}");
        let in_order: Vec<SettingRow> = catalog::sections(tab).iter().flat_map(|section| section.rows.iter().map(|id| SettingRow::Fixed(*id))).collect();
        assert_eq!(screen.state.rows, in_order, "{tab:?}");
        assert!(matches!(screen.state.list.entries.first(), Some(ListItem::Heading(_))), "{tab:?} does not begin with a heading");
        assert_eq!(
            screen.state.list.entries.len() > visible_rows(),
            matches!(tab, SettingTab::Network | SettingTab::Display),
            "{tab:?} has {} entries and the screen shows {}",
            screen.state.list.entries.len(),
            visible_rows()
        );
        let on_show = screen.state.list.entries.iter().take(visible_rows()).filter(|entry| matches!(entry, ListItem::Row(_))).count();
        let drawn = screen.app.shared.hot.iter().filter(|(_, hot)| matches!(hot, Hot::SettingRow(_))).count();
        assert_eq!(drawn, on_show, "{tab:?} does not draw every row it has on show");
    }
}

/// The rail names every group with the count of its rows, a press on a group opens it with the
/// cursor on its first row, and a group with a row that is not what it ships as says how many.
#[test]
fn the_rail_opens_a_group_and_counts_what_was_changed_in_it() {
    let mut screen = screen_on("layout-rail", SettingTab::Judge);
    let scene = screen.scene();
    assert_eq!(scene.groups.iter().map(|group| group.name).collect::<Vec<_>>(), SettingTab::SHOWN.map(SettingTab::label).to_vec());
    for (group, tab) in scene.groups.iter().zip(SettingTab::SHOWN) {
        assert_eq!((group.rows, group.changed), (screen.app.shared.settings_rows(tab).len(), 0), "{tab:?}");
    }
    assert_eq!(scene.summary, catalog::group_summary(SettingTab::Judge));

    screen.focus(SettingId::LnMode);
    let audio = SettingTab::SHOWN.iter().position(|tab| *tab == SettingTab::Audio).expect("the screen shows the AUDIO group");
    let on_audio = middle(crate::number_row::hot_rect(&screen.app.shared, Hot::SettingTab(audio)).expect("the rail draws the AUDIO group"));
    screen.press(on_audio, MouseButton::Left);
    screen.frame();
    assert_eq!((screen.state.current_tab(), screen.state.sel, screen.state.list.top), (SettingTab::Audio, 0, 0));
    assert_eq!(screen.scene().group, audio);

    rbms_config::set_number_steps(&mut screen.app.shared.config, SettingId::KeyVolume, QUIETER_PERCENT);
    change(&mut screen, SettingId::GuideSe);
    assert_eq!(changed_on_the_rail(&mut screen), 2);
    assert_eq!(screen.scene().groups.iter().filter(|group| group.changed > 0).count(), 1, "a change in one group is counted in another");
    assert!(line_of(&mut screen, SettingId::KeyVolume).changed && line_of(&mut screen, SettingId::GuideSe).changed);
    assert!(!line_of(&mut screen, SettingId::BgmVolume).changed);
}

/// A switch is turned over by either arrow and by a press on the switch or its word, and by nothing
/// else on its row.
#[test]
fn a_switch_is_turned_over_by_an_arrow_and_by_a_press_on_it() {
    let mut screen = screen_on("layout-switch", SettingTab::Display);
    let id = SettingId::Bga;
    assert!(matches!(line_of(&mut screen, id).control, RowControl::Switch { on: true }));
    screen.focus(id);
    screen.key(KeyCode::ArrowRight);
    assert!(!screen.app.shared.config.display.bga);
    assert!(matches!(line_of(&mut screen, id).control, RowControl::Switch { on: false }));
    screen.key(KeyCode::ArrowLeft);
    assert!(screen.app.shared.config.display.bga);

    screen.frame();
    screen.focus(SettingId::DebugMode);
    let row = screen.row_of(id);
    screen.press(middle(row_switch(row)), MouseButton::Left);
    assert_eq!((screen.app.shared.config.display.bga, screen.state.sel), (false, screen.at(id)), "a press on the switch");
    screen.press((row_switch(row).x - 1.0, row.y + row.h * 0.5), MouseButton::Left);
    assert!(!screen.app.shared.config.display.bga, "a press beside the switch turned it over");
}

/// A choice drawn with all its values on show takes the value a press lands on, whichever it stood
/// on, and is stepped by the arrows and stepped back by the right button like any other row.
#[test]
fn a_choice_with_its_values_on_show_takes_the_value_that_is_pressed() {
    let mut screen = screen_on("layout-segments", SettingTab::Judge);
    let id = SettingId::JudgeAlgorithm;
    let values = cycle_values(id);
    for from in 0..values.len() {
        for to in 0..values.len() {
            let row = screen.row_of(id);
            screen.press(middle(row_segment(row, from, values.len())), MouseButton::Left);
            assert_eq!(value_of(&screen, id), values[from]);
            screen.press(middle(row_segment(row, to, values.len())), MouseButton::Left);
            assert_eq!((value_of(&screen, id).as_str(), screen.state.sel), (values[to], screen.at(id)), "from {} to {}", values[from], values[to]);
            assert_eq!(line_of(&mut screen, id).control, RowControl::Segments { values, chosen: Some(to) });
        }
    }
    screen.focus(id);
    let standing = value_of(&screen, id);
    screen.key(KeyCode::ArrowRight);
    assert_ne!(value_of(&screen, id), standing, "the right arrow did not step the choice");
    screen.frame();
    screen.press(screen.on_the_label_of(id), MouseButton::Right);
    assert_eq!(value_of(&screen, id), standing, "the right button did not step the choice back");
    screen.press(screen.on_the_label_of(id), MouseButton::Left);
    assert_eq!(value_of(&screen, id), standing, "a press on the label of a choice stepped it");
}

/// The layout of the built-in note field is named by the value pressed even while the run stands on
/// a layout file of its own, which is none of the values on show: the press lets go of that file.
#[test]
fn a_pressed_layout_lets_go_of_the_file_the_run_was_started_with() {
    let mut screen = screen_on("layout-skin", SettingTab::Display);
    let id = SettingId::Skin;
    screen.app.shared.launch.skin_path = Some("custom.ron".to_string());
    let line = line_of(&mut screen, id);
    assert_eq!((line.value.as_str(), line.changed), (rbms_config::CUSTOM_VALUE, true));
    assert!(matches!(line.control, RowControl::Segments { chosen: None, .. }), "a layout that is none of the values is drawn as one of them");
    screen.frame();
    let values = cycle_values(id);
    for place in (0..values.len()).rev() {
        screen.press(middle(row_segment(screen.row_of(id), place, values.len())), MouseButton::Left);
        assert_eq!((value_of(&screen, id).as_str(), screen.app.shared.launch.skin_path.is_none()), (values[place], true));
    }
    assert!(!line_of(&mut screen, id).changed);

    screen.app.shared.launch.skin_path = Some("custom.ron".to_string());
    screen.focus(id);
    screen.key(KeyCode::Digit0);
    assert_eq!((value_of(&screen, id), screen.app.shared.launch.skin_path.is_none()), (display_value(&Config::default(), id), true), "the zero key");
}

/// A choice of many values is stepped by a press on either of its arrows and forward by a press on
/// its value, and the output device steps through what the host reported.
#[test]
fn a_choice_of_many_is_stepped_by_its_arrows() {
    let mut screen = screen_on("layout-stepper", SettingTab::Judge);
    let id = SettingId::GaugeAutoShift;
    let values = cycle_values(id);
    assert_eq!(line_of(&mut screen, id).control, RowControl::Stepper);
    screen.frame();
    let row = screen.row_of(id);
    screen.press(middle(row_back(row)), MouseButton::Left);
    assert_eq!(value_of(&screen, id), values[values.len() - 1], "the arrow back does not go round to the last value");
    screen.press(middle(row_forward(row)), MouseButton::Left);
    screen.press((row.x + row.w - 1.0, row.y + 1.0), MouseButton::Left);
    assert_eq!((value_of(&screen, id).as_str(), screen.state.sel), (values[1], screen.at(id)), "the arrow forward and the value before it");

    let mut audio = screen_on("layout-device", SettingTab::Audio);
    audio.state.audio_devices = vec!["Studio Monitors".to_string(), "Headphones".to_string()];
    let device = SettingId::AudioDevice;
    audio.frame();
    audio.press(middle(row_forward(audio.row_of(device))), MouseButton::Left);
    assert_eq!(value_of(&audio, device), "Studio Monitors");
    assert!(audio.app.shared.config.audio.reopen_pending() && line_of(&mut audio, device).changed);
    audio.app.shared.config.audio.clear_reopen_pending();
    audio.frame();
    audio.press(middle(row_back(audio.row_of(device))), MouseButton::Left);
    assert_eq!((value_of(&audio, device).as_str(), audio.app.shared.config.audio.device.is_none()), (rbms_config::DEFAULT_VALUE, true));
    audio.press(middle(row_back(audio.row_of(device))), MouseButton::Left);
    assert_eq!(value_of(&audio, device), "Headphones");
    audio.app.shared.config.audio.clear_reopen_pending();
    audio.focus(device);
    audio.key(KeyCode::Digit0);
    assert_eq!(audio.app.shared.config.audio.device, None, "the zero key did not put the device back to the system's");
    assert!(audio.app.shared.config.audio.reopen_pending(), "the output was not asked to open again on the device it was put back to");
}

/// A press on a field opens it to be typed into, on that row and no other, and Enter stores what
/// was typed; a press on the mark of a row that opens a list opens the list, and one on the mark of
/// the row of the key editor opens the editor.
#[test]
fn a_press_on_a_field_or_a_mark_opens_or_runs_the_row() {
    let mut screen = screen_on("layout-open", SettingTab::Network);
    let email = SettingId::Email;
    screen.press(screen.on_the_label_of(email), MouseButton::Left);
    assert!(screen.state.text_input.is_none(), "a press on the label of a field opened it");
    screen.press(middle(row_open(screen.row_of(email))), MouseButton::Left);
    assert_eq!(screen.state.text_edit_row, Some(email));
    assert_eq!(screen.scene().editor.map(|(at, _)| at), Some(screen.at(email)));
    assert_eq!(screen.scene().hint, info::text_edit_hint());
    for text in ["d", "j", "@", "x", ".", "t"] {
        screen.typed(text);
    }
    screen.key(KeyCode::Enter);
    assert_eq!(screen.app.shared.config.network.ir_email.as_deref(), Some("dj@x.t"));
    assert_eq!(line_of(&mut screen, email).control, RowControl::Field { filled: true });
    assert!(!line_of(&mut screen, email).changed, "a line of text is marked as something the zero key puts back");
    screen.focus(email);
    screen.key(KeyCode::Digit0);
    assert_eq!(screen.app.shared.config.network.ir_email.as_deref(), Some("dj@x.t"), "the zero key wiped a line of text");

    screen.frame();
    screen.press(middle(row_open(screen.row_of(SettingId::Rivals))), MouseButton::Left);
    assert!(screen.state.rivals_open, "a press on the mark of the RIVALS row did not open the list");
    assert_eq!(screen.scene().hint, rivals_hint());

    let mut input = screen_on("layout-open-keys", SettingTab::Input);
    let on_the_mark = middle(row_open(input.row_of(SettingId::KeyConfig)));
    assert!(matches!(input.press(input.on_the_label_of(SettingId::KeyConfig), MouseButton::Left), Transition::Stay), "a press on the label opened the editor");
    assert!(matches!(input.press(on_the_mark, MouseButton::Right), Transition::Stay), "the right button opened the editor");
    assert!(matches!(input.press(on_the_mark, MouseButton::Left), Transition::Open(Stage::KeyConfig(_))));
}

/// The font and the folder of the system sounds are drawn as the path that was chosen, and are
/// marked while one is.
#[test]
fn a_chosen_file_or_folder_is_what_its_row_reads() {
    let mut screen = screen_on("layout-path", SettingTab::Display);
    assert_eq!(line_of(&mut screen, SettingId::Font).control, RowControl::Path { chosen: false });
    change(&mut screen, SettingId::Font);
    let font = line_of(&mut screen, SettingId::Font);
    assert_eq!((font.value.as_str(), font.control, font.changed), ("/fonts/a-face-of-the-test.ttf", RowControl::Path { chosen: true }, true));
    assert_eq!(value_of(&screen, SettingId::Font), rbms_config::CUSTOM_VALUE, "what the table says the row holds changed");

    let mut audio = screen_on("layout-path-sounds", SettingTab::Audio);
    change(&mut audio, SettingId::SoundFolder);
    let folder = line_of(&mut audio, SettingId::SoundFolder);
    assert_eq!((folder.value.as_str(), folder.control, folder.changed), ("/sounds/a-set-of-the-test", RowControl::Path { chosen: true }, true));
}

/// Every row the zero key puts back is marked once it is not what it ships as, is counted on the
/// rail, and reads exactly what a fresh configuration reads after the key -- and is then marked no
/// more. The key leaves a row that is already what it ships as alone.
#[test]
fn the_zero_key_puts_every_row_back_and_the_mark_goes_with_it() {
    let shipped = Config::default();
    let rows = resettable_rows();
    assert!(rows.len() > SettingTab::SHOWN.len(), "the fixture lost its rows");
    for (tab, id) in rows {
        let mut screen = screen_on("layout-zero", tab);
        screen.state.audio_devices = vec!["Studio Monitors".to_string()];
        screen.focus(id);
        assert!(matches!(screen.key(KeyCode::Digit0), Transition::Stay));
        assert_eq!(screen.app.shared.config, shipped, "{id:?}: the zero key moved a row that is what it ships as");

        change(&mut screen, id);
        assert_ne!(value_of(&screen, id), display_value(&shipped, id), "{id:?}: the fixture did not change the row");
        assert!(line_of(&mut screen, id).changed, "{id:?} is not marked");
        assert_eq!(changed_on_the_rail(&mut screen), 1, "{id:?} is not counted on the rail");

        screen.focus(id);
        assert!(matches!(screen.key(if number_spec(id).is_some() { KeyCode::Numpad0 } else { KeyCode::Digit0 }), Transition::Stay), "{id:?}");
        assert_eq!(value_of(&screen, id), display_value(&shipped, id), "{id:?}: the zero key did not put the row back");
        assert_eq!(number_steps(&screen.app.shared.config, id), number_steps(&shipped, id), "{id:?}");
        assert!(!line_of(&mut screen, id).changed, "{id:?} is still marked");
        assert_eq!(changed_on_the_rail(&mut screen), 0, "{id:?}");
    }
}

/// The zero key with Shift held, and the button under the groups, ask before every row of the open
/// group is put back, and ask nothing while no row of it would move. Escape and a press anywhere
/// but on the first button keep the group as it is; Enter and a press on that button put it back
/// and write the settings file. Another group is left as it was either way, and while the question
/// is up no other key reaches the rows.
#[test]
fn putting_a_whole_group_back_is_asked_about_first() {
    let mut screen = screen_on("layout-group", SettingTab::Audio);
    let settings = screen.app.shared.settings_path.clone();
    let on_the_button = middle(reset_group_rect(SettingTab::SHOWN.len()));
    screen.shifted(KeyCode::Digit0);
    screen.press(on_the_button, MouseButton::Left);
    assert!(screen.scene().confirm.is_none(), "the screen asks about a group in which nothing was changed");

    screen.app.shared.config.display.bga = false;
    let change_the_group = |screen: &mut Screen| {
        rbms_config::set_number_steps(&mut screen.app.shared.config, SettingId::KeyVolume, QUIETER_PERCENT);
        rbms_config::set_number_steps(&mut screen.app.shared.config, SettingId::AudioPolyphony, rbms_config::AUDIO_POLYPHONY_MAX_VOICES as i64);
        change(screen, SettingId::GuideSe);
        change(screen, SettingId::AudioBuffer);
        change(screen, SettingId::SoundFolder);
        screen.focus(SettingId::BgmVolume);
    };
    change_the_group(&mut screen);
    let changed = screen.app.shared.config.clone();
    screen.shifted(KeyCode::Digit0);
    let asked = screen.scene().confirm.expect("the screen did not ask");
    assert!(asked.title.contains(SettingTab::Audio.label()) && asked.body.starts_with("5 CHANGED ROWS GO ") && !asked.body.contains("TEXT"), "{asked:?}");
    assert_eq!(screen.scene().hint, confirm_hint());
    for code in [KeyCode::ArrowDown, KeyCode::ArrowRight, KeyCode::Tab, KeyCode::Digit0, KeyCode::F8, KeyCode::F12, KeyCode::F5] {
        assert!(matches!(screen.key(code), Transition::Stay), "{code:?} left the screen from under the question");
    }
    assert_eq!((screen.state.sel, screen.state.current_tab()), (screen.at(SettingId::BgmVolume), SettingTab::Audio), "a key reached the rows");
    assert_eq!(screen.app.shared.config, changed);
    assert!(matches!(screen.key(KeyCode::Escape), Transition::Stay), "Escape left the screen instead of the question");
    assert!(screen.scene().confirm.is_none());
    assert_eq!(screen.app.shared.config, changed, "Escape put the group back");
    assert!(!settings.exists(), "the settings file was written with nothing put back");

    screen.shifted(KeyCode::Numpad0);
    assert!(matches!(screen.key(KeyCode::Enter), Transition::Stay), "Enter left the screen instead of answering");
    assert!(screen.scene().confirm.is_none());
    let mut expected = Config::default();
    expected.display.bga = false;
    expected.audio.mark_reopen_pending();
    assert_eq!(screen.app.shared.config, expected, "the group was not put back, or another group was");
    assert!(screen.app.shared.config.audio.reopen_pending(), "the output was not asked to open again with the parameters it was put back to");
    assert!(!screen.app.shared.syssound.guide_enabled());
    assert_eq!(changed_on_the_rail(&mut screen), 0);
    let written = rbms_config::load(&settings).expect("the file the answer wrote reads back").config;
    assert_eq!((written.audio.key, written.audio.guide_se, written.display.bga), (expected.audio.key, false, false));

    change_the_group(&mut screen);
    let changed = screen.app.shared.config.clone();
    screen.press(on_the_button, MouseButton::Left);
    assert!(screen.scene().confirm.is_some(), "the button under the groups did not ask");
    screen.press(middle(confirm_no_rect()), MouseButton::Left);
    assert!(screen.scene().confirm.is_none());
    assert_eq!(screen.app.shared.config, changed, "the second button put the group back");
    screen.press(on_the_button, MouseButton::Left);
    screen.press(middle(screen.row_of(SettingId::MasterVolume)), MouseButton::Left);
    assert!(screen.scene().confirm.is_none(), "a press off the question left it up");
    assert_eq!(screen.app.shared.config, changed, "a press off the question reached the row under it");
    screen.press(on_the_button, MouseButton::Left);
    screen.press(middle(confirm_yes_rect()), MouseButton::Left);
    assert!(screen.scene().confirm.is_none());
    assert_eq!(screen.app.shared.config, expected, "the first button did not put the group back");
}

/// In a list longer than the screen the cursor is always on show after a key moved it, the heading
/// of a section comes back with its first row, the wheel moves the list and not the cursor and only
/// while the pointer is over the rows, and the list stops at both its ends.
#[test]
fn a_list_longer_than_the_screen_follows_the_cursor_and_the_wheel() {
    let mut screen = screen_on("layout-scroll", SettingTab::Judge);
    lengthen(&mut screen);
    let (rows, entries) = (screen.state.rows.len(), screen.state.list.entries.len());
    let last_top = entries - visible_rows();
    for _ in 0..rows * 2 {
        screen.key(KeyCode::ArrowDown);
        assert!(cursor_is_on_show(&screen), "the cursor went off the foot of the list at row {}", screen.state.sel);
    }
    assert_eq!((screen.state.sel, screen.state.list.top), (rows - 1, last_top), "the cursor does not stop on the last row");
    for _ in 0..rows * 2 {
        screen.key(KeyCode::ArrowUp);
        assert!(cursor_is_on_show(&screen), "the cursor went off the top of the list at row {}", screen.state.sel);
    }
    assert_eq!((screen.state.sel, screen.state.list.top), (0, 0), "the first row came back without the heading over it");

    screen.key(KeyCode::End);
    assert_eq!((screen.state.sel, cursor_is_on_show(&screen)), (rows - 1, true));
    screen.key(KeyCode::Home);
    assert_eq!((screen.state.sel, screen.state.list.top), (0, 0));

    let firsts: Vec<usize> = (0..rows).filter(|row| matches!(screen.state.list.entries.iter().position(|e| *e == ListItem::Row(*row)), Some(at) if matches!(screen.state.list.entries[at - 1], ListItem::Heading(_)))).collect();
    assert!(firsts.len() > LIST_COPIES);
    for first in &firsts[1..] {
        screen.key(KeyCode::PageDown);
        assert_eq!((screen.state.sel, cursor_is_on_show(&screen)), (*first, true), "Page Down does not land on the first row of the next section");
    }
    screen.key(KeyCode::PageDown);
    assert_eq!(screen.state.sel, firsts[firsts.len() - 1], "Page Down moved the cursor from the last section");
    screen.key(KeyCode::ArrowDown);
    screen.key(KeyCode::PageUp);
    assert_eq!(screen.state.sel, firsts[firsts.len() - 1], "Page Up from inside a section does not land on its first row");
    for first in firsts[..firsts.len() - 1].iter().rev() {
        screen.key(KeyCode::PageUp);
        assert_eq!(screen.state.sel, *first);
    }
    assert_eq!(screen.state.list.top, 0);

    screen.app.shared.cursor = middle(crate::settings_view::key_config_link());
    screen.state.handle_scroll(&mut ctx(&mut screen.app), WHEEL_LINES);
    assert_eq!(screen.state.list.top, 0, "the wheel scrolled the list from over the rail");
    screen.app.shared.cursor = middle(list_rect());
    screen.state.handle_scroll(&mut ctx(&mut screen.app), WHEEL_LINES);
    assert_eq!((screen.state.list.top, screen.state.sel), (WHEEL_LINES as usize, 0), "the wheel did not move the list alone");
    screen.state.handle_scroll(&mut ctx(&mut screen.app), f32::NAN);
    screen.state.handle_scroll(&mut ctx(&mut screen.app), entries as f32 * 2.0);
    assert_eq!(screen.state.list.top, last_top, "the wheel took the list past its end");
    screen.state.handle_scroll(&mut ctx(&mut screen.app), -(entries as f32) * 2.0);
    assert_eq!(screen.state.list.top, 0, "the wheel took the list past its top");
    screen.state.handle_scroll(&mut ctx(&mut screen.app), WHEEL_LINES);
    screen.key(KeyCode::ArrowDown);
    assert!(cursor_is_on_show(&screen), "a key did not bring the cursor back onto a list the wheel had moved");

    let mut short = screen_on("layout-no-scroll", SettingTab::Select);
    short.app.shared.cursor = middle(list_rect());
    short.state.handle_scroll(&mut ctx(&mut short.app), WHEEL_LINES);
    assert_eq!(short.state.list.top, 0, "a list that fits was scrolled");
}

/// The line of keys names the keys the row under the cursor hears before the keys of the screen,
/// for every row of every group, and Enter does on each row what the line says it does. The line
/// fits the screen with the three keys to the other screens moved to the longest names a key has.
#[test]
fn the_line_of_keys_says_what_the_row_under_the_cursor_hears() {
    rbms_render::font::use_embedded_fonts_only();
    let longest = [LONGEST_KEY.to_string(), LONGEST_KEY.to_string(), LONGEST_KEY.to_string()];
    for tab in SettingTab::SHOWN {
        let mut screen = screen_on("layout-hint", tab);
        for at in 0..screen.state.rows.len() {
            let SettingRow::Fixed(id) = screen.state.rows[at] else {
                panic!("{tab:?} lists a row that is no row of the table");
            };
            screen.focus(id);
            let scene = screen.scene();
            let keys = info::row_keys(id, scene.rows[at].control);
            assert!(keys.len() <= crate::settings_view::INFO_KEY_LINES, "{id:?} hears more keys than the panel has lines for");
            let own: String = keys.iter().map(|(key, action)| format!("{key} {action}   ")).collect();
            assert!(scene.hint.starts_with(&own), "{id:?}: {}", scene.hint);
            assert!(
                scene.hint.ends_with("UP DOWN MOVE   PGUP PGDN SECTION   TAB GROUP   SHIFT+0 RESET GROUP   F8 PLAY   F12 SKINS   F5 KEYS   ESC SAVE"),
                "{id:?}"
            );
            let panel = scene.info.expect("the panel says nothing of the row under the cursor");
            assert_eq!(panel.keys.iter().map(|(key, action)| (key.as_str(), *action)).collect::<Vec<_>>(), keys, "{id:?}: the panel and the line disagree");
            let widest = text_width(&info::hint(&keys, &longest), crate::stage::preview_menu::HINT_SCALE);
            assert!(widest <= CW as f32 - MARGIN * 2.0, "{id:?}: the line of keys is {widest} wide with long keys");

            let enter = keys.iter().find(|(key, _)| *key == "ENTER").map(|(_, action)| *action);
            if id == SettingId::Font {
                assert_eq!(enter, Some("CHOOSE A FILE"));
                continue;
            }
            if matches!(id, SettingId::SoundFolder | SettingId::DownloadFolder) {
                assert!(keys.contains(&("ENTER RIGHT", "CHOOSE A FOLDER")), "{keys:?}");
                continue;
            }
            let before = value_of(&screen, id);
            let asked = screen.key(KeyCode::Enter);
            match enter {
                Some("NEXT") => assert!(matches!(asked, Transition::Stay) && (value_of(&screen, id) != before || id == SettingId::AudioDevice), "{id:?}"),
                Some("TYPE") => assert!(matches!(asked, Transition::Stay) && screen.scene().typed.is_some(), "{id:?}"),
                Some("EDIT") => assert!(matches!(asked, Transition::Stay) && screen.state.text_edit_row == Some(id), "{id:?}"),
                Some("SWITCH") => assert!(matches!(asked, Transition::Stay) && value_of(&screen, id) != before, "{id:?}"),
                Some("OPEN THE LIST") => assert!(matches!(asked, Transition::Stay) && (screen.state.rivals_open || screen.state.profiles_open), "{id:?}"),
                Some("OPEN") => assert!(matches!(asked, Transition::Open(Stage::KeyConfig(_))), "{id:?}"),
                Some("RUN" | "SHOW ITS STATE") => assert!(matches!(asked, Transition::Stay) && value_of(&screen, id) == before, "{id:?}"),
                other => panic!("{id:?}: the line says Enter does {other:?}, which this test does not know how to check"),
            }
            screen = screen_on("layout-hint", tab);
        }
    }
}

/// The panel beside the list says something of every row: what it is for, what kind of row it is,
/// what it holds and ships as, the ends and the step of a number and the values of a choice.
#[test]
fn the_panel_describes_every_row() {
    for tab in SettingTab::SHOWN {
        let mut screen = screen_on("layout-info", tab);
        screen.state.audio_devices = vec!["Studio Monitors".to_string()];
        for at in 0..screen.state.rows.len() {
            let SettingRow::Fixed(id) = screen.state.rows[at] else {
                panic!("{tab:?} lists a row that is no row of the table");
            };
            screen.focus(id);
            let panel = screen.scene().info.expect("the panel says nothing of the row under the cursor");
            assert_eq!(panel.label, descriptor(id).label);
            assert_eq!(panel.description, catalog::description(id));
            assert!(!panel.kind.is_empty() && !panel.description.is_empty(), "{id:?}");
            assert_eq!((panel.remark, panel.caution), (None, None), "{id:?} is cautioned about as it ships");
            let fact = |name: &str| panel.facts.iter().find(|(fact, _)| *fact == name).map(|(_, value)| value.clone());
            assert_eq!(fact("DEFAULT").is_some(), is_resettable(id), "{id:?}");
            assert_eq!(fact("RANGE").is_some() && fact("STEP").is_some(), number_spec(id).is_some(), "{id:?}");
            assert_eq!(fact("CHOICES").is_some(), !cycle_values(id).is_empty(), "{id:?}");
            if is_resettable(id) {
                assert_eq!(fact("DEFAULT"), Some(display_value(&Config::default(), id)), "{id:?}");
            }
        }
    }
    let mut audio = screen_on("layout-info-facts", SettingTab::Audio);
    audio.state.audio_devices = vec!["Studio Monitors".to_string()];
    rbms_config::set_number_steps(&mut audio.app.shared.config, SettingId::KeyVolume, QUIETER_PERCENT);
    audio.state.dirty = true;
    audio.focus(SettingId::KeyVolume);
    let panel = audio.scene().info.expect("the panel is drawn");
    assert_eq!(panel.kind, "NUMBER");
    assert_eq!(
        panel.facts,
        vec![
            ("VALUE", "30%".to_string()),
            ("DEFAULT", "50%  (0 PUTS IT BACK)".to_string()),
            ("RANGE", "0% TO 100%".to_string()),
            ("STEP", "1%, 10% WITH SHIFT".to_string())
        ]
    );
    audio.focus(SettingId::AudioDevice);
    let panel = audio.scene().info.expect("the panel is drawn");
    assert_eq!(panel.kind, "CHOICE  1 OF 2");
    assert!(panel.facts.contains(&("CHOICES", "DEFAULT, Studio Monitors".to_string())), "{:?}", panel.facts);
}

/// A judge window or a long-note margin over what the chart asks for is cautioned about on the
/// panel, since a run played with it sets no best score; one at or under it is not. A row a skin does not
/// read has what its note means written out.
#[test]
fn the_panel_cautions_about_a_widened_window_and_explains_a_note() {
    let mut screen = screen_on("layout-caution", SettingTab::Judge);
    for id in [SettingId::JudgeWidthKeyPGreat, SettingId::JudgeWidthScratchGood, SettingId::LongNoteMargin] {
        for (percent, cautioned) in [(WIDENED_PERCENT, true), (100, false), (80, false)] {
            rbms_config::set_number_steps(&mut screen.app.shared.config, id, percent);
            screen.state.dirty = true;
            screen.focus(id);
            let panel = screen.scene().info.expect("the panel is drawn");
            assert_eq!(panel.caution.is_some(), cautioned, "{id:?} at {percent}%");
        }
    }
    screen.focus(SettingId::Total);
    assert_eq!(screen.scene().info.and_then(|panel| panel.caution), None);

    let mut display = screen_on("layout-remark", SettingTab::Display);
    display.app.shared.config.skin.select(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS, Some("/skins/mine/play7.luaskin".to_string()));
    display.state.dirty = true;
    display.focus(SettingId::JudgeTextY);
    let panel = display.scene().info.expect("the panel is drawn");
    assert!(panel.remark.is_some_and(|said| said.contains("built-in screen")), "{:?}", panel.remark);
    display.focus(SettingId::Bga);
    assert_eq!(display.scene().info.and_then(|panel| panel.remark), None);
}

/// While a number or a line of text is typed, a list is drawn over the rows or the question about a
/// group is up, the panel lists the keys that hear then and writes what is being typed as the
/// value, a secret masked as the row draws it; once that is done the row's own keys come back.
#[test]
fn the_panel_follows_what_has_the_keys() {
    let keys_of = |screen: &mut Screen| {
        screen.scene().info.expect("the panel is drawn").keys.into_iter().map(|(key, action)| (key, action.to_string())).collect::<Vec<_>>()
    };
    let listed = |keys: &[(&str, &str)]| keys.iter().map(|(key, action)| ((*key).to_string(), (*action).to_string())).collect::<Vec<_>>();
    let value_of =
        |screen: &mut Screen| screen.scene().info.and_then(|panel| panel.facts.into_iter().find(|(name, _)| *name == "VALUE").map(|(_, value)| value));

    let mut audio = screen_on("layout-busy-number", SettingTab::Audio);
    audio.focus(SettingId::BgmVolume);
    let own = keys_of(&mut audio);
    audio.key(KeyCode::Enter);
    audio.typed("4");
    audio.typed("5");
    assert_eq!(keys_of(&mut audio), listed(&info::number_edit_keys()));
    assert_eq!(value_of(&mut audio).as_deref(), Some("45"), "the panel does not show the number being typed");
    audio.key(KeyCode::Escape);
    assert_eq!(keys_of(&mut audio), own, "the row's own keys did not come back");

    let mut network = screen_on("layout-busy-text", SettingTab::Network);
    network.press(middle(row_open(network.row_of(SettingId::Email))), MouseButton::Left);
    network.typed("d");
    network.typed("j");
    assert_eq!(keys_of(&mut network), listed(&info::text_edit_keys()));
    assert!(value_of(&mut network).is_some_and(|value| value.starts_with("dj")), "{:?}", value_of(&mut network));
    network.key(KeyCode::Escape);
    network.press(middle(row_open(network.row_of(SettingId::Password))), MouseButton::Left);
    network.typed("s");
    network.typed("e");
    assert!(value_of(&mut network).is_some_and(|value| !value.contains("se")), "the panel shows a password: {:?}", value_of(&mut network));
    network.key(KeyCode::Escape);
    network.press(middle(row_open(network.row_of(SettingId::Rivals))), MouseButton::Left);
    assert_eq!(keys_of(&mut network), listed(&info::list_keys()));

    let mut judge = screen_on("layout-busy-question", SettingTab::Judge);
    change(&mut judge, SettingId::LnMode);
    judge.shifted(KeyCode::Digit0);
    assert_eq!(keys_of(&mut judge), listed(&info::confirm_keys()));
}

/// Every group of the screen and the states of it worth looking at, drawn and saved for a person to
/// open when a capture folder was named.
#[test]
fn every_group_and_state_of_the_screen_can_be_captured() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    for size in DRAWN_SIZES {
        let tag = |name: &str| format!("layout-capture-{name}-{}", size.0);
        for tab in SettingTab::SHOWN {
            let screen = Screen::sized(&tag(tab.label()), tab, size);
            screen.save(&format!("settings-group-{}", tab.label().to_ascii_lowercase()));
        }

        let mut judge = Screen::sized(&tag("changed"), SettingTab::Judge, size);
        rbms_config::set_number_steps(&mut judge.app.shared.config, SettingId::JudgeWidthKeyGreat, WIDENED_PERCENT);
        rbms_config::set_number_steps(&mut judge.app.shared.config, SettingId::Total, 300);
        change(&mut judge, SettingId::LnMode);
        change(&mut judge, SettingId::GaugeAutoShift);
        judge.focus(SettingId::JudgeWidthKeyGreat);
        judge.save("settings-state-changed");
        judge.shifted(KeyCode::Digit0);
        judge.frame();
        judge.save("settings-state-question");
        lengthen(&mut judge);
        judge.key(KeyCode::Escape);
        for _ in 0..judge.state.rows.len() / 2 {
            judge.key(KeyCode::ArrowDown);
        }
        judge.frame();
        judge.save("settings-state-long-list");

        let mut audio = Screen::sized(&tag("audio"), SettingTab::Audio, size);
        audio.app.shared.audio_failed = true;
        rbms_config::set_number_steps(&mut audio.app.shared.config, SettingId::KeyVolume, QUIETER_PERCENT);
        change(&mut audio, SettingId::SoundFolder);
        audio.focus(SettingId::BgmVolume);
        let slider = audio.slider_of(SettingId::BgmVolume);
        audio.press(along(slider, 0.5), MouseButton::Left);
        audio.drag(along(slider, 0.72));
        audio.frame();
        audio.save("settings-state-dragging");
        audio.let_go(along(slider, 0.72));
        audio.key(KeyCode::Enter);
        audio.typed("4");
        audio.typed("5");
        audio.frame();
        audio.save("settings-state-typing-a-number");

        let mut network = Screen::sized(&tag("network"), SettingTab::Network, size);
        network.press(middle(row_open(network.row_of(SettingId::Email))), MouseButton::Left);
        for text in ["d", "j", "@", "e", "x", "a", "m", "p", "l", "e", ".", "t", "e", "s", "t"] {
            network.typed(text);
        }
        network.frame();
        network.save("settings-state-typing-text");
        network.key(KeyCode::Enter);
        network.app.shared.config.network.rivals = vec!["friend".to_string(), "rival".to_string()];
        network.frame();
        network.press(middle(row_open(network.row_of(SettingId::Rivals))), MouseButton::Left);
        network.frame();
        network.save("settings-state-rival-list");

        let mut display = Screen::sized(&tag("noted"), SettingTab::Display, size);
        display.app.shared.config.skin.select(rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS, Some("/skins/mine/play7.luaskin".to_string()));
        change(&mut display, SettingId::Font);
        change(&mut display, SettingId::WindowResolution);
        display.focus(SettingId::JudgeTextY);
        display.save("settings-state-row-a-skin-does-not-read");
    }
}

/// The SCORES section of the NETWORK group begins with the row that sends every run the score
/// server does not hold yet.
#[test]
fn the_scores_section_starts_with_sync_records_now() {
    let scores = catalog::sections(SettingTab::Network).iter().find(|section| section.heading == "SCORES").expect("the group has a SCORES section");
    assert_eq!(scores.rows.first(), Some(&SettingId::SyncRecords));
    let screen = screen_on("layout-sync-records", SettingTab::Network);
    assert_eq!(value_of(&screen, SettingId::SyncRecords), rbms_config::ACTION_VALUE);
}

/// SYNC RECORDS NOW asks for an account the way the other account rows do, and sends nothing
/// without one.
#[test]
fn sync_records_now_while_signed_out_says_log_in_first() {
    let mut screen = screen_on("layout-sync-signed-out", SettingTab::Network);
    screen.app.shared.config.network.server_url = Some("https://records.test".to_owned());
    screen.focus(SettingId::SyncRecords);
    screen.key(KeyCode::Enter);
    assert_eq!(screen.app.shared.net_status, "log in first");
    assert!(screen.app.shared.record_sync.is_none());
}
