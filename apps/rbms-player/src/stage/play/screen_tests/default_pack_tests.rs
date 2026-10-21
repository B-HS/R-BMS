//! What a run holds for its player, on the default skin pack and on runs the play screen is really
//! entered for.
//!
//! A held green number: a chart at one tempo, the same chart with its lane cover moved, a chart at
//! another tempo, and that chart again on the pack's wide lanes, with and without its score graph and
//! with the offset set to adjust itself, each drawn once the chart is playing. A judge offset that adjusts itself: a run
//! played by hand a steady forty milliseconds early, drawn once the offset has moved.
//!
//! What is held to here is the run itself: the travel time the two charts are played at is the one
//! the settings hold and the speed is what differs, and the offset is where the reference's rule
//! puts it. What the pack makes of them -- its green number, its white number, its speed figure and
//! its timing figure -- is for a person to look at: with [`CAPTURE_DIR_ENV`] naming a folder each
//! frame is saved there as a PNG, and without it nothing is written.

use std::path::Path;

use rbms_config::{green_of_duration, travel_duration_ms};

use super::*;

/// Names the folder the frames are saved in, as it does for the captures of the other screens.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The default pack, relative to this crate.
const DEFAULT_PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// How many times smaller than the pack's own size a frame nobody saves is drawn.
const UNSAVED_SHRINK: u32 = 3;

/// The seed a pack's random picks are pinned with, so two captures of one frame are alike.
const PACK_SEED: u64 = 1;

/// How long the pack's play document is waited for.
const PACK_LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// The two tempos, and the cover the runs begin under and the one it is moved to.
const SLOW_BPM: u32 = 150;
const FAST_BPM: u32 = 200;
const COVER: f32 = 0.2;
const COVER_STEPS_UP: usize = 15;

/// The green number the runs hold.
const HELD_GREEN: i32 = 300;

/// How far into the chart a frame is drawn, in milliseconds: far enough for the field to be full.
const INTO_THE_CHART_MS: i64 = 4_000;

/// How many measures of notes the charts have, which is longer than any frame here is drawn at.
const MEASURES: u32 = 24;

/// The rows of the pack's play document the last chart is drawn under once more, by the name each
/// frame is saved under: wide lanes, and wide lanes without the score graph, which is where the
/// boxes beside the field are narrowest.
const WIDE_LANES: (&str, &str) = ("Lane Width", "Wide");
const WITHOUT_SCORE_GRAPH: (&str, &str) = ("Score Graph", "Off");
const WIDE_SHOTS: [(&str, &[(&str, &str)]); 2] =
    [("held-green-200bpm-wide-lanes", &[WIDE_LANES]), ("held-green-200bpm-wide-lanes-no-graph", &[WIDE_LANES, WITHOUT_SCORE_GRAPH])];

/// How far apart two travel times may read and still be the one, in milliseconds.
const TRAVEL_ROUNDING_MS: i32 = 1;

/// A seven-key chart at `bpm` with a stair of notes in every measure.
fn stairs(bpm: u32) -> String {
    let mut chart = format!(
        "#PLAYER 1\n#GENRE Green Number\n#TITLE Held At {bpm} BPM\n#ARTIST Capture\n#BPM {bpm}\n#PLAYLEVEL 5\n#DIFFICULTY 2\n#RANK 3\n#TOTAL 300\n#WAV01 a.wav\n"
    );
    for measure in 1..=MEASURES {
        for (channel, notes) in [("11", "01000000"), ("12", "00010000"), ("13", "00000100"), ("14", "00000001"), ("15", "01000100"), ("16", "00010001")] {
            chart.push_str(&format!("#{measure:03}{channel}:{notes}\n"));
        }
    }
    chart
}

/// A run of [`stairs`] at `bpm` that the game plays.
fn stair_run(bpm: u32) -> PlayState {
    let src = rbms_parser::parse_with(stairs(bpm).as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "stairs.bms"));
    PlayState::new(
        PlaySession::new(model, SessionOptions { autoplay: true, ..SessionOptions::default() }),
        std::collections::HashMap::new(),
        0,
        SCORE_LN_MODE_FROM_CHART.to_string(),
    )
}

/// The settings of a player who holds [`HELD_GREEN`] under a lane cover and lets the game play.
fn holding_a_green_number() -> Config {
    let mut config = Config::default();
    config.play.autoplay = true;
    config.play.keep_green_number = true;
    config.play.duration_ms = rbms_config::duration_of_green(HELD_GREEN);
    config.play.enable_cover = true;
    config.play.cover = COVER;
    config
}

/// An app drawn by the default pack, on `config`.
fn app_on_the_default_pack(tag: &str, config: Config) -> App {
    app_on_the_pack(tag, config, &Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER))
}

/// An app drawn by the pack in `folder`, on `config`.
fn app_on_the_pack(tag: &str, mut config: Config, folder: &Path) -> App {
    CLOCKS.set(Clocks::UNSTARTED);
    let settings = settings_of(&format!("play-{tag}"));
    let pack = rbms_skin::resolve::plain_path(&folder.canonicalize().unwrap_or_else(|error| panic!("{} cannot be resolved: {error}", folder.display())));
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.shared.mode = Mode::BEAT_7K;
    app.shared.active_keys = app.shared.keyconfig.lane_keys(app.shared.mode);
    app.shared.active_reverse_keys = app.shared.keyconfig.scratch_reverse_keys(app.shared.mode);
    app.shared.replay = None;
    app
}

/// Enter `run` and walk its scene on the pack's own times to the frame the chart starts playing on.
fn start_on_the_pack(app: &mut App, pixels: &mut HeadlessCanvas, run: PlayState) {
    CLOCKS.set(Clocks::UNSTARTED);
    enter(app, run);
    let deadline = Instant::now() + PACK_LOAD_TIMEOUT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_PLAY_7KEYS) {
        stay(app, pixels);
        assert_eq!(app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS), None, "the default pack's play skin could not be read");
        assert!(Instant::now() < deadline, "the default pack's play skin was not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs());
    }
    stay(app, pixels);
    let skin = app.shared.skins.document(SKIN_TYPE_PLAY_7KEYS).expect("a compiled screen has its document");
    let (loadend_ms, playstart_ms) = (i64::from(skin.play.loadend), i64::from(skin.play.playstart));
    stay_after(app, pixels, loadend_ms + SLACK_MS);
    assert_eq!(phase(app), Some(PlayPhase::Ready));
    stay_after(app, pixels, playstart_ms + SLACK_MS);
    assert_eq!(phase(app), Some(PlayPhase::Play));
}

/// Enter `run` and walk its scene until the chart has played for [`INTO_THE_CHART_MS`], with the
/// last frame drawn on `pixels`.
fn play_into(app: &mut App, pixels: &mut HeadlessCanvas, run: PlayState) {
    start_on_the_pack(app, pixels, run);
    walk_to(app, pixels, INTO_THE_CHART_MS);
    stay(app, pixels);
}

/// Walk the run that is playing on to `chart_ms` of its chart, by the test's own count of the song
/// clock, and draw the frame there.
fn walk_to(app: &mut App, pixels: &mut HeadlessCanvas, chart_ms: i64) {
    let at_ms = CLOCKS.get().song.map_or(0, |song| i64::try_from(song.as_millis()).expect("a short chart"));
    stay_after(app, pixels, (chart_ms - at_ms).max(0));
}

/// How long a note is on show in the run that is up, at the tempo `bpm`, in milliseconds.
fn travel_ms(app: &App, bpm: u32) -> i32 {
    let play = &app.shared.config.play;
    travel_duration_ms(f64::from(bpm), play.hispeed, play.enable_cover.then_some(play.cover))
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

/// The size the frames are drawn at: the one the pack's seven-key play document was authored at
/// when they are being saved for a person to look at, and a third of it otherwise, which is all a
/// run that is only held to its numbers needs and a ninth of the pixels to fill on every frame.
fn frame_size(app: &App) -> (u32, u32) {
    let header = app.shared.skins.header_of(&app.shared.config, SKIN_TYPE_PLAY_7KEYS).expect("the default pack has a seven-key play skin");
    let authored = (
        u32::try_from(header.width).ok().filter(|width| *width > 0).unwrap_or(CW),
        u32::try_from(header.height).ok().filter(|height| *height > 0).unwrap_or(CH),
    );
    if std::env::var_os(CAPTURE_DIR_ENV).is_some() { authored } else { (authored.0 / UNSAVED_SHRINK, authored.1 / UNSAVED_SHRINK) }
}

/// A chart at 150 and a chart at 200 are played at the one travel time the settings hold, under
/// the cover as it began and as it was moved in the run, and the speed is what differs each time.
/// The second chart is entered on an app of its own, with the play settings the first run left: a
/// Lua skin is read afresh for every chart, and an app that has drawn one run already says its
/// play skin is compiled before the next one's has been read.
#[test]
fn the_default_pack_is_drawn_for_a_held_green_number_at_two_tempos_and_under_a_moved_cover() {
    let mut app = app_on_the_default_pack("held-green", holding_a_green_number());
    let size = frame_size(&app);
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let held_ms = app.shared.config.play.duration_ms;
    assert_eq!(green_of_duration(held_ms), HELD_GREEN);

    play_into(&mut app, &mut pixels, stair_run(SLOW_BPM));
    let slow_speed = app.shared.config.play.hispeed;
    assert_eq!(travel_ms(&app, SLOW_BPM), held_ms);
    save("held-green-150bpm", &pixels, size);

    let cover_up = app.shared.control_key_in_force(ControlAction::CoverUp).expect("the cover has a key that raises it");
    for _ in 0..COVER_STEPS_UP {
        key(&mut app, press(cover_up));
        key(&mut app, release(cover_up));
    }
    stay(&mut app, &mut pixels);
    let covered_speed = app.shared.config.play.hispeed;
    assert!(app.shared.config.play.cover > COVER, "the bound key did not move the lane cover");
    assert!((travel_ms(&app, SLOW_BPM) - held_ms).abs() <= TRAVEL_ROUNDING_MS, "moving the cover moved the travel time to {}", travel_ms(&app, SLOW_BPM));
    assert!(covered_speed < slow_speed, "more cover with the same number is a slower scroll");
    save("held-green-150bpm-cover-moved", &pixels, size);

    let mut left = app.shared.config.clone();
    left.play.cover = COVER;
    let mut app = app_on_the_default_pack("held-green-next-chart", left);
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    play_into(&mut app, &mut pixels, stair_run(FAST_BPM));
    let fast_speed = app.shared.config.play.hispeed;
    assert!((travel_ms(&app, FAST_BPM) - held_ms).abs() <= TRAVEL_ROUNDING_MS, "a chart at {FAST_BPM} is played at {} ms", travel_ms(&app, FAST_BPM));
    assert!(fast_speed < slow_speed, "a faster chart with the same number is a lower speed: {fast_speed} against {slow_speed}");
    save("held-green-200bpm", &pixels, size);

    let mut widened = app.shared.config.clone();
    widened.judge.auto_offset = true;
    for (name, rows) in WIDE_SHOTS {
        let mut app = app_on_the_default_pack(name, widened.clone());
        for (row, item) in rows {
            assert!(crate::stage::capture::choose_in_skin(&mut app, SKIN_TYPE_PLAY_7KEYS, row, item), "the pack's play document has no {row} of {item}");
        }
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        play_into(&mut app, &mut pixels, stair_run(FAST_BPM));
        assert!((travel_ms(&app, FAST_BPM) - held_ms).abs() <= TRAVEL_ROUNDING_MS, "{name}: the travel time is {} ms", travel_ms(&app, FAST_BPM));
        assert!((app.shared.config.play.hispeed - fast_speed).abs() < f64::EPSILON, "{name}: the rows of the skin moved the speed");
        save(name, &pixels, size);
    }
}

/// How far ahead of each note the run by hand presses, how long it holds, and how many notes it
/// plays, in milliseconds and notes.
const EARLY_BY_MS: i64 = 40;
const HELD_FOR_MS: i64 = 30;
const NOTES_PLAYED: i32 = 6;

/// How long after the last press the frame is drawn, in milliseconds: inside the time a pack keeps
/// a judgement up.
const SHOWN_AFTER_MS: i64 = 300;

/// The first key lane, and how long one measure of [`one_key_chart`] lasts at [`SLOW_BPM`].
const FIRST_LANE: usize = 0;
const MEASURE_MS: i64 = 1_600;

/// Far enough ahead that the wall clock never catches a song clock the test has set by hand.
const CLOCK_HELD_BACK: Duration = Duration::from_secs(3_600);

/// A seven-key chart at [`SLOW_BPM`] with one note of its first key at the head of every measure.
fn one_key_chart() -> String {
    let mut chart = format!(
        "#PLAYER 1\n#GENRE Judge Timing\n#TITLE Forty Early\n#ARTIST Capture\n#BPM {SLOW_BPM}\n#PLAYLEVEL 1\n#DIFFICULTY 2\n#RANK 3\n#TOTAL 300\n#WAV01 a.wav\n"
    );
    for measure in 1..=MEASURES {
        chart.push_str(&format!("#{measure:03}11:01\n"));
    }
    chart
}

/// Send the first lane's key to the run with the song clock put at `chart_ms` and held there, so
/// the input is read at that moment of the chart however long the machine takes to get to it.
fn first_lane_key_at(app: &mut App, chart_ms: i64, pressed: bool) {
    let lane_key = app.shared.active_keys.iter().find(|(_, lane)| *lane == FIRST_LANE).map(|(code, _)| *code).expect("the first lane has a key");
    app.shared.clock = Instant::now() + CLOCK_HELD_BACK;
    app.shared.song_us_last.set(chart_ms * MICROS_PER_MILLI);
    key(app, if pressed { press(lane_key) } else { release(lane_key) });
}

/// A player who presses every note forty milliseconds early, with AUTO ADJUST on, has the judge
/// offset taken down a millisecond by every press (`JudgeManager.java:697-703`), which draws the
/// notes that much later, the settings hold it as it moves, and the pack's timing figure follows it.
#[test]
fn the_default_pack_is_drawn_for_a_judge_offset_that_adjusted_itself() {
    let mut config = Config::default();
    config.play.autoplay = false;
    config.judge.auto_offset = true;
    let mut app = app_on_the_default_pack("adjusted-offset", config);
    let size = frame_size(&app);
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let src = rbms_parser::parse_with(one_key_chart().as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "early.bms"));
    let run = PlayState::new(PlaySession::new(model, SessionOptions::default()), std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string());
    start_on_the_pack(&mut app, &mut pixels, run);

    for note in 1..=i64::from(NOTES_PLAYED) {
        let down_ms = note * MEASURE_MS - EARLY_BY_MS;
        walk_to(&mut app, &mut pixels, down_ms - SLACK_MS);
        first_lane_key_at(&mut app, down_ms, true);
        first_lane_key_at(&mut app, down_ms + HELD_FOR_MS, false);
        stay(&mut app, &mut pixels);
    }
    assert_eq!(play(&app).session.judge().total_judged(), u32::try_from(NOTES_PLAYED).expect("a small number"), "a press did not reach its note");
    assert_eq!(app.shared.config.judge.offset_ms, -NOTES_PLAYED, "every press forty milliseconds early is one millisecond off the offset");
    walk_to(&mut app, &mut pixels, i64::from(NOTES_PLAYED) * MEASURE_MS + SHOWN_AFTER_MS);
    stay(&mut app, &mut pixels);
    save("adjusted-offset-forty-early", &pixels, size);
}

/// What the target stands in for the frames below: the share of the chart's EX ceiling the stored
/// record holds, as a numerator over [`RECORD_SHARE_OF`].
const RECORD_SHARE: u32 = 3;
const RECORD_SHARE_OF: u32 = 5;

/// A record of `ex` on the chart `md5`, as a run by hand leaves one.
fn record_of(md5: &str, ex: u32, total_notes: u32) -> rbms_store::ScoreRecord {
    rbms_store::ScoreRecord {
        md5: md5.to_owned(),
        title: "record".to_owned(),
        mode: Mode::BEAT_7K.name.to_owned(),
        clear: 3,
        ex_score: ex,
        max_ex: total_notes * 2,
        counts: [0; 6],
        empty_poor: 0,
        max_combo: 0,
        total_notes,
        gauge: String::new(),
        gauge_value: 0.0,
        random: String::new(),
        played_at: 0,
        replay_file: None,
        rule_version: rbms_store::SCORE_RULE_VERSION,
        ln_mode: SCORE_LN_MODE_FROM_CHART.to_owned(),
        assisted: false,
        assist: 0,
        log_id: None,
    }
}

/// The score graph and the figure over the judgement of the default pack, for a player whose TARGET
/// is LOCAL BEST: on a chart they have a record of, the graph's TARGET bar is that record and the
/// legend reads LOCAL BEST; on a chart with none it is the fallback they named and the legend says
/// so. Each frame is drawn once the chart has played [`INTO_THE_CHART_MS`] of itself.
#[test]
fn the_default_pack_names_the_target_a_run_is_really_paced_against() {
    let mut config = holding_a_green_number();
    config.judge.target = rbms_config::ScoreTarget::LocalBest;
    config.play.keep_green_number = false;

    let mut with_record = app_on_the_default_pack("target-record", config.clone());
    let size = frame_size(&with_record);
    let run = stair_run(SLOW_BPM);
    let total = run.session.judge().total_notes();
    let best = total * 2 * RECORD_SHARE / RECORD_SHARE_OF;
    with_record.shared.scores.push(record_of(&run.session.model().md5, best, total));
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    play_into(&mut with_record, &mut pixels, run);
    let paced = play(&with_record).pace_target.clone().expect("the run settled its target");
    assert_eq!((paced.used, paced.name.as_str(), paced.ex), (rbms_config::ScoreTarget::LocalBest, "LOCAL BEST", best));
    save("target-play-local-best", &pixels, size);

    let mut without = app_on_the_default_pack("target-no-record", config);
    let mut pixels = HeadlessCanvas::new(size.0, size.1);
    let run = stair_run(SLOW_BPM);
    let total = run.session.judge().total_notes();
    play_into(&mut without, &mut pixels, run);
    let paced = play(&without).pace_target.clone().expect("the run settled its target");
    assert_eq!((paced.used, paced.name.as_str()), (rbms_config::ScoreTarget::RateAaa, "RATE AAA"), "the fallback is the one the player did not choose");
    assert_eq!(paced.ex, crate::target::band_target_ex(total, 24));
    save("target-play-no-record", &pixels, size);
}

/// Names a skin pack the SCRATCH AUTO frame is also drawn by, as it does for the other screens.
const SKIN_PACK_ENV: &str = "RBMS_SKIN_PACK";

/// The scratch lane of seven keys.
const TURNTABLE_LANE: usize = 7;

/// [`stairs`] at `bpm` with the sixth and seventh keys, which make it a seven-key chart, notes on
/// the turntable between the stairs and a long note on it every fourth measure.
fn stairs_with_a_turntable(bpm: u32) -> String {
    let mut chart = stairs(bpm).replace("#TITLE Held At", "#TITLE Scratch Auto At");
    for measure in 1..=MEASURES {
        let turntable = if measure % 4 == 0 { ("56", "0100000000000001") } else { ("16", "0000010000000100") };
        chart.push_str(&format!("#{measure:03}{}:{}\n#{measure:03}18:0000000100000000\n#{measure:03}19:0000000000000001\n", turntable.0, turntable.1));
    }
    chart
}

/// The SCRATCH AUTO run of [`stairs_with_a_turntable`] at `bpm`, set up as the load path sets it up:
/// the keys played by the game and the turntable left to the chart.
fn scratch_auto_run(bpm: u32) -> PlayState {
    let src = rbms_parser::parse_with(stairs_with_a_turntable(bpm).as_bytes(), Default::default());
    let model = rbms_chart::to_model(&src, rbms_chart::detect_mode(&src, "turntable.bms"));
    let mode = model.mode;
    let auto_lanes = (0..mode.key).map(|lane| mode.is_scratch(lane)).collect();
    let session = PlaySession::new(model, SessionOptions { autoplay: true, auto_lanes, ..SessionOptions::default() });
    PlayState::new(session, std::collections::HashMap::new(), 0, SCORE_LN_MODE_FROM_CHART.to_string())
}

/// Whether a pixel is the green a note of a lane the chart plays by itself is drawn in.
fn is_auto_lane_green(rgba: &[u8]) -> bool {
    let green = rbms_render::skin_render::frame::AUTO_LANE_NOTE;
    rgba[..3] == [green.r, green.g, green.b]
}

/// The turntable SCRATCH AUTO plays is drawn in green on the default pack, and on the pack
/// [`SKIN_PACK_ENV`] names when it names one, while the run counts the key notes alone. Saved as
/// `scratch-auto-play-<pack folder>.png`.
#[test]
fn the_turntable_scratch_auto_plays_is_drawn_green_and_counts_for_nothing() {
    let mut config = holding_a_green_number();
    config.play.scratch_auto = true;
    let default_pack = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER);
    let mut packs = vec![default_pack];
    packs.extend(std::env::var_os(SKIN_PACK_ENV).map(std::path::PathBuf::from));
    for folder in packs {
        let name = folder.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let mut app = app_on_the_pack(&format!("scratch-auto-{name}"), config.clone(), &folder);
        let size = frame_size(&app);
        let mut pixels = HeadlessCanvas::new(size.0, size.1);
        play_into(&mut app, &mut pixels, scratch_auto_run(SLOW_BPM));
        let session = &play(&app).session;
        assert!(session.has_auto_played_notes(), "{name}: the turntable was not left to the chart");
        assert!(session.field_timelines().iter().any(|line| line.notes[TURNTABLE_LANE].is_some()), "{name}: the field lost the turntable");
        assert!(session.model().timelines.iter().all(|line| line.notes[TURNTABLE_LANE].is_none()), "{name}: the run judges the turntable");
        assert_eq!(session.judge().combo, session.judge().total_judged(), "{name}: the run played its key notes whole");
        let green = pixels.rgba().chunks_exact(4).filter(|pixel| is_auto_lane_green(pixel)).count();
        assert!(green > 0, "{name}: no turntable note was drawn green");
        save(&format!("scratch-auto-play-{name}"), &pixels, size);
    }
}
