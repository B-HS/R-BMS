//! Property cluster A: the chart in hand.
//!
//! Where the values come from: what the library knows about a chart and what was measured from its
//! notes: title, artist and genre, level and difficulty, tempo, length, note counts by kind,
//! density and total, and which of the chart's extras (a BGA, a text document, long notes, a stage
//! image) it has.
//!
//! What this cluster answers: numbers 45-49, 74, 90-92, 96, 106, 350-353, 360-365, 368, 400 and
//! 1163-1164; floats 360, 362, 367 and 368; options 150-155, 160-164, 170-184, 190-195, 1008,
//! 1160-1161 and 1177; strings 10-16, 1001-1003 and 1030-1031.
//!
//! A value is read exactly as the reference reads it from the selected `SongData` and
//! `SongInformation` (`IntegerPropertyFactory`, `BooleanPropertyFactory`, `StringPropertyFactory`
//! and `FloatPropertyFactory`), including the odd ones: the five folder levels all show the chart's
//! own level, the judge options take the raw `#RANK` value, and the density digits after the dot
//! are two truncated digits.
//!
//! What the player does not know about a chart is `None` in its [`ChartMeta`], and a field that is
//! `None` is an id this cluster does not answer rather than a guess, so the host falls through to
//! whatever else can.

use std::borrow::Cow;

use rbms_library::{ChartDetail, SongEntry};
use rbms_model::{Mode, default_total_for_mode};
use rbms_skin::property::generated::*;
use rbms_skin::property::{FLOAT_ABSENT, INTEGER_ABSENT};
use rbms_skin::timer::MICROS_PER_MILLI;

use super::ClusterState;

/// The difficulty slots a chart can name, from beginner to insane. Anything outside them is the
/// "unnamed" slot, which is also the one a chart with no `#DIFFICULTY` has.
const FIRST_NAMED_DIFFICULTY: i32 = 1;
const LAST_NAMED_DIFFICULTY: i32 = 5;

/// How the digits after the dot of a density are cut from it: two digits, truncated.
const DENSITY_FRACTION_SCALE: f64 = 100.0;
const DENSITY_FRACTION_MODULUS: i32 = 100;

const MILLIS_PER_SECOND: i32 = 1000;
const MILLIS_PER_MINUTE: i32 = 60_000;
const CLOCK_DIGIT_MODULUS: i32 = 60;

/// One judge option's reading of the raw judge value: the rank it names outright, and the range of
/// `#DEFEXRANK`-style percentages it also covers (`None` as the upper end for no limit). Ordered
/// from `OPTION_JUDGE_VERYHARD` to `OPTION_JUDGE_VERYEASY`.
const JUDGE_BANDS: [(i32, i32, Option<i32>); 5] = [(0, 10, Some(35)), (1, 35, Some(60)), (2, 60, Some(85)), (3, 85, Some(110)), (4, 110, None)];

/// The note counts a chart is measured to have, by kind, as `SongInformation` holds them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoteCounts {
    /// Normal notes on the keys.
    pub normal: i32,
    /// Long notes on the keys.
    pub long: i32,
    /// Normal notes on the scratch.
    pub scratch: i32,
    /// Long notes on the scratch (BSS).
    pub long_scratch: i32,
}

/// How many notes a second a chart has, as `SongInformation` measures it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Density {
    /// The mean over the seconds that have notes worth counting.
    pub average: f64,
    /// The busiest second.
    pub peak: f64,
    /// The busiest stretch after the gauge's border.
    pub end: f64,
}

/// The lowest and highest tempo a chart plays at, in whole BPM.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BpmRange {
    pub min: i32,
    pub max: i32,
}

/// Which extras a chart has. Each is `None` while the player cannot tell.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChartContents {
    /// A BGA is defined (`SongData.hasBGA`).
    pub bga: Option<bool>,
    /// A text document comes with it (`SongData.hasDocument`).
    pub text: Option<bool>,
    /// Any long note, of any flavour (`SongData.hasAnyLongNote`).
    pub long_note: Option<bool>,
    /// A random sequence (`SongData.hasRandomSequence`).
    pub random_sequence: Option<bool>,
    /// A stop sequence (`SongData.isBpmstop`).
    pub bpm_stop: Option<bool>,
    /// A stage image is loaded (`BMSResource.getStagefile() != null`).
    pub stagefile: Option<bool>,
    /// A banner is loaded.
    pub banner: Option<bool>,
    /// A back image is loaded.
    pub backbmp: Option<bool>,
}

/// Everything the properties of one chart read, as one snapshot.
///
/// Text borrows from wherever the player keeps it. Everything the player may not know yet is an
/// `Option`; see the module documentation for what `None` means.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChartMeta<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub genre: &'a str,
    pub artist: &'a str,
    pub subartist: &'a str,
    /// A title that replaces the chart's own for `STRING_TITLE` and `STRING_FULLTITLE`: the title of
    /// the folder under the browser's cursor, or of the course on the decide and course result
    /// screens. The caller decides when one applies, as the reference does per screen.
    pub heading: Option<&'a str>,
    pub md5: &'a str,
    pub sha256: &'a str,
    /// The difficulty table the chart was picked from and the level it holds there; both empty for
    /// a chart that did not come from one.
    pub table_name: &'a str,
    pub table_level: &'a str,
    /// The `#PLAYLEVEL` as a number; 0 when it does not parse.
    pub level: i32,
    /// The `#DIFFICULTY` slot, 1 to 5 from beginner to insane. Anything else names none.
    pub difficulty: i32,
    pub mode: Option<Mode>,
    /// The raw judge value of the chart (`SongData.getJudge`): its `#RANK`.
    pub judge: Option<i32>,
    /// How long the chart plays, in milliseconds.
    pub length_ms: Option<i32>,
    /// How many notes there are to judge.
    pub notes: Option<i32>,
    pub bpm: Option<BpmRange>,
    /// The tempo the most notes are played at.
    pub main_bpm: Option<f64>,
    pub note_counts: Option<NoteCounts>,
    pub density: Option<Density>,
    /// The chart's `#TOTAL`.
    pub total: Option<f64>,
    pub contents: ChartContents,
}

impl<'a> ChartMeta<'a> {
    /// What a library entry says about its chart, which is everything a scan reads from the header
    /// block: no measurement of the notes and none of the extras but the stage image and the banner.
    pub fn of_entry(entry: &'a SongEntry) -> ChartMeta<'a> {
        ChartMeta {
            title: &entry.title,
            subtitle: &entry.subtitle,
            genre: &entry.genre,
            artist: &entry.artist,
            md5: &entry.md5,
            level: entry.level.trim().parse().unwrap_or_default(),
            difficulty: entry.difficulty,
            mode: Some(entry.mode),
            judge: Some(entry.rank),
            total: Some(entry.total).filter(|total| *total > 0.0),
            contents: ChartContents { stagefile: Some(!entry.stagefile.is_empty()), banner: Some(!entry.banner.is_empty()), ..ChartContents::default() },
            ..ChartMeta::default()
        }
    }

    /// This chart with what measuring its notes found out. A chart that states no `#TOTAL` gets the
    /// one its note count implies, as the reference's model does.
    pub fn with_detail(self, detail: &ChartDetail) -> ChartMeta<'a> {
        let notes = detail.notes;
        ChartMeta {
            notes: i32::try_from(notes).ok(),
            length_ms: i32::try_from(detail.duration_us / MICROS_PER_MILLI).ok(),
            bpm: Some(BpmRange { min: detail.bpm_min as i32, max: detail.bpm_max as i32 }),
            density: Some(Density { average: detail.avg_density, peak: detail.peak_density, end: detail.end_density }),
            total: self.total.or_else(|| self.mode.map(|mode| default_total_for_mode(&mode, notes))),
            ..self
        }
    }

    /// The title `STRING_FULLTITLE` shows: the title, then the subtitle after a space when there is one.
    fn full_title(&self) -> Cow<'_, str> {
        joined(self.title, self.subtitle)
    }

    fn full_artist(&self) -> Cow<'_, str> {
        joined(self.artist, self.subartist)
    }

    fn table_full(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{}{}", self.table_level, self.table_name))
    }
}

/// `first`, then `second` after a space, or `first` alone when there is no `second`.
fn joined<'t>(first: &'t str, second: &str) -> Cow<'t, str> {
    if second.is_empty() { Cow::Borrowed(first) } else { Cow::Owned(format!("{first} {second}")) }
}

/// The slot the chart in hand fills.
#[derive(Debug, Clone, Copy, Default)]
pub enum ChartState<'a> {
    /// Nothing is connected: this cluster knows nothing, so another source may answer.
    #[default]
    Unconnected,
    /// The screen has a chart slot and nothing in it, like a browser resting on a folder. Every id
    /// reads as the reference reads it without a song: no value, no text, no option.
    Empty,
    /// The chart in hand, lent by whoever holds its snapshot.
    Chart(&'a ChartMeta<'a>),
}

/// The option under `id`, as `Some(Some(_))` when the chart knows it, `Some(None)` when `id` is this
/// cluster's but the chart does not say, and `None` when it is not this cluster's.
fn read_boolean(chart: &ChartMeta<'_>, id: i32) -> Option<Option<bool>> {
    let contents = chart.contents;
    let mode = |wanted: Mode| chart.mode.map(|mode| mode == wanted);
    let judge_band = |index: usize| {
        let (rank, from, until) = JUDGE_BANDS[index];
        chart.judge.map(|judge| judge == rank || (from <= judge && until.is_none_or(|until| judge < until)))
    };
    Some(match id {
        OPTION_DIFFICULTY0 => Some(chart.difficulty < FIRST_NAMED_DIFFICULTY || chart.difficulty > LAST_NAMED_DIFFICULTY),
        OPTION_DIFFICULTY1..=OPTION_DIFFICULTY5 => Some(chart.difficulty == id - OPTION_DIFFICULTY0),
        OPTION_7KEYSONG => mode(Mode::BEAT_7K),
        OPTION_5KEYSONG => mode(Mode::BEAT_5K),
        OPTION_14KEYSONG => mode(Mode::BEAT_14K),
        OPTION_10KEYSONG => mode(Mode::BEAT_10K),
        OPTION_9KEYSONG => mode(Mode::POPN_9K),
        OPTION_24KEYSONG => mode(Mode::KEYBOARD_24K),
        OPTION_24KEYDPSONG => chart.mode.map(|_| false),
        OPTION_NO_BGA => contents.bga.map(|has| !has),
        OPTION_BGA => contents.bga,
        OPTION_NO_LN => contents.long_note.map(|has| !has),
        OPTION_LN => contents.long_note,
        OPTION_NO_TEXT => contents.text.map(|has| !has),
        OPTION_TEXT => contents.text,
        OPTION_NO_BPMCHANGE => chart.bpm.map(|bpm| bpm.min == bpm.max),
        OPTION_BPMCHANGE => chart.bpm.map(|bpm| bpm.min < bpm.max),
        OPTION_NO_RANDOMSEQUENCE => contents.random_sequence.map(|has| !has),
        OPTION_RANDOMSEQUENCE => contents.random_sequence,
        OPTION_BPMSTOP => contents.bpm_stop,
        OPTION_JUDGE_VERYHARD => judge_band(0),
        OPTION_JUDGE_HARD => judge_band(1),
        OPTION_JUDGE_NORMAL => judge_band(2),
        OPTION_JUDGE_EASY => judge_band(3),
        OPTION_JUDGE_VERYEASY => judge_band(4),
        OPTION_NO_STAGEFILE => contents.stagefile.map(|has| !has),
        OPTION_STAGEFILE => contents.stagefile,
        OPTION_NO_BANNER => contents.banner.map(|has| !has),
        OPTION_BANNER => contents.banner,
        OPTION_NO_BACKBMP => contents.backbmp.map(|has| !has),
        OPTION_BACKBMP => contents.backbmp,
        OPTION_TABLE_SONG => Some(!chart.table_name.is_empty()),
        _ => return None,
    })
}

/// The number under `id`, read as [`read_boolean`] reads an option.
fn read_integer(chart: &ChartMeta<'_>, id: i32) -> Option<Option<i32>> {
    let counts = chart.note_counts;
    let density = chart.density;
    let digits_after_dot = |value: f64| ((value * DENSITY_FRACTION_SCALE) as i32) % DENSITY_FRACTION_MODULUS;
    Some(match id {
        NUMBER_PLAYLEVEL | NUMBER_FOLDER_BEGINNER..=NUMBER_FOLDER_INSANE => Some(chart.level),
        NUMBER_TOTALNOTES | NUMBER_TOTALNOTES2 => chart.notes,
        NUMBER_MAXBPM => chart.bpm.map(|bpm| bpm.max),
        NUMBER_MINBPM => chart.bpm.map(|bpm| bpm.min),
        NUMBER_MAINBPM => chart.main_bpm.map(|bpm| bpm as i32),
        NUMBER_TOTALNOTE_NORMAL => counts.map(|counts| counts.normal),
        NUMBER_TOTALNOTE_LN => counts.map(|counts| counts.long),
        NUMBER_TOTALNOTE_SCRATCH => counts.map(|counts| counts.scratch),
        NUMBER_TOTALNOTE_BSS => counts.map(|counts| counts.long_scratch),
        NUMBER_DENSITY_PEAK => density.map(|density| density.peak as i32),
        NUMBER_DENSITY_PEAK_AFTERDOT => density.map(|density| digits_after_dot(density.peak)),
        NUMBER_DENSITY_END => density.map(|density| density.end as i32),
        NUMBER_DENSITY_END_AFTERDOT => density.map(|density| digits_after_dot(density.end)),
        NUMBER_DENSITY_AVERAGE => density.map(|density| density.average as i32),
        NUMBER_DENSITY_AVERAGE_AFTERDOT => density.map(|density| digits_after_dot(density.average)),
        NUMBER_SONGGAUGE_TOTAL => chart.total.map(|total| total as i32),
        NUMBER_JUDGERANK => chart.judge,
        NUMBER_SONGLENGTH_MINUTE => chart.length_ms.map(|length| (length / MILLIS_PER_MINUTE) % CLOCK_DIGIT_MODULUS),
        NUMBER_SONGLENGTH_SECOND => chart.length_ms.map(|length| (length / MILLIS_PER_SECOND) % CLOCK_DIGIT_MODULUS),
        _ => return None,
    })
}

/// The number under a `FLOAT_*` id, read as [`read_boolean`] reads an option.
fn read_float(chart: &ChartMeta<'_>, id: i32) -> Option<Option<f32>> {
    let density = chart.density;
    Some(match id {
        FLOAT_CHART_PEAKDENSITY => density.map(|density| density.peak as f32),
        FLOAT_CHART_ENDDENSITY => density.map(|density| density.end as f32),
        FLOAT_CHART_AVERAGEDENSITY => density.map(|density| density.average as f32),
        FLOAT_CHART_TOTALGAUGE => chart.total.map(|total| total as f32),
        _ => return None,
    })
}

/// The text under a `STRING_*` id, or `None` for an id that is not this cluster's.
fn read_text<'c>(chart: &'c ChartMeta<'_>, id: i32) -> Option<Cow<'c, str>> {
    Some(match id {
        STRING_TITLE => Cow::Borrowed(chart.heading.unwrap_or(chart.title)),
        STRING_SUBTITLE => Cow::Borrowed(chart.subtitle),
        STRING_FULLTITLE => chart.heading.map_or_else(|| chart.full_title(), Cow::Borrowed),
        STRING_GENRE => Cow::Borrowed(chart.genre),
        STRING_ARTIST => Cow::Borrowed(chart.artist),
        STRING_SUBARTIST => Cow::Borrowed(chart.subartist),
        STRING_FULLARTIST => chart.full_artist(),
        STRING_TABLE_NAME => Cow::Borrowed(chart.table_name),
        STRING_TABLE_LEVEL => Cow::Borrowed(chart.table_level),
        STRING_TABLE_FULL => chart.table_full(),
        STRING_SONG_HASH_MD5 => Cow::Borrowed(chart.md5),
        STRING_SONG_HASH_SHA256 => Cow::Borrowed(chart.sha256),
        _ => return None,
    })
}

/// What a skin's resource options say about a slot with no chart in it: no stage image, no banner and
/// no back image are loaded, so the "no ..." options are on.
fn empty_slot_option(id: i32) -> bool {
    matches!(id, OPTION_NO_STAGEFILE | OPTION_NO_BANNER | OPTION_NO_BACKBMP)
}

impl ClusterState for ChartState<'_> {
    fn boolean(&self, id: i32) -> Option<bool> {
        match self {
            ChartState::Unconnected => None,
            ChartState::Empty => read_boolean(&ChartMeta::default(), id).map(|_| empty_slot_option(id)),
            ChartState::Chart(chart) => read_boolean(chart, id)?,
        }
    }

    fn integer(&self, id: i32) -> Option<i32> {
        match self {
            ChartState::Unconnected => None,
            ChartState::Empty => read_integer(&ChartMeta::default(), id).map(|_| INTEGER_ABSENT),
            ChartState::Chart(chart) => read_integer(chart, id)?,
        }
    }

    fn float(&self, id: i32) -> Option<f32> {
        match self {
            ChartState::Unconnected => None,
            ChartState::Empty => read_float(&ChartMeta::default(), id).map(|_| FLOAT_ABSENT),
            ChartState::Chart(chart) => read_float(chart, id)?,
        }
    }

    fn text(&self, id: i32) -> Option<Cow<'_, str>> {
        match self {
            ChartState::Unconnected => None,
            ChartState::Empty => read_text(&ChartMeta::default(), id).map(|_| Cow::Borrowed("")),
            ChartState::Chart(chart) => read_text(chart, id),
        }
    }
}

#[cfg(test)]
mod tests;
