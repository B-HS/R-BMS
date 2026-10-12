//! The judge offset as the default skin pack draws it: one chart, one moment of the song, and the
//! note field at an offset of nothing and at one of a tenth of a second.
//!
//! The chart has a row of notes on every beat, and the moment is a tenth of a second before one of
//! them is due. At an offset of nothing the row is that far up the lane. At +100 ms it is on the
//! judge line, a tenth of a second before the game takes it, which is where the reference draws it
//! (`LaneRenderer.java:300-301`). Nothing else of the run differs: the two have taken the same
//! notes with the same judgements at that moment.
//!
//! The chart's background is not the note field's: it changes when the song reaches the change,
//! whatever the offset (`SkinBGA.java:42-44` reads the play timer and no judge timing), and that is
//! held to as well, on the same pack.
//!
//! What is held to here is that much. What the frames look like is for a person: with
//! [`CAPTURE_DIR_ENV`] naming a folder each is saved there as a PNG at the pack's own size, and
//! without it they are drawn small and nothing is written.

use std::ops::Range;
use std::path::Path;
use std::time::{Duration, Instant};

use rbms_play::SessionOptions;
use rbms_render::skin_render::frame::BgaPick;
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;

use super::*;
use crate::stage::capture::{app_in, settings_of};
use crate::stage::render_tests::FRAME_DT;
use crate::stage::{HeadlessCanvas, Stage, StageId};
use crate::{App, CH, CW, Config};

/// Names the folder the frames are saved in, as it does for the captures of the other screens.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The default pack, relative to this crate.
const DEFAULT_PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// The seed a pack's random picks are pinned with, so two captures of one frame are alike.
const PACK_SEED: u64 = 1;

/// How long the pack's play document is waited for.
const PACK_LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// How many times smaller than the pack's own size a frame nobody saves is drawn.
const UNSAVED_SHRINK: u32 = 3;

/// How far past a time of the scene a frame is run to be on the other side of it, in milliseconds.
const SLACK_MS: i64 = 250;

/// How far a run is walked at a time between the frames that are drawn, in milliseconds.
const WALK_STEP_MS: i64 = 16;

/// Far enough ahead that the wall clock never catches the song clock the test counts.
const CLOCK_HELD_BACK: Duration = Duration::from_secs(3_600);

/// The tempo of the chart, how long one of its beats lasts at it, and how many measures it has.
const BPM: u32 = 150;
const BEAT_MS: i64 = 400;
const MEASURES: u32 = 12;

/// The beat whose row of notes the frames are drawn for, counted from the start of the chart.
const SHOWN_BEAT: i64 = 12;

/// The offset the second frame is drawn at, in milliseconds, which is also how long before the
/// row is due both frames are drawn.
const OFFSET_MS: i32 = 100;

/// The offsets the chart's background is watched at, in milliseconds.
const BACKGROUND_OFFSETS_MS: [i32; 3] = [0, OFFSET_MS, -OFFSET_MS];

/// How long either side of a moment a frame is drawn to be inside the offset of it, in
/// milliseconds: less than the offset, so what is drawn the offset ahead of the song and what is
/// not are on different sides of the moment.
const INSIDE_THE_OFFSET_MS: i64 = 50;

/// The picture the chart puts up, the measure it puts it up at the head of, and how many beats a
/// measure has.
const PICTURE: i32 = 1;
const PICTURE_MEASURE: i64 = 4;
const BEATS_PER_MEASURE: i64 = 4;

/// The share of a frame's width its note field is looked for in, and of its height the lanes are:
/// the left half, above the keys.
const FIELD_COLUMNS_SHARE: u32 = 2;

/// A seven-key chart with a row of three notes on every beat.
fn rows_on_every_beat() -> String {
    let mut chart = format!(
        "#PLAYER 1\n#GENRE Judge Offset\n#TITLE Rows On The Beat\n#ARTIST Capture\n#BPM {BPM}\n#PLAYLEVEL 1\n#DIFFICULTY 2\n#RANK 3\n#TOTAL 300\n#WAV01 a.wav\n"
    );
    for measure in 1..=MEASURES {
        for channel in ["11", "13", "15"] {
            chart.push_str(&format!("#{measure:03}{channel}:01010101\n"));
        }
    }
    chart
}

/// The same chart with a picture put up at the head of one measure.
fn rows_under_a_picture() -> String {
    format!("{}#BMP{PICTURE:02} picture.png\n#{PICTURE_MEASURE:03}04:{PICTURE:02}\n", rows_on_every_beat())
}

/// A run of `chart` the game plays.
fn autoplayed(chart: &str) -> PlayState {
    let src = rbms_parser::parse_with(chart.as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "rows.bms"));
    PlayState::new(
        PlaySession::new(model, SessionOptions { autoplay: true, ..SessionOptions::default() }),
        std::collections::HashMap::new(),
        0,
        SCORE_LN_MODE_FROM_CHART.to_string(),
    )
}

/// The two clocks of a run, counted here rather than read off the machine: how long the scene has
/// run, and where the song stands once the chart is playing.
#[derive(Default)]
struct Clocks {
    scene: Duration,
    song_ms: Option<i64>,
}

impl Clocks {
    /// Move both clocks on by `millis` and run one frame of the play screen that is up: its update,
    /// and its draw onto `pixels` when there is one.
    fn frame_after(&mut self, app: &mut App, millis: i64, pixels: Option<&mut HeadlessCanvas>) {
        self.scene += Duration::from_millis(u64::try_from(millis).expect("a run only moves forward"));
        self.song_ms = self.song_ms.map(|song_ms| song_ms + millis);
        app.shared.put_skin_scene_at(self.scene);
        if let Some(song_ms) = self.song_ms {
            app.shared.clock = Instant::now() + CLOCK_HELD_BACK;
            app.shared.song_us_last.set(song_ms * MICROS_PER_MILLI);
        }
        let now = Instant::now();
        let transition = app.stage.update(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT });
        assert!(matches!(transition, Transition::Stay), "the play screen left in the {:?} state", phase(app));
        if self.song_ms.is_none() && phase(app) == Some(PlayPhase::Play) {
            self.song_ms = Some(0);
        }
        if let Some(pixels) = pixels {
            app.stage.draw(&mut FrameCtx { shared: &mut app.shared, now, dt: FRAME_DT }, &mut Canvas::Headless(pixels));
        }
    }

    /// Walk the chart that is playing on to `chart_ms` without drawing it, and draw the frame there.
    fn walk_to(&mut self, app: &mut App, pixels: &mut HeadlessCanvas, chart_ms: i64) {
        loop {
            let left = chart_ms - self.song_ms.expect("the chart is playing");
            if left <= WALK_STEP_MS {
                return self.frame_after(app, left.max(0), Some(pixels));
            }
            self.frame_after(app, WALK_STEP_MS, None);
        }
    }
}

/// The play screen that is up.
fn play(app: &App) -> &PlayState {
    let Stage::Play(state) = &app.stage else {
        panic!("the {:?} screen is up, not the play screen", app.stage.id());
    };
    state
}

fn phase(app: &App) -> Option<PlayPhase> {
    play(app).phase()
}

/// An app drawn by the default pack whose game plays the chart, at a judge offset of `offset_ms`.
/// The machine's side of its scene clock is stopped, so the scene stands where [`Clocks`] put it
/// from a frame's update to its draw however long the machine takes between the two.
fn app_on_the_default_pack(tag: &str, offset_ms: i32) -> App {
    let mut config = Config::default();
    config.play.autoplay = true;
    config.judge.offset_ms = offset_ms;
    let settings = settings_of(&format!("play-{tag}"));
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER);
    let pack = rbms_skin::resolve::plain_path(&folder.canonicalize().unwrap_or_else(|error| panic!("{} cannot be resolved: {error}", folder.display())));
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    app.shared.replay = None;
    app.shared.stop_scene_clock();
    app
}

/// The size the frames are drawn at: the one the pack's seven-key play document was authored at
/// when they are being saved for a person to look at, and a third of it otherwise.
fn frame_size(app: &App) -> (u32, u32) {
    let header = app.shared.skins.header_of(&app.shared.config, SKIN_TYPE_PLAY_7KEYS).expect("the default pack has a seven-key play skin");
    let authored = (
        u32::try_from(header.width).ok().filter(|width| *width > 0).unwrap_or(CW),
        u32::try_from(header.height).ok().filter(|height| *height > 0).unwrap_or(CH),
    );
    if std::env::var_os(CAPTURE_DIR_ENV).is_some() { authored } else { (authored.0 / UNSAVED_SHRINK, authored.1 / UNSAVED_SHRINK) }
}

/// Save the frame on `pixels` under the capture folder, when one was asked for.
fn save(name: &str, pixels: &HeadlessCanvas, size: (u32, u32)) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).expect("the capture folder can be created");
    image::save_buffer(folder.join(format!("{name}.png")), pixels.rgba(), size.0, size.1, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

/// What a run has come to at the moment its frame was drawn.
struct Drawn {
    rgba: Vec<u8>,
    size: (u32, u32),
    judged: u32,
    counts: [u32; 6],
    field_us: i64,
}

/// A run on the default pack with its chart playing, and what it is walked and drawn with.
struct Playing {
    app: App,
    pixels: HeadlessCanvas,
    clocks: Clocks,
    size: (u32, u32),
}

/// Enter `run` on the default pack at a judge offset of `offset_ms` and walk its scene on the
/// pack's own times until the chart plays.
fn playing_on_the_default_pack(tag: &str, offset_ms: i32, run: PlayState) -> Playing {
    let mut app = app_on_the_default_pack(tag, offset_ms);
    let size = frame_size(&app);
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let mut clocks = Clocks::default();
    app.switch(Transition::To(Stage::Play(Box::new(run))));
    assert_eq!(app.stage.id(), StageId::Play);
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS) {
        clocks.frame_after(&mut app, 0, Some(&mut pixels));
        assert_eq!(app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS), None, "the default pack's play skin could not be read");
        assert!(Instant::now() < deadline, "the default pack's play skin was not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs());
    }
    clocks.frame_after(&mut app, 0, Some(&mut pixels));
    let skin = app.shared.skins.document(SKIN_TYPE_PLAY_7KEYS).expect("a compiled screen has its document");
    let (loadend_ms, playstart_ms) = (i64::from(skin.play.loadend), i64::from(skin.play.playstart));
    clocks.frame_after(&mut app, loadend_ms + SLACK_MS, Some(&mut pixels));
    assert_eq!(phase(&app), Some(PlayPhase::Ready));
    clocks.frame_after(&mut app, playstart_ms + SLACK_MS, Some(&mut pixels));
    assert_eq!(phase(&app), Some(PlayPhase::Play));
    Playing { app, pixels, clocks, size }
}

impl Playing {
    /// Walk the chart on to `chart_ms` and draw the frame there.
    fn walk_to(&mut self, chart_ms: i64) {
        self.clocks.walk_to(&mut self.app, &mut self.pixels, chart_ms);
        assert_eq!(play(&self.app).chart_ms(), chart_ms, "the frame was not drawn at the moment asked for");
    }
}

/// The rows of notes at a judge offset of `offset_ms`, drawn at `chart_ms`.
fn drawn_at(tag: &str, offset_ms: i32, chart_ms: i64) -> Drawn {
    let mut playing = playing_on_the_default_pack(tag, offset_ms, autoplayed(&rows_on_every_beat()));
    playing.walk_to(chart_ms);
    let Playing { app, pixels, size, .. } = playing;
    save(tag, &pixels, size);
    let judge = play(&app).session.judge();
    Drawn { rgba: pixels.rgba().to_vec(), size, judged: judge.total_judged(), counts: judge.counts, field_us: play(&app).drawn_chart_us(&app.shared) }
}

/// How many pixels of `columns` two frames of one size differ in.
fn differing_in(one: &Drawn, other: &Drawn, columns: Range<u32>) -> usize {
    const CHANNELS: usize = 4;
    let row = one.size.0 as usize * CHANNELS;
    let cut = columns.start as usize * CHANNELS..columns.end as usize * CHANNELS;
    one.rgba
        .chunks(row)
        .zip(other.rgba.chunks(row))
        .map(|(a, b)| a[cut.clone()].chunks(CHANNELS).zip(b[cut.clone()].chunks(CHANNELS)).filter(|(a, b)| a != b).count())
        .sum()
}

/// A tenth of a second before a row of notes is due, a run at an offset of +100 ms is drawn with
/// that row on the judge line and a run at none with it a tenth of a second up the lane. The two
/// have judged the same notes the same way: the offset moved what is drawn and nothing else.
#[test]
fn the_default_pack_draws_the_notes_the_offset_ahead_of_the_song() {
    let due_ms = SHOWN_BEAT * BEAT_MS;
    let shown_ms = due_ms - i64::from(OFFSET_MS);
    let unmoved = drawn_at("offset-0-100ms-before-the-row", 0, shown_ms);
    let ahead = drawn_at("offset-plus-100-100ms-before-the-row", OFFSET_MS, shown_ms);

    assert_eq!(unmoved.field_us, shown_ms * MICROS_PER_MILLI, "at no offset the field is drawn where the song is");
    assert_eq!(ahead.field_us, due_ms * MICROS_PER_MILLI, "at +{OFFSET_MS} ms the row that is due {OFFSET_MS} ms from now is on the line");
    assert_eq!((ahead.judged, ahead.counts), (unmoved.judged, unmoved.counts), "the offset moved a judgement");
    assert!(unmoved.judged > 0, "the run had taken nothing yet, so the two had nothing to differ in");
    let field = 0..unmoved.size.0 / FIELD_COLUMNS_SHARE;
    assert!(differing_in(&unmoved, &ahead, field) > 0, "the note field is drawn alike at the two offsets");
}

/// A frame for whoever looks at the two above: the run at no offset once more, a moment before the
/// row is taken, which is the row where +100 ms has it a tenth of a second sooner. Saved only, so
/// it is drawn only when the captures are asked for.
#[test]
fn the_row_on_the_line_at_no_offset_is_captured_for_comparison() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let due_ms = SHOWN_BEAT * BEAT_MS;
    let on_the_line = drawn_at("offset-0-1ms-before-the-row", 0, due_ms - 1);
    assert_eq!(on_the_line.field_us, (due_ms - 1) * MICROS_PER_MILLI);
}

/// Two more frames for whoever looks, of the time between a row reaching the line and the game
/// taking it. A note is drawn until the field's time reaches it and no longer, whatever has judged
/// it (`LaneRenderer.java:546`): at +100 ms the row is gone from the line and the game has not
/// taken it yet, and at -100 ms the game has taken the row and it is still on its way down. Saved
/// only, so they are drawn only when the captures are asked for.
#[test]
fn the_rows_between_the_line_and_the_game_taking_them_are_captured_for_comparison() {
    if std::env::var_os(CAPTURE_DIR_ENV).is_none() {
        return;
    }
    let due_ms = SHOWN_BEAT * BEAT_MS;
    let just_before = drawn_at("offset-0-50ms-before-the-row", 0, due_ms - INSIDE_THE_OFFSET_MS);
    let gone_from_the_line = drawn_at("offset-plus-100-50ms-before-the-row", OFFSET_MS, due_ms - INSIDE_THE_OFFSET_MS);
    assert_eq!(gone_from_the_line.judged, just_before.judged, "the row was taken before it was due at +{OFFSET_MS} ms");
    let just_after = drawn_at("offset-0-50ms-after-the-row", 0, due_ms + INSIDE_THE_OFFSET_MS);
    let still_falling = drawn_at("offset-minus-100-50ms-after-the-row", -OFFSET_MS, due_ms + INSIDE_THE_OFFSET_MS);
    assert_eq!(still_falling.judged, just_after.judged, "the row was not taken when it was due at -{OFFSET_MS} ms");
    assert!(just_after.judged > just_before.judged, "the row is taken between the two moments");
}

/// The chart's picture goes up when the song reaches it, at every offset: the background is not the
/// note field, and the reference draws it by the play timer alone (`SkinBGA.java:42-44`).
#[test]
fn the_default_pack_changes_the_background_with_the_song_at_every_offset() {
    let picture_ms = PICTURE_MEASURE * BEATS_PER_MEASURE * BEAT_MS;
    for offset_ms in BACKGROUND_OFFSETS_MS {
        let mut playing = playing_on_the_default_pack(&format!("background-offset-{offset_ms}"), offset_ms, autoplayed(&rows_under_a_picture()));
        playing.walk_to(picture_ms - INSIDE_THE_OFFSET_MS);
        let before = play(&playing.app).background.pick();
        playing.walk_to(picture_ms + INSIDE_THE_OFFSET_MS);
        let after = play(&playing.app).background.pick();
        assert_eq!(before, BgaPick::Playing { base: None, layer: None }, "the picture was up before the song reached it at {offset_ms:+} ms");
        assert_eq!(after, BgaPick::Playing { base: Some(PICTURE), layer: None }, "the picture was not up once the song had reached it at {offset_ms:+} ms");
    }
}
