//! The decoder there is: H.264 out of an MP4 file, through OpenH264.
//!
//! Samples are read from the file one at a time, in the order they are stored, rewritten as the
//! byte stream the decoder reads ([`AnnexB`]) and fed to it. The decoder hands frames back in the
//! order they are shown, some samples later than it was fed them, so the frames still inside it
//! when the last sample has gone in are asked for at the end.
//!
//! The decoder is not told when a sample is shown, and does not say which sample a frame came
//! from. It does not need to: the n-th frame out is the n-th frame shown, and the file's own tables
//! say when that is. A sample that comes to nothing -- the decoder would not take it -- gives up
//! its place in that count, so the frames after a damaged stretch are still shown at their own
//! times, and the frame before it stays on show in between.
//!
//! Decoding can also start at any of the samples the file marks for it ([`Movie::keys`]), which is
//! how a time far from where the decoder is gets reached without decoding everything before it.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use openh264::OpenH264API;
use openh264::decoder::{DecodedYUV, Decoder, DecoderConfig, Flush};
use openh264::formats::YUVSource;

use crate::bitstream::AnnexB;
use crate::color::{ColorSpec, Yuv420, yuv420_to_rgba};
use crate::mp4::Movie;
use crate::{VideoDecoder, VideoError, VideoInfo};

/// A frame the decoder gave up at the end of the stream, copied out of it: the decoder gives up
/// everything it still holds at once, and a caller takes one frame at a time.
#[derive(Debug)]
struct HeldFrame {
    y: Vec<u8>,
    u: Vec<u8>,
    v: Vec<u8>,
    y_stride: usize,
    uv_stride: usize,
    width: usize,
    height: usize,
}

impl HeldFrame {
    fn of(decoded: &DecodedYUV<'_>) -> HeldFrame {
        let (width, height) = decoded.dimensions();
        let (y_stride, uv_stride, _) = decoded.strides();
        HeldFrame { y: decoded.y().to_vec(), u: decoded.u().to_vec(), v: decoded.v().to_vec(), y_stride, uv_stride, width, height }
    }

    fn planes(&self) -> Yuv420<'_> {
        Yuv420 { y: &self.y, u: &self.u, v: &self.v, y_stride: self.y_stride, uv_stride: self.uv_stride, width: self.width, height: self.height }
    }
}

/// The planes of a frame still inside the decoder.
fn planes_of<'a>(decoded: &'a DecodedYUV<'_>) -> Yuv420<'a> {
    let (width, height) = decoded.dimensions();
    let (y_stride, uv_stride, _) = decoded.strides();
    Yuv420 { y: decoded.y(), u: decoded.u(), v: decoded.v(), y_stride, uv_stride, width, height }
}

/// Writes `planes` into `rgba` as a frame of `info`'s size.
///
/// A decoded frame larger than the movie says it is -- a decoder that did not crop -- is cut to the
/// movie's size from its top left corner. One that is smaller cannot be the movie's frame.
fn write_frame(info: &VideoInfo, color: ColorSpec, planes: Yuv420<'_>, rgba: &mut Vec<u8>) -> Result<(), VideoError> {
    let (width, height) = (info.width as usize, info.height as usize);
    if planes.width < width || planes.height < height {
        return Err(VideoError::Decode(format!("a frame came out {}x{}, smaller than the movie's {width}x{height}", planes.width, planes.height)));
    }
    rgba.resize(info.frame_bytes(), 0);
    if !yuv420_to_rgba(color, &Yuv420 { width, height, ..planes }, rgba) {
        return Err(VideoError::Decode("a frame came out with planes shorter than its size".into()));
    }
    Ok(())
}

/// What feeding the decoder one sample got out of it.
enum Fed {
    /// A frame, which may be of a sample fed some time ago.
    Frame,
    /// Nothing yet: the decoder is holding the sample's frame back until the ones shown before it
    /// have gone out.
    Nothing,
    /// The decoder would not take the sample. The stream goes on with the next.
    Refused,
}

/// A decoder that has not been fed anything.
fn fresh_decoder() -> Result<Decoder, VideoError> {
    let config = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
    Decoder::with_api_config(OpenH264API::from_source(), config).map_err(|error| VideoError::Decode(error.to_string()))
}

/// One MP4 file's H.264 track being decoded.
pub(crate) struct Mp4H264Decoder {
    file: File,
    movie: Movie,
    decoder: Decoder,
    stream: AnnexB,
    /// The next sample to feed.
    next_sample: usize,
    /// Which frame in show order comes out next: how many have come out since the first sample
    /// was fed, the samples that came to nothing counted with them.
    shown: usize,
    /// The frames the decoder gave up at the end of the stream, oldest first.
    held: VecDeque<HeldFrame>,
    /// Whether the decoder has been asked for what it still held.
    drained: bool,
    /// The sample being fed, as it is in the file and as the decoder reads it.
    sample: Vec<u8>,
    packet: Vec<u8>,
    /// How many samples came to nothing since the movie was opened: the decoder refused them, or
    /// they held no NAL unit to feed it.
    lost: u64,
}

impl std::fmt::Debug for Mp4H264Decoder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Mp4H264Decoder")
            .field("info", &self.movie.info)
            .field("next_sample", &self.next_sample)
            .field("shown", &self.shown)
            .field("lost", &self.lost)
            .finish_non_exhaustive()
    }
}

impl Mp4H264Decoder {
    /// Opens the movie at `path` and reads where its samples are.
    pub(crate) fn open(path: &Path) -> Result<Mp4H264Decoder, VideoError> {
        let mut file = File::open(path).map_err(VideoError::io)?;
        let file_bytes = file.metadata().map_err(VideoError::io)?.len();
        let movie = Movie::read(&mut file, file_bytes)?;
        let stream = AnnexB::new(movie.config.clone());
        Ok(Mp4H264Decoder {
            file,
            movie,
            decoder: fresh_decoder()?,
            stream,
            next_sample: 0,
            shown: 0,
            held: VecDeque::new(),
            drained: false,
            sample: Vec::new(),
            packet: Vec::new(),
            lost: 0,
        })
    }

    /// Counts one more frame out and answers when it is shown. A frame past the last the tables
    /// count has no time of its own and is shown with the last.
    fn count_frame(&mut self) -> i64 {
        let at = self.movie.show_us.get(self.shown).or(self.movie.show_us.last()).copied().unwrap_or(0);
        self.shown += 1;
        at
    }

    /// Counts a sample that came to nothing, and gives up the place its frame had among the frames
    /// shown: the next frame out is the one after it.
    fn lose_sample(&mut self) {
        self.lost += 1;
        self.shown += 1;
    }

    /// Reads sample `index` out of the file into `self.sample`.
    fn read_sample(&mut self, index: usize) -> Result<(), VideoError> {
        let sample = self.movie.samples[index];
        self.sample.resize(sample.bytes, 0);
        self.file.seek(SeekFrom::Start(sample.offset)).map_err(VideoError::io)?;
        self.file.read_exact(&mut self.sample).map_err(VideoError::io)
    }

    /// Makes sample `sample` the next one fed, to a decoder that has seen nothing. It has to be one
    /// of [`Movie::keys`], which is what makes it the `sample`-th frame shown as well.
    fn start_at(&mut self, sample: usize) -> Result<(), VideoError> {
        self.decoder = fresh_decoder()?;
        self.stream.rewind();
        self.next_sample = sample;
        self.shown = sample;
        self.held.clear();
        self.drained = false;
        Ok(())
    }
}

impl VideoDecoder for Mp4H264Decoder {
    fn info(&self) -> VideoInfo {
        self.movie.info
    }

    fn next_time_us(&self) -> Option<i64> {
        let more = self.next_sample < self.movie.samples.len() || !self.drained || !self.held.is_empty();
        if more { self.movie.show_us.get(self.shown).copied() } else { None }
    }

    fn next_frame(&mut self, mut rgba: Option<&mut Vec<u8>>) -> Result<Option<i64>, VideoError> {
        let (info, color) = (self.movie.info, self.movie.color);
        while self.next_sample < self.movie.samples.len() {
            self.read_sample(self.next_sample)?;
            self.next_sample += 1;
            self.stream.convert(&self.sample, &mut self.packet);
            if self.packet.is_empty() {
                self.lose_sample();
                continue;
            }
            let fed = match self.decoder.decode(&self.packet) {
                Ok(Some(decoded)) => {
                    if let Some(rgba) = rgba.as_deref_mut() {
                        write_frame(&info, color, planes_of(&decoded), rgba)?;
                    }
                    Fed::Frame
                }
                Ok(None) => Fed::Nothing,
                Err(_) => Fed::Refused,
            };
            match fed {
                Fed::Frame => return Ok(Some(self.count_frame())),
                Fed::Nothing => {}
                Fed::Refused => self.lose_sample(),
            }
        }
        if !std::mem::replace(&mut self.drained, true) {
            let remaining = self.decoder.flush_remaining().map_err(|error| VideoError::Decode(error.to_string()))?;
            self.held = remaining.iter().map(HeldFrame::of).collect();
        }
        let Some(held) = self.held.pop_front() else {
            return Ok(None);
        };
        if let Some(rgba) = rgba {
            write_frame(&info, color, held.planes(), rgba)?;
        }
        Ok(Some(self.count_frame()))
    }

    fn rewind(&mut self) -> Result<(), VideoError> {
        self.start_at(0)
    }

    /// Goes to the latest sample decoding can start from that is shown no later than `time_us`,
    /// unless decoding on from here reaches `time_us` sooner.
    ///
    /// The file's word for where decoding can start is checked against the sample itself before it
    /// is acted on: a sample marked as a start that holds no IDR picture is struck off the list and
    /// the one before it tried, down to the first sample, which a movie always starts from.
    fn seek(&mut self, time_us: i64) -> Result<bool, VideoError> {
        let can_reach_from_here = self.next_time_us().is_some_and(|next_us| next_us <= time_us);
        loop {
            let reachable = self.movie.keys.partition_point(|key| self.movie.show_us.get(*key).is_some_and(|at_us| *at_us <= time_us));
            let slot = reachable.saturating_sub(1);
            let Some(key) = self.movie.keys.get(slot).copied() else {
                return Ok(false);
            };
            if can_reach_from_here && key <= self.next_sample {
                return Ok(false);
            }
            if slot > 0 {
                self.read_sample(key)?;
                if !self.stream.starts_afresh(&self.sample) {
                    self.movie.keys.remove(slot);
                    continue;
                }
            }
            self.start_at(key)?;
            return Ok(true);
        }
    }

    fn lost_samples(&self) -> u64 {
        self.lost
    }
}

#[cfg(test)]
mod tests;
