//! A document whose font is a bitmap font, drawn by the application: the font's pages are decoded
//! off the frame loop, only the ones its text needs, and the text is drawn from the frame they
//! arrive on.
//!
//! The font is the renderer's own fixture, `crates/rbms-render/tests/skin/fonts/mini.fnt`: an `A`
//! whose ink is a white box six pixels by eight on the first of two pages, and glyphs nothing here
//! draws on the second.

use rbms_render::skin_render::textures::rgba_bytes;

use super::*;
use crate::stage::HeadlessCanvas;

/// How long a page is given to arrive, and how long a frame that finds it still decoding stands
/// back for the worker pool.
const DECODE_WAIT: Duration = Duration::from_secs(20);
const DECODE_FRAME_PAUSE: Duration = Duration::from_millis(1);

/// The files of the fixture font, and the size of each of its pages.
const FONT_FILES: [&str; 3] = ["mini.fnt", "mini_0.png", "mini_1.png"];
const PAGE_SIZE: (u32, u32) = (32, 16);

/// The size the fixture font was made at, how many times that the text is drawn, and the ink of its
/// `A` at that size.
const FONT_SIZE: u32 = 10;
const ENLARGED: u32 = 4;
const INK: (u32, u32) = (6 * ENLARGED, 8 * ENLARGED);

/// Where the text's destination starts and how far below the top of the screen its top edge is.
const TEXT_LEFT: u32 = 100;
const TEXT_TOP: u32 = 180;

/// The room the text is given.
const TEXT_ROOM: u32 = 400;

/// The ink of the fixture's `A`.
const WHITE: crate::Color = crate::Color::rgb(u8::MAX, u8::MAX, u8::MAX);

/// An app drawing its key configuration screen with a document that shows one `A` in the fixture
/// font, and the canvas the screen is compiled against.
struct FontPack {
    app: crate::App,
    folder: PathBuf,
    pixels: HeadlessCanvas,
}

impl FontPack {
    fn new(tag: &str) -> FontPack {
        rbms_render::font::use_embedded_fonts_only();
        let home = std::env::temp_dir().join(format!("rbms-skin-font-pages-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let folder = home.join("pack");
        std::fs::create_dir_all(&folder).expect("the pack folder is writable");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/rbms-render/tests/skin/fonts");
        for file in FONT_FILES {
            std::fs::copy(fixture.join(file), folder.join(file)).expect("the fixture font is copied");
        }
        let size = FONT_SIZE * ENLARGED;
        let bottom = UI_SIZE.1 - TEXT_TOP - size;
        let document = format!(
            r#"{{
                "type": {SKIN_TYPE_KEY_CONFIG}, "name": "Font pages", "w": {}, "h": {},
                "font": [{{ "id": "0", "path": "mini.fnt" }}],
                "text": [{{ "id": "label", "font": "0", "size": {size}, "constantText": "A" }}],
                "destination": [{{ "id": "label", "dst": [{{ "time": 0, "x": {TEXT_LEFT}, "y": {bottom}, "w": {TEXT_ROOM}, "h": {size} }}] }}]
            }}"#,
            UI_SIZE.0, UI_SIZE.1
        );
        std::fs::write(folder.join("keys.json"), document).expect("the document is written");
        let mut config = crate::Config::default();
        config.skin.pack = Some(folder.to_string_lossy().into_owned());
        let app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
        FontPack { app, folder, pixels: HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1) }
    }

    /// One frame of the key configuration screen, drawn when its document has compiled; answers
    /// whether the document drew it.
    fn frame(&mut self) -> bool {
        let mut canvas = Canvas::Headless(&mut self.pixels);
        self.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
        let drew = self.app.shared.draw_keyconfig_skin(&mut canvas, &[]);
        self.app.shared.finish_skin_frame(&mut canvas);
        drew
    }

    /// Frames until `done` says so of the frame just drawn.
    fn frames_until(&mut self, what: &str, done: impl Fn(&FontPack, bool) -> bool) {
        let began = Instant::now();
        loop {
            let drew = self.frame();
            if done(self, drew) {
                return;
            }
            assert!(began.elapsed() <= DECODE_WAIT, "{what} never happened: {:?}", self.app.shared.skin_failure(SKIN_TYPE_KEY_CONFIG));
            std::thread::sleep(DECODE_FRAME_PAUSE);
        }
    }

    /// Whether the middle of where the `A` is drawn is inked.
    fn inked(&self) -> bool {
        self.pixels.pixel_at(TEXT_LEFT + INK.0 / 2, TEXT_TOP + INK.1 / 2) == WHITE
    }

    /// Whether the texture pool holds the page called `name`.
    fn uploaded(&self, name: &str) -> bool {
        let pool = &self.app.shared.skin_screens.textures;
        self.app
            .shared
            .skins
            .document(SKIN_TYPE_KEY_CONFIG)
            .is_some_and(|document| document.fonts.values().filter_map(|font| font.parent()).any(|beside| pool.contains(&beside.join(name))))
    }
}

impl Drop for FontPack {
    fn drop(&mut self) {
        if let Some(home) = self.folder.parent() {
            let _ = std::fs::remove_dir_all(home);
        }
    }
}

/// The first frame a document draws has no page to draw its text from, and says which page it
/// needs. A worker decodes that page and no other, the text is there from the frame it arrives on,
/// and the page counts among the screen's textures.
#[test]
fn a_bitmap_fonts_page_is_decoded_off_the_frame_loop_and_only_when_text_needs_it() {
    let mut pack = FontPack::new("lazy");
    pack.frames_until("the document compiling", |_, drew| drew);
    assert!(!pack.inked(), "the text was drawn before any page was asked for");
    assert_eq!(pack.app.shared.skin_screens.texture_stats().count, 0, "a page was uploaded before any text needed it");

    pack.frames_until("the page arriving", |pack, _| pack.inked());
    assert!(pack.uploaded(FONT_FILES[1]), "the page the glyph is on is not uploaded");
    assert!(!pack.uploaded(FONT_FILES[2]), "a page no glyph was drawn from is uploaded");
    assert_eq!(pack.app.shared.skin_screens.texture_stats().bytes, rgba_bytes(PAGE_SIZE));
    assert!(pack.app.shared.skin_screens.font_pages.is_empty(), "nothing is left being decoded");
    assert!(
        pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG).iter().all(|warning| warning.contains("font \"0\": line")),
        "only the fixture's own broken lines are said"
    );
}

/// A page that will not decode is answered for all the same: the text that needed it stops waiting
/// and the screen says what it went without.
#[test]
fn a_page_that_will_not_decode_stops_being_waited_for() {
    let mut pack = FontPack::new("broken");
    std::fs::write(pack.folder.join(FONT_FILES[1]), b"not a picture").expect("the page is written over");
    pack.frames_until("the document compiling", |_, drew| drew);
    pack.frames_until("the page being given up", |pack, _| {
        pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG).iter().any(|warning| warning.contains("font page"))
    });

    assert!(!pack.inked());
    assert!(!pack.uploaded(FONT_FILES[1]));
    assert!(pack.app.shared.skin_screens.font_pages.is_empty(), "the page is asked for again and again");
    pack.frame();
    assert!(pack.app.shared.skin_screens.font_pages.is_empty(), "the page is asked for again once it was given up");
}
