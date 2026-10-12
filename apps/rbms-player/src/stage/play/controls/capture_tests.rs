//! The held controls as the default skin pack shows them: a run played by hand with START down and
//! the turntable moving the lane cover, START and SELECT down and the lift moved, and the judge
//! offset just after its key.
//!
//! What is held to here is that the moves are the ones on show: the figures a skin reads are the
//! ones the controls left, and the frame changed with them. What the frames look like is for a
//! person: with [`CAPTURE_DIR_ENV`] naming a folder each is saved there as a PNG at the pack's own
//! size, and without it they are drawn small and nothing is written.

use std::path::Path;
use std::time::{Duration, Instant};

use rbms_config::{Config, number_steps};
use rbms_model::Mode;
use rbms_play::{PlaySession, SessionOptions};
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
use rbms_skin::property::generated::{NUMBER_DURATION_GREEN, NUMBER_JUDGETIMING, NUMBER_LANECOVER1, NUMBER_LIFT1, OPTION_LANECOVER1_CHANGING};
use rbms_store::SCORE_LN_MODE_FROM_CHART;
use winit::keyboard::KeyCode;

use super::*;
use crate::skin_host::ClusterState;
use crate::skin_host::play::{PlayPhase, PlayShown};
use crate::stage::capture::{app_in, settings_of};
use crate::stage::play::{DocumentFrame, MICROS_PER_MILLI};
use crate::stage::render_tests::FRAME_DT;
use crate::stage::select::tests::press;
use crate::stage::{Canvas, FrameCtx, HeadlessCanvas, Stage, StageId, Transition};
use crate::{App, CH, CW};

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

/// Where in the chart the frames are drawn, in milliseconds: a few measures in, with notes on the
/// lanes.
const SHOWN_MS: i64 = 4_650;

/// The keys START and SELECT ship on, and the key the judge offset ships up on.
const START_KEY: KeyCode = KeyCode::KeyA;
const SELECT_KEY: KeyCode = KeyCode::KeyW;
const OFFSET_UP_KEY: KeyCode = KeyCode::Digit0;

/// How far down a frame the foot of the default pack's field begins, in thousandths of its height:
/// under the gauge and the counts beside it, and above the figures of the foot.
const FOOT_TOP_PER_MILLE: usize = 900;
const PER_MILLE: usize = 1_000;

/// The bytes of one pixel of a frame.
const RGBA_BYTES: usize = 4;

/// The figures the run starts with, and how far the turntable is turned.
const COVER_AT: i64 = 300;
const LIFT_AT: i64 = 150;
const TURNED_TICKS: i32 = 25;
const OFFSET_PRESSES: i32 = 3;

/// A seven-key chart with a row of three notes on every beat.
fn rows_on_every_beat() -> String {
    let mut chart = String::from(
        "#PLAYER 1\n#GENRE Held Controls\n#TITLE Rows On The Beat\n#ARTIST Capture\n#BPM 150\n#PLAYLEVEL 1\n#DIFFICULTY 2\n#RANK 3\n#TOTAL 300\n#WAV01 a.wav\n",
    );
    for measure in 1..=12 {
        for channel in ["11", "13", "15"] {
            chart.push_str(&format!("#{measure:03}{channel}:01010101\n"));
        }
    }
    chart
}

/// The two clocks of a run, counted here rather than read off the machine.
#[derive(Default)]
struct Clocks {
    scene: Duration,
    song_ms: Option<i64>,
}

/// A run on the default pack with its chart playing by hand, and what it is walked and drawn with.
struct Playing {
    app: App,
    pixels: HeadlessCanvas,
    clocks: Clocks,
    size: (u32, u32),
    now: Instant,
}

fn play(app: &App) -> &PlayState {
    let Stage::Play(state) = &app.stage else {
        panic!("the {:?} screen is up, not the play screen", app.stage.id());
    };
    state
}

impl Playing {
    /// Move both clocks on by `millis` and run one frame of the play screen: its update, and its
    /// draw when `drawn`.
    fn frame_after(&mut self, millis: i64, drawn: bool) {
        let step = Duration::from_millis(u64::try_from(millis).expect("a run only moves forward"));
        self.clocks.scene += step;
        self.now += step;
        self.clocks.song_ms = self.clocks.song_ms.map(|song_ms| song_ms + millis);
        self.app.shared.put_skin_scene_at(self.clocks.scene);
        if let Some(song_ms) = self.clocks.song_ms {
            self.app.shared.clock = Instant::now() + CLOCK_HELD_BACK;
            self.app.shared.song_us_last.set(song_ms * MICROS_PER_MILLI);
        }
        let transition = self.app.stage.update(&mut FrameCtx { shared: &mut self.app.shared, now: self.now, dt: FRAME_DT });
        assert!(matches!(transition, Transition::Stay), "the play screen left in the {:?} state", play(&self.app).phase());
        if self.clocks.song_ms.is_none() && play(&self.app).phase() == Some(PlayPhase::Play) {
            self.clocks.song_ms = Some(0);
        }
        if drawn {
            self.app.stage.draw(&mut FrameCtx { shared: &mut self.app.shared, now: self.now, dt: FRAME_DT }, &mut Canvas::Headless(&mut self.pixels));
        }
    }

    /// Walk the chart that is playing on to `chart_ms` without drawing it, and draw the frame there.
    fn walk_to(&mut self, chart_ms: i64) {
        loop {
            let left = chart_ms - self.clocks.song_ms.expect("the chart is playing");
            if left <= WALK_STEP_MS {
                return self.frame_after(left.max(0), true);
            }
            self.frame_after(WALK_STEP_MS, false);
        }
    }

    /// One key going down, as the window reports it, and the frame after it.
    fn key(&mut self, code: KeyCode) {
        let transition = self.app.stage.handle_key(&mut FrameCtx { shared: &mut self.app.shared, now: self.now, dt: 0.0 }, press(code));
        assert!(matches!(transition, Transition::Stay));
        self.frame_after(WALK_STEP_MS, true);
    }

    /// One frame of the held controls with the turntable turned `ticks`, and the frame after it.
    fn turn(&mut self, ticks: i32) {
        let Stage::Play(state) = &mut self.app.stage else {
            panic!("the play screen is not up");
        };
        state.read_held(&mut self.app.shared, self.now, Some(ticks));
        self.frame_after(WALK_STEP_MS, true);
    }

    /// One figure of the frame the screen lends a skin right now.
    fn figures(&mut self) -> Figures {
        let Stage::Play(state) = &mut self.app.stage else {
            panic!("the play screen is not up");
        };
        let shared = &self.app.shared;
        let (bpm, scroll) = state.tempo_at(state.song_us);
        let frame = DocumentFrame { bpm, travel: (bpm, scroll), hispeed: shared.config.play.hispeed, background: None };
        let feed = state.feed.as_deref().expect("a run a skin draws has something to feed it from");
        let shown = PlayShown::of(&state.session, &state.live(shared, feed, frame));
        let cluster = crate::skin_host::play::PlayState::of(&shown);
        let number = |id: i32| cluster.integer(id).expect("the play cluster answers the id");
        Figures {
            white: number(NUMBER_LANECOVER1),
            lift: number(NUMBER_LIFT1),
            green: number(NUMBER_DURATION_GREEN),
            timing: number(NUMBER_JUDGETIMING),
            changing: cluster.boolean(OPTION_LANECOVER1_CHANGING).expect("the play cluster answers the option"),
        }
    }

    /// The pixels of the frame under the field and its gauge, on the field's side of the screen:
    /// the foot, where the figures of the speed and of the shades are, and nothing that moves with
    /// the chart.
    fn foot(&self) -> Vec<u8> {
        let (width, height) = (self.size.0 as usize, self.size.1 as usize);
        let top = height * FOOT_TOP_PER_MILLE / PER_MILLE;
        let row = |y: usize| &self.pixels.rgba()[y * width * RGBA_BYTES..(y * width + width / 2) * RGBA_BYTES];
        (top..height).flat_map(row).copied().collect()
    }

    /// Save the frame under the capture folder, when one was asked for, and answer its pixels.
    fn save(&self, name: &str) -> Vec<u8> {
        if let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) {
            let folder = Path::new(&folder);
            std::fs::create_dir_all(folder).expect("the capture folder can be created");
            image::save_buffer(folder.join(format!("{name}.png")), self.pixels.rgba(), self.size.0, self.size.1, image::ExtendedColorType::Rgba8)
                .expect("the capture is written");
        }
        self.pixels.rgba().to_vec()
    }
}

/// What a skin reads of the shades, the green number and the judge offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Figures {
    white: i32,
    lift: i32,
    green: i32,
    timing: i32,
    changing: bool,
}

/// A run of the chart played by hand on the default pack, under a lane cover and a lift, walked on
/// the pack's own times until the chart plays and then to [`SHOWN_MS`].
fn playing_by_hand(tag: &str) -> Playing {
    playing_by_hand_with_the_lift(tag, true)
}

/// The same run with the lift switched on or off.
fn playing_by_hand_with_the_lift(tag: &str, lifted: bool) -> Playing {
    let mut config = Config::default();
    config.play.autoplay = false;
    config.play.enable_cover = true;
    config.play.enable_lift = lifted;
    config.judge.offset_ms = 0;
    config.judge.auto_offset = false;
    rbms_config::set_number_steps(&mut config, SettingId::LaneCover, COVER_AT);
    rbms_config::set_number_steps(&mut config, SettingId::Lift, LIFT_AT);
    let settings = settings_of(&format!("held-controls-{tag}"));
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER);
    let pack = rbms_skin::resolve::plain_path(&folder.canonicalize().unwrap_or_else(|error| panic!("{} cannot be resolved: {error}", folder.display())));
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.shared.keyconfig = crate::keyconfig::KeyConfig::default();
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    app.shared.replay = None;
    app.shared.stop_scene_clock();

    let header = app.shared.skins.header_of(&app.shared.config, SKIN_TYPE_PLAY_7KEYS).expect("the default pack has a seven-key play skin");
    let authored = (
        u32::try_from(header.width).ok().filter(|width| *width > 0).unwrap_or(CW),
        u32::try_from(header.height).ok().filter(|height| *height > 0).unwrap_or(CH),
    );
    let size = if std::env::var_os(CAPTURE_DIR_ENV).is_some() { authored } else { (authored.0 / UNSAVED_SHRINK, authored.1 / UNSAVED_SHRINK) };

    let src = rbms_parser::parse_with(rows_on_every_beat().as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "rows.bms"));
    let run = PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());
    app.switch(Transition::To(Stage::Play(Box::new(run))));
    assert_eq!(app.stage.id(), StageId::Play);

    let mut playing = Playing { app, pixels: HeadlessCanvas::new(size.0, size.1), clocks: Clocks::default(), size, now: Instant::now() };
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !playing.app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS) {
        playing.frame_after(0, true);
        assert_eq!(playing.app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS), None, "the default pack's play skin could not be read");
        assert!(Instant::now() < deadline, "the default pack's play skin was not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs());
    }
    playing.frame_after(0, true);
    let skin = playing.app.shared.skins.document(SKIN_TYPE_PLAY_7KEYS).expect("a compiled screen has its document");
    let (loadend_ms, playstart_ms) = (i64::from(skin.play.loadend), i64::from(skin.play.playstart));
    playing.frame_after(loadend_ms + SLACK_MS, true);
    assert_eq!(play(&playing.app).phase(), Some(PlayPhase::Ready));
    playing.frame_after(playstart_ms + SLACK_MS, true);
    assert_eq!(play(&playing.app).phase(), Some(PlayPhase::Play));
    playing.walk_to(SHOWN_MS);
    playing
}

/// With START down the turntable moves the lane cover a step a tick while the green number stands,
/// with SELECT down as well it moves the lift, and the figures of the shades are up while either is
/// held and for a moment after both are let go.
#[test]
fn the_default_pack_shows_the_cover_and_the_lift_the_turntable_moves() {
    let mut playing = playing_by_hand("turntable");
    let before = playing.figures();
    assert_eq!((before.white, before.lift, before.changing), (COVER_AT as i32, LIFT_AT as i32, false));
    let unmoved = playing.save("turntable-0-before-start");

    playing.app.shared.note_key(&press(START_KEY));
    playing.frame_after(WALK_STEP_MS, true);
    playing.frame_after(WALK_STEP_MS, true);
    playing.turn(TURNED_TICKS);
    let covered = playing.figures();
    assert_eq!((covered.white, covered.lift, covered.changing), (COVER_AT as i32 + TURNED_TICKS, LIFT_AT as i32, true));
    assert!((covered.green - before.green).abs() <= 1, "the turntable moved the green number from {} to {}", before.green, covered.green);
    assert_eq!(number_steps(&playing.app.shared.config, SettingId::LaneCover), Some(COVER_AT + i64::from(TURNED_TICKS)));
    assert_ne!(playing.save("turntable-1-start-held-cover-moved"), unmoved, "the frame is the one drawn before the cover moved");

    playing.app.shared.note_key(&press(SELECT_KEY));
    playing.frame_after(WALK_STEP_MS, true);
    playing.turn(TURNED_TICKS);
    let lifted = playing.figures();
    assert_eq!((lifted.white, lifted.lift, lifted.changing), (covered.white, LIFT_AT as i32 - TURNED_TICKS, true));
    assert_eq!(lifted.green, covered.green, "the lift moved the green number");
    playing.save("turntable-2-start-and-select-held-lift-moved");

    playing.app.shared.release_held_inputs();
    playing.frame_after(WALK_STEP_MS, true);
    assert!(playing.figures().changing, "the figures of the shades went the moment the keys were let go");
    playing.save("turntable-3-just-after-letting-go");
    playing.frame_after(2_000, true);
    assert!(!playing.figures().changing, "the figures of the shades stayed up two seconds after the keys were let go");
}

/// A lift that is switched off moves with START, SELECT and the turntable as one that is on does,
/// and stays off. Its figure is up at the foot of the field while it moves, beside the lane cover's,
/// so each turn changes what is drawn there: a shade that is off has nothing on the field to show
/// that it moved.
#[test]
fn the_default_pack_shows_a_lift_that_is_off_while_the_turntable_moves_it() {
    let mut playing = playing_by_hand_with_the_lift("lift-off", false);
    playing.save("turntable-4-lift-off-before");
    playing.app.shared.note_key(&press(START_KEY));
    playing.app.shared.note_key(&press(SELECT_KEY));
    playing.frame_after(WALK_STEP_MS, true);
    playing.frame_after(WALK_STEP_MS, true);

    playing.turn(TURNED_TICKS);
    let moved = playing.figures();
    assert_eq!((moved.white, moved.lift, moved.changing), (COVER_AT as i32, LIFT_AT as i32 - TURNED_TICKS, true));
    assert!(!playing.app.shared.config.play.enable_lift, "the turntable switched the lift on");
    let once = playing.foot();
    playing.save("turntable-5-lift-off-moved");

    playing.turn(TURNED_TICKS);
    assert_eq!(playing.figures().lift, LIFT_AT as i32 - TURNED_TICKS * 2);
    assert!(playing.foot() != once, "nothing at the foot of the field changed when a lift that is off was moved again");
}

/// The judge offset moves from the key it ships on while the chart plays, and the figure a skin
/// shows for it is the new one on the next frame.
#[test]
fn the_default_pack_shows_the_judge_offset_its_key_moved() {
    let mut playing = playing_by_hand("offset");
    assert_eq!(playing.figures().timing, 0);
    let unmoved = playing.save("offset-0-before-the-key");
    for _ in 0..OFFSET_PRESSES {
        playing.key(OFFSET_UP_KEY);
    }
    assert_eq!(playing.figures().timing, OFFSET_PRESSES);
    assert_eq!(playing.app.shared.config.judge.offset_ms, OFFSET_PRESSES);
    assert_ne!(playing.save("offset-1-after-three-presses"), unmoved, "the frame is the one drawn before the key");
}
