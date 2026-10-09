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
            app.shared.draw_decide_skin(&mut canvas, 0.0, false, TITLE, DecideChart::default())
        })
        .collect()
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
