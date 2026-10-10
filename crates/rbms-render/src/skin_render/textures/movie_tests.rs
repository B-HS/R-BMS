//! A source that is a movie: what a screen asks of its host for one, when the movie is started,
//! what is drawn before its first frame and after, and what it is counted at.
//!
//! No movie is decoded here. The documents are written to a scratch folder and read by the real
//! loader, so a source is a movie because its file name says so; the host below answers a frame
//! size for it, and the tests hand the screen flat frames of that size themselves.

use std::path::PathBuf;

use rbms_model::Mode;
use rbms_skin::loader::{LoadedSkin, SkinLoadOptions, SkinUserConfig, load_skin};
use rbms_skin::property::DefaultState;
use rbms_skin::timer::TimerState;

use super::*;
use crate::font::TextContext;
use crate::skin_render::SkinScreen;
use crate::{Color, CpuCanvas, FrameData, RenderCtx, SkinFrame};

/// Edge of the canvas the screens are built on, and of the documents drawn on it.
const CANVAS: u32 = 64;

/// The size of the movies' frames.
const FRAME: (u32, u32) = (4, 2);

/// Edge of the stills drawn beside a movie.
const STILL_EDGE: u32 = 4;

/// A timer no test switches on, so a destination that follows it is never placed.
const TIMER_LEFT_OFF: i32 = 40;

/// The frame clock's reading the tests draw their first frame at.
const FIRST_NOW_US: i64 = 5_000_000;

/// A longest texture edge a movie frame does not fit.
const NARROW_EDGE: u32 = 3;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// A folder of the test's own, removed when the test is done with it.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("rbms-skin-movies-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch folder is writable");
        Scratch { root }
    }

    /// Writes a document and a placeholder for each file it names, and reads it.
    fn load(&self, document: &str, files: &[&str]) -> LoadedSkin {
        for file in files {
            std::fs::write(self.root.join(file), b"never decoded").expect("the scratch file is writable");
        }
        let path = self.root.join("skin.json");
        std::fs::write(&path, document).expect("the scratch document is writable");
        let user = SkinUserConfig::default();
        load_skin(&path, SkinLoadOptions { rng_seed: Some(1), ..SkinLoadOptions::new(&self.root, &user, Mode::BEAT_7K) }).expect("the scratch document loads")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A host that plays every movie as one of [`FRAME`]-sized frames, and remembers what it was asked.
#[derive(Default)]
struct Host {
    images_asked: Vec<String>,
    movies_asked: Vec<String>,
    /// The frame size answered, or `None` for a host that cannot open the movie.
    frame: Option<(u32, u32)>,
}

/// Why the host that cannot open a movie says it cannot.
const WHY_NOT: &str = "the file is damaged";

impl Host {
    fn playing() -> Host {
        Host { frame: Some(FRAME), ..Host::default() }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned()
}

impl SkinAssets for Host {
    fn image(&mut self, path: &Path) -> Option<SkinImage> {
        self.images_asked.push(file_name(path));
        SkinImage::new(STILL_EDGE, STILL_EDGE, [9, 9, 9, 255].repeat((STILL_EDGE * STILL_EDGE) as usize))
    }

    fn movie(&mut self, path: &Path) -> Result<(u32, u32), String> {
        self.movies_asked.push(file_name(path));
        self.frame.ok_or_else(|| WHY_NOT.to_owned())
    }
}

/// A canvas whose textures may be no longer than [`NARROW_EDGE`] on an edge.
struct Narrow(CpuCanvas);

impl Renderer for Narrow {
    fn size(&self) -> (u32, u32) {
        self.0.size()
    }

    fn clear(&mut self, color: Color) {
        self.0.clear(color);
    }

    fn fill_rect(&mut self, rect: crate::Rect, color: Color) {
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

    fn draw_textured_quad(&mut self, tex: TextureId, params: crate::QuadParams) {
        self.0.draw_textured_quad(tex, params);
    }

    fn push_clip(&mut self, rect: crate::Rect) {
        self.0.push_clip(rect);
    }

    fn pop_clip(&mut self) {
        self.0.pop_clip();
    }

    fn max_texture_size(&self) -> u32 {
        NARROW_EDGE
    }
}

/// A document of `sources`, with `objects` between its sources and its destinations.
fn document(sources: &[(&str, &str)], objects: &str, destinations: &[String]) -> String {
    let sources: Vec<String> = sources.iter().map(|(id, path)| format!(r#"{{ "id": "{id}", "path": "{path}" }}"#)).collect();
    format!(
        r#"{{ "type": 6, "name": "movies", "w": {CANVAS}, "h": {CANVAS}, "source": [{}], {objects} "destination": [{}] }}"#,
        sources.join(", "),
        destinations.join(", ")
    )
}

/// One image drawn from the whole of its source.
fn image(id: &str, src: &str) -> String {
    format!(r#"{{ "id": "{id}", "src": "{src}", "x": 0, "y": 0, "w": -1, "h": -1 }}"#)
}

/// One destination over the whole document, with `extra` fields ahead of its keyframe.
fn destination(id: &str, extra: &str) -> String {
    format!(r#"{{ "id": "{id}", {extra} "dst": [{{ "time": 0, "x": 0, "y": 0, "w": {CANVAS}, "h": {CANVAS} }}] }}"#)
}

/// A document that draws one movie over everything.
fn movie_document() -> String {
    document(&[("clip", "clip.mp4")], &format!(r#""image": [{}],"#, image("bg", "clip")), &[destination("bg", "")])
}

/// A frame of the movies' size in one colour.
fn flat(pixel: [u8; 4]) -> Vec<u8> {
    pixel.repeat((FRAME.0 * FRAME.1) as usize)
}

/// Draws one frame of `screen` on a cleared canvas with the frame clock at `now_us`, answering how
/// many objects were drawn.
fn draw_at<R: Renderer>(screen: &SkinScreen, target: &mut R, now_us: i64) -> usize {
    let timers = TimerState::new();
    let frame = SkinFrame { now_us, timers: &timers, state: &DefaultState, lua: None, mouse: None, data: FrameData::default() };
    target.clear(Color::BLACK);
    let mut text = TextContext::embedded_only();
    let mut ctx = RenderCtx::new(crate::theme(), &mut text);
    screen.draw(&mut ctx, target, &frame)
}

fn build<R: Renderer>(target: &mut R, skin: &LoadedSkin, host: &mut Host) -> SkinScreen {
    SkinScreen::build(target, &mut TextContext::embedded_only(), skin, host)
}

fn middle(canvas: &CpuCanvas) -> Color {
    canvas.pixel_at(CANVAS / 2, CANVAS / 2)
}

/// A movie is asked of the host as a movie and never as an image, and the files a host decodes
/// ahead of a build do not include it.
#[test]
fn a_movie_source_is_asked_for_as_a_movie_and_never_decoded_as_an_image() {
    let scratch = Scratch::new("asked");
    let body = document(
        &[("clip", "clip.mp4"), ("still", "still.png"), ("spare", "spare.webm")],
        &format!(r#""image": [{}, {}, {}],"#, image("bg", "clip"), image("frame", "still"), image("idle", "spare")),
        &[destination("bg", ""), destination("frame", "")],
    );
    let skin = scratch.load(&body, &["clip.mp4", "still.png", "spare.webm"]);
    let names = |files: BTreeSet<&Path>| files.into_iter().map(file_name).collect::<Vec<_>>();
    assert_eq!(names(referenced_source_files(&skin)), ["still.png"]);
    assert_eq!(names(referenced_movie_files(&skin)), ["clip.mp4"], "a movie nothing draws from was asked for, or the one that is drawn was not");

    let mut host = Host::playing();
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut host);
    assert_eq!(host.images_asked, ["still.png"]);
    assert_eq!(host.movies_asked, ["clip.mp4"]);
    assert_eq!(screen.object_count(), 2);
    assert!(screen.warnings().is_empty(), "{:?}", screen.warnings());
    let movies = screen.movies();
    assert_eq!(movies.len(), 1);
    assert_eq!((movies[0].index, movies[0].size, movies[0].started_us, movies[0].starts), (0, FRAME, None, 0));
    assert_eq!(movies[0].path, scratch.root.join("clip.mp4"));
    screen.release(&mut target);
}

/// The reference leaves an image undrawn while its movie has no frame for it, and the movie is
/// started by the first prepare. A frame handed over is what is drawn from then on.
#[test]
fn a_movie_object_draws_nothing_until_it_has_a_frame_and_its_frame_after() {
    let scratch = Scratch::new("first-frame");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    assert_eq!(target.live_texture_count(), 0, "a movie with no frame yet was given a texture");

    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US), 0, "an object was drawn from a movie with no frame");
    assert_eq!(middle(&target), Color::BLACK);
    let started = &screen.movies()[0];
    assert_eq!((started.started_us, started.starts), (Some(FIRST_NOW_US), 1), "the first prepare did not start the movie");

    assert!(screen.show_movie_frame(&mut target, 0, &flat(RED)));
    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US + 1), 1);
    assert_eq!(middle(&target), Color::rgb(255, 0, 0));
    assert_eq!(screen.movies()[0].started_us, Some(FIRST_NOW_US), "a later frame started the movie again");
    screen.release(&mut target);
}

/// Every frame of a movie goes into the one texture, on the CPU canvas as on any renderer that
/// replaces a key's pixels in place.
#[test]
fn each_frame_replaces_the_last_in_the_same_texture() {
    let scratch = Scratch::new("in-place");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    draw_at(&screen, &mut target, 0);

    assert!(screen.show_movie_frame(&mut target, 0, &flat(RED)));
    let first = screen.textures.movies[0].frame().expect("a frame is on show");
    for (pixel, wanted) in [(BLUE, Color::rgb(0, 0, 255)), (RED, Color::rgb(255, 0, 0)), (BLUE, Color::rgb(0, 0, 255))] {
        assert!(screen.show_movie_frame(&mut target, 0, &flat(pixel)));
        assert_eq!(screen.textures.movies[0].frame(), Some(first), "a new frame was given a new texture");
        assert_eq!(target.live_texture_count(), 1, "the frames of a movie piled up as textures");
        draw_at(&screen, &mut target, 0);
        assert_eq!(middle(&target), wanted);
    }
    assert_eq!(first.1, FRAME);
    assert_eq!(target.texture_size(first.0), Some(FRAME));

    screen.release(&mut target);
    assert_eq!(target.live_texture_count(), 0, "a released screen left its movie's texture uploaded");
    assert!(screen.movies().is_empty());
}

/// A buffer that is not a whole frame is not shown, and neither is a frame for a movie the screen
/// does not have.
#[test]
fn a_frame_of_the_wrong_length_or_for_no_movie_is_refused() {
    let scratch = Scratch::new("refused-frame");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    let whole = flat(RED);
    assert!(!screen.show_movie_frame(&mut target, 0, &whole[..whole.len() - 1]));
    assert!(!screen.show_movie_frame(&mut target, 0, &[]));
    assert!(!screen.show_movie_frame(&mut target, 0, &[whole.clone(), whole.clone()].concat()));
    assert!(!screen.show_movie_frame(&mut target, 1, &whole));
    assert_eq!(draw_at(&screen, &mut target, 0), 0, "a refused frame was put on show");
    assert_eq!(target.live_texture_count(), 0);
    screen.hide_movie(7);
    screen.release(&mut target);
}

/// A movie the host cannot play leaves its image out, says so once, and is not counted.
#[test]
fn a_movie_the_host_cannot_play_drops_its_object_with_a_warning() {
    let scratch = Scratch::new("unplayable");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    for (mut host, why) in [(Host::default(), WHY_NOT), (Host { frame: Some((0, 2)), ..Host::default() }, "0x2")] {
        let mut screen = build(&mut target, &skin, &mut host);
        assert!(screen.warnings().iter().any(|line| line.contains(why)), "the warning does not say why: {:?}", screen.warnings());
        assert_eq!(host.movies_asked, ["clip.mp4"]);
        assert!(host.images_asked.is_empty(), "a movie the host cannot play was tried as an image");
        assert_eq!(screen.object_count(), 0);
        assert!(screen.movies().is_empty());
        assert_eq!(screen.texture_stats(), TextureStats::default());
        assert!(screen.warnings().iter().any(|line| line.contains("movie source") && line.contains("clip.mp4")), "{:?}", screen.warnings());
        assert_eq!(draw_at(&screen, &mut target, 0), 0);
        screen.release(&mut target);
    }

    /// A host that says nothing about movies plays none.
    struct Stills;
    impl SkinAssets for Stills {
        fn image(&mut self, _path: &Path) -> Option<SkinImage> {
            None
        }
    }
    let screen = SkinScreen::build(&mut target, &mut TextContext::embedded_only(), &skin, &mut Stills);
    assert_eq!(screen.object_count(), 0);
    assert!(screen.warnings().iter().any(|line| line.contains("plays no movies")), "{:?}", screen.warnings());
}

/// A movie is one frame against the screen's budget from the moment the screen is built, whether or
/// not a frame has arrived, and a movie that does not fit is left out like an image that does not.
#[test]
fn a_movie_is_counted_as_one_frame_and_held_to_the_budget_and_the_edge_limit() {
    let scratch = Scratch::new("budget");
    let body = document(
        &[("a-still", "a-still.png"), ("clip", "clip.mp4")],
        &format!(r#""image": [{}, {}],"#, image("frame", "a-still"), image("bg", "clip")),
        &[destination("frame", ""), destination("bg", "")],
    );
    let skin = scratch.load(&body, &["a-still.png", "clip.mp4"]);
    let (still, movie) = (rgba_bytes((STILL_EDGE, STILL_EDGE)), rgba_bytes(FRAME));
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut text = TextContext::embedded_only();

    let mut roomy = SkinTexturePool::with_budget(still + movie);
    let mut screen = SkinScreen::build_shared(&mut target, &mut text, &skin, &mut Host::playing(), &mut roomy);
    assert_eq!(screen.texture_stats(), TextureStats { count: 2, bytes: still + movie });
    assert_eq!(roomy.stats().count, 1, "a movie was put in the pool of files screens share");
    assert_eq!(screen.movies().len(), 1);
    screen.release_shared(&mut target, &mut roomy);
    roomy.sweep(&mut target);

    let mut tight = SkinTexturePool::with_budget(still + movie - 1);
    let mut screen = SkinScreen::build_shared(&mut target, &mut text, &skin, &mut Host::playing(), &mut tight);
    assert_eq!(screen.texture_stats(), TextureStats { count: 1, bytes: still }, "a movie past the budget was admitted");
    assert!(screen.movies().is_empty());
    assert_eq!(screen.object_count(), 1);
    assert!(screen.warnings().iter().any(|line| line.contains("clip.mp4") && line.contains("MiB")), "{:?}", screen.warnings());
    screen.release_shared(&mut target, &mut tight);
    tight.sweep(&mut target);

    let mut narrow = Narrow(CpuCanvas::new(CANVAS, CANVAS));
    let movie_only = scratch.load(&movie_document(), &["clip.mp4"]);
    let screen = build(&mut narrow, &movie_only, &mut Host::playing());
    assert!(screen.movies().is_empty(), "a movie wider than the renderer's textures was admitted");
    assert!(screen.warnings().iter().any(|line| line.contains("clip.mp4") && line.contains("past")), "{:?}", screen.warnings());
}

/// The reference asks a movie for a frame whether or not its object is drawn, so an object that is
/// not placed on the first frame has still started its movie.
#[test]
fn an_object_that_is_not_drawn_still_starts_its_movie() {
    let scratch = Scratch::new("undrawn");
    let body =
        document(&[("clip", "clip.mp4")], &format!(r#""image": [{}],"#, image("bg", "clip")), &[destination("bg", &format!(r#""timer": {TIMER_LEFT_OFF},"#))]);
    let skin = scratch.load(&body, &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    assert!(screen.show_movie_frame(&mut target, 0, &flat(RED)));
    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US), 0, "an object whose timer is off was drawn");
    assert_eq!(screen.movies()[0].started_us, Some(FIRST_NOW_US));
    screen.release(&mut target);
}

/// A frame clock that has gone back to before the movie was started is a scene that began again
/// under the same screen: the movie is started over from there, and what was on show is not.
#[test]
fn a_clock_that_goes_back_starts_the_movie_over() {
    let scratch = Scratch::new("restart");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    draw_at(&screen, &mut target, FIRST_NOW_US);
    assert!(screen.show_movie_frame(&mut target, 0, &flat(RED)));
    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US + 10), 1);
    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US), 1, "the moment the movie was started is not before it");

    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US - 1), 0, "the frame of the pass that ended stayed on show");
    let restarted = &screen.movies()[0];
    assert_eq!((restarted.started_us, restarted.starts), (Some(FIRST_NOW_US - 1), 2));
    assert!(screen.show_movie_frame(&mut target, 0, &flat(BLUE)));
    assert_eq!(draw_at(&screen, &mut target, FIRST_NOW_US), 1);
    assert_eq!(middle(&target), Color::rgb(0, 0, 255));
    assert_eq!(target.live_texture_count(), 1);
    screen.release(&mut target);
}

/// A movie the host gave up on draws nothing again, until a frame is shown.
#[test]
fn a_hidden_movie_draws_nothing() {
    let scratch = Scratch::new("hidden");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    assert!(screen.show_movie_frame(&mut target, 0, &flat(RED)));
    assert_eq!(draw_at(&screen, &mut target, 0), 1);
    screen.hide_movie(0);
    assert_eq!(draw_at(&screen, &mut target, 0), 0);
    assert_eq!(middle(&target), Color::BLACK);
    assert!(screen.show_movie_frame(&mut target, 0, &flat(BLUE)));
    assert_eq!(draw_at(&screen, &mut target, 0), 1);
    screen.release(&mut target);
}

/// The reference draws a movie through the linear filter whatever its destination's `filter` says,
/// so a frame stretched over the screen blends where two of its pixels meet.
#[test]
fn a_movie_is_read_through_the_linear_filter_whatever_its_destination_says() {
    let scratch = Scratch::new("filter");
    let skin = scratch.load(&movie_document(), &["clip.mp4"]);
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut Host::playing());
    let halves: Vec<u8> = (0..FRAME.0 * FRAME.1).flat_map(|pixel| if pixel % FRAME.0 < FRAME.0 / 2 { [0, 0, 0, 255] } else { [255, 255, 255, 255] }).collect();
    assert!(screen.show_movie_frame(&mut target, 0, &halves));
    assert_eq!(draw_at(&screen, &mut target, 0), 1);
    let (left, seam, right) = (target.pixel_at(1, CANVAS / 2), middle(&target), target.pixel_at(CANVAS - 2, CANVAS / 2));
    assert_eq!((left.r, right.r), (0, 255), "the frame was not stretched over the whole destination");
    assert!(seam.r > 0 && seam.r < 255, "the seam between two movie pixels is {seam:?}: the frame was read point by point");
    screen.release(&mut target);
}

/// Two source ids that resolve to one file play one movie, and only an image is drawn from a movie:
/// anything else that names one has no source, as in the reference.
#[test]
fn two_sources_of_one_file_share_a_movie_and_only_an_image_draws_from_one() {
    let scratch = Scratch::new("shared");
    let objects = format!(
        r#""image": [{}, {}, {}],
           "imageset": [{{ "id": "picked", "images": ["member"] }}],
           "value": [{{ "id": "digits", "src": "first", "x": 0, "y": 0, "w": 40, "h": 4, "divx": 10, "digit": 2, "ref": 100 }}],"#,
        image("one", "first"),
        image("two", "second"),
        image("member", "first")
    );
    let body = document(
        &[("first", "clip.mp4"), ("second", "clip.mp4")],
        &objects,
        &[destination("one", ""), destination("two", ""), destination("picked", ""), destination("digits", "")],
    );
    let skin = scratch.load(&body, &["clip.mp4"]);
    let mut host = Host::playing();
    let mut target = CpuCanvas::new(CANVAS, CANVAS);
    let mut screen = build(&mut target, &skin, &mut host);
    assert_eq!(host.movies_asked, ["clip.mp4"], "one file was opened once for each source that names it");
    assert_eq!(screen.movies().len(), 1);
    assert_eq!(screen.texture_stats(), TextureStats { count: 1, bytes: rgba_bytes(FRAME) });
    assert_eq!(screen.object_count(), 2, "something other than an image was built from a movie: {:?}", screen.warnings());

    assert!(screen.show_movie_frame(&mut target, 0, &flat(RED)));
    assert_eq!(draw_at(&screen, &mut target, 0), 2, "both images of the one movie show its frame");
    screen.release(&mut target);
}
