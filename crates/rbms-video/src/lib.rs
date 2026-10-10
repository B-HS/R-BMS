//! Decoding the movie files a skin draws from into RGBA frames.
//!
//! A skin may name a movie where it would name an image, and draws whatever frame of it is due.
//! This crate is everything between that file and those pixels, and nothing that draws:
//!
//! - [`VideoDecoder`] is one open movie read front to back: how large and how long it is, its next
//!   frame as RGBA, back to its first frame, and over to wherever in it decoding can start.
//!   [`open`] answers one for a file.
//! - [`VideoPlayer`] runs a decoder on a thread of its own and keeps a few frames ahead, so
//!   whoever draws asks only for the frame that is due now and never waits on a decode, wherever
//!   its clock has gone to.
//! - [`color`], [`bitstream`] and [`container`] are the parts in between that are plain arithmetic
//!   on bytes: the colour conversion, the rewrite of a file's samples into the stream a decoder
//!   reads, and the walk that checks a file's boxes before anything trusts them.
//!
//! The one decoder there is reads H.264 out of an MP4 file, and is compiled in by the `openh264`
//! feature. With the feature off the crate keeps its whole surface, no decoder is left in it and
//! [`open`] answers [`VideoError::Disabled`] for every file, so whatever is built on it draws a
//! movie's place empty instead of failing to build.

#![forbid(unsafe_code)]

use std::fmt::Debug;
use std::path::Path;

pub mod bitstream;
pub mod color;
pub mod container;
#[cfg(feature = "openh264")]
mod h264;
#[cfg(feature = "openh264")]
mod mp4;
mod player;

pub use player::{DEFAULT_QUEUE_FRAMES, VideoFrame, VideoPlayer};

/// Microseconds in a second, which is the unit every time here is said in.
pub const MICROS_PER_SECOND: i64 = 1_000_000;

/// Why a movie could not be opened or decoded.
#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    /// This build has no decoder in it.
    #[error("video support is not built in")]
    Disabled,
    #[error("the file could not be read: {0}")]
    Io(String),
    /// The file is not what its boxes say it is.
    #[error("the file is damaged: {0}")]
    Malformed(String),
    /// The file is whole, and holds something this build does not decode.
    #[error("{0}")]
    Unsupported(String),
    /// The decoder gave up on the stream.
    #[error("the stream could not be decoded: {0}")]
    Decode(String),
}

impl VideoError {
    /// A failed read, as an error of this crate.
    pub(crate) fn io(error: std::io::Error) -> VideoError {
        VideoError::Io(error.to_string())
    }
}

/// What a movie says about itself before any of it is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoInfo {
    /// The size of a frame in pixels.
    pub width: u32,
    pub height: u32,
    /// How many frames one pass over the movie has.
    pub frame_count: u32,
    /// How long one pass over the movie lasts, in microseconds. A frame's time is measured from the
    /// start of the pass it belongs to, so a movie played round and round shows frame `n` of pass
    /// `p` at `p * duration_us` plus that.
    pub duration_us: i64,
}

impl VideoInfo {
    /// How long one frame is on show, on average, in microseconds. A movie of no frames answers its
    /// whole length.
    pub fn frame_interval_us(&self) -> i64 {
        self.duration_us / i64::from(self.frame_count.max(1))
    }

    /// Bytes one frame takes as RGBA.
    pub fn frame_bytes(&self) -> usize {
        self.width as usize * self.height as usize * color::RGBA_BYTES
    }
}

/// One open movie, decoded front to back.
///
/// Frames come out in the order they are shown. A decoder is driven from one thread at a time, and
/// is handed to the thread that drives it.
pub trait VideoDecoder: Send + Debug {
    /// The movie's size and length.
    fn info(&self) -> VideoInfo;

    /// When the frame the next call to [`VideoDecoder::next_frame`] decodes is shown, in
    /// microseconds from the start of the movie, when the decoder can tell before decoding it.
    /// `None` at the end of the movie.
    fn next_time_us(&self) -> Option<i64>;

    /// Decodes the next frame and answers when it is shown, or `None` at the end of the movie.
    ///
    /// With `rgba` the frame's pixels are written into it, top row first, replacing what it held:
    /// exactly [`VideoInfo::frame_bytes`] of them. Without it the frame is decoded and thrown away,
    /// which is what catching up with a clock costs: every frame has to be decoded for the ones
    /// after it, but only the one that is shown has to be turned into pixels.
    ///
    /// A frame the stream has damaged is left out rather than reported, and counted
    /// ([`VideoDecoder::lost_samples`]): the frames after it still come out at their own times. An
    /// error here means nothing more is coming.
    fn next_frame(&mut self, rgba: Option<&mut Vec<u8>>) -> Result<Option<i64>, VideoError>;

    /// Goes back to the first frame.
    fn rewind(&mut self) -> Result<(), VideoError>;

    /// Gets ready for the frame shown at `time_us` with as little to decode before it as the movie
    /// allows, and answers whether that moved the decoder.
    ///
    /// The decoder goes to the latest frame decoding can start from that is shown no later than
    /// `time_us`. It stays where it is when it is already between that frame and the one shown at
    /// `time_us`, which decoding on from here reaches sooner.
    ///
    /// A movie that can only be decoded from its first frame goes back to it when it is past
    /// `time_us` and stays otherwise, which is what this does unless a decoder knows better.
    fn seek(&mut self, time_us: i64) -> Result<bool, VideoError> {
        if self.next_time_us().is_some_and(|next_us| next_us <= time_us) {
            return Ok(false);
        }
        self.rewind()?;
        Ok(true)
    }

    /// How many of the movie's samples have come to nothing since it was opened: damage the
    /// decoder would not take, whose frames were left out.
    fn lost_samples(&self) -> u64 {
        0
    }
}

/// Whether this build can decode anything at all.
pub const fn is_enabled() -> bool {
    cfg!(feature = "openh264")
}

/// Opens the movie at `path`.
///
/// Only the file's header is read here: which track is decoded, its size, its length and where its
/// samples are. Nothing is decoded until the first frame is asked for.
#[cfg(feature = "openh264")]
pub fn open(path: &Path) -> Result<Box<dyn VideoDecoder>, VideoError> {
    Ok(Box::new(h264::Mp4H264Decoder::open(path)?))
}

/// Opens the movie at `path`, which this build cannot: it has no decoder in it.
#[cfg(not(feature = "openh264"))]
pub fn open(path: &Path) -> Result<Box<dyn VideoDecoder>, VideoError> {
    let _ = path;
    Err(VideoError::Disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_movie_says_how_long_a_frame_is_on_show_and_how_large_it_is() {
        let info = VideoInfo { width: 1920, height: 1080, frame_count: 600, duration_us: 20_020_000 };
        assert_eq!(info.frame_interval_us(), 33_366);
        assert_eq!(info.frame_bytes(), 1920 * 1080 * 4);
        let empty = VideoInfo { width: 0, height: 0, frame_count: 0, duration_us: 0 };
        assert_eq!((empty.frame_interval_us(), empty.frame_bytes()), (0, 0));
    }

    #[cfg(not(feature = "openh264"))]
    #[test]
    fn a_build_with_no_codec_opens_nothing() {
        assert!(!is_enabled());
        assert!(matches!(open(Path::new("anything.mp4")), Err(VideoError::Disabled)));
    }

    #[test]
    fn a_file_that_is_not_there_is_an_error_and_not_a_panic() {
        assert!(open(Path::new("/nonexistent/rbms-video/missing.mp4")).is_err());
    }
}
