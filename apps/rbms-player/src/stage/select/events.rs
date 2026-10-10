//! The events a skin's browser runs: what each id of the reference's event table comes to here.
//!
//! An event reaches the browser two ways. A button of the skin, or a script of it calling
//! `main_state.event_exec`, leaves a request in the queue of the cluster that owns the id
//! ([`crate::skin_host::EVENT_ROUTES`]), and the browser takes the events of its own cluster and of
//! the settings cluster from there once a frame ([`SelectState::carry_out_skin_events`]). The keys of
//! an option panel run the same events by number, as the reference's do ([`super::panel`]).
//!
//! The table is `EventFactory.EventType`. An event that steps a setting steps the field of the
//! configuration the settings screen and the option overlay edit, so a value changed from the skin
//! is the value those show ([`SettingEvent`]). An argument of zero or more steps to the next value
//! and one below zero to the one before, round the ends, and the sound of a changed option is
//! played where the reference plays it.
//!
//! What the reference keeps and this player has no setting for is left out of the table, and an id
//! that is not in it does nothing at all: the difficulty filter (10), the note display time (59) and
//! its automatic adjustment (342), the stretch of the background animation (73), the rival (79), the
//! star on a whole song (89), the replay auto-save conditions (321 to 324), the chart replication
//! mode (344), the extra note depth (350), the mine mode (351), the 7 to 9 conversion (360, 361) and
//! the constant (400). What the reference hands to the machine's own programs does nothing either:
//! the text files of a chart (17), the page of the score server (210), the folder of a chart (212)
//! and its download page (213), and the rescan of a folder (211), which is a screen of its own here.
//! The seven assist ids (301 to 307) are no event in the reference: its second panel is driven by
//! keys alone.
//!
//! The buttons that leave the browser go where this player keeps the same thing: the key
//! configuration (13) is its own screen, the skin configuration (14) is the skin tab of the settings
//! screen, practice (315) is the practice screen, and the four replay slots (19, 316 to 318) are the
//! four newest replays saved of the chart under the cursor. Autoplay (16) plays the chart once by
//! itself, whatever the autoplay setting says, without touching the setting.

use rbms_config::{AdjustOutcome, Config, JUDGE_OFFSET_MAX_MS, JUDGE_OFFSET_MIN_MS, SettingId, SettingTab, adjust};
use rbms_judge::JudgeAlgorithm;
use rbms_skin::property::generated::*;
use rbms_store::ScoreBook;

use super::SelectState;
use super::skinned::FocusedBar;
use crate::skin_host::options::{JUDGE_ALGORITHMS_NUMBERED, OPTION_GENERAL, random_option_index};
use crate::skin_host::{Cluster, ClusterRequest};
use crate::stage::{KeyConfigState, SettingsState, Stage, Transition};
use crate::{AppShared, SelectTab, SystemSound};

/// The events that step a setting and that the reference numbers without a constant of their own
/// (`EventType.hispeed1p`, `notesdisplaytimingautoadjust`, `guidese`).
const EVENT_HISPEED: i32 = 57;
const EVENT_JUDGE_TIMING_AUTO: i32 = 75;
const EVENT_GUIDE_SE: i32 = 343;

/// The event that steps the order of the list through every order the reference has
/// (`EventType.songbar_sort`). This browser has one list of orders, so it steps that.
const EVENT_SONGBAR_SORT: i32 = 312;

/// One step of a setting towards its next value and towards the one before.
const STEP_NEXT: i32 = 1;
const STEP_PREVIOUS: i32 = -1;

/// The step an event's first argument asks for (`arg1 >= 0`).
const fn step_of(arg1: i32) -> i32 {
    if arg1 >= 0 { STEP_NEXT } else { STEP_PREVIOUS }
}

/// An event that steps one setting of the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingEvent {
    /// `gauge1p`: the gauge, round the six of them.
    Gauge,
    /// `option1p` and `option2p`: the random option. This player deals both sides of a double chart
    /// by the one option, so both events step it.
    Random,
    /// `optiondp`: what is done to the two sides as a whole.
    DoubleOption,
    /// `hsfix`: which tempo the green number is pinned to.
    FixHiSpeed,
    /// `hispeed1p`: the scroll speed, one of its own steps at a time and held to its range.
    HiSpeed,
    /// `bga`: the background animation, which is shown or not here.
    Bga,
    /// `notesdisplaytiming`: the judge timing, one millisecond at a time and held to its range.
    JudgeTiming,
    /// `notesdisplaytimingautoadjust`.
    JudgeTimingAuto,
    /// `target`: the score the play is paced against. The one event with no sound.
    Target,
    /// `gaugeautoshift`.
    GaugeAutoShift,
    /// `bottomshiftablegauge`.
    BottomShiftableGauge,
    /// `lanecover`, `lift` and `hidden`: whether each cover is applied.
    LaneCover,
    Lift,
    Hidden,
    /// `judgealgorithm`: round the algorithms the reference lists, and nothing from one it does not.
    JudgeAlgorithm,
    /// `guidese`.
    GuideSe,
    /// `scrollmode`, and the second panel's key for a constant speed: the speed is constant or not.
    ScrollMode,
    /// `longnotemode`, and the second panel's key for legacy notes: long notes are drawn as plain
    /// notes or not.
    LongNoteMode,
}

impl SettingEvent {
    /// Whether the reference plays the sound of a changed option for this event. It plays none for
    /// the target, which its first panel scrolls with a sound of its own.
    const fn sounds(self) -> bool {
        !matches!(self, SettingEvent::Target)
    }

    /// The row of the settings screen an event steps exactly as the row itself is stepped, or `None`
    /// for the three that are stepped their own way.
    const fn row(self) -> Option<SettingId> {
        Some(match self {
            SettingEvent::Gauge => SettingId::Gauge,
            SettingEvent::DoubleOption => SettingId::LaneOption,
            SettingEvent::FixHiSpeed => SettingId::FixHiSpeed,
            SettingEvent::HiSpeed => SettingId::HiSpeed,
            SettingEvent::Bga => SettingId::Bga,
            SettingEvent::JudgeTimingAuto => SettingId::AutoCal,
            SettingEvent::Target => SettingId::Target,
            SettingEvent::GaugeAutoShift => SettingId::GaugeAutoShift,
            SettingEvent::BottomShiftableGauge => SettingId::BottomShiftableGauge,
            SettingEvent::LaneCover => SettingId::EnableCover,
            SettingEvent::Lift => SettingId::EnableLift,
            SettingEvent::Hidden => SettingId::EnableHidden,
            SettingEvent::GuideSe => SettingId::GuideSe,
            SettingEvent::ScrollMode => SettingId::SpeedFix,
            SettingEvent::LongNoteMode => SettingId::LegacyNote,
            SettingEvent::Random | SettingEvent::JudgeTiming | SettingEvent::JudgeAlgorithm => return None,
        })
    }
}

/// Step the setting of `event` in `config` the way `arg1` asks, and answer whether it moved.
///
/// Most settings are the rows of the settings screen and are stepped as a row is, which goes round
/// the ends of a list the way the reference's `(value + 1) % count` and `(value + count - 1) % count`
/// do. Three are not. The random option goes round in the order the reference numbers its options
/// (`Random.OPTION_GENERAL`), which is not the order of the row. The judge timing moves a
/// millisecond where the row moves five, and stops at the ends of its range. The judge algorithm
/// goes round the three the reference lists (`JudgeAlgorithm.defaultAlgorithm`), and from the one it
/// does not list the event finds no place to step from, as the reference's loop finds none.
pub(super) fn step_setting(config: &mut Config, event: SettingEvent, arg1: i32) -> bool {
    let step = step_of(arg1);
    if let Some(row) = event.row() {
        return adjust(config, row, step) == AdjustOutcome::Changed;
    }
    match event {
        SettingEvent::Random => {
            let count = OPTION_GENERAL.len();
            let at = usize::try_from(random_option_index(config.play.random)).unwrap_or_default();
            let next = OPTION_GENERAL[(at + if step == STEP_NEXT { 1 } else { count - 1 }) % count];
            let moved = next != config.play.random;
            config.play.random = next;
            moved
        }
        SettingEvent::JudgeTiming => {
            let next = (config.judge.offset_ms + step).clamp(JUDGE_OFFSET_MIN_MS, JUDGE_OFFSET_MAX_MS);
            let moved = next != config.judge.offset_ms;
            config.judge.offset_ms = next;
            moved
        }
        SettingEvent::JudgeAlgorithm => {
            let listed = &JudgeAlgorithm::ALL[..JUDGE_ALGORITHMS_NUMBERED];
            let Some(at) = listed.iter().position(|algorithm| *algorithm == config.judge.judge_algorithm) else {
                return false;
            };
            config.judge.judge_algorithm = listed[(at + if step == STEP_NEXT { 1 } else { listed.len() - 1 }) % listed.len()];
            true
        }
        _ => false,
    }
}

/// What an event id does on the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectEvent {
    /// Step a setting of the configuration.
    Setting(SettingEvent),
    /// `mode`: the play mode the list is held to.
    Mode,
    /// `sort` and `songbar_sort`: the order of the list.
    Sort,
    /// `lnmode`: the long note mode, which the list is built again for.
    LnMode,
    /// `keyconfig`: the key configuration screen.
    KeyConfig,
    /// `skinconfig`: the skin tab of the settings screen.
    SkinConfig,
    /// `play`, `autoplay` and `practice`: start what is under the cursor.
    Play,
    Autoplay,
    Practice,
    /// `replay1` to `replay4`: play the replay in this slot, the newest first.
    Replay(usize),
    /// `favorite_chart`: star or unstar the chart under the cursor.
    FavoriteChart,
}

/// The reference's event table as this browser carries it out, by id.
const EVENTS: &[(i32, SelectEvent)] = &[
    (BUTTON_MODE, SelectEvent::Mode),
    (BUTTON_SORT, SelectEvent::Sort),
    (EVENT_SONGBAR_SORT, SelectEvent::Sort),
    (BUTTON_KEYCONFIG, SelectEvent::KeyConfig),
    (BUTTON_SKINSELECT, SelectEvent::SkinConfig),
    (BUTTON_PLAY, SelectEvent::Play),
    (BUTTON_AUTOPLAY, SelectEvent::Autoplay),
    (BUTTON_PRACTICE, SelectEvent::Practice),
    (BUTTON_REPLAY, SelectEvent::Replay(0)),
    (BUTTON_REPLAY2, SelectEvent::Replay(1)),
    (BUTTON_REPLAY3, SelectEvent::Replay(2)),
    (BUTTON_REPLAY4, SelectEvent::Replay(3)),
    (BUTTON_FAVORITTE_CHART, SelectEvent::FavoriteChart),
    (BUTTON_LNMODE, SelectEvent::LnMode),
    (BUTTON_GAUGE_1P, SelectEvent::Setting(SettingEvent::Gauge)),
    (BUTTON_RANDOM_1P, SelectEvent::Setting(SettingEvent::Random)),
    (BUTTON_RANDOM_2P, SelectEvent::Setting(SettingEvent::Random)),
    (BUTTON_DPOPTION, SelectEvent::Setting(SettingEvent::DoubleOption)),
    (BUTTON_HSFIX, SelectEvent::Setting(SettingEvent::FixHiSpeed)),
    (EVENT_HISPEED, SelectEvent::Setting(SettingEvent::HiSpeed)),
    (BUTTON_BGA, SelectEvent::Setting(SettingEvent::Bga)),
    (BUTTON_JUDGE_TIMING, SelectEvent::Setting(SettingEvent::JudgeTiming)),
    (EVENT_JUDGE_TIMING_AUTO, SelectEvent::Setting(SettingEvent::JudgeTimingAuto)),
    (BUTTON_TARGET, SelectEvent::Setting(SettingEvent::Target)),
    (BUTTON_GAUGEAUTOSHIFT, SelectEvent::Setting(SettingEvent::GaugeAutoShift)),
    (BUTTON_BOTTOMSIFTABLEFGAUGE, SelectEvent::Setting(SettingEvent::BottomShiftableGauge)),
    (BUTTON_LANECOVER, SelectEvent::Setting(SettingEvent::LaneCover)),
    (BUTTON_LIFT, SelectEvent::Setting(SettingEvent::Lift)),
    (BUTTON_HIDDEN, SelectEvent::Setting(SettingEvent::Hidden)),
    (BUTTON_JUDGEALGORITHM, SelectEvent::Setting(SettingEvent::JudgeAlgorithm)),
    (EVENT_GUIDE_SE, SelectEvent::Setting(SettingEvent::GuideSe)),
    (BUTTON_SCROLLMODE, SelectEvent::Setting(SettingEvent::ScrollMode)),
    (BUTTON_LONGNOTEMODE, SelectEvent::Setting(SettingEvent::LongNoteMode)),
];

/// What the event `id` does here, or `None` for an id that does nothing.
pub(super) fn event_of(id: i32) -> Option<SelectEvent> {
    EVENTS.iter().find(|(event, _)| *event == id).map(|(_, does)| *does)
}

/// The replay file in `slot` of the chart `md5`: the replays saved of it, the newest first.
pub(super) fn replay_in_slot(scores: &ScoreBook, md5: &str, slot: usize) -> Option<String> {
    scores.for_md5(md5).into_iter().filter_map(|record| record.replay_file.clone()).nth(slot)
}

impl SelectState {
    /// Carries out the events the skin asked for since the last frame, oldest first within the
    /// browser's own and then the settings', until one of them leaves the browser. With the record
    /// modal up an event closes it instead, as a press on a bar does.
    ///
    /// The writes of the two clusters are left where they are: the browser's own are carried out
    /// beside this ([`SelectState::carry_out_skin_writes`]) and a volume never waits in the queue.
    pub(super) fn carry_out_skin_events(&mut self, shared: &mut AppShared) -> Transition {
        let mut events = Vec::new();
        for cluster in [Cluster::Select, Cluster::Options] {
            events.extend(shared.skin_requests().take_if(cluster, |request| matches!(request, ClusterRequest::Event { .. })));
        }
        if self.record_modal.is_some() {
            if !events.is_empty() {
                self.record_modal = None;
            }
            return Transition::Stay;
        }
        for request in events {
            let ClusterRequest::Event { id, arg1, .. } = request else {
                continue;
            };
            let asked = self.run_event(shared, id, arg1);
            if !matches!(asked, Transition::Stay) {
                return asked;
            }
        }
        Transition::Stay
    }

    /// Run the event `id` with its first argument (`MainState.executeEvent`).
    pub(super) fn run_event(&mut self, shared: &mut AppShared, id: i32, arg1: i32) -> Transition {
        let forward = arg1 >= 0;
        match event_of(id) {
            Some(SelectEvent::Setting(event)) => self.run_setting_event(shared, event, arg1),
            Some(SelectEvent::Mode) => self.mode_event(shared, forward),
            Some(SelectEvent::Sort) => self.sort_event(shared, forward),
            Some(SelectEvent::LnMode) => self.ln_mode_event(shared, forward),
            Some(SelectEvent::KeyConfig) => return Transition::Open(Stage::KeyConfig(KeyConfigState::new())),
            Some(SelectEvent::SkinConfig) => return Transition::Open(Stage::Settings(SettingsState::on_tab(SettingTab::Skin))),
            Some(SelectEvent::Play) => return self.play_focused(shared),
            Some(SelectEvent::Autoplay) => return self.start_autoplay(shared),
            Some(SelectEvent::Practice) => return self.practise_focused(shared),
            Some(SelectEvent::Replay(slot)) => return self.play_replay_slot(shared, slot),
            Some(SelectEvent::FavoriteChart) => self.favorite_event(shared),
            None => {}
        }
        Transition::Stay
    }

    /// Step the setting of `event`, with the sound of a changed option when it moved and the event
    /// has one. The change is written out once no panel is up
    /// ([`SelectState::settle_option_changes`]), so a key held on a panel does not write the file at
    /// every step.
    ///
    /// The settings the reference keeps for the kind of play under the cursor -- the scroll speed and
    /// its fix, the covers, the judge algorithm -- have no place to go on a course whose charts are
    /// not all there (`MusicSelector.getSelectedBarPlayConfig`), and the events that step them do
    /// nothing on one.
    pub(super) fn run_setting_event(&mut self, shared: &mut AppShared, event: SettingEvent, arg1: i32) {
        let of_the_play = matches!(
            event,
            SettingEvent::FixHiSpeed
                | SettingEvent::HiSpeed
                | SettingEvent::LaneCover
                | SettingEvent::Lift
                | SettingEvent::Hidden
                | SettingEvent::JudgeAlgorithm
        );
        if of_the_play && !self.has_play_settings() {
            return;
        }
        if !step_setting(&mut shared.config, event, arg1) {
            return;
        }
        self.options_dirty = true;
        if event == SettingEvent::GuideSe {
            shared.syssound.set_guide_enabled(shared.config.audio.guide_se);
        }
        if event.sounds() {
            shared.play_system_sound(SystemSound::OptionChange);
        }
    }

    /// Whether the bar under the cursor has play settings to step: anything but a course that cannot
    /// be played for want of a chart.
    fn has_play_settings(&self) -> bool {
        self.tab != SelectTab::Courses || self.courses.focused().is_none_or(|entry| entry.is_playable())
    }

    /// Write out the settings the skin's events changed, once no option panel is up. The built-in
    /// field is laid out again as it is when the option overlay closes on a change, since the lift
    /// is one of the things an event switches.
    pub(super) fn settle_option_changes(&mut self, shared: &mut AppShared) {
        if !self.panel.is_open() {
            self.write_option_changes(shared);
        }
    }

    /// Write out the settings the skin's events changed, whatever is up: the browser is being left.
    pub(super) fn write_option_changes(&mut self, shared: &mut AppShared) {
        if !std::mem::take(&mut self.options_dirty) {
            return;
        }
        shared.rebuild_skin();
        shared.save_settings();
    }

    /// `play`: start the chart or the course under the cursor. A bar that opens is left alone
    /// (`MusicSelector.render`, which starts nothing from a folder but an autoplay of it).
    fn play_focused(&mut self, shared: &mut AppShared) -> Transition {
        if self.focused_bar(shared) != FocusedBar::Playable {
            return Transition::Stay;
        }
        self.select_enter(shared)
    }

    /// `practice`: the practice screen on a chart, and a course played as it is, which has none.
    fn practise_focused(&mut self, shared: &mut AppShared) -> Transition {
        match (self.focused_bar(shared), self.tab) {
            (FocusedBar::Playable, SelectTab::Courses) => self.course_enter(shared),
            (FocusedBar::Playable, SelectTab::Songs) => self.start_practice(shared),
            _ => Transition::Stay,
        }
    }

    /// `autoplay`: play the chart under the cursor by itself, once.
    ///
    /// The run is marked as one that plays itself ([`AppShared::run_plays_itself`]) and the mark is
    /// taken off when the browser is next arrived at ([`SelectState::end_autoplay_run`]). The
    /// AUTOPLAY row of the settings is not what says so and is left as the player has it, so
    /// whatever writes the settings out while the run is on -- the result, an escape from the play
    /// -- writes the player's own choice. A course is not started this way: it leaves the browser
    /// for good, and there would be nobody to take the mark off. A folder is not played through
    /// either.
    pub(super) fn start_autoplay(&mut self, shared: &mut AppShared) -> Transition {
        if self.tab != SelectTab::Songs || self.focused_bar(shared) != FocusedBar::Playable {
            return Transition::Stay;
        }
        let asked = self.select_enter(shared);
        shared.autoplay_once = !matches!(asked, Transition::Stay);
        asked
    }

    /// Take the mark off a run that was started to play itself: the browser is up again, and the
    /// next run is whatever the AUTOPLAY row says.
    pub(super) fn end_autoplay_run(shared: &mut AppShared) {
        shared.autoplay_once = false;
    }

    /// `replay1` to `replay4`: play the replay in `slot` of the chart under the cursor. The slots are
    /// the replays saved of the chart, the newest first; one with nothing in it does nothing, and so
    /// does any slot of a bar that is no chart.
    pub(super) fn play_replay_slot(&mut self, shared: &mut AppShared, slot: usize) -> Transition {
        if self.tab != SelectTab::Songs {
            return Transition::Stay;
        }
        let Some(md5) = shared.focused_md5() else {
            return Transition::Stay;
        };
        match replay_in_slot(&shared.scores, &md5, slot) {
            Some(file) => self.play_replay_file(shared, &file),
            None => Transition::Stay,
        }
    }

    /// `favorite_chart`: star or unstar the chart under the cursor, as the browser's own key does,
    /// with the sound the reference gives it. The reference's mark has a third state, hidden, which
    /// this player's star has not.
    fn favorite_event(&mut self, shared: &mut AppShared) {
        if self.tab != SelectTab::Songs || shared.focused_md5().is_none() {
            return;
        }
        shared.toggle_focused_favorite();
        shared.play_system_sound(SystemSound::OptionChange);
    }
}
