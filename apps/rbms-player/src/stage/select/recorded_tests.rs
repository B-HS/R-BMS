//! What the browser shows of a run a player who never signed in has just played, on the default
//! skin pack: a chart is played by hand to its end with no score server set up, the application is
//! handed the run the way the play screen hands one over, and the browser is put up on the chart.
//!
//! The score window a skin draws is read from the best the score database holds
//! ([`AppShared::chart_records_shown`]), which the browser reads when its cursor comes to rest on a
//! chart or a run is recorded. So what is held here is that best as the browser lends it to its
//! skin: the judgements the bars are drawn from, the EX and the rate the rank is read off, the
//! combo, the bad-poor count, the lamp and the play count.
//!
//! With `RBMS_SKIN_CAPTURE_DIR` set the frame is drawn at the size the pack was authored at and
//! saved there as `select-recorded.png`, the frame of a chart played with SCRATCH AUTO on as
//! `select-recorded-scratch-auto.png`, and that of a chart played only with the judge widened as
//! `select-recorded-widened.png`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rbms_library::Library;
use rbms_play::{PlaySession, PlaySummary};
use rbms_skin::resolve::plain_path;
use rbms_store::scoredb::UNSET_MINIMUM;

use super::tests::{entry, press, release};
use super::*;
use crate::app_result::record_tests::{HAND_OFFSETS_US, end_run, played_by_hand, played_widened, recorded_model};
use crate::skin_host::select::SelectShown;
use crate::stage::HeadlessCanvas;
use crate::stage::capture::{app_in, scene_frame_at, settings_of};
use crate::{App, Config};

/// The default skin pack, named from this package's folder.
const DEFAULT_PACK_FOLDER: &str = "../../assets/skins/rbms-default";

/// The variable that names the folder captures are saved in, which the capture harness reads.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The size the frames are drawn at when nothing is saved, and the size the pack is authored at,
/// which a saved frame is drawn at.
const CHECK_SIZE: (u32, u32) = (640, 360);
const AUTHORED_SIZE: (u32, u32) = (1920, 1080);

/// How long the pack's files are given to be read before a test gives up on them, which is as
/// long as the capture harness gives a pack.
pub(crate) const PACK_LOAD_WAIT: Duration = Duration::from_secs(180);

/// The seed every read of the pack is pinned to.
const PACK_SEED: u64 = 1;

/// The moment of the browser's scene its cursor is moved at, in milliseconds from its beginning,
/// and the moment its settled layout is read and saved at.
const MOVE_MS: i64 = 1000;
const SETTLED_MS: i64 = 3000;

/// How long a slide of the wheel is waited out a step at a time.
const SLIDE_STEP: Duration = Duration::from_millis(1);

/// The titles of the charts listed above and below the one that was played, and its own.
const TITLE_ABOVE: &str = "Prelude";
const RECORDED_TITLE: &str = "Recorded";
const TITLE_BELOW: &str = "Sequel";

/// An app drawing with the default pack, every read of it pinned, with no sound device, no score
/// server set up and nobody signed in.
pub(crate) fn default_pack_app(tag: &str) -> App {
    let settings = settings_of(tag);
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_PACK_FOLDER);
    let pack = plain_path(&folder.canonicalize().expect("the default pack is in the repository"));
    let mut config = Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    config.library.preview = false;
    config.play.autoplay = false;
    let mut app = app_in(settings.clone(), config);
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.shared.audio_failed = true;
    assert!(!app.shared.has_primary_ir_server() && !app.shared.session.is_logged_in(), "the fixture is a player who never signed in");
    app
}

/// The same app over a library of three charts, the middle one the chart the record tests play.
pub(crate) fn browsing_app(tag: &str) -> App {
    let mut app = default_pack_app(tag);
    let model = recorded_model();
    let recorded = rbms_library::SongEntry { md5: model.md5, mode: model.mode, path: PathBuf::from("recorded.bms"), ..entry(RECORDED_TITLE, "rbms", "7") };
    app.shared.library = Library::from_songs(vec![entry(TITLE_ABOVE, "rbms", "3"), recorded, entry(TITLE_BELOW, "rbms", "9")]);
    app.shared.open_for_tests([SelectFolder::AllSongs]);
    app.shared.config.library.sort = SortMode::Title;
    app.shared.rebuild_select_items();
    app
}

/// Play the recorded chart by hand in `app` and hand the run over as the play screen does.
pub(crate) fn play_a_run(app: &mut App, session: PlaySession) -> PlaySummary {
    let (entered, summary) = end_run(&mut app.shared, session);
    assert!(matches!(entered, Transition::To(Stage::Result(_))), "the run did not reach the result screen");
    summary
}

/// The size a screen of the pack is drawn at: the pack's own when the frame is to be saved, and a
/// small one otherwise, since what is held is what the skin is lent and not how it is drawn.
pub(crate) fn frame_size() -> (u32, u32) {
    if std::env::var_os(CAPTURE_DIR_ENV).is_some() { AUTHORED_SIZE } else { CHECK_SIZE }
}

/// Save the frame in `pixels`, which is [`frame_size`] large, under the capture folder as
/// `<name>.png`, when one was asked for.
pub(crate) fn save_frame(name: &str, pixels: &HeadlessCanvas) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV) else {
        return;
    };
    let folder = PathBuf::from(folder);
    std::fs::create_dir_all(&folder).expect("the capture folder can be created");
    let (width, height) = frame_size();
    image::save_buffer(folder.join(format!("{name}.png")), pixels.rgba(), width, height, image::ExtendedColorType::Rgba8).expect("the capture is written");
}

/// Put the browser up with its cursor on the row above the recorded chart, wait for the pack's
/// document, move the cursor down onto the chart with the key a player presses, wait the slide out
/// and draw the settled layout: the walk the capture harness makes, since a skin shows what belongs
/// to the bar under the cursor from the move that brought it there.
pub(crate) fn browse_to_the_recorded_chart(app: &mut App, pixels: &mut HeadlessCanvas) {
    let songs = app.shared.library.songs();
    let row = app
        .shared
        .select_items
        .iter()
        .position(|item| matches!(item, SelectItem::Song(index) if songs.get(*index).is_some_and(|song| song.title == RECORDED_TITLE)))
        .expect("the recorded chart is listed");
    app.shared.sel = row.checked_sub(1).expect("a chart is listed above the recorded one");
    app.stage = Stage::Select(Box::new(SelectState::new()));
    let deadline = Instant::now() + PACK_LOAD_WAIT;
    while !app.shared.has_compiled_skin(SKIN_TYPE_MUSIC_SELECT) {
        scene_frame_at(app, 0, &mut Canvas::Headless(pixels));
        assert_eq!(app.shared.skin_failure(SKIN_TYPE_MUSIC_SELECT), None, "the default pack's browser failed to load");
        assert!(Instant::now() < deadline, "the default pack's browser never compiled");
    }
    scene_frame_at(app, MOVE_MS, &mut Canvas::Headless(pixels));
    let now = Instant::now();
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, press(KeyCode::ArrowDown));
    scene_frame_at(app, MOVE_MS, &mut Canvas::Headless(pixels));
    app.stage.handle_key(&mut FrameCtx { shared: &mut app.shared, now, dt: 0.0 }, release(KeyCode::ArrowDown));
    while !matches!(&app.stage, Stage::Select(state) if state.pictures_are_in() && state.wheel_is_at_rest()) {
        std::thread::sleep(SLIDE_STEP);
        scene_frame_at(app, MOVE_MS, &mut Canvas::Headless(pixels));
        assert!(Instant::now() < deadline, "the browser's wheel never came to rest");
    }
    scene_frame_at(app, SETTLED_MS, &mut Canvas::Headless(pixels));
    assert_eq!(app.shared.sel, row, "the cursor is not on the recorded chart");
}

/// The browser's frame as the property host of its skin reads it, from what the browser that is up
/// lends: the list its wheel draws and the records it read of the chart under the cursor.
pub(crate) fn shown(app: &mut App) -> SelectShown {
    let Stage::Select(state) = &mut app.stage else {
        panic!("the browser is not up");
    };
    let state: &mut SelectState = state;
    let cursor = state.cursor(&app.shared);
    let wheel = SongBars::new(state.bars.wheel(0), cursor);
    let lent = BrowserLent {
        best: state.focused_records.best(),
        replays_stored: state.focused_records.replays_stored(),
        selected_replay: state.selected_replay,
        ..BrowserLent::default()
    };
    app.shared.select_shown(Some(&wheel), &lent)
}

/// A run played to its end with nobody signed in is on the browser when the player comes back to
/// it: the score window of the default pack is lent the run's judgements, its EX and rate, its
/// combo, its bad-poor count, its lamp, one play and one replay.
#[test]
fn the_browser_shows_a_run_played_without_signing_in() {
    let mut app = browsing_app("recorded");
    let summary = play_a_run(&mut app, played_by_hand(&HAND_OFFSETS_US, false));
    let (width, height) = frame_size();
    let mut pixels = HeadlessCanvas::new(width, height);
    browse_to_the_recorded_chart(&mut app, &mut pixels);
    save_frame("select-recorded", &pixels);

    let shown = shown(&mut app);
    let record = shown.bar.as_ref().and_then(|bar| bar.record.as_ref()).expect("the bar under the cursor has a record");
    assert_eq!((record.sheet.early, record.sheet.late), (summary.early, summary.late), "the judgements the bars are drawn from");
    assert_eq!(record.sheet.ex_score(), summary.ex_score, "EX SCORE");
    assert_eq!((record.sheet.max_combo, record.sheet.min_bp, record.sheet.notes), (summary.max_combo, summary.min_bp, summary.total_notes));
    assert_eq!(record.sheet.clear, rbms_judge::clear_type_id(summary.clear_lamp), "the lamp");
    assert_eq!((record.play_count, record.clear_count), (1, 1), "PLAY COUNT and CLEAR COUNT");
    assert!(record.last_played > 0, "the run has no date");
    assert_eq!(shown.standing.now_ex, i32::try_from(summary.ex_score).expect("an EX score fits"), "the EX the rank is read off");
    assert!(shown.standing.rate > 0.0 && shown.standing.rank.iter().any(|reached| *reached), "RATE and RANK: {:?}", shown.standing.rank);
    assert_eq!(shown.bar.as_ref().and_then(|bar| bar.replays), Some([true, false, false, false]), "the run's replay is in the first slot");
}

/// A chart played with SCRATCH AUTO on shows that run as its best: the EX, the judgements, the
/// combo and the notes it counted -- the turntable's are not among them -- under the LIGHT ASSIST
/// CLEAR lamp the assist demotes a clear to.
#[test]
fn the_browser_shows_the_score_of_a_chart_played_with_scratch_auto() {
    let mut app = browsing_app("recorded-scratch-auto");
    app.shared.config.play.scratch_auto = true;
    let summary = play_a_run(&mut app, played_by_hand(&HAND_OFFSETS_US, true));
    assert!(summary.ex_score > 0, "the run itself scored");
    let (width, height) = frame_size();
    let mut pixels = HeadlessCanvas::new(width, height);
    browse_to_the_recorded_chart(&mut app, &mut pixels);
    save_frame("select-recorded-scratch-auto", &pixels);

    let shown = shown(&mut app);
    let record = shown.bar.as_ref().and_then(|bar| bar.record.as_ref()).expect("the bar under the cursor has a record");
    assert_eq!(record.sheet.clear, rbms_judge::clear_type_id(rbms_judge::ClearType::LightAssistEasy));
    assert_eq!((record.play_count, record.clear_count), (1, 1));
    assert_eq!((record.sheet.ex_score(), record.sheet.max_combo, record.sheet.notes), (summary.ex_score, summary.max_combo, summary.total_notes));
    assert_eq!(summary.total_notes + 1, played_by_hand(&HAND_OFFSETS_US, false).summary().total_notes, "the turntable note was counted");
    assert_eq!(shown.standing.now_ex, i32::try_from(summary.ex_score).expect("an EX score fits"), "the EX the rank is read off");
    assert_eq!(shown.bar.as_ref().and_then(|bar| bar.replays), Some([true, false, false, false]), "the run's replay is kept");
}

/// A chart played only with the judge widened past its own shows its lamp and its play count and no
/// score: such a run sets no best, which is all the reference keeps of one.
#[test]
fn the_browser_shows_the_lamp_and_no_score_of_a_chart_played_only_with_a_widened_judge() {
    let mut app = browsing_app("recorded-widened");
    let summary = play_a_run(&mut app, played_widened(&HAND_OFFSETS_US));
    assert!(summary.ex_score > 0, "the run itself scored");
    let (width, height) = frame_size();
    let mut pixels = HeadlessCanvas::new(width, height);
    browse_to_the_recorded_chart(&mut app, &mut pixels);
    save_frame("select-recorded-widened", &pixels);

    let shown = shown(&mut app);
    let record = shown.bar.as_ref().and_then(|bar| bar.record.as_ref()).expect("the bar under the cursor has a record");
    assert_eq!(record.sheet.clear, rbms_judge::clear_type_id(rbms_judge::ClearType::AssistEasy));
    assert_eq!((record.play_count, record.clear_count), (1, 1));
    assert_eq!((record.sheet.ex_score(), record.sheet.max_combo, record.sheet.notes), (0, 0, 0), "a widened run set a best score");
    assert_eq!(i64::from(record.sheet.min_bp), UNSET_MINIMUM, "a widened run set a bad-poor count");
    assert_eq!(shown.bar.as_ref().and_then(|bar| bar.replays), Some([true, false, false, false]), "the widened run's replay is kept");
}
