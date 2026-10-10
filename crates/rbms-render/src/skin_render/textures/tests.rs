//! Which of a document's image sources a screen takes, what it refuses, and how the pool shares and
//! frees them.
//!
//! Every document here is written to a scratch folder and read by the real loader, so "referenced"
//! is decided against the destinations the loader actually assembled. The images are never on disk:
//! the host below hands over a flat sheet of whatever size the test gave the file, and remembers
//! every file it was asked for, which is what a decode costs.

use std::path::PathBuf;

use rbms_model::Mode;
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin_with_host};
use rbms_skin::property::{DefaultState, MapHost, SkinHost};

use super::*;
use crate::font::TextContext;
use crate::skin_render::SkinScreen;
use crate::{Color, CpuCanvas, QuadParams, Rect};

/// Edge of the canvas the screens are built on, and of the documents drawn on it.
const CANVAS_EDGE: u32 = 64;

/// Edge of a sheet nothing about a test depends on the size of.
const SHEET_EDGE: u32 = 4;

/// The longest texture edge the capped renderer admits.
const CAPPED_EDGE: u32 = 8;

/// Edge of a sheet the capped renderer cannot hold.
const OVERSIZE_EDGE: u32 = 16;

/// The option the documents' one customisation row grants, and the one it does not.
const GRANTED_OPTION: i32 = 901;
const WITHHELD_OPTION: i32 = 902;

/// An option of the game's own that a host settles once, when the skin is read.
const SETTLED_OPTION: i32 = 40;

/// What a sheet is filled with before and after its file is "edited".
const FIRST_FILL: u8 = 40;
const SECOND_FILL: u8 = 200;

/// A folder of the test's own, removed when the test is done with it.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-textures-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }

    /// Writes a document and a placeholder for each file it names, and reads it against `host`.
    fn load(&self, name: &str, document: &str, files: &[&str], host: &dyn SkinHost) -> LoadedSkin {
        for file in files {
            std::fs::write(self.root.join(file), b"sheet").expect("the scratch file is writable");
        }
        let path = self.root.join(name);
        std::fs::write(&path, document).expect("the scratch document is writable");
        let user = SkinUserConfig::default();
        let options = SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&self.root, &user, Mode::BEAT_7K) };
        load_skin_with_host(&path, options, host).expect("the scratch document loads")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A host that decodes nothing: every file is a flat sheet of the size the test gave it.
struct Sheets {
    /// Every file asked for, in the order it was asked for.
    asked: Vec<String>,
    /// The edge of each file's sheet, by file name; a file not listed is [`SHEET_EDGE`] square.
    edges: Vec<(&'static str, u32)>,
    /// Files there is nothing to hand over for, as if their decode had failed or never ran.
    absent: Vec<&'static str>,
    fill: u8,
}

impl Sheets {
    fn new() -> Sheets {
        Sheets { asked: Vec::new(), edges: Vec::new(), absent: Vec::new(), fill: FIRST_FILL }
    }

    fn sized(edges: &[(&'static str, u32)]) -> Sheets {
        Sheets { edges: edges.to_vec(), ..Sheets::new() }
    }
}

impl SkinAssets for Sheets {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned();
        self.asked.push(name.clone());
        if self.absent.contains(&name.as_str()) {
            return None;
        }
        let edge = self.edges.iter().find(|(file, _)| *file == name).map_or(SHEET_EDGE, |(_, edge)| *edge);
        let pixel = [self.fill, self.fill, self.fill, u8::MAX];
        SkinImage::new(edge, edge, pixel.repeat((edge * edge) as usize))
    }
}

/// A canvas that refuses a texture past [`CAPPED_EDGE`], the way an adapter with a small limit does.
struct Capped(CpuCanvas);

impl Renderer for Capped {
    fn size(&self) -> (u32, u32) {
        self.0.size()
    }

    fn clear(&mut self, color: Color) {
        self.0.clear(color);
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.0.fill_rect(rect, color);
    }

    fn register_texture(&mut self, key: &str, rgba: &[u8], width: u32, height: u32) -> TextureId {
        self.0.register_texture(key, rgba, width, height)
    }

    fn release_texture(&mut self, tex: TextureId) {
        self.0.release_texture(tex);
    }

    fn texture_size(&self, tex: TextureId) -> Option<(u32, u32)> {
        self.0.texture_size(tex)
    }

    fn draw_textured_quad(&mut self, tex: TextureId, params: QuadParams) {
        self.0.draw_textured_quad(tex, params);
    }

    fn push_clip(&mut self, rect: Rect) {
        self.0.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.0.pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        CAPPED_EDGE
    }
}

fn canvas() -> CpuCanvas {
    CpuCanvas::new(CANVAS_EDGE, CANVAS_EDGE)
}

/// One image definition cut from the whole of a [`SHEET_EDGE`] sheet.
fn image(id: &str, src: &str) -> String {
    format!(r#"{{ "id": "{id}", "src": "{src}", "x": 0, "y": 0, "w": {SHEET_EDGE}, "h": {SHEET_EDGE} }}"#)
}

/// One destination over the whole document, gated on `op` when there is one.
fn destination(id: &str, op: Option<i32>) -> String {
    let gate = op.map_or(String::new(), |op| format!(r#""op": [{op}], "#));
    format!(r#"{{ "id": "{id}", {gate}"dst": [{{ "time": 0, "x": 0, "y": 0, "w": {CANVAS_EDGE}, "h": {CANVAS_EDGE} }}] }}"#)
}

/// A decide-screen document with one customisation row, granting [`GRANTED_OPTION`].
fn document(sources: &[(&str, &str)], images: &[String], extra: &str, destinations: &[String]) -> String {
    let sources: Vec<String> = sources.iter().map(|(id, path)| format!(r#"{{ "id": "{id}", "path": "{path}" }}"#)).collect();
    format!(
        r#"{{
            "type": 6, "name": "textures", "w": {CANVAS_EDGE}, "h": {CANVAS_EDGE},
            "property": [{{ "name": "Row", "item": [{{ "name": "on", "op": {GRANTED_OPTION} }}, {{ "name": "off", "op": {WITHHELD_OPTION} }}], "def": "on" }}],
            "source": [{}],
            "image": [{}],
            {extra}
            "destination": [{}]
        }}"#,
        sources.join(", "),
        images.join(", "),
        destinations.join(", ")
    )
}

/// A document that draws one image from each of `files`, the source and the object named for it.
fn plain_document(files: &[&str]) -> String {
    let sources: Vec<(&str, &str)> = files.iter().map(|file| (*file, *file)).collect();
    let images: Vec<String> = files.iter().map(|file| image(file, file)).collect();
    let destinations: Vec<String> = files.iter().map(|file| destination(file, None)).collect();
    document(&sources, &images, "", &destinations)
}

/// A source that is only declared costs nothing: the host is asked for the files the assembled
/// destinations draw from -- directly, or through an image set -- and for no other.
#[test]
fn only_the_sources_a_drawn_object_names_are_asked_for() {
    let scratch = Scratch::new("referenced");
    let body = document(
        &[("shown", "shown.tex"), ("spare", "spare.tex"), ("member", "member.tex"), ("orphan", "orphan.tex")],
        &[image("plain", "shown"), image("idle", "spare"), image("picked-one", "member"), image("unpicked", "orphan")],
        r#""imageset": [{ "id": "picked", "images": ["picked-one"] }, { "id": "unplaced", "images": ["unpicked"] }],"#,
        &[destination("plain", None), destination("picked", None)],
    );
    let skin = scratch.load("skin.json", &body, &["shown.tex", "spare.tex", "member.tex", "orphan.tex"], &DefaultState);

    let referenced: Vec<String> = referenced_sources(&skin).into_iter().collect();
    assert_eq!(referenced, ["member", "shown"], "a source no drawn object reads was kept, or one that is read was dropped");
    assert_eq!(referenced_source_files(&skin).len(), 2);

    let mut assets = Sheets::new();
    let mut screen = SkinScreen::build(&mut canvas(), &mut TextContext::embedded_only(), &skin, &mut assets);
    assets.asked.sort();
    assert_eq!(assets.asked, ["member.tex", "shown.tex"], "a file the screen does not draw from was decoded");
    assert_eq!(screen.texture_stats(), TextureStats { count: 2, bytes: 2 * rgba_bytes((SHEET_EDGE, SHEET_EDGE)) });
    assert_eq!(screen.object_count(), 2);
    assert!(screen.warnings().is_empty(), "{:?}", screen.warnings());
    screen.release(&mut canvas());
}

/// An object the loader removed while it prepared the skin -- it asked for an option the player's
/// customisation does not grant, or for one the game settles once and that came out false -- is not
/// drawn, so its source is not read either. The same object left in keeps its source.
#[test]
fn an_object_removed_while_the_skin_was_prepared_does_not_keep_its_source() {
    let scratch = Scratch::new("removed");
    let body = document(
        &[("always", "always.tex"), ("withheld", "withheld.tex"), ("settled", "settled.tex")],
        &[image("always", "always"), image("withheld", "withheld"), image("settled", "settled")],
        "",
        &[destination("always", Some(GRANTED_OPTION)), destination("withheld", Some(WITHHELD_OPTION)), destination("settled", Some(SETTLED_OPTION))],
    );
    let files = ["always.tex", "withheld.tex", "settled.tex"];

    let mut host = MapHost::default();
    host.static_booleans.insert(SETTLED_OPTION);
    host.booleans.insert(SETTLED_OPTION, false);
    let skin = scratch.load("off.json", &body, &files, &host);
    let mut assets = Sheets::new();
    let screen = SkinScreen::build(&mut canvas(), &mut TextContext::embedded_only(), &skin, &mut assets);
    assert_eq!(assets.asked, ["always.tex"], "the source of an object that is never drawn was decoded");
    assert_eq!(screen.object_count(), 1);

    host.booleans.insert(SETTLED_OPTION, true);
    let skin = scratch.load("on.json", &body, &files, &host);
    let mut assets = Sheets::new();
    SkinScreen::build(&mut canvas(), &mut TextContext::embedded_only(), &skin, &mut assets);
    assets.asked.sort();
    assert_eq!(assets.asked, ["always.tex", "settled.tex"], "an object whose settled option holds lost its source");
}

/// Two source ids that resolve to one file are one decode and one texture, and both draw.
#[test]
fn a_file_two_sources_name_is_decoded_and_counted_once() {
    let scratch = Scratch::new("twice");
    let body = document(
        &[("first", "sheet.tex"), ("second", "sheet.tex")],
        &[image("one", "first"), image("two", "second")],
        "",
        &[destination("one", None), destination("two", None)],
    );
    let skin = scratch.load("skin.json", &body, &["sheet.tex"], &DefaultState);

    let mut target = canvas();
    let mut assets = Sheets::new();
    let mut screen = SkinScreen::build(&mut target, &mut TextContext::embedded_only(), &skin, &mut assets);
    assert_eq!(assets.asked, ["sheet.tex"]);
    assert_eq!(screen.texture_stats().count, 1);
    assert_eq!(screen.object_count(), 2, "the second source of a shared file lost its object");
    assert_eq!(target.live_texture_count(), 1);
    screen.release(&mut target);
    assert_eq!(target.live_texture_count(), 0);
}

/// An image longer on an edge than the renderer can hold is never registered: the screen says so
/// and goes without the objects that drew from it. Against a pool the same file is said once,
/// however many screens name it afterwards.
#[test]
fn an_image_past_the_renderers_edge_is_left_out_with_one_warning() {
    let scratch = Scratch::new("oversize");
    let skin = scratch.load("skin.json", &plain_document(&["small.tex", "huge.tex"]), &["small.tex", "huge.tex"], &DefaultState);
    let edges = [("huge.tex", OVERSIZE_EDGE)];

    let mut target = Capped(canvas());
    let alone = SkinScreen::build(&mut target, &mut TextContext::embedded_only(), &skin, &mut Sheets::sized(&edges));
    let said: Vec<&String> = alone.warnings().iter().filter(|line| line.contains("past the")).collect();
    assert_eq!(said.len(), 1, "{:?}", alone.warnings());
    assert!(said[0].contains("16x16") && said[0].contains("huge.tex"), "{}", said[0]);
    assert_eq!(alone.texture_stats().count, 1);
    assert_eq!(target.0.live_texture_count(), 1, "the oversize image was registered anyway");

    let mut pool = SkinTexturePool::new();
    let first = SkinScreen::build_shared(&mut target, &mut TextContext::embedded_only(), &skin, &mut Sheets::sized(&edges), &mut pool);
    assert_eq!(first.warnings().iter().filter(|line| line.contains("past the")).count(), 1);
    assert!(pool.is_refused(&scratch.root.join("huge.tex")) && !pool.contains(&scratch.root.join("huge.tex")));

    let mut later = Sheets::sized(&edges);
    later.absent = vec!["huge.tex"];
    let second = SkinScreen::build_shared(&mut target, &mut TextContext::embedded_only(), &skin, &mut later, &mut pool);
    let about_it: Vec<&String> = second.warnings().iter().filter(|line| line.contains("\"huge.tex\"")).collect();
    assert!(about_it.iter().all(|line| line.contains("no usable source")), "the oversize file was said a second time: {about_it:?}");
    assert_eq!(second.texture_stats().count, 1);
}

/// A screen's textures are held to a byte budget, spent in the order of the source ids: the source
/// that would go past it is left out with a warning, and a smaller one after it still fits.
#[test]
fn a_source_that_does_not_fit_the_budget_is_left_out_and_a_smaller_one_after_it_still_fits() {
    let scratch = Scratch::new("budget");
    let files = ["a-first.tex", "b-large.tex", "c-small.tex"];
    let skin = scratch.load("skin.json", &plain_document(&files), &files, &DefaultState);
    let edges = [("a-first.tex", CAPPED_EDGE), ("b-large.tex", OVERSIZE_EDGE), ("c-small.tex", SHEET_EDGE)];
    let budget = rgba_bytes((CAPPED_EDGE, CAPPED_EDGE)) + rgba_bytes((SHEET_EDGE, SHEET_EDGE));

    let mut target = canvas();
    let mut pool = SkinTexturePool::with_budget(budget);
    let mut screen = SkinScreen::build_shared(&mut target, &mut TextContext::embedded_only(), &skin, &mut Sheets::sized(&edges), &mut pool);
    assert_eq!(screen.texture_stats(), TextureStats { count: 2, bytes: budget }, "{:?}", screen.warnings());
    let said: Vec<&String> = screen.warnings().iter().filter(|line| line.contains("would take this skin's textures past")).collect();
    assert_eq!(said.len(), 1, "{:?}", screen.warnings());
    assert!(said[0].contains("b-large.tex"), "{}", said[0]);
    assert!(!pool.contains(&scratch.root.join("b-large.tex")), "a source that was left out was uploaded anyway");
    assert_eq!(pool.stats(), TextureStats { count: 2, bytes: budget });
    assert_eq!(screen.object_count(), 2);

    screen.release_shared(&mut target, &mut pool);
    pool.sweep(&mut target);
    assert_eq!(target.live_texture_count(), 0);
}

/// Two screens built against one pool hold one texture for a file they both draw from, and the
/// second is built without the file being decoded for it. Releasing a screen frees nothing by
/// itself: the texture goes when nobody holds it and the pool is swept.
#[test]
fn two_screens_share_one_texture_for_a_file_and_it_goes_with_the_last_of_them() {
    let scratch = Scratch::new("shared");
    let select = scratch.load("select.json", &plain_document(&["common.tex", "select.tex"]), &["common.tex", "select.tex"], &DefaultState);
    let decide = scratch.load("decide.json", &plain_document(&["common.tex", "decide.tex"]), &["decide.tex"], &DefaultState);
    let common = scratch.root.join("common.tex");

    let mut target = canvas();
    let mut pool = SkinTexturePool::new();
    let mut first = SkinScreen::build_shared(&mut target, &mut TextContext::embedded_only(), &select, &mut Sheets::new(), &mut pool);
    assert_eq!(pool.stats().count, 2);

    let mut undecoded = Sheets::new();
    undecoded.absent = vec!["common.tex"];
    let mut second = SkinScreen::build_shared(&mut target, &mut TextContext::embedded_only(), &decide, &mut undecoded, &mut pool);
    assert!(second.warnings().is_empty(), "a file the pool already held was missed: {:?}", second.warnings());
    assert_eq!(second.object_count(), 2);
    assert_eq!(pool.holders(&common), 2);
    assert_eq!(pool.stats().count, 3, "the shared file was uploaded twice");
    assert_eq!(target.live_texture_count(), 3);

    first.release_shared(&mut target, &mut pool);
    assert_eq!(target.live_texture_count(), 3, "releasing a screen freed a texture before the sweep");
    assert_eq!(pool.unheld(), 1);
    assert_eq!(pool.sweep(&mut target), 1, "only the file the first screen had to itself goes");
    assert!(pool.contains(&common) && !pool.contains(&scratch.root.join("select.tex")));
    assert_eq!(target.live_texture_count(), 2);

    second.release_shared(&mut target, &mut pool);
    assert_eq!(pool.sweep(&mut target), 2);
    assert_eq!(pool.stats(), TextureStats::default());
    assert_eq!(target.live_texture_count(), 0, "a swept pool left a texture uploaded");
}

/// A texture nobody holds waits for the sweep, and a hold taken in the meantime keeps it: that is
/// how the screen being entered takes over a file from the screen that was just let go, without the
/// file being decoded again.
#[test]
fn a_released_texture_can_be_taken_over_before_the_sweep() {
    let scratch = Scratch::new("takeover");
    let skin = scratch.load("skin.json", &plain_document(&["kept.tex", "dropped.tex"]), &["kept.tex", "dropped.tex"], &DefaultState);
    let kept = scratch.root.join("kept.tex");

    let mut target = canvas();
    let mut pool = SkinTexturePool::new();
    let mut screen = SkinScreen::build_shared(&mut target, &mut TextContext::embedded_only(), &skin, &mut Sheets::new(), &mut pool);
    screen.release_shared(&mut target, &mut pool);
    assert_eq!(pool.unheld(), 2);

    assert!(pool.hold(&kept), "a texture waiting to be swept could not be held");
    assert!(!pool.hold(&scratch.root.join("never.tex")), "a file that was never uploaded was held");
    assert_eq!(pool.sweep(&mut target), 1);
    assert!(pool.contains(&kept));

    pool.release(&kept);
    pool.release(&kept);
    assert_eq!(pool.holders(&kept), 0, "giving a hold back twice went below none");
    pool.sweep(&mut target);
    assert_eq!(target.live_texture_count(), 0);
}

/// A file the host decoded again replaces the pixels the pool held, under the handle every screen
/// drawing from it already has.
#[test]
fn a_file_decoded_again_replaces_what_the_pool_held_of_it() {
    let scratch = Scratch::new("redecoded");
    let skin = scratch.load("skin.json", &plain_document(&["sheet.tex"]), &["sheet.tex"], &DefaultState);

    let mut target = canvas();
    let mut text = TextContext::embedded_only();
    let mut pool = SkinTexturePool::new();
    let first = SkinScreen::build_shared(&mut target, &mut text, &skin, &mut Sheets::new(), &mut pool);
    let mut edited = Sheets::new();
    edited.fill = SECOND_FILL;
    let second = SkinScreen::build_shared(&mut target, &mut text, &skin, &mut edited, &mut pool);
    assert_eq!(pool.stats().count, 1);
    assert_eq!(pool.holders(&scratch.root.join("sheet.tex")), 2);
    assert_eq!(target.live_texture_count(), 1, "the edited file was uploaded beside the old one");

    let timers = rbms_skin::timer::TimerState::new();
    let frame = crate::SkinFrame { now_us: 0, timers: &timers, state: &DefaultState, lua: None, mouse: None, data: crate::FrameData::default() };
    for screen in [&first, &second] {
        target.clear(Color::BLACK);
        let mut ctx = crate::RenderCtx::new(crate::theme(), &mut text);
        screen.draw(&mut ctx, &mut target, &frame);
        assert_eq!(target.pixel_at(CANVAS_EDGE / 2, CANVAS_EDGE / 2), Color::rgb(SECOND_FILL, SECOND_FILL, SECOND_FILL));
    }
}
