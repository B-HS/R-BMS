//! What the settings screen says of its groups and rows beyond what the table of rows says
//! (`rbms_config::SETTINGS`): the sections a group is listed in, and what each row is for.
//!
//! The table names a row, the group it belongs to and the kind of value it holds. It lists the rows
//! of a group in the order they were added, which is not an order to read them in, and its one line
//! of help names a row without saying what its values mean. This file is the screen's own account:
//! a group is cut into sections with a heading over each ([`sections`]), in the order a player
//! looks for the rows, and every row has a few sentences for the panel beside the list
//! ([`description`]).
//!
//! Neither changes what a row is. A row the table lists in a group and no section names is listed
//! under [`OTHER_HEADING`] at the end of its group, and a row with no sentences of its own is
//! described by the table's line of help, so a row added to the table is on the screen and
//! described before anything here is written for it.

use rbms_config::{SettingId, SettingTab, descriptor};
use rbms_model::translations::Catalog;
use rbms_render::text::language;

use crate::settings_ui::SettingRow;

/// The rows listed together under one heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Section {
    pub(crate) heading: &'static str,
    pub(crate) rows: &'static [SettingId],
}

/// The heading over the rows of a group that no section names.
pub(crate) const OTHER_HEADING: &str = "OTHER";

const JUDGE_SECTIONS: &[Section] = &[
    Section {
        heading: "JUDGING",
        rows: &[
            SettingId::JudgeAlgorithm,
            SettingId::JudgeWidthKeyPGreat,
            SettingId::JudgeWidthKeyGreat,
            SettingId::JudgeWidthKeyGood,
            SettingId::JudgeWidthScratchPGreat,
            SettingId::JudgeWidthScratchGreat,
            SettingId::JudgeWidthScratchGood,
        ],
    },
    Section { heading: "LONG NOTES", rows: &[SettingId::LongNoteMargin, SettingId::LnMode] },
    Section {
        heading: "GAUGE",
        rows: &[SettingId::GaugeSet, SettingId::CourseGauge, SettingId::GaugeAutoShift, SettingId::BottomShiftableGauge, SettingId::Total],
    },
];

const DISPLAY_SECTIONS: &[Section] = &[
    Section { heading: "LANGUAGE", rows: &[SettingId::Language] },
    Section { heading: "WINDOW", rows: &[SettingId::WindowMode, SettingId::WindowResolution, SettingId::Letterbox, SettingId::Font] },
    Section { heading: "SCREENS", rows: &[SettingId::Bga, SettingId::ScoreGraph, SettingId::ResultGraphs, SettingId::ReplayAnalysis, SettingId::Preview] },
    Section { heading: "BUILT-IN PLAY FIELD", rows: &[SettingId::Skin, SettingId::FiveKeyLayout, SettingId::WhiteNumber, SettingId::JudgeTextY] },
    Section { heading: "DIAGNOSTICS", rows: &[SettingId::DebugMode] },
];

const INPUT_SECTIONS: &[Section] =
    &[Section { heading: "CONTROLS", rows: &[SettingId::ScratchSide, SettingId::KeyConfig, SettingId::KeyDebounce, SettingId::PadDebounce] }];

const NETWORK_SECTIONS: &[Section] = &[
    Section {
        heading: "SERVER AND ACCOUNT",
        rows: &[
            SettingId::ServerUrl,
            SettingId::PlayerId,
            SettingId::Account,
            SettingId::Email,
            SettingId::Password,
            SettingId::Login,
            SettingId::Register,
            SettingId::Logout,
            SettingId::SyncSettings,
            SettingId::UploadSettings,
            SettingId::DownloadSettings,
        ],
    },
    Section {
        heading: "SCORES",
        rows: &[SettingId::SyncRecords, SettingId::AutoUploadReplay, SettingId::AutoUploadTrace, SettingId::Rivals, SettingId::IrProfiles],
    },
    Section { heading: "DISCORD", rows: &[SettingId::DiscordStatus] },
];

const AUDIO_SECTIONS: &[Section] = &[
    Section { heading: "VOLUME", rows: &[SettingId::MasterVolume, SettingId::KeyVolume, SettingId::BgmVolume, SettingId::SystemVolume] },
    Section { heading: "OUTPUT", rows: &[SettingId::AudioDevice, SettingId::AudioBuffer, SettingId::AudioSampleRate, SettingId::AudioPolyphony] },
    Section { heading: "SYSTEM SOUNDS", rows: &[SettingId::SoundFolder, SettingId::GuideSe] },
];

const SELECT_SECTIONS: &[Section] = &[
    Section { heading: "SONG LIST", rows: &[SettingId::Sort, SettingId::FavoriteOnly, SettingId::ShowMissingCharts, SettingId::RankingKeys] },
    Section { heading: "PREVIEW", rows: &[SettingId::PreviewVolume, SettingId::PreviewFade] },
    Section { heading: "DOWNLOADS", rows: &[SettingId::DownloadFolder] },
];

/// The sections one group of the settings screen is listed in, top to bottom. A group another
/// screen lists has none.
pub(crate) fn sections(tab: SettingTab) -> &'static [Section] {
    match tab {
        SettingTab::Judge => JUDGE_SECTIONS,
        SettingTab::Display => DISPLAY_SECTIONS,
        SettingTab::Input => INPUT_SECTIONS,
        SettingTab::Network => NETWORK_SECTIONS,
        SettingTab::Audio => AUDIO_SECTIONS,
        SettingTab::Select => SELECT_SECTIONS,
        SettingTab::PlaySpeed | SettingTab::PlayLane | SettingTab::PlayJudge | SettingTab::PlayChart | SettingTab::Skin => &[],
    }
}

/// What one group of the settings screen holds, said beside the screen's title.
pub(crate) fn group_summary(tab: SettingTab) -> &'static str {
    match tab {
        SettingTab::Judge => "THE RULES A RUN IS JUDGED AND ITS GAUGE IS KEPT BY",
        SettingTab::Display => "THE WINDOW AND WHAT THE SCREENS SHOW",
        SettingTab::Input => "KEYS, CONTROLLERS AND THE SCRATCH LANE",
        SettingTab::Network => "THE SCORE SERVER, YOUR ACCOUNT ON IT AND DISCORD",
        SettingTab::Audio => "VOLUMES, THE OUTPUT DEVICE AND SYSTEM SOUNDS",
        SettingTab::Select => "THE SONG LIST AND ITS PREVIEW",
        SettingTab::PlaySpeed | SettingTab::PlayLane | SettingTab::PlayJudge | SettingTab::PlayChart | SettingTab::Skin => "",
    }
}

/// The rows a group lists for this configuration, arranged under the headings of its sections: each
/// section with the rows of it that are listed, in the section's order, and then every listed row
/// no section names under [`OTHER_HEADING`], in the order they were listed. A section none of whose
/// rows is listed is left out.
pub(crate) fn arranged(tab: SettingTab, listed: &[SettingRow]) -> Vec<(&'static str, Vec<SettingRow>)> {
    let mut out: Vec<(&'static str, Vec<SettingRow>)> = Vec::new();
    for section in sections(tab) {
        let rows: Vec<SettingRow> = section.rows.iter().map(|id| SettingRow::Fixed(*id)).filter(|row| listed.contains(row)).collect();
        if !rows.is_empty() {
            out.push((section.heading, rows));
        }
    }
    let named = |row: &SettingRow| out.iter().any(|(_, rows)| rows.contains(row));
    let unnamed: Vec<SettingRow> = listed.iter().copied().filter(|row| !named(row)).collect();
    if !unnamed.is_empty() {
        out.push((OTHER_HEADING, unnamed));
    }
    out
}

/// What is said of every JUDGE WIDTH row after what it scales.
macro_rules! judge_width_description {
    ($tier:literal, $lanes:literal) => {
        concat!(
            "Scales the ",
            $tier,
            " window of the ",
            $lanes,
            " lanes, as a share of the width the chart asks for. 100% is the chart's own. ",
            "Narrowing is free; over 100% makes a run a custom judge: no best score, the ASSIST lamp at best; the run is still kept, with its replay, and sent."
        )
    };
}

rbms_model::translations! {
    /// The sentences of the rows added since the screen's words are written in every language.
    pub(crate) enum RowDescriptionText {
        RankingKeys => (
            "Holding a blue key on the chart under the cursor opens the IR RANKING of that chart. A short press does what the key always did, \
             once it is let go. On the keyboard keys 2 and 4 open it, and 9KEYS uses buttons 2 and 8. With OFF the keys work as they always have.",
            "커서가 있는 차트에서 파란 건반을 누르고 있으면 그 차트의 IR RANKING 화면이 열립니다. 짧게 누르면 건반을 뗄 때 원래 동작을 합니다. \
             키보드에서는 2번·4번 키로 열고, 9KEYS 모드에서는 2번·8번 버튼을 씁니다. OFF 상태에서는 건반이 원래대로 동작합니다.",
            "カーソルの譜面で青い鍵盤を押し続けると、その譜面のIR RANKING画面が開きます。短く押すと、離したときに元の動作をします。\
             キーボードでは2番・4番キーで開き、9KEYSモードではボタン2・8を使います。OFFでは鍵盤は元のとおりに動作します。"
        ),
    }
}

/// What one row of the settings screen is for, written on the panel beside the list: what the row
/// changes, and what its values mean where their names do not say. A row with no sentences of its
/// own here is described by the table's line of help.
pub(crate) fn description(id: SettingId) -> &'static str {
    match id {
        SettingId::JudgeAlgorithm => {
            "Which note a press is given to when several notes of its lane are in range. COMBO passes over a note that can no longer be GOOD or better, \
             DURATION takes the one nearest in time, LOWEST always takes the earliest, and SCORE passes over a note that can no longer be GREAT or better."
        }
        SettingId::JudgeWidthKeyPGreat => judge_width_description!("PGREAT", "key"),
        SettingId::JudgeWidthKeyGreat => judge_width_description!("GREAT", "key"),
        SettingId::JudgeWidthKeyGood => judge_width_description!("GOOD", "key"),
        SettingId::JudgeWidthScratchPGreat => judge_width_description!("PGREAT", "scratch"),
        SettingId::JudgeWidthScratchGreat => judge_width_description!("GREAT", "scratch"),
        SettingId::JudgeWidthScratchGood => judge_width_description!("GOOD", "scratch"),
        SettingId::LongNoteMargin => {
            "Scales the window the end of a long note is released in, as a share of the mode's own. 100% is the mode's own. \
             Over 100% makes a run a custom judge: no best score, the ASSIST lamp at best; the run is still kept, with its replay, and sent."
        }
        SettingId::LnMode => {
            "What a long note is played as when its chart does not say: LN (long note), CN (charge note) or HCN (hell charge note). \
             A chart that states the kind is played as it states."
        }
        SettingId::GaugeSet => {
            "Which table the gauges of a run are built from. AUTO takes the table of the chart's mode; LR2 takes the LR2 table for every chart."
        }
        SettingId::CourseGauge => {
            "Which table the gauges of a course are built from, whatever gauge the course itself asks for. LR2 takes the LR2 table for every chart. \
             BEATORAJA takes the table of each chart's mode. IIDX takes the DAN gauge of the arcade game, which takes half the damage at or below 30%."
        }
        SettingId::GaugeAutoShift => {
            "How the gauge may change during a run. NONE fails the run when the gauge empties. CONTINUE plays on with an empty gauge. \
             SURVIVAL TO GROOVE drops an emptied survival gauge to NORMAL. BEST CLEAR keeps the strongest gauge still clearing, \
             and SELECT TO UNDER the strongest at or below the one selected."
        }
        SettingId::BottomShiftableGauge => "The weakest gauge GAUGE AUTO SHIFT may drop a run to: ASSIST EASY, EASY or NORMAL.",
        SettingId::Total => {
            "The TOTAL every chart is played with, which sets how much the gauge gains from a note. AUTO, the lowest step, keeps each chart's own TOTAL; \
             a number from 1 to 5000 is used for every chart instead."
        }
        SettingId::Language => {
            "The language the screens are written in, from the next frame on. AUTO follows the first language the system prefers, \
             and English when the screens are not written in it. Each language is named in itself. Words a skin or a chart supplies stay as they are."
        }
        SettingId::WindowMode => "WINDOWED is an ordinary window of the size RESOLUTION names. BORDERLESS covers the whole monitor the window is on.",
        SettingId::WindowResolution => "The size of the window in WINDOWED mode. A BORDERLESS window takes the size of its monitor instead.",
        SettingId::Letterbox => {
            "ON fits the screen inside the window at its own proportions. OFF stretches it to the window. A BORDERLESS window is always fitted."
        }
        SettingId::Font => {
            "The font the player writes its text with. Choose a TTF, OTF or TTC file; it is loaded at once and again at every start. \
             DEFAULT is the font built into the player."
        }
        SettingId::Bga => "Shows the background animation of a chart while it is played.",
        SettingId::ScoreGraph => "Shows score ranks and score differences on the built-in screens and in the lists of records.",
        SettingId::ResultGraphs => "Draws the gauge, timing and judge distribution graphs on the built-in result screen.",
        SettingId::ReplayAnalysis => "Lets a replay be stepped through and slowed down while it is played back.",
        SettingId::Preview => "Plays a preview of the song under the cursor on the song select. Its volume and fade are in the SELECT group.",
        SettingId::Skin => {
            "The layout of the built-in note field: NORMAL or WIDE. It reads CUSTOM while a layout file named on the command line is in use, \
             and choosing a value here lets go of that file."
        }
        SettingId::FiveKeyLayout => "Lays a five-key chart out on lane widths of its own on the built-in note field, rather than on the seven-key ones.",
        SettingId::WhiteNumber => "Shows the white number on the built-in note field: how long a note takes to reach the bottom of the lane cover.",
        SettingId::JudgeTextY => {
            "Where the judgement is written on the built-in note field, as a share of the screen's height. 0% leaves it where the layout puts it."
        }
        SettingId::DebugMode => "Draws the frame and audio counters on the screen.",
        SettingId::ScratchSide => {
            "The side of the built-in note field the scratch lane is on. A screen drawn by a skin follows the skin's own SCRATCH SIDE option in the PLAY menu."
        }
        SettingId::KeyConfig => "Opens the key editor, where every key and controller button the player reads is listed and can be changed.",
        SettingId::KeyDebounce => {
            "How long a lane key of the keyboard is held after it goes down while a chart is played, so a switch that chatters plays as one press, the way LR2 does. A press is always played the instant it comes; a release inside the window is played when the window ends, if the key is still up then. 0 MS plays every change. A controller keeps its own window."
        }
        SettingId::PadDebounce => {
            "How long a controller button is held after it goes down, on every screen, so a switch that chatters reads as one press, the way LR2 does. A press counts at once; a release inside the window counts when it ends, if still up. A turntable is deaf that long after each change. 0 MS reads every change. Kept in the key config."
        }
        SettingId::ServerUrl => "The address of the score server. Left empty the player stays offline and nothing is sent.",
        SettingId::PlayerId => {
            "The id LOGIN and REGISTER sign in with. While signed in, scores and settings go to the account the token belongs to. Empty is guest, and LOGOUT puts it back to guest."
        }
        SettingId::Account => "The account this player is signed in to. ENTER writes its state on the status line.",
        SettingId::Email => "The address REGISTER creates an account with. LOGIN does not need it.",
        SettingId::Password => {
            "The password for the next LOGIN or REGISTER. It is kept in memory only, is never written to the settings file, and is forgotten once it is used."
        }
        SettingId::Login => "Signs in to the score server with PLAYER ID and PASSWORD.",
        SettingId::Register => "Creates an account on the score server from PLAYER ID, EMAIL and PASSWORD.",
        SettingId::Logout => "Forgets the signed-in account on this machine and puts PLAYER ID back to guest.",
        SettingId::SyncSettings => "Mirrors the settings and the key configuration of this machine to the account.",
        SettingId::UploadSettings => "Sends the settings and the key configuration of this machine to the account now.",
        SettingId::DownloadSettings => "Takes the settings and the key configuration the account holds and applies them to this machine now.",
        SettingId::SyncRecords => {
            "Sends every run kept on this machine that the score server has not taken yet, with its replay. It also runs at start and on sign-in while you are signed in."
        }
        SettingId::AutoUploadReplay => "Sends the replay of a run to the score server after a score that was ranked.",
        SettingId::AutoUploadTrace => {
            "Sends each run's note-by-note record (judgments, gauges, scroll changes) once the server took its score. Only you read it; \
             with sharing on in the web settings, a rival who has you as one too sees its summary by section. \
             Every run keeps it here; an earlier run gets one from its replay when that plays back exactly."
        }
        SettingId::Rivals => "The players shown beside you on a ranking. ENTER opens the list to add and remove them.",
        SettingId::IrProfiles => "More score servers every run is also sent to. ENTER opens the list to add, switch off and remove them.",
        SettingId::DiscordStatus => {
            "Shows R-BMS as the game you are playing on your Discord profile, with the song, its mode and difficulty, the course and how long you have played. \
             Anyone who can see your profile sees it. Needs the Discord desktop app on this computer."
        }
        SettingId::MasterVolume => "The volume of everything the player sounds, applied after the three below are mixed.",
        SettingId::KeyVolume => "The volume of the key sounds: what a note sounds when it is hit.",
        SettingId::BgmVolume => "The volume of the background sounds of a chart.",
        SettingId::SystemVolume => "The volume of the system sounds, which are read from SOUND FOLDER.",
        SettingId::AudioDevice => {
            "The device sound is played on. DEFAULT follows the system's own choice. The line at the foot of this panel says what was opened."
        }
        SettingId::AudioBuffer => "How many frames the output is asked for at a time. AUTO leaves it to the audio backend. Changing it opens the output again.",
        SettingId::AudioSampleRate => {
            "The rate the output is asked to run at, in Hz. AUTO takes the device's own. When the device cannot run at the rate asked for, \
             the line at the foot of this panel says what it was opened at."
        }
        SettingId::AudioPolyphony => "How many sounds the mixer may play at once. Changing it opens the output again.",
        SettingId::SoundFolder => "The folder the system sounds are read from. With none chosen every system sound is silent.",
        SettingId::GuideSe => "Sounds a guide cue at every judgement of a run. The cues are system sounds, read from SOUND FOLDER.",
        SettingId::Sort => "The order of the song list. The sort key of the song select steps the same value.",
        SettingId::FavoriteOnly => "Lists only the charts marked as favourites.",
        SettingId::ShowMissingCharts => "Lists the charts a difficulty table names that the library does not have, dimmed. Choosing one offers its download.",
        SettingId::RankingKeys => RowDescriptionText::RankingKeys.get(language()),
        SettingId::DownloadFolder => {
            "The folder the song list unpacks a chart it downloads into. It is scanned with the song folders. With none chosen, the first download asks for one."
        }
        SettingId::PreviewVolume => "How loud the preview of the song under the cursor is played.",
        SettingId::PreviewFade => "How long the preview takes to fade in and to fade out. 0 MS starts and stops it at once.",
        _ => descriptor(id).help,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rbms_config::{Config, tab_rows};

    /// Every group the screen shows is cut into sections that name each of its rows exactly once
    /// and name no row of another group, so nothing is listed under the catch-all heading and no
    /// section is left empty.
    #[test]
    fn the_sections_of_a_group_name_every_row_of_it_once_and_no_other() {
        let config = Config::default();
        for tab in SettingTab::SHOWN {
            let listed = tab_rows(tab, &config);
            let named: Vec<SettingId> = sections(tab).iter().flat_map(|section| section.rows.iter().copied()).collect();
            for id in &listed {
                assert_eq!(named.iter().filter(|entry| *entry == id).count(), 1, "{tab:?}: {id:?} is named by {} sections", named.len());
            }
            for id in &named {
                assert!(listed.contains(id), "{tab:?}: a section names {id:?}, which the group does not list");
            }
            let rows: Vec<SettingRow> = listed.iter().copied().map(SettingRow::Fixed).collect();
            let under = arranged(tab, &rows);
            assert_eq!(under.len(), sections(tab).len(), "{tab:?}");
            assert!(under.iter().all(|(heading, rows)| *heading != OTHER_HEADING && !rows.is_empty()), "{tab:?}");
            assert_eq!(under.iter().map(|(_, rows)| rows.len()).sum::<usize>(), listed.len(), "{tab:?}");
            assert!(!group_summary(tab).is_empty(), "{tab:?} says nothing of itself");
        }
    }

    /// A row the table lists and no section names is still on the screen: at the end of its group,
    /// under the catch-all heading, in the order it was listed.
    #[test]
    fn a_row_no_section_names_is_listed_at_the_end_of_its_group() {
        let listed = [SettingRow::Fixed(SettingId::Target), SettingRow::Fixed(SettingId::Sort), SettingRow::Fixed(SettingId::Autoplay)];
        let under = arranged(SettingTab::Select, &listed);
        assert_eq!(under.len(), 2);
        assert_eq!(under[0], ("SONG LIST", vec![SettingRow::Fixed(SettingId::Sort)]));
        assert_eq!(under[1], (OTHER_HEADING, vec![SettingRow::Fixed(SettingId::Target), SettingRow::Fixed(SettingId::Autoplay)]));
        assert!(arranged(SettingTab::Skin, &[]).is_empty());
    }

    /// No row of the screen is without sentences of its own, and none of them is the table's bare
    /// line of help.
    #[test]
    fn every_row_the_screen_lists_has_a_description_of_its_own() {
        let config = Config::default();
        for tab in SettingTab::SHOWN {
            for id in tab_rows(tab, &config) {
                let said = description(id);
                assert!(!said.trim().is_empty(), "{id:?} is not described");
                assert_ne!(said, descriptor(id).help, "{id:?} is described by the table's line of help alone");
                assert!(said.ends_with('.'), "{id:?}: {said}");
            }
        }
        assert_eq!(description(SettingId::HiSpeed), descriptor(SettingId::HiSpeed).help, "a row of another screen is described by the table");
    }
}
