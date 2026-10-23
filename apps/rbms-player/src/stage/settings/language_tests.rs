//! The settings screen written in each language: what it says changes with the language and
//! nothing it decides by does. The fonts of the tests have no Korean or Japanese, so these read
//! the words the screen draws rather than the pixels, and measure what has to fit by a count.

use rbms_config::{LanguageChoice, cycle_values, descriptor};
use rbms_model::Language;
use rbms_model::translations::Catalog;

use super::number_tests::Screen;
use super::*;
use crate::stage::preview_menu::MARGIN;
use crate::text::settings::screen::NumberRowText;
use crate::text::settings::{InstalledLanguage, group_summary, row_label};

/// How wide a character the line of keys is written with is, in pixels: a full-width character is
/// as wide as the size it is written at, which is `round(1.2 * 8.5)` there.
const HINT_EM_PX: f32 = 10.0;

/// How wide a letter of the alphabet is, in the em of the font, on average over the capitals and
/// the numbers the screen writes among Korean and Japanese, and how wide a space is.
const LATIN_EM: f32 = 0.68;
const SPACE_EM: f32 = 0.3;

/// How wide one character is, in em: the fonts of the tests have no Korean or Japanese to measure
/// them with, so a character outside ASCII is counted as a whole em, which is as wide as the widest
/// of them is.
fn em_of(ch: char) -> f32 {
    match ch {
        ' ' => SPACE_EM,
        ch if ch.is_ascii() => LATIN_EM,
        _ => 1.0,
    }
}

/// What a sentence takes of a line, in em.
pub(crate) fn em_width(text: &str) -> f32 {
    text.chars().map(em_of).sum()
}

/// How many lines of `room_em` a sentence is broken over, the way the panel breaks one: at the last
/// space of a line when it has one, and at the character the width is passed on when it has none.
pub(crate) fn lines_needed(text: &str, room_em: f32) -> usize {
    let mut rest: Vec<char> = text.trim().chars().collect();
    let mut lines = 0;
    while !rest.is_empty() {
        lines += 1;
        let (mut used, mut space) = (0.0, None);
        let mut cut = rest.len();
        for (at, ch) in rest.iter().enumerate() {
            used += em_of(*ch);
            if used > room_em {
                cut = space.unwrap_or(at).max(1);
                break;
            }
            if *ch == ' ' {
                space = Some(at);
            }
        }
        rest.drain(..cut);
        let spaces = rest.iter().take_while(|ch| **ch == ' ').count();
        rest.drain(..spaces);
    }
    lines
}

/// The lines of the screen as it draws one group, read afresh.
fn lines_of(screen: &mut Screen) -> Vec<SettingsLine> {
    screen.scene().rows
}

/// What a row is told apart by is the same whichever language the screen is drawn in: the value the
/// table gives it, whether it is changed, the number it holds, the note beside it and what sets it.
#[test]
fn what_a_row_is_told_apart_by_is_the_same_in_every_language() {
    for tab in SettingTab::SHOWN {
        let english = lines_of(&mut Screen::on("language-logic-english", tab));
        for language in [Language::Korean, Language::Japanese] {
            let _installed = InstalledLanguage::new(language);
            let written = lines_of(&mut Screen::on("language-logic", tab));
            assert_eq!(written.len(), english.len(), "{tab:?} in {language:?}");
            for (line, plain) in written.iter().zip(&english) {
                assert_eq!(
                    (&line.value, line.changed, &line.number, line.note),
                    (&plain.value, plain.changed, &plain.number, plain.note),
                    "{} in {language:?}",
                    plain.label
                );
                match (line.control, plain.control) {
                    (RowControl::Segments { chosen: here, .. }, RowControl::Segments { chosen: there, .. }) => assert_eq!(here, there, "{}", plain.label),
                    (RowControl::Segments { .. } | RowControl::Stepper, RowControl::Segments { .. } | RowControl::Stepper) => {}
                    (here, there) => assert_eq!(here, there, "{} in {language:?}", plain.label),
                }
            }
        }
    }
}

/// A switch reads ON or OFF and a number its figure and unit whatever the language, since those are
/// terms the glossary keeps; the words a row is named by and the marks of an empty row change.
#[test]
fn a_switch_and_a_number_read_as_they_do_in_english_and_the_rest_is_written_in_the_language() {
    let _korean = InstalledLanguage::new(Language::Korean);
    let mut audio = Screen::on("language-values", SettingTab::Audio);
    let volume = audio.scene().rows.into_iter().find(|line| line.label == row_label(SettingId::MasterVolume)).expect("the master volume is listed");
    assert_eq!((volume.label.as_str(), volume.shown.as_str()), ("마스터 볼륨", "100%"));
    let guide = audio.scene().rows.into_iter().find(|line| line.label == row_label(SettingId::GuideSe)).expect("the guide sound is listed");
    assert_eq!((guide.shown.as_str(), guide.control), ("OFF", RowControl::Switch { on: false }));
    let mut network = Screen::on("language-values-network", SettingTab::Network);
    let email = network.scene().rows.into_iter().find(|line| line.label == row_label(SettingId::Email)).expect("the address is listed");
    assert_eq!((email.value.as_str(), email.shown.as_str(), email.control), ("(none)", "(없음)", RowControl::Field { filled: false }));
    let password = network.scene().rows.into_iter().find(|line| line.label == row_label(SettingId::Password)).expect("the password is listed");
    assert_eq!((password.value.as_str(), password.shown.as_str()), ("(not set)", "(설정 안 됨)"));
}

/// A choice of a few plain words is drawn with them written in the language, in the cells it is
/// drawn in, and it is still told apart from the others by the English the table gives it.
#[test]
fn a_choice_of_plain_words_is_drawn_in_the_language_and_chosen_by_its_english() {
    let _japanese = InstalledLanguage::new(Language::Japanese);
    let mut input = Screen::on("language-choice", SettingTab::Input);
    let side = input.scene().rows.into_iter().find(|line| line.label == row_label(SettingId::ScratchSide)).expect("the side is listed");
    assert_eq!((side.value.as_str(), side.shown.as_str()), ("RIGHT", "右"));
    assert_eq!(cycle_values(SettingId::ScratchSide), ["RIGHT", "LEFT"]);
    if let RowControl::Segments { values, chosen } = side.control {
        assert_eq!((values.len(), chosen, side.choices.clone()), (2, Some(0), vec!["右".to_string(), "左".to_string()]));
    }
    input.focus(SettingId::ScratchSide);
    input.key(KeyCode::ArrowRight);
    assert_eq!(input.app.shared.setting_line(SettingId::ScratchSide).1, "LEFT", "a step moved the English the table gives");
    let moved = input.scene().rows.into_iter().find(|line| line.label == row_label(SettingId::ScratchSide)).expect("the side is listed");
    assert_eq!((moved.value.as_str(), moved.shown.as_str()), ("LEFT", "左"));
}

/// The screen writes its rail, its headings, its summary and its rows again as soon as the language
/// is another, without a key being pressed and without the rows being marked as moved.
#[test]
fn the_screen_is_written_again_as_soon_as_the_language_changes() {
    let mut audio = Screen::on("language-follow", SettingTab::Audio);
    let before = audio.scene();
    assert_eq!(before.groups[1].name, "DISPLAY");
    assert_eq!(before.summary, group_summary(SettingTab::Audio));
    {
        let _korean = InstalledLanguage::new(Language::Korean);
        assert!(!audio.state.dirty, "nothing moved");
        let after = audio.scene();
        let names: Vec<&str> = after.groups.iter().map(|group| group.name).collect();
        assert_eq!(names, ["판정", "표시", "입력", "네트워크", "오디오", "선곡"]);
        assert_eq!(after.summary, "볼륨, 출력 장치, 시스템 사운드");
        assert_eq!(after.entries.first(), Some(&ListItem::Heading("볼륨")));
        assert_eq!(after.rows[0].label, "마스터 볼륨");
        assert_eq!(after.rows.len(), before.rows.len());
        assert_eq!(after.info.as_ref().map(|panel| panel.label.as_str()), Some("마스터 볼륨"));
        audio.frame();
    }
    let back = audio.scene();
    assert_eq!(back.groups[1].name, "DISPLAY");
    assert_eq!(back.rows[0].label, "MASTER VOL");
}

/// Stepping the LANGUAGE row rewrites the screen in the language it names on the frame that
/// settles it, and the screen goes on working: the row it was stepped on is still the cursor's.
#[test]
fn stepping_the_language_row_rewrites_the_screen_in_the_language_it_names() {
    let _restore = InstalledLanguage::new(Language::English);
    let mut display = Screen::on("language-row", SettingTab::Display);
    display.focus(SettingId::Language);
    display.key(KeyCode::ArrowRight);
    display.key(KeyCode::ArrowRight);
    assert_eq!(display.app.shared.config.display.language, LanguageChoice::Korean);
    assert_eq!(display.scene().rows[display.state.sel].label, "LANGUAGE", "the screen was written before the language was settled");
    display.app.shared.language.follow(display.app.shared.config.display.language);
    let scene = display.scene();
    assert_eq!(scene.rows[scene.sel].label, "언어");
    assert_eq!(scene.rows[scene.sel].shown, "한국어");
    assert_eq!(scene.groups[scene.group].name, "표시");
    assert_eq!(display.state.focused_fixed(), Some(SettingId::Language));
    display.frame();
}

/// The question about a group is asked in the language, with its group named in it and a row or
/// rows as the number says, and a group that holds text says the text is kept.
#[test]
fn the_question_about_a_group_is_asked_in_the_language() {
    for (language, title, one, many, one_keeping_text) in [
        (
            Language::English,
            "RESET JUDGE?",
            "1 CHANGED ROW GOES BACK TO DEFAULT.",
            "2 CHANGED ROWS GO BACK TO DEFAULT.",
            "1 CHANGED ROW GOES BACK TO DEFAULT. TEXT IS KEPT.",
        ),
        (
            Language::Korean,
            "판정 초기화?",
            "변경된 행 1개를 기본값으로 되돌립니다.",
            "변경된 행 2개를 기본값으로 되돌립니다.",
            "변경된 행 1개를 기본값으로 되돌립니다. 텍스트는 그대로 둡니다.",
        ),
        (
            Language::Japanese,
            "判定を初期化しますか？",
            "変更された行1件を既定値に戻します。",
            "変更された行2件を既定値に戻します。",
            "変更された行1件を既定値に戻します。テキストはそのままです。",
        ),
    ] {
        let _installed = InstalledLanguage::new(language);
        let mut judge = Screen::on("language-question", SettingTab::Judge);
        judge.app.shared.config.judge.judge_rate_key[0] = 120;
        judge.state.dirty = true;
        judge.frame();
        judge.shifted(KeyCode::Digit0);
        let asked = judge.scene().confirm.expect("the screen did not ask");
        assert_eq!((asked.title.as_str(), asked.body.as_str()), (title, one), "{language:?}");
        judge.key(KeyCode::Escape);
        judge.app.shared.config.judge.judge_rate_key[1] = 120;
        judge.state.dirty = true;
        judge.frame();
        judge.shifted(KeyCode::Digit0);
        assert_eq!(judge.scene().confirm.expect("the screen did not ask").body, many, "{language:?}");

        let mut network = Screen::on("language-question-text", SettingTab::Network);
        network.app.shared.config.network.discord_status = !network.app.shared.config.network.discord_status;
        network.state.dirty = true;
        network.frame();
        network.shifted(KeyCode::Digit0);
        assert_eq!(network.scene().confirm.expect("the screen did not ask").body, one_keeping_text, "{language:?}");
    }
}

/// The panel names the kind of row and what it holds in the language, finds the value being typed
/// by the name it was given, and says a choice's place among its values.
#[test]
fn the_panel_writes_what_the_row_is_and_holds_in_the_language() {
    let _korean = InstalledLanguage::new(Language::Korean);
    let mut audio = Screen::on("language-panel", SettingTab::Audio);
    audio.focus(SettingId::BgmVolume);
    let panel = audio.scene().info.expect("the panel says nothing of the row");
    assert_eq!((panel.kind.as_str(), panel.label.as_str()), ("숫자", "BGM 볼륨"));
    assert_eq!(panel.facts[0], ("값", "50%".to_string()));
    assert!(panel.facts.iter().any(|(name, _)| *name == "범위"), "{:?}", panel.facts);
    assert_eq!(panel.keys[0].1, "1단계");
    audio.key(KeyCode::Enter);
    audio.typed("4");
    audio.typed("5");
    let typed = audio.scene().info.expect("the panel says nothing of the row");
    assert_eq!(typed.facts[0], ("값", "45".to_string()), "the number being typed was not put under the name of the value");
    assert_eq!(audio.scene().hint, NumberRowText::EditHint.get(Language::Korean));
    audio.key(KeyCode::Escape);

    let mut select = Screen::on("language-panel-sort", SettingTab::Select);
    select.focus(SettingId::Sort);
    let sort = select.scene().info.expect("the panel says nothing of the row");
    assert_eq!(sort.kind, "선택  11개 중 1번째");
    let choices = sort.facts.iter().find(|(name, _)| *name == "선택지").map(|(_, value)| value.clone());
    assert_eq!(choices.as_deref(), Some("DEFAULT, 제목, 아티스트, BPM, 길이, 레벨, CLEAR, 스코어, MISS COUNT, DURATION, 최종 갱신"));
}

/// Every group is drawn in every language, with the same regions clickable and every row still on
/// the list: a language changes the words and moves nothing.
#[test]
fn every_group_is_drawn_in_every_language_with_the_same_regions() {
    for tab in SettingTab::SHOWN {
        let mut english = Screen::on("language-frames", tab);
        english.frame();
        let regions = english.app.shared.hot.len();
        for language in [Language::Korean, Language::Japanese] {
            let _installed = InstalledLanguage::new(language);
            let mut written = Screen::on("language-frames", tab);
            for at in 0..written.state.rows.len() {
                if let SettingRow::Fixed(id) = written.state.rows[at] {
                    written.focus(id);
                }
            }
            written.frame();
            assert_eq!(written.app.shared.hot.len(), regions, "{tab:?} in {language:?}");
            assert_eq!(written.state.rows, english.state.rows, "{tab:?} in {language:?}");
        }
    }
}

/// The line of keys along the foot of the screen fits the screen's width for every row of every
/// group, and for the states that take the keys, in every language.
#[test]
fn the_line_of_keys_fits_the_screen_in_every_language() {
    let room_em = (CW as f32 - MARGIN * 2.0) / HINT_EM_PX;
    for language in Language::ALL {
        let _installed = InstalledLanguage::new(language);
        for tab in SettingTab::SHOWN {
            let mut screen = Screen::on("language-hints", tab);
            let ids: Vec<SettingId> = screen.state.rows.iter().filter_map(|row| if let SettingRow::Fixed(id) = row { Some(*id) } else { None }).collect();
            for id in ids {
                screen.focus(id);
                let hint = screen.scene().hint;
                assert!(em_width(&hint) <= room_em, "{id:?} in {language:?}: {} em over {room_em}: {hint}", em_width(&hint));
            }
        }
        for hint in [rivals_hint(), confirm_hint(), info::text_edit_hint(), crate::number_row::number_edit_hint()] {
            assert!(em_width(hint) <= room_em, "{language:?}: {hint}");
        }
    }
}

/// A row is called, in English, what the table of rows calls it.
#[test]
fn a_row_is_called_what_the_table_calls_it_in_english() {
    for id in SettingId::ALL {
        assert_eq!(row_label(id), descriptor(id).label, "{id:?}");
    }
}
