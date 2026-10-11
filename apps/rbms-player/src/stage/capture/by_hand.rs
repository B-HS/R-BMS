//! Play scenes drawn for a run somebody plays by hand, so what a skin says about a press that came
//! early or late can be looked at and held to.
//!
//! The captures in the parent module leave a chart to the engine, which takes every note on the
//! beat: nothing a skin draws for a press that was off shows there. Here the chart is a handful of
//! notes a measure apart, and the harness plays each one itself, a set time ahead of the note or
//! behind it ([`Stroke`]). A press is handed to the run with the moment of the chart it is made at,
//! which is what the play screen does with a key once it has read the song clock for it. The clock
//! is left out on purpose: it is a real one, and a press read off it would be judged by how long the
//! machine took to get from one line to the next. So the judgement a stroke takes is decided by the
//! time asked for and by nothing else, and the screen hears of it on its next frame the way it
//! hears of a note the engine took.
//!
//! Two tests hold the default pack, which is in the repository, to telling the player which way a
//! press was off: its field is drawn differently for a press that came early than for one that came
//! as late, on one field of a single run and on the field the press was made on of a double one. One
//! more draws the pack [`SKIN_PACK_ENV`] names at full size for a person to look at, as it comes and
//! with the rows that turn its timing words and its timing figures on.
//!
//! What a run by hand is made of -- running the scene on to a moment of the chart, a key going down
//! and coming up at one -- is here once, and the sibling module that sets a pack's field up before
//! playing it by hand uses the same.

use super::*;

/// The size the frames the default pack is held to are drawn at, 16 to 9 like the pack and half of
/// its size: small enough to draw quickly, large enough for the words over the field to be told apart.
pub(super) const HELD_SIZE: (u32, u32) = (960, 540);

/// The tempo of the chart played by hand and how long one of its measures lasts at it.
const HAND_BPM: u32 = 120;
const HAND_MEASURE_MS: i64 = 2_000;

/// The measure of the first note, which leaves the field time to empty of nothing before it.
const HAND_FIRST_MEASURE: u32 = 1;

/// A note far past the ones that are played, so the run is still playing when its last frame is drawn.
const HAND_LAST_MEASURE: u32 = 40;

/// How long a key stays down, in milliseconds.
pub(super) const HAND_HOLD_MS: i64 = 60;

/// How long after a note its frame is drawn: past everything a skin plays on the lane for a press
/// (its flash and its key beam) and inside the time its judgement stays up.
const HAND_SHOWN_AFTER_MS: i64 = 350;

/// How far apart the two frames of one stroke are drawn. A skin whose judgement blinks has its word
/// on one of the two; one that holds its judgement still, as the default pack does, draws the two
/// alike.
const HAND_BLINK_APART_MS: i64 = 40;

/// How far off the beat a press takes a GREAT and a GOOD, in milliseconds: inside the second and the
/// third of the widest and the narrowest sets of windows a seven-key chart is judged with.
pub(super) const GREAT_OFF_MS: i64 = 40;
pub(super) const GOOD_OFF_MS: i64 = 100;

/// The channels the first key of each side is written on.
pub(super) const FIRST_SIDE_KEY: &str = "11";
pub(super) const SECOND_SIDE_KEY: &str = "21";

/// The judgements of a run, in the order the engine counts them.
const GREAT: usize = 1;

/// One note of a chart played by hand: what the frame of it is called, and how many milliseconds
/// ahead of the note its key goes down. A press behind the note has a negative lead.
#[derive(Clone, Copy)]
struct Stroke {
    name: &'static str,
    lead_ms: i64,
}

const EARLY_GREAT: Stroke = Stroke { name: "great-fast", lead_ms: GREAT_OFF_MS };
const LATE_GREAT: Stroke = Stroke { name: "great-slow", lead_ms: -GREAT_OFF_MS };

/// The strokes of a captured run: a GREAT and a GOOD each way, and one on the beat.
const CAPTURED_STROKES: [Stroke; 5] = [
    EARLY_GREAT,
    LATE_GREAT,
    Stroke { name: "good-fast", lead_ms: GOOD_OFF_MS },
    Stroke { name: "good-slow", lead_ms: -GOOD_OFF_MS },
    Stroke { name: "on-the-beat", lead_ms: 0 },
];

/// A chart with one note a measure on `channel`, as many as `strokes`, and one far behind them.
fn hand_chart(title: &str, channel: &str, strokes: usize) -> String {
    let mut chart = format!(
        "#PLAYER 1\n#GENRE Capture Genre\n#TITLE {title}\n#ARTIST Capture Artist\n#BPM {HAND_BPM}\n#PLAYLEVEL 3\n#DIFFICULTY 2\n#RANK 3\n#TOTAL 300\n#WAV01 a.wav\n"
    );
    for measure in (HAND_FIRST_MEASURE..).take(strokes).chain([HAND_LAST_MEASURE]) {
        chart.push_str(&format!("#{measure:03}{channel}:01\n"));
    }
    chart
}

/// When the note of the stroke at `index` is, in milliseconds of the chart.
fn note_ms(index: usize) -> i64 {
    (i64::from(HAND_FIRST_MEASURE) + index as i64) * HAND_MEASURE_MS
}

/// The lane the chart's first note is on.
pub(super) fn played_lane(model: &rbms_model::Model) -> Result<usize, String> {
    model.timelines.iter().find_map(|timeline| timeline.notes.iter().position(Option::is_some)).ok_or_else(|| "the chart played by hand has no note".to_owned())
}

/// The run of the play screen that is up, to hand a press to.
fn run_of(app: &mut App) -> Result<&mut PlaySession, String> {
    match &mut app.stage {
        Stage::Play(state) => Ok(&mut state.session),
        other => Err(format!("the {other:?} screen is up, not the play screen")),
    }
}

/// Run the play screen on to `to_ms` of the chart without drawing it, in steps of [`PLAY_STEP_MS`]
/// and a last one that lands on the moment. Where the chart stands is the harness's own count, not
/// what a frame read off the clock.
pub(super) fn run_to(app: &mut App, clocks: &mut PlayClocks, to_ms: i64) -> Result<(), String> {
    loop {
        let at_ms = clocks.song.map_or(0, |song| song.as_millis() as i64);
        let left = to_ms - at_ms;
        if left <= PLAY_STEP_MS {
            return clocks.frame_after(app, left.max(0), None);
        }
        clocks.frame_after(app, PLAY_STEP_MS, None)?;
    }
}

/// Put the key of `lane` down at `down_ms` of the chart. A press is handed to the run with the
/// moment of the chart it is made at, so the judgement it takes is decided by that moment and by
/// nothing a clock says.
pub(super) fn press(app: &mut App, clocks: &mut PlayClocks, lane: usize, down_ms: i64) -> Result<(), String> {
    run_to(app, clocks, down_ms)?;
    run_of(app)?.press(lane, down_ms * MICROS_PER_MILLI, &mut NullSink);
    Ok(())
}

/// Let the key of `lane` that went down at `down_ms` up again, [`HAND_HOLD_MS`] later.
pub(super) fn release(app: &mut App, clocks: &mut PlayClocks, lane: usize, down_ms: i64) -> Result<(), String> {
    let up_ms = down_ms + HAND_HOLD_MS;
    run_to(app, clocks, up_ms)?;
    run_of(app)?.release(lane, up_ms * MICROS_PER_MILLI);
    Ok(())
}

/// Play one stroke of the note at `at_ms` on `lane`: down its lead ahead of the note, up
/// [`HAND_HOLD_MS`] later.
fn strike(app: &mut App, clocks: &mut PlayClocks, lane: usize, at_ms: i64, stroke: Stroke) -> Result<(), String> {
    let down_ms = at_ms - stroke.lead_ms;
    press(app, clocks, lane, down_ms)?;
    release(app, clocks, lane, down_ms)
}

/// A pack's play scene entered for a chart played by hand, walked until the chart plays.
struct HandRun {
    app: App,
    size: (u32, u32),
    clocks: PlayClocks,
    lane: usize,
    pixels: HeadlessCanvas,
    _keysounds: std::sync::mpsc::Sender<(u32, rbms_audio::DecodedAudio)>,
}

/// What a run by hand is entered with: the settings folder of its own app, the channel its notes are
/// on and the mode that channel is read in, how many notes it has, the size it is drawn at (the
/// pack's own when `None`), and the rows of the pack's settings that are switched before the pack's
/// document is read, each to the item whose name starts as said.
struct HandSetup<'a> {
    tag: &'a str,
    channel: &'a str,
    mode: rbms_model::Mode,
    strokes: usize,
    size: Option<(u32, u32)>,
    choices: &'a [(&'a str, &'a str)],
}

impl HandRun {
    /// Enter the play scene of `pack` for a chart played by hand and walk it until the chart plays.
    /// Answers why the scene cannot be entered when the pack has no document for the mode.
    fn enter(pack: &Path, setup: &HandSetup<'_>) -> Result<HandRun, String> {
        let name = format!("{}.bms", setup.tag);
        let model = chart_model(&hand_chart("Played By Hand", setup.channel, setup.strokes), &name, Some(setup.mode));
        let chart = settings_of(&format!("{}-chart", setup.tag)).with_file_name(&name);
        let EnteredPlay { mut app, screen, size, play_time_ms: _, arrived, _keysounds } =
            enter_play_scene(pack, setup.tag, &chart, model.clone(), PlayEnding::Failure)?;
        for (row, item) in setup.choices {
            choose_in_skin(&mut app, screen, row, item);
        }
        app.shared.skins.request_for(&app.shared.config, screen);
        let lane = played_lane(&model)?;
        let size = setup.size.unwrap_or(size);
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        wait_for_scene_document(&mut app, screen, &mut Canvas::Headless(&mut pixels))?;
        let skin = app.shared.skins.document(screen).ok_or_else(|| "the skin was let go of".to_owned())?;
        let timings = (i64::from(skin.play.loadend), i64::from(skin.play.playstart));
        let mut clocks = PlayClocks::default();
        clocks.open(&mut app, &mut pixels, &arrived, timings, |_, _, _, _| Ok(()))?;
        Ok(HandRun { app, size, clocks, lane, pixels, _keysounds })
    }

    /// Play the note at `index` with `stroke` and draw the frame `after_ms` past the note.
    fn play(&mut self, index: usize, stroke: Stroke, after_ms: i64) -> Result<(), String> {
        strike(&mut self.app, &mut self.clocks, self.lane, note_ms(index), stroke)?;
        run_to(&mut self.app, &mut self.clocks, note_ms(index) + after_ms)?;
        self.clocks.frame_after(&mut self.app, 0, Some(&mut Canvas::Headless(&mut self.pixels)))
    }

    /// How many of each judgement the run has taken.
    fn counts(&self) -> Result<[u32; 6], String> {
        Ok(play_screen(&self.app)?.session.judge().counts)
    }
}

/// The frame the default pack draws [`HAND_SHOWN_AFTER_MS`] after one note of a chart on `channel`
/// was taken as a GREAT with `stroke`, and the columns of the frame each field of the pack's
/// document takes up, first side first.
fn held_frame(label: &str, channel: &str, mode: rbms_model::Mode, stroke: Stroke) -> (Vec<u8>, Vec<Range<u32>>) {
    let tag = format!("held-{label}-{}", stroke.name);
    let setup = HandSetup { tag: &tag, channel, mode, strokes: 1, size: Some(HELD_SIZE), choices: &[] };
    let mut run = HandRun::enter(&default_pack(), &setup).unwrap_or_else(|reason| panic!("{label}: not entered: {reason}"));
    run.play(0, stroke, HAND_SHOWN_AFTER_MS).unwrap_or_else(|reason| panic!("{label}: not played: {reason}"));
    let counts = run.counts().unwrap_or_else(|reason| panic!("{label}: {reason}"));
    assert_eq!(counts[GREAT], 1, "{label}: a press {} ms ahead of its note was not taken as a GREAT: {counts:?}", stroke.lead_ms);
    save(&format!("held-{label}-{}", stroke.name), CAPTURE_EXTENSION, run.size, run.pixels.rgba());
    let screen = rbms_skin::loader::mode_skin_type(mode).expect("the mode has a play screen");
    let skin = run.app.shared.skins.document(screen).expect("a compiled screen has its document");
    let lanes = &skin.def.note.as_ref().expect("a play document has a note field").dst;
    let sides = if mode.player > 1 { 2 } else { 1 };
    let across = run.size.0 as f32 / skin.def.w.max(1) as f32;
    let fields = lanes
        .chunks(lanes.len().div_ceil(sides).max(1))
        .map(|side| {
            let left = side.iter().filter_map(|lane| lane.x).min().unwrap_or_default();
            let right = side.iter().filter_map(|lane| Some(lane.x? + lane.w?)).max().unwrap_or_default();
            (left as f32 * across).ceil() as u32..(right as f32 * across).floor() as u32
        })
        .collect();
    (run.pixels.rgba().to_vec(), fields)
}

/// How many pixels of `columns` two frames of [`HELD_SIZE`] differ in.
fn differing_in(one: &[u8], other: &[u8], columns: &Range<u32>) -> usize {
    const CHANNELS: usize = 4;
    let row = HELD_SIZE.0 as usize * CHANNELS;
    one.chunks(row)
        .zip(other.chunks(row))
        .map(|(a, b)| {
            let cut = columns.start as usize * CHANNELS..columns.end as usize * CHANNELS;
            a[cut.clone()].chunks(CHANNELS).zip(b[cut].chunks(CHANNELS)).filter(|(a, b)| a != b).count()
        })
        .sum()
}

/// A GREAT that came early and one that came as late are drawn apart on the field of a single run:
/// the pack says which way the press was off without being asked to.
#[test]
fn the_default_pack_draws_an_early_press_and_a_late_one_apart_on_the_field() {
    let (early, fields) = held_frame("single", FIRST_SIDE_KEY, rbms_model::Mode::BEAT_7K, EARLY_GREAT);
    let (late, _) = held_frame("single", FIRST_SIDE_KEY, rbms_model::Mode::BEAT_7K, LATE_GREAT);
    let apart = differing_in(&early, &late, &fields[0]);
    assert!(apart > 0, "the field of the default pack is drawn alike for a GREAT that came early and one that came late");
}

/// On a double run the press is told apart on the field it was made on and on no other.
#[test]
fn the_default_pack_draws_an_early_press_and_a_late_one_apart_on_the_field_they_were_made_on() {
    let (early, fields) = held_frame("double", SECOND_SIDE_KEY, rbms_model::Mode::BEAT_14K, EARLY_GREAT);
    let (late, _) = held_frame("double", SECOND_SIDE_KEY, rbms_model::Mode::BEAT_14K, LATE_GREAT);
    assert_eq!(fields.len(), 2, "a double document has two fields");
    assert_eq!(differing_in(&early, &late, &fields[0]), 0, "a press on the second side changed what the first side's field says");
    assert!(differing_in(&early, &late, &fields[1]) > 0, "the second side's field is drawn alike for a GREAT that came early and one that came late");
}

/// The rows of a pack's settings that turn on what it draws for a press that was off, by the name of
/// the capture: as the pack comes, with its words, and with its figures. A row a pack does not offer
/// is passed over, so the rows of every pack these captures are taken of are listed together.
const HAND_CHOICES: [(&str, &[(&str, &str)]); 3] = [
    ("default", &[]),
    ("words", &[("Judge Detail", "Fast-Slow"), ("判定タイミング", "表示"), ("判定タイミングの種類", "FAST/SLOW")]),
    ("figures", &[("Judge Detail", "Milliseconds"), ("判定タイミング", "表示"), ("判定タイミングの種類", "+-ms")]),
];

/// The runs by hand that are captured: seven keys on the first side, and fourteen with the second
/// side's key.
const HAND_MODES: [(&str, &str, rbms_model::Mode); 2] =
    [("play7-hand", FIRST_SIDE_KEY, rbms_model::Mode::BEAT_7K), ("play14-hand", SECOND_SIDE_KEY, rbms_model::Mode::BEAT_14K)];

/// Play [`CAPTURED_STROKES`] on the pack's play scene and save two frames of each, a blink apart, as
/// `<prefix>-<choice>-<stroke>-a` and `-b`. Answers what was saved with what the run had counted.
fn capture_by_hand(pack: &Path, prefix: &str, channel: &str, mode: rbms_model::Mode, (choice, rows): (&str, &[(&str, &str)])) -> Result<Vec<String>, String> {
    let tag = format!("{prefix}-{choice}");
    let setup = HandSetup { tag: &tag, channel, mode, strokes: CAPTURED_STROKES.len(), size: None, choices: rows };
    let mut run = HandRun::enter(pack, &setup)?;
    let mut saved = Vec::new();
    for (index, stroke) in CAPTURED_STROKES.into_iter().enumerate() {
        run.play(index, stroke, HAND_SHOWN_AFTER_MS)?;
        for (frame, later_ms) in [("a", 0), ("b", HAND_BLINK_APART_MS)] {
            if later_ms > 0 {
                run.clocks.frame_after(&mut run.app, later_ms, Some(&mut Canvas::Headless(&mut run.pixels)))?;
            }
            let name = format!("{tag}-{}-{frame}", stroke.name);
            save(&name, CAPTURE_EXTENSION, run.size, run.pixels.rgba());
            saved.push(format!("{name}: PG/GR/GD/BD/PR/MS {:?}", run.counts()?));
        }
    }
    Ok(saved)
}

/// Captures the play scene of a pack somebody else wrote for runs played by hand, with presses that
/// come early and late, as the pack comes and with its timing words and figures switched on.
///
/// Opt-in like the captures of the parent module: without [`SKIN_PACK_ENV`] this passes without
/// drawing anything. With it, the pack's folder has to be left as it was.
#[test]
fn the_play_scene_of_a_skin_pack_named_by_the_environment_is_captured_as_it_is_played_by_hand() {
    let Some(pack) = crate::skin_select::pack_from_environment(std::env::var_os(SKIN_PACK_ENV)) else {
        return;
    };
    assert!(pack.is_dir(), "{SKIN_PACK_ENV} names {}, which is not a folder", pack.display());
    let before = files_under(&pack);
    for (prefix, channel, mode) in HAND_MODES {
        for choice in HAND_CHOICES {
            for line in drawn_or_passed_over(prefix, capture_by_hand(&pack, prefix, channel, mode, choice)).into_iter().flatten() {
                println!("{line}");
            }
        }
    }
    assert_eq!(files_under(&pack), before, "capturing the pack's play scene by hand changed its folder");
}
