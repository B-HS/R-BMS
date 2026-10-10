//! The movies a screen's document draws from: when one starts, which of its frames is on show as
//! the scene clock moves, what it does while the clock stands still, and what becomes of it when its
//! screen is left.
//!
//! No movie is decoded. A document here names a file that is a script ([`ScriptedMovie`]), which
//! the worker that opens movies opens as a movie of flat frames, each a colour that says which
//! frame it is; so a pixel of the frame drawn says which frame of the movie was on show. A frame of
//! a test waits for the movie frame that is due at its scene time, as a capture does, and every
//! scene time asked for is the middle of the time a movie frame is on show for, a whole half a
//! frame away from the frames beside it.

use rbms_video::VideoError;

use super::*;
use crate::assets::scripted_movie::{self, ScriptedMovie};
use crate::gpu::Gpu;
use crate::stage::HeadlessCanvas;

/// How long a document is given to compile, and how long a frame that finds it still on the worker
/// pool stands back.
const COMPILE_WAIT: Duration = Duration::from_secs(20);
const COMPILE_FRAME_PAUSE: Duration = Duration::from_millis(1);

/// How long a frame waits for the frame of a movie that is due.
const FRAME_PATIENCE: Duration = Duration::from_secs(20);

/// The size of the scripted movies' frames.
const MOVIE_SIZE: (u32, u32) = (4, 2);

/// How many frames a scripted movie has, and how long each is on show: long enough that the
/// moments a real clock adds to a scene time asked for never reach the next frame.
const MOVIE_FRAMES: u32 = 5;
const MOVIE_FRAME_US: i64 = 2_000_000;

/// How long one pass over a scripted movie lasts.
const MOVIE_PASS_US: i64 = MOVIE_FRAMES as i64 * MOVIE_FRAME_US;

/// The scene time the first frame of every test is drawn at, which is when its movies start.
const STARTED_AT_US: i64 = 1_000_000;

/// How many frames are drawn at one scene time to see that the movie does not move.
const STILL_FRAMES: usize = 4;

/// What the frame is cleared to before a document is drawn on it.
const NOTHING_DRAWN: crate::Color = crate::Color::rgb(0, 0, 0);

/// The option the documents' customisation row grants.
const GRANTED_OPTION: i32 = 901;

/// An app drawing its key configuration screen with a document of the test's own.
struct Pack {
    app: crate::App,
    folder: PathBuf,
    pixels: HeadlessCanvas,
}

impl Pack {
    /// A pack whose key configuration document is `document`, with each of `movies` written beside
    /// it as a script.
    fn new(tag: &str, document: &str, movies: &[(&str, ScriptedMovie)]) -> Pack {
        rbms_render::font::use_embedded_fonts_only();
        let home = std::env::temp_dir().join(format!("rbms-skin-movies-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let folder = home.join("pack");
        std::fs::create_dir_all(&folder).expect("the pack folder is writable");
        std::fs::write(folder.join("keys.json"), document).expect("the document is written");
        for (name, movie) in movies {
            movie.write(&folder.join(name));
        }
        let mut config = crate::Config::default();
        config.skin.pack = Some(folder.to_string_lossy().into_owned());
        let mut app = crate::App::new(String::new(), config, crate::LaunchOptions::default(), home.join("settings.ron"));
        app.shared.skin_screens.wait_for_movie_frames(Some(FRAME_PATIENCE));
        Pack { app, folder, pixels: HeadlessCanvas::new(UI_SIZE.0, UI_SIZE.1) }
    }

    /// Puts the scene clock at `scene_us`.
    fn set_clock(&mut self, scene_us: i64) {
        let age = Duration::from_micros(u64::try_from(scene_us).expect("a scene has no time before it began"));
        self.app.shared.scene_started = Instant::now().checked_sub(age).expect("the process has been up for longer than the scene time asked for");
    }

    /// One frame of the key configuration screen with the scene clock where it is; answers whether
    /// its document drew it.
    fn frame(&mut self) -> bool {
        let mut canvas = Canvas::Headless(&mut self.pixels);
        self.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
        let drew = self.app.shared.draw_keyconfig_skin(&mut canvas, &[]);
        self.app.shared.finish_skin_frame(&mut canvas);
        drew
    }

    /// One frame of the key configuration screen `scene_us` into its scene.
    fn frame_at(&mut self, scene_us: i64) -> bool {
        self.set_clock(scene_us);
        self.frame()
    }

    /// Frames at the scene's start until the document has compiled, and then the frame that starts
    /// its movies, at [`STARTED_AT_US`].
    fn start(&mut self) {
        let began = Instant::now();
        while !self.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG) {
            let mut canvas = Canvas::Headless(&mut self.pixels);
            self.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
            self.app.shared.finish_skin_frame(&mut canvas);
            assert!(began.elapsed() <= COMPILE_WAIT, "the document never compiled: {:?}", self.app.shared.skin_failure(SKIN_TYPE_KEY_CONFIG));
            std::thread::sleep(COMPILE_FRAME_PAUSE);
        }
        assert!(self.frame_at(STARTED_AT_US), "the compiled document did not draw its screen");
    }

    /// One frame with the movies `into_movie_us` into themselves: that long after they were started.
    fn frame_into_movie(&mut self, into_movie_us: i64) {
        assert!(self.frame_at(STARTED_AT_US + into_movie_us), "the document did not draw its screen");
    }

    /// The colour at the middle of slot `slot` of `slots` equal columns of the last frame drawn.
    fn ink(&self, slot: u32, slots: u32) -> crate::Color {
        self.pixels.pixel_at(UI_SIZE.0 * (2 * slot + 1) / (2 * slots), UI_SIZE.1 / 2)
    }

    fn movie_path(&self, name: &str) -> PathBuf {
        self.folder.join(name)
    }

    fn playing(&self) -> usize {
        self.app.shared.skin_screens.movies_playing()
    }
}

/// The colour frame `index` of a scripted movie is.
fn frame_ink(index: u32) -> crate::Color {
    let [r, g, b, _] = ScriptedMovie::pixel_of(index);
    crate::Color::rgb(r, g, b)
}

/// The middle of the time frame `index` of pass `pass` is on show for, measured from the movie's
/// start.
fn middle_of(pass: i64, index: u32) -> i64 {
    pass * MOVIE_PASS_US + i64::from(index) * MOVIE_FRAME_US + MOVIE_FRAME_US / 2
}

/// A key configuration document that draws each of `movies` over its own column of the screen.
fn document(movies: &[&str]) -> String {
    let slots = movies.len() as u32;
    let sources: Vec<String> = movies.iter().map(|file| format!(r#"{{ "id": "{file}", "path": "{file}" }}"#)).collect();
    let images: Vec<String> = movies.iter().map(|file| format!(r#"{{ "id": "{file}", "src": "{file}", "x": 0, "y": 0, "w": -1, "h": -1 }}"#)).collect();
    let destinations: Vec<String> = movies
        .iter()
        .enumerate()
        .map(|(slot, file)| {
            let (x, w) = (UI_SIZE.0 * slot as u32 / slots, UI_SIZE.0 / slots);
            format!(r#"{{ "id": "{file}", "dst": [{{ "time": 0, "x": {x}, "y": 0, "w": {w}, "h": {} }}] }}"#, UI_SIZE.1)
        })
        .collect();
    format!(
        r#"{{
            "type": {SKIN_TYPE_KEY_CONFIG}, "name": "Movies", "w": {}, "h": {},
            "property": [{{ "name": "Row", "item": [{{ "name": "on", "op": {GRANTED_OPTION} }}], "def": "on" }}],
            "source": [{}], "image": [{}], "destination": [{}]
        }}"#,
        UI_SIZE.0,
        UI_SIZE.1,
        sources.join(", "),
        images.join(", "),
        destinations.join(", ")
    )
}

fn movie() -> ScriptedMovie {
    ScriptedMovie::new(MOVIE_SIZE.0, MOVIE_SIZE.1, MOVIE_FRAMES, MOVIE_FRAME_US)
}

/// A pack that draws one scripted movie over the whole screen.
fn one_movie_pack(tag: &str, movie: ScriptedMovie) -> Pack {
    Pack::new(tag, &document(&["clip.mp4"]), &[("clip.mp4", movie)])
}

/// A movie is opened with its screen and decoded by nothing until an object drawn from it is first
/// prepared. From then on the frame on show is the one due that long into the movie, on the scene
/// clock, and the movie goes round when its length has passed.
#[test]
fn a_movie_starts_on_the_first_frame_of_its_screen_and_follows_the_scene_clock_round() {
    let mut pack = one_movie_pack("follows", movie());
    let path = pack.movie_path("clip.mp4");
    let began = Instant::now();
    while !pack.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG) {
        let mut canvas = Canvas::Headless(&mut pack.pixels);
        pack.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
        pack.app.shared.finish_skin_frame(&mut canvas);
        assert!(began.elapsed() <= COMPILE_WAIT, "the document never compiled");
        std::thread::sleep(COMPILE_FRAME_PAUSE);
    }
    assert_eq!(scripted_movie::open_on(&path), 1, "the movie was not opened with its screen");
    assert_eq!(pack.playing(), 0, "a movie nothing has prepared yet is being decoded");
    assert!(pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG).is_empty(), "{:?}", pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG));

    assert!(pack.frame_at(STARTED_AT_US));
    assert_eq!(pack.playing(), 1);
    assert_eq!(pack.ink(0, 1), frame_ink(0), "the frame that started the movie does not show its first frame");

    for (pass, index) in [(0, 0), (0, 2), (0, 4), (1, 0), (1, 3), (3, 1)] {
        pack.frame_into_movie(middle_of(pass, index));
        assert_eq!(pack.ink(0, 1), frame_ink(index), "pass {pass}, frame {index}");
    }
    assert_eq!(pack.playing(), 1);
    assert_eq!(scripted_movie::open_on(&path), 1, "going round opened the movie again");
}

/// The scene clock is the only clock a movie has: frame after frame at one scene time shows one
/// movie frame, however long the frames take, and so does a scene that was parked and put back.
#[test]
fn a_movie_stands_still_while_the_scene_clock_does() {
    let mut pack = one_movie_pack("stands-still", movie());
    pack.start();
    pack.frame_into_movie(middle_of(0, 1));
    assert_eq!(pack.ink(0, 1), frame_ink(1));

    for _ in 0..STILL_FRAMES {
        pack.frame_into_movie(middle_of(0, 1));
        assert_eq!(pack.ink(0, 1), frame_ink(1), "the movie moved on under a scene clock that did not");
    }

    pack.set_clock(STARTED_AT_US + middle_of(0, 1));
    let parked = pack.app.shared.suspend_skin_scene();
    assert_eq!(pack.playing(), 1, "a parked screen's movie was stopped rather than left waiting");
    std::thread::sleep(Duration::from_millis(30));
    pack.app.shared.resume_skin_scene(parked);
    assert!(pack.frame());
    assert_eq!(pack.ink(0, 1), frame_ink(1), "the time a scene spent parked was played off its movie");

    pack.frame_into_movie(middle_of(0, 3));
    assert_eq!(pack.ink(0, 1), frame_ink(3), "a movie that stood still did not go on again");
}

/// A scene clock that is moved -- a replay stepped back, or ahead -- takes its movies with it: the
/// frame on show becomes the one due at the time the clock was put at, whichever way that is.
#[test]
fn a_movie_follows_a_scene_clock_that_jumps_back_and_ahead() {
    let mut pack = one_movie_pack("jumps", movie());
    let path = pack.movie_path("clip.mp4");
    pack.start();
    for (pass, index) in [(0, 3), (0, 1), (0, 0), (2, 4), (1, 2), (1, 3), (0, 4), (3, 0)] {
        pack.frame_into_movie(middle_of(pass, index));
        assert_eq!(pack.ink(0, 1), frame_ink(index), "pass {pass}, frame {index}");
    }
    assert_eq!((pack.playing(), scripted_movie::open_on(&path)), (1, 1), "following the clock opened the movie again or let go of it");
}

/// A screen that is left stops its movies: the players are gone with the screen and the files are
/// closed, which is the decoder threads having ended.
#[test]
fn leaving_the_screen_stops_its_movies_and_closes_their_files() {
    let mut pack = one_movie_pack("leaving", movie());
    let path = pack.movie_path("clip.mp4");
    pack.start();
    pack.frame_into_movie(middle_of(0, 2));
    assert_eq!((pack.playing(), scripted_movie::open_on(&path)), (1, 1));
    assert_eq!(pack.app.shared.skin_screens.texture_stats().count, 0, "a movie's texture was put among the files screens share");

    pack.app.shared.begin_skin_scene();
    let mut canvas = Canvas::Headless(&mut pack.pixels);
    pack.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_RESULT);
    pack.app.shared.finish_skin_frame(&mut canvas);
    assert!(!pack.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG), "the screen that was left is still compiled");
    assert_eq!(pack.playing(), 0, "the movie of a screen that was left is still being decoded");
    assert_eq!(scripted_movie::open_on(&path), 0, "the movie's file is still open, so its decoder outlived the screen");
}

/// A movie that was opened and never started is closed with its screen all the same.
#[test]
fn a_movie_that_never_started_is_closed_with_its_screen() {
    let mut pack = one_movie_pack("never-started", movie());
    let path = pack.movie_path("clip.mp4");
    let began = Instant::now();
    while !pack.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG) {
        let mut canvas = Canvas::Headless(&mut pack.pixels);
        pack.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
        pack.app.shared.finish_skin_frame(&mut canvas);
        assert!(began.elapsed() <= COMPILE_WAIT, "the document never compiled");
        std::thread::sleep(COMPILE_FRAME_PAUSE);
    }
    assert_eq!(scripted_movie::open_on(&path), 1);
    pack.app.shared.begin_skin_scene();
    let mut canvas = Canvas::Headless(&mut pack.pixels);
    pack.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_RESULT);
    pack.app.shared.finish_skin_frame(&mut canvas);
    assert_eq!(scripted_movie::open_on(&path), 0);
}

/// The file name of the movie that breaks, which no other test's movie has: what is said about a
/// movie names its file, and every test's messages go through one queue.
const BREAKING_MOVIE: &str = "breaks-part-way.mp4";

/// A movie that stops decoding part way is said once, is drawn no more, and has its thread and its
/// file given up; the frames before the break were shown.
#[test]
fn a_movie_that_stops_decoding_is_said_once_and_drawn_no_more() {
    crate::notify::exclusive(|| {
        let breaking = ScriptedMovie { breaks_at: Some(2), ..movie() };
        let mut pack = Pack::new("breaks", &document(&[BREAKING_MOVIE]), &[(BREAKING_MOVIE, breaking)]);
        let path = pack.movie_path(BREAKING_MOVIE);
        pack.start();
        pack.frame_into_movie(middle_of(0, 0));
        assert_eq!(pack.ink(0, 1), frame_ink(0));

        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        assert!(!said.iter().any(|(_, message)| message.contains(BREAKING_MOVIE)), "something was said about a movie that is playing: {said:?}");

        pack.frame_into_movie(middle_of(0, 1));
        assert_eq!(pack.ink(0, 1), NOTHING_DRAWN, "the last frame before the break stayed on show after its turn");
        for into in [middle_of(0, 3), middle_of(0, 4), middle_of(1, 1)] {
            pack.frame_into_movie(into);
            assert_eq!(pack.ink(0, 1), NOTHING_DRAWN, "a movie that stopped decoding is still drawn");
        }
        crate::notify::drain(&mut said);
        let about_movie: Vec<&String> = said.iter().map(|(_, message)| message).filter(|message| message.contains(BREAKING_MOVIE)).collect();
        assert_eq!(about_movie.len(), 1, "a movie that stopped decoding was not said exactly once: {said:?}");
        assert!(about_movie[0].contains("stopped playing"), "{about_movie:?}");
        assert_eq!((pack.playing(), scripted_movie::open_on(&path)), (0, 0), "a movie that stopped decoding kept its thread or its file");
    });
}

/// The file name of the movie that is damaged, which no other test's movie has.
const DAMAGED_MOVIE: &str = "damaged-part-way.mp4";

/// A movie with a frame that does not decode goes on playing around it: the frame before stays on
/// show through the time of the lost one, the frames after it are shown at their own times, and
/// the damage is said once.
#[test]
fn a_damaged_movie_is_said_once_and_played_around_its_damage() {
    crate::notify::exclusive(|| {
        let damaged = ScriptedMovie { loses: Some(2), ..movie() };
        let mut pack = Pack::new("damaged", &document(&[DAMAGED_MOVIE]), &[(DAMAGED_MOVIE, damaged)]);
        pack.start();
        for (pass, shown, index) in [(0, 0, 0), (0, 1, 1), (0, 2, 1), (0, 3, 3), (0, 4, 4), (1, 1, 1), (1, 2, 1), (1, 3, 3)] {
            pack.frame_into_movie(middle_of(pass, shown));
            assert_eq!(pack.ink(0, 1), frame_ink(index), "pass {pass}: frame {index} was to be on show in the time of frame {shown}");
        }
        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        let about_movie: Vec<&String> = said.iter().map(|(_, message)| message).filter(|message| message.contains(DAMAGED_MOVIE)).collect();
        assert_eq!(about_movie.len(), 1, "a damaged movie was not said exactly once: {said:?}");
        assert!(about_movie[0].contains("is damaged"), "{about_movie:?}");
        assert_eq!(pack.playing(), 1, "a movie with damage in it was given up on");
    });
}

/// A file that will not open as a movie leaves its object out of the screen, with the reason in
/// the one warning the screen is compiled with; a file that is no movie at all is the same, with
/// or without a decoder built in.
#[test]
fn a_movie_that_cannot_be_opened_leaves_its_object_out_and_says_why() {
    let unplayable = ScriptedMovie { unplayable: true, ..movie() };
    let mut pack = Pack::new("unopened", &document(&["clip.mp4", "noise.mp4", "fine.mp4"]), &[("clip.mp4", unplayable), ("fine.mp4", movie())]);
    std::fs::write(pack.movie_path("noise.mp4"), b"this was never a movie").expect("the file is written");
    pack.start();
    let warnings = pack.app.shared.skin_warnings(SKIN_TYPE_KEY_CONFIG);
    let wanted = VideoError::Unsupported("the scripted movie is not one that plays".into()).to_string();
    assert!(warnings.iter().any(|line| line.contains("clip.mp4") && line.contains(&wanted)), "{warnings:?}");
    assert!(warnings.iter().any(|line| line.contains("noise.mp4") && line.contains("movie source")), "{warnings:?}");
    assert_eq!(warnings.len(), 2, "{warnings:?}");

    pack.frame_into_movie(middle_of(0, 1));
    assert_eq!(pack.ink(0, 3), NOTHING_DRAWN, "something was drawn for a movie that would not open");
    assert_eq!(pack.ink(1, 3), NOTHING_DRAWN, "something was drawn for a file that is no movie");
    assert_eq!(pack.ink(2, 3), frame_ink(1), "the movie beside them was held back with them");
    assert_eq!(pack.playing(), 1);
}

/// No more than [`movies::PLAYING_MOVIES_LIMIT`] movies are decoded at a time: the ones past it are
/// said and left undrawn, and the ones inside it play.
#[test]
fn movies_past_the_limit_are_not_played() {
    crate::notify::exclusive(|| {
        let names: Vec<String> = (0..=movies::PLAYING_MOVIES_LIMIT).map(|index| format!("clip{index}.mp4")).collect();
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let scripts: Vec<(&str, ScriptedMovie)> = names.iter().map(|name| (*name, movie())).collect();
        let mut pack = Pack::new("limit", &document(&names), &scripts);
        pack.start();
        pack.frame_into_movie(middle_of(0, 2));
        let slots = names.len() as u32;
        assert_eq!(pack.playing(), movies::PLAYING_MOVIES_LIMIT);
        for slot in 0..slots - 1 {
            assert_eq!(pack.ink(slot, slots), frame_ink(2), "movie {slot} is inside the limit and does not play");
        }
        assert_eq!(pack.ink(slots - 1, slots), NOTHING_DRAWN, "a movie past the limit was drawn");

        let mut said = Vec::new();
        crate::notify::drain(&mut said);
        let over: Vec<&String> = said.iter().map(|(_, message)| message).filter(|message| message.contains("is not played")).collect();
        assert_eq!(over.len(), 1, "{said:?}");
        pack.frame_into_movie(middle_of(0, 3));
        crate::notify::drain(&mut said);
        assert_eq!(said.iter().filter(|(_, message)| message.contains("is not played")).count(), 1, "the movie past the limit was said again: {said:?}");
    });
}

/// How far a channel the GPU drew may sit from the frame's own colour.
const GPU_TOLERANCE: i32 = 2;

/// The GPU backend takes each frame of a movie into the texture it already has for it, as the
/// headless canvas does: the frame on show is the frame drawn, frame after frame. Passes without
/// drawing on a machine with no adapter.
#[test]
fn a_movie_plays_on_the_gpu_backend_frame_after_frame() {
    let Some(mut gpu) = Gpu::offscreen_if_available(UI_SIZE.0, UI_SIZE.1, UI_SIZE) else {
        return;
    };
    let mut pack = one_movie_pack("gpu", movie());
    let frame_on = |pack: &mut Pack, gpu: &mut Gpu, scene_us: Option<i64>| {
        if let Some(scene_us) = scene_us {
            pack.set_clock(scene_us);
        }
        let mut canvas = Canvas::Window(gpu);
        pack.app.shared.prepare_skin(&mut canvas, SKIN_TYPE_KEY_CONFIG);
        let drew = pack.app.shared.draw_keyconfig_skin(&mut canvas, &[]);
        pack.app.shared.finish_skin_frame(&mut canvas);
        drew
    };
    let began = Instant::now();
    while !pack.app.shared.has_compiled_skin(SKIN_TYPE_KEY_CONFIG) {
        frame_on(&mut pack, &mut gpu, None);
        assert!(began.elapsed() <= COMPILE_WAIT, "the document never compiled");
        std::thread::sleep(COMPILE_FRAME_PAUSE);
    }
    assert!(frame_on(&mut pack, &mut gpu, Some(STARTED_AT_US)));

    let middle = ((UI_SIZE.1 / 2 * UI_SIZE.0 + UI_SIZE.0 / 2) * 4) as usize;
    for (pass, index) in [(0, 0), (0, 3), (1, 1), (1, 4), (2, 2)] {
        assert!(frame_on(&mut pack, &mut gpu, Some(STARTED_AT_US + middle_of(pass, index))));
        let drawn = gpu.capture().expect("an offscreen target reads back");
        let wanted = ScriptedMovie::pixel_of(index);
        for channel in 0..3 {
            let apart = (i32::from(drawn[middle + channel]) - i32::from(wanted[channel])).abs();
            assert!(apart <= GPU_TOLERANCE, "pass {pass}, frame {index}: the GPU drew {:?}, wanted {wanted:?}", &drawn[middle..middle + 4]);
        }
    }
    assert_eq!(pack.playing(), 1);
}
