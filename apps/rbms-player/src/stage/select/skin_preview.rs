//! The list the skin preview shows on the song browser: sample bars, turned into what a skin's wheel
//! draws the way the browser turns its own.
//!
//! It lives under the browser because it stands in for the browser's own list: the bars are made
//! in the shape the browser hands a wheel, and the timers are the ones a browser's skin is animated
//! on. The bars are invented. Nothing is read from the library, the score book or the difficulty
//! tables.
//!
//! # What is on the list
//!
//! Two folders, a difficulty table, a dozen charts and two courses ([`SAMPLE_SONGS`]). The charts
//! are spread over every difficulty slot, a range of levels, every mode a play screen draws and
//! every clear lamp from none to the best, so a skin's bar has each of its looks on show somewhere.
//! A chart with a lamp has a best score to match it, which is what the browser's score panel reads.
//!
//! # What moves
//!
//! The cursor goes down a bar every [`CURSOR_STEP_US`] and round to the top after the last. Each
//! step is one notch of the wheel, so the bars slide as they do when the wheel of the mouse is
//! turned, and the bar timer starts over as it does whenever the bar under the cursor changes. The
//! slide is measured on the preview's own time in place of the wall clock the browser reads.

use rbms_chart::{RADAR_AXIS_COUNT, RADAR_DISPLAY_MAX, RadarAxis};
use rbms_config::Config;
use rbms_model::Mode;
use rbms_render::skin_render::frame::{BarDistribution, BarHold, BarKind, BarScroller, BarTrophy, LAMP_KINDS, SongBar};
use rbms_render::{BgaFrame, FrameData, ReferenceImages, SongBars};
use rbms_skin::loader::SKIN_TYPE_MUSIC_SELECT;
use rbms_skin::timer::{MICROS_PER_MILLI, timer_id};
use rbms_store::scoredb::{BestScore, JUDGE_TIER_COUNT, JudgeCounts};

use crate::AppShared;
use crate::skin_host::chart::{BpmRange, ChartContents, ChartMeta, ChartState, Density, LaneNotes, NoteCounts, is_lane_counted};
use crate::skin_host::ir::IrBrowser;
use crate::skin_host::select::{ChartInputs, CourseShown, SelectInputs, SelectShown};
use crate::skin_preview::SampleStep;
use crate::skin_screen::preview::{PreviewDrawn, PreviewFrame, PreviewScene, PreviewSelect};
use crate::stage::Canvas;
use crate::stage::scene_life::{SceneTimes, advance};

/// How long the cursor rests on a bar before it moves to the next, in microseconds.
const CURSOR_STEP_US: i64 = 1_500_000;

/// One notch of the wheel towards the next bar, and none.
const ONE_NOTCH: i32 = 1;
const NO_NOTCH: i32 = 0;

/// What the preview's time is counted from when it stands in for the wall clock a wheel slides on,
/// in milliseconds: a slide that ends at nought is no slide, so the clock never reads it.
const SLIDE_CLOCK_BASE_MS: i64 = 1;

/// The length of a scene that never ends on its own, which is what a browser's is.
const NO_SCENE_END_MS: i64 = i64::MAX;

/// The titles of the bars that are not charts, and of the folder the list is said to be in.
const FOLDER_TITLES: [&str; 2] = ["SAMPLE FOLDER A", "SAMPLE FOLDER B"];
const TABLE_TITLE: &str = "SAMPLE TABLE";
const COURSE_TITLES: [&str; 2] = ["SAMPLE COURSE 01", "SAMPLE COURSE 02"];
const OPEN_FOLDER_TITLE: &str = "SAMPLE LIBRARY";

/// The stages the first sample course lists.
const COURSE_STAGE_TITLES: [&str; 4] = ["SAMPLE STAGE 1", "SAMPLE STAGE 2", "SAMPLE STAGE 3", "SAMPLE STAGE 4"];

/// How many charts of the first sample folder sit on each clear lamp, from none played to the best.
const FOLDER_LAMPS: [u32; LAMP_KINDS] = [2, 1, 0, 1, 3, 5, 4, 2, 1, 1, 0];

/// How the charts of that folder are spread over the score ranks: this many more on each rank up to
/// the wave's length, then over again.
const FOLDER_RANK_WAVE: usize = 4;

/// The lamps from which a run broke no combo, and the one a chart never played has.
const LAMP_FULL_COMBO: u8 = 8;
const LAMP_NO_PLAY: u8 = 0;

/// The judgements a sample best score keeps counts of, in the order a score keeps them.
const TIER_PGREAT: usize = 0;
const TIER_GREAT: usize = 1;
const TIER_GOOD: usize = 2;
const TIER_POOR: usize = 4;

/// The moment every sample best score was played at, in seconds: a day of 2025.
const SAMPLE_PLAYED_AT: i64 = 1_750_000_000;

/// The judge rank and the total every sample chart states.
const SAMPLE_JUDGE_RANK: i32 = 2;
const SAMPLE_TOTAL: f64 = 300.0;

/// How a sample chart's notes are shared out by kind: one in this many is on the turntable of a mode
/// that has one, and one in this many is long on a chart with long notes.
const SCRATCH_NOTE_EVERY: i32 = 12;
const LONG_NOTE_EVERY: i32 = 10;

/// How much busier than its average a sample chart's busiest second is, and the busiest stretch of
/// its end.
const PEAK_DENSITY_SHARE: f64 = 2.0;
const END_DENSITY_SHARE: f64 = 1.5;

const MILLIS_PER_SECOND: i32 = 1_000;

/// How a sample chart's notes on the keys are shared out among them, key by key from the first, and
/// round again for a mode with more keys than this.
const KEY_LANE_WEIGHTS: [i32; 7] = [4, 3, 5, 3, 4, 3, 5];

/// How far each NOTES RADAR axis of a sample chart reaches, NOTES to CHORD, as a share of the axis
/// at [`SAMPLE_RADAR_FULL_LEVEL`]. A chart without long notes has no CHARGE.
const SAMPLE_RADAR_SHARES: [f32; RADAR_AXIS_COUNT] = [0.8, 0.7, 0.45, 0.2, 0.55, 0.6];

/// The level at which a sample chart's radar reaches [`SAMPLE_RADAR_SHARES`]; one of a lower level
/// reaches as far below it as its level is.
const SAMPLE_RADAR_FULL_LEVEL: f32 = 12.0;

/// How many judgements a sample best score is written down with: a perfect great, a great, a good
/// and a poor.
const SAMPLE_JUDGEMENTS: usize = 4;

/// How many ways a judgement count is split: early and late.
const TIMING_HALVES: u32 = 2;

/// The best score a sample chart is given: how many notes took a perfect great, a great, a good and
/// a poor, the longest combo, and how often the chart was played and cleared.
#[derive(Clone, Copy)]
struct SampleBest {
    judged: [u32; SAMPLE_JUDGEMENTS],
    max_combo: u32,
    plays: u32,
    clears: u32,
}

/// One invented chart of the sample list.
#[derive(Clone, Copy)]
struct SampleSong {
    title: &'static str,
    artist: &'static str,
    genre: &'static str,
    mode: Mode,
    level: i32,
    /// The difficulty slot, 1 to 5 from beginner to insane.
    difficulty: i32,
    /// The clear lamp, as the reference numbers lamps.
    lamp: u8,
    notes: i32,
    /// The slowest and the fastest tempo.
    bpm: (i32, i32),
    length_seconds: i32,
    /// What the chart is made of, as the wheel's feature bits.
    features: u32,
    /// Whether the chart counts as added within the day.
    is_new: bool,
    /// Whether the player has starred it.
    favorite: bool,
    best: Option<SampleBest>,
}

/// A sample chart with nothing but the plain: seven keys, no features, not new, not starred, never
/// played.
const PLAIN_SONG: SampleSong = SampleSong {
    title: "",
    artist: "PREVIEW ARTIST",
    genre: "SAMPLE GENRE",
    mode: Mode::BEAT_7K,
    level: 0,
    difficulty: 0,
    lamp: LAMP_NO_PLAY,
    notes: 0,
    bpm: (0, 0),
    length_seconds: 0,
    features: 0,
    is_new: false,
    favorite: false,
    best: None,
};

/// The charts of the sample list, top to bottom.
const SAMPLE_SONGS: [SampleSong; 12] = [
    SampleSong {
        title: "SAMPLE TRACK 01",
        level: 3,
        difficulty: 1,
        lamp: 10,
        notes: 420,
        bpm: (140, 140),
        length_seconds: 96,
        favorite: true,
        best: Some(SampleBest { judged: [420, 0, 0, 0], max_combo: 420, plays: 6, clears: 6 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 02",
        level: 5,
        difficulty: 2,
        lamp: 9,
        notes: 610,
        bpm: (150, 150),
        length_seconds: 104,
        best: Some(SampleBest { judged: [570, 40, 0, 0], max_combo: 610, plays: 9, clears: 8 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 03",
        artist: "PREVIEW ARTIST B",
        level: 7,
        difficulty: 3,
        lamp: 8,
        notes: 880,
        bpm: (128, 128),
        length_seconds: 121,
        features: SongBar::FEATURE_LONG_NOTE,
        best: Some(SampleBest { judged: [760, 105, 15, 0], max_combo: 880, plays: 12, clears: 9 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 04",
        artist: "PREVIEW ARTIST B",
        genre: "SAMPLE GENRE B",
        level: 9,
        difficulty: 4,
        lamp: 7,
        notes: 1240,
        bpm: (165, 165),
        length_seconds: 118,
        best: Some(SampleBest { judged: [1010, 190, 30, 10], max_combo: 612, plays: 15, clears: 7 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 05",
        genre: "SAMPLE GENRE B",
        level: 10,
        difficulty: 4,
        lamp: 6,
        notes: 1380,
        bpm: (90, 180),
        length_seconds: 126,
        features: SongBar::FEATURE_UNDEFINED_LN,
        favorite: true,
        best: Some(SampleBest { judged: [1040, 270, 50, 20], max_combo: 540, plays: 21, clears: 6 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 06",
        artist: "PREVIEW ARTIST C",
        level: 11,
        difficulty: 5,
        lamp: 5,
        notes: 1560,
        bpm: (172, 172),
        length_seconds: 132,
        features: SongBar::FEATURE_MINE_NOTE,
        best: Some(SampleBest { judged: [1090, 360, 70, 40], max_combo: 498, plays: 18, clears: 4 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 07",
        artist: "PREVIEW ARTIST C",
        genre: "SAMPLE GENRE C",
        level: 12,
        difficulty: 5,
        lamp: 4,
        notes: 1820,
        bpm: (185, 185),
        length_seconds: 140,
        features: SongBar::FEATURE_CHARGE_NOTE,
        best: Some(SampleBest { judged: [1180, 470, 100, 70], max_combo: 431, plays: 30, clears: 2 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 08",
        mode: Mode::BEAT_5K,
        level: 6,
        difficulty: 3,
        lamp: 3,
        notes: 700,
        bpm: (135, 135),
        length_seconds: 99,
        best: Some(SampleBest { judged: [480, 170, 30, 20], max_combo: 260, plays: 5, clears: 2 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 09",
        genre: "SAMPLE GENRE C",
        mode: Mode::BEAT_14K,
        level: 8,
        difficulty: 4,
        lamp: 2,
        notes: 1500,
        bpm: (160, 160),
        length_seconds: 124,
        best: Some(SampleBest { judged: [900, 440, 100, 60], max_combo: 300, plays: 4, clears: 1 }),
        ..PLAIN_SONG
    },
    SampleSong {
        title: "SAMPLE TRACK 10",
        artist: "PREVIEW ARTIST B",
        mode: Mode::POPN_9K,
        level: 28,
        difficulty: 3,
        lamp: 1,
        notes: 900,
        bpm: (200, 200),
        length_seconds: 110,
        best: Some(SampleBest { judged: [500, 250, 60, 90], max_combo: 180, plays: 4, clears: 0 }),
        ..PLAIN_SONG
    },
    SampleSong { title: "SAMPLE TRACK 11", level: 2, difficulty: 1, notes: 350, bpm: (120, 120), length_seconds: 92, is_new: true, ..PLAIN_SONG },
    SampleSong {
        title: "SAMPLE TRACK 12",
        genre: "SAMPLE GENRE B",
        mode: Mode::BEAT_10K,
        level: 9,
        difficulty: 4,
        notes: 1100,
        bpm: (148, 148),
        length_seconds: 115,
        features: SongBar::FEATURE_RANDOM,
        ..PLAIN_SONG
    },
];

impl SampleSong {
    /// The chart as a bar of the wheel.
    fn bar(&self) -> SongBar {
        SongBar {
            is_new: self.is_new,
            level: self.level,
            difficulty: self.difficulty,
            lamp: i32::from(self.lamp),
            features: self.features,
            ..SongBar::new(BarKind::Song { exists: true }, self.title)
        }
    }

    /// The best score the chart is given, as the score database would hold it, or `None` for a
    /// chart never played. Each judgement is split evenly between early and late.
    fn best(&self) -> Option<BestScore> {
        let best = self.best?;
        let [pgreat, great, good, poor] = best.judged;
        let mut counts = [0; JUDGE_TIER_COUNT];
        counts[TIER_PGREAT] = pgreat;
        counts[TIER_GREAT] = great;
        counts[TIER_GOOD] = good;
        counts[TIER_POOR] = poor;
        let early = counts.map(|count| count / TIMING_HALVES);
        let late = std::array::from_fn(|tier| counts[tier] - early[tier]);
        Some(BestScore {
            clear: self.lamp,
            judge: JudgeCounts::from_early_late(early, late),
            notes: u32::try_from(self.notes).unwrap_or_default(),
            combo: best.max_combo,
            minbp: i64::from(poor),
            playcount: best.plays,
            clearcount: best.clears,
            date: SAMPLE_PLAYED_AT,
            ..BestScore::default()
        })
    }

    fn has(&self, feature: u32) -> bool {
        self.features & feature != 0
    }

    /// The chart as the chart cluster reads the chart under the cursor. It has no picture of any
    /// kind and comes from no difficulty table.
    fn meta(&self) -> ChartMeta<'static> {
        let (slowest, fastest) = self.bpm;
        let has_long_notes =
            self.has(SongBar::FEATURE_LONG_NOTE | SongBar::FEATURE_UNDEFINED_LN | SongBar::FEATURE_CHARGE_NOTE | SongBar::FEATURE_HELL_CHARGE_NOTE);
        let long = if has_long_notes { self.notes / LONG_NOTE_EVERY } else { 0 };
        let scratch = if self.mode.scratch.is_empty() { 0 } else { self.notes / SCRATCH_NOTE_EVERY };
        let average = f64::from(self.notes) / f64::from(self.length_seconds.max(1));
        ChartMeta {
            title: self.title,
            genre: self.genre,
            artist: self.artist,
            level: self.level,
            difficulty: self.difficulty,
            mode: Some(self.mode),
            judge: Some(SAMPLE_JUDGE_RANK),
            length_ms: Some(self.length_seconds * MILLIS_PER_SECOND),
            notes: Some(self.notes),
            bpm: Some(BpmRange { min: slowest, max: fastest }),
            main_bpm: (slowest == fastest).then(|| f64::from(slowest)),
            note_counts: Some(NoteCounts { normal: self.notes - long - scratch, long, scratch, long_scratch: 0 }),
            lane_notes: self.lane_notes(scratch),
            density: Some(Density { average, peak: average * PEAK_DENSITY_SHARE, end: average * END_DENSITY_SHARE }),
            notes_radar: self.notes_radar(has_long_notes),
            total: Some(SAMPLE_TOTAL),
            contents: ChartContents {
                bga: Some(false),
                text: Some(false),
                long_note: Some(has_long_notes),
                random_sequence: Some(self.has(SongBar::FEATURE_RANDOM)),
                bpm_stop: Some(false),
                stagefile: Some(false),
                banner: Some(false),
                backbmp: Some(false),
            },
            ..ChartMeta::default()
        }
    }
}

impl SampleSong {
    /// The notes on each lane: the turntable's `scratch` shared among the turntables, and the rest
    /// among the keys by [`KEY_LANE_WEIGHTS`]. A mode that is not counted lane by lane has none.
    fn lane_notes(&self, scratch: i32) -> Option<LaneNotes> {
        if !is_lane_counted(&self.mode) {
            return None;
        }
        let turntables = i32::try_from(self.mode.scratch.len()).unwrap_or(1).max(1);
        let keys = self.mode.key - self.mode.scratch.len();
        let weight = |key: usize| KEY_LANE_WEIGHTS[key % KEY_LANE_WEIGHTS.len()];
        let all_weights: i32 = (0..keys).map(weight).sum();
        let mut key = 0;
        let counts: Vec<i32> = (0..self.mode.key)
            .map(|lane| {
                if self.mode.is_scratch(lane) {
                    return scratch / turntables;
                }
                key += 1;
                (self.notes - scratch) * weight(key - 1) / all_weights.max(1)
            })
            .collect();
        LaneNotes::of_counts(&counts)
    }

    /// The NOTES RADAR as shown, reaching further the higher the chart's level. A mode that has no
    /// radar has none.
    fn notes_radar(&self, has_long_notes: bool) -> Option<[f32; RADAR_AXIS_COUNT]> {
        if !is_lane_counted(&self.mode) {
            return None;
        }
        let reach = (self.level as f32 / SAMPLE_RADAR_FULL_LEVEL).min(1.0) * RADAR_DISPLAY_MAX;
        let mut radar = SAMPLE_RADAR_SHARES.map(|share| share * reach);
        if !has_long_notes {
            radar[RadarAxis::Charge as usize] = 0.0;
        }
        Some(radar)
    }
}

/// How the charts of the first sample folder are cleared and scored.
fn folder_distribution() -> BarDistribution {
    BarDistribution { lamps: FOLDER_LAMPS, ranks: std::array::from_fn(|rank: usize| u32::try_from(rank % FOLDER_RANK_WAVE).unwrap_or_default()) }
}

/// The bars of the sample list that are not charts and stand above them: the two folders, the first
/// with its charts counted and the second new, and the table.
fn bars_above() -> Vec<SongBar> {
    let [counted, new] = FOLDER_TITLES;
    vec![
        SongBar { distribution: Some(Box::new(folder_distribution())), ..SongBar::new(BarKind::Folder, counted) },
        SongBar { is_new: true, ..SongBar::new(BarKind::Folder, new) },
        SongBar::new(BarKind::Table, TABLE_TITLE),
    ]
}

/// The bars that stand below the charts: a course every chart of which is there, with its medal, and
/// one that is missing a chart.
fn bars_below() -> Vec<SongBar> {
    let [complete, missing] = COURSE_TITLES;
    vec![
        SongBar { trophy: Some(BarTrophy::Gold), lamp: i32::from(LAMP_FULL_COMBO), ..SongBar::new(BarKind::Course { complete: true }, complete) },
        SongBar::new(BarKind::Course { complete: false }, missing),
    ]
}

/// The sample list on the song browser: its bars, where the cursor is and how the wheel is sliding.
pub(crate) struct SelectSample {
    bars: Vec<SongBar>,
    /// Where the first chart is in `bars`, which is where the cursor begins.
    first_song: usize,
    /// The best score of each chart, in the order of [`SAMPLE_SONGS`].
    bests: Vec<Option<BestScore>>,
    /// The stages of the first sample course.
    course: CourseShown,
    cursor: usize,
    /// How many times the cursor has moved.
    steps: i64,
    scroller: BarScroller,
}

impl SelectSample {
    /// The sample list with the cursor on its first chart and the wheel at rest.
    pub(crate) fn new() -> SelectSample {
        let above = bars_above();
        let first_song = above.len();
        let bars = above.into_iter().chain(SAMPLE_SONGS.iter().map(SampleSong::bar)).chain(bars_below()).collect();
        SelectSample {
            bars,
            first_song,
            bests: SAMPLE_SONGS.iter().map(SampleSong::best).collect(),
            course: CourseShown::new(&[], COURSE_STAGE_TITLES.into_iter().map(|title| (title, true))),
            cursor: first_song,
            steps: 0,
            scroller: BarScroller::default(),
        }
    }

    /// The preview's time as the clock a wheel slides on, in milliseconds.
    fn slide_clock_ms(now_us: i64) -> i64 {
        SLIDE_CLOCK_BASE_MS + now_us / MICROS_PER_MILLI
    }

    /// The chart under the cursor, when the bar there is one, with its place in [`SAMPLE_SONGS`].
    fn song_under_cursor(&self) -> Option<(usize, &'static SampleSong)> {
        let index = self.cursor.checked_sub(self.first_song)?;
        Some((index, SAMPLE_SONGS.get(index)?))
    }

    /// Move the list on to `now_us`. `ready` is whether the skin that draws it can be drawn.
    ///
    /// The timers are the ones the browser keeps for its own skin: the start of input once the
    /// skin's input time has passed, and the bar timer, which is on from the first frame and starts
    /// over each time the cursor moves. A browser's scene does not end, so this never says it has.
    pub(crate) fn step(&mut self, shared: &mut AppShared, now_us: i64, ready: bool) -> SampleStep {
        if !ready {
            return SampleStep::Running;
        }
        let times = SceneTimes { scene_ms: NO_SCENE_END_MS, ..SceneTimes::of_skin(shared.skins.document(SKIN_TYPE_MUSIC_SELECT)) };
        advance(times, &mut shared.skin_timers, now_us);
        let due = now_us / CURSOR_STEP_US;
        let moves = due > self.steps && !self.bars.is_empty();
        self.scroller.input(if moves { ONE_NOTCH } else { NO_NOTCH }, BarHold::None, SelectSample::slide_clock_ms(now_us));
        if moves {
            self.steps = due;
            self.cursor = (self.cursor + 1) % self.bars.len();
            shared.skin_timers.set_on(timer_id::SONGBAR_CHANGE, now_us);
        }
        shared.skin_timers.switch(timer_id::SONGBAR_CHANGE, true, now_us);
        SampleStep::Running
    }

    /// Draw one frame of the browser into `at.rect`.
    ///
    /// The frame is what the browser hands its own skin: the list as bars with the cursor and the
    /// slide, the chart under the cursor for the chart cluster, and the browser's frame for the
    /// clusters that read the bar, its best score, the settings and the ranking. The settings are
    /// the player's own. No panel is open, no mode is filtered for, and there is no score server.
    pub(crate) fn draw(&self, shared: &AppShared, canvas: &mut Canvas<'_>, at: PreviewFrame) -> PreviewDrawn {
        let config: &Config = &shared.config;
        let wheel = SongBars {
            ln_mode: i32::from(config.judge.ln_mode.id()),
            scroll: self.scroller.at(SelectSample::slide_clock_ms(at.now_us)),
            ..SongBars::new(&self.bars, self.cursor)
        };
        let song = self.song_under_cursor();
        let best = song.and_then(|(index, _)| self.bests.get(index)).and_then(Option::as_ref);
        let on_course = self.bars.get(self.cursor).is_some_and(|bar| bar.kind == BarKind::Course { complete: true });
        let shown = SelectShown::of(SelectInputs {
            bars: &self.bars,
            cursor: self.cursor,
            chart: song.map(|(_, song)| ChartInputs { mode: song.mode, best, favorite: song.favorite, replays_stored: usize::from(best.is_some()) }),
            course: on_course.then(|| self.course.clone()),
            directory: vec![OPEN_FOLDER_TITLE.to_owned()],
            mode_filter: None,
            sort: config.library.sort,
            panel: Some(0),
            selected_replay: best.map(|_| 0),
            config,
            ir: IrBrowser::default(),
        });
        let meta = song.map(|(_, song)| song.meta());
        let chart = meta.as_ref().map_or(ChartState::Empty, ChartState::Chart);
        let images = ReferenceImages { notes_radar: meta.as_ref().and_then(|meta| meta.notes_radar), ..ReferenceImages::default() };
        let data = FrameData { bars: Some(&wheel), images, bga: BgaFrame::of(None), ..FrameData::default() };
        shared.draw_skin_preview(canvas, at, PreviewScene::Select(PreviewSelect { data, chart, shown: &shown }))
    }

    /// Where the cursor is, for the tests that check it moves.
    #[cfg(test)]
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every sample chart of a mode counted lane by lane has its notes on its lanes and a radar a
    /// skin can draw; one of another mode has neither, as a chart of that mode would not.
    #[test]
    fn a_sample_chart_has_lane_counts_and_a_radar_as_a_chart_of_its_mode_would() {
        for song in SAMPLE_SONGS {
            let meta = song.meta();
            if !is_lane_counted(&song.mode) {
                assert_eq!((meta.lane_notes, meta.notes_radar), (None, None), "{}", song.title);
                continue;
            }
            let lanes = meta.lane_notes.expect("a counted mode has lane counts");
            assert_eq!((0..song.mode.key).filter(|lane| lanes.on(*lane).is_some()).count(), song.mode.key, "{}", song.title);
            assert!((0..song.mode.key).filter_map(|lane| lanes.on(lane)).sum::<i32>() <= song.notes, "{}", song.title);
            let radar = meta.notes_radar.expect("a counted mode has a radar");
            assert!(radar.iter().all(|value| (0.0..=RADAR_DISPLAY_MAX).contains(value)), "{}: {radar:?}", song.title);
        }
    }
}
