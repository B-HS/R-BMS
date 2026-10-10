//! The note field a play document draws for itself: the notes in each lane, the long notes that run
//! between them, the mines, and the lines that cross the field.
//!
//! This is the reference's `LaneRenderer.drawLane` carried over expression by expression, together
//! with the part of `JsonPlaySkinObjectLoader` that builds a `SkinNote`. What the document owns and
//! what the running chart owns are split the way the reference splits them:
//!
//! - The document says where each lane sits (`note.dst`), how tall a note is (`note.size`, or the
//!   height of the note image's first cell), which image each kind of note draws, and where the
//!   lines that mark a measure, a tempo change and a stop are drawn from.
//! - The chart, handed over as [`LaneNotes`], says where the play head is, which timelines there
//!   are, what the player has done to each note and how fast the field scrolls.
//!
//! Nothing here reads the built-in field's geometry. The judgement line is the foot of lane zero's
//! rectangle, raised by the player's lift, and the ceiling is the top of that same rectangle; every
//! other lane is drawn against those two rows whatever its own rectangle says, as the reference has
//! it.
//!
//! Everything is worked out in the reference's own space: output pixels with the origin at the
//! bottom left and `y` growing upwards. A rectangle is turned the screen's way up only as it is
//! handed to the renderer, so every expression below reads as its original does.
//!
//! Several things the reference does are kept although they look like accidents, because a skin is
//! drawn against them:
//!
//! - The row of the first timeline is `section * (time - now) / time`, which is not a number while
//!   that timeline sits at time zero and the play head has not passed it. The walk ends there, so a
//!   field whose chart has not started draws no note at all.
//! - A note the player hit early stays on screen until its own moment comes.
//! - A long note of the plain kind draws no image for its end.
//! - Nothing is clipped. A long note whose head has passed stops at the judgement line only because
//!   a timeline in the past is never moved off it.
//! - A hidden note and a note the document supplied no image for are drawn from the flat
//!   placeholders the reference makes for them, and the document's `hidden` and `processed` lists
//!   are not read, because the reference's loader never reads them.

use std::cell::Cell;

use rbms_model::{LnKind, NoteKind, TimeLine};
use rbms_skin::dst::{DestinationTrack, DrawStateSource, SkinOffset, SkinRect, prepare};
use rbms_skin::loader::{LoadedSkin, NamedTrack, StretchKind};
use rbms_skin::model::{Animation, NoteSet};
use rbms_skin::timer::MICROS_PER_MILLI;

use super::draw::{ImageSelect, Placement};
use super::object::{Body, ImageBody, SkinObject, Sprite, image_sprite};
use super::text::Fonts;
use super::textures::Source;
use super::{SkinAssets, SkinFrame, SkinViewport};
use crate::ctx::RenderCtx;
use crate::{BlendMode, Color, QuadParams, Rect, Renderer, TextureFilter, UvRect};

/// The alpha shift that hides whatever follows the hidden cover's offset while the player has no
/// hidden cover up.
const HIDDEN_COVER_OFF_ALPHA: f32 = -255.0;

/// Milliseconds one measure lasts at one beat a minute.
const MEASURE_MS_AT_ONE_BPM: f64 = 240_000.0;

/// Microseconds one measure lasts at one beat a minute.
const MEASURE_US_AT_ONE_BPM: f64 = 240_000_000.0;

/// The numerator of the speed that keeps a fixed duration, which the reference writes as
/// `2400 / (bpm / 100) / duration`.
const FIXED_HISPEED_NUMERATOR: f32 = 2400.0;

/// The tempo unit that same expression divides by.
const FIXED_HISPEED_BPM_UNIT: f64 = 100.0;

/// Microseconds in a second.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// Milliseconds in a minute, a second and a tenth of a second, which a time line's label is cut by.
const MILLIS_PER_MINUTE: i64 = 60_000;
const MILLIS_PER_SECOND: i64 = 1_000;
const MILLIS_PER_TENTH: i64 = 100;

/// Seconds in a minute and tenths in a second, which the same label wraps its digits at.
const SECONDS_PER_MINUTE: i64 = 60;
const TENTHS_PER_SECOND: i64 = 10;

/// How many timelines behind the walk's start the tempo in force is looked for from.
const TEMPO_LOOKBACK: usize = 5;

/// Percent one whole expansion rate is written as.
const FULL_EXPANSION_PERCENT: i32 = 100;

/// Milliseconds a note takes to swell after a quarter note (`noteExpansionTime`).
const NOTE_EXPANSION_MS: f32 = 9.0;

/// Milliseconds it then takes to shrink back (`noteContractionTime`).
const NOTE_CONTRACTION_MS: f32 = 150.0;

/// The `Note.getState()` of an object nothing has judged yet.
pub const NOTE_UNJUDGED: u8 = 0;

/// The first `Note.getState()` that says the object went badly: a bad, a poor or a miss. A field
/// that drops its missed notes keeps showing these.
const NOTE_STATE_BAD: u8 = 4;

/// How many images a lane keeps for its long notes.
const LONG_IMAGES: usize = 10;

/// The long-note images by the place the reference keeps each in (`SkinLane.longnote`).
const LONG_END: usize = 0;
const LONG_START: usize = 1;
const LONG_BODY_HELD: usize = 2;
const LONG_BODY_IDLE: usize = 3;
const HELL_END: usize = 4;
const HELL_START: usize = 5;
const HELL_BODY_HELD: usize = 6;
const HELL_BODY_IDLE: usize = 7;
const HELL_BODY_GAINING: usize = 8;
const HELL_BODY_DRAINING: usize = 9;

/// The size of the flat image the reference makes for a note the document supplied none for.
const PLACEHOLDER_W: f32 = 32.0;
const PLACEHOLDER_H: f32 = 8.0;

/// How thick the frame of the two outlined placeholders is, in that image's own pixels.
const PLACEHOLDER_BORDER: f32 = 2.0;

/// The colours of those placeholders: a note, a long note, a mine, a hidden note and a note already
/// judged (`Color.WHITE`, `YELLOW`, `RED`, `ORANGE` and `CYAN`).
const PLACEHOLDER_NOTE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
const PLACEHOLDER_LONG: Color = Color { r: 255, g: 255, b: 0, a: 255 };
const PLACEHOLDER_MINE: Color = Color { r: 255, g: 0, b: 0, a: 255 };
const PLACEHOLDER_HIDDEN: Color = Color { r: 255, g: 165, b: 0, a: 255 };
const PLACEHOLDER_PROCESSED: Color = Color { r: 0, g: 255, b: 255, a: 255 };

/// The colour the batch is set to while notes are drawn (`Color.WHITE`): every channel full, which
/// leaves an image exactly as it is.
const BATCH_WHITE: Color = Color { r: u8::MAX, g: u8::MAX, b: u8::MAX, a: u8::MAX };

/// The legacy text scale that comes to the eighteen pixels the reference sets its line labels in.
const LABEL_SCALE: f32 = 2.1;

/// How far above its line a label's top edge sits.
const LABEL_RISE: f32 = 20.0;

/// How far in from the left of a lane group a time label starts.
const TIME_LABEL_INSET: f32 = 4.0;

/// The colours of the time, tempo and stop labels (`40c0c0`, `00c000` and `c0c000`).
const TIME_LABEL_COLOR: Color = Color { r: 0x40, g: 0xc0, b: 0xc0, a: 255 };
const BPM_LABEL_COLOR: Color = Color { r: 0x00, g: 0xc0, b: 0x00, a: 255 };
const STOP_LABEL_COLOR: Color = Color { r: 0xc0, g: 0xc0, b: 0x00, a: 255 };

/// How many judgement windows the judgement area overlay draws.
pub const JUDGE_AREA_WINDOWS: usize = 5;

/// The colours of those windows, best first (`0000ff20`, `00ff0020`, `ffff0020`, `ff800020` and
/// `ff000020`).
const JUDGE_AREA_COLORS: [Color; JUDGE_AREA_WINDOWS] = [
    Color { r: 0x00, g: 0x00, b: 0xff, a: 0x20 },
    Color { r: 0x00, g: 0xff, b: 0x00, a: 0x20 },
    Color { r: 0xff, g: 0xff, b: 0x00, a: 0x20 },
    Color { r: 0xff, g: 0x80, b: 0x00, a: 0x20 },
    Color { r: 0xff, g: 0x00, b: 0x00, a: 0x20 },
];

/// What a run has done to the notes of its chart, asked note by note.
///
/// Asked rather than handed over because a frame draws a few dozen notes of a chart of thousands,
/// and the run already keeps the answer.
pub trait NoteStates {
    /// The reference's `Note.getState()` of the note whose head is in `lane` at chart time
    /// `time_us`: [`NOTE_UNJUDGED`] while nothing has judged it, and the judgement it took plus one
    /// after that, so 1 is a perfect great and 5 a note that went by unhit. A long note is asked
    /// for by its head and answers for its head.
    fn state(&self, lane: usize, time_us: i64) -> u8;
}

/// A run in which nothing has been judged, which is every chart before it is played.
#[derive(Debug, Default, Clone, Copy)]
pub struct Unplayed;

impl NoteStates for Unplayed {
    fn state(&self, _lane: usize, _time_us: i64) -> u8 {
        NOTE_UNJUDGED
    }
}

/// The long note one lane has in hand. Each note is named by the chart time of its head, which no
/// two notes of a lane share.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LaneLong {
    /// The long note the player is holding (`JudgeManager.getProcessingLongNote`), which is what
    /// picks the held body image.
    pub processing: Option<i64>,
    /// The hell charge note the play head is inside (`JudgeManager.getPassingLongNote`).
    pub passing: Option<i64>,
    /// Whether that note is gaining gauge rather than draining it
    /// (`JudgeManager.getHellChargeJudge`).
    pub increasing: bool,
}

/// The reference's CONSTANT option: notes further ahead than a fixed time are not drawn, and the
/// ones about to come in fade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantScroll {
    /// How far ahead a note becomes visible, in milliseconds (`PlayConfig.getDuration`).
    pub duration_ms: i32,
    /// How long the fade lasts, in milliseconds (`PlayConfig.getConstantFadeinTime`). Negative
    /// fades a note in before it reaches the visible time rather than after.
    pub fadein_ms: f32,
}

/// The judgement windows the judgement area overlay draws above the line.
#[derive(Debug, Clone, Copy)]
pub struct JudgeArea<'a> {
    /// How early a key may be pressed for each judgement, best first, in microseconds
    /// (`JudgeWindow.getTime(NoteType.NOTE, judge, true)` for judges zero to four).
    pub key_us: [i64; JUDGE_AREA_WINDOWS],
    /// The same for a turntable (`NoteType.SCRATCH`).
    pub scratch_us: [i64; JUDGE_AREA_WINDOWS],
    /// Which lanes are turntables, in lane order (`JudgeManager.isScratch`).
    pub scratch_lanes: &'a [bool],
}

/// What the player chose to have drawn in the field.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoteDisplay<'a> {
    /// Draw a line where the tempo changes and where the chart stops (`isBpmguide`).
    pub bpm_guide: bool,
    /// Keep a note nobody hit on the judgement line once its moment has passed
    /// (`isShowpastnote`).
    pub past_notes: bool,
    /// Draw a note that has been judged with the processed image (`isMarkprocessednote`).
    pub mark_processed: bool,
    /// Draw the chart's hidden notes (`isShowhiddennote`).
    pub hidden_notes: bool,
    /// Draw the judgement windows above the line (`isShowjudgearea`).
    pub judge_area: Option<JudgeArea<'a>>,
    /// The reference's practice setup state: the field stands still at the speed of one, the
    /// CONSTANT option is ignored, and time, tempo and stop lines are drawn.
    pub practice: bool,
}

/// The chart under the play head, for the note object.
///
/// Everything is borrowed from the run: the timelines are the chart's own and the state of a note
/// is asked for when the note is drawn, so a frame copies nothing of the chart.
pub struct LaneNotes<'a> {
    /// Where the play head is, in microseconds of chart time, with the player's judgement timing
    /// already added (`time * 1000`, where `time` is the play timer's elapsed milliseconds plus
    /// `getJudgetiming`). The reference only ever has a whole millisecond here; a host that wants
    /// the same steps passes a multiple of a thousand. Zero while the chart has not started.
    pub microtime: i64,
    /// The chart's timelines in time order, each with its time, section, tempo, stop, scroll and
    /// the notes of every lane. The reference keeps only the timelines that carry a note, a line
    /// or a change; the ones in between sit on the same slope and change nothing drawn. The first
    /// has to be the one at section zero for the walk to begin as the reference's does.
    pub timelines: &'a [TimeLine],
    /// The tempo the chart starts at (`BMSModel.getBpm`), in force until the first timeline.
    pub initial_bpm: f64,
    /// The chart's `#SPEED` at the play head, interpolated between the two timelines that set it
    /// (`getCurrentSpeed`). One for a chart with none.
    pub speed: f64,
    /// The scroll speed in force (`PlayConfig.getHispeed`), after any fixing to a tempo
    /// ([`fixed_hispeed`]).
    pub hispeed: f32,
    /// The CONSTANT option, when it is on.
    pub constant: Option<ConstantScroll>,
    /// How much of the field the lift takes from its foot, when the lift is on (`getLift`).
    pub lift: Option<f32>,
    /// How much of the field the lane cover takes from its top, when the cover is on
    /// (`getLanecover`).
    pub lanecover: Option<f32>,
    /// How much of the field the hidden cover takes, when it is on (`getHidden`).
    pub hidden: Option<f32>,
    /// What a long note the chart gave no kind is played as (`BMSModel.getLntype`). A note that
    /// states its own kind keeps it.
    pub ln_mode: LnKind,
    /// What the run has done to each note.
    pub states: &'a dyn NoteStates,
    /// The long note each lane has in hand, in lane order. A lane past the end has none.
    pub longs: &'a [LaneLong],
    pub show: NoteDisplay<'a>,
    /// Milliseconds since the last quarter note, on the scene clock
    /// (`now - getNowQuarterNoteTime()`). Only a document with an expansion rate reads it.
    pub quarter_note_ms: i64,
    /// How long after its moment a missed note starts to fall, in microseconds
    /// (`|JudgeWindow.getTime(NoteType.NOTE, 4, false)|`). Only a document with `dst2` reads it.
    pub late_window_us: i64,
}

impl<'a> LaneNotes<'a> {
    /// A chart nobody has played, at the speed of one with nothing covered and nothing extra shown.
    pub fn new(timelines: &'a [TimeLine], microtime: i64, initial_bpm: f64) -> LaneNotes<'a> {
        LaneNotes {
            microtime,
            timelines,
            initial_bpm,
            speed: 1.0,
            hispeed: 1.0,
            constant: None,
            lift: None,
            lanecover: None,
            hidden: None,
            ln_mode: LnKind::Ln,
            states: &Unplayed,
            longs: &[],
            show: NoteDisplay::default(),
            quarter_note_ms: i64::MAX,
            late_window_us: 0,
        }
    }

    /// The tempo and the scroll in force at the play head (`LaneRenderer.getNowBPM` and the
    /// `nscroll` beside it): those of the last timeline the play head has reached, and the chart's
    /// opening tempo with a scroll of one before the first.
    pub fn tempo(&self) -> (f64, f64) {
        let reached = self.timelines.partition_point(|timeline| timeline.time_us <= self.microtime);
        reached.checked_sub(1).map_or((self.initial_bpm, 1.0), |last| (self.timelines[last].bpm, self.timelines[last].scroll))
    }

    /// How long a note takes to cross what the lane cover leaves of the field, in milliseconds
    /// (`LaneRenderer.getCurrentDuration`).
    pub fn current_duration_ms(&self) -> i32 {
        let (bpm, scroll) = self.tempo();
        let hispeed = if self.show.practice { 1.0 } else { self.hispeed };
        let speed = if self.constant.is_some() && !self.show.practice { 1.0 } else { self.speed };
        current_duration_ms(bpm, scroll, hispeed, speed, self.lanecover)
    }
}

impl std::fmt::Debug for LaneNotes<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LaneNotes")
            .field("microtime", &self.microtime)
            .field("timelines", &self.timelines.len())
            .field("hispeed", &self.hispeed)
            .field("lift", &self.lift)
            .field("lanecover", &self.lanecover)
            .field("hidden", &self.hidden)
            .finish_non_exhaustive()
    }
}

/// The three offsets the note field publishes for the objects that follow the lift and the covers,
/// each under the id the property table gives it ([`rbms_skin::property::generated`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneOffsets {
    /// `OFFSET_LIFT`: how far the lift raises the judgement line.
    pub lift: SkinOffset,
    /// `OFFSET_LANECOVER`: how far down the lane cover has come, which is a negative `y`.
    pub lanecover: SkinOffset,
    /// `OFFSET_HIDDEN_COVER`: how far up the hidden cover has come, and an alpha that hides what
    /// follows it while the player has none.
    pub hidden: SkinOffset,
}

/// The offsets the reference writes each time it draws the note field (`LaneRenderer.drawLane`).
///
/// `first_lane` is the rectangle of lane zero, and the offsets come back in whatever unit it is
/// in. The reference writes them while it draws the note object, so an object the document placed
/// before the note field reads the values of the frame before.
///
/// While the player has no hidden cover the reference leaves that offset's `y` as it last was and
/// only sets its alpha; here the `y` reads as zero, which nothing sees through an alpha that hides
/// the object.
pub fn lane_offsets(first_lane: SkinRect, lift: Option<f32>, lanecover: Option<f32>, hidden: Option<f32>) -> LaneOffsets {
    let (ceiling, line) = field_rows(first_lane, lift);
    let foot = f64::from(first_lane.y);
    let cover = lanecover.unwrap_or_default();
    let hidden = match hidden {
        Some(share) => SkinOffset { y: lift.map_or(share, |lift| (1.0 - lift) * share) * first_lane.h, ..SkinOffset::default() },
        None => SkinOffset { a: HIDDEN_COVER_OFF_ALPHA, ..SkinOffset::default() },
    };
    LaneOffsets {
        lift: SkinOffset { y: (line - foot) as f32, ..SkinOffset::default() },
        lanecover: SkinOffset { y: ((line - ceiling) * f64::from(cover)) as f32, ..SkinOffset::default() },
        hidden,
    }
}

/// The rectangle of lane zero as the document wrote it, which is what [`lane_offsets`] is measured
/// against, or `None` for a document with no note field.
pub fn first_lane_rect(skin: &LoadedSkin) -> Option<SkinRect> {
    skin.def.note.as_ref().and_then(|note| note.dst.first()).map(animation_rect)
}

/// The scroll speed that makes a note take `duration_ms` to cross the field at `bpm`
/// (`LaneRenderer.resetHispeed`), which is what the reference sets the speed to whenever the cover
/// or the duration moves while the speed is fixed to a tempo.
pub fn fixed_hispeed(bpm: f64, duration_ms: i32, lanecover: Option<f32>) -> f32 {
    ((f64::from(FIXED_HISPEED_NUMERATOR) / (bpm / FIXED_HISPEED_BPM_UNIT) / f64::from(duration_ms)) as f32) * (1.0 - lanecover.unwrap_or_default())
}

/// How long a note takes to cross what the lane cover leaves of the field, in milliseconds, at a
/// given tempo, scroll and speed (`region * (1 - lanecover)`, rounded).
pub fn current_duration_ms(bpm: f64, scroll: f64, hispeed: f32, speed: f64, lanecover: Option<f32>) -> i32 {
    let region = if scroll > 0.0 { (MEASURE_MS_AT_ONE_BPM / bpm / f64::from(hispeed) / speed) / scroll } else { 0.0 };
    java_round(region * f64::from(1.0 - lanecover.unwrap_or_default()))
}

/// What `Math.round` adds before it rounds down.
const ROUND_HALF: f64 = 0.5;

/// `Math.round` of a double narrowed to an int: half rounds up, and what does not fit saturates.
fn java_round(value: f64) -> i32 {
    (value + ROUND_HALF).floor() as i32
}

/// The top of the field and its judgement line, in the unit of `first_lane`: `hu` and `hl`.
fn field_rows(first_lane: SkinRect, lift: Option<f32>) -> (f64, f64) {
    let ceiling = f64::from(first_lane.y + first_lane.h);
    let line = f64::from(lift.map_or(first_lane.y, |lift| first_lane.y + first_lane.h * lift));
    (ceiling, line)
}

/// The tempo and scroll of the last timeline at or before the play head, looked for from a few
/// timelines before `start`.
fn tempo_at(timelines: &[TimeLine], start: usize, microtime: i64, initial_bpm: f64) -> (f64, f64) {
    let from = start.saturating_sub(TEMPO_LOOKBACK);
    timelines
        .iter()
        .skip(from)
        .take_while(|timeline| timeline.time_us <= microtime)
        .last()
        .map_or((initial_bpm, 1.0), |timeline| (timeline.bpm, timeline.scroll))
}

/// What one kind of note is drawn from.
#[derive(Debug, Clone, Copy)]
pub(crate) enum NoteImage {
    /// An image of the document's.
    Sprite(Sprite),
    /// The reference's flat placeholder for an image the document did not supply.
    Solid(Color),
    /// Its outlined placeholder, which is all a hidden or an already judged note is ever drawn from.
    Outline(Color),
}

/// One lane of the field: where it sits, how tall its notes are, and what it draws them with.
#[derive(Debug)]
pub(crate) struct NoteLane {
    /// The lane rectangle the document declared, in its own space.
    pub(crate) rect: SkinRect,
    /// How tall one note is, in the document's own pixels (`SkinLane.scale` before it is scaled).
    pub(crate) height: f32,
    pub(crate) note: NoteImage,
    /// The long-note images by the reference's own numbering.
    pub(crate) long: [NoteImage; LONG_IMAGES],
    pub(crate) mine: NoteImage,
    pub(crate) hidden: NoteImage,
    pub(crate) processed: NoteImage,
}

impl NoteLane {
    /// A lane drawn from the reference's placeholders alone.
    pub(crate) fn placeholder(rect: SkinRect, height: f32) -> NoteLane {
        NoteLane {
            rect,
            height,
            note: NoteImage::Solid(PLACEHOLDER_NOTE),
            long: [NoteImage::Solid(PLACEHOLDER_LONG); LONG_IMAGES],
            mine: NoteImage::Solid(PLACEHOLDER_MINE),
            hidden: NoteImage::Outline(PLACEHOLDER_HIDDEN),
            processed: NoteImage::Outline(PLACEHOLDER_PROCESSED),
        }
    }

    /// Every image of the lane, for the prepare stage.
    fn images(&self) -> impl Iterator<Item = &NoteImage> {
        [&self.note, &self.mine, &self.hidden, &self.processed].into_iter().chain(self.long.iter())
    }
}

/// One line across the field: an image with a destination of its own, which the chart moves up from
/// where that destination puts it.
#[derive(Debug)]
pub(crate) struct FieldLine {
    /// The image object the line is, with its own keyframes, colour, blend, conditions and offsets.
    pub(crate) object: SkinObject,
    pub(crate) sprite: Sprite,
}

impl FieldLine {
    /// A line over one destination and the image it shows.
    pub(crate) fn new(track: DestinationTrack, sprite: Sprite) -> FieldLine {
        let body = Body::Image(ImageBody { variants: vec![Some((sprite, 0, sprite.cells()))], select: ImageSelect::First });
        FieldLine { object: SkinObject { stretch: StretchKind::from_id(track.stretch), track, body }, sprite }
    }
}

/// Where the walk over the timelines was left, so the next frame need not start from the top of the
/// chart (`LaneRenderer.pos`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Walk {
    /// Which chart this was, by where its timelines are and how many there are.
    chart: (usize, usize),
    microtime: i64,
    pos: usize,
}

/// The note field, resolved from the document's `note` object.
#[derive(Debug)]
pub(crate) struct NoteBody {
    pub(crate) lanes: Vec<NoteLane>,
    /// The lines drawn on a measure line (`group`), one for each lane group.
    pub(crate) section_lines: Vec<FieldLine>,
    /// The lines drawn where a second begins, in the practice setup (`time`).
    pub(crate) time_lines: Vec<FieldLine>,
    /// The lines drawn where the tempo changes (`bpm`).
    pub(crate) bpm_lines: Vec<FieldLine>,
    /// The lines drawn where the chart stops (`stop`).
    pub(crate) stop_lines: Vec<FieldLine>,
    /// The first rectangle of every `group` entry, which the labels of those lines are set against.
    pub(crate) groups: Vec<SkinRect>,
    /// How much wider and taller a note swells on a quarter note, in percent.
    pub(crate) expansion: (i32, i32),
    /// The row a missed note falls to, in the document's own pixels (`dst2`), for the field that
    /// drops its missed notes.
    pub(crate) fall_to: Option<i32>,
    walk: Cell<Walk>,
}

impl NoteBody {
    /// A field over `lanes` with no lines, no expansion and nothing falling.
    pub(crate) fn new(lanes: Vec<NoteLane>) -> NoteBody {
        NoteBody {
            lanes,
            section_lines: Vec::new(),
            time_lines: Vec::new(),
            bpm_lines: Vec::new(),
            stop_lines: Vec::new(),
            groups: Vec::new(),
            expansion: (FULL_EXPANSION_PERCENT, FULL_EXPANSION_PERCENT),
            fall_to: None,
            walk: Cell::new(Walk::default()),
        }
    }

    /// Every line of the field, for the prepare stage.
    fn lines(&self) -> impl Iterator<Item = &FieldLine> {
        self.section_lines.iter().chain(&self.time_lines).chain(&self.bpm_lines).chain(&self.stop_lines)
    }

    /// Where this frame's walk starts: where the last one was left, unless the chart is another one
    /// or its clock has gone back, which is a walk from the top.
    fn resume(&self, notes: &LaneNotes<'_>) -> usize {
        let walk = self.walk.get();
        let same_chart = walk.chart == chart_identity(notes.timelines);
        if notes.show.practice || !same_chart || notes.microtime < walk.microtime { 0 } else { walk.pos.min(notes.timelines.len()) }
    }
}

/// What tells one chart from another between two frames.
fn chart_identity(timelines: &[TimeLine]) -> (usize, usize) {
    (timelines.as_ptr() as usize, timelines.len())
}

/// The note field behind `id`, or `None` when the document declares no `note` by that name.
pub(crate) fn build_note(
    skin: &LoadedSkin,
    id: &str,
    sources: Source<'_>,
    _fonts: &Fonts,
    _assets: &mut dyn SkinAssets,
    warnings: &mut Vec<String>,
) -> Option<Body> {
    let def = skin.def.note.as_ref().filter(|note| note.id == id)?;
    let names = long_names(def);
    let count = def.note.len().min(def.dst.len());
    if count < def.note.len() {
        warnings.push(format!("note {id:?} names {} lanes and places {}; the rest are not drawn", def.note.len(), def.dst.len()));
    }
    let lanes: Vec<NoteLane> = (0..count).map(|lane| build_lane(skin, def, &names, lane, sources)).collect();
    if lanes.is_empty() {
        warnings.push(format!("note {id:?} declares no lane it both names and places"));
        return None;
    }
    let nested = &skin.nested;
    let groups = nested.note_group.len();
    let expansion = |index: usize| def.expansionrate.get(index).copied().unwrap_or(FULL_EXPANSION_PERCENT);
    Some(Body::Note(NoteBody {
        section_lines: build_lines(skin, &nested.note_group, groups, sources),
        time_lines: build_lines(skin, &nested.note_time, groups, sources),
        bpm_lines: build_lines(skin, &nested.note_bpm, groups, sources),
        stop_lines: build_lines(skin, &nested.note_stop, groups, sources),
        groups: nested.note_group.iter().filter_map(|named| named.track.frames.first().map(|frame| frame.rect)).collect(),
        expansion: (expansion(0), expansion(1)),
        fall_to: def.dst2,
        ..NoteBody::new(lanes)
    }))
}

/// Which of the document's lists fills each of a lane's ten long-note images
/// (`JsonPlaySkinObjectLoader`).
///
/// The reference has two namings. The older one is used whenever the newer `lnbodyActive` or
/// `hcnbodyActive` is left empty, and its names do not say what they do: `lnbody` is the body of a
/// note being held and `lnactive` the body of one that is not, `hcndamage` is a hell charge note
/// gaining and `hcnreactive` one draining.
pub(crate) fn long_names(def: &NoteSet) -> [&[String]; LONG_IMAGES] {
    let (held, idle) = if def.lnbody_active.is_empty() { (&def.lnbody, &def.lnactive) } else { (&def.lnbody_active, &def.lnbody) };
    let hell = if def.hcnbody_active.is_empty() {
        [&def.hcnbody, &def.hcnactive, &def.hcndamage, &def.hcnreactive]
    } else {
        [&def.hcnbody_active, &def.hcnbody, &def.hcnbody_reactive, &def.hcnbody_miss]
    };
    let mut names: [&[String]; LONG_IMAGES] = [&[]; LONG_IMAGES];
    names[LONG_END] = &def.lnend;
    names[LONG_START] = &def.lnstart;
    names[LONG_BODY_HELD] = held;
    names[LONG_BODY_IDLE] = idle;
    names[HELL_END] = &def.hcnend;
    names[HELL_START] = &def.hcnstart;
    names[HELL_BODY_HELD] = hell[0];
    names[HELL_BODY_IDLE] = hell[1];
    names[HELL_BODY_GAINING] = hell[2];
    names[HELL_BODY_DRAINING] = hell[3];
    names
}

/// One lane's rectangle, note height and images.
///
/// A note is as tall as `size` says for its lane and, for a lane `size` does not reach, as tall as
/// the first cell of its note image. A lane with no note image at all is as tall as the placeholder
/// drawn in its place; the reference does not load such a document.
fn build_lane(skin: &LoadedSkin, def: &NoteSet, names: &[&[String]; LONG_IMAGES], lane: usize, sources: Source<'_>) -> NoteLane {
    let image = |list: &[String], missing: NoteImage| list.get(lane).and_then(|name| sprite_named(skin, name, sources)).map_or(missing, NoteImage::Sprite);
    let note = image(&def.note, NoteImage::Solid(PLACEHOLDER_NOTE));
    let cell_height = match note {
        NoteImage::Sprite(sprite) => sprite.cell_size().1,
        NoteImage::Solid(_) | NoteImage::Outline(_) => PLACEHOLDER_H,
    };
    NoteLane {
        note,
        long: std::array::from_fn(|slot| image(names[slot], NoteImage::Solid(PLACEHOLDER_LONG))),
        mine: image(&def.mine, NoteImage::Solid(PLACEHOLDER_MINE)),
        ..NoteLane::placeholder(animation_rect(&def.dst[lane]), def.size.get(lane).copied().unwrap_or(cell_height))
    }
}

/// The lines one of the note set's four lists declares, each over the track the loader assembled
/// for it. Only as many as there are lane groups are read, which is the length the reference gives
/// every one of these lists.
///
/// A track with no keyframes is a slot the document's own customisation ruled out, and an entry
/// whose image the document does not declare has nothing to show; neither is kept. The reference
/// stops at the second with an error the first time such a line would be drawn.
fn build_lines(skin: &LoadedSkin, tracks: &[NamedTrack], groups: usize, sources: Source<'_>) -> Vec<FieldLine> {
    tracks
        .iter()
        .take(groups)
        .filter(|named| !named.track.frames.is_empty())
        .filter_map(|named| Some(FieldLine::new(named.track.clone(), sprite_named(skin, &named.id, sources)?)))
        .collect()
}

/// The sprite behind one of a note set's image ids.
fn sprite_named(skin: &LoadedSkin, name: &str, sources: Source<'_>) -> Option<Sprite> {
    let image = skin.def.image.iter().find(|image| image.id == name)?;
    image_sprite(image, sources)
}

/// The rectangle one keyframe states, with everything it left out reading as zero.
fn animation_rect(dst: &Animation) -> SkinRect {
    SkinRect::new(dst.x.unwrap_or_default() as f32, dst.y.unwrap_or_default() as f32, dst.w.unwrap_or_default() as f32, dst.h.unwrap_or_default() as f32)
}

/// Everything the field reads from the skin's Lua, read while the frame is prepared
/// (`SkinNote.prepare`, and the `prepare` each line runs as it is drawn).
///
/// The reference asks a line's conditions and its timer once for every place the line is drawn at.
/// Here they are asked once a frame, drawn or not, and the answers are what every place is drawn
/// with.
pub(crate) fn prepare_note(body: &NoteBody, frame: &SkinFrame<'_>) {
    for image in body.lanes.iter().flat_map(NoteLane::images) {
        if let NoteImage::Sprite(sprite) = image {
            sprite.prepare(frame);
        }
    }
    let state: &dyn DrawStateSource = frame.state;
    for line in body.lines() {
        prepare(&line.object.track, frame.now_us, frame.timers, state, frame.script(), (0.0, 0.0), frame.mouse);
        line.sprite.prepare(frame);
    }
}

/// The screen as the reference measures it: output pixels from the bottom left, `y` upwards.
#[derive(Debug, Clone, Copy)]
struct Output {
    scale_x: f32,
    scale_y: f32,
    /// The height of the screen, which is what turns a row the other way up.
    height: f32,
}

impl Output {
    fn of(viewport: &SkinViewport) -> Output {
        Output { scale_x: viewport.scale_x(), scale_y: viewport.scale_y(), height: viewport.place(SkinRect::default()).y }
    }

    /// A rectangle of the document in output pixels (`dest.x * dx` and so on).
    fn scaled(&self, rect: SkinRect) -> SkinRect {
        SkinRect::new(rect.x * self.scale_x, rect.y * self.scale_y, rect.w * self.scale_x, rect.h * self.scale_y)
    }

    /// The screen rectangle and the two flips of a quad the reference draws at `(x, y, w, h)`, or
    /// `None` when it has no area or a side that is not a number, which reaches no pixel.
    ///
    /// A negative width or height reaches back from the corner and reads its image the other way
    /// round, which is `SpriteBatch.draw` given one.
    fn quad(&self, x: f32, y: f32, w: f32, h: f32) -> Option<(Rect, bool, bool)> {
        if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) || w == 0.0 || h == 0.0 {
            return None;
        }
        let (left, mirrored) = if w < 0.0 { (x + w, true) } else { (x, false) };
        let (top, flipped) = if h < 0.0 { (y, true) } else { (y + h, false) };
        Some((Rect::new(left, self.height - top, w.abs(), h.abs()), mirrored, flipped))
    }
}

/// One lane as a frame draws it: its rectangle and its note height in output pixels.
struct Lane<'a> {
    region: SkinRect,
    scale: f32,
    images: &'a NoteLane,
    long: LaneLong,
}

/// A colour with its alpha scaled by the batch's.
fn faded(color: Color, alpha: u8) -> Color {
    Color { a: ((u16::from(color.a) * u16::from(alpha)) / u16::from(u8::MAX)) as u8, ..color }
}

/// What the CONSTANT option does to one timeline.
enum Fade {
    /// It is too far ahead to be drawn, and is not walked over either.
    Skip,
    /// It is drawn at this alpha.
    Alpha(f32),
}

impl ConstantScroll {
    /// What becomes of a timeline at `time_us` with the play head at `microtime`.
    fn fade(&self, time_us: i64, microtime: i64) -> Fade {
        let target = microtime + i64::from(self.duration_ms) * MICROS_PER_MILLI;
        let limit = self.fadein_ms * MICROS_PER_MILLI as f32;
        let difference = (time_us - target) as f32;
        let ahead = time_us >= target;
        if limit >= 0.0 {
            match (ahead, difference < limit) {
                (true, true) => Fade::Alpha((limit - difference) / limit),
                (true, false) => Fade::Skip,
                (false, _) => Fade::Alpha(1.0),
            }
        } else if ahead {
            Fade::Skip
        } else if difference > limit {
            Fade::Alpha(1.0 - (limit - difference) / limit)
        } else {
            Fade::Alpha(1.0)
        }
    }
}

/// How far the row moves up for the timeline at `index`, which is in the future.
///
/// From the timeline before it: the whole distance between the two while that one is still ahead or
/// holding the chart in a stop, and otherwise the share of it the play head has yet to cover. The
/// very first timeline is measured from the start of the chart instead.
fn rise(timelines: &[TimeLine], index: usize, microtime: i64, rxhs: f64) -> f64 {
    let timeline = &timelines[index];
    match index.checked_sub(1).map(|before| &timelines[before]) {
        Some(previous) => step(previous, timeline, microtime, rxhs),
        None => timeline.section * (timeline.time_us - microtime) as f64 / timeline.time_us as f64 * rxhs,
    }
}

/// The distance between two neighbouring timelines of which the later is in the future.
fn step(previous: &TimeLine, timeline: &TimeLine, microtime: i64, rxhs: f64) -> f64 {
    if previous.time_us + previous.stop_us > microtime {
        (timeline.section - previous.section) * previous.scroll * rxhs
    } else {
        (timeline.section - previous.section) * previous.scroll * (timeline.time_us - microtime) as f64
            / (timeline.time_us - previous.time_us - previous.stop_us) as f64
            * rxhs
    }
}

/// The note in `lane` of a timeline.
fn note_in(timeline: &TimeLine, lane: usize) -> Option<&rbms_model::Note> {
    timeline.notes.get(lane).and_then(Option::as_ref)
}

/// The timeline that ends the long note whose head is in `lane` of the timeline at `head`.
fn long_end(timelines: &[TimeLine], head: usize, lane: usize) -> Option<&TimeLine> {
    timelines.iter().skip(head + 1).find(|timeline| matches!(note_in(timeline, lane).map(|note| &note.kind), Some(NoteKind::LongEnd { .. })))
}

/// Whether a timeline the play head has passed still has something to draw, which keeps the walk
/// from starting after it: a long note that has not ended, or a note nobody hit while those are
/// kept on the line.
fn still_drawn(notes: &LaneNotes<'_>, index: usize, lanes: usize) -> bool {
    let timeline = &notes.timelines[index];
    (0..lanes).any(|lane| match note_in(timeline, lane).map(|note| &note.kind) {
        Some(NoteKind::LongStart { .. }) => long_end(notes.timelines, index, lane).is_some_and(|end| end.time_us >= notes.microtime),
        Some(NoteKind::LongEnd { .. }) => timeline.time_us >= notes.microtime,
        Some(NoteKind::Normal) => notes.show.past_notes && notes.states.state(lane, timeline.time_us) == NOTE_UNJUDGED,
        Some(NoteKind::Mine { .. }) | None => false,
    })
}

/// How much each side of a note is scaled on a quarter note (`1 + (rate / 100 - 1) * share`), or
/// `None` while the note is at rest.
fn swell(expansion: (i32, i32), since_quarter_ms: i64) -> Option<(f32, f32)> {
    let (wide, tall) = expansion;
    if wide == FULL_EXPANSION_PERCENT && tall == FULL_EXPANSION_PERCENT {
        return None;
    }
    let since = since_quarter_ms as f32;
    let share = if since < NOTE_EXPANSION_MS {
        since / NOTE_EXPANSION_MS
    } else if since <= NOTE_EXPANSION_MS + NOTE_CONTRACTION_MS {
        (NOTE_CONTRACTION_MS - (since - NOTE_EXPANSION_MS)) / NOTE_CONTRACTION_MS
    } else {
        return None;
    };
    let rate = |percent: i32| 1.0 + (percent as f32 / FULL_EXPANSION_PERCENT as f32 - 1.0) * share;
    Some((rate(wide), rate(tall)))
}

/// One long note as its head's timeline finds it.
struct LongNote {
    /// The kind it is played as: the chart's own, or the player's for a note that states none.
    kind: LnKind,
    /// Which timeline its head is in.
    head: usize,
    /// The `Note.getState()` of its head.
    state: u8,
    /// The alpha the batch is at.
    alpha: f32,
}

/// The field of one frame: the chart, the lanes in output pixels and the two rows everything is
/// measured between.
struct Field<'a> {
    out: Output,
    frame: &'a SkinFrame<'a>,
    body: &'a NoteBody,
    notes: &'a LaneNotes<'a>,
    lanes: Vec<Lane<'a>>,
    /// How many output pixels one measure is tall (`rxhs`).
    rxhs: f64,
    /// The top of the field (`hu`).
    ceiling: f64,
    /// The judgement line (`hl`).
    line: f64,
    /// The sum of the note object's offsets, in output pixels.
    nudge: SkinRect,
    constant: Option<ConstantScroll>,
}

impl Field<'_> {
    /// Draws one note image at `(x, y, w, h)` with the batch at `alpha`, answering whether anything
    /// reached the screen.
    ///
    /// The batch is white, alpha blended and unfiltered while the notes are drawn, whatever the
    /// note object's own destination says (`sprite.setColor(Color.WHITE)`, `setBlend(0)`,
    /// `setType(TYPE_NORMAL)`).
    fn put<R: Renderer>(&self, r: &mut R, image: &NoteImage, at: (f32, f32, f32, f32), alpha: f32) -> bool {
        let (x, y, w, h) = at;
        let Some((dst, mirrored, flipped)) = self.out.quad(x, y, w, h) else {
            return false;
        };
        let alpha = (alpha.clamp(0.0, 1.0) * f32::from(u8::MAX)) as u8;
        match image {
            NoteImage::Sprite(sprite) => {
                let frame = self.frame;
                let cell = sprite.animation_index(sprite.cells(), frame.now_us, frame.timers, frame.script());
                let uv = sprite.uv(cell);
                let (u0, u1) = if mirrored { (uv.u1, uv.u0) } else { (uv.u0, uv.u1) };
                let (v0, v1) = if flipped { (uv.v1, uv.v0) } else { (uv.v0, uv.v1) };
                let params = QuadParams {
                    dst,
                    src: UvRect::new(u0, v0, u1, v1),
                    tint: Color { a: alpha, ..BATCH_WHITE },
                    blend: BlendMode::Alpha,
                    filter: TextureFilter::Nearest,
                    angle_deg: 0.0,
                    center: (0.0, 0.0),
                };
                r.draw_textured_quad(sprite.tex, params);
            }
            NoteImage::Solid(color) => r.fill_rect(dst, faded(*color, alpha)),
            NoteImage::Outline(color) => {
                let (across, down) = (dst.w * PLACEHOLDER_BORDER / PLACEHOLDER_W, dst.h * PLACEHOLDER_BORDER / PLACEHOLDER_H);
                let color = faded(*color, alpha);
                r.fill_rect(Rect::new(dst.x, dst.y, dst.w, down), color);
                r.fill_rect(Rect::new(dst.x, dst.y + dst.h - down, dst.w, down), color);
                r.fill_rect(Rect::new(dst.x, dst.y + down, across, dst.h - down * 2.0), color);
                r.fill_rect(Rect::new(dst.x + dst.w - across, dst.y + down, across, dst.h - down * 2.0), color);
            }
        }
        true
    }

    /// A note's rectangle swollen about its lane's own middle, when a quarter note has just passed.
    fn swollen(&self, at: (f32, f32, f32, f32), lane: &Lane<'_>) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = at;
        match swell(self.body.expansion, self.notes.quarter_note_ms) {
            Some((wide, tall)) => {
                let (w, h) = (w * wide, h * tall);
                (x - (w - lane.region.w) / 2.0, y - (h - lane.scale) / 2.0, w, h)
            }
            None => at,
        }
    }

    /// Draws every note, long note, mine and hidden note from the walk's start to the first
    /// timeline past the top of the field, timeline by timeline and lane by lane within each.
    ///
    /// Answers whether anything was drawn and the row the walk was left at, which the pass after
    /// reads.
    fn draw_notes<R: Renderer>(&self, r: &mut R, pos: usize) -> (bool, f64) {
        let notes = self.notes;
        let timelines = notes.timelines;
        let microtime = notes.microtime;
        let falls = self.body.fall_to.is_some();
        let mut drawn = false;
        let mut alpha = 1.0;
        let mut y = self.line;
        let mut index = pos;
        while index < timelines.len() && y <= self.ceiling {
            let timeline = &timelines[index];
            match self.constant.map(|constant| constant.fade(timeline.time_us, microtime)) {
                Some(Fade::Skip) => {
                    index += 1;
                    continue;
                }
                Some(Fade::Alpha(faded)) => alpha = faded,
                None => {}
            }
            let ahead = timeline.time_us >= microtime;
            if ahead {
                y += rise(timelines, index, microtime, self.rxhs);
            }
            for (lane_index, lane) in self.lanes.iter().enumerate() {
                if let Some(note) = note_in(timeline, lane_index) {
                    let plain = (lane.region.x + self.nudge.x, y as f32 + self.nudge.y, lane.region.w + self.nudge.w, lane.scale + self.nudge.h);
                    let at = self.swollen(plain, lane);
                    match note.kind {
                        NoteKind::Normal => {
                            let state = notes.states.state(lane_index, timeline.time_us);
                            let unjudged = state == NOTE_UNJUDGED;
                            let shown = if falls { ahead && (unjudged || state >= NOTE_STATE_BAD) } else { ahead || (notes.show.past_notes && unjudged) };
                            if shown {
                                let image = if notes.show.mark_processed && !unjudged { &lane.images.processed } else { &lane.images.note };
                                drawn |= self.put(r, image, at, alpha);
                            }
                        }
                        NoteKind::LongStart { ln } => {
                            let kind = if ln == LnKind::Undefined { notes.ln_mode } else { ln };
                            let long = LongNote { kind, head: index, state: notes.states.state(lane_index, timeline.time_us), alpha };
                            drawn |= self.draw_long_note(r, lane_index, at, &long);
                        }
                        NoteKind::LongEnd { .. } => {}
                        NoteKind::Mine { .. } => {
                            if ahead {
                                drawn |= self.put(r, &lane.images.mine, at, alpha);
                            }
                        }
                    }
                }
                if notes.show.hidden_notes && ahead && timeline.hidden.get(lane_index).is_some_and(Option::is_some) {
                    drawn |= self.put(r, &lane.images.hidden, (lane.region.x, y as f32, lane.region.w, lane.scale), alpha);
                }
            }
            index += 1;
        }
        (drawn, y)
    }

    /// Draws one long note from its head's timeline, when its end has not passed and lies above its
    /// head. `at` is where the head would be drawn as a plain note.
    ///
    /// The distance to the end is walked the way a row is, from the head's timeline to the one at
    /// the end's section, and the note is then laid out from the foot of its end (`drawLongNote`):
    /// the body first, then the end, then the start over both. A plain long note has no end image.
    fn draw_long_note<R: Renderer>(&self, r: &mut R, lane_index: usize, at: (f32, f32, f32, f32), long: &LongNote) -> bool {
        let timelines = self.notes.timelines;
        let microtime = self.notes.microtime;
        let lane = &self.lanes[lane_index];
        let Some(end) = long_end(timelines, long.head, lane_index).filter(|end| end.time_us >= microtime) else {
            return false;
        };
        let head_us = timelines[long.head].time_us;
        let mut span = 0.0f64;
        let mut previous = &timelines[long.head];
        for timeline in &timelines[long.head + 1..] {
            if previous.section == end.section {
                break;
            }
            if timeline.time_us >= microtime {
                span += step(previous, timeline, microtime, self.rxhs);
            }
            previous = timeline;
        }
        if span.is_nan() || span <= 0.0 {
            return false;
        }

        let (x, y, w, scale) = at;
        let grown = if scale > lane.scale { (scale - lane.scale) / 2.0 } else { 0.0 };
        let foot = lane.region.y - grown;
        let height = (if y < foot { f64::from(y - foot) } else { span }) as f32;
        let end_y = (f64::from(y) + span) as f32;
        let images = &lane.images.long;
        let held = lane.long.processing == Some(head_us);
        let body_at = (x, end_y - height + scale, w, height - scale);
        let end_at = (x, end_y, w, scale);
        let start_at = (x, end_y - height, w, scale);
        let alpha = long.alpha;
        match long.kind {
            LnKind::Hcn => {
                let passing = lane.long.passing == Some(head_us) && long.state != NOTE_UNJUDGED;
                let slot = match (held, passing, lane.long.increasing) {
                    (true, _, _) => HELL_BODY_HELD,
                    (false, true, true) => HELL_BODY_GAINING,
                    (false, true, false) => HELL_BODY_DRAINING,
                    (false, false, _) => HELL_BODY_IDLE,
                };
                let body = self.put(r, &images[slot], body_at, alpha);
                let end = self.put(r, &images[HELL_END], end_at, alpha);
                self.put(r, &images[HELL_START], start_at, alpha) | body | end
            }
            LnKind::Cn => {
                let body = self.put(r, &images[if held { LONG_BODY_HELD } else { LONG_BODY_IDLE }], body_at, alpha);
                let end = self.put(r, &images[LONG_END], end_at, alpha);
                self.put(r, &images[LONG_START], start_at, alpha) | body | end
            }
            LnKind::Ln => {
                let body = self.put(r, &images[if held { LONG_BODY_HELD } else { LONG_BODY_IDLE }], body_at, alpha);
                self.put(r, &images[LONG_START], start_at, alpha) | body
            }
            LnKind::Undefined => false,
        }
    }

    /// Draws the notes the player missed as they fall below the line, for the field that drops them
    /// (`dst2`, the reference's nine-button screens).
    ///
    /// A missed note stays on the line until the late window has passed and then falls at the speed
    /// the field scrolls at with the speed of one, down to the row the document names. `fall_to` is
    /// that row in output pixels and `leftover` the row the note pass was left at, which is what
    /// the reference first tests this pass against.
    fn draw_missed_notes<R: Renderer>(&self, r: &mut R, pos: usize, fall_to: i32, leftover: f64) -> bool {
        let notes = self.notes;
        let timelines = notes.timelines;
        let microtime = notes.microtime;
        let (Some(first), Some(last)) = (self.lanes.first(), timelines.len().checked_sub(1)) else {
            return false;
        };
        let late = notes.late_window_us;
        let origin = self.line;
        let floor = f64::from(fall_to).max(f64::from(-first.region.h)).min(origin);
        let rxhs = self.ceiling - self.line;
        let fallen = |from: &TimeLine, by: &TimeLine, until: i64| {
            let held = ((from.time_us + from.stop_us + late - by.time_us - by.stop_us).max(0)) as f64;
            ((until - by.time_us - by.stop_us) as f64 - held) * rxhs * by.bpm / MEASURE_US_AT_ONE_BPM
        };

        let mut drawn = false;
        let mut y = leftover;
        let mut next = Some((pos..timelines.len()).find(|index| timelines[*index].time_us >= microtime).unwrap_or(last));
        while let Some(index) = next.filter(|_| y >= floor) {
            let timeline = &timelines[index];
            let released = timeline.time_us + timeline.stop_us + late;
            y = origin;
            if index < last {
                let mut by = index;
                while by < last && timelines[by + 1].time_us < microtime {
                    if timelines[by + 1].time_us > released {
                        y -= fallen(timeline, &timelines[by], timelines[by + 1].time_us);
                    }
                    by += 1;
                }
                if timelines[by].time_us + timelines[by].stop_us < microtime && microtime > released {
                    y -= fallen(timeline, &timelines[by], microtime);
                }
            } else if timeline.time_us + timeline.stop_us < microtime && microtime > released {
                y -= fallen(timeline, timeline, microtime);
            }
            for (lane_index, lane) in self.lanes.iter().enumerate() {
                if !matches!(note_in(timeline, lane_index).map(|note| &note.kind), Some(NoteKind::Normal)) {
                    continue;
                }
                let state = notes.states.state(lane_index, timeline.time_us);
                if (state == NOTE_UNJUDGED || state >= NOTE_STATE_BAD) && timeline.time_us <= microtime && y >= floor {
                    let (x, at_y, w, h) = self.swollen((lane.region.x, y as f32, lane.region.w, lane.scale), lane);
                    let row = if y > origin { (origin - f64::from((h - lane.scale) / 2.0)) as f32 } else { at_y };
                    drawn |= self.put(r, &lane.images.note, (x, row, w, h), 1.0);
                }
            }
            next = index.checked_sub(1);
        }
        drawn
    }

    /// Draws the judgement windows above the line in every lane, each as tall as its window is long
    /// at the speed the first timeline ahead is coming in at.
    fn draw_judge_area<R: Renderer>(&self, r: &mut R, area: &JudgeArea<'_>, pos: usize) -> bool {
        let timelines = self.notes.timelines;
        let Some(index) = (pos..timelines.len()).find(|index| timelines[*index].time_us >= self.notes.microtime) else {
            return false;
        };
        let timeline = &timelines[index];
        let (section, scroll, since) = match index.checked_sub(1).map(|before| &timelines[before]) {
            Some(previous) => (previous.section, previous.scroll, previous.time_us + previous.stop_us),
            None => (0.0, 1.0, 0),
        };
        let rate = (timeline.section - section) * scroll * self.rxhs / (timeline.time_us - since) as f64;
        let mut drawn = false;
        for (lane_index, lane) in self.lanes.iter().enumerate() {
            let windows = if area.scratch_lanes.get(lane_index).copied().unwrap_or_default() { &area.scratch_us } else { &area.key_us };
            for judge in (0..JUDGE_AREA_WINDOWS).rev() {
                let inner = if judge > 0 { windows[judge - 1] } else { 0 };
                let (y, h) = ((self.line + inner as f64 * rate) as f32, ((windows[judge] - inner) as f64 * rate) as f32);
                if let Some((dst, ..)) = self.out.quad(lane.region.x, y, lane.region.w, h) {
                    r.fill_rect(dst, JUDGE_AREA_COLORS[judge]);
                    drawn = true;
                }
            }
        }
        drawn
    }
}

/// Draws the field, answering whether anything reached the screen.
///
/// A frame that carries no chart draws none, which is what a document loaded on a screen with no
/// chart running wants.
pub(crate) fn draw_note<R: Renderer>(
    ctx: &mut RenderCtx<'_>,
    r: &mut R,
    place: &Placement<'_>,
    body: &NoteBody,
    _rect: SkinRect,
    frame: &SkinFrame<'_>,
) -> bool {
    let Some(notes) = frame.data.notes else {
        return false;
    };
    let out = Output::of(place.viewport);
    let lanes: Vec<Lane<'_>> = body
        .lanes
        .iter()
        .enumerate()
        .map(|(lane, images)| Lane {
            region: out.scaled(images.rect),
            scale: images.height * out.scale_y,
            images,
            long: notes.longs.get(lane).copied().unwrap_or_default(),
        })
        .collect();
    let Some(first) = lanes.first().map(|lane| lane.region) else {
        return false;
    };

    let timelines = notes.timelines;
    let microtime = notes.microtime;
    let practice = notes.show.practice;
    let hispeed = if practice { 1.0 } else { notes.hispeed };
    let mut pos = body.resume(notes);
    let (mut tempo, _) = tempo_at(timelines, pos, microtime, notes.initial_bpm);
    let constant = notes.constant.filter(|_| !practice);
    let speed = if constant.is_some() { 1.0 } else { notes.speed };
    let (ceiling, line) = field_rows(first, notes.lift);
    let rxhs = (ceiling - line) * f64::from(hispeed) * speed;
    let nudge = out.scaled(note_nudge(place, frame));
    let field = Field { out, frame, body, notes, lanes, rxhs, ceiling, line, nudge, constant };

    let mut drawn = false;
    if let Some(area) = &notes.show.judge_area {
        drawn |= field.draw_judge_area(r, area, pos);
    }

    let mut lines = Lines { ctx, out, viewport: place.viewport, frame, chart_us: microtime };
    let mut y = line;
    let mut index = pos;
    while index < timelines.len() && y <= ceiling {
        let timeline = &timelines[index];
        if timeline.time_us >= microtime {
            if matches!(constant.map(|constant| constant.fade(timeline.time_us, microtime)), Some(Fade::Skip)) {
                index += 1;
                continue;
            }
            y += rise(timelines, index, microtime, rxhs);
            let shift = (y - line) as i32;
            let time_ms = timeline.time_us / MICROS_PER_MILLI;
            if practice && index > 0 && timeline.time_us / MICROS_PER_SECOND > timelines[index - 1].time_us / MICROS_PER_SECOND {
                drawn |= lines.draw(r, &body.time_lines, shift);
                let label = format!(
                    "{:2}:{:02}.{:1}",
                    time_ms / MILLIS_PER_MINUTE,
                    (time_ms / MILLIS_PER_SECOND) % SECONDS_PER_MINUTE,
                    (time_ms / MILLIS_PER_TENTH) % TENTHS_PER_SECOND
                );
                lines.label(r, &body.groups, &label, (y, false), TIME_LABEL_COLOR);
            }
            if notes.show.bpm_guide || practice {
                if timeline.bpm != tempo {
                    drawn |= lines.draw(r, &body.bpm_lines, shift);
                    lines.label(r, &body.groups, &format!("BPM{}", timeline.bpm as i32), (y, true), BPM_LABEL_COLOR);
                }
                let stop_ms = timeline.stop_us / MICROS_PER_MILLI;
                if stop_ms > 0 {
                    drawn |= lines.draw(r, &body.stop_lines, shift);
                    lines.label(r, &body.groups, &format!("STOP {}ms", stop_ms as i32), (y, true), STOP_LABEL_COLOR);
                }
            }
            if timeline.section_line {
                drawn |= lines.draw(r, &body.section_lines, shift);
            }
            tempo = timeline.bpm;
        } else if index > 0 && pos == index - 1 && !still_drawn(notes, index, field.lanes.len()) {
            pos = index;
        }
        index += 1;
    }

    let (notes_drawn, leftover) = field.draw_notes(r, pos);
    drawn |= notes_drawn;
    if let Some(fall_to) = body.fall_to {
        drawn |= field.draw_missed_notes(r, pos, java_round(f64::from(fall_to as f32 * out.scale_y)), leftover);
    }
    body.walk.set(Walk { chart: chart_identity(timelines), microtime, pos });
    drawn
}

/// The sum of the offsets the note object's own destination names, which every note is moved and
/// resized by (`SkinNote.draw` handing `getOffsets()` to `drawLane`).
fn note_nudge(place: &Placement<'_>, frame: &SkinFrame<'_>) -> SkinRect {
    place
        .object
        .track
        .offsets
        .iter()
        .filter_map(|id| frame.state.offset(*id))
        .fold(SkinRect::default(), |sum, offset| SkinRect::new(sum.x + offset.x, sum.y + offset.y, sum.w + offset.w, sum.h + offset.h))
}

/// What every line of one frame is drawn with.
struct Lines<'a, 'c> {
    ctx: &'a mut RenderCtx<'c>,
    out: Output,
    viewport: &'a SkinViewport,
    frame: &'a SkinFrame<'a>,
    /// The chart's own clock in microseconds, which is what a line's keyframes and its image are
    /// animated against rather than the scene's.
    chart_us: i64,
}

impl Lines<'_, '_> {
    /// Draws each of `lines` where its own destination puts it, moved up by `shift` output pixels
    /// (`line.draw(sprite, time, main, 0, (int) (y - hl))`).
    fn draw<R: Renderer>(&mut self, r: &mut R, lines: &[FieldLine], shift: i32) -> bool {
        let frame = self.frame;
        let state: &dyn DrawStateSource = frame.state;
        let viewport = self.viewport;
        let lift = if self.out.scale_y > 0.0 { shift as f32 / self.out.scale_y } else { 0.0 };
        let mut drawn = false;
        for line in lines {
            let track = &line.object.track;
            let Some(resolved) = prepare(track, self.chart_us, frame.timers, state, frame.script(), (0.0, lift), frame.mouse) else {
                continue;
            };
            if resolved.color.a == 0 {
                continue;
            }
            let place = Placement {
                object: &line.object,
                blend: BlendMode::from_skin_blend(track.blend),
                tint: resolved.color.into(),
                angle_deg: resolved.angle_deg,
                viewport,
            };
            if let Some(clip) = resolved.clip {
                r.push_clip(viewport.place(clip));
            }
            let cell = line.sprite.animation_index(line.sprite.cells(), self.chart_us, frame.timers, frame.script());
            drawn |= place.cell(r, &line.sprite, cell, resolved.rect);
            if resolved.clip.is_some() {
                r.pop_clip();
            }
        }
        drawn
    }

    /// Writes `text` beside a line, once for each lane group. `at` is the line's row and whether
    /// the text starts from the group's middle rather than just inside its left edge.
    fn label<R: Renderer>(&mut self, r: &mut R, groups: &[SkinRect], text: &str, at: (f64, bool), color: Color) {
        let (y, from_middle) = at;
        for group in groups {
            let region = self.out.scaled(*group);
            let x = if from_middle { region.x + region.w / 2.0 } else { region.x + TIME_LABEL_INSET };
            let top = self.out.height - (y as f32 + LABEL_RISE);
            if x.is_finite() && top.is_finite() {
                self.ctx.draw_text(r, x, top, LABEL_SCALE, color, text);
            }
        }
    }
}

#[cfg(test)]
mod tests;
