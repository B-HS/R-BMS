//! Property cluster E: the player's settings.
//!
//! Where the values come from: what the player chose: gauge, random and double options, hi-speed
//! and its fix, BGA, assists, long note mode, auto-save slots and the other switches a skin shows
//! as a picked image.
//!
//! On a result screen these are the choices the run was played with, kept in its
//! [`ResultSnapshot`](super::result::snapshot::ResultSnapshot) as [`PlayedOptions`], and the gauge the
//! screen has the graph on; the cluster reads them from there ([`OptionsState::of_result`]). The settings of the browser and the play screen, which
//! read the live configuration, are answered when those screens bring theirs.
//!
//! What this cluster answers on a result screen: image indices 40 (the gauge the graph is on), 42-43
//! (the random options), 54 (the double option), 61-63, 89-90 and 301-308; options 60-62. The
//! indices that show a setting of the configuration screens -- hi-speed, lane cover, judge
//! algorithm and the like (55, 72, 75, 78, 321-324, 330-332, 340-343, 350-353, 360-361, 400) -- and
//! the numbers 10, 12, 57-59 and 310-313, float 310 and rates 17-19 are not answered yet.
//!
//! A value is read as `IntegerPropertyFactory.IndexType` and `BooleanPropertyFactory` read it for a
//! result screen: the random options are the ones the replay recorded, the gauge is the one the
//! graph is switched to, and the options of the target score are absent because the target here is
//! a score and not a play. `no_save_clear` (62) is defined as the opposite of what its name says,
//! the same as `enable_save_score` (61), and is kept so.
//!
//! On the song browser the same ids read the configuration itself, as the reference's
//! `PlayerConfig` and `PlayConfig` hold it ([`SettingsView`], from [`SettingsView::of_config`]): the
//! gauge, the random and double options, the fixed tempo, the background animation, the assists, the
//! long note mode, the lane covers, the judge algorithm, the gauge auto shift and its floor, the
//! target, the judge timing, the hi-speed and the guide sound. What the configuration of this player
//! has no setting for -- the 7 to 9 conversion, the extra note depth, the mine mode, the judge area,
//! the processed note mark, the BPM guide, the replay auto-save conditions, the constant, the
//! automatic hi-speed adjustment -- reads as absent, not as off: the reference always has a value
//! there, and a skin that shows one has nothing true to show.
//!
//! These are the fields the browser's events step (`stage/select/events.rs`), so what a skin's
//! button changed is what the skin then shows, and so is what the settings screen or the option
//! overlay changed.
//!
//! The browser also answers the ids of the settings' names (the target's, and the ones before and
//! after it in the list) and the numbers of the hi-speed and the judge timing.
//!
//! A cluster that has no snapshot answers nothing, so the ids fall through to whatever else can.

use std::borrow::Cow;

use rbms_chart::shuffle::NoteOption;
use rbms_config::{Config, FixHiSpeed, ScoreTarget};
use rbms_judge::JudgeAlgorithm;
use rbms_judge::gauge::{GaugeIndex, GaugeKind};
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT};

use super::result::snapshot::FinishedRun;
use super::{
    ClusterState, INDEX_CONSTANT, INDEX_GUIDE_SOUND, INDEX_TARGET_OPTION_FIRST, INDEX_TARGET_OPTION_LAST, INDEX_TIMING_AUTO_ADJUST, STRING_TARGET_NAME,
    STRING_TARGET_NEXT_FIRST, STRING_TARGET_PREVIOUS_FIRST,
};
use crate::judge_setup::{is_custom_judge, judge_setup_of};

/// What a favourite mark reads as: not marked, marked, and hidden. The player has no way to hide a
/// chart, so only the first two are ever shown (`IndexType.favorite_chart`).
const FAVORITE_NONE: i32 = 0;
const FAVORITE_MARKED: i32 = 1;

/// The random options in the order the reference numbers them, off first (`Random.OPTION_GENERAL`,
/// whose two EX options this player does not have).
pub(crate) const OPTION_GENERAL: [NoteOption; 8] = [
    NoteOption::Off,
    NoteOption::Mirror,
    NoteOption::Random,
    NoteOption::RRandom,
    NoteOption::SRandom,
    NoteOption::Rotate,
    NoteOption::HRandom,
    NoteOption::AllScratch,
];

/// The random option as the reference numbers it: its place in [`OPTION_GENERAL`].
pub fn random_option_index(option: NoteOption) -> i32 {
    OPTION_GENERAL.iter().position(|general| *general == option).and_then(|index| i32::try_from(index).ok()).unwrap_or_default()
}

/// The choices a run was played with, in the numbers the reference shows them as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayedOptions {
    /// The random option of the first player's lanes and of the second's
    /// (`ReplayData.randomoption`, `randomoption2`).
    pub random: i32,
    pub random_2p: i32,
    /// The double option (`ReplayData.doubleoption`).
    pub double_option: i32,
    /// The assists a run shows beside its score: a custom judge window, a constant scroll speed,
    /// the judge area shown, legacy long notes, processed notes marked, the BPM guide and no mines.
    pub custom_judge: bool,
    pub constant_scroll: bool,
    pub judge_area: bool,
    pub legacy_long_note: bool,
    pub mark_note: bool,
    pub bpm_guide: bool,
    pub no_mine: bool,
    /// The long note mode, as the reference numbers it (0 long note, 1 charge note, 2 hell charge).
    pub long_note_mode: i32,
}

/// What `Config.getBga` numbers the background animation as: shown, shown in auto-play only, off.
/// This player's switch is shown or off.
const BGA_SHOWN: i32 = 0;
const BGA_OFF: i32 = 2;

/// How many names each of the two runs of texts that name the targets before and after the current
/// one holds (`StringPropertyPattern.TARGET_NAME_PREVIOUS` and `TARGET_NAME_NEXT`).
const TARGET_NAMES_PER_RUN: i32 = 10;

/// The settings of the chart under the browser's cursor that depend on which chart it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartSettings {
    /// Which tempo the green number is pinned to, as the reference numbers it: off, start, max, main
    /// and min (`PlayConfig.getFixhispeed`).
    pub fix_hispeed: i32,
    /// The scroll speed (`PlayConfig.getHispeed`).
    pub hispeed: f32,
}

/// The configuration as the reference's `PlayerConfig` and `PlayConfig` show it, in its numbering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingsView {
    /// The random options, the assists and the long note mode.
    pub played: PlayedOptions,
    /// The gauge the player chose (`PlayerConfig.getGauge`).
    pub gauge: i32,
    /// The background animation (`Config.getBga`).
    pub bga: i32,
    /// The judge timing in milliseconds (`PlayerConfig.getJudgetiming`) and whether it adjusts by
    /// itself.
    pub judge_timing: i32,
    pub timing_auto_adjust: bool,
    /// The gauge auto shift and the gauge it may not fall below (`PlayerConfig.getGaugeAutoShift`,
    /// `getBottomShiftableGauge`).
    pub gauge_auto_shift: i32,
    pub bottom_shiftable_gauge: i32,
    /// The judge algorithm in the order of `JudgeAlgorithm.Combo`, `Duration` and `Lowest`. The
    /// reference lists no place for the fourth algorithm, so it reads as absent.
    pub judge_algorithm: Option<i32>,
    /// Whether the lane cover, the lift and the hidden cover are on.
    pub lane_cover: bool,
    pub lift: bool,
    pub hidden: bool,
    /// Whether a cue sounds on every judgement (`PlayerConfig.isGuideSE`).
    pub guide_se: bool,
    /// The target the play is paced against.
    pub target: ScoreTarget,
    /// The settings of the chart under the cursor, when it is a chart (or a course made of charts
    /// the library holds): a folder has no tempo to pin.
    pub chart: Option<ChartSettings>,
}

impl SettingsView {
    /// The configuration, with the settings of a chart under the cursor.
    pub(crate) fn of_config(config: &Config) -> SettingsView {
        let play = &config.play;
        let judge = &config.judge;
        let random = random_option_index(play.random);
        SettingsView {
            played: PlayedOptions {
                random,
                random_2p: random,
                double_option: i32::from(play.lane_option.recorded_value()),
                custom_judge: is_custom_judge(&judge_setup_of(config)),
                constant_scroll: play.constant_speed,
                legacy_long_note: play.legacy_note,
                long_note_mode: i32::from(judge.ln_mode.id()),
                ..PlayedOptions::default()
            },
            gauge: gauge_number(play.gauge),
            bga: if config.display.bga { BGA_SHOWN } else { BGA_OFF },
            judge_timing: judge.offset_ms,
            timing_auto_adjust: judge.auto_offset,
            gauge_auto_shift: i32::from(judge.gauge_auto_shift.id()),
            bottom_shiftable_gauge: gauge_number(judge.bottom_shiftable_gauge),
            judge_algorithm: JudgeAlgorithm::ALL
                .iter()
                .take(JUDGE_ALGORITHMS_NUMBERED)
                .position(|algorithm| *algorithm == judge.judge_algorithm)
                .map(index_number),
            lane_cover: play.enable_cover,
            lift: play.enable_lift,
            hidden: play.enable_hidden,
            guide_se: config.audio.guide_se,
            target: judge.target,
            chart: Some(ChartSettings {
                fix_hispeed: FixHiSpeed::ALL.iter().position(|choice| *choice == play.fix_hispeed).map_or(IMAGE_INDEX_ABSENT, index_number),
                hispeed: play.hispeed as f32,
            }),
        }
    }

    /// These settings on a screen whose cursor is not on a chart.
    pub fn without_chart(self) -> SettingsView {
        SettingsView { chart: None, ..self }
    }

    /// The name of the target `steps` places on from the current one, the list wrapping round
    /// (`StringType.createTargetname`, which turns a step below zero into one counted from the end).
    fn target_name(&self, steps: i32) -> &'static str {
        let count = i32::try_from(ScoreTarget::ALL.len()).unwrap_or(i32::MAX);
        let current = ScoreTarget::ALL.iter().position(|target| *target == self.target).map_or(0, index_number);
        let offset = if steps >= 0 { steps } else { count + steps };
        usize::try_from((current + offset) % count).ok().and_then(|index| ScoreTarget::ALL.get(index)).map_or("", |target| target.label())
    }
}

/// How many judge algorithms the reference's skin index numbers (`IndexType.judgealgorithm` lists
/// combo, duration and lowest).
pub(crate) const JUDGE_ALGORITHMS_NUMBERED: usize = 3;

/// A place in a short list as the whole number a skin shows it as.
fn index_number(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// A gauge as the reference numbers gauges.
fn gauge_number(kind: GaugeKind) -> i32 {
    index_number(GaugeIndex::from_kind(kind).index())
}

/// What this cluster reads from the running game, borrowed for one frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct OptionsState<'a> {
    /// The finished run, when there is one on the screen.
    pub result: Option<FinishedRun<'a>>,
    /// The player's settings, on a screen that shows them.
    pub settings: Option<SettingsView>,
}

impl<'a> OptionsState<'a> {
    /// The choices of the finished run on a result screen.
    pub fn of_result(finished: FinishedRun<'a>) -> OptionsState<'a> {
        OptionsState { result: Some(finished), settings: None }
    }

    /// The settings of the player, on the song browser.
    pub fn of_settings(settings: SettingsView) -> OptionsState<'a> {
        OptionsState { result: None, settings: Some(settings) }
    }
}

/// How a mark that may be unknown is shown: absent while it is not known.
fn favorite_index(mark: Option<bool>) -> i32 {
    mark.map_or(IMAGE_INDEX_ABSENT, |marked| if marked { FAVORITE_MARKED } else { FAVORITE_NONE })
}

impl SettingsView {
    /// The image index under `id` as the settings show it.
    fn image_index(&self, id: i32) -> Option<i32> {
        let options = &self.played;
        Some(match id {
            BUTTON_GAUGE_1P => self.gauge,
            BUTTON_RANDOM_1P => options.random,
            BUTTON_RANDOM_2P => options.random_2p,
            BUTTON_DPOPTION => options.double_option,
            INDEX_TARGET_OPTION_FIRST..=INDEX_TARGET_OPTION_LAST => IMAGE_INDEX_ABSENT,
            BUTTON_HSFIX => self.chart.map_or(IMAGE_INDEX_ABSENT, |chart| chart.fix_hispeed),
            BUTTON_BGA => self.bga,
            INDEX_TIMING_AUTO_ADJUST => i32::from(self.timing_auto_adjust),
            BUTTON_GAUGEAUTOSHIFT => self.gauge_auto_shift,
            BUTTON_BOTTOMSIFTABLEFGAUGE => self.bottom_shiftable_gauge,
            BUTTON_ASSIST_EXJUDGE => i32::from(options.custom_judge),
            BUTTON_ASSIST_CONSTANT | BUTTON_SCROLLMODE => i32::from(options.constant_scroll),
            BUTTON_ASSIST_LEGACY | BUTTON_LONGNOTEMODE => i32::from(options.legacy_long_note),
            BUTTON_LNMODE => options.long_note_mode,
            BUTTON_LANECOVER => i32::from(self.lane_cover),
            BUTTON_LIFT => i32::from(self.lift),
            BUTTON_HIDDEN => i32::from(self.hidden),
            BUTTON_JUDGEALGORITHM => self.judge_algorithm.unwrap_or(IMAGE_INDEX_ABSENT),
            INDEX_GUIDE_SOUND => i32::from(self.guide_se),
            BUTTON_ASSIST_JUDGEAREA
            | BUTTON_ASSIST_MARKNOTE
            | BUTTON_ASSIST_BPMGUIDE
            | BUTTON_ASSIST_NOMINE
            | BUTTON_AUTOSAVEREPLAY_1..=BUTTON_AUTOSAVEREPLAY_4
            | BUTTON_HISPEEDAUTOADJUST
            | BUTTON_EXTRANOTE
            | BUTTON_MINEMODE
            | BUTTON_SEVENTONINE_PATTERN
            | BUTTON_SEVENTONINE_TYPE
            | INDEX_CONSTANT => IMAGE_INDEX_ABSENT,
            _ => return None,
        })
    }

    /// The number under `id`: the judge timing and the scroll speed in the three ways it is read
    /// (`createHispeedProperty`).
    fn integer(&self, id: i32) -> Option<i32> {
        let hispeed = |read: fn(f32) -> i32| self.chart.map_or(INTEGER_ABSENT, |chart| read(chart.hispeed));
        Some(match id {
            NUMBER_JUDGETIMING => self.judge_timing,
            NUMBER_HISPEED_LR2 => hispeed(|speed| (speed * HISPEED_LR2_SCALE) as i32),
            NUMBER_HISPEED => hispeed(|speed| speed as i32),
            NUMBER_HISPEED_AFTERDOT => hispeed(|speed| (speed * HISPEED_LR2_SCALE) as i32 % HISPEED_LR2_SCALE as i32),
            NUMBER_DURATION | NUMBER_DURATION_GREEN => INTEGER_ABSENT,
            _ => return None,
        })
    }

    /// The text under `id`: the target and its neighbours in the list.
    fn text(&self, id: i32) -> Option<&'static str> {
        match id {
            STRING_TARGET_NAME => Some(self.target.label()),
            _ => {
                let run = |first: i32| id.checked_sub(first).filter(|place| (0..TARGET_NAMES_PER_RUN).contains(place));
                run(STRING_TARGET_PREVIOUS_FIRST)
                    .map(|place| self.target_name(place - TARGET_NAMES_PER_RUN))
                    .or_else(|| run(STRING_TARGET_NEXT_FIRST).map(|place| self.target_name(place + 1)))
            }
        }
    }
}

/// What a scroll speed is multiplied by to read it as the reference's LR2 style number does.
const HISPEED_LR2_SCALE: f32 = 100.0;

impl ClusterState for OptionsState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let run = self.result?.snapshot;
        match id {
            OPTION_DISABLE_SAVE_SCORE => Some(!run.updates_score),
            OPTION_ENABLE_SAVE_SCORE | OPTION_NO_SAVE_CLEAR => Some(run.updates_score),
            _ => None,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
        self.settings?.integer(id)
    }

    fn float(&self, id: i32) -> Option<f32> {
        let settings = self.settings?;
        (id == FLOAT_HISPEED).then(|| settings.chart.map_or(FLOAT_ABSENT, |chart| chart.hispeed))
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        self.settings?.text(id).map(Cow::Borrowed)
    }

    fn image_index(&self, id: i32) -> Option<i32> {
        let Some(finished) = self.result else {
            return self.settings?.image_index(id);
        };
        let run = finished.snapshot;
        let options = &run.options;
        Some(match id {
            BUTTON_GAUGE_1P => finished.scene.gauge_type as i32,
            BUTTON_RANDOM_1P => options.random,
            BUTTON_RANDOM_2P => options.random_2p,
            BUTTON_DPOPTION => options.double_option,
            INDEX_TARGET_OPTION_FIRST..=INDEX_TARGET_OPTION_LAST => IMAGE_INDEX_ABSENT,
            BUTTON_FAVORITTE_SONG => favorite_index(run.favorite_song),
            BUTTON_FAVORITTE_CHART => favorite_index(run.favorite_chart),
            BUTTON_ASSIST_EXJUDGE => i32::from(options.custom_judge),
            BUTTON_ASSIST_CONSTANT => i32::from(options.constant_scroll),
            BUTTON_ASSIST_JUDGEAREA => i32::from(options.judge_area),
            BUTTON_ASSIST_LEGACY => i32::from(options.legacy_long_note),
            BUTTON_ASSIST_MARKNOTE => i32::from(options.mark_note),
            BUTTON_ASSIST_BPMGUIDE => i32::from(options.bpm_guide),
            BUTTON_ASSIST_NOMINE => i32::from(options.no_mine),
            BUTTON_LNMODE => options.long_note_mode,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests;
