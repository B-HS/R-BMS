//! How the settings screen is laid out and drawn: its three columns, what a frame makes clickable,
//! which part of a row a press lands on, and that nothing the screen writes is cut short at the size
//! it is laid out for.
//!
//! These are the tests of [`crate::settings_view`], kept beside the screen's other tests. They draw
//! onto a [`CpuCanvas`] with the embedded fonts pinned, and read the rows of a fresh configuration
//! through the same calls the screen builds its scene with.

use super::*;
use crate::settings_ui::SettingRow;
use crate::{Config, LaunchOptions};
use rbms_config::{SettingId, SettingKind, SettingTab, cycle_values, descriptor, number_spec, tab_rows};
use rbms_render::CpuCanvas;

/// The size the screen is laid out for.
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

/// Bytes one pixel of a drawn frame takes.
const RGBA_BYTES: usize = 4;

/// How far inside a rectangle a press is made to be clear of its edges.
const INSIDE: f32 = 3.0;

/// More rows than the list has room for, for the scenes that scroll.
const MANY_ROWS: usize = 40;

/// The widest count of rows the rail has room for.
const WIDEST_COUNT: &str = "99";

/// An app with no library, no window and no server, so every row reads out of a fresh document.
fn shared(tag: &str) -> crate::AppShared {
    rbms_render::font::use_embedded_fonts_only();
    let dir = std::env::temp_dir().join(format!("rbms-settings-view-tests-{tag}-{}", std::process::id()));
    crate::App::new(String::new(), Config::default(), LaunchOptions::default(), dir.join("settings.ron")).shared
}

/// Every row of one group as the screen draws it, by its id.
fn lines_of(shared: &crate::AppShared, tab: SettingTab) -> Vec<(SettingId, SettingsLine)> {
    tab_rows(tab, &shared.config).into_iter().map(|id| (id, shared.settings_scene_line(SettingRow::Fixed(id)))).collect()
}

/// A scene of `rows` under one heading, with the cursor on the first and nothing else going on.
fn scene_of(rows: Vec<SettingsLine>) -> SettingsScene {
    let entries = std::iter::once(ListItem::Heading("SECTION")).chain((0..rows.len()).map(ListItem::Row)).collect();
    SettingsScene {
        groups: SettingTab::SHOWN.iter().map(|tab| RailGroup { name: tab.label(), rows: rows.len(), changed: 0 }).collect(),
        group: 0,
        summary: "WHAT THE GROUP HOLDS".to_string(),
        entries,
        rows,
        sel: 0,
        top: 0,
        info: None,
        hint: "UP DOWN MOVE".to_string(),
        link_keys: ["F8".to_string(), "F12".to_string(), "F5".to_string()],
        editor: None,
        status: String::new(),
        rivals: None,
        confirm: None,
        typed: None,
        dragged: None,
        refused: None,
    }
}

fn line(label: &str, value: &str, control: RowControl) -> SettingsLine {
    SettingsLine { control, ..SettingsLine::bare(label, value) }
}

/// Every pixel of the screen drawn for `scene`, with what the frame made clickable.
fn drawn(scene: &SettingsScene) -> (Vec<u8>, Vec<(Rect, SettingsHot)>) {
    rbms_render::font::use_embedded_fonts_only();
    let mut canvas = CpuCanvas::new(WIDTH, HEIGHT);
    let hot = render_settings(&mut canvas, scene);
    (canvas.pixels().to_vec(), hot)
}

fn middle(rect: Rect) -> (f32, f32) {
    (rect.x + rect.w * 0.5, rect.y + rect.h * 0.5)
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// The bytes of the frame that fall outside the rows of pixels `rect` covers.
fn outside_the_rows_of(pixels: &[u8], rect: Rect) -> (Vec<u8>, Vec<u8>) {
    let row_bytes = WIDTH as usize * RGBA_BYTES;
    (pixels[..rect.y as usize * row_bytes].to_vec(), pixels[(rect.y + rect.h) as usize * row_bytes..].to_vec())
}

/// Whether what is written is all of `said`, however it was broken over lines.
fn says_all_of(lines: &[String], said: &str) -> bool {
    lines.join(" ").split_whitespace().eq(said.split_whitespace())
}

/// The rail, the list and the panel stand side by side inside the screen with a gap between them,
/// and everything on the rail -- the groups, the button that puts a group back and the buttons to
/// the other screens -- is on it without one running under another.
#[test]
fn the_three_columns_and_the_parts_of_the_rail_do_not_meet() {
    let screen = Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32);
    for column in [RAIL_RECT, LIST_RECT, INFO_RECT] {
        assert!(column.x >= MARGIN && column.x + column.w <= screen.w - MARGIN && column.y + column.h <= HINT_Y - HINT_GAP, "{column:?}");
        assert!(column.w > 0.0 && column.h > 0.0);
    }
    assert!((LIST_RECT.x - (RAIL_RECT.x + RAIL_RECT.w) - COLUMN_GAP).abs() < f32::EPSILON);
    assert!((INFO_RECT.x - (LIST_RECT.x + LIST_RECT.w) - COLUMN_GAP).abs() < f32::EPSILON);

    let groups = SettingTab::SHOWN.len();
    let mut parts: Vec<Rect> = (0..groups).map(rail_entry).collect();
    parts.push(reset_group_rect(groups));
    parts.extend((0..LINK_COUNT).map(link_rect));
    for (at, part) in parts.iter().enumerate() {
        assert!(part.x >= RAIL_RECT.x && part.x + part.w <= RAIL_RECT.x + RAIL_RECT.w, "{part:?} runs off the rail");
        assert!(part.y >= RAIL_RECT.y && part.y + part.h <= RAIL_RECT.y + RAIL_RECT.h + f32::EPSILON, "{part:?} runs off the rail");
        assert!(parts[at + 1..].iter().all(|other| !overlap(*part, *other)), "{part:?} meets another part of the rail");
    }
    let heading_top = link_rect(PLAY_MENU_PLACE).y - LINKS_HEADING_RISE;
    assert!(heading_top > reset_group_rect(groups).y + RESET_H, "the heading over the buttons runs under the button that puts a group back");
    assert_eq!(key_config_link(), link_rect(KEY_CONFIG_PLACE));
    assert!(VISIBLE_SLOTS as f32 * ROW_PITCH + LIST_PAD <= LIST_RECT.h - COUNTER_RISE, "the last slot runs under the cursor's place in the list");
}

/// A frame makes one region of every group on the rail, one of each button to another screen and
/// one of every row it drew, and none of a heading.
#[test]
fn a_frame_reports_one_region_for_every_group_button_and_row() {
    let scene = scene_of((0..5).map(|at| line(&format!("ROW {at}"), "ON", RowControl::Switch { on: true })).collect());
    let (pixels, hot) = drawn(&scene);
    assert_eq!(hot.iter().filter(|(_, hot)| matches!(hot, SettingsHot::Tab(_))).count(), SettingTab::SHOWN.len());
    for link in [SettingsHot::PlayMenuLink, SettingsHot::SkinMenuLink, SettingsHot::KeyConfigLink] {
        assert_eq!(hot.iter().filter(|(_, hot)| *hot == link).count(), 1, "{link:?} is not clickable once");
    }
    let rows: Vec<usize> = hot.iter().filter_map(|(_, hot)| if let SettingsHot::Row(at) = hot { Some(*at) } else { None }).collect();
    assert_eq!(rows, vec![0, 1, 2, 3, 4]);
    assert_eq!(hot.len(), SettingTab::SHOWN.len() + LINK_COUNT + scene.rows.len(), "a heading or something else is a region of the frame");
    assert!(pixels != drawn(&scene_of(Vec::new())).0, "the rows are not drawn");
}

/// A list longer than its room draws the entries on show from the first one named and no others,
/// with a bar beside them that says where in the list they are; a list that fits has no bar.
#[test]
fn a_long_list_draws_what_is_on_show_and_a_bar_beside_it() {
    let rows: Vec<SettingsLine> = (0..MANY_ROWS).map(|at| line(&format!("ROW {at}"), "ON", RowControl::Switch { on: true })).collect();
    let mut scene = scene_of(rows);
    assert!(scene.entries.len() > visible_rows());
    let (at_the_top, hot) = drawn(&scene);
    let shown = |hot: &[(Rect, SettingsHot)]| -> Vec<usize> {
        hot.iter().filter_map(|(_, hot)| if let SettingsHot::Row(at) = hot { Some(*at) } else { None }).collect()
    };
    assert_eq!(shown(&hot), (0..visible_rows() - 1).collect::<Vec<_>>(), "the heading takes the first slot and the rows the rest");

    scene.top = scene.entries.len() - visible_rows();
    scene.sel = MANY_ROWS - 1;
    let (at_the_end, hot) = drawn(&scene);
    assert_eq!(shown(&hot), (MANY_ROWS - visible_rows()..MANY_ROWS).collect::<Vec<_>>());
    assert!(hot.iter().all(|(rect, hot)| !matches!(hot, SettingsHot::Row(_)) || rect.y + rect.h <= LIST_RECT.y + LIST_RECT.h), "a row is drawn off the list");

    let bar_x = (LIST_RECT.x + LIST_RECT.w - SCROLL_BAR_INSET - SCROLL_BAR_W * 0.5) as usize;
    let pixel = |pixels: &[u8], y: f32| {
        let at = (y as usize * WIDTH as usize + bar_x) * RGBA_BYTES;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    };
    let (first_slot, last_slot) = (slot_rect(0), slot_rect(visible_rows() - 1));
    assert_ne!(pixel(&at_the_top, first_slot.y + INSIDE), pixel(&at_the_top, last_slot.y + INSIDE), "the bar's thumb is not at the top of a list at its top");
    assert_eq!(pixel(&at_the_top, first_slot.y + INSIDE), pixel(&at_the_end, last_slot.y + INSIDE), "the thumb is not at the foot of a list at its end");

    let short = scene_of((0..3).map(|at| line(&format!("ROW {at}"), "ON", RowControl::Switch { on: true })).collect());
    let (fits, _) = drawn(&short);
    let panel = theme().panel;
    assert_eq!(pixel(&fits, first_slot.y + INSIDE), [panel.r, panel.g, panel.b], "a list that fits has a scroll bar");
}

/// A press is told by the part of the row it lands on: the switch and its word turn a switch over,
/// a value of a choice is that value, the two arrows of a longer choice step it either way, a
/// slider and a number are a number row's, and the room of a field, a path or an action opens it.
/// The label of every one of them is the label and nothing more.
#[test]
fn a_press_is_told_by_the_part_of_the_row_it_lands_on() {
    rbms_render::font::use_embedded_fonts_only();
    let row = slot_rect(3);
    let on_the_label = (row.x + LABEL_X + INSIDE, row.y + row.h * 0.5);
    let values: &[&str] = &["LN", "CN", "HCN"];
    let controls = [
        RowControl::Switch { on: false },
        RowControl::Segments { values, chosen: Some(0) },
        RowControl::Stepper,
        RowControl::Slider,
        RowControl::Field { filled: true },
        RowControl::Path { chosen: false },
        RowControl::Action,
        RowControl::Info,
    ];
    for control in controls {
        assert_eq!(control_part_at(row, control, on_the_label), ControlPart::Label, "{control:?}");
        assert_eq!(control_part_at(row, control, (row.x + row.w * 0.5, row.y - INSIDE)), ControlPart::Label, "{control:?}: a press above the row");
    }

    assert_eq!(control_part_at(row, RowControl::Switch { on: true }, middle(switch_track(row))), ControlPart::Switch);
    assert_eq!(
        control_part_at(row, RowControl::Switch { on: true }, (row_switch(row).x + INSIDE, row.y + INSIDE)),
        ControlPart::Switch,
        "the word of a switch"
    );
    for place in 0..values.len() {
        let cell = row_segment(row, place, values.len());
        assert_eq!(control_part_at(row, RowControl::Segments { values, chosen: None }, middle(cell)), ControlPart::Segment(place));
        assert!(cell.x >= control_rect(row).x && cell.x + cell.w <= control_right(row) + f32::EPSILON, "{cell:?} is outside the room of the row");
    }
    assert_eq!(control_part_at(row, RowControl::Stepper, middle(back_cell(row))), ControlPart::Back);
    assert_eq!(control_part_at(row, RowControl::Stepper, middle(end_cell(row))), ControlPart::Forward);
    assert_eq!(control_part_at(row, RowControl::Stepper, (control_right(row) - STEPPER_W * 0.5, row.y + INSIDE)), ControlPart::Forward, "the value");
    assert_eq!(control_part_at(row, RowControl::Slider, middle(row_slider(row))), ControlPart::Slider(row_slider(row)));
    assert_eq!(control_part_at(row, RowControl::Slider, middle(row_number(row))), ControlPart::Number);
    for control in [RowControl::Field { filled: false }, RowControl::Path { chosen: true }, RowControl::Action] {
        assert_eq!(control_part_at(row, control, middle(control_rect(row))), ControlPart::Open, "{control:?}");
        assert_eq!(control_part_at(row, control, middle(end_cell(row))), ControlPart::Open, "{control:?}");
    }
    assert_eq!(control_part_at(row, RowControl::Info, middle(control_rect(row))), ControlPart::Label);

    assert!(row_slider(row).x >= control_rect(row).x, "the slider and its number take more than the room a row keeps");
    assert!(back_cell(row).x >= control_rect(row).x && row_switch(row).x >= control_rect(row).x);
}

/// A choice is drawn with all its values on show when they are few and each can be written whole in
/// its cell, and as its value between two arrows otherwise.
#[test]
fn a_choice_is_drawn_whole_only_when_every_value_fits_its_cell() {
    rbms_render::font::use_embedded_fonts_only();
    assert!(segments_fit(&["ON", "OFF"]));
    assert!(segments_fit(&["COMBO", "DURATION", "LOWEST", "SCORE"]));
    assert!(!segments_fit(&["A", "B", "C", "D", "E"]), "more values than are shown side by side");
    assert!(!segments_fit(&["NONE", "CONTINUE", "SURVIVAL TO GROOVE", "BEST"]), "a value too long for its cell");
    assert!(!segments_fit(&[]));

    let shared = shared("choices");
    let whole = [SettingId::JudgeAlgorithm, SettingId::LnMode, SettingId::GaugeSet, SettingId::BottomShiftableGauge, SettingId::WindowMode, SettingId::Skin];
    for id in whole {
        let line = shared.settings_scene_line(SettingRow::Fixed(id));
        let RowControl::Segments { values, chosen } = line.control else {
            panic!("{id:?} is drawn as {:?}", line.control);
        };
        assert_eq!(values, cycle_values(id));
        assert_eq!(chosen.map(|at| values[at]), Some(line.value.as_str()), "{id:?}");
    }
    for id in
        [SettingId::GaugeAutoShift, SettingId::WindowResolution, SettingId::Sort, SettingId::AudioBuffer, SettingId::AudioSampleRate, SettingId::AudioDevice]
    {
        assert_eq!(shared.settings_scene_line(SettingRow::Fixed(id)).control, RowControl::Stepper, "{id:?}");
    }
}

/// Every row of the screen is set by what its kind asks for: a switch for what is on or off, a
/// slider for a number, a field for text, the mark of a picker for the font and the folder of the
/// system sounds, and the mark of an action for what is run or opened.
#[test]
fn every_row_is_set_by_what_its_kind_asks_for() {
    let shared = shared("kinds");
    for tab in SettingTab::SHOWN {
        for (id, line) in lines_of(&shared, tab) {
            let expected = match (descriptor(id).kind, number_spec(id).is_some()) {
                (_, true) => matches!(line.control, RowControl::Slider) && line.number.is_some(),
                (SettingKind::Toggle, _) => line.control == RowControl::Switch { on: line.value == "ON" },
                (SettingKind::Cycle { .. }, _) => matches!(line.control, RowControl::Segments { .. } | RowControl::Stepper),
                (SettingKind::Text { .. }, _) => matches!(line.control, RowControl::Field { .. }),
                (SettingKind::FilePick, _) => line.control == RowControl::Path { chosen: false },
                (SettingKind::Action, _) if id == SettingId::SoundFolder => line.control == RowControl::Path { chosen: false },
                (SettingKind::Action, _) => line.control == RowControl::Action,
                (kind, _) => panic!("{id:?} is a {kind:?}, which no row of the screen was"),
            };
            assert!(expected, "{id:?} is set by {:?}", line.control);
            assert!(!line.changed, "{id:?} is marked in a configuration nothing was changed in");
        }
    }
    let fields = [(SettingId::ServerUrl, false), (SettingId::PlayerId, true), (SettingId::Email, false), (SettingId::Password, false)];
    for (id, filled) in fields {
        assert_eq!(shared.settings_scene_line(SettingRow::Fixed(id)).control, RowControl::Field { filled }, "{id:?}");
    }
}

/// At the size the screen is laid out for, no label, heading, note or name on the rail is cut
/// short, every value a row can show fits the room its row gives it, and the three buttons to the
/// other screens have room for the longest key a player can put them on.
#[test]
fn nothing_the_screen_writes_is_cut_short_at_the_size_it_is_laid_out_for() {
    let shared = shared("fits");
    let row = slot_rect(0);
    for tab in SettingTab::SHOWN {
        assert!(text_width(tab.label(), RAIL_NAME_SCALE) + text_width(WIDEST_COUNT, RAIL_COUNT_SCALE) + MARK_X + MARK_SIZE <= rail_room(), "{tab:?}");
        let summary = crate::stage::settings::catalog::group_summary(tab);
        let left = MARGIN + text_width(TITLE, TITLE_SCALE) + SUBTITLE_GAP;
        assert_eq!(fit_text(summary, SUBTITLE_SCALE, WIDTH as f32 - MARGIN - left), summary, "{tab:?}");
        for section in crate::stage::settings::catalog::sections(tab) {
            assert!(text_width(section.heading, HEADING_SCALE) + HEADING_X + HEADING_RULE_GAP < row.w * 0.5, "{}", section.heading);
        }
        for (id, line) in lines_of(&shared, tab) {
            assert_eq!(fit_text(&line.label, LABEL_SCALE, label_room(row)), line.label, "{id:?}: the label is cut short");
            if let Some(skinned) = descriptor(id).skinned {
                let (left, room) = note_span(row, &line.label);
                assert!(left > row.x && text_width(skinned.note, NOTE_SCALE) <= room, "{id:?}: the note needs more than {room}");
            }
            let stepper_room = STEPPER_W - CELL_SIZE * 2.0 - STEPPER_PAD * 2.0;
            match line.control {
                RowControl::Stepper => {
                    for value in cycle_values(id) {
                        assert!(text_width(value, VALUE_SCALE) <= stepper_room, "{id:?}: {value} does not fit between the arrows");
                    }
                }
                RowControl::Segments { values, .. } => assert!(segments_fit(values), "{id:?}"),
                RowControl::Slider => {
                    let spec = number_spec(id).expect("a row with a slider holds a number");
                    for shown in [spec.text(spec.min), spec.text(spec.max), line.value.clone()] {
                        assert!(text_width(&shown, VALUE_SCALE) <= NUMBER_VALUE_W, "{id:?}: {shown} does not fit the room of a number");
                    }
                }
                _ => {}
            }
        }
    }
    for name in [PLAY_MENU_LINK, SKIN_MENU_LINK, KEY_CONFIG_LINK] {
        let key = format!("BACKSLASH {FORWARD_MARK}");
        let room = rail_room() - text_width(name, LINK_NAME_SCALE);
        assert_eq!(fit_text(&key, LINK_KEY_SCALE, room), key, "{name} leaves no room for a long key");
    }
    assert!(text_width(RESET_GROUP_LABEL, RESET_SCALE) + text_width(RESET_GROUP_KEY, RESET_SCALE) <= rail_room());
    for words in [CONFIRM_YES, CONFIRM_NO] {
        assert!(text_width(words, CONFIRM_BUTTON_SCALE) <= confirm_button_w() - CONFIRM_PAD);
    }
    assert!(confirm_no_rect().x + confirm_no_rect().w <= confirm_rect().x + confirm_rect().w - CONFIRM_PAD + f32::EPSILON);
}

/// What the panel says a row is for is written whole on the lines it has, for every row of the
/// screen.
#[test]
fn every_description_is_written_whole_on_the_panel() {
    let shared = shared("descriptions");
    for tab in SettingTab::SHOWN {
        for id in tab_rows(tab, &shared.config) {
            let said = crate::stage::settings::catalog::description(id);
            let lines = description_lines(said);
            assert!(lines.len() <= DESCRIPTION_LINES && says_all_of(&lines, said), "{id:?} is cut short on the panel: {lines:?}");
            assert!(INFO_DESCRIPTION_Y + lines.len() as f32 * INFO_LINE_PITCH <= INFO_RULE_Y, "{id:?} runs under the line below it");
        }
    }
}

/// A path is cut short from its front, so the folder that tells it from another stays on the row,
/// and one that fits is written as it is.
#[test]
fn a_path_is_cut_short_from_its_front() {
    rbms_render::font::use_embedded_fonts_only();
    let path = "/Users/someone/Library/Application Support/rbms/sounds/default-set";
    let room = text_width("/rbms/sounds/default-set", VALUE_SCALE) + text_width(ELLIPSIS, VALUE_SCALE);
    let cut = fit_tail(path, VALUE_SCALE, room);
    assert!(cut.starts_with(ELLIPSIS) && cut.ends_with("/sounds/default-set") && text_width(&cut, VALUE_SCALE) <= room, "{cut}");
    assert_eq!(fit_tail("sounds", VALUE_SCALE, room), "sounds");
    assert_eq!(fit_tail(path, VALUE_SCALE, 0.0), path, "no room at all is not a width to cut to");
}

/// The mark of a row that is not what it ships as is drawn inside that row and changes nothing else
/// of the frame, and neither does the note beside a row.
#[test]
fn the_mark_and_the_note_of_a_row_are_drawn_inside_it() {
    let rows = |changed: bool, note: Option<&'static str>| {
        vec![
            line("FIRST", "ON", RowControl::Switch { on: true }),
            SettingsLine { changed, note, ..line("SECOND", "OFF", RowControl::Switch { on: false }) },
            line("THIRD", "ON", RowControl::Switch { on: true }),
        ]
    };
    let (plain, _) = drawn(&scene_of(rows(false, None)));
    let second = slot_rect(2);
    for (name, other) in [("mark", rows(true, None)), ("note", rows(false, Some(rbms_config::BUILT_IN_ONLY_NOTE)))] {
        let (frame, _) = drawn(&scene_of(other));
        assert!(frame != plain, "a row with a {name} is drawn the same as one without");
        assert!(outside_the_rows_of(&frame, second) == outside_the_rows_of(&plain, second), "the {name} changed something outside its row");
    }
}

/// What sets a row is drawn for the value it holds: a switch that is on is not drawn as one that is
/// off, the chosen value of a choice is lit where it stands, and a row under the cursor is lit.
#[test]
fn what_sets_a_row_is_drawn_for_the_value_it_holds() {
    let values: &[&str] = &["LN", "CN", "HCN"];
    let frame = |rows: Vec<SettingsLine>, sel: usize| drawn(&SettingsScene { sel, ..scene_of(rows) }).0;
    let switch = |on: bool| vec![line("A SWITCH", if on { "ON" } else { "OFF" }, RowControl::Switch { on }), line("OTHER", ">", RowControl::Action)];
    assert!(frame(switch(true), 1) != frame(switch(false), 1));
    let choice = |chosen: Option<usize>| vec![line("A CHOICE", "LN", RowControl::Segments { values, chosen }), line("OTHER", ">", RowControl::Action)];
    let frames = [frame(choice(Some(0)), 1), frame(choice(Some(1)), 1), frame(choice(Some(2)), 1), frame(choice(None), 1)];
    for (at, one) in frames.iter().enumerate() {
        assert!(frames[at + 1..].iter().all(|other| other != one), "two choices standing on different values are drawn the same");
    }
    assert!(frame(choice(Some(0)), 0) != frame(choice(Some(0)), 1), "the row under the cursor is not lit");
}

/// The panel is drawn for the row it is handed and for nothing when it is handed none, and what it
/// writes stays above the keys it lists however much a row has to say for itself.
#[test]
fn the_panel_says_what_it_is_handed_within_its_room() {
    let rows = vec![line("A ROW", "ON", RowControl::Switch { on: true })];
    let info = InfoScene {
        label: "A ROW".to_string(),
        kind: "SWITCH".to_string(),
        description: "What the row is for.",
        facts: vec![("VALUE", "ON".to_string()), ("DEFAULT", "OFF".to_string())],
        remark: Some("What is said of it in particular."),
        caution: Some("A CAUTION."),
        keys: vec![("LEFT RIGHT".to_string(), "SWITCH")],
    };
    let (bare, _) = drawn(&scene_of(rows.clone()));
    let (with_info, _) = drawn(&SettingsScene { info: Some(info), status: "A STATUS".to_string(), ..scene_of(rows) });
    assert!(bare != with_info, "the panel draws nothing of the row it is handed");
    let status_top = INFO_RECT.h - STATUS_FOOT - STATUS_LINES as f32 * INFO_LINE_PITCH - KEYS_FIRST_Y;
    assert!(INFO_KEYS_Y + KEYS_FIRST_Y + INFO_KEY_LINES as f32 * INFO_LINE_PITCH <= status_top, "the keys of a row run under the status at the panel's foot");
    const { assert!(INFO_FACTS_Y > INFO_RULE_Y && INFO_KEYS_Y > INFO_FACTS_Y, "the parts of the panel are not one under another") };
}

/// The inline rival list is drawn over the rows and each of its rows is a region of the frame, and
/// the question about a group is drawn over them too with its two buttons inside its panel.
#[test]
fn the_inline_list_and_the_question_are_drawn_over_the_rows() {
    let rows = || (0..6).map(|at| line(&format!("ROW {at}"), ">", RowControl::Action)).collect::<Vec<_>>();
    let (plain, _) = drawn(&scene_of(rows()));
    let listed = RivalsScene { rows: vec!["friend".into(), crate::ir_panel::RIVAL_ADD_ROW.into()], sel: 1, editor: Some("typing_".into()) };
    let (with_list, hot) = drawn(&SettingsScene { rivals: Some(listed), ..scene_of(rows()) });
    let rivals: Vec<usize> = hot.iter().filter_map(|(_, hot)| if let SettingsHot::RivalRow(at) = hot { Some(*at) } else { None }).collect();
    assert_eq!(rivals, vec![0, 1]);
    assert!(with_list != plain);
    const { assert!(RIVALS_X >= LIST_RECT.x && RIVALS_X + RIVALS_W <= LIST_RECT.x + LIST_RECT.w, "the inline list is not over the rows") };

    let asked = ConfirmScene { title: "RESET AUDIO?".to_string(), body: "3 CHANGED ROWS GO BACK TO DEFAULT. TEXT IS KEPT.".to_string() };
    let (with_question, hot) = drawn(&SettingsScene { confirm: Some(asked), ..scene_of(rows()) });
    assert!(with_question != plain);
    assert_eq!(hot.len(), drawn(&scene_of(rows())).1.len(), "the question added a region to the frame");
    let panel = confirm_rect();
    for button in [confirm_yes_rect(), confirm_no_rect()] {
        assert!(button.x >= panel.x && button.x + button.w <= panel.x + panel.w && button.y >= panel.y && button.y + button.h <= panel.y + panel.h);
    }
    assert!(!overlap(confirm_yes_rect(), confirm_no_rect()));
    assert!(panel.x >= LIST_RECT.x && panel.x + panel.w <= LIST_RECT.x + LIST_RECT.w);
}

/// A field being typed into is drawn with what has been typed, on the row it belongs to and on no
/// other, and an empty one is not drawn as a filled one.
#[test]
fn a_field_is_drawn_with_what_is_being_typed_into_it() {
    let rows = || vec![line("SERVER URL", "(none)", RowControl::Field { filled: false }), line("EMAIL", "dj@example.test", RowControl::Field { filled: true })];
    let (plain, _) = drawn(&scene_of(rows()));
    let (typing, _) = drawn(&SettingsScene { editor: Some((0, "https://ir_".to_string())), ..scene_of(rows()) });
    assert!(typing != plain);
    let first = slot_rect(1);
    assert!(outside_the_rows_of(&typing, first) == outside_the_rows_of(&plain, first), "typing into one row changed another");
}
