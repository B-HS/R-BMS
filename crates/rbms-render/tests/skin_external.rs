//! Drawing an external skin pack through the skin renderer, with no application in between.
//!
//! The pack is whatever `RBMS_SKIN_PACK` names. Nothing of it is committed and nothing here needs
//! it to pass: with the variable unset the capture test returns at once, and the one test that
//! always runs only checks the scenario files next door.
//!
//! With a pack named, each screen in [`SHOTS`] is taken through the whole path a frame takes:
//!
//! 1. A scenario file (`tests/skin/external/<screen>.json`, a [`MapHost`] written out) stands in for
//!    the game. Every option the reference declares reads as off unless the scenario turns it on,
//!    so a condition written as "not a folder bar" holds.
//! 2. The skin is loaded against that host -- header pass, body pass, the lot.
//! 3. The image sources no drawn object names are never decoded, because a pack ships sheets for
//!    every customisation it offers and decoding them all costs gigabytes. Which ones those are is
//!    the renderer's own answer ([`referenced_sources`]), the one the application decodes by too.
//! 4. [`SkinScreen`] is built on a 1920 by 1080 [`CpuCanvas`] and drawn at a few scene times. Each
//!    frame binds the host to the skin's interpreter once ([`SkinLua::frame`](rbms_skin::lua::SkinLua::frame))
//!    and prepares every object inside that binding, so every `draw`, `value` and `timer` the skin
//!    wrote as a function is called rather than read as its fallback. The frame is drawn after the
//!    binding has ended, from what was prepared.
//! 5. With `RBMS_SKIN_CAPTURE_DIR` set, each frame is written there as `<screen>-<ms>.png`.
//!
//! What is printed along the way -- objects by kind, everything dropped while building, how many
//! images were read and how long a frame took -- is the record of what this build draws of a full
//! skin and what it does not yet.
//!
//! The pack is read only. A skin's writes go to an overlay in the temporary directory, and the
//! pack's folder is compared before and after.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use rbms_chart::to_model;
use rbms_model::{Mode, NoteKind, TimeLine};
use rbms_parser::parse;
use rbms_render::skin_render::frame::{BarDistribution, BarKind, BarScroll, BarTrophy, GAUGE_TYPES, GaugeScale, SongBar};
use rbms_render::skin_render::frame::{BgaEvent, BgaExpand, BgaPicture, BgaPlayhead, BgaTextures, DEFAULT_MISS_LAYER_DURATION_MS};
use rbms_render::skin_render::frame::{JUDGE_REGIONS, JudgeFrame, JudgeHit};
use rbms_render::skin_render::frame::{LaneLong, LaneNotes, NoteDisplay};
use rbms_render::skin_render::graphs::{EARLY_LATE_BUCKETS, GAUGE_SAMPLE_MS, JUDGEMENTS, NOTE_KINDS, PlayCursor, TIMING_JUDGE_AREAS};
use rbms_render::skin_render::textures::referenced_sources;
use rbms_render::{
    BgaFrame, BpmTimeline, Color, CpuCanvas, FrameData, FrameSeries, GaugeFrame, GaugeHistory, NoteDistribution, RecentHits, ReferenceImages, RenderCtx,
    Renderer, SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen, SongBars, TextContext, TextureId, TimingHistogram,
};
use rbms_skin::dst::{DrawCondition, TimerRef};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin_with_host, parse_value};
use rbms_skin::model::EventRef;
use rbms_skin::property::{MapHost, PropertyKind};
use rbms_skin::timer::{MICROS_PER_MILLI, TimerId, TimerState};
use serde::Deserialize;

/// The environment variable that names an external skin pack.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The environment variable that names the folder the frames are written to.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// Width of the canvas a pack is drawn on, the size the pack this was written against is authored at.
const CANVAS_W: u32 = 1920;

/// Height of the canvas a pack is drawn on.
const CANVAS_H: u32 = 1080;

/// The seed every random choice a skin makes while it loads is pinned to.
const TEST_SEED: u64 = 7;

/// The first of the six difficulty options, exactly one of which a scenario turns on.
const DIFFICULTY_FIRST: i32 = 150;

/// The last of the six difficulty options.
const DIFFICULTY_LAST: i32 = 155;

/// The extension of the only image format this test decodes.
const PNG_EXTENSION: &str = "png";

/// Bytes one RGBA8 pixel takes.
const RGBA_BYTES: usize = 4;

/// Bytes one RGB8 pixel takes, which is what a capture is written as: a finished frame is opaque,
/// and an alpha channel left over from blending would only make a viewer show it against its own
/// background.
const RGB_BYTES: usize = 3;

/// An opaque alpha value.
const OPAQUE: u8 = u8::MAX;

/// How many lines of one kind a report prints before it only counts the rest.
const REPORT_LINES: usize = 12;

/// Width of the stand-in background image a frame carries for the skin's `bga` object.
const BACKDROP_W: u32 = 256;

/// Height of the stand-in background image.
const BACKDROP_H: u32 = 144;

/// The scene time, in milliseconds, at which the play scenario's chart starts scrolling: the moment
/// its scenario switches the play timer on.
const PLAY_STARTS_MS: i64 = 4_500;

/// Width and height of the stand-in pictures the background scenario's chart names, 16:9 so a
/// picture fits the pack's background rectangle without bars.
const BGA_PICTURE: (u32, u32) = (640, 360);

/// The chart's picture numbers in the background scenario: the picture, the layer over it, and the
/// picture its miss layer shows.
const BGA_PICTURE_BASE: i32 = 0;
const BGA_PICTURE_LAYER: i32 = 1;
const BGA_PICTURE_MISS: i32 = 2;

/// How far into the chart, in milliseconds, the background scenario's player misses.
const BGA_MISS_AT_MS: i64 = 800;

/// The side of the lit square and the thickness of the bar the layer picture has on black.
const BGA_LAYER_SQUARE: u32 = 160;
const BGA_LAYER_BAR: u32 = 20;

/// The red of the picture the miss layer shows.
const BGA_MISS_RED: u8 = 180;

/// Scroll speed the play scenario's notes are drawn at.
const PLAY_HISPEED: f32 = 1.5;

/// The percent of a full gauge the play scenario's gauge clears at.
const PLAY_GAUGE_BORDER: f32 = 80.0;

/// The chart every play scenario scrolls, written for this test, on the channels a five-key field
/// has on either side, so one text reads as a chart of every mode: a long note that is under the
/// play head while the scenario's last frame is drawn and one that is still on its way, a mine, a
/// tempo that halves half way through the second measure, and plain notes around them.
const PLAY_CHART_FIVE: &[u8] = b"#PLAYER 3\r\n#BPM 150\r\n#WAV01 a.wav\r\n\
#00051:00000001\r\n#00061:00000001\r\n#00151:00010000\r\n#00161:00010000\r\n\
#00153:0001000100000000\r\n#00163:0001000100000000\r\n#001D5:00010000\r\n#001E5:00010000\r\n#00103:00004B00\r\n\
#00112:01010000\r\n#00122:01010000\r\n#00114:0001000000010000\r\n#00124:0001000000010000\r\n#00116:0101\r\n#00126:0101\r\n\
#00211:0101\r\n#00212:0001\r\n#00214:0101\r\n#00215:0001\r\n#00216:0100\r\n#00221:0101\r\n#00222:0001\r\n#00224:0101\r\n#00225:0001\r\n#00226:0100\r\n";

/// What a seven-key field adds to that chart: its sixth and seventh keys, on either side. A
/// five-key field reads none of it.
const PLAY_CHART_SEVEN: &[u8] = b"#00118:0000000100000000\r\n#00128:0000000100000000\r\n#00119:01000100\r\n#00129:01000100\r\n\
#00218:0101\r\n#00219:0001\r\n#00228:0101\r\n#00229:0001\r\n";

/// The lane of the play scenarios whose long note the player holds while the play head is inside
/// it. The other long note of the chart is never held, so a frame shows both bodies.
const PLAY_HELD_LANE: usize = 0;

/// The entry files of the play screens that are not the seven-key one, each of which is drawn
/// against the chart read as its own mode.
const ENTRY_PLAY5: &str = "play5_hw.luaskin";
const ENTRY_PLAY10: &str = "play10_hw.luaskin";
const ENTRY_PLAY14: &str = "play14_hw.luaskin";

/// The scene time the other play screens are drawn at: a second and a half into the chart, with
/// the first long note under the play head.
const PLAY_RUNNING_MS: i64 = 6_000;

/// The reference's number for the gauge the result scenario's run was played on: normal.
const RESULT_GAUGE_TYPE: usize = 2;

/// What that gauge stood at when the run ended, which is also what the scenario's numbers say.
const RESULT_GAUGE_END: f32 = 86.4;

/// What a gauge that clears part way up starts a run at.
const GROOVE_GAUGE_START: f32 = 20.0;

/// The most any gauge of the scenario holds.
const GAUGE_FULL: f32 = 100.0;

/// The limits of the nine gauges of a seven-key chart, by the reference's gauge type: the least,
/// the most and the clear line of each (`GaugeProperty.SEVENKEYS`).
const SEVEN_KEY_GAUGES: [GaugeScale; GAUGE_TYPES] = [
    GaugeScale::new(2.0, 100.0, 60.0),
    GaugeScale::new(2.0, 100.0, 80.0),
    GaugeScale::new(2.0, 100.0, 80.0),
    GaugeScale::new(0.0, 100.0, 0.0),
    GaugeScale::new(0.0, 100.0, 0.0),
    GaugeScale::new(0.0, 100.0, 0.0),
    GaugeScale::new(0.0, 100.0, 0.0),
    GaugeScale::new(0.0, 100.0, 0.0),
    GaugeScale::new(0.0, 100.0, 0.0),
];

/// How fast each gauge of the result scenario gains while the run goes well, per sample, by gauge
/// type. The three that clear part way up gain quickly; the six that clear at nothing barely do.
const GAUGE_GAIN: [f32; GAUGE_TYPES] = [0.6, 0.55, 0.474, 0.06, 0.04, 0.04, 0.05, 0.04, 0.03];

/// How much each gauge loses when the run slips, by gauge type.
const GAUGE_LOSS: [f32; GAUGE_TYPES] = [4.0, 6.0, 9.0, 16.0, 26.0, 100.0, 5.0, 9.0, 15.0];

/// How many stages the course scenario reads the result scenario's run as, each as long as the
/// others.
const COURSE_STAGES: usize = 4;

/// Samples between two slips of the result scenario's run.
const GAUGE_SLIP_EVERY: usize = 37;

/// How many milliseconds to either side of a note's moment the result scenario's timing spread
/// counts, which is the reference's range.
const TIMING_RANGE_MS: i32 = 150;

/// The offset the result scenario's hits are centred on, in milliseconds early.
const TIMING_CENTRE_MS: f64 = 4.0;

/// How widely they are spread around it, in milliseconds.
const TIMING_SPREAD_MS: f64 = 14.0;

/// How many hits the fullest millisecond of the result scenario holds.
const TIMING_PEAK: f64 = 46.0;

/// The judgement windows of the result scenario's run in milliseconds, best first, each as how late
/// and how early a hit may be: a seven-key chart at the reference's normal judge rank.
const RESULT_JUDGE_AREA: [[i32; 2]; TIMING_JUDGE_AREAS] = [[-20, 20], [-60, 60], [-150, 150], [-280, 220], [-150, 500]];

/// The judgement counts a result scenario's graphs draw, best first.
const RESULT_JUDGE_DIST: [u32; 6] = [1320, 250, 40, 6, 8, 0];

/// Milliseconds in a second, which is what turns the scenario chart's length into gauge samples.
const MS_PER_SECOND: usize = 1_000;

/// How long the scenario chart lasts, in seconds, which is how many columns its distribution has.
const CHART_SECONDS: usize = 150;

/// The seconds of the scenario chart in which its notes come three times as thick.
const CHART_BURST: std::ops::Range<usize> = 60..75;

/// The seconds of the scenario chart in which two long notes are held at a time.
const CHART_HOLDS: std::ops::Range<usize> = 25..40;

/// Notes a quiet second of the scenario chart holds, before the wobble the second adds.
const CHART_BASE_NOTES: u32 = 4;

/// How much the second adds to that, at most, in notes.
const CHART_WOBBLE_NOTES: u32 = 11;

/// Notes a burst adds on top.
const CHART_BURST_NOTES: u32 = 22;

/// Seconds between two mines.
const CHART_MINE_EVERY: usize = 17;

/// Seconds between two seconds in which notes were played worse.
const CHART_SLIP_EVERY: usize = 9;

/// Where the scenario chart's tempo changes, as `(speed, time in milliseconds)`: half speed for
/// twenty seconds, a stop of one second and a faster finish, ending where the chart does.
const CHART_TEMPO: [(f64, f64); 6] = [(180.0, 0.0), (90.0, 40_000.0), (180.0, 60_000.0), (0.0, 100_000.0), (240.0, 101_000.0), (240.0, 150_000.0)];

/// The tempo the scenario chart is mostly played at.
const CHART_MAIN_BPM: f64 = 180.0;

/// The slowest tempo of the scenario chart.
const CHART_MIN_BPM: f64 = 90.0;

/// The fastest tempo of the scenario chart.
const CHART_MAX_BPM: f64 = 240.0;

/// How long the scenario chart's song is, in milliseconds.
const CHART_LENGTH_MS: i32 = 150_000;

/// What the scenario chart is made of, second by second, for the graphs that plot a chart.
#[derive(Debug)]
struct Analysis {
    kinds: Vec<[u32; NOTE_KINDS]>,
    judgements: Vec<[u32; JUDGEMENTS]>,
    early_late: Vec<[u32; EARLY_LATE_BUCKETS]>,
}

/// A chart of [`CHART_SECONDS`] seconds that gets busier in a burst and holds two long notes for a
/// while, and a run of it that hit most notes best and slipped now and then.
fn analysis() -> Analysis {
    let kinds: Vec<[u32; NOTE_KINDS]> = (0..CHART_SECONDS)
        .map(|second| {
            let burst = if CHART_BURST.contains(&second) { CHART_BURST_NOTES } else { 0 };
            let keys = CHART_BASE_NOTES + (second as u32 * 3) % CHART_WOBBLE_NOTES + burst;
            let scratch = keys / 8;
            let held = if CHART_HOLDS.contains(&second) { 2 } else { 0 };
            [0, 0, scratch, 0, held, keys - scratch, u32::from(second % CHART_MINE_EVERY == 0)]
        })
        .collect();
    let judgements: Vec<[u32; JUDGEMENTS]> = kinds
        .iter()
        .enumerate()
        .map(|(second, kinds)| {
            let notes = kinds[2] + kinds[5];
            let best = notes * 7 / 10;
            let great = notes * 2 / 10;
            let good = if second % CHART_SLIP_EVERY == 0 { notes - best - great } else { 0 };
            let rest = notes - best - great - good;
            [0, best, great, good, rest / 2, rest - rest / 2]
        })
        .collect();
    let early_late = judgements
        .iter()
        .map(|[_, best, great, good, bad, poor]| {
            [0, *best, great / 2, good / 2, bad / 2, poor / 2, great - great / 2, good - good / 2, bad - bad / 2, poor - poor / 2]
        })
        .collect();
    Analysis { kinds, judgements, early_late }
}

/// How every gauge the result scenario's run could have been played on moved, by gauge type, a sample
/// every half second of the chart: each starts where its kind starts, gains while the run goes well
/// and loses when it slips, and one that empties stays empty. The gauge the run was played on ends
/// on the value the scenario's numbers report.
fn gauge_histories() -> Vec<Vec<f32>> {
    let samples = CHART_SECONDS * MS_PER_SECOND / GAUGE_SAMPLE_MS as usize;
    (0..GAUGE_TYPES)
        .map(|gauge_type| {
            let scale = SEVEN_KEY_GAUGES[gauge_type];
            let survival = scale.border == 0.0;
            let mut value = if survival { GAUGE_FULL } else { GROOVE_GAUGE_START };
            let mut history: Vec<f32> = (1..=samples)
                .map(|sample| {
                    let emptied = survival && value <= 0.0;
                    let change = if sample % GAUGE_SLIP_EVERY == 0 { -GAUGE_LOSS[gauge_type] } else { GAUGE_GAIN[gauge_type] };
                    value = if emptied { 0.0 } else { (value + change).clamp(scale.min, scale.max) };
                    value
                })
                .collect();
            if gauge_type == RESULT_GAUGE_TYPE
                && let Some(last) = history.last_mut()
            {
                *last = RESULT_GAUGE_END;
            }
            history
        })
        .collect()
}

/// How the result scenario's hits were spread around their notes: a bell a few milliseconds early,
/// one count to a millisecond from [`TIMING_RANGE_MS`] late to as far early.
fn timing_distribution() -> Vec<u32> {
    (-TIMING_RANGE_MS..=TIMING_RANGE_MS)
        .map(|offset| {
            let distance = (f64::from(offset) - TIMING_CENTRE_MS) / TIMING_SPREAD_MS;
            (TIMING_PEAK * (-distance * distance / 2.0).exp()).round() as u32
        })
        .collect()
}

/// The bar of the browser scenario that is under the cursor: the chart the scenario file describes.
const SELECT_CURSOR: usize = 10;

/// The clear lamps the browser scenario's bars hold, by the reference's numbers.
const LAMP_FAILED: i32 = 1;
const LAMP_ASSIST: i32 = 2;
const LAMP_LIGHT_ASSIST: i32 = 3;
const LAMP_EASY: i32 = 4;
const LAMP_NORMAL: i32 = 5;
const LAMP_HARD: i32 = 6;
const LAMP_EX_HARD: i32 = 7;
const LAMP_FULL_COMBO: i32 = 8;
const LAMP_PERFECT: i32 = 9;
const LAMP_MAX: i32 = 10;

/// The difficulties of the browser scenario's charts, by the reference's numbers.
const DIFFICULTY_UNKNOWN: i32 = 0;
const DIFFICULTY_BEGINNER: i32 = 1;
const DIFFICULTY_NORMAL: i32 = 2;
const DIFFICULTY_HYPER: i32 = 3;
const DIFFICULTY_ANOTHER: i32 = 4;
const DIFFICULTY_INSANE: i32 = 5;

/// How the charts under the browser scenario's counted folders are spread over the clear lamps,
/// from no play to max.
const FOLDER_LAMPS: [u32; 11] = [6, 2, 1, 1, 4, 9, 7, 3, 2, 1, 1];

/// How long one slot's travel takes in the browser scenario's slide, and how much of it is left in
/// the frame that is drawn: half.
const SLIDE_TRAVEL_MS: i32 = 300;
const SLIDE_LEFT_MS: i64 = 150;

/// A wall clock for the browser scenario's slide to be measured on.
const SLIDE_NOW_MS: i64 = 1_700_000_000_000;

/// The folder, next to the scenario files, that holds the pictures a frame carries for the objects a
/// skin names by a negative id. They are drawn for this test and stand in for a chart's own.
const REFERENCE_IMAGE_DIR: &str = "images";

/// The stems of those pictures: a chart's stage file, its back bitmap and its banner.
const REFERENCE_IMAGE_STEMS: [&str; 3] = ["stagefile", "backbmp", "banner"];

/// What a frame carries beside its property reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Extra {
    /// Nothing: the screen draws scalar objects alone.
    None,
    /// The browser's bars, with the wheel at rest.
    Select,
    /// The browser's bars half way through sliding one slot towards the next bar.
    SelectSliding,
    /// The series a result screen's graphs draw.
    Result,
    /// The series a course's result screen draws: the gauge through every stage with the stage ends
    /// marked, and no timing spread, which the reference keeps to a single chart's result.
    CourseResult,
    /// A note field.
    Play,
    /// A note field with the chart's pictures behind it: the background the chart is showing at the
    /// moment, as the game would supply it.
    PlayBga,
}

/// An option that changes while the scene runs, which a scenario file cannot say: it describes one
/// moment.
#[derive(Debug, Clone, Copy)]
struct Switch {
    /// The scene time from which the option reads as `on`.
    at_ms: i64,
    option: i32,
    on: bool,
}

/// A click on an image of the skin, which a scenario file cannot say either: the skin keeps what
/// the click changed in its own interpreter.
#[derive(Debug, Clone, Copy)]
struct Click {
    /// The scene time from which the click has happened.
    at_ms: i64,
    /// The id of the image whose `act` is run.
    image: &'static str,
}

/// One screen of the pack and the moments it is drawn at.
#[derive(Debug, Clone, Copy)]
struct Shot {
    /// The stem of the scenario file.
    name: &'static str,
    /// The stem of every capture.
    capture: &'static str,
    /// The entry file, relative to the pack.
    entry: &'static str,
    /// The scene times that are drawn, in milliseconds and in order.
    times_ms: &'static [i64],
    extra: Extra,
    switches: &'static [Switch],
    clicks: &'static [Click],
}

/// The image a result screen of the pack this was written against switches its information panel
/// with.
const RESULT_MENU_BUTTON: &str = "mainMenu";

/// The scene time at which the second result shot presses that button.
const RESULT_MENU_PRESSED_MS: i64 = 3_000;

/// The argument a left click hands the event it runs.
const CLICK_ARGUMENT: i32 = 1;

/// The option a play screen reads while the chart is loading.
const OPTION_NOW_LOADING: i32 = 80;

/// The option a play screen reads once the chart has loaded.
const OPTION_LOADED: i32 = 81;

/// The scene time at which the play scenario's load ends, the `loadend` of the pack this was
/// written against.
const PLAY_LOADED_MS: i64 = 3_500;

/// The timer each judgement region switches on when it is judged, by region (`JudgeManager.JUDGE_TIMER`).
const JUDGE_TIMERS: [i32; JUDGE_REGIONS] = [46, 47, 247];

/// The combo timer each region switches on at the same moment (`JudgeManager.COMBO_TIMER`).
const COMBO_TIMERS: [i32; JUDGE_REGIONS] = [446, 447, 448];

/// How many judgements a region can report, best first.
const JUDGEMENT_KINDS: usize = 6;

/// One judgement a play scenario's run takes: which region took it, which judgement it was, the
/// combo the run stood at and the scene time it landed at.
#[derive(Debug, Clone, Copy, Deserialize)]
struct JudgeRow {
    region: usize,
    judgement: usize,
    combo: i32,
    at_ms: i64,
}

/// What a play scenario says beside its host: the judgements its run takes as the scene goes on,
/// which no property id carries and a [`MapHost`] therefore has no table for.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct PlayScript {
    judge_hits: Vec<JudgeRow>,
    /// The hits the visualisers plot, each `[error in milliseconds, judgement]` with an early hit
    /// positive, oldest first.
    recent_hits: Vec<(i64, u8)>,
}

/// The screens that are drawn.
///
/// The decide, result and browser screens are drawn as they open, when the reference starts to take
/// input, in the middle of their animation and at rest; the decide screen once more partway through
/// its fade to black, and the result screen twice more while its gauge fills and its graphs are
/// being revealed. The result screen is then drawn again with its information panel switched to the
/// second page, where the rest of its graphs are, and the course's result screen is drawn against
/// the same scenario with the run read as four stages. The browser is drawn once more at rest in
/// every other way, with its wheel half way through sliding one slot along. The play screen is
/// drawn while it loads (twice), in its ready phase and with the chart running. It is drawn twice
/// more against scenarios of its own: one whose run takes a perfect great, a good and a bad half a
/// second apart, drawn while each pop-up is on show and once after the last has played out, and one
/// with the lift, the hidden cover and the lane cover all on and a great on show. The pack these
/// were written against blinks every word but the best on an eighty millisecond cycle, so those
/// frames are taken on the half of the cycle the word is lit in.
const SHOTS: &[Shot] = &[
    Shot {
        name: "decide",
        capture: "decide",
        entry: "decide.luaskin",
        times_ms: &[0, 500, 1_500, 3_000, 3_750],
        extra: Extra::None,
        switches: &[],
        clicks: &[],
    },
    Shot {
        name: "result",
        capture: "result",
        entry: "result.luaskin",
        times_ms: &[0, 250, 500, 750, 1_500, 3_000],
        extra: Extra::Result,
        switches: &[],
        clicks: &[],
    },
    Shot {
        name: "result",
        capture: "result_menu2",
        entry: "result.luaskin",
        times_ms: &[3_000, 3_250, 5_000],
        extra: Extra::Result,
        switches: &[],
        clicks: &[Click { at_ms: RESULT_MENU_PRESSED_MS, image: RESULT_MENU_BUTTON }],
    },
    Shot { name: "result", capture: "course", entry: "course.luaskin", times_ms: &[0, 1_500, 3_000], extra: Extra::CourseResult, switches: &[], clicks: &[] },
    Shot {
        name: "musicselect",
        capture: "musicselect",
        entry: "musicselect.luaskin",
        times_ms: &[0, 500, 1_500, 3_000],
        extra: Extra::Select,
        switches: &[],
        clicks: &[],
    },
    Shot {
        name: "musicselect",
        capture: "musicselect_slide",
        entry: "musicselect.luaskin",
        times_ms: &[0, 3_000],
        extra: Extra::SelectSliding,
        switches: &[],
        clicks: &[],
    },
    Shot {
        name: "play7_hw",
        capture: "play7_hw",
        entry: "play7_hw.luaskin",
        times_ms: &[0, 2_000, 4_000, 6_000],
        extra: Extra::Play,
        switches: &[Switch { at_ms: PLAY_LOADED_MS, option: OPTION_NOW_LOADING, on: false }, Switch { at_ms: PLAY_LOADED_MS, option: OPTION_LOADED, on: true }],
        clicks: &[],
    },
    Shot {
        name: "play7_judge",
        capture: "play7_judge",
        entry: "play7_hw.luaskin",
        times_ms: &[5_100, 5_700, 6_320, 6_800],
        extra: Extra::Play,
        switches: &[],
        clicks: &[],
    },
    Shot {
        name: "play7_visualizers",
        capture: "play7_visualizers",
        entry: "play7_hw.luaskin",
        times_ms: &[PLAY_RUNNING_MS],
        extra: Extra::Play,
        switches: &[],
        clicks: &[],
    },
    Shot { name: "play7_cover", capture: "play7_cover", entry: "play7_hw.luaskin", times_ms: &[5_120], extra: Extra::Play, switches: &[], clicks: &[] },
    Shot {
        name: "play7_bga",
        capture: "play7_bga",
        entry: "play7_hw.luaskin",
        times_ms: &[4_000, 5_000, 5_500, 6_000],
        extra: Extra::PlayBga,
        switches: &[],
        clicks: &[],
    },
    Shot { name: "play5_hw", capture: "play5_hw", entry: ENTRY_PLAY5, times_ms: &[PLAY_RUNNING_MS], extra: Extra::Play, switches: &[], clicks: &[] },
    Shot { name: "play14_hw", capture: "play14_hw", entry: ENTRY_PLAY14, times_ms: &[PLAY_RUNNING_MS], extra: Extra::Play, switches: &[], clicks: &[] },
    Shot { name: "play10_hw", capture: "play10_hw", entry: ENTRY_PLAY10, times_ms: &[PLAY_RUNNING_MS], extra: Extra::Play, switches: &[], clicks: &[] },
];

/// The mode a shot's chart is read as and its skin is loaded for, which the entry file says.
fn mode_of(shot: &Shot) -> Mode {
    match shot.entry {
        ENTRY_PLAY5 => Mode::BEAT_5K,
        ENTRY_PLAY10 => Mode::BEAT_10K,
        ENTRY_PLAY14 => Mode::BEAT_14K,
        _ => Mode::BEAT_7K,
    }
}

/// The long note `lane` has under the play head, by the chart time of its head: the one a player
/// holding that lane down would be holding.
fn long_in_hand(timelines: &[TimeLine], lane: usize, microtime: i64) -> Option<i64> {
    let passed = timelines.iter().take_while(|timeline| timeline.time_us <= microtime);
    passed.fold(None, |head, timeline| match timeline.notes.get(lane).and_then(Option::as_ref).map(|note| &note.kind) {
        Some(NoteKind::LongStart { .. }) => Some(timeline.time_us),
        Some(NoteKind::LongEnd { .. }) => None,
        _ => head,
    })
}

/// Where the scenario files live.
fn scenario_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("skin").join("external")
}

/// The host one screen is loaded and drawn against.
///
/// The file is a [`MapHost`] as the dump tool reads one. On top of it every option the reference
/// declares is given an answer: off, unless the file says otherwise. A host made of maps answers
/// "no such option" for an id it does not list, and a condition on an option nobody implements never
/// holds, whichever sign it was written with; the game answers every one of them, and so does this.
fn scenario(name: &str) -> MapHost {
    let path = scenario_dir().join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    let (value, _) = parse_value(&path, &text).unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
    let mut host = MapHost::deserialize(value).unwrap_or_else(|error| panic!("{} should describe a host: {error}", path.display()));
    for (option, _) in PropertyKind::Boolean.tables().iter().flat_map(|table| table.iter()) {
        host.booleans.entry(*option).or_insert(false);
    }
    host
}

/// What a play scenario says beside its host, read from the same file. A scenario that says nothing
/// of the kind judges nothing.
fn play_script(name: &str) -> PlayScript {
    let path = scenario_dir().join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    let (value, _) = parse_value(&path, &text).unwrap_or_else(|error| panic!("{} should parse: {error}", path.display()));
    PlayScript::deserialize(value).unwrap_or_else(|error| panic!("{} should describe a run: {error}", path.display()))
}

/// What each judgement region last reported by `now_ms`: the latest judgement of the script that
/// has landed in it.
fn judge_frame(hits: &[JudgeRow], now_ms: i64) -> JudgeFrame {
    hits.iter().filter(|hit| hit.at_ms <= now_ms).fold(JudgeFrame::default(), |frame, hit| {
        frame.with_region(hit.region, JudgeHit { judgement: hit.judgement, combo: hit.combo, at_us: hit.at_ms * MICROS_PER_MILLI })
    })
}

/// Switches the judge and combo timer of every region that has been judged by `now_ms` on at the
/// moment its last judgement landed, in the host a script reads and in the store destinations read.
fn switch_judge_timers(host: &mut MapHost, timers: &mut TimerState, hits: &[JudgeRow], now_ms: i64) {
    let judged = judge_frame(hits, now_ms);
    for (region, hit) in judged.regions.iter().enumerate() {
        let Some(hit) = hit else {
            continue;
        };
        for id in [JUDGE_TIMERS[region], COMBO_TIMERS[region]] {
            host.timers.insert(id, hit.at_us);
            timers.set_on(TimerId(id), hit.at_us);
        }
    }
}

/// The timers a scenario has switched on by `now_us`.
///
/// A scenario gives each timer the moment its screen switches it on, so one that starts later in the
/// scene -- the reference opens a screen to input after `input` milliseconds and starts its fade
/// after `scene` -- is still off in an earlier frame.
fn timers_at(scheduled: &BTreeMap<i32, i64>, now_us: i64) -> BTreeMap<i32, i64> {
    scheduled.iter().filter(|(_, since_us)| **since_us <= now_us).map(|(id, since_us)| (*id, *since_us)).collect()
}

/// Moves a host to one moment of its scene: the clock, the timers that are on by then and the
/// options that have changed by then. Answers the same timers as the store destinations read.
fn settle(host: &mut MapHost, scheduled: &BTreeMap<i32, i64>, switches: &[Switch], now_ms: i64) -> TimerState {
    let now_us = now_ms * MICROS_PER_MILLI;
    host.now_us = now_us;
    host.timers = timers_at(scheduled, now_us);
    for switch in switches.iter().filter(|switch| switch.at_ms <= now_ms) {
        host.booleans.insert(switch.option, switch.on);
    }
    let mut timers = TimerState::new();
    for (id, since_us) in &host.timers {
        timers.set_on(TimerId(*id), *since_us);
    }
    timers
}

/// Every file and folder under `root` with its size and the time it was last written, so a pack can
/// be shown to be untouched.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (u64, Option<SystemTime>)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("the directory should be listable").flatten() {
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(root).map(Path::to_path_buf) else {
                continue;
            };
            if path.is_dir() {
                found.insert(relative, (0, None));
                pending.push(path);
            } else {
                let metadata = entry.metadata().ok();
                found.insert(relative, (metadata.as_ref().map_or(0, std::fs::Metadata::len), metadata.and_then(|metadata| metadata.modified().ok())));
            }
        }
    }
    found
}

/// Decodes a PNG of any colour type into RGBA8.
fn decode_png(bytes: &[u8]) -> Option<SkinImage> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer.chunks_exact(RGB_BYTES).flat_map(|pixel| [pixel[0], pixel[1], pixel[2], OPAQUE]).collect(),
        png::ColorType::GrayscaleAlpha => buffer.chunks_exact(2).flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]]).collect(),
        png::ColorType::Grayscale => buffer.iter().flat_map(|level| [*level, *level, *level, OPAQUE]).collect(),
        png::ColorType::Indexed => return None,
    };
    SkinImage::new(info.width, info.height, rgba)
}

/// One of the stand-in pictures a frame carries as a reference image, decoded.
fn reference_image(stem: &str) -> SkinImage {
    let path = scenario_dir().join(REFERENCE_IMAGE_DIR).join(format!("{stem}.{PNG_EXTENSION}"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    decode_png(&bytes).unwrap_or_else(|| panic!("{} should decode", path.display()))
}

/// Registers the stand-in pictures with `canvas`, as a frame carries them.
fn reference_images(canvas: &mut CpuCanvas) -> ReferenceImages {
    let [stagefile, backbmp, banner] = REFERENCE_IMAGE_STEMS.map(|stem| {
        let image = reference_image(stem);
        Some(canvas.register_texture(&format!("rbms.external.reference.{stem}"), &image.rgba, image.width, image.height))
    });
    ReferenceImages { stagefile, backbmp, banner }
}

/// Reads a pack's images and fonts from disk and keeps count of what that cost.
#[derive(Debug, Default)]
struct PackAssets {
    /// Images decoded.
    images: usize,
    /// Pixels those images hold between them.
    pixels: u64,
    /// Bytes those images take on disk.
    image_bytes: u64,
    /// The largest image decoded, by pixel count.
    largest: (u32, u32),
    /// Time spent reading and decoding images.
    decoding: Duration,
    /// Files asked for that are not an image this test decodes.
    refused: Vec<PathBuf>,
    /// Font files read.
    fonts: usize,
    /// Bytes those fonts take on disk.
    font_bytes: u64,
}

impl SkinAssets for PackAssets {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        let started = Instant::now();
        let is_png = path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(PNG_EXTENSION));
        let bytes = if is_png { std::fs::read(path).ok() } else { None };
        let image = bytes.as_deref().and_then(decode_png);
        self.decoding += started.elapsed();
        match &image {
            Some(image) => {
                self.images += 1;
                self.pixels += u64::from(image.width) * u64::from(image.height);
                self.image_bytes += bytes.map_or(0, |bytes| bytes.len() as u64);
                if u64::from(image.width) * u64::from(image.height) > u64::from(self.largest.0) * u64::from(self.largest.1) {
                    self.largest = (image.width, image.height);
                }
            }
            None => self.refused.push(path.to_path_buf()),
        }
        image
    }

    fn font(&mut self, path: &Path) -> Option<Vec<u8>> {
        let data = std::fs::read(path).ok()?;
        self.fonts += 1;
        self.font_bytes += data.len() as u64;
        Some(data)
    }
}

/// A warning with everything it quotes taken out, so warnings that differ only in the object they
/// name count as one kind.
fn shape_of(warning: &str) -> String {
    let mut shape = String::with_capacity(warning.len());
    let mut quoted = false;
    for character in warning.chars() {
        if character == '"' {
            if !quoted {
                shape.push_str("\"..\"");
            }
            quoted = !quoted;
        } else if !quoted {
            shape.push(character);
        }
    }
    shape.split(" from ").next().unwrap_or_default().to_owned()
}

/// Prints `lines` grouped by shape, with the first few of each shape in full.
fn print_grouped(label: &str, lines: &[String]) {
    let mut groups: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for line in lines {
        groups.entry(shape_of(line)).or_default().push(line);
    }
    println!("  {label}: {} in {} kinds", lines.len(), groups.len());
    for (shape, members) in &groups {
        println!("    {} x {shape}", members.len());
        let mut distinct: BTreeMap<&String, usize> = BTreeMap::new();
        for member in members {
            *distinct.entry(*member).or_default() += 1;
        }
        for (member, count) in distinct.iter().take(REPORT_LINES) {
            println!("      {count} x {member}");
        }
        if distinct.len() > REPORT_LINES {
            println!("      ... and {} more distinct lines", distinct.len() - REPORT_LINES);
        }
    }
}

/// Every kind of object a screen can hold, in the order a report lists them.
const OBJECT_KINDS: [SkinObjectKind; 20] = [
    SkinObjectKind::Image,
    SkinObjectKind::Number,
    SkinObjectKind::Float,
    SkinObjectKind::Text,
    SkinObjectKind::Slider,
    SkinObjectKind::Graph,
    SkinObjectKind::Background,
    SkinObjectKind::Note,
    SkinObjectKind::Gauge,
    SkinObjectKind::Judge,
    SkinObjectKind::SongList,
    SkinObjectKind::HiddenCover,
    SkinObjectKind::LiftCover,
    SkinObjectKind::GaugeGraph,
    SkinObjectKind::JudgeGraph,
    SkinObjectKind::BpmGraph,
    SkinObjectKind::TimingDistribution,
    SkinObjectKind::TimingVisualizer,
    SkinObjectKind::HitError,
    SkinObjectKind::Reference,
];

/// How many of a skin's top-level destinations are gated by a function, and how many are timed by
/// one: the places a frame with no interpreter bound would read a fallback.
fn function_counts(skin: &LoadedSkin) -> (usize, usize) {
    let gated =
        skin.destinations.iter().filter(|named| named.track.draw_conditions.iter().any(|condition| matches!(condition, DrawCondition::Function(_)))).count();
    let timed = skin.destinations.iter().filter(|named| matches!(named.track.timer, Some(TimerRef::Lua(_)))).count();
    (gated, timed)
}

/// The ids of the top-level destinations that carry no keyframe at all, each with how often it
/// occurs.
///
/// A repeating object -- a note field, a song wheel, a judgement pop-up -- is placed by the lists it
/// carries and is named by a destination with no `dst` of its own. The renderer gives such an
/// object the keyframe the reference constructs it with, so it is drawn all the same; any other
/// kind of object named this way has nowhere to be and is never drawn.
fn unplaced(skin: &LoadedSkin) -> BTreeMap<&str, usize> {
    let mut ids = BTreeMap::new();
    for named in skin.destinations.iter().filter(|named| named.track.frames.is_empty()) {
        *ids.entry(named.id.as_str()).or_default() += 1;
    }
    ids
}

/// How many top-level destinations have a keyframe of negative width or height, which the reference
/// draws mirrored.
fn mirrored(skin: &LoadedSkin) -> usize {
    skin.destinations.iter().filter(|named| named.track.frames.iter().any(|frame| frame.rect.w < 0.0 || frame.rect.h < 0.0)).count()
}

/// A stand-in for the image a play screen shows in its `bga` object: a diagonal ramp, so its
/// placement and its orientation are both visible.
fn backdrop() -> Vec<u8> {
    let mut rgba = Vec::with_capacity((BACKDROP_W * BACKDROP_H) as usize * RGBA_BYTES);
    for y in 0..BACKDROP_H {
        for x in 0..BACKDROP_W {
            let across = (x * u32::from(u8::MAX) / BACKDROP_W) as u8;
            let down = (y * u32::from(u8::MAX) / BACKDROP_H) as u8;
            rgba.extend_from_slice(&[across, down, OPAQUE - across, OPAQUE]);
        }
    }
    rgba
}

/// The pictures the background scenario's chart names, by picture number: a diagonal ramp, a layer
/// that is black everywhere but in a lit square and a bar, and a flat red for the miss layer.
fn chart_pictures() -> [Vec<u8>; 3] {
    let (width, height) = BGA_PICTURE;
    let ramp = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let across = (x * u32::from(u8::MAX) / width) as u8;
                let down = (y * u32::from(u8::MAX) / height) as u8;
                [across, down, OPAQUE - across, OPAQUE]
            })
        })
        .collect();
    let (square_x, square_y) = ((width - BGA_LAYER_SQUARE) / 2, (height - BGA_LAYER_SQUARE) / 2);
    let layer = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let in_square = (square_x..square_x + BGA_LAYER_SQUARE).contains(&x) && (square_y..square_y + BGA_LAYER_SQUARE).contains(&y);
                let in_bar = y < BGA_LAYER_BAR;
                match (in_square, in_bar) {
                    (true, _) => [OPAQUE, 0, OPAQUE, OPAQUE],
                    (false, true) => [OPAQUE, OPAQUE, 0, OPAQUE],
                    (false, false) => [0, 0, 0, OPAQUE],
                }
            })
        })
        .collect();
    let miss = (0..width * height).flat_map(|_| [BGA_MISS_RED, 0, 0, OPAQUE]).collect();
    [ramp, layer, miss]
}

/// The background the scenario's chart shows `play_ms` into it, which is negative while the chart
/// has not started: its picture and layer from the start, and a miss at [`BGA_MISS_AT_MS`].
fn chart_background(canvas: &mut CpuCanvas, play_ms: i64) -> BgaFrame {
    let mut head = BgaPlayhead::new(vec![BgaEvent { time_ms: 0, base: BGA_PICTURE_BASE, layer: BGA_PICTURE_LAYER, miss: Some(vec![BGA_PICTURE_MISS]) }]);
    head.prepare(-1);
    head.prepare(play_ms);
    head.start_miss(BGA_MISS_AT_MS, DEFAULT_MISS_LAYER_DURATION_MS);
    let pictures = chart_pictures();
    let (width, height) = BGA_PICTURE;
    BgaTextures::default().frame(canvas, head.pick(), BgaExpand::default(), |number| {
        let rgba = pictures.get(usize::try_from(number).ok()?)?;
        Some(BgaPicture { generation: u64::from(number.unsigned_abs()), width, height, rgba })
    })
}

/// A chart of the browser scenario that is on disk.
fn chart(title: &str, level: i32, difficulty: i32, lamp: i32, features: u32) -> SongBar {
    SongBar { level, difficulty, lamp, features, ..SongBar::new(BarKind::Song { exists: true }, title) }
}

/// A bar of the browser scenario that opens, with the charts under it counted.
fn counted(kind: BarKind, title: &str) -> SongBar {
    SongBar { distribution: Some(Box::new(BarDistribution { lamps: FOLDER_LAMPS, ..BarDistribution::default() })), ..SongBar::new(kind, title) }
}

/// The browser scenario's bars: one of every kind a wheel draws differently, and a chart on every
/// clear lamp, difficulty and label. The wheel of the pack this was written against shows the
/// seventeen around the cursor, the first and the last of them all but off the screen.
fn select_bars() -> Vec<SongBar> {
    let (mines, random) = (SongBar::FEATURE_MINE_NOTE, SongBar::FEATURE_RANDOM);
    let (open, long, charge, hell) =
        (SongBar::FEATURE_UNDEFINED_LN, SongBar::FEATURE_LONG_NOTE, SongBar::FEATURE_CHARGE_NOTE, SongBar::FEATURE_HELL_CHARGE_NOTE);
    vec![
        chart("Bar Above The Wheel", 1, DIFFICULTY_BEGINNER, 0, 0),
        chart("Another Bar Above The Wheel", 2, DIFFICULTY_NORMAL, 0, 0),
        counted(BarKind::Table, "Scenario Table Insane"),
        SongBar::new(BarKind::Search, "Search : 'scenario'"),
        SongBar::new(BarKind::Command, "Scenario Command"),
        SongBar { is_new: true, ..counted(BarKind::Folder, "Scenario Folder (new, counted)") },
        SongBar { trophy: Some(BarTrophy::Gold), lamp: LAMP_EASY, features: long, ..SongBar::new(BarKind::Course { complete: true }, "Scenario Course") },
        SongBar { lamp: LAMP_LIGHT_ASSIST, ..SongBar::new(BarKind::Course { complete: false }, "Scenario Course With A Chart Missing") },
        chart("Opening Track", 3, DIFFICULTY_BEGINNER, LAMP_FAILED, 0),
        chart("Second Track -a long title to see how a bar treats a line wider than its box-", 5, DIFFICULTY_NORMAL, LAMP_ASSIST, mines),
        chart("Scenario Title", 12, DIFFICULTY_ANOTHER, LAMP_NORMAL, long | mines | random),
        chart("Fifth Track", 12, DIFFICULTY_INSANE, LAMP_HARD, open),
        chart("Sixth Track", 9, DIFFICULTY_HYPER, LAMP_EX_HARD, charge | random),
        chart("Seventh Track", 10, DIFFICULTY_ANOTHER, LAMP_FULL_COMBO, hell),
        SongBar { is_new: true, ..chart("Eighth Track (new)", 11, DIFFICULTY_ANOTHER, LAMP_PERFECT, 0) },
        chart("Ninth Track", 1, DIFFICULTY_UNKNOWN, LAMP_MAX, 0),
        SongBar { level: 8, difficulty: DIFFICULTY_HYPER, ..SongBar::new(BarKind::Song { exists: false }, "Chart That Is Not On Disk") },
        counted(BarKind::Table, "Scenario Table Normal"),
        SongBar::new(BarKind::Executable, "[RANDOM] Scenario Random Select"),
        SongBar::new(BarKind::RandomCourse { complete: true }, "Scenario Random Course"),
        SongBar::new(BarKind::Folder, "Folder Below The Wheel"),
        chart("Bar Below The Wheel", 4, DIFFICULTY_HYPER, 0, 0),
    ]
}

/// Writes the canvas to `path` as an opaque PNG.
fn save_png(path: &Path, canvas: &CpuCanvas) {
    let file = std::fs::File::create(path).unwrap_or_else(|error| panic!("{} should be writable: {error}", path.display()));
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), CANVAS_W, CANVAS_H);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let rgb: Vec<u8> = canvas.pixels().chunks_exact(RGBA_BYTES).flat_map(|pixel| [pixel[0], pixel[1], pixel[2]]).collect();
    let mut writer = encoder.write_header().expect("the capture's header should be writable");
    writer.write_image_data(&rgb).expect("the capture's pixels should be writable");
    writer.finish().expect("the capture should be finished");
}

/// What one frame came to.
#[derive(Debug, Clone, Copy)]
struct Painted {
    /// Objects the prepare stage left to be drawn.
    visible: usize,
    /// Objects that put something on screen.
    drawn: usize,
    /// Answers the skin's Lua gave while the frame was prepared.
    answers: usize,
}

/// Everything one frame is drawn with, beyond the moment it is drawn at.
struct Stage<'a> {
    shot: &'a Shot,
    skin: &'a LoadedSkin,
    screen: &'a SkinScreen,
    backdrop: TextureId,
    images: ReferenceImages,
    bars: &'a [SongBar],
    chart: &'a rbms_model::Model,
    analysis: &'a Analysis,
    /// How every gauge of the result scenario's run moved, by gauge type.
    gauges: &'a [Vec<f32>],
    /// How the result scenario's hits were spread around their notes.
    timing: &'a [u32],
    /// The judgements a play scenario's run takes as its scene goes on.
    judged: &'a [JudgeRow],
    /// The hits a play scenario's two visualisers plot.
    recent: &'a [(i64, u8)],
}

impl Stage<'_> {
    /// Draws the screen at `now_ms` onto a cleared canvas and answers what the frame came to.
    ///
    /// With `bound` the host is bound to the skin's interpreter while the frame is prepared, and
    /// every function value is asked there. Without, the frame is prepared as a renderer with no
    /// interpreter prepares it: a function gate reads false and a function timer reads off. Either
    /// way the frame is drawn with nothing bound.
    fn draw(&self, canvas: &mut CpuCanvas, text: &mut TextContext, host: &MapHost, timers: &TimerState, now_ms: i64, bound: bool) -> Painted {
        let now_us = now_ms * MICROS_PER_MILLI;
        let sliding = BarScroll { duration_ms: SLIDE_NOW_MS + SLIDE_LEFT_MS, angle: SLIDE_TRAVEL_MS, now_ms: SLIDE_NOW_MS };
        let scroll = if self.shot.extra == Extra::SelectSliding { sliding } else { BarScroll::default() };
        let list = SongBars { scroll, ..SongBars::new(self.bars, SELECT_CURSOR) };
        let tempo = BpmTimeline::of_chart(&CHART_TEMPO, CHART_MAIN_BPM, CHART_MIN_BPM, CHART_MAX_BPM, Some(CHART_LENGTH_MS));
        let chart_only = NoteDistribution { kinds: &self.analysis.kinds, ..NoteDistribution::default() };
        let run = NoteDistribution {
            judgements: &self.analysis.judgements,
            early_late: &self.analysis.early_late,
            ..NoteDistribution { kinds: &self.analysis.kinds, ..NoteDistribution::of_judgements(&RESULT_JUDGE_DIST) }
        };
        let series = FrameSeries {
            gauge_history: Some(GaugeHistory::of_kinds(self.gauges)),
            timing: Some(TimingHistogram::new(self.timing).with_judge_area(RESULT_JUDGE_AREA)),
            bpm: Some(tempo),
            notes: Some(run),
            recent_hits: None,
        };
        let ended = GaugeFrame::finished(RESULT_GAUGE_TYPE, RESULT_GAUGE_END, SEVEN_KEY_GAUGES);
        let samples = self.gauges.first().map_or(0, Vec::len);
        let stage_ends: [usize; COURSE_STAGES] = std::array::from_fn(|stage| samples * (stage + 1) / COURSE_STAGES);
        let course = FrameSeries { gauge_history: Some(GaugeHistory::of_kinds(self.gauges).with_sections(&stage_ends)), timing: None, ..series };
        let chart_us = (now_ms - PLAY_STARTS_MS).max(0) * MICROS_PER_MILLI;
        let held = LaneLong { processing: long_in_hand(&self.chart.timelines, PLAY_HELD_LANE, chart_us), ..LaneLong::default() };
        let longs: Vec<LaneLong> = (0..=PLAY_HELD_LANE).map(|lane| if lane == PLAY_HELD_LANE { held } else { LaneLong::default() }).collect();
        let notes = LaneNotes {
            hispeed: PLAY_HISPEED,
            longs: &longs,
            show: NoteDisplay { bpm_guide: true, ..NoteDisplay::default() },
            ..LaneNotes::new(&self.chart.timelines, chart_us, self.chart.init_bpm)
        };
        let bga = match self.shot.extra {
            Extra::PlayBga => chart_background(canvas, now_ms - PLAY_STARTS_MS),
            _ => BgaFrame::of(Some(self.backdrop)),
        };
        let behind = FrameData { bga, images: self.images, ..FrameData::default() };
        let data = match self.shot.extra {
            Extra::None => FrameData { series: FrameSeries { bpm: Some(tempo), notes: Some(chart_only), ..FrameSeries::default() }, ..behind },
            Extra::Select | Extra::SelectSliding => {
                FrameData { bars: Some(&list), series: FrameSeries { bpm: Some(tempo), notes: Some(chart_only), ..FrameSeries::default() }, ..behind }
            }
            Extra::Result => FrameData { series, gauge: Some(ended), ..behind },
            Extra::CourseResult => FrameData { series: course, gauge: Some(ended), ..behind },
            Extra::Play | Extra::PlayBga => FrameData {
                notes: Some(&notes),
                gauge: Some(GaugeFrame::of_kind(0, PLAY_GAUGE_BORDER)),
                series: FrameSeries {
                    recent_hits: Some(RecentHits::new(self.recent).with_judge_area(RESULT_JUDGE_AREA)),
                    bpm: Some(tempo),
                    notes: Some(NoteDistribution { playing: Some(PlayCursor::default()), ..run }),
                    ..FrameSeries::default()
                },
                ..behind
            },
        };

        let data = FrameData { judge: judge_frame(self.judged, now_ms), ..data };
        let frame = SkinFrame { now_us, timers, state: host, lua: None, mouse: None, data };
        let prepared = match self.skin.runtime().filter(|_| bound) {
            Some(runtime) => {
                runtime.frame(host, |lua| self.screen.prepare(&SkinFrame { lua: Some(lua), ..frame })).expect("the host should bind to the skin's interpreter")
            }
            None => self.screen.prepare(&frame),
        };
        canvas.clear(Color::BLACK);
        let mut ctx = RenderCtx::new(rbms_render::theme(), text);
        let drawn = self.screen.draw_prepared(&mut ctx, canvas, &frame, &prepared);
        Painted { visible: prepared.visible_count(), drawn, answers: prepared.answer_count() }
    }
}

/// Runs what a left click on one of the skin's images runs: the function its `act` names, inside the
/// skin's own interpreter, with the host bound as it is for a frame.
fn press(skin: &LoadedSkin, host: &MapHost, click: &Click) {
    let image = skin.def.image.iter().find(|image| image.id == click.image).unwrap_or_else(|| panic!("the skin should declare an image {:?}", click.image));
    let Some(EventRef::Lua(function)) = image.act else {
        panic!("the image {:?} should run a function of the skin when it is clicked", click.image);
    };
    let runtime = skin.runtime().expect("a skin whose image runs a function has an interpreter");
    runtime.frame(host, |lua| lua.call_event(function, CLICK_ARGUMENT)).expect("the host should bind to the skin's interpreter");
}

/// Loads, builds and draws one screen of the pack, printing what it came to.
fn capture(pack: &Path, overlay: &Path, capture_dir: Option<&Path>, shot: &Shot) {
    let entry = pack.join(shot.entry);
    let mut host = scenario(shot.name);
    let scheduled = host.timers.clone();
    settle(&mut host, &scheduled, shot.switches, 0);

    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), write_overlay: Some(overlay), ..SkinLoadOptions::new(pack, &user, mode_of(shot)) };
    let loading = Instant::now();
    let skin = load_skin_with_host(&entry, options, &host).unwrap_or_else(|error| panic!("{} should load: {error}", shot.entry));
    let loaded_in = loading.elapsed();

    let declared_sources = skin.sources.len();
    let referenced = referenced_sources(&skin);
    let drawn_sources = skin.sources.keys().filter(|id| referenced.contains(*id)).count();
    let (gated, timed) = function_counts(&skin);
    println!(
        "{}: type {} {:?} {}x{} | input {} scene {} fadeout {} | loaded in {} ms | destinations {} (function gate {gated}, function timer {timed}) | functions {} | sources {} of {declared_sources} referenced | fonts {} | load warnings {}",
        shot.name,
        skin.def.skin_type,
        skin.def.name,
        skin.resolution.0,
        skin.resolution.1,
        skin.def.input,
        skin.def.scene,
        skin.def.fadeout,
        loaded_in.as_millis(),
        skin.destinations.len(),
        skin.runtime().map_or(0, rbms_skin::lua::SkinLua::function_count),
        drawn_sources,
        skin.fonts.len(),
        skin.warnings.len(),
    );
    print_grouped("load warnings", &skin.warnings);
    println!(
        "  destinations with no keyframe (drawn only when they name a note field, a song wheel or a judgement pop-up): {:?} | destinations with a mirrored keyframe: {}",
        unplaced(&skin),
        mirrored(&skin)
    );

    let mut canvas = CpuCanvas::new(CANVAS_W, CANVAS_H);
    let mut text = TextContext::embedded_only();
    let mut assets = PackAssets::default();
    let building = Instant::now();
    let mut screen = SkinScreen::build(&mut canvas, &mut text, &skin, &mut assets);
    let built_in = building.elapsed();
    let kinds: Vec<String> = OBJECT_KINDS
        .iter()
        .map(|kind| (kind, screen.count_of(*kind)))
        .filter(|(_, count)| *count > 0)
        .map(|(kind, count)| format!("{kind:?} {count}"))
        .collect();
    println!(
        "  built in {} ms: {} objects of {} destinations ({}) | textures {} ({} RGBA bytes) | images read {} ({} px, {} bytes on disk, largest {}x{}, decoded in {} ms) | not decoded {} | fonts read {} ({} bytes), families {}",
        built_in.as_millis(),
        screen.object_count(),
        skin.destinations.len(),
        kinds.join(", "),
        screen.texture_stats().count,
        screen.texture_stats().bytes,
        assets.images,
        assets.pixels,
        assets.image_bytes,
        assets.largest.0,
        assets.largest.1,
        assets.decoding.as_millis(),
        assets.refused.len(),
        assets.fonts,
        assets.font_bytes,
        screen.families().len(),
    );
    for refused in &assets.refused {
        println!("    not decoded: {}", refused.strip_prefix(pack).unwrap_or(refused).display());
    }
    print_grouped("build warnings", screen.warnings());

    let bars = select_bars();
    let chart = to_model(&parse(&[PLAY_CHART_FIVE, PLAY_CHART_SEVEN].concat()), mode_of(shot));
    let backdrop = canvas.register_texture("rbms.external.backdrop", &backdrop(), BACKDROP_W, BACKDROP_H);
    let images = reference_images(&mut canvas);
    let analysis = analysis();
    let gauges = gauge_histories();
    let timing = timing_distribution();
    let script = play_script(shot.name);
    let stage = Stage {
        shot,
        skin: &skin,
        screen: &screen,
        backdrop,
        images,
        bars: &bars,
        chart: &chart,
        analysis: &analysis,
        gauges: &gauges,
        timing: &timing,
        judged: &script.judge_hits,
        recent: &script.recent_hits,
    };

    let mut last = (TimerState::new(), 0);
    let mut clicked = 0;
    for now_ms in shot.times_ms {
        let mut timers = settle(&mut host, &scheduled, shot.switches, *now_ms);
        switch_judge_timers(&mut host, &mut timers, &script.judge_hits, *now_ms);
        for click in shot.clicks.iter().skip(clicked).take_while(|click| click.at_ms <= *now_ms) {
            press(&skin, &host, click);
            clicked += 1;
        }
        let drawing = Instant::now();
        let painted = stage.draw(&mut canvas, &mut text, &host, &timers, *now_ms, true);
        let drawn_in = drawing.elapsed();
        let on: Vec<String> = host.timers.keys().map(i32::to_string).collect();
        let cost = skin.runtime().map(rbms_skin::lua::SkinLua::frame_cost).unwrap_or_default();
        println!(
            "  {}-{now_ms}: prepared {} of {} objects to draw with {} Lua answers ({} calls and {} reused timer reads, {} us inside Lua), drew {} in {} ms | timers on [{}]",
            shot.capture,
            painted.visible,
            screen.object_count(),
            painted.answers,
            cost.calls,
            cost.reused,
            cost.spent.as_micros(),
            painted.drawn,
            drawn_in.as_millis(),
            on.join(", ")
        );
        assert!(painted.drawn > 0, "{} drew nothing at {now_ms} ms", shot.capture);
        if let Some(directory) = capture_dir {
            let path = directory.join(format!("{}-{now_ms}.png", shot.capture));
            save_png(&path, &canvas);
            assert!(path.is_file(), "{} was not written", path.display());
        }
        last = (timers, *now_ms);
    }

    let (timers, now_ms) = last;
    let bound: Vec<u8> = canvas.pixels().to_vec();
    let unbound = stage.draw(&mut canvas, &mut text, &host, &timers, now_ms, false);
    let differing = bound.chunks_exact(RGBA_BYTES).zip(canvas.pixels().chunks_exact(RGBA_BYTES)).filter(|(with, without)| with != without).count();
    println!("  {}-{now_ms} again with no interpreter bound: drew {} objects, {differing} pixels differ from the bound frame", shot.capture, unbound.drawn);

    if let Some(runtime) = skin.runtime() {
        let diagnostics = runtime.diagnostics();
        println!(
            "  lua: function failures {} | errors caught by pcall {} | prints {} | frames over budget {}",
            diagnostics.function_failures.len(),
            diagnostics.swallowed.len(),
            diagnostics.prints.len(),
            diagnostics.frames_over_budget,
        );
        for failure in diagnostics.function_failures.iter().take(REPORT_LINES) {
            println!("    {:?} #{} failed {} times: {}", failure.kind, failure.function.0, failure.count, failure.first_message);
        }
        for caught in diagnostics.swallowed.iter().take(REPORT_LINES) {
            println!("    caught {} times: {}", caught.count, caught.message);
        }
        for printed in diagnostics.prints.iter().take(REPORT_LINES) {
            println!("    printed {} times: {}", printed.count, printed.text);
        }
    }
    let commands = host.take_calls();
    println!("  host commands issued by the skin: {}", commands.len());
    for command in commands.iter().take(REPORT_LINES) {
        println!("    {command:?}");
    }

    screen.release(&mut canvas);
}

/// Draws each screen of the pack `RBMS_SKIN_PACK` names at a few scene times, and writes the frames
/// to `RBMS_SKIN_CAPTURE_DIR` when that is set. Passes silently when no pack is named.
#[test]
fn an_external_skin_pack_draws_through_the_skin_renderer() {
    let Some(pack) = std::env::var_os(SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let capture_dir = std::env::var_os(CAPTURE_DIR_ENV).map(PathBuf::from);
    if let Some(directory) = &capture_dir {
        std::fs::create_dir_all(directory).unwrap_or_else(|error| panic!("{} should be creatable: {error}", directory.display()));
    }
    rbms_render::font::use_embedded_fonts_only();
    let overlay = std::env::temp_dir().join(format!("rbms-render-external-{}-overlay", std::process::id()));
    let _ = std::fs::remove_dir_all(&overlay);
    let before = snapshot(&pack);

    for shot in SHOTS {
        capture(&pack, &overlay, capture_dir.as_deref(), shot);
    }

    let written: Vec<PathBuf> = if overlay.is_dir() { snapshot(&overlay).into_keys().collect() } else { Vec::new() };
    println!("the skins wrote {} entries to their overlay: {written:?}", written.len());
    let _ = std::fs::remove_dir_all(&overlay);
    assert_eq!(snapshot(&pack), before, "drawing the pack changed its folder");
}

/// The scenario files are this repository's own, so they are checked whether or not a pack is
/// named: each one a screen is drawn against parses, describes exactly one difficulty and schedules
/// no timer before its scene begins, and the pictures drawn as reference images decode.
#[test]
fn every_scenario_describes_a_host_a_screen_can_be_drawn_against() {
    for shot in SHOTS {
        let host = scenario(shot.name);
        assert!(shot.clicks.is_sorted_by_key(|click| click.at_ms), "{} should be clicked in scene order", shot.capture);
        let difficulties = (DIFFICULTY_FIRST..=DIFFICULTY_LAST).filter(|option| host.booleans.get(option).copied().unwrap_or(false)).count();
        assert_eq!(difficulties, 1, "{} should turn exactly one difficulty on", shot.name);
        assert!(host.timers.values().all(|since_us| *since_us >= 0), "{} schedules a timer before its scene begins", shot.name);
        assert_eq!(host.screen, Some((CANVAS_W as i32, CANVAS_H as i32)), "{} should describe the window the frames are drawn at", shot.name);
        assert!(shot.times_ms.is_sorted(), "{} should be drawn in scene order", shot.name);

        let at_start = timers_at(&host.timers, 0);
        let at_end = timers_at(&host.timers, shot.times_ms.last().copied().unwrap_or_default() * MICROS_PER_MILLI);
        assert!(at_start.len() <= at_end.len(), "{} loses a timer as its scene runs", shot.name);
        assert_eq!(at_end, host.timers, "{} schedules a timer after its last frame", shot.name);
    }
    for stem in REFERENCE_IMAGE_STEMS {
        let image = reference_image(stem);
        assert!(image.width > 0 && image.height > 0, "the stand-in {stem} should be a picture");
    }
}

/// The judgements a play scenario scripts are this repository's own too: each lands in a region a
/// play screen has, is a judgement a region can report, and lands in scene order no earlier than the
/// scene begins and no later than its last frame, so the latest one a frame finds is the last one
/// written and none goes undrawn.
#[test]
fn every_scripted_judgement_is_one_a_play_screen_can_take() {
    for shot in SHOTS {
        let script = play_script(shot.name);
        assert!(script.judge_hits.is_sorted_by_key(|hit| hit.at_ms), "{} should be judged in scene order", shot.name);
        for hit in &script.judge_hits {
            assert!(hit.region < JUDGE_REGIONS, "{} judges region {}, which no play screen has", shot.name, hit.region);
            assert!(hit.judgement < JUDGEMENT_KINDS, "{} takes judgement {}, which no region reports", shot.name, hit.judgement);
            assert!(hit.at_ms >= 0, "{} is judged before its scene begins", shot.name);
        }
        let last = shot.times_ms.last().copied().unwrap_or_default();
        assert!(script.judge_hits.iter().all(|hit| hit.at_ms <= last), "{} is judged after its last frame", shot.name);
    }
}
