//! The player against a decoder that is a script: frames that say which they are, at times that
//! are a multiple of ten, so every question about which frame is on show has one right answer.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::*;
use crate::VideoInfo;

/// Frames in the scripted movie.
const FRAMES: u32 = 10;

/// Frames in the scripted movie that is long enough for a jump inside it to be told from a walk.
const LONG_FRAMES: u32 = 100;

/// How far apart the frames decoding can start from are in the scripted movie that has them.
const KEY_EVERY: u32 = 10;

/// Microseconds each of its frames is on show.
const INTERVAL_US: i64 = 100_000;

/// How long one pass over it lasts.
const PASS_US: i64 = FRAMES as i64 * INTERVAL_US;

/// How long one pass over the long one lasts.
const LONG_PASS_US: i64 = LONG_FRAMES as i64 * INTERVAL_US;

/// How many passes ahead a clock jumps that no decoder could follow by decoding them.
const MANY_PASSES: i64 = 100_000;

/// The size of its frames.
const WIDTH: u32 = 2;
const HEIGHT: u32 = 1;

/// How long a test waits for the thread before it calls the wait failed.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long a test gives the thread to do something it should not, before it is satisfied that it
/// did not.
const SETTLE: Duration = Duration::from_millis(60);

/// What a test watches a scripted decoder through.
#[derive(Debug, Default)]
struct Watch {
    /// Frames decoded with their pixels written.
    converted: AtomicU64,
    /// Frames decoded and thrown away.
    skipped: AtomicU64,
    rewinds: AtomicU64,
    /// Times the decoder went to a frame decoding can start from other than by a rewind.
    jumps: AtomicU64,
    dropped: AtomicBool,
    /// Whether the thread is kept from starting on the movie, so a test can say a time first.
    held: AtomicBool,
}

impl Watch {
    /// Frames decoded, shown or not.
    fn decoded(&self) -> u64 {
        self.converted.load(Ordering::SeqCst) + self.skipped.load(Ordering::SeqCst)
    }
}

/// A movie of [`FRAMES`] frames whose every pixel byte is the frame's own index.
#[derive(Debug)]
struct Scripted {
    frames: u32,
    at: u32,
    /// The index of a frame that fails to decode, on every pass.
    breaks_at: Option<u32>,
    /// The index of a frame the decoder panics at.
    panics_at: Option<u32>,
    /// The index of a frame that is damaged: it never comes out, and is counted as lost.
    loses: Option<u32>,
    /// The index of the frame from which nothing more comes out, though the movie says it is longer.
    ends_at: Option<u32>,
    lost: u64,
    watch: Arc<Watch>,
}

impl Scripted {
    fn new(watch: &Arc<Watch>) -> Scripted {
        Scripted { frames: FRAMES, at: 0, breaks_at: None, panics_at: None, loses: None, ends_at: None, lost: 0, watch: Arc::clone(watch) }
    }

    fn long(watch: &Arc<Watch>) -> Scripted {
        let mut script = Scripted::new(watch);
        script.frames = LONG_FRAMES;
        script
    }
}

impl Drop for Scripted {
    fn drop(&mut self) {
        self.watch.dropped.store(true, Ordering::SeqCst);
    }
}

impl VideoDecoder for Scripted {
    fn info(&self) -> VideoInfo {
        while self.watch.held.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        VideoInfo { width: WIDTH, height: HEIGHT, frame_count: self.frames, duration_us: i64::from(self.frames) * INTERVAL_US }
    }

    fn next_time_us(&self) -> Option<i64> {
        (self.at < self.frames).then(|| i64::from(self.at) * INTERVAL_US)
    }

    fn next_frame(&mut self, rgba: Option<&mut Vec<u8>>) -> Result<Option<i64>, VideoError> {
        if self.loses == Some(self.at) {
            self.at += 1;
            self.lost += 1;
        }
        if self.at >= self.frames || self.ends_at.is_some_and(|end| self.at >= end) {
            return Ok(None);
        }
        if self.breaks_at == Some(self.at) {
            return Err(VideoError::Decode("the script breaks here".into()));
        }
        assert!(self.panics_at != Some(self.at), "the script blows up here");
        match rgba {
            Some(rgba) => {
                rgba.clear();
                rgba.resize((WIDTH * HEIGHT) as usize * crate::color::RGBA_BYTES, self.at as u8);
                self.watch.converted.fetch_add(1, Ordering::SeqCst);
            }
            None => {
                self.watch.skipped.fetch_add(1, Ordering::SeqCst);
            }
        }
        self.at += 1;
        Ok(Some(i64::from(self.at - 1) * INTERVAL_US))
    }

    fn rewind(&mut self) -> Result<(), VideoError> {
        self.at = 0;
        self.watch.rewinds.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn lost_samples(&self) -> u64 {
        self.lost
    }
}

/// A scripted movie that decoding can start in at every [`KEY_EVERY`]-th frame.
#[derive(Debug)]
struct Keyed(Scripted);

impl VideoDecoder for Keyed {
    fn info(&self) -> VideoInfo {
        self.0.info()
    }

    fn next_time_us(&self) -> Option<i64> {
        self.0.next_time_us()
    }

    fn next_frame(&mut self, rgba: Option<&mut Vec<u8>>) -> Result<Option<i64>, VideoError> {
        self.0.next_frame(rgba)
    }

    fn rewind(&mut self) -> Result<(), VideoError> {
        self.0.rewind()
    }

    fn seek(&mut self, time_us: i64) -> Result<bool, VideoError> {
        let wanted = (time_us.max(0) / INTERVAL_US).min(i64::from(self.0.frames) - 1) as u32;
        let key = wanted / KEY_EVERY * KEY_EVERY;
        if self.0.at < self.0.frames && self.0.at <= wanted && key <= self.0.at {
            return Ok(false);
        }
        self.0.at = key;
        self.0.watch.jumps.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}

fn player(watch: &Arc<Watch>) -> VideoPlayer {
    VideoPlayer::spawn(Box::new(Scripted::new(watch))).expect("a thread starts")
}

/// Which frame of the script is on show at `time_us`, as its index and its time.
fn shown_at(player: &mut VideoPlayer, time_us: i64) -> Option<(u8, i64)> {
    player.advance_blocking(time_us, PATIENCE);
    player.frame().map(|frame| (frame.rgba[0], frame.time_us))
}

/// Whether the player's thread has returned.
fn has_exited(player: &VideoPlayer) -> bool {
    player.shared.lock().exited
}

/// Waits until `settled` holds, failing the test when it never does.
fn wait_for(what: &str, settled: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !settled() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::yield_now();
    }
}

#[test]
fn the_frame_on_show_is_the_newest_one_whose_time_has_come() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    assert_eq!(player.frame(), None, "a frame was on show before any time was said");
    assert_eq!(shown_at(&mut player, 0), Some((0, 0)));
    assert_eq!(shown_at(&mut player, INTERVAL_US - 1), Some((0, 0)));
    assert_eq!(shown_at(&mut player, INTERVAL_US), Some((1, INTERVAL_US)));
    assert_eq!(shown_at(&mut player, 2 * INTERVAL_US + INTERVAL_US / 2), Some((2, 2 * INTERVAL_US)));
    assert_eq!(shown_at(&mut player, PASS_US - 1), Some((9, 9 * INTERVAL_US)));
    let frame = player.frame().expect("a frame is on show");
    assert_eq!((frame.width, frame.height, frame.rgba.len()), (WIDTH, HEIGHT, (WIDTH * HEIGHT * 4) as usize));
}

#[test]
fn the_movie_starts_over_when_its_length_has_passed_and_its_times_keep_counting() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    assert_eq!(shown_at(&mut player, PASS_US - 1), Some((9, 9 * INTERVAL_US)));
    assert_eq!(shown_at(&mut player, PASS_US), Some((0, PASS_US)), "the first frame of the second pass is due exactly one length in");
    assert_eq!(shown_at(&mut player, PASS_US + INTERVAL_US / 2), Some((0, PASS_US)));
    assert_eq!(shown_at(&mut player, 2 * PASS_US - 1), Some((9, PASS_US + 9 * INTERVAL_US)));
    assert_eq!(shown_at(&mut player, 2 * PASS_US + 3 * INTERVAL_US + 1), Some((3, 2 * PASS_US + 3 * INTERVAL_US)));
    assert!(watch.rewinds.load(Ordering::SeqCst) >= 2, "the decoder was not sent back to its first frame for each pass");
}

#[test]
fn a_time_that_does_not_move_on_changes_nothing() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    assert!(player.advance_blocking(INTERVAL_US, PATIENCE));
    wait_for("the queue to fill behind the frame on show", || watch.decoded() == 2 + DEFAULT_QUEUE_FRAMES as u64);
    for _ in 0..50 {
        assert!(!player.advance(INTERVAL_US), "a clock standing still changed the frame on show");
    }
    std::thread::sleep(SETTLE);
    assert_eq!(watch.decoded(), 2 + DEFAULT_QUEUE_FRAMES as u64, "the thread went on decoding for a clock that stands still");
    assert_eq!(player.frame().map(|frame| frame.rgba[0]), Some(1));
}

#[test]
fn the_thread_stops_when_the_queue_is_full_and_goes_on_when_a_frame_is_taken() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    let full = DEFAULT_QUEUE_FRAMES as u64;
    wait_for("the queue to fill", || watch.decoded() == full);
    std::thread::sleep(SETTLE);
    assert_eq!(watch.decoded(), full, "the thread decoded past a full queue that nobody took from");
    assert!(!has_exited(&player));

    assert!(player.advance(0), "the first frame was not due at the start");
    wait_for("the place of the frame taken to be filled", || watch.decoded() == full + 1);
    std::thread::sleep(SETTLE);
    assert_eq!(watch.decoded(), full + 1, "taking one frame made room for more than one");
}

#[test]
fn a_queue_of_one_frame_is_the_smallest_there_is() {
    let watch = Arc::new(Watch::default());
    let mut player = VideoPlayer::with_queue(Box::new(Scripted::new(&watch)), 0).expect("a thread starts");
    wait_for("the queue to fill", || watch.decoded() == 1);
    std::thread::sleep(SETTLE);
    assert_eq!(watch.decoded(), 1);
    assert_eq!(shown_at(&mut player, 4 * INTERVAL_US), Some((4, 4 * INTERVAL_US)));
}

#[test]
fn dropping_the_player_ends_its_thread_and_its_decoder() {
    let watch = Arc::new(Watch::default());
    let player = player(&watch);
    wait_for("the queue to fill", || watch.decoded() == DEFAULT_QUEUE_FRAMES as u64);
    assert!(!watch.dropped.load(Ordering::SeqCst));
    drop(player);
    assert!(watch.dropped.load(Ordering::SeqCst), "the decoder outlived its player, so its thread did too");

    let idle = Arc::new(Watch::default());
    drop(VideoPlayer::spawn(Box::new(Scripted::new(&idle))).expect("a thread starts"));
    assert!(idle.dropped.load(Ordering::SeqCst), "a player dropped at once left its thread behind");
}

#[test]
fn catching_up_inside_a_pass_decodes_the_frames_in_between_and_converts_only_the_ones_that_can_be_shown() {
    let watch = Arc::new(Watch::default());
    let mut player = VideoPlayer::spawn(Box::new(Scripted::long(&watch))).expect("a thread starts");
    wait_for("the queue to fill", || watch.decoded() == DEFAULT_QUEUE_FRAMES as u64);
    let far = 84 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, far), Some((84, far)));
    let (converted, skipped) = (watch.converted.load(Ordering::SeqCst), watch.skipped.load(Ordering::SeqCst));
    assert_eq!(skipped, 84 - DEFAULT_QUEUE_FRAMES as u64, "the frames in between were not each decoded once and thrown away");
    assert!(converted <= 2 * DEFAULT_QUEUE_FRAMES as u64 + 2, "frames nobody could be shown were turned into pixels: {converted}");
    assert_eq!(watch.rewinds.load(Ordering::SeqCst), 0, "a decoder that was behind the clock was sent back to its first frame");
}

#[test]
fn a_clock_that_jumps_passes_ahead_is_caught_up_with_inside_the_pass_it_landed_in() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    wait_for("the queue to fill", || watch.decoded() == DEFAULT_QUEUE_FRAMES as u64);
    let far = MANY_PASSES * PASS_US + 6 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, far), Some((6, far)));
    assert!(watch.decoded() <= u64::from(FRAMES), "{} frames were decoded for a jump one pass could catch up with", watch.decoded());
    assert_eq!(watch.rewinds.load(Ordering::SeqCst), 0, "a decoder that was before where the clock landed in its pass was sent back");

    let further = 2 * MANY_PASSES * PASS_US + INTERVAL_US;
    let before = watch.decoded();
    assert_eq!(shown_at(&mut player, further), Some((1, further)), "a clock that landed before the decoder's place in the pass was not followed");
    assert_eq!(watch.rewinds.load(Ordering::SeqCst), 1);
    assert!(watch.decoded() - before <= u64::from(FRAMES));

    assert_eq!(shown_at(&mut player, further + PASS_US), Some((1, further + PASS_US)), "the movie did not go round from where the clock landed");
    assert_eq!(player.failure(), None);
}

#[test]
fn a_clock_that_goes_back_is_followed_back() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    let later = 2 * PASS_US + 7 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, later), Some((7, later)));
    let earlier = 2 * PASS_US + 2 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, earlier + INTERVAL_US / 2), Some((2, earlier)), "the frame of a time the clock went back to is not on show");
    assert_eq!(shown_at(&mut player, earlier + INTERVAL_US), Some((3, earlier + INTERVAL_US)), "a movie that went back did not go on from there");

    let pass_before = PASS_US + 9 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, pass_before), Some((9, pass_before)), "a clock that went back into the pass before was not followed");
    assert_eq!(shown_at(&mut player, 0), Some((0, 0)), "a clock that went back to the start was not followed");
    assert_eq!(shown_at(&mut player, -INTERVAL_US), Some((0, 0)), "a time before the movie started is not its start");
    assert_eq!(player.failure(), None);
}

#[test]
fn a_clock_that_goes_back_again_before_it_was_caught_up_with_is_still_followed() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    assert_eq!(shown_at(&mut player, 8 * INTERVAL_US), Some((8, 8 * INTERVAL_US)));
    player.advance(5 * INTERVAL_US);
    assert_eq!(shown_at(&mut player, 2 * INTERVAL_US), Some((2, 2 * INTERVAL_US)), "going back twice left the movie where it went the first time");
}

#[test]
fn a_clock_that_goes_back_inside_the_time_of_the_frame_on_show_throws_nothing_away() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    assert_eq!(shown_at(&mut player, 4 * INTERVAL_US + INTERVAL_US / 2), Some((4, 4 * INTERVAL_US)));
    wait_for("the queue to fill behind the frame on show", || watch.decoded() == 5 + DEFAULT_QUEUE_FRAMES as u64);
    assert!(!player.advance(4 * INTERVAL_US + 1), "a frame that is still the one due was replaced");
    std::thread::sleep(SETTLE);
    assert_eq!(player.frame().map(|frame| frame.rgba[0]), Some(4));
    assert_eq!((watch.decoded(), watch.rewinds.load(Ordering::SeqCst)), (5 + DEFAULT_QUEUE_FRAMES as u64, 0), "what was decoded ahead was thrown away");
}

#[test]
fn a_jump_is_caught_up_with_from_the_nearest_frame_decoding_can_start_from() {
    let watch = Arc::new(Watch::default());
    let mut player = VideoPlayer::spawn(Box::new(Keyed(Scripted::long(&watch)))).expect("a thread starts");
    wait_for("the queue to fill", || watch.decoded() == DEFAULT_QUEUE_FRAMES as u64);

    let ahead = 87 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, ahead), Some((87, ahead)));
    assert_eq!(watch.skipped.load(Ordering::SeqCst), 7, "a jump ahead was not caught up with from the start before it");

    let back = 23 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, back), Some((23, back)));
    assert_eq!(watch.skipped.load(Ordering::SeqCst), 7 + 3, "a jump back was not caught up with from the start before it");

    let passes_on = MANY_PASSES * LONG_PASS_US + 51 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, passes_on), Some((51, passes_on)));
    assert_eq!(watch.skipped.load(Ordering::SeqCst), 7 + 3 + 1, "a jump of many passes was not caught up with from the start before where it landed");
    assert_eq!((watch.jumps.load(Ordering::SeqCst), watch.rewinds.load(Ordering::SeqCst)), (3, 0));

    let walked = passes_on + 4 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, walked), Some((55, walked)));
    assert_eq!(watch.jumps.load(Ordering::SeqCst), 3, "a decoder that was keeping up was moved");
}

#[test]
fn starting_over_shows_the_first_frame_again_at_time_zero() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    assert_eq!(shown_at(&mut player, 7 * INTERVAL_US), Some((7, 7 * INTERVAL_US)));
    player.restart();
    assert_eq!(player.frame(), None, "the frame of the pass that was thrown away stayed on show");
    assert_eq!(shown_at(&mut player, 0), Some((0, 0)));
    assert_eq!(shown_at(&mut player, INTERVAL_US), Some((1, INTERVAL_US)));
    player.restart();
    player.restart();
    assert_eq!(shown_at(&mut player, 2 * INTERVAL_US), Some((2, 2 * INTERVAL_US)));
}

#[test]
fn a_decoder_that_gives_up_ends_the_thread_and_says_why() {
    let watch = Arc::new(Watch::default());
    let mut broken = Scripted::new(&watch);
    broken.breaks_at = Some(2);
    let mut player = VideoPlayer::spawn(Box::new(broken)).expect("a thread starts");
    wait_for("the thread to end", || has_exited(&player));
    assert!(!player.has_ended(), "the frames decoded before the break were written off with the thread");
    assert_eq!(shown_at(&mut player, PASS_US), Some((1, INTERVAL_US)), "the frames before the break were not shown");
    assert!(player.failure().is_some_and(|why| why.contains("the script breaks here")), "{:?}", player.failure());
    assert!(player.has_ended(), "a player whose thread is gone and whose frames are all taken has not ended");
    assert!(watch.dropped.load(Ordering::SeqCst), "the decoder was kept after it gave up");
    assert!(!player.advance_blocking(2 * PASS_US, PATIENCE), "a player with no thread waited or found a frame");
    assert!(!player.advance(0), "a player with no thread changed its frame for a clock that went back");
    assert_eq!(player.frame().map(|frame| frame.rgba[0]), Some(1), "a player with no thread threw its last frame away for a clock that went back");
}

#[test]
fn a_decoder_that_panics_ends_the_thread_and_says_so() {
    let watch = Arc::new(Watch::default());
    let mut blows_up = Scripted::new(&watch);
    blows_up.panics_at = Some(2);
    let mut player = VideoPlayer::spawn(Box::new(blows_up)).expect("a thread starts");
    wait_for("the thread to end", || has_exited(&player));
    assert!(watch.dropped.load(Ordering::SeqCst), "the decoder was kept after it panicked");
    assert_eq!(player.failure().as_deref(), Some(PANICKED));
    assert_eq!(shown_at(&mut player, PASS_US), Some((1, INTERVAL_US)), "the frames before the panic were not shown");
    assert!(player.has_ended(), "a player whose thread panicked goes on as if it were decoding");
    let began = Instant::now();
    assert!(!player.advance_blocking(2 * PASS_US, PATIENCE));
    assert!(began.elapsed() < PATIENCE, "a player whose thread panicked was waited on for all the patience there was");
}

#[test]
fn a_movie_with_no_frame_is_given_up_on_rather_than_played_round_for_ever() {
    let watch = Arc::new(Watch::default());
    let mut empty = Scripted::new(&watch);
    empty.frames = 0;
    let mut player = VideoPlayer::spawn(Box::new(empty)).expect("a thread starts");
    wait_for("the thread to end", || has_exited(&player));
    assert!(player.failure().is_some());
    assert!(!player.advance_blocking(0, PATIENCE));
    assert_eq!(player.frame(), None);
    assert_eq!(watch.rewinds.load(Ordering::SeqCst), 0);
}

#[test]
fn a_jump_into_a_stretch_with_no_frame_does_not_give_the_movie_up() {
    let watch = Arc::new(Watch::default());
    watch.held.store(true, Ordering::SeqCst);
    let mut cut_short = Scripted::long(&watch);
    cut_short.ends_at = Some(50);
    let mut player = VideoPlayer::spawn(Box::new(Keyed(cut_short))).expect("a thread starts");
    assert!(!player.advance(75 * INTERVAL_US));
    watch.held.store(false, Ordering::SeqCst);
    wait_for("the jump to where nothing decodes", || watch.jumps.load(Ordering::SeqCst) == 1);

    let next_pass = LONG_PASS_US + 2 * INTERVAL_US;
    assert_eq!(shown_at(&mut player, next_pass), Some((2, next_pass)), "the movie did not go round after a pass that began where nothing decodes");
    assert_eq!(player.failure(), None, "a movie was given up on for the frames after a jump, not for all of them");
    assert_eq!(watch.rewinds.load(Ordering::SeqCst), 1, "the pass that began where nothing decodes was not followed by one from the first frame");
}

#[test]
fn damaged_frames_are_counted_and_the_frame_before_them_stays_on_show() {
    let watch = Arc::new(Watch::default());
    let mut damaged = Scripted::new(&watch);
    damaged.loses = Some(2);
    let mut player = VideoPlayer::spawn(Box::new(damaged)).expect("a thread starts");
    assert_eq!(shown_at(&mut player, INTERVAL_US), Some((1, INTERVAL_US)));
    assert_eq!(shown_at(&mut player, 2 * INTERVAL_US + INTERVAL_US / 2), Some((1, INTERVAL_US)), "something was shown in the time of a frame that was lost");
    assert_eq!(shown_at(&mut player, 3 * INTERVAL_US), Some((3, 3 * INTERVAL_US)), "the frame after a lost one is not shown at its own time");
    assert_eq!(player.lost_samples(), 1);
    assert_eq!(player.failure(), None);
}

#[test]
fn pixel_buffers_are_written_into_again_rather_than_allocated_for_every_frame() {
    let watch = Arc::new(Watch::default());
    let mut player = player(&watch);
    for step in 0..(4 * FRAMES as i64) {
        player.advance_blocking(step * INTERVAL_US, PATIENCE);
    }
    for step in [3, 17, 2, 31, 8] {
        player.advance_blocking(step * INTERVAL_US, PATIENCE);
    }
    let kept = player.shared.lock().spare.len();
    assert!(kept <= DEFAULT_QUEUE_FRAMES + SPARE_BUFFERS_BEYOND_QUEUE, "{kept} buffers were kept");
}
