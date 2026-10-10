//! Movies for the tests of everything that plays one: a few lines of text where a movie file would
//! be, opened as a movie whose every frame is one flat colour that says which frame it is.
//!
//! A test cannot ship a real movie and has no encoder to make one with, and what it wants to know
//! is never whether a codec works: it is which frame is on show when, and what becomes of the
//! thing decoding them. So the file a document names is written as a script
//! ([`ScriptedMovie::write`]) and the worker that opens movies opens it as one ([`open`]). A file
//! that is not a script is left to the real decoder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use rbms_video::{VideoDecoder, VideoError, VideoInfo};

/// What a script file starts with.
const MAGIC: &str = "rbms-scripted-movie";

/// How much brighter each frame's red is than the frame before it.
pub(crate) const RED_STEP: u8 = 24;

/// How many scripted decoders are open on each file, for the tests that ask what became of theirs.
static OPEN: Mutex<BTreeMap<PathBuf, usize>> = Mutex::new(BTreeMap::new());

/// How many decoders are open on the script at `path` right now.
pub(crate) fn open_on(path: &Path) -> usize {
    OPEN.lock().unwrap_or_else(PoisonError::into_inner).get(path).copied().unwrap_or(0)
}

/// A movie described rather than encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScriptedMovie {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) frames: u32,
    /// Microseconds each frame is on show.
    pub(crate) interval_us: i64,
    /// The frame that fails to decode, when the movie is one that breaks.
    pub(crate) breaks_at: Option<u32>,
    /// Whether the file cannot be opened as a movie at all.
    pub(crate) unplayable: bool,
    /// The frame that is damaged, when the movie has one: it never comes out, and the frames after
    /// it come out at their own times.
    pub(crate) loses: Option<u32>,
}

impl ScriptedMovie {
    /// A movie of `frames` frames of `width` by `height`, each on show for `interval_us`.
    pub(crate) fn new(width: u32, height: u32, frames: u32, interval_us: i64) -> ScriptedMovie {
        ScriptedMovie { width, height, frames, interval_us, breaks_at: None, unplayable: false, loses: None }
    }

    /// The colour every pixel of frame `index` is.
    pub(crate) fn pixel_of(index: u32) -> [u8; 4] {
        let red = (index as u8).wrapping_mul(RED_STEP);
        [red, u8::MAX - red, index as u8, u8::MAX]
    }

    /// Writes the script where a movie file would be.
    pub(crate) fn write(&self, path: &Path) {
        let breaks_at = self.breaks_at.map_or(String::new(), |frame| frame.to_string());
        let loses = self.loses.map_or(String::new(), |frame| frame.to_string());
        let lines = [
            MAGIC.to_owned(),
            self.width.to_string(),
            self.height.to_string(),
            self.frames.to_string(),
            self.interval_us.to_string(),
            breaks_at,
            self.unplayable.to_string(),
            loses,
        ];
        std::fs::write(path, lines.join("\n")).expect("the scripted movie is writable");
    }

    /// Reads a script back, or `None` for a file that is not one.
    fn read(path: &Path) -> Option<ScriptedMovie> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut lines = text.lines();
        if lines.next()? != MAGIC {
            return None;
        }
        Some(ScriptedMovie {
            width: lines.next()?.parse().ok()?,
            height: lines.next()?.parse().ok()?,
            frames: lines.next()?.parse().ok()?,
            interval_us: lines.next()?.parse().ok()?,
            breaks_at: lines.next()?.parse().ok(),
            unplayable: lines.next()? == "true",
            loses: lines.next().and_then(|line| line.parse().ok()),
        })
    }
}

/// A script being played.
#[derive(Debug)]
struct Playing {
    script: ScriptedMovie,
    path: PathBuf,
    at: u32,
    /// How many frames were passed over as damaged since the script was opened.
    lost: u64,
}

impl Drop for Playing {
    fn drop(&mut self) {
        if let Some(open) = OPEN.lock().unwrap_or_else(PoisonError::into_inner).get_mut(&self.path) {
            *open = open.saturating_sub(1);
        }
    }
}

impl VideoDecoder for Playing {
    fn info(&self) -> VideoInfo {
        let script = &self.script;
        VideoInfo { width: script.width, height: script.height, frame_count: script.frames, duration_us: i64::from(script.frames) * script.interval_us }
    }

    fn next_time_us(&self) -> Option<i64> {
        (self.at < self.script.frames).then(|| i64::from(self.at) * self.script.interval_us)
    }

    fn next_frame(&mut self, rgba: Option<&mut Vec<u8>>) -> Result<Option<i64>, VideoError> {
        if self.script.loses == Some(self.at) {
            self.at += 1;
            self.lost += 1;
        }
        if self.at >= self.script.frames {
            return Ok(None);
        }
        if self.script.breaks_at == Some(self.at) {
            return Err(VideoError::Decode("the scripted movie breaks here".into()));
        }
        if let Some(rgba) = rgba {
            let pixels = (self.script.width * self.script.height) as usize;
            *rgba = ScriptedMovie::pixel_of(self.at).repeat(pixels);
        }
        self.at += 1;
        Ok(Some(i64::from(self.at - 1) * self.script.interval_us))
    }

    fn rewind(&mut self) -> Result<(), VideoError> {
        self.at = 0;
        Ok(())
    }

    fn lost_samples(&self) -> u64 {
        self.lost
    }
}

/// Opens the file at `path` as the movie it scripts, or answers `None` when it is not a script.
pub(crate) fn open(path: &Path) -> Option<Result<Box<dyn VideoDecoder>, VideoError>> {
    let script = ScriptedMovie::read(path)?;
    if script.unplayable {
        return Some(Err(VideoError::Unsupported("the scripted movie is not one that plays".into())));
    }
    *OPEN.lock().unwrap_or_else(PoisonError::into_inner).entry(path.to_path_buf()).or_default() += 1;
    Some(Ok(Box::new(Playing { script, path: path.to_path_buf(), at: 0, lost: 0 })))
}
