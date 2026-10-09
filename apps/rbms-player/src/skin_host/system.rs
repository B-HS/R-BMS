//! Property cluster I: the clock, the player's totals and the machine.
//!
//! Where the values come from: the wall clock, the player's lifetime totals, the volumes and the
//! keys held on the keyboard.
//!
//! What this cluster answers: numbers 20-37, 57-59 and 333; rates 17-19; options 280-290 (the
//! course being played); strings 2 and 1010; and what `main_state` asks of the machine itself: the
//! three volumes.
//!
//! A value is read as `IntegerPropertyFactory` and `FloatPropertyFactory` read it from the main
//! controller, the player's data and the audio configuration. What the player has no source for is
//! left `None` and is not answered: the total play time (numbers 17-19) is one, for the player
//! keeps the length of a run in its score database and not in the scores this cluster borrows.
//!
//! Whether a key is held is also this cluster's to answer, and nothing fills it in yet.

use std::borrow::Cow;

use rbms_judge::clear_type_from_id;
use rbms_skin::lua::LocalTime;
use rbms_skin::property::VolumeBus;
use rbms_skin::property::generated::*;
use rbms_store::ScoreRecord;

use super::ClusterState;

const MILLIS_PER_SECOND: i64 = 1000;
const MILLIS_PER_MINUTE: i64 = 60_000;
const MILLIS_PER_HOUR: i64 = 3_600_000;
const CLOCK_DIGIT_MODULUS: i64 = 60;

/// How a volume's share becomes the whole percent a skin shows.
const VOLUME_PERCENT_SCALE: f32 = 100.0;

/// The judgements the player's totals keep, in the order a score record counts them: perfect, great,
/// good, bad, poor and miss.
pub const JUDGEMENT_KINDS: usize = 6;

/// The first judgement kind the total of played notes leaves out, which is poor: a note that was
/// only poor or missed was not played.
const FIRST_UNPLAYED_JUDGEMENT: usize = 4;

/// The player's lifetime totals, which the reference keeps as `PlayerData` and which here are summed
/// from the scores the player has recorded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerTotals {
    /// How many runs have been recorded.
    pub plays: u64,
    /// How many of them cleared.
    pub clears: u64,
    /// How many of each judgement were given, in the order of a score record's counts.
    pub judgements: [u64; JUDGEMENT_KINDS],
}

impl PlayerTotals {
    /// The totals of `records`: a run counts as played, and as cleared when its lamp is above
    /// failed (`PlayDataAccessor.updatePlayerData`).
    pub fn of_records(records: &[ScoreRecord]) -> PlayerTotals {
        records.iter().fold(PlayerTotals::default(), |mut totals, record| {
            totals.plays += 1;
            totals.clears += u64::from(clear_type_from_id(record.clear).is_cleared());
            for (total, count) in totals.judgements.iter_mut().zip(record.counts) {
                *total += u64::from(count);
            }
            totals
        })
    }

    /// The notes played: every judgement but poor and miss (`player_notes`).
    fn played_notes(&self) -> u64 {
        self.judgements[..FIRST_UNPLAYED_JUDGEMENT].iter().sum()
    }
}

/// The three volumes, each from nothing to one.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Volumes {
    /// System and skin sounds (`volume_sys`).
    pub system: f32,
    pub key: f32,
    pub background: f32,
}

impl Volumes {
    fn of(&self, bus: VolumeBus) -> f32 {
        match bus {
            VolumeBus::System => self.system,
            VolumeBus::Key => self.key,
            VolumeBus::Background => self.background,
        }
    }
}

/// Where a course in progress stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseStage {
    /// The stage being played, from zero.
    pub index: usize,
    /// How many stages the course has.
    pub count: usize,
}

/// What this cluster reads from the running game, borrowed for one frame.
///
/// A field left `None` is a source the frame did not bring, and the ids that read it are not
/// answered. The one exception is [`SystemState::course`]: no course is a fact, not a gap, and the
/// options that tell a course from a single chart read it either way.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemState<'a> {
    /// The date and the time of day in the machine's local time zone, which is the zone the
    /// reference's calendar reads in.
    pub clock: Option<LocalTime>,
    /// How long the application has been running, in milliseconds.
    pub uptime_ms: Option<u64>,
    pub fps: Option<f32>,
    pub player_name: Option<&'a str>,
    pub version: Option<&'a str>,
    /// Every run the player has recorded, summed into the lifetime totals when one is read.
    pub history: Option<&'a [ScoreRecord]>,
    pub volumes: Option<Volumes>,
    /// The course in progress, or `None` for a single chart.
    pub course: Option<CourseStage>,
}

impl SystemState<'_> {
    fn totals(&self) -> Option<PlayerTotals> {
        self.history.map(PlayerTotals::of_records)
    }

    /// A count as the whole number a skin reads, saturating where it would not fit.
    fn count(count: u64) -> i32 {
        i32::try_from(count).unwrap_or(i32::MAX)
    }

    /// One digit group of the time the application has been up.
    fn uptime(&self, millis_per_unit: i64, modulus: Option<i64>) -> Option<i32> {
        let units = i64::try_from(self.uptime_ms?).unwrap_or(i64::MAX) / millis_per_unit;
        Some(i32::try_from(modulus.map_or(units, |modulus| units % modulus)).unwrap_or(i32::MAX))
    }

    fn volume_percent(&self, bus: VolumeBus) -> Option<i32> {
        self.volumes.map(|volumes| (volumes.of(bus) * VOLUME_PERCENT_SCALE) as i32)
    }
}

impl ClusterState for SystemState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        let (index, count) = self.course.map_or((None, None), |stage| (Some(stage.index), Some(stage.count)));
        match id {
            OPTION_MODE_COURSE => Some(self.course.is_some()),
            OPTION_COURSE_STAGE1..=OPTION_COURSE_STAGE4 => {
                let stage = usize::try_from(id - OPTION_COURSE_STAGE1).ok();
                Some(index == stage && index.zip(count).is_some_and(|(index, count)| index + 1 != count))
            }
            OPTION_COURSE_STAGE_FINAL => Some(index.zip(count).is_some_and(|(index, count)| index + 1 == count)),
            _ => None,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
        let clock = self.clock;
        match id {
            NUMBER_CURRENT_FPS => self.fps.map(|fps| fps.round() as i32),
            NUMBER_TIME_YEAR => clock.map(|clock| clock.year),
            NUMBER_TIME_MONTH => clock.map(|clock| clock.month),
            NUMBER_TIME_DAY => clock.map(|clock| clock.day),
            NUMBER_TIME_HOUR => clock.map(|clock| clock.hour),
            NUMBER_TIME_MINUTE => clock.map(|clock| clock.minute),
            NUMBER_TIME_SECOND => clock.map(|clock| clock.second),
            NUMBER_OPERATING_TIME_HOUR => self.uptime(MILLIS_PER_HOUR, None),
            NUMBER_OPERATING_TIME_MINUTE => self.uptime(MILLIS_PER_MINUTE, Some(CLOCK_DIGIT_MODULUS)),
            NUMBER_OPERATING_TIME_SECOND => self.uptime(MILLIS_PER_SECOND, Some(CLOCK_DIGIT_MODULUS)),
            NUMBER_TOTALPLAYCOUNT => self.totals().map(|totals| SystemState::count(totals.plays)),
            NUMBER_TOTALCLEARCOUNT => self.totals().map(|totals| SystemState::count(totals.clears)),
            NUMBER_TOTALFAILCOUNT => self.totals().map(|totals| SystemState::count(totals.plays) - SystemState::count(totals.clears)),
            NUMBER_TOTALPERFECT..=NUMBER_TOTALPOOR => {
                let kind = usize::try_from(id - NUMBER_TOTALPERFECT).ok()?;
                self.totals().map(|totals| SystemState::count(totals.judgements[kind]))
            }
            NUMBER_TOTALPLAYNOTES => self.totals().map(|totals| SystemState::count(totals.played_notes())),
            NUMBER_MASTER_VOLUME => self.volume_percent(VolumeBus::System),
            NUMBER_KEY_VOLUME => self.volume_percent(VolumeBus::Key),
            NUMBER_BGM_VOLUME => self.volume_percent(VolumeBus::Background),
            _ => None,
        }
    }

    fn rate(&self, id: i32) -> Option<f32> {
        match id {
            RATE_MASTERVOLUME => self.volume(VolumeBus::System),
            RATE_KEYVOLUME => self.volume(VolumeBus::Key),
            RATE_BGMVOLUME => self.volume(VolumeBus::Background),
            _ => None,
        }
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        match id {
            STRING_PLAYER => self.player_name.map(Cow::Borrowed),
            STRING_VERSION => self.version.map(Cow::Borrowed),
            _ => None,
        }
    }

    fn volume(&self, bus: VolumeBus) -> Option<f32> {
        self.volumes.map(|volumes| volumes.of(bus))
    }
}

#[cfg(test)]
mod tests;
