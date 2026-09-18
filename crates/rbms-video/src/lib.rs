//! Video backgrounds for a chart whose `#BMPxx` names a movie file rather than a picture.
//!
//! The whole contract is four calls on [`VideoSource`]: open it off the frame loop, [`play`] it at
//! the song time the background timeline switches to it, ask [`frame_at`] for the picture at every
//! later song time, and [`stop`] it when the run ends. There is no seek anywhere, which is the
//! model the reference implementation uses: decoding runs forward on its own thread, the song clock
//! is mapped into the stream by the offset taken at `play`, and a decoder that falls behind has its
//! stale frames dropped rather than the clock waiting for them. At the end of the stream the
//! background goes blank and stays blank; a play background does not loop.
//!
//! [`play`]: VideoSource::play
//! [`frame_at`]: VideoSource::frame_at
//! [`stop`]: VideoSource::stop
//!
//! # Backends
//!
//! Both are pure Rust and build on the pinned stable toolchain, so neither costs a system library
//! or a C toolchain, and both are on by default. A build that wants only one turns the other off.
//!
//! * `mpeg1` — `.mpg` / `.mpeg` program streams and `.m1v` / `.m2v` elementary streams, through
//!   `mpeg-ps` + `mpeg-pes` for the demux and `oxideav-mpeg12video` for the video. That decoder has
//!   no streaming entry point: it reconstructs a whole sequence in one call, so the worker thread
//!   holds every decoded picture of the file at once. That suits the short loops a chart ships and
//!   is the reason a long movie is a bad idea in this container.
//! * `h264` — `.mp4` / `.m4v` holding H.264, through `re_mp4` for the demux and
//!   `rusty_h264-decoder` for the video. This one decodes one access unit at a time, so only the
//!   bounded queue below is ever in flight. Pictures are emitted in decode order stamped with their
//!   own composition time, which is the display order for the baseline streams a chart ships.
//!
//! `.avi`, `.wmv` and `.webm` are recognised as video names so a chart does not try to load one as
//! a picture, but no pure-Rust decoder for them exists to depend on and opening one reports
//! [`VideoError::Unsupported`].

#[cfg(feature = "h264")]
mod h264;
#[cfg(feature = "mpeg1")]
mod mpeg1;
mod yuv;

use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, sync_channel};
use std::thread::JoinHandle;

/// Every file extension a chart may name for a video background, the same list the reference
/// implementation gates on. Lower case; a name is matched without regard to case.
pub const VIDEO_EXTENSIONS: [&str; 9] = ["mpg", "mpeg", "m1v", "m2v", "mp4", "m4v", "avi", "wmv", "webm"];

/// How many decoded frames the worker may run ahead of the screen.
///
/// The queue is what keeps a decoder that is faster than real time from reading a whole film into
/// memory: once it is full the worker blocks on the send until the play screen takes a frame.
const FRAME_QUEUE_DEPTH: usize = 8;

/// Microseconds in one second, for turning a stream's own time base into the song clock.
const MICROS_PER_SECOND: i64 = 1_000_000;

/// Distinguishes one decoded frame from the next, so a draw target can recognise the picture it
/// already holds and skip re-uploading it.
static NEXT_FRAME_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Whether `name` ends in one of [`VIDEO_EXTENSIONS`].
pub fn is_video_name(name: &str) -> bool {
    Path::new(name).extension().and_then(|ext| ext.to_str()).is_some_and(is_video_extension)
}

/// Whether `extension` (without the dot) is one of [`VIDEO_EXTENSIONS`].
pub fn is_video_extension(extension: &str) -> bool {
    VIDEO_EXTENSIONS.iter().any(|known| extension.eq_ignore_ascii_case(known))
}

/// Why a video background could not be opened or played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoError {
    /// The container or codec is one this build has no decoder for. The chart keeps playing with
    /// no background rather than failing.
    Unsupported(String),
    /// The file could not be read.
    Io(String),
    /// The file was read but does not decode.
    Decode(String),
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoError::Unsupported(what) => write!(f, "no decoder for {what}"),
            VideoError::Io(what) => write!(f, "cannot read the video: {what}"),
            VideoError::Decode(what) => write!(f, "cannot decode the video: {what}"),
        }
    }
}

impl std::error::Error for VideoError {}

/// One picture of a video background, ready to upload.
#[derive(Clone)]
pub struct VideoFrame {
    /// Tightly packed RGBA, `width * height * 4` bytes, opaque throughout.
    pub rgba: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    /// This frame's own number. It changes when the picture does and only then, so a draw target
    /// uploads once per frame rather than once per draw.
    pub generation: u64,
}

impl fmt::Debug for VideoFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoFrame")
            .field("bytes", &self.rgba.len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("generation", &self.generation)
            .finish()
    }
}

/// One decoded picture as it leaves the worker thread, with the span of stream time it covers.
pub(crate) struct DecodedFrame {
    /// When this picture starts, in microseconds from the start of the stream.
    pub(crate) start_us: i64,
    /// When the next picture starts, or when the stream ends for the last one. Past this, with
    /// nothing left to decode, the background goes blank.
    pub(crate) end_us: i64,
    pub(crate) rgba: Arc<[u8]>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// What a backend reports about a stream before any picture is decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VideoGeometry {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Where a decoded frame goes. Returns `false` once the screen has stopped listening, which is how
/// a worker learns to abandon the rest of the file.
pub(crate) type FrameSink<'a> = dyn FnMut(DecodedFrame) -> bool + 'a;

/// Which decoder a file's extension asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    /// MPEG-1 / MPEG-2 video, in a program stream or on its own.
    Mpeg12,
    /// H.264 in an MP4 container.
    H264Mp4,
}

/// The decoder an extension names, or `None` when no pure-Rust decoder for it exists.
fn backend_for(extension: &str) -> Option<Backend> {
    match extension.to_ascii_lowercase().as_str() {
        "mpg" | "mpeg" | "m1v" | "m2v" => Some(Backend::Mpeg12),
        "mp4" | "m4v" => Some(Backend::H264Mp4),
        _ => None,
    }
}

/// Read the stream's geometry without decoding a picture, so a caller learns straight away whether
/// this build can play the file.
fn probe(backend: Backend, bytes: &[u8]) -> Result<VideoGeometry, VideoError> {
    match backend {
        #[cfg(feature = "mpeg1")]
        Backend::Mpeg12 => mpeg1::probe(bytes),
        #[cfg(not(feature = "mpeg1"))]
        Backend::Mpeg12 => {
            let _ = bytes;
            Err(VideoError::Unsupported("MPEG-1 video: this build has no mpeg1 backend".to_owned()))
        }
        #[cfg(feature = "h264")]
        Backend::H264Mp4 => h264::probe(bytes),
        #[cfg(not(feature = "h264"))]
        Backend::H264Mp4 => {
            let _ = bytes;
            Err(VideoError::Unsupported("H.264 in MP4: this build has no h264 backend".to_owned()))
        }
    }
}

/// Decode the whole stream forward, handing each picture to `sink` in the order it is displayed.
fn decode(backend: Backend, bytes: &[u8], geometry: VideoGeometry, sink: &mut FrameSink<'_>) -> Result<(), VideoError> {
    match backend {
        #[cfg(feature = "mpeg1")]
        Backend::Mpeg12 => mpeg1::decode(bytes, geometry, sink),
        #[cfg(not(feature = "mpeg1"))]
        Backend::Mpeg12 => {
            let _ = (bytes, geometry, sink);
            Err(VideoError::Unsupported("MPEG-1 video: this build has no mpeg1 backend".to_owned()))
        }
        #[cfg(feature = "h264")]
        Backend::H264Mp4 => h264::decode(bytes, geometry, sink),
        #[cfg(not(feature = "h264"))]
        Backend::H264Mp4 => {
            let _ = (bytes, geometry, sink);
            Err(VideoError::Unsupported("H.264 in MP4: this build has no h264 backend".to_owned()))
        }
    }
}

/// One video background, decoding forward on its own thread.
///
/// Dropping it stops the worker, so a run that ends mid-video leaves nothing behind.
pub struct VideoSource {
    width: u32,
    height: u32,
    /// Frames from the worker, in display order. Taken away by [`VideoSource::release`], which is
    /// what tells a worker blocked on a full queue to give up.
    frames: Option<Receiver<DecodedFrame>>,
    worker: Option<JoinHandle<()>>,
    cancel: Arc<AtomicBool>,
    /// The song time frame 0 is shown at, set by [`VideoSource::play`]. `None` until then, and
    /// [`VideoSource::frame_at`] hands back nothing while it is.
    offset_us: Option<i64>,
    /// The picture on screen, with the generation it was given when it arrived.
    current: Option<(DecodedFrame, u64)>,
    /// The next picture, already decoded but not due yet.
    next: Option<DecodedFrame>,
    /// Set once the worker has closed the queue: there are no more pictures after `next`.
    drained: bool,
    /// Set once the last picture's span has passed. The background is blank from here on.
    ended: bool,
}

impl fmt::Debug for VideoSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VideoSource")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("playing", &self.offset_us.is_some())
            .field("drained", &self.drained)
            .field("ended", &self.ended)
            .finish()
    }
}

impl VideoSource {
    /// Open `path` and start decoding it.
    ///
    /// The container is parsed here, on the calling thread, so an unplayable file is reported
    /// before a run starts rather than silently going blank halfway through. Decoding itself moves
    /// to a worker thread that runs ahead by at most [`FRAME_QUEUE_DEPTH`] frames and stops when
    /// this source is dropped.
    pub fn open(path: impl AsRef<Path>) -> Result<VideoSource, VideoError> {
        let path = path.as_ref();
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default();
        let backend = backend_for(extension).ok_or_else(|| VideoError::Unsupported(format!("{extension} files")))?;
        let bytes = std::fs::read(path).map_err(|err| VideoError::Io(format!("{}: {err}", path.display())))?;
        let geometry = probe(backend, &bytes)?;
        let (sender, frames) = sync_channel(FRAME_QUEUE_DEPTH);
        let cancel = Arc::new(AtomicBool::new(false));
        let worker = std::thread::spawn({
            let cancel = Arc::clone(&cancel);
            move || decode_into(backend, &bytes, geometry, &sender, &cancel)
        });
        Ok(VideoSource {
            width: geometry.width,
            height: geometry.height,
            frames: Some(frames),
            worker: Some(worker),
            cancel,
            offset_us: None,
            current: None,
            next: None,
            drained: false,
            ended: false,
        })
    }

    /// The stream's own size, known before the first picture is decoded.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Start the video at `song_us`, which becomes the song time frame 0 is shown at.
    ///
    /// There is no seek: the offset is what maps the song clock onto the stream, so a video that
    /// starts late simply starts at its first frame late. A second call is ignored, because
    /// restarting would mean rewinding a stream that only decodes forward.
    pub fn play(&mut self, song_us: i64) {
        if self.offset_us.is_none() {
            self.offset_us = Some(song_us);
        }
    }

    /// Whether [`VideoSource::play`] has been called and the stream has not ended.
    pub fn is_playing(&self) -> bool {
        self.offset_us.is_some() && !self.ended
    }

    /// The newest picture whose start is at or before `song_us`, or `None` before the video is
    /// played and once it has ended.
    ///
    /// Frames the clock has already passed are dropped rather than shown: a decoder that falls
    /// behind catches up by skipping, never by holding the song back. While the decoder is merely
    /// starved the last picture stays on screen.
    pub fn frame_at(&mut self, song_us: i64) -> Option<VideoFrame> {
        let offset = self.offset_us?;
        if self.ended {
            return None;
        }
        let target = song_us - offset;
        self.advance_to(target);
        let exhausted = self.drained && self.next.is_none();
        let past_the_last = self.current.as_ref().is_none_or(|(frame, _)| target >= frame.end_us);
        if exhausted && past_the_last {
            self.ended = true;
            self.release();
            self.current = None;
            return None;
        }
        self.current.as_ref().map(|(frame, generation)| VideoFrame {
            rgba: Arc::clone(&frame.rgba),
            width: frame.width,
            height: frame.height,
            generation: *generation,
        })
    }

    /// Stop the video and the thread decoding it. The background is blank from here on.
    pub fn stop(&mut self) {
        self.release();
        self.offset_us = None;
        self.current = None;
        self.ended = true;
    }

    /// Take every picture the worker has produced whose start has passed, keeping only the last.
    fn advance_to(&mut self, target: i64) {
        loop {
            if self.next.is_none() {
                match self.frames.as_ref().map(Receiver::try_recv) {
                    Some(Ok(frame)) => self.next = Some(frame),
                    Some(Err(TryRecvError::Empty)) => return,
                    Some(Err(TryRecvError::Disconnected)) | None => {
                        self.drained = true;
                        return;
                    }
                }
            }
            match self.next.take_if(|frame| frame.start_us <= target) {
                Some(frame) => self.current = Some((frame, NEXT_FRAME_GENERATION.fetch_add(1, Ordering::Relaxed))),
                None => return,
            }
        }
    }

    /// Close the queue and join the worker. Dropping the receiver is what wakes a worker blocked on
    /// a full queue, so this returns promptly even mid-file.
    fn release(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.frames = None;
        self.next = None;
        self.drained = true;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for VideoSource {
    fn drop(&mut self) {
        self.release();
    }
}

/// The worker thread: decode forward and push each picture into the bounded queue, stopping as
/// soon as the screen has let go of the other end.
fn decode_into(backend: Backend, bytes: &[u8], geometry: VideoGeometry, sender: &SyncSender<DecodedFrame>, cancel: &AtomicBool) {
    let mut sink = |frame: DecodedFrame| !cancel.load(Ordering::Relaxed) && sender.send(frame).is_ok();
    let _ = decode(backend, bytes, geometry, &mut sink);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_extension_list_is_recognised() {
        for extension in VIDEO_EXTENSIONS {
            assert!(is_video_extension(extension), "{extension} is a video extension");
            assert!(is_video_name(&format!("bga.{}", extension.to_uppercase())), "{extension} is matched without regard to case");
        }
        assert!(!is_video_extension("png"));
        assert!(!is_video_name("bga.bmp"));
        assert!(!is_video_name("bga"));
    }

    #[test]
    fn only_the_two_built_backends_have_a_decoder() {
        assert_eq!(backend_for("MPG"), Some(Backend::Mpeg12));
        assert_eq!(backend_for("m2v"), Some(Backend::Mpeg12));
        assert_eq!(backend_for("mp4"), Some(Backend::H264Mp4));
        assert_eq!(backend_for("m4v"), Some(Backend::H264Mp4));
        for extension in ["avi", "wmv", "webm", "png"] {
            assert_eq!(backend_for(extension), None, "{extension} has no pure-Rust decoder");
        }
    }

    #[test]
    fn a_container_without_a_decoder_is_refused_by_name() {
        let error = VideoSource::open(Path::new("bga.webm")).expect_err("webm has no decoder");
        assert_eq!(error, VideoError::Unsupported("webm files".to_owned()));
    }

    #[test]
    fn a_missing_file_reports_the_read_that_failed() {
        let error = VideoSource::open(Path::new("no-such-background.mpg")).expect_err("the file does not exist");
        assert!(matches!(error, VideoError::Io(_)), "got {error:?}");
    }
}
