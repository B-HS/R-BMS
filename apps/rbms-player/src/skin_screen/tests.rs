use rbms_render::result::ResultView;
use rbms_skin::dst::DrawStateSource;
use rbms_skin::loader::SKIN_TYPE_PLAY_7KEYS;
use rbms_skin::property::generated::{BUTTON_LNMODE, FLOAT_LOADING_PROGRESS, NUMBER_PLAYLEVEL, OFFSET_ALL, RATE_LOAD_PROGRESS};
use rbms_skin::property::{FLOAT_ABSENT, IMAGE_INDEX_ABSENT, INTEGER_ABSENT, SkinHost};
use rbms_skin::timer::{TIMER_OFF, TimerId};

use super::*;

/// A window pixel becomes a document coordinate: scaled onto the authored size, and measured up
/// from the bottom rather than down from the top, which is the one flip a document expects.
#[test]
fn the_cursor_reaches_a_document_in_its_own_upward_coordinates() {
    let authored = (1280.0, 720.0);
    let at = document_cursor((640.0, 180.0), (1280, 720), authored).expect("a sized window maps the cursor");
    assert_eq!(at, (640.0, 540.0), "the document measures y up from its own bottom edge");

    let scaled = document_cursor((320.0, 90.0), (640, 360), authored).expect("a half-size window maps the cursor");
    assert_eq!(scaled, (640.0, 540.0), "the same spot on a smaller window is the same spot in the document");
}

/// A window with no extent has no coordinates to map onto, and must not divide by its own zero.
#[test]
fn a_window_with_no_extent_reports_no_cursor() {
    assert!(document_cursor((10.0, 10.0), (0, 720), (1280.0, 720.0)).is_none());
    assert!(document_cursor((10.0, 10.0), (1280, 0), (1280.0, 720.0)).is_none());
}

/// Compiling a document reads nothing off the disk: every file it names was read on the worker pool
/// before the screen was built, and a file that is not in that map is one the screen goes without.
/// Falling back to reading it here would put the decode back on the frame loop -- which is the whole
/// thing the pool exists to keep it off.
#[test]
fn compiling_a_document_never_reads_a_file_itself() {
    let dir = std::env::temp_dir().join(format!("rbms-skin-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the scratch folder is writable");
    let image = dir.join("source.png");
    let font = dir.join("face.ttf");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255])).save(&image).expect("the image is written");
    std::fs::write(&font, b"not really a font").expect("the font is written");

    let mut assets = PlayerSkinAssets::new(BTreeMap::new());
    assert!(assets.image(&image).is_none(), "an image nobody prepared is one the screen goes without");
    assert!(assets.font(&font).is_none(), "and so is a font");

    let decoded = SkinImage::new(2, 2, vec![9; 16]).expect("a two by two image");
    let prepared = BTreeMap::from([
        ((SkinAssetKind::Image, image.clone()), SkinAsset::Image(decoded)),
        ((SkinAssetKind::Font, font.clone()), SkinAsset::Font(vec![7, 7])),
    ]);
    let mut assets = PlayerSkinAssets::new(prepared);
    assert_eq!(assets.image(&image).map(|image| image.rgba), Some(vec![9; 16]), "what the worker read is what the screen gets");
    assert_eq!(assets.font(&font), Some(vec![7, 7]));
}

/// A nudge is read out of the choices stored for the document being drawn, and an id nothing was
/// moved under -- or a document nobody has customised at all -- answers nothing, which is what
/// leaves a destination where its author put it.
#[test]
fn a_document_nudge_is_read_from_the_choices_stored_for_that_document() {
    let nudge = SkinOffset { x: 3.0, ..SkinOffset::default() };
    let mut document = SkinCustomisation::default();
    document.offsets.insert(46, nudge);

    let offsets = DocumentOffsets::drawn_at(Some(&document), (1280.0, 720.0), (1280, 720));
    assert_eq!(offsets.offset(46), Some(nudge), "a row the document carries is read");
    assert_eq!(offsets.offset(48), None, "a row the document does not carry is answered anyway");

    assert_eq!(DocumentOffsets::default().offset(46), None, "a document nobody has customised nudges nothing");
}

/// A nudge is so many pixels of what the screen is drawn on, whatever size the document was authored
/// at: a document drawn at two thirds of its size is told a nudge half as large again in its own
/// units, which the drawing then scales back to the pixels that were stored. An angle and an alpha
/// are not lengths, and the offset that moves the whole screen is a share of it; none of them is
/// touched.
#[test]
fn a_nudge_is_pixels_of_the_target_whatever_size_the_document_is_drawn_at() {
    const AUTHORED: (f32, f32) = (1920.0, 1080.0);
    const SMALLER: (u32, u32) = (1280, 720);
    const SCALE: f32 = 1.5;
    let nudge = SkinOffset { x: 30.0, y: -12.0, w: 6.0, h: 4.0, r: 45.0, a: -100.0 };
    let mut document = SkinCustomisation::default();
    document.offsets.insert(46, nudge);
    document.offsets.insert(OFFSET_ALL, nudge);

    let full = DocumentOffsets::drawn_at(Some(&document), AUTHORED, (1920, 1080));
    assert_eq!(full.offset(46), Some(nudge), "a document drawn at its own size reads a nudge as stored");

    let smaller = DocumentOffsets::drawn_at(Some(&document), AUTHORED, SMALLER);
    let told = smaller.offset(46).expect("the row is stored");
    assert_eq!((told.x, told.y, told.w, told.h), (nudge.x * SCALE, nudge.y * SCALE, nudge.w * SCALE, nudge.h * SCALE));
    assert_eq!((told.r, told.a), (nudge.r, nudge.a), "an angle or an alpha was scaled like a length");
    let drawn_scale = SMALLER.0 as f32 / AUTHORED.0;
    assert_eq!(told.x * drawn_scale, nudge.x, "the nudge does not come to the stored pixels once the document is drawn");
    assert_eq!(smaller.offset(OFFSET_ALL), Some(nudge), "the whole-screen offset is a share of the screen, not a length");

    let unknown = DocumentOffsets::drawn_at(Some(&document), (0.0, 0.0), (0, 0));
    assert_eq!(unknown.offset(46), Some(nudge), "a document with no size to go by is read one to one");
}

/// The `STRING_TITLE` id, which the decide screen's state answers with the title it was handed.
const STRING_TITLE: i32 = 10;

/// Frames the screens below are drawn for: far more than the two a Lua skin needs to be read and
/// taken in, so a read that happened once a frame would show.
const SKIN_FRAMES: usize = 12;

/// Frames a skin with no files is given to compile, which it does within a frame or two of being
/// read.
const COMPILE_FRAMES: usize = 240;

/// The title the decide screen is drawn with.
const TITLE: &str = "Sample Song";

/// A Lua skin for the decide screen whose body notes each run in a file and then raises.
const BROKEN_DECIDE: &str = r#"
if skin_config then
    local runs = io.open("runs.txt", "a")
    runs:write("x")
    runs:close()
    error("no such asset")
end
return { type = 6, name = "Broken", w = 1280, h = 720 }
"#;

/// A Lua skin for the decide screen whose body writes down the title its host answered.
const TITLED_DECIDE: &str = r#"
local main_state = require("main_state")
if skin_config then
    local seen = io.open("title.txt", "w")
    seen:write(main_state.text(STRING_TITLE_ID))
    seen:close()
    return { type = 6, name = "Titled", w = 1280, h = 720, destination = {} }
end
return { type = 6, name = "Titled", w = 1280, h = 720 }
"#;

/// The file a pack's decide-screen skin is written to.
const DECIDE_SKIN_FILE: &str = "decide.luaskin";

/// An app whose skin pack is one folder holding one decide-screen skin, and that skin's path.
fn app_with_decide_skin(tag: &str, source: &str) -> (crate::App, std::path::PathBuf) {
    app_with_pack_skin(tag, DECIDE_SKIN_FILE, source)
}

/// An app whose skin pack is one folder holding one skin under `file`, and that skin's path.
fn app_with_pack_skin(tag: &str, file: &str, source: &str) -> (crate::App, std::path::PathBuf) {
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-screen-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let pack = home.join("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    let document = pack.join(file);
    std::fs::write(&document, source).expect("the skin is written");
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
    (app, document.canonicalize().expect("the skin was just written"))
}

/// Draw the decide screen the way its stage does, `frames` times over, and answer whether each
/// frame was drawn by the document.
fn draw_decide(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas, frames: usize) -> Vec<bool> {
    (0..frames)
        .map(|_| {
            let mut canvas = Canvas::Headless(pixels);
            app.shared.prepare_skin(&mut canvas, SKIN_TYPE_DECIDE);
            let chart = ChartMeta { title: TITLE, ..ChartMeta::default() };
            app.shared.draw_decide_skin(&mut canvas, &DecideDraw { chart: &chart, progress: 0.0, data: FrameData::default() })
        })
        .collect()
}

/// How long a skin with a file to decode is given to compile, and how long a frame that finds it
/// still decoding stands back for the worker pool. The decode is off the frame loop, so the number
/// of frames drawn says nothing about how long a worker has had.
const DECODE_WAIT: Duration = Duration::from_secs(20);
const DECODE_FRAME_PAUSE: Duration = Duration::from_millis(1);

/// Draw the decide screen until its skin has compiled, answering whether it did in time.
fn draw_decide_until_compiled(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas) -> bool {
    let began = Instant::now();
    loop {
        draw_decide(app, pixels, 1);
        if app.shared.has_compiled_skin(SKIN_TYPE_DECIDE) {
            return true;
        }
        if began.elapsed() > DECODE_WAIT {
            return false;
        }
        std::thread::sleep(DECODE_FRAME_PAUSE);
    }
}

/// A Lua skin that cannot be read costs the screen one read and one message: its body runs once,
/// the reason is said once, and every frame after is the built-in layout's to draw.
#[test]
fn a_lua_skin_that_cannot_be_read_is_reported_once_and_leaves_the_built_in_screen_drawing() {
    crate::notify::exclusive(|| {
        let (mut app, document) = app_with_decide_skin("broken", BROKEN_DECIDE);
        let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);

        let drew = draw_decide(&mut app, &mut pixels, SKIN_FRAMES);
        assert!(drew.iter().all(|by_document| !by_document), "a skin that raised drew a frame: {drew:?}");
        assert!(!app.shared.has_skin_document(SKIN_TYPE_DECIDE), "the screen still stands aside for a skin that is not coming");
        assert!(app.shared.skin_failure(SKIN_TYPE_DECIDE).is_some_and(|reason| reason.contains("no such asset")));

        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        let reasons: Vec<&String> = said.iter().map(|(_, message)| message).filter(|message| message.contains("no such asset")).collect();
        assert_eq!(reasons.len(), 1, "the failure was not said exactly once: {reasons:?}");
        assert!(reasons[0].ends_with(SKIN_FALLBACK_NOTE) && !reasons[0].contains('\n'), "{:?}", reasons[0]);

        let overlay = crate::assets::skin_overlay_folder(&app.shared.settings_path, &document);
        assert_eq!(std::fs::read_to_string(overlay.join("runs.txt")).ok().as_deref(), Some("x"), "the skin's body did not run exactly once");

        app.shared.begin_skin_scene();
        draw_decide(&mut app, &mut pixels, SKIN_FRAMES);
        assert_eq!(std::fs::read_to_string(overlay.join("runs.txt")).ok().as_deref(), Some("x"), "a new scene ran a skin that had already failed");
    });
}

/// A Lua skin is read on its screen's first frame against that screen's own state, writes into the
/// overlay kept for its pack, and draws the screen alone once it has compiled. A new scene reads it
/// again, against the state the new scene brings.
#[test]
fn a_lua_skin_is_read_against_the_state_of_the_screen_it_draws() {
    let (mut app, document) = app_with_decide_skin("titled", &TITLED_DECIDE.replace("STRING_TITLE_ID", &STRING_TITLE.to_string()));
    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    assert!(!app.shared.has_compiled_skin(SKIN_TYPE_DECIDE));

    let first = draw_decide(&mut app, &mut pixels, 1);
    assert_eq!(first, vec![false], "the frame that reads a skin has nothing compiled to draw with");
    let overlay = crate::assets::skin_overlay_folder(&app.shared.settings_path, &document);
    assert_eq!(std::fs::read_to_string(overlay.join("title.txt")).ok().as_deref(), Some(TITLE), "the skin was not read against the screen's own state");
    assert!(!document.with_file_name("title.txt").exists(), "the skin wrote into its own folder");

    let compiled = (0..COMPILE_FRAMES).any(|_| {
        draw_decide(&mut app, &mut pixels, 1);
        app.shared.has_compiled_skin(SKIN_TYPE_DECIDE)
    });
    assert!(compiled, "the skin never compiled: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));
    assert_eq!(draw_decide(&mut app, &mut pixels, 1), vec![true], "a compiled skin did not draw its screen");

    std::fs::remove_file(overlay.join("title.txt")).expect("the overlay is the test's own");
    draw_decide(&mut app, &mut pixels, 1);
    assert!(!overlay.join("title.txt").exists(), "a skin was read again in the middle of a scene");
    app.shared.begin_skin_scene();
    draw_decide(&mut app, &mut pixels, 1);
    assert_eq!(std::fs::read_to_string(overlay.join("title.txt")).ok().as_deref(), Some(TITLE), "a new scene did not read its skin again");
}

/// A document for the decide screen that is only data.
const PLAIN_DECIDE: &str = r#"{ "type": 6, "name": "Plain", "w": 1280, "h": 720, "destination": [] }"#;

/// Taking the pack away releases the screen compiled from it even though the screen being drawn is
/// another one: the decide screen's document goes while the browser is the one on show.
#[test]
fn a_pack_that_is_taken_away_releases_the_screens_nobody_is_drawing() {
    let (mut app, document) = app_with_decide_skin("released", TITLED_DECIDE);
    std::fs::remove_file(&document).expect("the pack is the test's own");
    std::fs::write(document.with_extension("json"), PLAIN_DECIDE).expect("the document is written");
    app.shared.rescan_skins();
    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    let compiled = (0..COMPILE_FRAMES).any(|_| {
        draw_decide(&mut app, &mut pixels, 1);
        app.shared.has_compiled_skin(SKIN_TYPE_DECIDE)
    });
    assert!(compiled, "the document never compiled: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));

    app.shared.config.skin.pack = None;
    let mut canvas = Canvas::Headless(&mut pixels);
    app.shared.prepare_skin(&mut canvas, SKIN_TYPE_MUSIC_SELECT);
    assert!(!app.shared.has_compiled_skin(SKIN_TYPE_DECIDE), "a screen compiled from a pack that is gone is still held");
    assert!(app.shared.skins.document(SKIN_TYPE_DECIDE).is_none(), "the document of a pack that is gone is still loaded");
    assert!(!app.shared.has_skin_document(SKIN_TYPE_DECIDE));
}

/// A Lua skin for the decide screen with one object, gated by a function that counts its own calls
/// and reads the scene clock, which only a bound host can answer.
const COUNTED_DECIDE: &str = r#"
local main_state = require("main_state")
calls = 0
local function counted()
    calls = calls + 1
    return main_state.time() >= 0
end
local skin = { type = 6, name = "Counted", w = 1280, h = 720 }
if skin_config then
    skin.destination = { { id = -110, draw = counted, dst = { { x = 0, y = 0, w = 1280, h = 720 } } } }
end
return skin
"#;

/// How often the counted skin's function has run.
fn counted_calls(app: &crate::App) -> i64 {
    let runtime = app.shared.skins.document(SKIN_TYPE_DECIDE).and_then(LoadedSkin::runtime).expect("the counted skin is loaded with its interpreter");
    runtime.lua().globals().get("calls").expect("the skin keeps its count in a global")
}

/// A frame binds its host to the skin's interpreter once and prepares every object inside that one
/// binding, so a function the skin wrote runs once a frame; the frame is drawn after the binding
/// has ended, and drawing it calls nothing.
#[test]
fn a_frame_calls_a_skin_function_once_and_only_while_its_host_is_bound() {
    let (mut app, _) = app_with_decide_skin("counted", COUNTED_DECIDE);
    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    let compiled = (0..COMPILE_FRAMES).any(|_| {
        draw_decide(&mut app, &mut pixels, 1);
        app.shared.has_compiled_skin(SKIN_TYPE_DECIDE)
    });
    assert!(compiled, "the skin never compiled: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));

    let before = counted_calls(&app);
    let drew = draw_decide(&mut app, &mut pixels, SKIN_FRAMES);
    assert!(drew.iter().all(|by_document| *by_document), "a compiled skin did not draw its screen: {drew:?}");
    assert_eq!(counted_calls(&app) - before, SKIN_FRAMES as i64, "the function did not run exactly once a frame");

    let runtime = app.shared.skins.document(SKIN_TYPE_DECIDE).and_then(LoadedSkin::runtime).expect("the counted skin is loaded with its interpreter");
    assert_eq!(runtime.diagnostics().function_failures, Vec::new(), "a function ran with no host bound, where its reads fail");
}

/// A timer the tests below switch on, and the moment it was switched on.
const RUNNING_TIMER: i32 = 1;
const STARTED_US: i64 = 250_000;

/// An option the reference answers that a host with nothing on it has no cluster to answer from.
const OPTION_NOW_LOADING: i32 = 80;

/// The host a screen is drawn through answers from its clusters and from nothing else: an id no
/// cluster knows reads as the absent value of its kind, whichever way it is asked. The scene's
/// timers are the host's own.
#[test]
fn an_id_no_cluster_knows_reads_as_absent_and_a_timer_is_read_from_the_scenes_own_table() {
    let mut timers = TimerState::new();
    timers.set_on(TimerId(RUNNING_TIMER), STARTED_US);
    let host = ScreenHost::new(STARTED_US, &timers);

    assert_eq!(host.text(STRING_TITLE), "", "a host with no chart on it knows no title");
    assert_eq!(host.boolean(OPTION_NOW_LOADING), None);
    assert_eq!(host.boolean(-OPTION_NOW_LOADING), None, "and negating an option nobody knows does not make it known");
    assert_eq!(host.integer(NUMBER_PLAYLEVEL), INTEGER_ABSENT);
    assert_eq!(host.rate(RATE_LOAD_PROGRESS), None);
    assert_eq!(host.float(FLOAT_LOADING_PROGRESS).to_bits(), FLOAT_ABSENT.to_bits());
    assert_eq!(host.image_index(BUTTON_LNMODE), IMAGE_INDEX_ABSENT);

    assert_eq!(host.now_us(), STARTED_US);
    assert_eq!(host.timer_us(RUNNING_TIMER), STARTED_US, "a script's timer read comes from the scene's own table");
    assert_eq!(host.timer_us(RUNNING_TIMER + 1), TIMER_OFF);
}

/// The two colours the picked skin's image set is made of, neither of which a built-in screen paints.
const FIRST_SET: crate::Color = crate::Color::rgb(12, 200, 90);
const SECOND_SET: crate::Color = crate::Color::rgb(200, 12, 90);

/// Edge of one image of the picked skin's sheet, which holds the two side by side.
const SET_EDGE: u32 = 8;

/// The file the picked skin's sheet is written to, beside the skin.
const SET_SHEET: &str = "sets.png";

/// A Lua skin for the decide screen whose one object is a set of two images over the whole screen,
/// picked by an image index only the browser's and the player's settings can answer.
const PICKED_DECIDE: &str = r#"
local skin = { type = 6, name = "Picked", w = 1280, h = 720 }
if skin_config then
    skin.source = { { id = 0, path = "SET_SHEET" } }
    skin.image = {
        { id = "first", src = 0, x = 0, y = 0, w = SET_EDGE, h = SET_EDGE },
        { id = "second", src = 0, x = SET_EDGE, y = 0, w = SET_EDGE, h = SET_EDGE },
    }
    skin.imageset = { { id = "picked", ref = PICKED_BY, images = { "first", "second" } } }
    skin.destination = { { id = "picked", dst = { { x = 0, y = 0, w = 1280, h = 720 } } } }
end
return skin
"#;

/// An image picked by one of the player's settings is drawn with the image that setting names, on a
/// screen that is neither the browser nor a result: the reference reads the setting off the player's
/// configuration whichever screen is up.
#[test]
fn an_image_picked_by_a_setting_is_drawn_with_the_image_the_setting_names() {
    let source = PICKED_DECIDE.replace("SET_SHEET", SET_SHEET).replace("SET_EDGE", &SET_EDGE.to_string()).replace("PICKED_BY", &BUTTON_LNMODE.to_string());
    let (mut app, document) = app_with_decide_skin("picked", &source);
    let sheet = image::RgbaImage::from_fn(SET_EDGE * 2, SET_EDGE, |x, _| {
        let set = if x < SET_EDGE { FIRST_SET } else { SECOND_SET };
        image::Rgba([set.r, set.g, set.b, set.a])
    });
    sheet.save(document.with_file_name(SET_SHEET)).expect("the sheet is written beside the skin");

    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    let compiled = draw_decide_until_compiled(&mut app, &mut pixels);
    assert!(compiled, "the skin never compiled: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));

    assert_eq!(draw_decide(&mut app, &mut pixels, 1), vec![true], "a compiled skin did not draw its screen");
    assert_eq!(pixels.pixel_at(UI_SIZE.0 / 2, UI_SIZE.1 / 2), FIRST_SET, "the long note mode is the first of the three, and so is the image");

    app.shared.config.judge.ln_mode = rbms_judge::ln::LnMode::ChargeNote;
    assert_eq!(draw_decide(&mut app, &mut pixels, 1), vec![true]);
    assert_eq!(pixels.pixel_at(UI_SIZE.0 / 2, UI_SIZE.1 / 2), SECOND_SET, "the image did not follow the setting it is picked by");
}

/// The file the whole-offset pack's play skin is written to.
const WHOLE_PLAY_FILE: &str = "play7.luaskin";

/// The colours the whole-offset skin paints: its ground over the whole screen, and one mark on it.
const WHOLE_GROUND: crate::Color = crate::Color::rgb(20, 40, 80);
const WHOLE_MARK: crate::Color = crate::Color::rgb(255, 255, 255);

/// A seven-key play skin authored at the size it is drawn at, so a skin pixel is a screen pixel: a
/// ground over the whole screen and a mark 128 by 72 whose bottom left corner is 256 in and 144 up.
const WHOLE_PLAY: &str = r#"
local skin = { type = 0, name = "Whole", w = 1280, h = 720 }
if skin_config then
    skin.destination = {
        { id = -111, dst = { { x = 0, y = 0, w = 1280, h = 720, r = 20, g = 40, b = 80 } } },
        { id = -111, dst = { { x = 256, y = 144, w = 128, h = 72, r = 255, g = 255, b = 255 } } },
    }
end
return skin
"#;

/// What the SKIN tab calls the offset every play skin has for moving the whole screen, which it
/// shows in capitals like every row.
const WHOLE_OFFSET_ROW: &str = "ALL OFFSET(%)";

/// A chart for the whole-offset skin's play screen to be drawn over.
const WHOLE_CHART: &[u8] = b"#PLAYER 1\n#BPM 120\n#WAV01 a.wav\n#00111:01\n";

/// Presses the SKIN tab's rows for the whole-screen offset of the seven-key play skin of `app`:
/// each step is an axis and how many presses, to the right when positive and to the left when not.
fn step_whole_offset(app: &mut crate::App, steps: &[(crate::skin_select::OffsetAxis, i32)]) {
    app.shared.config.skin.screen = SKIN_TYPE_PLAY_7KEYS;
    let rows = app.shared.skins.rows(&app.shared.config);
    for (axis, presses) in steps {
        let row = rows
            .iter()
            .copied()
            .find(|row| {
                matches!(row, crate::skin_select::SkinRow::Offset(_, on) if on == axis)
                    && app.shared.skins.line(&app.shared.config, *row).0.contains(WHOLE_OFFSET_ROW)
            })
            .unwrap_or_else(|| panic!("the SKIN tab has no {axis:?} row for the whole-screen offset among {rows:?}"));
        for _ in 0..presses.abs() {
            assert!(app.shared.skins.step(&mut app.shared.config, row, presses.signum()), "the row did not move");
        }
    }
}

/// What a play screen nobody has played on yet shows of its run.
fn unplayed_run() -> PlayShown {
    let source = rbms_parser::parse_with(WHOLE_CHART, Default::default());
    let session = rbms_play::PlaySession::new(rbms_chart::to_model(&source, rbms_model::Mode::BEAT_7K), rbms_play::SessionOptions::default());
    PlayShown::of(&session, &crate::skin_host::play::PlayLive::default())
}

/// One frame of the seven-key play screen of `app` as its skin draws it, answering whether it did.
fn play_frame(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas, shown: &PlayShown) -> bool {
    let offsets = PlayOffsets::default();
    let chart = ChartMeta { title: TITLE, ..ChartMeta::default() };
    let mut canvas = Canvas::Headless(pixels);
    app.shared.prepare_skin(&mut canvas, SKIN_TYPE_PLAY_7KEYS);
    app.shared.draw_play_skin(&mut canvas, SKIN_TYPE_PLAY_7KEYS, &PlayDraw { chart: &chart, shown, offsets: &offsets, data: FrameData::default() })
}

/// Draws the play screen of the whole-offset skin after the player stepped the rows of its
/// whole-screen offset on the SKIN tab by `steps`.
fn whole_play_frame(tag: &str, steps: &[(crate::skin_select::OffsetAxis, i32)]) -> crate::stage::HeadlessCanvas {
    let (mut app, _) = app_with_pack_skin(tag, WHOLE_PLAY_FILE, WHOLE_PLAY);
    let settings = app.shared.settings_path.clone();
    app.shared.skins.rescan(&settings, &app.shared.config);
    step_whole_offset(&mut app, steps);

    let shown = unplayed_run();
    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    let began = Instant::now();
    while !play_frame(&mut app, &mut pixels, &shown) {
        assert!(began.elapsed() < DECODE_WAIT, "the play skin never drew: {:?}", app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS));
        std::thread::sleep(DECODE_FRAME_PAUSE);
    }
    pixels
}

/// What the player sets on the SKIN tab's whole-screen offset rows moves and grows everything the
/// play skin draws (`Skin.ensureRenderer`): X and Y are a share of the screen, with Y going up, and
/// W and H stretch the screen away from its bottom left corner. What the moved screen no longer
/// covers is left black.
#[test]
fn the_whole_screen_offset_set_on_the_skin_tab_moves_and_grows_the_play_screen() {
    use crate::skin_select::OffsetAxis;
    let black = crate::Color::rgb(0, 0, 0);

    let at_rest = whole_play_frame("whole-rest", &[]);
    assert_eq!(at_rest.pixel_at(320, 540), WHOLE_MARK, "the mark is not where the skin put it");
    assert_eq!(at_rest.pixel_at(448, 468), WHOLE_GROUND);
    assert_eq!(at_rest.pixel_at(60, 300), WHOLE_GROUND, "the ground does not cover the screen");

    let moved = whole_play_frame("whole-moved", &[(OffsetAxis::X, 10), (OffsetAxis::Y, 10)]);
    assert_eq!(moved.pixel_at(448, 468), WHOLE_MARK, "ten percent right and up is 128 and 72 pixels of a 1280 by 720 screen");
    assert_eq!(moved.pixel_at(320, 540), WHOLE_GROUND, "the mark was also left where it had been");
    assert_eq!(moved.pixel_at(60, 300), black, "the ground moved with the mark, and uncovered the left of the screen");
    assert_eq!(moved.pixel_at(640, 700), black, "and the foot of it");

    let grown = whole_play_frame("whole-grown", &[(OffsetAxis::X, 10), (OffsetAxis::Y, 10), (OffsetAxis::W, 50), (OffsetAxis::H, 50)]);
    assert_eq!(
        grown.pixel_at(608, 378),
        WHOLE_MARK,
        "half as large again from the bottom left, then moved: the mark spans 512 to 704 across and 324 to 432 down"
    );
    assert_eq!(grown.pixel_at(515, 327), WHOLE_MARK);
    assert_eq!(grown.pixel_at(701, 429), WHOLE_MARK);
    assert_eq!(grown.pixel_at(508, 378), WHOLE_GROUND, "the mark grew past its left edge");
    assert_eq!(grown.pixel_at(448, 468), WHOLE_GROUND);
}

/// The file the pressed skin's one image source is written to, beside the skin, and its edge.
const PANEL_SHEET: &str = "panel.png";
const PANEL_EDGE: u32 = 8;

/// The event the pressed skin's numbered button runs, which the result screen and the browser both
/// answer, and one the reference defines nothing for.
const REPLAY_EVENT: i32 = rbms_skin::property::generated::BUTTON_REPLAY;
const UNDEFINED_EVENT: i32 = 9_999;

/// The rate the pressed skin's plain slider reads and writes.
const VOLUME_RATE: i32 = rbms_skin::property::generated::RATE_MASTERVOLUME;

/// A Lua skin for the decide screen with three buttons, each 100 by 40, and two sliders that travel
/// 200 to the right. `scripted` runs a function that counts its presses and keeps its argument,
/// `numbered` runs [`REPLAY_EVENT`] and `unknown` runs [`UNDEFINED_EVENT`]. `computed` writes where
/// it was dragged to through a function and `volume` writes it to [`VOLUME_RATE`]. An object of no
/// size polls the right arrow key on every frame, the way a published skin switches its menus.
const PRESSED_DECIDE: &str = r#"
presses = 0
argument = 0
written = -1
right_held = false
local skin = { type = 6, name = "Pressed", w = 1280, h = 720 }
if skin_config then
    local gdx = luajava.bindClass("com.badlogic.gdx.Gdx")
    local keys = luajava.bindClass("com.badlogic.gdx.Input").Keys
    local function shape(id, fields)
        local object = { id = id, src = 0, x = 0, y = 0, w = PANEL_EDGE, h = PANEL_EDGE }
        for key, value in pairs(fields) do object[key] = value end
        return object
    end
    local function at(x, y, w, h) return { { x = x, y = y, w = w, h = h } } end
    skin.source = { { id = 0, path = "PANEL_SHEET" } }
    skin.image = {
        shape("scripted", { act = function(direction)
            presses = presses + 1
            argument = direction
        end }),
        shape("numbered", { act = REPLAY_EVENT }),
        shape("unknown", { act = UNDEFINED_EVENT }),
    }
    skin.slider = {
        shape("computed", { angle = 1, range = 200, value = function() return 0 end, event = function(value) written = value end }),
        shape("volume", { angle = 1, range = 200, type = VOLUME_RATE }),
    }
    skin.destination = {
        { id = -110, draw = function() right_held = gdx.input:isKeyPressed(keys.RIGHT) end, dst = at(0, 0, 0, 0) },
        { id = "scripted", dst = at(100, 100, 100, 40) },
        { id = "numbered", dst = at(300, 100, 100, 40) },
        { id = "unknown", dst = at(500, 100, 100, 40) },
        { id = "computed", dst = at(100, 300, 20, 30) },
        { id = "volume", dst = at(100, 400, 20, 30) },
    }
end
return skin
"#;

/// A point of the pressed skin, given in the skin's own upward coordinates, as the cursor position
/// the window would report for it. The skin is authored at the size the cursor is measured in.
fn cursor_at(x: f32, y: f32) -> (f32, f32) {
    (x, UI_SIZE.1 as f32 - y)
}

/// Where the cursor is over each of the pressed skin's buttons, over the middle and the quarter of
/// its sliders' travel, and over nothing.
fn over_scripted() -> (f32, f32) {
    cursor_at(150.0, 120.0)
}
fn over_numbered() -> (f32, f32) {
    cursor_at(350.0, 120.0)
}
fn over_unknown() -> (f32, f32) {
    cursor_at(550.0, 120.0)
}
fn over_computed_quarter() -> (f32, f32) {
    cursor_at(150.0, 310.0)
}
fn over_volume_middle() -> (f32, f32) {
    cursor_at(200.0, 410.0)
}
fn over_nothing() -> (f32, f32) {
    cursor_at(900.0, 600.0)
}

const LEFT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: true };
const RIGHT_PRESS: PointerInput = PointerInput::Button { button: MouseButton::Right, pressed: true };
const LEFT_RELEASE: PointerInput = PointerInput::Button { button: MouseButton::Left, pressed: false };

/// An app drawing the pressed skin on its decide screen, with one whole frame of it behind it.
fn app_with_pressed_skin(tag: &str) -> (crate::App, crate::stage::HeadlessCanvas) {
    let source = PRESSED_DECIDE
        .replace("PANEL_SHEET", PANEL_SHEET)
        .replace("PANEL_EDGE", &PANEL_EDGE.to_string())
        .replace("REPLAY_EVENT", &REPLAY_EVENT.to_string())
        .replace("UNDEFINED_EVENT", &UNDEFINED_EVENT.to_string())
        .replace("VOLUME_RATE", &VOLUME_RATE.to_string());
    let (mut app, document) = app_with_decide_skin(tag, &source);
    image::RgbaImage::from_pixel(PANEL_EDGE, PANEL_EDGE, image::Rgba([u8::MAX; 4])).save(document.with_file_name(PANEL_SHEET)).expect("the sheet is written");
    let mut pixels = crate::stage::HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1);
    let compiled = draw_decide_until_compiled(&mut app, &mut pixels);
    assert!(compiled, "the skin never compiled: {:?}", app.shared.skin_failure(SKIN_TYPE_DECIDE));
    assert_eq!(app.shared.skin_warnings(SKIN_TYPE_DECIDE), &[] as &[String], "the skin did not compile whole");
    whole_frame(&mut app, &mut pixels);
    (app, pixels)
}

/// One frame of the decide screen as the frame loop makes it: drawn, and then ended.
fn whole_frame(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas) {
    assert_eq!(draw_decide(app, pixels, 1), vec![true], "the skin did not draw its screen");
    app.shared.finish_skin_frame(&mut Canvas::Headless(pixels));
}

/// A number the pressed skin keeps in a global.
fn skin_number(app: &crate::App, name: &str) -> f64 {
    let runtime = app.shared.skins.document(SKIN_TYPE_DECIDE).and_then(LoadedSkin::runtime).expect("the pressed skin is loaded with its interpreter");
    runtime.lua().globals().get(name).expect("the skin keeps the number in a global")
}

/// A flag the pressed skin keeps in a global.
fn skin_flag(app: &crate::App, name: &str) -> bool {
    let runtime = app.shared.skins.document(SKIN_TYPE_DECIDE).and_then(LoadedSkin::runtime).expect("the pressed skin is loaded with its interpreter");
    runtime.lua().globals().get(name).expect("the skin keeps the flag in a global")
}

/// A press on an object whose event is a function of the skin is taken at once and run in the next
/// frame, in the skin's own interpreter, with the direction of the button as its one argument.
#[test]
fn a_press_on_a_scripted_object_calls_the_skins_function_in_the_next_frame() {
    let (mut app, mut pixels) = app_with_pressed_skin("scripted");

    assert!(app.shared.skin_pointer(over_scripted(), LEFT_PRESS), "the object under the cursor did not take the press");
    assert_eq!(skin_number(&app, "presses"), 0.0, "the function ran outside a frame, where its host is not bound");
    whole_frame(&mut app, &mut pixels);
    assert_eq!((skin_number(&app, "presses"), skin_number(&app, "argument")), (1.0, 1.0));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(skin_number(&app, "presses"), 1.0, "one press ran the function on a second frame");

    assert!(app.shared.skin_pointer(over_scripted(), RIGHT_PRESS));
    whole_frame(&mut app, &mut pixels);
    assert_eq!((skin_number(&app, "presses"), skin_number(&app, "argument")), (2.0, -1.0), "the right button asks for the previous");

    let runtime = app.shared.skins.document(SKIN_TYPE_DECIDE).and_then(LoadedSkin::runtime).expect("the pressed skin is loaded with its interpreter");
    assert_eq!(runtime.diagnostics().function_failures, Vec::new());
}

/// A press on an object whose event is an id is handed to the host and reaches the screens that own
/// the event once the frame has ended; an id the reference defines nothing for takes the press all
/// the same and then comes to nothing.
#[test]
fn a_press_on_a_numbered_object_reaches_the_screen_that_owns_the_event_and_an_undefined_one_nobody() {
    let (mut app, mut pixels) = app_with_pressed_skin("numbered");
    let replay = ClusterRequest::Event { id: REPLAY_EVENT, arg1: 1, arg2: 0 };

    assert!(app.shared.skin_pointer(over_numbered(), LEFT_PRESS));
    assert!(app.shared.skin_requests().is_empty(), "the event was carried out before its frame");
    whole_frame(&mut app, &mut pixels);
    assert_eq!(app.shared.skin_requests().take(Cluster::Result), std::slice::from_ref(&replay));
    assert_eq!(app.shared.skin_requests().take(Cluster::Result), [], "a request is taken once");
    assert_eq!(app.shared.skin_requests().take(Cluster::Select), [replay], "the browser answers the same event its own way");

    assert!(app.shared.skin_pointer(over_numbered(), LEFT_PRESS));
    whole_frame(&mut app, &mut pixels);
    whole_frame(&mut app, &mut pixels);
    assert!(app.shared.skin_requests().is_empty(), "a request no screen took outlived the frame after it");

    assert!(app.shared.skin_pointer(over_unknown(), LEFT_PRESS), "an object takes the press whatever its event comes to");
    whole_frame(&mut app, &mut pixels);
    assert!(app.shared.skin_requests().is_empty(), "an event nobody defines reached a screen");
}

/// A slider is set by a press on its travel and by a drag along it. One that writes through a
/// function has it called with the value; one that writes a rate hands the value to the settings.
#[test]
fn a_slider_is_set_by_a_press_and_by_a_drag_and_writes_where_its_document_says() {
    let (mut app, mut pixels) = app_with_pressed_skin("sliders");

    assert!(app.shared.skin_pointer(over_computed_quarter(), PointerInput::Drag));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(skin_number(&app, "written"), 0.25);

    app.shared.config.audio.system = 0.9;
    assert!(app.shared.skin_pointer(over_volume_middle(), LEFT_PRESS));
    whole_frame(&mut app, &mut pixels);
    assert_eq!(app.shared.config.audio.system, 0.5, "the volume the slider writes is carried out when the frame ends");
    assert!(app.shared.skin_requests().is_empty(), "and is not left waiting for a screen to answer it");

    assert!(!app.shared.skin_pointer(over_scripted(), PointerInput::Drag), "a drag moved something that is not a slider");
    whole_frame(&mut app, &mut pixels);
    assert_eq!(skin_number(&app, "presses"), 0.0);
}

/// What a skin's objects do not take is still the screen's: a press over nothing, a button coming
/// back up, the wheel, and anything over a region the application itself made clickable. Nothing is
/// judged against a skin that is no longer the one on show.
#[test]
fn an_event_no_object_takes_is_left_to_the_screen() {
    let (mut app, mut pixels) = app_with_pressed_skin("left");

    assert!(!app.shared.skin_pointer(over_nothing(), LEFT_PRESS));
    assert!(!app.shared.skin_pointer(over_scripted(), LEFT_RELEASE), "letting go pressed an object");
    assert!(!app.shared.skin_pointer(over_scripted(), PointerInput::Scroll { lines: 1.0 }), "the wheel pressed an object");

    app.shared.hot.push((crate::Rect::new(0.0, 0.0, UI_SIZE.0 as f32, UI_SIZE.1 as f32), crate::Hot::SelectRow(0)));
    assert!(!app.shared.skin_pointer(over_scripted(), LEFT_PRESS), "a skin took a press on something of the application's drawn over it");
    app.shared.hot.clear();
    assert!(app.shared.skin_pointer(over_scripted(), LEFT_PRESS));

    app.shared.finish_skin_frame(&mut Canvas::Headless(&mut pixels));
    assert!(!app.shared.skin_pointer(over_scripted(), LEFT_PRESS), "a frame no skin drew left its objects pressable");
    whole_frame(&mut app, &mut pixels);
    assert_eq!(skin_number(&app, "presses"), 0.0, "a press made on a frame that was not drawn again was run later");
    assert!(app.shared.skin_pointer(over_scripted(), LEFT_PRESS));

    app.shared.begin_skin_scene();
    assert!(!app.shared.skin_pointer(over_scripted(), LEFT_PRESS), "a scene that was left kept its objects pressable");
}

/// A screen that counts the mouse events it is handed.
#[derive(Default)]
struct Counting {
    seen: usize,
}

impl crate::stage::StageHandler for Counting {
    fn update(&mut self, _ctx: &mut crate::stage::FrameCtx<'_>) -> crate::stage::Transition {
        crate::stage::Transition::Stay
    }

    fn draw(&mut self, _ctx: &mut crate::stage::FrameCtx<'_>, _canvas: &mut Canvas<'_>) {}

    fn handle_key(&mut self, _ctx: &mut crate::stage::FrameCtx<'_>, _key: crate::stage::KeyInput<'_>) -> crate::stage::Transition {
        crate::stage::Transition::Stay
    }

    fn handle_mouse(&mut self, _ctx: &mut crate::stage::FrameCtx<'_>, _at: (f32, f32), _button: MouseButton, _pressed: bool) -> crate::stage::Transition {
        self.seen += 1;
        crate::stage::Transition::Stay
    }

    fn handle_scroll(&mut self, _ctx: &mut crate::stage::FrameCtx<'_>, _lines: f32) -> crate::stage::Transition {
        self.seen += 1;
        crate::stage::Transition::Stay
    }
}

/// The screen underneath is not handed a press an object of its skin took, and is handed every
/// other event as it always was.
#[test]
fn a_screen_is_not_handed_the_press_its_skin_took() {
    let (mut app, mut pixels) = app_with_pressed_skin("routed");
    let mut screen = Counting::default();
    let mut route = |app: &mut crate::App, at: (f32, f32), input: PointerInput| {
        crate::pointer::route_pointer(&mut screen, &mut crate::stage::FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 }, at, input);
        screen.seen
    };

    assert_eq!(route(&mut app, over_scripted(), LEFT_PRESS), 0, "the screen was handed a press its skin took");
    assert_eq!(route(&mut app, over_scripted(), LEFT_RELEASE), 1, "letting go is the screen's");
    assert_eq!(route(&mut app, over_nothing(), LEFT_PRESS), 2, "a press over nothing is the screen's");
    assert_eq!(route(&mut app, over_scripted(), PointerInput::Scroll { lines: 1.0 }), 3, "the wheel is the screen's");

    whole_frame(&mut app, &mut pixels);
    assert_eq!(skin_number(&app, "presses"), 1.0, "the press the skin took was not run");
}

/// A skin that polls a key through `Gdx.input:isKeyPressed` reads the keys the window reported as
/// down, by the code its own `Input.Keys` table gives the key.
#[test]
fn a_skin_polling_a_key_reads_the_keys_that_are_down() {
    let (mut app, mut pixels) = app_with_pressed_skin("keys");
    assert!(!skin_flag(&app, "right_held"));

    app.shared.note_key(&crate::stage::KeyInput { code: crate::KeyCode::ArrowRight, pressed: true, released: false, text: None });
    whole_frame(&mut app, &mut pixels);
    assert!(skin_flag(&app, "right_held"), "the skin did not see the right arrow go down");

    app.shared.note_key(&crate::stage::KeyInput { code: crate::KeyCode::ArrowLeft, pressed: true, released: false, text: None });
    app.shared.note_key(&crate::stage::KeyInput { code: crate::KeyCode::ArrowRight, pressed: false, released: true, text: None });
    whole_frame(&mut app, &mut pixels);
    assert!(!skin_flag(&app, "right_held"), "another key being down read as the right arrow");
}

/// Environment variable naming the folder the published-pack test below saves its two frames in.
const CAPTURE_DIR_ENV: &str = "RBMS_SKIN_CAPTURE_DIR";

/// The seed the published pack's random choices are pinned to, so two runs draw the same files.
const PACK_SEED: u64 = 1;

/// How long the published pack's result skin is given to be read and decoded.
const PACK_LOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// How far into its scene the result screen is drawn, which is past its opening, and how long after
/// the press the second frame is drawn, which is past the fade a menu comes in with.
const PACK_SETTLED: Duration = Duration::from_secs(8);
const PACK_SWITCHED: Duration = Duration::from_secs(3);

/// The size the published pack is authored at, and the region its menu switch is drawn in there,
/// measured up from the bottom: `(x, y, w, h)`.
const PACK_SIZE: (u32, u32) = (1920, 1080);
const PACK_MENU_SWITCH: (f32, f32, f32, f32) = (46.0, 601.0, 234.0, 43.0);

/// The panel the two menus are drawn in, in the pack's own pixels measured down from the top:
/// columns and rows.
const PACK_MENU_COLUMNS: std::ops::Range<u32> = 35..700;
const PACK_MENU_ROWS: std::ops::Range<u32> = 428..1010;

/// The share of the menu panel's pixels that has to change for the menu to count as switched.
const PACK_MENU_CHANGE: f64 = 0.05;

/// A finished run for the published pack's result screen to report.
fn pack_result_view() -> ResultView {
    ResultView {
        title: "snapshot".into(),
        artist: String::new(),
        mode_label: "7K",
        counts: [3, 2, 1, 0, 0, 0],
        ex_score: 8,
        max_score: 12,
        max_combo: 5,
        total_notes: 6,
        fast: [1, 0],
        slow: [1, 0],
        gauge: 80.0,
        clear_label: "CLEAR",
        clear_color: Color::GREEN,
        prev_best_ex: Some(6),
        prev_ex: Some(4),
        show_graph: true,
        show_result_graphs: true,
        gauge_series: Vec::new(),
        timing_hist: Box::new([]),
        judge_dist: [0; 6],
    }
}

/// One frame of whichever screen is up, as the frame loop makes it.
fn stage_frame(app: &mut crate::App, pixels: &mut crate::stage::HeadlessCanvas) {
    app.shared.hot.clear();
    let mut canvas = Canvas::Headless(pixels);
    let mut ctx = crate::stage::FrameCtx { shared: &mut app.shared, now: Instant::now(), dt: 0.0 };
    app.stage.draw(&mut ctx, &mut canvas);
    app.shared.finish_skin_frame(&mut canvas);
}

/// Saves a frame under `name` in the folder [`CAPTURE_DIR_ENV`] names, when it names one.
fn save_capture(name: &str, pixels: &crate::stage::HeadlessCanvas) {
    let Some(folder) = std::env::var_os(CAPTURE_DIR_ENV).map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&folder).expect("the capture folder is writable");
    let frame = image::RgbaImage::from_raw(PACK_SIZE.0, PACK_SIZE.1, pixels.rgba().to_vec()).expect("the canvas holds one frame of the pack's size");
    frame.save(folder.join(format!("{name}.png"))).expect("the capture is written");
}

/// How many pixels of the menu panel differ between two frames of the published pack.
fn menu_panel_difference(one: &[u8], other: &[u8]) -> usize {
    let pixel = rbms_render::BYTES_PER_PIXEL;
    let row_bytes = PACK_SIZE.0 as usize * pixel;
    PACK_MENU_ROWS
        .flat_map(|row| PACK_MENU_COLUMNS.map(move |column| row as usize * row_bytes + column as usize * pixel))
        .filter(|at| one[*at..*at + pixel] != other[*at..*at + pixel])
        .count()
}

/// A press on the menu switch of a published result skin switches the menu it shows: the press is
/// taken by the object the skin drew there, the function the skin gave it runs in the skin's own
/// interpreter, and the next frames draw the other menu. Holding the right arrow, which the skin
/// polls on every frame, brings the first menu back.
///
/// The pack is whatever [`SKIN_PACK_ENV`](crate::skin_select::SKIN_PACK_ENV) names; without it the
/// test passes at once. The region pressed is where the pack this was written against draws its
/// switch, so another pack is reported and passed over rather than failed.
#[test]
fn a_press_on_a_published_result_skin_s_menu_switch_switches_its_menu() {
    let Some(pack) = std::env::var_os(crate::skin_select::SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-screen-pack-press-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the settings folder is writable");
    let settings = home.join("settings.ron");
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), settings.clone());
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    app.stage = crate::stage::Stage::Result(crate::stage::ResultState::new(pack_result_view()).cleared(true));

    let mut pixels = crate::stage::HeadlessCanvas::new(PACK_SIZE.0, PACK_SIZE.1);
    let began = Instant::now();
    while !app.shared.has_compiled_skin(SKIN_TYPE_RESULT) {
        stage_frame(&mut app, &mut pixels);
        if let Some(reason) = app.shared.skin_failure(SKIN_TYPE_RESULT) {
            println!("the pack's result skin was not read: {reason}");
            return;
        }
        if began.elapsed() > PACK_LOAD_TIMEOUT {
            println!("the pack's result skin was not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs());
            return;
        }
        std::thread::sleep(DECODE_FRAME_PAUSE);
    }
    app.shared.age_skin_scene(PACK_SETTLED);
    stage_frame(&mut app, &mut pixels);
    save_capture("result-menu-before", &pixels);
    let before = pixels.rgba().to_vec();

    let (x, y, w, h) = PACK_MENU_SWITCH;
    let middle = (x + w / 2.0, y + h / 2.0);
    let cursor = (middle.0 / PACK_SIZE.0 as f32 * UI_SIZE.0 as f32, (PACK_SIZE.1 as f32 - middle.1) / PACK_SIZE.1 as f32 * UI_SIZE.1 as f32);
    if !app.shared.skin_pointer(cursor, LEFT_PRESS) {
        println!("nothing of the pack's result skin takes a press at {middle:?}; it is not the pack this was written against");
        return;
    }
    stage_frame(&mut app, &mut pixels);
    app.shared.age_skin_scene(PACK_SWITCHED);
    stage_frame(&mut app, &mut pixels);
    save_capture("result-menu-after", &pixels);

    let changed = menu_panel_difference(&before, pixels.rgba());
    let panel = PACK_MENU_ROWS.len() * PACK_MENU_COLUMNS.len();
    println!("the menu panel changed in {changed} of {panel} pixels");
    assert!(changed as f64 > panel as f64 * PACK_MENU_CHANGE, "the press was taken and the menu panel changed in only {changed} of {panel} pixels");

    let switched = pixels.rgba().to_vec();
    app.shared.note_key(&crate::stage::KeyInput { code: crate::KeyCode::ArrowRight, pressed: true, released: false, text: None });
    stage_frame(&mut app, &mut pixels);
    app.shared.note_key(&crate::stage::KeyInput { code: crate::KeyCode::ArrowRight, pressed: false, released: true, text: None });
    app.shared.age_skin_scene(PACK_SWITCHED);
    stage_frame(&mut app, &mut pixels);
    save_capture("result-menu-keyed", &pixels);
    let keyed = menu_panel_difference(&switched, pixels.rgba());
    println!("the right arrow changed the menu panel in {keyed} of {panel} pixels");
    assert!(keyed as f64 > panel as f64 * PACK_MENU_CHANGE, "the skin polls the right arrow for its first menu, and holding it changed only {keyed} pixels");
}

/// What the player sets the published pack's whole-screen offset to for the capture below: a little
/// to the right and up, and a fifth smaller.
const PACK_WHOLE_STEPS: [(crate::skin_select::OffsetAxis, i32); 4] = [
    (crate::skin_select::OffsetAxis::X, 5),
    (crate::skin_select::OffsetAxis::Y, 5),
    (crate::skin_select::OffsetAxis::W, -20),
    (crate::skin_select::OffsetAxis::H, -20),
];

/// One frame of the published pack's seven-key play screen, a few seconds into its scene, with the
/// whole-screen offset the SKIN tab was stepped to; `None` when the pack's play skin never drew.
fn pack_play_frame(pack: &Path, tag: &str, steps: &[(crate::skin_select::OffsetAxis, i32)]) -> Option<Vec<u8>> {
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-screen-pack-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("the settings folder is writable");
    let settings = home.join("settings.ron");
    let mut config = crate::Config::default();
    config.skin.pack = Some(pack.to_string_lossy().into_owned());
    let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), settings.clone());
    app.shared.skins.pin_seed(Some(PACK_SEED));
    app.shared.skins.rescan(&settings, &app.shared.config);
    step_whole_offset(&mut app, steps);

    let shown = unplayed_run();
    let mut pixels = crate::stage::HeadlessCanvas::new(PACK_SIZE.0, PACK_SIZE.1);
    let began = Instant::now();
    while !play_frame(&mut app, &mut pixels, &shown) {
        if let Some(reason) = app.shared.skin_failure(SKIN_TYPE_PLAY_7KEYS) {
            println!("the pack's play skin was not read: {reason}");
            return None;
        }
        if began.elapsed() > PACK_LOAD_TIMEOUT {
            println!("the pack's play skin was not read within {} seconds", PACK_LOAD_TIMEOUT.as_secs());
            return None;
        }
        std::thread::sleep(DECODE_FRAME_PAUSE);
    }
    app.shared.age_skin_scene(PACK_SETTLED);
    play_frame(&mut app, &mut pixels, &shown);
    save_capture(tag, &pixels);
    Some(pixels.rgba().to_vec())
}

/// A published play skin is drawn whole under the offset the SKIN tab's rows were stepped to: the
/// frame it draws at rest and the one it draws moved and shrunk are saved side by side for a person
/// to look at, and are not the same frame.
///
/// The pack is whatever [`SKIN_PACK_ENV`](crate::skin_select::SKIN_PACK_ENV) names; without it the
/// test passes at once, and a pack with no seven-key play skin is reported and passed over.
#[test]
fn a_published_play_skin_is_moved_and_shrunk_whole_by_the_offset_set_on_the_skin_tab() {
    let Some(pack) = std::env::var_os(crate::skin_select::SKIN_PACK_ENV).map(PathBuf::from) else {
        return;
    };
    let Some(at_rest) = pack_play_frame(&pack, "play-whole-rest", &[]) else {
        return;
    };
    let Some(moved) = pack_play_frame(&pack, "play-whole-offset", &PACK_WHOLE_STEPS) else {
        return;
    };
    assert!(at_rest != moved, "the whole-screen offset left the pack's play screen where it was");
}
