//! The key guide of a browser a skin draws.
//!
//! A skin owns the whole screen, so the built-in browser's row of buttons and its guide line are not
//! there, and the keys that open the managers, the search, the filter and the rest have nothing on
//! screen to name them. The guide is the application's own panel over the skin that does: one key
//! ([`GUIDE_KEY`]) puts it up and the same key, Escape or Enter takes it down, and while it is up it
//! takes every key and every press of the mouse.
//!
//! What it lists is read rather than written out. Every key that can be rebound is asked of the key
//! configuration as the guide is drawn: the browser's shortcuts and the keys to its screens
//! ([`Shortcut`]), START, SELECT and the controls of a run ([`ControlAction`]), and the keys of the
//! reference's key table ([`KeyLayout::indices_of`]). A key the player moved is the key the guide
//! names, and one they left without a key is shown as having none. The few keys no screen lets go of
//! ([`FIXED_KEYS`]) are the only ones written down here.

use rbms_model::translations::Catalog;
use rbms_play::ScratchDir;
use rbms_render::text::language;
use rbms_render::{Color, Rect, Renderer, draw_text, draw_text_right, fit_text, theme};
use winit::keyboard::KeyCode;

use super::SelectState;
use super::keys::{KeyLayout, SelectKey};
pub(super) use crate::keyconfig::GUIDE_KEY;
use crate::keyconfig::{ControlAction, SCRATCH_BACKWARD_INDEX, SCRATCH_FORWARD_INDEX, Shortcut, ShortcutGroup, key_index_of, key_name};
use crate::text::select::guide::GuideText;
use crate::{AppShared, CH, CW, Hot};

/// What a key that is bound to nothing is called on the guide.
const UNBOUND: &str = "-";

/// One of the browser's keys that cannot be rebound, as the guide lists it.
pub(super) struct FixedKey {
    pub(super) keys: &'static [KeyCode],
    pub(super) does: GuideText,
}

/// The keys of the browser that are its own and fixed: the ones that move through the list, open
/// what is under the cursor and go back.
pub(super) const FIXED_KEYS: [FixedKey; 3] = [
    FixedKey { keys: &[KeyCode::ArrowUp, KeyCode::ArrowDown], does: GuideText::MoveTheList },
    FixedKey { keys: &[KeyCode::Enter], does: GuideText::OpenFolderOrPlay },
    FixedKey { keys: &[KeyCode::Escape], does: GuideText::BackTwiceQuits },
];

/// What each role of the reference's key table is called on the guide, in the order they are listed.
const KEY_ROLES: [(SelectKey, GuideText); 7] = [
    (SelectKey::Play, GuideText::PlayTheChart),
    (SelectKey::Practice, GuideText::Practice),
    (SelectKey::Auto, GuideText::PlayItByItself),
    (SelectKey::FolderOpen, GuideText::OpenAFolder),
    (SelectKey::FolderClose, GuideText::BackOneFolder),
    (SelectKey::Up, GuideText::NextBar),
    (SelectKey::Down, GuideText::PreviousBar),
];

/// The controls of a run the guide lists two to a line: the pair of keys, the one that lessens
/// first, and what the pair moves.
const CONTROL_PAIRS: [(ControlAction, ControlAction, GuideText); 6] = [
    (ControlAction::HiSpeedDown, ControlAction::HiSpeedUp, GuideText::HiSpeedDownUp),
    (ControlAction::CoverDown, ControlAction::CoverUp, GuideText::LaneCoverLessMore),
    (ControlAction::LiftDown, ControlAction::LiftUp, GuideText::LiftDownUp),
    (ControlAction::HiddenDown, ControlAction::HiddenUp, GuideText::HiddenLessMore),
    (ControlAction::GreenDown, ControlAction::GreenUp, GuideText::GreenNumberDownUp),
    (ControlAction::OffsetDown, ControlAction::OffsetUp, GuideText::JudgeOffsetHold),
];

/// The controls of a run the guide lists one to a line.
const CONTROL_SWITCHES: [(ControlAction, GuideText); 5] = [
    (ControlAction::CoverToggle, GuideText::LaneCoverOnOff),
    (ControlAction::LiftToggle, GuideText::LiftOnOff),
    (ControlAction::HiddenToggle, GuideText::HiddenOnOff),
    (ControlAction::KeepGreenToggle, GuideText::KeepGreenNumber),
    (ControlAction::AutoAdjustToggle, GuideText::AutoAdjust),
];

/// The panel's width and the room it keeps round what it lists.
const GUIDE_W: f32 = 1232.0;
const PAD_X: f32 = 24.0;
const TITLE_Y: f32 = 16.0;
const FIRST_SECTION_Y: f32 = 60.0;
const SECTION_H: f32 = 30.0;
const SECTION_GAP: f32 = 12.0;
const ROW_PITCH: f32 = 24.0;
const BOTTOM_PAD: f32 = 20.0;
const COLUMN_GAP: f32 = 28.0;
const COLUMN_COUNT: usize = 3;
const KEY_COLUMN_W: f32 = 148.0;
const KEY_COLUMN_GAP: f32 = 8.0;

/// The rule under a column's heading and the one along the top of the panel: how far above the first
/// line the first stands, and how thick either is.
const HEADING_RULE_RISE: f32 = 8.0;
const RULE_THICKNESS: f32 = 2.0;

/// How far the hint at the right of the title is let down, so it sits on the title's baseline.
const HINT_DROP: f32 = 8.0;

/// The text sizes of the panel.
const TITLE_SCALE: f32 = 2.0;
const SECTION_SCALE: f32 = 1.2;
const ROW_SCALE: f32 = 1.2;
const HINT_SCALE: f32 = 1.0;

/// How opaque what is behind the panel is dimmed to.
const BACKDROP_ALPHA: u8 = 150;

/// What a key is called on the guide: the word the key configuration has for it, with the two the
/// guide writes shorter and the one key no configuration names.
pub(super) fn shortcut_label(code: KeyCode) -> &'static str {
    match code {
        KeyCode::Slash => "/",
        KeyCode::Escape => "ESC",
        other => key_name(other),
    }
}

/// What a key that may be bound to nothing is called.
fn bound_label(code: Option<KeyCode>) -> &'static str {
    code.map_or(UNBOUND, shortcut_label)
}

/// The keys of one entry, as one string.
fn keys_label(keys: &[KeyCode]) -> String {
    keys.iter().map(|code| shortcut_label(*code)).collect::<Vec<_>>().join(" ")
}

/// One line of the guide: the keys, and what they do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuideRow {
    pub(super) keys: String,
    pub(super) does: String,
}

impl GuideRow {
    fn new(keys: impl Into<String>, does: impl Into<String>) -> GuideRow {
        GuideRow { keys: keys.into(), does: does.into() }
    }
}

/// One heading of the guide and the lines under it.
pub(super) struct Section {
    pub(super) heading: &'static str,
    pub(super) rows: Vec<GuideRow>,
}

/// What a shortcut is called on the guide, in the language the screens are written in: the name the
/// key editor gives it, except where a line here is shorter than a row of the key editor.
fn shortcut_does(shortcut: Shortcut) -> &'static str {
    let named = match shortcut {
        Shortcut::Sort => GuideText::ShortcutSort,
        Shortcut::Records => GuideText::ShortcutRecords,
        Shortcut::Settings => GuideText::ShortcutSettings,
        Shortcut::RefDetail => GuideText::ShortcutDetailOptions,
        Shortcut::Search | Shortcut::RefSearch => GuideText::Search,
        Shortcut::Filter => GuideText::FilterPanel,
        Shortcut::Practice => GuideText::Practice,
        Shortcut::Favourite | Shortcut::RefFavourite => GuideText::Favourite,
        Shortcut::Ranking => GuideText::IrRanking,
        Shortcut::ReloadTables => GuideText::ReloadTables,
        Shortcut::SkinMenu => GuideText::SkinMenu,
        Shortcut::PlayMenu => GuideText::PlayMenu,
        Shortcut::KeyConfig | Shortcut::RefKeyConfig => GuideText::KeyConfig,
        Shortcut::Folders => GuideText::MusicFolders,
        Shortcut::Tables => GuideText::DifficultyTables,
        Shortcut::RefMode => GuideText::ModeFilter,
        Shortcut::RefSort => GuideText::SortOrder,
        Shortcut::RefLnMode => GuideText::LongNoteMode,
        Shortcut::RefReplay => GuideText::ReplaySlot,
        other => return other.label(),
    };
    named.get(language())
}

/// The key of the keyboard bound to the key index `index` of the reference's table in the mode that
/// is up, or [`UNBOUND`] for an index no key of the keyboard is on.
fn index_label(shared: &AppShared, index: usize) -> &'static str {
    let forward = shared.active_keys.iter().find(|(_, lane)| key_index_of(shared.mode, *lane, ScratchDir::Forward) == Some(index)).map(|(code, _)| *code);
    let backward =
        shared.active_reverse_keys.iter().find(|(_, lane)| key_index_of(shared.mode, *lane, ScratchDir::Backward) == Some(index)).map(|(code, _)| *code);
    bound_label(forward.or(backward))
}

/// The key a control is read from right now, or [`UNBOUND`].
fn control_label(shared: &AppShared, action: ControlAction) -> &'static str {
    bound_label(shared.control_key_in_force(action))
}

/// The lines of one group of shortcuts, each with the key it is on now.
fn group_rows(shared: &AppShared, group: ShortcutGroup) -> Vec<GuideRow> {
    Shortcut::ALL
        .into_iter()
        .filter(|shortcut| shortcut.group() == group && !matches!(shortcut, Shortcut::OptionsPin | Shortcut::OptionsHold))
        .map(|shortcut| GuideRow::new(bound_label(shared.shortcut_key(shortcut)), shortcut_does(shortcut)))
        .collect()
}

/// The lines of the browser's own keys: the fixed ones, its shortcuts as they are bound, the two
/// keys of the option overlay on one line, and the guide's.
pub(super) fn browser_rows(shared: &AppShared) -> Vec<GuideRow> {
    let language = language();
    let fixed = FIXED_KEYS.iter().map(|fixed| GuideRow::new(keys_label(fixed.keys), fixed.does.get(language)));
    let options = format!("{} {}", bound_label(shared.shortcut_key(Shortcut::OptionsPin)), bound_label(shared.shortcut_key(Shortcut::OptionsHold)));
    fixed
        .chain(group_rows(shared, ShortcutGroup::Browser))
        .chain([GuideRow::new(options, GuideText::OptionsOverlay.get(language)), GuideRow::new(shortcut_label(GUIDE_KEY), GuideText::ThisGuide.get(language))])
        .collect()
}

/// The lines of the keys that open a screen from the browser.
pub(super) fn screen_rows(shared: &AppShared) -> Vec<GuideRow> {
    group_rows(shared, ShortcutGroup::Screens)
}

/// The lines of the reference's number keys, which a browser a skin draws reads.
pub(super) fn number_rows(shared: &AppShared) -> Vec<GuideRow> {
    group_rows(shared, ShortcutGroup::SkinBrowser)
}

/// The lines of the three option panels: the keys held for each, as they are bound now, then what
/// the turntable does on each. What keys one to seven do on a panel is the skin's to show. The
/// turntable of the second and third panels does something only on a skin that names this player's
/// own ids; to any other it is the reference's, which gives it nothing there.
pub(super) fn panel_rows(shared: &AppShared) -> Vec<GuideRow> {
    let language = language();
    let start = control_label(shared, ControlAction::Start);
    let select = control_label(shared, ControlAction::Select);
    let detail = bound_label(shared.shortcut_key(Shortcut::RefDetail));
    let mut rows = vec![
        GuideRow::new(start, GuideText::HoldForPlayOptions.get(language)),
        GuideRow::new(select, GuideText::HoldForAssistOptions.get(language)),
        GuideRow::new(format!("{start} + {select} / {detail}"), GuideText::HoldForDetailOptions.get(language)),
        GuideRow::new(GuideText::PlusTurntable.fill(language, &[&start]), GuideText::Target.get(language)),
    ];
    if KeyLayout::of_browser(shared).is_extended() {
        rows.extend([
            GuideRow::new(GuideText::PlusTurntable.fill(language, &[&select]), GuideText::HiddenOnOff.get(language)),
            GuideRow::new(GuideText::PlusTurntable.fill(language, &[&format!("{start}+{select}")]), GuideText::JudgeAlgorithm.get(language)),
        ]);
    }
    rows
}

/// The lines of the keys of the browser's table: for each role, the keyboard keys that are on the
/// indices that carry it in the mode that is up.
pub(super) fn key_rows(shared: &AppShared) -> Vec<GuideRow> {
    let layout = KeyLayout::of_browser(shared);
    let language = language();
    KEY_ROLES
        .iter()
        .map(|(role, does)| {
            let keys = layout.indices_of(*role).into_iter().map(|index| index_label(shared, index)).collect::<Vec<_>>().join(" ");
            GuideRow::new(keys, does.get(language))
        })
        .collect()
}

/// The lines of the controls of a run, as they are bound now: the ones that come in pairs two to a
/// line, then each switch, then the modifier held for the finer step, which is called the shift that
/// plays no lane ([`AppShared::fine_pressed`]) while it has no key of its own.
pub(super) fn play_rows(shared: &AppShared) -> Vec<GuideRow> {
    let language = language();
    let pairs = CONTROL_PAIRS
        .iter()
        .map(|(less, more, does)| GuideRow::new(format!("{} {}", control_label(shared, *less), control_label(shared, *more)), does.get(language)));
    let switches = CONTROL_SWITCHES.iter().map(|(action, does)| GuideRow::new(control_label(shared, *action), does.get(language)));
    let fine = shared.control_key_in_force(ControlAction::Fine).map_or(GuideText::ShiftNoLane.get(language), shortcut_label);
    pairs.chain(switches).chain([GuideRow::new(fine, GuideText::HoldFineStep.get(language))]).collect()
}

/// The lines of what START and SELECT do in a run with the keys held beside them
/// (`ControlInputProcessor.input`, and the turntable with both, which is this player's own), named
/// by the keys they are bound to now. The first line names the keys of the keyboard the lines
/// after it call the turntable, and which way each moves a shade.
pub(super) fn combination_rows(shared: &AppShared) -> Vec<GuideRow> {
    let language = language();
    let start = control_label(shared, ControlAction::Start);
    let select = control_label(shared, ControlAction::Select);
    let turntable = format!("{} {}", index_label(shared, SCRATCH_FORWARD_INDEX), index_label(shared, SCRATCH_BACKWARD_INDEX));
    vec![
        GuideRow::new(turntable, GuideText::TurntableDownUp.get(language)),
        GuideRow::new(GuideText::PlusKeys.fill(language, &[&start]), GuideText::HiSpeed.get(language)),
        GuideRow::new(GuideText::PlusTurntable.fill(language, &[&start]), GuideText::LaneCoverByOneGreenKept.get(language)),
        GuideRow::new(GuideText::PlusTurntable.fill(language, &[&format!("{start}+{select}")]), GuideText::LiftByOne.get(language)),
        GuideRow::new(GuideText::Twice.fill(language, &[&start]), GuideText::LaneCoverOnOff.get(language)),
        GuideRow::new(GuideText::PlusKeys.fill(language, &[&select]), GuideText::GreenNumberByOneMs.get(language)),
        GuideRow::new(format!("{start} + {select}"), GuideText::LiftOrHiddenTurns.get(language)),
        GuideRow::new(GuideText::MouseWheel.get(language), GuideText::LaneCover.get(language)),
        GuideRow::new(shortcut_label(KeyCode::Escape), GuideText::StopTheChart.get(language)),
    ]
}

/// Everything the guide lists, column by column.
pub(super) fn columns(shared: &AppShared) -> [Vec<Section>; COLUMN_COUNT] {
    let language = language();
    [
        vec![
            Section { heading: GuideText::SectionSelect.get(language), rows: browser_rows(shared) },
            Section { heading: GuideText::SectionScreens.get(language), rows: screen_rows(shared) },
        ],
        vec![
            Section { heading: GuideText::SectionNumberKeys.get(language), rows: number_rows(shared) },
            Section { heading: GuideText::SectionOptionPanels.get(language), rows: panel_rows(shared) },
            Section { heading: GuideText::SectionKeysAndTurntable.get(language), rows: key_rows(shared) },
        ],
        vec![
            Section { heading: GuideText::SectionInPlay.get(language), rows: play_rows(shared) },
            Section { heading: GuideText::SectionInPlayTogether.get(language), rows: combination_rows(shared) },
        ],
    ]
}

/// How far a column reaches below where the first heading starts.
fn column_extent(sections: &[Section]) -> f32 {
    let gaps = sections.len().saturating_sub(1) as f32 * SECTION_GAP;
    sections.iter().map(|section| SECTION_H + section.rows.len() as f32 * ROW_PITCH).sum::<f32>() + gaps
}

/// Where the panel is: centred on the screen the application lays itself out for, and tall enough
/// for the longest of its columns.
pub(super) fn guide_rect(columns: &[Vec<Section>]) -> Rect {
    let tallest = columns.iter().map(|sections| column_extent(sections)).fold(0.0, f32::max);
    let height = FIRST_SECTION_Y + tallest + BOTTOM_PAD;
    Rect::new((CW as f32 - GUIDE_W) * 0.5, (CH as f32 - height) * 0.5, GUIDE_W, height)
}

/// Draws one section: its heading, then a line for each row, the keys in a column of their own.
/// Answers where the section ends.
fn draw_section<R: Renderer>(r: &mut R, x: f32, y: f32, width: f32, section: &Section) -> f32 {
    let th = theme();
    draw_text(r, x, y, SECTION_SCALE, th.accent, section.heading);
    r.fill_rect(Rect::new(x, y + SECTION_H - HEADING_RULE_RISE, width, RULE_THICKNESS), th.divider);
    let does_width = width - KEY_COLUMN_W;
    for (index, row) in section.rows.iter().enumerate() {
        let row_y = y + SECTION_H + index as f32 * ROW_PITCH;
        draw_text(r, x, row_y, ROW_SCALE, th.focus, &fit_text(&row.keys, ROW_SCALE, KEY_COLUMN_W - KEY_COLUMN_GAP));
        draw_text(r, x + KEY_COLUMN_W, row_y, ROW_SCALE, th.text, &fit_text(&row.does, ROW_SCALE, does_width));
    }
    y + SECTION_H + section.rows.len() as f32 * ROW_PITCH
}

/// Draw the guide over the screen, and answer what a press on it hits: everything on the screen is
/// the guide's to close it, and the panel itself takes a press without closing it.
pub(super) fn render_guide<R: Renderer>(r: &mut R, shared: &AppShared) -> Vec<(Rect, Hot)> {
    let th = theme();
    let columns = columns(shared);
    let screen = Rect::new(0.0, 0.0, CW as f32, CH as f32);
    let panel = guide_rect(&columns);
    r.fill_rect(screen, Color { r: 0, g: 0, b: 0, a: BACKDROP_ALPHA });
    r.fill_rect(panel, th.panel_hi);
    r.fill_rect(Rect::new(panel.x, panel.y, panel.w, RULE_THICKNESS), th.focus);
    draw_text(r, panel.x + PAD_X, panel.y + TITLE_Y, TITLE_SCALE, th.text, GuideText::Title.get(language()));
    let hint =
        GuideText::Hint.fill(language(), &[&shortcut_label(GUIDE_KEY), &shortcut_label(GUIDE_KEY), &bound_label(shared.shortcut_key(Shortcut::KeyConfig))]);
    draw_text_right(r, panel.x + panel.w - PAD_X, panel.y + TITLE_Y + HINT_DROP, HINT_SCALE, th.text_muted, &hint);

    let column_w = (panel.w - PAD_X * 2.0 - COLUMN_GAP * (COLUMN_COUNT - 1) as f32) / COLUMN_COUNT as f32;
    for (index, sections) in columns.iter().enumerate() {
        let x = panel.x + PAD_X + index as f32 * (column_w + COLUMN_GAP);
        let mut y = panel.y + FIRST_SECTION_Y;
        for section in sections {
            y = draw_section(r, x, y, column_w, section) + SECTION_GAP;
        }
    }
    vec![(screen, Hot::GuideClose), (panel, Hot::OverlayPanel)]
}

impl SelectState {
    /// One key while the guide is up: it closes on its own key, Escape and Enter, and takes every
    /// other key without doing anything with it, so the list under it cannot move.
    pub(super) fn guide_key(&mut self, code: KeyCode) {
        if matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter) || code == GUIDE_KEY {
            self.guide_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::Language;

    use super::*;
    use crate::stage::select::tests::app;
    use crate::text::settings::InstalledLanguage;

    /// The shortcuts whose line on the guide is not the name the key editor gives them.
    const SHORTER_ON_THE_GUIDE: [Shortcut; 4] = [Shortcut::Sort, Shortcut::Records, Shortcut::Settings, Shortcut::RefDetail];

    /// The shortcuts the guide lists a line for, one each.
    fn listed() -> impl Iterator<Item = Shortcut> {
        Shortcut::ALL.into_iter().filter(|shortcut| {
            matches!(shortcut.group(), ShortcutGroup::Browser | ShortcutGroup::Screens | ShortcutGroup::SkinBrowser)
                && !matches!(shortcut, Shortcut::OptionsPin | Shortcut::OptionsHold)
        })
    }

    #[test]
    fn a_shortcut_is_called_on_the_guide_as_the_key_editor_calls_it_unless_the_guide_has_a_shorter_line() {
        for shortcut in listed().filter(|shortcut| !SHORTER_ON_THE_GUIDE.contains(shortcut)) {
            assert_eq!(shortcut_does(shortcut), shortcut.label(), "{shortcut:?}");
        }
        assert_eq!(SHORTER_ON_THE_GUIDE.map(shortcut_does), ["SORT (SHIFT: BACKWARDS)", "RECORDS", "SETTINGS (SHIFT: COURSES)", "HOLD: DETAIL OPTIONS"]);
    }

    #[test]
    fn every_shortcut_the_guide_lists_is_named_in_every_language_but_the_terms_the_glossary_keeps() {
        for language in Language::ALL {
            let _installed = InstalledLanguage::new(language);
            for shortcut in listed() {
                let name = shortcut_does(shortcut);
                let kept = shortcut == Shortcut::Ranking;
                assert!(language == Language::English || kept || !name.is_ascii(), "{language:?}: {shortcut:?} is named {name:?} in English");
            }
        }
    }

    #[test]
    fn the_rows_are_worded_in_the_installed_language_and_keep_the_terms_every_language_keeps() {
        let app = app();
        let _korean = InstalledLanguage::new(Language::Korean);
        let rows: Vec<(String, String)> = columns(&app.shared).into_iter().flatten().flat_map(|section| section.rows).map(|row| (row.keys, row.does)).collect();
        let does = |wanted: &str| rows.iter().any(|(_, does)| does == wanted);
        for wanted in [
            "목록 이동",
            "폴더 열기 / 플레이",
            "HI-SPEED 내리기 / 올리기",
            "레인 커버 ON/OFF",
            "LIFT ON/OFF",
            "누르고 있기: 미세 조정",
            "턴테이블: 내리기 / 올리기",
        ] {
            assert!(does(wanted), "{wanted:?} is not on the guide: {rows:#?}");
        }
        assert!(rows.iter().any(|(keys, does)| keys.ends_with("+ 턴테이블") && does == "타깃"), "{rows:#?}");
        assert!(rows.iter().any(|(keys, does)| keys.ends_with("두 번") && does == "레인 커버 ON/OFF"), "{rows:#?}");
        let headings: Vec<&str> = columns(&app.shared).into_iter().flatten().map(|section| section.heading).collect();
        assert_eq!(headings, ["선곡", "화면", "숫자 키", "옵션 패널", "건반 1-7 및 턴테이블", "플레이 중", "플레이 중, 함께 누르기"]);
    }

    #[test]
    fn the_guide_has_the_same_lines_in_every_language() {
        let app = app();
        let counts = |language| {
            let _installed = InstalledLanguage::new(language);
            columns(&app.shared).iter().map(|sections| sections.iter().map(|section| section.rows.len()).collect::<Vec<_>>()).collect::<Vec<_>>()
        };
        for language in Language::ALL {
            assert_eq!(counts(language), counts(Language::English), "{language:?}");
        }
    }
}
