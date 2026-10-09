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
//! 3. The image sources no drawn object names are dropped before anything is decoded, because a
//!    pack ships sheets for every customisation it offers and decoding them all costs gigabytes.
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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use rbms_chart::to_model;
use rbms_model::Mode;
use rbms_parser::parse;
use rbms_render::playfield::LaneShade;
use rbms_render::skin_render::graphs::{EARLY_LATE_BUCKETS, JUDGEMENTS, NOTE_KINDS, PlayCursor};
use rbms_render::{
    BgaFrame, BpmTimeline, Color, CpuCanvas, FrameData, FrameSeries, GaugeFrame, GaugeHistory, NoteDistribution, NoteField, PlayfieldView, RecentHits,
    ReferenceImages, RenderCtx, Renderer, SelectRow, Skin, SkinAssets, SkinFrame, SkinImage, SkinObjectKind, SkinScreen, SongBars, TextContext, TextureId,
    TimingHistogram,
};
use rbms_skin::dst::{DrawCondition, TimerRef};
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin_with_host, parse_value};
use rbms_skin::model::Destination;
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

/// Scroll speed the play scenario's notes are drawn at.
const PLAY_HISPEED: f64 = 1.5;

/// A two-measure chart with every lane of a seven-key field filled, written for this test.
const PLAY_CHART: &[u8] = b"#PLAYER 1\r\n#BPM 150\r\n#WAV01 a.wav\r\n\
#00111:01000100\r\n#00112:00010001\r\n#00113:01000000\r\n#00114:00000100\r\n#00115:00010000\r\n#00116:01010000\r\n#00118:00000001\r\n#00119:01000000\r\n\
#00211:0101\r\n#00212:0001\r\n#00213:0100\r\n#00214:0101\r\n#00215:0001\r\n#00216:0100\r\n#00218:0101\r\n#00219:0001\r\n";

/// The gauge a result scenario's trend graph draws, in percent, one sample per step of the run.
const RESULT_GAUGE_SERIES: [f32; 16] = [20.0, 24.0, 30.0, 38.0, 46.0, 52.0, 50.0, 58.0, 66.0, 72.0, 70.0, 76.0, 80.0, 84.0, 82.0, 86.4];

/// The timing histogram a result scenario's distribution graph draws.
const RESULT_TIMING_HIST: [u32; 11] = [2, 6, 18, 60, 180, 420, 210, 70, 22, 8, 3];

/// The judgement counts a result scenario's graphs draw, best first.
const RESULT_JUDGE_DIST: [u32; 6] = [1320, 250, 40, 6, 8, 0];

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

/// The row of the browser scenario that is under the cursor.
const SELECT_CURSOR: usize = 4;

/// The titles of the browser scenario's rows, a folder first.
const SELECT_TITLES: [&str; 9] = [
    "Scenario Folder",
    "Opening Track",
    "Second Track -long title to see how a bar treats overflow-",
    "Third Track",
    "Scenario Title",
    "Fifth Track",
    "Sixth Track",
    "Seventh Track",
    "Eighth Track",
];

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
    /// The browser's rows.
    Select,
    /// The series a result screen's graphs draw.
    Result,
    /// A note field.
    Play,
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

/// One screen of the pack and the moments it is drawn at.
#[derive(Debug, Clone, Copy)]
struct Shot {
    /// The stem of the scenario file and of every capture.
    name: &'static str,
    /// The entry file, relative to the pack.
    entry: &'static str,
    /// The scene times that are drawn, in milliseconds and in order.
    times_ms: &'static [i64],
    extra: Extra,
    switches: &'static [Switch],
}

/// The option a play screen reads while the chart is loading.
const OPTION_NOW_LOADING: i32 = 80;

/// The option a play screen reads once the chart has loaded.
const OPTION_LOADED: i32 = 81;

/// The scene time at which the play scenario's load ends, the `loadend` of the pack this was
/// written against.
const PLAY_LOADED_MS: i64 = 3_500;

/// The screens that are drawn.
///
/// The decide, result and browser screens are drawn as they open, when the reference starts to take
/// input, in the middle of their animation and at rest; the decide screen once more partway through
/// its fade to black. The play screen is drawn while it loads (twice), in its ready phase and with
/// the chart running.
const SHOTS: &[Shot] = &[
    Shot { name: "decide", entry: "decide.luaskin", times_ms: &[0, 500, 1_500, 3_000, 3_750], extra: Extra::None, switches: &[] },
    Shot { name: "result", entry: "result.luaskin", times_ms: &[0, 500, 1_500, 3_000], extra: Extra::Result, switches: &[] },
    Shot { name: "musicselect", entry: "musicselect.luaskin", times_ms: &[0, 500, 1_500, 3_000], extra: Extra::Select, switches: &[] },
    Shot {
        name: "play7_hw",
        entry: "play7_hw.luaskin",
        times_ms: &[0, 2_000, 4_000, 6_000],
        extra: Extra::Play,
        switches: &[Switch { at_ms: PLAY_LOADED_MS, option: OPTION_NOW_LOADING, on: false }, Switch { at_ms: PLAY_LOADED_MS, option: OPTION_LOADED, on: true }],
    },
];

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

/// Adds the object id of each nested destination to `named`.
fn name_all(named: &mut BTreeSet<String>, destinations: &[Destination]) {
    named.extend(destinations.iter().map(|destination| destination.id.clone()));
}

/// The image sources the skin's drawn objects read, by source id.
///
/// An object is drawn when a top-level destination names it, or when a repeating object a
/// destination names -- a note set, a gauge, a judgement pop-up, a song wheel -- lists it. Every
/// definition with a `src` that is named either way keeps its source; everything else the skin
/// declares is a sheet for a customisation the player did not pick.
fn referenced_sources(skin: &LoadedSkin) -> BTreeSet<String> {
    let def = &skin.def;
    let mut named: BTreeSet<String> = skin.destinations.iter().map(|track| track.id.clone()).collect();
    if let Some(note) = def.note.as_ref().filter(|note| named.contains(&note.id)) {
        let lists = [
            &note.note,
            &note.lnstart,
            &note.lnend,
            &note.lnbody,
            &note.lnbody_active,
            &note.lnactive,
            &note.hcnstart,
            &note.hcnend,
            &note.hcnbody,
            &note.hcnactive,
            &note.hcnbody_active,
            &note.hcndamage,
            &note.hcnbody_miss,
            &note.hcnreactive,
            &note.hcnbody_reactive,
            &note.mine,
            &note.hidden,
            &note.processed,
        ];
        named.extend(lists.into_iter().flatten().cloned());
        for nested in [&note.group, &note.bpm, &note.stop, &note.time] {
            name_all(&mut named, nested);
        }
    }
    if let Some(gauge) = def.gauge.as_ref().filter(|gauge| named.contains(&gauge.id)) {
        named.extend(gauge.nodes.iter().cloned());
    }
    for judge in def.judge.iter().filter(|judge| named.contains(&judge.id)).collect::<Vec<_>>() {
        name_all(&mut named, &judge.images);
        name_all(&mut named, &judge.numbers);
    }
    if let Some(list) = def.songlist.as_ref().filter(|list| named.contains(&list.id)) {
        for nested in [&list.listoff, &list.liston, &list.text, &list.level, &list.lamp, &list.playerlamp, &list.rivallamp, &list.trophy, &list.label] {
            name_all(&mut named, nested);
        }
        named.extend(list.graph.iter().map(|graph| graph.id.clone()));
    }
    let from_sets: Vec<String> = def.imageset.iter().filter(|set| named.contains(&set.id)).flat_map(|set| set.images.iter().cloned()).collect();
    named.extend(from_sets);

    let mut sources = BTreeSet::new();
    let mut keep = |id: &str, src: &str| {
        if named.contains(id) {
            sources.insert(src.to_owned());
        }
    };
    def.image.iter().for_each(|image| keep(&image.id, &image.src));
    def.value.iter().for_each(|value| keep(&value.id, &value.src));
    def.floatvalue.iter().for_each(|value| keep(&value.id, &value.src));
    def.slider.iter().for_each(|slider| keep(&slider.id, &slider.src));
    def.graph.iter().for_each(|graph| keep(&graph.id, &graph.src));
    def.hidden_cover.iter().for_each(|cover| keep(&cover.id, &cover.src));
    def.lift_cover.iter().for_each(|cover| keep(&cover.id, &cover.src));
    def.pmchara.iter().for_each(|chara| keep(&chara.id, &chara.src));
    sources
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

/// The browser scenario's rows.
fn select_rows() -> Vec<SelectRow> {
    SELECT_TITLES
        .iter()
        .enumerate()
        .map(|(index, title)| SelectRow {
            folder: index == 0,
            title: (*title).to_owned(),
            mode_short: "7K",
            mode_color: Color::rgb(120, 200, 255),
            level: format!("{}", index + 3),
            difficulty_color: Color::rgb(255, 192, 0),
            lamp: Color::rgb(80, 200, 120),
            folder_count: (index == 0).then_some(SELECT_TITLES.len() - 1),
            dj_level: (index == SELECT_CURSOR).then_some("A"),
            favorite: false,
        })
        .collect()
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
    rows: &'a [SelectRow],
    field: &'a Skin,
    chart: &'a rbms_model::Model,
    analysis: &'a Analysis,
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
        let list = SongBars { rows: self.rows, sel: SELECT_CURSOR, options_open: false };
        let tempo = BpmTimeline::of_chart(&CHART_TEMPO, CHART_MAIN_BPM, CHART_MIN_BPM, CHART_MAX_BPM, Some(CHART_LENGTH_MS));
        let chart_only = NoteDistribution { kinds: &self.analysis.kinds, ..NoteDistribution::default() };
        let run = NoteDistribution {
            judgements: &self.analysis.judgements,
            early_late: &self.analysis.early_late,
            ..NoteDistribution { kinds: &self.analysis.kinds, ..NoteDistribution::of_judgements(&RESULT_JUDGE_DIST) }
        };
        let series = FrameSeries {
            gauge_history: Some(GaugeHistory::new(&RESULT_GAUGE_SERIES)),
            timing: Some(TimingHistogram::new(&RESULT_TIMING_HIST)),
            bpm: Some(tempo),
            notes: Some(run),
            recent_hits: None,
        };
        let playfield = PlayfieldView {
            timelines: &self.chart.timelines,
            microtime: (now_ms - PLAY_STARTS_MS).max(0) * MICROS_PER_MILLI,
            hispeed: PLAY_HISPEED,
            beam_on: &[],
            beam_off: &[],
            constant: false,
            legacy_note: false,
        };
        let play = NoteField { field: self.field, playfield: &playfield, shade: LaneShade::default(), bomb: &[], keys_down: &[] };
        let behind = FrameData { bga: BgaFrame::of(Some(self.backdrop)), images: self.images, ..FrameData::default() };
        let data = match self.shot.extra {
            Extra::None => FrameData { series: FrameSeries { bpm: Some(tempo), notes: Some(chart_only), ..FrameSeries::default() }, ..behind },
            Extra::Select => {
                FrameData { bars: Some(&list), series: FrameSeries { bpm: Some(tempo), notes: Some(chart_only), ..FrameSeries::default() }, ..behind }
            }
            Extra::Result => FrameData { series, ..behind },
            Extra::Play => FrameData {
                field: Some(&play),
                gauge: Some(GaugeFrame { kind: 0, clear_threshold: self.field.gauge_clear_threshold }),
                series: FrameSeries {
                    recent_hits: Some(RecentHits::new(&[])),
                    bpm: Some(tempo),
                    notes: Some(NoteDistribution { playing: Some(PlayCursor::default()), ..run }),
                    ..FrameSeries::default()
                },
                ..behind
            },
        };

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

/// Loads, builds and draws one screen of the pack, printing what it came to.
fn capture(pack: &Path, overlay: &Path, capture_dir: Option<&Path>, shot: &Shot) {
    let entry = pack.join(shot.entry);
    let mut host = scenario(shot.name);
    let scheduled = host.timers.clone();
    settle(&mut host, &scheduled, shot.switches, 0);

    let user = SkinUserConfig::default();
    let options = SkinLoadOptions { rng_seed: Some(TEST_SEED), write_overlay: Some(overlay), ..SkinLoadOptions::new(pack, &user, Mode::BEAT_7K) };
    let loading = Instant::now();
    let mut skin = load_skin_with_host(&entry, options, &host).unwrap_or_else(|error| panic!("{} should load: {error}", shot.entry));
    let loaded_in = loading.elapsed();

    let declared_sources = skin.sources.len();
    let referenced = referenced_sources(&skin);
    skin.sources.retain(|id, _| referenced.contains(id));
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
        skin.sources.len(),
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
        "  built in {} ms: {} objects of {} destinations ({}) | images read {} ({} px, {} bytes on disk, largest {}x{}, decoded in {} ms) | not decoded {} | fonts read {} ({} bytes), families {}",
        built_in.as_millis(),
        screen.object_count(),
        skin.destinations.len(),
        kinds.join(", "),
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

    let rows = select_rows();
    let field = Skin::default_for(Mode::BEAT_7K, CANVAS_W as f32, CANVAS_H as f32);
    let chart = to_model(&parse(PLAY_CHART), Mode::BEAT_7K);
    let backdrop = canvas.register_texture("rbms.external.backdrop", &backdrop(), BACKDROP_W, BACKDROP_H);
    let images = reference_images(&mut canvas);
    let analysis = analysis();
    let stage = Stage { shot, skin: &skin, screen: &screen, backdrop, images, rows: &rows, field: &field, chart: &chart, analysis: &analysis };

    let mut last = (TimerState::new(), 0);
    for now_ms in shot.times_ms {
        let timers = settle(&mut host, &scheduled, shot.switches, *now_ms);
        let drawing = Instant::now();
        let painted = stage.draw(&mut canvas, &mut text, &host, &timers, *now_ms, true);
        let drawn_in = drawing.elapsed();
        let on: Vec<String> = host.timers.keys().map(i32::to_string).collect();
        let cost = skin.runtime().map(rbms_skin::lua::SkinLua::frame_cost).unwrap_or_default();
        println!(
            "  {}-{now_ms}: prepared {} of {} objects to draw with {} Lua answers ({} calls and {} reused timer reads, {} us inside Lua), drew {} in {} ms | timers on [{}]",
            shot.name,
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
        assert!(painted.drawn > 0, "{} drew nothing at {now_ms} ms", shot.name);
        if let Some(directory) = capture_dir {
            let path = directory.join(format!("{}-{now_ms}.png", shot.name));
            save_png(&path, &canvas);
            assert!(path.is_file(), "{} was not written", path.display());
        }
        last = (timers, *now_ms);
    }

    let (timers, now_ms) = last;
    let bound: Vec<u8> = canvas.pixels().to_vec();
    let unbound = stage.draw(&mut canvas, &mut text, &host, &timers, now_ms, false);
    let differing = bound.chunks_exact(RGBA_BYTES).zip(canvas.pixels().chunks_exact(RGBA_BYTES)).filter(|(with, without)| with != without).count();
    println!("  {}-{now_ms} again with no interpreter bound: drew {} objects, {differing} pixels differ from the bound frame", shot.name, unbound.drawn);

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
