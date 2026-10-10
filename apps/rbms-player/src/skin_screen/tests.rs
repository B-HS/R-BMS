use rbms_skin::dst::DrawStateSource;
use rbms_skin::property::generated::{BUTTON_LNMODE, FLOAT_LOADING_PROGRESS, NUMBER_PLAYLEVEL, RATE_LOAD_PROGRESS};
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

    let offsets = DocumentOffsets { document: Some(&document) };
    assert_eq!(offsets.offset(46), Some(nudge), "a row the document carries is read");
    assert_eq!(offsets.offset(48), None, "a row the document does not carry is answered anyway");

    assert_eq!(DocumentOffsets::default().offset(46), None, "a document nobody has customised nudges nothing");
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

/// An app whose skin pack is one folder holding one decide-screen skin, and that skin's path.
fn app_with_decide_skin(tag: &str, source: &str) -> (crate::App, std::path::PathBuf) {
    rbms_render::font::use_embedded_fonts_only();
    let home = std::env::temp_dir().join(format!("rbms-skin-screen-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let pack = home.join("pack");
    std::fs::create_dir_all(&pack).expect("the pack folder is writable");
    let document = pack.join("decide.luaskin");
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

/// The option the decide screen's older state answers while its load is still running.
const OPTION_NOW_LOADING: i32 = 80;

/// The host a screen is drawn through answers from its clusters first, and until those are filled
/// in the state the screen was drawn from before answers whatever they do not know. The scene's
/// timers are the host's own all along.
#[test]
fn an_id_no_cluster_knows_is_answered_by_the_state_the_screen_was_drawn_from_before() {
    let mut timers = TimerState::new();
    timers.set_on(TimerId(RUNNING_TIMER), STARTED_US);
    let chart = DecideChart { artist: "Composer", level: 12, ..DecideChart::default() };
    let older = DecideViewState { progress: 0.5, done: false, title: TITLE, chart, now_us: STARTED_US, offsets: None };

    let mut host = ScreenHost::new(older.now_us(), &timers);
    assert_eq!(host.text(STRING_TITLE), "", "with no state behind it the host knows no title");
    assert_eq!(host.boolean(OPTION_NOW_LOADING), None);

    host.fallback = Some(&older);
    assert_eq!(host.text(STRING_TITLE), TITLE);
    assert_eq!(host.boolean(OPTION_NOW_LOADING), Some(true));
    assert_eq!(host.boolean(-OPTION_NOW_LOADING), Some(false), "a negated read reaches the older state with its sign");
    assert_eq!(host.integer(NUMBER_PLAYLEVEL), 12);
    assert_eq!(host.rate(RATE_LOAD_PROGRESS), Some(0.5));
    assert_eq!(host.float(FLOAT_LOADING_PROGRESS), 0.5);
    assert_eq!(host.image_index(BUTTON_LNMODE), 0, "an image no cluster picks for shows the first of its set, as the older state drew it");

    assert_eq!(older.timer_us(RUNNING_TIMER), TIMER_OFF, "the older state never knew a timer");
    assert_eq!(host.timer_us(RUNNING_TIMER), STARTED_US, "so a script's timer read comes from the scene's own table");
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

/// An image picked by an index no cluster answers yet is still drawn, showing the first of its set:
/// the state the screen was drawn from before stands in for the cluster, and it never hid an image.
#[test]
fn an_image_picked_by_an_index_no_cluster_knows_is_drawn_with_its_first_set() {
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
    assert_eq!(pixels.pixel_at(UI_SIZE.0 / 2, UI_SIZE.1 / 2), FIRST_SET, "the image was not drawn with the first of its set");
}
