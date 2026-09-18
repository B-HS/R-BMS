//! Decoding the committed fixture clips through the public API, frame by frame on a song clock.
//!
//! The clips are a one-second test pattern at 48x32 and 25 fps, one per container, made with
//! ffmpeg; `frame5.rgba` and `frame5-h264.rgba` are ffmpeg's own RGBA of the sixth picture of each
//! and are what the crate's own decode plus its own BT.601 conversion is scored against.

use rbms_video::{VideoError, VideoFrame, VideoSource};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a clip's worker thread is given to produce the next picture before the test calls it
/// stuck. Generous: the fixtures decode in milliseconds, and a loaded build machine is not a bug.
const DECODE_TIMEOUT: Duration = Duration::from_secs(30);

/// Song-clock step the fixtures are sampled at: far finer than the 40 ms between pictures, so no
/// picture of a clip can be stepped over.
const STEP_US: i64 = 1_000;

/// How far past the start the clock is allowed to run before a clip is called stuck. The fixtures
/// are 400 ms long.
const MAX_SPAN_US: i64 = 2_000_000;

/// Pictures each fixture holds: a test pattern at 25 fps, cut to 0.4 s.
const FIXTURE_FRAMES: usize = 10;

/// Microseconds one fixture picture is shown for, at 25 fps.
const FRAME_INTERVAL_US: i64 = 40_000;

/// The fixtures' picture size.
const FIXTURE_WIDTH: u32 = 48;
const FIXTURE_HEIGHT: u32 = 32;

/// Which picture the golden RGBA files hold.
const GOLDEN_FRAME: usize = 5;

/// Bytes one RGBA pixel occupies.
const RGBA_BYTES: usize = 4;

/// Colour channels a difference is scored over: the three that carry the picture, not the alpha
/// every decoded frame fills with the same opaque value.
const SCORED_CHANNELS: usize = 3;

/// The peak an 8-bit channel can differ by, the numerator of the peak signal-to-noise ratio.
const PEAK_VALUE: f64 = 255.0;

/// How close the crate's own decode has to come to ffmpeg's, in decibels. A different but correct
/// IDCT and a nearest-sample chroma upsample both cost a little; a wrong picture costs far more.
const MIN_PSNR_DB: f64 = 30.0;

/// Where the committed clips are.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(name)
}

/// Drive a clip through the real playback path, stepping the song clock one picture at a time and
/// keeping the picture shown at each step.
///
/// The clock waits at each step until the decoder has a new picture for it. Without that wait the
/// test would measure how fast the machine running it decodes: `frame_at` deliberately drops every
/// picture the clock has already passed, so a clock that runs ahead of a cold worker thread sees
/// the clip with most of its pictures missing.
fn walk(name: &str, from: i64) -> Walked {
    let mut source = VideoSource::open(fixture(name)).unwrap_or_else(|err| panic!("open {name}: {err}"));
    assert!(source.frame_at(from).is_none(), "{name} hands back a picture before it is played");
    source.play(from);
    let mut frames: Vec<(i64, VideoFrame)> = Vec::new();
    let mut at = from;
    let deadline = Instant::now() + DECODE_TIMEOUT;
    while source.is_playing() {
        assert!(at - from <= MAX_SPAN_US, "{name} ran past {MAX_SPAN_US} us without ending");
        assert!(Instant::now() < deadline, "{name} produced no picture for {at} us within {DECODE_TIMEOUT:?}");
        let shown = source.frame_at(at);
        let fresh = shown.as_ref().is_some_and(|frame| frames.last().map(|(_, last)| last.generation) != Some(frame.generation));
        match (fresh, shown) {
            (true, Some(frame)) => {
                frames.push((at, frame));
                at += FRAME_INTERVAL_US;
            }
            _ => std::thread::yield_now(),
        }
    }
    assert!(source.frame_at(at).is_none(), "{name} hands back a picture after the end of the stream");
    Walked { frames, ends_at: at }
}

/// What one walk of a clip saw: every picture with the song time it was shown at, and the song time
/// the background went blank.
struct Walked {
    frames: Vec<(i64, VideoFrame)>,
    ends_at: i64,
}

/// The peak signal-to-noise ratio between two RGBA buffers, over the colour channels.
fn psnr(decoded: &[u8], golden: &[u8]) -> f64 {
    assert_eq!(decoded.len(), golden.len(), "the decoded frame and the golden frame are different sizes");
    let mut squared_error = 0f64;
    let mut counted = 0usize;
    for (decoded, golden) in decoded.chunks_exact(RGBA_BYTES).zip(golden.chunks_exact(RGBA_BYTES)) {
        for channel in 0..SCORED_CHANNELS {
            let difference = f64::from(decoded[channel]) - f64::from(golden[channel]);
            squared_error += difference * difference;
            counted += 1;
        }
    }
    let mean = squared_error / counted as f64;
    match mean {
        0.0 => f64::INFINITY,
        mean => 10.0 * (PEAK_VALUE * PEAK_VALUE / mean).log10(),
    }
}

/// Assert a clip decoded to the expected count and size, and that its sixth picture matches the
/// golden RGBA well enough.
fn assert_clip(name: &str, golden_name: &str) {
    let walked = walk(name, 0);
    assert_eq!(walked.frames.len(), FIXTURE_FRAMES, "{name} decoded {} pictures", walked.frames.len());
    assert_eq!(walked.ends_at, FIXTURE_FRAMES as i64 * FRAME_INTERVAL_US, "{name} went blank at the wrong song time");
    for (at, frame) in &walked.frames {
        assert_eq!((frame.width, frame.height), (FIXTURE_WIDTH, FIXTURE_HEIGHT), "{name} picture size at {at}");
        assert_eq!(frame.rgba.len(), FIXTURE_WIDTH as usize * FIXTURE_HEIGHT as usize * RGBA_BYTES, "{name} picture bytes at {at}");
        assert!(frame.rgba.chunks_exact(RGBA_BYTES).all(|pixel| pixel[3] == u8::MAX), "{name} pictures are opaque");
    }
    let golden = std::fs::read(fixture(golden_name)).unwrap_or_else(|err| panic!("read {golden_name}: {err}"));
    let score = psnr(&walked.frames[GOLDEN_FRAME].1.rgba, &golden);
    assert!(score >= MIN_PSNR_DB, "{name} picture {GOLDEN_FRAME} scored {score:.2} dB against {golden_name}");
}

#[cfg(feature = "mpeg1")]
#[test]
fn an_mpeg1_program_stream_decodes_to_the_golden_picture() {
    assert_clip("clip.mpg", "frame5.rgba");
}

#[cfg(feature = "mpeg1")]
#[test]
fn an_mpeg1_elementary_stream_decodes_to_the_golden_picture() {
    assert_clip("clip.m1v", "frame5.rgba");
}

#[cfg(feature = "mpeg1")]
#[test]
fn a_video_reports_its_size_before_a_picture_is_decoded() {
    let source = VideoSource::open(fixture("clip.mpg")).expect("open the program stream");
    assert_eq!(source.size(), (FIXTURE_WIDTH, FIXTURE_HEIGHT));
    assert!(!source.is_playing(), "a video that has not been played is not playing");
}

#[cfg(feature = "mpeg1")]
#[test]
fn the_song_clock_offset_is_what_starts_the_video() {
    let started_at = 5_000_000;
    let walked = walk("clip.mpg", started_at);
    assert_eq!(walked.frames.len(), FIXTURE_FRAMES, "the whole clip plays whatever song time it starts at");
    assert_eq!(walked.frames[0].0, started_at, "the first picture is shown at the song time the video was played at");
    assert_eq!(walked.ends_at - started_at, FIXTURE_FRAMES as i64 * FRAME_INTERVAL_US, "the clip lasts its own length, measured from where it started");
}

#[cfg(feature = "mpeg1")]
#[test]
fn a_picture_stays_on_screen_until_the_next_one_is_due() {
    let mut source = VideoSource::open(fixture("clip.mpg")).expect("open the program stream");
    source.play(0);
    let deadline = Instant::now() + DECODE_TIMEOUT;
    let first = loop {
        assert!(Instant::now() < deadline, "the first picture never arrived");
        if let Some(frame) = source.frame_at(0) {
            break frame;
        }
    };
    let held = source.frame_at(FRAME_INTERVAL_US - STEP_US).expect("a picture is still on screen a step before the next one is due");
    assert_eq!(held.generation, first.generation, "the same picture is held between pictures");
    assert_eq!(held.rgba, first.rgba, "the held picture is the same bytes");
}

#[cfg(not(feature = "mpeg1"))]
#[test]
#[ignore = "the mpeg1 backend is not built: build with --features mpeg1 to decode .mpg, .mpeg, .m1v and .m2v"]
fn without_the_mpeg1_backend_a_program_stream_is_unsupported() {
    let error = VideoSource::open(fixture("clip.mpg")).expect_err("there is no MPEG-1 decoder in this build");
    assert!(matches!(error, VideoError::Unsupported(_)), "got {error:?}");
}

#[cfg(feature = "h264")]
#[test]
fn h264_in_an_mp4_decodes_to_the_golden_picture() {
    assert_clip("clip.mp4", "frame5-h264.rgba");
}

#[cfg(not(feature = "h264"))]
#[test]
#[ignore = "the h264 backend is not built: build with --features h264 to decode .mp4 and .m4v"]
fn without_the_h264_backend_an_mp4_is_unsupported() {
    let error = VideoSource::open(fixture("clip.mp4")).expect_err("there is no H.264 decoder in this build");
    assert!(matches!(error, VideoError::Unsupported(_)), "got {error:?}");
}

#[test]
fn a_container_with_no_pure_rust_decoder_is_refused() {
    let error = VideoSource::open(Path::new("bga.wmv")).expect_err("there is no pure-Rust WMV decoder");
    assert_eq!(error, VideoError::Unsupported("wmv files".to_owned()));
}
