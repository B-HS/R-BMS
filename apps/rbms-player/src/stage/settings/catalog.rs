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
//! under the heading for the rest ([`other_heading`]) at the end of its group, and a row with no
//! sentences of its own is described by the table's line of help, so a row added to the table is
//! on the screen and described before anything here is written for it.
//!
//! The sections, and the rows they hold, are named in English: a section is told from another by
//! its heading, and the table's own rows are the English of `rbms_config`. What the screen writes
//! is the language's rendering of them, which lives with the other fixed words in
//! [`crate::text::settings`].

use rbms_config::{SettingId, SettingTab, descriptor};
use rbms_model::translations::Catalog;
use rbms_render::text::language;

use crate::settings_ui::SettingRow;
use crate::text::settings::{other_heading, section_heading};

pub(crate) use crate::text::settings::descriptions::RowDescriptionText;
pub(crate) use crate::text::settings::group_summary;

/// The rows listed together under one heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Section {
    pub(crate) heading: &'static str,
    pub(crate) rows: &'static [SettingId],
}

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

/// The rows a group lists for this configuration, arranged under the headings of its sections: each
/// section with the rows of it that are listed, in the section's order, and then every listed row
/// no section names under [`other_heading`], in the order they were listed. A section none of whose
/// rows is listed is left out. The headings are written in the language the screens are drawn in.
pub(crate) fn arranged(tab: SettingTab, listed: &[SettingRow]) -> Vec<(&'static str, Vec<SettingRow>)> {
    let mut out: Vec<(&'static str, Vec<SettingRow>)> = Vec::new();
    for section in sections(tab) {
        let rows: Vec<SettingRow> = section.rows.iter().map(|id| SettingRow::Fixed(*id)).filter(|row| listed.contains(row)).collect();
        if !rows.is_empty() {
            out.push((section_heading(section.heading), rows));
        }
    }
    let named = |row: &SettingRow| out.iter().any(|(_, rows)| rows.contains(row));
    let unnamed: Vec<SettingRow> = listed.iter().copied().filter(|row| !named(row)).collect();
    if !unnamed.is_empty() {
        out.push((other_heading(), unnamed));
    }
    out
}

/// The sentence every JUDGE WIDTH row shares, with the judgement it scales and the lanes it is for.
fn judge_width_description(tier: &str, lanes: RowDescriptionText) -> String {
    RowDescriptionText::JudgeWidth.fill(language(), &[&tier, &lanes.get(language())])
}

/// What one row of the settings screen is for, written on the panel beside the list: what the row
/// changes, and what its values mean where their names do not say. A row with no sentences of its
/// own here is described by the table's line of help.
pub(crate) fn description(id: SettingId) -> String {
    let entry = match id {
        SettingId::JudgeAlgorithm => RowDescriptionText::JudgeAlgorithm,
        SettingId::JudgeWidthKeyPGreat => return judge_width_description("PGREAT", RowDescriptionText::LanesOfKeys),
        SettingId::JudgeWidthKeyGreat => return judge_width_description("GREAT", RowDescriptionText::LanesOfKeys),
        SettingId::JudgeWidthKeyGood => return judge_width_description("GOOD", RowDescriptionText::LanesOfKeys),
        SettingId::JudgeWidthScratchPGreat => return judge_width_description("PGREAT", RowDescriptionText::LanesOfScratch),
        SettingId::JudgeWidthScratchGreat => return judge_width_description("GREAT", RowDescriptionText::LanesOfScratch),
        SettingId::JudgeWidthScratchGood => return judge_width_description("GOOD", RowDescriptionText::LanesOfScratch),
        SettingId::LongNoteMargin => RowDescriptionText::LongNoteMargin,
        SettingId::LnMode => RowDescriptionText::LnMode,
        SettingId::GaugeSet => RowDescriptionText::GaugeSet,
        SettingId::CourseGauge => RowDescriptionText::CourseGauge,
        SettingId::GaugeAutoShift => RowDescriptionText::GaugeAutoShift,
        SettingId::BottomShiftableGauge => RowDescriptionText::BottomShiftableGauge,
        SettingId::Total => RowDescriptionText::Total,
        SettingId::Language => RowDescriptionText::Language,
        SettingId::WindowMode => RowDescriptionText::WindowMode,
        SettingId::WindowResolution => RowDescriptionText::WindowResolution,
        SettingId::Letterbox => RowDescriptionText::Letterbox,
        SettingId::Font => RowDescriptionText::Font,
        SettingId::Bga => RowDescriptionText::Bga,
        SettingId::ScoreGraph => RowDescriptionText::ScoreGraph,
        SettingId::ResultGraphs => RowDescriptionText::ResultGraphs,
        SettingId::ReplayAnalysis => RowDescriptionText::ReplayAnalysis,
        SettingId::Preview => RowDescriptionText::Preview,
        SettingId::Skin => RowDescriptionText::Skin,
        SettingId::FiveKeyLayout => RowDescriptionText::FiveKeyLayout,
        SettingId::WhiteNumber => RowDescriptionText::WhiteNumber,
        SettingId::JudgeTextY => RowDescriptionText::JudgeTextY,
        SettingId::DebugMode => RowDescriptionText::DebugMode,
        SettingId::ScratchSide => RowDescriptionText::ScratchSide,
        SettingId::KeyConfig => RowDescriptionText::KeyConfig,
        SettingId::KeyDebounce => RowDescriptionText::KeyDebounce,
        SettingId::PadDebounce => RowDescriptionText::PadDebounce,
        SettingId::ServerUrl => RowDescriptionText::ServerUrl,
        SettingId::PlayerId => RowDescriptionText::PlayerId,
        SettingId::Account => RowDescriptionText::Account,
        SettingId::Email => RowDescriptionText::Email,
        SettingId::Password => RowDescriptionText::Password,
        SettingId::Login => RowDescriptionText::Login,
        SettingId::Register => RowDescriptionText::Register,
        SettingId::Logout => RowDescriptionText::Logout,
        SettingId::SyncSettings => RowDescriptionText::SyncSettings,
        SettingId::UploadSettings => RowDescriptionText::UploadSettings,
        SettingId::DownloadSettings => RowDescriptionText::DownloadSettings,
        SettingId::SyncRecords => RowDescriptionText::SyncRecords,
        SettingId::AutoUploadReplay => RowDescriptionText::AutoUploadReplay,
        SettingId::AutoUploadTrace => RowDescriptionText::AutoUploadTrace,
        SettingId::Rivals => RowDescriptionText::Rivals,
        SettingId::IrProfiles => RowDescriptionText::IrProfiles,
        SettingId::DiscordStatus => RowDescriptionText::DiscordStatus,
        SettingId::MasterVolume => RowDescriptionText::MasterVolume,
        SettingId::KeyVolume => RowDescriptionText::KeyVolume,
        SettingId::BgmVolume => RowDescriptionText::BgmVolume,
        SettingId::SystemVolume => RowDescriptionText::SystemVolume,
        SettingId::AudioDevice => RowDescriptionText::AudioDevice,
        SettingId::AudioBuffer => RowDescriptionText::AudioBuffer,
        SettingId::AudioSampleRate => RowDescriptionText::AudioSampleRate,
        SettingId::AudioPolyphony => RowDescriptionText::AudioPolyphony,
        SettingId::SoundFolder => RowDescriptionText::SoundFolder,
        SettingId::GuideSe => RowDescriptionText::GuideSe,
        SettingId::Sort => RowDescriptionText::Sort,
        SettingId::FavoriteOnly => RowDescriptionText::FavoriteOnly,
        SettingId::ShowMissingCharts => RowDescriptionText::ShowMissingCharts,
        SettingId::RankingKeys => RowDescriptionText::RankingKeys,
        SettingId::DownloadFolder => RowDescriptionText::DownloadFolder,
        SettingId::PreviewVolume => RowDescriptionText::PreviewVolume,
        SettingId::PreviewFade => RowDescriptionText::PreviewFade,
        _ => return descriptor(id).help.to_string(),
    };
    entry.get(language()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::settings::InstalledLanguage;
    use rbms_config::{Config, tab_rows};
    use rbms_model::Language;

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
            assert!(under.iter().all(|(heading, rows)| *heading != other_heading() && !rows.is_empty()), "{tab:?}");
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
        assert_eq!(under[1], (other_heading(), vec![SettingRow::Fixed(SettingId::Target), SettingRow::Fixed(SettingId::Autoplay)]));
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

    /// The sentence the six JUDGE WIDTH rows share reads, in English, as it did when each row had
    /// its own copy of it.
    #[test]
    fn the_judge_width_sentence_is_the_english_it_was_with_the_judgement_and_the_lanes_put_in() {
        assert_eq!(
            description(SettingId::JudgeWidthScratchGood),
            "Scales the GOOD window of the scratch lanes, as a share of the width the chart asks for. 100% is the chart's own. \
             Narrowing is free; over 100% makes a run a custom judge: no best score, the ASSIST lamp at best; the run is still kept, with its replay, and sent."
        );
        assert!(description(SettingId::JudgeWidthKeyPGreat).starts_with("Scales the PGREAT window of the key lanes, "));
    }

    /// Every row the screen lists is described in every language, with nothing left to fill in, and
    /// the Korean and the Japanese are not the English over again.
    #[test]
    fn every_row_the_screen_lists_is_described_in_every_language() {
        let config = Config::default();
        for tab in SettingTab::SHOWN {
            for id in tab_rows(tab, &config) {
                let english = description(id);
                for language in Language::ALL {
                    let _installed = InstalledLanguage::new(language);
                    let said = description(id);
                    assert!(!said.trim().is_empty() && !said.contains('{') && !said.contains('}'), "{id:?} in {language:?}: {said}");
                    assert_eq!(said == english, language == Language::English, "{id:?} in {language:?}");
                }
            }
        }
    }

    /// Every section has a heading in every language, and a language that writes one differently
    /// finds it by the English the sections are named by.
    #[test]
    fn every_section_is_headed_in_every_language() {
        for tab in SettingTab::SHOWN {
            for section in sections(tab) {
                let _korean = InstalledLanguage::new(Language::Korean);
                let heading = section_heading(section.heading);
                assert!(!heading.is_empty() && (heading != section.heading || heading == "DISCORD"), "{tab:?}: {} has no Korean heading", section.heading);
            }
        }
        let _japanese = InstalledLanguage::new(Language::Japanese);
        let rows = [SettingRow::Fixed(SettingId::Target)];
        assert_eq!(arranged(SettingTab::Select, &rows), vec![("その他", vec![SettingRow::Fixed(SettingId::Target)])]);
    }
}
